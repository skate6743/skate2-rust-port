//! The action-graph intents the AI writes, and `OnBoardMotion` / `OffBoardMotion` /
//! `ToggleOffboardState`.
//!
//! `Intents::operator[]` assigns (`insert_or_assign`); `Intents::Insert` 0x824d0c48 leaves an
//! existing value.
use super::{attributes::AiSkaterAttributes, tricks::TrickTable};
use crate::actor_graph_construction::ActorGraphIntents;
use crate::actor_graph_construction::ActorGraphIntentError;

/// `Skater::gSkaterIntentNames` 0x83020618, indexed by `SkaterIntentTuple::mSkaterIntent`, with the
/// eight `OB_Walk*` names Title Update 2 (1.02) appends.
pub(crate) const SKATER_INTENT_NAMES: [&str; 106] = [
    "INVALID", "MarkGonzales", "DannyWay", "QuickOllie", "ForceBrake", "WipeOutRecover", "WipeOutRequest",
    "GestureDown", "GestureLeft", "GestureRight", "GestureUp", "RightPush", "LeftPush", "NewPush", "Pushing",
    "BodySpin", "LeftGroundGrab", "RightGroundGrab", "NewLeftGroundGrab", "NewRightGroundGrab", "LeftAirGrab",
    "RightAirGrab", "NewLeftAirGrab", "NewRightAirGrab", "NewDismount", "Dismount", "TweakX", "TweakY",
    "BoardAdjustAngle", "BoardAdjustMag", "AnticAngle", "AnticMag", "KickTurn", "Turn", "HardTurn",
    "GrindBalanceX", "Brake", "WipeoutGestureX", "WipeoutGestureY", "WipeoutControlX", "WipeoutControlY", "Slide",
    "LeftSlideStart", "RightSlideStart", "LeftSlide", "RightSlide", "Manual", "ManualBrake", "HardTurnCrouch",
    "Crouch", "HoldPattern", "GestureSpeed", "FrontFlip", "BackFlip", "NoInput", "Trick", "TrickHeight",
    "DontMirrorTrick", "SlideFs180", "SlideBs180", "AutoPumpAngle", "AutoPumpMag", "ExitGrind", "GrabWorld",
    "PhysGrindTranslation", "PhysGrindStabilityNudge", "PhysGrindUpDown", "PhysBodySpin", "HandPlantTweakX",
    "HandPlantTweakY", "HandPlantDismount", "HandPlantOneFootRight", "HandPlantOneFootLeft", "FingerFlip",
    "FingerFlipShuv", "FingerFlipVar", "Ollie", "ToggleOffBoardState", "NewToggleOffBoardState", "AI_ForceOffBoard",
    "OB_Steer", "OB_Mag", "OBL_Mag", "OBL_Steer", "OB_MagHeld", "OB_Jump", "OB_Sprint", "OB_MOMag", "OB_MOSteer",
    "OB_ObjectMvZ", "OB_ObjectMvX", "OB_ObjectMvRot", "OB_LookAtX", "OB_LookAtY", "OB_HoldBoard", "OB_DropBoard",
    "OB_ThrowBoard", "OB_RetrieveBoard", "OB_WalkForward", "OB_WalkForwardRight", "OB_WalkRight",
    "OB_WalkBackwardRight", "OB_WalkBackward", "OB_WalkBackwardLeft", "OB_WalkLeft", "OB_WalkForwardLeft",
];

/// `Skater::*Intent` names the AI writes directly.
pub(crate) mod name {
    pub(crate) const WIPE_OUT_REQUEST: &str = "WipeOutRequest";
    pub(crate) const GESTURE_DOWN: &str = "GestureDown";
    pub(crate) const GESTURE_LEFT: &str = "GestureLeft";
    pub(crate) const GESTURE_RIGHT: &str = "GestureRight";
    pub(crate) const RIGHT_PUSH: &str = "RightPush";
    pub(crate) const LEFT_PUSH: &str = "LeftPush";
    pub(crate) const NEW_PUSH: &str = "NewPush";
    pub(crate) const PUSHING: &str = "Pushing";
    pub(crate) const BODY_SPIN: &str = "BodySpin";
    pub(crate) const BOARD_ADJUST_ANGLE: &str = "BoardAdjustAngle";
    pub(crate) const BOARD_ADJUST_MAG: &str = "BoardAdjustMag";
    pub(crate) const ANTIC_ANGLE: &str = "AnticAngle";
    pub(crate) const ANTIC_MAG: &str = "AnticMag";
    pub(crate) const KICK_TURN: &str = "KickTurn";
    pub(crate) const TURN: &str = "Turn";
    pub(crate) const BRAKE: &str = "Brake";
    pub(crate) const LEFT_SLIDE_START: &str = "LeftSlideStart";
    pub(crate) const RIGHT_SLIDE_START: &str = "RightSlideStart";
    pub(crate) const LEFT_SLIDE: &str = "LeftSlide";
    pub(crate) const RIGHT_SLIDE: &str = "RightSlide";
    pub(crate) const MANUAL: &str = "Manual";
    pub(crate) const MANUAL_BRAKE: &str = "ManualBrake";
    pub(crate) const CROUCH: &str = "Crouch";
    pub(crate) const HOLD_PATTERN: &str = "HoldPattern";
    pub(crate) const GESTURE_SPEED: &str = "GestureSpeed";
    pub(crate) const TRICK: &str = "Trick";
    pub(crate) const TRICK_HEIGHT: &str = "TrickHeight";
    pub(crate) const DONT_MIRROR_TRICK: &str = "DontMirrorTrick";
    pub(crate) const SLIDE_FS_180: &str = "SlideFs180";
    pub(crate) const SLIDE_BS_180: &str = "SlideBs180";
    pub(crate) const GRAB_WORLD: &str = "GrabWorld";
    pub(crate) const TOGGLE_OFF_BOARD_STATE: &str = "ToggleOffBoardState";
    pub(crate) const NEW_TOGGLE_OFF_BOARD_STATE: &str = "NewToggleOffBoardState";
    pub(crate) const OB_STEER: &str = "OB_Steer";
    pub(crate) const OB_MAG: &str = "OB_Mag";
    pub(crate) const OBL_MAG: &str = "OBL_Mag";
    pub(crate) const OBL_STEER: &str = "OBL_Steer";
    pub(crate) const OB_MAG_HELD: &str = "OB_MagHeld";
    pub(crate) const OB_JUMP: &str = "OB_Jump";
}

