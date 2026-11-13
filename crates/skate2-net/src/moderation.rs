//! The dedicated server's moderation (PC port addition): the ban lists the server checks
//! datagrams, chat lines and matchmaking against, and the settings and key of its moderation
//! panel ([`crate::panel`]). A ban holds an IP address, as a nickname changes at will.
//!
//! The files sit together in the folder of `server-config.json`: beside the server's executable
//! or, when only it holds one, the working directory. They stay with the server: git ignores
//! them and an update never replaces them.

use crate::json::{self, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The server's settings: the moderation panel's port, `"moderation_panel_port": 8080`.
pub const CONFIG_FILE: &str = "server-config.json";
/// The moderation panel's key, on one line.
pub const KEY_FILE: &str = "moderation-key.txt";
/// The chat-shadowbanned addresses: their chat lines reach only themselves.
pub const CHAT_SHADOWBANS_FILE: &str = "chat-shadowbans.json";
/// The matchmaking-shadowbanned addresses: they play alone. Matchmaking and the lobby browser
/// offer them no game and no one a game they are in, and one in a game with others is taken out
/// of it.
pub const MATCHMAKING_SHADOWBANS_FILE: &str = "matchmaking-shadowbans.json";
/// The banned addresses: the server ignores every datagram from them.
pub const IP_BANS_FILE: &str = "ip-bans.json";
/// The fewest characters a panel key holds, and the most.
pub const MIN_KEY_CHARACTERS: usize = 16;
const MAX_KEY_CHARACTERS: usize = 256;

/// The folder of the moderation files.
#[must_use]
pub fn folder() -> PathBuf {
    let config = crate::server::server_file(CONFIG_FILE);
    match config.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// An address as kicks and bans hold it: the IPv4 address an IPv6 socket reports as
/// `::ffff:a.b.c.d` is that IPv4 address.
#[must_use]
pub fn address(ip: IpAddr) -> IpAddr {
    ip.to_canonical()
}

/// `server-config.json`'s moderation panel port; `Ok(None)` without the file or the setting.
///
/// # Errors
/// The file could not be read or does not read as a JSON object, or its port is not one from 1
/// to 65535.
pub fn panel_port(path: &Path) -> Result<Option<u16>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{} could not be read: {error}", path.display())),
    };
    let fields = json::object(&text).ok_or_else(|| format!("{} does not read as a JSON object", path.display()))?;
    let Some((_, value)) = fields.iter().find(|(key, _)| key == "moderation_panel_port") else { return Ok(None) };
    let port = match value {
        Value::Number(text) | Value::Text(text) => text.trim().parse::<u16>().ok().filter(|&port| port != 0),
        Value::Other => None,
    };
    port.map(Some).ok_or_else(|| format!("{}: moderation_panel_port is not a port from 1 to 65535", path.display()))
}

/// Whether a key may hold the character: one a URL carries as it is.
fn key_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
}

/// The moderation panel's key, kept as its SHA-256: a request's key is compared by its hash, in
/// time that does not depend on where the two differ.
#[derive(Clone)]
pub struct PanelKey([u8; 32]);

impl std::fmt::Debug for PanelKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PanelKey(..)")
    }
}

impl PanelKey {
    /// The key a key file's text holds: the text, white space around it left out.
    ///
    /// # Errors
    /// Why the text is no key: a key is 16 to 256 letters, digits, `-`, `_`, `.` or `~`.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let key = text.strip_prefix('\u{feff}').unwrap_or(text).trim();
        if !(MIN_KEY_CHARACTERS..=MAX_KEY_CHARACTERS).contains(&key.len()) {
            return Err(format!("a key holds {MIN_KEY_CHARACTERS} to {MAX_KEY_CHARACTERS} characters"));
        }
        if !key.bytes().all(key_character) {
            return Err("a key holds only letters, digits, -, _, . and ~".into());
        }
        Ok(Self(hash(key)))
    }

    /// The key in the file at `path`.
    ///
    /// # Errors
    /// The file could not be read, or holds no key ([`Self::from_text`]).
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|error| format!("{} could not be read: {error}", path.display()))?;
        Self::from_text(&text).map_err(|why| format!("{}: {why}", path.display()))
    }

    /// Whether `candidate` is the key.
    #[must_use]
    pub fn matches(&self, candidate: &str) -> bool {
        hash(candidate).iter().zip(self.0).fold(0, |difference, (a, b)| difference | (a ^ b)) == 0
    }
}

fn hash(text: &str) -> [u8; 32] {
    let mut hash = [0; 32];
    hash.copy_from_slice(&Sha256::digest(text.as_bytes()));
    hash
}

