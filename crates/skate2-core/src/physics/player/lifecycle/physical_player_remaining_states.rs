//! Remaining 22 actual `HiLOD` state constructors, mounted as an assembly child.
//! Typed optional cells preserve unwritten `PhysicsSkater` allocation storage.
#![expect(
    dead_code,
    reason = "retained state constructors precede original state update consumers"
)]
use super::PhysicalPlayerAssemblyOwner;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_footplant::RetainedTrajectory,
    physical_player_grab_spline::GrabSpline,
    physical_player_physics_state::PhysicalStateBindings,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};
#[path = "physical_player_remaining_states_fields.rs"]
mod fields;
#[path = "../air/physical_player_known_air_rotation.rs"]
pub(crate) mod known_air_rotation;
#[path = "../air/physical_player_known_air_trajectory.rs"]
pub(crate) mod known_air_trajectory;
pub(crate) use fields::*;
#[path = "physical_player_remaining_states_pushing.rs"]
mod pushing;
#[path = "../board/physical_player_pushing_geometry.rs"]
mod pushing_geometry;
#[path = "../board/physical_player_pushing_move.rs"]
mod pushing_move;
pub(crate) use pushing_move::MoveControls;
#[path = "../board/physical_player_pushing_position.rs"]
mod pushing_position;
#[path = "physical_player_remaining_states_simple.rs"]
mod simple;
#[path = "physical_player_remaining_states_skitch.rs"]
mod skitch;
#[path = "physical_player_skitch_deck_offset.rs"]
mod skitch_deck_offset;
#[path = "physical_player_skitch_entry.rs"]
pub(crate) mod skitch_entry;
#[path = "physical_player_skitch_follow.rs"]
mod skitch_follow;
#[path = "physical_player_skitch_forces.rs"]
mod skitch_forces;
#[path = "physical_player_skitch_geometry.rs"]
mod skitch_geometry;
#[path = "physical_player_skitch_hands.rs"]
mod skitch_hands;
#[path = "physical_player_skitch_ik.rs"]
pub(crate) mod skitch_ik;
#[path = "physical_player_skitch_state.rs"]
pub(crate) mod skitch_state;
#[path = "physical_player_remaining_states_teleporting.rs"]
mod teleporting;
pub(crate) use teleporting::{TeleportingUpdate, TeleportingUpdateError};
type Vector = [f32; 4];
type Matrix = [Vector; 4];
const ZERO: Vector = [0.0; 4];
const UP: Vector = [0.0, 1.0, 0.0, 0.0];
const IDENTITY: Matrix = [[1.0, 0.0, 0.0, 0.0], UP, [0.0, 0.0, 1.0, 0.0], ZERO];
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrabDescriptor {
    pub(crate) object_type: i32,
    pub(crate) object_id: i32,
}
impl GrabDescriptor {
    const INVALID: Self = Self {
        object_type: 0,
        object_id: 0,
    };
}
pub(crate) struct JumpInfo {
    pub(crate) velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) com_height: f32,
    pub(crate) do_jump: bool,
}
pub(crate) struct OffboardTrajectoryResult {
    pub(crate) com_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) com_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) landing_normal: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) landing_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) landing_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) time_to_land: f32,
    pub(crate) frame: i32,
    pub(crate) landing_valid: bool,
}
/// References into this same retained `Skitch` allocation; no copied matrix masquerades as a pointer.
#[derive(Clone, Copy)]
pub(crate) enum MatrixSlot {
    Future,
    Current,
}
pub(crate) struct VectorPid {
    pub(crate) control: [f32; 4],
    pub(crate) output: Vector,
    pub(crate) last_error: Vector,
    pub(crate) integral: Vector,
    pub(crate) derivative: Vector,
}
pub(crate) struct ScalarPid {
    pub(crate) control: [f32; 4],
    pub(crate) output: f32,
    pub(crate) last_error: f32,
    pub(crate) integral: f32,
    pub(crate) derivative: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OwnerMember {
    Reckoning,
    Wipeout,
    TrajectorySelector,
    GrindAirAdjust,
    FootPlantManager,
    HandPlantManager,
    PlayerGrabSpline,
    OffboardTrajectorySelector,
    SkateboardController,
    OffBoardFeetIK,
    LandingOnDeckManager,
    Pumping,
    OffboardLocationTest,
    OffboardJumpingTest,
    SkeletonController,
    GrindJumper,
    PhysOutConditioners,
}
#[derive(Clone)]
pub(crate) struct StateOwnerLink {
    pub(crate) owner: Weak<PhysicalPlayerAssemblyOwner>,
    pub(crate) member: OwnerMember,
}
#[derive(Clone)]
pub(crate) struct StateConstructorBindings {
    pub(crate) physical: PhysicalStateBindings,
    pub(crate) processed: Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) systems: BTreeMap<OwnerMember, Option<StateOwnerLink>>,
}

