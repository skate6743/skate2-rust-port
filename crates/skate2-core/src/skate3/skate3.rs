//! PC port addition: a linked Skate 3 data folder, played as downloadable content. The player
//! picks Skate 3's `default.xex` on the Downloadable Content screen ("Link Skate 3 Data"); its
//! folder's `data` holds the archives read here, in Skate 3's own formats (`EB` archives,
//! `chunkref` entries, the skater database split into `skaterschema` and `skatercollections`,
//! loose global locators).
//!
//! What it adds:
//! - Port Carverton's districts (University, Downtown, Industrial) as additional worlds, their
//!   streams joining the game's mounts after the downloadable content's. A district stands where
//!   San Vanelona does, so while one is loaded the city's own collections leave the focus
//!   selections ([`is_district_world`]).
//! - Each district's challenge map teleports: Skate 3's `fe_locations` of the district, named by
//!   Skate 3's English strings, at their global locators.
//! - Skate 3's database (`render_locations` for each district's lighting, Skate 3's clothing) and
//!   strings, read from Skate 3's own set of mounts so that no Skate 2 file is replaced.
//! - The online Map Pool's Skate 3 maps: the online challenges of Skate 3's challenge bank
//!   (`challengebanks\main`) whose type Skate 2 has, at their district's global locators
//!   ([`OnlineSpot`]). Their games run no scenes, and their hulls' streamed splines (a spot's
//!   trick area) draw in Skate 2's blue ribbon colour.
//! - Skate 3's instance ids as Lua and the float lookups take them ([`id_float`]): exact, where
//!   Skate 2's `(float)(double)` folds every one of them into one float.
//! - Skate 3's downloadable content linked over it ([`dlc`]): its maps stand beside the districts,
//!   with their teleports and online challenges, and its clothing joins Skate 3's.

use crate::challenge::{
    attrib_db::{AttribDb, Element, ForeignNode, Synthesis},
    keys::{class, hash64, hash_value32},
    runtime::field::STATES,
};
use skate2_formats::cac_parts::{CacPart, CacPartDb, SLOT_NAMES};
use skate2_formats::{
    localization::{Histogram, LanguageCategory},
    location::LocationTable,
    mount::{MountSet, MountedBig},
    pegasus::{LOCATION_DESC_DATA_TYPE_ID, LocationDescData, TABLE_OF_CONTENTS_TYPE_ID, TableOfContents},
    rw4::Arena,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError, RwLock},
};

/// Skate 3's downloadable content packs.
#[path = "skate3_dlc.rs"]
pub mod dlc;

/// Skate 2's head under Skate 3's tops.
#[path = "skate3_head_fit.rs"]
pub mod head_fit;

/// One of Port Carverton's districts the link adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DistrictSpec {
    /// The `fe_locations` (and `world`) collection prefix of its locations.
    pub key: &'static str,
    /// Its `world` record's `Name`, which names its stream XMLs ([`world_stream_xml`]).
    pub world: &'static str,
    /// Its world stream under `data\content\world\stream`, and its archive
    /// `data\content\world<stream>.big`.
    pub stream: &'static str,
    /// Its `render_locations` collection; empty for a downloadable content map, whose world record
    /// names its own by key ([`District::render_location`]).
    pub render_location: &'static str,
    /// Its proxy stream (Skate 2's `PROXY`), in `data\content\proxy<proxy>.big`; empty for none
    /// (a downloadable content map's).
    pub proxy: &'static str,
    /// What the challenge map and the online map pool call it.
    pub title: &'static str,
    /// Its loose world models in `miscload.big`, each after its textures: the sky and water its
    /// Vault `world` record names (sky textures `2a0bf629355afb15` and model
    /// `c7a0a84f018e87ba`, water textures `ca5a157a65e75934` and model `951898f6c0fa6856`, a
    /// field the record lacks taken from its parent's). `DIST_Water` is Industrial's water and
    /// backdrop alone (its coast, sailboat and tree cards), which would float over the other
    /// districts.
    pub loose: &'static [&'static str],
}

pub const DISTRICTS: [DistrictSpec; 3] = [
    DistrictSpec {
        key: "dist_university",
        world: "dist_university",
        stream: "DIST_University",
        render_location: "university",
        proxy: "University_100_Proxy",
        title: "University District",
        loose: &[
            "data\\content\\world\\models\\DIST_skybox_Textures.rx2",
            "data\\content\\world\\models\\DIST_skybox.rx2",
            "data\\content\\world\\models\\DIST_Water_University_Textures.rx2",
            "data\\content\\world\\models\\DIST_Water_University.rx2",
        ],
    },
    DistrictSpec {
        key: "dist_downtown",
        world: "dist_downtown",
        stream: "DIST_DownTown",
        render_location: "downtown",
        proxy: "Downtown_100_Proxy",
        title: "Downtown District",
        loose: &[
            "data\\content\\world\\models\\DIST_skybox_downtown_Textures.rx2",
            "data\\content\\world\\models\\DIST_skybox_downtown.rx2",
            "data\\content\\world\\models\\DIST_Water_DownTown_Textures.rx2",
            "data\\content\\world\\models\\DIST_Water_DownTown.rx2",
        ],
    },
    DistrictSpec {
        key: "dist_industrial",
        world: "dist_industrial",
        stream: "DIST_Industrial",
        render_location: "industrial",
        proxy: "Industrial_100_Proxy",
        title: "Industrial District",
        loose: &[
            "data\\content\\world\\models\\DIST_skybox_industrial_Textures.rx2",
            "data\\content\\world\\models\\DIST_skybox_industrial.rx2",
            "data\\content\\world\\models\\DIST_Water_Textures.rx2",
            "data\\content\\world\\models\\DIST_Water.rx2",
        ],
    },
];

/// The archives a link opens, under the folder's `data`: the database, the language databases,
/// the clothing and each district's stream.
const DATABASE_ARCHIVE: &str = "data\\big\\db.big";
const BOOT_ARCHIVE: &str = "data\\big\\miscboot.big";
const CLOTHING_ARCHIVE: &str = "data\\content\\createacharacter.big";
/// The online games' boundaries (`data\boundary`), in Skate 3's own mounts.
const LOAD_ARCHIVE: &str = "data\\big\\miscload.big";
/// The clothing thumbnails (`data\fe\cas`), in Skate 3's own mounts.
const THUMBNAIL_ARCHIVE: &str = "data\\big\\fedynamic.big";
/// The scene archive: its scenes' animations and ICE cameras.
const SCENE_ARCHIVE: &str = "data\\big\\scene.big";
/// The part slots whose Skate 3 parts join Skate 2's: the clothing, accessory and board slots
/// Slappy's categories list, the inner torsos its shirts layer over, the arm and leg models its
/// shirts and pants require, and the hair slot, of whose parts only [`JOINED_HAIR_STYLES`] join.
/// Skate 3's bodies, heads and faces stay out of Back Alley Pete's.
const MERCHANDISE_SLOTS: [u8; 16] = [0, 1, 2, 5, 6, 11, 12, 13, 15, 17, 18, 20, 21, 22, 23, 25];
/// The hair slot (`Hair`, one of [`MERCHANDISE_SLOTS`]), and `HairStyle` among a part's string
/// indices.
const HAIR_SLOT: u8 = 2;
const HAIR_STYLE: usize = 17;
/// PC port addition (user request): the hair styles of Skate 3's that join Back Alley Pete's, the
/// emo styles the Skate.Create Upgrade Pack adds, which fit Skate 2's heads where Skate 3's other
/// hair does not. Each joins with its hat model (`HairModelType` hat), which a hat that needs hat
/// hair swaps in as it swaps Skate 2's.
const JOINED_HAIR_STYLES: [&str; 2] = ["emo short", "emo long"];

/// `Attrib::Gen::fe_locations` and `render_locations` (the same keys as Skate 2's).
const FE_LOCATIONS: u64 = 0xddcd_6bc2_3255_e508;
const RENDER_LOCATIONS: u64 = 0x6a2b_5946_514b_8490;
/// `Attrib::Gen::world`, the world an `fe_locations`' `World` names, and what a downloadable
/// content map takes of it: its stream (`WorldStream`), its title, its `RenderLocation`, its sky
/// and water models ([`DistrictSpec::loose`]'s four fields) and, for a world on Skate 3's disc,
/// its archive (`worldDIST_BlackBoxPark.big`) under its folder (`data/content`).
const WORLD: u64 = 0xbc69_c54e_6e10_2d88;
const FE_WORLD: u64 = 0x7565_cd2a_80e9_3a5e;
const WORLD_STREAM: u64 = 0x2a65_5e8a_29c1_f642;
/// `world.Name`.
const WORLD_NAME: u64 = 0x942a_b8ae_e4b4_14ed;
const WORLD_TITLE: u64 = 0x095e_1d97_78f0_c6f4;
const WORLD_RENDER_LOCATION: u64 = 0x7e23_d107_85c4_3717;
const WORLD_MODELS: [u64; 4] = [0x2a0b_f629_355a_fb15, 0xc7a0_a84f_018e_87ba, 0xca5a_157a_65e7_5934, 0x9518_98f6_c0fa_6856];
const WORLD_FOLDER: u64 = 0x3fb3_32c6_26ad_7c24;
const WORLD_ARCHIVE: u64 = 0x295c_85d2_b299_7ed3;
/// A world flag (its name unknown) that TU3 `0x8255b8c8` tests before it indexes a world's
/// location activities: set on the four maps Skate 3's downloadable content streams, clear on its
/// Skate.Park lots.
const WORLD_OWN_ACTIVITIES: u64 = 0x1719_c577_673e_b7c3;
/// The `render_locations` a downloadable content map's world names when Skate 3 holds none of
/// its own (After Dark's `sanitarium`).
const DEFAULT_RENDER_LOCATION: &str = "default";
/// Every part slot: a pack's clothing joins Skate 3's in each.
const ALL_SLOTS: [u8; 26] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25];
/// Skate 3's challenge bank, beside its skater database.
const CHALLENGE_BANK: &str = "data\\db\\challengebanks\\main.vlt";
/// `challenges` and its `GlobalData` and `Name`; `challenge_global_data`'s online flag,
/// `ChallengeType`, `Location` and `Title` (Skate 3 moved `Title` into the global data): the same
/// keys as Skate 2's.
const CHALLENGES: u64 = 0x8876_e0b5_5674_0e36;
const GLOBAL_DATA: u64 = 0x2449_1bf0_0316_aa45;
const CHALLENGE_NAME: u64 = 0x942a_b8ae_e4b4_14ed;
const ONLINE: u64 = 0xf9a9_a1dd_ed84_a643;
const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
const LOCATION: u64 = 0xf1c6_d5fa_4a23_951e;
const TITLE: u64 = 0xe2a5_0788_b6c2_0c92;
/// The online challenge types a Skate 3 map hosts: online free skate (`eChallengeTypes` 22),
/// S.K.A.T.E. (4), Spot Battle (53, `own_the_spots_battles`), Hall of Meat (31), Death Race
/// (19) and 1-Up (32), whose rounds run Skate 2's own over the game's Skate 3 data
/// ([`Skate3Data::graft_online_challenges`]). Skate 3's Own the Spot (9) is its career's: the
/// online flag on six of them names the Spot Battles of the same spots.
const ONLINE_GAME_TYPES: [i32; 6] = [22, 4, SPOT_BATTLE_GAME_TYPE, 31, 19, ONE_UP_GAME_TYPE];
/// 1-Up, the same `eChallengeTypes` in both games.
const ONE_UP_GAME_TYPE: i32 = 32;
/// `eChallengeTypes` online free skate, the one type without a challenge of its own to join.
const FREE_SKATE_GAME_TYPE: i32 = 22;
/// Skate 3's Spot Battle, which Skate 2 has no `eChallengeTypes` for: its games are Skate 2's
/// Own the Spot (9), the menus' Spot Battle.
const SPOT_BATTLE_GAME_TYPE: i32 = 53;
const OWN_THE_SPOT_GAME_TYPE: i32 = 9;

/// `GetNetChallengeType` of a Skate 3 `eChallengeTypes`.
fn net_challenge_type(game_type: i32) -> Option<u8> {
    crate::online::catalog::net_challenge_type(if game_type == SPOT_BATTLE_GAME_TYPE { OWN_THE_SPOT_GAME_TYPE } else { game_type })
}
/// `challenges.LocalData`.
const LOCAL_DATA: u64 = 0xd7bb_2b16_0caf_8ed6;
/// Each Skate 3 challenge's local data Vault, beside the challenge bank.
const LOCAL_DATA_VAULTS: &str = "data\\db\\challenge_local_data";
/// The Vault of the local data's shared records, which each game's local data inherits.
const LOCAL_DATA_FRAMEWORK: &str = "data\\db\\challenge_local_data_framework.vlt";

/// Challenge fields a graft reads or replaces (`Attrib::StringHash64` of their names, where
/// known; the rest by what they hold).
const SPOT_VOLUMES: u64 = 0x00ec_6afd_7f8a_76d8;
const CHALLENGE_BOUNDARY: u64 = 0xb894_8105_0d48_4ed9;
const SCORING_BOUNDARY: u64 = 0xfc7c_6353_408b_885c;
const TURN_BASED_START_VOLUME: u64 = 0xed8c_312e_a6b4_662f;
const TURN_BASED_ATTEMPT_SPAWN_POINT: u64 = 0x5382_98df_635d_3f72;
const TURN_BASED_WAITING_LOCATION: u64 = 0x8357_b4a9_1366_b98e;
const VISUAL_INDICATORS: u64 = 0x7ae8_fef7_7a54_bb91;
const START_LOCATION: u64 = 0xe9b6_99e3_1f7c_45ca;
const MAP_START_LOCATION: u64 = 0xa6e8_9247_15a1_5cec;
const SKATE_CHALLENGE_PATHS: u64 = 0x94e9_e347_4802_a0f5;
/// Skate 3's element is 16 bytes, Skate 2's 12.
const INTRO_PRESENTATION_EVENTS: u64 = 0xbc45_6e59_56e1_ef68;
const DEBUG_NAME: u64 = 0xfaa8_1ec4_261f_c04c;
const DESCRIPTION: u64 = 0x6b0b_d2ba_e508_b742;
/// `TimeLimit`, and the session flag of Skate 3's Spot Battle (`CumulativeScoring`, its local
/// data's) and of Skate 2's (`ContestCumulativeScoring`, the challenge's): a session spot banks
/// every line inside the time limit into the turn's score, a spot without it scores one line.
const TIME_LIMIT: u64 = 0xf89b_fdfc_8899_0f17;
const CUMULATIVE_SCORING: u64 = 0xdd21_d0f4_e716_d82f;
const CONTEST_CUMULATIVE_SCORING: u64 = 0xbf51_d1ae_70a1_a24e;
/// `challenge_local_data`: the spot volumes the sequence connector tracks (Skate 3's local data
/// holds them as `SpotVolumes`), and `NumSequencesToEndTurn`, the lines a turn without the
/// session flag scores before it ends.
const LOCAL_SPOT_VOLUMES: u64 = 0x0a67_5615_20c8_5c04;
const NUM_SEQUENCES_TO_END_TURN: u64 = 0xbeba_1d8b_453f_8137;
/// `challenge_local_data.NISFlythroughDefinition`, the spot's flythrough the online state graphs'
/// `nis_flythrough` plays: Skate 3 keeps it under Skate 2's name (`ots_univ_03_intro`,
/// `HOM_indu_01`). Skate 3's `IntroNIS` (`dom_intro`, a scene with the players) has no Skate 2
/// reader.
const NIS_FLYTHROUGH_DEFINITION: u64 = 0x8132_5b07_7445_a249;
/// Death Race's `RaceType`, its finish (`tLocationID`) and its heat (`tRaceHeatDefinition`).
const RACE_TYPE: u64 = 0x4e46_eb05_f1da_f0c8;
const RACE_LOCATION: u64 = 0xc0f3_2560_3d50_d547;
const RACE_HEAT: u64 = 0xeea0_f28b_2d5f_863c;
/// Skate 3's state graphs (`challenge_stategraph`, a Vault each): a graph's `challenge_states`,
/// which a Skate 3 game of a graft with `graph` runs.
const STATE_GRAPHS: u64 = 0x3eac_d8f6_f7b3_1f63;
const STATE_GRAPH_VAULTS: &str = "data\\db\\challenge_stategraph";
/// How deep the records a graph game's values name may name further records.
const GRAPH_RECORD_DEPTH: u32 = 3;
/// Value types that name another record by class and collection key (two big-endian 64-bit
/// keys), which a graft joins with the challenge that names it.
const RECORD_TYPES: [&str; 4] = [
    "Sk8::Challenge::tRaceHeatDefinition",
    "Sk8::Challenge::tRaceLegDefinition",
    "Sk8::Challenge::tRaceBranchDefinition",
    GATE_RECORD_TYPE,
];
/// A race leg's gates (`challenge_race_gates`): each sets only its `GateVolume`, its indicator
/// type and ribbon (Skate 2's own race gate ribbon) coming from Skate 3's root gate, so a gate
/// joins with every field its parents give it.
const GATE_RECORD_TYPE: &str = "Sk8::Challenge::tRaceGateDefinition";
/// Record fields Skate 3 renamed, as `(class, Skate 3 field, Skate 2 field)`: a race leg's
/// gates, Skate 3's `Gates`, are the `RandomGates` Skate 2's Death Race reads.
const RENAMED_RECORD_FIELDS: [(u64, u64, u64); 1] = [(0x7cbd_399d_98bb_0d37, 0x8198_4915_9b9e_26ad, 0x6342_f924_d4cf_4726)];
/// How deep the records a graft joins may name further records.
const RECORD_DEPTH: u32 = 4;

