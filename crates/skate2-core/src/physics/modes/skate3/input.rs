//! Platform input adapter; no animation or physics state mutation here.

use crate::skate3_native::math_host::*;

mod controllers;
pub(crate) mod gesture_catalog;
mod gesture_mapping_data;
pub(crate) mod gesture_mapping;
pub(crate) mod gesture_input;
pub(crate) mod platform;
pub(crate) use controllers::{ControllerInput, ControllerStatus, RawInput};
use skate3_core::input::tick::TickInput;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PublishedTickInput(pub TickInput);

impl Default for PublishedTickInput {
    fn default() -> Self {
        Self(TickInput::new(
            0,
            skate3_core::input::gameplay_map::GameplayActions::from_values([0.0; 18]),
            false,
        ))
    }
}

/// Host-supplied packet through the original cache, Pad and history pipeline.
pub(crate) fn collect_embedded(input: &mut ControllerInput, state: Option<skate3_core::input::xbox::XboxState>) {
    let number = input.publications.wrapping_add(1) as u32;
    input.collect([
        state.map(|state| platform::DevicePacket { number, state, subtype: 1 }).ok_or(platform::DeviceError::Disconnected),
        Err(platform::DeviceError::Disconnected),
        Err(platform::DeviceError::Disconnected),
        Err(platform::DeviceError::Disconnected),
    ]);
}

pub(crate) fn publish_embedded(input: &mut ControllerInput) -> TickInput {
    input.publish_actions();
    input.tick_input()
}

pub(crate) fn sample_embedded(input: &mut ControllerInput, state: skate3_core::input::xbox::XboxState) -> TickInput {
    collect_embedded(input, Some(state));
    input.publish_actions();
    input.tick_input()
}

