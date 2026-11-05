//! UpdateSkateboard82D38800 inputs from the actual processed packet, animation
//! attributes, physical pose and source board toolkit. No raw button-to-force
//! or guessed contact flags are introduced at this boundary.
use super::super::{
    animated_skeleton::AnimatedSkeleton, animation_input::AnimationInput,
    riding_outputs::RidingOutputs,
};
use super::{GroundRuntime, GroundSettings};
use skate3_core::{
    math::Vector3,
    physics::{
        board_runtime::BoardRuntime, board_toolkit::BoardToolkit,
        drive_frames::RetailAffineTransform, manual::controller::ManualInput,
    },
    player::input_phase::ProcessedPhysicsInput,
    riding::{
        anti_flip::AntiFlipInput,
        ground_correction_math::dot_product as dot3,
        grounded::{
            drag::GroundDragInput,
            propulsion::GroundPropulsionInput,
            state::{
                board_types::{GroundBoardInput, GroundForceFrame},
                corrections::{
                    AntiFlipNudgeInput, HalfpipeWheelCatchInput, HangUpInput, PinningInput,
                },
            },
        },
        heading::HeadingInput,
        pumping::{PumpForceInput, state::PumpingState},
        slide_friction::SlideFrictionInput,
        speed_model::SpeedModelInput,
        speed_wobble::SpeedWobbleInput,
        steering::SteeringInput,
        straighten::StraightenInput,
    },
};

