//! Animation.Update82e31970 after Physics.Update; not full `UpdateAll`.
use super::{
    Conditioners, PhysicalPlayerPhysOut, animation::turn::AnimationTurnError,
    stairs::PhysicsUpdateComplete,
};
use crate::{
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::InputCvarReferences,
    physical_player_phys_out::construct::conditioner_animation_output::{
        AnimationOutput, AnimationOutputError,
    },
};
use std::sync::Arc;

pub(crate) struct AnimationUpdateComplete {
    output: Arc<()>,
}
impl AnimationUpdateComplete {
    pub(crate) fn matches_output(&self, output: &PhysicalPlayerPhysOut) -> bool {
        Arc::ptr_eq(&self.output, output.occurrence())
    }
}
#[derive(Debug)]
pub(crate) enum AnimationUpdateError {
    ForeignOutput,
    MissingAnimation,
    MissingAttributes,
    Output(AnimationOutputError),
    Turn(AnimationTurnError<AnimationOutputError>),
}
impl std::fmt::Display for AnimationUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignOutput => f.write_str("animation conditioner foreign output"),
            Self::MissingAttributes => {
                f.write_str("animation conditioner missing retained attributes")
            }
            Self::MissingAnimation => f.write_str("animation conditioner not published"),
            Self::Output(e) => write!(f, "animation conditioner: {e}"),
            Self::Turn(AnimationTurnError::Source(e)) => write!(f, "animation turn: {e}"),
            Self::Turn(AnimationTurnError::MissingFilter(filter)) => {
                write!(f, "animation turn missing filter {filter:?}")
            }
        }
    }
}
impl Conditioners {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume Physics prerequisite token"
    )]
    pub(crate) fn try_update_animation(
        &mut self,
        before: PhysicsUpdateComplete,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<AnimationUpdateComplete, AnimationUpdateError> {
        use AnimationUpdateError as E;
        if !before.matches_output(output) || !self.matches_output(output) {
            return Err(E::ForeignOutput);
        }
        let animation = self
            .animation
            .published_value_mut()
            .ok_or(E::MissingAnimation)?;
        if !Arc::ptr_eq(&animation.output, output.occurrence()) {
            return Err(E::ForeignOutput);
        }
        let attributes = self
            .animation_attributes
            .as_ref()
            .ok_or(E::MissingAttributes)?;
        let mut source = AnimationOutput {
            output,
            attributes,
            cvars,
            overlay,
        };
        animation
            .payload
            .try_update_current_trick(&mut source)
            .map_err(E::Output)?;
        animation
            .payload
            .try_update_completed_trick(&mut source)
            .map_err(E::Output)?;
        animation
            .payload
            .try_update_animation_turn(&mut source)
            .map_err(E::Turn)?;
        animation
            .payload
            .try_update_body_tilt(&mut source)
            .map_err(E::Output)?;
        animation
            .payload
            .try_update_deck_ground_acceleration(&mut source)
            .map_err(E::Output)?;
        Ok(AnimationUpdateComplete {
            output: Arc::clone(output.occurrence()),
        })
    }
}

#[cfg(feature = "skate3-runtime")]
impl Conditioners {
    /// Source movement has already completed its animation calculations.
    /// Retain the host's authored trick history for its HUD and career scoring.
    pub(crate) fn try_update_skate3_animation(
        &mut self,
        publication: crate::physical_player_phys_out::skate3::Skate3Publication,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<AnimationUpdateComplete, AnimationUpdateError> {
        use AnimationUpdateError as E;
        if !publication.matches(output) || !self.matches_output(output) { return Err(E::ForeignOutput); }
        let animation = self.animation.published_value_mut().ok_or(E::MissingAnimation)?;
        if !Arc::ptr_eq(&animation.output, output.occurrence()) { return Err(E::ForeignOutput); }
        let attributes = self.animation_attributes.as_ref().ok_or(E::MissingAttributes)?;
        let mut source = AnimationOutput { output, attributes, cvars, overlay };
        animation.payload.try_update_current_trick(&mut source).map_err(E::Output)?;
        animation.payload.try_update_completed_trick(&mut source).map_err(E::Output)?;
        Ok(AnimationUpdateComplete { output: Arc::clone(output.occurrence()) })
    }
}
