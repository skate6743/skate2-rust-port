//! Authored normal-camera shots and TU3 interpolation82E06980/82E074A8.
//! The state graph selects stock shot names; these values do not select a view.
use super::shot_orientation::interpolate_orientation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub distance: f32,
    pub lens_length: f32,
    /// Direction, elevation, yaw, pitch; native shot68..80.
    pub smoothing: [f32; 4],
    /// Native shot84..120 follows Subject's ten reference-point entries.
    pub reference_weights: [f32; 10],
    pub board_offset: f32,
    pub position_heading: f32,
    pub position_elevation: f32,
    /// Roll, yaw, pitch; native shot136..144, in radians after stock loading.
    pub framing: [f32; 3],
    pub follow_subject_in_air: u8,
    pub mirror_for_stance: u8,
    pub snap_to_reference_point: u8,
    pub use_previous_shot: u8,
    pub use_drop_predictor: u8,
    pub use_free_camera_stick: u8,
    pub avoidance_override: u8,
    pub blur: f32,
    pub transition_blur: f32,
    pub subject_opacity: f32,
    pub collision_hint: u32,
    pub anchor: u32,
    pub compass_north: u32,
    pub world_heading: f32,
    pub arm_orientation: [f32; 4],
    pub camera_orientation: [f32; 4],
}

