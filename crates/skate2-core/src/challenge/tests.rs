use super::{
    ChallengeSystem, Event, Output,
    attrib_db::AttribDb,
    cas_unlock_parts::{self, CasUnlockPartDb},
    signpost::CompletionResults,
    signup_poster::{PosterAccept, PosterView},
    unlock_overlay::UnlockOverlayView,
};
use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
use std::sync::Arc;

mod career_map;
mod dynamic_hulls;

fn mounts() -> MountSet {
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for path in ["data\\db\\db.big", "data\\big\\miscload.big", "data\\content\\createacharacter.big"] {
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == path).unwrap();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
    }
    mounts
}

fn career_db() -> Arc<AttribDb> {
    Arc::new(AttribDb::load(&mounts(), &super::attrib_db::CAREER_VAULTS).unwrap())
}

/// The career's challenge system as the session starts it in game
/// (`FrontEndManager::StartInGame`).
fn career_system() -> ChallengeSystem {
    let global = mounts().read_path("data\\script\\challenge\\global.lua").unwrap();
    let mut system = ChallengeSystem::new_career(career_db(), &global).unwrap();
    system.world_mut().cas_unlock_parts = CasUnlockPartDb::parse(&mounts().read_path(cas_unlock_parts::PATH).unwrap()).unwrap();
    system.set_boot_flow_executed();
    system.start_in_game(false);
    system
}

/// One frame: the local skater's frame, the sim update, the hull loader, the
/// front end, the `PostUpdate` flush, then the HUD. A NIS the frame asked for
/// ends at once, as the NIS manager reports its end:
/// `cMsgNISChallengeNotify(1)`, then `cMsgNISComplete`.
fn frame(system: &mut ChallengeSystem) {
    let before = nis_requests(system);
    sync_local_skater(system);
    system.update(false, 1.0 / 60.0);
    answer_hull_requests(system);
    system.update_frontend();
    system.flush();
    system.post_update();
    system.update_hud();
    for _ in before..nis_requests(system) {
        system.post(Event::NisChallengeNotify { notification: 1 });
        system.post(Event::NisComplete);
    }
}

/// The skater manager's local skater (player 0), as the career session gives the system each
/// update: the scripted player's position and trigger tester. Other skaters the test added stay.
fn sync_local_skater(system: &mut ChallengeSystem) {
    use super::mission::MissionFrame;
    let world = system.world_mut();
    if !world.skaters.iter().any(|frame| frame.player == 0) {
        world.skaters.push(MissionFrame { player: 0, local: true, is_regular_stance: true, ..MissionFrame::default() });
    }
    let (position, tester) = (world.player.position, world.player.tester_base);
    if let Some(frame) = world.skaters.iter_mut().find(|frame| frame.player == 0) {
        frame.position = position;
        frame.tester_valid = tester.is_some();
        frame.tester_base = tester.unwrap_or(position);
    }
    world.skaters.sort_by_key(|frame| frame.player);
}

/// The asset manager's hull loader, which the career session runs after each update: every hull
/// asked for loads (or unloads) at once and answers `OnComplete`.
fn answer_hull_requests(system: &mut ChallengeSystem) {
    loop {
        let outputs = std::mem::take(&mut system.world_mut().outputs);
        let (hulls, rest): (Vec<_>, Vec<_>) =
            outputs.into_iter().partition(|output| matches!(output, Output::LoadHull { .. } | Output::UnloadHull { .. }));
        system.world_mut().outputs = rest;
        if hulls.is_empty() {
            break;
        }
        for hull in hulls {
            if let Output::LoadHull { hull, .. } | Output::UnloadHull { hull } = hull {
                super::assets::on_complete(system.world_mut(), hull);
            }
        }
    }
}

fn nis_requests(system: &ChallengeSystem) -> usize {
    system.outputs().iter().filter(|o| matches!(o, Output::PlayNis { .. })).count()
}

/// A frame whose started speech ends at once, as without voice playback;
/// the outputs are kept.
fn frame_speaking(system: &mut ChallengeSystem, outputs: &mut Vec<Output>) {
    frame(system);
    let new = system.take_outputs();
    for o in &new {
        if let Output::PlayNpcSpeech { character, id, .. } = o {
            system.post(Event::SpeechEnded { character: *character, id: *id });
        }
    }
    outputs.extend(new);
}

/// A speaking frame whose replay-controlled scene ends at once, as `frame` ends a basic NIS.
fn frame_replaying(system: &mut ChallengeSystem, outputs: &mut Vec<Output>) {
    let before = outputs.len();
    frame_speaking(system, outputs);
    for _ in outputs[before..].iter().filter(|o| matches!(o, Output::RunReplayNis { .. })) {
        system.post(Event::NisChallengeNotify { notification: 1 });
        system.post(Event::NisComplete);
    }
}

fn errors(outputs: &[Output]) -> Vec<&Output> {
    outputs.iter().filter(|o| matches!(o, Output::ScriptError { .. })).collect()
}

/// The Vault ships each progression handler as source and as bytecode built
/// offline with the challenge-handler wrapper
/// (`function %s( self, message, vars ) \n%s\n end`, string `0x8230bdec`).
/// Compiling every source that way must reproduce the shipped functions.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn compiler_reproduces_every_shipped_progression_handler() {
    use super::keys::{class, hash64};
    use skate2_formats::lua_chunk::{self, Prototype};
    fn shape(p: &Prototype) -> (Vec<u32>, Vec<lua_chunk::Constant>, u8, u8, u8, u8, usize) {
        (p.code.clone(), p.constants.clone(), p.parameter_count, p.vararg, p.max_stack_size, p.upvalue_count, p.prototypes.len())
    }
    let db = career_db();
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for &handler in db.collections(class::PROGRESSION_HANDLERS) {
        let name = db.attribute(class::PROGRESSION_HANDLERS, handler, hash64("Name")).and_then(|a| a.text(0)).unwrap();
        let source = db.attribute(class::PROGRESSION_HANDLERS, handler, 0x6391_aceab5a0_3cd2).and_then(|a| a.text(0)).unwrap();
        let blob = db.attribute(class::PROGRESSION_HANDLERS, handler, 0xc3cd_4b41_8327_4819).and_then(|a| a.blob()).unwrap();
        let (shipped, _) = lua_chunk::parse(blob).unwrap();
        let wrapped = format!("function h_{:016X}( self, message, vars ) \n{source}\n end", hash64(name));
        let compiled = super::lua::compiler::compile(wrapped.as_bytes(), "handler").unwrap_or_else(|e| panic!("{handler:016x}: {e}"));
        checked += 1;
        let (a, b) = (&compiled.prototypes[0], &shipped.prototypes[0]);
        if shape(a) != shape(b) || shape(&compiled) != shape(&shipped) {
            if mismatches.is_empty() {
                eprintln!("main compiled {:?}\nmain shipped  {:?}", shape(&compiled), shape(&shipped));
                eprintln!("fn compiled {:x?}\nfn shipped  {:x?}", shape(a), shape(b));
            }
            mismatches.push(handler);
        }
    }
    assert_eq!(checked, 910);
    assert!(mismatches.is_empty(), "{} of {checked} differ, first {:016x}", mismatches.len(), mismatches[0]);
}

/// New career: the Start State Machine plays its NIS, asks "New to Skate?",
/// plays the tutorial FMV on Yes, then executes m_1a_01_goto_slappys.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn new_career_start_state_machine_reaches_first_mission() {
    let mut system = career_system();
    // StartInGame: phonelist was made active after main, so it is in front.
    let hud = &system.world().hud;
    let hash = super::keys::hash_value32;
    assert_eq!(hud.active, [hash("phonelist"), hash("main")]);
    assert!(hud.group("main").is_some_and(|g| g.instances.iter().any(|i| i.component == "minimap" && i.shown)));
    system.set_enable_progression(true);
    // The machine's NIS ends the frame after it starts; its notify moves the
    // machine on to the dialog.
    for _ in 0..4 {
        frame(&mut system);
    }
    let outputs = system.take_outputs();
    assert!(errors(&outputs).is_empty(), "{outputs:#?}");
    assert!(outputs.iter().any(|o| matches!(o, Output::PlayNis { scene, .. } if !scene.is_empty())), "{outputs:#?}");
    let dialog = outputs.iter().find_map(|o| match o {
        Output::ShowDialog { title, text, .. } => Some((title.clone(), text.clone())),
        _ => None,
    });
    assert_eq!(dialog, Some(("ID_MSG_TITLE_BASICS".to_string(), "ID_MSG_INFO_BASICS".to_string())), "{outputs:#?}");
    assert_eq!(system.state_machine_names(), vec!["Start State Machine".to_string()]);

    system.post(Event::HudYes);
    system.flush();
    let outputs = system.take_outputs();
    assert!(errors(&outputs).is_empty(), "{outputs:#?}");
    // FMV_Basics waits for `tutorial_skate01`'s cMsgFMVFinishedPlaying.
    assert!(outputs.iter().any(|o| matches!(o, Output::PlayMovie { key: 0x67e4_900b_0507_15b4, .. })), "{outputs:#?}");
    assert_eq!(system.state_machine_names(), vec!["Start State Machine".to_string()]);
    system.post(Event::FmvFinishedPlaying);
    system.flush();
    let outputs = system.take_outputs();
    assert!(errors(&outputs).is_empty(), "{outputs:#?}");
    assert!(system.state_machine_names().is_empty(), "Start State Machine terminates without save");
    assert!(!outputs.contains(&Output::AutoSave));
    assert_eq!(system.main_challenge().as_deref(), Some("m_1a_01_goto_slappys"));
}

