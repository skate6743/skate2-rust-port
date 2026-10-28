//! `Sk8::Scene::SceneBinDirectory`: the scene directory file. `FindSceneDataFile`
//! (`0x82873f98`) opens the highest-numbered `data\scene\scene_NNN.bin`; the game
//! ships only `scene_000.bin` (in `miscboot.big`). `InitSceneDataBuffer`
//! (`0x82871b58`) reads a 16-byte header (magic, version, scene count, character
//! count), the 104-byte `SceneBinProperties_t` records, then the 180-byte
//! `SceneBinCharacterInfo_t` records, all big-endian, and maps each record by its
//! name hash (`DoInsertValue`: the first record of a hash wins).

use std::collections::HashMap;

pub const PATH: &str = "data\\scene\\scene_000.bin";

const HEADER: usize = 16;
const PROPERTIES: usize = 104;
const CHARACTER: usize = 180;

/// `Sk8::Scene::eSceneType`.
pub mod scene_type {
    pub const NONE: u8 = 0;
    pub const FRONTEND: u8 = 1;
    pub const FLYTHROUGH: u8 = 2;
    pub const RACE_START: u8 = 3;
    pub const HIGHLIGHT: u8 = 4;
    pub const PHOTO_INTRO: u8 = 5;
    pub const PHOTO_OUTRO: u8 = 6;
    pub const SKATE_INTRO_OFFER: u8 = 7;
    pub const SKATE_INTRO_ACCEPT: u8 = 8;
    pub const SKATE_INTRO_DECLINE: u8 = 9;
    pub const SKATE_OUTRO_WIN: u8 = 10;
    pub const SKATE_OUTRO_LOSE: u8 = 11;
    pub const SPOT_INTRO: u8 = 12;
    pub const SPOT_OUTRO: u8 = 13;
    pub const PRO_INTRO: u8 = 14;
    pub const PRO_CHALLENGE_INTRO: u8 = 15;
    pub const PRO_CHALLENGE_OUTRO: u8 = 16;
    pub const RIGHT_OF_PASSAGE: u8 = 17;
    pub const STORY: u8 = 18;
    pub const TUTORIAL_INTRO: u8 = 19;
    pub const TUTORIAL_OUTRO_SUCCESS: u8 = 20;
    pub const TUTORIAL_OUTRO_FAILURE: u8 = 21;
    pub const VIEW_MODE: u8 = 22;
    pub const FILM: u8 = 23;
    pub const ONE_UP: u8 = 24;
    pub const BEST_TRICK: u8 = 25;
    pub const GOTO: u8 = 26;
    pub const RACE_END: u8 = 27;
}

/// `SceneBinProperties_t`.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneProperties {
    pub name_hash: u32,
    pub kind: u8,
    /// `mBitField`: bit 0 snaps the scene to the ground, bit 1 waits for the
    /// stream focus before the scene is revealed, bit 2 keeps DMOs enabled.
    pub bit_field: u8,
    pub is_replay_on: u8,
    pub camera_track: u8,
    pub location: [f32; 3],
    /// A forward direction, e.g. (0, 0, -1).
    pub orientation: [f32; 3],
    /// `mLength`, in seconds.
    pub length: f32,
    pub character_count: u16,
    pub character_start: u16,
    pub name: String,
}

/// `SceneBinCharacterInfo_t`.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneCharacterInfo {
    pub name: String,
    pub category: String,
    pub start_anim_db: String,
    pub start_anim_name: String,
    pub location: [f32; 3],
    pub orientation: [f32; 3],
    pub anim_offset: f32,
    pub snap_to_ground: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectoryError {
    Truncated,
    CharacterRange { scene: String },
}

impl std::fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("scene directory truncated"),
            Self::CharacterRange { scene } => write!(f, "scene {scene} characters outside the directory"),
        }
    }
}

impl std::error::Error for DirectoryError {}

/// `ICELITE::StringHash` (`0x8297e018`): seed `0xFFFFFFFF`, `h = 33h + byte`,
/// case-sensitive. `ConvertStringToSceneHash` (`0x8287db48`, vtable slot 3)
/// hashes a scene name with it.
#[must_use]
pub fn scene_hash(name: &str) -> u32 {
    name.bytes().fold(u32::MAX, |h, b| h.wrapping_mul(33).wrapping_add(u32::from(b)))
}

fn be_u32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn be_f32(b: &[u8], at: usize) -> f32 {
    f32::from_bits(be_u32(b, at))
}

fn vector(b: &[u8], at: usize) -> [f32; 3] {
    [be_f32(b, at), be_f32(b, at + 4), be_f32(b, at + 8)]
}

fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..end].iter().map(|&c| char::from(c)).collect()
}

