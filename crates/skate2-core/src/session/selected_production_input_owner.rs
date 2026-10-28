//! One-shot selected input composition. The application supplies real pad and
//! Juice platforms, mounted configuration, scheduler, and Simulation fields.
//! Draft module awaiting root integration into the registered application.
use crate::{
    actor_construction::{
        selected_startup::SelectedPlayerStartupOwners, selected_tick::SelectedQueuedTickInput,
    },
    input_cvar_runtime::InputCvarReferences,
    input_map::{InputMapParseError, InputMapReadError, RetainedPadView, SelectedInputMap},
    input_pad::{
        device::{PadActuatorError, PadPlatform},
        device_manager::{
            DeviceManagerError, InputDeviceManager,
            selected_runtime::{
                SelectedDeviceInputError, SelectedDeviceInputEvents, SelectedDeviceInputRuntime,
                SelectedInputGroup,
            },
        },
    },
    input_pad_cache::{InputPadCache, InputPadCacheError},
    juice_trick_input::{
        JuiceTrickInput,
        boot::{
            JuiceBootError, JuiceBootPlatform, JuiceHostOption, JuiceInputBoot, JuiceRemapHandle,
        },
    },
    native_skater_random::SimulationCommandRandom,
    player::PlayerLoaderSlot,
    players_backend_settings::{PlayersSettingsError, controller::BackendControllerSelection},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    selected_sim_command_queue::{
        CommandQueueError, QueuePlayerInputError, SelectedSimCommandQueue,
    },
    simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
use skate2_formats::mount::{MountError, MountSet};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

type DeviceInput<C> =
    SelectedDeviceInputRuntime<JuiceRemapHandle, HostSkeletonCollisionNumeric, JuiceInputBoot<C>>;

/// Native `InputManager::Init826e2788` precedes Juice construction/Start in
/// `InitBoot82740458`. This owner can be built before mounted player startup.
/// A failed one-shot stage leaves its partial fields here for inspection.
pub(crate) struct SelectedInputBootstrap<C> {
    boot: JuiceInputBoot<C>,
    devices: Option<InputDeviceManager<JuiceRemapHandle>>,
    map: Option<Arc<SelectedInputMap>>,
    simulation: Option<InputPadCache>,
    presentation: Option<InputPadCache>,
    attempted: bool,
    initialized: bool,
}

#[derive(Debug)]
pub(crate) enum ProductionInputStartError<P, J> {
    AlreadyAttempted,
    Device(DeviceManagerError<(), (), ()>),
    Mount(MountError),
    Utf8(std::str::Utf8Error),
    Map(InputMapParseError),
    Cache {
        group: SelectedInputGroup,
        error: InputPadCacheError,
    },
    Actuator {
        device: usize,
        error: PadActuatorError<P, ReckoningHostNumericError>,
    },
    JuiceConstruct(JuiceBootError<J>),
    JuiceStart(JuiceBootError<J>),
    JuiceInstall(JuiceBootError<Infallible>),
}

impl<C> SelectedInputBootstrap<C> {
    pub(crate) fn new(juice: JuiceTrickInput) -> Self {
        Self {
            boot: JuiceInputBoot::new_static_initialized(juice),
            devices: None,
            map: None,
            simulation: None,
            presentation: None,
            attempted: false,
            initialized: false,
        }
    }

    /// Supply this same shared occurrence to `MountedSelectedOwners::juice`.
    pub(crate) fn trick_input(&self) -> JuiceTrickInput {
        self.boot.trick_input()
    }

    /// The single `input.cfg` map parsed during `InputManager` initialization.
    /// Give this same Arc to selected player startup; that path must not parse
    /// another map before Actor/input reset.
    pub(crate) fn shared_map(&self) -> Option<Arc<SelectedInputMap>> {
        self.initialized
            .then(|| Arc::clone(self.map.as_ref().expect("complete map")))
    }

    /// `juice_ini=None` is an actual absent-file result, never a mount/read
    /// error. The host explicitly chooses original configuration or disable.
    /// A connected socket stops at typed `HandshakePending`; no ready remap is
    /// fabricated before protocol handshake and controller registration.
    pub(crate) fn try_initialize<P: PadPlatform, J: JuiceBootPlatform<Connection = C>>(
        &mut self,
        pad: &mut P,
        juice_platform: &mut J,
        mounts: &MountSet,
        cvars: &InputCvarReferences,
        juice_ini: Option<&[u8]>,
        juice_policy: JuiceHostOption,
    ) -> Result<(), ProductionInputStartError<P::Error, J::Error>> {
        use ProductionInputStartError as E;
        if self.attempted {
            return Err(E::AlreadyAttempted);
        }
        self.attempted = true;
        let mut devices = InputDeviceManager::new();
        devices.initialize().map_err(E::Device)?;
        self.devices = Some(devices);
        // `InputManager::Init826e27ec` loads the map between device-manager
        // initialization and both group-cache constructors.
        let bytes = mounts
            .read_path("data\\config\\input.cfg")
            .map_err(E::Mount)?;
        let map = SelectedInputMap::parse_with_frontend_inputs(
            std::str::from_utf8(&bytes).map_err(E::Utf8)?,
        )
        .map_err(E::Map)?;
        self.map = Some(Arc::new(map));
        let mut simulation = InputPadCache::new();
        simulation.try_initialize().map_err(|error| E::Cache {
            group: SelectedInputGroup::Simulation,
            error,
        })?;
        self.simulation = Some(simulation);
        let mut presentation = InputPadCache::new();
        presentation.try_initialize().map_err(|error| E::Cache {
            group: SelectedInputGroup::Presentation,
            error,
        })?;
        self.presentation = Some(presentation);
        // InputManager::Init creates vibration owners and resets each pad before
        // `InitBoot` starts Juice. This also samples real XInput capabilities.
        for (device, slot) in self
            .devices
            .as_mut()
            .expect("initialized manager")
            .devices_mut()
            .expect("initialized slots")
            .iter_mut()
            .enumerate()
        {
            slot.reset_actuators(pad, cvars, &mut HostReckoningNumeric)
                .map_err(|error| E::Actuator { device, error })?;
        }
        self.boot
            .try_construct(juice_ini, juice_policy, juice_platform)
            .map_err(E::JuiceConstruct)?;
        self.boot
            .try_start(cvars, juice_platform)
            .map_err(E::JuiceStart)?;
        self.boot
            .try_install(self.devices.as_mut().expect("initialized manager"))
            .map_err(E::JuiceInstall)?;
        self.initialized = true;
        Ok(())
    }

    /// Bind only after mounted player startup publishes its actual shared map.
    /// On failure, all existing owners and supplied Simulation fields return in
    /// `ProductionInputBindFailure`, with no partial move or retry illusion.
    #[expect(
        clippy::result_large_err,
        reason = "failed binding returns the unique input bootstrap and queue without dropping either"
    )]
    pub(crate) fn try_bind_player(
        mut self,
        players: &SelectedPlayerStartupOwners,
        queue: SelectedSimCommandQueue,
        random: SimulationCommandRandom,
    ) -> Result<SelectedProductionInputOwner<C>, ProductionInputBindFailure<C>> {
        if !self.initialized {
            return Err(ProductionInputBindFailure {
                error: ProductionInputBindError::IncompleteStart,
                bootstrap: self,
                queue,
                random,
            });
        }
        let Some(map) = players.map.as_ref() else {
            return Err(ProductionInputBindFailure {
                error: ProductionInputBindError::MissingMap,
                bootstrap: self,
                queue,
                random,
            });
        };
        if !Arc::ptr_eq(self.map.as_ref().expect("complete map"), map) {
            return Err(ProductionInputBindFailure {
                error: ProductionInputBindError::ForeignMap,
                bootstrap: self,
                queue,
                random,
            });
        }
        let selection = match BackendControllerSelection::try_capture(
            &players.services.settings,
            &players.loader,
        ) {
            Ok(selection) => selection,
            Err(error) => {
                return Err(ProductionInputBindFailure {
                    error: ProductionInputBindError::Controller(error),
                    bootstrap: self,
                    queue,
                    random,
                });
            }
        };
        let player_index = players.loader.player_index();
        let controller = selection.device_index();
        let input = SelectedDeviceInputRuntime::from_owners(
            self.devices.take().expect("complete device manager"),
            self.simulation.take().expect("complete SIM cache"),
            self.presentation.take().expect("complete PRES cache"),
            self.map.take().expect("complete map"),
            HostSkeletonCollisionNumeric,
            self.boot,
        );
        Ok(SelectedProductionInputOwner {
            focus: Arc::clone(&players.focus),
            pause: Arc::clone(&players.pause),
            settings: Arc::clone(&players.services.settings),
            player_index,
            controller,
            input,
            queue,
            random,
        })
    }
}