/// One progression message caught after a frame, with no script errors
/// and nothing left unpresented.
fn send(system: &mut ChallengeSystem, name: &str) -> Vec<Output> {
    super::signpost::send_message_by_name(system.world_mut(), name);
    frame(system);
    let outputs = system.take_outputs();
    assert!(errors(&outputs).is_empty(), "{name}: {outputs:#?}");
    assert!(!outputs.iter().any(|o| matches!(o, Output::Unpresented { .. })), "{name}: {outputs:#?}");
    outputs
}

/// Message types 4, 5, 7 and 16 as `HandleCaughtMessage` takes them with
/// `kEnableSignPostMedia` on: NPC speech, Skate 101, the cell-phone setting,
/// and BLOG and news entries added and taken away.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn progression_messages_speak_show_skate101_and_edit_the_blog() {
    use super::{
        keys::{class, hash64},
        signpost_progress::blog_type,
        system::Skate101View,
    };
    // The start state machine waits in MsgBox for the "New to Skate?"
    // answer, so nothing else runs.
    let mut system = career_system();
    system.set_enable_progression(true);
    for _ in 0..3 {
        frame(&mut system);
    }
    system.take_outputs();
    let outputs = send(&mut system, "rem_line_before_map_fmv");
    assert!(outputs.contains(&Output::PlayNpcSpeech { character: 40, id: 5241, param: 0 }), "{outputs:#?}");
    let outputs = send(&mut system, "201_Follow_Slappy");
    let view = Skate101View { title: "ID_201_TITLE_FOLLOW_SLAPPY".into(), text: "ID_201_TEXT_FOLLOW_SLAPPY".into() };
    assert!(outputs.contains(&Output::ShowSkate101(view)), "{outputs:#?}");
    let before = system.world().signpost.message_setting_value("shingo01");
    send(&mut system, "shingo01");
    let after = system.world().signpost.message_setting_value("shingo01");
    assert_eq!(after, before.map(|v| v.max(1)));
    // A progression message's key, by its Name.
    let key = |system: &ChallengeSystem, name: &str| {
        let db = &system.world().db;
        let messages = class::PROGRESSION_MESSAGES;
        db.collections(messages)
            .iter()
            .copied()
            .find(|&k| db.attribute(messages, k, hash64("Name")).and_then(|a| a.text(0)) == Some(name))
            .unwrap()
    };
    let blog = |system: &ChallengeSystem| system.world().signpost.progress.blog.clone();
    let last = blog(&system).values().map(|b| b.order).max().unwrap_or(0);
    send(&mut system, "TestBLOGAdd");
    send(&mut system, "Blog_t_1a_01_1up_add");
    let entries = blog(&system);
    let first = (blog_type::MESSAGE, key(&system, "TestBLOGAdd"));
    let second = (blog_type::MESSAGE, key(&system, "Blog_t_1a_01_1up_add"));
    assert_eq!((entries[&first].order, entries[&first].new, entries[&second].order), (last + 1, true, last + 2));
    // TestBLOGRemove carries TestBLOGAdd's HALText: that entry leaves and
    // the later one moves down.
    send(&mut system, "TestBLOGRemove");
    let entries = blog(&system);
    assert!(!entries.contains_key(&first));
    assert_eq!(entries[&second].order, last + 1);
    let news = key(&system, "NewsArticle1BothAdd");
    send(&mut system, "NewsArticle1BothAdd");
    assert!(system.world().signpost.progress.news.contains(&news));
    send(&mut system, "NewsArticle1BothRemove");
    assert!(!system.world().signpost.progress.news.contains(&news));
}

/// "New to Skate?" answered No (the dialog's default option): the
/// MsgBox state's cMsgHUDNo handler executes m_1a_01_goto_slappys without
/// the tutorial FMV.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn new_career_declining_the_basics_skips_the_fmv() {
    let mut system = career_system();
    system.set_enable_progression(true);
    for _ in 0..4 {
        frame(&mut system);
    }
    let outputs = system.take_outputs();
    assert!(outputs.iter().any(|o| matches!(o, Output::ShowDialog { .. })), "{outputs:#?}");
    system.post(Event::HudNo);
    system.flush();
    let outputs = system.take_outputs();
    assert!(errors(&outputs).is_empty(), "{outputs:#?}");
    assert!(!outputs.iter().any(|o| matches!(o, Output::PlayMovie { .. })), "{outputs:#?}");
    assert!(system.state_machine_names().is_empty(), "Start State Machine terminates without save");
    assert_eq!(system.main_challenge().as_deref(), Some("m_1a_01_goto_slappys"));
    // The mission's presentation adds its teleport icon at the challenge
    // locator (presentation event 30).
    frame(&mut system);
    let outputs = system.take_outputs();
    let icon = Output::ChallengeMapIcon {
        icon: "mini_teleport".into(),
        location: Some("m_1a_01_goto_slappys_challengelocator01".into()),
    };
    assert!(outputs.contains(&icon), "{outputs:#?}");
    assert!(!outputs.iter().any(|o| matches!(o, Output::Unpresented { kind: -1, .. })), "{outputs:#?}");
}

/// The first mission's state graph from `initial` through its HUD group,
/// as far as the ported natives reach.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn first_mission_state_graph_runs() {
    let mut system = career_system();
    system.set_enable_progression(true);
    for _ in 0..3 {
        frame(&mut system);
    }
    system.post(Event::HudYes);
    system.flush();
    // FMV_Basics' Tutorial_Skate01 ends.
    system.post(Event::FmvFinishedPlaying);
    system.flush();
    let mut states = Vec::new();
    for _ in 0..30 {
        frame(&mut system);
        if let Some(state) = system.main_challenge_state()
            && states.last() != Some(&state)
        {
            states.push(state);
        }
    }
    let outputs = system.take_outputs();
    eprintln!("states {states:?}");
    for o in &outputs {
        eprintln!("{o:?}");
    }
    assert!(states.contains(&"challenge_running".to_string()), "{states:?}");
}

/// Drives the first mission's objectives with a scripted player: on board and
/// moving, entering each objective's exit volume in turn.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn first_mission_objectives_progress() {
    use super::objectives::{PlayerState, Status};
    let mut system = career_system();
    system.set_enable_progression(true);
    for _ in 0..3 {
        frame(&mut system);
    }
    system.post(Event::HudYes);
    system.flush();
    // FMV_Basics' Tutorial_Skate01 ends.
    system.post(Event::FmvFinishedPlaying);
    system.flush();
    for _ in 0..30 {
        frame(&mut system);
    }
    assert_eq!(system.main_challenge_state().as_deref(), Some("challenge_running"));
    system.world_mut().player =
        PlayerState { exists: true, onboard_or_hippy_jumping: true, surface_speed: 3.0, on_ground: true, ..PlayerState::default() };
    let summary = |s: &ChallengeSystem| -> String {
        s.objective_summary().iter().map(|(n, st)| format!("{}={st:?}", n.trim_start_matches("m_1a_01_"))).collect::<Vec<_>>().join(" ")
    };
    eprintln!("start: {}", summary(&system));
    // Exit volumes of the first objectives' completion conditions.
    let volumes: [(u64, &str); 5] = [
        (1_606_811_755_185_926, "push"),
        (1_606_807_460_218_630, "minimap"),
        (1_414_276_961_240_838, "duck"),
        (1_606_953_489_106_694, "buffer"),
        (1_606_962_079_041_286, "stairs"),
    ];
    for (volume, name) in volumes {
        for _ in 0..5 {
            frame(&mut system);
        }
        system.post(Event::TriggerCollision { enter: true, volume, player: 0 });
        frame(&mut system);
        eprintln!("after {name}: {}", summary(&system));
    }
    let outputs = system.take_outputs();
    for o in outputs.iter().filter(|o| matches!(o, Output::ScriptError { .. })) {
        eprintln!("{o:?}");
    }
    let done: Vec<String> =
        system.objective_summary().into_iter().filter(|(_, s)| *s == Status::Complete).map(|(n, _)| n).collect();
    assert!(done.iter().any(|n| n.ends_with("_push")), "{done:?}");
}

/// The first grab of the `grabs` trick group (or of a group descending from
/// it), as `mTrickAttribKey`.
fn a_grab(system: &ChallengeSystem) -> u64 {
    use super::tricks::{TRICK_GROUP_CLASS, TRICKS as tricks};
    let db = &system.world.db;
    let grabs = super::keys::hash64("grabs");
    let mut groups = vec![grabs];
    groups.extend(db.collections(TRICK_GROUP_CLASS).iter().copied().filter(|&k| db.parent(TRICK_GROUP_CLASS, k) == Some(grabs)));
    groups
        .into_iter()
        .find_map(|g| db.attribute(TRICK_GROUP_CLASS, g, tricks).and_then(|a| a.ref_spec(0)).map(|(_, k)| k))
        .expect("a grab trick")
}

