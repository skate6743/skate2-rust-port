//! Complete TU3 Xbox pad conversion 8296D5F8, after successful platform polling.
use super::controller::magnitude;

/// Fields of the native Xbox state passed by value to 8296D5F8.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XboxState {
    pub buttons: u16,
    pub triggers: [u8; 2],
    pub left: [i16; 2],
    pub right: [i16; 2],
}

/// Returns all 24 native button values. `device_byte_13` is explicit because
/// its producer/meaning is not inferred from the conversion's four clear stores.
pub fn convert(state: &XboxState, device_byte_13: u8) -> [f32; 24] {
    let mut result = [0.0; 24];
    for (bit, output) in result[..10].iter_mut().enumerate() {
        *output = if state.buttons & (1 << bit) != 0 {
            1.0
        } else {
            0.0
        };
    }
    result[10] = f32::from(state.triggers[0]) * f32::from_bits(0x3b808081);
    result[11] = f32::from(state.triggers[1]) * f32::from_bits(0x3b808081);
    for (slot, bit) in [(12, 12), (13, 13), (14, 14), (15, 15)] {
        result[slot] = if state.buttons & (1 << bit) != 0 {
            1.0
        } else {
            0.0
        };
    }
    let left = condition(state.left);
    let right = condition(state.right);
    result[16..20].copy_from_slice(&left);
    result[20..24].copy_from_slice(&right);
    if device_byte_13 != 0 {
        for slot in [10, 11, 21, 22] {
            result[slot] = 0.0;
        }
    }
    result
}

fn condition(stick: [i16; 2]) -> [f32; 4] {
    let x = f32::from(stick[0]) * f32::from_bits(0x38000000);
    let y = f32::from(stick[1]) * f32::from_bits(0x38000000);
    let squared = y.mul_add(y, x * x);
    let length = magnitude(squared);
    let factor = if length < f32::from_bits(0x3a83126f) {
        0.0
    } else {
        let scaled = (length - 0.25) * f32::from_bits(0x3fb6db6e);
        let bounded = if scaled < 0.0 {
            0.0
        } else if scaled > 1.0 {
            1.0
        } else {
            scaled
        };
        bounded / length
    };
    let x = factor * x;
    let y = factor * y;
    [positive(x), positive(-x), positive(y), positive(-y)]
}

fn positive(value: f32) -> f32 {
    // fsel(-value, +0, value), including the native signed-zero selection.
    if -value >= 0.0 { 0.0 } else { value }
}
