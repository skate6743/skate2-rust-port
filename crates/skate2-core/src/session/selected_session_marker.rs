//! Production owners of `Sk8::SessionMarker`, `PlayerUI::UpdateSessionMarker`
//! and the teleport command they request, over the selected session's Actor,
//! World, terrain, cvars and input. Game-layer owners the port has no producer
//! for answer with the state their native constructors leave.
use crate::{
    actor_construction::{
        ActorConstruction,
        breadcrumbs::{
            ActorBreadCrumbMountError, ActorBreadCrumbOwnerError, BreadCrumbCollisionGateError,
            calc_respawn_transform::{CalcRespawnTransformError, try_calc_respawn_transform},
        },
        physical_mount::ActorPhysicalMountError,
        teleport::ActorTeleportError,
    },
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_phys_out::{PhysicalPlayerPhysOut, construct::ConcretePhysOutError},
    physics_state_ids::PHYSICS_STATE_TELEPORTING,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, try_length},
    selected_production_input_owner::{SelectedProductionInputOwner, SessionMarkerStateError},
    selected_session_boot_owner::terrain_input::SelectedSessionTerrainSlot,
    selected_session_terrain::SelectedSessionTerrainError,
    session_marker::{
        FootForward, LocalSkaterOutput, SECURITY_GUARD_AREAS, SessionMarker, SessionMarkerRules,
        player_ui::{
            PlayerUiSessionMarker, PlayerUiSessionMarkerSource, SessionMarkerCvar,
            SessionMarkerInput,
        },
        teleport::{SessionMarkerTeleportQueues, SkaterManagerTeleport, TeleportCommandError},
    },
    simulation_assembly_backend::constraint_session::world_services::{
        PhysicalGameInfo, PhysicalWorldServicesError,
    },
    skater_manager_adjust_spawn_position::{AdjustSpawnPositionError, try_adjust_spawn_position},
    skater_manager_local_identity::{LocalIdentityError, SkaterManagerLocalIdentity},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    static_ground_pool::StaticGroundPool,
    world_painter_layer::WorldPainterLayerError,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

/// `CommonBackEnd::GameMode` values read by `CreateSessionMarker`82729c28.
const GM_PARTY_PLAY: u32 = 2;
const GM_ONLINE_CHALLENGE: u32 = 5;
/// `Input::eInputStates` read by `UpdateSessionMarker`.
const IS_SESSION_MARKER_SET: u32 = 52;
const IS_SESSION_MARKER_USE: u32 = 53;

#[derive(Debug)]
pub(crate) enum SelectedSessionMarkerError {
    Physical(ActorPhysicalMountError),
    OutputLock,
    Output(ConcretePhysOutError<Infallible>),
    Identity(LocalIdentityError),
    /// A required actor disappeared after its presence check.
    MissingActor,
    GameInfo(PhysicalWorldServicesError),
    Cvar(InputCvarError),
    Input(SessionMarkerStateError),
    FootForward(ActorBreadCrumbMountError),
    Calc(CalcRespawnTransformError<ReckoningHostNumericError>),
    Numeric(ReckoningHostNumericError),
    Terrain(SelectedSessionTerrainError),
    LayerLock,
    Layer(WorldPainterLayerError),
    WorldLock,
    SimRateLock,
    SimRate(crate::simulation_sim_rate::SimRateError),
    LivingWorldLock,
    MissingChase,
    Breadcrumbs(crate::actor_construction::ActorConstructionError),
    MissingBreadcrumbs,
    BreadcrumbLock,
    BreadcrumbOwner(ActorBreadCrumbOwnerError),
    Collision(BreadCrumbCollisionGateError<ReckoningHostNumericError>),
    Skate3Breadcrumbs(crate::actor_construction::breadcrumbs::LiveBreadCrumbSelectionError),
    /// `GetLocalActor` returns an admitted ghost first; none is ported.
    PendingGhost,
    Observers(crate::skater_manager_local_identity::observers::SkaterObserverError),
    ChallengeQueue(crate::hermes_challenge_queue::ChallengeQueueError),
    /// `CreateSessionMarker` reads an OnlineChallenge's net challenge type, which was not given.
    PendingOnlineChallengeType,
    /// `TeleportRequiresLoadScreen` needs the unported `IsAreaLoaded`.
    PendingLoadScreenQuery,
    /// The security dismissal popup is front-end owned.
    PendingSecurityPopup,
}
impl std::fmt::Display for SelectedSessionMarkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "selected session marker: {self:?}")
    }
}

