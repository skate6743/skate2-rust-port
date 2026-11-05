//! TU3 RawControllerInput 82598FE0 and DerivedControllerInput 825992D8.
//! Native action-map evaluation is an explicit boundary, not a platform mapping.
use crate::physics::reciprocal_sqrt::estimate;

/// Evaluated native cInputMap actions. IDs are TU3 map indices, not XInput
/// buttons. Implementations must preserve the call order; the stock expression
/// evaluator 8296C350 and loaded mapping still determine deadzones and scaling.
pub trait ActionMap {
    fn value(&mut self, action: u32) -> f32;
    fn state(&mut self, action: u32) -> u8;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawControllerInput {
    /// Six binary32 axes/triggers followed by native packed button bits.
    words: [u32; 7],
}

impl RawControllerInput {
    pub fn from_words(words: [u32; 7]) -> Self {
        Self { words }
    }
    pub fn words(&self) -> &[u32; 7] {
        &self.words
    }

    /// Complete 82598FE0, including simultaneous bumper arbitration and
    /// retention of the low twenty bits of the destination flags word.
    pub fn update(
        &mut self,
        previous: &Self,
        map: &mut impl ActionMap,
        state_502: bool,
        state_104: bool,
    ) {
        for (slot, action) in [64, 65, 67, 68, 70, 71].into_iter().enumerate() {
            self.words[slot] = map.value(action).to_bits();
        }
        for (action, bit) in [(66, 31), (69, 30), (72, 29), (73, 28)] {
            self.set_bit(bit, map.state(action));
        }
        let both = (1 << 29) | (1 << 28);
        if self.words[6] & both == both {
            let prior = previous.words[6] & both;
            if (prior == 1 << 29 || prior == 1 << 28) && (state_502 || state_104) {
                self.words[6] = (self.words[6] & !both) | prior;
            } else {
                self.words[6] &= !(1 << 28);
            }
        }
        for action in 74..=81 {
            self.set_bit(101 - action, map.state(action));
        }
    }

    fn set_bit(&mut self, bit: u32, value: u8) {
        self.words[6] = (self.words[6] & !(1 << bit)) | (u32::from(value & 1) << bit);
    }
}

/// Actual inputlistener attribute result for key 1F3C1C1C68AF4C23.
/// Missing collection/attribute uses the live 830D0850 fallback.
pub struct MagnitudeHeldSettings {
    pub attribute: Option<f32>,
    pub missing_attribute_value: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedControllerInput {
    words: [u32; 26],
}

impl DerivedControllerInput {
    /// Requires native initialized state. No reset or inferred default supplied.
    pub fn from_words(words: [u32; 26]) -> Self {
        Self { words }
    }
    pub fn words(&self) -> &[u32; 26] {
        &self.words
    }

    /// Prefix construction recovered from TU3 `82599718`. The native
    /// constructor clears the controller axes and timers, preserves the low
    /// twenty flag bits in the two packed raw words, and seeds the four
    /// edge-timer fields from `flt_822F8B34` (`0x7effffff`). This is only the
    /// controller prefix; listener registration/reset belongs to its owner.
    pub fn initialize(&mut self) {
        let flags_last = self.words[6] & 0x000f_ffff;
        let flags_current = self.words[13] & 0x000f_ffff;
        self.words = [0; 26];
        self.words[6] = flags_last;
        self.words[13] = flags_current;
        self.words[16..20].fill(0x7eff_ffff);
    }

    /// Complete 825992D8: raw history, trigger/button edge timers and stick
    /// magnitude-held timers. Timing uses caller dt without an imposed rate.
    pub fn update(
        &mut self,
        map: &mut impl ActionMap,
        timestep: f32,
        state_502: bool,
        state_104: bool,
        settings: &MagnitudeHeldSettings,
    ) {
        let previous = RawControllerInput::from_words(self.words[7..14].try_into().unwrap());
        self.words[..7].copy_from_slice(previous.words());
        let mut current = previous;
        current.update(&previous, map, state_502, state_104);
        self.words[7..14].copy_from_slice(current.words());
        for (slot, axis) in [(14, 4), (15, 5)] {
            self.advance_or_reset(slot, f32::from_bits(current.words[axis]) == 1.0, timestep);
        }
        let before = previous.words[6];
        let now = current.words[6];
        for (bit, rise, fall) in [(31, 16, 17), (30, 18, 19)] {
            let was_down = before & (1 << bit) != 0;
            let is_down = now & (1 << bit) != 0;
            self.advance_or_reset(rise, !(!was_down && is_down), timestep);
            self.advance_or_reset(fall, !(was_down && !is_down), timestep);
        }
        for (slot, action) in [(20, 80), (21, 81), (22, 78), (23, 79)] {
            self.advance_or_reset(slot, map.state(action) != 0, timestep);
        }
        // Native evaluates these actions again after the button timers.
        let lx = map.value(64);
        let ly = map.value(65);
        let rx = map.value(67);
        let ry = map.value(68);
        let right = magnitude(ry.mul_add(ry, rx * rx));
        let left = magnitude(ly.mul_add(ly, lx * lx));
        let threshold = settings
            .attribute
            .unwrap_or(settings.missing_attribute_value);
        // PPC ble treats unordered as false; retain that branch direction.
        self.advance_or_reset(24, !(left <= threshold), timestep);
        self.advance_or_reset(25, !(right <= threshold), timestep);
    }

    fn advance_or_reset(&mut self, slot: usize, advance: bool, timestep: f32) {
        self.words[slot] = if advance {
            (f32::from_bits(self.words[slot]) + timestep).to_bits()
        } else {
            0
        };
    }
}

#[cfg(test)]
#[path = "controller_startup.rs"]
mod startup_tests;

/// Independent reciprocal-square-root estimate followed by the original two
/// refinement steps. Host arithmetic is not claimed to be bit-exact Xenon math.
pub(super) fn magnitude(squared: f32) -> f32 {
    let mut inverse = estimate(squared);
    for _ in 0..2 {
        let correction = (-squared).mul_add(inverse * inverse, 1.0);
        inverse = (inverse * 0.5).mul_add(correction, inverse);
    }
    if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    }
}
