use super::*;
use crate::nis::directory::{SceneDirectory, scene_hash, scene_type};

const DT: f32 = 1.0 / 60.0;

/// A directory of one scene of `kind` and `length` seconds.
fn directory(name: &str, kind: u8, length: f32) -> Arc<SceneDirectory> {
    directory_at(name, kind, 0, [0.0; 3], length)
}

/// A directory of one scene with `mBitField` `bits` at `location`, facing
/// -Z.
fn directory_at(name: &str, kind: u8, bits: u8, location: [f32; 3], length: f32) -> Arc<SceneDirectory> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"scen");
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&scene_hash(name).to_be_bytes());
    bytes.extend_from_slice(&[kind, bits, 0, 0]);
    for v in [location[0], location[1], location[2], 0.0, 0.0, -1.0, length] {
        bytes.extend_from_slice(&v.to_be_bytes());
    }
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    let mut text = [0u8; 64];
    text[..name.len()].copy_from_slice(name.as_bytes());
    bytes.extend_from_slice(&text);
    Arc::new(SceneDirectory::parse(&bytes).unwrap())
}

/// A take of `length` seconds at full speed.
struct FixedTrack {
    length: f32,
}

impl CameraTrack for FixedTrack {
    fn update(&mut self, elapsed: f32) -> TrackUpdate {
        TrackUpdate { finished: elapsed / self.length >= 1.0, set_time: None }
    }
    fn sim_time(&self) -> f32 {
        1.0
    }
    fn camera(&mut self, _: &SceneFrame, _: &CameraAnchors, _: &mut Mover, _: f32) -> Option<IceFrame> {
        Some(IceFrame { matrix: [0.0; 16], fov: 45.0, near_plane: Some(0.05), bloom: None, fade: None, postfx: PostfxChannels::default() })
    }
}

/// A take whose camera frames hold fixed lens and post-fx values; its eye
/// starts at (1, 2, 3) looking down +Z.
struct LensTrack {
    length: f32,
    frame: Option<IceFrame>,
}

impl CameraTrack for LensTrack {
    fn update(&mut self, elapsed: f32) -> TrackUpdate {
        TrackUpdate { finished: elapsed / self.length >= 1.0, set_time: None }
    }
    fn sim_time(&self) -> f32 {
        1.0
    }
    fn camera(&mut self, _: &SceneFrame, _: &CameraAnchors, _: &mut Mover, _: f32) -> Option<IceFrame> {
        self.frame
    }
    fn time_zero(&self, _: &SceneFrame, _: &CameraAnchors) -> Option<([f32; 3], [f32; 3])> {
        Some(([1.0, 2.0, 3.0], [1.0, 2.0, 8.0]))
    }
}

struct LensTracks(Option<IceFrame>);

impl CameraTracks for LensTracks {
    fn connect(&self, scene: &SceneProperties, _: u8, _: Option<f32>) -> Option<(Box<dyn CameraTrack>, Option<f32>)> {
        Some((Box::new(LensTrack { length: scene.length, frame: self.0 }), None))
    }
}

struct FixedTracks;

impl CameraTracks for FixedTracks {
    fn connect(&self, scene: &SceneProperties, _: u8, _: Option<f32>) -> Option<(Box<dyn CameraTrack>, Option<f32>)> {
        Some((Box::new(FixedTrack { length: scene.length }), None))
    }
}

/// A `subtitles.big` holding the English `XBOX2_<name>.sub` of `keys`
/// (start in 0.1 s, HAL id).
fn subtitle_big(name: &str, keys: &[(u16, u32)]) -> Arc<Vec<u8>> {
    let path = format!("data\\fe\\subtitles\\SubTitles_English\\XBOX2_{name}.sub\0");
    let header = 16 + 8 + path.len();
    let mut data = Vec::new();
    for &(start, hash) in keys {
        data.extend_from_slice(&start.to_le_bytes());
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&hash.to_le_bytes());
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BIGF");
    bytes.extend_from_slice(&u32::try_from(header + data.len()).unwrap().to_le_bytes());
    for v in [1, header, header, data.len()] {
        bytes.extend_from_slice(&u32::try_from(v).unwrap().to_be_bytes());
    }
    bytes.extend_from_slice(path.as_bytes());
    bytes.extend_from_slice(&data);
    Arc::new(bytes)
}

/// One game frame: the simulation update, then the presentation frame.
fn frame(nis: &mut NisSystem) -> Vec<NisEvent> {
    nis.update_simulation(&mut |_, _| Ok(None)).unwrap();
    nis.update_presentation(DT);
    nis.take_events()
}

#[test]
fn basic_nis_fades_plays_its_take_and_notifies_at_full_white() {
    let mut nis = NisSystem::new(directory("tutorial", scene_type::TUTORIAL_INTRO, 2.0), Arc::new(FixedTracks), None, "English");
    nis.run_basic_nis("tutorial", 0, false, true);
    assert_eq!(nis.take_events(), vec![NisEvent::PauseSim]);
    assert!(nis.is_in_nis() && nis.view().hud_hidden);
    let mut frames = 0u32;
    let mut started = None;
    let mut completed = None;
    let mut resumed = None;
    while !nis.idle() {
        frames += 1;
        let events = frame(&mut nis);
        if started.is_none() && nis.is_in_playback() {
            started = Some(frames);
            // The scene shows through the bloom-out from white.
            assert!(nis.view().camera.is_some());
        }
        if events.contains(&NisEvent::Completed) {
            completed = Some(frames);
            // The notify follows the full white by a frame, which the bloom
            // back out has already advanced.
            assert!(nis.view().bloom_to_white > 0.9);
        }
        if events.contains(&NisEvent::ResumeSim) {
            resumed = Some(frames);
        }
        assert!(frames < 600);
    }
    let (started, completed, resumed) = (started.unwrap(), completed.unwrap(), resumed.unwrap());
    // A fifth of a second of bloom to white before the scene plays, its two
    // seconds, a fifth of a second of bloom back to white, then the notify.
    assert!((13..=19).contains(&started), "{started}");
    assert!((completed - started).abs_diff(133) <= 4, "{started} {completed}");
    assert_eq!(resumed, completed);
    // The bloom out to the game picture follows.
    assert!(!nis.is_in_nis());
    assert_eq!(nis.view().bloom_to_white, 0.0);
    assert!(!nis.view().hud_hidden);
    assert!(nis.view().camera.is_none());
}