/// The whole first mission with a scripted player, through its results and
/// `Trigger_m_1a_01_goto_slappys` to the autosave; its outputs.
fn complete_first_mission(system: &mut ChallengeSystem) -> Vec<Output> {
    use super::objectives::PlayerState;
    use super::system::{Field, Message};
    system.set_enable_progression(true);
    for _ in 0..3 {
        frame(system);
    }
    system.post(Event::HudYes);
    system.flush();
    // FMV_Basics' Tutorial_Skate01 ends.
    system.post(Event::FmvFinishedPlaying);
    system.flush();
    for _ in 0..30 {
        frame(system);
    }
    assert_eq!(system.main_challenge_state().as_deref(), Some("challenge_running"));
    let riding = PlayerState { exists: true, onboard_or_hippy_jumping: true, surface_speed: 3.0, on_ground: true, ..PlayerState::default() };
    system.world_mut().player = riding.clone();
    let grab = a_grab(&system);
    let mut outputs = Vec::new();
    let mut frames = |s: &mut ChallengeSystem, n: usize, outputs: &mut Vec<Output>| {
        for _ in 0..n {
            frame_replaying(s, outputs);
        }
    };
    let marker = |name: &'static str| Event::Challenge(Message::new(name, vec![("PlayerID", Field::Number(0.0))]));
    for volume in [1_606_811_755_185_926, 1_606_807_460_218_630, 1_414_276_961_240_838, 1_606_953_489_106_694, 1_606_962_079_041_286] {
        frames(system, 5, &mut outputs);
        // cTriggerVolumeManager::Update runs after Sk8ChallengeSystem::Update.
        system.update(false, 1.0 / 60.0);
        system.post(Event::TriggerCollision { enter: true, volume, player: 0 });
        system.update_frontend();
        system.flush();
    }
    frames(system, 1, &mut outputs);
    // Ollie: standing in the landing volume.
    system.world_mut().player.colliding_triggers = vec![(1_414_315_615_946_502, 0)];
    frames(system, 2, &mut outputs);
    system.world_mut().player.colliding_triggers.clear();
    // Moveable: with no DMO locator the tester is never in either freedom volume.
    frames(system, 2, &mut outputs);
    system.world_mut().player.colliding_triggers = vec![(1_414_328_500_848_390, 0)];
    frames(system, 2, &mut outputs);
    system.world_mut().player.colliding_triggers.clear();
    system.post(marker("cMsgSetMarker"));
    frames(system, 2, &mut outputs);
    system.world_mut().player.colliding_triggers = vec![(1_545_045_830_502_150, 0)];
    frames(system, 2, &mut outputs);
    system.world_mut().player.colliding_triggers.clear();
    system.post(marker("cMsgUseMarker"));
    frames(system, 2, &mut outputs);
    // Drop in: launch volume, a grab in the air, the landing volume, then the
    // sequence completes.
    system.world_mut().player.colliding_triggers = vec![(1_610_419_527_714_566, 0)];
    frames(system, 1, &mut outputs);
    system.world_mut().player.colliding_triggers.clear();
    system.world_mut().player.tricks_ended = vec![grab];
    frames(system, 1, &mut outputs);
    system.world_mut().player.tricks_ended.clear();
    system.world_mut().player.colliding_triggers = vec![(1_748_296_567_850_758, 0)];
    frames(system, 1, &mut outputs);
    system.world_mut().player.sequence_completed_successfully = true;
    frames(system, 1, &mut outputs);
    system.world_mut().player = riding;
    for _ in 0..6000 {
        frame_replaying(system, &mut outputs);
        if outputs.contains(&Output::AutoSave) {
            break;
        }
    }
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    outputs
}

/// The first mission completes and autosaves, and the SignPost savable
/// reloads into a fresh career with the same progression.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn first_mission_completes_and_autosaves() {
    use super::signpost_progress::{self, Leaf};
    let mut system = career_system();
    let outputs = complete_first_mission(&mut system);
    // The ollie's and the drop-in's completion events replay their last atol("3.5") and
    // atol("4.5") seconds, skippable and without a camera hold. The scripted player completes
    // the objectives within a few frames, so later groups overflow the director's eight-group
    // queue (StartPresentation 0x82e07bc0) and the drop-in's replay need not play here.
    let replays: Vec<(String, u32)> = outputs
        .iter()
        .filter_map(|o| match o {
            Output::RunReplayNis { scene, start, end, hold_camera: false, skip_enabled: true, .. } => {
                Some((scene.clone(), end.wrapping_sub(*start)))
            }
            _ => None,
        })
        .collect();
    assert_eq!(replays.first(), Some(&("highlight_generic_left".to_owned(), 180)), "{replays:?}");
    let expected = [("highlight_generic_left", 180), ("highlight_generic_front", 240)];
    assert!(replays.iter().all(|(scene, frames)| expected.contains(&(scene.as_str(), *frames))), "{replays:?}");
    let db = Arc::clone(&system.world.db);
    let name = |k: u64| db.attribute(super::keys::class::PROGRESSION_CHALLENGES, k, super::keys::hash64("Name")).and_then(|a| a.text(0)).map(str::to_owned);
    let completed: Vec<_> = system.world.signpost.completed.keys().map(|&k| name(k)).collect();
    assert_eq!(completed, [Some("m_1a_01_goto_slappys".to_owned())]);
    let next = system.world.signpost.available.iter().find(|(k, _)| name(**k).as_deref() == Some("m_1a_03_tutorial_fliptricks"));
    assert_eq!(next.map(|(_, &new)| new), Some(true));

    // The SignPost savable reloads into a fresh career with the same
    // progression; a reload's save is stable.
    let leaves = signpost_progress::save(&system.world);
    let reload = |leaves: &[Leaf; 3]| {
        let mut loaded = career_system();
        let saved = |i: usize| Some((leaves[i].1, leaves[i].2.as_slice()));
        signpost_progress::load(&mut loaded.world, saved(0), saved(1), saved(2));
        loaded
    };
    let loaded = reload(&leaves);
    let (a, b) = (&system.world.signpost, &loaded.world.signpost);
    assert_eq!(a.completed, b.completed);
    assert_eq!(a.available, b.available);
    assert_eq!(a.progress.groups, b.progress.groups);
    assert_eq!(a.progress.group_counts, b.progress.group_counts);
    assert_eq!(a.progress.point_pools, b.progress.point_pools);
    assert_eq!(a.progress.rankings, b.progress.rankings);
    assert_eq!(a.progress.challenge_save, b.progress.challenge_save);
    assert_eq!(a.progress.blog, b.progress.blog);
    assert_eq!(a.progress.news, b.progress.news);
    // The native writes each completed exit's number through a point pool
    // lookup, so a reload finds none of them.
    let exits_after_reload: Vec<u64> = if a.progress.group_exits.is_empty() { Vec::new() } else { vec![0] };
    assert_eq!(b.progress.group_exits.iter().copied().collect::<Vec<_>>(), exits_after_reload);
    let again = signpost_progress::save(&loaded.world);
    assert_eq!(signpost_progress::save(&reload(&again).world), again);
}

/// `FrontEndState_MainMenu::EnterGame` continues a career without a saved
/// orientation at the `fe_locations` start's location name.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn continued_career_start_location_is_authored() {
    let db = career_db();
    assert_eq!(db.layout_text(super::keys::class::FE_LOCATIONS, 0x2e33_dbe1_d6b3_3472, 1), Some("J_Gamestart"));
}