#[derive(Debug)]
pub(crate) enum ProductionInputBindError {
    IncompleteStart,
    MissingMap,
    ForeignMap,
    Controller(PlayersSettingsError),
}
pub(crate) struct ProductionInputBindFailure<C> {
    pub(crate) error: ProductionInputBindError,
    pub(crate) bootstrap: SelectedInputBootstrap<C>,
    pub(crate) queue: SelectedSimCommandQueue,
    pub(crate) random: SimulationCommandRandom,
}

/// Retains exactly the input/cache/map/queue/RNG owners consumed by the
/// selected queued player tick. Hardware remains supplied by the application.
pub(crate) struct SelectedProductionInputOwner<C> {
    focus: Arc<Mutex<crate::selected_sim_command_queue::focus::SelectedPlayerFocus>>,
    pause: Arc<crate::selected_pause_manager::SelectedPauseManager>,
    /// The players back end `PartyPlayUI::SetPlayer` reads.
    settings: Arc<crate::players_backend_settings::PlayersBackendSettings>,
    player_index: u32,
    controller: usize,
    input: DeviceInput<C>,
    queue: SelectedSimCommandQueue,
    random: SimulationCommandRandom,
}
#[derive(Debug)]
pub(crate) enum ProductionInputFrameError<P> {
    FrontendMap(InputMapReadError<crate::input_pad::InputPadError>),
    Device(SelectedDeviceInputError<P, Infallible, JuiceBootError<Infallible>>),
    FocusLock,
    Focus(CommandQueueError),
    Player(
        crate::input_pad::device_manager::selected_runtime::player_sample::SelectedPlayerSampleError<
            crate::selected_pause_manager::PauseError,
            CommandQueueError,
        >,
    ),
    Queue(QueuePlayerInputError<ReckoningHostNumericError>),
    ForeignPlayer { captured: u32, supplied: u32 },
    Sync(CommandQueueError),
}