#[test]
fn the_scene_keeps_the_lens_and_post_fx_the_channels_set_until_it_is_purged() {
    let lens = IceFrame {
        matrix: [2.0; 16],
        fov: 50.0,
        near_plane: Some(0.3),
        bloom: Some([0.9, 0.7]),
        fade: Some([1.0, 1.0, 1.0, 0.5]),
        postfx: PostfxChannels {
            tonemap: Some(1.5),
            fisheye: Some(0.9),
            near_far: Some([120.0, 30.0]),
            brightness: Some([0.1, 0.2]),
            contrast: Some([[1.0, 0.48], [2.5, 0.66]]),
            saturate: Some([-0.6, 10.0]),
            tint: Some([[1.0, 0.9, 0.7], [1.0, 0.75, 0.5]]),
            scanline: Some(0.25),
            radial_blur: Some(1.0),
        },
    };
    let mut nis = NisSystem::new(directory("tutorial", scene_type::TUTORIAL_INTRO, 2.0), Arc::new(LensTracks(Some(lens))), None, "English");
    nis.run_basic_nis("tutorial", 0, false, true);
    let _ = nis.take_events();
    while nis.view().scene.is_none() {
        let _ = frame(&mut nis);
    }
    let view = nis.view();
    assert_eq!(view.camera, Some(NisCamera { matrix: Some([2.0; 16]), fov: 50.0, near_plane: 0.3 }));
    // Scene::Init froze the stream focus at the time-zero camera.
    assert_eq!(view.frozen_focus, Some(FrozenFocus::At([1.0, 2.0, 3.0])));
    let ice = IcePostfx {
        bloom: [0.9, 0.7],
        fade: [1.0, 1.0, 1.0, 0.5],
        grade: [
            Grade { brightness: 0.1, contrast: 1.0, midpoint: 0.48, saturate: -0.6, colour: [1.0, 0.9, 0.7] },
            Grade { brightness: 0.2, contrast: 2.5, midpoint: 0.66, saturate: 10.0, colour: [1.0, 0.75, 0.5] },
        ],
        far: [120.0, 30.0],
        tonemap: 1.5,
        fisheye: 0.9,
        scanline: 0.25,
        radial_blur: 1.0,
        ..IcePostfx::default()
    };
    assert_eq!(view.ice_postfx, Some(ice));
    while !nis.idle() {
        let _ = frame(&mut nis);
    }
    let view = nis.view();
    assert_eq!((view.camera, view.frozen_focus, view.ice_postfx), (None, None, None));
}

#[test]
fn without_a_camera_frame_the_scene_shows_its_time_zero_transform() {
    let mut nis = NisSystem::new(directory("tutorial", scene_type::TUTORIAL_INTRO, 2.0), Arc::new(LensTracks(None)), None, "English");
    nis.run_basic_nis("tutorial", 0, false, true);
    let _ = nis.take_events();
    while nis.view().scene.is_none() {
        let _ = frame(&mut nis);
    }
    let camera = nis.view().camera.unwrap();
    // The CameraPresentation constructor's field of view and near plane.
    assert_eq!((camera.fov, camera.near_plane), (30.0, 0.2));
    // ComputeTransform: Z against the look direction, X = normal x Z.
    assert_eq!(camera.matrix, Some([-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 2.0, 3.0, 1.0]));
}

#[test]
fn a_frontend_scene_leaves_the_streamer_alone() {
    let mut nis = NisSystem::new(directory("menu", scene_type::FRONTEND, 2.0), Arc::new(LensTracks(None)), None, "English");
    nis.run_basic_nis("menu", 0, false, true);
    let _ = nis.take_events();
    let mut frames = 0;
    while nis.view().scene.is_none() {
        let _ = frame(&mut nis);
        frames += 1;
        assert!(frames < 600);
    }
    assert!(nis.view().camera.is_some());
    assert_eq!(nis.view().frozen_focus, None);
}

#[test]
fn a_story_scene_skips_only_after_two_seconds() {
    let mut nis = NisSystem::new(directory("story", scene_type::STORY, 10.0), Arc::new(FixedTracks), None, "English");
    nis.run_basic_nis("story", 0, false, true);
    let _ = nis.take_events();
    while !nis.is_in_playback() {
        let _ = frame(&mut nis);
    }
    assert!(nis.fe_state_active());
    // One second into the scene: A is ignored (IsNISSkippable).
    for _ in 0..60 {
        let _ = frame(&mut nis);
    }
    nis.fe_select();
    for _ in 0..90 {
        assert!(!frame(&mut nis).contains(&NisEvent::Completed));
    }
    // Past two seconds A stops the scene: a fifth of a second of bloom to
    // white, then the notify.
    nis.fe_select();
    let mut frames = 0;
    loop {
        frames += 1;
        if frame(&mut nis).contains(&NisEvent::Completed) {
            break;
        }
        assert!(frames < 120);
    }
    assert!((14..=17).contains(&frames), "{frames}");
}

#[test]
fn a_second_request_during_a_nis_is_dropped() {
    let mut nis = NisSystem::new(directory("tutorial", scene_type::TUTORIAL_INTRO, 2.0), Arc::new(FixedTracks), None, "English");
    nis.run_basic_nis("tutorial", 0, false, true);
    nis.run_basic_nis("tutorial", 0, false, true);
    assert_eq!(nis.take_events(), vec![NisEvent::PauseSim]);
    let mut completions = 0;
    for _ in 0..400 {
        completions += frame(&mut nis).iter().filter(|e| **e == NisEvent::Completed).count();
    }
    assert_eq!(completions, 1);
}