/// `GameModule+0x50`, the PlayerUI stage fields and the Flow/eEQGameSim
/// queues between them and the command stage.
pub(crate) struct SelectedSessionMarker {
    pub(crate) hud: crate::ui::phonelist::MarkerHudSnapshot,
    pub(crate) prop_actions: Vec<(u32,u8)>,
    pub(crate) restore_props: bool,
    pub(crate) marker: Option<SessionMarker>,
    pub(crate) player_ui: PlayerUiSessionMarker,
    pub(crate) teleport: SessionMarkerTeleportQueues,
}

impl SelectedSessionMarker {
    /// `GameModule::ResetGame`'s `CreateSessionMarker`82729c28: PartyPlay has
    /// none, an OnlineChallenge none for Own the Spot, S.K.A.T.E. and Death Race (its
    /// challenge's `GetNetChallengeType` 0, 5 or 6), and every other mode allocates one.
    pub(crate) fn try_construct(game_mode: u32, net_challenge_type: Option<u8>) -> Result<Self, SelectedSessionMarkerError> {
        let marker = match game_mode {
            GM_PARTY_PLAY => None,
            GM_ONLINE_CHALLENGE => match net_challenge_type {
                Some(0 | 5 | 6) => None,
                Some(_) => Some(SessionMarker::construct()),
                None => return Err(SelectedSessionMarkerError::PendingOnlineChallengeType),
            },
            _ => Some(SessionMarker::construct()),
        };
        Ok(Self {
            hud: Default::default(),
            prop_actions: Vec::new(),
            restore_props: false,
            marker,
            player_ui: PlayerUiSessionMarker::default(),
            teleport: SessionMarkerTeleportQueues::construct(),
        })
    }

    /// A new game's `GameModule::ResetGame` (`0x8273a680`): `CreateSessionMarker` for its mode.
    pub(crate) fn try_reset_game(&mut self, game_mode: u32, net_challenge_type: Option<u8>) -> Result<(), SelectedSessionMarkerError> {
        self.marker = Self::try_construct(game_mode, net_challenge_type)?.marker;
        Ok(())
    }
}

/// The selected owners read by `SessionMarker::Update` and the PlayerUI stage.
pub(crate) struct SelectedSessionMarkerSources<'a, C> {
    pub(crate) local: &'a SkaterManagerLocalIdentity,
    pub(crate) world: &'a Arc<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) terrain: &'a dyn SelectedSessionTerrainSlot,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) game_info: &'a PhysicalGameInfo,
    pub(crate) input: &'a SelectedProductionInputOwner<C>,
    /// The career's challenge system; other modes run no challenge.
    pub(crate) challenge: Option<crate::challenge::system::MarkerRules>,
}

impl<C> SelectedSessionMarkerSources<'_, C> {
    /// `Actor::GetPhysOut` (Actor+0x28 vtable +0x14), read under its lock.
    fn output<R>(
        actor: &ActorConstruction,
        read: impl FnOnce(&PhysicalPlayerPhysOut) -> Result<R, SelectedSessionMarkerError>,
    ) -> Result<R, SelectedSessionMarkerError> {
        use SelectedSessionMarkerError as E;
        let physical = actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::OutputLock)?;
        read(&*output)
    }

    /// SessionMarker uses GetLocal828ab060, independently of PlayerUI's index.
    fn local_actor(&self) -> Result<Arc<ActorConstruction>, SelectedSessionMarkerError> {
        self.local.try_local_actor()
            .map_err(SelectedSessionMarkerError::Identity)?
            .ok_or(SelectedSessionMarkerError::MissingActor)
    }

    /// PlayerUI's GetLocalActor828aa8b0: the ghost, then FindByIndex(mPlayerIndex).
    fn find_player(&self) -> Result<Option<Arc<ActorConstruction>>, SelectedSessionMarkerError> {
        use SelectedSessionMarkerError as E;
        if !self.local.try_ghost_is_null().map_err(E::Identity)? {
            return Err(E::PendingGhost);
        }
        let index = i32::from_ne_bytes(self.input.player_index().to_ne_bytes());
        self.local.try_find_by_index(index).map_err(E::Identity)
    }

    fn player_actor(&self) -> Result<Arc<ActorConstruction>, SelectedSessionMarkerError> {
        self.find_player()?.ok_or(SelectedSessionMarkerError::MissingActor)
    }
}

