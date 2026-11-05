//! Adapter tests use synthetic physical parameters, not stock parity fixtures.
//! They check writes to actual BoardRuntime bodies/drives rather than a call log.
use super::*;
use board_possession::{Materials, VolumeFlags};
use native::{IDENTITY, lifecycle::Alignment};
use skate3_core::{
    math::Vector3,
    physics::{
        board_runtime::{BoardMotion, BoardRuntime},
        contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
        rigid_body::{
            RetailBodyMassProperties, RetailInertiaDynamics, RetailLocalMassFrame,
            RetailSimulationStep,
        },
    },
    point_graph::PointGraph,
};

fn owner() -> Owner {
    let graph = |value| PointGraph {
        x: [0., 1., 2., 3., 4., 5., 6., 7.],
        y: [value; 8],
    };
    Owner {
        state: native::State::default(),
        settings: Settings {
            hide_distance: 30.,
            hide_offset: 1000.,
            return_distance: 30.,
            mounted_return_distance: 5.,
            mounting_time: 0.1,
            retrieval_time: graph(1.),
            retrieval_weight: graph(0.5),
            throw_pitch: 18.,
            throw_velocity: graph(2.),
            throw_target_pitch: 20.,
            throw_pitch_scalar: 0.1,
            throw_roll_scalar: 0.1,
            throw_yaw_scalar: 0.2,
        },
    }
}

fn observation() -> Observation {
    Observation {
        processed: native::Processed {
            board_frame_64: IDENTITY,
            player_frame_192: IDENTITY,
            position_592: [0.; 4],
            velocity_912: [0.; 4],
            direction_400: [0., 0., 1., 0.],
            hide_direction_464: [0., 0., 1., 0.],
            flags_2476: 0,
            flags_2480: 0,
            flags_2488: 0,
        },
        board_collision_flags_872: 0,
        board_state_840: 0,
        hand_contacts: [false; 2],
        physical_hand_positions: [[0.; 4]; 2],
        animation_board_frame_12624: IDENTITY,
        animation_hand_frames: [IDENTITY; 2],
        attachment_frame_0: IDENTITY,
    }
}

fn board() -> BoardRuntime {
    let mass = RetailBodyMassProperties {
        local_mass_frame: RetailLocalMassFrame::IDENTITY,
        dynamics: RetailInertiaDynamics {
            inverse_tensor: Vector3::new(1., 1., 1.),
            inverse_mass: 1.,
            spherical: 0.,
            maximum_linear_velocity: 100.,
            maximum_angular_velocity: 100.,
            linear_drag: 0.,
            angular_drag: 0.,
        },
    };
    BoardRuntime::new(
        [mass; 7],
        [RetailAffineTransform::IDENTITY; 7],
        RetailAffineTransform::IDENTITY,
        RetailSimulationStep::fixed_60_hz(0, 0., Vector3::ZERO),
        BoardMotion::Active,
    )
}