#[test]
fn story_and_goto_scenes_teleport_to_the_last_breadcrumb_unless_ignored() {
    let mut nis = NisSystem::new(directory("goto", scene_type::GOTO, 1.0), Arc::new(FixedTracks), None, "English");
    nis.ignore_teleport_to_breadcrumb_on_next_nis();
    nis.run_basic_nis("goto", 0, false, true);
    assert!(!nis.take_events().contains(&NisEvent::TeleportToLastBreadcrumb));
    for _ in 0..400 {
        let _ = frame(&mut nis);
    }
    nis.run_basic_nis("goto", 0, false, true);
    assert_eq!(nis.take_events(), vec![NisEvent::TeleportToLastBreadcrumb, NisEvent::PauseSim]);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_first_career_nis_plays_its_assembly_edit_with_subtitles() {
    use skate2_formats::{
        ice_camera::CameraSet,
        mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    };
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for path in ["data\\big\\miscboot.big", "data\\big\\scene.big"] {
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == path).unwrap();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
    }
    let directory = Arc::new(SceneDirectory::parse(&mounts.read_path(crate::nis::directory::PATH).unwrap()).unwrap());
    let set = Arc::new(CameraSet::parse(&mounts.read_path(crate::nis::camera::PATH).unwrap()).unwrap());
    let subtitles = Arc::new(mounts.read_path(crate::nis::subtitles::BIG_PATH).unwrap());
    let mut nis = NisSystem::new(directory, Arc::new(crate::nis::camera::IceCameras::new(set)), Some(subtitles), "English");
    nis.run_basic_nis("tutorial_m_1a_goto_slappys", 0, false, true);
    let mut frames = 0;
    let mut started = None;
    let mut lines = Vec::new();
    let mut last_line = None;
    loop {
        frames += 1;
        let events = frame(&mut nis);
        if started.is_none() && nis.is_in_playback() {
            started = Some(frames);
        }
        let view = nis.view();
        if view.subtitle != last_line {
            last_line = view.subtitle;
            lines.push((frames, view.subtitle));
        }
        if nis.is_in_playback() && view.scene.is_some() {
            assert!(view.camera.is_some(), "camera: {:?}", nis.pres.handle.as_ref().and_then(|s| s.track.as_ref()).and_then(|t| t.camera_error().map(str::to_owned)));
        }
        if events.contains(&NisEvent::Completed) {
            break;
        }
        assert!(frames < 2000);
    }
    let started = started.unwrap();
    // Three shots: 3.749 scene seconds at 85 %, 3.688 at 90 % and 4.964 at
    // 93 %, about 13.85 s of play, then a fifth of a second of bloom to white.
    let played = (frames - started) as f32 / 60.0 - 0.2;
    assert!((played - 13.85).abs() < 0.15, "{played}");
    // tutorial_m_1a_goto_slappys.sub: three lines, each cleared.
    let hal: Vec<_> = lines.iter().filter_map(|&(_, l)| l).collect();
    assert_eq!(hal, vec![0x96c6_5a5b, 0x96c6_5a5c, 0x96c6_5a5d]);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_first_career_nis_animates_its_character_from_the_scene_clip() {
    use crate::nis::characters::{LocalPlayerRecipe, SceneCharacterAnim, SceneCharacterSource, TwistRatios, twist_ratios};
    use skate2_formats::{
        ice_camera::CameraSet,
        mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    };
    struct Mounted(MountSet, TwistRatios);
    impl SceneCharacterSource for Mounted {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0.read_path(path).ok()
        }
        fn twist_ratios(&self) -> TwistRatios {
            self.1
        }
        fn local_player(&self) -> Option<LocalPlayerRecipe> {
            None
        }
    }
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    // The scene's recipes too: a character without one is left out.
    for archive in SKATE2_ARCHIVES {
        let path = archive.path_under(std::path::Path::new(&root));
        if path.is_file() {
            mounts.push(MountedBig::open(path, archive.use_external_header).unwrap());
        }
    }
    let directory = Arc::new(SceneDirectory::parse(&mounts.read_path(crate::nis::directory::PATH).unwrap()).unwrap());
    let set = Arc::new(CameraSet::parse(&mounts.read_path(crate::nis::camera::PATH).unwrap()).unwrap());
    let ratios = twist_ratios(&mounts).unwrap();
    let mut nis = NisSystem::new(directory, Arc::new(crate::nis::camera::IceCameras::new(set)), None, "English");
    nis.set_character_source(Arc::new(Mounted(mounts, ratios)));
    nis.run_basic_nis("tutorial_m_1a_goto_slappys", 0, false, true);
    let mut frames = 0;
    while !(nis.is_in_playback() && nis.view().scene.is_some()) {
        frame(&mut nis);
        frames += 1;
        assert!(frames < 600);
    }
    let characters = nis.scene_characters();
    assert_eq!(characters.len(), 1);
    let rob = &characters[0];
    assert_eq!((rob.slot, rob.schema.as_str(), rob.category.as_str(), rob.borrowed), (0, "rob_dyrdek", "marquee", false));
    let trajectory = |nis: &NisSystem| nis.scene_characters()[0].anim.pose()[nis.scene_characters()[0].anim.trajectory_bone()].translation;
    let start = trajectory(&nis);
    for _ in 0..120 {
        frame(&mut nis);
    }
    let moved = trajectory(&nis);
    assert_ne!(start, moved);
    // The clip runs on the scene's time: SetTimeElapsed jumps it with the
    // camera's cuts and AddTime advances it with the scene.
    let time = nis.view().scene.unwrap().1;
    let character = &nis.scene_characters()[0];
    let mut fresh = SceneCharacterAnim::new(Arc::clone(character.anim.data()), "tutorial_m_1a_goto_slappys_A01_0", 0.0, ratios).unwrap();
    fresh.set_time(time).unwrap();
    let expected = fresh.pose()[fresh.trajectory_bone()].translation;
    let gap = (0..3).map(|i| (expected[i] - moved[i]).abs()).fold(0.0f32, f32::max);
    eprintln!("trajectory {start:?} -> {moved:?} at {time}, fresh {expected:?}");
    assert!(gap < 0.05, "{gap}");
    // tutorial_m_1a_goto_slappys sits at the origin facing -Z.
    assert_eq!(character.scene_transform(), [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn a_line_the_scene_leaves_up_goes_on_the_update_after_the_nis_state_pops() {
    let subtitles = subtitle_big("tutorial", &[(5, 0x1234_5678), (0xffff, crate::nis::subtitles::NULL_HASH)]);
    let mut nis = NisSystem::new(directory("tutorial", scene_type::TUTORIAL_INTRO, 2.0), Arc::new(FixedTracks), Some(subtitles), "English");
    nis.run_basic_nis("tutorial", 0, false, true);
    let mut frames = 0u32;
    let mut shown = false;
    loop {
        frames += 1;
        let events = frame(&mut nis);
        if events.contains(&NisEvent::Completed) {
            break;
        }
        if nis.view().subtitle.is_some() {
            assert!(nis.is_in_playback());
            shown = true;
        }
        assert!(frames < 600);
    }
    assert!(shown);
    // FrontEndState_NIS pops with the line still up: OnLeave only turns the
    // time-out on, and the next front end update clears the line.
    assert_eq!(nis.view().subtitle, Some(0x1234_5678));
    frame(&mut nis);
    assert_eq!(nis.view().subtitle, None);
}

#[test]
fn a_scene_snapped_to_the_ground_waits_for_the_stream_focus_then_takes_the_grounds_frame() {
    // mBitField bit 0: snap the scene to the ground.
    let directory = directory_at("snap", scene_type::STORY, 1, [10.0, 5.0, 20.0], 2.0);
    let mut nis = NisSystem::new(directory, Arc::new(FixedTracks), None, "English");
    nis.run_basic_nis("snap", 0, false, true);
    let queries = std::cell::RefCell::new(Vec::new());
    let mut ground = |start: [f32; 4], end: [f32; 4]| {
        queries.borrow_mut().push((start, end));
        // A slope under the scene.
        Ok(Some(([10.0, 3.0, 20.0], [0.0, 0.8, 0.6])))
    };
    let mut step = |nis: &mut NisSystem| {
        nis.update_simulation(&mut ground).unwrap();
        nis.update_presentation(DT);
        nis.take_events();
    };
    let mut frames = 0;
    while nis.stream_focus().is_none() {
        step(&mut nis);
        frames += 1;
        assert!(frames < 120);
    }
    // MoveStreamFocus to the scene, then nothing until it has loaded.
    assert_eq!(nis.stream_focus(), Some(FrozenFocus::At([10.0, 5.0, 20.0])));
    assert_eq!(nis.view().frozen_focus, nis.stream_focus());
    for _ in 0..10 {
        nis.set_stream_focus_loaded(false);
        step(&mut nis);
    }
    assert!(queries.borrow().is_empty());
    while nis.pres.handle.is_none() {
        nis.set_stream_focus_loaded(true);
        step(&mut nis);
        frames += 1;
        assert!(frames < 600);
    }
    // Scene::Init moved the focus to the camera: the game's, as the take
    // has no time-zero frame.
    assert_eq!(nis.stream_focus(), Some(FrozenFocus::GameCamera));
    // GetGroundPositionAndOrientationAtLocation: from one unit above to 500
    // below that; the scene has no records to snap afterwards.
    assert_eq!(*queries.borrow(), vec![([10.0, 6.0, 20.0, 0.0], [10.0, -494.0, 20.0, 0.0])]);
    // The scene sits on the ground with its forward laid on the slope:
    // N x (-Z x N).
    let frame = nis.pres.handle.as_ref().unwrap().frame;
    assert_eq!((frame.location, frame.normal), ([10.0, 3.0, 20.0], [0.0, 0.8, 0.6]));
    assert_eq!(frame.orientation, vmx_cross([0.0, 0.8, 0.6], vmx_cross([0.0, 0.0, -1.0], [0.0, 0.8, 0.6])));
    assert!((frame.orientation[1] - 0.48).abs() < 1e-6 && (frame.orientation[2] + 0.64).abs() < 1e-6);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn snapping_records_stand_on_the_ground_the_simulation_finds() {
    use crate::nis::characters::{LocalPlayerRecipe, SceneCharacterSource, TwistRatios, twist_ratios};
    use skate2_formats::{
        ice_camera::CameraSet,
        mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    };
    struct Mounted(MountSet, TwistRatios);
    impl SceneCharacterSource for Mounted {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0.read_path(path).ok()
        }
        fn twist_ratios(&self) -> TwistRatios {
            self.1
        }
        fn local_player(&self) -> Option<LocalPlayerRecipe> {
            None
        }
    }
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for archive in SKATE2_ARCHIVES {
        let path = archive.path_under(std::path::Path::new(&root));
        if path.is_file() {
            mounts.push(MountedBig::open(path, archive.use_external_header).unwrap());
        }
    }
    let directory = Arc::new(SceneDirectory::parse(&mounts.read_path(crate::nis::directory::PATH).unwrap()).unwrap());
    let set = Arc::new(CameraSet::parse(&mounts.read_path(crate::nis::camera::PATH).unwrap()).unwrap());
    let ratios = twist_ratios(&mounts).unwrap();
    let props = directory.properties(scene_hash("07_Cuz_Bowl_Series")).unwrap().clone();
    let records = directory.characters(&props).to_vec();
    let mut nis = NisSystem::new(Arc::clone(&directory), Arc::new(crate::nis::camera::IceCameras::new(set)), None, "English");
    nis.set_character_source(Arc::new(Mounted(mounts, ratios)));
    nis.run_basic_nis("07_Cuz_Bowl_Series", 0, false, true);
    // Flat ground at world y = 30 under every query.
    let mut ground = |start: [f32; 4], _: [f32; 4]| Ok(Some(([start[0], 30.0, start[2]], [0.0, 1.0, 0.0])));
    let mut frames = 0;
    while nis.pres.handle.is_none() {
        nis.set_stream_focus_loaded(true);
        nis.update_simulation(&mut ground).unwrap();
        nis.update_presentation(DT);
        nis.take_events();
        frames += 1;
        assert!(frames < 600);
    }
    let scene = nis.pres.handle.as_ref().unwrap();
    let m = compute_transform(scene.frame.location, scene.frame.orientation, scene.frame.normal);
    let mut snapped = 0;
    // current_player is left out without a local player; the others follow
    // their records in order.
    let instanced = records.iter().filter(|r| r.category != "dynamic" && r.name != "current_player");
    for (character, record) in scene.characters.iter().zip(instanced) {
        let world = character.scene_transform();
        let expected: [f32; 3] = std::array::from_fn(|c| {
            m[3][c] + record.location[0] * m[0][c] + record.location[1] * m[1][c] + record.location[2] * m[2][c]
        });
        if record.snap_to_ground != 0 {
            snapped += 1;
            assert!((world[13] - 30.0).abs() < 1e-3, "{}: {:?}", record.name, &world[12..15]);
            assert!((world[12] - expected[0]).abs() < 1e-3 && (world[14] - expected[2]).abs() < 1e-3);
        } else {
            assert!((0..3).all(|c| (world[12 + c] - expected[c]).abs() < 1e-3), "{}", record.name);
        }
    }
    assert_eq!(snapped, 9);
}

#[test]
fn a_scene_with_bit_1_holds_load_state_6_until_the_stream_focus_has_loaded() {
    let mut nis = NisSystem::new(directory_at("streamed", scene_type::STORY, 2, [0.0; 3], 2.0), Arc::new(FixedTracks), None, "English");
    nis.run_basic_nis("streamed", 0, false, true);
    let mut frames = 0;
    while nis.pres.handle.is_none() {
        nis.set_stream_focus_loaded(true);
        frame(&mut nis);
        frames += 1;
        assert!(frames < 600);
    }
    // Scene::Init moved the focus; nothing has loaded there yet.
    for _ in 0..30 {
        nis.set_stream_focus_loaded(false);
        frame(&mut nis);
        assert_eq!(nis.pres.load_state, 6);
    }
    nis.set_stream_focus_loaded(true);
    frame(&mut nis);
    assert_eq!(nis.pres.load_state, 7);
    frame(&mut nis);
    frame(&mut nis);
    assert!(nis.is_in_playback());
}

#[test]
fn an_online_game_snaps_and_plays_a_streamed_scene_with_its_stream_focus_unloaded() {
    // UpdateLoadState (0x82875bc0) waits for IsStreamFocusLoaded in states 2 and 6 only outside
    // an online game.
    let directory = directory_at("online", scene_type::RACE_START, 3, [10.0, 5.0, 20.0], 2.0);
    let mut nis = NisSystem::new(directory, Arc::new(FixedTracks), None, "English");
    nis.set_online_game(true);
    nis.run_basic_nis("online", 0, false, true);
    let mut ground = |_: [f32; 4], _: [f32; 4]| Ok(Some(([10.0, 3.0, 20.0], [0.0, 1.0, 0.0])));
    let mut frames = 0;
    while !nis.is_in_playback() {
        nis.set_stream_focus_loaded(false);
        nis.update_simulation(&mut ground).unwrap();
        nis.update_presentation(DT);
        nis.take_events();
        frames += 1;
        assert!(frames < 120);
    }
    assert_eq!(nis.pres.handle.as_ref().unwrap().frame.location, [10.0, 3.0, 20.0]);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_tutorial_nis_preloads_and_plays_the_ollie_gesture_tip_from_its_overlay_channel() {
    use crate::nis::fe_overlays::TUTORIAL_GESTURE_OLLIE;
    use skate2_formats::{
        ice_camera::CameraSet,
        mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    };
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for archive in SKATE2_ARCHIVES {
        let path = archive.path_under(std::path::Path::new(&root));
        if path.is_file() {
            mounts.push(MountedBig::open(path, archive.use_external_header).unwrap());
        }
    }
    let directory = Arc::new(SceneDirectory::parse(&mounts.read_path(crate::nis::directory::PATH).unwrap()).unwrap());
    let set = Arc::new(CameraSet::parse(&mounts.read_path(crate::nis::camera::PATH).unwrap()).unwrap());
    let mut nis = NisSystem::new(directory, Arc::new(crate::nis::camera::IceCameras::new(set)), None, "English");
    nis.run_basic_nis("tutorial_m_1a_goto_slappys", 0, false, true);
    let mut commands = Vec::new();
    let mut created = None;
    let mut playing = None;
    let mut frames = 0;
    while !nis.idle() {
        nis.set_stream_focus_loaded(true);
        frame(&mut nis);
        for command in nis.take_screen_commands() {
            if matches!(command, ScreenCommand::Create { .. }) {
                created = Some(frames);
            }
            commands.push((frames, command));
        }
        if playing.is_none() && nis.is_in_playback() {
            playing = Some(frames);
        }
        frames += 1;
        assert!(frames < 6000);
    }
    // The tip registers during the load, before the scene plays; each of
    // the assembly's three takes runs Clear, OlliePt1, OlliePt2, Clear
    // (OnTakeChange leaves the channel's state); PopNIS removes it.
    assert!(created.unwrap() < playing.unwrap());
    let tip = |state| ScreenCommand::SetState { id: 0, state };
    assert_eq!(commands.into_iter().map(|c| c.1).collect::<Vec<_>>(), vec![
        ScreenCommand::Create { id: 0, template: TUTORIAL_GESTURE_OLLIE },
        tip(1),
        tip(2),
        tip(1),
        tip(2),
        tip(1),
        tip(2),
        ScreenCommand::Destroy { id: 0 },
    ]);
}

/// A take with freeze channel intervals starting at `starts` (seconds), each
/// with its command and frame count.
struct FreezeTrack {
    length: f32,
    elapsed: f32,
    starts: Vec<f32>,
    commands: Vec<(u32, u32)>,
}

impl CameraTrack for FreezeTrack {
    fn update(&mut self, elapsed: f32) -> TrackUpdate {
        self.elapsed = elapsed;
        TrackUpdate { finished: elapsed / self.length >= 1.0, set_time: None }
    }
    fn sim_time(&self) -> f32 {
        1.0
    }
    fn camera(&mut self, _: &SceneFrame, _: &CameraAnchors, _: &mut Mover, _: f32) -> Option<IceFrame> {
        Some(IceFrame { matrix: [0.0; 16], fov: 45.0, near_plane: None, bloom: None, fade: None, postfx: PostfxChannels::default() })
    }
    fn freeze_interval(&self) -> Option<usize> {
        self.starts.iter().rposition(|&s| s <= self.elapsed)
    }
    fn freeze_command(&self, interval: usize) -> (u32, u32) {
        self.commands[interval]
    }
    fn parameter(&self) -> f32 {
        self.elapsed / self.length
    }
}

struct FreezeTracks(Vec<f32>, Vec<(u32, u32)>);

impl CameraTracks for FreezeTracks {
    fn connect(&self, scene: &SceneProperties, _: u8, _: Option<f32>) -> Option<(Box<dyn CameraTrack>, Option<f32>)> {
        let track = FreezeTrack { length: scene.length, elapsed: 0.0, starts: self.0.clone(), commands: self.1.clone() };
        Some((Box::new(track), None))
    }
}

#[test]
fn a_freeze_interval_holds_the_scene_clock_for_its_frames() {
    // Owned (3) for 30 frames: a second at 1/30 s a frame.
    let tracks = FreezeTracks(vec![0.0, 0.5], vec![(0, 0), (3, 30)]);
    let mut nis = NisSystem::new(directory("spot", scene_type::SPOT_OUTRO, 3.0), Arc::new(tracks), None, "English");
    nis.run_basic_nis("spot", 0, false, true);
    let mut times = Vec::new();
    while !nis.idle() {
        frame(&mut nis);
        if let Some((_, time)) = nis.view().scene {
            times.push(time);
        }
        assert!(times.len() < 2000);
    }
    // The clock passes 0.5 s, then stands for the second's 60 frames.
    let held = times.iter().position(|&t| t >= 0.5).unwrap();
    let stand = times[held..].iter().take_while(|&&t| t == times[held]).count();
    assert!((59..=61).contains(&stand), "{stand}");
    assert!(times.last().unwrap() > &2.9);
}

#[test]
fn freeze_intervals_crossed_in_one_frame_fire_in_order_and_a_jump_sets_the_time() {
    // A Jump (7) to 2 s at 0.2 s, crossed in the same frame as a Permanent
    // freeze (2) at 0.1 s: the freeze holds the clock at the jump's time.
    let tracks = FreezeTracks(vec![0.0, 0.1, 0.1], vec![(0, 0), (2, 0), (7, 60)]);
    let mut nis = NisSystem::new(directory("race", scene_type::RACE_START, 3.0), Arc::new(tracks), None, "English");
    nis.run_basic_nis("race", 0, false, true);
    let mut times = Vec::new();
    for _ in 0..400 {
        frame(&mut nis);
        if let Some((_, time)) = nis.view().scene {
            times.push(time);
        }
    }
    let jumped = times.iter().position(|&t| t >= 1.9).unwrap();
    assert!((times[jumped] - 2.0).abs() < 1e-6);
    assert!(times[jumped..].iter().all(|&t| t == times[jumped]));
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_first_spot_outro_freezes_on_its_owned_interval_for_a_second() {
    use skate2_formats::{
        ice_camera::CameraSet,
        mount::{MountSet, MountedBig, SKATE2_ARCHIVES},
    };
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for archive in SKATE2_ARCHIVES {
        let path = archive.path_under(std::path::Path::new(&root));
        if path.is_file() {
            mounts.push(MountedBig::open(path, archive.use_external_header).unwrap());
        }
    }
    let directory = Arc::new(SceneDirectory::parse(&mounts.read_path(crate::nis::directory::PATH).unwrap()).unwrap());
    let set = Arc::new(CameraSet::parse(&mounts.read_path(crate::nis::camera::PATH).unwrap()).unwrap());
    let mut nis = NisSystem::new(directory, Arc::new(crate::nis::camera::IceCameras::new(set)), None, "English");
    nis.run_basic_nis("Fp_u1_ownspot_01_outro", 0, false, true);
    let mut times = Vec::new();
    let mut commands = Vec::new();
    let mut frames = 0;
    while !nis.idle() {
        nis.set_stream_focus_loaded(true);
        frame(&mut nis);
        commands.extend(nis.take_screen_commands());
        if let Some((_, time)) = nis.view().scene {
            times.push(time);
        }
        frames += 1;
        assert!(frames < 6000);
    }
    eprintln!("screens {commands:?}");
    // Freeze command 3 for 30 frames: the clock stands for a second.
    let stand = times.windows(2).fold((0, 0), |(run, best), w| {
        let run = if w[1] == w[0] && w[0] > 0.0 { run + 1 } else { 0 };
        (run, best.max(run))
    });
    assert!((59..=61).contains(&stand.1), "{}", stand.1);
}

/// `FireEvent`'s photo flash (`0x826d1f44..0x826d1f8c`: the current
/// threshold to `s_fBloom` -0.6 over `s_fBloomUpTime` 0.15 s and back over
/// `s_fBloomBackTime` 0.1 s) through `PostfxPresentation::Update`: the
/// threshold leaves its base on the frame after the flash and rests at it
/// again.
#[test]
fn a_photo_flash_dips_the_bloom_threshold_and_returns() {
    let mut postfx = IcePostfx::default();
    postfx.bloom_anim = BloomAnim {
        base: postfx.bloom[0],
        value: f32::from_bits(0xbf19_999a),
        up: f32::from_bits(0x3e19_999a),
        down: f32::from_bits(0x3dcc_cccd),
        time: 0.0,
    };
    let thresholds: Vec<f32> = (0..12)
        .map(|_| {
            postfx.update(1.0 / 30.0);
            postfx.bloom[0]
        })
        .collect();
    assert_eq!(thresholds[0], 1.4);
    assert!(thresholds[1] < 1.0 && thresholds[4] < -0.3, "{thresholds:?}");
    assert!(thresholds[5] > thresholds[4] && thresholds[7] > 1.0, "{thresholds:?}");
    assert_eq!(thresholds[8], 1.4);
    assert_eq!(postfx.bloom_anim.time, -1.0);
    assert_eq!(*thresholds.last().unwrap(), 1.4);
}

/// A photo outro's take: a camera at the origin looking down +Z (45 degrees), and a
/// `PhotoShot_FixedFrame` (5) of 30 frames at 0.5 s.
struct PhotoTrack {
    length: f32,
    elapsed: f32,
}

impl CameraTrack for PhotoTrack {
    fn update(&mut self, elapsed: f32) -> TrackUpdate {
        self.elapsed = elapsed;
        TrackUpdate { finished: elapsed / self.length >= 1.0, set_time: None }
    }
    fn sim_time(&self) -> f32 {
        1.0
    }
    fn camera(&mut self, _: &SceneFrame, _: &CameraAnchors, _: &mut Mover, _: f32) -> Option<IceFrame> {
        let matrix = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        Some(IceFrame { matrix, fov: 45.0, near_plane: None, bloom: None, fade: None, postfx: PostfxChannels::default() })
    }
    fn freeze_interval(&self) -> Option<usize> {
        Some(usize::from(self.elapsed >= 0.5))
    }
    fn freeze_command(&self, interval: usize) -> (u32, u32) {
        [(0, 0), (5, 30)][interval]
    }
    fn parameter(&self) -> f32 {
        self.elapsed / self.length
    }
}

struct PhotoTracks;

impl CameraTracks for PhotoTracks {
    fn connect(&self, scene: &SceneProperties, _: u8, _: Option<f32>) -> Option<(Box<dyn CameraTrack>, Option<f32>)> {
        Some((Box::new(PhotoTrack { length: scene.length, elapsed: 0.0 }), None))
    }
    fn fixed_frame_photo_count(&self, _: &SceneProperties, _: u8) -> i32 {
        1
    }
}

/// The view once the scene's playback starts.
fn playing(mut nis: NisSystem, name: &str) -> NisView {
    nis.run_basic_nis(name, 0, false, true);
    for _ in 0..240 {
        if nis.is_in_playback() {
            return nis.view();
        }
        frame(&mut nis);
    }
    panic!("{name} never played");
}

/// A photo outro draws every entity but the photographers' pedestrians, and when its take
/// shoots fixed frames, the skaters on another recipe; other scenes keep to their render types.
#[test]
fn a_photo_outro_leaves_out_the_photographers_and_with_fixed_frames_the_other_skaters() {
    let (atiba, burnette, marquee) = (0x88d7_2df4_43d5_6751, 0x4e6e_5331_d54e_4f61, 0x66f0_8967_879f_825c);
    let fixed = playing(NisSystem::new(directory("photo", scene_type::PHOTO_OUTRO, 3.0), Arc::new(PhotoTracks), None, "English"), "photo");
    assert_eq!(fixed.photo_outro, Some(PhotoOutro { hide_ai_skaters: true }));
    assert!(!fixed.draws(0x222, atiba, false) && !fixed.draws(0x222, burnette, false) && fixed.draws(0x222, marquee, false));
    assert!(fixed.draws(0x122, 0, true) && !fixed.draws(0x122, 0, false));
    assert!(fixed.draws(0x822, 0, false) && fixed.draws(0x422, 0, false) && fixed.draws(0x42, 0, false));
    let shots = playing(NisSystem::new(directory("photo", scene_type::PHOTO_OUTRO, 3.0), Arc::new(FixedTracks), None, "English"), "photo");
    assert_eq!(shots.photo_outro, Some(PhotoOutro { hide_ai_skaters: false }));
    assert!(!shots.draws(0x222, atiba, false) && shots.draws(0x122, 0, false));
    let tutorial = playing(NisSystem::new(directory("tutorial", scene_type::TUTORIAL_INTRO, 2.0), Arc::new(FixedTracks), None, "English"), "tutorial");
    assert_eq!(tutorial.photo_outro, None);
    assert!(!tutorial.draws(0x122, 0, true) && !tutorial.draws(0x222, marquee, false) && tutorial.draws(0x822, 0, false));
}

/// `PhotoShot_FixedFrame` with the local skater in the last frame's view: the scene clock
/// stands for the 30 frames, the bloom threshold dips, and a frozen skater is instanced at the
/// frame's end. Behind the camera, the shot does nothing.
#[test]
fn a_fixed_frame_photo_freezes_flashes_and_instances_the_skater_in_view() {
    for (hips, seen) in [([0.0, 0.0, 5.0], true), ([0.0, 0.0, -5.0], false)] {
        let mut nis = NisSystem::new(directory("photo", scene_type::PHOTO_OUTRO, 3.0), Arc::new(PhotoTracks), None, "English");
        nis.run_basic_nis("photo", 0, false, true);
        nis.set_camera_local_hips(Some(hips));
        nis.set_local_bounds(Some([[-0.5, -1.0, -0.2], [0.5, 1.0, 0.2]]));
        // A photo outro holds its last frame for the photo selection; four seconds cover the
        // take and the freeze.
        let (mut times, mut dipped, mut frozen) = (Vec::new(), false, 0);
        for _ in 0..240 {
            frame(&mut nis);
            let view = nis.view();
            if let Some((_, time)) = view.scene {
                times.push(time);
            }
            dipped |= view.ice_postfx.is_some_and(|postfx| postfx.bloom[0] < 1.0);
            frozen += usize::from(nis.take_frozen_skater());
        }
        // The outro's hold on its last frame for the photo selection is not the shot's freeze.
        let last = times.last().copied().unwrap_or_default();
        let stand = times.windows(2).fold((0, 0), |(run, best), w| {
            let run = if w[1] == w[0] && w[0] > 0.0 && w[0] < last { run + 1 } else { 0 };
            (run, best.max(run))
        });
        assert_eq!(dipped, seen, "{hips:?}");
        assert_eq!(frozen, usize::from(seen), "{hips:?}");
        if seen {
            assert!((59..=61).contains(&stand.1), "{}", stand.1);
        } else {
            assert!(stand.1 < 30, "{}", stand.1);
        }
    }
}

/// The audio system calls a scene makes, with whether its post and speech
/// count as queued and the audio clock.
#[derive(Default)]
struct RecordingAudio {
    calls: std::sync::Mutex<Vec<String>>,
    queued: std::sync::atomic::AtomicBool,
    time: std::sync::Mutex<f64>,
}

impl RecordingAudio {
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.calls.lock().unwrap())
    }
    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

impl crate::nis::audio::AudioInterface for RecordingAudio {
    fn system_time(&self) -> f64 {
        *self.time.lock().unwrap()
    }
    fn queue_nis_post(&self, id: i32) {
        self.record(format!("queue post {id}"));
    }
    fn play_nis_post(&self) {
        self.record("play post".into());
    }
    fn is_post_queued(&self) -> bool {
        self.queued.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn queue_nis_speech(&self, character: i32, id: i32) {
        self.record(format!("queue speech {character} {id}"));
    }
    fn play_nis_speech(&self, character: i32, id: i32) {
        self.record(format!("play speech {character} {id}"));
    }
    fn is_speech_queued(&self, _: i32, _: i32) -> bool {
        self.queued.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn notify_nis_playback_start(&self, scene_type: i32, special_flags: i32, is_replay_nis: bool, scene_length: f32) {
        self.record(format!("start {scene_type} {special_flags} {is_replay_nis} {scene_length}"));
    }
    fn notify_nis_playback_finish(&self) {
        self.record("finish".into());
    }
    fn play_mission_speech(&self, character: i32, id: i32, param: i32) {
        self.record(format!("mission {character} {id} {param}"));
    }
    fn play_camera_man_csis_event(&self, function: &'static str) {
        self.record(format!("camera man {function}"));
    }
    fn trigger_mission_music(&self, trigger: i32, challenge: u64, challenge_type: i32, race_tuning: u64) {
        self.record(format!("mission music {trigger} {challenge:#x} {challenge_type} {race_tuning:#x}"));
    }
    fn restart_challenge(&self, retry: bool) {
        self.record(format!("restart challenge {retry}"));
    }
    fn quit_challenge(&self) {
        self.record("quit challenge".into());
    }
    fn take_speech_messages(&self) -> Vec<crate::nis::audio::SpeechMessage> {
        Vec::new()
    }
}

/// A take that starts with post 5002 and the cameraman's line 5008, both
/// played (command 2) in the first interval of their channels.
struct SpokenTrack {
    length: f32,
}

impl CameraTrack for SpokenTrack {
    fn update(&mut self, elapsed: f32) -> TrackUpdate {
        TrackUpdate { finished: elapsed / self.length >= 1.0, set_time: None }
    }
    fn sim_time(&self) -> f32 {
        1.0
    }
    fn camera(&mut self, _: &SceneFrame, _: &CameraAnchors, _: &mut Mover, _: f32) -> Option<IceFrame> {
        None
    }
    fn audio_speech_interval(&self) -> Option<usize> {
        Some(0)
    }
    fn audio_speech_command(&self, _: usize) -> (u32, i32, i32) {
        (2, 40, 5008)
    }
    fn audio_post_interval(&self) -> Option<usize> {
        Some(0)
    }
    fn audio_post_command(&self, _: usize) -> (u32, i32) {
        (2, 5002)
    }
}

struct SpokenTracks;

impl CameraTracks for SpokenTracks {
    fn connect(&self, scene: &SceneProperties, _: u8, _: Option<f32>) -> Option<(Box<dyn CameraTrack>, Option<f32>)> {
        Some((Box::new(SpokenTrack { length: scene.length }), None))
    }
    fn start_audio(&self, _: &SceneProperties, _: u8) -> Option<StartAudio> {
        Some(StartAudio { post_command: 2, post_id: 5002, speech_command: 2, speech_id: 5008, speech_character: 40 })
    }
}

/// `AudioQueueForStart` queues the take's post and line while the scene
/// loads, `IsAudioReadyForStart` holds the scene until both are queued, the
/// speech and post channels play them as the take starts, and `Scene::Init`
/// and `Scene::Purge` notify the playback's start and finish.
#[test]
fn a_scene_queues_its_post_and_line_and_plays_them_once_queued() {
    let audio = Arc::new(RecordingAudio::default());
    let mut nis = NisSystem::new(directory("goto", scene_type::GOTO, 1.0), Arc::new(SpokenTracks), None, "English");
    nis.set_audio(Arc::clone(&audio) as Arc<dyn crate::nis::audio::AudioInterface>);
    nis.run_basic_nis("goto", 0, false, true);
    let step = |nis: &mut NisSystem| {
        *audio.time.lock().unwrap() += f64::from(DT);
        let _ = frame(nis);
    };
    let mut frames = 0;
    while audio.calls.lock().unwrap().is_empty() {
        step(&mut nis);
        frames += 1;
        assert!(frames < 600);
    }
    assert_eq!(audio.take(), ["queue post 5002", "queue speech 40 5008"]);
    for _ in 0..120 {
        step(&mut nis);
        assert!(nis.view().scene.is_none());
    }
    audio.queued.store(true, std::sync::atomic::Ordering::SeqCst);
    while nis.view().scene.is_none() {
        step(&mut nis);
        frames += 1;
        assert!(frames < 1200);
    }
    assert_eq!(audio.take(), [format!("start {} 0 false 1", scene_type::GOTO)]);
    while !nis.is_in_playback() || audio.calls.lock().unwrap().is_empty() {
        step(&mut nis);
        frames += 1;
        assert!(frames < 1200);
    }
    assert_eq!(audio.take(), ["play speech 40 5008", "play post"]);
    while !nis.idle() {
        step(&mut nis);
        frames += 1;
        assert!(frames < 2400);
    }
    assert_eq!(audio.take(), ["finish"]);
}

/// With an audio system the scene steps by the audio clock's time since the
/// last presentation frame (`kNISUsesTheAudioClock`), whatever the frame's.
#[test]
fn the_scene_steps_by_the_audio_clock() {
    let audio = Arc::new(RecordingAudio::default());
    audio.queued.store(true, std::sync::atomic::Ordering::SeqCst);
    let mut nis = NisSystem::new(directory("story", scene_type::STORY, 10.0), Arc::new(SpokenTracks), None, "English");
    nis.set_audio(Arc::clone(&audio) as Arc<dyn crate::nis::audio::AudioInterface>);
    nis.run_basic_nis("story", 0, false, true);
    let mut frames = 0;
    while !nis.is_in_playback() {
        *audio.time.lock().unwrap() += f64::from(DT);
        let _ = frame(&mut nis);
        frames += 1;
        assert!(frames < 600);
    }
    let elapsed = |nis: &NisSystem| nis.view().scene.map_or(0.0, |(_, time)| time);
    let before = elapsed(&nis);
    *audio.time.lock().unwrap() += 0.25;
    let _ = frame(&mut nis);
    let stepped = elapsed(&nis) - before;
    assert!((stepped - 0.25).abs() < 1e-5, "{stepped}");
    let before = elapsed(&nis);
    let _ = frame(&mut nis);
    assert_eq!(elapsed(&nis), before);
}
