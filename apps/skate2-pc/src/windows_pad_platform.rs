//! Windows API boundary only; native mapping, polling cadence and caches live in core. The one
//! host-side cache: a slot XInput found empty is asked again after a second (`poll_xinput`).
use crate::{pad_sticks::PadSticks, sdl_pads::SdlPads};
use skate2_core::{
    PadCapabilitiesResult, PadPlatform, PadStateResult, PadVibration, XboxPadSample,
};
use std::time::{Duration, Instant};

const STILL: PadVibration = PadVibration { left: 0, right: 0 };
/// How long a slot XInput found empty goes without another XInput call.
const EMPTY_SLOT_RECHECK: Duration = Duration::from_secs(1);

pub(crate) struct WindowsPadPlatform {
    /// Explicit diagnostic replay sample for controller zero; None reads XInput.
    pub(crate) demo_sample: Option<XboxPadSample>,
    /// Pads XInput does not list, in the slots it leaves empty.
    sdl: Option<SdlPads>,
    /// The vibration the game last set on each slot.
    requested: [PadVibration; 4],
    /// PC: the window has the focus. Without it the game takes no pad input and its motors stay
    /// still.
    focused: bool,
    /// PC: the stick settings on each sample, and each slot's sample as its pad reported it.
    sticks: PadSticks,
    /// When XInput last found each slot empty.
    empty_since: [Option<Instant>; 4],
}
impl WindowsPadPlatform {
    pub(crate) fn new(demo_sample: Option<XboxPadSample>, sdl: Option<SdlPads>) -> Self {
        Self {
            demo_sample,
            sdl,
            requested: [STILL; 4],
            focused: false,
            sticks: PadSticks::default(),
            empty_since: [None; 4],
        }
    }
    /// One XInput call for a slot, or its status code. An XInput call on an empty slot looks
    /// through the system's devices (about 0.1 ms) and the game asks every slot every tick, so a
    /// slot found empty answers device-not-connected, as XInput did, until `EMPTY_SLOT_RECHECK`
    /// has passed; a pad plugged in meanwhile is found then.
    fn poll_xinput<T>(&mut self, index: u32, call: impl FnOnce() -> Result<T, xinput::Error>) -> Result<T, u32> {
        let slot = usize::try_from(index).ok().filter(|slot| *slot < self.empty_since.len());
        let now = Instant::now();
        if let Some(slot) = slot
            && self.empty_since[slot].is_some_and(|since| now.saturating_duration_since(since) < EMPTY_SLOT_RECHECK)
        {
            return Err(xinput::error::DEVICE_NOT_CONNECTED.to_u32());
        }
        let result = call();
        if let Some(slot) = slot {
            self.empty_since[slot] = match &result {
                Err(error) if error.kind() == xinput::error::DEVICE_NOT_CONNECTED => Some(now),
                _ => None,
            };
        }
        result.map_err(|error| error.kind().to_u32())
    }
    /// The stick settings the next samples take.
    pub(crate) fn set_stick_settings(&mut self, settings: skate2_core::stick_settings::ControllerSettings) {
        self.sticks.set_settings(settings);
    }
    /// A slot's sticks as its pad last reported them: left x and y, then right.
    pub(crate) fn raw_sticks(&self, slot: usize) -> Option<[i16; 4]> {
        self.sticks.raw(slot)
    }
    /// The window gained or lost the focus: the motors stop, or take what the game last set.
    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused == focused {
            return;
        }
        self.focused = focused;
        for (index, vibration) in (0u32..).zip(self.requested) {
            if vibration != STILL {
                self.send(index, if focused { vibration } else { STILL });
            }
        }
    }
    fn send(&mut self, index: u32, vibration: PadVibration) -> u32 {
        match self.poll_xinput(index, || xinput::set_state(index, [vibration.left, vibration.right])) {
            Ok(()) => 0,
            Err(_) if self.sdl.as_mut().is_some_and(|sdl| sdl.vibrate(index, vibration)) => 0,
            Err(status) => status,
        }
    }
}
impl Drop for WindowsPadPlatform {
    /// XInput's motors keep their last speed after the game: leave every slot still.
    fn drop(&mut self) {
        silence_xinput();
        if let Some(sdl) = self.sdl.as_mut() {
            for index in 0..4 {
                sdl.vibrate(index, STILL);
            }
        }
    }
}
/// Every XInput slot still; the panic hook's too, as the process may end without dropping the
/// pads' owner.
pub(crate) fn silence_xinput() {
    for index in 0..4 {
        let _ = xinput::set_state(index, [0, 0]);
    }
}
#[derive(Debug)]
pub(crate) struct UnsupportedCapabilitiesFlags(u32);
impl std::fmt::Display for UnsupportedCapabilitiesFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported native controller capability flags {}",
            self.0
        )
    }
}
impl std::error::Error for UnsupportedCapabilitiesFlags {}