/// A career autosave through the profile store: the reopened file is no
/// longer a new profile and its SignPost savable reloads the progression.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn career_autosave_round_trips_through_the_profile_store() {
    use super::signpost_progress::{self, DATA, MESSAGES, POSITION, SAVABLE, SAVABLES_BEFORE};
    use crate::edit_skater_profile::EditSkaterProfileStore;
    use skate2_formats::skater_profile::ProfileData;
    let mut system = career_system();
    complete_first_mission(&mut system);
    let root = std::env::temp_dir().join(format!("sk82-career-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("SKATER.P");
    let mut data = ProfileData::reset("PLAYER 1", "PLAYER 1");
    data.new_profile = false;
    let leaves = signpost_progress::save(&system.world);
    EditSkaterProfileStore::open(&path).unwrap().save_career(&data, SAVABLE, &SAVABLES_BEFORE, &leaves).unwrap();
    let store = EditSkaterProfileStore::open(&path).unwrap();
    assert!(!store.is_new_profile().unwrap());
    let leaf = |name| store.load_leaf(SAVABLE, name).unwrap().unwrap();
    let (d, p, m) = (leaf(DATA), leaf(POSITION), leaf(MESSAGES));
    let mut loaded = career_system();
    signpost_progress::load(&mut loaded.world, Some((d.version, &d.tuples)), Some((p.version, &p.tuples)), Some((m.version, &m.tuples)));
    let (a, b) = (&system.world.signpost, &loaded.world.signpost);
    assert_eq!(a.completed, b.completed);
    assert_eq!(a.available, b.available);
    assert_eq!(a.progress.blog, b.progress.blog);
    assert_eq!(a.progress.point_pools, b.progress.point_pools);
    // Only the uniquely-created test directory is removed.
    std::fs::remove_dir_all(root).unwrap();
}

/// `cLocationManager` positions from `BAM_Sim.loc`, as the career session
/// loads them.
fn load_locations(system: &mut ChallengeSystem) {
    use skate2_formats::location::LocationTable;
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let mut mounts = MountSet::new();
    for path in ["data\\content\\worldbam.big", "data\\big\\globallocators.big", "data\\big\\miscload.big"] {
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == path).unwrap();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
    }
    let table = LocationTable::parse(&mounts.read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc").unwrap()).unwrap();
    for location in &table.locations {
        system.world_mut().locations.insert(super::keys::hash_value32(&location.name), location.transform);
    }
}

/// Signs up for a sign-up challenge at its location indicator: prompt, D-pad
/// up, poster, accept. Where other sign-up boxes take the tester in too, the
/// prompt lists them all and the press opens the choice, answered with this
/// challenge (`SignUpHUD::MultipleSignupsCallback`). Returns the outputs of
/// those frames.
fn sign_up_at_indicator(system: &mut ChallengeSystem, name: &str) -> (Vec<Output>, PosterView) {
    let activity = system.world.activities.find(super::keys::hash64(name)).expect("activity");
    let locator = super::keys::hash_value32(&super::signup::location_name(&system.world, activity).expect("location"));
    let group = system.world.signup.indicator_group.expect("sign-up indicators");
    let transform = system.world.indicators.group(group).expect("group").locations[&locator].transform;
    let at = [transform[12], transform[13], transform[14]];
    system.world_mut().player.tester_base = Some(at);
    let mut outputs = Vec::new();
    // The prompt, then its SIGN_UP_DELAY frames before a press counts.
    for _ in 0..40 {
        frame_speaking(system, &mut outputs);
    }
    let prompt = outputs
        .iter()
        .rev()
        .find_map(|o| match o {
            Output::SignUpPrompt { challenges } if !challenges.is_empty() => Some(challenges.clone()),
            _ => None,
        })
        .unwrap_or_default();
    assert!(prompt.iter().any(|c| c == name), "{outputs:?}");
    // Only the several-challenges popup pushes kPAUSE_STATE_FRONTEND (0x826bd738).
    assert_eq!(system.sign_up_dpad_up(), prompt.len() > 1);
    if prompt.len() > 1 {
        let before = outputs.len();
        frame_speaking(system, &mut outputs);
        let choice = outputs[before..].iter().find_map(|o| match o {
            Output::SignUpChoice { options } => options.iter().position(|c| c == name),
            _ => None,
        });
        system.sign_up_choose(Some(choice.expect("the challenge among the sign-up choices")));
    }
    for _ in 0..10 {
        frame_speaking(system, &mut outputs);
    }
    assert!(outputs.contains(&Output::OpenSignUpPoster { challenge: name.to_owned() }), "{outputs:?}");
    let poster = system.sign_up_poster_view().expect("poster view");
    // The poster's OnIntroDone: cMsgHUDIntroDone, delivered by the frame's
    // flush, slows the next update to half speed (1/60 / 0.5 per tick).
    system.post(Event::HudIntroDone);
    let before = outputs.len();
    for _ in 0..2 {
        frame_speaking(system, &mut outputs);
    }
    let slowed = Output::SetSimRate { sim_time: f32::from_bits(0x3c88_8889) / 0.5, ticks: 1 };
    assert!(outputs[before..].contains(&slowed), "{:?}", &outputs[before..]);
    assert_eq!(system.sign_up_accept(), PosterAccept::Accepted);
    let before = outputs.len();
    for _ in 0..120 {
        frame_speaking(system, &mut outputs);
    }
    // PlayerAcceptChallenge disables the slow-down.
    assert!(!outputs[before..].iter().any(|o| matches!(o, Output::SetSimRate { .. })));
    // The select's teleport moves the player once its fade-out ends.
    assert!(outputs.iter().any(|o| matches!(o, Output::TeleportPlayer { .. })), "{outputs:?}");
    system.world_mut().player.tester_base = None;
    (outputs, poster)
}

/// The first mission, its trigger machine settled, and the world loaded the
/// way the career session loads it.
fn after_first_mission() -> ChallengeSystem {
    let mut system = career_system();
    complete_first_mission(&mut system);
    let mut outputs = Vec::new();
    for _ in 0..600 {
        frame_speaking(&mut system, &mut outputs);
    }
    load_locations(&mut system);
    system.game_load();
    system.loading_reset();
    system
}

/// After the first mission, `m_1a_03` is a sign-up challenge: its box at its
/// locator shows the D-pad-up prompt, a press runs the select operations and
/// opens the poster, and accepting it fades out and starts the challenge.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn sign_up_starts_the_next_mission() {
    let mut system = after_first_mission();
    let name = "m_1a_03_tutorial_fliptricks";
    let (outputs, poster) = sign_up_at_indicator(&mut system, name);
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::SignUpPrompt { challenges: vec![name.to_owned()] }), "{outputs:?}");
    assert_eq!(system.main_challenge().as_deref(), Some(name));
    // The poster: the formatted title and description, the description
    // shown (local data), no objectives or competitors list, no fee, the
    // flip trick tutorial video offered, and ObjectiveFlow type 1 lists
    // one objective. The minimum bet only matters for betting challenges.
    let title = super::signup_poster::PosterText { text: "ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_TITLE".into(), formatted: true };
    assert_eq!(poster.title, title);
    assert!(poster.description.formatted && !poster.description.text.is_empty(), "{poster:?}");
    assert!(poster.description_enabled && !poster.objectives_enabled && !poster.competitors_enabled, "{poster:?}");
    assert!(!poster.betting && !poster.competitor_challenge, "{poster:?}");
    assert_eq!(poster.tutorial_nis.as_deref(), Some("ID_CIC_VIEW_FLIP_TRICKS"));
    assert_eq!((poster.entry_fee, poster.entry_fee_paid), (0, false), "{poster:?}");
    assert_eq!(poster.objectives, [("ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1".to_owned(), String::new())]);
    // SignUpHUD: Show(true)'s RefreshFEScreen (the data dirty) and Show's
    // own SetScreenState show the overlay with the formatted title; the
    // select's HideDPadUpHUD unbinds the component, which hides it.
    let states = &system.world().signup_hud.screen_states;
    let view = super::signup_hud::SignUpHudView { title: "ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_TITLE".into(), formatted: true, dmo: None };
    assert_eq!(states[..2], [(1, view.clone()), (1, view)], "{states:?}");
    assert_eq!(states.get(2).map(|(s, _)| *s), Some(0), "{states:?}");
}

/// The first trick of a trick group, or of a group descending from it, as
/// `mTrickAttribKey`.
fn a_trick_of(system: &ChallengeSystem, group: &str) -> u64 {
    use super::tricks::{TRICK_GROUP_CLASS, TRICKS as tricks};
    let db = &system.world.db;
    let key = super::keys::hash64(group);
    let mut groups = vec![key];
    groups.extend(db.collections(TRICK_GROUP_CLASS).iter().copied().filter(|&k| db.parent(TRICK_GROUP_CLASS, k) == Some(key)));
    groups
        .into_iter()
        .find_map(|g| db.attribute(TRICK_GROUP_CLASS, g, tricks).and_then(|a| a.ref_spec(0)).map(|(_, k)| k))
        .unwrap_or_else(|| panic!("a trick of {group}"))
}

/// Lands each trick in turn as the scripted player: the trick ends this
/// frame, then the sequence completes on the next.
fn land_tricks(system: &mut ChallengeSystem, tricks: &[u64], outputs: &mut Vec<Output>) {
    for &trick in tricks {
        for _ in 0..30 {
            frame_speaking(system, outputs);
        }
        system.world_mut().player.tricks_ended = vec![trick];
        frame_speaking(system, outputs);
        system.world_mut().player.tricks_ended.clear();
        system.world_mut().player.sequence_completed_successfully = true;
        frame_speaking(system, outputs);
        system.world_mut().player.sequence_completed_successfully = false;
    }
}

/// Runs frames until the autosave, answering the unlock overlay with A.
/// Returns what the overlay showed and the completion results it read.
fn run_to_autosave(system: &mut ChallengeSystem, outputs: &mut Vec<Output>) -> Option<(UnlockOverlayView, CompletionResults)> {
    let mut overlay = None;
    for _ in 0..6000 {
        let before = outputs.len();
        frame_speaking(system, outputs);
        if outputs[before..].contains(&Output::OpenUnlockOverlay) {
            overlay = Some((system.unlock_overlay_view(), system.world.signpost.completion.clone()));
            system.unlock_overlay_continue();
        }
        if outputs[before..].contains(&Output::AutoSave) {
            break;
        }
    }
    overlay
}

