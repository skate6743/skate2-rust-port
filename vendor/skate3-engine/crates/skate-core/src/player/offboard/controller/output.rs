//! Original result exporter82D80F48 and frame helper82D2CDC0.
use super::state::{IDENTITY, ZERO};
use super::{Frame, State, Vector};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundResult {
    pub physical_frame: Frame,  //result0 <- Biped64
    pub animation_frame: Frame, //64 <-128
    pub surface_frame: Frame,   //128 <-82D2CDC0(axis416,forward32)
    pub velocity: Vector,       //192 <-512
    pub position: Vector,       //208 <-368
    pub angular_velocity: f32,  //224 <-688
    pub alternate: bool,        //228 <-709
    pub sliding: bool,          //229 <-710
}
pub(super) fn export(s: &State) -> GroundResult {
    GroundResult {
        physical_frame: s.motion.published_frame_64,
        animation_frame: s.frame_output.frame,
        surface_frame: build_frame(s.surface.spring_normal, s.motion.frame_0[2]),
        velocity: s.frame_output.velocity,
        position: s.position_368,
        angular_velocity: s.motion.angular_velocity_688,
        alternate: s.alternate_709,
        sliding: s.sliding.active_710,
    }
}
fn dot(a: Vector, b: Vector) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn invsqrt(q: f32) -> f32 {
    let mut r = 1.0 / q.sqrt();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.0), r);
    }
    r
}
fn normalize(v: Vector) -> Vector {
    let r = invsqrt(dot(v, v));
    v.map(|x| x * r)
}
pub(crate) fn build_frame(up: Vector, forward: Vector) -> Frame {
    let right = cross(up, forward);
    let q = dot(right, right);
    let raw_length = q * invsqrt(q);
    let length = if q == 0.0 { 0.0 } else { raw_length };
    //830BD300 initialized82F82690 from821647E0, bits37800000.
    if !(length > f32::from_bits(0x3780_0000)) {
        return IDENTITY;
    }
    let mut inverse = 1.0 / length;
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(length, 1.0), inverse);
    }
    let right = right.map(|x| x * inverse);
    let up = normalize(up);
    let forward = normalize(cross(right, up));
    [right, up, forward, ZERO]
}
