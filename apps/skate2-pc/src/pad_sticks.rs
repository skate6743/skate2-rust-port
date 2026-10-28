//! PC port addition: the stick settings on every pad's sample (`skate2_core::stick_settings`) as
//! the pad platforms hand samples to the game, and each slot's last sample as its pad reported it,
//! whose sticks the Stick Settings pages show.
use skate2_core::{XboxPadSample, stick_settings::ControllerSettings};

/// The four `XInputGetState` slots.
const SLOTS: usize = 4;

#[derive(Default)]
pub(crate) struct PadSticks {
    settings: ControllerSettings,
    raw: [Option<XboxPadSample>; SLOTS],
}

impl PadSticks {
    /// The settings the next samples take.
    pub(crate) fn set_settings(&mut self, settings: ControllerSettings) {
        self.settings = settings;
    }

    /// A slot's sample as the game takes it; the slot keeps it as the pad reported it.
    pub(crate) fn apply(&mut self, index: u32, sample: XboxPadSample) -> XboxPadSample {
        if let Some(raw) = usize::try_from(index).ok().and_then(|slot| self.raw.get_mut(slot)) {
            *raw = Some(sample);
        }
        self.settings.apply(sample)
    }

    /// A slot that reported no pad.
    pub(crate) fn clear(&mut self, index: u32) {
        if let Some(raw) = usize::try_from(index).ok().and_then(|slot| self.raw.get_mut(slot)) {
            *raw = None;
        }
    }

    /// The left then the right stick's axes, x then y, of a slot's last sample as its pad reported
    /// it.
    pub(crate) fn raw(&self, slot: usize) -> Option<[i16; 4]> {
        self.raw.get(slot).copied().flatten().map(|sample| [sample.left_x, sample.left_y, sample.right_x, sample.right_y])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_core::stick_settings::StickSettings;

    #[test]
    fn samples_take_the_settings_and_slots_keep_what_the_pad_reported() {
        let mut sticks = PadSticks::default();
        let sample = XboxPadSample { buttons: 0x10, left_trigger: 1, right_trigger: 2, left_x: 3277, left_y: -3277, right_x: 0, right_y: 32767 };
        assert_eq!(sticks.apply(1, sample), sample, "vanilla until set");
        sticks.set_settings(ControllerSettings { left: StickSettings { deadzone: 20, ..StickSettings::VANILLA }, ..ControllerSettings::VANILLA });
        let game = sticks.apply(1, sample);
        assert_eq!(game, XboxPadSample { left_x: 0, left_y: 0, ..sample }, "the left stick sits inside its deadzone");
        assert_eq!(sticks.raw(1), Some([3277, -3277, 0, 32767]));
        assert_eq!((sticks.raw(0), sticks.raw(7)), (None, None));
        sticks.clear(1);
        assert_eq!(sticks.raw(1), None);
        // A slot past the four is still handed on, and kept nowhere.
        assert_eq!(sticks.apply(9, sample).left_x, 0);
    }
}
