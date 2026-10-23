//! Retained generated Motion controller constructor8292a1b0.
use super::{
    GeneratedMotionLoad, GeneratedMotionStatics, MotionAllocation, MotionLoadError,
    construct::MotionBehaviour,
};
use crate::motion_graph_entity_authority::{
    BoundMotionGraphContext, MotionGraphControllerToken, MotionGraphEntityAuthority,
};
use crate::state_graph_context::StateGraphContextHandle;
use crate::{
    generated_motion_load::construct::ToggleBoardNames,
    play_animation::{EMPTY_FAST_STRING_36, FastString36},
};
use std::{collections::BTreeMap, sync::Arc};

/// Condition cache size: one result and one active bit per declaration.
const CONDITIONS: usize = super::data::CONDITIONS.len();

/// ToggleBoard::Data, allocated by CreateInstance82cbb1c0 (0x64 bytes).
/// The constructor at 82cbb0a8 only initializes the retrieval-name words;
/// the other fields have no value until the source writes them.
pub(crate) struct ToggleBoardInstance {
    pub(crate) state: Option<u32>,           // native +0x08
    pub(crate) yaw: Option<f32>,             // native +0x0c
    pub(crate) pitch: Option<f32>,           // native +0x10
    pub(crate) signed_yaw: Option<f32>,      // native +0x14
    pub(crate) retrieve: [FastString36; 3],  // native +0x18..+0x5f
    pub(crate) yaw_cached: Option<bool>,     // native +0x60 bit 7
    pub(crate) throw_selected: Option<bool>, // native +0x60 bit 6
}

impl ToggleBoardInstance {
    pub(crate) const fn after_allocation() -> Self {
        Self {
            state: None,
            yaw: None,
            pitch: None,
            signed_yaw: None,
            retrieve: [EMPTY_FAST_STRING_36; 3],
            yaw_cached: None,
            throw_selected: None,
        }
    }

    /// ToggleBoard::Begin82cbb098 -> ResetYawPitch82caad80.
    pub(crate) fn begin(&mut self, names: &ToggleBoardNames) {
        self.state = Some(0);
        self.reset_yaw_pitch(names);
    }

    /// ResetYawPitch82caad80 leaves the state and throw-selection bit intact.
    pub(crate) fn reset_yaw_pitch(&mut self, names: &ToggleBoardNames) {
        self.yaw = Some(0.0);
        self.pitch = Some(0.0);
        self.signed_yaw = Some(0.0);
        self.yaw_cached = Some(false);
        self.retrieve = names.retrieve_front;
    }
}

