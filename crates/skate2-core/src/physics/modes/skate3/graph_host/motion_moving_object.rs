//! TU3 MovingObject (ctor 0x82bb64b8, Begin 0x82bb6860, Update 0x82bb6a20, ClampAngle
//! 0x82bb79d8), InitMovingObjects::Begin 0x82bb63d8 and AngleAdjustByAnimationEndLessThan
//! (factory 0x82bc6398, Activation 0x82ba7598) over SkaterAnim's 0x28-byte
//! MovingObjectsCoefficients. Skate 2's MovingObject is the same code reading the
//! Animation_MovingObjects cvars; TU3 folds their defaults into constants.
use super::motion_animation::MotionAnimation;
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, wrap_turns_to_radians};
use skate3_core::{
    animation::{
        playback_parameters::{AttributeSink, SettableAttribute},
        skeleton_input::name::encode,
    },
    point_graph::PointGraph,
};
use skate_data::{collections::Collections, state_graph::attributes::Attributes};

const PI: f32 = f32::from_bits(0x4049_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const FRAMES_PER_SECOND: f32 = f32::from_bits(0x426f_ffff);
/// kMOAngleMaxDelta (2 degrees): flt_822F9010 and its negation flt_822F900C.
const ANGLE_MAX_DELTA: f32 = f32::from_bits(0x3d0e_fa35);
/// kMOStrengthMaxDelta.
const STRENGTH_MAX_DELTA: f32 = f32::from_bits(0x3c23_d70a);
/// kMOSpeedMaxDelta and kMOCrouchMaxDelta.
const SPEED_MAX_DELTA: f32 = f32::from_bits(0x3cf5_c28f);
const CROUCH_MAX_DELTA: f32 = f32::from_bits(0x3cf5_c28f);
/// The reciprocal of kMORawSpeedMax (2.5); Update multiplies by it.
const INVERSE_RAW_SPEED_MAX: f32 = f32::from_bits(0x3ecc_cccd);
/// The strength curve's class (Skate 2's moving-object attributes) and its field at layout 0xA0.
const CLASS: &str = "0x97b0272a2a7e54f8";
const COLLECTION: &str = "0xd7edbd362d7d2152";
const STRENGTH_CURVE: &str = "0xc3a15e5de438066f";

/// SkaterAnim::MovingObjectsCoefficients, shared with each MovingObject instance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coefficients {
    pub last_full_angle: f32,
    pub last_angle: f32,
    pub last_speed: f32,
    pub last_strength: f32,
    pub last_rotation: f32,
    pub last_crouch: f32,
    pub blend_target_angle: f32,
    pub current_direction: u32,
    pub last_direction: u32,
    /// +0x24; bit 7 marks the push/pull flip handled.
    pub flags: u8,
}

impl Coefficients {
    /// InitMovingObjects::Begin: every field cleared, the flags byte keeping its low seven bits.
    pub fn reset(&mut self) {
        *self = Self {
            flags: self.flags & 0x7f,
            ..Self::default()
        };
    }
}

/// The pushing state's FillPhysOut (0x82d46fd0) record the behaviour reads (PhysOut+0x48
/// +0..+0x1C): movement and force along the hands' X and Z, and the animation crouch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Physical {
    /// X (+4) then Z (+0).
    pub movement: [f32; 2],
    /// X (+0xC) then Z (+8).
    pub force: [f32; 2],
    /// +0x1C.
    pub crouch: f32,
}

/// The authored behaviour: `direction` and `BlendOutOverNextCycle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Behaviour {
    pub direction: u32,
    pub blend_out_over_next_cycle: bool,
}

impl Behaviour {
    pub fn parse(a: &Attributes<'_>) -> Self {
        const DIRECTIONS: [&str; 12] = [
            "PUSHLEFT",
            "PUSHRIGHT",
            "PUSHLEFTToRIGHT",
            "PUSHRIGHTToLEFT",
            "PUSHToPULLLEFT",
            "PUSHToPULLRIGHT",
            "PULLLEFT",
            "PULLRIGHT",
            "PULLLEFTToRIGHT",
            "PULLRIGHTToLEFT",
            "PULLToPUSHLEFT",
            "PULLToPUSHRIGHT",
        ];
        let text = a.text("direction").unwrap_or("");
        Self {
            // eMoveObjectDirection: STILL (0, also any other text), then the list above.
            direction: DIRECTIONS
                .iter()
                .position(|name| *name == text)
                .map_or(0, |index| index as u32 + 1),
            blend_out_over_next_cycle: a.boolean_byte("BlendOutOverNextCycle", 0) != 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    strength: PointGraph<8>,
}

impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let words = data.words::<20>(CLASS, COLLECTION, STRENGTH_CURVE)?;
        Ok(Self {
            strength: PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
                y: std::array::from_fn(|i| f32::from_bits(words[12 + i])),
            },
        })
    }
}

