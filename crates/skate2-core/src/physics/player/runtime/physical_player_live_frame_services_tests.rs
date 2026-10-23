use super::*;
use crate::input_cvar_runtime::InputCvarManager;
type World = SimulationAssemblyConstraintSession<(), ()>;

#[test]
fn sync_uses_the_constructed_game_info_and_actual_zero_level() {
    let mut info = PhysicalGameInfo::construct_network_field();
    let retained = Arc::clone(&info);
    assert_eq!(info.sync_level().unwrap(), 0);
    assert!(Arc::ptr_eq(&info, &retained));
    // The CRC2 fixture is deliberately not this constructor's production value.
    assert!(info.sync_level().unwrap() < 2);
}
#[test]
fn live_freezing_requires_link_and_observes_later_target_changes() {
    let mut world = World::try_new(1, 0, 0, 0).unwrap();
    let refs = InputCvarReferences::new_static_initialized();
    assert!(matches!(
        world.try_live_contact_feedback(&refs),
        Err(LiveContactFeedbackError::MissingManagerParameters)
    ));
    world.try_initialize_world_feedback_parameters().unwrap();
    assert_eq!(world.backend.rigid_bodies.cooldown(), 2);
    assert_eq!(world.backend.rigid_bodies.minimum_energy().to_bits(), 0x3727_c5ac);
    let mut input = world.try_live_contact_feedback(&refs).unwrap();
    assert!(matches!(
        input.current_enable_freezing(),
        Err(LiveContactFeedbackError::Cvar(InputCvarError::Unlinked(
            "kEnableFreezing"
        )))
    ));
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert_eq!(input.current_enable_freezing().unwrap(), 1);
    // Same live references, not a creation-time captured boolean target/value.
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkEnableFreezing=0\r\n"))
        .unwrap();
    assert_eq!(input.current_enable_freezing().unwrap(), 0);
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkEnableFreezing=1\r\n"))
        .unwrap();
    assert_eq!(input.current_enable_freezing().unwrap(), 1);
    world.backend.rigid_bodies.set_cooldown(73);
    assert_eq!(input.current_frames_before_freezing().unwrap(), 2);
    assert!(matches!(
        world.try_initialize_world_feedback_parameters(),
        Err(LiveContactFeedbackError::AlreadyInitialized)
    ));
    assert_eq!(world.backend.rigid_bodies.cooldown(), 73);
}

#[test]
fn prephysics_iterations_use_actual_flags_and_retain_store_across_target_changes() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut single = World::try_new_with_island_flags(1, 0, 0, 0, 3).unwrap();
    assert!(matches!(
        single.try_sample_prephysics_iterations(&refs),
        Err(LiveContactFeedbackError::MissingManagerParameters)
    ));
    single.try_initialize_world_feedback_parameters().unwrap();
    assert_eq!(single.simulation_max_iterations, Some(25));
    assert!(matches!(
        single.try_sample_prephysics_iterations(&refs),
        Err(LiveContactFeedbackError::Cvar(InputCvarError::Unlinked(
            "kMaxSinglePlayerIterations"
        )))
    ));
    assert_eq!(single.simulation_max_iterations, Some(25));
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let snapshot = single.try_sample_prephysics_iterations(&refs).unwrap();
    assert_eq!(snapshot, 50);
    assert_eq!(single.simulation_max_iterations, Some(snapshot));
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkMaxSinglePlayerIterations=73\r\nkMaxOnlineIterations=29\r\n",
        ))
        .unwrap();
    // A changed target does not retroactively change the prephysics store or solve snapshot.
    assert_eq!(single.simulation_max_iterations, Some(snapshot));
    assert_eq!(single.try_sample_prephysics_iterations(&refs).unwrap(), 73);
    for flags in [0, 1, 2, 7] {
        let mut other = World::try_new_with_island_flags(1, 0, 0, 0, flags).unwrap();
        other.try_initialize_world_feedback_parameters().unwrap();
        assert_eq!(other.try_sample_prephysics_iterations(&refs).unwrap(), 29);
    }
}

#[test]
fn an_online_game_solves_its_both_island_with_the_online_count() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut world = World::try_new_with_island_flags(1, 0, 0, 0, 3).unwrap();
    world.try_initialize_world_feedback_parameters().unwrap();
    assert_eq!(world.try_sample_prephysics_iterations(&refs).unwrap(), 50);
    world.sim_rate.lock().unwrap().set_online_game(true);
    assert_eq!(world.try_sample_prephysics_iterations(&refs).unwrap(), 25);
    assert_eq!(world.simulation_max_iterations, Some(25));
    world.sim_rate.lock().unwrap().set_online_game(false);
    assert_eq!(world.try_sample_prephysics_iterations(&refs).unwrap(), 50);
}
