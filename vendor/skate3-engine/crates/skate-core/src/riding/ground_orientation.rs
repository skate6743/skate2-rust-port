//! Riding up-vector and ground-normal calculation in TU3 Reckoning82D8C8F0.
//! Inputs are assembled by82D8E5C0; this module owns the three persistent
//! filters, target, acceleration and damping. Transform/tilt publication is a
//! separate downstream calculation. Numerical primitives remain unverified
//! against independent hardware captures.
use crate::physics::{
    board_ground::angle_between,
    board_motion_output::{add, dot, inverse_length_squared, length, scale, subtract},
    native_arithmetic,
};
use crate::{air::ground_normal::GroundNormalFilter, math::Vector3, point_graph::PointGraph};

const UP: Vector3 = Vector3::new(0.0, 1.0, 0.0);
const EPSILON: f32 = f32::from_bits(0x3586_37BD); //82F826F8 installs82181A88 at830BD350.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundOrientationSettings {
    pub ground_normal_smoothing: [f32; 4],
    pub up_vector_smoothing_slow: [f32; 4],
    pub up_vector_smoothing_fast: [f32; 4],
    pub dynamic_up_vs_ground_y: PointGraph<8>,
    pub ground_vector_blend: PointGraph<8>,
    /// The eight x/y pairs inside PointNegGraphData8 at layout256+16.
    ///82D8CF10 calls ordinary PointGraphEval directly, ignoring its sign mode.
    pub deck_angle_usage_vs_speed: PointGraph<8>,
    pub up_vector_smoothing_vs_speed: PointGraph<8>,
    pub up_vector_max_delta_vs_speed: PointGraph<8>,
    pub ground_blend_max_delta: f32,
    pub up_vector_max_acceleration: f32,
    pub anti_wobble_damping: f32,
    pub extra_side_damping: f32,
    pub minimum_wheels_for_ground_blend: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundOrientationInput {
    /// Processed752, copied from Skeleton11008 in82BD8918.
    pub com_to_deck: Vector3,
    /// Processed464 <- Ground96 (retained wheel normal).
    pub ground_normal: Vector3,
    /// Processed528 <- Ground64 (overall normal).
    pub dynamic_up: Vector3,
    /// Processed2652 <- SkateboardMotion160 (raw deck speed).
    pub speed: f32,
    pub wheel_contact_count: i32,
    /// Processed2720; the native deck-angle gate tests exact zero.
    pub animation_balance: f32,
    /// Processed2616 input to DeckAngleUsageVsSpeed.
    pub deck_angle_curve_input: f32,
    /// Processed80/96: current board transform Y/Z.
    pub board_up: Vector3,
    pub board_forward: Vector3,
    /// Processed160: effective board transform Z.
    pub effective_board_forward: Vector3,
    /// The previous complete Reckoning transform X (native816), including flip.
    pub previous_reckoning_right: Vector3,
    /// Processed2476 bit30, the coffin restriction.
    pub prevent_up_behind_board: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroundOrientation {
    pub dynamic_up: Vector3,
    pub up: Vector3,
    pub target: Vector3,
    pub up_velocity: Vector3,
    pub ground_normal: Vector3,
    pub ground_blend: f32,
    pub(crate) ground_filter: GroundNormalFilter,
    pub(crate) slow_filter: GroundNormalFilter,
    pub(crate) fast_filter: GroundNormalFilter,
}
impl GroundOrientation {
    /// Native82D8DA68 filter history and82D8C3A8/82D8BE38 vector seeds.
    pub fn new(settings: &GroundOrientationSettings) -> Self {
        Self {
            dynamic_up: UP,
            up: UP,
            target: UP,
            up_velocity: Vector3::ZERO,
            ground_normal: UP,
            ground_blend: 0.0,
            ground_filter: GroundNormalFilter::initialized(
                settings.ground_normal_smoothing,
                lanes(UP),
            ),
            slow_filter: GroundNormalFilter::initialized(
                settings.up_vector_smoothing_slow,
                lanes(UP),
            ),
            fast_filter: GroundNormalFilter::initialized(
                settings.up_vector_smoothing_fast,
                lanes(UP),
            ),
        }
    }

    /// Reset82D8C3A8 resets these vectors/scalars but preserves filter history.
    pub fn reset(&mut self) {
        self.dynamic_up = UP;
        self.up = UP;
        self.target = UP;
        self.up_velocity = Vector3::ZERO;
        self.ground_normal = UP;
        self.ground_blend = 0.0;
    }
    /// Complete up/normal branch82D8C934..82D8D650, before CalculateTransform,
    /// CalculateDynamicLean and CalculateTilt. BodySpin runs before this stage.
    pub fn update(&mut self, settings: &GroundOrientationSettings, input: GroundOrientationInput) {
        let normalized_com = normalize_safe(input.com_to_deck, Vector3::ZERO);
        let amount = settings
            .dynamic_up_vs_ground_y
            .evaluate(input.ground_normal.y);
        let prediction = normalize_safe(
            blend(
                self.ground_normal,
                add(UP, subtract(input.dynamic_up, normalized_com)),
                amount,
            ),
            Vector3::ZERO,
        );
        self.dynamic_up = input.dynamic_up;
        self.ground_normal = vector(
            self.ground_filter
                .update(settings.ground_normal_smoothing, lanes(input.ground_normal)),
        );
        let horizontal_axis = cross(UP, self.ground_normal);
        if dot(horizontal_axis, horizontal_axis) > f32::from_bits(0x3A83_126F) {
            let axis = scale(
                horizontal_axis,
                inverse_length_squared(dot(horizontal_axis, horizontal_axis), 2),
            );
            let projected = subtract(prediction, scale(axis, dot(axis, prediction)));
            let prediction_to_ground = angle_between(projected, self.ground_normal);
            let prediction_to_up = angle_between(projected, UP);
            let ground_to_up = angle_between(self.ground_normal, UP);
            let old_up_to_up = angle_between(self.up, UP);
            let mut candidate = if prediction_to_ground < prediction_to_up {
                self.ground_normal
            } else {
                UP
            };
            if prediction_to_ground < ground_to_up && prediction_to_up < ground_to_up {
                candidate = projected;
            }
            let alpha = (input.speed + 1.0) * f32::from_bits(0x3A83_126F);
            if !(old_up_to_up > ground_to_up || prediction_to_up > old_up_to_up) {
                candidate = madd(self.up, 1.0 - alpha, scale(candidate, alpha));
            }
            let slope = unit_saturate(ground_to_up * f32::from_bits(0x3EA2_F983));
            let mut target_blend = settings.ground_vector_blend.evaluate(slope);
            if input.wheel_contact_count <= settings.minimum_wheels_for_ground_blend {
                target_blend = 0.0;
            }
            let delta = target_blend - self.ground_blend;
            let lower = fsel(
                -settings.ground_blend_max_delta - delta,
                -settings.ground_blend_max_delta,
                delta,
            );
            self.ground_blend += fsel(
                settings.ground_blend_max_delta - lower,
                lower,
                settings.ground_blend_max_delta,
            );
            self.target = normalize_safe(
                blend(candidate, self.ground_normal, self.ground_blend),
                Vector3::ZERO,
            );
        } else {
            self.target = self.ground_normal;
        }
        if input.wheel_contact_count >= 2 && input.animation_balance == 0.0 {
            let usage = settings
                .deck_angle_usage_vs_speed
                .evaluate(input.deck_angle_curve_input);
            let axis = normalize_safe(
                cross(input.board_forward, self.ground_normal),
                Vector3::ZERO,
            );
            let deck_up = normalize_safe(
                subtract(input.board_up, scale(axis, dot(input.board_up, axis))),
                Vector3::ZERO,
            );
            self.target = normalize_safe(blend(self.target, deck_up, usage), Vector3::ZERO);
        }
        let slow = normalize_safe(
            vector(
                self.slow_filter
                    .filter_raw(settings.up_vector_smoothing_slow, lanes(self.target)),
            ),
            Vector3::ZERO,
        );
        let fast = normalize_safe(
            vector(
                self.fast_filter
                    .filter_raw(settings.up_vector_smoothing_fast, lanes(self.target)),
            ),
            Vector3::ZERO,
        );
        let speed_fraction = input.speed * f32::from_bits(0x3DCC_CCCD);
        let mix = settings
            .up_vector_smoothing_vs_speed
            .evaluate(speed_fraction);
        let filtered = normalize_safe(blend(slow, fast, mix), Vector3::ZERO);
        let max_delta = settings
            .up_vector_max_delta_vs_speed
            .evaluate(speed_fraction)
            * f32::from_bits(0x3E4C_CCCD);
        let difference = subtract(filtered, self.up);
        let magnitude = length(difference);
        let bounded = fsel(magnitude - max_delta, max_delta, magnitude);
        let stepped = normalize_safe(
            madd(normalize_safe(difference, Vector3::ZERO), bounded, self.up),
            Vector3::ZERO,
        );
        let desired_velocity = subtract(stepped, self.up);
        let acceleration = clamp_length(
            subtract(desired_velocity, self.up_velocity),
            settings.up_vector_max_acceleration * f32::from_bits(0x3C88_8889),
        );
        self.up_velocity = add(self.up_velocity, acceleration);
        let side = scale(
            input.previous_reckoning_right,
            dot(self.up_velocity, input.previous_reckoning_right),
        );
        self.up_velocity = madd(
            side,
            settings.extra_side_damping,
            subtract(self.up_velocity, side),
        );
        let unnormalized = add(self.up, self.up_velocity);
        self.up = normalize_safe(unnormalized, unnormalized);
        if 0.0 > dot(self.up_velocity, desired_velocity) {
            self.up_velocity = scale(self.up_velocity, settings.anti_wobble_damping);
        }
        if input.prevent_up_behind_board && dot(self.up, input.effective_board_forward) < 0.0 {
            let side = cross(self.up, input.effective_board_forward);
            self.up = normalize_safe(cross(input.effective_board_forward, side), self.up);
            self.up_velocity = scale(self.up_velocity, 0.5);
        }
        self.slow_filter.publish_current(lanes(self.up));
        self.fast_filter.publish_current(lanes(self.up));
    }
}

fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn vector(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= -0.0 { positive } else { negative }
}
fn unit_saturate(value: f32) -> f32 {
    let value = fsel(-value, 0.0, value);
    fsel(1.0 - value, value, 1.0)
}
fn madd(v: Vector3, factor: f32, offset: Vector3) -> Vector3 {
    Vector3::new(
        v.x.mul_add(factor, offset.x),
        v.y.mul_add(factor, offset.y),
        v.z.mul_add(factor, offset.z),
    )
}
fn blend(from: Vector3, to: Vector3, amount: f32) -> Vector3 {
    madd(to, amount, scale(from, 1.0 - amount))
}
fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}
fn normalize_safe(value: Vector3, fallback: Vector3) -> Vector3 {
    let squared = dot(value, value);
    let inverse = inverse_length_squared(squared, 2);
    let magnitude = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    if magnitude > EPSILON {
        scale(value, inverse)
    } else {
        fallback
    }
}
/// Complete ClampVectorWithinMaxLength82BD3D90.
pub(crate) fn clamp_length(value: Vector3, maximum: f32) -> Vector3 {
    let magnitude = length(value);
    if magnitude < f32::from_bits(0x3780_0000) {
        return value;
    }
    let bounded = fsel(maximum - magnitude, magnitude, maximum);
    let mut inverse = native_arithmetic::reciprocal_estimate(magnitude);
    for _ in 0..2 {
        let error = (-inverse).mul_add(magnitude, 1.0);
        inverse = inverse.mul_add(error, inverse);
    }
    scale(scale(value, bounded), inverse)
}
#[cfg(test)]
#[path = "tests/ground_orientation.rs"]
mod tests;