/// The MotionHost's SkaterAnim block and the latest pushing output.
#[derive(Clone, Debug)]
pub struct MovingObjects {
    pub coefficients: Coefficients,
    pub physical: Physical,
    settings: Settings,
}

impl MovingObjects {
    pub fn new(settings: Settings) -> Self {
        Self {
            coefficients: Coefficients::default(),
            physical: Physical::default(),
            settings,
        }
    }

    /// MovingObject::Begin: the instance takes SkaterAnim's block, the direction history moves
    /// on, and the next animation mirrors when the direction's side is not the leading foot.
    /// The block is not published back until Update.
    pub fn begin(
        &self,
        instance: &mut Coefficients,
        behaviour: Behaviour,
        animation: &mut MotionAnimation,
    ) -> Result<(), String> {
        *instance = self.coefficients;
        instance.last_direction = instance.current_direction;
        instance.current_direction = behaviour.direction;
        instance.flags &= 0x7f;
        let foot_forward = foot_forward(animation);
        let direction = behaviour.direction;
        let mirror =
            (is_right(direction) && foot_forward == 0) || (is_left(direction) && foot_forward == 1);
        // SetMirrorNextAnim(mirror, mirror).
        let flags = animation
            .skater_animation_flags
            .as_mut()
            .ok_or("MovingObject requires SkaterAnim flags")?;
        *flags = (*flags & !0x0060_0000) | (u32::from(mirror) << 22) | (u32::from(mirror) << 21);
        Ok(())
    }

    /// MovingObject::Update, ending with SetMovingObjectsCoefficients.
    pub fn update(
        &mut self,
        instance: &mut Coefficients,
        behaviour: Behaviour,
        animation: &mut MotionAnimation,
    ) -> Result<(), String> {
        let numeric = &mut HostReckoningNumeric;
        let Physical {
            movement,
            force,
            crouch,
        } = self.physical;
        let (raw_speed, aligned_force) = speed_and_aligned_force(movement, force, numeric)?;
        let strength = bound(
            self.settings.strength.evaluate(aligned_force),
            instance.last_strength - STRENGTH_MAX_DELTA,
            instance.last_strength + STRENGTH_MAX_DELTA,
        );
        instance.last_strength = strength;
        let speed_target =
            crate::camera_compass::runtime::math::unit_fraction(raw_speed * INVERSE_RAW_SPEED_MAX);
        let smoothed_crouch = bound(
            crouch,
            instance.last_crouch - CROUCH_MAX_DELTA,
            instance.last_crouch + CROUCH_MAX_DELTA,
        );
        instance.last_crouch = smoothed_crouch;
        let speed = bound(
            speed_target,
            instance.last_speed - SPEED_MAX_DELTA,
            instance.last_speed + SPEED_MAX_DELTA,
        );
        let mut last_angle = instance.last_full_angle;
        if ((is_pull(instance.last_direction) && is_push(instance.current_direction))
            || (is_push(instance.last_direction) && is_pull(instance.current_direction)))
            && instance.flags & 0x80 == 0
        {
            instance.flags |= 0x80;
            last_angle = angle_wrap(last_angle + PI);
        }
        let direction = behaviour.direction;
        let step = if is_transition(direction) {
            let time = animation.current_time()?;
            let length = animation.current_length()?;
            let remaining = (length - time)
                + if behaviour.blend_out_over_next_cycle {
                    length
                } else {
                    0.0
                };
            instance.blend_target_angle = 0.0;
            if is_pull(direction) {
                instance.blend_target_angle = PI;
            }
            angle_wrap(instance.blend_target_angle - last_angle) / frames(remaining)
        } else {
            let heading = crate::camera_compass::runtime::math::camera_atan2_pending(
                -movement[0],
                movement[1],
                numeric,
            )
            .map_err(|e| format!("MovingObject heading: {e:?}"))?
            .finish();
            bound(
                angle_wrap(heading - last_angle),
                -ANGLE_MAX_DELTA,
                ANGLE_MAX_DELTA,
            )
        };
        let (angle, coefficient) =
            directional_angle(direction, angle_wrap(last_angle + step), instance.last_angle);
        instance.last_full_angle = angle;
        instance.last_angle = coefficient;
        instance.last_speed = speed;
        // Animatable vt+0x50 (normalized false, sequence -1), in native order.
        for (name, value) in [
            (&b"angle"[..], coefficient),
            (b"strength", strength),
            (b"crouch", smoothed_crouch),
            (b"Crouch", crouch),
            (b"Speed", speed),
        ] {
            animation.set_attribute(SettableAttribute {
                name: encode(name),
                value,
                normalized: false,
                sequence_id: -1,
            });
        }
        self.coefficients = *instance;
        Ok(())
    }
}