#[derive(Debug, Default)]
pub struct SceneDirectory {
    scenes: Vec<SceneProperties>,
    by_hash: HashMap<u32, usize>,
    characters: Vec<SceneCharacterInfo>,
}

impl SceneDirectory {
    /// # Errors
    ///
    /// A buffer shorter than its header's counts, or a scene whose characters
    /// lie outside the character records.
    pub fn parse(bytes: &[u8]) -> Result<Self, DirectoryError> {
        if bytes.len() < HEADER {
            return Err(DirectoryError::Truncated);
        }
        let scene_count = be_u32(bytes, 8) as usize;
        let character_count = be_u32(bytes, 12) as usize;
        let characters_at = HEADER + PROPERTIES * scene_count;
        if bytes.len() < characters_at + CHARACTER * character_count {
            return Err(DirectoryError::Truncated);
        }
        let mut directory = Self::default();
        for i in 0..scene_count {
            let r = &bytes[HEADER + PROPERTIES * i..HEADER + PROPERTIES * (i + 1)];
            let scene = SceneProperties {
                name_hash: be_u32(r, 0),
                kind: r[4],
                bit_field: r[5],
                is_replay_on: r[6],
                camera_track: r[7],
                location: vector(r, 8),
                orientation: vector(r, 20),
                length: be_f32(r, 32),
                character_count: u16::from_be_bytes([r[36], r[37]]),
                character_start: u16::from_be_bytes([r[38], r[39]]),
                name: text(&r[40..104]),
            };
            if usize::from(scene.character_start) + usize::from(scene.character_count) > character_count {
                return Err(DirectoryError::CharacterRange { scene: scene.name });
            }
            directory.by_hash.entry(scene.name_hash).or_insert(i);
            directory.scenes.push(scene);
        }
        for j in 0..character_count {
            let r = &bytes[characters_at + CHARACTER * j..characters_at + CHARACTER * (j + 1)];
            directory.characters.push(SceneCharacterInfo {
                name: text(&r[0..32]),
                category: text(&r[32..48]),
                start_anim_db: text(&r[48..112]),
                start_anim_name: text(&r[112..148]),
                location: vector(r, 148),
                orientation: vector(r, 160),
                anim_offset: be_f32(r, 172),
                snap_to_ground: be_u32(r, 176),
            });
        }
        Ok(directory)
    }

    #[must_use]
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// `GetSceneProperties(hash)` (`0x8286f240`).
    #[must_use]
    pub fn properties(&self, hash: u32) -> Option<&SceneProperties> {
        self.by_hash.get(&hash).map(|&i| &self.scenes[i])
    }

    /// `IsSceneHashValid` (`0x8286ead8`).
    #[must_use]
    pub fn is_valid(&self, hash: u32) -> bool {
        self.by_hash.contains_key(&hash)
    }

    /// `GetSceneType` (`0x8286ef48`): `SCENE_TYPE_NONE` for an unknown hash.
    #[must_use]
    pub fn scene_type(&self, hash: u32) -> u8 {
        self.properties(hash).map_or(scene_type::NONE, |p| p.kind)
    }

    /// `IsSceneSkippable` (`0x8286eb40`): a known scene other than a photo outro.
    #[must_use]
    pub fn is_skippable(&self, hash: u32) -> bool {
        self.properties(hash).is_some_and(|p| p.kind != scene_type::PHOTO_OUTRO)
    }

    /// `IsSceneControllingReplay` (`0x8286ebd0`): `mIsReplayOn`.
    #[must_use]
    pub fn is_controlling_replay(&self, hash: u32) -> bool {
        self.properties(hash).is_some_and(|p| p.is_replay_on != 0)
    }

    /// `IsSceneNonVisIndicator` (`0x8286ec58`): true for an unknown hash, else
    /// for every type but a flythrough.
    #[must_use]
    pub fn is_non_vis_indicator(&self, hash: u32) -> bool {
        self.properties(hash).is_none_or(|p| p.kind != scene_type::FLYTHROUGH)
    }

    /// `IsSceneWithCharacters` (`0x8286ece0`): not for the none, flythrough,
    /// photo and spot-outro types (an unknown hash reads as none), else when
    /// the scene does not control the replay.
    #[must_use]
    pub fn is_with_characters(&self, hash: u32) -> bool {
        let (kind, replay) = self.properties(hash).map_or((scene_type::NONE, false), |p| (p.kind, p.is_replay_on != 0));
        let typed = !matches!(
            kind,
            scene_type::NONE | scene_type::FLYTHROUGH | scene_type::PHOTO_INTRO | scene_type::PHOTO_OUTRO | scene_type::SPOT_OUTRO
        );
        typed && !replay
    }

    /// `IsSceneWithDMOsEnabled` (`0x8286edc0`): `mBitField` bit 2.
    #[must_use]
    pub fn is_with_dmos_enabled(&self, hash: u32) -> bool {
        self.properties(hash).is_some_and(|p| (p.bit_field >> 2) & 1 != 0)
    }

