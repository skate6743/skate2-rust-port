//! PhysState_Skitching (104), TU3 vtable 0x82327398: Enter 0x82D47278, Update 0x82D477C0 (its
//! UpdateSkateboard 0x82D4AC38) and FillPhysOut 0x82D4C078; Exit is empty. The state's own
//! functions are in skitching/; this module feeds them the Skate 3 controller's inputs and
//! applies what they produce: board forces and angular displacements, the hand IK targets, the
//! deck animation offset and the grab messages for the skitched object.
#[path = "skitching/contact.rs"]
mod skitch_contact;
#[path = "skitching/deck_offset.rs"]
mod skitch_deck_offset;
#[path = "skitching/follow.rs"]
mod skitch_follow;
#[path = "skitching/forces.rs"]
mod skitch_forces;
#[path = "skitching/geometry.rs"]
mod skitch_geometry;
#[path = "skitching/hands.rs"]
mod skitch_hands;
#[path = "skitching/ik.rs"]
mod skitch_ik;
#[path = "skitching/reset.rs"]
mod skitch_reset;
#[path = "skitching/state.rs"]
mod skitch_state;

use super::{
    Skate3ControllerState, Skate3PhysicsState, ground_runtime::GroundInputObservations,
    riding_outputs::RidingPoseInputs,
};
use crate::{
    physical_player_grab_spline::geometry::transform_point,
    physical_player_grind_manager::{
        math::GrindNum,
        view::GrindError,
    },
    physical_player_trajectory_owners::selector_grind::vmax,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
};
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId,
        deck_angular_correction,
        force_queue::QueuedPointForce,
        manual::controller::{self, ManualAngleMeasurement},
    },
    player::{
        input_phase::ProcessedPhysicsInput,
        offboard::grab_scene::{Descriptor, Record},
    },
    riding::{
        anti_flip, collision_response::signed_angle, heading, slide_friction, speed_wobble,
        steering,
    },
};
use skate_data::collections::Collections;
use skitch_ik::SkitchIkInput;
use skitch_state::{END, SkitchControlCvars, SkitchError, SkitchExitForce};
use std::collections::HashMap;

type Vector = [f32; 4];
type Matrix = [Vector; 4];
const ZERO: Vector = [0.0; 4];
const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    ZERO,
];

/// The skitching cvars at their TU3 defaults: kSkitchMinTurnForExit, kStickHandSlowDownEffect,
/// kForceSkitchStick and kFramesToExitSkitching.
const CONTROLS: SkitchControlCvars = SkitchControlCvars {
    min_turn_for_exit: 0.5,
    stick_hand_slow_down: 1.0,
    force_stick: false,
    frames_to_exit: 3,
};
/// kEnableCentrifugalAcceleration, kEnableFakeFollowPointPhysics, kSkitchingUseTorque and
/// UseAuthoredVehicleGrabSplines are on; kSkitchForceLeft/RightHand are off.
const CENTRIFUGAL: bool = true;
const FAKE_FOLLOW_PHYSICS: bool = true;
const USE_TORQUE: bool = true;
const AUTHORED_SPLINES: bool = true;
/// kSkitchCrouchSpringScalar, kDeckBlendAmount, kDeckMaxOffsetDelta and kMinDeckOffsetAngle.
const CROUCH_SPRING: f32 = 0.1;
const DECK_BLEND: f32 = 0.25;
const DECK_MAX_OFFSET_DELTA: f32 = 10.0;
const DECK_MIN_OFFSET_ANGLE: f32 = 2.0;
/// Update 0x82D477F0 disables the hand collision for five frames every tick.
const HAND_COLLISION_FRAMES: u32 = 5;
/// UpdateDeckAnimOffset 0x82D4A350..54: the orientation channel only, refreshed for 15 frames.
const DECK_OFFSET_FRAMES: f32 = 15.0;
/// The skeleton parts whose original animation positions the hand IK reads (Skeleton+14160).
const SKATEBOARD_ROOT: usize = 0;
const LEFT_HAND: usize = 3;
const LEFT_ARM: usize = 5;
const RIGHT_HAND: usize = 7;
const RIGHT_ARM: usize = 9;

/// The `physics_state_skitching` default collection, with its parents, by attribute key.
pub(crate) struct Attribs(HashMap<u64, Vec<u32>>);

impl Attribs {
    fn load(data: &Collections) -> Self {
        use skate_data::attrib_hash::numeric_name;
        let class = numeric_name("physics_state_skitching");
        let mut values = HashMap::new();
        let mut key = String::from("default");
        for _ in 0..=data.entries().len() {
            let id = numeric_name(&key);
            let Some(entry) = data
                .entries()
                .iter()
                .find(|e| numeric_name(&e.class_name) == class && numeric_name(&e.key) == id)
            else {
                break;
            };
            for (name, field) in &entry.fields {
                let Some(field_key) = numeric_name(name)
                    .strip_prefix("Hash_")
                    .and_then(|hex| u64::from_str_radix(hex, 16).ok())
                else {
                    continue;
                };
                values.entry(field_key).or_insert_with(|| words(&field.data));
            }
            if entry.parent.is_empty() {
                break;
            }
            key = entry.parent.clone();
        }
        Self(values)
    }
}

