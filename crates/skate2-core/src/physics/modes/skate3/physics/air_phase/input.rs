//! Actual Processed/animation/stock owners read by PhysicsAir and its launch.
use super::*;
use skate3_core::{
    air::{
        state::{PhysicsAirFrame, PhysicsAirSettings},
        trajectory::{LaunchInfo, SelectorInput},
    },
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

pub(crate) struct AirSettings {
    pub state: PhysicsAirSettings,
    pub steering_blend: f32,
    pub grind_lock_distance: [f32; 5],
}
impl AirSettings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let values = data
            .words::<16>("physics_airstates", "default", "BodySpinInputFilter")?
            .map(f32::from_bits);
        let f = |name| data.float("physics_airstates", "default", name);
        let mut distances = [0.0; 5];
        for (i, mode) in ["easy", "normal", "hardcore", "motorized", "test"]
            .into_iter()
            .enumerate()
        {
            distances[i] = data.float("physics_mode", mode, "GrindLockDist")?;
        }
        Ok(Self {
            state: PhysicsAirSettings {
                body_spin_over_time_320: PointGraph {
                    x: values[..8].try_into().unwrap(),
                    y: values[8..].try_into().unwrap(),
                },
                landing_normal_blend_388: f("SpeedToAlignToGround_PhysAir")?,
                body_spin_scale_428: f("MaxSpinSpeed")?,
                landing_normal_angle_limit_444: f("DontAlignAnglePhysicsAir")?,
            },
            steering_blend: data.float("physics_steering", "default", "SteeringTiltBlending")?,
            grind_lock_distance: distances,
        })
    }
}
pub(super) fn frame(skater: &Skate3ControllerState) -> PhysicsAirFrame {
    let p = &skater.player_input.processed;
    PhysicsAirFrame {
        current_velocity_400: p.vectors_400_416[0].map(f32::from_bits),
        ground_position_y_500: f32::from_bits(p.vectors_464_480_496_512_528[2][1]),
        trajectory_position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
        trajectory_velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
        jump_velocity_848: skater.player_state.post.jump_reference.map(f32::from_bits),
        flags_2468: p.flags_2468,
        previous_physics_state_2504: p.state_2504 as i32,
        previous_physics_category_2516: p.category_2516 as i32,
        frames_since_jump_correction_2576: skater.player_state.post.jump_fix_frames as i32,
        delta_time_2604: p.timestep_2604,
        body_spin_input_2640: skater.animation_input.fields.body_spin,
        gravity_y_2648: p.gravity_2648,
        state_timer_2664: p.state_timer_2664,
    }
}
pub(crate) fn selector_input(
    physics: &Skate3PhysicsState,
    skater: &Skate3ControllerState,
) -> Result<SelectorInput, String> {
    let p = &skater.player_input.processed;
    let g = physics.settings.step.simulation.gravity_acceleration;
    Ok(SelectorInput {
        gravity: [g.x, g.y, g.z, 0.0],
        ground_normal: p.vectors_464_480_496_512_528[0].map(f32::from_bits),
        contact_position: p.vectors_464_480_496_512_528[2].map(f32::from_bits),
        heading_direction: p.vectors_464_480_496_512_528[3].map(f32::from_bits),
        reference_up: p.vectors_544_560_592_608[0].map(f32::from_bits),
        board_vertical_velocity: f32::from_bits(p.vectors_400_416[0][1]),
        directional_input: p.transition_2636,
        previous_physics_state: p.state_2504,
        flags_2472: p.flags_2472,
        flags_2476: p.flags_2476,
        offboard_flags_1776: p.external_physics_1616.flags,
        grind_lock_distance: *skater
            .air_settings
            .grind_lock_distance
            .get(p.state_variant_index_2528 as usize)
            .ok_or_else(|| {
                format!(
                    "Invalid trajectory physics mode {}",
                    p.state_variant_index_2528
                )
            })?,
    })
}
pub(crate) fn launch_info(
    physics: &Skate3PhysicsState,
    skater: &Skate3ControllerState,
) -> Result<LaunchInfo, String> {
    let p = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Air launch requires current BoardToolkit")?;
    Ok(LaunchInfo {
        reckoning_transform: physics.riding.reckoning_frames.system,
        reckoning_inverse: physics.riding.reckoning_frames.inverse_system,
        start_velocity: p.vectors_400_416[0].map(f32::from_bits),
        com_velocity: p.prepared_jump_704.map(f32::from_bits),
        skeleton_vector_160: skater.animated_skeleton.board_frames.local_centre_of_mass,
        skeleton_vector_176: skater.animated_skeleton.board_frames.local_board_position,
        board_position: toolkit.deck[3],
        animation_com_position: p.vectors_544_560_592_608[2].map(f32::from_bits),
        cone_angle_x: skater.trajectory.settings.cone_x,
        cone_angle_z: skater.trajectory.settings.cone_z,
        timestep: p.timestep_2604,
        trajectory_count: if p.flags_2468 & 0x2000 != 0 { 7 } else { 1 },
        ..LaunchInfo::default()
    })
}