#[derive(Debug)]
pub(crate) enum SessionMarkerStateError {
    Pad(InputPadCacheError),
    Map(InputMapReadError<crate::input_pad::InputPadError>),
}

#[derive(Debug)]
pub(crate) enum PlayerUiTeleportError {
    Command(CommandQueueError),
    Pause(crate::selected_pause_manager::PauseError),
    Player(crate::players_backend_settings::PlayersSettingsError),
    FocusLock,
}
impl<C> SelectedProductionInputOwner<C> {
    /// `TimerCallback82729770`: poll, SIM cache, PRES cache; host owns cadence.
    pub(crate) fn poll_and_cache<P: PadPlatform>(
        &mut self,
        pad: &mut P,
    ) -> Result<(), ProductionInputFrameError<P::Error>> {
        self.input
            .poll_hardware(pad)
            .map_err(ProductionInputFrameError::Device)?;
        <DeviceInput<C> as SelectedDeviceInputEvents<P>>::cache_input(
            &mut self.input,
            SelectedInputGroup::Simulation,
        )
        .map_err(ProductionInputFrameError::Device)?;
        <DeviceInput<C> as SelectedDeviceInputEvents<P>>::cache_input(
            &mut self.input,
            SelectedInputGroup::Presentation,
        )
        .map_err(ProductionInputFrameError::Device)
    }

    /// The `Vibration` of this owner's controller (`PlayerUI::mControllerIndex`).
    pub(crate) fn vibration_mut(&mut self) -> Option<&mut crate::input_pad::vibration::Vibration> {
        self.input.vibration_mut(self.controller)
    }

