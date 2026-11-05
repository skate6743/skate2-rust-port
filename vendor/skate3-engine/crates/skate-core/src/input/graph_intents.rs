//! Native CreateMGIntentFromAGIntent and AttachIntent behavior boundaries.

/// Complete82BA0C20. Ordinals come from82BA0A48 and stock filter names.
pub fn apply_filter(value: f32, kind: u32) -> f32 {
    let pi = f32::from_bits(0x40490fdb);
    let half_pi = f32::from_bits(0x3fc90fdb);
    match kind {
        0 => value,
        1 => value * -1.0,
        2 => value.abs(),
        3 => {
            if value >= 0.0 {
                1.0 - value
            } else {
                -1.0 - value
            }
        }
        4 => {
            let lower = if -value >= 0.0 { 0.0 } else { value };
            if 1.0 - lower >= 0.0 { lower } else { 1.0 }
        }
        5 => {
            if value >= 0.0 {
                pi - value
            } else {
                -pi - value
            }
        }
        6 => {
            let rotated = value - half_pi;
            if rotated < -pi {
                rotated + f32::from_bits(0x40c90fdb)
            } else {
                rotated
            }
        }
        _ => {
            let rotated = value + half_pi;
            if rotated > pi {
                rotated - f32::from_bits(0x40c90fdb)
            } else {
                rotated
            }
        }
    }
}

/// Four source ordinals: filter1, filter2, fakieFilter, mirrorFilter.
/// Missing stance component bypasses every filter, matching82BA2930.
pub fn filter_chain(mut value: f32, filters: [u32; 4], stance: Option<(bool, bool)>) -> f32 {
    if let Some((fakie, mirror)) = stance {
        for (kind, enabled) in filters.into_iter().zip([true, true, fakie, mirror]) {
            if enabled && kind <= 7 {
                value = apply_filter(value, kind);
            }
        }
    }
    value
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntentMutation {
    None,
    Remove,
    Set(f32),
}

pub struct CreateMgIntent {
    pub on_update: bool,
    pub default_value: Option<f32>,
    pub scale: f32,
    pub filters: [u32; 4],
}

impl CreateMgIntent {
    /// Complete82BA2790 with container lookup/removal/insertion represented by
    /// an explicit mutation. Keys are the caller's decoded MGIntent/AGIntent.
    pub fn emit(&self, action_value: Option<f32>, stance: Option<(bool, bool)>) -> IntentMutation {
        match action_value.or(self.default_value) {
            None => IntentMutation::Remove,
            Some(value) => {
                IntentMutation::Set(filter_chain(self.scale * value, self.filters, stance))
            }
        }
    }
    /// Complete82BA26F0 lifecycle write, including absent-input removal.
    pub fn enter(
        &self,
        created_this_frame: &mut bool,
        action_value: Option<f32>,
        stance: Option<(bool, bool)>,
    ) -> IntentMutation {
        let result = self.emit(action_value, stance);
        *created_this_frame = true;
        result
    }
    /// Complete82BA2728: onUpdate=false removes on subsequent updates.
    pub fn update(
        &self,
        created_this_frame: &mut bool,
        action_value: Option<f32>,
        stance: Option<(bool, bool)>,
    ) -> IntentMutation {
        let result = if self.on_update {
            self.emit(action_value, stance)
        } else if !*created_this_frame {
            IntentMutation::Remove
        } else {
            IntentMutation::None
        };
        *created_this_frame = false;
        result
    }
    ///82BA2788 ->82BA28B8 unconditionally removes the owned MG intent.
    pub fn exit(&self) -> IntentMutation {
        IntentMutation::Remove
    }
}

/// Complete82BB5AE0 at resolved lookup/sink boundaries. The packet sink is
/// invoked first; optional skeleton sink receives the same value afterwards.
pub fn attach_intent(
    value: Option<f32>,
    set_skeleton: bool,
    mut packet: impl FnMut(f32),
    mut skeleton: impl FnMut(f32),
) {
    if let Some(value) = value {
        packet(value);
        if set_skeleton {
            skeleton(value);
        }
    }
}
