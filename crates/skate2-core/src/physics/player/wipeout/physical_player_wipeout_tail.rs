//! Wipeout impulse82d84da8 and recovery82d8f600 through teleport decisions.
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError, wipeout::WipeoutSkeletonError,
        },
    },
    input_cvar_runtime::{InputCvarError, wipeout_control::WipeoutRecoveryFloat},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_wipeout_state::WipeoutGroundFields,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{
    cmp::Ordering,
    convert::Infallible,
    sync::{Arc, Mutex},
};
const FIXED_DT: f32 = f32::from_bits(0x3c88_8889);
pub(crate) const MAX_AUTO_TIME: u64 = 0x74ca_33ae_fd03_ca71;
pub(crate) const MIN_AUTO_TIME: u64 = 0x823d_d222_bb31_1d13;
pub(crate) const MIN_SETTLED_TIME: u64 = 0xb3cf_a69b_b284_bbaf;
pub(crate) const AUTO_FADE_TIME: u64 = 0xb565_383e_5da5_382b;
#[derive(Debug)]
pub(crate) enum WipeoutTailError {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Cvar(InputCvarError),
    Transform(DynamicAssemblyPartTransformError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
    Global(WipeoutSkeletonError<Infallible>),
}
impl std::fmt::Display for WipeoutTailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(e) => write!(f, "wipeout tail missing: {e}"),
            Self::Poisoned => f.write_str("wipeout tail poisoned"),
            Self::ForeignState => f.write_str("wipeout tail foreign state"),
            Self::Pair(e) => write!(f, "wipeout tail pair: {e}"),
            Self::Pool(e) => write!(f, "wipeout tail pool: {e}"),
            Self::Cvar(e) => write!(f, "wipeout tail cvar: {e:?}"),
            Self::Transform(e) => write!(f, "wipeout tail transform: {e}"),
            Self::Part(e) => write!(f, "wipeout tail part: {e}"),
            Self::Body(e) => write!(f, "wipeout tail body: {e}"),
            Self::Global(e) => write!(f, "wipeout tail global: {e}"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecoveryFlag {
    PreventManual,
    IgnoreReset,
    ResetPressed,
    Impaled,
    Ai,
}
pub(crate) trait WipeoutRecoverySource {
    type Error;
    fn impulse_limit(&mut self) -> Result<i32, Self::Error>;
    fn apply_impulse(&mut self, force: Option<SkeletonRootVector>) -> Result<(), Self::Error>;
    fn flag(&mut self, flag: RecoveryFlag) -> Result<bool, Self::Error>;
    fn threshold(&mut self, field: WipeoutRecoveryFloat) -> Result<f32, Self::Error>;
    fn global(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn unknown(&mut self, field: &'static str) -> Self::Error;
    /// PC port: the game's limit on a wipeout's length before its auto teleport (an online round
    /// of S.K.A.T.E. or Spot Battle); none keeps the original's timing.
    fn turn_bail_limit(&mut self) -> Result<Option<f32>, Self::Error> {
        Ok(None)
    }
}
pub(crate) fn try_impulse<S: WipeoutRecoverySource>(
    fields: &mut WipeoutGroundFields,
    source: &mut S,
) -> Result<(), S::Error> {
    if !fields
        .is_impulse
        .ok_or_else(|| source.unknown("is impulse73"))?
    {
        return Ok(());
    }
    let frames = fields
        .impulse_frames
        .ok_or_else(|| source.unknown("impulse frames80"))?;
    if frames < source.impulse_limit()? {
        source.apply_impulse(fields.impulse)?;
        let frames = fields
            .impulse_frames
            .ok_or_else(|| source.unknown("impulse frames80"))?;
        fields.impulse_frames = Some(frames.wrapping_add(1));
    }
    Ok(())
}
pub(crate) fn try_manage_recovery<S: WipeoutRecoverySource>(
    fields: &mut WipeoutGroundFields,
    source: &mut S,
) -> Result<(), S::Error> {
    let prevent = fields.prevent_respawn;
    let ever = fields.ever_at_rest;
    let over = fields.over;
    let ignore = fields.ignore_reset_button;
    let flag = source.flag(RecoveryFlag::PreventManual)?;
    let reset = fields.reset_ever_pressed;
    let ever = ever || over;
    fields.prevent_respawn = prevent || flag;
    let ignore_flag = source.flag(RecoveryFlag::IgnoreReset)?;
    fields.ignore_reset_button = ignore || ignore_flag;
    let reset_flag = source.flag(RecoveryFlag::ResetPressed)?;
    fields.ever_at_rest = ever;
    fields.reset_ever_pressed = reset || reset_flag;
    let enough =
        ever || fields.time_in_state > source.threshold(WipeoutRecoveryFloat::MinimumUser)?;
    fields.enough_time_for_reset |= enough;
    fields.time_settled = if over {
        fields.time_settled + FIXED_DT
    } else {
        0.0
    };
    if try_request_teleport(fields, source)? {
        fields.request_teleport = true;
    }
    Ok(())
}
fn try_request_teleport<S: WipeoutRecoverySource>(
    fields: &mut WipeoutGroundFields,
    source: &mut S,
) -> Result<bool, S::Error> {
    if fields.has_been_impaled {
        fields.time_impaled += FIXED_DT;
    }
    if source.flag(RecoveryFlag::Impaled)? {
        fields.has_been_impaled = true;
        if fields.time_impaled > source.threshold(WipeoutRecoveryFloat::MinimumImpaled)? {
            return Ok(true);
        }
    }
    if fields.prevent_respawn {
        return Ok(false);
    }
    if fields.enough_time_for_reset && fields.reset_ever_pressed && !fields.ignore_reset_button {
        return Ok(true);
    }
    if fields.auto_teleport_pending {
        fields.time_until_teleport -= source.time_delta()?;
        // Native BO12/BI25 exits false only for ordered positive. NaN returns true.
        return Ok(fields.time_until_teleport.partial_cmp(&0.0) != Some(Ordering::Greater));
    }
    if try_auto_teleport(fields, source)? {
        fields.time_until_teleport = source.global(AUTO_FADE_TIME)?;
        fields.auto_teleport_pending = true;
    }
    Ok(false)
}
fn try_auto_teleport<S: WipeoutRecoverySource>(
    fields: &WipeoutGroundFields,
    source: &mut S,
) -> Result<bool, S::Error> {
    if source.flag(RecoveryFlag::Ai)? {
        return Ok(fields.time_in_state > source.threshold(WipeoutRecoveryFloat::MaximumAi)?);
    }
    let maximum = source.global(MAX_AUTO_TIME)?;
    // PC port: in an online turn the wipeout ends by the game's limit at the latest, however long
    // its Hall of Meat moves keep the body from settling, so the turn passes on.
    let maximum = source.turn_bail_limit()?.map_or(maximum, |limit| maximum.min(limit));
    if fields.time_in_state > maximum {
        return Ok(true);
    }
    let minimum = source.global(MIN_AUTO_TIME)?;
    if fields.time_in_state > minimum {
        let settled = source.global(MIN_SETTLED_TIME)?;
        return Ok(fields.time_settled > settled);
    }
    Ok(false)
}
struct LiveRecovery<'a> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    owner: &'a PhysicalPlayerAssemblyOwner,
    processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
}
impl WipeoutRecoverySource for LiveRecovery<'_> {
    type Error = WipeoutTailError;
    fn unknown(&mut self, field: &'static str) -> Self::Error {
        WipeoutTailError::Missing(field)
    }
    fn impulse_limit(&mut self) -> Result<i32, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_max_wipeout_impulse_frames()
            .map_err(WipeoutTailError::Cvar)
    }
    fn apply_impulse(&mut self, force: Option<SkeletonRootVector>) -> Result<(), Self::Error> {
        use WipeoutTailError as E;
        let board = self
            .world
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .session(self.board)
            .map_err(E::Pool)?;
        // Body.GetPosition82de7b20 calls Part.GetTransform, not GetBodyTransform.
        let part = board
            .try_live_part_transform_typed(&self.world.backend, 6)
            .map_err(E::Transform)?;
        let observation = board
            .try_part_contact_spy_observation(&self.world.backend, 6)
            .map_err(E::Part)?;
        let force = force.ok_or(E::Missing("wipeout impulse50"))?;
        let point = self
            .processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .deck_transform()
            .ok_or(E::Missing("Processed deck transform"))?
            .columns()[3];
        self.world
            .backend
            .try_apply_skateboard_world_force(observation, force, point, part.columns()[3])
            .map_err(E::Body)
    }
    fn flag(&mut self, flag: RecoveryFlag) -> Result<bool, Self::Error> {
        use WipeoutTailError as E;
        let p = self.processed.lock().map_err(|_| E::Poisoned)?;
        let (value, known, mask) = match flag {
            RecoveryFlag::PreventManual => {
                let (v, k) = p.known_flags_8c4();
                (v, k, 2)
            }
            RecoveryFlag::IgnoreReset => {
                let (v, k) = p.known_flags_8c4();
                (v, k, 1)
            }
            RecoveryFlag::ResetPressed => (
                p.known_flags_8c8().ok_or(E::Missing("Processed8c8"))?,
                u32::MAX,
                0x0020_0000,
            ),
            RecoveryFlag::Ai => (
                p.known_flags_8c8().ok_or(E::Missing("Processed8c8"))?,
                u32::MAX,
                0x2000_0000,
            ),
            RecoveryFlag::Impaled => {
                let (v, k) = p.physics_state_fields().flags_8d4;
                (v, k, 0x0040_0000)
            }
        };
        if known & mask != mask {
            return Err(E::Missing("selected recovery flag"));
        }
        Ok(value & mask != 0)
    }
    fn threshold(&mut self, field: WipeoutRecoveryFloat) -> Result<f32, Self::Error> {
        self.owner
            .inputs()
            .cvars
            .try_wipeout_recovery_float(field)
            .map_err(WipeoutTailError::Cvar)
    }
    fn global(&mut self, key: u64) -> Result<f32, Self::Error> {
        use WipeoutTailError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_pair_global_wipeout_float(&self.world.backend, self.board, self.skeleton, key)
            .map_err(E::Global)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        use WipeoutTailError as E;
        self.processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("DT93c"))
    }
    fn turn_bail_limit(&mut self) -> Result<Option<f32>, Self::Error> {
        let Some(services) = self.owner.world_services() else { return Ok(None) };
        services.game_info().try_turn_bail_limit().map_err(|_| WipeoutTailError::Poisoned)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_finish_pair_wipeout_recovery(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), WipeoutTailError> {
        use WipeoutTailError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        if state.current != CurrentPhysicalState::WipeoutGround {
            return Err(E::ForeignState);
        }
        let state = state
            .wipeout_ground
            .as_mut()
            .ok_or(E::Missing("WipeoutGround"))?;
        if !state.bindings.board.same_as(board)
            || !state.bindings.skeleton.same_as(skeleton)
            || !state
                .bindings
                .owner
                .upgrade()
                .is_some_and(|p| Arc::ptr_eq(&p, &owner))
        {
            return Err(E::ForeignState);
        }
        let mut source = LiveRecovery {
            world: self,
            board,
            skeleton,
            owner: &owner,
            processed: &processed,
        };
        try_impulse(&mut state.fields, &mut source)?;
        try_manage_recovery(&mut state.fields, &mut source)
    }
}