/// Source root and Default instance allocations; unwritten fields stay absent.
pub(crate) enum MotionBehaviourInstance {
    SkitchShimmying {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        speed: f32,
    },
    EnterSkitching {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::skitching::kernel::EnterInstance,
    },
    Skitching {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::skitching::kernel::HoldingInstance,
    },
    /// MovingObject.CreateInstance82cb3948: allocation0x30, coefficient block+8.
    MovingObject {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_animation_construction::moving_objects::MovingObjectsCoefficients,
    },
    HippyJumpAntic { token: crate::motion_graph_entity_authority::MotionGraphInstanceToken, time: f32 },
    AirDismounting { token: crate::motion_graph_entity_authority::MotionGraphInstanceToken, frames: i32 },
    SetManualAngle { token: crate::motion_graph_entity_authority::MotionGraphInstanceToken, angle: f32, velocity: f32 },
    JumpInto { token: crate::motion_graph_entity_authority::MotionGraphInstanceToken, first_update: bool },
    LandingOffBoardAir {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::landing_offboard_air::LandingOffBoardAirInstance,
    },
    /// Create827126f8 (0x0c bytes): byte +8, cleared; Begin82c95bf8 sets it.
    StandingJumpDescent {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        begun: bool,
    },
    ToggleBoard {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: ToggleBoardInstance,
    },
    SetPushCoefs {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::set_push_coefs::SetPushCoefsInstance,
    },
    FirstPush {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::compute_first_push::FirstPushInstance,
    },
    PushCycle {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::push_cycle::PushCycleInstance,
    },
    DisableTricks { token: crate::motion_graph_entity_authority::MotionGraphInstanceToken, data: crate::actor_construction::motion_runtime_session::landing_callbacks::timer::Instance },
    AirLeg {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::air_leg::kernel::AirLegInstance,
    },
    Wipeout {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::wipeout::kernel::WipeoutInstance,
    },
    Pumping {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::pumping::kernel::PumpingInstance,
    },
    Crouching {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::crouching::kernel::CrouchingInstance,
    },
    SetTurning {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::set_turning::kernel::TurningInstance,
    },
    KickTurnSteering {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::motion_kickturn_steering::KickTurnSteeringInstance,
    },
    RidingFakie {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        slow_time: f32,
        teleport_time: f32,
    },
    /// FilterMotionGraphIntent CreateInstance allocates 0x14 and zeros +8/+c/+10.
    FilterMotionGraphIntent {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: [f32; 3],
    },
    FakieHead {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        torso: f32,
        active: bool,
    },
    PlayAnimation {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::play_animation_lifecycle::PlayAnimationRuntimeInstance,
    },
    SettingBodyTilt {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::setting_body_tilt::kernel::TiltInstance,
    },
    PowerSliding {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::power_sliding::kernel::SlidingInstance,
    },
    PowerSlideDecel {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        value: f32,
    },
    BodySpin {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::body_spin::kernel::BodySpinInstance,
    },
    StoreLandingData {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::store_landing_data::kernel::LandingInstance,
    },
    /// Create82712148 (0x0c bytes): float +8 zeroed; Begin82cbea20 zeroes it again.
    FingerFlipOut {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        timer: f32,
    },
    /// Create82712148 (0x0c bytes): float +8 zeroed; Begin82cb1d10 seeds it.
    ControlGrindCrouch {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::grinding::kernel::GrindCrouchInstance,
    },
    /// Create82cbce08 (0x24 bytes).
    GrindControlFade {
        token: crate::motion_graph_entity_authority::MotionGraphInstanceToken,
        data: crate::actor_construction::motion_runtime_session::grinding::kernel::GrindControlFadeInstance,
    },
}
impl MotionBehaviourInstance {
    pub(crate) fn token(&self) -> &crate::motion_graph_entity_authority::MotionGraphInstanceToken {
        match self {
            Self::MovingObject { token, .. }
            | Self::EnterSkitching { token, .. }
            | Self::Skitching { token, .. }
            | Self::SkitchShimmying { token, .. }
            | Self::HippyJumpAntic { token, .. }
            | Self::AirDismounting { token, .. }
            | Self::SetManualAngle { token, .. }
            | Self::JumpInto { token, .. }
            | Self::LandingOffBoardAir { token, .. }
            | Self::StandingJumpDescent { token, .. }
            | Self::ToggleBoard { token, .. }
            | Self::SetPushCoefs { token, .. }
            | Self::FirstPush { token, .. }
            | Self::PushCycle { token, .. }
            | Self::AirLeg { token, .. }
            | Self::Wipeout { token, .. }
            | Self::RidingFakie { token, .. }
            | Self::FilterMotionGraphIntent { token, .. }
            | Self::Pumping { token, .. }
            | Self::Crouching { token, .. }
            | Self::SetTurning { token, .. }
            | Self::KickTurnSteering { token, .. }
            | Self::FakieHead { token, .. }
            | Self::PlayAnimation { token, .. }
            | Self::SettingBodyTilt { token, .. }
            | Self::PowerSliding { token, .. }
            | Self::PowerSlideDecel { token, .. }
            | Self::BodySpin { token, .. }
            | Self::StoreLandingData { token, .. }
            | Self::FingerFlipOut { token, .. }
            | Self::ControlGrindCrouch { token, .. }
            | Self::GrindControlFade { token, .. }
            | Self::DisableTricks { token, .. } => token,
        }
    }
}
pub(crate) struct BehaviourInstanceSlot {
    pub(crate) behaviour: Option<Arc<MotionBehaviour>>,
    pub(crate) instance: Option<MotionBehaviourInstance>,
}
pub(crate) struct GeneratedMotionController<K> {
    entity: MotionGraphEntityAuthority<K>,
    wrapper: MotionGraphControllerToken,
    pub(super) load: GeneratedMotionLoad,
    pub(crate) current_state: u16,
    pub(crate) last_state: u16,
    pub(crate) delta_time: f32,
    /// Source `fixed_map` has capacity11; mutation must enforce that limit.
    pub(crate) time_in_state: BTreeMap<u16, f32>,
    /// Source `IndexSet` has capacity40; mutation must enforce that limit.
    pub(crate) current_behaviours: Vec<u16>,
    pub(crate) instances: [BehaviourInstanceSlot; 40],
    pub(crate) condition_active: [u32; CONDITIONS.div_ceil(32)],
    /// Native cached result bits are unknown until the matching active bit.
    pub(crate) condition_results: [Option<bool>; CONDITIONS],
    pub(crate) call_behaviours: bool,
    pub(crate) allow_evaluations: bool,
}
impl<K> GeneratedMotionController<K> {
    pub(crate) fn try_load(
        context: &BoundMotionGraphContext<K>,
        statics: &GeneratedMotionStatics,
        call_behaviours: bool,
    ) -> Result<Self, MotionLoadError> {
        let entity = context
            .entity()
            .ok_or(MotionLoadError::ContextRequired)?
            .shallow_copy();
        let wrapper = context
            .controller()
            .ok_or(MotionLoadError::ContextRequired)?
            .shallow_copy();
        let mut current_behaviours = Vec::new();
        current_behaviours
            .try_reserve_exact(40)
            .map_err(|_| MotionLoadError::Allocation)?;
        let mut result = Self {
            entity,
            wrapper,
            load: GeneratedMotionLoad::new(statics),
            current_state: u16::MAX,
            last_state: u16::MAX,
            delta_time: 0.0,
            time_in_state: BTreeMap::new(),
            current_behaviours,
            instances: std::array::from_fn(|_| BehaviourInstanceSlot {
                behaviour: None,
                instance: None,
            }),
            condition_active: [0; CONDITIONS.div_ceil(32)],
            condition_results: [None; CONDITIONS],
            call_behaviours,
            allow_evaluations: false,
        };
        result
            .load
            .try_construct(&mut |_| Ok(MotionAllocation::Host))?;
        Ok(result)
    }
    pub(crate) fn validate_context(
        &self,
        context: &BoundMotionGraphContext<K>,
    ) -> Result<(), MotionLoadError> {
        if context.entity() != Some(&self.entity) || context.controller() != Some(&self.wrapper) {
            return Err(MotionLoadError::ForeignContext);
        }
        Ok(())
    }
    pub(crate) fn hook(
        &self,
        index: usize,
    ) -> Result<Option<&Arc<super::hooks::MotionHook>>, MotionLoadError> {
        self.load
            .hooks
            .get(index)
            .map(Option::as_ref)
            .ok_or(MotionLoadError::DescriptorBounds)
    }
    #[expect(
        clippy::unused_self,
        reason = "root descriptor belongs to this concrete generated controller interface"
    )]
    pub(crate) const fn root_state(&self) -> &'static super::StateDeclaration {
        &super::data::STATES[0]
    }
}

