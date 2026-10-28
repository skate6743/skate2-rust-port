//! Actual Xenon device state; OS calls and unknown native output are explicit.
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences, pad_vibration::PadVibrationFloat},
    input_pad::{
        InputPadConversionError, InputPadDeviceEffects, XboxPadSample, try_convert_xbox_pad,
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};

/// Native DWORD status: every nonzero bit pattern, including signed-negative,
/// is failure. Packet number is carried but never used to deduplicate samples.
pub struct PadStateResult {
    pub status: u32,
    pub state: Option<(u32, XboxPadSample)>,
}
pub struct PadCapabilitiesResult {
    pub status: u32,
    pub device_type: Option<u8>,
    pub subtype: Option<u8>,
    pub flags: Option<u16>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadVibration {
    pub left: u16,
    pub right: u16,
}
pub trait PadPlatform {
    type Error;
    /// # Errors
    /// Returns host transport errors; native API failure statuses remain in the result.
    fn get_state(&mut self, index: u32) -> Result<PadStateResult, Self::Error>;
    /// # Errors
    /// Returns host transport errors; native API failure statuses remain in the result.
    fn get_capabilities(
        &mut self,
        index: u32,
        flags: u32,
    ) -> Result<PadCapabilitiesResult, Self::Error>;
    /// # Errors
    /// Returns host transport errors; native API failure statuses remain in the result.
    fn set_state(&mut self, index: u32, vibration: PadVibration) -> Result<u32, Self::Error>;
}
pub(crate) trait PadRemap<R> {
    type Error;
    fn update(
        &mut self,
        handle: &R,
        player: u32,
        values: &mut [f32; 24],
    ) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum PadMappingError<E> {
    UnknownGuitar,
    Remap(E),
}
#[derive(Debug, PartialEq)]
pub(crate) enum PadSampleError<P, N, R> {
    Platform(P),
    UnknownSuccessfulState,
    Mapping(InputPadConversionError<N, PadMappingError<R>>),
}
#[derive(Debug, PartialEq)]
pub(crate) enum PadActuatorError<P, N> {
    Index(u32),
    Platform(P),
    UnknownCapability(&'static str),
    Cvar(InputCvarError),
    MissingTarget(&'static str),
    Numeric(N),
}
#[derive(Clone, Copy)]
struct Actuator {
    desired: u16,
    set: u16,
}
pub(crate) struct XenonPadDevice<R> {
    index: u32,
    remap: Option<R>,
    active: bool,
    guitar: Option<bool>,
    actuators: [Actuator; 2],
    vibration: PadVibration,
}
impl<R: Clone> XenonPadDevice<R> {
    /// One successful allocation from Construct8298d310's ascending four slots.
    pub(crate) fn new(index: u32) -> Self {
        Self {
            index,
            remap: None,
            active: false,
            guitar: None,
            actuators: [Actuator { desired: 0, set: 1 }; 2],
            vibration: PadVibration { left: 0, right: 0 },
        }
    }
    pub(crate) fn set_remap(&mut self, remap: Option<R>) {
        self.remap = remap;
    }
    #[expect(
        clippy::type_complexity,
        reason = "preserves separate platform, numeric and remap errors"
    )]
    pub(crate) fn sample<P: PadPlatform, N: SkeletonCollisionNumeric, M: PadRemap<R>>(
        &mut self,
        values: &mut [f32; 24],
        platform: &mut P,
        numeric: &mut N,
        remap: &mut M,
    ) -> Result<i32, PadSampleError<P::Error, N::Error, M::Error>> {
        let sampled = platform
            .get_state(self.index)
            .map_err(PadSampleError::Platform)?;
        if sampled.status != 0 {
            self.active = false;
            return Ok(0);
        }
        if self.actuators[0].desired != self.actuators[0].set
            || self.actuators[1].desired != self.actuators[1].set
        {
            self.vibration.left = self.actuators[0].desired;
            self.actuators[0].set = self.actuators[0].desired;
            self.vibration.right = self.actuators[1].desired;
            self.actuators[1].set = self.actuators[1].desired;
            let _status = platform
                .set_state(self.index, self.vibration)
                .map_err(PadSampleError::Platform)?;
        }
        let (_, sample) = sampled
            .state
            .ok_or(PadSampleError::UnknownSuccessfulState)?;
        // A pad no capabilities read classified (one connected after `InputManager::Init` read
        // its empty slot keeps that failed read's unknown subtype): its own capabilities
        // classify it as `SetActuator` does; one whose subtype they cannot prove maps nothing.
        if self.guitar.is_none() {
            let caps = platform
                .get_capabilities(self.index, 1)
                .map_err(PadSampleError::Platform)?;
            if caps.status == 0 {
                self.guitar = caps.subtype.map(|subtype| subtype == 7);
            }
            if self.guitar.is_none() {
                return Ok(0);
            }
        }
        let mut effects = Mapping {
            device: self,
            remap,
        };
        try_convert_xbox_pad(sample, values, numeric, &mut effects)
            .map_err(PadSampleError::Mapping)?;
        Ok(24)
    }
    #[expect(
        clippy::manual_clamp,
        reason = "preserves native ordered comparisons and unordered input behavior"
    )]
    pub(crate) fn set_actuator<P: PadPlatform, N: ReckoningNumeric>(
        &mut self,
        actuator: u32,
        mut frequency: f32,
        platform: &mut P,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), PadActuatorError<P::Error, N::Error>> {
        use PadActuatorError as E;
        if actuator >= 2 {
            return Err(E::Index(actuator));
        }
        let caps = platform
            .get_capabilities(self.index, 1)
            .map_err(E::Platform)?;
        let supported =
            if caps.status == 0 && caps.device_type.ok_or(E::UnknownCapability("Type"))? == 1 {
                caps.flags.ok_or(E::UnknownCapability("Flags"))? & 2 != 0
            } else {
                false
            };
        // The native unconditional SubType read and false/conditional-true stores
        // produce unknown guitar when the platform cannot prove the output byte.
        // Propagate that uncertainty; disconnected samples never consume it.
        self.guitar = caps.subtype.map(|subtype| subtype == 7);
        // PC port: a still request stays still. On a wireless pad the 360 tuning's high
        // frequency offset (`kHighFreqOffset`, +0.05) would otherwise hold that motor at 5%
        // whenever nothing rumbles.
        if supported && frequency != 0.0 {
            if let Some(low) = refs
                .capture_pad_vibration(PadVibrationFloat::LowScale)
                .map_err(E::Cvar)?
            {
                let (scale, offset) = if actuator == 0 {
                    let scale = low.read().map_err(E::Cvar)?;
                    let offset = refs
                        .capture_pad_vibration(PadVibrationFloat::LowOffset)
                        .map_err(E::Cvar)?
                        .ok_or(E::MissingTarget("kLowFreqOffset"))?
                        .read()
                        .map_err(E::Cvar)?;
                    (scale, offset)
                } else {
                    let scale = refs
                        .capture_pad_vibration(PadVibrationFloat::HighScale)
                        .map_err(E::Cvar)?
                        .ok_or(E::MissingTarget("kHighFreqScale"))?;
                    let offset = refs
                        .capture_pad_vibration(PadVibrationFloat::HighOffset)
                        .map_err(E::Cvar)?
                        .ok_or(E::MissingTarget("kHighFreqOffset"))?;
                    (
                        scale.read().map_err(E::Cvar)?,
                        offset.read().map_err(E::Cvar)?,
                    )
                };
                frequency = scale.mul_add(frequency, offset);
                // Native BGE/ BLE are LT-clear / GT-clear: unordered survives.
                if frequency < 0.0 {
                    frequency = 0.0;
                } else if frequency > 1.0 {
                    frequency = 1.0;
                }
            }
        }
        let desired = if frequency == 0.0 {
            0
        } else {
            let value = numeric
                .truncate_i32(frequency * f32::from_bits(0x477f_ff00))
                .map_err(E::Numeric)?;
            u16::try_from(value.clamp(0, 65535)).expect("source integer clamp")
        };
        self.actuators[usize::try_from(actuator).expect("checked actuator")].desired = desired;
        Ok(())
    }
    pub(crate) fn reset_actuators<P: PadPlatform, N: ReckoningNumeric>(
        &mut self,
        platform: &mut P,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), PadActuatorError<P::Error, N::Error>> {
        for actuator in 0..2 {
            self.set_actuator(actuator, 0.0, platform, refs, numeric)?;
        }
        Ok(())
    }
}
struct Mapping<'a, R, M> {
    device: &'a mut XenonPadDevice<R>,
    remap: &'a mut M,
}
impl<R: Clone, M: PadRemap<R>> InputPadDeviceEffects for Mapping<'_, R, M> {
    type Error = PadMappingError<M::Error>;
    type Remap = R;
    fn is_guitar(&mut self) -> Result<bool, Self::Error> {
        self.device.guitar.ok_or(PadMappingError::UnknownGuitar)
    }
    fn capture_remap(&mut self) -> Result<Option<R>, Self::Error> {
        Ok(self.device.remap.clone())
    }
    fn remap(&mut self, remap: &R, player: u32, values: &mut [f32; 24]) -> Result<(), Self::Error> {
        self.remap
            .update(remap, player, values)
            .map_err(PadMappingError::Remap)
    }
}
#[cfg(test)]
#[path = "input_xenon_pad_device_tests.rs"]
pub(crate) mod tests;