    /// `InputManager::UpdateVibration`: every pad's rumble into its actuators.
    pub(crate) fn try_update_vibration<P: PadPlatform>(
        &mut self,
        pad: &mut P,
        cvars: &InputCvarReferences,
    ) -> Result<(), PadActuatorError<P::Error, ReckoningHostNumericError>> {
        self.input.try_update_vibration(pad, cvars, &mut HostReckoningNumeric)
    }

    /// `PlayerUI::ReceiveRumbleResetMsg` 0x828aa390: this controller's `Vibration::Reset`.
    pub(crate) fn try_reset_vibration<P: PadPlatform>(
        &mut self,
        pad: &mut P,
        cvars: &InputCvarReferences,
    ) -> Result<(), PadActuatorError<P::Error, ReckoningHostNumericError>> {
        self.input.try_reset_vibration(self.controller, pad, cvars, &mut HostReckoningNumeric)
    }

    /// Host calls after native preceding network/session effects, with actual
    /// focus/can-simulate and `PhysicalGameInfo`. The pad argument selects the
    /// same concrete platform type as the timer event; no OS call occurs here.
    pub(crate) fn try_preupdate_enqueue<P: PadPlatform>(
        &mut self,
        pad: &mut P,
        loader: &PlayerLoaderSlot,
        can_simulate: bool,
        has_focus: bool,
        game_info: &PhysicalGameInfo,
    ) -> Result<bool, ProductionInputFrameError<P::Error>> {
        if !self.try_last_input(pad, loader)? {
            return Ok(false);
        }
        if !self.try_player_ui_prefix::<P>(can_simulate, has_focus)? {
            return Ok(false);
        }
        self.try_player_ui_sample_and_sync::<P>(loader, has_focus, None, game_info)
    }

    /// `Sk8AppMan::PreUpdate`8272c4d8 `InputManager::LastInput(kINPUT_SIM)`:
    /// false returns before Flow, and GameModule's `mInputUpdate` stays clear.
    pub(crate) fn try_last_input<P: PadPlatform>(
        &mut self,
        _pad: &mut P,
        loader: &PlayerLoaderSlot,
    ) -> Result<bool, ProductionInputFrameError<P::Error>> {
        if loader.player_index() != self.player_index {
            return Err(ProductionInputFrameError::ForeignPlayer {
                captured: self.player_index,
                supplied: loader.player_index(),
            });
        }
        <DeviceInput<C> as SelectedDeviceInputEvents<P>>::last_input(
            &mut self.input,
            SelectedInputGroup::Simulation,
        )
        .map_err(ProductionInputFrameError::Device)
    }

    /// `PlayerUI::Update`828af9f8 through its focus prefix; false is the
    /// no-simulation return. `UpdateSessionMarker` follows a true result.
    pub(crate) fn try_player_ui_prefix<P: PadPlatform>(
        &mut self,
        can_simulate: bool,
        has_focus: bool,
    ) -> Result<bool, ProductionInputFrameError<P::Error>> {
        self.focus
            .lock()
            .map_err(|_| ProductionInputFrameError::FocusLock)?
            .try_update_prefix(can_simulate, has_focus, &mut self.queue)
            .map_err(ProductionInputFrameError::Focus)
    }

    /// The rest of `PlayerUI::Update` (sampling and proxy commands), then
    /// `GameModule::DoGameUpdate`'s `GenerateSyncCommand`. `offboard` holds the
    /// frames of the 1.02 off-board stick while its skater is off its board.
    pub(crate) fn try_player_ui_sample_and_sync<P: PadPlatform>(
        &mut self,
        loader: &PlayerLoaderSlot,
        has_focus: bool,
        offboard: Option<
            &crate::input_pad::device_manager::selected_runtime::player_sample::OffBoardStickFrames,
        >,
        game_info: &PhysicalGameInfo,
    ) -> Result<bool, ProductionInputFrameError<P::Error>> {
        let mut pause = Arc::clone(&self.pause);
        let sample = self
            .input
            .try_player_sample(
                loader,
                self.controller,
                game_info,
                has_focus,
                offboard,
                &mut pause,
                &mut self.queue,
            )
            .map_err(ProductionInputFrameError::Player)?;
        sample
            .try_queue_into(&mut self.queue, &mut HostReckoningNumeric)
            .map_err(ProductionInputFrameError::Queue)?;
        self.queue
            .try_generate_sync_command()
            .map_err(ProductionInputFrameError::Sync)
    }