/// `m_1a_03` played with a scripted player: its five objectives (an ollie, a
/// kickflip, a pop shove-it, a 360 shove-it, a nollie) complete in sequence,
/// the challenge is won, and `Trigger_m_1a_03_tutorial_fliptricks` activates
/// the next challenges, unlocks Reda, shows the unlock overlay, plays the
/// pros' arrival NIS and autosaves.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn fliptrick_tutorial_completes_and_unlocks() {
    use super::keys::{class, hash64};
    use super::objective_hud::{objective_state, screen_state};
    use super::objectives::{PlayerState, Status};
    use super::unlockables::{category, string_hash};
    let mut system = after_first_mission();
    let name = "m_1a_03_tutorial_fliptricks";
    let (mut outputs, _) = sign_up_at_indicator(&mut system, name);
    for _ in 0..600 {
        if system.main_challenge_state().as_deref() == Some("challenge_running") {
            break;
        }
        frame_speaking(&mut system, &mut outputs);
    }
    assert_eq!(system.main_challenge_state().as_deref(), Some("challenge_running"));
    system.world_mut().player =
        PlayerState { exists: true, onboard_or_hippy_jumping: true, surface_speed: 3.0, on_ground: true, ..PlayerState::default() };
    // `challenge_running` repositions the player behind a fade; the
    // objectives are evaluated once `cMsgTeleportTransitionFinished` arrives.
    for _ in 0..600 {
        if system.world.objectives.groups.iter().any(|g| g.evaluate) {
            break;
        }
        frame_speaking(&mut system, &mut outputs);
    }
    assert!(system.world.objectives.groups.iter().any(|g| g.evaluate), "objective evaluation never enabled");
    // The objective HUD lists the one active objective of the sequence.
    frame_speaking(&mut system, &mut outputs);
    let hud = &system.world.objective_hud;
    assert!(hud.visible, "objective HUD hidden");
    assert_eq!(hud.view.num_objectives, 1);
    assert_eq!(hud.view.rows[0].title.0, "ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1");
    assert_eq!(hud.view.rows[0].state, objective_state::ACTIVE);
    assert!(hud.screen_states.iter().any(|(s, _)| *s == screen_state::INTRO), "{:?}", hud.screen_states);
    let tricks = [a_trick_of(&system, "ollie"), hash64("kickflip"), hash64("popshuvit"), hash64("360popshuvit"), a_trick_of(&system, "nollies")];
    land_tricks(&mut system, &tricks[..1], &mut outputs);
    // The completed objective shows alone while its notification window runs.
    let hud = &system.world.objective_hud;
    assert_eq!(hud.view.rows[0].state, objective_state::COMPLETE);
    // The objective's activation group showed its tip through the
    // presentation's own HUD group (`TutorialCalloutHUDEvent`).
    let tutorial = &system.world.tutorial_hud.screen_states;
    assert!(tutorial.iter().any(|(s, _)| *s == super::tutorial_hud::screen_state::SHOW), "{tutorial:?}");
    assert!(
        tutorial.iter().any(|(s, v)| *s == super::tutorial_hud::screen_state::OLLIE && v.hal_id == "ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1_TIP"),
        "{tutorial:?}"
    );
    land_tricks(&mut system, &tricks[1..], &mut outputs);
    let done = system.objective_summary().iter().filter(|(_, s)| *s == Status::Complete).count();
    let overlay = run_to_autosave(&mut system, &mut outputs);
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave; {done} objectives complete");
    let (overlay, completion) = overlay.expect("no unlock overlay");
    assert!(outputs.iter().any(|o| matches!(o, Output::PlayNis { scene, .. } if scene == "02_Pros_Arrival")), "{outputs:?}");
    let db = Arc::clone(&system.world.db);
    let title = |k: u64| db.attribute(class::PROGRESSION_CHALLENGES, k, hash64("Name")).and_then(|a| a.text(0)).map(str::to_owned);
    let completed: Vec<_> = system.world.signpost.completed.keys().filter_map(|&k| title(k)).collect();
    assert!(completed.contains(&name.to_owned()), "{completed:?}");
    let available: Vec<_> = system.world.signpost.available.keys().filter_map(|&k| title(k)).collect();
    for next in ["m_1a_04_tutorial_manuals", "m_1a_05_photo_rail", "m_2a_01_tutorial_slappy"] {
        assert!(available.contains(&next.to_owned()), "{next} not available: {available:?}");
    }
    // The overlay read Reda's unlock and the three activated challenges;
    // the tutorial pays no cash and unlocks no medal list's challenge.
    assert_eq!(overlay.characters, ["ID_COMMON_REDA"]);
    assert!(overlay.character_unlocked);
    assert_eq!(overlay.phone_numbers, ["ID_UNLOCK_PHONE_NUMBER"]);
    assert!(!overlay.numbers_unlocked && !overlay.location_unlocked && !overlay.coverage_unlocked && !overlay.property);
    assert!(!overlay.mag_minute && !overlay.rob && !overlay.throwdown && !overlay.dway && !overlay.sponsor_tape);
    assert_eq!((overlay.sponsors_to_cycle, overlay.sponsor_unlock_text.as_str()), (0, "ID_COMMON_UNKNOWN"));
    assert_eq!([overlay.prize, overlay.purse, overlay.objective_bonus, overlay.wager, overlay.sponsor_bonus, overlay.trick_callout], [0; 6]);
    assert_eq!(overlay.cash, super::signpost_progress::cash(&system.world));
    for next in ["m_1a_04_tutorial_manuals", "m_1a_05_photo_rail", "m_2a_01_tutorial_slappy"] {
        assert!(completion.unlocked_challenges.contains(&hash64(next)), "{next} not in the results");
    }
    assert!(completion.unlocked_property.is_empty());
    let reda = db.layout_text(0x91bc_a669_3efc_9aa7, hash64("reda"), 0).expect("reda characters_marquee");
    assert!(system.world.unlockables.is_unlocked(category::CHARACTERS, string_hash(reda), false));
    assert!(system.world.unlockables.is_new_unlock(category::CHARACTERS, string_hash(reda)));
}

/// `AddDefaultUnlocks` and the `UNLOCKABLES` savable: a reload restores the
/// list in the same order with the same `new` flags.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn unlockables_defaults_and_round_trip() {
    use super::unlockables::{CAPACITY, Unlockables, category, string_hash};
    let db = career_db();
    let mut unlocks = Unlockables::new(&db);
    assert!(unlocks.is_unlocked(category::CHARACTERS, string_hash("slappy"), false));
    assert!(unlocks.is_unlocked(category::GESTURES, string_hash("gesture_fliptable"), false));
    assert!(unlocks.items().iter().any(|u| u.category == category::LOCATIONS), "no default locations");
    assert!(unlocks.items().iter().all(|u| !u.is_new));
    assert!(unlocks.unlock(category::CHARACTERS, string_hash("reda"), false));
    assert!(!unlocks.unlock(category::CHARACTERS, string_hash("Reda"), false));
    let saved = unlocks.save();
    assert_eq!(saved[0].2.len(), 1 + 3 * CAPACITY);
    let mut loaded = Unlockables::new(&db);
    loaded.load(Some((saved[0].1, &saved[0].2)));
    assert_eq!(loaded.items(), unlocks.items());
    assert_eq!(loaded.save(), saved);
}

/// `NIS.GetProInfo()`: the placeholders without a main challenge or under a
/// typed one (the pro missions are types 23 and 40), else the pro its name
/// gives; no shipped activity is a `fp_u3_tutorial`, so that is Carroll.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_pro_bio_reads_the_main_challenges_pro() {
    let mut system = career_system();
    let placeholder = ["#Bad Pro", "#Brooklyn, NY", "#Goofy"].map(String::from);
    system.world_mut().activities.main = None;
    assert_eq!(system.pro_info(), placeholder);
    let activities = &system.world().activities.activities;
    let index = |name: &str| activities.iter().position(|a| a.name == name).expect(name);
    let (cardiel, grinds) = (index("m_4c_01_pro_cardiel"), index("m_1a_08_tutorial_grinds"));
    system.world_mut().activities.main = Some(cardiel);
    assert_eq!(system.pro_info(), placeholder);
    system.world_mut().activities.main = Some(grinds);
    assert_eq!(
        system.pro_info(),
        ["ID_CHEAT_PROSKATER03", "ID_CHEAT_PRODETAIL_LOCATION2", "ID_CHEAT_PRODETAIL_GOOFY"].map(String::from)
    );
}

/// The scripted player on its board, rolling on the ground.
fn riding() -> super::objectives::PlayerState {
    super::objectives::PlayerState {
        exists: true,
        onboard_or_hippy_jumping: true,
        surface_speed: 3.0,
        on_ground: true,
        ..Default::default()
    }
}

/// Frames until the main challenge's state graph enters `state`, at most `limit`.
fn run_to_state(system: &mut ChallengeSystem, state: &str, limit: usize, outputs: &mut Vec<Output>) -> bool {
    for _ in 0..limit {
        if system.main_challenge_state().as_deref() == Some(state) {
            return true;
        }
        frame_speaking(system, outputs);
    }
    system.main_challenge_state().as_deref() == Some(state)
}

/// Frames until the main challenge's objective group is evaluated, at most `limit`.
fn run_to_evaluation(system: &mut ChallengeSystem, limit: usize, outputs: &mut Vec<Output>) -> bool {
    let evaluating = |system: &ChallengeSystem| {
        let main = system.world.activities.main;
        system.world.objectives.groups.iter().any(|g| Some(g.activity) == main && g.evaluate)
    };
    for _ in 0..limit {
        if evaluating(system) {
            return true;
        }
        frame_speaking(system, outputs);
    }
    evaluating(system)
}

/// Frames that only advance the world, their outputs dropped.
fn settle(system: &mut ChallengeSystem, frames: usize) {
    let mut outputs = Vec::new();
    for _ in 0..frames {
        frame_speaking(system, &mut outputs);
    }
}

/// The script errors other than the AI layer's report that the headless system has no
/// `AISkaterManager` (no `AiLink`): `AI.EnterAiBasedChallenge`, `AI.SpawnAIAtLocation` and the
/// asset manager's character and path loads of an AI challenge make it.
fn errors_without_ai(outputs: &[Output]) -> Vec<&Output> {
    outputs.iter().filter(|o| matches!(o, Output::ScriptError { error, .. } if !error.contains("no AI link"))).collect()
}

/// A skater the skater manager holds besides the local one (an AI challenge's competitor), as
/// the career session lists every skater it updates.
fn add_skater(system: &mut ChallengeSystem, player: i32) {
    use super::mission::MissionFrame;
    let skaters = &mut system.world_mut().skaters;
    if !skaters.iter().any(|frame| frame.player == player) {
        skaters.push(MissionFrame { player, is_regular_stance: true, ..MissionFrame::default() });
        skaters.sort_by_key(|frame| frame.player);
    }
}

/// The state and points of the main challenge's objectives.
fn main_objectives(system: &ChallengeSystem) -> Vec<(super::objectives::Status, u32)> {
    let main = system.world.activities.main;
    system
        .world
        .objectives
        .groups
        .iter()
        .find(|g| Some(g.activity) == main)
        .map(|g| g.objectives.iter().map(|o| (o.state, o.total_points)).collect())
        .unwrap_or_default()
}

