//! PC port addition: `online-config.json` in the game's root directory (next
//! to `saves`), holding the nickname the player uses in online lobbies, the
//! dedicated server the game connects to (`skate2-pc --server` on that host), and
//! whether lobby chat shows and whether a member waiting in a challenge's pre-game lobby
//! sees its players as ghosts (Settings' Online Settings).
//!
//! The server the game connects to by default is part of the build: compiled with
//! `SKATE2_DEFAULT_SERVER=<host:port>` (a release's public server), else this machine's. The
//! file's `server` overrides it; the game writes the file's own value back, empty when it names
//! none, so a later release with another default reaches every player who did not pick one.
use std::{fs, io, path::PathBuf};

/// Longest nickname the prompt accepts; the server drops a connection that signs in with a
/// longer one.
pub const NICKNAME_LIMIT: usize = skate2_net::protocol::NICKNAME_CHARACTERS;
/// The server a config without one connects to: `SKATE2_DEFAULT_SERVER` as the game was
/// compiled, else this machine's.
pub const DEFAULT_SERVER: &str = match option_env!("SKATE2_DEFAULT_SERVER") {
    Some(server) if !server.is_empty() => server,
    _ => "127.0.0.1:32082",
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnlineConfig {
    pub nickname: String,
    /// The file's dedicated server, `host:port` (or `host`, at the server's default port); empty
    /// for [`DEFAULT_SERVER`]. Not asked for in game.
    pub server: String,
    /// Show Lobby Chat: off, T opens no chat prompt and members' chat lines stay out of the
    /// event log. On unless the file says otherwise.
    pub show_lobby_chat: bool,
    /// Show Ghost Players in pregame lobby: off, a member waiting in a challenge's pre-game
    /// lobby sees nothing of the challenge's players. On unless the file says otherwise.
    pub show_ghost_players: bool,
}

impl Default for OnlineConfig {
    fn default() -> Self {
        Self { nickname: String::new(), server: String::new(), show_lobby_chat: true, show_ghost_players: true }
    }
}

impl OnlineConfig {
    /// The server the game signs in to.
    #[must_use]
    pub fn server(&self) -> &str {
        match self.server.trim() {
            "" => DEFAULT_SERVER,
            server => server,
        }
    }

    /// `SKATE2_SAVE_ROOT` relocates it with the saves; otherwise it sits in the
    /// working directory the game runs from.
    pub fn path() -> io::Result<PathBuf> {
        Ok(match std::env::var_os("SKATE2_SAVE_ROOT") {
            Some(root) => PathBuf::from(root),
            None => std::env::current_dir()?,
        }
        .join("online-config.json"))
    }

    /// A missing or unreadable file is the first-use state: no nickname.
    pub fn load() -> Self {
        Self::path()
            .and_then(fs::read_to_string)
            .ok()
            .and_then(|text| parse(&text))
            .unwrap_or_default()
    }

    /// Whether the nickname is one the game signs in with: the server's rule, so a nickname
    /// written into the file by hand that the prompt could not make asks for another.
    pub fn has_nickname(&self) -> bool {
        skate2_net::protocol::valid_nickname(self.nickname.trim())
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.text())
    }

    fn text(&self) -> String {
        format!(
            "{{\n  \"nickname\": {},\n  \"server\": {},\n  \"show_lobby_chat\": {},\n  \"show_ghost_players\": {}\n}}\n",
            quote(&self.nickname),
            quote(&self.server),
            self.show_lobby_chat,
            self.show_ghost_players
        )
    }
}

pub(super) fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Reads the top-level object's `nickname` and `server` strings and its `show_lobby_chat` and
/// `show_ghost_players` booleans, and skips every other value.
fn parse(text: &str) -> Option<OnlineConfig> {
    let mut reader = Reader { bytes: text.as_bytes(), at: 0 };
    let mut config = OnlineConfig::default();
    reader.expect(b'{')?;
    if reader.peek()? == b'}' {
        return Some(config);
    }
    loop {
        let key = reader.string()?;
        reader.expect(b':')?;
        if key == "nickname" && reader.peek()? == b'"' {
            config.nickname = reader.string()?;
        } else if key == "server" && reader.peek()? == b'"' {
            config.server = reader.string()?.trim().to_owned();
        } else if key == "show_lobby_chat" && matches!(reader.peek()?, b't' | b'f') {
            config.show_lobby_chat = reader.boolean()?;
        } else if key == "show_ghost_players" && matches!(reader.peek()?, b't' | b'f') {
            config.show_ghost_players = reader.boolean()?;
        } else {
            reader.skip_value()?;
        }
        match reader.next()? {
            b',' => continue,
            b'}' => return Some(config),
            _ => return None,
        }
    }
}