    /// `GetSceneCameraTrack` (`0x8286efc8`): 0 for an unknown hash.
    #[must_use]
    pub fn camera_track(&self, hash: u32) -> u8 {
        self.properties(hash).map_or(0, |p| p.camera_track)
    }

    /// `GetSceneCharacterInfoFromIndex` (`0x828618c0`) over the scene's range.
    #[must_use]
    pub fn characters(&self, scene: &SceneProperties) -> &[SceneCharacterInfo] {
        let start = usize::from(scene.character_start);
        &self.characters[start..start + usize::from(scene.character_count)]
    }

    /// PC port: a later directory's scenes (downloadable content's `scene_NNN.bin`) that this one
    /// lacks, with their characters. The game's own scenes stay this directory's.
    pub fn add_new_scenes(&mut self, other: &Self) {
        for scene in &other.scenes {
            if self.by_hash.contains_key(&scene.name_hash) {
                continue;
            }
            let Ok(start) = u16::try_from(self.characters.len()) else { return };
            let mut added = scene.clone();
            added.character_start = start;
            self.characters.extend_from_slice(other.characters(scene));
            self.by_hash.insert(added.name_hash, self.scenes.len());
            self.scenes.push(added);
        }
    }
}

/// The downloadable content's numbered scene files `load_with_content` looks for.
const CONTENT_SCENE_FILES: u32 = 32;

/// `FindSceneDataFile`'s scene directory and the ICE data, with the downloadable content's. The
/// original reads only the highest-numbered `scene_NNN.bin` and `ice_NNN.bin`, which a pack
/// builds with the game's own scenes as the release build has them; here the game's own
/// (`_000`) stay as this build ships them, and each pack's file adds the scenes it brings.
///
/// # Errors
///
/// A file that cannot be read or parsed.
pub fn load_with_content(
    mounts: &skate2_formats::mount::MountSet,
) -> Result<(SceneDirectory, skate2_formats::ice_camera::CameraSet), String> {
    use skate2_formats::ice_camera::CameraSet;
    let read = |path: &str| mounts.read_path(path).map_err(|e| format!("{path}: {e}"));
    let mut directory = SceneDirectory::parse(&read(PATH)?).map_err(|e| format!("{PATH}: {e}"))?;
    let mut cameras = CameraSet::parse(&read(super::camera::PATH)?)?;
    for number in 1..=CONTENT_SCENE_FILES {
        let scene = format!("data\\scene\\scene_{number:03}.bin");
        if mounts.contains_path(&scene) {
            directory.add_new_scenes(&SceneDirectory::parse(&read(&scene)?).map_err(|e| format!("{scene}: {e}"))?);
        }
        let ice = format!("data\\scene\\ice_{number:03}.bin");
        if mounts.contains_path(&ice) {
            cameras.add_new_groups(CameraSet::parse(&read(&ice)?).map_err(|e| format!("{ice}: {e}"))?);
        }
    }
    Ok((directory, cameras))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_hash_matches_the_directory_names() {
        assert_eq!(scene_hash("02_Pros_Arrival"), 0xa5fe_d0f4);
        assert_eq!(scene_hash("tutorial_m_1a_goto_slappys"), 0x7725_b313);
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn shipped_directory_reads_every_scene() {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\big\\miscboot.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        let directory = SceneDirectory::parse(&mounts.read_path(PATH).unwrap()).unwrap();
        assert_eq!(directory.scene_count(), 561);
        assert!(directory.scenes.iter().all(|s| scene_hash(&s.name) == s.name_hash));
        let tutorial = directory.properties(scene_hash("tutorial_m_1a_goto_slappys")).unwrap();
        assert_eq!((tutorial.kind, tutorial.camera_track, tutorial.length), (scene_type::TUTORIAL_INTRO, 0, 15.0));
        let characters = directory.characters(tutorial);
        assert_eq!(characters.len(), 1);
        assert_eq!((characters[0].name.as_str(), characters[0].category.as_str()), ("rob_dyrdek", "marquee"));
        assert_eq!(characters[0].start_anim_db, "scene/anim/tutorial_m_1a_goto_slappys_A01_01.abin");
        let welcome = directory.properties(scene_hash("01_Welcome_Back")).unwrap();
        assert_eq!((welcome.kind, welcome.bit_field, welcome.character_count), (scene_type::STORY, 1, 3));
        assert!(directory.is_skippable(welcome.name_hash));
        assert!(!directory.is_skippable(scene_hash("photo_out_m_1a_05_photo_rail")));
        assert_eq!(directory.scene_type(0), scene_type::NONE);
        assert!(directory.is_non_vis_indicator(0));
    }
}