/// Where a Skate 3 game holds a grafted field's value.
#[derive(Clone, Copy)]
enum Held {
    Global,
    Local,
}

/// A Skate 2 online challenge a Skate 3 game of its type runs as: the template's state graph,
/// rules, timers and messages, with each of `fields` (the template's own spot in San Vanelona)
/// taken from where the Skate 3 game holds its counterpart, or, without one, from the template's
/// parent. With `local_fields`, the game's local data joins too, as the template's with each
/// field taken from the Skate 3 field beside it in the same way. With `graph`, the game runs
/// Skate 3's own state graph of that name over all of its own data instead
/// ([`Skate3Data::graft_graph_game`]).
struct OnlineGraft {
    game_type: i32,
    template: &'static str,
    fields: &'static [(u64, Option<(u64, Held)>)],
    local_fields: &'static [(u64, u64)],
    graph: Option<&'static str>,
}

const ONLINE_GRAFTS: [OnlineGraft; 5] = [
    // S.K.A.T.E.: `skate2_online`'s state graph; Skate 3 keeps the same fields.
    OnlineGraft {
        game_type: 4,
        template: "ol_skate_01_ot_plazastairs",
        fields: &[
            (SPOT_VOLUMES, Some((SPOT_VOLUMES, Held::Local))),
            (CHALLENGE_BOUNDARY, Some((CHALLENGE_BOUNDARY, Held::Local))),
            (TURN_BASED_START_VOLUME, Some((TURN_BASED_START_VOLUME, Held::Local))),
            (TURN_BASED_ATTEMPT_SPAWN_POINT, Some((TURN_BASED_ATTEMPT_SPAWN_POINT, Held::Local))),
            (TURN_BASED_WAITING_LOCATION, Some((TURN_BASED_WAITING_LOCATION, Held::Local))),
            (VISUAL_INDICATORS, Some((VISUAL_INDICATORS, Held::Local))),
            (SKATE_CHALLENGE_PATHS, None),
            (INTRO_PRESENTATION_EVENTS, Some((INTRO_PRESENTATION_EVENTS, Held::Local))),
            (DEBUG_NAME, None),
            (DESCRIPTION, None),
        ],
        local_fields: &[],
        graph: None,
    },
    // Spot Battle: Skate 3's local data holds the spot volume, its boundary, the start, attempt
    // and waiting locators, the time limit and the session flag under Skate 2's names.
    OnlineGraft {
        game_type: SPOT_BATTLE_GAME_TYPE,
        template: "f_1a_24_ots_svm13_rockgap_ol",
        fields: &[
            (SPOT_VOLUMES, Some((SPOT_VOLUMES, Held::Local))),
            (SCORING_BOUNDARY, Some((SCORING_BOUNDARY, Held::Local))),
            (VISUAL_INDICATORS, Some((VISUAL_INDICATORS, Held::Local))),
            (START_LOCATION, Some((START_LOCATION, Held::Local))),
            (TURN_BASED_ATTEMPT_SPAWN_POINT, Some((TURN_BASED_ATTEMPT_SPAWN_POINT, Held::Local))),
            (TURN_BASED_WAITING_LOCATION, Some((TURN_BASED_WAITING_LOCATION, Held::Local))),
            (MAP_START_LOCATION, Some((MAP_START_LOCATION, Held::Global))),
            (TIME_LIMIT, Some((TIME_LIMIT, Held::Local))),
            (CONTEST_CUMULATIVE_SCORING, Some((CUMULATIVE_SCORING, Held::Local))),
            (INTRO_PRESENTATION_EVENTS, Some((INTRO_PRESENTATION_EVENTS, Held::Local))),
            (DEBUG_NAME, None),
            (DESCRIPTION, None),
        ],
        local_fields: &[
            (LOCAL_SPOT_VOLUMES, SPOT_VOLUMES),
            (NUM_SEQUENCES_TO_END_TURN, NUM_SEQUENCES_TO_END_TURN),
            (NIS_FLYTHROUGH_DEFINITION, NIS_FLYTHROUGH_DEFINITION),
        ],
        graph: None,
    },
    // Hall of Meat: Skate 3's hull holds the spot; the start as for Own the Spot; the local data
    // joins for the spot's flythrough.
    OnlineGraft {
        game_type: 31,
        template: "ol_hom_20_svm05_megacompound",
        fields: &[
            (VISUAL_INDICATORS, Some((VISUAL_INDICATORS, Held::Local))),
            (START_LOCATION, Some((START_LOCATION, Held::Local))),
            (TURN_BASED_ATTEMPT_SPAWN_POINT, Some((START_LOCATION, Held::Local))),
            (TURN_BASED_WAITING_LOCATION, Some((START_LOCATION, Held::Local))),
            (MAP_START_LOCATION, Some((MAP_START_LOCATION, Held::Global))),
            (INTRO_PRESENTATION_EVENTS, Some((INTRO_PRESENTATION_EVENTS, Held::Local))),
            (DEBUG_NAME, None),
            (DESCRIPTION, None),
        ],
        local_fields: &[(NIS_FLYTHROUGH_DEFINITION, NIS_FLYTHROUGH_DEFINITION)],
        graph: None,
    },
    // Death Race: the same fields; its heat and the heat's leg join as records.
    OnlineGraft {
        game_type: 19,
        template: "r_2_2_dr_ur_02_ol",
        fields: &[
            (RACE_TYPE, Some((RACE_TYPE, Held::Local))),
            (VISUAL_INDICATORS, Some((VISUAL_INDICATORS, Held::Local))),
            (RACE_LOCATION, Some((RACE_LOCATION, Held::Local))),
            (START_LOCATION, Some((START_LOCATION, Held::Local))),
            (RACE_HEAT, Some((RACE_HEAT, Held::Local))),
            (DEBUG_NAME, None),
            (DESCRIPTION, None),
        ],
        local_fields: &[],
        graph: None,
    },
    // 1-Up: Skate 3's own 1-Up state graph and rules over all of the game's own data; the
    // template gives the challenge what Skate 2's systems read of it (its hull and location).
    OnlineGraft {
        game_type: ONE_UP_GAME_TYPE,
        template: "ol_1up_01_svm_mountainup",
        fields: &[(TURN_BASED_START_VOLUME, None), (DEBUG_NAME, None), (DESCRIPTION, None)],
        local_fields: &[],
        graph: Some("1up"),
    },
];

/// The records of a graph game ([`Skate3Data::graft_graph_game`]): its template, the template's
/// global data (class and key), and the game's own global and local data in Skate 3's bank.
struct GraphGame {
    template: u64,
    global: (u64, u64),
    source_global: (u64, u64),
    local: (u64, u64),
}

/// A record's `Attrib::RefSpec` as an array element holds it: class, collection, then padding.
fn record_reference(class: u64, key: u64) -> Vec<u8> {
    [class.to_be_bytes(), key.to_be_bytes(), [0; 8]].concat()
}

/// A message handler's chunk with the global it defines (`h_<key>`) named for `to`.
fn renamed_chunk(chunk: &[u8], from: u64, to: u64) -> Vec<u8> {
    let (from, to) = (format!("h_{from:016X}"), format!("h_{to:016X}"));
    let mut renamed = chunk.to_vec();
    let mut at = 0;
    while let Some(found) = renamed[at..].windows(from.len()).position(|window| window == from.as_bytes()) {
        let start = at + found;
        renamed[start..start + from.len()].copy_from_slice(to.as_bytes());
        at = start + from.len();
    }
    renamed
}

/// The Skate 3 online games no graft could join this run, which the menus leave out.
static UNPLAYABLE: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());
/// `challenge_global_data.FELargeImage`, and the global data's template title
/// (`ID_MISSION_TEMPLATE_SKATE_TITLE`), which titles a game whose own title has no English.
const FE_LARGE_IMAGE: u64 = 0x7974_27ed_230c_9980;
const TEMPLATE_TITLE: u64 = 0xbca0_ade9_4b04_b8a7;
/// `RequiredChallengeHull`: Skate 2's `challenges` field, which Skate 3 moved into the global
/// data. The spot's hull (`data\content\missions\<name>`, Skate 3's `missions.big`) holds its
/// start and waiting locators and volumes.
const REQUIRED_CHALLENGE_HULL: u64 = 0x876b_f081_9aa5_8858;
/// Skate 3's challenge hulls, which join the game's mounts after Skate 2's own.
const MISSIONS_ARCHIVE: &str = "data\\content\\missions.big";
/// The location image a Skate 3 game shows in the menus, which have none of Skate 2's own: the
/// Online options' "any location" image (`fe\source\images\locations\any_location`).
pub const LOCATION_IMAGE: &str = "any_location";

/// A location image's small version (`<image>_sm`), which the online menus show; a Skate 3 game's
/// ([`LOCATION_IMAGE`]) has none, and shows "any location" itself, as the menus do for no
/// particular location (`mDefaultImage`).
#[must_use]
pub fn small_location_image(image: &str) -> String {
    if image == LOCATION_IMAGE { image.to_owned() } else { format!("{image}_sm") }
}

/// One of Skate 3's online challenges at a district: a game of the online Map Pool's Skate 3 maps.
#[derive(Clone, Debug, PartialEq)]
pub struct OnlineSpot {
    /// Its `challenges` collection key in Skate 3's challenge bank.
    pub key: u64,
    /// Its network challenge type.
    pub net_type: u8,
    /// The challenge's `Name` (`freeskate_university_central`).
    pub name: String,
    /// Its title, Skate 3's English string.
    pub title: String,
    /// Its global data's `Location`, a global locator of the district.
    pub location: String,
    /// The district's world stream.
    pub stream: &'static str,
    /// The district's `render_locations` key.
    pub render_location: u64,
    /// The locator's transform.
    pub transform: [f32; 16],
}

/// A teleport of a district's challenge map: an `fe_locations` collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    /// Its `fe_locations` collection key in Skate 3's database.
    pub key: u64,
    /// The collection's name (`dist_university_clocktower`).
    pub name: String,
    /// Its title, Skate 3's English string (`ID_LOT_UN_08`), else the collection's name.
    pub title: String,
    /// The global locator it spawns at (`Z_UN_ClockTower`).
    pub locator: String,
    /// The locator's transform.
    pub transform: [f32; 16],
}

#[derive(Clone, Debug)]
pub struct District {
    pub spec: DistrictSpec,
    /// Its `render_locations` collection key in Skate 3's database.
    pub render_location: u64,
    pub spots: Vec<Spot>,
}

/// A linked Skate 3 data folder.
pub struct Skate3Data {
    /// The folder holding `default.xex` and `data`.
    pub root: PathBuf,
    /// Skate 3's own archives and loose files, its skater database joined.
    pub mounts: Arc<MountSet>,
    /// Skate 3's skater database.
    pub db: Arc<AttribDb>,
    /// Skate 3's English strings by `StringHash`.
    pub strings: HashMap<u32, String>,
    pub districts: Vec<District>,
    /// The online games at the districts.
    pub online: Vec<OnlineSpot>,
    /// The global locators' transforms by `GetHashValue32(name)`.
    locators: HashMap<u32, [f32; 16]>,
    /// Skate 3's clothing database.
    clothing: Option<Arc<CacPartDb>>,
    /// The local data's shared records ([`LOCAL_DATA_FRAMEWORK`] and the packs' frameworks),
    /// sorted: what a game's local data inherits, which only a graph game's graft reads.
    framework_locals: Vec<u64>,
    /// What each linked downloadable content pack added, in the order linked.
    pub packs: Vec<PackContents>,
    /// The packs' archives, which join the game's mounts through [`dlc::game_view`].
    pack_archives: Vec<MountedBig>,
    /// The worlds on Skate 3's disc the packs' unlocks open.
    disc_worlds: Vec<PathBuf>,
}

/// What a linked downloadable content pack added, which the Downloadable Content screen shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackContents {
    /// The pack ([`dlc::Pack::identity`]).
    pub identity: String,
    /// Its maps' titles.
    pub maps: Vec<String>,
    /// How many online games its maps host.
    pub online: usize,
    /// How many pieces of clothing it adds to Edit Skater's Merchandise.
    pub clothing: usize,
}

impl std::fmt::Debug for Skate3Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Skate3Data").field("root", &self.root).field("districts", &self.districts.len()).finish_non_exhaustive()
    }
}

/// Why a folder is not Skate 3's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Skate3Error {
    /// The file picked is not `default.xex`, or its folder has no `data` beside it.
    NotSkate3(String),
    /// A file of Skate 3's could not be read.
    Read(String),
}

impl std::fmt::Display for Skate3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSkate3(why) => write!(f, "is not a Skate 3 game folder ({why})"),
            Self::Read(why) => write!(f, "could not be read ({why})"),
        }
    }
}

impl std::error::Error for Skate3Error {}

/// The archive file of a district's stream under `root`.
fn district_archive(root: &Path, spec: &DistrictSpec) -> PathBuf {
    root.join("data").join("content").join(format!("world{}.big", spec.stream))
}

fn archive(root: &Path, relative: &str) -> PathBuf {
    root.join(relative.replace('\\', std::path::MAIN_SEPARATOR_STR))
}

/// The folder of a picked file: Skate 3's `default.xex` names the folder its `data` is in, and a
/// disc image is read in place as the folder itself (`skate2_formats::disc`).
#[must_use]
pub fn folder_of_pick(picked: &Path) -> PathBuf {
    if skate2_formats::host_path::is_dir(picked) { picked.to_owned() } else { picked.parent().map_or_else(|| picked.to_owned(), Path::to_owned) }
}

/// Whether `root` looks like Skate 3's folder: `default.xex`, the database and every district's
/// stream.
///
/// # Errors
///
/// [`Skate3Error::NotSkate3`] naming the first file missing.
pub fn validate(root: &Path) -> Result<(), Skate3Error> {
    let present = |path: &Path| skate2_formats::host_path::is_file(&skate2_formats::host_path::on_disk(path));
    if !present(&root.join("default.xex")) {
        return Err(Skate3Error::NotSkate3("no default.xex".into()));
    }
    for relative in [DATABASE_ARCHIVE, BOOT_ARCHIVE] {
        if !present(&archive(root, relative)) {
            return Err(Skate3Error::NotSkate3(format!("no {relative}")));
        }
    }
    for spec in &DISTRICTS {
        let path = district_archive(root, spec);
        if !present(&path) {
            return Err(Skate3Error::NotSkate3(format!("no data\\content\\world{}.big", spec.stream)));
        }
    }
    Ok(())
}

fn open(path: &Path) -> Result<MountedBig, Skate3Error> {
    MountedBig::open(skate2_formats::host_path::on_disk(path), false).map_err(|e| Skate3Error::Read(format!("{}: {e}", path.display())))
}

/// The world whose own global locators a locator file holds: the folder it sits in under its
/// group (`global_locators\BAM\DIST_University\`, `global_locators\DLC_BAM\DIST_AGPark\`),
/// lowercase.
fn locator_stream(path: &str) -> Option<String> {
    const FOLDER: &str = "global_locators\\";
    let path = path.to_ascii_lowercase().replace('/', "\\");
    let rest = &path[path.find(FOLDER)? + FOLDER.len()..];
    let mut parts = rest.split('\\');
    let (_group, stream, _file) = (parts.next()?, parts.next()?, parts.next()?);
    Some(stream.to_owned())
}

/// The collection a reference field of `collection` names. A `ClassRefSpec_<class>` field
/// (`fe_locations.World`, `world.RenderLocation`) names its own class, so its value holds the
/// collection first; an `Attrib::RefSpec` holds the class, then the collection.
fn referenced(db: &AttribDb, class: u64, collection: u64, field: u64) -> Option<u64> {
    let value = db.attribute(class, collection, field)?;
    let (first, second) = value.ref_spec(0)?;
    let names_class = db.type_name(value.definition.type_id).is_some_and(|name| name.starts_with("Attrib::Gen::ClassRefSpec_"));
    let key = if names_class { first } else { second };
    (key != 0).then_some(key)
}