/// A file's modification time and length, to tell when it changed.
fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    std::fs::metadata(path).ok().map(|metadata| (metadata.modified().unwrap_or(UNIX_EPOCH), metadata.len()))
}

/// A list of banned addresses with the name each was banned under: a JSON object of
/// `"address": "name"` pairs. The file is read again when it changes, so a ban added or lifted by
/// hand applies without a restart; a file that no longer reads as such a list leaves the bans as
/// they were, and a file that is gone lifts them all.
#[derive(Debug, Default)]
pub struct BanList {
    path: Option<PathBuf>,
    /// The file's modification time and length at the last read or write.
    stamp: Option<(SystemTime, u64)>,
    bans: BTreeMap<IpAddr, String>,
}

impl BanList {
    /// The list in the file at `path`, read at its first [`Self::refresh`].
    #[must_use]
    pub fn from_file(path: PathBuf) -> Self {
        Self { path: Some(path), stamp: None, bans: BTreeMap::new() }
    }

    /// A list of these bans, with no file.
    #[must_use]
    pub fn from_bans<'a>(bans: impl IntoIterator<Item = (IpAddr, &'a str)>) -> Self {
        Self { path: None, stamp: None, bans: bans.into_iter().map(|(ip, name)| (address(ip), name.to_owned())).collect() }
    }

    /// The file the list is kept in.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bans.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bans.is_empty()
    }

    /// Whether `ip` is banned.
    #[must_use]
    pub fn contains(&self, ip: IpAddr) -> bool {
        !self.bans.is_empty() && self.bans.contains_key(&address(ip))
    }

    /// The bans, by address.
    pub fn iter(&self) -> impl Iterator<Item = (IpAddr, &str)> {
        self.bans.iter().map(|(&ip, name)| (ip, name.as_str()))
    }

    /// Reads the file again when it changed or went since the last read or write: what changed,
    /// for the server's console.
    pub fn refresh(&mut self) -> Option<String> {
        let path = self.path.as_ref()?;
        let stamp = stamp(path);
        if stamp == self.stamp {
            return None;
        }
        self.stamp = stamp;
        if stamp.is_none() {
            if self.bans.is_empty() {
                return None;
            }
            self.bans.clear();
            return Some(format!("{} is gone: its bans are lifted", path.display()));
        }
        let Some((bans, unread)) = std::fs::read_to_string(path).ok().and_then(|text| parse(&text)) else {
            return Some(format!(
                "{} does not read as a JSON object of \"address\": \"name\" pairs: the {} bans in force stay",
                path.display(),
                self.bans.len()
            ));
        };
        self.bans = bans;
        let unread = if unread == 0 { String::new() } else { format!(" ({unread} of its entries name no IP address and are passed over)") };
        Some(format!("{}: {} bans{unread}", path.display(), self.bans.len()))
    }

    /// Bans `ip` under `name` and writes the file: `None` when it was banned already, else
    /// whether the file was written (when it was not, the ban holds until the server stops).
    pub fn insert(&mut self, ip: IpAddr, name: &str) -> Option<io::Result<()>> {
        let ip = address(ip);
        if self.bans.contains_key(&ip) {
            return None;
        }
        self.bans.insert(ip, name.to_owned());
        Some(self.save())
    }

    /// Lifts the ban on `ip` and writes the file: `None` when it was not banned, else the name it
    /// was banned under and whether the file was written (when it was not, the ban stays lifted
    /// until the server stops).
    pub fn remove(&mut self, ip: IpAddr) -> Option<(String, io::Result<()>)> {
        let name = self.bans.remove(&address(ip))?;
        Some((name, self.save()))
    }

    /// Writes the list beside its file, then puts it in the file's place, so the file is never
    /// left half written.
    fn save(&mut self) -> io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let mut written = path.clone().into_os_string();
        written.push(".new");
        let written = PathBuf::from(written);
        std::fs::write(&written, self.text())?;
        std::fs::rename(&written, path)?;
        self.stamp = stamp(path);
        Ok(())
    }

    fn text(&self) -> String {
        if self.bans.is_empty() {
            return "{}\n".into();
        }
        let mut text = String::from("{\n");
        for (index, (ip, name)) in self.bans.iter().enumerate() {
            let separator = if index + 1 < self.bans.len() { "," } else { "" };
            let _ = writeln!(text, "  {}: {}{separator}", json::quote(&ip.to_string()), json::quote(name));
        }
        text.push_str("}\n");
        text
    }
}

