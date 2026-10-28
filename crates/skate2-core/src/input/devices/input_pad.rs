//! Native pad conversion82989448 and retained `cInputPad::Update829863b0`.
//! Platform sampling, device/cache timing and Actor ownership remain callers.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected input owners await platform polling and Actor scheduling"
    )
)]
use crate::skeleton_collision_numeric::{
    SkeletonCollisionNumeric, length_from_squared_and_estimate,
};
use std::{cmp::Ordering, fmt};
/// Raw XInput-equivalent integer semantics, before any deadzone or Y inversion.
/// A disconnected sample is not this packet: native `SampleButtons` returns count0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XboxPadSample {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
}
/// Actual retained device flags/remap. No implicit ordinary-device defaults.
pub(crate) trait InputPadDeviceEffects {
    type Error;
    type Remap;
    fn is_guitar(&mut self) -> Result<bool, Self::Error>;
    fn capture_remap(&mut self) -> Result<Option<Self::Remap>, Self::Error>;
    fn remap(
        &mut self,
        remap: &Self::Remap,
        player: u32,
        values: &mut [f32; 24],
    ) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum InputPadConversionError<N, D> {
    Numeric(N),
    Device(D),
}
/// Fixed24 capacity establishes the native output-capacity assertion. Failure
/// preserves completed output stores, including those before the right seed.
pub(crate) fn try_convert_xbox_pad<N: SkeletonCollisionNumeric, D: InputPadDeviceEffects>(
    sample: XboxPadSample,
    output: &mut [f32; 24],
    numeric: &mut N,
    device: &mut D,
) -> Result<(), InputPadConversionError<N::Error, D::Error>> {
    use InputPadConversionError as E;
    for (bit, value) in output[..10].iter_mut().enumerate() {
        *value = if sample.buttons & (1u16 << bit) != 0 {
            1.0
        } else {
            0.0
        };
    }
    output[10] = f32::from(sample.left_trigger) * f32::from_bits(0x3b80_8081);
    output[11] = f32::from(sample.right_trigger) * f32::from_bits(0x3b80_8081);
    for (index, value) in output[12..16].iter_mut().enumerate() {
        *value = if sample.buttons & (1u16 << (index + 12)) != 0 {
            1.0
        } else {
            0.0
        };
    }
    let (x, y) = stick(sample.left_x, sample.left_y, numeric).map_err(E::Numeric)?;
    output[18] = positive(y);
    output[16] = positive(x);
    output[17] = negative(x);
    output[19] = negative(y);
    let (x, y) = stick(sample.right_x, sample.right_y, numeric).map_err(E::Numeric)?;
    output[20] = positive(x);
    output[22] = positive(y);
    output[23] = negative(y);
    output[21] = negative(x);
    if device.is_guitar().map_err(E::Device)? {
        for index in [10, 11, 21, 22] {
            output[index] = 0.0;
        }
    }
    if let Some(remap) = device.capture_remap().map_err(E::Device)? {
        device.remap(&remap, 0, output).map_err(E::Device)?;
    }
    Ok(())
}
#[expect(
    clippy::manual_clamp,
    reason = "retain original ordered branch selection, including unordered radius propagation"
)]
fn stick<N: SkeletonCollisionNumeric>(
    raw_x: i16,
    raw_y: i16,
    numeric: &mut N,
) -> Result<(f32, f32), N::Error> {
    let x = f32::from(raw_x) * f32::from_bits(0x3800_0000);
    let y = f32::from(raw_y) * f32::from_bits(0x3800_0000);
    let x_squared = x * x;
    let squared = y.mul_add(y, x_squared);
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let length = length_from_squared_and_estimate(squared, seed);
    let scale = if length.partial_cmp(&f32::from_bits(0x3a83_126f)) == Some(Ordering::Less) {
        0.0
    } else {
        let radius = (length - f32::from_bits(0x3e80_0000)) * f32::from_bits(0x3fb6_db6e);
        let radius = if radius < 0.0 {
            0.0
        } else if radius > 1.0 {
            1.0
        } else {
            radius
        };
        radius / length
    };
    Ok((scale * x, scale * y))
}
fn positive(axis: f32) -> f32 {
    if -axis >= 0.0 { 0.0 } else { axis }
}
fn negative(axis: f32) -> f32 {
    if axis >= 0.0 { 0.0 } else { -axis }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "four independently retained native button flags with different transition/repeat semantics"
)]
#[derive(Clone, Copy)]
struct Button {
    value: f32,
    pressed: bool,
    released: bool,
    held: bool,
    repeat: bool,
    repeat_counter: i32,
    transition_counter: i32,
}
impl Button {
    const ZERO: Self = Self {
        value: 0.0,
        pressed: false,
        released: false,
        held: false,
        repeat: false,
        repeat_counter: 0,
        transition_counter: 0,
    };
    fn update(&mut self, value: f32) {
        self.value = value;
        let held = value > 0.5;
        if self.transition_counter < 3 {
            self.pressed = false;
            self.released = false;
            self.transition_counter = self.transition_counter.wrapping_add(1);
        } else if self.held == held {
            self.pressed = false;
            self.released = false;
        } else {
            self.held = held;
            self.pressed = held;
            self.released = !held;
            self.transition_counter = 0;
        }
        if self.held {
            if self.repeat_counter == 0 {
                self.repeat = true;
                self.repeat_counter = 24;
            } else {
                self.repeat_counter = self.repeat_counter.wrapping_sub(1);
                if self.repeat_counter == 0 {
                    self.repeat = true;
                    self.repeat_counter = 12;
                } else {
                    self.repeat = false;
                }
            }
        } else {
            self.repeat_counter = 0;
            self.repeat = false;
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InputPadError {
    CountTooLarge(i32),
    NegativeCount(i32),
    ValuesTooShort { count: usize, length: usize },
    Index { index: i32, count: i32 },
}
impl fmt::Display for InputPadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountTooLarge(n) => write!(f, "pad count {n} exceeds24"),
            Self::NegativeCount(n) => write!(f, "negative pad count {n}"),
            Self::ValuesTooShort { count, length } => {
                write!(f, "pad needs {count} values, received {length}")
            }
            Self::Index { index, count } => write!(f, "pad index {index} outside count {count}"),
        }
    }
}
/// Only count0 is initialized by ctor82986398. Dormant records remain unknown
/// until source Update growth clears them. Shrink/disconnect does not clear.
pub(crate) struct InputPad {
    count: i32,
    buttons: [Option<Button>; 24],
}
impl InputPad {
    pub(crate) const fn new() -> Self {
        Self {
            count: 0,
            buttons: [None; 24],
        }
    }
    pub(crate) const fn num_buttons(&self) -> i32 {
        self.count
    }
    pub(crate) fn try_update(&mut self, values: &[f32], count: i32) -> Result<(), InputPadError> {
        if count > 24 {
            return Err(InputPadError::CountTooLarge(count));
        }
        if count < 0 {
            return Err(InputPadError::NegativeCount(count));
        }
        // Old count remains constructor/checked-update invariant, hence its two native
        // assertions cannot fail through this typed API. No unrelated reset exists.
        let count_usize = usize::try_from(count).expect("nonnegative checked count");
        if values.len() < count_usize {
            return Err(InputPadError::ValuesTooShort {
                count: count_usize,
                length: values.len(),
            });
        }
        let old = usize::try_from(self.count).expect("retained count invariant");
        if count_usize > old {
            self.buttons[old..count_usize].fill(Some(Button::ZERO));
        }
        self.count = count;
        for (button, value) in self.buttons[..count_usize].iter_mut().zip(values) {
            button
                .as_mut()
                .expect("source initialized exposed record")
                .update(*value);
        }
        Ok(())
    }
    fn button(&self, index: i32) -> Result<Option<&Button>, InputPadError> {
        if self.count == 0 {
            return Ok(None);
        }
        if index < 0 || index >= self.count {
            return Err(InputPadError::Index {
                index,
                count: self.count,
            });
        }
        Ok(self.buttons[usize::try_from(index).expect("checked index")].as_ref())
    }
    pub(crate) fn value(&self, index: i32) -> Result<f32, InputPadError> {
        Ok(self.button(index)?.map_or(0.0, |b| b.value))
    }
    pub(crate) fn held(&self, index: i32) -> Result<bool, InputPadError> {
        Ok(self.button(index)?.is_some_and(|b| b.held))
    }
    pub(crate) fn pressed(&self, index: i32) -> Result<bool, InputPadError> {
        Ok(self.button(index)?.is_some_and(|b| b.pressed))
    }
    pub(crate) fn released(&self, index: i32) -> Result<bool, InputPadError> {
        Ok(self.button(index)?.is_some_and(|b| b.released))
    }
    pub(crate) fn repeat(&self, index: i32) -> Result<bool, InputPadError> {
        Ok(self.button(index)?.is_some_and(|b| b.repeat))
    }
}
#[cfg(test)]
#[path = "input_pad_tests.rs"]
pub(crate) mod tests;

impl<N: fmt::Debug, D: fmt::Debug> fmt::Display for InputPadConversionError<N, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(e) => write!(f, "pad conversion numeric: {e:?}"),
            Self::Device(e) => write!(f, "pad device effect: {e:?}"),
        }
    }
}

#[path = "../input_xenon_pad_device.rs"]
pub(crate) mod device;
#[path = "../input_device_manager.rs"]
pub(crate) mod device_manager;
#[path = "../input_vibration.rs"]
pub(crate) mod vibration;
#[path = "../input_rumble_manager.rs"]
pub(crate) mod rumble;
