//! Observe the same hardware poll used by Skate 2, before conversion/remapping.
use crate::input_pad::{
    XboxPadSample,
    device::{PadCapabilitiesResult, PadPlatform, PadStateResult, PadVibration},
};

pub(super) struct CapturePad<'a, P> {
    pub(super) platform: &'a mut P,
    pub(super) samples: &'a mut [Option<XboxPadSample>; 4],
}

/// While the PC park editor is open the game reads every pad idle (B held while `brake`: a
/// riding skater puts its foot down and stops), so the skater stays put as the world runs; the
/// editor reads `real`, what the pads actually sent.
pub(crate) struct IdlePad<'a, P> {
    pub(crate) platform: &'a mut P,
    pub(crate) real: [Option<XboxPadSample>; 4],
    pub(crate) brake: bool,
}

impl<P: PadPlatform> PadPlatform for IdlePad<'_, P> {
    type Error = P::Error;
    fn get_state(&mut self, index: u32) -> Result<PadStateResult, Self::Error> {
        let result = self.platform.get_state(index)?;
        if let Some(slot) = self.real.get_mut(index as usize) {
            *slot = if result.status == 0 { result.state.map(|(_, sample)| sample) } else { None };
        }
        let buttons = if self.brake { 1 << 13 } else { 0 };
        let idle = XboxPadSample { buttons, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
        Ok(PadStateResult { status: result.status, state: result.state.map(|(packet, _)| (packet, idle)) })
    }
    fn get_capabilities(
        &mut self,
        index: u32,
        flags: u32,
    ) -> Result<PadCapabilitiesResult, Self::Error> {
        self.platform.get_capabilities(index, flags)
    }
    fn set_state(&mut self, index: u32, vibration: PadVibration) -> Result<u32, Self::Error> {
        self.platform.set_state(index, vibration)
    }
}

impl<P: PadPlatform> PadPlatform for CapturePad<'_, P> {
    type Error = P::Error;
    fn get_state(&mut self, index: u32) -> Result<PadStateResult, Self::Error> {
        let result = self.platform.get_state(index)?;
        if let Some(slot) = self.samples.get_mut(index as usize) {
            *slot = if result.status == 0 {
                result.state.map(|(_, sample)| sample)
            } else {
                None
            };
        }
        Ok(result)
    }
    fn get_capabilities(
        &mut self,
        index: u32,
        flags: u32,
    ) -> Result<PadCapabilitiesResult, Self::Error> {
        self.platform.get_capabilities(index, flags)
    }
    fn set_state(&mut self, index: u32, vibration: PadVibration) -> Result<u32, Self::Error> {
        self.platform.set_state(index, vibration)
    }
}