impl<C> SessionMarkerRules for SelectedSessionMarkerSources<'_, C> {
    type Error = SelectedSessionMarkerError;
    /// The port runs only the in-game session (`eFE_STATE_IN_GAME`).
    fn fe_in_game(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    /// `Simulation::IsNetworkGame`8289e448, which `SessionMarker::Update`, `VerifyPosition` and
    /// `VerifyLocalPlayerOkToSet` ask: an online game in play, else `sGameInfo.isNetworkGame`.
    fn network_game(&mut self) -> Result<bool, Self::Error> {
        use SelectedSessionMarkerError as E;
        let world = self.world.lock().map_err(|_| E::WorldLock)?;
        let rate = world.sim_rate.lock().map_err(|_| E::SimRateLock)?;
        rate.try_is_network_game().map_err(E::SimRate)
    }
    /// The Create-A-Spot service's in-use bytes (+0x34/+0x35) are only set by
    /// entering Create-A-Spot from the front end, which the port has no path to.
    fn in_create_a_spot(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn disable_preconditions(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_disable_session_marker_pre_conditions()
            .map_err(SelectedSessionMarkerError::Cvar)
    }
    /// `LivingWorld::Manager::IsBigBlackServiceActive`8271d3a8 reads +0x1b5b4,
    /// which its constructor and Reset clear; the port spawns no service.
    fn security_service_running(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn game_mode(&mut self) -> Result<u32, Self::Error> {
        self.game_info
            .try_game_mode()
            .map_err(SelectedSessionMarkerError::GameInfo)
    }
    fn security_setting(&mut self) -> Result<bool, Self::Error> {
        self.game_info
            .try_free_skate()
            .map(|(_, _, security)| security)
            .map_err(SelectedSessionMarkerError::GameInfo)
    }
    /// `GetLocal`828ab060: `SkaterManager::GetLocal` (+0x170).
    fn local_present(&mut self) -> Result<bool, Self::Error> {
        let local = self
            .local
            .try_local_actor()
            .map_err(SelectedSessionMarkerError::Identity)?;
        Ok(local.is_some())
    }
    fn local_output(&mut self) -> Result<LocalSkaterOutput, Self::Error> {
        use SelectedSessionMarkerError as E;
        Self::output(self.local_actor()?.as_ref(), |output| {
            let state = output.try_state_output().map_err(E::Output)?;
            Ok(LocalSkaterOutput {
                category: state.physics_state_category,
                state: state.physics_state,
                wiping_out: state.is_wiping_out,
                can_leave_wipeout: state.can_leave_wipeout,
                wheels_in_contact: output
                    .try_collision_wheels_in_contact()
                    .map_err(E::Output)?,
                surface: output
                    .try_breadcrumb_collision_surface()
                    .map_err(E::Output)?,
            })
        })
    }
    /// `VerifyLocalPlayerOkToSet`828ab4a0 in a network game: `IsPostionOk` at the local Actor's
    /// `CalcRespawnTransform` 0x825f09a8.
    fn local_respawn_position_ok(&mut self) -> Result<bool, Self::Error> {
        let cvars = self.cvars;
        let respawn = Self::output(self.local_actor()?.as_ref(), |output| {
            try_calc_respawn_transform(output, cvars, &mut HostReckoningNumeric)
                .map_err(SelectedSessionMarkerError::Calc)
        })?;
        self.position_ok(&respawn)
    }
    /// `GetMainActivity`828ab0f0: only a career runs challenge activities.
    fn main_activity_allows(&mut self, use_marker: bool) -> Result<Option<bool>, Self::Error> {
        Ok(self.challenge.and_then(|rules| rules.main_activity).map(|(set, use_)| if use_marker { use_ } else { set }))
    }
    /// Only challenges push `cTeleportManager` overrides.
    fn no_overrides(&mut self, use_marker: bool) -> Result<bool, Self::Error> {
        Ok(self.challenge.is_none_or(|rules| if use_marker { rules.no_overrides.1 } else { rules.no_overrides.0 }))
    }
    /// VerifyChallengeOkToUse828ab6d0 calls ChaseIsOn826f3b48.
    fn chase_is_on(&mut self) -> Result<bool, Self::Error> {
        use SelectedSessionMarkerError as E;
        let owner = Arc::clone(self.world.lock().map_err(|_| E::WorldLock)?
            .grind_services.living_world());
        let living = owner.lock().map_err(|_| E::LivingWorldLock)?;
        Ok(living.chase.as_ref().ok_or(E::MissingChase)?.chase_is_on())
    }
    /// The ChallengeHOMManager constructor sets `mAllowUseSessionMarker`.
    fn hom_allows_use(&mut self) -> Result<bool, Self::Error> {
        Ok(self.challenge.is_none_or(|rules|rules.hom_allows_use))
    }
    /// The career's sign-post message table; without one `GetMessageSetting` returns its
    /// default.
    fn message_setting(&mut self, name: &'static str) -> Result<bool, Self::Error> {
        Ok(self.challenge.is_none_or(|rules| match name {
            "Set Session Marker Visible" => rules.visible.0,
            "Use Session Marker Visible" => rules.visible.1,
            _ => true,
        }))
    }
    /// The career's sign-up manager; other modes have none signing up.
    fn signing_up(&mut self) -> Result<bool, Self::Error> {
        Ok(self.challenge.is_some_and(|rules| rules.signing_up))
    }
    /// The simulation `LayerMan` of the selected terrain.
    fn security_area_key(&mut self, position: [f32; 2]) -> Result<u64, Self::Error> {
        use SelectedSessionMarkerError as E;
        self.terrain
            .terrain_for(self.world)
            .map_err(E::Terrain)?
            .layers
            .lock()
            .map_err(|_| E::LayerLock)?
            .try_attrib_key(SECURITY_GUARD_AREAS, position)
            .map_err(E::Layer)
    }
    /// `IsPostionOk`82ce1fc0 on the local Actor's breadcrumbs:
    /// `_TestForCollisionConstraints(marker, false)`.
    fn position_ok(&mut self, marker: &SkeletonRootMatrix) -> Result<bool, Self::Error> {
        use SelectedSessionMarkerError as E;
        let ground = self
            .terrain
            .ground_pool_for(self.world)
            .map_err(E::Terrain)?;
        let actor = self.local_actor()?;
        let breadcrumbs = actor
            .retained_breadcrumbs()
            .map_err(E::Breadcrumbs)?
            .ok_or(E::MissingBreadcrumbs)?;
        let mut world = self.world.lock().map_err(|_| E::WorldLock)?;
        let breadcrumbs = breadcrumbs.lock().map_err(|_| E::BreadcrumbLock)?;
        breadcrumbs
            .constructor_owners
            .try_verify_world(self.world)
            .map_err(E::BreadcrumbOwner)?;
        // Skate 3 physics: TU3's IsPositionOk 82BFAFF0 (its ground test also takes category 8).
        #[cfg(feature = "skate3-runtime")]
        {
            let mut numeric = HostCollisionLineNumeric;
            let mut source = crate::actor_construction::breadcrumbs::LiveBreadCrumbSelectionSource::new(
                &breadcrumbs, &actor, self.world, &mut world, ground, self.cvars, &mut numeric);
            if let Some(ok) = source.try_skate3_position_ok(marker).map_err(E::Skate3Breadcrumbs)? {
                return Ok(ok);
            }
        }
        breadcrumbs
            .try_collision_constraints(
                marker,
                false,
                &mut world,
                ground,
                &mut HostCollisionLineNumeric,
            )
            .map_err(E::Collision)
    }
}

impl<C> PlayerUiSessionMarkerSource for SelectedSessionMarkerSources<'_, C> {
    fn player_actor_present(&mut self) -> Result<bool, Self::Error> {
        Ok(self.find_player()?.is_some())
    }
    fn input_state(&mut self, input: SessionMarkerInput) -> Result<bool, Self::Error> {
        let state = match input {
            SessionMarkerInput::Set => IS_SESSION_MARKER_SET,
            SessionMarkerInput::Use => IS_SESSION_MARKER_USE,
        };
        self.input
            .try_session_marker_state(state)
            .map_err(SelectedSessionMarkerError::Input)
    }
    fn is_offboard(&mut self) -> Result<bool, Self::Error> {
        Self::output(self.player_actor()?.as_ref(), |output| {
            Ok(output
                .try_state_output()
                .map_err(SelectedSessionMarkerError::Output)?
                .is_offboard)
        })
    }
    /// The port's getter returns RIGHT as true; the enum stores RIGHT as zero.
    fn foot_forward_with_fakie(&mut self) -> Result<FootForward, Self::Error> {
        let right = self.player_actor()?
            .try_current_foot_forward_with_fakie()
            .map_err(SelectedSessionMarkerError::FootForward)?;
        Ok(FootForward::from(!right))
    }
    fn respawn_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let cvars = self.cvars;
        Self::output(self.player_actor()?.as_ref(), |output| {
            try_calc_respawn_transform(output, cvars, &mut HostReckoningNumeric)
                .map_err(SelectedSessionMarkerError::Calc)
        })
    }
    fn distance_to(&mut self, point: SkeletonRootVector) -> Result<f32, Self::Error> {
        use SelectedSessionMarkerError as E;
        Self::output(self.player_actor()?.as_ref(), |output| {
            let root = output
                .try_skeleton_output()
                .map_err(E::Output)?
                .skeleton_root_matrix
                .columns()[3];
            try_length(root.zip(point, |a, b| a - b), &mut HostReckoningNumeric).map_err(E::Numeric)
        })
    }
    fn teleport_requires_load_screen(
        &mut self,
        _transform: &SkeletonRootMatrix,
    ) -> Result<bool, Self::Error> {
        Err(SelectedSessionMarkerError::PendingLoadScreenQuery)
    }
    /// `IsSafeToUseSessionMarker`828aa130: the Skeleton byte is only read
    /// while wiping out.
    fn safe_to_use(&mut self) -> Result<bool, Self::Error> {
        use SelectedSessionMarkerError as E;
        Self::output(self.player_actor()?.as_ref(), |output| {
            let state = output.try_state_output().map_err(E::Output)?;
            if state.physics_requested_teleport {
                return Ok(false);
            }
            if state.is_wiping_out
                && output
                    .try_skeleton_output()
                    .map_err(E::Output)?
                    .is_wipeout_teleport_pending
            {
                return Ok(false);
            }
            Ok(state.physics_state != PHYSICS_STATE_TELEPORTING)
        })
    }
    fn debug_session_teleport(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_debug_session_teleport()
            .map_err(SelectedSessionMarkerError::Cvar)
    }
    fn cvar(&mut self, cvar: SessionMarkerCvar) -> Result<f32, Self::Error> {
        let cvars = self.cvars;
        let value = match cvar {
            SessionMarkerCvar::MinDistanceForTeleport => cvars.try_min_distance_for_teleport(),
            SessionMarkerCvar::MinTimeHeld => cvars.try_min_time_use_session_marker_held(),
            SessionMarkerCvar::MaxTimeHeld => cvars.try_max_time_use_session_marker_held(),
            SessionMarkerCvar::MinDistanceForMinTime => {
                cvars.try_min_distance_for_min_time_use_session_marker_held()
            }
            SessionMarkerCvar::MaxDistanceForMaxTime => {
                cvars.try_max_distance_for_max_time_use_session_marker_held()
            }
        };
        value.map_err(SelectedSessionMarkerError::Cvar)
    }
    fn send_set_marker(&mut self) -> Result<(), Self::Error> {
        use SelectedSessionMarkerError as E;
        let player = i32::from_ne_bytes(self.input.player_index().to_ne_bytes());
        self.local.try_challenge_broadcaster().map_err(E::Observers)?.send_marker(true, player).map_err(E::ChallengeQueue)
    }
    fn dismissal_popup(&mut self) -> Self::Error {
        SelectedSessionMarkerError::PendingSecurityPopup
    }
}