/// Only values owned by separate state producers, not derivable from a rigid
/// body. The coordinator supplies the actual output of those producers.
pub(crate) struct GroundInputObservations {
    /// Reset82BF9EF0 writes0; preserve subsequent producer updates.
    pub manual_drag_2724: f32,
    /// Actual trajectory output, Processed1776.
    pub trajectory_state_bits: u32,
    /// Processed1516, copied from the grind/edge observation producer.
    pub edge_flags: u32,
    /// Processed1232, the native detected edge point.
    pub edge_point: [f32; 4],
}
impl GroundRuntime {
    /// Input preparation before Ground. Retained normal has its own native
    /// lifetime and is updated here, not in post-solver contact publication.
    pub fn prepare_toolkit(
        &mut self,
        board: &BoardRuntime,
        p: &ProcessedPhysicsInput,
    ) -> BoardToolkit {
        let result = BoardToolkit::from_board(
            board,
            p.flags_2468,
            p.scalar_2612,
            raw(p.vectors_464_480_496_512_528[0]),
            self.retained_board_normal,
        );
        self.retained_board_normal = result.filtered_normal;
        result
    }
}
impl GroundSettings {
    pub fn input(
        &self,
        t: &BoardToolkit,
        p: &ProcessedPhysicsInput,
        a: &AnimationInput,
        pumping: &PumpingState,
        unintentional_pump_scalar: f32,
        riding: &RidingOutputs,
        skeleton: &AnimatedSkeleton,
        base_trucks: [RetailAffineTransform; 2],
        extra: GroundInputObservations,
    ) -> GroundBoardInput {
        let f = &a.fields;
        let normal = raw(p.vectors_464_480_496_512_528[0]);
        let velocity = raw(p.vectors_400_416[0]);
        let angular = raw(p.vectors_400_416[1]);
        let axis544 = raw(p.vectors_544_560_592_608[0]);
        let ground_normal = v(riding.reckoning.ground_normal);
        let ground = riding.reckoning_frames.ground;
        let edge_delta = std::array::from_fn(|i| t.deck[3][i] - extra.edge_point[i]);
        let hang_axis = raw(p.vectors_720_784_800_816_832_864[4]);
        coast_trace(p);
        {
            static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_FOOT_FORCE").is_some()) {
                eprintln!(
                    "skate3 riding feet: state {} ground {:.3} crouch {:.4} delta {:.4} absorption {:.4} front {:.3} rear {:.3} balance {:.4} speed {:.3}",
                    p.filtered_state_2524, p.time_on_ground_2752, p.crouch_2776, p.crouch_delta_2780,
                    pumping.absorption, self.absorption_front, self.absorption_rear, f.balance, p.scalar_2612
                );
            }
        }
        if f.balance != 0.0 && p.spin_input_2672 != 0.0 {
            static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_MANUAL_TURN").is_some()) {
                eprintln!(
                    "skate3 manual turn: state {} balance {:.3} spin {:.4} wheels {}{} curve {:.3} dt {:.4} speed {:.3} normal {:?} fakie {}",
                    p.filtered_state_2524, f.balance, p.spin_input_2672,
                    u8::from(p.flags_2472 & 0x0800_0000 != 0), u8::from(p.flags_2472 & 0x0400_0000 != 0),
                    p.scalar_2652, p.timestep_2604, p.scalar_2612, normal, p.flags_2468 & (1 << 20) != 0
                );
            }
        }
        GroundBoardInput {
            steering: SteeringInput {
                turn: f.turn,
                hard_turn: f.hard_turn,
                absolute_body_speed: t.absolute_speed,
                flipped_controls_scalar: t.control_sign,
                balance: f.balance,
                truck_tightness: p.truck_tightness_2760,
                pushing: p.flags_2468 & 0x0800_0000 != 0,
            },
            //Ground's core controller replaces tilt and height before consuming
            //them; these are deliberately not used as substitute observations.
            speed_wobble: SpeedWobbleInput {
                tilt: 0.0,
                speed: p.scalar_2612,
                center_of_mass_height: 0.0,
                truck_tightness: p.truck_tightness_2760,
                activation_threshold: self.wobble_activation,
                amplitude_multiplier: self.wobble_amplitude,
            },
            truck_flags_2468: p.flags_2468,
            truck_flags_2472: p.flags_2472,
            contact: riding
                .probes
                .wall_contact_frame(p.time_on_ground_2752, p.signed_ground_time_2756),
            ground_vector_1216: ground_normal,
            propulsion: GroundPropulsionInput {
                flags_2468: p.flags_2468,
                flags_2472: p.flags_2472,
                target_speed: push_target(a.contacts.push_speed, p.scalar_2612, self.push_target_multiplier),
                signed_speed: p.scalar_2612,
                absolute_body_speed: t.absolute_speed,
                scalar_2660: t.total_mass,
                timestep: p.timestep_2604,
                brake_input: f.brake,
                push_direction: xyz(t.forward),
                brake_direction: xyz(t.horizontal_forward),
                surface_braking_factor: self.surface_braking_factor,
            },
            ground_force: GroundForceFrame {
                argument_1_2752: p.time_on_ground_2752,
                processed_2776: p.crouch_2776,
                processed_2780: p.crouch_delta_2780,
                previous_state_scalar_56: pumping.absorption,
                ground_scalar_1216: self.foot_force_offset,
                ground_scalar_1232: self.absorption_front,
                ground_scalar_1236: self.absorption_rear,
                ground_scalar_1240: self.foot_force_offset,
                balance_2720: f.balance,
                surface_speed_2656: p.scalar_2656,
                axis_384: t.transverse_up,
                velocity_400: velocity,
                axis_544: axis544,
            },
            speed_model: SpeedModelInput {
                flags_2468: p.flags_2468,
                flags_2476: p.flags_2476,
                wheel_contact_count: p.wheel_count_2556 as i32,
                frames_2580: p.spin_same_direction_frames_2580 as i32,
                timestep: p.timestep_2604,
                signed_speed: p.scalar_2612,
                angle_2652: p.scalar_2652,
                surface_speed: p.scalar_2656,
                turn_2672: p.spin_input_2672,
                balance: f.balance,
                elapsed_without_input: p.time_since_last_input_2748,
                manual_state_276: 0.0,
                vector_160: t.effective[2],
                vector_352: t.forward,
                velocity_416: angular,
                normal_464: normal,
                effective_forward: t.effective[2],
                mass: t.total_mass,
            },
            slide_friction: SlideFrictionInput {
                heading_time: 0.0,
                normal,
                velocity,
                side_axis: t.deck[0],
                surface_speed: p.scalar_2656,
                scalar_2764: p.scalar_2764,
            },
            pump_force: PumpForceInput {
                flags_2476: p.flags_2476,
                mode_multiplier: unintentional_pump_scalar,
                pumping_scalar: pumping.pump_acceleration,
                input_scalar_2660: t.total_mass,
                timestep: p.timestep_2604,
                direction_432: t.forward_velocity,
                normal_threshold: [f32::from_bits(0x3586_37bd); 4],
            },
            straighten: StraightenInput {
                heading_time: 0.0,
                scalar_2764: p.scalar_2764,
                turn: f.turn,
                forward: t.forward,
                velocity,
                normal,
            },
            heading: HeadingInput {
                balance: f.balance,
                manual_turn: p.spin_input_2672,
                flags_2472: p.flags_2472,
                timestep: p.timestep_2604,
                signed_speed: p.scalar_2612,
                manual_curve_input: p.scalar_2652,
                turn_2712: f.raw_turn,
                scalar_2740: riding.heading_adjust_factor(),
                velocity: t.forward_velocity,
                normal,
                angular_velocity: raw(p.vectors_720_784_800_816_832_864[0]),
                transform: ground,
            },
            anti_flip: AntiFlipInput {
                flags_2468: p.flags_2468,
                balance: f.balance,
                axis_96: t.deck[2],
                axis_64: t.deck[0],
                projection_axis_544: axis544,
            },
            manual: ManualInput {
                balance: f.balance,
                flipped_controls: t.control_sign,
                procedural_noise_time: p.state_timer_2664,
                absolute_speed: t.absolute_speed,
                animation_noise: skeleton.board_at_y_delta,
                timestep: p.timestep_2604,
                powersliding: false,
                braking: p.flags_2468 & 0x2000_0000 != 0,
                positive_balance_contact: p.flags_2472 & 0x0800_0000 != 0,
                negative_balance_contact: p.flags_2472 & 0x0400_0000 != 0,
                reversed_point_selection: p.flags_2468 & 0x0010_0000 != 0,
                reference_x: ground[0],
                reference_z: ground[2],
                deck_z: t.deck[2],
                velocity_frame_z: raw(p.effective_anim_transform_192[2]),
                angular_velocity_world: angular,
                correction_point_7888: v(base_trucks[0].translation),
                correction_point_7952: v(base_trucks[1].translation),
            },
            ground_drag: GroundDragInput {
                flags_2468: p.flags_2468,
                absolute_body_speed_2616: t.absolute_speed,
                balance_2720: f.balance,
                scalar_2724: extra.manual_drag_2724,
                ground_normal_y: ground_normal[1],
            },
            contact_time_2756: p.signed_ground_time_2756,
            trajectory_state_1776_bits: extra.trajectory_state_bits,
            anti_flip_nudge: AntiFlipNudgeInput {
                deck_speed_2652: p.scalar_2652,
                deck_axis_96: t.deck[2],
            },
            hang_up: HangUpInput {
                flags_1516: extra.edge_flags,
                deck_speed_2652: p.scalar_2652,
                scalar_84: t.deck[1][1],
                geometry_axis_dot_positive: dot3(hang_axis, hang_axis) > 0.0,
            },
            halfpipe_wheel_catch: HalfpipeWheelCatchInput {
                flags_1516: extra.edge_flags,
                deck_speed_2652: p.scalar_2652,
                deck_x_axis_y: t.deck[0][1],
                deck_y_axis_y: t.deck[1][1],
                signed_deck_distance: dot3(t.deck[1], edge_delta),
            },
            pinning: PinningInput {
                flags_2488: p.flags_2488,
                frames_since_teleport_2584: p.frames_since_teleport_2584 as i32,
                flags_2472: p.flags_2472,
            },
        }
    }
}
/// `SKATE2_TRACE_SKATE3_COAST=1`: once a second on the ground, the speed, the no-input timer and
/// the motion graph's NoInput attribute (flags 2472 bit 23) that runs it.
fn coast_trace(p: &ProcessedPhysicsInput) {
    use std::sync::{OnceLock, atomic::{AtomicU32, Ordering}};
    static ON: OnceLock<bool> = OnceLock::new();
    static TICKS: AtomicU32 = AtomicU32::new(0);
    if !*ON.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_COAST").is_some()) {
        return;
    }
    if TICKS.fetch_add(1, Ordering::Relaxed) % 60 == 0 {
        eprintln!(
            "skate3 coast: speed={:.3} surface_speed={:.3} no_input={} time_since_input={:.3} wheels={} gate2468={:08x} gate2476={}",
            p.scalar_2612,
            p.scalar_2656,
            p.flags_2472 & (1 << 23) != 0,
            p.time_since_last_input_2748,
            p.wheel_count_2556,
            p.flags_2468 & 0x6201_0000,
            p.flags_2476 & 0x0800_0000 != 0,
        );
    }
}