/// A list file's bans, and how many of its entries name no IP address; `None` for text that is
/// no JSON object.
fn parse(text: &str) -> Option<(BTreeMap<IpAddr, String>, usize)> {
    let mut bans = BTreeMap::new();
    let mut unread = 0;
    for (key, value) in json::object(text)? {
        let Ok(ip) = key.trim().parse::<IpAddr>() else {
            unread += 1;
            continue;
        };
        let name = match value {
            Value::Text(name) => name,
            Value::Number(_) | Value::Other => String::new(),
        };
        bans.insert(address(ip), name);
    }
    Some((bans, unread))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn a_ban_list_reads_back_what_it_writes() {
        let path = std::env::temp_dir().join(format!("skate2-net-bans-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut list = BanList::from_file(path.clone());
        assert_eq!(list.refresh(), None, "no file: no bans");
        assert!(list.insert(ip("203.0.113.7"), "Troll \"1\"").unwrap().is_ok());
        assert!(list.insert(ip("::ffff:203.0.113.7"), "Again").is_none(), "the same IPv4 address");
        assert!(list.insert(ip("2001:db8::1"), "Six").unwrap().is_ok());
        let mut read = BanList::from_file(path.clone());
        assert!(read.refresh().is_some());
        assert_eq!(read.iter().collect::<Vec<_>>(), [(ip("203.0.113.7"), "Troll \"1\""), (ip("2001:db8::1"), "Six")]);
        assert!(read.contains(ip("::ffff:203.0.113.7")));
        let (name, written) = list.remove(ip("203.0.113.7")).unwrap();
        assert_eq!((name.as_str(), written.is_ok()), ("Troll \"1\"", true));
        assert!(list.remove(ip("203.0.113.7")).is_none());
        assert!(read.refresh().is_some());
        assert!(!read.contains(ip("203.0.113.7")), "a lift reaches the other reader");
        std::fs::remove_file(&path).unwrap();
        assert!(read.refresh().is_some());
        assert!(read.is_empty(), "a file that is gone lifts every ban");
    }

    #[test]
    fn a_hand_edit_applies_and_a_broken_one_changes_nothing() {
        let path = std::env::temp_dir().join(format!("skate2-net-bans-edit-{}.json", std::process::id()));
        std::fs::write(&path, "\u{feff}{\n  \"198.51.100.2\": \"Spammer\",\n  \"not an address\": \"x\",\n  \"198.51.100.3\": null\n}\n").unwrap();
        let mut list = BanList::from_file(path.clone());
        let read = list.refresh().unwrap();
        assert!(read.contains("2 bans") && read.contains("1 of its entries"), "{read}");
        assert!(list.contains(ip("198.51.100.2")) && list.contains(ip("198.51.100.3")));
        std::fs::write(&path, "{\"198.51.100.2\": \"Spammer\",").unwrap();
        assert!(list.refresh().unwrap().contains("the 2 bans in force stay"));
        assert_eq!(list.len(), 2);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_key_is_long_and_carried_by_a_url_as_it_is() {
        let key = PanelKey::from_text("\u{feff}  mychosenkeysomewherefromtxtfile\r\n").unwrap();
        assert!(key.matches("mychosenkeysomewherefromtxtfile"));
        assert!(!key.matches("mychosenkeysomewherefromtxtfil"));
        assert!(!key.matches(""));
        assert!(PanelKey::from_text("short-key").is_err());
        assert!(PanelKey::from_text("sixteen chars ok").is_err(), "no spaces");
        assert!(PanelKey::from_text("key&admin=1&x=yz0").is_err());
        assert!(PanelKey::from_text(&"k".repeat(257)).is_err());
        assert!(PanelKey::from_text("A-z_0.9~A-z_0.9~").is_ok());
        assert_eq!(format!("{key:?}"), "PanelKey(..)", "the key is never printed");
    }

    #[test]
    fn the_panel_port_comes_from_the_server_config() {
        let path = std::env::temp_dir().join(format!("skate2-net-server-config-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(panel_port(&path), Ok(None));
        for (text, port) in [("{\"moderation_panel_port\": 8080}", Ok(Some(8080))), ("{\"moderation_panel_port\": \"8081\"}", Ok(Some(8081))), ("{}", Ok(None))] {
            std::fs::write(&path, text).unwrap();
            assert_eq!(panel_port(&path), port, "{text}");
        }
        for text in ["{\"moderation_panel_port\": 0}", "{\"moderation_panel_port\": 70000}", "{\"moderation_panel_port\": 80.5}", "port = 8080"] {
            std::fs::write(&path, text).unwrap();
            assert!(panel_port(&path).is_err(), "{text}");
        }
        std::fs::remove_file(&path).unwrap();
    }
}
