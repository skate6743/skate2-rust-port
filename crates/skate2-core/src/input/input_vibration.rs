//! `Sk8::Input::Vibration`, one per pad (`InputManager::Init` 0x826e2788): eight rumble channels
//! keyed by the event that set them, which `Update` 0x826e1650 sums into the pad's two actuators
//! and decays every frame. `kInputEnableVibration`, `kInputEnableAdditiveVibration` and
//! `kInputEnableVibrationCap` (Input_Hardware) are on, as their cvars default.
use std::collections::BTreeMap;

/// `kVibrationCapScale`.
const CAP_SCALE: f32 = 0.2;
/// `kVibrationCutOffFreq`: a channel whose two values both decayed below it is freed.
const CUT_OFF: f32 = 0.001;
/// `kUpdateTime`: the 60 Hz frame `AddRumbleEvent` turns a decay rate into a factor with.
const UPDATE_TIME: f32 = 1.0 / 60.0;
const CHANNELS: usize = 8;

/// One pad's rumble.
#[derive(Clone, Debug)]
pub(crate) struct Vibration {
    /// `mFrequencyChannels`: low frequency, high frequency and the per-frame decay factor.
    channels: [[f32; 3]; CHANNELS],
    /// `mChannelFreeList`, taken from its back.
    free: Vec<usize>,
    /// `mEventChannelMap`: each event's channel, in event order.
    events: BTreeMap<u32, usize>,
    /// `mEnabled`: `AddRumbleEvent` adds nothing while it is clear.
    pub(crate) enabled: bool,
}

impl Default for Vibration {
    fn default() -> Self {
        Self::new()
    }
}

/// `AddRumbleEvent`'s two `fsel` clamps of a frequency to 0..1 (an unordered value reads 0).
fn unit(value: f32) -> f32 {
    let value = if value >= 0.0 { value } else { 0.0 };
    if 1.0 - value >= 0.0 { value } else { 1.0 }
}

/// The ordered clamp `Update` and `AddRumbleEvent` branch through: below `low` reads `low`, above
/// `high` reads `high`, an unordered value stays.
fn clamp(value: f32, low: f32, high: f32) -> f32 {
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}

impl Vibration {
    /// The constructor's `mEnabled = 1`, then `Reset`.
    pub(crate) fn new() -> Self {
        let mut vibration =
            Self { channels: [[0.0; 3]; CHANNELS], free: Vec::with_capacity(CHANNELS), events: BTreeMap::new(), enabled: true };
        vibration.clear();
        vibration
    }

    /// `Reset` 0x826e1590 up to its `cPadDevice::ResetActuators`, which the pad's owner runs:
    /// every channel zeroed and free again.
    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.free.clear();
        for (index, channel) in self.channels.iter_mut().enumerate() {
            *channel = [0.0; 3];
            self.free.push(index);
        }
    }

    /// `AddRumbleEvent` 0x826e2300: the event's channel (a free one the first time, none while all
    /// eight are taken) takes the two frequencies, each clamped to 0..1, and the decay factor
    /// `1 - decay / 60`, clamped too.
    pub(crate) fn add_rumble_event(&mut self, low: f32, high: f32, decay: f32, event: u32) {
        if !self.enabled {
            return;
        }
        let channel = match self.events.get(&event) {
            Some(&channel) => channel,
            None => {
                let Some(channel) = self.free.pop() else { return };
                self.events.insert(event, channel);
                channel
            }
        };
        let factor = (-decay).mul_add(UPDATE_TIME, 1.0);
        self.channels[channel] = [unit(low), unit(high), clamp(factor, 0.0, 1.0)];
    }

    /// `Update`: the channels' sums, each capped a fifth of the way from its strongest channel to
    /// full and clamped to 0..1, for actuator 0 (the low-frequency motor) and 1 (the high); every
    /// channel then decays by its factor and is freed once both its values fall below the cut-off.
    pub(crate) fn update(&mut self) -> [f32; 2] {
        let (mut low, mut high, mut max_low, mut max_high) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let mut freed = Vec::new();
        for (&event, &channel) in &self.events {
            let [channel_low, channel_high, factor] = self.channels[channel];
            low = channel_low + low;
            high = channel_high + high;
            if channel_low > max_low {
                max_low = channel_low;
            }
            if channel_high > max_high {
                max_high = channel_high;
            }
            let decayed_low = factor * channel_low;
            let decayed_high = factor * channel_high;
            self.channels[channel][0] = decayed_low;
            self.channels[channel][1] = decayed_high;
            if decayed_low < CUT_OFF && decayed_high < CUT_OFF {
                freed.push(event);
            }
        }
        for event in freed {
            if let Some(channel) = self.events.remove(&event) {
                self.free.push(channel);
            }
        }
        let low = clamp(low, 0.0, (1.0 - max_low).mul_add(CAP_SCALE, max_low));
        let high = clamp(high, 0.0, (1.0 - max_high).mul_add(CAP_SCALE, max_high));
        [clamp(low, 0.0, 1.0), clamp(high, 0.0, 1.0)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rumble_decays_by_its_rate_until_the_cut_off_frees_its_channel() {
        let mut vibration = Vibration::new();
        vibration.add_rumble_event(0.5, 0.5, 5.0, 0);
        let factor = (-5.0f32).mul_add(UPDATE_TIME, 1.0);
        assert_eq!(vibration.update(), [0.5, 0.5]);
        assert_eq!(vibration.update(), [0.5 * factor, 0.5 * factor]);
        let mut frames = 2;
        while vibration.update() != [0.0, 0.0] {
            frames += 1;
            assert!(frames < 120, "a decaying rumble must stop");
        }
        assert!(vibration.events.is_empty());
        assert_eq!(vibration.free.len(), CHANNELS);
    }

    #[test]
    fn channels_add_up_to_the_cap_above_the_strongest_one() {
        let mut vibration = Vibration::new();
        vibration.add_rumble_event(0.6, 0.1, 0.0, 2);
        vibration.add_rumble_event(0.6, 0.1, 0.0, 6);
        let cap = (1.0f32 - 0.6).mul_add(CAP_SCALE, 0.6);
        assert_eq!(vibration.update(), [cap, 0.1 + 0.1]);
    }

    #[test]
    fn an_event_keeps_its_channel_and_a_full_set_takes_no_more() {
        let mut vibration = Vibration::new();
        for event in 0..10 {
            vibration.add_rumble_event(0.1, 0.0, 1.0, event);
        }
        assert_eq!(vibration.events.len(), CHANNELS);
        assert!(!vibration.events.contains_key(&8));
        let channel = vibration.events[&3];
        vibration.add_rumble_event(2.0, -1.0, 120.0, 3);
        assert_eq!(vibration.events[&3], channel);
        assert_eq!(vibration.channels[channel], [1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_disabled_pad_adds_nothing_and_what_it_had_dies_out() {
        let mut vibration = Vibration::new();
        vibration.add_rumble_event(1.0, 1.0, 10.0, 0);
        vibration.enabled = false;
        vibration.add_rumble_event(1.0, 1.0, 0.0, 1);
        assert_eq!(vibration.events.len(), 1);
        let mut frames = 0;
        while vibration.update() != [0.0, 0.0] {
            frames += 1;
            assert!(frames < 120);
        }
    }

    #[test]
    fn clear_frees_every_channel() {
        let mut vibration = Vibration::new();
        vibration.add_rumble_event(1.0, 1.0, 0.0, 4);
        vibration.clear();
        assert_eq!(vibration.update(), [0.0, 0.0]);
        assert_eq!(vibration.free.len(), CHANNELS);
    }
}