    /// `cInputMap::GetState`8298c8c8 (nonzero) of a session marker state on
    /// this controller's drained SIM pad, as `UpdateSessionMarker` reads it.
    pub(crate) fn try_session_marker_state(
        &self,
        state: u32,
    ) -> Result<bool, SessionMarkerStateError> {
        let pad = self
            .input
            .cache(SelectedInputGroup::Simulation)
            .pad(self.controller)
            .map_err(SessionMarkerStateError::Pad)?;
        Ok(self
            .input
            .map()
            .try_value(state, &mut RetainedPadView(pad))
            .map_err(SessionMarkerStateError::Map)?
            != 0.0)
    }

    /// Frontend consumes the same timer's PRES sample, retaining native edges
    /// and 24/12-tick repeats even while simulation is paused.
    pub(crate) fn try_pause_menu_input<P: PadPlatform>(
        &mut self,
    ) -> Result<crate::ui::pause_menu::PauseMenuInput, ProductionInputFrameError<P::Error>> {
        <DeviceInput<C> as SelectedDeviceInputEvents<P>>::last_input(
            &mut self.input, SelectedInputGroup::Presentation,
        ).map_err(ProductionInputFrameError::Device)?;
        let pad = self.input.cache(SelectedInputGroup::Presentation).pad(self.controller)
            .map_err(|e| ProductionInputFrameError::Device(SelectedDeviceInputError::Cache(e)))?;
        let mut values = [0.0; 20];
        for (result, state) in values.iter_mut().zip([134, 136, 137, 146, 147, 148, 149, 205, 35, 139, 144, 141, 138, 164, 165, 135, 188, 185, 150, 151]) {
            *result = self.input.map().try_value(state, &mut RetainedPadView(pad))
                .map_err(ProductionInputFrameError::FrontendMap)?;
        }
        Ok(crate::ui::pause_menu::PauseMenuInput {
            start: values[0] != 0.0, select: values[1] != 0.0, cancel: values[2] != 0.0, up: values[3] != 0.0, down: values[4] != 0.0,
            left: values[5] != 0.0, right: values[6] != 0.0, morph_axis: values[7], morph_axis_secondary: values[8],
            misc: values[9] != 0.0, left_shift: values[10] != 0.0, right_shift: values[11] != 0.0,
            option: values[12] != 0.0, left_released: values[13] != 0.0, right_released: values[14] != 0.0,
            back: values[15] != 0.0, left_shift_released: values[16] != 0.0, right_shift_released: values[17] != 0.0,
            dpad_up: values[18] != 0.0, dpad_down: values[19] != 0.0,
        })
    }

    /// TranslateEHudInputBit82641648 accepts FE keycodes16..19/32, which
    /// LoadInputMap826de5e8..618/6e8 binds to150..153/166. The generic
    /// AptUp/Down/Left/Right also contain stick navigation and are NOT HUD keys.
    /// Read the already-drained PRES sample; do not advance its history twice.
    pub(crate) fn try_hud_dpad_input<P: PadPlatform>(
        &self,
    ) -> Result<[bool; 5], ProductionInputFrameError<P::Error>> {
        let pad = self.input.cache(SelectedInputGroup::Presentation).pad(self.controller)
            .map_err(|e| ProductionInputFrameError::Device(SelectedDeviceInputError::Cache(e)))?;
        let mut values = [false; 5];
        for (value, state) in values.iter_mut().zip([150, 151, 152, 153, 166]) {
            *value = self.input.map().try_value(state, &mut RetainedPadView(pad))
                .map_err(ProductionInputFrameError::FrontendMap)? != 0.0;
        }
        Ok(values)
    }

