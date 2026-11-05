//! Persistent physical-to-animation conditioning with stock collection values.
use skate3_core::{
    animation::{
        ground_acceleration,
        physical_feedback::{self, ControlFeedback, PhysicalFeedback, ReckoningFeedback},
    },
    input::turn_conditioner,
    physics::board_motion_output::BoardMotionOutput,
    riding::{pumping::state::PumpingState, speed_wobble::SpeedWobbleState},
};
use skate_data::collections::Collections;

pub(crate) struct AnimationFeedback {
    settings: turn_conditioner::Settings,
    bump_settings: ground_acceleration::Settings,
    state: turn_conditioner::State,
    previous_lateral_tilt: [f32; 4],
    pub published_previous_lateral_tilt: [f32; 4],
}

impl AnimationFeedback {
    pub fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            //82DEFAA0..FAC8 zeros the retained vector96.
            previous_lateral_tilt: [0.0; 4],
            published_previous_lateral_tilt: [0.0; 4],
            settings: super::animation_feedback_settings::load(data)?,
            // Global344 at8289FCA0 binds hash7823ADA8E7C896AA = bumps.
            bump_settings: ground_acceleration::Settings {
                scale_x_acc: data.float("anim_motion", "bumps", "scale_x_acc")?,
                min_bump_mag: data.float("anim_motion", "bumps", "min_bump_mag")?,
            },
            //82DEFAA0 resets histories to zero. Every82DEFF38 update refreshes
            //the four coefficient words before using each filter.
            state: turn_conditioner::State {
                history: [0.0; 8],
                filters: [[0.0; 9]; 3],
            },
        })
    }

    pub fn reset(&mut self) {
        self.state.reset_history();
        self.previous_lateral_tilt = [0.0; 4];
        self.published_previous_lateral_tilt = [0.0; 4];
    }

    /// Call once after the native physical outputs are published. Absence of a
    /// completed Reckoning frame belongs to the caller, not a fallback here.
    pub fn update(
        &mut self,
        motion: &BoardMotionOutput,
        pumping: &PumpingState,
        wobble: &SpeedWobbleState,
        reckoning: ReckoningFeedback,
        controls: ControlFeedback,
        acceleration: ground_acceleration::Input,
        lateral_tilt: [f32; 4],
    ) -> PhysicalFeedback {
        //82DEFB6C..FB7C publishes the preceding Animation0 at Animation16,
        //then retains current Reckoning1248 for the next conditioner update.
        self.published_previous_lateral_tilt = self.previous_lateral_tilt;
        self.previous_lateral_tilt = lateral_tilt;
        physical_feedback::publish(
            &mut self.state,
            &self.settings,
            motion,
            pumping,
            wobble,
            reckoning,
            controls,
            ground_acceleration::publish(acceleration, &self.bump_settings),
        )
    }
}
