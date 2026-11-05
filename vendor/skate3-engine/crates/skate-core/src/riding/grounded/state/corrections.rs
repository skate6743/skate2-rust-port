//! Ground correction paths called by `82D38800`.

use crate::{
    math::Vector3,
    physics::force_queue::{BoardForceQueue, QueuedPointForce},
};

use super::data::PhysicsGroundState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AntiFlipNudgeInput {
    pub deck_speed_2652: f32,
    pub deck_axis_96: [f32; 4],
}

pub trait AntiFlipNudgeMath {
    type Error;

    /// Exact `vmsum3fp128` operation. Generic host dot arithmetic is not a
    /// substitute for the recovered Xenon operation.
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error>;
    /// Exact `vrsqrtefp128` plus two refinements and final scale at
    /// `82D39498..82D394EC`.
    fn scale_to_magnitude(
        &mut self,
        vector: [f32; 4],
        squared_length: f32,
        magnitude: f32,
    ) -> Result<[f32; 4], Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AntiFlipNudgeResult {
    pub attempted: bool,
    pub queued: bool,
}

/// Complete state/branch/queue contract of `UpdateAntiFlipNudge` `82D39338`.
pub fn update_anti_flip_nudge<M: AntiFlipNudgeMath>(
    state: &mut PhysicsGroundState,
    input: AntiFlipNudgeInput,
    queue: &mut BoardForceQueue,
    math: &mut M,
) -> Result<AntiFlipNudgeResult, M::Error> {
    let anti_flip_squared = math.dot3(state.anti_flip_torque_2624, state.anti_flip_torque_2624)?;
    state.anti_flip_nudge_frames_2752 = if anti_flip_squared > 0.0 {
        state.anti_flip_nudge_frames_2752.wrapping_add(1)
    } else {
        // `ble` after `fcmpu` also takes the unordered case.
        0
    };
    if state.anti_flip_nudge_frames_2752 <= 12
        || !(input.deck_speed_2652 < f32::from_bits(0x3f8c_cccd))
    {
        return Ok(AntiFlipNudgeResult {
            attempted: false,
            queued: false,
        });
    }

    let mut direction = input.deck_axis_96;
    if direction[1] > 0.0 {
        direction = direction.map(flip_sign_bit);
    }
    direction[1] = 0.0;
    let squared = math.dot3(direction, direction)?;
    if !(squared > f32::from_bits(0x3a83_126f)) {
        return Ok(AntiFlipNudgeResult {
            attempted: false,
            queued: false,
        });
    }
    let force = math.scale_to_magnitude(direction, squared, 50.0)?;
    let queued = queue.append(QueuedPointForce {
        tag: 17,
        force_world: xyz(force),
        point_body: Vector3::ZERO,
    });
    // The native byte is set after the append call regardless of capacity.
    state.anti_flip_nudge_applied_2723 = true;
    Ok(AntiFlipNudgeResult {
        attempted: true,
        queued,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HangUpInput {
    pub flags_1516: u32,
    pub deck_speed_2652: f32,
    pub scalar_84: f32,
    /// Exact `vmsum3fp128` comparison at `82D39744..82D39764`.
    pub geometry_axis_dot_positive: bool,
}

pub trait HangUpServices {
    type Error;

    /// `82D37048`, `82D370F8` and the direction/up-vector construction at
    /// `82D395AC..82D396AC`, including its signed side test.
    fn build_hang_force(&mut self) -> Result<[f32; 4], Self::Error>;
    /// Direct `ApplyWorldSpaceForceToDeck` call at `82D396DC`.
    fn apply_hang_force(&mut self, force: [f32; 4]) -> Result<(), Self::Error>;
    /// Exact `82C20530/5D8/728/C08/6C0` geometry-object lifetime and result.
    fn detect_hung_up_geometry(&mut self) -> Result<bool, Self::Error>;
    fn request_wipeout(&mut self) -> Result<(), Self::Error>;
}

/// Complete counter and effect order of `ManageHangUps` `82D39510`.
pub fn manage_hang_ups<S: HangUpServices>(
    state: &mut PhysicsGroundState,
    input: HangUpInput,
    services: &mut S,
) -> Result<(), S::Error> {
    let investigating = input.flags_1516 & 0x0800_0000 != 0;
    if investigating {
        if input.deck_speed_2652 < f32::from_bits(0x3f8c_cccd)
            && input.scalar_84 < f32::from_bits(0x3f66_6666)
            && input.scalar_84 > 0.0
        {
            state.hang_detection_frames_2740 = state.hang_detection_frames_2740.wrapping_add(1);
        }
    } else {
        state.hang_detection_frames_2740 = 0;
    }

    if state.hang_detection_frames_2740 == 20 {
        state.hang_force_frames_2744 = 10;
        state.vector_2608 = services.build_hang_force()?;
    }
    if state.hang_force_frames_2744 > 0 {
        if investigating {
            services.apply_hang_force(state.vector_2608)?;
        }
        state.hang_force_frames_2744 = state.hang_force_frames_2744.wrapping_sub(1);
        if state.hang_force_frames_2744 == 0 {
            state.hang_detection_frames_2740 = 0;
        }
    }

    let geometry_candidate = input.deck_speed_2652 < f32::from_bits(0x3f8c_cccd)
        && input.flags_1516 & 0x0c00_0000 == 0x0c00_0000
        && input.geometry_axis_dot_positive;
    let hung = if geometry_candidate {
        services.detect_hung_up_geometry()?
    } else {
        false
    };
    state.hung_wipeout_frames_2748 = if hung {
        state.hung_wipeout_frames_2748.wrapping_add(1)
    } else {
        0
    };
    if state.hung_wipeout_frames_2748 > 20 {
        services.request_wipeout()?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfpipeWheelCatchInput {
    pub flags_1516: u32,
    pub deck_speed_2652: f32,
    pub deck_x_axis_y: f32,
    pub deck_y_axis_y: f32,
    pub signed_deck_distance: f32,
}

pub trait HalfpipeWheelCatchServices {
    type Error;

    /// Exact signed axis/cross-product construction at `82D39950..82D399E8`.
    fn angular_displacement(&mut self) -> Result<[f32; 4], Self::Error>;
    fn apply_angular_displacement(&mut self, value: [f32; 4]) -> Result<(), Self::Error>;
}

/// Gating and effect order of `ManageHalfpipeWheelCatches` `82D39868`.
pub fn manage_halfpipe_wheel_catches<S: HalfpipeWheelCatchServices>(
    input: HalfpipeWheelCatchInput,
    services: &mut S,
) -> Result<bool, S::Error> {
    let candidate = input.flags_1516 & 0x0800_0000 != 0
        && input.deck_speed_2652 < 3.0
        && input.deck_x_axis_y.abs() < f32::from_bits(0x3f11_eb85)
        && input.deck_y_axis_y.abs() < f32::from_bits(0x3df5_c28f)
        && input.signed_deck_distance < f32::from_bits(0x3de1_47ae)
        && input.signed_deck_distance > 0.0;
    if !candidate {
        return Ok(false);
    }
    let displacement = services.angular_displacement()?;
    services.apply_angular_displacement(displacement)?;
    Ok(true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinningInput {
    pub flags_2488: u32,
    pub frames_since_teleport_2584: i32,
    pub flags_2472: u32,
}

pub trait PinningServices {
    type Error;

    /// Exact reciprocal-refinement velocity and `82C04168` application at
    /// `82D39B04..82D39B8C` using captured X/Z, live Y, position and timestep.
    fn pin_to_captured_position(
        &mut self,
        captured_x: f32,
        captured_z: f32,
    ) -> Result<(), Self::Error>;
}

/// Complete state gating of `ConsiderPinning` `82D39A18`.
pub fn consider_pinning<S: PinningServices>(
    state: &mut PhysicsGroundState,
    input: PinningInput,
    services: &mut S,
) -> Result<(), S::Error> {
    if input.flags_2488 & 1 != 0 {
        state.pinning_2727 = false;
        state.was_pinning_2728 = false;
        return Ok(());
    }
    if state.human_player_2724 && input.frames_since_teleport_2584 < 90 {
        let eligible = state.was_pinning_2728 || input.frames_since_teleport_2584 < 30;
        let controls_seen =
            input.frames_since_teleport_2584 > 0 && input.flags_2472 & 0x0080_0000 == 0;
        state.controls_latched_2725 |= controls_seen;
        if eligible && state.captured_position_valid_2726 && !state.controls_latched_2725 {
            services.pin_to_captured_position(
                state.captured_position_x_2656,
                state.captured_position_z_2660,
            )?;
            state.pinning_2727 = true;
        }
    }
    state.was_pinning_2728 = state.pinning_2727;
    Ok(())
}

fn flip_sign_bit(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}

fn xyz(value: [f32; 4]) -> Vector3 {
    Vector3::new(value[0], value[1], value[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::force_queue::FORCE_CAPACITY;

    struct NudgeMath {
        dots: [f32; 2],
        dot_count: usize,
        scale_count: usize,
    }

    impl AntiFlipNudgeMath for NudgeMath {
        type Error = ();

        fn dot3(&mut self, _left: [f32; 4], _right: [f32; 4]) -> Result<f32, Self::Error> {
            let value = self.dots[self.dot_count];
            self.dot_count += 1;
            Ok(value)
        }

        fn scale_to_magnitude(
            &mut self,
            vector: [f32; 4],
            _squared_length: f32,
            magnitude: f32,
        ) -> Result<[f32; 4], Self::Error> {
            self.scale_count += 1;
            Ok(vector.map(|lane| lane * magnitude))
        }
    }

    fn state() -> PhysicsGroundState {
        PhysicsGroundState {
            collision_force_2528: [0.0; 4],
            collision_point_2544: [0.0; 4],
            word_2560: 0,
            word_2564: 0,
            vector_2592: [0.0; 4],
            vector_2608: [0.0; 4],
            anti_flip_torque_2624: [1.0, 0.0, 0.0, 0.0],
            steering_push_scalar_2640: 1.0,
            steering_damped_turn_2644: 0.0,
            elapsed_2648: 0.0,
            collision_countdown_2652: 0.0,
            captured_position_x_2656: 0.0,
            captured_position_z_2660: 0.0,
            scalar_2664: 0.0,
            scalar_2668: 0.0,
            straighten_scale_2672: 1.0,
            vector_2688: [0.0; 4],
            scalar_2704: 0.0,
            flag_2708: false,
            flag_2720: false,
            flag_2721: false,
            flag_2722: false,
            anti_flip_nudge_applied_2723: false,
            human_player_2724: false,
            controls_latched_2725: false,
            captured_position_valid_2726: false,
            pinning_2727: false,
            was_pinning_2728: false,
            flag_2729: false,
            push_suppressed_2730: false,
            flag_2731: false,
            manual_correction_2732: false,
            manual_opposition_2733: false,
            hang_detection_frames_2740: 0,
            hang_force_frames_2744: 0,
            hung_wipeout_frames_2748: 0,
            anti_flip_nudge_frames_2752: 13,
        }
    }

    #[test]
    fn nudge_unordered_comparisons_take_the_native_rejection_paths() {
        let mut state = state();
        let mut queue = BoardForceQueue::default();
        let mut math = NudgeMath {
            dots: [f32::NAN, 1.0],
            dot_count: 0,
            scale_count: 0,
        };
        let result = update_anti_flip_nudge(
            &mut state,
            AntiFlipNudgeInput {
                deck_speed_2652: 0.0,
                deck_axis_96: [1.0, 0.0, 0.0, 0.0],
            },
            &mut queue,
            &mut math,
        )
        .unwrap();
        assert_eq!(state.anti_flip_nudge_frames_2752, 0);
        assert!(!result.attempted);

        state.anti_flip_nudge_frames_2752 = 13;
        math.dots = [1.0, f32::NAN];
        math.dot_count = 0;
        let result = update_anti_flip_nudge(
            &mut state,
            AntiFlipNudgeInput {
                deck_speed_2652: 0.0,
                deck_axis_96: [1.0, 0.0, 0.0, 0.0],
            },
            &mut queue,
            &mut math,
        )
        .unwrap();
        assert!(!result.attempted);
        assert_eq!(math.scale_count, 0);
    }

    #[test]
    fn nudge_latches_presence_even_when_the_native_queue_is_full() {
        let mut state = state();
        let mut queue = BoardForceQueue::default();
        for _ in 0..FORCE_CAPACITY {
            assert!(queue.append(QueuedPointForce::default()));
        }
        let mut math = NudgeMath {
            dots: [1.0, 1.0],
            dot_count: 0,
            scale_count: 0,
        };

        let result = update_anti_flip_nudge(
            &mut state,
            AntiFlipNudgeInput {
                deck_speed_2652: 0.0,
                deck_axis_96: [1.0, 0.0, 0.0, 0.0],
            },
            &mut queue,
            &mut math,
        )
        .unwrap();

        assert!(result.attempted);
        assert!(!result.queued);
        assert!(state.anti_flip_nudge_applied_2723);
        assert_eq!(queue.entries().len(), FORCE_CAPACITY);
    }
}
