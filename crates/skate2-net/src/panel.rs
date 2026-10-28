//! The moderation panel (PC port addition): an HTTP page at `/moderationpanel?key=<key>` on the
//! port `server-config.json` names ([`crate::moderation`]). It lists the signed-in players, to
//! chat-shadowban or IP-ban, and the bans in force, to lift. A request without the key in
//! `moderation-key.txt` is answered as one for a page that does not exist, and an address that
//! sent [`MAX_FAILURES`] wrong keys is answered so for the rest of [`FAILURE_WINDOW`], whatever
//! key it sends. The page runs no script, loads nothing else and is never cached, and it shows no
//! player's address: its boxes send references that the panel keeps for the addresses it listed.
//!
//! The same port serves the public statistics at [`STATS_PATH`] ([`crate::stats`]) to anyone,
//! with or without a key file: no address, no private game, no one outside a public game.
//!
//! The panel serves on threads of its own and asks the server's loop for what it shows and does
//! ([`Request`]), so the loop never waits on a browser.

use crate::moderation::{self, PanelKey};
use crate::stats::{self, Stats};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Read, Write as _};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// The panel's path.
pub const PATH: &str = "/moderationpanel";
/// The public statistics' path.
pub const STATS_PATH: &str = "/serverstats";
/// How long one reading of the statistics answers every request: a stream of requests asks the
/// server's loop at most once in this time.
const STATS_TIME: Duration = Duration::from_secs(1);
/// Connections served at once; one past them is closed unanswered.
const MAX_CONNECTIONS: usize = 8;
/// How long a request may take to arrive whole, and one read or write to wait.
const REQUEST_TIME: Duration = Duration::from_secs(10);
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// The longest request line and headers, and the longest form.
const MAX_HEAD: usize = 8 * 1024;
const MAX_BODY: usize = 16 * 1024;
/// Wrong keys an address may send in a window before its requests are refused for the rest of
/// it.
pub const MAX_FAILURES: u32 = 10;
pub const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);
/// The addresses whose wrong keys are counted at most: past them the oldest count is forgotten.
const MAX_COUNTED: usize = 4096;
/// How long the panel waits for the server's loop to answer.
const ANSWER_TIME: Duration = Duration::from_secs(2);
/// How long an action's outcome waits to be shown on the page.
const NOTICE_TIME: Duration = Duration::from_secs(60);
/// The most players or bans one action takes.
const MAX_TARGETS: usize = 512;
/// How long the page's references to the addresses it listed are kept since they were last
/// listed.
const SHOWN_TIME: Duration = Duration::from_secs(60 * 60);

/// What the panel asks of the server's loop.
#[derive(Debug)]
pub enum Request {
    /// What the page shows.
    View(Sender<View>),
    /// An action: the answer is its outcome, for the page.
    Act(Action, Sender<String>),
    /// The public statistics.
    Stats(Sender<Stats>),
}

/// An action of the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Chat-shadowbans these addresses, each under the name it was shown with.
    ChatShadowban(Vec<(IpAddr, String)>),
    /// Bans these addresses, each under the name it was shown with.
    IpBan(Vec<(IpAddr, String)>),
    /// Lifts these chat shadowbans and IP bans.
    Lift { chat: Vec<IpAddr>, ip: Vec<IpAddr> },
}

impl Action {
    fn is_empty(&self) -> bool {
        match self {
            Self::ChatShadowban(players) | Self::IpBan(players) => players.is_empty(),
            Self::Lift { chat, ip } => chat.is_empty() && ip.is_empty(),
        }
    }
}

/// A signed-in player, as the page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub name: String,
    pub address: IpAddr,
    /// The game version its hello named.
    pub version: String,
    /// Its game, and whether it hosts it.
    pub game: Option<(u32, bool)>,
    /// How long it has been connected.
    pub online: Duration,
    pub chat_shadowbanned: bool,
}

/// What the page shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    pub players: Vec<Player>,
    pub games: usize,
    pub chat_shadowbans: Vec<(IpAddr, String)>,
    pub ip_bans: Vec<(IpAddr, String)>,
}

/// A line for the server's console.
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

