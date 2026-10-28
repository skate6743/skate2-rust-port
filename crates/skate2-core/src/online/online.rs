//! Online play's game-side pieces that stand apart from the live session: the audio and trick HUD
//! records carried in replay frames, the online challenge lists the matchmaking screens offer, the lobby
//! chat's prompt and lines, and the front-end controllers of the online screens. The connection and the remote skaters live in
//! the session (`selected_session_online.rs`); the server and wire format in `skate2-net`.

#[path = "activities.rs"]
pub mod activities;
#[path = "audio_record.rs"]
pub mod audio_record;
#[path = "catalog.rs"]
pub mod catalog;
#[path = "chat.rs"]
pub mod chat;
#[path = "event_log.rs"]
pub mod event_log;
#[path = "facts.rs"]
pub mod facts;
#[path = "hud_record.rs"]
pub mod hud_record;

pub use skate2_net::protocol::challenge as challenge_type;

/// The build string players are matched on (`Net::GetProtocolVersion`'s role).
pub const GAME_VERSION: &str = concat!("skate2-pc/", env!("CARGO_PKG_VERSION"), "/net2");