/// A Hermes message from the local player's physics output (`PhysOutModule_MissionSystem`).
fn local_message(name: &str, mut fields: Vec<(&'static str, super::system::Field)>) -> Event {
    fields.insert(0, ("PlayerID", super::system::Field::Number(0.0)));
    Event::Challenge(super::system::Message::new(name, fields))
}

/// The `progression_challenges` names of the available and the completed challenges.
fn progression_names(system: &ChallengeSystem) -> (Vec<String>, Vec<String>) {
    use super::keys::{class, hash64};
    let db = &system.world.db;
    let name = |k: &u64| db.attribute(class::PROGRESSION_CHALLENGES, *k, hash64("Name")).and_then(|a| a.text(0)).map(str::to_owned);
    let sp = &system.world.signpost;
    (sp.available.keys().filter_map(name).collect(), sp.completed.keys().filter_map(name).collect())
}

/// Whether the named progression challenge is available as new (`mAvailableChallengeData`).
fn available_as_new(system: &ChallengeSystem, name: &str) -> bool {
    use super::keys::{class, hash64};
    let db = &system.world.db;
    let named = |k: u64| db.attribute(class::PROGRESSION_CHALLENGES, k, hash64("Name")).and_then(|a| a.text(0)) == Some(name);
    system.world.signpost.available.iter().any(|(&k, &new)| new && named(k))
}

/// `m_1a_01` and `m_1a_03` played as their own tests play them, `Trigger_m_1a_03_tutorial_
/// fliptricks` run to its autosave, and the world settled.
fn after_fliptricks() -> ChallengeSystem {
    use super::keys::hash64;
    let mut system = after_first_mission();
    let (mut outputs, _) = sign_up_at_indicator(&mut system, "m_1a_03_tutorial_fliptricks");
    assert!(run_to_state(&mut system, "challenge_running", 600, &mut outputs), "{:?}", system.main_challenge_state());
    system.world_mut().player = riding();
    assert!(run_to_evaluation(&mut system, 600, &mut outputs), "objective evaluation never enabled");
    frame_speaking(&mut system, &mut outputs);
    let tricks =
        [a_trick_of(&system, "ollie"), hash64("kickflip"), hash64("popshuvit"), hash64("360popshuvit"), a_trick_of(&system, "nollies")];
    land_tricks(&mut system, &tricks, &mut outputs);
    assert!(run_to_autosave(&mut system, &mut outputs).is_some(), "no unlock overlay");
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    for _ in 0..600 {
        frame_speaking(&mut system, &mut outputs);
    }
    // The trigger's Final sends "pSpawn Pros Arrive" (message type 9, TeleportPlayerMoment
    // 0x825c5db0): the local player moves to m_1a_01_postchallenge_playerspawn.
    let spawn = |o: &Output| matches!(o, Output::TeleportPlayer { location, .. } if location == "m_1a_01_postchallenge_playerspawn");
    assert!(outputs.iter().any(spawn), "{outputs:?}");
    system
}

/// Wins a challenge the way presentation event 26 does (`cSignPostManager::CompleteChallenge`
/// 0x825e3098: `WinChallenge`, then `ProcessChallengeResults`), then runs frames to the
/// autosave, answering the unlock overlay. Returns the frames' outputs and what the overlay
/// showed.
fn complete_by_name(system: &mut ChallengeSystem, name: &str) -> (Vec<Output>, Option<(UnlockOverlayView, CompletionResults)>) {
    super::signpost::complete_challenge_by_name(system.world_mut(), name);
    let mut outputs = Vec::new();
    let overlay = run_to_autosave(system, &mut outputs);
    (outputs, overlay)
}

/// `after_fliptricks`, then `m_1a_05_photo_rail` won the way presentation event 26 wins a
/// challenge, standing in for the photo flow past its highlight reel (replay-controlled NIS
/// playback is not ported), and its trigger machine run to the autosave.
fn after_photo_rail() -> ChallengeSystem {
    let mut system = after_fliptricks();
    let (outputs, _) = complete_by_name(&mut system, "m_1a_05_photo_rail");
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    settle(&mut system, 300);
    system
}

/// `after_photo_rail`, then `m_1a_06_photo_puzzle` won likewise and its trigger machine run to
/// the autosave.
fn after_photo_puzzle() -> ChallengeSystem {
    let mut system = after_photo_rail();
    let (outputs, _) = complete_by_name(&mut system, "m_1a_06_photo_puzzle");
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    settle(&mut system, 300);
    system
}

/// `m_1a_04` played with a scripted player: two seconds of tail manual, two of nose manual, then
/// a flip trick out of a manual and the sequence completes. The challenge is won and
/// `Trigger_m_1a_04_tutorail_manual` only terminates, with the autosave.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn manual_tutorial_completes_and_saves() {
    use super::objectives::Status;
    let mut system = after_fliptricks();
    let name = "m_1a_04_tutorial_manuals";
    let (mut outputs, _) = sign_up_at_indicator(&mut system, name);
    assert_eq!(system.main_challenge().as_deref(), Some(name));
    assert!(run_to_state(&mut system, "challenge_running", 600, &mut outputs), "{:?}", system.main_challenge_state());
    system.world_mut().player = riding();
    assert!(run_to_evaluation(&mut system, 600, &mut outputs), "objective evaluation never enabled");
    // Manual.TailManualTime/NoseManualTime read mManualTime while the trick animation's
    // mIsTailManual (bit 26) or mIsNoseManual (bit 27) is set.
    for bits in [1 << 26, 1 << 27] {
        let player = &mut system.world_mut().player;
        player.manual_length = 1.0;
        player.manual_time = 2.5;
        player.anim_trick_bits = bits;
        for _ in 0..3 {
            frame_speaking(&mut system, &mut outputs);
        }
    }
    let flip = a_trick_of(&system, "fliptricks");
    {
        let player = &mut system.world_mut().player;
        player.anim_trick_bits = 0;
        player.tricks_ended = vec![flip];
    }
    frame_speaking(&mut system, &mut outputs);
    {
        let player = &mut system.world_mut().player;
        player.tricks_ended.clear();
        player.sequence_completed_successfully = true;
    }
    frame_speaking(&mut system, &mut outputs);
    let done: Vec<Status> = main_objectives(&system).into_iter().map(|(state, _)| state).collect();
    system.world_mut().player = riding();
    let overlay = run_to_autosave(&mut system, &mut outputs);
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave; objectives {done:?}");
    assert!(overlay.is_none());
    let (_, completed) = progression_names(&system);
    assert!(completed.contains(&name.to_owned()), "{completed:?}");
}

/// `m_1a_05_photo_rail` from its sign-up to its highlight reel: its two photographers stand as
/// players 1 and 2 (`FilmAIID`; the AI layer that spawns them has no link here), the countdown
/// repositions the player, and a sequence that enters the spot volume and grinds the rail three
/// metres completes the objective with the sequence's points (`IsSequenceNewlyCompletePoints`;
/// 600 kills it against `KilledItScore` 500). `display_photo` waits `TimeToWaitBeforeReplay`,
/// pauses the simulation and plays the highlight reel. The group holds no replay pins (the
/// replay recorder is not ported), so the reel ends at once with `cMsgHighlightReelComplete`,
/// which `display_photo` does not handle, and the photo NIS whose `cMsgNISChallengeNotify` it
/// waits for never comes.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn photo_rail_runs_to_its_highlight_reel() {
    use super::objectives::Status;
    use super::system::Field;
    let mut system = after_fliptricks();
    let name = "m_1a_05_photo_rail";
    let (mut outputs, _) = sign_up_at_indicator(&mut system, name);
    assert_eq!(system.main_challenge().as_deref(), Some(name));
    add_skater(&mut system, 1);
    add_skater(&mut system, 2);
    assert!(run_to_state(&mut system, "challenge_running", 1200, &mut outputs), "{:?}", system.main_challenge_state());
    system.world_mut().player = riding();
    assert!(run_to_evaluation(&mut system, 600, &mut outputs), "objective evaluation never enabled");
    system.post(local_message("cMsgSequenceBegin", Vec::new()));
    frame_speaking(&mut system, &mut outputs);
    system.post(Event::TriggerCollision { enter: true, volume: 1_414_882_551_629_574, player: 0 });
    frame_speaking(&mut system, &mut outputs);
    let rail = Field::Key(450_138_407_667_463);
    system.post(local_message(
        "cMsgGrindCompleted",
        vec![
            ("Wipedout", Field::Number(0.0)),
            ("UsedSessionMarker", Field::Bool(false)),
            ("GrindLength", Field::Number(3.0)),
            ("PathedSplineID", rail.clone()),
            ("LocalSplineID", rail),
        ],
    ));
    frame_speaking(&mut system, &mut outputs);
    system.post(local_message(
        "cMsgSequenceComplete",
        vec![
            ("SequenceScore", Field::Number(600.0)),
            ("StartVisualGameTick", Field::Number(0.0)),
            ("StartTick", Field::Number(0.0)),
            ("TotalTrickCount", Field::Number(1.0)),
            ("StoredTrickCount", Field::Number(1.0)),
            ("Wipedout", Field::Bool(false)),
            ("UsedSessionMarker", Field::Bool(false)),
            ("StartSimGameTick", Field::Number(0.0)),
        ],
    ));
    frame_speaking(&mut system, &mut outputs);
    // The photo objective, then the group's points objective (PointRequirement 0).
    let objectives = main_objectives(&system);
    assert_eq!(objectives.first(), Some(&(Status::Complete, 600)), "{objectives:?}");
    assert!(objectives.iter().all(|&(state, _)| state == Status::Complete), "{objectives:?}");
    assert!(run_to_state(&mut system, "display_photo", 300, &mut outputs), "{:?}", system.main_challenge_state());
    let before = outputs.len();
    for _ in 0..300 {
        frame_speaking(&mut system, &mut outputs);
    }
    assert!(outputs[before..].contains(&Output::ChallengePause { pause: true }), "{:?}", &outputs[before..]);
    assert_eq!(system.main_challenge_state().as_deref(), Some("display_photo"));
    let main = system.world.activities.main;
    let group = system.world.highlight_groups.iter().find(|g| Some(g.activity) == main).expect("highlight group");
    assert!(group.highlights.is_empty());
    assert!(errors_without_ai(&outputs).is_empty(), "{:?}", errors_without_ai(&outputs));
    assert!(!outputs.contains(&Output::AutoSave));
}