struct Panel {
    /// The moderation panel's key; without one only the statistics are served.
    key: Option<PanelKey>,
    requests: Sender<Request>,
    /// The statistics last read, and when.
    stats: Mutex<Option<(Instant, Vec<u8>)>>,
    /// Each address's wrong keys in its window, and when the first came.
    failures: Mutex<HashMap<IpAddr, (u32, Instant)>>,
    /// The last action's outcome, shown on the next page.
    notice: Mutex<Option<(Instant, String)>>,
    /// What the page's references are drawn from, new at each start, so that a reference tells
    /// nothing of its address.
    salt: [u8; 32],
    /// The addresses the page listed, under the references its boxes send in their place, and
    /// when each was last listed.
    shown: Mutex<HashMap<String, (IpAddr, Instant)>>,
    log: Log,
}

/// Opens the panel at `address` and serves it on threads of its own; the server's loop answers
/// its requests from `requests`. Without a key only the statistics are served. The address it
/// listens at.
///
/// # Errors
/// The system's random source failed, or the port could not be opened.
pub fn start(address: SocketAddr, key: Option<PanelKey>, requests: Sender<Request>, log: Log) -> io::Result<SocketAddr> {
    let mut salt = [0; 32];
    getrandom::fill(&mut salt).map_err(|error| io::Error::other(format!("the system's random source failed: {error}")))?;
    let listener = TcpListener::bind(address)?;
    let listening = listener.local_addr()?;
    let panel = Arc::new(Panel {
        key,
        requests,
        stats: Mutex::default(),
        failures: Mutex::default(),
        notice: Mutex::default(),
        salt,
        shown: Mutex::default(),
        log,
    });
    std::thread::Builder::new().name("moderation panel".into()).spawn(move || serve(&listener, &panel))?;
    Ok(listening)
}

/// A connection being served, counted until it ends.
struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn serve(listener: &TcpListener, panel: &Arc<Panel>) {
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            // Out of descriptors or the like: wait a moment rather than spin.
            std::thread::sleep(Duration::from_millis(50));
            continue;
        };
        if active.fetch_add(1, Ordering::AcqRel) >= MAX_CONNECTIONS {
            active.fetch_sub(1, Ordering::AcqRel);
            continue;
        }
        let slot = Slot(Arc::clone(&active));
        let panel = Arc::clone(panel);
        // A thread that cannot start drops the connection and its slot with it.
        let _ = std::thread::Builder::new().name("moderation request".into()).spawn(move || {
            let _slot = slot;
            connection(&panel, stream);
        });
    }
}

fn connection(panel: &Panel, mut stream: TcpStream) {
    let Ok(peer) = stream.peer_addr() else { return };
    let started = Instant::now();
    if stream.set_read_timeout(Some(IO_TIMEOUT)).is_err() || stream.set_write_timeout(Some(IO_TIMEOUT)).is_err() {
        return;
    }
    let response = match read_request(&mut stream, started + REQUEST_TIME) {
        Ok(request) => panel.respond(moderation::address(peer.ip()), &request, Instant::now()),
        Err(Some(status)) => Response::status(status),
        Err(None) => return,
    };
    let _ = stream.write_all(&response.bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
}

#[derive(Debug, PartialEq, Eq)]
struct HttpRequest {
    method: String,
    path: String,
    query: String,
    body: Vec<u8>,
}

/// A request's line, headers and body: `Err(Some(status))` for one to refuse, `Err(None)` for a
/// connection that closed or stalled before a whole request came.
fn read_request(stream: &mut impl Read, deadline: Instant) -> Result<HttpRequest, Option<u16>> {
    let mut data = Vec::new();
    let mut chunk = [0; 4096];
    let mut read = |data: &mut Vec<u8>| -> Result<(), Option<u16>> {
        if Instant::now() >= deadline {
            return Err(None);
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => Err(None),
            Ok(count) => {
                data.extend_from_slice(&chunk[..count]);
                Ok(())
            }
        }
    };
    let head_end = loop {
        if let Some(end) = data.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        if data.len() > MAX_HEAD {
            return Err(Some(431));
        }
        read(&mut data)?;
    };
    if head_end > MAX_HEAD {
        return Err(Some(431));
    }
    let head = std::str::from_utf8(&data[..head_end]).map_err(|_| Some(400))?;
    let mut lines = head.split("\r\n");
    let mut parts = lines.next().unwrap_or_default().split(' ');
    let (Some(method), Some(target), Some(version), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(Some(400));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(Some(400));
    }
    let mut length = 0;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(Some(400))?;
        if name.eq_ignore_ascii_case("content-length") {
            length = value.trim().parse::<usize>().map_err(|_| Some(400))?;
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            // Only a body of a stated length is taken.
            return Err(Some(400));
        }
    }
    if length > MAX_BODY {
        return Err(Some(413));
    }
    let (method, target) = (method.to_owned(), target.to_owned());
    let mut body = data.split_off(head_end + 4);
    while body.len() < length {
        read(&mut body)?;
    }
    body.truncate(length);
    let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
    Ok(HttpRequest { method, path: path.to_owned(), query: query.to_owned(), body })
}

