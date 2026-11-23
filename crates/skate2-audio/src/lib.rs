//! Skate 2 audio: the RenderWare Audio Core (RWAC) mixer, its plug-ins and decoders, and the
//! middleware the game drives through it (Splicer, AEMS, Csis, Nicotine). Each module names the
//! debug-executable functions it reproduces; `docs/audio/implementation.md` records the facts.

pub mod aems;
pub mod collision;
pub mod csis;
pub mod custom_song;
pub mod formats;
pub mod global_effects;
pub mod grain_player;
pub mod main_objects;
pub mod math;
pub mod movie;
pub mod music;
pub mod nicotine;
pub mod player;
pub mod presentation;
pub mod random;
pub mod reverb;
pub mod rwac;
pub mod scorables;
pub mod speech;
pub mod splicer;
pub mod surface;
pub mod system;
pub mod utils;
pub mod world;
pub mod vault;
pub mod vmx;