fn words(text: &str) -> Vec<u32> {
    let hex: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    hex.chunks_exact(8)
        .filter_map(|chunk| {
            std::str::from_utf8(chunk)
                .ok()
                .and_then(|word| u32::from_str_radix(word, 16).ok())
        })
        .collect()
}

#[derive(Debug)]
pub(crate) struct StateConstructorError(String);

/// Attrib::Collection::GetData: a missing attribute reads as the zero default record.
fn read_floats<const N: usize>(
    attribs: &Attribs,
    key: u64,
) -> Result<[f32; N], StateConstructorError> {
    let Some(words) = attribs.0.get(&key) else {
        return Ok([0.0; N]);
    };
    if words.len() < N {
        return Err(StateConstructorError(format!(
            "skitching attribute {key:016X} holds {} words, {N} needed",
            words.len()
        )));
    }
    Ok(std::array::from_fn(|i| f32::from_bits(words[i])))
}

mod pushing_position {
    /// PointGraphEval::Evaluate over a PointGraphData8 (header, eight X, eight Y).
    pub(super) fn graph(values: [f32; 20], input: f32) -> f32 {
        crate::physics::PointGraph::<8> {
            x: values[4..12].try_into().expect("eight graph abscissae"),
            y: values[12..20].try_into().expect("eight graph ordinates"),
        }
        .evaluate(input)
    }
}

/// References into this state's own current/future frames.
#[derive(Clone, Copy, Debug)]
pub(crate) enum MatrixSlot {
    Future,
    Current,
}

pub(crate) struct Inertia {
    pub(crate) inverse_mass: f32,
}

/// The state's GrabSpline copy (Skitching+0x470), taken from ProcessedPhysIn's first payload.
pub(crate) struct GrabSpline {
    transform: Matrix,
    pub(crate) p1: Vector,
    pub(crate) p2: Vector,
    pub(crate) length_unit: Option<Vector>,
    pub(crate) velocity: Vector,
    pub(crate) body_position: Vector,
    pub(crate) half_length: f32,
    pub(crate) status: u8,
    pub(crate) inertia: Inertia,
    descriptor: Descriptor,
    points: Vec<Vector>,
}

impl GrabSpline {
    fn from_record(record: &Record) -> Self {
        Self {
            transform: record.frame(),
            p1: record.vector(64),
            p2: record.vector(80),
            length_unit: Some(record.length_unit()),
            velocity: record.velocity(),
            body_position: record.vector(160),
            half_length: record.half_length(),
            status: record.byte(200),
            inertia: Inertia {
                inverse_mass: record.inverse_mass(),
            },
            descriptor: record.descriptor(),
            points: record.points().to_vec(),
        }
    }

    /// GetPosAlongCurveByStraightLength: the offset is measured from the chord centre while
    /// segment progress is projected onto the local first-to-last chord.
    pub(crate) fn position_at_straight_length(
        &self,
        offset: f32,
        num: &mut GrindNum<'_>,
    ) -> Result<Vector, GrindError> {
        if -self.half_length >= offset {
            return Ok(self.p1);
        }
        if offset >= self.half_length {
            return Ok(self.p2);
        }
        let points = &self.points;
        if points.len() <= 1 {
            return Ok(self.p1);
        }
        let offset = if self.status & 0x20 != 0 {
            self.half_length - offset
        } else {
            offset + self.half_length
        };
        let sub = |a: Vector, b: Vector| -> Vector { std::array::from_fn(|i| a[i] - b[i]) };
        let chord = num.normalize_unsafe(sub(points[points.len() - 1], points[0]))?;
        let mut total = 0.0;
        for segment in points.windows(2) {
            let delta = sub(segment[1], segment[0]);
            let projected = vmax(num.dot(delta, chord)?, f32::from_bits(0x38d1_b717));
            total += projected;
            if total > offset {
                let local = if 0.0 >= projected {
                    segment[1]
                } else {
                    let scale = num.reciprocal(projected)? * (total - offset);
                    sub(segment[1], delta.map(|v| v * scale))
                };
                return Ok(transform_point(&self.transform, local));
            }
        }
        Ok(self.p2)
    }
}

