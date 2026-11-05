//! Original82D40AF8: collision -> pop -> engagement -> derived substate.
//! All writes target current board, manager jumper and skeleton owners.
mod nonspecific;
mod settings;
mod skeleton;
use super::super::{Skate3PhysicsState, Skate3ControllerState};
use super::{Family, ManagerObservation};
pub(super) use nonspecific::execute as execute_nonspecific;
pub(super) use settings::Settings;
use skate3_core::{
    math::Vector3,
    physics::{force_queue::QueuedPointForce, grind_forces::launch::Input as LaunchInput},
    riding::collision_response::{CollisionResponsePhysical, collision_response},
};
type V = [f32; 4];

pub(super) fn execute(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    manager: &ManagerObservation,
) -> Result<(), String> {
    let family = skater
        .grind
        .active
        .ok_or("Grind substate requires active physical family")?;
    let index = family as usize;
    let state = skater.grind.states[index];
    let mut jumped_now = false;
    let prediction_velocity = if let Some(velocity) = collision_force(physics, skater)? {
        //82D40BC8 queues force15; optional velocity is prediction-only here.
        Some(velocity)
    } else if skater.player_input.processed.flags_2468 & 0x0040_0000 != 0 {
        let p = &skater.player_input.processed;
        let position = skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("Grind pop requires current BoardToolkit")?
            .deck[3];
        //Native virtual64 (vertical) precedes virtual60 (geometry side).
        let vertical_jump = skater.grind.settings.substate.vertical(
            family,
            p.state_variant_index_2528,
            skater.animation_input.extra.jump_strength,
        )?;
        let geometry_side_jump = settings::side(family, p.grind.geometry_kind_1464);
        let input = LaunchInput {
            velocity_400: p.vectors_400_416[0].map(f32::from_bits),
            position_112: position,
            current_point_1120: p.grind.point_1120.map(f32::from_bits),
            balance_2800: skater.animation_input.extra.grind_stability_nudge,
            geometry_side_jump,
            vertical_jump,
        };
        let velocity = super::launch::apply(
            &mut physics.board,
            &mut skater.player_input.grind.jumper,
            &mut skater.player_input.processed.flags_2476,
            input,
        );
        let state = &mut skater.grind.states[index];
        state.already_jumped = true;
        state.output.just_jumped = true;
        state.output.jump_velocity = velocity;
        jumped_now = true;
        Some(velocity)
    } else if !state.already_jumped {
        if skater.player_input.processed.grind.flags_1516 & 0x4000_0000 != 0 {
            let velocity = skater
                .player_input
                .processed
                .grind
                .entry_velocity_1184
                .map(f32::from_bits);
            //82D40C84 SetVelocity updates all seven real board parts.
            skater
                .ground_runtime
                .set_animated_velocity(&mut physics.board, velocity);
            Some(velocity)
        } else {
            //Do not restore the copied State after these calls: each derived
            //implementation owns its full state writes, including transitions.
            match state.output.substate {
                1 => super::contact::advance(physics, skater, manager)?,
                2 => super::involuntary::advance(physics, skater, manager)?,
                _ => {}
            }
            None
        }
    } else {
        None
    };
    if let Some(velocity) = prediction_velocity {
        let position = skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("Grind prediction requires current BoardToolkit")?
            .deck[3];
        let dt = skater.player_input.processed.timestep_2604;
        let prediction = core::array::from_fn(|i| velocity[i].mul_add(dt, position[i]));
        //82D40CFC stores16112 and82D40D0C sets16416. No guessed trajectory.
        skater.animated_skeleton.roots.predicted_board_position = prediction;
        skater.animated_skeleton.roots.supplied_prediction = Some(prediction);
    }
    if jumped_now {
        skeleton::animated(physics, skater)?;
    } else {
        //82BDF530 then82C04368: CURRENT wrapper also retains physics error.
        super::super::input_phase::update_ground(physics, skater)?;
    }
    Ok(())
}

///Shared82D944E8 caller for common grind and Nonspecific. The late-false
///output is intentionally ignored, and angular displacement is not applied.
///Returns prediction velocity ONLY on the branch that queued force15.
pub(super) fn collision_force(
    physics: &mut Skate3PhysicsState,
    skater: &Skate3ControllerState,
) -> Result<Option<V>, String> {
    let p = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Grind collision requires current BoardToolkit")?;
    let normal = physics.riding.reckoning.ground_normal;
    let response = collision_response(
        &skater.grind.settings.substate.collision,
        CollisionResponsePhysical {
            flags_2472: p.flags_2472,
            collision_displacement: p.collision_pose_error_736.map(f32::from_bits),
            velocity: p.vectors_400_416[1].map(f32::from_bits),
            forward: toolkit.travel_direction,
            up: p.vectors_544_560_592_608[0].map(f32::from_bits),
            ground_normal: [normal.x, normal.y, normal.z, 0.],
            time_step: p.timestep_2604,
            mass: toolkit.total_mass,
        },
    );
    let Some(response) = response.filter(|r| r.applied) else {
        return Ok(None);
    };
    let xyz = |v: V| Vector3::new(v[0], v[1], v[2]);
    //A full native queue drops this append but still takes the true branch.
    physics.board.forces_mut().append(QueuedPointForce {
        tag: 15,
        force_world: xyz(response.force),
        point_body: xyz(response.point),
    });
    Ok(Some(response.target_velocity))
}
