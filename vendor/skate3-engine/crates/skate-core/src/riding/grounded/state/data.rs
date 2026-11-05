//! Persistent fields owned by TU3 `PhysState_PhysicsGround`.
//!
//! Offsets remain in names when the original semantic name has not been
//! recovered. This prevents an adapter from silently assigning an invented
//! meaning to a stock field.

use super::contact_state::GroundContactStateInput;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsGroundState {
    /// Collision-force record retained by the successful `82D944E8` branch.
    pub collision_force_2528: [f32; 4],
    pub collision_point_2544: [f32; 4],
    pub word_2560: u32,
    pub word_2564: u32,
    /// Collision response retained and decayed by `82D38800`.
    pub vector_2592: [f32; 4],
    /// Direct hang-up force assembled by `82D39510`.
    pub vector_2608: [f32; 4],
    /// Output of `Toolkit_AntiFlipTorque` (`82D94190`).
    pub anti_flip_torque_2624: [f32; 4],
    /// Optional steering history arguments passed to `82D92440`.
    pub steering_push_scalar_2640: f32,
    pub steering_damped_turn_2644: f32,
    /// Accumulated by ProcessedPhysIn+2604 once per Ground Update.
    pub elapsed_2648: f32,
    /// Collision-response countdown set from the selected global settings.
    pub collision_countdown_2652: f32,
    /// First contacted position captured once for spawn pinning.
    pub captured_position_x_2656: f32,
    pub captured_position_z_2660: f32,
    /// Time remaining before the skater may enter the skitch path.
    pub scalar_2664: f32,
    /// Height of the selected skitch spline.
    pub scalar_2668: f32,
    /// Multiplies the straighten-out torque before heading calculation.
    pub straighten_scale_2672: f32,
    /// Animated-board velocity/displacement state used by Enter/Exit and Update.
    pub vector_2688: [f32; 4],
    /// Explicitly zeroed by `82D93DF0` through its Ground+2688 output record.
    pub scalar_2704: f32,
    /// Selects the animated-board branch of `82D38800`.
    pub flag_2708: bool,
    pub flag_2720: bool,
    pub flag_2721: bool,
    pub flag_2722: bool,
    pub anti_flip_nudge_applied_2723: bool,
    /// Pinning is restricted to the human-player path.
    pub human_player_2724: bool,
    pub controls_latched_2725: bool,
    pub captured_position_valid_2726: bool,
    pub pinning_2727: bool,
    pub was_pinning_2728: bool,
    pub flag_2729: bool,
    pub push_suppressed_2730: bool,
    pub flag_2731: bool,
    pub manual_correction_2732: bool,
    pub manual_opposition_2733: bool,
    pub hang_detection_frames_2740: i32,
    pub hang_force_frames_2744: i32,
    pub hung_wipeout_frames_2748: i32,
    pub anti_flip_nudge_frames_2752: i32,
}

impl PhysicsGroundState {
    /// Ground constructor82D37358..37534. Fields whose first native write is
    /// Enter are safe host zero storage until Enter replaces them; this method
    /// does not imply that entering Ground has already occurred.
    pub fn before_first_enter(human_player: bool) -> Self {
        Self {
            collision_force_2528: [0.0; 4],
            collision_point_2544: [0.0; 4],
            word_2560: 0,
            word_2564: 0,
            vector_2592: [0.0; 4],
            vector_2608: [0.0; 4],
            anti_flip_torque_2624: [0.0; 4],
            steering_push_scalar_2640: 0.0,
            steering_damped_turn_2644: 0.0,
            elapsed_2648: 0.0,
            collision_countdown_2652: 0.0,
            captured_position_x_2656: 0.0,
            captured_position_z_2660: 0.0,
            scalar_2664: 0.0,
            scalar_2668: 0.0,
            straighten_scale_2672: 0.0,
            vector_2688: [0.0; 4],
            scalar_2704: 0.0,
            flag_2708: false,
            flag_2720: false,
            flag_2721: false,
            flag_2722: false,
            anti_flip_nudge_applied_2723: false,
            human_player_2724: human_player,
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
            anti_flip_nudge_frames_2752: 0,
        }
    }

    /// State-owned prefix82D3758C..5C0 after the physical collision mode change.
    pub fn begin_entry(&mut self) {
        self.word_2560 = 0;
        self.word_2564 = 0;
        self.flag_2708 = false;
        self.controls_latched_2725 = false;
        self.captured_position_valid_2726 = false;
        self.was_pinning_2728 = false;
        self.flag_2731 = false;
        self.manual_correction_2732 = false;
        self.manual_opposition_2733 = false;
    }
    /// State-owned tail82D378AC..7914, after pumping and controller-mode reset.
    pub fn finish_entry(&mut self, previous_state: u32) {
        self.straighten_scale_2672 = if previous_state == 101 {
            f32::from_bits(0x3e23_d70a)
        } else {
            1.0
        };
        self.steering_push_scalar_2640 = 1.0;
        self.steering_damped_turn_2644 = 0.0;
        self.elapsed_2648 = 0.0;
        self.collision_countdown_2652 = 0.0;
        self.vector_2592 = [0.0; 4];
        self.vector_2608 = [0.0; 4];
        self.anti_flip_torque_2624 = [0.0; 4];
        self.hang_detection_frames_2740 = 0;
        self.hang_force_frames_2744 = 0;
        self.hung_wipeout_frames_2748 = 0;
        self.anti_flip_nudge_frames_2752 = 0;
        self.flag_2721 = false;
        self.anti_flip_nudge_applied_2723 = false;
        self.push_suppressed_2730 = false;
    }

    /// Prefix of `Update` `82D37CB4..82D37D28`, before contact-state work.
    pub fn begin_update(&mut self, input: GroundUpdateInput) -> GroundUpdateSignals {
        self.begin_update_at(input.wheel_contact_count_2556, input.position_112)
    }

    pub fn begin_update_at(
        &mut self,
        wheel_contacts: i32,
        position: [f32; 4],
    ) -> GroundUpdateSignals {
        if !self.captured_position_valid_2726 && wheel_contacts > 0 {
            self.captured_position_x_2656 = position[0];
            self.captured_position_z_2660 = position[2];
            self.captured_position_valid_2726 = true;
        }
        self.anti_flip_torque_2624 = [0.0; 4];
        self.flag_2722 = false;
        self.anti_flip_nudge_applied_2723 = false;
        self.pinning_2727 = false;
        self.scalar_2664 = -1.0;
        self.flag_2729 = false;
        GroundUpdateSignals {
            set_skeleton_flag_16505: self.elapsed_2648 > f32::from_bits(0x3d4c_cccd),
        }
    }

    /// Tail of `Update` `82D37EBC..82D37EEC` after prediction succeeds.
    pub fn finish_update(&mut self, timestep_2604: f32) {
        self.elapsed_2648 += timestep_2604;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundUpdateInput {
    pub wheel_contact_count_2556: i32,
    pub position_112: [f32; 4],
    pub timestep_2604: f32,
    pub contact_state: GroundContactStateInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundUpdateSignals {
    pub set_skeleton_flag_16505: bool,
}

/// Contact identity history updated by `82D38430`. The concrete collision
/// object handle is supplied by the world adapter rather than stored as a host
/// pointer in the engine-independent core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundContactHistory<T> {
    pub differing_contact_frames_2756: i32,
    pub tracked_contact_2760: Option<T>,
}

impl<T> GroundContactHistory<T> {
    pub const fn empty() -> Self {
        Self {
            differing_contact_frames_2756: 0,
            tracked_contact_2760: None,
        }
    }
}