impl Panel {
    fn respond(&self, peer: IpAddr, request: &HttpRequest, now: Instant) -> Response {
        if request.path == STATS_PATH {
            return if request.method == "GET" { self.stats(now) } else { Response::status(405) };
        }
        if request.path != PATH {
            return Response::status(404);
        }
        let Some(key) = self.admit(peer, &request.query, now) else { return Response::status(404) };
        match request.method.as_str() {
            "GET" => self.page(&key, now),
            "POST" => self.act(&key, &request.body, now),
            _ => Response::status(405),
        }
    }

    /// The public statistics, read from the server's loop at most once in [`STATS_TIME`].
    fn stats(&self, now: Instant) -> Response {
        let mut kept = self.stats.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((at, body)) = kept.as_ref()
            && now.saturating_duration_since(*at) < STATS_TIME
        {
            return Response::json(body.clone());
        }
        let (reply, answer) = mpsc::channel();
        if self.requests.send(Request::Stats(reply)).is_err() {
            return Response::status(503);
        }
        let Ok(stats) = answer.recv_timeout(ANSWER_TIME) else { return Response::status(503) };
        let body = stats::json(&stats).into_bytes();
        *kept = Some((now, body.clone()));
        Response::json(body)
    }

    /// The key the query holds when it is the panel's. The address's wrong keys are counted, and
    /// the key of one that sent too many is not even compared. Without a key file no key is the
    /// panel's.
    fn admit(&self, peer: IpAddr, query: &str, now: Instant) -> Option<String> {
        let panel_key = self.key.as_ref()?;
        let mut failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        failures.retain(|_, (_, first)| now.saturating_duration_since(*first) < FAILURE_WINDOW);
        if failures.get(&peer).is_some_and(|(count, _)| *count >= MAX_FAILURES) {
            return None;
        }
        let key = fields(query).into_iter().find(|(name, _)| name == "key").map(|(_, value)| value);
        if let Some(key) = key.filter(|key| panel_key.matches(key)) {
            failures.remove(&peer);
            return Some(key);
        }
        let (count, _) = failures.entry(peer).or_insert((0, now));
        *count += 1;
        if *count == MAX_FAILURES {
            (self.log)(&format!(
                "moderation panel: {peer} sent {MAX_FAILURES} wrong keys and is refused for {} minutes",
                FAILURE_WINDOW.as_secs() / 60
            ));
        }
        if failures.len() > MAX_COUNTED {
            let oldest = failures.iter().min_by_key(|(_, (_, first))| *first).map(|(&address, _)| address);
            if let Some(oldest) = oldest {
                failures.remove(&oldest);
            }
        }
        None
    }

    fn page(&self, key: &str, now: Instant) -> Response {
        let (reply, answer) = mpsc::channel();
        if self.requests.send(Request::View(reply)).is_err() {
            return Response::status(503);
        }
        let Ok(view) = answer.recv_timeout(ANSWER_TIME) else { return Response::status(503) };
        let notice = self.notice.lock().unwrap_or_else(PoisonError::into_inner).take();
        let notice = notice.filter(|(at, _)| now.saturating_duration_since(*at) < NOTICE_TIME).map(|(_, text)| text);
        self.list(&view, now);
        Response::html(render(&view, key, notice.as_deref(), &|address: IpAddr| self.reference(address)))
    }

    /// The page's reference to `address`: the same at every listing, and no way back to the
    /// address without the panel's salt.
    fn reference(&self, address: IpAddr) -> String {
        let mut hash = Sha256::new();
        hash.update(self.salt);
        hash.update(address.to_string());
        hash.finalize()[..12].iter().fold(String::with_capacity(24), |mut reference, byte| {
            let _ = write!(reference, "{byte:02x}");
            reference
        })
    }