/// PhysState_Skitching's retained fields (Skitching+0x24..0x544).
pub(crate) struct Skitching {
    attribs: Option<Attribs>,
    pub(crate) grab_spline_last_space: Option<Matrix>,
    pub(crate) grab_spline_current_space: Option<Matrix>,
    pub(crate) grab_spline_future_space: Option<Matrix>,
    pub(crate) grab_spline_forward_step_transform: Option<Matrix>,
    pub(crate) grab_spline_to_physical_hand_space_transform: Option<Matrix>,
    pub(crate) grab_spline_to_ik_hand_space_transform: Option<Matrix>,
    pub(crate) grab_spline_anim_space_to_ik_hand_space_transform: Option<Matrix>,
    pub(crate) grab_spline_space: Option<MatrixSlot>,
    pub(crate) grab_spline_physical_hand_space: Option<MatrixSlot>,
    pub(crate) grab_spline_ik_hand_space: Option<MatrixSlot>,
    pub(crate) grab_spline_aim_space: Option<Matrix>,
    pub(crate) deck_pos: Option<Vector>,
    pub(crate) vector_to_deck_pos_on_aim_space: Option<Vector>,
    pub(crate) normalized_vector_to_deck_pos_on_aim_space_ground: Option<Vector>,
    pub(crate) vector_to_deck_pos_on_hand_space: Option<Vector>,
    pub(crate) normalized_vector_to_deck_pos_on_hand_space_ground: Option<Vector>,
    pub(crate) vector_to_deck_pos_on_curve_from_base: Option<Vector>,
    pub(crate) normalized_vector_to_deck_pos_on_curve_from_base_ground: Option<Vector>,
    pub(crate) grab_spline_rh_pos: Option<Vector>,
    pub(crate) grab_spline_lh_pos: Option<Vector>,
    pub(crate) grab_spline_follow_point_target: Option<Vector>,
    pub(crate) grab_spline_velocity: Option<Vector>,
    pub(crate) grab_spline_velocity_ground: Option<Vector>,
    pub(crate) normalized_vector_to_skitch_target_ground: Option<Vector>,
    pub(crate) ground_normal: Option<Vector>,
    pub(crate) grab_spline_speed_ground: Option<f32>,
    pub(crate) grab_spline_deck_offset: Option<f32>,
    pub(crate) grab_spline_deck_offset_speed: Option<f32>,
    pub(crate) grab_spline_false_deck_offset_speed_due_to_turning: Option<f32>,
    pub(crate) distance_to_deck_pos_on_curve_from_base_ground: Option<f32>,
    pub(crate) deck_dist: Option<f32>,
    pub(crate) deck_dist_speed: Option<f32>,
    pub(crate) deck_dist_on_aim_space: Option<f32>,
    pub(crate) grab_spline_follow_point_offset: Option<f32>,
    pub(crate) grab_spline_centrifugal_acceleration: Option<f32>,
    pub(crate) grab_spline_turn_acceleration: Option<f32>,
    pub(crate) grab_spline_grip_acceleration: Option<f32>,
    pub(crate) grab_spline_auto_recentering_acceleration: Option<f32>,
    pub(crate) grab_spline_follow_point_acceleration: Option<f32>,
    pub(crate) grab_spline_desired_follow_point_velocity: Option<f32>,
    pub(crate) grab_spline_follow_point_velocity: Option<f32>,
    pub(crate) grab_spline_target_offset: Option<f32>,
    pub(crate) grab_spline_hand_target_offset: Option<f32>,
    pub(crate) grab_spline_hands_dist: Option<f32>,
    pub(crate) grab_spline_hands_dist_delta: Option<f32>,
    pub(crate) predicted_left_hand_extension: Option<f32>,
    pub(crate) predicted_right_hand_extension: Option<f32>,
    pub(crate) turn_scalar: Option<f32>,
    pub(crate) grab_spline_half_length_mag: Option<f32>,
    pub(crate) grab_spline_half_length_mag_with_edge_gap: Option<f32>,
    pub(crate) parametric_position: Option<f32>,
    pub(crate) deck_angle: Option<f32>,
    pub(crate) anim_deck_angle: Option<f32>,
    pub(crate) time_in_skitch_state: Option<f32>,
    pub(crate) desired_heading_angle: Option<f32>,
    pub(crate) contact_speed: Option<f32>,
    pub(crate) time_on_edge_hanging_on_for_dear_life: Option<f32>,
    pub(crate) time_hands_not_moving: Option<f32>,
    pub(crate) left_blend: Option<f32>,
    pub(crate) right_blend: Option<f32>,
    pub(crate) left_blend_target: Option<f32>,
    pub(crate) right_blend_target: Option<f32>,
    pub(crate) grip_hand: Option<i32>,
    pub(crate) anim_crouch: Option<f32>,
    pub(crate) time_grabbing: Option<f32>,
    pub(crate) centrifugal_force_state: Option<i32>,
    pub(crate) follow_point_mode: Option<i32>,
    pub(crate) follow_point_positive_side_clamped_time: Option<f32>,
    pub(crate) follow_point_negative_side_clamped_time: Option<f32>,
    pub(crate) delay_until_reskitchable: Option<f32>,
    pub(crate) delay_until_hands_off_matters: Option<f32>,
    pub(crate) grab_spline: Option<GrabSpline>,
    pub(crate) grab_spline_desc: Option<Descriptor>,
    pub(crate) exit_count: Option<i32>,
    pub(crate) skitch_state: Option<i32>,
    pub(crate) left_hand_state: Option<i32>,
    pub(crate) right_hand_state: Option<i32>,
    pub(crate) last_left_hand_state: Option<i32>,
    pub(crate) last_right_hand_state: Option<i32>,
    pub(crate) grab_spline_target_offset_locked: Option<bool>,
    pub(crate) grab_spline_target_offset_active: Option<bool>,
    pub(crate) hands_moving: Option<bool>,
    pub(crate) force_exit: Option<bool>,
    pub(crate) push_forward_on_exit: Option<bool>,
    pub(crate) push_back_on_exit: Option<bool>,
    pub(crate) grab_spline_valid: Option<bool>,
    pub(crate) in_air: Option<bool>,
    pub(crate) skitch_steering_active: Option<bool>,
    pub(crate) first_eval: Option<bool>,
    /// Skate 3 +0x330: UpdateSkateboard's speed force, zeroed before it is calculated.
    pub(crate) speed_force: Option<Vector>,
    /// Skate 3 +0x3E4 and the +0x541 0x10/0x08 bits: the push target speed, the PushContact
    /// latch and the brake contact.
    pub(crate) push_target_speed: Option<f32>,
    pub(crate) push_latched: Option<bool>,
    pub(crate) braking: Option<bool>,
}

