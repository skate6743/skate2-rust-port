//! ZIP orientation formulas checked against original S3 family bodies.
//! S2 equivalents82D9B818/918/BAC0/BCF8/BD88 retain tunable angles/blends;
//! S3 uses the literals below and adds darkslide's inverted normal.
use super::{V, length, reciprocal, scale, sub};
use crate::physics::{board_motion_output::inverse_length_squared, native_arithmetic::dot3};
pub type Frame = [V; 4];

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// Exact Processed64..127, not an effective/fakie-adjusted board frame.
    pub board: Frame,
    pub direction: V,
    pub normal: V,
    pub point: V,
    pub yaw_1504: f32,
    pub pitch_1508: f32,
    pub switched: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub frame: Frame,
    pub noise_amount: f32,
}

pub fn target(family: u32, i: Input) -> Result<Target, &'static str> {
    let frame = match family {
        0 if i.pitch_1508.abs() <= 0.12 => {
            let forward = signed(i.direction, dot3(i.direction, i.board[2]) > 0.);
            [cross(i.normal, forward), i.normal, forward, i.board[3]]
        }
        0 | 3 => truck(i),
        1 | 5 => {
            let right = signed(i.direction, dot3(i.direction, i.board[0]) > 0.);
            let up = if family == 5 {
                scale(i.normal, -1.)
            } else {
                i.normal
            };
            [right, up, cross(right, up), i.board[3]]
        }
        2 | 4 => {
            let across = cross(i.direction, i.normal);
            let axis = signed(i.direction, !(dot3(across, sub(i.board[3], i.point)) > 0.));
            // Original822F8618/822F8614,20/42degrees expressed in radians.
            let angle = f32::from_bits(if family == 4 {
                0x3f3b_a866
            } else {
                0x3eb2_b8c3
            });
            let up = rotate(axis, i.normal, angle);
            let right = signed(i.direction, dot3(i.direction, i.board[0]) > 0.);
            [right, up, cross(right, up), i.board[3]]
        }
        _ => return Err("Unknown physical grind family"),
    };
    Ok(Target {
        frame,
        noise_amount: if family == 0 { 0. } else { 0.09 },
    })
}

///82D40290: zero angles skip rotation; near-parallel support returns the entire
///input frame. Neither branch introduces a replacement normal or heading.
fn truck(i: Input) -> Frame {
    let yaw = i.yaw_1504 * -0.68;
    let mut forward = i.board[2];
    if yaw.abs() > 0. {
        forward = rotate(i.normal, forward, yaw);
    }
    let right = cross(i.normal, forward);
    let magnitude = length(right);
    if magnitude <= 0.001 {
        return i.board;
    }
    let right = scale(right, reciprocal(magnitude));
    let mut up = cross(forward, right);
    let pitch = (if i.switched {
        -i.pitch_1508
    } else {
        i.pitch_1508
    }) * 0.68;
    if pitch.abs() > 0. {
        forward = rotate(right, forward, pitch);
        up = cross(forward, right);
    }
    [right, up, forward, i.board[3]]
}

///82D40890: reverse-order classical Gram-Schmidt on BOTH frames, then
///CURRENT's original82BD3150 helper. The temporary orthonormal frames' fourth
///vectors are explicitly zeroed (82D40930/82D409C8), not a position teleport.
pub fn blend(current: Frame, target: Frame) -> Frame {
    crate::animation::foot_ik::interpolate_native(&orthonormal(current), &orthonormal(target), 0.1)
        .0
}

fn orthonormal(input: Frame) -> Frame {
    let mut output = [[0.; 4]; 4];
    for axis in (0..3).rev() {
        let mut value = input[axis];
        for previous in ((axis + 1)..3).rev() {
            value = sub(
                value,
                scale(output[previous], dot3(output[previous], input[axis])),
            );
        }
        output[axis] = scale(value, inverse_length_squared(dot3(value, value), 2));
    }
    output
}

fn rotate(axis: V, value: V, angle: f32) -> V {
    let (sine, cosine) = crate::trigonometry::sin_cos(angle * 0.5);
    let q = scale(axis, sine);
    let first = cross(q, value);
    let inner = core::array::from_fn(|i| cosine.mul_add(value[i], first[i]));
    let second = cross(q, inner);
    core::array::from_fn(|i| 2.0f32.mul_add(second[i], value[i]))
}

fn signed(v: V, positive: bool) -> V {
    if positive { v } else { scale(v, -1.) }
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
