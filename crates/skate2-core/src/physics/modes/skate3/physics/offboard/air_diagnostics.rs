//! Bounded numeric history for user-reproduced off-board bails. No per-frame IO.
use crate::skate3_native::physics::Skate3ControllerState;
use std::collections::VecDeque;

#[derive(Debug)]
#[allow(dead_code)] // Fields are serialized together only when a bail is requested.
struct Sample {
    physical_state: u32,
    tick: i32,
    flags: [u32; 3],
    packet: skate3_core::player::offboard::air_prediction::Packet,
    root: [[f32; 4]; 4],
    root_velocity: [f32; 4],
    body: [f32; 4],
    lift: f32,
    extra: [[f32; 4]; 2],
    maximum_error: Option<f32>,
    pose_errors: [[f32; 4]; 24],
    physical_positions: [[f32; 4]; 24],
    drive_positions: [[f32; 4]; 24],
    animation_positions: [[f32; 4]; 24],
    animation_basis_squared: [[f32; 3]; 24],
    original_positions: [[f32; 4]; 24],
    foot_targets: [skate3_core::animation::foot_ik::transforms::LimbFrames; 2],
    animation_name: Option<String>,
    contacts: [bool; 24],
    contact_age: [f32; 24],
    drive_weight: f32,
    partial_ragdoll: bool,
    limbs: [skate3_core::animation::foot_ik::status::LimbStatus; 4],
    reasons: [bool; 34],
}

pub(crate) struct History {
    samples: VecDeque<Sample>,
    reports: usize,
}
impl Default for History {
    fn default() -> Self {
        Self { samples: VecDeque::with_capacity(32), reports: 0 }
    }
}
impl History {
    pub(crate) fn clear(&mut self) { self.samples.clear(); }
}

pub(crate) fn record(skater: &mut Skate3ControllerState) {
    let history = &mut skater.offboard.air_diagnostics;
    if history.reports >= 3 { return; }
    let state = &skater.offboard.air_state;
    let p = &skater.player_input.processed;
    if history.samples.len() == 32 { history.samples.pop_front(); }
    history.samples.push_back(Sample {
        physical_state: p.state_2508,
        tick: state.tick,
        flags: [p.flags_2476, p.flags_2480, p.flags_2484],
        packet: state.packet,
        root: skater.animated_skeleton.roots.animation_to_world,
        root_velocity: skater.skeleton_input.root_velocity,
        body: state.body_position,
        lift: state.lift,
        extra: skater.collision_extra_displacements,
        maximum_error: skater.collision_maximum_error,
        pose_errors: skater.pose_errors.parts,
        physical_positions: std::array::from_fn(|i| skater.skeleton.record.pose[i][3]),
        drive_positions: std::array::from_fn(|i| skater.skeleton_input.drive_frames[i][3]),
        animation_positions: std::array::from_fn(|i| skater.animated_skeleton.record.pose[i][3]),
        animation_basis_squared: std::array::from_fn(|i| std::array::from_fn(|axis| {
            let v = skater.animated_skeleton.record.pose[i][axis];
            (v[0] * v[0] + v[1] * v[1]) + v[2] * v[2]
        })),
        original_positions: std::array::from_fn(|i| {
            skater.animation.packet.hierarchy[skater.animated_skeleton.bone_indices[i]][3]
        }),
        foot_targets: [skater.foot_ik.state.frames[0], skater.foot_ik.state.frames[1]],
        animation_name: skater.animation.motion.animation.current_name.clone(),
        contacts: skater.collision_feedback.current,
        contact_age: skater.collision_feedback.contact_age,
        drive_weight: skater.collision_feedback.drive_weight,
        partial_ragdoll: skater.skeleton_collision.partial_ragdoll,
        limbs: skater.foot_ik.state.limbs,
        reasons: skater.wipeout.state.reasons,
    });
    if skater.wipeout.state.reasons.iter().any(|&requested| requested) {
        // Format in memory, then one write; never send Debug's individual
        // formatting fragments to the Windows redirected stderr handle.
        let report = format!("SKATE_BIPED_BAIL history={:#?}", history.samples);
        eprintln!("{report}");
        history.samples.clear();
        history.reports += 1;
    }
}