/// FillPhysOut's animation and ground publication, which the PhysOut record does not carry.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Output {
    /// Animation +0x84/+0x8C/+0x88.
    pub crouch: f32,
    pub grip_hand: i32,
    pub hand_speed: f32,
    /// Ground +0x118/+0x11C/+0x120/+0x124/+0x134.
    pub spline_height: f32,
    pub contact_speed: f32,
    pub parametric_position: f32,
    pub delay_until_reskitchable: f32,
    pub push_target_speed: f32,
    /// State +0x35.
    pub braking: bool,
}

/// The grab manager calls of one Update: ApplyForceToGrabSplineObjectDeferred (vtable +0x24)
/// for the push and brake contacts, then the grabbing notification (+0x28, message type 2).
#[derive(Clone, Copy, Debug)]
pub(crate) enum MessageKind {
    ApplyForce {
        force: Vector,
        position: Vector,
        torque: Vector,
    },
    NotifyGrabbing,
    NotifyLocalPlayer,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Message {
    pub descriptor: Descriptor,
    pub kind: MessageKind,
}

impl Skitching {
    pub(crate) fn load(data: &Collections) -> Self {
        Self {
            attribs: Some(Attribs::load(data)),
            grab_spline_last_space: None,
            grab_spline_current_space: None,
            grab_spline_future_space: None,
            grab_spline_forward_step_transform: None,
            grab_spline_to_physical_hand_space_transform: None,
            grab_spline_to_ik_hand_space_transform: None,
            grab_spline_anim_space_to_ik_hand_space_transform: None,
            grab_spline_space: None,
            grab_spline_physical_hand_space: None,
            grab_spline_ik_hand_space: None,
            grab_spline_aim_space: None,
            deck_pos: None,
            vector_to_deck_pos_on_aim_space: None,
            normalized_vector_to_deck_pos_on_aim_space_ground: None,
            vector_to_deck_pos_on_hand_space: None,
            normalized_vector_to_deck_pos_on_hand_space_ground: None,
            vector_to_deck_pos_on_curve_from_base: None,
            normalized_vector_to_deck_pos_on_curve_from_base_ground: None,
            grab_spline_rh_pos: None,
            grab_spline_lh_pos: None,
            grab_spline_follow_point_target: None,
            grab_spline_velocity: None,
            grab_spline_velocity_ground: None,
            normalized_vector_to_skitch_target_ground: None,
            ground_normal: None,
            grab_spline_speed_ground: None,
            grab_spline_deck_offset: None,
            grab_spline_deck_offset_speed: None,
            grab_spline_false_deck_offset_speed_due_to_turning: None,
            distance_to_deck_pos_on_curve_from_base_ground: None,
            deck_dist: None,
            deck_dist_speed: None,
            deck_dist_on_aim_space: None,
            grab_spline_follow_point_offset: None,
            grab_spline_centrifugal_acceleration: None,
            grab_spline_turn_acceleration: None,
            grab_spline_grip_acceleration: None,
            grab_spline_auto_recentering_acceleration: None,
            grab_spline_follow_point_acceleration: None,
            grab_spline_desired_follow_point_velocity: None,
            grab_spline_follow_point_velocity: None,
            grab_spline_target_offset: None,
            grab_spline_hand_target_offset: None,
            grab_spline_hands_dist: None,
            grab_spline_hands_dist_delta: None,
            predicted_left_hand_extension: None,
            predicted_right_hand_extension: None,
            turn_scalar: None,
            grab_spline_half_length_mag: None,
            grab_spline_half_length_mag_with_edge_gap: None,
            parametric_position: None,
            deck_angle: None,
            anim_deck_angle: None,
            time_in_skitch_state: None,
            desired_heading_angle: None,
            contact_speed: None,
            time_on_edge_hanging_on_for_dear_life: None,
            time_hands_not_moving: None,
            left_blend: None,
            right_blend: None,
            left_blend_target: None,
            right_blend_target: None,
            grip_hand: None,
            anim_crouch: None,
            time_grabbing: None,
            centrifugal_force_state: None,
            follow_point_mode: None,
            follow_point_positive_side_clamped_time: None,
            follow_point_negative_side_clamped_time: None,
            delay_until_reskitchable: None,
            delay_until_hands_off_matters: None,
            grab_spline: None,
            grab_spline_desc: None,
            exit_count: None,
            skitch_state: None,
            left_hand_state: None,
            right_hand_state: None,
            last_left_hand_state: None,
            last_right_hand_state: None,
            grab_spline_target_offset_locked: None,
            grab_spline_target_offset_active: None,
            hands_moving: None,
            force_exit: None,
            push_forward_on_exit: None,
            push_back_on_exit: None,
            grab_spline_valid: None,
            in_air: None,
            skitch_steering_active: None,
            first_eval: None,
            speed_force: None,
            push_target_speed: None,
            push_latched: None,
            braking: None,
        }
    }

