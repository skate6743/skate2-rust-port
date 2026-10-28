//! PC port addition: `config.json` beside `online-config.json` (the working directory the game
//! runs from, or `SKATE2_SAVE_ROOT`), naming the folder of the original game a launch plays from,
//! the release whose changes the game last showed and the downloadable content linked. The
//! player picks the folder once, by the game's `.xex`; `SKATE2_GAME_ROOT` overrides it.
use super::online_config::{quote, string_fields, string_list_fields};
use std::{fs, io, path::PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameConfig {
    /// The folder of the game's `.xex`, which holds its `data` folder; empty for none.
    pub game_root: String,
    /// The release whose changes the game last showed (`changelog_seen`); empty for none. A
    /// player's first launch of another release shows its own.
    pub changelog_seen: String,
    /// The downloadable content files linked on the Downloadable Content screen, in the order
    /// they were linked: content packages or archives extracted from them (`linked_dlc`). A
    /// launch checks each and drops one that no longer opens as downloadable content.
    pub linked_dlc: Vec<String>,
}

impl GameConfig {
    /// `SKATE2_SAVE_ROOT` relocates it with the saves; otherwise it sits in the working
    /// directory the game runs from.
    pub fn path() -> io::Result<PathBuf> {
        Ok(match std::env::var_os("SKATE2_SAVE_ROOT") {
            Some(root) => PathBuf::from(root),
            None => std::env::current_dir()?,
        }
        .join("config.json"))
    }

    /// A missing or unreadable file names no folder.
    #[must_use]
    pub fn load() -> Self {
        Self::path().and_then(fs::read_to_string).ok().map_or_else(Self::default, |text| parse(&text))
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.text())
    }

    fn text(&self) -> String {
        let dlc = if self.linked_dlc.is_empty() {
            "[]".to_owned()
        } else {
            let lines: Vec<String> = self.linked_dlc.iter().map(|path| format!("    {}", quote(path))).collect();
            format!("[\n{}\n  ]", lines.join(",\n"))
        };
        format!(
            "{{\n  \"game_root\": {},\n  \"changelog_seen\": {},\n  \"linked_dlc\": {dlc}\n}}\n",
            quote(&self.game_root),
            quote(&self.changelog_seen)
        )
    }
}

fn parse(text: &str) -> GameConfig {
    let fields = string_fields(text).unwrap_or_default();
    let field = |name: &str| fields.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone()).unwrap_or_default();
    let linked_dlc = string_list_fields(text)
        .unwrap_or_default()
        .into_iter()
        .find(|(key, _)| key == "linked_dlc")
        .map(|(_, list)| list)
        .unwrap_or_default();
    GameConfig { game_root: field("game_root"), changelog_seen: field("changelog_seen"), linked_dlc }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_folder_round_trips_through_its_own_writer() {
        let config = GameConfig {
            game_root: r#"D:\Games\Skate 2 "Copy" \ é"#.into(),
            changelog_seen: "0.2.5".into(),
            linked_dlc: vec![r"D:\DLC\4541087F\00000002\B6E4D01C47E3928D521B81CE44E209CCEBE809E1".into(), r"D:\DLC\filmpack_xbox2.big".into()],
        };
        assert_eq!(parse(&config.text()), config);
        let none = GameConfig { linked_dlc: Vec::new(), ..config };
        assert_eq!(parse(&none.text()), none);
    }

    #[test]
    fn other_keys_and_broken_text_name_no_folder_or_are_skipped() {
        assert_eq!(parse(r#"{"other": [1, {"a": "b"}], "game_root": "C:\\Games\\Skate 2"}"#).game_root, r"C:\Games\Skate 2");
        assert_eq!(parse("not json"), GameConfig::default());
        assert_eq!(parse("{}"), GameConfig::default());
        assert_eq!(parse(r#"{"linked_dlc": ["a", 2], "game_root": "x"}"#), GameConfig { game_root: "x".into(), ..GameConfig::default() });
    }

    #[test]
    fn a_file_from_before_the_changelog_has_seen_none() {
        let config = parse("{\n  \"game_root\": \"C:\\\\Games\"\n}\n");
        assert_eq!((config.game_root.as_str(), config.changelog_seen.as_str()), (r"C:\Games", ""));
        assert!(config.linked_dlc.is_empty());
    }
}
