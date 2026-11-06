//! Per-node allocation retained by the production MotionHost.
use super::*;
pub(super) enum Instance {
    Grind(super::super::motion_grind::State),
    Trick(i32),
    Stateless,
    MatchCadence(super::super::motion_offboard_cadence::MatchCadence),
    ToggleBoard(skate3_core::player::offboard::toggle_board::State),
    Runout(super::super::motion_runout::State),
    IntentFilter(skate3_core::animation::intent_filter::State),
    Landing(super::super::motion_landing::State),
    Wipeout(super::super::motion_wipeout::State),
    OffboardBodyTweak(body_tweak::State),
    MatchAirTime(match_air_time::State),
    TwistLean(skate3_core::player::offboard::twist_lean::State),
    Play(PlayAnimationInstance),
    Push(PushInstance),
    Turning(set_turning::State),
    Crouching(Option<skate3_core::animation::crouching::State>),
    BodyTilt(skate3_core::animation::body_tilt::State),
    RidingFakie(skate3_core::animation::riding_fakie::State),
    Pumping(skate3_core::animation::pumping_channel::State),
    FakieHead(super::super::motion_channels::FakieHead),
    LandingData(super::super::motion_native::LandingData),
    Sliding(super::super::motion_sliding::State),
    SlideDeceleration(f32),
    KickTurn(skate3_core::animation::kickturn::State),
    BodySpin(super::super::motion_spin::State),
    AirLeg(skate3_core::animation::air_leg_extension::State),
    CharacterGesture(super::super::motion_character_gesture::CharacterGesture),
    Shove(super::super::motion_shove::ShoveState),
    SetManualAngle(skate3_core::animation::manual::State),
    HippyJumpAntic(super::super::motion_hippy_jump::State),
    FingerFlipOut(super::super::motion_finger_flip::State),
    AirDismounting(super::super::motion_air_dismounting::State),
    ///82BB5930 initializes byte8 once per allocated behavior instance.
    JumpInto { first_update: bool },
    MovingObject(super::super::motion_moving_object::Coefficients),
    EnterSkitching(super::super::motion_skitching::Enter),
    Skitching(super::super::motion_skitching::Holding),
    SkitchShimmying(f32),
}
impl Instance {
    pub(super) fn new(operation: &MotionOperation) -> Self {
        match operation {
            MotionOperation::Grind(_) => Self::Grind(Default::default()),
            MotionOperation::Trick(_) => Self::Trick(0),

            MotionOperation::ToggleBoard => Self::ToggleBoard(Default::default()),
            MotionOperation::Runout(_) => Self::Runout(Default::default()),
            MotionOperation::IntentFilter(_) => Self::IntentFilter(Default::default()),
            MotionOperation::Landing(_) => Self::Landing(Default::default()),
            MotionOperation::Wipeout(_) => Self::Wipeout(Default::default()),
            MotionOperation::TwistLean(_) => Self::TwistLean(Default::default()),
            MotionOperation::Play(_) => Self::Play(PlayAnimationInstance::default()),
            MotionOperation::Push(operation) => Self::Push(PushInstance::new(operation.clone())),
            MotionOperation::SetTurning(_) => Self::Turning(set_turning::State {
                elapsed: 0.0,
                smoothed: 0.0,
                mode: 2,
            }),
            MotionOperation::Crouching(_) => Self::Crouching(None),
            MotionOperation::SettingBodyTilt(_) => {
                Self::BodyTilt(skate3_core::animation::body_tilt::State::default())
            }
            MotionOperation::UpdateRidingFakie(_) => {
                Self::RidingFakie(skate3_core::animation::riding_fakie::State::default())
            }
            MotionOperation::Pumping => {
                Self::Pumping(skate3_core::animation::pumping_channel::State::default())
            }
            MotionOperation::BipedCadence => Self::Stateless,
            MotionOperation::MatchCadence => Self::MatchCadence(Default::default()),
            MotionOperation::OffboardBodyTweakBlend(_) => Self::OffboardBodyTweak(Default::default()),
            MotionOperation::StockGameplay(super::super::motion_stock_gameplay::Operation::MatchAirTime) =>
                Self::MatchAirTime(Default::default()),
            MotionOperation::FakieHeadChannel => {
                Self::FakieHead(super::super::motion_channels::FakieHead::default())
            }
            MotionOperation::Native(super::super::motion_native::Operation::StoreLandingData) => {
                Self::LandingData(super::super::motion_native::LandingData::default())
            }
            MotionOperation::Slide(super::super::motion_sliding::Operation::Update) => {
                Self::Sliding(super::super::motion_sliding::State::default())
            }
            MotionOperation::Slide(super::super::motion_sliding::Operation::Deceleration) => {
                Self::SlideDeceleration(0.0)
            }
            MotionOperation::KickTurn(super::super::motion_kickturn::Operation::Steering(_)) => {
                Self::KickTurn(skate3_core::animation::kickturn::State::new())
            }
            MotionOperation::BodySpin => Self::BodySpin(super::super::motion_spin::State::default()),
            MotionOperation::AirLeg(_) => Self::AirLeg(Default::default()),
            MotionOperation::CharacterGesture => {
                Self::CharacterGesture(super::super::motion_character_gesture::CharacterGesture::new())
            }
            MotionOperation::Shove(_) => Self::Shove(super::super::motion_shove::ShoveState::new()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::SetManualAngle { .. },
            ) => Self::SetManualAngle(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::HippyJumpAntic,
            ) => Self::HippyJumpAntic(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::FingerFlipOut { .. },
            ) => Self::FingerFlipOut(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::JumpInto { .. },
            ) => Self::JumpInto { first_update: true },
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::AirDismounting,
            ) => Self::AirDismounting(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::MovingObject(_),
            ) => Self::MovingObject(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::EnterSkitchingBehaviour,
            ) => Self::EnterSkitching(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::SkitchingBehaviour,
            ) => Self::Skitching(Default::default()),
            MotionOperation::StockGameplay(
                super::super::motion_stock_gameplay::Operation::SkitchShimmyingBehaviour { .. },
            ) => Self::SkitchShimmying(0.0),
            MotionOperation::ResetAnimation(_) => Self::Stateless,
            _ => Self::Stateless,
        }
    }
}
