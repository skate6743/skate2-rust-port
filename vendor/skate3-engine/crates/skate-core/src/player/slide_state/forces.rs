//! Full non-launch arithmetic82D3AB14..AE4C and point force82D925E0.
//! Uses independent PC arithmetic with the original refinement/operation order.
use crate::{
    math::Vector3,
    physics::{board_motion_output::inverse_length_squared, force_queue::QueuedPointForce},
    point_graph::PointGraph,
    riding::collision_response::signed_angle,
};
type V = [f32; 4];
#[derive(Clone, Debug)]
pub struct SlideSettings {
    pub input_remap: PointGraph<8>,
    pub remap_vs_speed: PointGraph<8>,
    pub force_vs_angle: PointGraph<8>,
    pub force_vs_speed: PointGraph<8>,
    pub softest_wheel_force: f32,
    pub softest_wheel_spin: f32,
    pub angular_force: f32,
    pub force_y_offset: f32,
}
#[derive(Clone, Debug)]
pub struct SlideSurface {
    pub speed_to_force: PointGraph<8>,
    pub yaw_strength: f32,
    pub yaw_damping: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct SlideInput {
    pub velocity: V,
    pub normal: V,
    pub side: V,
    pub effective_forward: V,
    pub reference_forward: V,
    pub angular_velocity: V,
    pub absolute_speed: f32,
    pub surface_speed: f32,
    pub slide: f32,
    pub elapsed: f32,
    pub wheel_hardness: f32,
}
/// The source does not add a zero-velocity normalization fallback here.
/// Retain that branch behavior rather than manufacturing a desired direction.
pub fn angular_correction(s: &SlideSettings, surface: &SlideSurface, p: SlideInput) -> V {
    let remap = s.input_remap.evaluate(p.slide.abs());
    let age = p.elapsed - 0.5;
    let age = fsel(-age, 0.0, age);
    let age = fsel(1.0 - age, age, 1.0);
    let weight = s.remap_vs_speed.evaluate(p.surface_speed) * age;
    let pi = f32::from_bits(0x4049_0fdb);
    let signed = fsel(p.slide, remap, -remap);
    let angle = (p.slide * pi).mul_add(1.0 - weight, (signed * pi) * weight);
    let velocity = normalize(p.velocity);
    let side = normalize(cross(p.normal, velocity));
    let desired = madd(
        velocity,
        crate::trigonometry::cos(angle),
        scale(side, crate::trigonometry::sin(angle)),
    );
    let mut error = cross(p.reference_forward, desired);
    if dot(p.reference_forward, desired) < 0.0 {
        error = if error[1] < 0.0 {
            p.normal.map(|v| -v)
        } else {
            p.normal
        };
    }
    let strength = (1.0 - s.softest_wheel_spin).mul_add(p.wheel_hardness, s.softest_wheel_spin)
        * surface.yaw_strength;
    let displacement = scale(
        madd(
            error,
            strength,
            scale(p.angular_velocity, surface.yaw_damping),
        ),
        s.angular_force,
    );
    scale(p.normal, dot(displacement, p.normal))
}
///82D925E0 rejects normal velocity, measures the authored slip angle and adds
/// its signed sideways response.82E0A4E8 normalizes before its rejection again.
pub fn sliding_force(s: &SlideSettings, surface: &SlideSurface, p: SlideInput) -> QueuedPointForce {
    let tangent = reject(p.velocity, p.normal);
    let lateral = scale(p.side, dot(tangent, p.side));
    let response = surface.speed_to_force.evaluate(p.absolute_speed);
    // Original ble is !GT, so unordered follows the <= branch too.
    let hardness = if response > 0.0 {
        p.wheel_hardness
    } else {
        1.0
    };
    let strength =
        (1.0 - s.softest_wheel_force).mul_add(hardness, s.softest_wheel_force) * response;
    let side = normalize_safe(cross(p.normal, p.velocity));
    let longitudinal = reject(scale(lateral, strength), normalize_safe(side));
    let angle = if dot(p.velocity, p.velocity) * dot(p.effective_forward, p.effective_forward)
        > f32::from_bits(0x3780_0000)
    {
        signed_angle(
            xyz(reject(p.velocity, p.normal)),
            xyz(reject(p.effective_forward, p.normal)),
            xyz(p.normal),
        )
    } else {
        0.0
    };
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    let wrapped = (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c9_0fdb);
    let degrees = wrapped * f32::from_bits(0x4265_2ee1);
    let sign = if degrees > 0.0 { 1.0 } else { -1.0 };
    let angular = scale(
        scale(side, s.force_vs_angle.evaluate(degrees.abs())),
        s.force_vs_speed.evaluate(p.surface_speed),
    );
    let force = madd(angular, sign, longitudinal);
    // bge skips negation for unordered as well as nonnegative.
    let y = if dot(force, p.velocity) < 0.0 {
        -s.force_y_offset
    } else {
        s.force_y_offset
    };
    QueuedPointForce {
        tag: 9,
        force_world: xyz(force),
        point_body: Vector3::new(0.0, y, 0.0),
    }
}
fn dot(a: V, b: V) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}
fn scale(v: V, s: f32) -> V {
    v.map(|v| v * s)
}
fn madd(v: V, s: f32, b: V) -> V {
    std::array::from_fn(|i| v[i].mul_add(s, b[i]))
}
fn reject(v: V, n: V) -> V {
    let d = dot(v, n);
    std::array::from_fn(|i| v[i] - n[i] * d)
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn normalize(v: V) -> V {
    scale(v, inverse_length_squared(dot(v, v), 2))
}
fn normalize_safe(v: V) -> V {
    let d = dot(v, v);
    let inv = inverse_length_squared(d, 2);
    let length = if d == 0.0 { 0.0 } else { d * inv };
    if length > f32::from_bits(0x3586_37bd) {
        scale(v, inv)
    } else {
        [0.0; 4]
    }
}
fn fsel(v: f32, a: f32, b: f32) -> f32 {
    if v >= -0.0 { a } else { b }
}
fn xyz(v: V) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curve(value: f32) -> PointGraph<8> {
        PointGraph {
            x: [0., 1., 2., 3., 4., 5., 6., 7.],
            y: [value; 8],
        }
    }
    fn settings() -> SlideSettings {
        SlideSettings {
            input_remap: curve(0.0),
            remap_vs_speed: curve(0.0),
            force_vs_angle: curve(0.0),
            force_vs_speed: curve(0.0),
            softest_wheel_force: 0.25,
            softest_wheel_spin: 1.0,
            angular_force: 0.1,
            force_y_offset: -0.12,
        }
    }
    fn input() -> SlideInput {
        SlideInput {
            velocity: [4., 0., 3., 0.],
            normal: [0., 1., 0., 0.],
            side: [1., 0., 0., 0.],
            effective_forward: [0., 0., 1., 0.],
            reference_forward: [0., 0., 1., 0.],
            angular_velocity: [0.; 4],
            absolute_speed: 5.,
            surface_speed: 5.,
            slide: 0.,
            elapsed: 0.,
            wheel_hardness: 0.5,
        }
    }
    #[test]
    fn negative_slide_force_bypasses_wheel_softness_and_reverses_application_height() {
        // Analytic projection of a world-X force onto velocity(4,0,3).
        // Source82D926A4 gates hardness only for a positive curve result;
        //82D92924 chooses the point height from signed work, not force X.
        let s = settings();
        let p = input();
        let mut surface = SlideSurface {
            speed_to_force: curve(2.),
            yaw_strength: 0.,
            yaw_damping: 0.,
        };
        let positive = sliding_force(&s, &surface, p);
        assert!((positive.force_world.x - 3.2).abs() < 0.00001);
        assert!((positive.force_world.z - 2.4).abs() < 0.00001);
        assert_eq!(positive.point_body.y, -0.12);
        surface.speed_to_force = curve(-2.);
        let negative = sliding_force(&s, &surface, p);
        assert!((negative.force_world.x + 5.12).abs() < 0.00001);
        assert!((negative.force_world.z + 3.84).abs() < 0.00001);
        assert_eq!(negative.point_body.y, 0.12);
    }
    #[test]
    fn slide_yaw_past_ninety_degrees_uses_signed_ground_axis() {
        // Opposing desired headings replace the cross error with +/-normal
        // at82D3ACCC..AD38; damping still uses actual angular velocity.
        let s = settings();
        let mut p = input();
        p.velocity = [0., 0., 4., 0.];
        p.angular_velocity = [9., -2., 5., 0.];
        p.slide = 0.75;
        let surface = SlideSurface {
            speed_to_force: curve(0.),
            yaw_strength: 2.,
            yaw_damping: 0.3,
        };
        let positive = angular_correction(&s, &surface, p);
        assert!((positive[1] - 0.14).abs() < 0.00001);
        assert_eq!(positive[0], 0.);
        assert_eq!(positive[2], 0.);
        p.slide = -0.75;
        let negative = angular_correction(&s, &surface, p);
        assert!((negative[1] + 0.26).abs() < 0.00001);
    }
}