#[derive(Debug)]
pub(crate) enum SkaterManagerCommandError {
    Identity(LocalIdentityError),
    ForeignActor,
    UnknownTranslationW,
    Adjust(AdjustSpawnPositionError),
    Actor(ActorTeleportError),
    Pin(crate::actor_construction::ActorConstructionError),
    Command(TeleportCommandError),
    Observers(crate::skater_manager_local_identity::observers::SkaterObserverError),
    ChallengeQueue(crate::hermes_challenge_queue::ChallengeQueueError),
}
impl std::fmt::Display for SkaterManagerCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SkaterManager::OnCommand: {self:?}")
    }
}

/// `SkaterManager::OnCommand` owners inside the selected tick's command stage.
pub(crate) struct SelectedSkaterManagerCommands<'a> {
    pub(crate) local: &'a SkaterManagerLocalIdentity,
    pub(crate) world: &'a mut PhysicalAssemblyWorld,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) cvars: &'a InputCvarReferences,
    /// The command's Actor: `FindByIndex` of its player index.
    pub(crate) found: Option<Arc<ActorConstruction>>,
}

impl SelectedSkaterManagerCommands<'_> {
    fn found(&self) -> Result<&ActorConstruction, SkaterManagerCommandError> {
        self.found.as_deref().ok_or(SkaterManagerCommandError::ForeignActor)
    }
}

