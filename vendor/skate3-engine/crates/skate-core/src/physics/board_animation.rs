//! Original TU3 board error capture82C04368 and animation blend82C01878.
//! PC arithmetic and the recovered refinements are used; this is not a claim
//! of instruction-identical Xenon estimates or exceptional-lane execution.
use super::{
    constraint_frames::{basis, reciprocal, reciprocal_sqrt},
    drive_frames::retail_quaternion_from_basis,
    native_arithmetic::dot4,
    rigid_body::RetailQuaternion,
    skeleton_animation_record::{AnimationPartTransform as Transform, IDENTITY, compose_affine},
    skeleton_root::{inverse_rigid, orthonormalize},
};
use crate::{math::Basis3, point_graph::PointGraph, trigonometry};

pub struct BoardAnimationSettings {
    pub slow: PointGraph<8>,
    pub fast: PointGraph<8>,
}

///Board240 and288. This state must be shared between physical Ground error
///capture and subsequent animated-board updates; it is not a per-state cache.
#[derive(Clone, Debug)]
pub struct BoardAnimation {
    pub rotation_error: [f32; 4],
    pub blending: bool,
}
impl Default for BoardAnimation {
    fn default() -> Self {
        //Original constructor82C01100/1104.
        Self {
            rotation_error: [0.0, 0.0, 0.0, 1.0],
            blending: false,
        }
    }
}
impl BoardAnimation {
    ///82C0606C resets the quaternion. The source does not clear byte288.
    pub fn reset(&mut self) {
        self.rotation_error = [0.0, 0.0, 0.0, 1.0];
    }

    ///82C04368: target inverse * actual DECK PART frame, reverse-axis
    ///orthonormalization, quaternion conversion, and unconditional activation.
    ///The scalar candidate conversion preserves the recovered comparisons and
    ///refinements; exact VMX gather-register encoding remains separately audited.
    pub fn capture_physics_error(&mut self, target: &Transform, deck_part: &Transform) {
        let relative = orthonormalize(compose_affine(&inverse_rigid(target), deck_part));
        let q = retail_quaternion_from_basis(Basis3 {
            columns: std::array::from_fn(|i| relative[i][..3].try_into().unwrap()),
        });
        self.rotation_error = [q.x, q.y, q.z, q.w];
        self.blending = true;
    }

    ///82C041C0 returns the target unchanged while inactive. Otherwise the
    ///slow/fast graph is evaluated once per invocation, without a dt factor.
    pub fn apply(
        &mut self,
        target: &Transform,
        fast: bool,
        settings: &BoardAnimationSettings,
    ) -> Transform {
        if !self.blending {
            return *target;
        }
        let angle = trigonometry::acos(self.rotation_error[3]) * 2.0;
        let turns = angle * f32::from_bits(0x3e22_f983);
        let fraction = turns - turns.floor();
        let folded = fraction - if fraction > 0.5 { 1.0 } else { 0.0 };
        let distance = (folded * f32::from_bits(0x40c9_0fdb)).abs();
        let fraction = if fast { settings.fast } else { settings.slow }.evaluate(distance);
        //Original fcmpu/blt: unordered graph output also completes the blend.
        if !(fraction < f32::from_bits(0x3f73_3333)) {
            self.blending = false;
            return *target;
        }
        let identity = [0.0, 0.0, 0.0, 1.0];
        let dot = dot4(self.rotation_error, identity);
        let reverse = 0.0 > dot;
        let magnitude = if reverse { -dot } else { dot };
        let q = self.rotation_error.map(|v| if reverse { -v } else { v });
        let q = if magnitude > f32::from_bits(0x3f7f_069e) {
            let same_sign = dot4(q, identity) > 0.0;
            let mixed: [f32; 4] = std::array::from_fn(|lane| {
                if same_sign {
                    fraction.mul_add(identity[lane] - q[lane], q[lane])
                } else {
                    (-(identity[lane] + q[lane])).mul_add(fraction, q[lane])
                }
            });
            let inverse = reciprocal_sqrt(dot4(mixed, mixed));
            mixed.map(|v| v * inverse)
        } else {
            let theta = trigonometry::acos(magnitude);
            let a = trigonometry::sin((1.0 - fraction) * theta);
            let b = trigonometry::sin(fraction * theta);
            let inverse = reciprocal(trigonometry::sin(theta));
            let weight_a = a * inverse;
            let weight_b = b * inverse;
            std::array::from_fn(|lane| identity[lane].mul_add(weight_b, q[lane] * weight_a))
        };
        self.rotation_error = q;
        //82C01B04..BE0 uses the same sqrt(2) / paired residual basis kernel
        //independently present in82AE1CC4. No quaternion normalization here.
        let rotation = basis(RetailQuaternion {
            x: q[0],
            y: q[1],
            z: q[2],
            w: q[3],
        });
        let mut correction = IDENTITY;
        for axis in 0..3 {
            correction[axis][..3].copy_from_slice(&rotation.columns[axis]);
        }
        compose_affine(target, &correction)
    }
}

///82C04270: target is compared with actual deck BODY COM, then all seven
///board bodies receive this XYZ velocity, leaving their angular rates alone.
pub fn target_velocity(target: [f32; 4], deck_body_position: [f32; 4], dt: f32) -> [f32; 4] {
    let inverse = 1.0 / dt;
    std::array::from_fn(|lane| (target[lane] - deck_body_position[lane]) * inverse)
}