/// `m_1a_06_photo_puzzle` (available once `m_1a_05` is won) from its sign-up to its highlight
/// reel: through the launch volume (`spotvolume04`), into the landing volume (`spotvolume01`) on
/// the board, and the sequence completes, which completes the objective and awards the skater's
/// finished sequence score at once (`AwardSequencePointsImmediately`; 800 kills it against
/// `KilledItScore` 700). `display_photo` then stops where the rail's does.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn photo_puzzle_runs_to_its_highlight_reel() {
    use super::objectives::Status;
    let mut system = after_photo_rail();
    let name = "m_1a_06_photo_puzzle";
    let (mut outputs, _) = sign_up_at_indicator(&mut system, name);
    assert_eq!(system.main_challenge().as_deref(), Some(name));
    add_skater(&mut system, 1);
    add_skater(&mut system, 2);
    assert!(run_to_state(&mut system, "challenge_running", 1200, &mut outputs), "{:?}", system.main_challenge_state());
    system.world_mut().player = riding();
    assert!(run_to_evaluation(&mut system, 600, &mut outputs), "objective evaluation never enabled");
    if let Some(local) = system.world_mut().skaters.iter_mut().find(|frame| frame.player == 0) {
        local.finished_sequence_score = 800;
    }
    for volume in [2_051_877_741_233_926, 1_417_038_625_212_166] {
        system.world_mut().player.colliding_triggers = vec![(volume, 0)];
        frame_speaking(&mut system, &mut outputs);
    }
    system.world_mut().player.colliding_triggers.clear();
    system.world_mut().player.sequence_completed_successfully = true;
    frame_speaking(&mut system, &mut outputs);
    system.world_mut().player.sequence_completed_successfully = false;
    let objectives = main_objectives(&system);
    assert_eq!(objectives.first(), Some(&(Status::Complete, 800)), "{objectives:?}");
    assert!(run_to_state(&mut system, "display_photo", 300, &mut outputs), "{:?}", system.main_challenge_state());
    let before = outputs.len();
    for _ in 0..300 {
        frame_speaking(&mut system, &mut outputs);
    }
    assert!(outputs[before..].contains(&Output::ChallengePause { pause: true }), "{:?}", &outputs[before..]);
    assert_eq!(system.main_challenge_state().as_deref(), Some("display_photo"));
    assert!(errors_without_ai(&outputs).is_empty(), "{:?}", errors_without_ai(&outputs));
}

/// `m_1a_05_photo_rail` won: `Trigger_m_1a_05_photo_rail` activates `m_1a_06_photo_puzzle`,
/// shows the unlock overlay with it, then the cameraman's progression reminder 4004, and saves.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn photo_rail_win_unlocks_the_puzzle_and_saves() {
    use super::keys::hash64;
    let mut system = after_fliptricks();
    let (outputs, overlay) = complete_by_name(&mut system, "m_1a_05_photo_rail");
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    let (_, completion) = overlay.expect("no unlock overlay");
    assert!(completion.unlocked_challenges.contains(&hash64("m_1a_06_photo_puzzle")), "{completion:?}");
    assert!(outputs.contains(&Output::PlayNpcSpeech { character: 40, id: 4004, param: 0 }), "{outputs:?}");
    let (_, completed) = progression_names(&system);
    assert!(completed.contains(&"m_1a_05_photo_rail".to_owned()), "{completed:?}");
    assert!(available_as_new(&system, "m_1a_06_photo_puzzle"));
}

/// `m_1a_06_photo_puzzle` won: `Trigger_m_1a_06_photo_puzzle` activates
/// `m_1a_07_follow_slappy`, shows the unlock overlay, plays `03_Pros_Leave`, then Slappy's
/// sponsor-me line (`SponsorMeStart`, message type 4) and the "Follow Slappy" Skate 101 (type
/// 5), and saves.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn photo_puzzle_win_unlocks_follow_slappy_and_saves() {
    use super::keys::hash64;
    use super::system::Skate101View;
    let mut system = after_photo_rail();
    let (outputs, overlay) = complete_by_name(&mut system, "m_1a_06_photo_puzzle");
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    let (_, completion) = overlay.expect("no unlock overlay");
    assert!(completion.unlocked_challenges.contains(&hash64("m_1a_07_follow_slappy")), "{completion:?}");
    assert!(outputs.iter().any(|o| matches!(o, Output::PlayNis { scene, .. } if scene == "03_Pros_Leave")), "{outputs:?}");
    assert!(outputs.contains(&Output::PlayNpcSpeech { character: 40, id: 2727, param: 0 }), "{outputs:?}");
    let skate101 = Skate101View { title: "ID_201_TITLE_FOLLOW_SLAPPY".into(), text: "ID_201_TEXT_FOLLOW_SLAPPY".into() };
    assert!(outputs.contains(&Output::ShowSkate101(skate101)), "{outputs:?}");
    assert!(available_as_new(&system, "m_1a_07_follow_slappy"));
}

/// Signing up for `m_1a_07_follow_slappy` at its locator: the accept (`CheckStateTriggers`
/// result 5, `WinLose` 8) runs `Trigger_m_1a_07_follow_accept`, which unlocks Slappy's hull and
/// sets the session marker without saving, and the challenge starts.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn follow_slappy_accept_unlocks_slappys_and_sets_the_marker() {
    let mut system = after_photo_puzzle();
    let name = "m_1a_07_follow_slappy";
    let (outputs, _) = sign_up_at_indicator(&mut system, name);
    let hull = Output::DynamicHullState { challenge: "slappys".into(), state: "Unlock".into() };
    assert!(outputs.contains(&hull), "{outputs:?}");
    assert!(outputs.contains(&Output::SetSessionMarker), "{outputs:?}");
    assert!(!outputs.contains(&Output::AutoSave));
    assert!(
        !outputs.iter().any(|o| matches!(o, Output::ScriptError { context, .. } if context.starts_with("Trigger_m_1a_07"))),
        "{outputs:?}"
    );
    assert_eq!(system.main_challenge().as_deref(), Some(name));
}

/// `m_1a_07_follow_slappy` won: `Trigger_m_1a_07_follow` activates the sponsor-me tape
/// (`sponsor_me_01`) and the Hall of Meat tutorial, unlocks Slappy's hull and the Hall of Meat
/// ("Hom Available"), plays `goto_m_1b_01_goto_school` and saves.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn follow_slappy_win_opens_the_next_area_and_saves() {
    let mut system = after_photo_puzzle();
    let (outputs, overlay) = complete_by_name(&mut system, "m_1a_07_follow_slappy");
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    assert!(overlay.is_none());
    let hull = Output::DynamicHullState { challenge: "slappys".into(), state: "Unlock".into() };
    assert!(outputs.contains(&hull), "{outputs:?}");
    assert!(outputs.iter().any(|o| matches!(o, Output::PlayNis { scene, .. } if scene == "goto_m_1b_01_goto_school")), "{outputs:?}");
    assert_eq!(system.world.signpost.message_setting_value("Hom Available"), Some(1));
    let (available, completed) = progression_names(&system);
    assert!(completed.contains(&"m_1a_07_follow_slappy".to_owned()), "{completed:?}");
    for next in ["sponsor_me_01", "hom_tutorial"] {
        assert!(available.contains(&next.to_owned()), "{next} not available: {available:?}");
    }
}

/// `m_1a_07_follow_slappy` lost or abandoned: its `complete` state's `Challenge.LoseChallenge` or
/// `AbandonChallenge` (here the abandon, result 2), then `Challenge.CompleteChallenge`
/// (`ProcessChallengeResults`), run `Trigger_m_1a_07_follow_fail` (`WinLose` 4), which locks
/// Slappy's hull again, teleports the player back to the challenge locator behind the
/// one-second transition, and terminates without saving.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn follow_slappy_failure_locks_slappys_and_returns_to_the_locator() {
    use super::keys::hash64;
    let mut system = after_photo_puzzle();
    super::signpost::end_challenge_unwon(system.world_mut(), hash64("m_1a_07_follow_slappy"), 2);
    super::signpost::process_challenge_results(system.world_mut());
    let mut outputs = Vec::new();
    for _ in 0..240 {
        frame_speaking(&mut system, &mut outputs);
    }
    assert!(errors(&outputs).is_empty(), "{:?}", errors(&outputs));
    let hull = Output::DynamicHullState { challenge: "slappys".into(), state: "Locked".into() };
    assert!(outputs.contains(&hull), "{outputs:?}");
    let locator =
        |o: &Output| matches!(o, Output::TeleportPlayer { location, .. } if location == "m_1a_07_follow_slappy_challengelocator01");
    assert!(outputs.iter().any(locator), "{outputs:?}");
    assert!(!outputs.contains(&Output::AutoSave));
    let machines = system.state_machine_names();
    assert!(!machines.iter().any(|m| m == "Trigger_m_1a_07_follow_fail"), "{machines:?}");
    let (available, completed) = progression_names(&system);
    assert!(available.contains(&"m_1a_07_follow_slappy".to_owned()));
    assert!(!completed.contains(&"m_1a_07_follow_slappy".to_owned()));
}

