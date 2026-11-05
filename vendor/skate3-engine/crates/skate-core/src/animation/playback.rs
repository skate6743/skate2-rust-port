//! MotionGraph PlayAnimation ctor82BB4B38, Begin82BB5188, Update82BB5670.
//! End is the verified no-op82B61BB8; leaving a graph state does not stop a tree.
use super::{
    output::attributes::AttributeName,
    playback_parameters::{AttributeSink, ParameterInputs, PlaybackParameter},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionSettings {
    ///1 play,2 blend,3 channelblend,4 sequence. Zero means no graph override.
    pub kind: u32,
    pub seconds: f32,
    pub under: u32,
    ///0 default,1 current frame,2 match phase,3 match frame.
    pub matching: u32,
    pub use_channels_from_weights: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayAnimation {
    pub animation: String,
    pub switch_animation: Option<String>,
    pub mirror_animation: Option<String>,
    pub no_board_animation: Option<String>,
    pub playback_speed: f32,
    pub apply_posture: bool,
    pub transition: TransitionSettings,
    pub parameters: Vec<PlaybackParameter>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackRequest {
    pub animation: String,
    pub speed: f32,
    pub start_time: f32,
    pub transition: TransitionSettings,
}

/// Values published by the actual skater-animation/physical interfaces. The
/// no-board bit is PhysOut+72's byte311, not a contact-derived guess.
pub struct PlaybackContext {
    pub is_switch: Option<bool>,
    pub is_mirrored: Option<bool>,
    pub board_available: Option<bool>,
    pub pro_skater: AttributeName,
    pub transition_override: Option<TransitionSettings>,
}

pub trait PlaybackService: ParameterInputs + AttributeSink {
    fn set_construction_value(&mut self, name: AttributeName, value: AttributeName);
    fn set_posture_enabled(&mut self, enabled: bool);
    /// Return false only when the native controller's Play/Blend/Sequence
    /// request fails. Unimplemented evaluation is an error, never false.
    fn play(&mut self, request: PlaybackRequest) -> Result<bool, String>;
}

#[derive(Clone, Debug, Default)]
pub struct PlayAnimationInstance {
    first_update: bool,
}
impl PlayAnimationInstance {
    pub fn begin(
        &mut self,
        operation: &PlayAnimation,
        context: &mut PlaybackContext,
        service: &mut impl PlaybackService,
    ) -> Result<(), String> {
        self.first_update = true;
        refresh_parameters(&operation.parameters, true, service)?;
        service.set_construction_value(
            super::skeleton_input::name::encode(b"ProSkater"),
            context.pro_skater,
        );
        // The graph override is consumed/reset before variant choice or play.
        let transition = context
            .transition_override
            .take()
            .filter(|settings| settings.kind != 0)
            .unwrap_or(operation.transition);
        let mut animation = &operation.animation;
        if let Some(switch) = &operation.switch_animation {
            if context
                .is_switch
                .ok_or("PlayAnimation needs skater switch state")?
            {
                animation = switch;
            }
        }
        if let Some(mirror) = &operation.mirror_animation {
            if context
                .is_mirrored
                .ok_or("PlayAnimation needs skater mirror state")?
            {
                animation = mirror;
            }
        }
        if let Some(no_board) = &operation.no_board_animation {
            if !context
                .board_available
                .ok_or("PlayAnimation needs PhysOut board availability")?
            {
                animation = no_board;
            }
        }
        service.set_posture_enabled(operation.apply_posture);
        let played = if (1..=4).contains(&transition.kind) {
            service.play(PlaybackRequest {
                animation: animation.clone(),
                speed: operation.playback_speed,
                start_time: 0.0,
                transition,
            })?
        } else {
            false
        };
        if !played {
            // This fallback is present in native Begin82BB5630..5660. The
            // service must not use it to conceal missing tree implementations.
            service.play(PlaybackRequest {
                animation: "KeepDefaultAnim".into(),
                speed: 1.0,
                start_time: 0.0,
                transition: TransitionSettings {
                    kind: 1,
                    ..operation.transition
                },
            })?;
        }
        Ok(())
    }

    pub fn update(
        &mut self,
        operation: &PlayAnimation,
        service: &mut impl PlaybackService,
    ) -> Result<(), String> {
        if !self.first_update {
            refresh_parameters(&operation.parameters, false, service)?;
        }
        self.first_update = false;
        Ok(())
    }
    pub fn end(&mut self) {}
}

fn refresh_parameters(
    parameters: &[PlaybackParameter],
    beginning: bool,
    service: &mut impl PlaybackService,
) -> Result<(), String> {
    // Keep read/write effects interleaved across parameters while avoiding two
    // mutable borrows of the same concrete animation service.
    struct One(Option<super::playback_parameters::SettableAttribute>);
    impl AttributeSink for One {
        fn set_attribute(&mut self, value: super::playback_parameters::SettableAttribute) {
            self.0 = Some(value);
        }
    }
    for parameter in parameters {
        let mut output = One(None);
        parameter.update(beginning, service, &mut output)?;
        if let Some(attribute) = output.0 {
            service.set_attribute(attribute);
        }
    }
    Ok(())
}