#[test]
fn hold_and_release_change_real_drives_without_repositioning_board() {
    let mut owner = owner();
    let mut board = board();
    let original = board.part_transforms();
    for body in board.bodies_mut() {
        body.rates.linear_velocity = Vector3::new(1., 2., 3.);
    }
    let mut fields = SkateboardControllerFields {
        state_448: 0,
        word_444: 0,
        system_on_452: true,
    };
    let observation = observation();
    let standard = RetailContactMaterial {
        static_friction: 0.5,
        dynamic_friction: 0.4,
        restitution: 0.1,
    };
    let released = RetailContactMaterial {
        static_friction: 0.2,
        dynamic_friction: 0.1,
        restitution: 0.3,
    };
    let (mut animated, mut wiping_out, mut aligned) = (0, false, false);
    let mut alignment = Alignment {
        first_1008: [0.; 4],
        second_1024: [0.; 4],
        factor_1040: 0.,
        flag_1044: false,
    };
    let mut volumes = VolumeFlags {
        deck: true,
        trucks: true,
        wheels: true,
        deck_children: vec![true; 2],
    };
    let [mut wheels, mut trucks, mut deck] = [standard; 3];
    let mut effects = Effects {
        board: &mut board,
        animated_290: &mut animated,
        wiping_out: &mut wiping_out,
        alignment_active: &mut aligned,
        alignment: &mut alignment,
        volumes: &mut volumes,
        materials: Materials {
            wheels: &mut wheels,
            trucks: &mut trucks,
            deck: &mut deck,
        },
        standard_materials: [standard; 3],
        released_material: released,
        standard_angular_drag: 3.,
        processed_dt_2604: 1. / 60.,
    };
    owner.hold(&mut fields, &observation, &mut effects);
    assert_eq!(owner.state.selected_hand_424, 1);
    assert_eq!(owner.state.hands[0].dynamics, [[0, 0, 0, 2]; 2]);
    assert_eq!(
        owner.state.hands[1].dynamics,
        [[0x4395ffff, 0, 0x468c9fff, 2]; 2]
    );
    assert_eq!(*effects.animated_290, 1);
    assert_eq!(effects.board.part_transforms(), original);

    // Real hand and deck body IDs must be preserved in the shared row builder.
    let mut rows = Vec::new();
    let bodies = effects.board.bodies();
    owner.append_drives(
        bodies[6],
        [bodies[0], bodies[1]],
        6,
        [11, 15],
        1. / 60.,
        &mut rows,
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].frame_a_body.reaction_index, 11);
    assert_eq!(rows[1].frame_a_body.reaction_index, 15);
    assert!(rows.iter().all(|row| row.frame_b_body.reaction_index == 6));

    owner.let_go(&mut fields, &observation, &mut effects);
    assert_eq!(
        fields.state_448, 1,
        "LetGo leaves the caller's state assignment alone"
    );
    assert_eq!(owner.state.selected_hand_424, 2);
    assert!(
        owner
            .state
            .hands
            .iter()
            .all(|hand| hand.dynamics == [[0, 0, 0, 2]; 2])
    );
    assert_eq!(
        effects.board.hook_mut().drive.dynamics,
        [0, 0, 0, 2, 0, 0, 0, 2]
    );
    assert_eq!(*effects.animated_290, 0);
    assert!(*effects.wiping_out);
    assert_eq!(*effects.materials.wheels, released);
    assert_eq!(*effects.materials.trucks, released);
    assert_eq!(*effects.materials.deck, released);
    assert_eq!(effects.board.part_transforms(), original);
    assert!(
        effects
            .board
            .bodies()
            .iter()
            .all(|body| body.rates.linear_velocity == Vector3::new(1., 2., 3.))
    );

    // Returning is the explicit exception: release clears retrieval velocity.
    fields.state_448 = 4;
    owner.let_go(&mut fields, &observation, &mut effects);
    assert!(
        effects
            .board
            .bodies()
            .iter()
            .all(|body| body.rates.linear_velocity == Vector3::ZERO)
    );

    // SetPhysicsState changes state448 itself. The callback must preserve
    // LetGo's throw countdown without restoring the old returning state.
    let mut throw_observation = observation;
    throw_observation.processed.flags_2476 = 0x1000;
    let mut transition = Transition::new(&mut owner, &throw_observation, &mut effects, fields);
    transition.let_go_of_skateboard();
    fields.state_448 = 0;
    fields.system_on_452 = false;
    fields.word_444 = 0;
    transition.finish(&mut fields);
    assert_eq!(fields.state_448, 0);
    assert!(!fields.system_on_452);
    assert_eq!(fields.word_444, 8);
    assert!(
        effects
            .board
            .bodies()
            .iter()
            .all(|body| body.rates.linear_velocity.z > 0.)
    );

    //Teleport74D30 resets retrieval, not the persistent drive allocation.
    fields.system_on_452 = true;
    owner.hold(&mut fields, &throw_observation, &mut effects);
    owner.state.retrieval.elapsed_192 = 4.;
    owner.reset_for_teleport(&mut fields, &throw_observation, &mut effects);
    assert_eq!(fields.state_448, 0);
    assert!(!fields.system_on_452);
    assert_eq!(fields.word_444, 8, "Stop retains LetGo's throw write");
    assert_eq!(owner.state.selected_hand_424, 2);
    assert_eq!(owner.state.retrieval.elapsed_192, 0.);
    assert_eq!(owner.state.retrieval.initial_0, IDENTITY);
    owner.reset_for_teleport(&mut fields, &throw_observation, &mut effects);
    assert_eq!(
        fields.word_444, 0,
        "second teleport reset clears it while off"
    );
    native::lifecycle::Effects::standard_board(&mut effects);
    assert!(!*effects.wiping_out);
    assert_eq!(*effects.materials.deck, standard);
    assert!(effects.volumes.deck && effects.volumes.trucks && effects.volumes.wheels);

    //75F00 ->75440 ->764F0: release must disable physical coupling BEFORE
    //the throw velocity and subsequent eight free-board torque updates.
    fields.system_on_452 = true;
    let mut throwing = throw_observation;
    throwing.processed.flags_2476 = 0x3000;
    owner.hold(&mut fields, &throwing, &mut effects);
    owner.update(&mut fields, &throwing, &mut effects);
    assert_eq!(fields.state_448, 2);
    assert_eq!(fields.word_444, 7);
    assert_eq!(owner.state.selected_hand_424, 2);
    let mut rows = Vec::new();
    let bodies = effects.board.bodies();
    owner.append_drives(
        bodies[6],
        [bodies[0], bodies[1]],
        6,
        [11, 15],
        1. / 60.,
        &mut rows,
    );
    assert_eq!(
        rows.len(),
        2,
        "native drives remain allocated when disabled"
    );
    for row in &rows {
        assert_eq!(row.linear_maximum_impulse, [0.; 3]);
        assert_eq!(row.angular_maximum_impulse, [0.; 3]);
    }
    let mut reactions = [skate3_core::physics::rigid_body::RetailReactionCorrections::default(); 16];
    skate3_core::physics::drive_solver::solve_drive_rows(&mut rows, &mut reactions, 8);
    assert_eq!(
        reactions,
        [Default::default(); 16],
        "released hands cannot pull either assembly"
    );
    for _ in 0..7 {
        owner.update(&mut fields, &throwing, &mut effects);
    }
    assert_eq!(fields.word_444, 0);
    let velocities = effects
        .board
        .bodies()
        .map(|body| body.rates.linear_velocity);
    owner.update(&mut fields, &throwing, &mut effects);
    assert_eq!(fields.state_448, 2);
    assert_eq!(
        fields.word_444, 0,
        "throw input in FREE does not restart the throw"
    );
    assert_eq!(
        effects
            .board
            .bodies()
            .map(|body| body.rates.linear_velocity),
        velocities
    );
    assert!(
        owner
            .state
            .hands
            .iter()
            .all(|h| h.dynamics == [[0, 0, 0, 2]; 2])
    );
}
