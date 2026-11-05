//! Special scalar handlers and unconditional suffix of82BDA0D0.
use super::{extended_attributes::ExtendedAttributes, scalar_attributes::ScalarAttributeInputs};
use crate::input::controller::ActionMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpAttributeState {
    /// Skeleton16488/16492, cached by PrepareJump.
    pub prepared_controls: [f32; 2],
    /// Skeleton16496/16511; this cache survives only the native suffix gate.
    pub height_override: f32,
    pub height_override_active: bool,
}

impl JumpAttributeState {
    /// Skeleton constructor82BD7A54..82BD7A98, once per skeleton.
    pub fn new() -> Self {
        Self {
            prepared_controls: [0.0; 2],
            height_override: 0.0,
            height_override_active: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FinalizationInput {
    /// Selected global292 layout1872/1876/1880.
    pub select_jump_extremes: bool,
    pub low_jump_threshold: f32,
    pub high_jump_threshold: f32,
    /// Selected state variant layout76.
    pub allow_height_override: bool,
    /// Selected global208 layout452.
    pub use_prepared_controls: bool,
    /// Actual AnimOutPhysIn10496 and10932.
    pub external_impulse_active: bool,
    pub animation_flags: u32,
}

/// Function locals are reset on every ProcessAnimAttributes invocation.
pub struct Accumulator {
    revert: bool,
    revert_direction: f32,
    minimum_jump: f32,
    trick_height: f32,
    has_trick_height: bool,
}
impl Accumulator {
    pub fn new() -> Self {
        Self {
            revert: false,
            revert_direction: 1.0,
            minimum_jump: 0.0,
            trick_height: 0.0,
            has_trick_height: false,
        }
    }
    pub fn dispatch(
        &mut self,
        name: &str,
        scalar: impl Fn() -> Result<f32, String>,
        fields: &mut ScalarAttributeInputs,
        cached: &mut JumpAttributeState,
        map: &mut Option<&mut dyn ActionMap>,
    ) -> Result<bool, String> {
        match name {
            "Revert" => self.revert = true,
            "RevertDir" => {
                self.revert_direction = scalar()?;
                fields.flags2472 |= 0x200000;
            }
            "MinJump" => self.minimum_jump = scalar()?,
            "TrickHeight" => {
                self.trick_height = scalar()?;
                self.has_trick_height = true;
            }
            "PrepareJump" => {
                fields.flags2468 |= 0x200000;
                if let Some(map) = map {
                    //82590358/825903C8 call stock cInputMap64 and65.
                    cached.prepared_controls =
                        [clamp_control(map.value(64)), clamp_control(map.value(65))];
                }
            }
            "JumpHeightOverride" => {
                cached.height_override = scalar()?;
                cached.height_override_active = true;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub fn finish(
        self,
        fields: &mut ScalarAttributeInputs,
        extra: &mut ExtendedAttributes,
        cached: &mut JumpAttributeState,
        input: FinalizationInput,
        map: Option<&mut dyn ActionMap>,
    ) {
        if self.revert {
            extra.revert_direction = self.revert_direction;
        }
        if fields.flags2468 & 0x400000 != 0 {
            extra.jump_strength = if input.select_jump_extremes
                && self.trick_height <= input.low_jump_threshold
            {
                self.minimum_jump
            } else if input.select_jump_extremes && self.trick_height >= input.high_jump_threshold {
                1.0
            } else {
                self.trick_height
            };
        }
        if self.has_trick_height && input.allow_height_override {
            if cached.height_override_active {
                let difference = cached.height_override - extra.jump_strength;
                if difference >= 0.0 {
                    extra.jump_strength = cached.height_override;
                }
            }
        } else {
            cached.height_override = 0.0;
            cached.height_override_active = false;
        }
        if extra.flags2480 & 0x1000 != 0 {
            extra.jump_strength = self.trick_height;
        }
        if input.use_prepared_controls {
            extra.jump_controls = cached.prepared_controls;
        } else if let Some(map) = map {
            //The suffix deliberately selects68 through8255E010, not65 used
            //by PrepareJump. Keep the recovered source's distinct routing.
            extra.jump_controls = [clamp_control(map.value(64)), clamp_control(map.value(68))];
        }
        if input.external_impulse_active {
            fields.flags2468 |= 0x40000;
        }
        //82BDC650 reads a BYTE at10932, the high byte of this big-endian
        //word. The later publications read the complete word separately.
        extra.flags2480 = (extra.flags2480 & !8) | ((input.animation_flags >> 24) & 8);
        fields.flags2484 = (fields.flags2484 & !2) | ((input.animation_flags >> 22) & 2);
        fields.flags2484 =
            (fields.flags2484 & 0x7fffffff) | ((input.animation_flags << 6) & 0x80000000);
        fields.flags2484 =
            (fields.flags2484 & !0x01000000) | ((input.animation_flags >> 4) & 0x01000000);
        if fields.flags2476 & 0x600 != 0 {
            fields.flags2476 &= !0x400000;
        }
    }
}

fn clamp_control(value: f32) -> f32 {
    let lower = if -1.0 - value >= 0.0 { -1.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}
