//! Ordinary command payload slice of `PlayerUI::Update828af9f8`.
//! Caller reaches this only after the native simulation admission, focus command,
//! session marker and optional network teleport stages. Spawner and proxy tick
//! follow delivery; this leaf neither schedules nor acknowledges those stages.
use super::SelectedDeviceInputRuntime;
use crate::{
    action_graph_input_listener::fill::{try_local_direction, world_direction},
    game_input_frame_session::GameInputFrameRequest,
    game_input_player::PlayerInputData,
    input_pad::device::PadRemap,
    input_pad_cache::InputPadCacheError,
    player::PlayerLoaderSlot,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, try_normalize_safe,
    },
    simulation_assembly_backend::constraint_session::world_services::{
        PhysicalGameInfo, PhysicalWorldServicesError,
    },
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};

/// Actual pause-owner query, reached only for an unfocused sample.
pub(crate) trait PlayerInputPause {
    type Error;
    fn try_is_simulation_active(&mut self) -> Result<bool, Self::Error>;
}
/// Actual selected command proxy; its unsigned frame is sampled only when simulation is active.
pub(crate) trait PlayerInputProxyFrame {
    type Error;
    fn try_next_generation_frame(&mut self) -> Result<u32, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum SelectedPlayerSampleError<P, F> {
    World(PhysicalWorldServicesError),
    Device(InputPadCacheError),
    OffBoardStick(ReckoningHostNumericError),
    Pause(P),
    Proxy(F),
}
impl<P: std::fmt::Debug, F: std::fmt::Debug> std::fmt::Display for SelectedPlayerSampleError<P, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::World(error) => write!(f, "player input network state: {error:?}"),
            Self::Device(error) => write!(f, "player input current cache: {error}"),
            Self::OffBoardStick(error) => write!(f, "player input off-board stick: {error}"),
            Self::Pause(error) => write!(f, "player input simulation pause: {error:?}"),
            Self::Proxy(error) => write!(f, "player input proxy frame: {error:?}"),
        }
    }
}

/// What 1.02 `sub_826F7668` reads for a local skater off its board: the first three
/// rows of `CameraPresentation::GetMatrix(true)` and of the skater's
/// `PhysOut_Skeleton::mEffectiveSkeletonRootMatrix`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OffBoardStickFrames {
    pub(crate) camera: [SkeletonRootVector; 3],
    pub(crate) skater: [SkeletonRootVector; 3],
}
impl OffBoardStickFrames {
    /// 1.02 `sub_826F7668` (826f7668..78b4), new in that update: the stick
    /// (LStickL - LStickR, 0, LStickU - LStickD) through the camera's rows, flattened,
    /// `NormalizeSafe`d with a zero default, then in the skater's frame through its
    /// adjugate inverse. Its X and Z replace the four stick buttons, so a held direction
    /// stays where the camera sees it while the skater turns toward it.
    fn try_apply<N: ReckoningNumeric>(
        &self,
        values: &mut [f32; 24],
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        let stick = [values[17] - values[16], 0.0, values[18] - values[19], 0.0];
        let mut view = world_direction(self.camera, stick);
        view[1] = 0.0;
        let direction = try_normalize_safe(view, [0.0; 4], numeric)?;
        let local = try_local_direction(self.skater, direction, numeric)?;
        // 826f7844..78a4: each axis splits back into its two buttons. The raw bge,
        // bgt and ble all leave an unordered axis at zero in both.
        values[17] = if local[0] < 0.0 { -local[0] } else { 0.0 };
        values[16] = if local[0] > 0.0 { local[0] } else { 0.0 };
        values[19] = if local[2] < 0.0 { -local[2] } else { 0.0 };
        values[18] = if local[2] > 0.0 { local[2] } else { 0.0 };
        Ok(())
    }
}

pub(crate) struct SelectedPlayerSample {
    input: PlayerInputData,
    receiver_id: u32,
}
impl SelectedPlayerSample {
    /// Copy the same produced payload into the retained ordinary command proxy.
    /// Native send and admitted command consumption occur separately later.
    pub(crate) fn try_queue_into<N: crate::game_input_command::GameInputCommandNumeric>(
        &self,
        proxy: &mut crate::selected_sim_command_queue::SelectedSimCommandQueue,
        numeric: &mut N,
    ) -> Result<(), crate::selected_sim_command_queue::QueuePlayerInputError<N::Error>> {
        proxy.try_queue_player_input(self.receiver_id, &self.input, numeric)
    }

    /// Borrow directly into the existing sparse codec/command/recognizer path.
    /// Update mask and CRC admission remain supplied by that path's caller.
    pub(crate) const fn frame_request(
        &self,
        update_mask: u32,
        add_to_crc: bool,
    ) -> GameInputFrameRequest<'_> {
        GameInputFrameRequest {
            input: &self.input,
            receiver_id: self.receiver_id,
            update_mask,
            add_to_crc,
        }
    }
}

impl<R: Clone, N: SkeletonCollisionNumeric, M: PadRemap<R>> SelectedDeviceInputRuntime<R, N, M> {
    /// Native payload slice `828afc1c..828afcc0`, over the same selected loader,
    /// global game information and device manager. Controller selection is an
    /// explicit host-supplied native identity, independently of player index.
    /// No pause/proxy default is provided: unavailable collaborators fail lazily.
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit retained runtime collaborators"
    )]
    pub(crate) fn try_player_sample<P: PlayerInputPause, F: PlayerInputProxyFrame>(
        &self,
        loader: &PlayerLoaderSlot,
        controller: usize,
        game_info: &PhysicalGameInfo,
        has_focus: bool,
        offboard: Option<&OffBoardStickFrames>,
        pause: &mut P,
        proxy: &mut F,
    ) -> Result<SelectedPlayerSample, SelectedPlayerSampleError<P::Error, F::Error>> {
        let allow_sensor = !game_info
            .try_is_network_game()
            .map_err(SelectedPlayerSampleError::World)?;
        let mut input = PlayerInputData {
            sample: self
                .raw_current_cache_line(controller)
                .map_err(SelectedPlayerSampleError::Device)?,
            allow_sensor,
            has_focus: true,
        };
        // 1.02 8232f7a4..f82c: before the focus test, the copied line of a local
        // skater off its board, not holding an object and not landing on its deck.
        if let Some(frames) = offboard {
            frames
                .try_apply(&mut input.sample.values, &mut HostReckoningNumeric)
                .map_err(SelectedPlayerSampleError::OffBoardStick)?;
        }
        if !has_focus {
            // Source clears all 24 words, including stale disconnected values;
            // it retains NumButtons and initially leaves payload focus true.
            input.sample.values.fill(0.0);
            if pause
                .try_is_simulation_active()
                .map_err(SelectedPlayerSampleError::Pause)?
                && proxy
                    .try_next_generation_frame()
                    .map_err(SelectedPlayerSampleError::Proxy)?
                    > 1
            {
                input.has_focus = false;
            }
        }
        Ok(SelectedPlayerSample {
            input,
            receiver_id: loader.player_index().wrapping_add(10),
        })
    }
}

#[cfg(test)]
#[path = "input_selected_player_sample_tests.rs"]
mod tests;
