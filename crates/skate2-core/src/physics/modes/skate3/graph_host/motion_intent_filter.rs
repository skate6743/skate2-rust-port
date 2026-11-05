//! Stock FilterMotionGraphIntent constructor and map lifecycle adapter.
use skate3_core::{animation::intent_filter::{Settings, State}, graph::intents::IntentMap};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub intent: String,
    pub filtered_intent: String,
    pub settings: Settings,
}

impl Operation {
    /// TU3 82BB12C8. Presence flags remain distinct from numeric defaults.
    pub fn parse(a: &Attributes<'_>) -> Self {
        let number = |name, default: f32| f32::from_bits(a.float_bits(name, default.to_bits()));
        let optional = |name| a.get(name).map(|value| f32::from_bits(value.float_bits));
        let blend = number("blend", 1.0);
        Self {
            intent: a.text("intent").unwrap_or("").into(),
            filtered_intent: a.text("filteredIntent").unwrap_or("").into(),
            settings: Settings {
                starting_value: number("startingValue", 0.0),
                default_value: number("defaultValue", 0.0),
                scale: number("scale", 1.0),
                filters: [
                    filter(a.text("filter1").or_else(|| a.text("filter"))),
                    filter(a.text("filter2")),
                    filter(a.text("fakieFilter")),
                    filter(a.text("mirrorFilter")),
                ],
                ramp_time: optional("rampTime"),
                blend_rising: number("blendRising", blend),
                blend_falling: number("blendFalling", blend),
                blend_out: optional("blendOut"),
                clamp_velocity: optional("clampVel"),
                clamp_acceleration: optional("clampAcc"),
            },
        }
    }

    pub fn begin(&self, state: &mut State, output: &mut IntentMap) {
        output.insert(&self.filtered_intent, state.begin(&self.settings));
    }

    pub fn update(
        &self,
        state: &mut State,
        input: &IntentMap,
        output: &mut IntentMap,
        dt: f32,
        stance: (bool, bool),
    ) {
        let value = state.update(&self.settings, input.get(&self.intent).copied(), dt, stance);
        output.insert(&self.filtered_intent, value);
    }

    /// End removes the key even when another active behavior wrote that alias.
    /// Native map removal 82BC1068 has no per-behavior ownership counter.
    pub fn end(&self, output: &mut IntentMap) {
        output.remove(&self.filtered_intent);
    }
}

fn filter(value: Option<&str>) -> u32 {
    // 82BA0A48 and original initializers 82F847C0..4850.
    use skate3_core::animation::playback_parameters::intent_key;
    let key = intent_key(value.unwrap_or("none"));
    ["negate", "abs", "oneMinus", "clamp", "angleFlip", "angleRot90", "angleRotN90"]
        .iter()
        .position(|name| intent_key(name) == key)
        .map_or(0, |index| index as u32 + 1)
}
