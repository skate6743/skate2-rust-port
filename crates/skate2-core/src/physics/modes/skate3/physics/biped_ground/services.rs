//! Concrete, stateless Ground bindings to the single Skate3ControllerState owners.
//! Grab-spline queries/actions are a separate missing host, not board possession.
use super::{Skate3PhysicsState, Skate3ControllerState, sync::LaunchInput};
use skate3_core::{
    animation::foot_ik::status::Mode,
    physics::skeleton_animation_record::compose_affine,
    player::offboard::{air_launch, air_selector, biped_air::recovered::feet},
};

///82D770D8; S2 OffBoardFeetIK::ConsiderReset82DCD110 corroborates the owner.
///Do not reset targets/history merely because the board has been released.
pub(crate) fn enter_feet(skater: &mut Skate3ControllerState) {
    let p = &skater.player_input.processed;
    if !matches!(p.state_2504, 500 | 501 | 502) {
        skater.offboard_feet.reset();
        skater.foot_ik.state.enable_feet(false);
        for limb in &mut skater.foot_ik.state.limbs {
            limb.board_blend = 0.;
            limb.external_blend = 0.;
            limb.mode = Mode::Disabled;
        }
    }
    skater.offboard_feet.flags_304_to_307[0] = p.state_2508 == 502;
}

///82D773F8 reads real line tests and82BE3220 world animation-record bones.
///Same input extraction as air_feet; neither its timer reset nor Air update
///is executed. Ground conditioning/history belong to super::feet::update.
pub(crate) fn feet_input(skater: &Skate3ControllerState) -> feet::Input {
    let p = &skater.player_input.processed;
    let animation = &skater.animated_skeleton;
    let root = animation.roots.animation_to_world;
    let effective = super::entry::effective_root(root, p.flags_2476);
    feet::Input {
        lines: std::array::from_fn(|i| {
            let line = p.line_tests_960_1008_1056[i];
            feet::Line {
                position: line.position.map(f32::from_bits),
                normal: line.normal.map(f32::from_bits),
                surface: line.surface,
                valid: line.valid != 0,
            }
        }),
        root,
        inverse_root: animation.roots.world_to_animation,
        effective_root: effective,
        local_foot_pairs: [15, 19]
            .map(|i| [animation.record.pose[i][3], animation.record.pose[i + 1][3]]),
        world_foot_pairs: [15, 19].map(|i| {
            [
                compose_affine(&root, &animation.record.pose[i])[3],
                compose_affine(&root, &animation.record.pose[i + 1])[3],
            ]
        }),
        position: p.vectors_544_560_592_608[2].map(f32::from_bits),
        velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        flags_2484: p.flags_2484,
        state: p.state_2508,
    }
}

pub(crate) fn update_feet(skater: &mut Skate3ControllerState) {
    let input = feet_input(skater);
    super::feet::update(&mut skater.offboard_feet, input, &mut skater.foot_ik.state);
}

pub(crate) fn launch_input(
    skater: &Skate3ControllerState,
    geometry: Option<air_launch::DepartureGeometry>,
) -> Result<LaunchInput, String> {
    let p = &skater.player_input.processed;
    let board = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Ground launch requires the completed BoardToolkit")?;
    Ok(LaunchInput {
        processed: air_launch::Processed {
            board_position_112: board.deck[3],
            forward_224: p.effective_anim_transform_192[2].map(f32::from_bits),
            up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
            position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
            velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
            velocity_912: p.vectors_880_896_912_928_944[2].map(f32::from_bits),
            departure_geometry: geometry,
            flags_2472: p.flags_2472,
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            previous_state_2504: p.state_2504,
            current_state_2508: p.state_2508,
            current_category_2512: p.category_2512,
            previous_category_2516: p.category_2516,
            raw_x_2692: skater.animation_input.extra.biped_world_x,
            raw_z_2688: skater.animation_input.extra.biped_world_z,
        },
        elapsed_2664: p.state_timer_2664,
        skeleton_point_10960: skater.animated_skeleton.record.centre_of_mass,
    })
}

///82D32088..32108, after Skeleton has published its real processed fields.
pub(crate) fn sync_grab(skater: &mut Skate3ControllerState) {
    let p = &skater.player_input.processed;
    let bone = compose_affine(
        &skater.animated_skeleton.roots.animation_to_world,
        &skater.animated_skeleton.record.pose[23],
    )[3];
    let input = super::grab::Input {
        frame: p
            .effective_anim_transform_192
            .map(|v| v.map(f32::from_bits)),
        position: p.vectors_544_560_592_608[2].map(f32::from_bits),
        bone,
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        flags_2484: p.flags_2484,
        elapsed: p.state_timer_2664,
        timer_2852: p.secondary_ground_timer_2852,
        context: skate3_core::player::offboard::ground_query::QueryContext {
            selection_flags_2948: p.actor_query_2948,
            matching_id_2952: p.actor_query_2952 as i32,
        },
    };
    super::grab::sync(
        &mut skater.biped_ground.ground,
        &mut skater.offboard_grab,
        skater.biped_ground.grab_settings,
        input,
    );
}

///82D6CA58 copies the actual gravity vector and submits; consumption remains
///at the shared selector's native phase, not an invented Ground frame delay.
pub(crate) fn submit_air(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    packet: air_launch::Packet,
) -> Result<(), String> {
    let p = &skater.player_input.processed;
    let gravity = physics.settings.step.simulation.gravity_acceleration;
    let context = air_selector::Context {
        selection_flags_2948: p.actor_query_2948,
        matching_group_2952: p.actor_query_2952 as i32,
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        forward_224: p.effective_anim_transform_192[2].map(f32::from_bits),
    };
    skater
        .offboard_air_selector
        .launch(
            &physics.world,
            packet,
            [gravity.x, gravity.y, gravity.z, 0.],
            context,
        )
        .map_err(str::to_owned)
}