    /// FillPhysOut 0x82D4C078's animation and ground values.
    fn output(&self, effective_y: f32) -> Result<Output, SkitchError> {
        use skitch_state::need;
        let physical = self.space(self.grab_spline_physical_hand_space)?;
        let hands = need(self.grab_spline_hands_dist, "hands distance")?;
        Ok(Output {
            crouch: need(self.anim_crouch, "crouch")?,
            grip_hand: need(self.grip_hand, "grip hand")?,
            // 0x82D4C0C4..D4: the hands' delta times 59.999996.
            hand_speed: need(self.grab_spline_hands_dist_delta, "hands delta")?
                * f32::from_bits(0x426f_ffff),
            // 0x82D4C104..34: relative to ProcessedPhysIn's effective position (+0xB0).
            spline_height: physical[0][1].mul_add(hands, physical[3][1]) - effective_y,
            contact_speed: need(self.contact_speed, "contact speed")?,
            parametric_position: need(self.parametric_position, "parametric position")?,
            delay_until_reskitchable: need(self.delay_until_reskitchable, "reskitch delay")?,
            push_target_speed: need(self.push_target_speed, "push target speed")?,
            braking: need(self.braking, "braking")?,
        })
    }
}

/// SKATE2_TRACE_SKATE3_SKITCH: the ground grab check's candidates and each Skitching tick.
pub(crate) fn trace_enabled() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_SKITCH").is_some())
}

fn raw(v: [u32; 4]) -> Vector {
    v.map(f32::from_bits)
}
fn xyz(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn skitch(error: SkitchError) -> String {
    format!("Skitching: {error:?}")
}

/// Enter 0x82D47278: Reset, DisableBoardAnimation, then the processed GrabSpline and its
/// descriptor are copied and the forced exit cleared.
pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let s = &mut skater.skitching;
    s.reset().map_err(|e| format!("Skitching Reset: {e:?}"))?;
    physics
        .board
        .hook_mut()
        .drive
        .disable_animation(&mut skater.ground_lifecycle.board_animated_290);
    if let Some(record) = skater.offboard_grab.published_primary() {
        let spline = GrabSpline::from_record(record);
        s.grab_spline_desc = Some(spline.descriptor);
        s.grab_spline = Some(spline);
    }
    s.force_exit = Some(false);
    skater.skitch_messages.clear();
    Ok(())
}