/// The top-level object's string values with their keys, every other value skipped; `None` for
/// text that is no JSON object.
pub(super) fn string_fields(text: &str) -> Option<Vec<(String, String)>> {
    let mut reader = Reader { bytes: text.as_bytes(), at: 0 };
    let mut fields = Vec::new();
    reader.expect(b'{')?;
    if reader.peek()? == b'}' {
        return Some(fields);
    }
    loop {
        let key = reader.string()?;
        reader.expect(b':')?;
        if reader.peek()? == b'"' {
            fields.push((key, reader.string()?));
        } else {
            reader.skip_value()?;
        }
        match reader.next()? {
            b',' => {}
            b'}' => return Some(fields),
            _ => return None,
        }
    }
}

/// The top-level object's numbers with their keys, every other value skipped; `None` for text
/// that is no JSON object.
pub(super) fn number_fields(text: &str) -> Option<Vec<(String, f64)>> {
    let mut reader = Reader { bytes: text.as_bytes(), at: 0 };
    let mut fields = Vec::new();
    reader.expect(b'{')?;
    if reader.peek()? == b'}' {
        return Some(fields);
    }
    loop {
        let key = reader.string()?;
        reader.expect(b':')?;
        if matches!(reader.peek()?, b'-' | b'0'..=b'9') {
            fields.push((key, reader.number()?));
        } else {
            reader.skip_value()?;
        }
        match reader.next()? {
            b',' => {}
            b'}' => return Some(fields),
            _ => return None,
        }
    }
}

/// The top-level object's arrays of strings with their keys, every other value skipped (an
/// array holding anything but strings among them); `None` for text that is no JSON object.
pub(super) fn string_list_fields(text: &str) -> Option<Vec<(String, Vec<String>)>> {
    let mut reader = Reader { bytes: text.as_bytes(), at: 0 };
    let mut fields = Vec::new();
    reader.expect(b'{')?;
    if reader.peek()? == b'}' {
        return Some(fields);
    }
    loop {
        let key = reader.string()?;
        reader.expect(b':')?;
        if reader.peek()? == b'[' {
            let start = reader.at;
            if let Some(list) = reader.string_list() {
                fields.push((key, list));
            } else {
                reader.at = start;
                reader.skip_value()?;
            }
        } else {
            reader.skip_value()?;
        }
        match reader.next()? {
            b',' => {}
            b'}' => return Some(fields),
            _ => return None,
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    /// An array of strings; `None`, the reader left inside it, for any other value in it.
    fn string_list(&mut self) -> Option<Vec<String>> {
        self.expect(b'[')?;
        let mut list = Vec::new();
        if self.peek()? == b']' {
            self.at += 1;
            return Some(list);
        }
        loop {
            if self.peek()? != b'"' {
                return None;
            }
            list.push(self.string()?);
            match self.next()? {
                b',' => {}
                b']' => return Some(list),
                _ => return None,
            }
        }
    }
    fn peek(&mut self) -> Option<u8> {
        while self.bytes.get(self.at)?.is_ascii_whitespace() {
            self.at += 1;
        }
        self.bytes.get(self.at).copied()
    }
    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.at += 1;
        Some(byte)
    }
    fn expect(&mut self, byte: u8) -> Option<()> {
        (self.next()? == byte).then_some(())
    }
    fn number(&mut self) -> Option<f64> {
        self.peek()?;
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(|b| b.is_ascii_digit() || matches!(b, b'-' | b'+' | b'.' | b'e' | b'E')) {
            self.at += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.at]).ok()?.parse().ok().filter(|value: &f64| value.is_finite())
    }
    fn boolean(&mut self) -> Option<bool> {
        self.peek()?;
        for (word, value) in [(&b"true"[..], true), (&b"false"[..], false)] {
            if self.bytes[self.at..].starts_with(word) {
                self.at += word.len();
                return Some(value);
            }
        }
        None
    }
    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut units = Vec::new();
        let mut out = String::new();
        loop {
            let byte = *self.bytes.get(self.at)?;
            self.at += 1;
            if byte != b'\\' && !units.is_empty() {
                out.push_str(&String::from_utf16(&std::mem::take(&mut units)).ok()?);
            }
            match byte {
                b'"' => return Some(out),
                b'\\' => {
                    let escape = *self.bytes.get(self.at)?;
                    self.at += 1;
                    match escape {
                        b'u' => {
                            let hex = std::str::from_utf8(self.bytes.get(self.at..self.at + 4)?).ok()?;
                            units.push(u16::from_str_radix(hex, 16).ok()?);
                            self.at += 4;
                        }
                        other => {
                            if !units.is_empty() {
                                out.push_str(&String::from_utf16(&std::mem::take(&mut units)).ok()?);
                            }
                            out.push(match other {
                                b'"' => '"',
                                b'\\' => '\\',
                                b'/' => '/',
                                b'b' => '\u{8}',
                                b'f' => '\u{c}',
                                b'n' => '\n',
                                b'r' => '\r',
                                b't' => '\t',
                                _ => return None,
                            });
                        }
                    }
                }
                _ => {
                    let start = self.at - 1;
                    let len = utf8_len(byte)?;
                    out.push_str(std::str::from_utf8(self.bytes.get(start..start + len)?).ok()?);
                    self.at = start + len;
                }
            }
        }
    }
    fn skip_value(&mut self) -> Option<()> {
        match self.peek()? {
            b'"' => self.string().map(drop),
            open @ (b'{' | b'[') => {
                self.at += 1;
                let close = if open == b'{' { b'}' } else { b']' };
                if self.peek()? == close {
                    self.at += 1;
                    return Some(());
                }
                loop {
                    if open == b'{' {
                        self.string()?;
                        self.expect(b':')?;
                    }
                    self.skip_value()?;
                    match self.next()? {
                        b',' => continue,
                        byte if byte == close => return Some(()),
                        _ => return None,
                    }
                }
            }
            _ => {
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'.'))
                {
                    self.at += 1;
                }
                (self.at > start).then_some(())
            }
        }
    }
}