impl PadPlatform for WindowsPadPlatform {
    type Error = UnsupportedCapabilitiesFlags;
    fn get_state(&mut self, index: u32) -> Result<PadStateResult, Self::Error> {
        if index == 0 {
            if let Some(sample) = self.demo_sample {
                return Ok(PadStateResult {
                    status: 0,
                    state: Some((7, sample)),
                });
            }
        }
        let state = match self.poll_xinput(index, || xinput::get_state(index)) {
            Ok(state) => {
                if let Some(sdl) = &mut self.sdl {
                    sdl.release(index);
                }
                let pad = state.gamepad;
                let sample = XboxPadSample {
                    buttons: pad.buttons.into(),
                    left_trigger: pad.left_trigger,
                    right_trigger: pad.right_trigger,
                    left_x: pad.left_thumb_x,
                    left_y: pad.left_thumb_y,
                    right_x: pad.right_thumb_x,
                    right_y: pad.right_thumb_y,
                };
                if index == 0 && std::env::var_os("SKATE2_TRACE_XINPUT").is_some() {
                    eprintln!(
                        "xinput slot=0 packet={} sample={sample:?}",
                        state.packet_number
                    );
                }
                (state.packet_number, self.sticks.apply(index, sample))
            }
            Err(status) => match self.sdl.as_mut().and_then(|sdl| sdl.claim(index)) {
                Some((packet, sample)) => {
                    if index == 0 && std::env::var_os("SKATE2_TRACE_XINPUT").is_some() {
                        eprintln!("sdl slot=0 packet={packet} sample={sample:?}");
                    }
                    (packet, self.sticks.apply(index, sample))
                }
                None => {
                    self.sticks.clear(index);
                    return Ok(PadStateResult {
                        status,
                        state: None,
                    });
                }
            },
        };
        Ok(PadStateResult {
            status: 0,
            state: Some(state),
        })
    }
    fn get_capabilities(
        &mut self,
        index: u32,
        flags: u32,
    ) -> Result<PadCapabilitiesResult, Self::Error> {
        let flags = match flags {
            1 => xinput::Flag::Gamepad,
            _ => return Err(UnsupportedCapabilitiesFlags(flags)),
        };
        // A diagnostic sample advertises the same controller-0 capabilities
        // as the original-data pad corpus. Pair synthetic state with synthetic
        // capabilities so a disconnected physical pad cannot leave guitar
        // classification unknown; ordinary XInput remains unchanged.
        if index == 0 && self.demo_sample.is_some() {
            return Ok(PadCapabilitiesResult {
                status: 0,
                device_type: Some(1),
                subtype: Some(0),
                flags: Some(0),
            });
        }
        let caps = match self.poll_xinput(index, || xinput::get_capabilities(index, flags)) {
            Ok(caps) => {
                if let Some(sdl) = &mut self.sdl {
                    sdl.release(index);
                }
                PadCapabilitiesResult {
                    status: 0,
                    device_type: Some(caps.ty.into()),
                    subtype: Some(caps.sub_type.into()),
                    flags: Some(caps.flags.into()),
                }
            }
            // DS4Windows showed these pads as a wired Xbox 360 gamepad: type and
            // subtype gamepad, no wireless flag.
            Err(_) if self.sdl.as_mut().and_then(|sdl| sdl.claim(index)).is_some() => {
                PadCapabilitiesResult {
                    status: 0,
                    device_type: Some(1),
                    subtype: Some(1),
                    flags: Some(0),
                }
            }
            // The safe wrapper does not expose failed-call output. Preserve
            // that uncertainty instead of claiming its initialized local survived.
            Err(status) => PadCapabilitiesResult {
                status,
                device_type: None,
                subtype: None,
                flags: None,
            },
        };
        Ok(caps)
    }
    fn set_state(&mut self, index: u32, vibration: PadVibration) -> Result<u32, Self::Error> {
        if let Some(requested) = usize::try_from(index).ok().and_then(|slot| self.requested.get_mut(slot)) {
            *requested = vibration;
        }
        Ok(self.send(index, if self.focused { vibration } else { STILL }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "queries actual Windows XInput controller slots"]
    fn actual_windows_api_preserves_connected_or_disconnected_status() {
        let mut platform = WindowsPadPlatform::new(None, None);
        for index in 0..4 {
            let caps = platform.get_capabilities(index, 1).unwrap();
            assert_ne!(caps.status, 1, "XInput capabilities API must load");
            let state = platform.get_state(index).unwrap();
            assert_ne!(state.status, 1, "XInput state API must load");
            assert_eq!(state.status == 0, state.state.is_some());
            if caps.status == 0 {
                assert!(
                    caps.device_type.is_some() && caps.subtype.is_some() && caps.flags.is_some()
                );
            } else {
                assert!(
                    caps.device_type.is_none() && caps.subtype.is_none() && caps.flags.is_none()
                );
            }
            eprintln!(
                "Windows controller slot{index}: capabilities={}, state={}",
                caps.status, state.status
            );
        }
        let status = platform
            .set_state(4, PadVibration { left: 0, right: 0 })
            .unwrap();
        assert_ne!(status, 0, "invalid slot cannot receive motor output");
    }
}