#[cfg(test)]
mod toggle_board_tests {
    use super::*;
    use crate::play_animation::FastString30;

    #[test]
    fn native_toggle_board_constructor_and_begin_keep_distinct_writes() {
        let names = ToggleBoardNames {
            retrieve: FastString36::encode("OB_RetrieveBoard").unwrap(),
            retrieving: FastString36::encode("OB_RetrievingBoard").unwrap(),
            dropping: FastString36::encode("OB_DroppingBoard").unwrap(),
            throw: FastString36::encode("OFB_THROW_0").unwrap(),
            drop: FastString36::encode("OFB_THROW_90").unwrap(),
            retrieve_front: [
                FastString36::encode("OFB_RETRIEVE_HIGH_FRONT_INTO").unwrap(),
                FastString36::encode("OFB_RETRIEVE_HIGH_FRONT_CYC").unwrap(),
                FastString36::encode("OFB_RETRIEVE_HIGH_FRONT_OUT").unwrap(),
            ],
            retrieve_back: [
                FastString36::encode("OFB_RETRIEVE_HIGH_BACK_INTO").unwrap(),
                FastString36::encode("OFB_RETRIEVE_HIGH_BACK_CYC").unwrap(),
                FastString36::encode("OFB_RETRIEVE_HIGH_BACK_OUT").unwrap(),
            ],
            yaw: FastString30::encode("yaw").unwrap(),
            pitch: FastString30::encode("pitch").unwrap(),
        };
        let mut data = ToggleBoardInstance::after_allocation();
        assert_eq!(data.state, None);
        assert_eq!(data.retrieve, [EMPTY_FAST_STRING_36; 3]);
        assert_eq!(data.yaw_cached, None);
        assert_eq!(data.throw_selected, None);

        data.begin(&names);
        assert_eq!(data.state, Some(0));
        assert_eq!(data.yaw, Some(0.0));
        assert_eq!(data.pitch, Some(0.0));
        assert_eq!(data.signed_yaw, Some(0.0));
        assert_eq!(data.retrieve, names.retrieve_front);
        assert_eq!(data.yaw_cached, Some(false));
        assert_eq!(data.throw_selected, None);

        data.state = Some(5);
        data.throw_selected = Some(true);
        data.yaw = Some(42.0);
        data.reset_yaw_pitch(&names);
        assert_eq!(data.state, Some(5));
        assert_eq!(data.throw_selected, Some(true));
        assert_eq!(data.yaw, Some(0.0));
    }
}