/// What a location's world record (`world`, the location's `World`) holds of it.
struct WorldRecord {
    name: String,
    stream: String,
    title: String,
    render_location: u64,
    models: Vec<String>,
    folder: String,
    archive: String,
    own_activities: bool,
}

impl WorldRecord {
    fn of(db: &AttribDb, location: u64) -> Option<Self> {
        let world = referenced(db, FE_LOCATIONS, location, FE_WORLD)?;
        let text = |field| db.attribute(WORLD, world, field).and_then(|value| value.text(0).map(str::to_owned)).unwrap_or_default();
        let stream = text(WORLD_STREAM);
        if stream.is_empty() {
            return None;
        }
        Some(Self {
            name: text(WORLD_NAME),
            stream,
            title: text(WORLD_TITLE),
            render_location: referenced(db, WORLD, world, WORLD_RENDER_LOCATION).unwrap_or(0),
            models: WORLD_MODELS.iter().map(|&field| text(field)).collect(),
            folder: text(WORLD_FOLDER),
            archive: text(WORLD_ARCHIVE),
            own_activities: db.attribute(WORLD, world, WORLD_OWN_ACTIVITIES).and_then(|value| value.bool(0)) == Some(true),
        })
    }
}

/// A location as a map beside the districts: its world (an additional world standing where San
/// Vanelona does) lit by its world's render location (`default` when Skate 3 holds none of that
/// name), its loose sky and water its world record's, titled by the location's string (else its
/// world's, else its name), the location itself its one teleport, at its global locator (else at
/// its world's own location of that name). `None` without a locator to stand at.
fn map_district(
    db: &AttribDb,
    mounts: &MountSet,
    text: &dyn Fn(&str) -> Option<String>,
    locators: &HashMap<u32, [f32; 16]>,
    location: u64,
    world: &WorldRecord,
) -> Option<District> {
    let name = db.layout_text(FE_LOCATIONS, location, 0)?.to_owned();
    let locator = db.layout_text(FE_LOCATIONS, location, 1).map(str::to_owned).filter(|locator| !locator.is_empty())?;
    let transform = match locators.get(&hash_value32(&locator)) {
        Some(&transform) => transform,
        None => {
            let path = format!("data\\content\\world\\stream\\{0}\\{0}_Sim.loc", world.stream);
            let found = mounts.read_path(&path).ok().and_then(|bytes| LocationTable::parse(&bytes).ok()).and_then(|table| table.find(&locator).map(|start| start.transform));
            let Some(transform) = found else {
                eprintln!("skate 3 dlc: {name}'s locator {locator} is no global locator");
                return None;
            };
            transform
        }
    };
    let title = db
        .layout_text(FE_LOCATIONS, location, 2)
        .and_then(text)
        .filter(|title| !title.is_empty())
        .or_else(|| text(&world.title).filter(|title| !title.is_empty()))
        .unwrap_or_else(|| name.clone());
    let folder = if world.folder.is_empty() { "data/content" } else { world.folder.as_str() };
    let loose: Vec<String> = world
        .models
        .iter()
        .filter(|model| !model.is_empty())
        .map(|model| format!("{folder}/{model}.rx2").replace('/', "\\"))
        .filter(|path| mounts.contains_path(path))
        .collect();
    let render_location = [world.render_location, hash64(DEFAULT_RENDER_LOCATION)]
        .into_iter()
        .find(|&key| key != 0 && db.has_collection(RENDER_LOCATIONS, key))
        .unwrap_or(0);
    // Its world's own name (`dist_artgallerydlc`) names its stream XMLs; the location's stands in
    // for a record without one.
    let world_name = if world.name.is_empty() { name.as_str() } else { world.name.as_str() };
    let spec = kept_spec(&name, world_name, &world.stream, &title, &loose);
    Some(District { spec: *spec, render_location, spots: vec![Spot { key: location, name, title, locator, transform }] })
}

/// The files a clothing database's part of `slot` loads, as Edit Skater composes it
/// (`edit_skater_character::compose_recipe`): each level of detail of its model, and the
/// textures there of its material (else of the first); none when the database's master recipe
/// lacks its model.
fn part_files(db: &CacPartDb, slot: u8, part: &CacPart) -> Option<Vec<String>> {
    let name = SLOT_NAMES.get(usize::from(slot))?;
    let component = db.master.components.iter().find(|component| component.name == *name)?;
    let model = component.models.iter().find(|model| model.asset_id == part.model_id)?;
    let mut files = Vec::new();
    for lod in &model.lods {
        files.push(format!("data\\content\\createacharacter\\model\\{}\\{}\\0x{:016x}.rx2", db.master.schema_name, component.name, lod.arena_asset_id));
        for instance in &lod.material_instances {
            let material = instance
                .materials
                .iter()
                .find(|material| material.asset_id == part.material_id)
                .or_else(|| instance.materials.iter().min_by_key(|material| material.asset_id));
            files.extend(
                material
                    .into_iter()
                    .flat_map(|material| &material.shader_parameters)
                    .map(|parameter| format!("data\\content\\createacharacter\\texture\\0x{:016x}.rx2", parameter.asset_id)),
            );
        }
    }
    Some(files)
}

/// The Merchandise pieces ([`MERCHANDISE_SLOTS`]) `joined` holds past `before`.
fn merchandise_added(joined: &CacPartDb, before: Option<&CacPartDb>) -> usize {
    MERCHANDISE_SLOTS
        .iter()
        .map(|&slot| joined.slot_range(slot).len().saturating_sub(before.map_or(0, |before| before.slot_range(slot).len())))
        .sum()
}

/// Every named location of a standalone locator arena (`tLocationDesc`s of its kind-21 assets).
fn locator_transforms(bytes: &[u8], found: &mut HashMap<u32, [f32; 16]>) -> Result<(), String> {
    let resources = Arena::split_file_resources(bytes).map_err(|e| format!("{e:?}"))?;
    let arena = Arena::parse(&resources).map_err(|e| format!("{e:?}"))?;
    let Some(toc) = arena.dictionary.iter().find(|entry| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID) else { return Ok(()) };
    let toc = TableOfContents::parse(arena.object_bytes(&resources, toc).ok_or("table of contents outside the arena")?)
        .map_err(|e| format!("{e:?}"))?;
    for entry in toc.entries.iter().filter(|entry| entry.object_type == LOCATION_DESC_DATA_TYPE_ID) {
        let source = arena.local_entry(entry.object).ok_or("location data outside the arena")?;
        let bytes = arena.object_bytes(&resources, source).ok_or("location data outside the arena")?;
        let data = LocationDescData::parse(bytes).map_err(|e| format!("{e:?}"))?;
        for desc in &data.locations {
            if let Some(name) = desc.name.as_deref() {
                found.entry(hash_value32(name)).or_insert(desc.transform);
            }
        }
    }
    Ok(())
}

/// Skate 3's English strings: its Global category through its histogram.
fn english(mounts: &MountSet) -> Result<HashMap<u32, String>, String> {
    let lang = "data\\fe\\languages\\english\\LANGUAGE_English";
    let histogram = mounts.read_path(&format!("{lang}_HISTOGRAM_skate3ng.BIN")).map_err(|e| e.to_string())?;
    let global = mounts.read_path(&format!("{lang}_Global_skate3ng.BIN")).map_err(|e| e.to_string())?;
    let mut strings = HashMap::new();
    decode_strings(&histogram, &global, &mut strings)?;
    Ok(strings)
}

/// A language category's strings through its histogram, each joining `strings` unless a string of
/// its id is there.
fn decode_strings(histogram: &[u8], category: &[u8], strings: &mut HashMap<u32, String>) -> Result<(), String> {
    let histogram = Histogram::parse(histogram).map_err(|e| e.to_string())?;
    let category = LanguageCategory::parse(category).map_err(|e| e.to_string())?;
    for (hash, packed) in category.packed_strings() {
        let Ok(decoded) = histogram.unpack(packed, 16384, false) else { continue };
        if decoded.valid && decoded.terminated {
            strings.entry(hash).or_insert_with(|| String::from_utf16_lossy(&decoded.units).trim_end_matches('\0').to_owned());
        }
    }
    Ok(())
}

impl Skate3Data {
    /// Opens the Skate 3 folder `root`.
    ///
    /// # Errors
    ///
    /// [`Skate3Error`] when it is not Skate 3's, or its database or strings cannot be read.
    pub fn open(root: &Path) -> Result<Self, Skate3Error> {
        Self::open_with(root, &[])
    }

