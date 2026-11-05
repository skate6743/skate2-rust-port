//! Concrete KnownAir boundary. All callbacks mutate the same live owners.
//! Fallible host queries/pose checks retain their first error for the phase caller.
use super::{Skate3PhysicsState, Skate3ControllerState, math::*};
use crate::skate3_native::physics::{
    footplant::FootplantFrame, input_phase, skeleton_input_runtime::SkeletonOwners,
    wipeout::Observations,
};
use skate3_core::{
    air::{
        known::*,
        trajectory::{Selection, Trajectory},
    },
    physics::board_toolkit::BoardToolkit,
};
pub(super) struct Runtime<'a> {
    pub physics: &'a mut Skate3PhysicsState,
    pub skater: &'a mut Skate3ControllerState,
    pub toolkit: BoardToolkit,
    selection: Selection,
    local_com: V,
    board_position: V,
    error: Option<String>,
}
impl<'a> Runtime<'a> {
    pub fn new(
        physics: &'a mut Skate3PhysicsState,
        skater: &'a mut Skate3ControllerState,
    ) -> Result<Self, String> {
        let selector = &skater.trajectory.selector;
        let selection = *selector
            .selection()
            .ok_or("KnownAir requires its actual winning query result")?;
        let local_com = selector
            .local_com_position()
            .ok_or("KnownAir requires selector local COM")?;
        let board_position = selector
            .board_position()
            .ok_or("KnownAir requires selector board position")?;
        let toolkit = skater
            .player_input
            .toolkit
            .ok_or("KnownAir requires prepared board toolkit")?;
        Ok(Self {
            physics,
            skater,
            toolkit,
            selection,
            local_com,
            board_position,
            error: None,
        })
    }
    pub fn finish(self) -> Result<(), String> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
    fn record(&mut self, result: Result<(), String>) {
        if self.error.is_none() {
            self.error = result.err();
        }
    }
}
pub(super) fn trajectory(t: Trajectory) -> KnownAirTrajectory {
    //82E09A14/2C splats the request horizon into all four final-vector lanes.
    //KnownAir Fill copies all64 bytes, including the three repeated words.
    KnownAirTrajectory {
        position: t.position,
        velocity: t.velocity,
        acceleration: t.acceleration,
        scalar_48: t.duration,
        word_52: t.duration.to_bits(),
        word_56: t.duration.to_bits(),
        word_60: t.duration.to_bits(),
    }
}
fn actual(t: &KnownAirTrajectory) -> Trajectory {
    Trajectory {
        position: t.position,
        velocity: t.velocity,
        acceleration: t.acceleration,
        duration: t.scalar_48,
    }
}
impl KnownAirRuntime for Runtime<'_> {
    fn enable_board_angular_drive_only(&mut self) {
        self.physics
            .board
            .hook_mut()
            .drive
            .enable_angular_only(&mut self.skater.ground_lifecycle.board_animated_290);
    }
    fn set_air_collision_update_enabled(&mut self, v: bool) {
        self.skater.ground_lifecycle.skeleton_controller.flag_18 = v;
    }
    fn set_skeleton_collision_state(&mut self, state: u32) {
        let result = self
            .skater
            .ground_lifecycle
            .skeleton_controller
            .request(state, &mut self.skater.skeleton_collision);
        self.record(result);
    }
    fn set_skeleton_physics_truck_tilt_enabled(&mut self, v: bool) {
        self.skater.ground_lifecycle.skeleton_elapsed_16505 = v;
    }
    fn set_skeleton_inverse_kinematics_enabled(&mut self, v: bool) {
        self.skater.foot_ik.enable_feet(v);
    }
    fn reset_reckoning_flipping(&mut self) {
        self.reset_flip();
    }
    fn set_footplant_flag_240(&mut self, v: bool) {
        self.skater.footplant.enabled = v;
    }
    fn reset_footplants(&mut self) {
        self.skater.footplant.reset();
    }
    fn board_transform_height(&mut self) -> f32 {
        self.physics.board.part_transforms()[6].translation.y
    }
    fn skeleton_com_position(&mut self) -> V {
        self.skater.animated_skeleton.board_frames.centre_of_mass
    }
    fn selector_targeting_grind(&self) -> bool {
        self.skater.trajectory.selector.grind_locked_to_middle()
    }
    fn start_grind_air_adjust_from_selector(&mut self) {
        if let Some(target) = self.selection.grind {
            self.skater
                .skeleton_input
                .grind_air
                .start(target.air_target());
            self.skater.skeleton_input.grind_air_started = true;
        } else {
            self.record(Err(
                "KnownAir grind lock has no accepted trajectory target".into()
            ));
        }
    }
    fn selector_closest_trajectory_point(&mut self, com: V, zero: V) -> (i32, V) {
        let t = self.selection.com_trajectory;
        let mut best = 100000000.0;
        let mut offset = zero;
        for i in 0..100 {
            let d = sub(com, t.position_at(i as f32 * f32::from_bits(0x3c888889)));
            let squared = dot(d, d);
            if squared > best {
                return (i - 1, offset);
            }
            best = squared;
            offset = d;
        }
        (0, zero)
    }
    fn selected_prediction(&self) -> KnownAirPrediction {
        KnownAirPrediction {
            trajectory: trajectory(self.selection.prediction.request.trajectory),
            collision_time_48: self.selection.prediction.result.contact_time,
            collision_frame_128: self.selection.prediction.result.contact_frame,
        }
    }
    fn selector_trajectory_2704(&self) -> KnownAirTrajectory {
        trajectory(self.selection.com_trajectory)
    }
    fn selector_contact_position(&mut self) -> V {
        self.selection.prediction.result.contact_position
    }
    fn selector_vector_2848(&self) -> V {
        self.board_position
    }
    fn selector_com_position_2784(&self) -> V {
        self.local_com
    }
    fn highest_trajectory_position(&mut self, t: &KnownAirTrajectory) -> (V, f32) {
        actual(t).highest_position()
    }
    fn selector_has_just_changed(&self) -> bool {
        self.skater.trajectory.selector.just_changed()
    }
    fn reset_trajectory_selector(&mut self) {
        self.skater.trajectory.selector.reset();
    }
    fn begin_reckoning_body_flip(&mut self, side: bool) {
        self.begin_flip(side);
    }
    fn update_air_collision(&mut self) {
        let p = &self.skater.player_input.processed;
        let result = self.skater.ground_lifecycle.skeleton_controller.update_air(
            p.flags_2472,
            f32::from_bits(p.vectors_400_416[0][1]),
            &mut self.skater.skeleton_collision,
        );
        self.record(result);
    }
    fn grind_air_adjust_started(&self) -> bool {
        self.skater.skeleton_input.grind_air_started
    }
    fn set_grind_air_adjust_started(&mut self, v: bool) {
        self.skater.skeleton_input.grind_air_started = v;
    }
    fn set_grind_air_adjust_activated(&mut self, v: bool) {
        self.skater.skeleton_input.grind_air_active = v;
    }
    fn grind_air_adjust_adjusting(&self) -> bool {
        self.skater.skeleton_input.grind_air_adjusting
    }
    fn disable_grind_foot_collision(&mut self, body: GrindFootBody) {
        let part = match body {
            GrindFootBody::Body3152 => 15,
            GrindFootBody::Body3156 => 16,
            GrindFootBody::Body3160 => 17,
            GrindFootBody::Body3168 => 19,
            GrindFootBody::Body3172 => 20,
            GrindFootBody::Body3176 => 21,
        };
        let collision = &mut self.skater.skeleton_collision;
        collision.pending_reenable = true;
        collision.disable_count[part] = 2;
        collision.parts[part].enabled = false;
    }
    fn update_reckoning_air_states(&mut self, n: V, blend: f32, spin: f32, flip: f32) {
        let result = self
            .skater
            .air_reckoning
            .update(
                &mut self.physics.riding,
                &self.skater.player_input.processed,
                self.skater.animation_input.extra.physical_body_spin,
                n,
                blend,
                spin,
                flip,
            )
            .map(|_| ());
        self.record(result);
    }
    fn update_known_air_skeleton(&mut self, target: V) {
        let collision = input_phase::collision(self.skater);
        let s = &mut self.skater;
        let mut owners = SkeletonOwners {
            animated: &mut s.animated_skeleton,
            body: &mut s.skeleton,
            drives: &mut s.skeleton_drives,
            ik: &mut s.foot_ik,
            animation_input: &mut s.animation_input,
            correction: &mut s.skeleton_output.correction,
            pose_errors: &mut s.pose_errors,
        };
        let packet = &s.animation.packet;
        let result = s
            .skeleton_input
            .update_known_air(
                &mut s.skeleton_air,
                &mut self.physics.board,
                &self.physics.riding.reckoning_frames.system,
                target,
                packet,
                &mut s.player_input.processed,
                &mut owners,
                &packet.hierarchy,
                &collision,
                self.physics.settings.step.simulation,
            )
            .map(|_| ());
        self.record(result);
    }
    fn set_skeleton_add_skateboard_error(&mut self, v: bool) {
        self.skater.skeleton_output.correction.pending = v;
    }
    fn set_head_tracking_target(&mut self, target: V, valid: bool) {
        self.skater.skeleton_input.head_tracking_history[5] = target;
        self.skater.skeleton_input.head_tracking_active = valid;
    }
    fn update_board_steering_tilt(&mut self, tilt: f32) {
        let p = &self.skater.player_input.processed;
        self.skater.ground.steering.update(
            tilt,
            self.skater.air_settings.steering_blend,
            p.flags_2468,
            p.flags_2472,
        );
    }
    fn shift_footplant_trajectory_to_index(&mut self, t: &mut KnownAirTrajectory, index: i32) {
        let source = actual(t);
        let time = index as f32 * f32::from_bits(0x3c888889);
        t.position = source.position_at(time);
        t.velocity = source.velocity_at(time);
    }
    fn update_footplant_prediction(&mut self, input: &KnownAirFootplantInput) {
        let g = self.physics.settings.step.simulation.gravity_acceleration;
        let f = FootplantFrame {
            processed: &self.skater.player_input.processed,
            toolkit: &self.toolkit,
            animated: &self.skater.animated_skeleton,
            body: &self.skater.skeleton,
            gravity: [g.x, g.y, g.z, 0.0],
            world: &self.physics.world,
            edges: self.physics.grind_world.primitives(),
        };
        self.skater.footplant.update_candidate(input, &f);
    }
    fn update_footplant_lock(&mut self, input: &KnownAirFootplantInput) {
        self.skater.footplant.update_launch(input);
    }
    fn update_footplant_pose(&mut self, input: &KnownAirFootplantInput) {
        let g = self.physics.settings.step.simulation.gravity_acceleration;
        let f = FootplantFrame {
            processed: &self.skater.player_input.processed,
            toolkit: &self.toolkit,
            animated: &self.skater.animated_skeleton,
            body: &self.skater.skeleton,
            gravity: [g.x, g.y, g.z, 0.0],
            world: &self.physics.world,
            edges: self.physics.grind_world.primitives(),
        };
        let result = self.skater.footplant.consume_and_submit(
            input,
            &f,
            &mut self.skater.foot_ik,
            &mut self.skater.skeleton_collision,
        );
        self.record(result);
    }
    fn board_body_velocity(&mut self) -> V {
        let v = self.physics.board.bodies()[6].rates.linear_velocity;
        [v.x, v.y, v.z, 0.0]
    }
    fn set_board_velocity(&mut self, v: V) {
        for body in self.physics.board.bodies_mut() {
            body.rates.linear_velocity = xyz(v);
        }
    }
    fn board_forward_axis(&mut self) -> V {
        let part = self.physics.board.part_transforms()[6];
        let c = part.basis.columns[2];
        scale(
            [c[0], c[1], c[2], 0.0],
            if self.skater.player_input.processed.flags_2468 & (1 << 20) != 0 {
                -1.0
            } else {
                1.0
            },
        )
    }
    fn check_for_air_wipeout(&mut self, use_com: bool) {
        let s = &mut self.skater;
        let observations = Observations {
            processed: &s.player_input.processed,
            board: &self.physics.riding.ground,
            collision: &s.collision_feedback,
            deck: crate::skate3_native::physics::solve::deck_frame(&self.physics.board),
            input_board: s.animated_skeleton.board_frames.animation_target,
            world_to_animation: s.animated_skeleton.roots.world_to_animation,
            pose_error: s.collision_pose_error,
            maximum_pose_error: s.collision_maximum_error,
            jump_fix_frames: s.player_state.post.jump_fix_frames,
            air: &s.air_reckoning.state,
            system_up_y: self.physics.riding.reckoning_frames.system[1][1],
            grind_locked_to_middle: s.trajectory.selector.grind_locked_to_middle(),
            grind_normal: s.trajectory.selector.grind_normal(),
        };
        let result = s.wipeout.check_air(&observations, use_com);
        self.record(result);
    }
    fn reckoning_com_transform_816(&self) -> AffineTransform {
        AffineTransform {
            vectors: self.physics.riding.reckoning_frames.system,
        }
    }
}