impl StatePayload {
    pub(crate) fn base(&self) -> &StateConstructorBindings {
        match self {
            Self::SlideGround(state) => &state.base,
            Self::RevertGround(state) => &state.base,
            Self::GroundAnimation(state) => &state.base,
            Self::KnownAir(state) => &state.base,
            Self::GrindBoardslide(state) => &state.common.base,
            Self::Grind5050(state)
            | Self::GrindTipslide(state)
            | Self::Grind5O(state)
            | Self::GrindBackslash(state) => &state.base,
            Self::Nonspecific(state) => &state.base,
            Self::Teleporting(state) => &state.base,
            Self::FollowPath(state) => &state.base,
            Self::Skitching(state) => &state.base,
            Self::HandPlant(state) => &state.base,
            Self::FootPlant(state) => &state.base,
            Self::Boneless(state) => &state.base,
            Self::OffBoard(state) => &state.base,
            Self::OffBoardLocomotion(state) => &state.base,
            Self::OffBoardPushing(state) => &state.base,
            Self::LandingOnDeck(state) => &state.base,
            Self::OffBoardAir(state) => &state.base,
            Self::OffBoardJump(state) => &state.base,
        }
    }
}
#[cfg(test)]
#[expect(
    clippy::too_many_lines,
    reason = "one original pair verifies the interleaved state constructor owners"
)]
pub(crate) fn check_original(states: &RemainingStates, owner: &Arc<PhysicalPlayerAssemblyOwner>) {
    assert!(states.is_complete());
    for kind in StateKind::ORDER {
        let retained = &states.states[&kind];
        assert!(retained.published);
        let base = retained
            .value
            .as_ref()
            .expect("original state allocation")
            .base();
        assert!(Weak::ptr_eq(&base.physical.owner, &Arc::downgrade(owner)));
        assert!(Arc::ptr_eq(
            &base.processed,
            owner.reckoning.lock().unwrap().processed()
        ));
        for link in base.systems.values().flatten() {
            assert!(Weak::ptr_eq(&link.owner, &Arc::downgrade(owner)));
        }
    }
    let StatePayload::SlideGround(slide) = states.states[&StateKind::SlideGround]
        .value
        .as_ref()
        .unwrap()
    else {
        unreachable!()
    };
    assert!(slide.push_scalar.is_none() && slide.damped_turn.is_none());
    let StatePayload::KnownAir(air) = states.states[&StateKind::KnownAir].value.as_ref().unwrap()
    else {
        unreachable!()
    };
    assert!(
        air.landing_heading.is_none()
            && air.coll_speed.is_none()
            && air.start_y.is_none()
            && air.com_max_y.is_none()
    );
    assert_eq!(
        air.landing_normal
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        UP.map(f32::to_bits)
    );
    let StatePayload::HandPlant(hand) =
        states.states[&StateKind::HandPlant].value.as_ref().unwrap()
    else {
        unreachable!()
    };
    assert!(hand.grab_spline_query_manager.is_none() && hand.hand_plant_state.is_none());
    let StatePayload::Skitching(skitch) =
        states.states[&StateKind::Skitching].value.as_ref().unwrap()
    else {
        unreachable!()
    };
    assert!(!skitch.attribs.as_ref().unwrap().is_null());
    assert!(
        skitch.grab_spline_to_ik_hand_space_transform.is_none()
            && skitch.apply_side_force.is_none()
            && skitch.grab_spline_centrifugal_acceleration_sign.is_none()
    );
    assert_eq!(skitch.first_eval, Some(true));
    skitch_state::check_original_release(skitch);
    skitch_geometry::check_original_aim_space(skitch);
    assert!(skitch.grab_spline.as_ref().unwrap().length_unit.is_none());
    assert_eq!(
        skitch.delay_until_hands_off_matters.unwrap().to_bits(),
        read_floats::<1>(skitch.attribs.as_ref().unwrap(), 0xcc25_7f85_6577_59a1).unwrap()[0]
            .to_bits()
    );
    let StatePayload::OffBoardPushing(push) = states.states[&StateKind::OffBoardPushing]
        .value
        .as_ref()
        .unwrap()
    else {
        unreachable!()
    };
    assert!(!push.attribs.as_ref().unwrap().is_null());
    assert!(
        push.target_push_normal.is_none()
            && push.last_push_normal.is_none()
            && push.target_x_direction.is_none()
    );
    assert_eq!(
        push.pid_object_vel
            .as_ref()
            .unwrap()
            .control
            .map(f32::to_bits),
        read_floats::<4>(push.attribs.as_ref().unwrap(), 0xdf79_539d_bda0_06ee)
            .unwrap()
            .map(f32::to_bits)
    );
    assert_eq!(
        push.pid_object_angular_vel
            .as_ref()
            .unwrap()
            .control
            .map(f32::to_bits),
        read_floats::<4>(push.attribs.as_ref().unwrap(), 0xb467_6428_5ad1_dc5f)
            .unwrap()
            .map(f32::to_bits)
    );
    assert!(push.grab_spline.as_ref().unwrap().length_unit.is_none());
    assert_eq!(
        push.spline_blend_coeff.unwrap().to_bits(),
        (-1.0_f32).to_bits()
    );
    assert_eq!(push.anim_crouch.unwrap().to_bits(), 0.5_f32.to_bits());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StateKind {
    SlideGround,
    RevertGround,
    GroundAnimation,
    KnownAir,
    GrindBoardslide,
    Grind5050,
    GrindTipslide,
    Grind5O,
    GrindBackslash,
    Nonspecific,
    Teleporting,
    FollowPath,
    Skitching,
    HandPlant,
    FootPlant,
    Boneless,
    OffBoard,
    OffBoardLocomotion,
    OffBoardPushing,
    LandingOnDeck,
    OffBoardAir,
    OffBoardJump,
}
impl StateKind {
    /// The retained payload of each grind type.
    pub(crate) const fn grind(kind: crate::physical_player_physics_state::GrindKind) -> Self {
        use crate::physical_player_physics_state::GrindKind as G;
        match kind {
            G::Boardslide => Self::GrindBoardslide,
            G::FiftyFifty => Self::Grind5050,
            G::Tipslide => Self::GrindTipslide,
            G::FiveO => Self::Grind5O,
            G::Backslash => Self::GrindBackslash,
        }
    }
    pub(crate) const ORDER: [Self; 22] = [
        Self::SlideGround,
        Self::RevertGround,
        Self::GroundAnimation,
        Self::KnownAir,
        Self::GrindBoardslide,
        Self::Grind5050,
        Self::GrindTipslide,
        Self::Grind5O,
        Self::GrindBackslash,
        Self::Nonspecific,
        Self::Teleporting,
        Self::FollowPath,
        Self::Skitching,
        Self::HandPlant,
        Self::FootPlant,
        Self::Boneless,
        Self::OffBoard,
        Self::OffBoardLocomotion,
        Self::OffBoardPushing,
        Self::LandingOnDeck,
        Self::OffBoardAir,
        Self::OffBoardJump,
    ];
    pub(crate) const fn allocation_size(self) -> usize {
        match self {
            Self::SlideGround => 48,
            Self::RevertGround | Self::GroundAnimation | Self::Boneless => 96,
            Self::KnownAir => 224,
            Self::GrindBoardslide => 240,
            Self::Grind5050 | Self::GrindTipslide | Self::Grind5O | Self::GrindBackslash => 208,
            Self::Nonspecific => 64,
            Self::Teleporting => 112,
            Self::FollowPath => 32,
            Self::Skitching => 1312,
            Self::HandPlant => 80,
            Self::FootPlant => 52,
            Self::OffBoard => 144,
            Self::OffBoardLocomotion | Self::LandingOnDeck => 92,
            Self::OffBoardPushing => 1136,
            Self::OffBoardAir => 192,
            Self::OffBoardJump => 176,
        }
    }
    pub(crate) const fn source_call(self) -> u32 {
        match self {
            Self::SlideGround => 0x82e0_45bc,
            Self::RevertGround => 0x82e0_4610,
            Self::GroundAnimation => 0x82e0_464c,
            Self::KnownAir => 0x82e0_46c4,
            Self::GrindBoardslide => 0x82e0_4704,
            Self::Grind5050 => 0x82e0_4754,
            Self::GrindTipslide => 0x82e0_47a4,
            Self::Grind5O => 0x82e0_47f4,
            Self::GrindBackslash => 0x82e0_4844,
            Self::Nonspecific => 0x82e0_48d0,
            Self::Teleporting => 0x82e0_4928,
            Self::FollowPath => 0x82e0_4968,
            Self::Skitching => 0x82e0_49b4,
            Self::HandPlant => 0x82e0_49f0,
            Self::FootPlant => 0x82e0_4a2c,
            Self::Boneless => 0x82e0_4a68,
            Self::OffBoard => 0x82e0_4aa4,
            Self::OffBoardLocomotion => 0x82e0_4ae0,
            Self::OffBoardPushing => 0x82e0_4b1c,
            Self::LandingOnDeck => 0x82e0_4b58,
            Self::OffBoardAir => 0x82e0_4bd4,
            Self::OffBoardJump => 0x82e0_4c10,
        }
    }
}
pub(crate) enum StatePayload {
    SlideGround(Box<SlideGround>),
    RevertGround(Box<RevertGround>),
    GroundAnimation(Box<GroundAnimation>),
    KnownAir(Box<KnownAir>),
    GrindBoardslide(Box<GrindBoardslide>),
    Grind5050(Box<GrindCommon>),
    GrindTipslide(Box<GrindCommon>),
    Grind5O(Box<GrindCommon>),
    GrindBackslash(Box<GrindCommon>),
    Nonspecific(Box<Nonspecific>),
    Teleporting(Box<Teleporting>),
    FollowPath(Box<FollowPath>),
    Skitching(Box<Skitching>),
    HandPlant(Box<HandPlant>),
    FootPlant(Box<FootPlant>),
    Boneless(Box<Boneless>),
    OffBoard(Box<OffBoard>),
    OffBoardLocomotion(Box<OffBoardLocomotion>),
    OffBoardPushing(Box<OffBoardPushing>),
    LandingOnDeck(Box<LandingOnDeck>),
    OffBoardAir(Box<OffBoardAir>),
    OffBoardJump(Box<OffBoardJump>),
}
pub(crate) struct RetainedState {
    pub(crate) value: Option<StatePayload>,
    pub(crate) published: bool,
}
#[derive(Default)]
pub(crate) struct RemainingStates {
    pub(crate) states: BTreeMap<StateKind, RetainedState>,
    next: usize,
}
impl RemainingStates {
    /// The pair bindings of a published state allocation.
    pub(crate) fn published_bindings(&self, kind: StateKind) -> Option<&PhysicalStateBindings> {
        self.states
            .get(&kind)
            .filter(|state| state.published)
            .and_then(|state| state.value.as_ref())
            .map(|value| &value.base().physical)
    }
    /// Root interleaves the four existing state owners at their native callsites.
    /// A failed constructor keeps its allocated prefix and cannot be replayed.
    #[expect(
        clippy::too_many_lines,
        reason = "native state-specific construction dispatch preserves ordered failure prefixes"
    )]
    pub(crate) fn try_construct(
        &mut self,
        kind: StateKind,
        bindings: StateConstructorBindings,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        allocated: bool,
    ) -> Result<(), StateConstructorError> {
        if self.states.contains_key(&kind) {
            return Err(StateConstructorError::AlreadyAttempted);
        }
        if StateKind::ORDER.get(self.next) != Some(&kind) {
            return Err(StateConstructorError::WrongOrder);
        }
        let value = if allocated {
            Some(match kind {
                StateKind::SlideGround => {
                    StatePayload::SlideGround(Box::new(SlideGround::before_constructor(bindings)))
                }
                StateKind::RevertGround => {
                    StatePayload::RevertGround(Box::new(RevertGround::before_constructor(bindings)))
                }
                StateKind::GroundAnimation => StatePayload::GroundAnimation(Box::new(
                    GroundAnimation::before_constructor(bindings),
                )),
                StateKind::KnownAir => {
                    StatePayload::KnownAir(Box::new(KnownAir::before_constructor(bindings)))
                }
                StateKind::GrindBoardslide => StatePayload::GrindBoardslide(Box::new(
                    GrindBoardslide::before_constructor(bindings),
                )),
                StateKind::Grind5050 => {
                    StatePayload::Grind5050(Box::new(GrindCommon::before_constructor(bindings)))
                }
                StateKind::GrindTipslide => {
                    StatePayload::GrindTipslide(Box::new(GrindCommon::before_constructor(bindings)))
                }
                StateKind::Grind5O => {
                    StatePayload::Grind5O(Box::new(GrindCommon::before_constructor(bindings)))
                }
                StateKind::GrindBackslash => StatePayload::GrindBackslash(Box::new(
                    GrindCommon::before_constructor(bindings),
                )),
                StateKind::Nonspecific => {
                    StatePayload::Nonspecific(Box::new(Nonspecific::before_constructor(bindings)))
                }
                StateKind::Teleporting => {
                    StatePayload::Teleporting(Box::new(Teleporting::before_constructor(bindings)))
                }
                StateKind::FollowPath => {
                    StatePayload::FollowPath(Box::new(FollowPath::before_constructor(bindings)))
                }
                StateKind::Skitching => {
                    StatePayload::Skitching(Box::new(Skitching::before_constructor(bindings)))
                }
                StateKind::HandPlant => {
                    StatePayload::HandPlant(Box::new(HandPlant::before_constructor(bindings)))
                }
                StateKind::FootPlant => {
                    StatePayload::FootPlant(Box::new(FootPlant::before_constructor(bindings)))
                }
                StateKind::Boneless => {
                    StatePayload::Boneless(Box::new(Boneless::before_constructor(bindings)))
                }
                StateKind::OffBoard => {
                    StatePayload::OffBoard(Box::new(OffBoard::before_constructor(bindings)))
                }
                StateKind::OffBoardLocomotion => StatePayload::OffBoardLocomotion(Box::new(
                    OffBoardLocomotion::before_constructor(bindings),
                )),
                StateKind::OffBoardPushing => StatePayload::OffBoardPushing(Box::new(
                    OffBoardPushing::before_constructor(bindings),
                )),
                StateKind::LandingOnDeck => StatePayload::LandingOnDeck(Box::new(
                    LandingOnDeck::before_constructor(bindings),
                )),
                StateKind::OffBoardAir => {
                    StatePayload::OffBoardAir(Box::new(OffBoardAir::before_constructor(bindings)))
                }
                StateKind::OffBoardJump => {
                    StatePayload::OffBoardJump(Box::new(OffBoardJump::before_constructor(bindings)))
                }
            })
        } else {
            None
        };
        self.states.insert(
            kind,
            RetainedState {
                value,
                published: false,
            },
        );
        let retained = self.states.get_mut(&kind).expect("inserted allocation");
        if let Some(value) = &mut retained.value {
            match value {
                StatePayload::SlideGround(value) => value.construct(vault),
                StatePayload::RevertGround(value) => value.construct(vault),
                StatePayload::GroundAnimation(value) => value.construct(vault),
                StatePayload::KnownAir(value) => value.construct(vault),
                StatePayload::GrindBoardslide(value) => value.construct(vault),
                StatePayload::Grind5050(value)
                | StatePayload::GrindTipslide(value)
                | StatePayload::Grind5O(value)
                | StatePayload::GrindBackslash(value) => value.construct(vault),
                StatePayload::Nonspecific(value) => value.construct(vault),
                StatePayload::Teleporting(value) => value.construct(vault),
                StatePayload::FollowPath(value) => value.construct(vault),
                StatePayload::Skitching(value) => value.construct(vault)?,
                StatePayload::HandPlant(value) => value.construct(vault),
                StatePayload::FootPlant(value) => value.construct(vault),
                StatePayload::Boneless(value) => value.construct(vault),
                StatePayload::OffBoard(value) => value.construct(vault),
                StatePayload::OffBoardLocomotion(value) => value.construct(vault),
                StatePayload::OffBoardPushing(value) => value.construct(vault, cvars)?,
                StatePayload::LandingOnDeck(value) => value.construct(vault),
                StatePayload::OffBoardAir(value) => value.construct(vault),
                StatePayload::OffBoardJump(value) => value.construct(vault),
            }
        }
        retained.published = true;
        self.next += 1;
        Ok(())
    }
    pub(crate) fn is_complete(&self) -> bool {
        self.next == StateKind::ORDER.len()
    }
}