/// `eScorableID_fsrevert`, `eScorableID_bsrevert`, `eScorableID_nosemanual`,
/// `eScorableID_tailmanual`.
pub(crate) mod scorable {
    pub(crate) const NOSEMANUAL: i32 = 2;
    pub(crate) const TAILMANUAL: i32 = 3;
    pub(crate) const FSREVERT: i32 = 4;
    pub(crate) const BSREVERT: i32 = 5;
}

/// `Intents::operator[]` followed by a store.
pub(crate) fn set(intents: &mut ActorGraphIntents, intent: &str, value: f32) -> Result<(), ActorGraphIntentError> {
    intents.insert_or_assign(ActorGraphIntents::native_name(intent.as_bytes()), value)
}

/// `Intents::Insert`.
pub(crate) fn insert(intents: &mut ActorGraphIntents, intent: &str, value: f32) -> Result<(), ActorGraphIntentError> {
    intents.insert_unique(ActorGraphIntents::native_name(intent.as_bytes()), value).map(drop)
}

/// Whether `Intents::find` locates the intent.
pub(crate) fn has(intents: &ActorGraphIntents, intent: &str) -> bool {
    intents.get(&ActorGraphIntents::native_name(intent.as_bytes())).is_some()
}

/// `kfDegreesToRadians`.
const DEGREES_TO_RADIANS: f32 = 0.017453292;

/// `AI::OnBoardMotion`.
pub(crate) mod on_board {
    use super::*;

    /// `SetTrickIntents` 0x824d2450: each of the scorable's skater-intent tuples whose frame
    /// window holds `trick_frame`.
    pub(crate) fn set_trick_intents(
        intents: &mut ActorGraphIntents,
        tricks: &TrickTable,
        scorable: i32,
        trick_frame: i32,
    ) -> Result<(), ActorGraphIntentError> {
        if !super::super::tricks::is_valid_scorable(scorable) {
            return Ok(());
        }
        let Some(settings) = tricks.settings(scorable) else {
            return Ok(());
        };
        for tuple in settings.skater_intent_tuples {
            if tuple.skater_intent != 0
                && trick_frame >= tuple.start_frame
                && (tuple.end_frame < 0 || trick_frame <= tuple.end_frame)
            {
                let name = SKATER_INTENT_NAMES.get(tuple.skater_intent as usize).copied().unwrap_or("INVALID");
                set(intents, name, tuple.value)?;
            }
        }
        Ok(())
    }

    /// `SetSlideIntent` 0x824d2530.
    pub(crate) fn set_slide_intent(intents: &mut ActorGraphIntents, deviation: f32) -> Result<(), ActorGraphIntentError> {
        let (start, slide) = if deviation >= 0.0 {
            (name::LEFT_SLIDE_START, name::LEFT_SLIDE)
        } else {
            (name::RIGHT_SLIDE_START, name::RIGHT_SLIDE)
        };
        set(intents, start, 1.0)?;
        set(intents, slide, f32::from_bits(0x3ecc_cccd))
    }

    /// `SetBoardAdjust` 0x824d25c0.
    pub(crate) fn set_board_adjust(intents: &mut ActorGraphIntents, angle: f32, magnitude: f32) -> Result<(), ActorGraphIntentError> {
        set(intents, name::BOARD_ADJUST_ANGLE, angle)?;
        set(intents, name::BOARD_ADJUST_MAG, magnitude)
    }