/// Update 0x82D477C0.
pub(crate) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = skater.player_input.processed;
    let toolkit = *skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Skitching requires PlayerInput's current board toolkit")?;
    let mut reckoning = HostReckoningNumeric;
    let mut sin_cos = HostReckoningSinCosNumeric;
    let mut n = GrindNum {
        n: &mut reckoning,
        s: &mut sin_cos,
    };
    // 0x82D477E4..EC: +0x540 bit 7 is ProcessedPhysIn+2480 bit 22, the first GrabSpline
    // publication.
    let ready = p.flags_2480 & 0x0040_0000 != 0;
    skater.skitching.grab_spline_valid = Some(ready);
    skater
        .skeleton_collision
        .disable_handplant_contacts(HAND_COLLISION_FRAMES);
    if ready {
        if let Some(record) = skater.offboard_grab.published_primary() {
            skater.skitching.grab_spline = Some(GrabSpline::from_record(record));
        }
        if let Err(error) = grabbing(physics, skater, &p, &toolkit, &mut n) {
            // PC port: a failed calculation lets go of the object, as a forced exit does, rather
            // than failing the whole physics tick.
            eprintln!("skate3 skitch: {error}");
            skater.skitching.force_exit = Some(true);
            skater.skitching.skitch_state = Some(END);
        }
    }
    // CheckForExit 0x82D4AFB8: the world-grab intent is ProcessedPhysIn+2476 bit 22.
    let s = &mut skater.skitching;
    let exit = s
        .check_exit(p.flags_2476 & 0x0040_0000 != 0, &CONTROLS)
        .map_err(skitch)?;
    let kinds: &[SkitchExitForce] = match exit {
        SkitchExitForce::None => &[],
        SkitchExitForce::Back => &[SkitchExitForce::Back],
        SkitchExitForce::Forward => &[SkitchExitForce::Forward],
        SkitchExitForce::Both => &[SkitchExitForce::Back, SkitchExitForce::Forward],
    };
    for &kind in kinds {
        // CalcExitPush*Force read ProcessedPhysIn+2660, the board's total mass.
        let force = s.exit_force(kind, toolkit.total_mass, &mut n).map_err(skitch)?;
        physics.board.forces_mut().append(QueuedPointForce {
            tag: 6,
            force_world: xyz(force),
            point_body: Vector3::ZERO,
        });
    }
    s.finish_exit_check(&CONTROLS).map_err(skitch)?;
    if trace_enabled() {
        eprintln!(
            "skate3 skitch: ready={ready} state={:?} hands={:?}/{:?} offset={:?} half={:?} speed={:?} force_exit={:?} exit_count={:?} in_air={:?} spline_speed={:?} board_speed={} desc={:?}",
            s.skitch_state, s.left_hand_state, s.right_hand_state, s.grab_spline_deck_offset,
            s.grab_spline_half_length_mag, s.speed_force, s.force_exit, s.exit_count, s.in_air,
            s.grab_spline_speed_ground, p.scalar_2612, s.grab_spline_desc,
        );
    }
    // 0x82D47968..A4: the Ground job's input packet with Reckoning+1136 from ProcessedPhysIn+528
    // and +1200 from ProcessedPhysIn+96 (the deck's forward axis), then
    // UpdateGroundStatesFixedHeading.
    let com = p.animation_com_to_deck_752.map(f32::from_bits);
    physics.riding.update_ground_reckoning_with_heading(
        &physics.board,
        RidingPoseInputs {
            com_to_deck: Vector3::new(com[0], com[1], com[2]),
            body_spin: skater.animation_input.extra.physical_body_spin,
        },
        p.flags_2468,
        skater.animation_input.fields.balance,
        p.flags_2476 & 0x4000_0000 != 0,
        &p,
        toolkit.deck[2],
    );
    // 0x82D479B0: the Skeleton's ground update.
    super::input_phase::update_ground(physics, skater)?;
    skater.skitching.first_eval = Some(false);
    Ok(())
}

/// The block of Update 0x82D47810..47960 that runs with a published GrabSpline.
fn grabbing(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    p: &ProcessedPhysicsInput,
    toolkit: &skate3_core::physics::board_toolkit::BoardToolkit,
    n: &mut GrindNum<'_>,
) -> Result<(), String> {
    let s = &mut skater.skitching;
    // UpdateLocalPhysicalParameters 0x82D47FC0: the board's wheel height and time without
    // wheel contact.
    s.update_local_physical(
        physics.riding.wheel_lines.minimum_distance,
        physics.riding.ground.time_without_wheel_contact,
    )
    .map_err(skitch)?;
    // 0x82D4781C..B0: ProcessedPhysIn+2676 (turn) against +2672 (spin).
    s.update_local_control(skater.animation_input.fields.turn, p.spin_input_2672);
    // 0x82D478B4..C0: the ground normal is ProcessedPhysIn+464.
    s.ground_normal = Some(raw(p.vectors_464_480_496_512_528[0]));
    // UpdateGrabSpline reads the deck from ProcessedPhysIn+128, the effective transform.
    s.update_grab_geometry(toolkit.effective, AUTHORED_SPLINES, n)
        .map_err(skitch)?;
    s.check_state_change(&CONTROLS).map_err(skitch)?;
    s.update_follow_point(toolkit.effective[2], CENTRIFUGAL, FAKE_FOLLOW_PHYSICS, n)
        .map_err(skitch)?;
    s.update_skitch_state(&CONTROLS).map_err(skitch)?;
    hand_ik(skater, p, n).map_err(skitch)?;
    board(physics, skater, p, toolkit, n)?;
    let s = &mut skater.skitching;
    // UpdateDeckAnimOffset 0x82D4A0C0: ProcessedPhysIn+2488 bit 23 is the PushSpeed record.
    if let Some(offset) = s
        .deck_animation_offset(
            DECK_BLEND,
            DECK_MAX_OFFSET_DELTA,
            DECK_MIN_OFFSET_ANGLE,
            p.flags_2488 & 0x0080_0000 != 0,
            n,
        )
        .map_err(skitch)?
    {
        let board_offset = &mut skater.animated_skeleton.board_offset;
        board_offset.transform = offset;
        board_offset.orientation_frames = DECK_OFFSET_FRAMES;
        board_offset.orientation_refreshed = true;
    }
    // sub_82D47BD8 (PushContact, ProcessedPhysIn+2488 bit 29) and sub_82D479C8 (the Brake
    // record, +2728).
    let push = s
        .push_contact(
            p.flags_2488 & 0x2000_0000 != 0,
            p.state_timer_2664,
            toolkit.effective[3],
            toolkit.effective[2],
            n,
        )
        .map_err(skitch)?;
    let brake = s
        .brake_contact(skater.animation_input.fields.brake, n)
        .map_err(skitch)?;
    let Some(descriptor) = s.grab_spline.as_ref().map(|spline| spline.descriptor) else {
        return Err("Skitching published without its GrabSpline".into());
    };
    for contact in [push, brake].into_iter().flatten() {
        skater.skitch_messages.push(Message {
            descriptor,
            kind: MessageKind::ApplyForce {
                force: contact.force,
                position: contact.position,
                torque: contact.torque,
            },
        });
    }
    // 0x82D47918..5C: GrabSpline::Init, RequestGrabSplineData and the grabbing notification.
    // The Skate 2 world's vehicles take the local player's grab as its own message.
    skater.offboard_grab.request_primary(descriptor);
    skater.skitch_messages.push(Message {
        descriptor,
        kind: MessageKind::NotifyGrabbing,
    });
    skater.skitch_messages.push(Message {
        descriptor,
        kind: MessageKind::NotifyLocalPlayer,
    });
    Ok(())
}

