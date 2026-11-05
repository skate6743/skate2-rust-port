//! Reckoning transform, dynamic lean and lateral tilt, TU3
//!82D8D688/82D8D930/82D8C4F8. State scheduling and frame producers are separate.
//! Numerical primitives retain their documented hardware-validation boundary.
use crate::{
    math::Vector3,
    physics::{
        board_motion_output::inverse_length_squared,
        native_arithmetic::dot3,
        skeleton_animation_record::{AnimationPartTransform, IDENTITY, compose_affine},
    },
    point_graph::PointGraph,
    riding::collision_response::signed_angle,
    trigonometry,
};

#[derive(Clone, Debug)]
pub struct ReckoningFrames {
    ///752: ground frame, whose translation is retained by CalculateTransform.
    pub ground: AnimationPartTransform,
    ///816: system frame after application of the body-flip transform.
    pub system: AnimationPartTransform,
    ///880: same frame before body flip, used by CalculateTilt.
    pub unflipped: AnimationPartTransform,
    ///944: rigid inverse of the final system frame.
    pub inverse_system: AnimationPartTransform,
    ///1008: body-flip transform, produced by UpdateBodyFlip.
    pub body_flip: AnimationPartTransform,
    ///1200: heading, reprojected onto the current up plane on each calculation.
    pub heading: [f32; 4],
    ///1576, published to PhysOutSystemReckoning160 with the stance sign.
    pub target_lean_angle: f32,
    ///1248: first lane consumed by the body-tilt animation handler.
    pub lateral_tilt: [f32; 4],
}

impl ReckoningFrames {
    ///82D33178 initializes each matrix independently to identity and seeds
    ///heading to world X. Reset82D8C3A8 later preserves the matrices.
    pub fn new() -> Self {
        Self {
            ground: IDENTITY,
            system: IDENTITY,
            unflipped: IDENTITY,
            inverse_system: IDENTITY,
            body_flip: IDENTITY,
            heading: [1.0, 0.0, 0.0, 0.0],
            target_lean_angle: 0.0,
            lateral_tilt: [0.0; 4],
        }
    }

    ///82D8D688. Cross products use the unnormalized intermediate axis;
    ///normalization has two refinements and no epsilon fallback in this leaf.
    pub fn calculate_transform(&mut self, up: [f32; 4], ground_normal: [f32; 4]) {
        let right = cross(up, self.heading);
        let forward = cross(right, up);
        self.heading = normalize(forward);
        self.system[0] = normalize(right);
        self.system[1] = up;
        self.system[2] = self.heading;
        self.unflipped = self.system;
        self.system = compose_affine(&self.body_flip, &self.system);
        self.inverse_system = inverse(&self.system);

        let ground_right = cross(ground_normal, self.heading);
        let ground_forward = cross(ground_right, ground_normal);
        self.ground[0] = normalize(ground_right);
        self.ground[1] = ground_normal;
        self.ground[2] = normalize(ground_forward);
    }

    ///82D8D930. Signed angle is wrapped by fraction/floor, not scalar atan2.
    pub fn calculate_dynamic_lean(&mut self, up: [f32; 4], dynamic_up: [f32; 4]) {
        let axis = self.system[2];
        let project = |value| {
            let amount = dot3(value, axis);
            std::array::from_fn(|i| value[i] - axis[i] * amount)
        };
        let from = project(up);
        let to = project(dynamic_up);
        let threshold = f32::from_bits(0x3727_c5ac); //8219B100
        self.target_lean_angle = if dot3(from, from) > threshold && dot3(to, to) > threshold {
            wrap_fraction(signed_angle(xyz(from), xyz(to), xyz(axis)))
        } else {
            0.0
        };
    }