/// AngleAdjustByAnimationEndLessThan: the per-frame turn still needed to reach the blend target
/// by the current animation's end is under the authored degrees.
pub fn angle_adjust_by_animation_end_less_than(
    coefficients: &Coefficients,
    animation: &MotionAnimation,
    max_delta_degrees: f32,
) -> Result<bool, String> {
    let remaining = animation.current_length()? - animation.current_time()?;
    let delta = angle_wrap(coefficients.blend_target_angle - coefficients.last_full_angle);
    let amount = (1.0 / frames(remaining)) * f32::from_bits(delta.to_bits() & 0x7fff_ffff);
    Ok(amount < max_delta_degrees * DEGREES_TO_RADIANS)
}

/// SkaterAnim::FootForward (ISkaterAnim vt[8]), without the fakie inversion of
/// Actor::GetFootForwardWithFakie 0x82592b68.
fn foot_forward(animation: &MotionAnimation) -> u32 {
    let (natural, relative) = (animation.natural_stance, animation.relative_stance);
    u32::from((natural == 0 && relative == 0) || (natural == 1 && relative == 1))
}

/// Remaining seconds as frames, at least one.
fn frames(remaining: f32) -> f32 {
    let frames = remaining * FRAMES_PER_SECOND;
    if 1.0 - frames >= 0.0 { 1.0 } else { frames }
}

fn angle_wrap(angle: f32) -> f32 {
    wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983))
}

/// VRSQRTEFP with two refinements, the zero lane selecting zero.
fn sqrt(squared: f32, numeric: &mut HostReckoningNumeric) -> Result<f32, String> {
    let estimate = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(|e| format!("MovingObject sqrt: {e:?}"))?;
    let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt(squared, estimate);
    Ok(if squared == 0.0 { 0.0 } else { squared * inverse })
}

/// The movement speed and the force's length along the movement (zero when it opposes it),
/// both zero under 1e-4 squared movement.
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native BLE/BLT do not take unordered branches"
)]
fn speed_and_aligned_force(
    movement: [f32; 2],
    force: [f32; 2],
    numeric: &mut HostReckoningNumeric,
) -> Result<(f32, f32), String> {
    let [x, z] = movement;
    let squared = (x * x) + (z * z);
    if squared <= f32::from_bits(0x38d1_b717) {
        return Ok((0.0, 0.0));
    }
    let speed = sqrt(squared, numeric)?;
    let dot = (force[0] * x) + (force[1] * z);
    let projection = dot / squared;
    let (projected_x, projected_z) = (x * projection, z * projection);
    let strength = if !(dot < 0.0) {
        sqrt(
            (projected_x * projected_x) + (projected_z * projected_z),
            numeric,
        )?
    } else {
        0.0
    };
    Ok((speed, strength))
}

/// Two FSELs: lower, then upper.
fn bound(value: f32, lower: f32, upper: f32) -> f32 {
    let selected = if lower - value >= 0.0 { lower } else { value };
    if upper - selected >= 0.0 { selected } else { upper }
}

fn is_push(direction: u32) -> bool {
    matches!(direction, 1..=6)
}
fn is_pull(direction: u32) -> bool {
    matches!(direction, 7..=12)
}
fn is_left(direction: u32) -> bool {
    matches!(direction, 1 | 3 | 5 | 7 | 9 | 11)
}
fn is_right(direction: u32) -> bool {
    matches!(direction, 2 | 4 | 6 | 8 | 10 | 12)
}
fn is_transition(direction: u32) -> bool {
    matches!(direction, 3..=6 | 9..=12)
}

/// ClampAngle: inside the (possibly wrapping) arc keeps the angle, else the circularly nearer
/// end, a tie choosing the end.
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "raw ordered PPC branch layout"
)]
fn clamp_angle(angle: f32, start: f32, end: f32) -> f32 {
    let inside = if !(start >= end) {
        !(angle < start) && angle <= end
    } else {
        angle >= start || angle <= end
    };
    if inside {
        return angle;
    }
    let to_start = f32::from_bits(angle_wrap(start - angle).to_bits() & 0x7fff_ffff);
    let to_end = f32::from_bits(angle_wrap(angle - end).to_bits() & 0x7fff_ffff);
    if to_start >= to_end { end } else { start }
}

/// The direction's arc and its normalized angle coefficient; still keeps the previous one.
fn directional_angle(direction: u32, full_angle: f32, previous: f32) -> (f32, f32) {
    if is_push(direction) {
        if is_left(direction) {
            let angle = clamp_angle(full_angle, -HALF_PI, 0.0);
            (angle, -angle / HALF_PI)
        } else {
            let angle = clamp_angle(full_angle, 0.0, HALF_PI);
            (angle, angle / HALF_PI)
        }
    } else if is_pull(direction) {
        if is_left(direction) {
            let angle = clamp_angle(full_angle, PI, -HALF_PI);
            (angle, angle_wrap(angle + PI) / HALF_PI)
        } else {
            let angle = clamp_angle(full_angle, HALF_PI, PI);
            (angle, angle_wrap(PI - angle) / HALF_PI)
        }
    } else {
        (full_angle, previous)
    }
}