    /// Keeps the references to the addresses `view` lists, for the forms its page posts.
    fn list(&self, view: &View, now: Instant) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        shown.retain(|_, (_, listed)| now.saturating_duration_since(*listed) < SHOWN_TIME);
        let bans = view.chat_shadowbans.iter().chain(&view.ip_bans).map(|(address, _)| *address);
        for address in view.players.iter().map(|player| player.address).chain(bans) {
            shown.insert(self.reference(address), (address, now));
        }
    }

    /// Does what the form asks and sends the browser back to the page, where its outcome shows
    /// once: reloading that page does nothing again.
    fn act(&self, key: &str, form: &[u8], now: Instant) -> Response {
        let asked = {
            let shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
            action(&String::from_utf8_lossy(form), |reference| shown.get(reference).map(|&(address, _)| address))
        };
        let outcome = match asked {
            Err(outcome) => outcome.to_owned(),
            Ok(action) => {
                let (reply, answer) = mpsc::channel();
                if self.requests.send(Request::Act(action, reply)).is_err() {
                    return Response::status(503);
                }
                answer.recv_timeout(ANSWER_TIME).unwrap_or_else(|_| "The server did not answer in time: refresh to see what it did.".into())
            }
        };
        *self.notice.lock().unwrap_or_else(PoisonError::into_inner) = Some((now, outcome));
        Response::see_other(&url(key))
    }
}

/// The action a posted form asks: its `action`, and the players (`reference|name`) or bans
/// (`chat|reference`, `ip|reference`) it ticked, `address` giving each reference's address; the
/// outcome for the page instead when it ticked none the panel still knows.
fn action(form: &str, address: impl Fn(&str) -> Option<IpAddr>) -> Result<Action, &'static str> {
    let fields = fields(form);
    let values = |name: &'static str| fields.iter().filter(move |(field, _)| field == name).map(|(_, value)| value.as_str()).take(MAX_TARGETS);
    let players = || -> Vec<(IpAddr, String)> {
        values("player")
            .filter_map(|player| {
                let (reference, name) = player.split_once('|')?;
                let name = name.chars().filter(|character| !character.is_control()).take(crate::protocol::NICKNAME_CHARACTERS).collect();
                Some((address(reference)?, name))
            })
            .collect()
    };
    let action = match values("action").next() {
        Some("chat-shadowban") => Action::ChatShadowban(players()),
        Some("ip-ban") => Action::IpBan(players()),
        Some("lift") => {
            let (mut chat, mut ip) = (Vec::new(), Vec::new());
            for ban in values("ban") {
                match ban.split_once('|') {
                    Some(("chat", reference)) => chat.extend(address(reference)),
                    Some(("ip", reference)) => ip.extend(address(reference)),
                    _ => {}
                }
            }
            Action::Lift { chat, ip }
        }
        _ => return Err("Nothing was ticked."),
    };
    if !action.is_empty() {
        return Ok(action);
    }
    let ticked = values("player").chain(values("ban")).next().is_some();
    Err(if ticked { "The page was out of date: refresh it and tick again." } else { "Nothing was ticked." })
}

