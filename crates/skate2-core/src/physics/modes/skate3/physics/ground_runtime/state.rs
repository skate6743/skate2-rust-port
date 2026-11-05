//! Native Ground controller ownership. Source entry/update and output retain
//! the same instances; constructing this does not mark Ground as entered.
use super::super::animation_input::AnimationInput;
use super::GroundPumping;
use skate3_core::physics::board_toolkit::BoardToolkit;
use skate3_core::{
    physics::manual::state::ManualState,
    player::input_phase::ProcessedPhysicsInput,
    riding::{
        grounded::state::{
            PhysicsGroundState,
            output::{self, GroundOutputFrame, GroundOutputSettings, PhysicsGroundOutput},
        },
        pumping::state::PumpingState,
        speed_model::SpeedModelState,
        speed_wobble::SpeedWobbleState,
        steering::TruckSteeringState,
    },
};
use skate_data::collections::Collections;

pub(crate) struct GroundState {
    pub state: PhysicsGroundState,
    pub pumping: PumpingState,
    pub wobble: SpeedWobbleState,
    pub manual: ManualState,
    pub steering: TruckSteeringState,
    pub speed: SpeedModelState,
    pub heading_previous: f32,
    pub pumping_settings: GroundPumping,
    output_settings: GroundOutputSettings,
    auto_push_enabled: [bool; 5],
    pub entered: bool,
    pub(super) entry_settings: super::entry::EntrySettings,
}
impl GroundState {
    pub(crate) fn adopt_mode_settings(&mut self, other: Self) { self.auto_push_enabled = other.auto_push_enabled; self.pumping_settings = other.pumping_settings; }
    pub fn load(data: &Collections, _mode: &str, human_player: bool) -> Result<Self, String> {
        let push = |field| data.float("physics_push", "default", field);
        Ok(Self {
            state: PhysicsGroundState::before_first_enter(human_player),
            pumping: PumpingState::reset_state(),
            wobble: SpeedWobbleState([0; 8]),
            manual: ManualState {
                filtered_angle_error: 0.0,
                target_angle: 0.0,
                measured_angle: 0.0,
                angular_correction: 0.0,
                elapsed: 0.0,
            },
            //Initial truck state; later board resets preserve activation timers.
            steering: TruckSteeringState::default(),
            speed: SpeedModelState {
                target_speed: 0.0,
                flags_1360: 0,
            },
            heading_previous: 0.0,
            pumping_settings: GroundPumping::load(data)?,
            output_settings: GroundOutputSettings {
                pushable_speed_terms_4_8: [
                    push("MaxPushableSpeed_CameraDelta")?,
                    push("MaxPushableSpeed")?,
                ],
                mode_speed_threshold_0: push("Hash_501D5581043D7D3C")?,
            },
            entry_settings: super::entry::EntrySettings::load(data)?,
            auto_push_enabled: crate::skate3_native::difficulty::NATIVE_MODES.map(|mode|
                data.boolean("physics_mode", mode, "AutoPushEnabled"))
                .into_iter().collect::<Result<Vec<_>, _>>()?.try_into().unwrap(),
            entered: false,
        })
    }
    pub fn output(
        &self,
        p: &ProcessedPhysicsInput,
        animation: &AnimationInput,
        t: &BoardToolkit,
    ) -> PhysicsGroundOutput {
        output::fill_physics_output(
            &self.state,
            GroundOutputFrame {
                axis_464: p.vectors_464_480_496_512_528[0].map(f32::from_bits),
                velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
                absolute_body_speed_2616: t.absolute_speed,
                deck_speed_2652: p.scalar_2652,
                state_timer_2664: p.state_timer_2664,
                scalar_2720: animation.fields.balance,
                flags_2476: p.flags_2476,
                flags_2484: p.flags_2484,
                selected_mode_flag_109: self.auto_push_enabled[p.state_variant_index_2528 as usize],
            },
            self.output_settings,
        )
    }
}
