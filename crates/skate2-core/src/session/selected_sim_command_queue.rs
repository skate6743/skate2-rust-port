//! Retained offline command stages of `SimController`, not its full scheduler.
//! Caller admits the real ordinary offline proxy and reaches consumption after
//! pause/broadcaster/network/NIS/streamer effects. Existing gameplay update runs
//! later once for the returned mask; it is never invoked per queued command.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained offline command stages await production session scheduling"
    )
)]
use crate::{
    game_input_command::{
        GameInputCommandNumeric, GameInputPackError, GameInputWrite, try_pack_player_input,
    },
    game_input_manager::{
        EncodedGameInputCommand, GameInputCommandManager, GameInputDispatch, GameInputManagerError,
        GameInputSyncLevel,
    },
    game_input_player::PlayerInputData,
    input_pad::{
        device::PadRemap,
        device_manager::selected_runtime::{
            SelectedDeviceInputRuntime,
            player_sample::{PlayerInputPause, PlayerInputProxyFrame, SelectedPlayerSampleError},
        },
    },
    native_skater_random::{NativeRandomLocked, SimulationCommandRandom},
    players_backend_settings::controller::BackendControllerSelection,
    simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

/// Native source globals83058ea0/ea4, initialized0 and shared by controllers.
pub(crate) struct SimCommandCounters {
    simulation: AtomicU32,
    visual: AtomicU32,
}
impl SimCommandCounters {
    pub(crate) fn from_static_initializer() -> Arc<Self> {
        Arc::new(Self {
            simulation: AtomicU32::new(0),
            visual: AtomicU32::new(0),
        })
    }
    fn increment(counter: &AtomicU32) {
        counter.store(
            counter.load(Ordering::Relaxed).wrapping_add(1),
            Ordering::Relaxed,
        );
    }
}

/// Native Command has a 256-byte data array; only its declared payload is read.
/// The host's unused array tail is not serialized or promoted to source data.
#[derive(Clone, Copy)]
struct QueuedCommand {
    receiver: u32,
    kind: u32,
    length: usize,
    bytes: [u8; 256],
}
impl QueuedCommand {
    fn try_copy(command: &EncodedGameInputCommand<'_>) -> Result<Self, CommandQueueError> {
        if command.payload.len() > 256 {
            return Err(CommandQueueError::PayloadTooLarge);
        }
        let mut bytes = [0; 256];
        bytes[..command.payload.len()].copy_from_slice(command.payload);
        Ok(Self {
            receiver: command.receiver_id,
            kind: command.kind,
            length: command.payload.len(),
            bytes,
        })
    }
    fn borrowed(&self) -> EncodedGameInputCommand<'_> {
        EncodedGameInputCommand {
            receiver_id: self.receiver,
            kind: self.kind,
            payload: &self.bytes[..self.length],
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CommandQueueError {
    PayloadTooLarge,
    Allocation,
    FailedStage,
}
impl std::fmt::Display for CommandQueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PayloadTooLarge => "command payload exceeds native256 capacity",
            Self::Allocation => "native command vector host allocation failed",
            Self::FailedStage => {
                "command stage previously failed; retained partial state cannot be replayed"
            }
        })
    }
}
#[derive(Debug)]
pub(crate) enum QueuePlayerInputError<N> {
    Pack(GameInputPackError<N>),
    Queue(CommandQueueError),
}
impl<N: std::fmt::Debug> std::fmt::Display for QueuePlayerInputError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pack(e) => e.fmt(f),
            Self::Queue(e) => e.fmt(f),
        }
    }
}
fn push(
    commands: &mut Vec<QueuedCommand>,
    command: &QueuedCommand,
) -> Result<(), CommandQueueError> {
    commands
        .try_reserve(1)
        .map_err(|_| CommandQueueError::Allocation)?;
    commands.push(*command);
    Ok(())
}
fn assign(
    destination: &mut Vec<QueuedCommand>,
    source: &[QueuedCommand],
) -> Result<(), CommandQueueError> {
    // Native copy assignment clears the old range before any overflow allocation.
    destination.clear();
    destination
        .try_reserve(source.len())
        .map_err(|_| CommandQueueError::Allocation)?;
    destination.extend_from_slice(source);
    Ok(())
}
struct OfflineCommandMailbox {
    commands: Vec<QueuedCommand>,
    valid: bool,
}
impl OfflineCommandMailbox {
    fn construct_fields() -> Self {
        Self {
            commands: Vec::new(),
            valid: false,
        }
    }
    fn try_send(&mut self, commands: &[QueuedCommand]) -> Result<bool, CommandQueueError> {
        if self.valid {
            return Ok(false);
        }
        assign(&mut self.commands, commands)?;
        self.valid = true;
        Ok(true)
    }
    fn try_get(&mut self) -> Result<Option<Vec<QueuedCommand>>, CommandQueueError> {
        if !self.valid {
            return Ok(None);
        }
        let mut output = Vec::new();
        assign(&mut output, &self.commands)?;
        self.valid = false;
        Ok(Some(output))
    }
}