    /// PhonelistHUD UpdateInput826c0848: LB maximizes; AptOptionReleased
    /// (FE key48 / input182) opens Contacts. Reuse the drained PRES sample.
    pub(crate) fn try_phone_options_input<P: PadPlatform>(&self)
        -> Result<bool,ProductionInputFrameError<P::Error>> {
        let pad=self.input.cache(SelectedInputGroup::Presentation).pad(self.controller)
            .map_err(|e|ProductionInputFrameError::Device(SelectedDeviceInputError::Cache(e)))?;
        let mut value=|state|self.input.map().try_value(state,&mut RetainedPadView(pad))
            .map_err(ProductionInputFrameError::FrontendMap);
        Ok(value(29)?!=0. && value(182)?!=0.)
    }

    /// LB held, and B pressed with it (PC port addition: LB's quick menu moves an online game's
    /// Spectate on to the next player), from the same drained PRES sample.
    pub(crate) fn try_spectate_cycle_input<P: PadPlatform>(&self)
        -> Result<(bool, bool), ProductionInputFrameError<P::Error>> {
        let pad=self.input.cache(SelectedInputGroup::Presentation).pad(self.controller)
            .map_err(|e|ProductionInputFrameError::Device(SelectedDeviceInputError::Cache(e)))?;
        let mut value=|state|self.input.map().try_value(state,&mut RetainedPadView(pad))
            .map_err(ProductionInputFrameError::FrontendMap);
        let held = value(29)? != 0.;
        Ok((held, held && value(137)? != 0.))
    }

    pub(crate) fn try_can_open_frontend(&self) -> Result<bool, crate::selected_pause_manager::PauseError> {
        self.pause.try_can_open_frontend()
    }

    /// LoadInputMap826de438 and Replay2::ReceiveInput82682b68. Frontend
    /// already drained PRES; retain its edges and 24/12-tick navigation repeats.
    pub(crate) fn try_replay_editor_input<P: PadPlatform>(&self)
        -> Result<crate::ui::replay_editor::Input,ProductionInputFrameError<P::Error>> {
        let pad=self.input.cache(SelectedInputGroup::Presentation).pad(self.controller)
            .map_err(|e|ProductionInputFrameError::Device(SelectedDeviceInputError::Cache(e)))?;
        let mut value=|state|self.input.map().try_value(state,&mut RetainedPadView(pad))
            .map_err(ProductionInputFrameError::FrontendMap);
        let mut out=crate::ui::replay_editor::Input::default();
        for (state,key) in [(134,0),(136,2),(137,3),(139,5),(140,6),(141,7),(143,9),(144,10),(145,11),
            (146,12),(147,13),(148,14),(149,15),(152,18),(153,19),(166,32),(167,33),(168,34),(169,35),(187,53)] {
            if value(state)?!=0. {out.keys.push(key);}
        }
        out.left=[value(205)?,value(207)?];out.right=[value(204)?,value(206)?];
        out.triggers=[value(80)?,value(81)?];
        out.dpad=[value(84)?!=0.,value(85)?!=0.,value(86)?!=0.,value(87)?!=0.];
        Ok(out)
    }

    /// ReplayEditorBackEnd::Start82567450/Stop825675d0 own pause state7.
    pub(crate) fn try_set_replay_pause(&self,paused:bool) -> Result<(),crate::selected_pause_manager::PauseError> {
        if paused {self.pause.try_push(7)} else {self.pause.try_pull(7)}
    }

    pub(crate) fn try_set_frontend_pause(&self, paused: bool) -> Result<(), crate::selected_pause_manager::PauseError> {
        // StateManagerFrontEnd::PauseGame82647910 / UnpauseGame: FRONTEND=3.
        if paused { self.pause.try_push(3) } else { self.pause.try_pull(3) }
    }

    /// `kPAUSE_STATE_ONLINE`, which `Online.OnlineChallengeRoundSyncPoint` (`0x8289adb8`) pulls and
    /// pushes and the sync point's completion (`StateOnlineGame::OnMessage` `0x8276b9cc`) pulls.
    pub(crate) fn try_set_online_pause(&self, paused: bool) -> Result<(), crate::selected_pause_manager::PauseError> {
        self.pause.try_pull(12)?;
        if paused { self.pause.try_push(12) } else { Ok(()) }
    }