/// The original (unblended) animation position of one skeleton part, ProcessData+14160.
fn original_part(skater: &Skate3ControllerState, part: usize) -> Vector {
    skater
        .animation
        .packet
        .hierarchy
        .get(skater.animated_skeleton.bone_indices[part])
        .map_or(ZERO, |bone| bone[3])
}

/// ClampVectorAWithinRangeOfVectorB.
fn clamp_within(target: Vector, centre: Vector, maximum: f32) -> Vector {
    let delta: Vector = std::array::from_fn(|i| target[i] - centre[i]);
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if length > maximum && length > 0.0 {
        std::array::from_fn(|i| centre[i] + delta[i] * (maximum / length))
    } else {
        target
    }
}

/// UpdateHandIK 0x82D4A378, then Skeleton::SetLeft/RightHandIKTargetPosition (0x82BD9728 and
/// its right-hand twin): each target within the arm's reach of that arm's animated position.
fn hand_ik(
    skater: &mut Skate3ControllerState,
    p: &ProcessedPhysicsInput,
    n: &mut GrindNum<'_>,
) -> Result<(), SkitchError> {
    let anim_to_world = skater.animated_skeleton.roots.animation_to_world;
    let left_arm = original_part(skater, LEFT_ARM);
    let right_arm = original_part(skater, RIGHT_ARM);
    let input = SkitchIkInput {
        anim_to_world,
        left_hand: original_part(skater, LEFT_HAND),
        right_hand: original_part(skater, RIGHT_HAND),
        left_arm,
        right_arm,
        world_grab_y: skater.animation_input.extra.world_grab_y,
        world_grab_z: skater.animation_input.extra.world_grab_z,
        mirrored: p.flags_2476 & 4 != 0,
        authored: AUTHORED_SPLINES,
        force_left: false,
        force_right: false,
        crouch_spring: CROUCH_SPRING,
    };
    let targets = skater.skitching.update_hand_ik(input, n)?;
    let ik = &mut skater.foot_ik.state;
    for (limb, target, arm) in [(2, &targets[0], left_arm), (3, &targets[1], right_arm)] {
        if let Some(target) = target {
            let arm = transform_point(&anim_to_world, arm);
            ik.external_targets[limb].world_position = clamp_within(target.position, arm, target.maximum);
            ik.limbs[limb].external_target_set = true;
            ik.limbs[limb].target_blend = target.blend;
        }
    }
    Ok(())
}