/// A frame as the career session runs one: the simulation's tick advances (`Simulation::mTick`,
/// `Time.GetVisualGameTick`), then the frame, then `DataSysMan::SubmitFrame` records it.
fn recorded_frame(system: &mut ChallengeSystem, outputs: &mut Vec<Output>) {
    system.world_mut().tick += 1;
    frame_speaking(system, outputs);
    system.store_replay_frame();
}

/// `m_1a_05_photo_rail` with the replay recorder running: the sequence's highlight is pinned
/// from its start tick to the frame written when the objective completed (plus the
/// definition's future frames), `display_photo` plays it (`NISManager::RunReplayNIS` over the
/// pin), the photo outro's `cMsgNISChallengeNotify(2)` opens the photo selection and moves to
/// `end_options`, and the pick's `cMsgNISComplete` ends the reel.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn photo_rail_replays_its_highlight_and_opens_the_photo_selection() {
    use super::system::Field;
    let mut system = after_fliptricks();
    let name = "m_1a_05_photo_rail";
    let (mut outputs, _) = sign_up_at_indicator(&mut system, name);
    add_skater(&mut system, 1);
    add_skater(&mut system, 2);
    assert!(run_to_state(&mut system, "challenge_running", 1200, &mut outputs), "{:?}", system.main_challenge_state());
    system.world_mut().player = riding();
    assert!(run_to_evaluation(&mut system, 600, &mut outputs), "objective evaluation never enabled");
    for _ in 0..240 {
        recorded_frame(&mut system, &mut outputs);
    }
    let begin = system.world.tick + 1;
    system.post(local_message("cMsgSequenceBegin", Vec::new()));
    recorded_frame(&mut system, &mut outputs);
    system.post(Event::TriggerCollision { enter: true, volume: 1_414_882_551_629_574, player: 0 });
    for _ in 0..90 {
        recorded_frame(&mut system, &mut outputs);
    }
    let rail = Field::Key(450_138_407_667_463);
    system.post(local_message(
        "cMsgGrindCompleted",
        vec![
            ("Wipedout", Field::Number(0.0)),
            ("UsedSessionMarker", Field::Bool(false)),
            ("GrindLength", Field::Number(3.0)),
            ("PathedSplineID", rail.clone()),
            ("LocalSplineID", rail),
        ],
    ));
    recorded_frame(&mut system, &mut outputs);
    system.post(local_message(
        "cMsgSequenceComplete",
        vec![
            ("SequenceScore", Field::Number(600.0)),
            ("StartVisualGameTick", Field::Number(0.0)),
            ("StartTick", Field::Number(0.0)),
            ("TotalTrickCount", Field::Number(1.0)),
            ("StoredTrickCount", Field::Number(1.0)),
            ("Wipedout", Field::Bool(false)),
            ("UsedSessionMarker", Field::Bool(false)),
            ("StartSimGameTick", Field::Number(0.0)),
        ],
    ));
    recorded_frame(&mut system, &mut outputs);
    let main = system.world.activities.main;
    let group = system.world.highlight_groups.iter().find(|g| Some(g.activity) == main).expect("highlight group");
    assert_eq!(group.highlights.len(), 1, "the objective's highlight is pinned");
    let pin = group.highlights[0].pin;
    let (start, end) = (system.world.replay.pin_start_frame(pin), system.world.replay.pin_end_frame(pin));
    assert!(start.is_some_and(|start| start <= begin), "{start:?} from {begin}");
    let before = outputs.len();
    let mut replay = None;
    for _ in 0..600 {
        recorded_frame(&mut system, &mut outputs);
        replay = outputs[before..].iter().find_map(|o| match o {
            Output::RunReplayNis { scene, start, end, .. } => Some((scene.clone(), *start, *end)),
            _ => None,
        });
        if replay.is_some() {
            break;
        }
    }
    let (scene, replay_start, replay_end) = replay.expect("the highlight reel plays its pin");
    assert!(!scene.is_empty());
    assert_eq!(Some(replay_start), start);
    assert!(replay_end > replay_start && end.is_some_and(|end| replay_end >= end), "{replay_start}..{replay_end}");
    assert!(outputs[before..].contains(&Output::ChallengePause { pause: true }));
    assert_eq!(system.main_challenge_state().as_deref(), Some("display_photo"));
    // The photo outro reaches its photo view: cMsgNISChallengeNotify(2).
    system.post(Event::NisChallengeNotify { notification: 2 });
    let before = outputs.len();
    frame_speaking(&mut system, &mut outputs);
    assert!(outputs[before..].iter().any(|o| matches!(o, Output::OpenPhotoSelection { .. })), "{:?}", &outputs[before..]);
    assert_eq!(system.main_challenge_state().as_deref(), Some("end_options"));
    // PhotoDone's cMsgNISComplete: the one highlight played, the reel completes.
    system.post(Event::NisComplete);
    frame_speaking(&mut system, &mut outputs);
    let group = system.world.highlight_groups.iter().find(|g| Some(g.activity) == main).expect("highlight group");
    assert_eq!(group.playback, super::players::PlaybackState::NotPlaying);
    // Continue on the end options: back to real time, the simulation unpaused, the NIS killed.
    let before = outputs.len();
    system.post(Event::HudContinue);
    frame_speaking(&mut system, &mut outputs);
    for expected in [Output::EnterRealTime, Output::ChallengePause { pause: false }, Output::KillNis] {
        assert!(outputs[before..].contains(&expected), "{expected:?}: {:?}", &outputs[before..]);
    }
    assert!(errors_without_ai(&outputs).is_empty(), "{:?}", errors_without_ai(&outputs));
}

/// `sponsor_me_01`, the first spot film, left to run out of film: the film timer expires, the
/// challenge cleans up into `killed_it`, fails without a replay (the 1.5 s central-message wait,
/// then `end_options`), and Continue returns to real time and ends it, all on ported natives.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn sponsor_me_film_runs_out_of_film_and_continues() {
    let mut system = after_photo_puzzle();
    let (outputs, _) = complete_by_name(&mut system, "m_1a_07_follow_slappy");
    assert!(outputs.contains(&Output::AutoSave), "no autosave");
    settle(&mut system, 300);
    let name = "sponsor_me_01";
    let (mut outputs, _) = sign_up_at_indicator(&mut system, name);
    assert_eq!(system.main_challenge().as_deref(), Some(name));
    add_skater(&mut system, 1);
    add_skater(&mut system, 2);
    assert!(
        run_to_state(&mut system, "challenge_running", 2400, &mut outputs),
        "{:?} {:?}",
        system.main_challenge_state(),
        errors_without_ai(&outputs)
    );
    system.world_mut().player = riding();
    let mut states: Vec<String> = Vec::new();
    for _ in 0..(60 * 600) {
        recorded_frame(&mut system, &mut outputs);
        if let Some(state) = system.main_challenge_state()
            && states.last() != Some(&state)
        {
            states.push(state);
        }
        if states.last().is_some_and(|state| state == "end_options") {
            break;
        }
    }
    assert_eq!(states, ["challenge_running", "killed_it", "end_options"]);
    let failed = |o: &Output| {
        matches!(o, Output::CentralMessage(message) if message.hal_id == "ID_CHALLENGE_CENTRAL_MESSAGE_OBJECTIVE_CHALLENGEFAILED")
    };
    assert!(outputs.iter().any(failed), "{outputs:?}");
    assert!(!outputs.iter().any(|o| matches!(o, Output::RunReplayNis { .. })), "a failed film plays no reel");
    let before = outputs.len();
    system.post(Event::HudContinue);
    for _ in 0..60 {
        frame_speaking(&mut system, &mut outputs);
    }
    assert!(outputs[before..].contains(&Output::EnterRealTime), "{:?}", &outputs[before..]);
    let errors = errors_without_ai(&outputs);
    assert!(errors.is_empty(), "{states:?}: {errors:?}");
}

/// The online "Can You Spell Girl?" lists the career challenge's own hull as its required one,
/// which an online game loads with asset flags 7 for `Dmo::SimManager`'s static objects.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn the_online_challenge_loads_the_career_challenge_hull() {
    let db = career_db();
    let hulls = |name: &str| super::assets::challenge_hull_names(&db, super::keys::hash64(name), name);
    assert_eq!(hulls("m_4a_01_skate_carroll_koston_ol"), ["m_4a_01_skate_carroll_koston"]);
    assert_eq!(hulls("m_4a_01_skate_carroll_koston"), ["m_4a_01_skate_carroll_koston"]);
}

/// A Spot Battle (`OwnTheSpot`, 9) scores without the multiplier and a challenge can turn trick
/// kinds off; once the challenge system's game unloads, the skaters score as with no challenge.
#[test]
fn a_game_unload_gives_the_skaters_the_scoring_of_no_challenge() {
    let mut scoring = super::skate::ChallengeScoring {
        current: Some(3),
        multiplier_active: false,
        ..Default::default()
    };
    scoring.enabled[4] = false;
    scoring.game_unload();
    let inputs = scoring.pop(true);
    assert!(inputs.multiplier_active);
    assert_eq!(inputs.enabled, [true; 14]);
    assert_eq!(scoring.current, None);
}