impl SkaterManagerTeleport for SelectedSkaterManagerCommands<'_> {
    type Error = SkaterManagerCommandError;
    /// ISkaterManager vtable +0x18 is `FindByIndex`825f19c0.
    fn has_actor(&mut self, player_index: i32) -> Result<bool, Self::Error> {
        self.found = self
            .local
            .try_find_by_index(player_index)
            .map_err(SkaterManagerCommandError::Identity)?;
        Ok(self.found.is_some())
    }
    fn adjust_spawn_position(
        &mut self,
        transform: &SkeletonRootMatrix,
    ) -> Result<SkeletonRootMatrix, Self::Error> {
        use SkaterManagerCommandError as E;
        let translation = transform.columns()[3]
            .known_lanes()
            .ok_or(E::UnknownTranslationW)?;
        let mut result = *transform;
        if let Some(adjusted) =
            try_adjust_spawn_position(self.world, self.ground, self.cvars, translation)
                .map_err(E::Adjust)?
        {
            result.set_column(3, SkeletonRootVector::known(adjusted));
        }
        Ok(result)
    }
    fn spawn_foot_forward_stance(&mut self, foot_forward: FootForward) -> Result<(), Self::Error> {
        self.found()?
            .try_spawn_foot_forward_stance(foot_forward)
            .map_err(SkaterManagerCommandError::Actor)
    }
    /// Actor vtable +0x3c.
    fn teleport(
        &mut self,
        transform: &SkeletonRootMatrix,
        on_board: bool,
    ) -> Result<(), Self::Error> {
        self.found()?
            .try_publish_teleport(transform, on_board)
            .map_err(SkaterManagerCommandError::Actor)
    }
    fn pin(&mut self, player_index: i32, pin: bool) -> Result<(), Self::Error> {
        if let Some(actor) = self
            .local
            .try_find_by_index(player_index)
            .map_err(SkaterManagerCommandError::Identity)?
        {
            actor.try_set_pinned(pin).map_err(SkaterManagerCommandError::Pin)?;
        }
        Ok(())
    }
    fn send_use_marker(&mut self, player_index: i32) -> Result<(), Self::Error> {
        use SkaterManagerCommandError as E;
        self.local.try_challenge_broadcaster().map_err(E::Observers)?.send_marker(false, player_index).map_err(E::ChallengeQueue)
    }
    fn command(&mut self, error: TeleportCommandError) -> Self::Error {
        SkaterManagerCommandError::Command(error)
    }
}