/// UpdateSkateboard 0x82D4AC38.
fn board(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    p: &ProcessedPhysicsInput,
    toolkit: &skate3_core::physics::board_toolkit::BoardToolkit,
    n: &mut GrindNum<'_>,
) -> Result<(), String> {
    // Skateboard::UpdatePhysicsError.
    let target = skater.animated_skeleton.board_frames.animation_target;
    skater
        .skeleton_air
        .capture_physics_error(&physics.board, &target);
    let edge = skater.ground_lifecycle.edge;
    let settings = std::sync::Arc::clone(&skater.ground_settings);
    let mut input = settings.input(
        toolkit,
        p,
        &skater.animation_input,
        &skater.ground.pumping,
        skater
            .ground
            .pumping_settings
            .mode(p.state_variant_index_2528)?
            .unintentional_scalar,
        &physics.riding,
        &skater.animated_skeleton,
        physics.settings.step.base_truck_transforms,
        GroundInputObservations {
            manual_drag_2724: skater.ground_lifecycle.manual_drag_2724,
            trajectory_state_bits: p.external_physics_1616.flags,
            edge_flags: edge.map_or(0, |e| e.flags),
            edge_point: edge.map_or([0.0; 4], |e| e.point),
        },
    );
    let board = settings.board();
    let com_to_deck = skater.animated_skeleton.record.com_to_deck_world;
    let com_height = (com_to_deck[0] * com_to_deck[0]
        + com_to_deck[1] * com_to_deck[1]
        + com_to_deck[2] * com_to_deck[2])
        .sqrt();
    let bone_board_z = original_part(skater, SKATEBOARD_ROOT)[2];
    let s = &mut skater.skitching;
    s.speed_force = Some(ZERO);
    let in_air = s.in_air == Some(true);
    // sub_82D92440 with no push or damped-turn retention, then (on the ground) the speed
    // wobble 0x82C03638 from the centre of mass's distance to the deck (Skeleton+11008).
    let tilt = |state: &mut speed_wobble::SpeedWobbleState| {
        if in_air {
            return 0.0;
        }
        let tilt = steering::calculate_tilt(board.steering, input.steering, None, None);
        let mut wobble = input.speed_wobble;
        wobble.tilt = tilt;
        wobble.center_of_mass_height = com_height;
        wobble.activation_threshold = settings.wobble_activation;
        wobble.amplitude_multiplier = settings.wobble_amplitude;
        speed_wobble::calculate(state, board.speed_wobble, wobble)
    };
    if s.skitch_state == Some(END) {
        let tilt = tilt(&mut skater.ground.wobble);
        skater.ground.steering.update(
            tilt,
            board.steering.tilt_blending,
            p.flags_2468,
            p.flags_2472,
        );
        return Ok(());
    }
    let speed = s
        .speed_force(
            bone_board_z,
            p.flags_2476 & 4 != 0,
            skater.animation_input.extra.world_grab_z,
            toolkit.total_mass,
            p.scalar_2612,
            n,
        )
        .map_err(skitch)?;
    input.slide_friction.heading_time = s.time_in_skitch_state.unwrap_or(0.0);
    let slide = slide_friction::calculate(board.slide_friction, &input.slide_friction);
    let heading_torque = s
        .heading_displacement(toolkit.effective, USE_TORQUE, n)
        .map_err(skitch)?;
    let steering_active = s.skitch_steering_active == Some(true);
    let tilt = tilt(&mut skater.ground.wobble);
    let anti = anti_flip::calculate(board.anti_flip, &input.anti_flip);
    skater.ground.steering.update(
        tilt,
        board.steering.tilt_blending,
        p.flags_2468,
        p.flags_2472,
    );
    // 0x82D4ADC4..AE50: a manual's heading (ProcessedPhysIn+2720 balance and +2672 spin)
    // targets an angular displacement.
    if skater.animation_input.fields.balance != 0.0 && p.spin_input_2672 != 0.0 {
        let heading = heading::calculate(board.heading, &input.heading, &mut skater.ground.heading_previous);
        skater
            .ground_runtime
            .apply_angular_target(&mut physics.board, heading);
    }
    let mut manual_input = input.manual;
    manual_input.powersliding = false;
    let manual = controller::calculate(
        &mut skater.ground.manual,
        board.manual,
        board.manual_mode,
        &manual_input,
        &mut Angle,
    )
    .map_err(|e| format!("Skitching manual controller: {e:?}"))?;
    skater
        .ground_runtime
        .apply_angular_displacement(&mut physics.board, manual.angular_displacement);
    skater
        .ground_runtime
        .apply_angular_displacement(&mut physics.board, anti);
    if steering_active {
        deck_angular_correction::apply_axis_displacement(
            &mut physics.board.bodies_mut()[BodyId::Deck.index()].rates,
            xyz(heading_torque),
        );
    }
    let s = &mut skater.skitching;
    s.speed_force = Some(speed);
    let queue = physics.board.forces_mut();
    queue.append(QueuedPointForce {
        tag: 6,
        force_world: xyz(speed),
        point_body: Vector3::ZERO,
    });
    queue.append(QueuedPointForce {
        tag: 1,
        force_world: Vector3::new(slide[0], slide[1], slide[2]),
        point_body: Vector3::new(slide[4], slide[5], slide[6]),
    });
    s.time_grabbing = Some(s.time_grabbing.unwrap_or(0.0) + f32::from_bits(0x3c88_8889));
    Ok(())
}

struct Angle;
impl ManualAngleMeasurement for Angle {
    type Error = std::convert::Infallible;
    fn angle_between(&mut self, a: Vector, b: Vector, c: Vector) -> Result<f32, Self::Error> {
        Ok(signed_angle(xyz(a), xyz(b), xyz(c)))
    }
}

/// FillPhysOut 0x82D4C078: the grab (OffBoard+0x130) holds while the spline is published and
/// no exit was forced; State+0x24/+0x28 carry the descriptor.
pub(crate) fn fill(
    s: &Skitching,
    effective_y: f32,
    physical: &mut skate3_core::player::input_phase::PhysicalPlayerInput,
) -> Result<Output, String> {
    let output = s.output(effective_y).map_err(skitch)?;
    if s.force_exit != Some(true) && s.grab_spline_valid == Some(true) {
        physical.off_board.flag_304 = 1;
    }
    physical.ground.scalar_292 = output.delay_until_reskitchable;
    if let Some(descriptor) = s.grab_spline_desc {
        physical.state.counter_36 = descriptor.kind;
        physical.state.skitch_value_40 = descriptor.id;
    }
    Ok(output)
}