    ///82D8C4F8. Degenerate projection preserves the preceding lateral tilt.
    ///The stock source uses the unflipped up/forward axes and two authored
    ///curves, including its absolute wrapped angle and stance sign change.
    pub fn calculate_tilt(
        &mut self,
        reverse_stance: bool,
        tilt_vs_angle: &PointGraph<8>,
        tilt_vs_up: &PointGraph<8>,
    ) {
        let axis = self.unflipped[1];
        let world_up = [0.0, 1.0, 0.0, 0.0];
        let dot = dot3(axis, world_up);
        let parallel = axis.map(|v| v * dot);
        let projected = std::array::from_fn(|i| world_up[i] - parallel[i]);
        let squared = dot3(projected, projected);
        if squared <= f32::from_bits(0x3a03_126f) {
            return;
        }
        let angle = if world_up == parallel {
            0.0
        } else {
            signed_angle(xyz(self.unflipped[2]), xyz(normalize(projected)), xyz(axis))
        };
        let half_pi = f32::from_bits(0x3fc9_0fdb);
        let scale = f32::from_bits(0x3f22_f983);
        let mut normalized_angle =
            ((wrap_fraction(angle - half_pi).abs() - half_pi) * -1.0) * scale;
        if reverse_stance {
            normalized_angle = -normalized_angle;
        }
        //Inline native asin polynomial82D8C704..7E0 is the same operation
        //tree retained by trigonometry::asin, including one rsqrt refinement.
        let normalized_up_angle = (half_pi - trigonometry::asin(axis[1])) * scale;
        let angle_tilt = tilt_vs_angle.evaluate(normalized_angle.abs());
        let up_tilt = tilt_vs_up.evaluate(normalized_up_angle.abs());
        let signed = if normalized_angle >= -0.0 {
            angle_tilt
        } else {
            -angle_tilt
        };
        self.lateral_tilt = [up_tilt * signed, 0.0, 0.0, 0.0];
    }
}

fn normalize(value: [f32; 4]) -> [f32; 4] {
    let reciprocal = inverse_length_squared(dot3(value, value), 2);
    value.map(|v| v * reciprocal)
}
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inverse(frame: &AnimationPartTransform) -> AnimationPartTransform {
    let axes: [[f32; 4]; 3] = std::array::from_fn(|i| [frame[0][i], frame[1][i], frame[2][i], 0.0]);
    let p = frame[3].map(|v| 0.0 - v);
    let position = std::array::from_fn(|i| {
        let z = axes[2][i] * p[2];
        let y = axes[1][i].mul_add(p[1], z);
        axes[0][i].mul_add(p[0], y)
    });
    [axes[0], axes[1], axes[2], position]
}
fn wrap_fraction(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    let fraction = fraction - if fraction > 0.5 { 1.0 } else { 0.0 };
    fraction * f32::from_bits(0x40c9_0fdb)
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn body_flip_changes_system_but_retains_ground_and_preflip_frames() {
        let mut state = ReckoningFrames::new();
        state.heading = [0.0, 0.0, 1.0, 0.0];
        state.body_flip = [
            [0., 0., -1., 0.],
            [0., 1., 0., 0.],
            [1., 0., 0., 0.],
            [3., 0., 0., 0.],
        ];
        state.calculate_transform([0., 1., 0., 0.], [0., 1., 0., 0.]);
        assert_eq!(state.unflipped, IDENTITY);
        assert_eq!(state.ground, IDENTITY);
        assert_eq!(state.system, state.body_flip);
        assert_eq!(
            compose_affine(&state.system, &state.inverse_system),
            IDENTITY
        );
        state.calculate_dynamic_lean([0., 1., 0., 0.], [0., 1., 0., 0.]);
        assert!(state.target_lean_angle.abs() < 0.001);
        state.lateral_tilt = [0.3, 0., 0., 0.];
        let curve = PointGraph {
            x: [0.; 8],
            y: [0.; 8],
        };
        state.calculate_tilt(false, &curve, &curve);
        assert_eq!(state.lateral_tilt, [0.3, 0., 0., 0.]);
    }
}