    /// [`Self::open`] with Skate 3's downloadable content `packs` ([`dlc`]), in the order linked:
    /// each pack's archive joins Skate 3's own mounts after the game's, its Vaults, strings and
    /// clothing join Skate 3's, and its maps stand beside the districts.
    ///
    /// # Errors
    ///
    /// [`Skate3Error`] when the folder is not Skate 3's, or its database or strings cannot be read.
    pub fn open_with(root: &Path, packs: &[Arc<dlc::Pack>]) -> Result<Self, Skate3Error> {
        validate(root)?;
        let mut mounts = MountSet::new();
        for relative in [DATABASE_ARCHIVE, BOOT_ARCHIVE] {
            mounts.push(open(&archive(root, relative))?);
        }
        for optional in [CLOTHING_ARCHIVE, LOAD_ARCHIVE, THUMBNAIL_ARCHIVE] {
            let path = archive(root, optional);
            if skate2_formats::host_path::is_file(&skate2_formats::host_path::on_disk(&path)) {
                mounts.push(open(&path)?);
            }
        }
        for spec in &DISTRICTS {
            mounts.push(open(&district_archive(root, spec))?);
        }
        // The packs' archives, whole, after the game's own (`dlcbig:`).
        for archive in packs.iter().flat_map(|pack| &pack.archives) {
            mounts.push(archive.clone());
        }
        mounts.set_loose_root(root);
        mounts.join_skater_database().map_err(|e| Skate3Error::Read(format!("skater database: {e}")))?;
        // The challenge bank joins the skater database, whose schema its collections read with;
        // without it the link holds no online maps.
        let db = match AttribDb::load(&mounts, &["data\\db\\skater.vlt", CHALLENGE_BANK]) {
            Ok(db) => db,
            Err(error) => {
                eprintln!("skate 3 data: challenge bank: {error}");
                AttribDb::load(&mounts, &["data\\db\\skater.vlt"]).map_err(|e| Skate3Error::Read(format!("skater database: {e}")))?
            }
        };
        let mut strings = english(&mounts).map_err(|e| Skate3Error::Read(format!("English strings: {e}")))?;
        // The packs' English after Skate 3's own.
        for (histogram, category) in packs.iter().flat_map(|pack| &pack.archives).flat_map(dlc::english) {
            if let Err(error) = decode_strings(&histogram, &category, &mut strings) {
                eprintln!("skate 3 dlc: English strings: {error}");
            }
        }
        // Each global locator's transform, and the world whose own locators its file holds:
        // Skate 3's loose files, then the packs'.
        let mut locators = HashMap::new();
        let mut locator_streams: HashMap<u32, String> = HashMap::new();
        let mut take = |name: &str, bytes: &[u8]| {
            let mut found = HashMap::new();
            if let Err(error) = locator_transforms(bytes, &mut found) {
                eprintln!("skate 3 data: {name}: {error}");
            }
            if let Some(stream) = locator_stream(name) {
                for &hash in found.keys() {
                    locator_streams.entry(hash).or_insert_with(|| stream.clone());
                }
            }
            for (hash, transform) in found {
                locators.entry(hash).or_insert(transform);
            }
        };
        for name in mounts.loose_files("data\\content\\global_locators\\") {
            if !name.to_ascii_lowercase().ends_with(".rx2") {
                continue;
            }
            let bytes = mounts.read_path(&name).map_err(|e| Skate3Error::Read(format!("{name}: {e}")))?;
            take(&name, &bytes);
        }
        for archive in packs.iter().flat_map(|pack| &pack.archives) {
            for name in dlc::locators(archive) {
                match archive.read_path(&name) {
                    Ok(bytes) => take(&name, &bytes),
                    Err(error) => eprintln!("skate 3 dlc: {name}: {error}"),
                }
            }
        }
        let mut db = db;
        // The state graphs Skate 3's own games run under, each in its Vault.
        for graph in ONLINE_GRAFTS.iter().filter_map(|graft| graft.graph) {
            let path = format!("{STATE_GRAPH_VAULTS}\\{graph}.vlt");
            if let Err(error) = db.load_more(&mounts, &path) {
                eprintln!("skate 3 data: {path}: {error}");
            }
        }
        // The local data's shared records (`default`, `main`, `1ups` and the like), the parents of
        // each game's local data, in a Vault of their own.
        let before = db.collections(class::CHALLENGE_LOCAL_DATA).to_vec();
        if let Err(error) = db.load_more(&mounts, LOCAL_DATA_FRAMEWORK) {
            eprintln!("skate 3 data: {LOCAL_DATA_FRAMEWORK}: {error}");
        }
        let mut framework_locals: Vec<u64> =
            db.collections(class::CHALLENGE_LOCAL_DATA).iter().copied().filter(|key| before.binary_search(key).is_err()).collect();
        // The packs' Vaults, each pack's lists in turn (`cVaultListLoadCallback`): a pack's own
        // local data framework shares its records as Skate 3's does. The locations each adds.
        let mut added_locations = Vec::with_capacity(packs.len());
        for pack in packs {
            let locations_before = db.collections(FE_LOCATIONS).to_vec();
            for path in pack.archives.iter().flat_map(dlc::vaults) {
                let locals_before = db.collections(class::CHALLENGE_LOCAL_DATA).to_vec();
                if let Err(error) = db.load_more(&mounts, &path) {
                    eprintln!("skate 3 dlc {}: {path}: {error}", pack.title);
                    continue;
                }
                if path.to_ascii_lowercase().contains("local_data_framework") {
                    framework_locals.extend(db.collections(class::CHALLENGE_LOCAL_DATA).iter().copied().filter(|key| locals_before.binary_search(key).is_err()));
                }
            }
            added_locations.push(db.table_order(FE_LOCATIONS).into_iter().filter(|key| locations_before.binary_search(key).is_err()).collect::<Vec<_>>());
        }
        framework_locals.sort_unstable();
        framework_locals.dedup();
        let text = |id: &str| strings.get(&skate2_formats::localization::string_hash(id.as_bytes())).cloned();
        let mut districts = Vec::new();
        for spec in DISTRICTS {
            let prefix = format!("{}_", spec.key);
            let mut spots = Vec::new();
            for key in db.table_order(FE_LOCATIONS) {
                let Some(name) = db.layout_text(FE_LOCATIONS, key, 0).map(str::to_owned) else { continue };
                if !name.to_ascii_lowercase().starts_with(&prefix) {
                    continue;
                }
                let Some(locator) = db.layout_text(FE_LOCATIONS, key, 1).map(str::to_owned).filter(|l| !l.is_empty()) else { continue };
                let Some(&transform) = locators.get(&hash_value32(&locator)) else {
                    eprintln!("skate 3 data: {name}'s locator {locator} is no global locator");
                    continue;
                };
                let title = db.layout_text(FE_LOCATIONS, key, 2).and_then(text).filter(|t| !t.is_empty()).unwrap_or_else(|| name.clone());
                spots.push(Spot { key, name, title, locator, transform });
            }
            let render_location = crate::challenge::keys::hash64(spec.render_location);
            let render_location = if db.has_collection(RENDER_LOCATIONS, render_location) { render_location } else { 0 };
            districts.push(District { spec, render_location, spots });
        }
        // The packs' maps: each pack's own locations whose worlds it streams (not a Skate.Park
        // lot), then the disc's worlds its unlocks open.
        let mut contents = Vec::with_capacity(packs.len());
        let mut pack_streams: Vec<Vec<&'static str>> = Vec::with_capacity(packs.len());
        let mut disc_worlds = Vec::new();
        for (pack, added) in packs.iter().zip(&added_locations) {
            let mut pack_contents = PackContents { identity: pack.identity.clone(), ..PackContents::default() };
            let mut streams = Vec::new();
            for &location in added {
                let Some(world) = WorldRecord::of(&db, location) else { continue };
                if !world.own_activities || !pack.archives.iter().any(|archive| dlc::streams(archive, &world.stream)) {
                    continue;
                }
                if let Some(district) = map_district(&db, &mounts, &text, &locators, location, &world) {
                    eprintln!("skate 3 dlc {}: {} ({})", pack.title, district.spec.title, district.spec.stream);
                    pack_contents.maps.push(district.spec.title.to_owned());
                    streams.push(district.spec.stream);
                    districts.push(district);
                }
            }
            for name in pack.archives.iter().flat_map(dlc::unlocked_locations) {
                let location = hash64(&name);
                if !db.has_collection(FE_LOCATIONS, location) || districts.iter().any(|district| district.spots.iter().any(|spot| spot.key == location)) {
                    continue;
                }
                let Some(world) = WorldRecord::of(&db, location) else { continue };
                let path = root.join(world.folder.replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR)).join(&world.archive);
                if world.archive.is_empty() || !skate2_formats::host_path::is_file(&skate2_formats::host_path::on_disk(&path)) {
                    eprintln!("skate 3 dlc {}: {name}'s world {} is not on the disc", pack.title, world.archive);
                    continue;
                }
                if let Some(district) = map_district(&db, &mounts, &text, &locators, location, &world) {
                    eprintln!("skate 3 dlc {}: {} ({}, on the disc)", pack.title, district.spec.title, district.spec.stream);
                    pack_contents.maps.push(district.spec.title.to_owned());
                    streams.push(district.spec.stream);
                    districts.push(district);
                    disc_worlds.push(path);
                }
            }
            contents.push(pack_contents);
            pack_streams.push(streams);
        }
        let districts_of: HashMap<u32, usize> = locator_streams
            .iter()
            .filter_map(|(&hash, stream)| Some((hash, districts.iter().position(|district| district.spec.stream.eq_ignore_ascii_case(stream))?)))
            .collect();
        let mut online = online_spots(&db, &text, &locators, &districts_of, &districts);
        // A challenge's game needs its spot's local data, in its own Vault.
        online.retain(|spot| {
            if spot.net_type == net_challenge_type(FREE_SKATE_GAME_TYPE).unwrap_or(u8::MAX) {
                return true;
            }
            let path = format!("{LOCAL_DATA_VAULTS}\\{}.vlt", spot.name);
            match db.load_more(&mounts, &path) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("skate 3 data: {}: {error}", spot.name);
                    false
                }
            }
        });
        for (pack_contents, streams) in contents.iter_mut().zip(&pack_streams) {
            pack_contents.online = online.iter().filter(|spot| streams.contains(&spot.stream)).count();
        }
        let mut clothing = match mounts.read_path(skate2_formats::cac_parts::SKATE3_DATABASE_PATH) {
            Ok(bytes) => match CacPartDb::parse(&bytes) {
                Ok(parts) => Some(parts),
                Err(error) => {
                    eprintln!("skate 3 data: clothing: {error}");
                    None
                }
            },
            Err(_) => None,
        };
        // The packs' clothing, each database's parts after each slot's own (as Skate 2's take
        // Skate 3's, [`Self::clothing_for`]). A pack's database also holds the parts of the packs
        // before it, whose files only those packs carry: a pack joins the parts whose files Skate
        // 3 or the pack itself carries, so each brings its own and none shows a part of a pack
        // not linked; those it carries are offered, whatever DLC ids they hold.
        let in_pack = |pack: &dlc::Pack, path: &str| pack.archives.iter().any(|archive| archive.archive().find_path(path).is_some());
        let in_skate3 = |path: &str| mounts.contains_path(path) && !packs.iter().any(|pack| in_pack(pack, path));
        for (pack, pack_contents) in packs.iter().zip(&mut contents) {
            for archive in &pack.archives {
                for path in dlc::clothing(archive) {
                    let parts = match archive.read_path(&path).map_err(|e| e.to_string()).and_then(|bytes| CacPartDb::parse(&bytes).map_err(|e| e.to_string())) {
                        Ok(parts) => parts,
                        Err(error) => {
                            eprintln!("skate 3 dlc {}: {path}: {error}", pack.title);
                            continue;
                        }
                    };
                    let Some(base) = clothing.as_ref() else {
                        eprintln!("skate 3 dlc {}: {path} joins Skate 3's own clothing, which did not load", pack.title);
                        continue;
                    };
                    let carried = |slot: u8, part: &CacPart| {
                        part_files(&parts, slot, part).is_some_and(|files| files.iter().all(|file| in_skate3(file) || in_pack(pack, file)))
                    };
                    match base.with_skate3_where(&parts, &ALL_SLOTS, carried, true) {
                        Ok(joined) => {
                            pack_contents.clothing += merchandise_added(&joined, Some(base));
                            clothing = Some(joined);
                        }
                        Err(error) => eprintln!("skate 3 dlc {}: {path} cannot join: {error}", pack.title),
                    }
                }
            }
        }
        for (pack, pack_contents) in packs.iter().zip(&contents) {
            eprintln!(
                "skate 3 dlc {}: {} maps, {} online games, {} pieces of clothing",
                pack.title,
                pack_contents.maps.len(),
                pack_contents.online,
                pack_contents.clothing
            );
        }
        let clothing = clothing.map(Arc::new);
        let pack_archives = packs.iter().flat_map(|pack| pack.archives.iter().cloned()).collect();
        Ok(Self {
            root: root.to_owned(),
            mounts: Arc::new(mounts),
            db: Arc::new(db),
            strings,
            districts,
            online,
            locators,
            clothing,
            framework_locals,
            packs: contents,
            pack_archives,
            disc_worlds,
        })
    }

    /// Slappy's parts with Skate 3's clothing joined ([`MERCHANDISE_SLOTS`]), and Back Alley
    /// Pete's hair with Skate 3's of [`JOINED_HAIR_STYLES`]; `None` without it.
    #[must_use]
    pub fn clothing_for(&self, skate2: &CacPartDb) -> Option<CacPartDb> {
        let clothing = self.clothing.as_ref()?;
        let joins = |slot: u8, part: &CacPart| {
            slot != HAIR_SLOT || clothing.string(part.attributes[HAIR_STYLE]).is_ok_and(|style| JOINED_HAIR_STYLES.contains(&style))
        };
        match skate2.with_skate3_where(clothing, &MERCHANDISE_SLOTS, joins, false) {
            Ok(joined) => Some(joined),
            Err(error) => {
                eprintln!("skate 3 data: clothing cannot join: {error}");
                None
            }
        }
    }

    /// PC port: each online challenge game at Skate 3's districts as a challenge of the challenge
    /// system's database `db`, so an online round of it runs: the Skate 2 online challenge of its
    /// type ([`ONLINE_GRAFTS`]: its fields and layout, under its state graph) over the game's
    /// Skate 3 data, with its own name, global data (its `Location` and image over the
    /// template's), hull and title, and the records its fields name (a race's heat
    /// and leg). The challenge bank's and local data Vaults join `db` for the nodes to read in.
    /// A game that cannot join is left out of the menus. Returns how many joined.
    pub fn graft_online_challenges(&self, db: &mut AttribDb) -> usize {
        if self.online.iter().all(|spot| net_challenge_type(FREE_SKATE_GAME_TYPE) == Some(spot.net_type)) {
            return 0;
        }
        // Skate 3's skater database (Vault 0) holds no challenge; the bank and local data do.
        let adopted: HashMap<u16, u16> = (1..self.db.vault_count()).filter_map(|index| Some((index, db.adopt_vault(&self.db, index)?))).collect();
        let mut added = 0;
        let mut unplayable = Vec::new();
        for graft in &ONLINE_GRAFTS {
            let Some(net_type) = net_challenge_type(graft.game_type) else { continue };
            let spots: Vec<&OnlineSpot> = self.online.iter().filter(|spot| spot.net_type == net_type).collect();
            if spots.is_empty() {
                continue;
            }
            let template = crate::challenge::keys::hash64(graft.template);
            let Some((global_class, template_global)) = db.attribute(CHALLENGES, template, GLOBAL_DATA).and_then(|global| global.ref_spec(0)) else {
                eprintln!("skate 3 data: the template {} is absent", graft.template);
                unplayable.extend(spots.iter().map(|spot| spot.key));
                continue;
            };
            let mut joined = 0;
            for spot in spots {
                if self.graft_online_challenge(db, &adopted, graft, spot, template, (global_class, template_global)) {
                    joined += 1;
                } else {
                    unplayable.push(spot.key);
                }
            }
            eprintln!("skate 3 data: {joined} games of type {} join the challenge system as {}", graft.game_type, graft.template);
            added += joined;
        }
        if let Ok(mut list) = UNPLAYABLE.lock() {
            *list = unplayable;
        }
        added
    }

    /// One game of [`Self::graft_online_challenges`]; false when it cannot join.
    fn graft_online_challenge(
        &self,
        db: &mut AttribDb,
        adopted: &HashMap<u16, u16>,
        graft: &OnlineGraft,
        spot: &OnlineSpot,
        template: u64,
        (global_class, template_global): (u64, u64),
    ) -> bool {
        let source = &*self.db;
        let reference = |field| source.attribute(CHALLENGES, spot.key, field).and_then(|value| value.ref_spec(0));
        let (Some((source_global_class, global)), Some((local_class, local))) = (reference(GLOBAL_DATA), reference(LOCAL_DATA)) else {
            eprintln!("skate 3 data: {} has no global or local data", spot.name);
            return false;
        };
        let node = |class, collection, field| self.graft_node(class, collection, field, adopted);
        let Some(location) = node(source_global_class, global, LOCATION) else { return false };
        let mut global_nodes = vec![(LOCATION, location)];
        global_nodes.extend(node(source_global_class, global, FE_LARGE_IMAGE).map(|image| (FE_LARGE_IMAGE, image)));
        let identity = self.identity_nodes(spot, (source_global_class, global), adopted);
        if let Some(graph) = graft.graph {
            let records = GraphGame { template, global: (global_class, template_global), source_global: (source_global_class, global), local: (local_class, local) };
            return self.graft_graph_game(db, graft, graph, spot, &records, global_nodes, identity, node(CHALLENGES, spot.key, LOCAL_DATA));
        }
        if !db.has_collection(global_class, global) && !db.graft(global_class, global, template_global, &[LOCATION, FE_LARGE_IMAGE], &global_nodes) {
            eprintln!("skate 3 data: {}'s global data cannot join", spot.name);
            return false;
        }
        let mut nodes = identity;
        for &(field, held) in graft.fields {
            let Some((key, held)) = held else { continue };
            let (class, collection) = match held {
                Held::Global => (source_global_class, global),
                Held::Local => (local_class, local),
            };
            if !self.graft_records(db, adopted, class, collection, key, RECORD_DEPTH) {
                eprintln!("skate 3 data: {}'s records cannot join", spot.name);
                return false;
            }
            if let Some(found) = node(class, collection, key) {
                nodes.push((field, found));
            }
        }
        let mut drop: Vec<u64> = graft.fields.iter().map(|&(field, _)| field).chain([CHALLENGE_NAME, GLOBAL_DATA, TITLE, REQUIRED_CHALLENGE_HULL]).collect();
        if !graft.local_fields.is_empty() {
            let (Some((_, template_local)), Some(reference)) =
                (db.attribute(CHALLENGES, template, LOCAL_DATA).and_then(|value| value.ref_spec(0)), node(CHALLENGES, spot.key, LOCAL_DATA))
            else {
                eprintln!("skate 3 data: {} has no local data to join", spot.name);
                return false;
            };
            let local_nodes: Vec<_> = graft.local_fields.iter().filter_map(|&(field, key)| Some((field, node(local_class, local, key)?))).collect();
            let local_drop: Vec<u64> = graft.local_fields.iter().map(|&(field, _)| field).collect();
            if !db.has_collection(local_class, local) && !db.graft(local_class, local, template_local, &local_drop, &local_nodes) {
                eprintln!("skate 3 data: {}'s local data cannot join", spot.name);
                return false;
            }
            nodes.push((LOCAL_DATA, reference));
            drop.push(LOCAL_DATA);
        }
        db.graft(CHALLENGES, spot.key, template, &drop, &nodes)
    }

    /// The node a graft takes of a Skate 3 value (`foreign_node`, not its layout's), through the
    /// record's parents short of the local data's shared records: what a game's local data
    /// inherits from them (an intro of Skate 3's presentation events, a time limit) stays the
    /// template's, as a field the game does not hold.
    fn graft_node(&self, class: u64, collection: u64, key: u64, adopted: &HashMap<u16, u16>) -> Option<ForeignNode> {
        let source = &*self.db;
        let mut current = collection;
        loop {
            if class == class::CHALLENGE_LOCAL_DATA && self.framework_locals.binary_search(&current).is_ok() {
                return None;
            }
            if source.own_keys(class, current).binary_search(&key).is_ok() {
                return source.foreign_node(class, current, key, adopted).filter(|found| !found.is_layout());
            }
            current = source.parent(class, current).filter(|&parent| parent != 0)?;
        }
    }

    /// The nodes of what Skate 2's systems read of a Skate 3 game's challenge: its name, global
    /// data, title and hull. The game's own title (Skate 3 moved it into the global data), which
    /// the HUD's `Mission.GetTitle` and the results' `SetInstanceNameData` show: Skate 2's
    /// interface reads a string id it lacks from the linked Skate 3 strings. A title Skate 3 has
    /// no English for keeps the template's.
    fn identity_nodes(&self, spot: &OnlineSpot, (source_global_class, global): (u64, u64), adopted: &HashMap<u16, u16>) -> Vec<(u64, ForeignNode)> {
        let source = &*self.db;
        let node = |class, collection, field| source.foreign_node(class, collection, field, adopted).filter(|found| !found.is_layout());
        let own_title = source
            .attribute(source_global_class, global, TITLE)
            .and_then(|value| value.text(0))
            .and_then(|id| self.text(id))
            .is_some_and(|text| !text.is_empty());
        let mut nodes = Vec::with_capacity(4);
        for (field, held) in [
            (CHALLENGE_NAME, (CHALLENGES, spot.key)),
            (GLOBAL_DATA, (CHALLENGES, spot.key)),
            (TITLE, (source_global_class, global)),
            (REQUIRED_CHALLENGE_HULL, (source_global_class, global)),
        ] {
            let key = if field == TITLE && !own_title { TEMPLATE_TITLE } else { field };
            if let Some(found) = node(held.0, held.1, key) {
                nodes.push((field, found));
            }
        }
        nodes
    }

    /// A game of a graft with a `graph` ([`Self::graft_online_challenge`]): the challenge, its
    /// global data and local data with each value of Skate 3's own that Skate 2's classes read
    /// alike, Skate 3's definitions joining where Skate 2's class lacks the field and neither of
    /// the other two defines it. Each value goes where the challenge system's data providers look
    /// for it first (`GetFieldProviderFromDataProviders`: the challenge, then its global and its
    /// local data), so a field Skate 3 moved reads as Skate 2's class holds it. The records the
    /// values name join too ([`Self::import_record`]); Skate 3's state graph `graph` is the
    /// challenge's states ([`Self::import_state_graph`]); and over all of it go the nodes Skate
    /// 2's systems read of the challenge (`identity`) and of its global data (`global_nodes`).
    #[expect(clippy::too_many_arguments, reason = "the graft, the game and the nodes the other grafts take too")]
    fn graft_graph_game(
        &self,
        db: &mut AttribDb,
        graft: &OnlineGraft,
        graph: &str,
        spot: &OnlineSpot,
        records: &GraphGame,
        global_nodes: Vec<(u64, ForeignNode)>,
        identity: Vec<(u64, ForeignNode)>,
        local_reference: Option<ForeignNode>,
    ) -> bool {
        let source = &*self.db;
        let (global_class, template_global) = records.global;
        let (local_class, local) = records.local;
        let providers = [CHALLENGES, global_class, local_class];
        for (index, &class) in providers.iter().enumerate() {
            let others: Vec<u64> = providers.iter().enumerate().filter(|&(other, _)| other != index).map(|(_, &other)| other).collect();
            db.merge_definitions(source, class, &others);
        }
        let mut synthesis = Synthesis::default();
        let mut made: Vec<(u64, u64, usize)> = Vec::new();
        let mut named = Vec::new();
        for (class, collection) in [(CHALLENGES, spot.key), records.source_global, (local_class, local)] {
            for key in source.definition_keys(class) {
                if [GLOBAL_DATA, LOCAL_DATA, STATES].contains(&key) || made.iter().any(|&(made_key, ..)| made_key == key) {
                    continue;
                }
                let Some(target) = providers.iter().copied().find(|&provider| db.definition(provider, key).is_some()) else { continue };
                if !db.defines_alike(target, source, class, key) {
                    continue;
                }
                let Some(index) = source.copy_value(class, collection, key, &mut synthesis) else { continue };
                made.push((key, target, index));
                named.extend(self.named_records(class, collection, key));
            }
        }
        let Some(states) = self.import_state_graph(db, graph) else {
            eprintln!("skate 3 data: {}'s state graph {graph} cannot join", spot.name);
            return false;
        };
        let Some(states_size) = db.definition(CHALLENGES, STATES).map(|definition| definition.size) else { return false };
        let references: Vec<Vec<u8>> = states.iter().map(|&key| record_reference(class::CHALLENGE_STATES, key)).collect();
        let elements: Vec<Element<'_>> = references.iter().map(|reference| Element::Bytes(reference.as_slice())).collect();
        let states_node = synthesis.array(states_size, &elements);
        let nodes = db.synthesize(synthesis);
        for (class, key) in named {
            self.import_record(db, class, key, GRAPH_RECORD_DEPTH);
        }
        let placed = |target: u64| -> Vec<(u64, ForeignNode)> { made.iter().filter(|&&(_, at, _)| at == target).map(|&(key, _, index)| (key, nodes[index])).collect() };
        let mut ours = placed(global_class);
        ours.extend(global_nodes);
        let drop: Vec<u64> = ours.iter().map(|&(key, _)| key).collect();
        let (_, global) = records.source_global;
        if !db.has_collection(global_class, global) && !db.graft(global_class, global, template_global, &drop, &ours) {
            eprintln!("skate 3 data: {}'s global data cannot join", spot.name);
            return false;
        }
        let (Some((_, template_local)), Some(local_reference)) =
            (db.attribute(CHALLENGES, records.template, LOCAL_DATA).and_then(|value| value.ref_spec(0)), local_reference)
        else {
            eprintln!("skate 3 data: {} has no local data to join", spot.name);
            return false;
        };
        let ours = placed(local_class);
        let drop: Vec<u64> = ours.iter().map(|&(key, _)| key).collect();
        if !db.has_collection(local_class, local) && !db.graft(local_class, local, template_local, &drop, &ours) {
            eprintln!("skate 3 data: {}'s local data cannot join", spot.name);
            return false;
        }
        let mut ours = placed(CHALLENGES);
        ours.extend(identity);
        ours.push((STATES, nodes[states_node]));
        ours.push((LOCAL_DATA, local_reference));
        let drop: Vec<u64> = ours.iter().map(|&(key, _)| key).chain(graft.fields.iter().map(|&(field, _)| field)).collect();
        db.graft(CHALLENGES, spot.key, records.template, &drop, &ours)
    }

    /// The records a Skate 3 value names (`Attrib::RefSpec` and the record types a graft joins),
    /// by class and collection key.
    fn named_records(&self, class: u64, collection: u64, key: u64) -> Vec<(u64, u64)> {
        let source = &*self.db;
        let Some(value) = source.attribute(class, collection, key) else { return Vec::new() };
        if !value.type_name().is_some_and(|name| name == "Attrib::RefSpec" || RECORD_TYPES.contains(&name)) {
            return Vec::new();
        }
        (0..value.len()).filter_map(|index| value.ref_spec(index)).filter(|&(class, key)| class != 0 && key != 0).collect()
    }

    /// A Skate 3 record `db` lacks, made in it with each of its values (through its parents and
    /// layout) that Skate 2's class reads alike (an objective's activation mode in Skate 2's
    /// numbering, [`skate2_activation_mode`]), Skate 3's definitions joining where the class
    /// lacks them; and, `depth` deep, the records those values name.
    fn import_record(&self, db: &mut AttribDb, class: u64, key: u64, depth: u32) {
        let source = &*self.db;
        if depth == 0 || db.has_collection(class, key) || !source.has_collection(class, key) {
            return;
        }
        db.merge_definitions(source, class, &[]);
        let mut synthesis = Synthesis::default();
        let mut made = Vec::new();
        let mut named = Vec::new();
        for field in source.definition_keys(class) {
            if !db.defines_alike(class, source, class, field) {
                continue;
            }
            let copied = if class == class::CHALLENGE_OBJECTIVE && field == ACTIVATION_MODE {
                source.attribute(class, key, field).and_then(|value| {
                    let mode = skate2_activation_mode(value.i32(0)?).to_be_bytes();
                    Some(if value.is_inline() { synthesis.word(mode) } else { synthesis.value(&mode, &[]) })
                })
            } else {
                source.copy_value(class, key, field, &mut synthesis)
            };
            if let Some(index) = copied {
                made.push((field, index));
                named.extend(self.named_records(class, key, field));
            }
        }
        let nodes = db.synthesize(synthesis);
        let ours: Vec<(u64, ForeignNode)> = made.into_iter().map(|(field, index)| (field, nodes[index])).collect();
        db.create(class, key, &ours);
        for (class, key) in named {
            self.import_record(db, class, key, depth - 1);
        }
    }

    /// Skate 3's state graph `graph` in `db`, once: each of its states and their message handlers
    /// under a key of its own (`skate3/<graph>/<key>`: Skate 3's 1-Up graph keeps some of Skate 2's
    /// handler keys), with its name, message kind and chunk, the chunk's `h_<key>` renamed to the
    /// new key. The states' keys in the graph's order; `None` without the graph.
    fn import_state_graph(&self, db: &mut AttribDb, graph: &str) -> Option<Vec<u64>> {
        use crate::challenge::runtime::field;
        let source = &*self.db;
        let remap = |key: u64| hash64(&format!("skate3/{graph}/{key:016x}"));
        let keys = |class, collection, field| {
            source.attribute(class, collection, field).map_or_else(Vec::new, |value| (0..value.len()).filter_map(|index| value.ref_spec(index)).map(|(_, key)| key).collect::<Vec<_>>())
        };
        let states = keys(STATE_GRAPHS, hash64(graph), field::STATES);
        let ours: Vec<u64> = states.iter().map(|&state| remap(state)).collect();
        let first = *ours.first()?;
        if db.has_collection(class::CHALLENGE_STATES, first) {
            return Some(ours);
        }
        db.merge_definitions(source, class::CHALLENGE_STATES, &[]);
        db.merge_definitions(source, class::CHALLENGE_MESSAGE_HANDLERS, &[]);
        let handlers_size = db.definition(class::CHALLENGE_STATES, field::STATE_HANDLERS)?.size;
        for (&state, &our_state) in states.iter().zip(&ours) {
            let name = source.attribute(class::CHALLENGE_STATES, state, field::NAME)?.text(0)?.to_owned();
            let mut synthesis = Synthesis::default();
            let mut handlers = Vec::new();
            for handler in keys(class::CHALLENGE_STATES, state, field::STATE_HANDLERS) {
                let our = remap(handler);
                let attribute = |field| source.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler, field);
                let handler_name = attribute(field::NAME).and_then(|value| value.text(0)).unwrap_or_default().to_owned();
                let kind = attribute(field::HANDLER_KIND)?.u32(0)?;
                let chunk = renamed_chunk(source.blob_of(class::CHALLENGE_MESSAGE_HANDLERS, handler, field::HANDLER_BLOB)?, handler, our);
                handlers.push((our, synthesis.text_value(&handler_name), synthesis.word(kind.to_be_bytes()), synthesis.blob(&chunk)));
            }
            let references: Vec<Vec<u8>> = handlers.iter().map(|&(our, ..)| record_reference(class::CHALLENGE_MESSAGE_HANDLERS, our)).collect();
            let elements: Vec<Element<'_>> = references.iter().map(|reference| Element::Bytes(reference.as_slice())).collect();
            let list = synthesis.array(handlers_size, &elements);
            let state_name = synthesis.text_value(&name);
            let nodes = db.synthesize(synthesis);
            for &(our, name, kind, chunk) in &handlers {
                db.create(class::CHALLENGE_MESSAGE_HANDLERS, our, &[(field::NAME, nodes[name]), (field::HANDLER_KIND, nodes[kind]), (field::HANDLER_BLOB, nodes[chunk])]);
            }
            db.create(class::CHALLENGE_STATES, our_state, &[(field::NAME, nodes[state_name]), (field::STATE_HANDLERS, nodes[list])]);
        }
        Some(ours)
    }

    /// The records a Skate 3 value (`class`, `collection`, `field`) names ([`RECORD_TYPES`]) that
    /// `db` lacks, joined to it from Skate 3's own, each on a record of its class in `db` (its
    /// parent's defaults under Skate 3's own fields), and the records those name in turn; false
    /// when one cannot join.
    fn graft_records(&self, db: &mut AttribDb, adopted: &HashMap<u16, u16>, class: u64, collection: u64, field: u64, depth: u32) -> bool {
        let source = &*self.db;
        let Some(value) = source.attribute(class, collection, field) else { return true };
        if !value.type_name().is_some_and(|name| RECORD_TYPES.contains(&name)) {
            return true;
        }
        let inherited = value.type_name() == Some(GATE_RECORD_TYPE);
        let named: Vec<(u64, u64)> = (0..value.len())
            .filter_map(|index| {
                let bytes = value.value_bytes(index)?;
                let class = u64::from_be_bytes(bytes.get(..8)?.try_into().ok()?);
                let key = u64::from_be_bytes(bytes.get(8..16)?.try_into().ok()?);
                Some((class, key))
            })
            .collect();
        for (record_class, key) in named {
            if db.has_collection(record_class, key) {
                continue;
            }
            if depth == 0 || !source.has_collection(record_class, key) {
                return false;
            }
            let Some(&template) = db.leaf_collections(record_class).first() else { return false };
            let fields = if inherited { source.inherited_keys(record_class, key) } else { source.own_keys(record_class, key) };
            let mut nodes = Vec::with_capacity(fields.len());
            for &inner in &fields {
                if !self.graft_records(db, adopted, record_class, key, inner, depth - 1) {
                    return false;
                }
                let named = RENAMED_RECORD_FIELDS
                    .iter()
                    .find(|&&(class, skate3, _)| class == record_class && skate3 == inner)
                    .map_or(inner, |&(_, _, skate2)| skate2);
                match source.foreign_node(record_class, key, inner, adopted) {
                    Some(found) if !found.is_layout() => nodes.push((named, found)),
                    Some(_) => return false,
                    None => {}
                }
            }
            let drop = db.own_keys(record_class, template);
            if !db.graft(record_class, key, template, &drop, &nodes) {
                return false;
            }
        }
        true
    }

    /// Skate 3's global locators' transforms by `GetHashValue32(name)`, which a Skate 3 game's
    /// challenge finds its spawn points and waiting locations among.
    #[must_use]
    pub fn locator_transforms(&self) -> &HashMap<u32, [f32; 16]> {
        &self.locators
    }

    /// The online games of a network challenge type at Skate 3's districts, sorted by title,
    /// but those no graft could join.
    #[must_use]
    pub fn online_spots(&self, net_type: u8) -> Vec<&OnlineSpot> {
        let unplayable = UNPLAYABLE.lock().map(|list| list.clone()).unwrap_or_default();
        self.online.iter().filter(|spot| spot.net_type == net_type && !unplayable.contains(&spot.key)).collect()
    }

    /// PC port: Skate 3's scene directory (`miscboot.big`) and ICE cameras (`scene.big`), the
    /// scenes a Skate 3 game's presentation names at its own spots.
    ///
    /// # Errors
    ///
    /// A file that cannot be read or parsed.
    pub fn scenes(&self) -> Result<(crate::nis::directory::SceneDirectory, skate2_formats::ice_camera::CameraSet), String> {
        let path = crate::nis::directory::PATH;
        let directory = self.mounts.read_path(path).map_err(|e| format!("{path}: {e}"))?;
        let directory = crate::nis::directory::SceneDirectory::parse(&directory).map_err(|e| format!("{path}: {e}"))?;
        let scenes = open(&archive(&self.root, SCENE_ARCHIVE)).map_err(|e| e.to_string())?;
        let path = crate::nis::camera::PATH;
        let cameras = scenes.read_path(path).map_err(|e| format!("{path}: {e}"))?;
        let cameras = skate2_formats::ice_camera::CameraSet::parse_skate3(&cameras).map_err(|e| format!("{path}: {e}"))?;
        Ok((directory, cameras))
    }

    /// The Skate 3 online game with this `challenges` key.
    #[must_use]
    pub fn online_spot(&self, key: u64) -> Option<&OnlineSpot> {
        self.online.iter().find(|spot| spot.key == key)
    }

    /// Whether the Skate 3 online game with this `challenges` key runs Skate 3's own state graph
    /// (a graft with a `graph`): its scripts read every player's objectives and the facts whole.
    #[must_use]
    pub fn runs_own_graph(&self, key: u64) -> bool {
        self.online_spot(key)
            .is_some_and(|spot| ONLINE_GRAFTS.iter().any(|graft| graft.graph.is_some() && net_challenge_type(graft.game_type) == Some(spot.net_type)))
    }

    /// The districts' stream archives, opened again for the game's mounts.
    ///
    /// # Errors
    ///
    /// [`Skate3Error::Read`] when one can no longer be opened.
    /// The archives that join the game's mounts: the districts' worlds, Skate 3's challenge
    /// hulls, which an online S.K.A.T.E. round streams in, and its clothing's models and
    /// textures, which a skater wearing it loads. The game's own files come first.
    pub fn world_archives(&self) -> Result<Vec<MountedBig>, Skate3Error> {
        let mut archives: Vec<MountedBig> = DISTRICTS.iter().map(|spec| open(&district_archive(&self.root, spec))).collect::<Result<_, _>>()?;
        for spec in &DISTRICTS {
            archives.push(open(&archive(&self.root, &format!("data\\content\\proxy{}.big", spec.proxy)))?);
        }
        for optional in [MISSIONS_ARCHIVE, CLOTHING_ARCHIVE] {
            let path = archive(&self.root, optional);
            if skate2_formats::host_path::is_file(&skate2_formats::host_path::on_disk(&path)) {
                archives.push(open(&path)?);
            }
        }
        // The packs' worlds, challenge hulls and models ([`dlc::game_view`]), and the disc's worlds
        // their unlocks open.
        archives.extend(self.pack_archives.iter().map(dlc::game_view));
        for path in &self.disc_worlds {
            archives.push(open(path)?);
        }
        Ok(archives)
    }

    #[must_use]
    pub fn district(&self, stream: &str) -> Option<&District> {
        self.districts.iter().find(|district| district.spec.stream.eq_ignore_ascii_case(stream))
    }

    /// The spot with this `fe_locations` key, and its district.
    #[must_use]
    pub fn spot(&self, key: u64) -> Option<(&District, &Spot)> {
        self.districts.iter().find_map(|district| district.spots.iter().find(|spot| spot.key == key).map(|spot| (district, spot)))
    }

    /// Skate 3's English text of a string id.
    #[must_use]
    pub fn text(&self, id: &str) -> Option<&str> {
        self.strings.get(&skate2_formats::localization::string_hash(id.as_bytes())).map(String::as_str)
    }
}