pub(crate) struct SelectedSimCommandQueue {
    counters: Arc<SimCommandCounters>,
    next_generation_frame: u32,
    commands: Vec<QueuedCommand>,
    ghost_commands: Vec<QueuedCommand>,
    exec_ghost_commands: Vec<QueuedCommand>,
    valid_ghost_command: bool,
    offline: OfflineCommandMailbox,
    // Host error boundary only: native assertions do not provide retry semantics.
    failed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandBatchPhase {
    Ghost,
    Simulation,
}
#[derive(Debug)]
pub(crate) enum CommandConsumeError<S> {
    Queue(CommandQueueError),
    Random(NativeRandomLocked),
    Dispatch {
        phase: CommandBatchPhase,
        index: usize,
        error: GameInputManagerError<S>,
    },
}
impl<S: std::fmt::Debug> std::fmt::Display for CommandConsumeError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queue(e) => e.fmt(f),
            Self::Random(e) => e.fmt(f),
            Self::Dispatch {
                phase,
                index,
                error,
            } => write!(f, "{phase:?} command {index}: {error}"),
        }
    }
}
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CommandStageDelivery {
    pub(crate) update_mask: u32,
    pub(crate) delivered: usize,
    pub(crate) missing_receivers: usize,
    /// Receiver-1 commands in batch order, for `SkaterManager::OnCommand`.
    pub(crate) skater_manager: Vec<(u32, Vec<u8>)>,
}
impl CommandStageDelivery {
    fn record(&mut self, result: GameInputDispatch) {
        match result {
            GameInputDispatch::Delivered => self.delivered += 1,
            GameInputDispatch::MissingReceiver => self.missing_receivers += 1,
            GameInputDispatch::SkaterManager { kind, payload } => {
                self.delivered += 1;
                self.skater_manager.push((kind, payload));
            }
        }
    }
}
impl SelectedSimCommandQueue {
    /// Explicit lower-level offline admission. Does not perform `ResetNetwork`'s
    /// online/playback/ghost selection, broadcaster registration or Simulation Reset.
    /// `SimController` fields828a4c24..74 and real `OfflineNetwork` ctor82793618.
    pub(crate) fn construct_offline_command_fields(counters: Arc<SimCommandCounters>) -> Self {
        Self {
            counters,
            next_generation_frame: 1,
            commands: Vec::new(),
            ghost_commands: Vec::new(),
            exec_ghost_commands: Vec::new(),
            valid_ghost_command: false,
            offline: OfflineCommandMailbox::construct_fields(),
            failed: false,
        }
    }
    fn require_usable(&self) -> Result<(), CommandQueueError> {
        if self.failed {
            Err(CommandQueueError::FailedStage)
        } else {
            Ok(())
        }
    }
    /// `DoCommand828a6470` copies, never calls the workspace dispatcher.
    pub(crate) fn try_queue(
        &mut self,
        command: &EncodedGameInputCommand<'_>,
    ) -> Result<(), CommandQueueError> {
        self.require_usable()?;
        push(&mut self.commands, &QueuedCommand::try_copy(command)?)
    }
    /// Existing codec producer; receiver/input come from the selected sample.
    pub(crate) fn try_queue_player_input<N: GameInputCommandNumeric>(
        &mut self,
        receiver: u32,
        input: &PlayerInputData,
        numeric: &mut N,
    ) -> Result<(), QueuePlayerInputError<N::Error>> {
        self.require_usable()
            .map_err(QueuePlayerInputError::Queue)?;
        let mut bytes = [0; 9];
        let size = {
            let mut stream = GameInputWrite::new(&mut bytes);
            try_pack_player_input(input, &mut stream, numeric)
                .map_err(QueuePlayerInputError::Pack)?;
            stream.position()
        };
        self.try_queue(&EncodedGameInputCommand {
            receiver_id: receiver,
            kind: 1,
            payload: &bytes[..size],
        })
        .map_err(QueuePlayerInputError::Queue)
    }
    /// `GenerateSyncCommand828a89a0`: successful send is the commit boundary.
    /// Inline native capacity10 permits overflow; it is not an admission limit.
    pub(crate) fn try_generate_sync_command(&mut self) -> Result<bool, CommandQueueError> {
        self.require_usable()?;
        let result = self.generate();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn generate(&mut self) -> Result<bool, CommandQueueError> {
        if self.valid_ghost_command || !self.offline.try_send(&self.commands)? {
            return Ok(false);
        }
        self.next_generation_frame = self.next_generation_frame.wrapping_add(1);
        self.commands.clear();
        assign(&mut self.exec_ghost_commands, &self.ghost_commands)?;
        self.ghost_commands.clear();
        self.valid_ghost_command = true;
        Ok(true)
    }
    /// Exact command-consuming slice828a74d8..7674, admitted after earlier
    /// native effects. Reuses the supplied actual workspace dispatcher/RNG.
    /// A failure retains completed writes and ends this host stage permanently.
    pub(crate) fn try_consume_admitted<S: GameInputSyncLevel>(
        &mut self,
        dispatcher: &mut GameInputCommandManager,
        random: &mut SimulationCommandRandom,
        sync: &mut S,
    ) -> Result<CommandStageDelivery, CommandConsumeError<S::Error>> {
        self.require_usable().map_err(CommandConsumeError::Queue)?;
        let result = self.consume(dispatcher, random, sync);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn consume<S: GameInputSyncLevel>(
        &mut self,
        dispatcher: &mut GameInputCommandManager,
        random: &mut SimulationCommandRandom,
        sync: &mut S,
    ) -> Result<CommandStageDelivery, CommandConsumeError<S::Error>> {
        let mut result = CommandStageDelivery::default();
        if self.valid_ghost_command {
            result.update_mask = 2;
            random.unlock_for_commands();
            for (index, command) in self.exec_ghost_commands.iter().enumerate() {
                result.record(
                    dispatcher
                        .try_do_command(&command.borrowed(), false, sync)
                        .map_err(|error| CommandConsumeError::Dispatch {
                            phase: CommandBatchPhase::Ghost,
                            index,
                            error,
                        })?,
                );
            }
            self.exec_ghost_commands.clear();
            self.counters.visual.store(0, Ordering::Relaxed);
            self.valid_ghost_command = false;
        } else {
            SimCommandCounters::increment(&self.counters.visual);
        }
        if let Some(commands) = self.offline.try_get().map_err(CommandConsumeError::Queue)? {
            result.update_mask |= 1;
            random.unlock_for_commands();
            for (index, command) in commands.iter().enumerate() {
                // GetRandom(0,1) return is discarded at828a7614: exact state
                // mutation matters; no host float estimate is substituted.
                random
                    .try_advance_for_discarded_range()
                    .map_err(CommandConsumeError::Random)?;
                result.record(
                    dispatcher
                        .try_do_command(&command.borrowed(), true, sync)
                        .map_err(|error| CommandConsumeError::Dispatch {
                            phase: CommandBatchPhase::Simulation,
                            index,
                            error,
                        })?,
                );
            }
            self.counters.simulation.store(0, Ordering::Relaxed);
        } else {
            SimCommandCounters::increment(&self.counters.simulation);
        }
        Ok(result)
    }
}
impl PlayerInputProxyFrame for SelectedSimCommandQueue {
    type Error = CommandQueueError;
    fn try_next_generation_frame(&mut self) -> Result<u32, Self::Error> {
        self.require_usable()?;
        Ok(self.next_generation_frame)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SelectedCommandSampleRequest<'a, 'loader> {
    pub(crate) selection: &'a BackendControllerSelection<'loader>,
    pub(crate) game_info: &'a PhysicalGameInfo,
    pub(crate) has_focus: bool,
}
#[derive(Debug)]
pub(crate) enum SelectedInputQueueError<P, N> {
    Sample(SelectedPlayerSampleError<P, CommandQueueError>),
    Queue(QueuePlayerInputError<N>),
}
impl<P: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for SelectedInputQueueError<P, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sample(e) => e.fmt(f),
            Self::Queue(e) => e.fmt(f),
        }
    }
}
impl<R: Clone, N: SkeletonCollisionNumeric, M: PadRemap<R>> SelectedDeviceInputRuntime<R, N, M> {
    /// Same retained input owner produces the native ordinary payload and copies
    /// its encoded command to the supplied retained proxy. No send/drain occurs.
    pub(crate) fn try_sample_and_queue_player<P: PlayerInputPause, Q: GameInputCommandNumeric>(
        &self,
        request: SelectedCommandSampleRequest<'_, '_>,
        pause: &mut P,
        proxy: &mut SelectedSimCommandQueue,
        numeric: &mut Q,
    ) -> Result<(), SelectedInputQueueError<P::Error, Q::Error>> {
        let sample = self
            .try_player_sample(
                request.selection.loader,
                request.selection.device_index(),
                request.game_info,
                request.has_focus,
                None,
                pause,
                proxy,
            )
            .map_err(SelectedInputQueueError::Sample)?;
        sample
            .try_queue_into(proxy, numeric)
            .map_err(SelectedInputQueueError::Queue)
    }
}
#[cfg(test)]
#[path = "selected_sim_command_queue_tests.rs"]
mod tests;

#[path = "selected_player_focus.rs"]
pub(crate) mod focus;