impl StateConstructorBindings {
    /// Snapshot each actual `PhysicsSystems` pointer once; NULL remains NULL after later publication.
    #[expect(
        clippy::too_many_lines,
        reason = "ordered PhysicsSystems pointer capture retains each actual owner"
    )]
    pub(crate) fn capture(physical: PhysicalStateBindings) -> Result<Self, StateConstructorError> {
        let owner = physical
            .owner
            .upgrade()
            .ok_or(StateConstructorError::OwnerExpired)?;
        let processed = Arc::clone(
            owner
                .reckoning
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .processed(),
        );
        let mut systems = BTreeMap::new();
        let mut set = |member: OwnerMember, present: bool| {
            systems.insert(
                member,
                present.then(|| StateOwnerLink {
                    owner: Arc::downgrade(&owner),
                    member,
                }),
            );
        };
        set(
            OwnerMember::Reckoning,
            owner
                .reckoning
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .reckoning()
                .is_some(),
        );
        set(
            OwnerMember::Wipeout,
            owner
                .wipeout
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .phase()
                == crate::physical_player_wipeout::WipeoutOwnerPhase::Constructed,
        );
        {
            let trajectories = owner
                .trajectories
                .lock()
                .map_err(|_| StateConstructorError::Lock)?;
            set(
                OwnerMember::TrajectorySelector,
                trajectories.onboard.is_some(),
            );
            set(
                OwnerMember::OffboardTrajectorySelector,
                trajectories.offboard.is_some(),
            );
        }
        set(
            OwnerMember::GrindAirAdjust,
            owner
                .grind_air
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .is_some(),
        );
        set(
            OwnerMember::FootPlantManager,
            owner
                .footplant
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .is_constructed(),
        );
        set(
            OwnerMember::HandPlantManager,
            owner
                .handplant
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .is_some(),
        );
        set(
            OwnerMember::PlayerGrabSpline,
            owner
                .grab_spline
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .as_ref()
                .is_some_and(|value| value.returned),
        );
        {
            let remaining = owner
                .remaining
                .lock()
                .map_err(|_| StateConstructorError::Lock)?;
            set(OwnerMember::OffBoardFeetIK, remaining.feet_ik.is_some());
            set(
                OwnerMember::LandingOnDeckManager,
                remaining.landing.is_some(),
            );
            set(
                OwnerMember::OffboardLocationTest,
                remaining.location.is_some(),
            );
            set(
                OwnerMember::OffboardJumpingTest,
                remaining.jumping.is_some(),
            );
        }
        set(
            OwnerMember::Pumping,
            owner
                .pumping
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .phase()
                == crate::physical_player_pumping::PumpingPhase::Constructed,
        );
        // Host inline storage does not establish native constructor publication.
        set(
            OwnerMember::SkateboardController,
            owner
                .skateboard_controller
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .phase()
                == crate::skateboard_controller::SkateboardControllerPhase::Complete,
        );
        set(
            OwnerMember::SkeletonController,
            owner
                .skeleton_controller
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .current()
                .is_some(),
        );
        {
            let grind = owner
                .grind
                .lock()
                .map_err(|_| StateConstructorError::Lock)?;
            let grind = grind
                .as_ref()
                .ok_or(StateConstructorError::MissingGrindManager)?;
            set(OwnerMember::GrindJumper, grind.jumper.is_some());
        }
        set(
            OwnerMember::PhysOutConditioners,
            owner
                .conditioners
                .lock()
                .map_err(|_| StateConstructorError::Lock)?
                .as_ref()
                .is_some_and(
                    crate::physical_player_phys_out::conditioners::Conditioners::is_complete,
                ),
        );
        Ok(Self {
            physical,
            processed,
            systems,
        })
    }
    fn link(&self, member: OwnerMember) -> Option<StateOwnerLink> {
        self.systems[&member].clone()
    }
}
#[derive(Debug)]
pub(crate) enum StateConstructorError {
    Lock,
    OwnerExpired,
    MissingGrindManager,
    AlreadyAttempted,
    WrongOrder,
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    AttributeSize {
        key: u64,
        needed: usize,
        actual: usize,
    },
}
impl From<AttributeRuntimeError> for StateConstructorError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Attribute(value)
    }
}
impl From<InputCvarError> for StateConstructorError {
    fn from(value: InputCvarError) -> Self {
        Self::Cvar(value)
    }
}
impl std::fmt::Display for StateConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "state constructor: {self:?}")
    }
}
pub(crate) fn read_floats<const N: usize>(
    attributes: &AttributeRuntimeInstance,
    key: u64,
) -> Result<[f32; N], StateConstructorError> {
    let Some(data) = attributes.try_get_data(key, 0)? else {
        return Ok([0.0; N]);
    };
    if data.bytes().len() < N * 4 {
        return Err(StateConstructorError::AttributeSize {
            key,
            needed: N * 4,
            actual: data.bytes().len(),
        });
    }
    Ok(std::array::from_fn(|i| {
        f32::from_bits(u32::from_be_bytes(
            data.bytes()[i * 4..i * 4 + 4]
                .try_into()
                .expect("checked field"),
        ))
    }))
}