/// The challenge bank's leaf `challenges` flagged online, of a type a Skate 3 map hosts
/// ([`ONLINE_GAME_TYPES`]), at a global locator of a district's own world (not a park's, the
/// skate school's or the downloadable content's), sorted by their English titles; test challenges
/// (titles beginning `###`) are left out.
fn online_spots(
    db: &AttribDb,
    text: &dyn Fn(&str) -> Option<String>,
    locators: &HashMap<u32, [f32; 16]>,
    districts_of: &HashMap<u32, usize>,
    districts: &[District],
) -> Vec<OnlineSpot> {
    let mut spots: Vec<OnlineSpot> = db
        .leaf_collections(CHALLENGES)
        .iter()
        .filter_map(|&key| {
            let (class, global) = db.attribute(CHALLENGES, key, GLOBAL_DATA)?.ref_spec(0)?;
            if db.attribute(class, global, ONLINE).and_then(|flag| flag.bool(0)) != Some(true) {
                return None;
            }
            let game_type = db.attribute(class, global, CHALLENGE_TYPE).and_then(|value| value.i32(0))?;
            if !ONLINE_GAME_TYPES.contains(&game_type) {
                return None;
            }
            let net_type = net_challenge_type(game_type)?;
            let location = db.attribute(class, global, LOCATION).and_then(|value| value.text(0))?.to_owned();
            let hash = hash_value32(&location);
            let district = districts.get(*districts_of.get(&hash)?)?;
            let transform = *locators.get(&hash)?;
            let title_id = db.attribute(class, global, TITLE).and_then(|value| value.text(0)).unwrap_or_default();
            if title_id.starts_with("###") {
                return None;
            }
            let name = db.attribute(CHALLENGES, key, CHALLENGE_NAME).and_then(|value| value.text(0))?.to_owned();
            let title = text(title_id).filter(|title| !title.is_empty()).unwrap_or_else(|| name.clone());
            Some(OnlineSpot { key, net_type, name, title, location, stream: district.spec.stream, render_location: district.render_location, transform })
        })
        .collect();
    // A pack's Vault may hold one of Skate 3's own challenges again: one game each.
    spots.sort_by_key(|spot| spot.key);
    spots.dedup_by_key(|spot| spot.key);
    spots.sort_by(|a, b| a.title.to_ascii_lowercase().cmp(&b.title.to_ascii_lowercase()));
    spots
}

/// The district of a world stream name, linked or not, or a map downloadable content added this
/// run ([`kept_spec`]).
#[must_use]
pub fn district_spec(stream: &str) -> Option<&'static DistrictSpec> {
    DISTRICTS.iter().find(|spec| spec.stream.eq_ignore_ascii_case(stream)).or_else(|| {
        MAP_SPECS.lock().unwrap_or_else(PoisonError::into_inner).iter().copied().find(|spec| spec.stream.eq_ignore_ascii_case(stream))
    })
}

/// Every world [`district_spec`] knows: the districts, then the maps downloadable content added
/// this run.
#[must_use]
pub fn district_specs() -> Vec<&'static DistrictSpec> {
    let maps = MAP_SPECS.lock().unwrap_or_else(PoisonError::into_inner).clone();
    DISTRICTS.iter().chain(maps).collect()
}