    /// `Net::ResumeGame` (`0x82791ca8`) / `Net::PauseGame` (`0x82791a88`) in an online game.
    pub(crate) fn try_set_network_resumed(&self, resumed: bool) -> Result<(), crate::selected_pause_manager::PauseError> {
        self.pause.try_set_network_resumed(resumed)
    }

    /// `NISManager::PauseSim` (`0x828a1070`) / `ResumeSim` (`0x828a1170`):
    /// `kPAUSE_STATE_NIS`.
    pub(crate) fn try_set_nis_pause(&self, paused: bool) -> Result<(), crate::selected_pause_manager::PauseError> {
        let state = crate::nis::manager::PAUSE_STATE_NIS;
        if paused { self.pause.try_push(state) } else { self.pause.try_pull(state) }
    }

    /// `PauseMan::IsPaused(kPAUSE_FLAG_SIM)` (`0x824e4778`).
    pub(crate) fn try_is_simulation_paused(&self) -> Result<bool, crate::selected_pause_manager::PauseError> {
        self.pause.try_is_paused(1)
    }

    pub(crate) const fn player_index(&self) -> u32 {
        self.player_index
    }

    /// `PlayerUI::mControllerIndex`.
    pub(crate) const fn controller(&self) -> usize {
        self.controller
    }

    /// The front end's simulation frame's sync command. Natively `PlayerUI::Update` 0x828af9f8
    /// first sends the unfocused first player an empty input, which `PlayerInputData::Unpack`
    /// 0x82a8c330 reads as Brake held; the port's front end pins that player instead.
    pub(crate) fn try_generate_frontend_sync_command(
        &mut self,
    ) -> Result<bool, crate::selected_sim_command_queue::CommandQueueError> {
        self.queue.try_generate_sync_command()
    }

    pub(crate) fn queued_tick_input(&mut self) -> SelectedQueuedTickInput<'_> {
        SelectedQueuedTickInput {
            queue: &mut self.queue,
            random: &mut self.random,
        }
    }
}

/// `ReceiveTeleportMsg` reaches this PlayerUI's proxy after the frame's sync
/// command, so the next admitted batch carries the command.
impl<C> crate::session_marker::teleport::PlayerUiTeleportReceiver
    for SelectedProductionInputOwner<C>
{
    type Error = PlayerUiTeleportError;
    fn do_command(
        &mut self,
        command: &crate::game_input_manager::EncodedGameInputCommand<'_>,
    ) -> Result<(), Self::Error> {
        self.queue
            .try_queue(command)
            .map_err(PlayerUiTeleportError::Command)
    }
    fn pull_pause(&mut self, state: u32) -> Result<(), Self::Error> {
        self.pause
            .try_pull(state)
            .map_err(PlayerUiTeleportError::Pause)
    }
    /// `PartyPlayUI::SetPlayer`828aa4c0: `mPlayerIndex`, `mControllerIndex` from
    /// `mControllers` (the constructor's `GetPlayerControllerIndex` of each human,
    /// which the back end still holds) and `mFootForward` from the stance.
    fn set_player(&mut self, player: i32) -> Result<crate::session_marker::FootForward, Self::Error> {
        let index = u32::from_ne_bytes(player.to_ne_bytes());
        let controller = self.settings.try_controller_index(index).map_err(PlayerUiTeleportError::Player)?;
        let regular = self.settings.try_stance_byte(index).map_err(PlayerUiTeleportError::Player)?;
        self.player_index = index;
        self.controller = usize::try_from(u32::from_ne_bytes(controller.to_ne_bytes())).expect("native32 controller word fits supported host");
        self.focus.lock().map_err(|_| PlayerUiTeleportError::FocusLock)?.set_player_index(index);
        Ok(u32::from(regular != 0))
    }
}

impl<C> SelectedProductionInputOwner<C> {
    pub(crate) fn simulation_random_mut(&mut self)->&mut SimulationCommandRandom{&mut self.random}
}
