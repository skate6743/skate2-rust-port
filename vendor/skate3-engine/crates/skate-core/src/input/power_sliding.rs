//! Complete PowerSliding update82BB28E8 at resolved graph/component boundaries.
//! Registration82F89300; factory82BC9060; vtable82320328.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub elapsed: f32,
    pub previous_right: f32,
    pub previous_left: f32,
    pub flags: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Global slot332 layout1684 and1632.
    pub minimum_speed: f32,
    pub minimum_slide_time: f32,
    /// Graph4 layouts x1408/y1424, x1360/y1376, x1456/y1472.
    pub stop_time: PointGraph<4>,
    pub speed_response: PointGraph<4>,
    pub angle_response: PointGraph<4>,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// PhysOut64->0 and PhysOut4->164.
    pub category: u32,
    pub speed: f32,
    pub right_slide: Option<f32>,
    pub left_slide: Option<f32>,
    /// Results of82454FD8 for keys830BE528 and830C0818.
    pub right_query: bool,
    pub left_query: bool,
    /// MotionGraph interface virtual224,82595930.
    pub graph_scalar: f32,
}
fn bit(word: &mut u32, mask: u32, value: bool) {
    *word = (*word & !mask) | if value { mask } else { 0 };
}

/// 82595930 arithmetic.
/// Inputs are PhysOut4->vector80,
/// PhysOut32->basis row32 and PhysOut4->byte273. None means absent Physical.
pub fn alignment(input: Option<([f32; 4], [f32; 4], u8)>) -> f32 {
    let Some((velocity, basis, flipped)) = input else {
        return 1.0;
    };
    fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
        crate::physics::native_arithmetic::dot3(a, b)
    }
    let squared = dot(velocity, velocity);
    if !(squared > f32::from_bits(0x3a83_126f)) {
        return 1.0;
    }
    let length = super::controller::magnitude(squared);
    let mut reciprocal = super::angle::reciprocal_estimate(length);
    for _ in 0..2 {
        reciprocal = reciprocal.mul_add((-reciprocal).mul_add(length, 1.0), reciprocal);
    }
    let velocity = velocity.map(|v| reciprocal * v);
    let basis = if flipped != 0 {
        basis.map(|v| -v)
    } else {
        basis
    };
    dot(velocity, basis)
}

/// Clock calls preserve the conditional first query and two subsequent timer
/// queries. The native context obtains each through its clock virtual40.
pub fn update(
    state: &mut State,
    latch: &mut [u32; 5],
    input: Input,
    settings: &Settings,
    mut clock: impl FnMut() -> f32,
) {
    let right = input.right_slide.unwrap_or(0.0);
    let left = input.left_slide.unwrap_or(0.0);
    let active = input.category == 1;
    if active && state.flags & 0x2000_0000 == 0 {
        state.elapsed = 0.0;
        state.previous_right = right;
        state.previous_left = left;
        state.flags &= 0x3fff_ffff;
    }
    bit(&mut state.flags, 0x2000_0000, active);
    if !active {
        return;
    }
    let fast = input.speed > settings.minimum_speed;
    let start_right =
        fast && input.right_query && state.flags & 0x8000_0000 == 0 && state.previous_right < right;
    bit(&mut latch[3], 0x8000_0000, start_right);
    state.previous_right = right;
    bit(&mut state.flags, 0x8000_0000, input.right_query);
    let start_left =
        fast && input.left_query && state.flags & 0x4000_0000 == 0 && state.previous_left > left;
    bit(&mut latch[1], 0x8000_0000, start_left);
    state.previous_left = left;
    bit(&mut state.flags, 0x4000_0000, input.left_query);
    let stop_time = settings.stop_time.evaluate(input.graph_scalar);
    let response = settings.speed_response.evaluate(input.speed) * (1.0 - input.graph_scalar.abs());
    let threshold = settings.angle_response.evaluate(input.graph_scalar);
    state.elapsed = if response < threshold {
        state.elapsed + clock()
    } else {
        0.0
    };
    let stop = state.elapsed > stop_time;
    let right_time = clock() + f32::from_bits(latch[2]);
    latch[2] = right_time.to_bits();
    let left_time = clock() + f32::from_bits(latch[0]);
    latch[0] = left_time.to_bits();
    let exit_right =
        right_time > settings.minimum_slide_time && (!input.right_slide.is_some() || !fast || stop);
    let exit_left =
        left_time > settings.minimum_slide_time && (!input.left_slide.is_some() || !fast || stop);
    bit(&mut latch[3], 0x4000_0000, exit_right);
    bit(&mut latch[1], 0x4000_0000, exit_left);
}