    /// `SetRevertIntent` 0x824d2630.
    pub(crate) fn set_revert_intent(intents: &mut ActorGraphIntents, revert: i32) -> Result<(), ActorGraphIntentError> {
        set(intents, name::TRICK, 1.0)?;
        match revert {
            scorable::FSREVERT => set(intents, name::SLIDE_FS_180, 1.0),
            scorable::BSREVERT => set(intents, name::SLIDE_BS_180, 1.0),
            _ => Ok(()),
        }
    }

    /// `TurnOnGround` 0x824d26b8: the deviation over the full-turn angle, culled below
    /// `kCullSmallTurns` (0), turning away from positive deviation.
    pub(crate) fn turn_on_ground(
        intents: &mut ActorGraphIntents,
        attributes: &AiSkaterAttributes,
        deviation: f32,
    ) -> Result<(), ActorGraphIntentError> {
        const CULL_SMALL_TURNS: f32 = 0.0;
        let full = attributes.full_turn_deviation_degrees * DEGREES_TO_RADIANS;
        let magnitude = deviation.abs();
        let fraction = if magnitude >= full { 1.0 } else { magnitude / full };
        let mut turn = if fraction >= CULL_SMALL_TURNS { fraction } else { 0.0 };
        if deviation > 0.0 {
            turn = -turn;
        }
        set(intents, name::TURN, turn)?;
        set(intents, name::BODY_SPIN, turn)?;
        set(intents, name::KICK_TURN, turn)
    }

    /// `TurnInAir` 0x824d2800: nothing inside the dead zone, else the excess over the
    /// remaining range.
    pub(crate) fn turn_in_air(
        intents: &mut ActorGraphIntents,
        attributes: &AiSkaterAttributes,
        deviation: f32,
    ) -> Result<(), ActorGraphIntentError> {
        let dead_zone = attributes.air_turn_dead_zone_degrees * DEGREES_TO_RADIANS;
        let excess = deviation.abs() - dead_zone;
        if excess > 0.0 {
            let range = attributes.air_full_turn_degrees.mul_add(DEGREES_TO_RADIANS, -dead_zone);
            let mut turn = if excess >= range { 1.0 } else { excess / range };
            if deviation > 0.0 {
                turn = -turn;
            }
            set(intents, name::TURN, turn)?;
            set(intents, name::BODY_SPIN, turn)?;
            set(intents, name::KICK_TURN, turn)?;
        }
        Ok(())
    }
}

/// `AI::OffBoardMotion`.
pub(crate) mod off_board {
    use super::*;

    /// `GenerateMoveIntents` 0x824d2350.
    pub(crate) fn generate_move_intents(intents: &mut ActorGraphIntents, amount: f32) -> Result<(), ActorGraphIntentError> {
        let value = if amount <= 1.0 { amount } else { 1.0 };
        set(intents, name::OB_MAG, value)?;
        set(intents, name::OBL_MAG, value)?;
        set(intents, name::OB_MAG_HELD, value)
    }

    /// `GenerateTurnIntents` 0x824d23d8.
    pub(crate) fn generate_turn_intents(intents: &mut ActorGraphIntents, amount: f32) -> Result<(), ActorGraphIntentError> {
        set(intents, name::OB_STEER, amount)?;
        set(intents, name::OBL_STEER, amount)?;
        set(intents, name::OB_MAG_HELD, amount)
    }
}

/// `ToggleOffboardState::OffBoardToggleStatus`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OffBoardToggleStatus {
    #[default]
    NoRequest,
    AttemptToGetOffBoard,
    AttemptToGetOnBoard,
}

/// `AI::ToggleOffboardState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ToggleOffboardState {
    pub(crate) requested_state: OffBoardToggleStatus,
    pub(crate) num_frames_since_toggling: i32,
}

impl ToggleOffboardState {
    /// `kMinFramesBetweenToggleRequests`.
    const MIN_FRAMES_BETWEEN_TOGGLE_REQUESTS: i32 = 4;

    /// The requests `Update` makes: a different state replaces the request.
    pub(crate) fn request(&mut self, state: OffBoardToggleStatus) {
        if self.requested_state != state {
            self.requested_state = state;
        }
    }

    /// `ResolveOffBoardStatus` 0x824d2980.
    pub(crate) fn resolve_off_board_status(
        &mut self,
        intents: &mut ActorGraphIntents,
        currently_off_board: bool,
        transitioning: bool,
    ) -> Result<(), ActorGraphIntentError> {
        let requested = self.requested_state;
        if requested != OffBoardToggleStatus::NoRequest
            && self.num_frames_since_toggling > Self::MIN_FRAMES_BETWEEN_TOGGLE_REQUESTS
            && !transitioning
        {
            let toggle = if currently_off_board {
                requested == OffBoardToggleStatus::AttemptToGetOnBoard
            } else {
                requested == OffBoardToggleStatus::AttemptToGetOffBoard
            };
            if toggle {
                set(intents, name::NEW_TOGGLE_OFF_BOARD_STATE, 1.0)?;
                set(intents, name::TOGGLE_OFF_BOARD_STATE, 1.0)?;
                self.num_frames_since_toggling = 0;
            }
        }
        self.num_frames_since_toggling += 1;
        Ok(())
    }
}