/// The specs of the maps downloadable content added, each kept for the run once made
/// ([`kept_spec`]): what [`district_spec`] finds beside [`DISTRICTS`].
static MAP_SPECS: Mutex<Vec<&'static DistrictSpec>> = Mutex::new(Vec::new());

/// A text kept for the run, made once.
fn kept(text: &str) -> &'static str {
    static KEPT: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&found) = kept.iter().find(|&&held| held == text) {
        return found;
    }
    let made: &'static str = Box::leak(text.to_owned().into_boxed_str());
    kept.push(made);
    made
}

/// A downloadable content map's spec, kept for the run: the one made before for the same world,
/// else a new one. It has no `render_locations` name ([`District::render_location`] holds its key)
/// and no proxy.
fn kept_spec(key: &str, world: &str, stream: &str, title: &str, loose: &[String]) -> &'static DistrictSpec {
    let mut specs = MAP_SPECS.lock().unwrap_or_else(PoisonError::into_inner);
    let same = |spec: &&&'static DistrictSpec| {
        spec.key == key
            && spec.world == world
            && spec.stream == stream
            && spec.title == title
            && spec.loose.len() == loose.len()
            && spec.loose.iter().zip(loose).all(|(held, path)| *held == path.as_str())
    };
    if let Some(&found) = specs.iter().find(same) {
        return found;
    }
    let loose: Vec<&'static str> = loose.iter().map(|path| kept(path)).collect();
    let spec: &'static DistrictSpec = Box::leak(Box::new(DistrictSpec {
        key: kept(key),
        world: kept(world),
        stream: kept(stream),
        render_location: "",
        proxy: "",
        title: kept(title),
        loose: Box::leak(loose.into_boxed_slice()),
    }));
    specs.push(spec);
    spec
}

/// How many linked areas (the districts, then the downloadable content's maps) the challenge
/// maps' downloaded content lists.
pub const AREA_SLOTS: usize = 12;

/// The challenge maps' label of linked area `index` (a district, then a downloadable content
/// map, as [`Skate3Data::districts`] holds them): `#<title> (%d)`, its count of teleports at `%d`.
#[must_use]
pub fn area_label(index: usize) -> &'static str {
    let title = linked()
        .and_then(|data| data.districts.get(index).map(|district| district.spec.title))
        .or_else(|| DISTRICTS.get(index).map(|spec| spec.title))
        .unwrap_or_default();
    kept(&format!("#{title} (%d)"))
}

/// Whether a linked area is one of Port Carverton's districts, not a downloadable content map.
#[must_use]
pub fn is_base_district(district: &District) -> bool {
    DISTRICTS.iter().any(|spec| spec.key == district.spec.key)
}

/// Whether a world stream is one of Port Carverton's districts', not a downloadable content map's.
#[must_use]
pub fn is_district_stream(stream: &str) -> bool {
    DISTRICTS.iter().any(|spec| spec.stream.eq_ignore_ascii_case(stream))
}

/// PC port: a linked Skate 3 world's stream XML of type `suffix` (`Pres` or `Sim`), which Skate
/// 3's `cStreamFile::LoadStreamXml` (0x8247dbe0 in its title update 3) reads as
/// `data\stream\<world>_<type>.xml` by the world record's `Name` (`dist_industrial`,
/// `dist_artgallerydlc`; `cGameAssetManager::LoadGameStreamFile` 0x82477c40 keeps it), from Skate
/// 3's own archives (`miscload.big`) or the world's downloadable content package. `None` for a
/// world that ships none (Black Box Park), for a world not Skate 3's, or unlinked.
#[must_use]
pub fn world_stream_xml(stream: &str, suffix: &str) -> Option<Vec<u8>> {
    let spec = district_spec(stream)?;
    linked()?.mounts.read_path(&format!("data\\stream\\{}_{suffix}.xml", spec.world)).ok()
}

/// PC port: the records a linked Skate 3 world's stream file of type `suffix` selects from
/// (`TileRecords`), with the world's stream XML of that type for Pres and Sim
/// (`cGameAssetManager::UpdateGameStreamFileLoaded` 0x82477da8 loads none for textures). A
/// malformed XML is reported and left out, as is one the world does not ship.
#[must_use]
pub fn world_records(stream: &str, space: &skate2_formats::stream::CollectionSpace, suffix: &str) -> skate2_formats::stream_xml::TileRecords {
    let mut records = skate2_formats::stream_xml::TileRecords::new(space, suffix);
    if suffix.eq_ignore_ascii_case("Tex") {
        return records;
    }
    match world_stream_xml(stream, suffix).map(|bytes| records.apply_stream_xml(&bytes)) {
        Some(Ok(())) => eprintln!("skate 3 world {stream}: {suffix} selects through its stream XML"),
        Some(Err(error)) => eprintln!("skate 3 world {stream}: {suffix} stream XML: {error}"),
        None => {}
    }
    records
}

/// Whether a world (`fe_locations.WorldFile`, an additional world's stream) is one of Skate 3's
/// districts, which stand in place of San Vanelona: while one is loaded the city's collections
/// leave the simulation, presentation and texture selections, its proxies are not drawn and its
/// living world and boundaries stand down.
#[must_use]
pub fn is_district_world(stream: &str) -> bool {
    district_spec(stream).is_some()
}

static LINKED: RwLock<Option<Arc<Skate3Data>>> = RwLock::new(None);

/// PC port: the scenes a Skate 3 game took from Skate 3's directory ([`Skate3Data::scenes`]).
static SCENES: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// PC port: the Skate 3 scenes a Skate 3 game plays at its spots.
pub fn set_scenes(names: Vec<String>) {
    *SCENES.write().unwrap_or_else(PoisonError::into_inner) = names;
}

/// PC port: whether `name` is one of the Skate 3 scenes [`set_scenes`] holds.
#[must_use]
pub fn is_scene(name: &str) -> bool {
    SCENES.read().unwrap_or_else(PoisonError::into_inner).iter().any(|scene| scene.eq_ignore_ascii_case(name))
}
static DISTRICT_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether a Skate 3 district is loaded in San Vanelona's place: the WorldPainter paints no
/// region (Skate 3's regions name its own palette, which its data holds), and Skate 2's
/// pedestrians and traffic take the default census over the district's own roads.
#[must_use]
pub fn district_active() -> bool {
    DISTRICT_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The skater's bone a linked Skate 3 clothing model's helper bone follows. Skate 3's rig adds a
/// helper at each shoulder and hip joint (`LeftShoulderHlp`, `LeftUpLegHlp` and the right ones),
/// which its character animators set each frame from the arm or thigh beside its twist bones
/// (`CASCharacterAnimator::Init` 0x82533ca8, `SkaterPresentation::InitAnimation` 0x82787ea8 list
/// them together); Skate 2's hierarchy has none. Skate 3's shirts and arms skin the shoulder's
/// vertices to one and its shorts and legs the hip's, so on the hips (the bone a model's other
/// unknown bones take) those vertices would stay with the hips as the arm, spine or thigh moved.
/// Each follows its limb's bone instead, with that bone's own bind pose.
/// `SKATE2_DIAGNOSTIC_S3_HELPER_PARENT` gives each the joint's parent side instead (the clavicle,
/// the hips), for comparison.
#[must_use]
pub fn helper_bone_stand_in(bone: &str) -> Option<&'static str> {
    let parent = std::env::var_os("SKATE2_DIAGNOSTIC_S3_HELPER_PARENT").is_some();
    HELPER_BONES
        .iter()
        .find(|(helper, _, _)| helper.eq_ignore_ascii_case(bone))
        .map(|&(_, limb, joint_parent)| if parent { joint_parent } else { limb })
}

/// Skate 3's helper bones (`helper_bone_stand_in`): each, its limb's bone and the joint's parent
/// side.
const HELPER_BONES: [(&str, &str, &str); 4] = [
    ("LeftShoulderHlp", "LeftArm", "LeftShoulder"),
    ("RightShoulderHlp", "RightArm", "RightShoulder"),
    ("LeftUpLegHlp", "LeftUpLeg", "Hips"),
    ("RightUpLegHlp", "RightUpLeg", "Hips"),
];

/// Whether a model bone is one of Skate 3's helper bones, which only a model made on Skate 3's rig
/// names (Skate 2's has none).
#[must_use]
pub fn is_helper_bone(bone: &str) -> bool {
    HELPER_BONES.iter().any(|(helper, _, _)| helper.eq_ignore_ascii_case(bone))
}

/// A linked Skate 3 clothing model's bone the skater's hierarchy lacks, reported once each, with
/// the hierarchy's own the first time.
pub fn note_unmatched_bone(bone: &str, hierarchy: &[String]) {
    static NOTED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut noted) = NOTED.lock()
        && !noted.iter().any(|seen| seen == bone)
    {
        if noted.is_empty() {
            eprintln!("skate 3 clothing: the skater's bones are {hierarchy:?}");
        }
        eprintln!("skate 3 clothing: bone {bone} is not the skater's; it follows the hips");
        noted.push(bone.to_owned());
    }
}

/// A linked Skate 3 clothing material without a map its Skate 2 program samples, reported once
/// for each effect and map.
pub fn note_missing_cas_channel(effect: &str, channel: &str) {
    static NOTED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let key = format!("{effect} {channel}");
    if let Ok(mut noted) = NOTED.lock()
        && !noted.contains(&key)
    {
        eprintln!("skate 3 clothing: a {effect} material has no {channel} map; drawn without one");
        noted.push(key);
    }
}

/// The session's world this frame.
pub fn set_district_active(active: bool) {
    DISTRICT_ACTIVE.store(active, std::sync::atomic::Ordering::Relaxed);
}

/// A native streaming table's fixed capacity, four times over while Skate 3 data is linked: a
/// district's collections join San Vanelona's registered ones (Industrial's Sim stream alone holds
/// 1215), past the tables Skate 2 sized for its own city.
#[must_use]
pub fn stream_capacity(native: usize) -> usize {
    if linked().is_some() { native * 4 } else { native }
}

/// The `material_character` collections Skate 3's clothing models name that Skate 2 lacks, with
/// the Skate 2 collection each reads as: leather, stamped and alpha cloth as `cloth`, stamped
/// cloth simulation as `cloth_ropa`, and the board default as `default_cloth`.
const CHARACTER_MATERIAL_ALIASES: [(&str, &str); 5] =
    [("leather", "cloth"), ("cloth_stamp", "cloth"), ("alpha", "cloth"), ("cloth_stamp_ropa", "cloth_ropa"), ("default_skateboard", "default_cloth")];

/// The Skate 2 `material_character` collection a linked Skate 3 data folder's clothing material
/// `(class, collection)` reads as ([`CHARACTER_MATERIAL_ALIASES`]); `None` unlinked or for any
/// other.
#[must_use]
pub fn character_material_alias(class: u64, collection: u64) -> Option<u64> {
    use crate::challenge::keys::hash64;
    if class != hash64("material_character") || linked().is_none() {
        return None;
    }
    CHARACTER_MATERIAL_ALIASES.iter().find(|(skate3, _)| hash64(skate3) == collection).map(|(_, skate2)| hash64(skate2))
}

/// A linked Skate 3 data folder's cloth effect at `path` (`data\ropa2\0x<guid>.r2b`), which
/// Skate 3 keeps in `miscload.big`, outside the archives that join the game's mounts; `None`
/// unlinked or when Skate 3 has none.
#[must_use]
pub fn cloth_effect(path: &str) -> Option<Vec<u8>> {
    // SKATE2_DIAGNOSTIC_S3_NO_CLOTH: Skate 3's clothing skins as plain models, with no cloth.
    if std::env::var_os("SKATE2_DIAGNOSTIC_S3_NO_CLOTH").is_some() {
        return None;
    }
    linked()?.mounts.read_path(path).ok()
}

/// Records a Skate 3 clothing morph target no Skate 2 body zone matches, logged once per name.
pub fn note_unmatched_morph(name: &str) {
    static NOTED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut noted) = NOTED.lock()
        && !noted.iter().any(|known| known == name)
    {
        eprintln!("skate 3: clothing morph target {name} has no Skate 2 body zone; it keeps the base shape");
        noted.push(name.to_owned());
    }
}

/// Records an effect a Skate 3 material names that Skate 2 ships no manifest for, logged once per
/// name; true: the material defers rather than failing the render queue.
pub fn note_missing_effect(name: &str) -> bool {
    static NOTED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut noted) = NOTED.lock()
        && !noted.iter().any(|known| known == name)
    {
        eprintln!("skate 3: effect {name} has no Skate 2 manifest; its materials defer");
        noted.push(name.to_owned());
    }
    true
}

/// The top 24 bits every Skate 3 instance id (trigger volume, spline, locator) shares: the next
/// byte is the id's kind, the low word a running number below `2^23`.
const ID_PREFIX: u64 = 0x2c_7017;

/// Whether an instance id is one of Skate 3's.
#[must_use]
pub const fn is_id(id: u64) -> bool {
    id >> 40 == ID_PREFIX
}

/// A 64-bit id as Lua and the float lookups take it: `(float)(double)id`, as
/// `GetTriggerVolumeFromID` (`0x82e21b78`) compares. Every Skate 3 id rounds to the same float
/// that way; Skate 3 itself hands ids to Lua whole (`uint64_t` userdata, `0x82925428`) and finds
/// volumes by all 64 bits (`0x82dd7818`). A Skate 3 id is its running number in the binade of its
/// kind instead, which a float holds exactly, apart from every other.
#[must_use]
#[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation, reason = "(float)_u64tod")]
pub fn id_float(id: u64) -> f32 {
    let (kind, number) = ((id >> 32) as u32 & 0xff, id as u32);
    if is_id(id) && number < 1 << 23 && kind < 64 {
        return f32::from_bits((127 + 23 + kind) << 23 | number);
    }
    id as f64 as f32
}