fn utf8_len(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7f => Some(1),
        0xc0..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf7 => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips_through_its_own_writer() {
        for show_lobby_chat in [true, false] {
            let config = OnlineConfig {
                nickname: "Sk8 \"Q\" \\ é".into(),
                server: "203.0.113.7:32082".into(),
                show_lobby_chat,
                show_ghost_players: !show_lobby_chat,
            };
            assert_eq!(parse(&config.text()), Some(config));
        }
    }

    #[test]
    fn ghost_players_show_unless_the_file_turns_them_off() {
        assert!(parse(r#"{"nickname": "Rob", "show_lobby_chat": false}"#).unwrap().show_ghost_players);
        let config = parse(r#"{"show_ghost_players": false, "nickname": "Rob"}"#).unwrap();
        assert!(!config.show_ghost_players && config.show_lobby_chat);
        assert!(parse(r#"{"show_ghost_players": 0}"#).unwrap().show_ghost_players);
    }

    #[test]
    fn lobby_chat_shows_unless_the_file_turns_it_off() {
        assert!(parse(r#"{"nickname": "Rob", "server": ""}"#).unwrap().show_lobby_chat);
        assert!(!parse(r#"{"show_lobby_chat": false, "nickname": "Rob"}"#).unwrap().show_lobby_chat);
        assert!(parse(r#"{"show_lobby_chat": "no"}"#).unwrap().show_lobby_chat);
        assert_eq!(parse(r#"{"show_lobby_chat": tru}"#), None);
    }

    #[test]
    fn a_config_without_a_server_uses_the_default_and_keeps_naming_none() {
        let config = parse(r#"{"nickname": "Rob"}"#).unwrap();
        assert_eq!(config.server(), DEFAULT_SERVER);
        assert_eq!(parse(r#"{"nickname": "Rob", "server": " "}"#).unwrap().server(), DEFAULT_SERVER);
        let saved = parse(&config.text()).unwrap();
        assert_eq!(saved.server, "");
        assert_eq!(saved.server(), DEFAULT_SERVER);
        assert_eq!(parse(r#"{"server": " play.example.net:4000 "}"#).unwrap().server(), "play.example.net:4000");
    }

    #[test]
    fn numbers_are_read_with_their_keys_and_other_values_skipped() {
        let fields = number_fields(r#"{"a": 1.25, "b": "2", "c": -3, "d": [4], "e": 5e1}"#).unwrap();
        assert_eq!(fields, [("a".to_string(), 1.25), ("c".to_string(), -3.), ("e".to_string(), 50.)]);
        assert_eq!(number_fields(r#"{"a": 1.2.3}"#), None);
        assert_eq!(number_fields("[]"), None);
    }

    #[test]
    fn other_keys_are_skipped_and_escapes_decoded() {
        let text = r#"{"version": 2, "servers": [{"a": [1, true, null]}], "nickname": "Aé\nB"}"#;
        assert_eq!(parse(text).map(|c| c.nickname), Some("Aé\nB".to_string()));
        assert_eq!(parse("{}"), Some(OnlineConfig::default()));
        assert_eq!(parse("{\"nickname\": }"), None);
    }
}