/// A query's or form's `name=value` pairs, `+` and `%XX` decoded.
fn fields(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(name), decode(value))
        })
        .collect()
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        index += 1;
        let escaped = (byte == b'%')
            .then(|| bytes.get(index..index + 2))
            .flatten()
            .filter(|digits| digits.iter().all(u8::is_ascii_hexdigit))
            .and_then(|digits| u8::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok());
        match (byte, escaped) {
            (_, Some(escaped)) => {
                decoded.push(escaped);
                index += 2;
            }
            (b'+', None) => decoded.push(b' '),
            (byte, None) => decoded.push(byte),
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The page's address with the key.
fn url(key: &str) -> String {
    format!("{PATH}?key={key}")
}

/// An answer: its status, the page it sends the browser on to, its body, and whether any page
/// may read it (the public statistics).
#[derive(Debug)]
struct Response {
    status: u16,
    content_type: &'static str,
    location: Option<String>,
    body: Vec<u8>,
    public: bool,
}

impl Response {
    fn status(status: u16) -> Self {
        Self { status, content_type: "text/plain; charset=utf-8", location: None, body: reason(status).as_bytes().to_vec(), public: false }
    }

    fn html(page: String) -> Self {
        Self { status: 200, content_type: "text/html; charset=utf-8", location: None, body: page.into_bytes(), public: false }
    }

    fn json(body: Vec<u8>) -> Self {
        Self { status: 200, content_type: "application/json; charset=utf-8", location: None, body, public: true }
    }

    fn see_other(location: &str) -> Self {
        Self { status: 303, content_type: "text/plain; charset=utf-8", location: Some(location.to_owned()), body: Vec::new(), public: false }
    }

    /// The answer as sent: never cached, framed or followed by a referrer, and running nothing
    /// but its own styles; the public statistics readable from any page's script.
    fn bytes(&self) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\n", self.status, reason(self.status), self.content_type);
        if let Some(location) = &self.location {
            let _ = write!(head, "Location: {location}\r\n");
        }
        if self.public {
            head.push_str("Access-Control-Allow-Origin: *\r\n");
        }
        head.push_str(
            "Cache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\n\
             Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'\r\n",
        );
        let _ = write!(head, "Content-Length: {}\r\nConnection: close\r\n\r\n", self.body.len());
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        303 => "See Other",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

/// `text` for an HTML page, in its text or a quoted attribute.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn count(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn duration(time: Duration) -> String {
    let seconds = time.as_secs();
    match seconds {
        0..60 => format!("{seconds} s"),
        60..3600 => format!("{} min", seconds / 60),
        _ => format!("{} h {:02} min", seconds / 3600, seconds / 60 % 60),
    }
}

const STYLE: &str = "\
:root{color-scheme:light dark;--bg:#f4f5f7;--card:#fff;--text:#1b1f24;--muted:#5f6b77;--line:#dfe3e8;--accent:#1f6feb;--danger:#cf222e;--tag:#eef1f4}
@media (prefers-color-scheme:dark){:root{--bg:#111418;--card:#1a1f25;--text:#e4e8ec;--muted:#93a0ad;--line:#2b323a;--accent:#58a6ff;--danger:#ff6b6b;--tag:#242b33}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:14px/1.5 system-ui,-apple-system,\"Segoe UI\",Roboto,sans-serif}
main{max-width:1120px;margin:0 auto;padding:24px 16px 48px}
h1{font-size:22px;margin:0}
h2{font-size:15px;margin:28px 0 10px}
a{color:var(--accent)}
.meta{margin:4px 0 0;color:var(--muted)}
.notice{margin:16px 0 0;padding:10px 14px;background:var(--card);border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:6px}
.layout{display:flex;gap:16px;align-items:flex-start}
.table{flex:1;min-width:0;overflow-x:auto;background:var(--card);border:1px solid var(--line);border-radius:8px}
table{width:100%;border-collapse:collapse}
th,td{padding:9px 12px;text-align:left;border-bottom:1px solid var(--line);white-space:nowrap}
th{font-size:12px;font-weight:600;color:var(--muted)}
tbody tr:last-child td{border-bottom:0}
td.pick{width:1%}
td.empty{padding:22px;text-align:center;color:var(--muted)}
.tag{padding:2px 8px;border-radius:999px;background:var(--tag);color:var(--muted);font-size:12px}
label{cursor:pointer}
.actions{flex:none;width:176px;position:sticky;top:16px;display:flex;flex-direction:column;gap:8px}
button{font:inherit;font-weight:600;padding:9px 12px;border-radius:6px;border:1px solid var(--line);background:var(--card);color:var(--text);cursor:pointer;text-align:left}
button:hover{border-color:var(--accent)}
button.danger{color:var(--danger);border-color:var(--danger)}
@media (max-width:720px){.layout{flex-direction:column}.actions{position:static;width:100%;flex-direction:row;flex-wrap:wrap}}
";

/// The page: `reference` gives what a box sends for an address, which the page never shows.
fn render(view: &View, key: &str, notice: Option<&str>, reference: &dyn Fn(IpAddr) -> String) -> String {
    let url = escape(&url(key));
    let mut page = String::with_capacity(16 * 1024);
    let _ = write!(
        page,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"referrer\" content=\"no-referrer\">\n<title>Moderation Panel</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<main>\n\
         <header><h1>Moderation Panel</h1><p class=\"meta\">{} &middot; {} &middot; <a href=\"{url}\">Refresh</a></p></header>\n",
        count(view.players.len(), "player online", "players online"),
        count(view.games, "game", "games"),
    );
    if let Some(notice) = notice {
        let _ = writeln!(page, "<p class=\"notice\">{}</p>", escape(notice));
    }
    let _ = write!(
        page,
        "<section>\n<h2>Players</h2>\n<form method=\"post\" action=\"{url}\" class=\"layout\">\n<div class=\"table\"><table>\n\
         <thead><tr><th></th><th>Name</th><th>Game</th><th>Online for</th><th>Version</th><th>Chat</th></tr></thead>\n<tbody>\n"
    );
    if view.players.is_empty() {
        page.push_str("<tr><td class=\"empty\" colspan=\"6\">No one is signed in.</td></tr>\n");
    }
    for (index, player) in view.players.iter().enumerate() {
        let game = match player.game {
            Some((game, true)) => format!("{game} (host)"),
            Some((game, false)) => game.to_string(),
            None => "-".to_owned(),
        };
        let chat = if player.chat_shadowbanned { "<span class=\"tag\">shadowbanned</span>" } else { "" };
        let _ = writeln!(
            page,
            "<tr><td class=\"pick\"><input type=\"checkbox\" id=\"p{index}\" name=\"player\" value=\"{}\"></td><td><label for=\"p{index}\">{}</label></td>\
             <td>{game}</td><td>{}</td><td>{}</td><td>{chat}</td></tr>",
            escape(&format!("{}|{}", reference(player.address), player.name)),
            escape(&player.name),
            duration(player.online),
            escape(&player.version),
        );
    }
    page.push_str(
        "</tbody>\n</table></div>\n<div class=\"actions\"><button type=\"submit\" name=\"action\" value=\"chat-shadowban\">Chat Shadowban</button>\
         <button type=\"submit\" name=\"action\" value=\"ip-ban\" class=\"danger\">IP Ban</button></div>\n</form>\n</section>\n",
    );
    let bans: Vec<(&str, &str, IpAddr, &str)> = view
        .chat_shadowbans
        .iter()
        .map(|(address, name)| ("chat", "Chat shadowban", *address, name.as_str()))
        .chain(view.ip_bans.iter().map(|(address, name)| ("ip", "IP ban", *address, name.as_str())))
        .collect();
    let _ = write!(
        page,
        "<section>\n<h2>Bans</h2>\n<form method=\"post\" action=\"{url}\" class=\"layout\">\n<div class=\"table\"><table>\n\
         <thead><tr><th></th><th>Ban</th><th>Name</th></tr></thead>\n<tbody>\n"
    );
    if bans.is_empty() {
        page.push_str("<tr><td class=\"empty\" colspan=\"3\">No one is banned.</td></tr>\n");
    }
    for (index, (kind, label, address, name)) in bans.iter().enumerate() {
        let _ = writeln!(
            page,
            "<tr><td class=\"pick\"><input type=\"checkbox\" id=\"b{index}\" name=\"ban\" value=\"{kind}|{}\"></td>\
             <td><label for=\"b{index}\">{label}</label></td><td>{}</td></tr>",
            reference(*address),
            escape(name),
        );
    }
    page.push_str(
        "</tbody>\n</table></div>\n<div class=\"actions\"><button type=\"submit\" name=\"action\" value=\"lift\">Lift Ban</button></div>\n</form>\n</section>\n\
         </main>\n</body>\n</html>\n",
    );
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "test-key-0123456789";

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    fn request(method: &str, target: &str, body: &str) -> HttpRequest {
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        HttpRequest { method: method.into(), path: path.into(), query: query.into(), body: body.as_bytes().to_vec() }
    }

    fn get(target: &str) -> HttpRequest {
        request("GET", target, "")
    }

    fn post(target: &str, form: &str) -> HttpRequest {
        request("POST", target, form)
    }

    /// Runs `test` against a panel whose server's loop shows one player and one ban and answers
    /// every action alike: the actions it was asked.
    fn served(test: impl FnOnce(&Panel)) -> Vec<Action> {
        served_with(Some(PanelKey::from_text(KEY).unwrap()), test)
    }

    fn served_with(key: Option<PanelKey>, test: impl FnOnce(&Panel)) -> Vec<Action> {
        let (requests, receiver) = mpsc::channel();
        let panel = Panel {
            key,
            requests,
            stats: Mutex::default(),
            failures: Mutex::default(),
            notice: Mutex::default(),
            salt: [7; 32],
            shown: Mutex::default(),
            log: Arc::new(|_: &str| {}),
        };
        let view = View {
            players: vec![Player {
                name: "O'Neil".into(),
                address: ip("10.0.0.2"),
                version: "<script>alert(1)</script>".into(),
                game: Some((3, true)),
                online: Duration::from_secs(75),
                chat_shadowbanned: true,
            }],
            games: 1,
            chat_shadowbans: vec![(ip("198.51.100.2"), "Spammer".into())],
            ip_bans: Vec::new(),
        };
        let server = std::thread::spawn(move || {
            let mut actions = Vec::new();
            while let Ok(request) = receiver.recv() {
                match request {
                    Request::View(reply) => {
                        let _ = reply.send(view.clone());
                    }
                    Request::Act(action, reply) => {
                        actions.push(action);
                        let _ = reply.send("Done & dusted.".into());
                    }
                    Request::Stats(reply) => {
                        let game = crate::stats::PublicGame {
                            challenge_type: crate::protocol::challenge::SKATE,
                            challenge_key: 7,
                            activity: None,
                            in_progress: true,
                            players: vec!["O'Neil".into()],
                        };
                        let _ = reply.send(Stats { players_online: 2, games: vec![game] });
                    }
                }
            }
            actions
        });
        test(&panel);
        drop(panel);
        server.join().unwrap()
    }

    #[test]
    fn the_statistics_need_no_key_and_name_no_address() {
        for key in [Some(PanelKey::from_text(KEY).unwrap()), None] {
            served_with(key, |panel| {
                let now = Instant::now();
                let response = panel.respond(ip("203.0.113.5"), &get(STATS_PATH), now);
                assert_eq!((response.status, response.content_type), (200, "application/json; charset=utf-8"));
                let body = String::from_utf8(response.body.clone()).unwrap();
                assert!(body.contains("\"players_online\": 2") && body.contains("\"challenge_type\": \"S.K.A.T.E.\"") && body.contains("\"players\": [\"O'Neil\"]"));
                assert!(!body.contains("10.0.0.2"));
                assert!(String::from_utf8(response.bytes()).unwrap().contains("Access-Control-Allow-Origin: *"));
                assert_eq!(panel.respond(ip("203.0.113.5"), &post(STATS_PATH, ""), now).status, 405);
                for _ in 0..MAX_FAILURES + 2 {
                    assert_eq!(panel.respond(ip("203.0.113.5"), &get(STATS_PATH), now).status, 200, "counted as no wrong key");
                }
            });
        }
        served_with(None, |panel| {
            assert_eq!(panel.respond(ip("192.0.2.1"), &get(&format!("{PATH}?key={KEY}")), Instant::now()).status, 404, "no key file, no panel");
        });
    }

    #[test]
    fn a_request_without_the_key_reads_as_a_page_that_does_not_exist() {
        let actions = served(|panel| {
            let (peer, now) = (ip("198.51.100.9"), Instant::now());
            for target in [
                "/moderationpanel".to_owned(),
                "/moderationpanel?key=".to_owned(),
                "/moderationpanel?key=wrong".to_owned(),
                format!("/moderationpanel?key={}", &KEY[..KEY.len() - 1]),
                format!("/moderationpanel?KEY={KEY}"),
                format!("/?key={KEY}"),
                format!("/moderationpanel/?key={KEY}"),
            ] {
                let response = panel.respond(peer, &get(&target), now);
                assert_eq!((response.status, response.body.as_slice()), (404, &b"Not Found"[..]), "{target}");
            }
            let posted = panel.respond(peer, &post("/moderationpanel?key=nope", "action=ip-ban&player=10.0.0.2%7CGuest"), now);
            assert_eq!(posted.status, 404);
        });
        assert!(actions.is_empty(), "nothing was done");
    }

    #[test]
    fn ten_wrong_keys_shut_an_address_out_for_the_window() {
        served(|panel| {
            let (attacker, owner, now) = (ip("203.0.113.5"), ip("192.0.2.1"), Instant::now());
            let page = format!("{PATH}?key={KEY}");
            for guess in 0..MAX_FAILURES {
                assert_eq!(panel.respond(attacker, &get(&format!("{PATH}?key=guess-{guess:016}")), now).status, 404);
            }
            assert_eq!(panel.respond(attacker, &get(&page), now).status, 404, "not even the key opens it now");
            assert_eq!(panel.respond(owner, &get(&page), now).status, 200, "another address is not shut out");
            assert_eq!(panel.respond(attacker, &get(&page), now + FAILURE_WINDOW).status, 200, "the window ends");
        });
    }

    #[test]
    fn the_page_lists_players_and_bans_without_their_addresses_and_escapes_what_players_sent() {
        served(|panel| {
            let response = panel.respond(ip("192.0.2.1"), &get(&format!("{PATH}?key={KEY}")), Instant::now());
            assert_eq!(response.status, 200);
            let page = String::from_utf8(response.body.clone()).unwrap();
            assert!(page.contains(&format!("value=\"{}|O&#39;Neil\"", panel.reference(ip("10.0.0.2")))), "{page}");
            assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt;") && !page.contains("<script"));
            assert!(page.contains("3 (host)") && page.contains("1 min") && page.contains(">shadowbanned<"));
            assert!(page.contains(&format!("value=\"chat|{}\"", panel.reference(ip("198.51.100.2")))) && page.contains("Spammer"));
            assert!(!page.contains("10.0.0.2") && !page.contains("198.51.100.2"), "no address shows, not even in the source");
            assert!(page.contains(&format!("action=\"{PATH}?key={KEY}\"")));
            let sent = String::from_utf8(response.bytes()).unwrap();
            for header in ["Cache-Control: no-store", "Referrer-Policy: no-referrer", "Content-Security-Policy: default-src 'none'", "X-Frame-Options: DENY"] {
                assert!(sent.contains(header), "{header}");
            }
        });
    }

    #[test]
    fn a_posted_form_does_what_it_ticked_and_goes_back_to_the_page() {
        let actions = served(|panel| {
            let (owner, now) = (ip("192.0.2.1"), Instant::now());
            let page = format!("{PATH}?key={KEY}");
            let (player, ban) = (panel.reference(ip("10.0.0.2")), panel.reference(ip("198.51.100.2")));
            let shown_after = |form: &str| {
                assert_eq!(panel.respond(owner, &post(&page, form), now).status, 303);
                String::from_utf8(panel.respond(owner, &get(&page), now).body).unwrap()
            };
            let early = shown_after(&format!("player={player}%7CO%27Neil&action=chat-shadowban"));
            assert!(early.contains("The page was out of date"), "a reference the page has not listed yet is refused");
            assert!(!String::from_utf8(panel.respond(owner, &get(&page), now).body).unwrap().contains("out of date"), "the outcome shows once");
            let unlisted = panel.reference(ip("10.0.0.3"));
            let posted = panel.respond(owner, &post(&page, &format!("player={player}%7CO%27Neil&player={unlisted}%7CBob&player=junk&action=chat-shadowban")), now);
            assert_eq!((posted.status, posted.location.as_deref()), (303, Some(page.as_str())));
            assert!(String::from_utf8(panel.respond(owner, &get(&page), now).body).unwrap().contains("<p class=\"notice\">Done &amp; dusted.</p>"));
            shown_after(&format!("action=ip-ban&player={player}%7CO%27Neil"));
            shown_after(&format!("action=lift&ban=chat%7C{ban}&ban=ip%7C{player}&ban=bogus"));
            assert!(shown_after("action=ip-ban").contains("Nothing was ticked."));
            assert_eq!(panel.respond(owner, &request("DELETE", &page, ""), now).status, 405);
        });
        assert_eq!(
            actions,
            [
                Action::ChatShadowban(vec![(ip("10.0.0.2"), "O'Neil".into())]),
                Action::IpBan(vec![(ip("10.0.0.2"), "O'Neil".into())]),
                Action::Lift { chat: vec![ip("198.51.100.2")], ip: vec![ip("10.0.0.2")] },
            ]
        );
    }

    #[test]
    fn a_request_is_read_whole_and_one_too_large_is_refused() {
        let deadline = Instant::now() + REQUEST_TIME;
        let read = |text: &[u8]| read_request(&mut &text[..], deadline);
        assert_eq!(
            read(b"POST /moderationpanel?key=k HTTP/1.1\r\nHost: x\r\ncontent-length: 5\r\n\r\nab=cdEXTRA"),
            Ok(HttpRequest { method: "POST".into(), path: PATH.into(), query: "key=k".into(), body: b"ab=cd".to_vec() })
        );
        assert_eq!(read(b"GET / HTTP/1.1\r\n\r\n").map(|request| request.path), Ok("/".into()));
        assert_eq!(read(b"GET / HTTP/1.1\r\nHost: x\r\n"), Err(None), "closed before the headers ended");
        assert_eq!(read(b"POST / HTTP/1.1\r\nContent-Length: 10\r\n\r\nshort"), Err(None), "closed before the body ended");
        assert_eq!(read(&[&b"GET / HTTP/1.1\r\nX: "[..], &[b'a'; MAX_HEAD], b"\r\n\r\n"].concat()), Err(Some(431)));
        assert_eq!(read(b"POST / HTTP/1.1\r\nContent-Length: 99999\r\n\r\n"), Err(Some(413)));
        assert_eq!(read(b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"), Err(Some(400)));
        assert_eq!(read(b"GET /\r\n\r\n"), Err(Some(400)));
        assert_eq!(read(b"GET / HTTP/1.1\r\nno colon\r\n\r\n"), Err(Some(400)));
        assert_eq!(
            fields("key=a%2Bb+c&x&%zz=%4&&n=%C3%A9"),
            [("key".into(), "a+b c".into()), ("x".into(), String::new()), ("%zz".into(), "%4".into()), ("n".into(), "\u{e9}".into())]
        );
    }
}
