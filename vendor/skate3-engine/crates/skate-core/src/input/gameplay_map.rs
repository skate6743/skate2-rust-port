//! Specialization of the 18 stock gameplay expressions registered by82697740.
//! Config identity and VM operand semantics are recorded in STEERING_INPUT.md.
use super::{controller::ActionMap, pad::Pad};

/// One mapped gameplay action as the retail `input.cfg` registers it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionInfo {
    /// cInputMap index, the ID `sdk.input.override_action` takes.
    pub id: u32,
    /// Retail `input.cfg` name.
    pub name: &'static str,
    /// Retail expression over the 24 native pad slots.
    pub expression: &'static str,
    /// Stable snake_case key for mods (`sdk.input.action_ids`).
    pub key: &'static str,
    /// Value range the expression produces.
    pub range: (f32, f32),
}

const fn action(
    id: u32,
    name: &'static str,
    expression: &'static str,
    key: &'static str,
    range: (f32, f32),
) -> ActionInfo {
    ActionInfo {
        id,
        name,
        expression,
        key,
        range,
    }
}

const AXIS: (f32, f32) = (-1.0, 1.0);
const UNIT: (f32, f32) = (0.0, 1.0);

/// The eighteen actions TU3 82697740 registers at 64..81, in index order.
/// Meanings (listener intents) are documented in `sdk/GENERAL_API.md` and
/// checked by `gameplay_map_tests.rs`.
pub const ACTIONS: [ActionInfo; 18] = [
    action(64, "GP_LStickX", "LStickR-LStickL", "left_stick_x", AXIS),
    action(65, "GP_LStickY", "LStickU-LStickD", "left_stick_y", AXIS),
    action(66, "GP_LStickIn", "LStick", "left_stick_click", UNIT),
    action(67, "GP_RStickX", "RStickR-RStickL", "right_stick_x", AXIS),
    action(68, "GP_RStickY", "RStickU-RStickD", "right_stick_y", AXIS),
    action(69, "GP_RStickIn", "RStick", "right_stick_click", UNIT),
    action(70, "GP_LTrigger", "LTrigger", "left_trigger", UNIT),
    action(71, "GP_RTrigger", "RTrigger", "right_trigger", UNIT),
    action(72, "GP_LBumper", "LBumper", "left_bumper", UNIT),
    action(73, "GP_RBumper", "RBumper", "right_bumper", UNIT),
    action(74, "GP_UDPad", "DPadU", "dpad_up", UNIT),
    action(75, "GP_DDPad", "DPadD", "dpad_down", UNIT),
    action(76, "GP_LDPad", "DPadL", "dpad_left", UNIT),
    action(77, "GP_RDPad", "DPadR", "dpad_right", UNIT),
    action(78, "GP_XFace", "X", "x", UNIT),
    action(79, "GP_YFace", "Y", "y", UNIT),
    action(80, "GP_AFace", "A", "a", UNIT),
    action(81, "GP_BFace", "B", "b", UNIT),
];

/// Looks an action up by its mod key (`"a"`, `"left_stick_x"`, ...).
pub fn action_by_key(key: &str) -> Option<&'static ActionInfo> {
    ACTIONS.iter().find(|a| a.key == key)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameplayActions {
    values: [f32; 18],
}

impl GameplayActions {
    pub fn from_values(values: [f32; 18]) -> Self {
        Self { values }
    }

    pub fn values(&self) -> &[f32; 18] {
        &self.values
    }
}
impl GameplayActions {
    /// The native pad retains storage even while count is zero. With a nonzero
    /// count, stock Button operands read their fixed index, without a per-index
    /// count check; the caller must provide the native 24-button storage.
    pub fn from_pad(pad: &Pad) -> Self {
        let value = |slot: usize| {
            if pad.count() == 0 {
                0.0
            } else {
                f32::from_bits(pad.records()[slot][0])
            }
        };
        Self {
            values: [
                value(16) - value(17),
                value(18) - value(19),
                value(6),
                value(20) - value(21),
                value(22) - value(23),
                value(7),
                value(10),
                value(11),
                value(8),
                value(9),
                value(0),
                value(1),
                value(2),
                value(3),
                value(14),
                value(15),
                value(12),
                value(13),
            ],
        }
    }
}
impl ActionMap for GameplayActions {
    fn value(&mut self, action: u32) -> f32 {
        self.values
            [usize::try_from(action.checked_sub(64).expect("gameplay action below 64")).unwrap()]
    }
    fn state(&mut self, action: u32) -> u8 {
        u8::from(self.value(action) != 0.0)
    }
}

#[cfg(test)]
#[path = "gameplay_map_tests.rs"]
mod tests;