fn raw(v: [u32; 4]) -> [f32; 4] {
    v.map(f32::from_bits)
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn v(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}


/// Scale the animation-requested speed *gain*, not the existing velocity.
/// Preserve the exact native target at 1x; caps/contacts still belong to core.
fn push_target(target:f32,current:f32,strength:f32)->f32 {
    if strength==1. {target} else {current+(target-current).max(0.)*strength}
}

#[cfg(any())]
mod custom_push_tests {
    use super::push_target;
    use skate3_core::{math::Vector3,riding::push::{calculate_acceleration,PushInput,PushLimits}};
    #[test]
    fn higher_limits_alone_do_not_increase_animation_requested_push() {
        let force=|strength,cap|calculate_acceleration(PushInput {
            flags_2468:0x0200_0000,flags_2472:0,target_speed:push_target(3.2,3.,strength),
            current_speed:3.,absolute_body_speed:3.,scale:80.,delta_seconds:1./60.,direction:Vector3::new(0.,0.,1.),
        },PushLimits {maximum_pushable_speed:8.5,low_speed_change:cap,high_speed_change:cap}).vector.z;
        assert_eq!(force(1.,1.5),force(1.,10.));
        assert!((force(5.,10.)/force(1.,10.)-5.).abs()<0.001);
        assert_eq!(force(0.,10.),0.);
        assert_eq!(push_target(3.2,5.,10.),5.);
        assert_eq!(push_target(3.2,5.,1.).to_bits(),3.2f32.to_bits());
    }
}