impl Shot {
    /// Complete Shot constructor82E06230; these initialize transition history
    /// before the first authored shot is loaded, and are not camera tuning.
    pub fn new() -> Self {
        Self {
            distance: 2.0,
            lens_length: 12.0,
            smoothing: [f32::from_bits(0x3f4ccccd); 4],
            reference_weights: [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            board_offset: 0.0,
            position_heading: f32::from_bits(0x40490fdb),
            position_elevation: 0.0,
            framing: [0.0; 3],
            follow_subject_in_air: 1,
            mirror_for_stance: 1,
            snap_to_reference_point: 0,
            use_previous_shot: 0,
            use_drop_predictor: 0,
            use_free_camera_stick: 0,
            avoidance_override: 0,
            blur: 1.0,
            transition_blur: 1.0,
            subject_opacity: 1.0,
            collision_hint: 0,
            anchor: 0,
            compass_north: 0,
            world_heading: 0.0,
            arm_orientation: [0.0, 0.0, 0.0, 1.0],
            camera_orientation: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Normal-shot evaluation82E06CB8/82E00D70. `north` is the subject's
    /// selected compass entry (the manager applies the special 6->5 case).
    /// UsePreviousShot preserves the already copied arm orientation.
    pub fn update_normal(&mut self, north: f32, heading_mirror: f32) {
        if self.use_previous_shot != 0 {
            return;
        }
        let from = self.position_heading;
        let delta = super::rig_tracking::wrap_vmx(-self.position_heading - from);
        let heading =
            super::rig_tracking::wrap_vmx(delta.mul_add((1.0 - heading_mirror) * 0.5, from));
        let heading = super::rig_tracking::wrap_vmx(heading + north);
        self.arm_orientation =
            super::orientation_math::quaternion_from_angles(self.position_elevation, heading, 0.0);
    }

    /// Complete weighted reference-point calculation82E01070. The board
    /// offset follows subject getter456 and is applied to every weighted
    /// reference point before summation, including points with zero weight.
    pub fn reference_point(
        &self,
        positions: &[[f32; 4]; 10],
        board_offset_direction: [f32; 4],
        fallback: [f32; 4],
    ) -> [f32; 4] {
        let offset = board_offset_direction.map(|v| v * self.board_offset);
        let mut sum = [0.0_f32; 4];
        let mut weight_sum = 0.0_f32;
        for (position, weight) in positions.iter().zip(self.reference_weights) {
            weight_sum = weight + weight_sum;
            sum = core::array::from_fn(|i| (position[i] + offset[i]).mul_add(weight, sum[i]));
        }
        if f32::from_bits(0x38d1b717) > weight_sum.abs() {
            fallback
        } else {
            let inverse = super::vector_tracker::refined_reciprocal(weight_sum);
            sum.map(|v| inverse * v)
        }
    }

    /// Complete field publication82E06980, also used by82E06E38 after it
    /// applies its first sine-shaped blend. Discrete options come from `to`
    /// even at fraction zero. Heading and shot metadata are not written.
    pub fn interpolate_from(&mut self, from: Self, to: Self, fraction: f32) {
        let blend = |a, b| interpolate_float(a, b, 0, fraction);
        self.distance = blend(from.distance, to.distance);
        self.lens_length = blend(from.lens_length, to.lens_length);
        self.smoothing = core::array::from_fn(|i| blend(from.smoothing[i], to.smoothing[i]));
        self.reference_weights =
            core::array::from_fn(|i| blend(from.reference_weights[i], to.reference_weights[i]));
        self.board_offset = blend(from.board_offset, to.board_offset);
        self.position_elevation = blend(from.position_elevation, to.position_elevation);
        self.framing = core::array::from_fn(|i| blend(from.framing[i], to.framing[i]));
        self.follow_subject_in_air = to.follow_subject_in_air;
        self.mirror_for_stance = to.mirror_for_stance;
        self.snap_to_reference_point = to.snap_to_reference_point;
        self.use_previous_shot = to.use_previous_shot;
        self.use_drop_predictor = to.use_drop_predictor;
        self.use_free_camera_stick = to.use_free_camera_stick;
        self.avoidance_override = to.avoidance_override;
        self.blur = blend(from.blur, to.blur);
        self.transition_blur = to.transition_blur;
        self.subject_opacity = blend(from.subject_opacity, to.subject_opacity);
        self.collision_hint = to.collision_hint;
        self.anchor = to.anchor;
        self.compass_north = to.compass_north;
        self.world_heading = blend(from.world_heading, to.world_heading);
        self.arm_orientation =
            interpolate_orientation(from.arm_orientation, to.arm_orientation, fraction);
        self.camera_orientation =
            interpolate_orientation(from.camera_orientation, to.camera_orientation, fraction);
    }
}

/// TU3 82E074A8. Style0 eases with native sine; style1 takes the target;
/// other styles preserve the source. The caller owns fraction clamping.
pub fn interpolate_float(from: f32, to: f32, style: u32, fraction: f32) -> f32 {
    match style {
        0 => (to - from).mul_add(sine_blend(fraction), from),
        1 => to,
        _ => from,
    }
}

pub(super) fn sine_blend(fraction: f32) -> f32 {
    let angle = fraction.mul_add(f32::from_bits(0x40490fdb), -f32::from_bits(0x3fc90fdb));
    crate::trigonometry::sin(angle).mul_add(0.5, 0.5)
}

/// Complete blend-value filtering82E07378 after82E07708 produces the raw
/// subject-dependent value. This filter intentionally does not multiply by dt.
pub fn filter_blend_value(
    previous: f32,
    raw: f32,
    smoothing: f32,
    minimum: f32,
    maximum: f32,
) -> f32 {
    let response = 1.0 - smoothing;
    let next = (response * response).mul_add(raw - previous, previous);
    let lower = if minimum - next >= 0.0 { minimum } else { next };
    if maximum - lower >= 0.0 {
        lower
    } else {
        maximum
    }
}

/// Exact82E07240 interval choice. Entries stop at the first absent child;
/// the returned indices may match at the ends. Zero entries publishes nothing.
pub fn blend_interval(
    points: [f32; 3],
    present: [bool; 3],
    value: f32,
) -> Option<(usize, usize, f32)> {
    let count = present.iter().take_while(|present| **present).count();
    if count == 0 {
        return None;
    }
    for i in 0..count {
        if points[i] > value {
            return Some(if i == 0 {
                (0, 0, 1.0)
            } else {
                (
                    i - 1,
                    i,
                    (value - points[i - 1]) / (points[i] - points[i - 1]),
                )
            });
        }
    }
    Some((count.saturating_sub(2), count - 1, 1.0))
}