/// The Skate 3 data the player linked, if any.
#[must_use]
pub fn linked() -> Option<Arc<Skate3Data>> {
    LINKED.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Whether Skate 3 data is linked, without the lock [`linked`] takes: what a frame reads.
static IS_LINKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[must_use]
pub fn is_linked() -> bool {
    IS_LINKED.load(std::sync::atomic::Ordering::Relaxed)
}

/// How many times the host linked or unlinked Skate 3 data: a holder of what it read compares it
/// to read the data again.
static LINK_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[must_use]
pub fn link_generation() -> u64 {
    LINK_GENERATION.load(std::sync::atomic::Ordering::Acquire)
}

/// The host linked (or unlinked) Skate 3 data.
pub fn set_linked(data: Option<Arc<Skate3Data>>) {
    IS_LINKED.store(data.is_some(), std::sync::atomic::Ordering::Relaxed);
    *LINKED.write().unwrap_or_else(PoisonError::into_inner) = data;
    LINK_GENERATION.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
}

/// The pack row the Downloadable Content screen lists for linked Skate 3 data.
pub const PACK_TITLE: &str = "Skate 3";
pub const PACK_DESCRIPTION: &str = "Port Carverton's University, Downtown and Industrial districts with their challenge map teleports, and Skate 3's clothing in Edit Skater's Merchandise. Skate 3's downloadable content links through Link DLCs.";

/// `challenge_objective` `ActivationMode` (`eTriggerActivationMode`).
const ACTIVATION_MODE: u64 = crate::challenge::objectives::field::ACTIVATION_MODE;

/// A Skate 3 objective's activation mode in Skate 2's numbering. Skate 3 put a mode of its own at
/// 1, the objectives `Objectives.ActivateChallengeStartObjectives` (TU3 0x82c61158) activates, so
/// its 2 to 5 are Skate 2's 1 to 4: TU3's `cChallengeObjective::TriggerEnterMsg` (0x82c74290)
/// tests 2 and 3, `TriggerExitMsg` (0x82c74450) 2, `WipeoutMsg` (0x82c71bd8) and `OnWipeoutOver`
/// (0x82c6d890) 4, and `OnSequenceBegin` (0x82c6da68) and `OnSequenceComplete` (0x82c6db50) 5,
/// where Skate 2's test 1, 2, 1, 3 and 4. Skate 3's own mode becomes
/// [`crate::challenge::objectives::CHALLENGE_START_MODE`], which no Skate 2 objective has.
const fn skate2_activation_mode(mode: i32) -> i32 {
    match mode {
        0 => 0,
        1 => crate::challenge::objectives::CHALLENGE_START_MODE,
        mode => mode - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_districts_name_their_streams() {
        assert!(is_district_world("dist_university"));
        assert!(is_district_world("DIST_DownTown"));
        assert!(!is_district_world("BAM"));
        assert_eq!(district_spec("DIST_Industrial").map(|spec| spec.title), Some("Industrial District"));
    }

    /// Skate 3's ids, which one float holds under `(float)(double)` (skate_indu_01's spot volumes
    /// and a spline indicator of the same running number), stay apart; Skate 2's hashes convert
    /// as the native does.
    #[test]
    #[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::float_cmp, reason = "the native conversion")]
    fn skate3_ids_stay_apart_as_floats() {
        let ids = [0x2c70_1706_0025_02e9_u64, 0x2c70_1706_0025_02ea, 0x2c70_1707_0025_02e9];
        assert_eq!(ids[0] as f64 as f32, ids[1] as f64 as f32);
        let floats = ids.map(id_float);
        assert!(floats[0] != floats[1] && floats[0] != floats[2] && floats[1] != floats[2]);
        assert_eq!(floats[0], 2_425_577.0 * 64.0 + 8_388_608.0 * 64.0);
        let hash = 0x9e37_79b9_7f4a_7c15_u64;
        assert!(!is_id(hash));
        assert_eq!(id_float(hash), hash as f64 as f32);
    }

    /// `SKATE3_DATA` names a Skate 3 folder: its districts' teleports read from its own data.
    #[test]
    fn a_skate3_folder_lists_its_districts_teleports() {
        let Some(root) = std::env::var_os("SKATE3_DATA") else { return };
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        for district in &data.districts {
            assert!(!district.spots.is_empty(), "{} has teleports", district.spec.title);
            assert_ne!(district.render_location, 0, "{} has a render location", district.spec.title);
        }
        let university = data.district("DIST_University").unwrap();
        assert!(university.spots.iter().any(|spot| spot.locator == "Z_UN_ClockTower"));
    }

    /// With `SKATE3_DATA` and `SKATE2_GAME_ROOT`, Skate 3's Spot Battles join Skate 2's challenge
    /// database with their own scoring: Library Exit scores one line, Stadium Bank two, Megapark
    /// banks every line for 65 seconds, each over its own spot volumes.
    #[test]
    fn skate3_spot_battles_join_with_their_scoring() {
        let (Some(root), Some(_)) = (std::env::var_os("SKATE3_DATA"), std::env::var_os("SKATE2_GAME_ROOT")) else { return };
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        let mut db = skate2_template_db();
        data.graft_online_challenges(&mut db);
        let spot_battle = net_challenge_type(SPOT_BATTLE_GAME_TYPE).unwrap();
        assert!(data.online_spots(spot_battle).len() >= 30, "{} Spot Battles", data.online_spots(spot_battle).len());
        let check = |name: &str, cumulative: bool, sequences: i32, time_limit: Option<f32>| {
            let key = crate::challenge::keys::hash64(name);
            let flag = db.attribute(CHALLENGES, key, CONTEST_CUMULATIVE_SCORING).and_then(|value| value.bool(0));
            assert_eq!(flag, Some(cumulative), "{name} session flag");
            if let Some(limit) = time_limit {
                assert_eq!(db.attribute(CHALLENGES, key, TIME_LIMIT).and_then(|value| value.f32(0)), Some(limit), "{name} time limit");
            }
            let (class, local) = db.attribute(CHALLENGES, key, LOCAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
            assert_eq!(local, key, "{name} reads its own local data");
            assert!(db.has_collection(class, local), "{name}'s local data joined");
            assert_eq!(db.attribute(class, local, NUM_SEQUENCES_TO_END_TURN).and_then(|value| value.i32(0)), Some(sequences), "{name} lines a turn");
            let ours = db.attribute(class, local, LOCAL_SPOT_VOLUMES).and_then(|value| value.value_bytes(0)).unwrap();
            let skate3_local = data.db.attribute(CHALLENGES, key, LOCAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
            let theirs = data.db.attribute(skate3_local.0, skate3_local.1, SPOT_VOLUMES).and_then(|value| value.value_bytes(0)).unwrap();
            assert_eq!(ours, theirs, "{name} tracks its own spot volume");
        };
        check("ots_univ_04_ol", false, 1, None);
        check("ots_univ_02_ol", false, 2, None);
        check("ots_univ_10_ol", true, 1, Some(65.0));
        check("ots_univ_14_ol", true, 1, Some(35.0));
    }

    fn skate2_template_db() -> AttribDb {
        use skate2_formats::mount::{MountedBig, SKATE2_ARCHIVES};
        let game = std::env::var("SKATE2_GAME_ROOT").unwrap();
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\db\\db.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(Path::new(&game)), archive.use_external_header).unwrap());
        AttribDb::load_career(&mounts).unwrap()
    }

    /// With `SKATE3_DATA` and `SKATE2_GAME_ROOT`, a Skate 3 game carries its own title, which
    /// the HUD and the results show from Skate 3's strings, and its type's Skate 2 map icon.
    #[test]
    fn skate3_online_games_keep_their_own_titles() {
        const CHALLENGE_ICON: u64 = 0xe616_acdf_5a36_bfac;
        let (Some(root), Some(_)) = (std::env::var_os("SKATE3_DATA"), std::env::var_os("SKATE2_GAME_ROOT")) else { return };
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        let mut db = skate2_template_db();
        data.graft_online_challenges(&mut db);
        for (name, english, icon) in [("skate_univ_02", "Campus SKATE", "mini_skate"), ("ots_univ_04_ol", "Library Exit", "mini_spotfound")] {
            let key = crate::challenge::keys::hash64(name);
            let title = db.attribute(CHALLENGES, key, TITLE).and_then(|value| value.text(0)).unwrap();
            assert_eq!(data.text(title), Some(english), "{name} title {title}");
            let shown = db
                .attribute(CHALLENGES, key, GLOBAL_DATA)
                .and_then(|global| global.ref_spec(0))
                .and_then(|(class, global)| db.attribute(class, global, CHALLENGE_ICON))
                .and_then(|value| value.ref_spec(0))
                .and_then(|(_, icon)| db.layout_text(crate::challenge::keys::class::MAP_ICON, icon, 0));
            assert_eq!(shown, Some(icon), "{name} icon");
        }
    }

    /// A handler's chunk defines the global of its new key, every occurrence renamed and nothing
    /// else changed.
    #[test]
    fn an_imported_handler_defines_its_own_global() {
        let chunk = b"..h_5D7772E746ED13EC..h_5D7772E746ED13EC..h_0000000000000001";
        let renamed = renamed_chunk(chunk, 0x5d77_72e7_46ed_13ec, 0xabcd_ef01_2345_6789);
        assert_eq!(renamed, b"..h_ABCDEF0123456789..h_ABCDEF0123456789..h_0000000000000001");
    }

    /// With `SKATE3_DATA` and `SKATE2_GAME_ROOT`, Skate 3's online 1-Ups run Skate 3's own 1-Up
    /// state graph: each game's states are the graph's, each handler's chunk defining the global
    /// its new key names; the game's own round starts, waiting spot and time limit read where the
    /// challenge's data providers look first; and the objective its lines score through joins.
    #[test]
    fn skate3_one_ups_run_skate3s_own_graph() {
        use crate::challenge::runtime::{StateGraph, field};
        const START_LOCATION_ROTATION: u64 = 0xd1cc_dcff_223f_ec17;
        const WAIT_LOCATION_ROTATION: u64 = 0x8b76_4557_f3ec_7068;
        const OBJECTIVES: u64 = 0xc4b3_4541_5e85_37be;
        const MULTIPLIER_ACTIVE: u64 = 0xcb83_7eef_5095_8fbb;
        let (Some(root), Some(_)) = (std::env::var_os("SKATE3_DATA"), std::env::var_os("SKATE2_GAME_ROOT")) else { return };
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        let mut db = skate2_template_db();
        data.graft_online_challenges(&mut db);
        let games = data.online_spots(net_challenge_type(ONE_UP_GAME_TYPE).unwrap());
        eprintln!("1-Ups: {:?}", games.iter().map(|game| (&game.name, &game.title)).collect::<Vec<_>>());
        assert!(games.len() >= 9, "Skate 3's online 1-Ups are listed");
        for game in games {
            let graph = StateGraph::gather(&db, game.key);
            let names: Vec<&str> = graph.states.iter().map(|state| state.name.as_str()).collect();
            assert_eq!(names.len(), 21, "{}: {names:?}", game.name);
            for state in ["initial", "setup_round", "setup_run", "countdown", "perform_run", "end_run", "end_round", "show_results", "global"] {
                assert!(names.contains(&state), "{} has {state}", game.name);
            }
            assert!(graph.global.is_some());
            for handler in graph.states.iter().flat_map(|state| &state.handlers) {
                assert_ne!(handler.kind, 0, "{} handler {:016x} has a message kind", game.name, handler.key);
                let chunk = db.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler.key, field::HANDLER_BLOB).and_then(|value| value.blob()).unwrap();
                let global = format!("h_{:016X}", handler.key);
                assert!(chunk.windows(global.len()).any(|window| window == global.as_bytes()), "{} handler {:016x} defines {global}", game.name, handler.key);
            }
            let (global_class, global) = db.attribute(CHALLENGES, game.key, GLOBAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
            let (local_class, local) = db.attribute(CHALLENGES, game.key, LOCAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
            let providers = [(CHALLENGES, game.key), (global_class, global), (local_class, local)];
            let read = |key| providers.iter().find(|&&(class, _)| db.definition(class, key).is_some()).and_then(|&(class, collection)| db.attribute(class, collection, key));
            let texts = |key| read(key).map(|value| (0..value.len()).filter_map(|index| value.text(index)).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default();
            let (skate3_class, skate3_local) = data.db.attribute(CHALLENGES, game.key, LOCAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
            let theirs = |key| {
                let value = data.db.attribute(skate3_class, skate3_local, key).unwrap();
                (0..value.len()).filter_map(|index| value.text(index)).map(str::to_owned).collect::<Vec<_>>()
            };
            assert_eq!(texts(START_LOCATION_ROTATION), theirs(START_LOCATION_ROTATION), "{} starts", game.name);
            assert_eq!(texts(WAIT_LOCATION_ROTATION), theirs(WAIT_LOCATION_ROTATION), "{} waits", game.name);
            assert!(read(TIME_LIMIT).and_then(|value| value.f32(0)).is_some_and(|limit| limit > 0.0), "{} has a run time", game.name);
            assert_eq!(read(MULTIPLIER_ACTIVE).and_then(|value| value.bool(0)), Some(false), "{} scores without the multiplier", game.name);
            let objectives = read(OBJECTIVES).unwrap();
            assert!(!objectives.is_empty(), "{} scores through an objective", game.name);
            for index in 0..objectives.len() {
                let (class, key) = objectives.ref_spec(index).unwrap();
                assert!(db.has_collection(class, key), "{}'s objective {key:016x} joined", game.name);
                let definition = db.attribute(class, key, crate::challenge::objectives::field::OBJECTIVE_DEFINITION);
                eprintln!(
                    "{} objective {key:016x}: {:?} {:?}",
                    game.name,
                    definition.and_then(|value| value.type_name()),
                    definition.and_then(|value| value.text_field(0))
                );
            }
        }
    }

    /// Where a Skate 3 1-Up's fields come from and where the graft puts them.
    #[test]
    #[ignore = "research probe"]
    fn probe_skate3_one_up_graft() {
        const FIELDS: [&str; 8] = [
            "CentralMessageHALIDSetScore", "AllowPlayerBail", "CumulativeScoring", "ProGoesFirst",
            "CentralMessageHALIDDidNotBeatHighScore", "TimeLimit", "HighlightDefinition", "UseStaticCameras",
        ];
        let (Some(root), Some(_)) = (std::env::var_os("SKATE3_DATA"), std::env::var_os("SKATE2_GAME_ROOT")) else { return };
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        let game = data.online_spots(net_challenge_type(ONE_UP_GAME_TYPE).unwrap()).into_iter().next().unwrap();
        let source = &*data.db;
        let reference = |db: &AttribDb, field| db.attribute(CHALLENGES, game.key, field).and_then(|value| value.ref_spec(0)).unwrap();
        let (their_global_class, their_global) = reference(source, GLOBAL_DATA);
        let (their_local_class, their_local) = reference(source, LOCAL_DATA);
        let mut db = skate2_template_db();
        eprintln!(
            "{} local {their_local_class:016x}/{their_local:016x} parent {:?}, in the template db {}",
            game.name,
            source.parent(their_local_class, their_local),
            db.has_collection(their_local_class, their_local)
        );
        data.graft_online_challenges(&mut db);
        let (global_class, global) = reference(&db, GLOBAL_DATA);
        let (local_class, local) = reference(&db, LOCAL_DATA);
        eprintln!("grafted global {global_class:016x}/{global:016x} local {local_class:016x}/{local:016x} parent {:?}", db.parent(local_class, local));
        let theirs = [(CHALLENGES, game.key), (their_global_class, their_global), (their_local_class, their_local)];
        let ours = [(CHALLENGES, game.key), (global_class, global), (local_class, local)];
        for name in FIELDS {
            let key = hash64(name);
            let source_side = theirs.map(|(class, collection)| {
                (source.definition(class, key).map(|d| (d.type_id, d.size, d.flags)), source.attribute(class, collection, key).is_some())
            });
            let our_side = ours.map(|(class, collection)| {
                (
                    db.definition(class, key).map(|d| (d.type_id, d.size, d.flags)),
                    db.attribute(class, collection, key).is_some(),
                    db.defines_alike(class, source, their_local_class, key),
                )
            });
            eprintln!("{name}: theirs {source_side:?}\n    ours {our_side:?}");
        }
    }

    /// Every challenge field Skate 3's 1-Up handlers read (`self.<field>`), as the challenge's
    /// data providers hold it after the graft.
    #[test]
    #[ignore = "research probe"]
    fn probe_skate3_one_up_fields() {
        const FIELDS: [&str; 52] = [
            "AIPathScoreThresholdExtreme", "AIPathScoreThresholdHard", "AIPathScoreThresholdMedium", "AIPathScoreThresholdVeryHard",
            "AIScoreMultiplierEasy", "AIScoreMultiplierExtreme", "AIScoreMultiplierHard", "AIScoreMultiplierMedium",
            "AIScoreMultiplierVeryHard", "AllowPlayerBail", "AudioDefineEpicRunCount", "AudioDefineHighMargin",
            "AudioDefineHighSequenceScoreFraction", "AudioDefineLowScore", "AudioDefineMediumMargin",
            "AudioDefineSecondsLeftBeforeIdle", "AudioDefineSecondsLeftBeforeLastChance", "AudioPlayerQuitChallenge", "BestScore",
            "CameraLocations", "CentralMessageHALIDDidNotBeatHighScore", "CentralMessageHALIDNameBailed",
            "CentralMessageHALIDRoundOver", "CentralMessageHALIDSetScore", "CentralMessageHALIDYourTurn", "Competitor",
            "CumulativeScoring", "CurrentCompetitorIndex", "CurrentTeam", "HighlightDefinition", "IntroNIS",
            "IntroPresentationEvents", "IsDebugEnabled", "LocalPlayerID", "LocalPlayerIndex", "ModifiedCentralMessageHALIDBailed",
            "ModifiedCentralMessageHALIDOneUpped", "Name", "NumCompetitors", "NumLetters", "Objectives", "OnlineOutroNIS",
            "ProGoesFirst", "SkillLevelText", "StartLocationRotation", "TeamIntroductionDelay", "TeamsEnabled", "TimeLimit",
            "Title", "UseStaticCameras", "VisualIndicators", "WaitLocationRotation",
        ];
        let (Some(root), Some(_)) = (std::env::var_os("SKATE3_DATA"), std::env::var_os("SKATE2_GAME_ROOT")) else { return };
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        let mut db = skate2_template_db();
        data.graft_online_challenges(&mut db);
        let game = data.online_spots(net_challenge_type(ONE_UP_GAME_TYPE).unwrap()).into_iter().next().unwrap();
        let (global_class, global) = db.attribute(CHALLENGES, game.key, GLOBAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
        let (local_class, local) = db.attribute(CHALLENGES, game.key, LOCAL_DATA).and_then(|value| value.ref_spec(0)).unwrap();
        let providers = [(CHALLENGES, game.key), (global_class, global), (local_class, local)];
        for name in FIELDS {
            let key = hash64(name);
            let Some(&(class, collection)) = providers.iter().find(|&&(class, _)| db.definition(class, key).is_some()) else {
                eprintln!("{} field {name}: undefined", game.name);
                continue;
            };
            let value = db.attribute(class, collection, key);
            eprintln!(
                "{} field {name}: {:?} len {:?} text {:?} f32 {:?} i32 {:?}",
                game.name,
                value.as_ref().and_then(|value| value.type_name()),
                value.as_ref().map(|value| value.len()),
                value.as_ref().and_then(|value| value.text(0)),
                value.as_ref().and_then(|value| value.f32(0)),
                value.as_ref().and_then(|value| value.i32(0)),
            );
        }
    }

    #[test]
    #[ignore = "research probe"]
    fn probe_skate3_spot_battles() {
        let root = std::env::var_os("SKATE3_DATA").unwrap();
        let data = Skate3Data::open(Path::new(&root)).unwrap();
        let db = AttribDb::load(&data.mounts, &["data\\db\\skater.vlt", CHALLENGE_BANK]).unwrap();
        let mut folder_of: HashMap<u32, String> = HashMap::new();
        for name in data.mounts.loose_files("data\\content\\global_locators\\") {
            if !name.to_ascii_lowercase().ends_with(".rx2") {
                continue;
            }
            let bytes = data.mounts.read_path(&name).unwrap();
            let mut found = HashMap::new();
            let _ = locator_transforms(&bytes, &mut found);
            for hash in found.keys() {
                folder_of.entry(*hash).or_insert_with(|| name.clone());
            }
        }
        let hex = |keys: Vec<u64>| keys.iter().map(|k| format!("{k:016x}")).collect::<Vec<_>>().join(" ");
        let mut types: std::collections::BTreeMap<i32, (usize, usize)> = std::collections::BTreeMap::new();
        for key in db.leaf_collections(CHALLENGES) {
            let Some((class, global)) = db.attribute(CHALLENGES, key, GLOBAL_DATA).and_then(|g| g.ref_spec(0)) else { continue };
            let online = db.attribute(class, global, ONLINE).and_then(|flag| flag.bool(0)) == Some(true);
            let Some(kind) = db.attribute(class, global, CHALLENGE_TYPE).and_then(|value| value.i32(0)) else { continue };
            let row = types.entry(kind).or_default();
            row.0 += 1;
            row.1 += usize::from(online);
            if ![7, 8, 15, 32, 33, 38].contains(&kind) || !online {
                if kind != 9 {
                    continue;
                }
            }
            let name = db.attribute(CHALLENGES, key, CHALLENGE_NAME).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
            let location = db.attribute(class, global, LOCATION).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
            let title = db.attribute(class, global, TITLE).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
            let folder = folder_of.get(&hash_value32(&location)).map_or("-", String::as_str);
            let score = db.attribute(class, global, 0x2e48_24c8_1fda_e87c).and_then(|value| value.u32(0));
            let connector = db.attribute(class, global, 0x0661_a17a_a0e5_c774).and_then(|value| value.bool(0));
            let english = data.text(&title).unwrap_or_default();
            let template = db.attribute(class, global, TEMPLATE_TITLE).and_then(|value| value.text(0)).unwrap_or("-").to_owned();
            eprintln!("ots type {kind} {name} online {online} location {location} folder {folder} title {title} = {english:?} template {template} score {score:?} connector {connector:?}");
            eprintln!("  global {}", hex(db.own_keys(class, global)));
        }
        eprintln!("types (all, online): {types:?}");
        let mut skate3 = db;
        let describe = |db: &AttribDb, class: u64, collection: u64, key: u64| -> String {
            let Some(value) = db.attribute(class, collection, key) else { return "-".into() };
            let kind = value.type_name().unwrap_or("?").to_owned();
            let lower = kind.to_ascii_lowercase();
            let mut text = format!("{kind} x{}", value.len());
            if lower.contains("refspec") {
                text += &format!(" ref {:x?}", value.ref_spec(0));
            } else if lower.contains("text") || lower.contains("string") || lower.contains("char") || lower.contains("locationid") {
                text += &format!(" text {:?}", value.text(0));
            } else if lower.contains("float") {
                text += &format!(" f32 {:?}", value.f32(0));
            } else if let Some(bytes) = value.value_bytes(0) {
                text += &format!(" bytes {:02x?}", &bytes[..bytes.len().min(16)]);
            }
            text
        };
        let battles: Vec<(u64, String, u64, u64)> = skate3
            .leaf_collections(CHALLENGES)
            .into_iter()
            .filter_map(|key| {
                let (class, global) = skate3.attribute(CHALLENGES, key, GLOBAL_DATA)?.ref_spec(0)?;
                (skate3.attribute(class, global, CHALLENGE_TYPE)?.i32(0)? == 53).then_some(())?;
                let name = skate3.attribute(CHALLENGES, key, CHALLENGE_NAME)?.text(0)?.to_owned();
                Some((key, name, class, global))
            })
            .collect();
        for (key, name, class, global) in &battles {
            let _ = skate3.load_more(&data.mounts, &format!("{LOCAL_DATA_VAULTS}\\{name}.vlt"));
            let title = skate3.attribute(*class, *global, TITLE).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
            let location = skate3.attribute(*class, *global, LOCATION).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
            let online = skate3.attribute(*class, *global, ONLINE).and_then(|flag| flag.bool(0));
            let folder = folder_of.get(&hash_value32(&location)).map_or("-", String::as_str);
            eprintln!("battle {name} = {:?} online {online:?} location {location} folder {folder}", data.text(&title).unwrap_or_default());
            for field in skate3.own_keys(*class, *global) {
                eprintln!("  global {field:016x}: {}", describe(&skate3, *class, *global, field));
            }
            if let Some((local_class, local)) = skate3.attribute(CHALLENGES, *key, LOCAL_DATA).and_then(|value| value.ref_spec(0)) {
                for field in skate3.own_keys(local_class, local) {
                    eprintln!("  local {field:016x}: {}", describe(&skate3, local_class, local, field));
                }
            }
        }
        let dump = std::path::PathBuf::from(std::env::var_os("SKATE3_DUMP").unwrap());
        let write_graph = |label: &str, db: &AttribDb, challenge: u64| {
            use crate::challenge::{keys::class, runtime::field};
            let mut out = String::new();
            for state in db.attribute(CHALLENGES, challenge, field::STATES).map(|states| (0..states.len()).filter_map(|i| states.ref_spec(i)).collect::<Vec<_>>()).unwrap_or_default() {
                let name = db.attribute(class::CHALLENGE_STATES, state.1, field::NAME).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
                out += &format!("=== state {name}\n");
                let handlers = db.attribute(class::CHALLENGE_STATES, state.1, field::STATE_HANDLERS).map(|h| (0..h.len()).filter_map(|i| h.ref_spec(i)).collect::<Vec<_>>()).unwrap_or_default();
                for (_, handler) in handlers {
                    let kind = db.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler, field::HANDLER_KIND).and_then(|value| value.u32(0)).unwrap_or(0);
                    let blob = db.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler, field::HANDLER_BLOB).and_then(|value| value.blob().map(<[u8]>::to_vec)).unwrap_or_default();
                    std::fs::write(dump.join(format!("{label}__{name}__{kind:08x}.luac")), &blob).unwrap();
                    out += &format!("--- handler kind {kind:08x} ({} bytes)\n{}\n", blob.len(), String::from_utf8_lossy(&blob));
                }
            }
            std::fs::write(dump.join(format!("{label}.lua.txt")), out).unwrap();
        };
        write_graph("skate2_ots_online", &skate2_template_db(), crate::challenge::keys::hash64("f_1a_24_ots_svm13_rockgap_ol"));
        let graphs: Vec<String> = data
            .mounts
            .archives()
            .iter()
            .flat_map(|archive| archive.archive().entries.iter())
            .filter_map(|entry| match &entry.key {
                skate2_formats::big::BigEntryKey::Name(name) if name.to_ascii_lowercase().contains("challenge_stategraph") && name.to_ascii_lowercase().ends_with(".vlt") => Some(name.clone()),
                _ => None,
            })
            .collect();
        for path in &graphs {
            if let Err(error) = skate3.load_more(&data.mounts, path) {
                eprintln!("state graph vault {path}: {error}");
            }
        }
        eprintln!("{} state graph vaults", graphs.len());
        for name in ["ots_univ_04_ol", "ots_univ_10_ol"] {
            write_graph(&format!("skate3_{name}"), &skate3, crate::challenge::keys::hash64(name));
        }
        {
            let skate2 = skate2_template_db();
            for name in ["spot_battle", "f_1a_24_ots_svm13_rockgap_ol", "f_1a_26_ots_svm05_megapark_ol", "f_1a_28_ots_svm05_funpark_ol"] {
                let key = crate::challenge::keys::hash64(name);
                eprintln!("skate2 {name} parent {:016x}", skate2.parent(CHALLENGES, key).unwrap_or(0));
                for field in skate2.own_keys(CHALLENGES, key) {
                    eprintln!("  s2own {field:016x}: {}", describe(&skate2, CHALLENGES, key, field));
                }
                if let Some((local_class, local)) = skate2.attribute(CHALLENGES, key, LOCAL_DATA).and_then(|value| value.ref_spec(0)) {
                    for field in skate2.own_keys(local_class, local) {
                        eprintln!("  s2local {field:016x}: {}", describe(&skate2, local_class, local, field));
                    }
                }
                if let Some((global_class, global)) = skate2.attribute(CHALLENGES, key, GLOBAL_DATA).and_then(|value| value.ref_spec(0)) {
                    for field in skate2.own_keys(global_class, global) {
                        eprintln!("  s2global {field:016x}: {}", describe(&skate2, global_class, global, field));
                    }
                }
            }
            const CUMULATIVE_SCORING: u64 = 0xbf51_d1ae_70a1_a24e;
            for (label, class) in [("challenges", CHALLENGES), ("local data", 0x46a5_6add_f284_c381), ("global data", 0xa4f3_5b15_9d74_32b5)] {
                eprintln!("skate2 {label} defines CumulativeScoring: {:?} TimeLimit: {:?}", skate2.definition(class, CUMULATIVE_SCORING).is_some(), skate2.definition(class, 0xf89b_fdfc_8899_0f17).is_some());
            }
            for key in skate2.collections(0x46a5_6add_f284_c381).to_vec() {
                if skate2.own_keys(0x46a5_6add_f284_c381, key).contains(&CUMULATIVE_SCORING) {
                    eprintln!("skate2 local data {key:016x} sets CumulativeScoring: {:?}", skate2.attribute(0x46a5_6add_f284_c381, key, CUMULATIVE_SCORING).and_then(|value| value.bool(0)));
                }
            }
            for key in skate2.collections(CHALLENGES).to_vec() {
                if skate2.own_keys(CHALLENGES, key).contains(&CUMULATIVE_SCORING) {
                    let name = skate2.attribute(CHALLENGES, key, CHALLENGE_NAME).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
                    eprintln!("skate2 challenge {name} sets CumulativeScoring: {:?}", skate2.attribute(CHALLENGES, key, CUMULATIVE_SCORING).and_then(|value| value.bool(0)));
                }
            }
        }
        // The Spot Battle challenge's parent chain, and its StateGraph.
        let mut current = crate::challenge::keys::hash64("ots_univ_10_ol");
        while current != 0 {
            eprintln!("chain {current:016x}");
            for field in skate3.own_keys(CHALLENGES, current) {
                eprintln!("  own {field:016x}: {}", describe(&skate3, CHALLENGES, current, field));
            }
            current = skate3.parent(CHALLENGES, current).unwrap_or(0);
        }
        const STATE_GRAPH: u64 = 0x7867_a00c_3a6f_893c;
        if let Some((graph_class, graph)) = skate3.attribute(CHALLENGES, crate::challenge::keys::hash64("ots_univ_10_ol"), STATE_GRAPH).and_then(|value| value.ref_spec(0)) {
            eprintln!("state graph {graph_class:016x}/{graph:016x}");
            for field in skate3.own_keys(graph_class, graph) {
                eprintln!("  graph {field:016x}: {}", describe(&skate3, graph_class, graph, field));
            }
            use crate::challenge::{keys::class, runtime::field};
            let mut out = String::new();
            let mut graph_chain = graph;
            while graph_chain != 0 {
                for list_field in skate3.own_keys(graph_class, graph_chain) {
                    let Some(value) = skate3.attribute(graph_class, graph_chain, list_field) else { continue };
                    for i in 0..value.len() {
                        let Some((state_class, state)) = value.ref_spec(i) else { continue };
                        if state_class != class::CHALLENGE_STATES {
                            continue;
                        }
                        let name = skate3.attribute(class::CHALLENGE_STATES, state, field::NAME).and_then(|value| value.text(0)).unwrap_or("?").to_owned();
                        out += &format!("=== state {name}\n");
                        let handlers = skate3.attribute(class::CHALLENGE_STATES, state, field::STATE_HANDLERS).map(|h| (0..h.len()).filter_map(|i| h.ref_spec(i)).collect::<Vec<_>>()).unwrap_or_default();
                        for (_, handler) in handlers {
                            let kind = skate3.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler, field::HANDLER_KIND).and_then(|value| value.u32(0)).unwrap_or(0);
                            let blob = skate3.attribute(class::CHALLENGE_MESSAGE_HANDLERS, handler, field::HANDLER_BLOB).and_then(|value| value.blob().map(<[u8]>::to_vec)).unwrap_or_default();
                            std::fs::write(dump.join(format!("skate3_spot_battle__{graph_chain:016x}__{name}__{kind:08x}.luac")), &blob).unwrap();
                            out += &format!("--- handler kind {kind:08x} ({} bytes)\n{}\n", blob.len(), String::from_utf8_lossy(&blob));
                        }
                    }
                }
                graph_chain = skate3.parent(graph_class, graph_chain).unwrap_or(0);
            }
            std::fs::write(dump.join("skate3_spot_battle_graph.lua.txt"), out).unwrap();
        }
        for parent in ["own_the_spots_battles", "main", "default", "own_the_spots", "skate2_online", "hall_of_meat", "death_race"] {
            let path = format!("{LOCAL_DATA_VAULTS}\\{parent}.vlt");
            let exists = data.mounts.read_path(&path).is_ok();
            let loaded = if exists { skate3.load_more(&data.mounts, &path).map_err(|error| error.to_string()) } else { Err("absent".into()) };
            eprintln!("parent vault {parent}: exists {exists} load {loaded:?}");
        }
        for (key, name, _, _) in &battles {
            let Some((local_class, local)) = skate3.attribute(CHALLENGES, *key, LOCAL_DATA).and_then(|value| value.ref_spec(0)) else { continue };
            let mut chain = Vec::new();
            let mut current = local;
            while current != 0 {
                chain.push(format!("{current:016x}"));
                current = skate3.parent(local_class, current).unwrap_or(0);
            }
            if let Some((global_class, global)) = skate3.attribute(CHALLENGES, *key, GLOBAL_DATA).and_then(|value| value.ref_spec(0)) {
                let hulls = skate3.attribute(global_class, global, REQUIRED_CHALLENGE_HULL).map(|value| (0..value.len()).map(|i| format!("{:?}", value.text(i))).collect::<Vec<_>>());
                eprintln!("hull {name}: {hulls:?}");
                for (label, class, collection) in [("challenge", CHALLENGES, *key), ("global", global_class, global)] {
                    for field in [TIME_LIMIT, CUMULATIVE_SCORING, 0xbeba_1d8b_453f_8137] {
                        if skate3.attribute(class, collection, field).is_some() {
                            eprintln!("provider {name} {label} {field:016x}: {}", describe(&skate3, class, collection, field));
                        }
                    }
                }
            }
            let resolved = |field: u64| describe(&skate3, local_class, local, field);
            for field in [START_LOCATION, TURN_BASED_WAITING_LOCATION, TURN_BASED_ATTEMPT_SPAWN_POINT] {
                let locator = skate3.attribute(local_class, local, field).and_then(|value| value.text(0)).unwrap_or("").to_owned();
                eprintln!("locator {name} {field:016x} {locator} global {}", data.locator_transforms().contains_key(&hash_value32(&locator)));
            }
            eprintln!(
                "resolved {name}: TimeLimit {} | CumulativeScoring {} | NumSequencesToEndTurn {} | chain {}",
                resolved(0xf89b_fdfc_8899_0f17),
                resolved(0xdd21_d0f4_e716_d82f),
                resolved(0xbeba_1d8b_453f_8137),
                chain.join(">")
            );
        }
        let db = skate3;
        for key in db.collections(CHALLENGES).to_vec() {
            let name = db.attribute(CHALLENGES, key, CHALLENGE_NAME).and_then(|value| value.text(0)).unwrap_or("").to_owned();
            if !name.to_ascii_lowercase().contains("ots") || true {
                continue;
            }
            let local = db.attribute(CHALLENGES, key, LOCAL_DATA);
            let bytes = local.as_ref().and_then(|value| value.value_bytes(0)).map(|bytes| format!("{bytes:02x?}")).unwrap_or_default();
            let kind = db.attribute(CHALLENGES, key, GLOBAL_DATA).and_then(|g| g.ref_spec(0)).and_then(|(class, global)| db.attribute(class, global, CHALLENGE_TYPE)).and_then(|value| value.i32(0));
            let leaf = db.leaf_collections(CHALLENGES).contains(&key);
            eprintln!("challenge {name} key {key:016x} leaf {leaf} type {kind:?} parent {:016x} local {} {bytes}", db.parent(CHALLENGES, key).unwrap_or(0), local.and_then(|value| value.type_name()).unwrap_or("-"));
        }
    }
}
