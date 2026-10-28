//! The dedicated server's public statistics (PC port addition): `GET /serverstats` on the
//! moderation panel's port ([`crate::panel`]) answers, with no key, a JSON object of how many
//! players are signed in and every public game: its challenge type and challenge by their
//! English names, whether its challenge (or a free skate's activity) is in progress, and its
//! players' nicknames in join order. It names no address, no private game and no one signed in
//! outside the public games.

use crate::json;
use crate::protocol::challenge;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

/// The English titles of the game's challenges by key (the `challenges` collection key as
/// decimal text), as the game's English localization reads them.
const CHALLENGE_NAMES: &str = include_str!("../data/challenge_names.json");

/// What a name reads for a challenge key the list does not hold.
pub const UNKNOWN: &str = "Unknown";

/// What `/serverstats` shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// The connections signed in.
    pub players_online: usize,
    /// The public games, oldest first.
    pub games: Vec<PublicGame>,
}

/// A public game as `/serverstats` shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicGame {
    pub challenge_type: u8,
    pub challenge_key: u64,
    /// The freeskate activity the game plays, while it plays one.
    pub activity: Option<u64>,
    /// The game is past lobby skate: its challenge (or activity) loads, runs or shows its results.
    pub in_progress: bool,
    /// The members' nicknames in join order, the host first.
    pub players: Vec<String>,
}

fn names() -> &'static HashMap<u64, String> {
    static NAMES: OnceLock<HashMap<u64, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        json::object(CHALLENGE_NAMES)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(key, value)| match value {
                json::Value::Text(name) => Some((key.parse().ok()?, name)),
                _ => None,
            })
            .collect()
    })
}

/// A challenge's English title, [`UNKNOWN`] for a key the list does not hold.
#[must_use]
pub fn challenge_name(key: u64) -> &'static str {
    names().get(&key).map_or(UNKNOWN, String::as_str)
}

/// A challenge type's English name: the title of its `gChallengeTemplate` entry
/// (`MissionCmdTranslator::sMethod_GetChallengeTypeNameFromType` 0x8260e0f0) for the game's
/// type (`Sk8::Net::GetSk8ChallengeType` 0x827840b0), as the lobby browser shows it. Slap's title
/// is a placeholder in the game's data, so it reads as its `Sk8::Net::ChallengeType` name.
#[must_use]
pub const fn challenge_type_name(challenge_type: u8) -> &'static str {
    match challenge_type {
        challenge::OWN_THE_SPOT => "Spot Battle",
        challenge::SLAP => "Slap",
        challenge::BEST_TRICK => "Best Trick",
        challenge::JAM => "Jam",
        challenge::ONE_UP => "1-Up",
        challenge::SKATE => "S.K.A.T.E.",
        challenge::DEATH_RACE => "Deathrace",
        challenge::HALL_OF_MEAT => "Hall of Meat",
        challenge::ONLINE_FREE_SKATE => "Freeskate",
        _ => UNKNOWN,
    }
}

/// The statistics as the endpoint sends them.
#[must_use]
pub fn json(stats: &Stats) -> String {
    let mut text = String::with_capacity(256 + stats.games.len() * 256);
    let _ = write!(text, "{{\n  \"players_online\": {},\n  \"public_games\": [", stats.players_online);
    for (index, game) in stats.games.iter().enumerate() {
        text.push_str(if index == 0 { "\n" } else { ",\n" });
        let activity = game.activity.map_or_else(|| "null".to_owned(), |key| json::quote(challenge_name(key)));
        let players: Vec<String> = game.players.iter().map(|name| json::quote(name)).collect();
        let _ = write!(
            text,
            "    {{\n      \"challenge_type\": {},\n      \"challenge\": {},\n      \"activity\": {activity},\n      \"in_progress\": {},\n      \"players\": [{}]\n    }}",
            json::quote(challenge_type_name(game.challenge_type)),
            json::quote(challenge_name(game.challenge_key)),
            game.in_progress,
            players.join(", "),
        );
    }
    text.push_str(if stats.games.is_empty() { "]\n}\n" } else { "\n  ]\n}\n" });
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_in_the_list_reads_and_an_unlisted_key_is_unknown() {
        assert!(names().len() > 500, "the list reads whole");
        assert_eq!(challenge_name(326_571_255_016_410_850), "The Rez: Midtown");
        assert_eq!(challenge_name(187_379_566_711_254_103), "It's On!", "escapes read as the text");
        assert_eq!(challenge_name(1), UNKNOWN);
        assert_eq!(challenge_type_name(challenge::SKATE), "S.K.A.T.E.");
        assert_eq!(challenge_type_name(challenge::OWN_THE_SPOT), "Spot Battle");
        assert_eq!(challenge_type_name(challenge::COUNT), UNKNOWN);
    }

    #[test]
    fn the_endpoint_reads_as_json_with_names_in_place_of_numbers() {
        let stats = Stats {
            players_online: 3,
            games: vec![
                PublicGame { challenge_type: challenge::ONLINE_FREE_SKATE, challenge_key: 326_571_255_016_410_850, activity: None, in_progress: false, players: vec!["Rob".into(), "Mc'Gill".into()] },
                PublicGame { challenge_type: challenge::SKATE, challenge_key: 7, activity: None, in_progress: true, players: vec!["Big \"B\"".into()] },
            ],
        };
        let text = json(&stats);
        assert!(text.contains("\"players_online\": 3"));
        assert!(text.contains("\"challenge_type\": \"Freeskate\"") && text.contains("\"challenge\": \"The Rez: Midtown\""));
        assert!(text.contains("\"players\": [\"Rob\", \"Mc'Gill\"]"));
        assert!(text.contains("\"challenge\": \"Unknown\"") && text.contains("\"in_progress\": true") && text.contains("\"activity\": null"));
        assert!(text.contains("\"Big \\\"B\\\"\""), "a name is quoted as JSON");
        let empty = json(&Stats::default());
        assert_eq!(empty, "{\n  \"players_online\": 0,\n  \"public_games\": []\n}\n");
    }
}
