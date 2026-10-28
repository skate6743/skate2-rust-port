//! Career progression on the live session. The challenge system runs once per
//! completed simulation update in the native order: the
//! `PhysOutModule_MissionSystem` messages, `Sk8ChallengeSystem::Update`
//! (`Simulation::SimUpdate2`), `cTriggerVolumeManager::Update`, then
//! `EventSys::Flush` (`Simulation::PostUpdate` `0x8289ff24`).
use super::{
    SelectedLiveError, SelectedLiveSession, SelectedLiveStage,
    scene_characters::{CareerSceneSource, SceneCharacterModels},
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    input_cvar_runtime::InputCvarReferences,
    challenge::{
        ChallengeSystem, Event, Output,
        attrib_db::AttribDb,
        cas_unlock_parts::{self, CasUnlockPartDb},
        dynamic_hull,
        hud::HudBinding,
        keys::{class, hash_value32},
        mission::{self, MissionFrame, MissionSystem, MissionTuning},
        objective_hud::ObjectiveHudView,
        objectives::{self, Status},
        results::{self, EndChallengeOptions, LeaderboardData, OptionAction},
        signpost_progress,
        signup_hud::SignUpHudView,
        signup_poster::{PosterAccept, PosterView},
        system::{Message, Skate101View},
        trigger_tester::{self, TesterTuning},
        turn_hud::TurnHudView,
        tutorial_hud::TutorialHudView,
        unlock_overlay::UnlockOverlayView,
        unlockables,
    },
    edit_skater_profile::{AudioProfile, EditSkaterProfileStore, HandedSave},
    grind_data_octree::OctreeBounds,
    narrow_phase::{GpPrimitive, rw_overlap},
    nis::{
        self,
        manager::{FrozenFocus, NisCamera, NisEvent, NisSystem, NisView},
    },
    ui::party_play::{self, ChangeAction, ChangeGame, ChangeSnapshot},
};
use skate2_formats::{
    localization::{Histogram, LanguageCategory, string_hash},
    location::LocationTable,
    mount::MountSet,
    skater_profile::{MemcardLeaf, ProfileData},
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

const GLOBAL_LUA: &str = "data\\script\\challenge\\global.lua";
const SIM_LOCATIONS: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";
/// `physics_wipeout` class, `default` collection, and the almost-over time
/// `PhysOutModule_MissionSystem::GenerateScoringMessages` compares.
const PHYSICS_WIPEOUT: u64 = 0xeb84_4cfd_bf92_8efd;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
const WIPEOUT_ALMOST_OVER: u64 = 0xb565_383e_5da5_382b;
/// The `fe_locations` collection `FrontEndState_MainMenu::EnterGame` starts a
/// continued career at without a saved orientation.
const CONTINUE_START: u64 = 0x2e33_dbe1_d6b3_3472;
/// `challenges.Title`.
const CHALLENGE_TITLE: u64 = 0xe2a5_0788_b6c2_0c92;

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Career, e.to_string())
}

pub(super) fn leaf_tuples(leaf: &Option<MemcardLeaf>) -> signpost_progress::Saved<'_> {
    leaf.as_ref().map(|l| (l.version, l.tuples.as_slice()))
}

/// `cPathedHullLoader::LoadHull` (`0x82eeedc8`) and `UnloadHull` (`0x82eee9c0`): the asset
/// manager's hull requests, queued for the loader's next update.
/// The career's answers to the challenge system's queries: trigger volumes from the standard
/// trigger groups and locations from the career's `cLocationManager`. The DMO queries are not
/// answered.
struct CareerHost {
    triggers: Arc<Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
    locations: Arc<Mutex<locations::LocationManager>>,
    photos: nis::manager::PhotoCounter,
    mounts: Option<Arc<MountSet>>,
}

impl crate::challenge::host::Host for CareerHost {
    fn dmo_locator_from_lua_id(&mut self, _: f32) -> Option<u64> {
        None
    }
    fn is_dmo_in_volume(&mut self, _: u64, _: f32) -> bool {
        false
    }
    fn trigger_volume_center(&mut self, volume_id: f32) -> Option<[f32; 3]> {
        let triggers = self.triggers.lock().ok()?;
        let record = triggers.try_find_volume_id(volume_id).ok()??;
        record.trigger_volume_center().ok()
    }
    fn race_gate_volume(&self, volume_id: f32) -> Option<([f32; 16], [f32; 3])> {
        let triggers = self.triggers.lock().ok()?;
        let record = triggers.try_find_volume_id(volume_id).ok()??;
        let (volume, transform) = record.trigger_bounding_volume().ok()??;
        Some((crate::world::compose_collision_affine(transform, volume.transform),
            volume.shape_words.map(f32::from_bits)))
    }
    fn crowd_volume(&self, volume_id: f32) -> Option<Option<([f32; 16], [f32; 4])>> {
        let triggers = self.triggers.lock().ok()?;
        let record = triggers.try_find_volume_id(volume_id).ok()??;
        let Some((volume, transform)) = record.trigger_bounding_volume().ok()? else { return Some(None) };
        let dimensions = if volume.volume_type() == Some(skate2_formats::collision::VolumeType::Box) {
            let [x, y, z] = volume.shape_words.map(f32::from_bits);
            [x, y, z, 0.0]
        } else {
            [volume.radius, volume.radius, volume.radius, 0.0]
        };
        Some(Some((crate::world::compose_collision_affine(transform, volume.transform), dimensions)))
    }
    fn location_orientation(&self, location: u32) -> Option<[f32; 16]> {
        self.locations.lock().ok()?.orientation(location)
    }
    fn nis_photo_count(&self, scene: &str) -> i32 {
        self.photos.count(scene)
    }
    fn localized(&self, label: &str) -> Option<String> {
        localized(self.mounts.as_deref()?, label).ok()
    }
}

/// A Challenge queue message as the challenge system receives it: `cMsgSkaterCreated`,
/// `cMsgSkaterDestroyed` and `cMsgLocalSkaterChanged` with their `PlayerID`, the radius pair
/// with their `Distance` and `PlayerID`, the local chase pair, `cMsgSecurityKnockedDown` with
/// its `ID`, and `cMsgSetMarker` and `cMsgUseMarker` with their `PlayerID`.
fn challenge_queue_event(message: crate::hermes_challenge_queue::ChallengeMessage) -> Event {
    use crate::challenge::system::Field;
    use crate::hermes_challenge_queue::{ChallengePayload, SkaterNotification};
    #[expect(clippy::cast_precision_loss, reason = "the messages' (float) fields")]
    let message = match message.payload {
        ChallengePayload::Skater { notification, player_id } => {
            let name = match notification {
                SkaterNotification::Created => "cMsgSkaterCreated",
                SkaterNotification::Destroyed => "cMsgSkaterDestroyed",
                SkaterNotification::LocalChanged => "cMsgLocalSkaterChanged",
            };
            Message::new(name, vec![("PlayerID", Field::Number(player_id as f32))])
        }
        ChallengePayload::PlayerRadius { enter, distance, player_id } => Message::new(
            if enter { "cMsgPlayerRadiusEnter" } else { "cMsgPlayerRadiusExit" },
            vec![("Distance", Field::Number(distance)), ("PlayerID", Field::Number(player_id as f32))],
        ),
        ChallengePayload::LocalChaseStart => Message::new("cMsgLocalChaseStart", Vec::new()),
        ChallengePayload::LocalChaseEnd => Message::new("cMsgLocalChaseEnd", Vec::new()),
        ChallengePayload::GuardKnockedDown { entity_id } => {
            Message::new("cMsgSecurityKnockedDown", vec![("ID", Field::Number(entity_id as f32))])
        }
        ChallengePayload::Marker { set, player_id } => Message::new(
            if set { "cMsgSetMarker" } else { "cMsgUseMarker" },
            vec![("PlayerID", Field::Number(player_id as f32))],
        ),
    };
    Event::Challenge(message)
}

fn queue_hull_requests(
    terrain: &mut (dyn crate::selected_session_boot_owner::terrain_input::SelectedSessionTerrainSlot + Send),
    hulls: Vec<Output>,
) -> Result<(), SelectedLiveError> {
    for request in hulls {
        match request {
            Output::LoadHull { file_name, register_on_load: true, high_priority, asset_flags, .. } => {
                terrain.load_hull(&file_name, high_priority, asset_flags).map_err(error)?;
            }
            Output::UnloadHull { hull } => terrain.unload_hull(hull).map_err(error)?,
            other => return Err(error(format!("unported hull request {other:?}"))),
        }
    }
    Ok(())
}

/// The dynamic hull manager's `cPathedHullLoader::LoadHull` (`0x82eeef40`) and `UnloadHull`
/// (`0x82eeec78`) requests, queued for its loader's next update.
pub(super) fn queue_dynamic_hull_requests(
    terrain: &mut (dyn crate::selected_session_boot_owner::terrain_input::SelectedSessionTerrainSlot + Send),
    hulls: Vec<Output>,
) -> Result<(), SelectedLiveError> {
    for request in hulls {
        match request {
            Output::LoadDynamicHull { file_name, callback, asset_flags, .. } => {
                terrain.load_dynamic_hull(&file_name, asset_flags, callback).map_err(error)?;
            }
            Output::UnloadDynamicHull { hull } => terrain.unload_dynamic_hull(hull).map_err(error)?,
            other => return Err(error(format!("unported dynamic hull request {other:?}"))),
        }
    }
    Ok(())
}

fn is_dynamic_hull_request(output: &Output) -> bool {
    matches!(output, Output::LoadDynamicHull { .. } | Output::UnloadDynamicHull { .. })
}

/// `StateLoading::DynamicHullState` (`0x82763be8`) once a travel's stream focus is in: the
/// dynamic hulls are spot-tested at the destination (`SpotTestAgainstPoint` `0x82db4318`; again
/// each update, which asks for nothing more) and the loader updates (`SpotTestUpdate`
/// `0x82da16b8`); `true` once it loads nothing (`IsLoading` `0x82da16b0`).
pub(super) fn try_dynamic_hull_state(
    career: Option<&mut CareerSession>,
    free_skate: Option<&mut ChallengeSystem>,
    terrain: &mut (dyn crate::selected_session_boot_owner::terrain_input::SelectedSessionTerrainSlot + Send),
    position: [f32; 3],
) -> Result<bool, SelectedLiveError> {
    let Some(system) = career.map(|career| &mut career.system).or(free_skate) else { return Ok(true) };
    system.spot_test_dynamic_hulls(position);
    update_dynamic_hull_loader(system, terrain)?;
    Ok(!terrain.dynamic_hulls_loading())
}

/// The manager's requests to its loader, the loader's update, then each loaded hull's callback.
fn update_dynamic_hull_loader(
    system: &mut ChallengeSystem,
    terrain: &mut (dyn crate::selected_session_boot_owner::terrain_input::SelectedSessionTerrainSlot + Send),
) -> Result<(), SelectedLiveError> {
    let outputs = std::mem::take(&mut system.world_mut().outputs);
    let (hulls, rest): (Vec<_>, Vec<_>) = outputs.into_iter().partition(is_dynamic_hull_request);
    system.world_mut().outputs = rest;
    queue_dynamic_hull_requests(terrain, hulls)?;
    for hull in terrain.try_update_dynamic_hulls().map_err(error)? {
        system.dynamic_hull_load_complete(hull);
    }
    Ok(())
}

/// The profile's `Sk8ChallengeSystem` as the front end has it, with no career started: the
/// unlockables (`UnlockablesBackEnd::LoadData` 0x8255cd08) and the SignPost savable
/// (`cSignPostManager::LoadData` 0x825e53d8) loaded over a fresh system.
pub(super) fn profile_challenge_system(db: Arc<AttribDb>, mounts: &MountSet) -> Result<ChallengeSystem, String> {
    let global = mounts.read_path(GLOBAL_LUA).map_err(|e| e.to_string())?;
    let mut system = ChallengeSystem::new_career(db, &global).map_err(|e| e.to_string())?;
    let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let unlocks = store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(|e| e.to_string())?;
    system.world_mut().unlockables.load(leaf_tuples(&unlocks));
    let leaf = |name: &str| store.load_leaf(signpost_progress::SAVABLE, name).map_err(|e| e.to_string());
    let saved = [leaf(signpost_progress::DATA)?, leaf(signpost_progress::POSITION)?, leaf(signpost_progress::MESSAGES)?];
    signpost_progress::load(system.world_mut(), leaf_tuples(&saved[0]), leaf_tuples(&saved[1]), leaf_tuples(&saved[2]));
    Ok(system)
}

/// `EA::Localizer::LocalizerManager::GetPackedString` from the English
/// tables.
pub(super) fn localized(mounts: &MountSet, label: &str) -> Result<String, SelectedLiveError> {
    let lang = "data\\fe\\languages\\english\\LANGUAGE_English";
    let histogram = Histogram::parse(&mounts.read_path(&format!("{lang}_HISTOGRAM_skate2ng.BIN")).map_err(error)?).map_err(error)?;
    for category in ["Global", "FrontEnd"] {
        let bytes = mounts.read_path(&format!("{lang}_{category}_skate2ng.BIN")).map_err(error)?;
        let table = LanguageCategory::parse(&bytes).map_err(error)?;
        if let Some(packed) = table.packed_string(string_hash(label.as_bytes())) {
            let text = histogram.unpack(packed, 1024, false).map_err(error)?;
            return String::from_utf16(&text.units).map_err(error);
        }
    }
    Err(error(format!("missing localized string {label}")))
}

/// `InstanceFrozenSkater`'s scene character: the local skater as a fixed-frame photo caught it.
pub(super) struct FrozenSkater {
    pub(super) palette: crate::character::palette::CharacterPalette,
    pub(super) hips: [f32; 3],
    /// World bounds, min then max.
    pub(super) bounds: [[f32; 3]; 2],
    pub(super) root: [f32; 16],
    /// Its own renderable's materials, built without cloth.
    pub(super) materials: crate::character::material_admission::CharacterMaterialAdmission,
}

/// The career's autosaves being written: one thread at a time, in the order they were made.
#[derive(Default)]
pub(super) struct CareerSaves {
    writing: Option<std::thread::JoinHandle<()>>,
}

impl CareerSaves {
    pub(super) fn busy(&self)->bool {self.writing.as_ref().is_some_and(|w|!w.is_finished())}
    pub(super) fn start(&mut self, write: impl FnOnce() + Send + 'static) {
        self.finish();
        match std::thread::Builder::new().name("career save".into()).spawn(write) {
            Ok(writing) => self.writing = Some(writing),
            Err(e) => eprintln!("career: autosave failed: {e}"),
        }
    }

    /// Waits for the save being written.
    fn finish(&mut self) {
        if let Some(writing) = self.writing.take() {
            let _ = writing.join();
        }
    }
}

impl Drop for CareerSaves {
    fn drop(&mut self) {
        self.finish();
    }
}

/// `FrontEndState_PhotoChallenge` (`0x54` bytes) as `FrontEnd.OpenPhotoSelection` opens it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhotoSelectView {
    /// `mSelectedPhotoIndex`.
    pub selected: i32,
    /// `mEnablePhotoPicking`: more than one photo.
    pub enable_picking: bool,
    /// `mNumPhotos`.
    pub num_photos: i32,
    /// `mRequestID`: the pick was made and its save asked for.
    pub requested: bool,
}

/// What `FEBackEnd`'s other custom Localizer arguments read: `POOL_FOUND` and `POOLTOBEFOUND`
/// (`ProcessPoolFound` `0x82563320`, `ProcessPoolAboutToBeFound` `0x825633e0`: the `Pools Found`
/// points pool, and one more), `TAPEGIVEN` (`ProcessTapesGiven` `0x82563960`: `Circulate Pool`),
/// `SponsorWorld` and `DemoWorld` (the signed and the demo sponsor's worlds) and `DEMOSPONSOR`
/// (`ProcessDemoSponsor` `0x82563780`: the HAL id it localizes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontendArguments {
    pub pools_found: i32,
    pub tapes_given: i32,
    pub sponsor_world: String,
    pub demo_world: String,
    pub demo_sponsor: String,
}

/// What the HUD shows of the running career.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CareerHud {
    /// ProcessPlayerName 0x8255a4d0 reads ProfileBackEnd +0x28, mProfileName.
    pub profile_name: String,
    /// ProcessBoard/Truck/Wheel/ShoeSponsor's current HAL IDs.
    pub sponsor_labels: [String; 4],
    pub contacts:Option<crate::challenge::contacts::View>,
    pub can_show_phone_options:bool,
    pub phone_service_icons:[bool;3],
    /// `PhonelistHUDData::MakeDirty` since the last take: `SetServiceIconStatus` (0x826b3748)
    /// dirties the data on every call, so the minimized menu asks the HUD again.
    pub phone_data_dirty:bool,
    pub phone_alert_facts:crate::ui::phonelist::AlertFacts,
    pub pro_film_pro_name:String,
    pub frontend_arguments: FrontendArguments,
    pub hom_results:Option<crate::challenge::hom_results::HomResultsView>,
    pub hom_annals:Option<crate::challenge::hom_annals::AnnalsView>,
    pub hom_completed:usize,
    /// `HUD2.ShowCentralMessage` messages not yet shown, with their format parameters.
    pub central_messages: Vec<crate::challenge::hud::CentralMessage>,
    /// The HAL key of the local player's active objective.
    pub objective: Option<String>,
    /// `SignUpHUD`: the title HAL keys of the challenges a D-pad-up press
    /// signs up for.
    pub sign_up_prompt: Vec<String>,
    /// `FrontEndState_SignUpPoster` (FE state 32): what its screen reads,
    /// answered with A (accept) or B (decline) once its intro is done.
    pub poster: Option<PosterView>,
    /// The several-challenges popup: titles and the highlighted option.
    pub sign_up_choice: Option<(Vec<String>, usize)>,
    /// `ReceiveProgPopupOpenerMsg` (`0x82638748`): the progression dialog's
    /// title and description HAL keys and the highlighted option (0
    /// `ID_COMMON_NO`, the default, or 1 `ID_COMMON_YES`).
    pub dialog: Option<(String, String, usize)>,
    /// A script's generic popup's options in place of `ID_COMMON_NO` and `ID_COMMON_YES`
    /// (`FrontEnd.ShowGenericAttributePopup`).
    pub dialog_options: Option<Vec<String>>,
    /// `FrontEndState_Unlock` (FE state 10) is open: what its screen reads,
    /// answered with A once the screen's cash count is done.
    pub unlock_overlay: Option<UnlockOverlayView>,
    /// `FrontEndState_Skate101` (FE state 79) is open: the title and text
    /// its screen shows, answered with A once the screen is ready.
    pub skate101: Option<Skate101View>,
    /// The Visual Director's fade info of the update (`VDReaderData::bFadeInfo` and
    /// `sFadeInfo`): the last `cMsgFadeAmount` the challenge effects sent
    /// (`cChallengeEffectsManager::Update` `0x82daa1a0`), none while no fade runs. The
    /// presentation draws it in `PostFX::VisualFX_Animation` (`0x82847610`) as a bloom.
    pub fade: Option<f32>,
    /// The Visual Director's challenge success and failure info of the last simulation submit
    /// (`VDReaderData::bChallengeSuccessInfo` and `bChallengeFailedInfo`), which
    /// `VisualFX_Animation` draws ahead of the fade.
    pub result_info: crate::challenge::vd_results::ResultInfo,
    /// `SetScreenState` calls on the objective HUD screen since the last
    /// hand-off, each with what the screen reads at that moment.
    pub objective_screen: Vec<(i32, ObjectiveHudView)>,
    /// The same for the tutorial instruction HUD screen.
    pub tutorial_screen: Vec<(i32, TutorialHudView)>,
    /// The same for the sign-up HUD's action overlay.
    pub signup_screen: Vec<(i32, SignUpHudView)>,
    /// The same for the turn HUD components: the leaderboard, the ABD trick list, the timer,
    /// the countdown, the vote, the throw-down message, the DPO panel and the distance meter.
    pub leaderboard_screen: Vec<(i32, TurnHudView)>,
    pub abd_screen: Vec<(i32, TurnHudView)>,
    pub timer_screen: Vec<(i32, TurnHudView)>,
    pub big_black_timer_screen:Vec<(i32,TurnHudView)>,
    pub countdown_screen: Vec<(i32, TurnHudView)>,
    pub voting_screen: Vec<(i32, TurnHudView)>,
    pub throwdown_screen: Vec<(i32, TurnHudView)>,
    pub dpo_screen: Vec<(i32, TurnHudView)>,
    pub distance_screen: Vec<(i32, TurnHudView)>,
    /// What a running NIS shows: its camera, the bloom to white, the HUD's
    /// NIS hiding, the subtitle line and the visible entity types.
    pub nis: NisView,
    /// `cMsgNISLoadingOverlay`: the FE loading tag.
    pub nis_loading: bool,
    /// What the front end's screen manager did with `FrontEndState_NIS`'s
    /// overlay screens since the last hand-off, in order.
    pub nis_screens: Vec<nis::fe_overlays::ScreenCommand>,
    /// `Mission.GetTitle()`, which the spot found banner's `SetState` reads.
    pub mission_title: crate::challenge::system::MissionTitle,
    /// `NIS.GetProInfo()`, which the pro bio overlay's intros read.
    pub pro_info: [String; 3],
    /// `ProfileBackEnd::tSettings::mSubtitles`: `FESubtitle::SetSubtitleText`
    /// (`0x82615ec8`) formats a line and `RefreshSubtitle` (`0x82614c80`) shows
    /// it only when set.
    pub subtitles: bool,
    /// `HUD2.ShowModifiedCentralMessage` HAL keys not yet shown.
    pub modified_central_messages: Vec<String>,
    /// `FrontEndState_PressA` is open: the turn it shows, answered with that
    /// player's A.
    pub press_a: Option<PressADialog>,
    /// `FrontEndState_EndChallengeOptions` (FE state 8) is open over the leaderboard.
    pub end_options: Option<EndOptionsView>,
    /// `FrontEndState_PhotoChallenge` (FE state 5) is open over a photo outro: what its
    /// screen (116, `challenge/photoselect`) reads.
    pub photo_select: Option<PhotoSelectView>,
    /// Pending `eBeMemcardSave_CareerPhoto`; completion returns through PhotoDone.
    pub photo_save_slot: Option<u32>,
    /// Its `SetScreenState` calls since the last take: the shown photo, -1 and -2 for the
    /// next and previous, -3 and -10 when one is picked.
    pub photo_select_screen: Vec<i32>,
    /// `FrontEndState_PartyPlayChangeGame` (FE state 39) is open, with what its screen reads.
    pub change_game: Option<ChangeSnapshot>,
    /// Its screen states since the last take.
    pub change_game_screen: Vec<u32>,
    /// What the free-skate HUD components are bound to.
    pub binding: HudBinding,
    /// What the start hub's screen reads of the career.
    pub start_hub: crate::challenge::start_hub::StartHubView,
    /// `FrontEndState_Sponsor` (FE state 11) is open: what its screen reads and its sponsor
    /// selector.
    pub sponsor_overlay: Option<crate::challenge::sponsor_overlay::SponsorOverlayHud>,
    /// Slappy's blog, an article or a cover (FE states 61 to 63) is on top.
    pub slappys: Option<crate::challenge::slappys_blog::SlappysHud>,
    /// The top blog screen's class calls since the last update.
    pub slappys_screen: Vec<crate::challenge::slappys_blog::SlappysCall>,
}

/// A front-end state whose `sFEStateInfo::mbPause` holds the game while it is
/// on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PausingState {
    Contacts,
    /// FrontEndState_HOMResults9, gFEStateInfo82fded80 +9*20.
    HomResults,
    AnnalsOfMeat,
    /// `FrontEndState_Unlock` (10).
    Unlock,
    /// `FrontEndState_Skate101` (79).
    Skate101,
    /// `FrontEndState_EndChallengeOptions` (8).
    EndChallengeOptions,
    /// `FrontEndState_PartyPlayChangeGame` (39).
    ChangeGame,
    /// `FrontEndState_Sponsor` (11).
    Sponsor,
    /// `FrontEndState_SlappysBlog` (61).
    Blog,
    /// `FrontEndState_SlappysArticles` (62).
    Articles,
    /// `FrontEndState_SlappysCovers` (63).
    Covers,
}

/// Party Play's game: `CommonBackEnd::sGameInfo.challengeKey` and the Party Play back end's
/// selection, which Change Game steps.
struct PartyState {
    challenge: u64,
    backend: party_play::BackEnd,
    /// `StateLoading::PartyPlayLoadState` (0x82770ea8): the challenge waits for its assets.
    loading: bool,
}

/// The career branch of `StateLoading::StartChallengeState8274f800`, entered after map
/// travel, followed by `WorldPrepareState8274f668`'s six rendered frames under the cover.
enum MapChallengeLoad {
    Teleport { key: u64 },
    StartChallenge { key: u64, ready_updates: u32 },
    WorldPrepare { rendered_frames: u32 },
}

/// Whose popup the yes/no dialog is: `ReceiveProgPopupOpenerMsg`'s progression popup, or
/// FE state 8's confirmations, whose callbacks act on yes only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DialogOwner {
    #[default]
    Progression,
    /// A script's popup and its id (`FrontEnd.ShowYesNoPopup`, `OpenProgressionPopupCallback`
    /// `0x82897720`, or with `generic` `ShowGenericAttributePopup`, `GenericPopupCallback`
    /// `0x82897910`).
    Script { id: i32, generic: bool },
    /// `eSk8FEPopup_RematchConfirmation` (157): `ConfirmRematchPopupCallback` (`0x82658db0`).
    Rematch,
    /// `eSk8FEPopup_QuitGameConfirmationNoDataLoss` (153): `QuitPopupCallback`
    /// (`0x82658ed8`).
    Quit,
    /// `eSk8FEPopup_Skateshop_SponsorConfirmation` (177): `FrontEndState_Sponsor::
    /// SponsorSelectedCallback` (`0x8266ff20`).
    SponsorConfirm,
    /// A popup without a callback (Slappy's blog's Annals of Meat warning).
    Notice,
    /// `eSk8FEPopup_ChallengeInfo_FeeFailed` (203): `FrontEndState_SignUpPoster::
    /// CantPayFeeCallback` (`0x82641ef8`) clears `mActionsBlocked`, which the failed accept left
    /// clear.
    FeeFailed,
    /// `eSk8FEPopup_Online_ConfirmQuitOnline` (52): `ConfirmLeaveOnlinePopupCallback`
    /// (`0x82658d78`) pops FE state 8 and enters the front end.
    LeaveOnline,
}

/// A challenge request on the skaters, applied in order after the system updates.
enum SkaterRequest {
    SetPlayer(i32),
    SetLocalIndex(i32),
    Pin(bool),
    PinActor(i32, bool),
    Teleport([f32; 16]),
    TeleportPlayer(i32, [f32; 16]),
}

/// A session-marker message of the teleport manager's broadcaster, in the order the scripts
/// sent them.
enum MarkerMessage {
    /// `cMsgSetSessionMarker` at the transform, or at the local skater's respawn transform.
    Set(Option<[f32; 16]>),
    /// `cMsgClearSessionMarker`.
    Clear,
}

/// `FrontEndState_PressA` (FE state 38, screen 64): `mPlayerTurn` (1-based) and the
/// round it shows, with what `Game.PartyPlay_GetDataBlock` (`0x82622768`) adds for its
/// screen: the turn player's `GetPlayerControllerIndex` and `GetPlayerName`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PressADialog {
    pub player_turn: i32,
    pub round: i32,
    pub controller: i32,
    pub name: String,
}

/// `FrontEndState_EndChallengeOptions` (FE state 8) and what its screen's results list reads:
/// the `FrontEndState_LeaderBoard` (FE state 27) current and second current leaderboards and the
/// local player's stance (`PlayerStance_IsRegular` `0x826b4bd8`).
#[derive(Clone, Debug, PartialEq)]
pub struct EndOptionsView {
    pub options: EndChallengeOptions,
    pub leaderboard: LeaderboardData,
    pub second: LeaderboardData,
    pub stance_regular: bool,
    /// Results_GetGoofyTeamScoreRegTeamScore826b9a90 reads final player 0/1 scores.
    pub team_totals: [i32; 2],
    /// The online display's data (`mIsOnlineDisplay`).
    pub online: Option<results::OnlineResults>,
}

impl EndOptionsView {
    fn new(options: EndChallengeOptions, world: &crate::challenge::system::World) -> Self {
        Self {
            options,
            leaderboard: world.results.current().clone(),
            second: world.results.second().clone(),
            stance_regular: world.players.as_deref().is_none_or(|players| players.is_stance_regular(world.local_player)),
            team_totals: std::array::from_fn(|i| world.results.final_results.players[i].score),
            online: None,
        }
    }
}

#[path = "selected_location_manager.rs"]
pub(super) mod locations;
#[path = "selected_session_career_living_world.rs"]
mod living_world;
#[path = "selected_session_career_movie.rs"]
mod movie;
#[path = "selected_session_career_start_hub.rs"]
mod start_hub;
#[path = "selected_session_career_sponsor.rs"]
mod sponsor;
#[path = "selected_session_career_blog.rs"]
mod blog;

impl crate::challenge::ai::SpawnLocations for Mutex<locations::LocationManager> {
    fn spawn_location(&self, name: &str, index: i32) -> Option<[f32; 16]> {
        self.lock().ok()?.spawn_location(hash_value32(name), index)
    }
    fn num_spawn_locations(&self, name: &str) -> usize {
        self.lock().map_or(0, |locations| locations.num_spawn_locations(hash_value32(name)))
    }
}

/// A scene's or speech line's event, held in an online round for its pacing leader to announce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HeldPresentation {
    /// `cMsgNISChallengeNotify` 1 (with `cMsgNISComplete`), 2 or 3.
    Nis(u8),
    /// `cMsgSpeechEnded`.
    Speech { character: i32, id: i32 },
}

pub(crate) struct CareerSession {
    pub(super) system: ChallengeSystem,
    mission: MissionSystem,
    tuning: MissionTuning,
    tester: TesterTuning,
    /// `cTriggerVolumeMessageGroup::PreviousCollisionIndicies` of each skater's
    /// tester by player, with each entry's global and local ids.
    previous: BTreeMap<i32, Vec<(u32, u64, u64)>>,
    /// `BAM_Sim.loc` transforms by `GetHashValue32(name)`.
    location_transforms: HashMap<u32, [f32; 16]>,
    /// `cLocationManager`'s streamed locations, which the challenge AI reads too.
    locations: Arc<Mutex<locations::LocationManager>>,
    hud: CareerHud,
    /// The poster's `mActionsBlocked`: `OnEnter` (`0x82647f68`) sets it,
    /// `OnIntroDone` (`0x826581a0`) clears it.
    poster_blocked: bool,
    /// The poster state's `mLocalInputState`: `UpdateOnEnter` (`0x82d663f0`) allows its input; an
    /// accept past the betting check blocks it (`0x82657da0`), and only a push or pop of a state
    /// over it would allow it again, so after the fee-failed popup the poster takes no input.
    poster_input_blocked: bool,
    /// challenge_info's `mBettingButtonReleased` and `mBettingTimer`, as its constructor sets them.
    poster_bet_input: (bool, i32),
    /// `FrontEndState_Unlock::mCashAnimDone` (+0x50): set by the screen's
    /// `Cash_SetCashAnimDone`, cleared by `OnIntroDone` (`0x826423c0`). The
    /// state lives for the whole game and its constructor leaves the flag
    /// unset; the port starts it false.
    cash_anim_done: bool,
    /// The FRONTEND pause the pausing states hold.
    overlay_paused: bool,
    /// The pausing states pushed (`StateManagerFrontEndBase::PushState`),
    /// the last on top.
    pausing: Vec<PausingState>,
    contacts:Option<crate::challenge::contacts::Contacts>,
    contacts_saving:bool,
    /// Each pausing state's screen state: `OnScreenReady` (`0x82d65f98`, run
    /// by the screen's `IntroComplete`) makes it ready, and
    /// `StateManagerFrontEnd::ReceiveInput` (`0x82656af8`) passes input only
    /// to a top state whose screen is ready (`IsScreenReady` `0x82d661a8`).
    unlock_ready: bool,
    skate101_ready: bool,
    hom_results_ready:bool,
    /// The results' Replay Editor asked for `PushState(12)`.
    hom_replay_editor:bool,
    annals_ready:bool,
    /// FE state 11's screen is ready.
    sponsor_ready: bool,
    /// FE state 11's `mSelectedSponsorIndex`, which outlives each overlay.
    sponsor_selected: usize,
    /// FE state 11 left; its screen's outro still has to end (`OnOutroDone`).
    sponsor_outro_pending: bool,
    /// FE states 61 to 63.
    slappys: blog::Slappys,
    end_options_ready: bool,
    end_options_play_once: bool,
    /// `mbWaitUntilOutroDone`: Continue or Quit left FE state 8, whose `OnOutroDone`
    /// (`0x826590c8`) then sends `cMsgHUDContinue`.
    end_options_wait_outro: bool,
    /// FE state 8 was pushed by NIS state 79; OnLeave must skip and pop that NIS.
    end_options_opened_by_nis: bool,
    /// Party Play's game, absent in the career.
    party: Option<PartyState>,
    /// An online challenge's random stream, seeded by the server's `GameReset` and lent to the
    /// system while it updates in place of the Simulation's.
    pub(super) online_random: Option<crate::native_skater_random::SimulationCommandRandom>,
    /// FE state 39 and whether its screen is ready.
    change_game: Option<ChangeGame>,
    change_game_ready: bool,
    /// FE state 8 under FE state 39: pushing 39 left it (`OnLeave`), and popping 39 enters it
    /// again (`OnEnter`).
    suspended_end_options: Option<EndChallengeOptions>,
    /// `kPAUSE_STATE_CHALLENGE` is pushed (`Pause.PauseSimulation`).
    pub(super) challenge_paused: bool,
    /// The several-challenges popup holds `kPAUSE_STATE_FRONTEND` (`SignUpHUD::UpdateDPadUp`
    /// pushes it, `MultipleSignupsCallback` pulls it), and whether the stack has it.
    sign_up_choice_paused: bool,
    sign_up_choice_pushed: bool,
    /// FE state 8's `mWinState` as `OpenPhotoSelection` left it.
    photo_end_win: bool,
    /// The running scene's `InstanceFrozenSkater` characters.
    pub(super) frozen_skaters: Vec<FrozenSkater>,
    /// Change Game's next challenge, started once the running one completes.
    party_switch: Option<party_play::Location>,
    dialog_owner: DialogOwner,
    /// The career front-end states' `GlobalFEPlaySound` calls, in order.
    sounds: Vec<&'static str>,
    /// `QuitPopupCallback`'s `QuittingToFE` and `Sk8AppMan::EnterFE`, for the session.
    quit_to_front_end: bool,
    /// `CameraPresentation::mChallengeCameraMatrix` while `mChallengeCameraActive`, which
    /// `GetMatrix` (`0x827f6308`) presents ahead of every other camera; the presentation does
    /// not yet take it.
    challenge_camera: Option<[f32; 16]>,
    /// `Sk8::NISManager` with the scene manager it drives.
    pub(super) nis: NisSystem,
    /// What the scene loaders read, and the local player for them.
    scene_source: Arc<CareerSceneSource>,
    /// The scene characters' renderables.
    scene_models: SceneCharacterModels,
    /// A scene character borrowed the first player's entity at the last publication.
    first_player_borrowed: bool,
    /// `FrontEndState_ChallengeMap` while the start hub's Map option has it on top.
    challenge_map: Option<crate::challenge::stuff_menu::ChallengeMap>,
    map_challenge_load: Option<MapChallengeLoad>,
    /// The movies the career pushed for the session's movie player, and the sign-up poster
    /// under one.
    movie: movie::CareerMovie,
    /// The audio system the challenge audio manager calls (`ChallengeAudioManager::
    /// AudioInterface`).
    audio: Option<Arc<dyn nis::audio::AudioInterface>>,
    /// `NISManager::TeleportToLastBreadCrumb` asked for the local skater's breadcrumb teleport.
    breadcrumb_teleport: bool,
    /// In an online round a scene's or speech line's end waits for the round's pacing leader
    /// (`FactsNotes::presented`), so every console's challenge takes it at the same tick.
    pub(super) hold_presentation: bool,
    pub(super) held_presentation: Vec<HeldPresentation>,
}

impl CareerSession {
    /// `cActivityManager::GetMainChallenge` is set.
    pub(crate) fn main_challenge_active(&self) -> bool {
        self.system.main_challenge().is_some()
    }
    /// The challenge system's answers to the session marker rules.
    pub(crate) fn session_marker_rules(&self) -> crate::challenge::system::MarkerRules {
        self.system.session_marker_rules()
    }
    /// `VDWriter::_PrepareChallengeVisualIndicators` for the local skater at `point`.
    pub(crate) fn visual_indicators(
        &self,
        point: [f32; 3],
    ) -> crate::challenge::visual_director::VisualIndicatorFrame {
        crate::challenge::visual_director::prepare(self.system.world(), point)
    }
    fn new(
        mounts: &Arc<MountSet>,
        wipeout_almost_over_time: f32,
        host_calendar: Option<fn() -> [u16; 6]>,
    ) -> Result<Self, String> {
        let db = Arc::new(AttribDb::load_career(mounts).map_err(|e| e.to_string())?);
        let global = mounts.read_path(GLOBAL_LUA).map_err(|e| e.to_string())?;
        let mut system = ChallengeSystem::new_career(db, &global).map_err(|e| e.to_string())?;
        system.world_mut().hom_local_calendar = host_calendar;
        let parts = mounts.read_path(cas_unlock_parts::PATH).map_err(|e| e.to_string())?;
        system.world_mut().cas_unlock_parts = CasUnlockPartDb::parse(&parts)?;
        let table = LocationTable::parse(&mounts.read_path(SIM_LOCATIONS).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let mut location_transforms = HashMap::new();
        for location in &table.locations {
            let key = hash_value32(&location.name);
            system.world_mut().locations.insert(key, location.transform);
            location_transforms.insert(key, location.transform);
        }
        let (directory, cameras) = nis::directory::load_with_content(mounts)?;
        let subtitles = mounts.read_path(nis::subtitles::BIG_PATH).map_err(|e| e.to_string())?;
        // LanguageManager::GetCurrentLanguageName of the English build.
        let mut nis = NisSystem::new(
            Arc::new(directory),
            Arc::new(nis::camera::IceCameras::new(Arc::new(cameras))),
            Some(Arc::new(subtitles)),
            "English",
        );
        let scene_source = Arc::new(CareerSceneSource::new(Arc::clone(mounts), nis::characters::twist_ratios(mounts)?));
        nis.set_character_source(Arc::clone(&scene_source) as Arc<dyn nis::characters::SceneCharacterSource>);
        Ok(Self {
            system,
            mission: MissionSystem::default(),
            tuning: MissionTuning { wipeout_almost_over_time, ..MissionTuning::default() },
            tester: TesterTuning::default(),
            previous: BTreeMap::new(),
            location_transforms,
            locations: Arc::new(Mutex::new(locations::LocationManager::with_global_locations(mounts)?)),
            hud: CareerHud::default(),
            poster_blocked: false,
            poster_input_blocked: false,
            poster_bet_input: (true, 0),
            cash_anim_done: false,
            overlay_paused: false,
            pausing: Vec::new(),
            contacts:None, contacts_saving:false,
            unlock_ready: false,
            skate101_ready: false,
            hom_results_ready:false,
            hom_replay_editor:false,annals_ready:false,
            sponsor_ready: false,
            sponsor_selected: 0,
            sponsor_outro_pending: false,
            slappys: blog::Slappys::default(),
            end_options_ready: false,
            end_options_play_once: true,
            end_options_wait_outro: false,
            end_options_opened_by_nis: false,
            party: None,
            online_random: None,
            change_game: None,
            change_game_ready: false,
            suspended_end_options: None,
            challenge_paused: false,
            sign_up_choice_paused: false,
            sign_up_choice_pushed: false,
            photo_end_win: false,
            frozen_skaters: Vec::new(),
            party_switch: None,
            dialog_owner: DialogOwner::Progression,
            sounds: Vec::new(),
            quit_to_front_end: false,
            challenge_camera: None,
            nis,
            scene_source,
            scene_models: SceneCharacterModels::default(),
            first_player_borrowed: false,
            challenge_map: None,
            map_challenge_load: None,
            movie: movie::CareerMovie::default(),
            audio: None,
            breadcrumb_teleport: false,
            hold_presentation: false,
            held_presentation: Vec::new(),
        })
    }

    /// The main challenge as `AudioPresentation::DoSimSubmit` (`0x827fb2f8`) hands it to
    /// `AudioSystem::UpdatePresentationData` (`0x82504848`): its `ChallengeType` (52 without
    /// one), its host (`challenges` layout word 0) and large-arena flag (attribute
    /// 0xF819DAE7_4AA347B5), and the game mode.
    pub(super) fn audio_challenge(&self) -> (i32, Option<i32>, bool, i32) {
        const LARGE_ARENA: u64 = 0xF819_DAE7_4AA3_47B5;
        let world = self.system.world();
        let Some(main) = world.activities.main else { return (52, None, false, world.game_mode) };
        let key = world.activities.activities[main].key;
        let host = world.db.layout_word(class::CHALLENGES, key, 0) as i32;
        let large_arena = world.db.attribute(class::CHALLENGES, key, LARGE_ARENA).and_then(|a| a.bool(0)).unwrap_or(false);
        (crate::challenge::signup::challenge_type(world, main), Some(host), large_arena, world.game_mode)
    }

    /// `UpdatePresentationData` (`0x82504848`)'s main challenge names: `c_3a_01_street_gvr`
    /// (`mbIsGVR`), `m_3a_03_besttrick_danny` (`mbIsDannyWayJam`) and `c_3a_01_jam_mega`
    /// (`mbIsMegaChallenge`).
    pub(super) fn audio_challenge_names(&self) -> (bool, bool, bool) {
        let world = self.system.world();
        let Some(main) = world.activities.main else { return (false, false, false) };
        let name = world.activities.activities[main].name.as_str();
        (name == "c_3a_01_street_gvr", name == "m_3a_03_besttrick_danny", name == "c_3a_01_jam_mega")
    }

    /// `cChallengeRaceManager::GetPlayerRank` (`0x825b0b08`) of the local player.
    pub(super) fn audio_local_race_ranking(&self) -> i32 {
        let world = self.system.world();
        world.race.rank(world.local_player)
    }

    /// The audio system the career's scenes and challenge audio manager call.
    fn set_audio(&mut self, audio: Option<Arc<dyn nis::audio::AudioInterface>>) {
        if let Some(audio) = audio.as_ref() {
            self.nis.set_audio(Arc::clone(audio));
        }
        self.audio = audio;
    }

    /// The scene characters' renderables after a presentation frame: built
    /// when the scene's characters change, then each posed.
    pub(super) fn present_scene_characters(&mut self, mounts: &MountSet, vault: &Arc<AttributeRuntimeVault>, cvars: &InputCvarReferences) {
        self.scene_models.present(self.nis.scene_characters_mut(), mounts, vault, cvars);
    }

    pub(super) fn scene_visuals(&self) -> Vec<super::SelectedSceneCharacterVisual<'_>> {
        let mut visuals = self.scene_models.visuals();
        visuals.extend(self.frozen_skaters.iter().map(|frozen| {
            let local = |corner: [f32; 3]| -> [f32; 3] { std::array::from_fn(|lane| corner[lane] - frozen.root[12 + lane]) };
            super::SelectedSceneCharacterVisual {
                palette: &frozen.palette,
                materials: Some(&frozen.materials),
                colours: None,
                hips_position: frozen.hips,
                world_bounds: frozen.bounds,
                root: frozen.root,
                local_bounds: [local(frozen.bounds[0]), local(frozen.bounds[1])],
            }
        }));
        visuals
    }

    /// Whether a scene character borrowed the first player's entity at the last
    /// [`Self::publish_borrowed_skaters`].
    pub(super) const fn first_player_borrowed(&self) -> bool {
        self.first_player_borrowed
    }

    /// `INetworkGameQuery::GetLocalPlayerID` (0 offline): the player id of the first player's skater.
    fn first_player_id(&self) -> i32 {
        crate::challenge::players::network_local_player(self.system.world())
    }

    /// ControlCharacterRenderable82870fb0 changes each lent entity's pose callbacks: a character
    /// lent by player id poses that player's entity (`GetSkaterPresEntityFromPlayerId`
    /// 0x82876aac), "current_player" the local index's, and an entity no character holds any
    /// more answers the game's pose again.
    pub(super) fn publish_borrowed_skaters(&mut self,
        mut first_runtime: Option<&mut crate::actor_face_playback::presentation::frame::selected_runtime::SelectedPresentationRuntime>,
        ai: &mut [crate::selected_session_boot_owner::party_skaters::PartySkater], party: &mut [crate::selected_session_boot_owner::party_skaters::PartySkater],
    ) -> Result<(), String> {
        if let Some(runtime) = first_runtime.as_deref_mut() {
            runtime.end_borrow();
        }
        for skater in ai.iter_mut().chain(party.iter_mut()) {
            skater.presentation_runtime.end_borrow();
        }
        self.first_player_borrowed = false;
        let first = self.first_player_id();
        let nis_time = self.nis.character_nis_time();
        for character in self.nis.scene_characters_mut().iter_mut().filter(|c| c.borrowed) {
            let player = character.player.unwrap_or(first);
            let later = ai.iter_mut().chain(party.iter_mut())
                .find(|s| i32::try_from(s.player_index()).ok() == Some(player)).map(|s| &mut s.presentation_runtime);
            let (runtime, first_player) = match later {
                Some(runtime) => (runtime, false),
                None if player == first => match first_runtime.as_deref_mut() {
                    Some(runtime) => (runtime, true),
                    None => continue,
                },
                None => continue,
            };
            if !character.has_render_entity()
                && let Some(inverse_bind) = runtime.inverse_bind(character.anim.trajectory_bone())
            {
                character.attach_render_entity(inverse_bind)?;
            }
            runtime.publish_borrowed(character, nis_time)?;
            self.first_player_borrowed |= first_player;
        }
        Ok(())
    }

    pub(super) fn set_local_player(&self, recipe: Option<nis::characters::LocalPlayerRecipe>) {
        self.scene_source.set_local_player(recipe);
        self.scene_source.set_local_player_id(self.first_player_id());
        self.scene_source.set_local_index(self.system.world().local_player);
    }

    /// The NIS scene's view of the local player's render recipe: its schema
    /// and, for a create-a-skater recipe, `CharacterEntity::IsMale`
    /// (`0x82cfe138`: no customizations reads as male).
    pub(super) fn local_player_recipe(recipe: &skate2_formats::recipe::CharacterRecipe) -> nis::characters::LocalPlayerRecipe {
        nis::characters::LocalPlayerRecipe {
            schema: recipe.schema_name.clone(),
            create_a_character: (recipe.category == 3).then(|| recipe.customizations.as_ref().is_none_or(|c| c.is_male)),
        }
    }

    /// A scene's or speech line's event to the challenge system, or held for an online round's
    /// pacing leader.
    fn present(&mut self, event: HeldPresentation) {
        if self.hold_presentation {
            self.held_presentation.push(event);
        } else {
            self.deliver_presentation(event);
        }
    }

    /// The challenge system's messages of a presentation event.
    pub(super) fn deliver_presentation(&mut self, event: HeldPresentation) {
        match event {
            HeldPresentation::Nis(1) => {
                self.system.post(Event::NisChallengeNotify { notification: 1 });
                self.system.post(Event::NisComplete);
            }
            HeldPresentation::Nis(notification) => self.system.post(Event::NisChallengeNotify { notification: i32::from(notification) }),
            HeldPresentation::Speech { character, id } => self.system.post(Event::SpeechEnded { character, id }),
        }
    }

    /// The NIS's requests: the pause changes to make on `PauseMan` (push
    /// `kPAUSE_STATE_NIS` for true), and at its end `cMsgNISChallengeNotify(1)`
    /// then `cMsgNISComplete` to the challenge system.
    pub(super) fn take_nis_events(&mut self) -> Vec<bool> {
        let mut pauses = Vec::new();
        for event in self.nis.take_events() {
            match event {
                NisEvent::PauseSim => pauses.push(true),
                NisEvent::ResumeSim => pauses.push(false),
                NisEvent::Completed => self.present(HeldPresentation::Nis(1)),
                NisEvent::EnterPhotoView => self.present(HeldPresentation::Nis(2)),
                NisEvent::LoadingOverlay(show) => self.hud.nis_loading = show,
                NisEvent::TeleportToLastBreadcrumb => self.breadcrumb_teleport = true,
                NisEvent::ExitTransition(kind) => eprintln!("career: NIS exit transition {kind} not applied"),
                // FrontEndState_NIS::OnEnterState(79), 0x826675a0: win, no throwdown,
                // offline, mOpenedByNIS, then PushState(8).
                NisEvent::OpenResultsScreen => {
                    let options = self.system.photo_end_challenge_options(true);
                    self.hud.end_options = Some(EndOptionsView::new(options, self.system.world()));
            self.end_options_play_once = true;
                    self.pausing.push(PausingState::EndChallengeOptions);
                    self.end_options_ready = false;
                    self.end_options_wait_outro = false;
                    self.end_options_opened_by_nis = true;
                }
                NisEvent::PlayingKillable => self.present(HeldPresentation::Nis(3)),
                NisEvent::FreezeCommandNotPorted(command) => eprintln!("career: NIS freeze command {command} not applied"),
            }
        }
        self.hud.nis = self.nis.view();
        self.hud.nis_screens.extend(self.nis.take_screen_commands());
        pauses
    }

    /// A pausing front-end state is open: `FrontEndState_Unlock` (FE state
    /// 10) or `FrontEndState_Skate101` (79).
    pub(super) fn open_contacts(&mut self,saving:bool) {
        let contacts=crate::challenge::contacts::Contacts::open(self.system.world(),saving);
        self.hud.contacts=Some(contacts.view.clone());self.contacts=Some(contacts);
        self.contacts_saving=saving;self.pausing.push(PausingState::Contacts);
        self.sounds.push("FE_SOUND_CELLPHONE_Y_BUTTON");
    }
    pub(super) fn set_contacts_saving(&mut self,saving:bool){self.contacts_saving=saving;}
    pub(super) fn can_open_contacts(&self)->bool {
        crate::challenge::contacts::can_show_options(self.system.world())
            && !self.overlay_open() && !self.popup_open() && self.hud.poster.is_none()
            && self.hud.press_a.is_none() && !self.nis.fe_state_active()
    }
    pub(super) fn overlay_open(&self) -> bool {
        !self.pausing.is_empty()
    }

    /// `CheckForOnlineHubMenu` (`0x82656428`) in the career: no popup or pausing state
    /// (`CheckForPauseState`), no NIS, the backpack and pause menu not disabled
    /// (`mbDisableBackpackAndPauseMenu`), and neither the sign-up poster nor the press-A dialog
    /// (FE states 32 and 38) on the stack.
    pub(super) fn online_hub_allowed(&self) -> bool {
        !self.popup_open() && !self.overlay_open() && self.hud.poster.is_none() && self.hud.press_a.is_none()
            && !self.nis.is_in_nis() && !self.system.world().pause_menu_disabled
    }

    /// A popup is open: the progression and confirmation dialogs or the several-challenges
    /// choice (`StateManagerFrontEnd::mPopupList`).
    pub(super) fn popup_open(&self) -> bool {
        self.hud.dialog.is_some() || self.hud.sign_up_choice.is_some()
    }

    /// The career's front-end state on top, which `UpdateFEAudioState` (`0x82d67948`) reports to
    /// the audio system: the press-A dialog, the pausing state on top, the sign-up poster, then
    /// `FrontEndState_NIS` while it is on the stack. Popups are not on the state stack
    /// (`OpenPopup` `0x82d6aaa0` enters FE state 34 directly), so they report nothing.
    pub(super) fn frontend_audio_state(&self) -> Option<crate::ui::FrontendAudioState> {
        use crate::ui::FrontendAudioState as State;
        if self.hud.press_a.is_some() {
            return Some(State::PRESS_A);
        }
        if let Some(state) = self.pausing.last() {
            return Some(match state {
                PausingState::Contacts => State{state:74,music_state:3,paused:true},
                PausingState::HomResults => State::HOM_RESULTS,
                PausingState::AnnalsOfMeat => State::ANNALS_OF_MEAT,
                PausingState::Unlock => State::UNLOCK_OVERLAY,
                PausingState::Skate101 => State::SKATE101,
                PausingState::EndChallengeOptions => State::END_CHALLENGE_OPTIONS,
                PausingState::ChangeGame => State::CHANGE_GAME,
                PausingState::Sponsor => State::SPONSOR_OVERLAY,
                PausingState::Blog => State::SLAPPYS_BLOG,
                PausingState::Articles => State::SLAPPYS_ARTICLES,
                PausingState::Covers => State::SLAPPYS_COVERS,
            });
        }
        if self.hud.poster.is_some() {
            return Some(State::SIGN_UP_POSTER);
        }
        self.nis.fe_state_active().then_some(State::NIS)
    }

    /// This update's `cMsgFadeAmount` from the challenge effects, none
    /// without a fade running.
    pub(super) fn fade_amount(&self) -> Option<f32> {
        self.system.world().effects.fade_amount
    }

    /// The FRONTEND pause the pausing states' `sFEStateInfo` asks for (`mbPause`):
    /// `StateManagerFrontEndBase::UpdateOnEnter` (`0x82d663f0`) pushes it and
    /// `UpdateOnLeave` (`0x82d66300`) pulls it. The change to apply, if any.
    pub(super) fn take_overlay_pause_change(&mut self) -> Option<bool> {
        let open = self.overlay_open();
        (open != self.overlay_paused).then(|| {
            self.overlay_paused = open;
            open
        })
    }

    /// The several-challenges popup's `kPAUSE_STATE_FRONTEND` change to apply, if any.
    fn take_sign_up_choice_pause_change(&mut self) -> Option<bool> {
        let paused = self.sign_up_choice_paused;
        (paused != self.sign_up_choice_pushed).then(|| {
            self.sign_up_choice_pushed = paused;
            paused
        })
    }

    /// The top pausing state's input once its screen is ready, after an open popup's.
    /// `FrontEndState_Unlock::ReceiveInput` (`0x826595c0`): A once the cash
    /// count is done (`mCashAnimDone`) pops the state; after the boot flow
    /// through `SaveAndClose` (`0x82659468`).
    /// `FrontEndState_Skate101::ReceiveInput` (`0x8269feb0`): A (keycode 2)
    /// sends `cMsgHUDContinue` and pops the state; its `OnLeave`
    /// (`0x8269fe08`) sends `cMsgPinPlayer(0)` and closes the screen.
    pub(super) fn overlay_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput) {
        if self.popup_input(input) {
            return;
        }
        match self.pausing.last() {
            Some(PausingState::Contacts) => {
                if let Some(mut contacts)=self.contacts.take() {
                    if contacts.input(self.system.world_mut(),input,self.contacts_saving,&mut self.sounds) {
                        self.pausing.pop();self.hud.contacts=None;
                    } else {self.hud.contacts=Some(contacts.view.clone());self.contacts=Some(contacts);}
                }
            }
            Some(PausingState::Unlock) => {
                if input.select && self.unlock_ready && self.cash_anim_done && self.hud.unlock_overlay.take().is_some() {
                    self.pausing.pop();
                    self.system.unlock_overlay_continue();
                }
            }
            Some(PausingState::HomResults) => self.hom_results_input(input),
            Some(PausingState::AnnalsOfMeat) => self.hom_annals_input(input),
            Some(PausingState::Sponsor) => self.sponsor_input(input),
            Some(PausingState::Blog) => self.blog_input(input),
            Some(PausingState::Articles) => self.articles_input(input),
            Some(PausingState::Covers) => self.covers_input(input),
            Some(PausingState::Skate101) => {
                if input.select && self.skate101_ready && self.hud.skate101.take().is_some() {
                    self.pausing.pop();
                    self.system.post(Event::HudContinue);
                    eprintln!("career: cMsgPinPlayer(0) not applied");
                }
            }
            // ReceiveInput (0x8267c798) takes A, B and X of any pad; the port's front-end
            // input has no Y, whose option slot the ported challenges leave Unknown.
            // OnUpdate (0x826488a0) each front-end update, then ReceiveInput once the screen is
            // ready.
            Some(PausingState::ChangeGame) => {
                if let Some(change) = self.change_game.as_mut() {
                    change.tick(1.0 / 60.0);
                    self.hud.change_game_screen.append(&mut change.take_screen_states());
                }
                if self.change_game_ready {
                    self.change_game_input(input);
                }
            }
            Some(PausingState::EndChallengeOptions) => {
                if !self.end_options_ready {
                    return;
                }
                if self.hud.end_options.as_ref().is_some_and(|view| view.online.is_some()) {
                    self.online_results_input(input);
                    return;
                }
                let keycode = if input.select {
                    2
                } else if input.cancel {
                    3
                } else if input.misc {
                    5
                } else {
                    return;
                };
                self.end_options_select(keycode);
            }
            None => {}
        }
    }

    /// `FrontEndState_Popup` (FE state 34): `StateManagerFrontEnd::ReceiveInput` (`0x82656af8`)
    /// hands an open popup the input before the state on top (FE state 8's Retry and Quit
    /// confirmations are popups over it). The popup is not cancellable; up and down step its
    /// options (two, or a script's generic popup's), sk8popup's menu_picker wrapping and sounding
    /// each move and A. `OpenProgressionPopupCallback` (`0x82897720`): option 0 sends
    /// `cMsgHUDNo`, option 1 `cMsgHUDYes`; `GenericPopupCallback` (`0x82897910`) sends the option;
    /// FE state 8's callbacks act on option 1. Returns whether a popup is open.
    fn popup_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput) -> bool {
        let count = self.hud.dialog_options.as_ref().map_or(2, Vec::len).max(1);
        let Some((_, _, selected)) = self.hud.dialog.as_mut() else { return false };
        if input.up {
            *selected = (*selected + count - 1) % count;
            self.sounds.push("FE_SOUND_CORE_NAV_UP");
        }
        if input.down {
            *selected = (*selected + 1) % count;
            self.sounds.push("FE_SOUND_CORE_NAV_DOWN");
        }
        if input.select {
            let option = *selected;
            let yes = option == 1;
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            self.hud.dialog = None;
            self.hud.dialog_options = None;
            match std::mem::take(&mut self.dialog_owner) {
                DialogOwner::Progression | DialogOwner::Script { generic: false, .. } => {
                    self.system.post(if yes { Event::HudYes } else { Event::HudNo });
                }
                DialogOwner::Script { generic: true, .. } => self.system.post(crate::challenge::teleporter::popup_option(option)),
                DialogOwner::Rematch if yes => self.confirm_rematch(),
                DialogOwner::Quit if yes => self.confirm_quit(),
                DialogOwner::LeaveOnline if yes => {
                    self.leave_end_options();
                    self.quit_to_front_end = true;
                }
                DialogOwner::SponsorConfirm if yes => self.sponsor_next_selected(),
                DialogOwner::FeeFailed => self.poster_blocked = false,
                DialogOwner::Rematch | DialogOwner::Quit | DialogOwner::SponsorConfirm | DialogOwner::Notice | DialogOwner::LeaveOnline => {}
            }
        }
        true
    }

    /// `ReceiveInput`'s option for a key, then `OptionSelect` (`0x82676da0`).
    fn end_options_select(&mut self, keycode: i32) {
        let Some(view) = self.hud.end_options.as_ref() else { return };
        let Some(slot) = results::option_slot(keycode) else { return };
        let win = view.options.win;
        let (action, sounds) = results::option_action(view.leaderboard.option(slot, win), win);
        if sounds && let Some(sound) = results::button_sound(keycode) {
            self.sounds.push(sound);
        }
        match action {
            OptionAction::Leave => {
                self.end_options_wait_outro = true;
                self.leave_end_options();
            }
            OptionAction::Rematch => self.open_dialog(DialogOwner::Rematch, "ID_REMATCH_CONFIRM_MESSAGE"),
            OptionAction::ExitPartyPlay => self.open_dialog(DialogOwner::Quit, "ID_QUIT_GAME_MESSAGE_NO_DATA_LOSS"),
            OptionAction::HighlightReel => {
                self.system.post(Event::Challenge(Message::new("cMsgHUDGotoHighlightReel", Vec::new())));
                self.leave_end_options();
            }
            OptionAction::AdjustSettings => self.adjust_settings(),
            OptionAction::Page(results::option::NEXT | results::option::PREVIOUS) => {
                let options = view.options;
                let next = action == OptionAction::Page(results::option::NEXT);
                let world = self.system.world_mut();
                world.results.set_active_results_leaderboard(if next {
                    results::result_group::FINAL_RESULTS
                } else {
                    results::result_group::ROUND_RESULTS
                });
                if next && std::mem::replace(&mut self.end_options_play_once, false) {
                    results::play_final_results_audio(world);
                }
                self.hud.end_options = Some(EndOptionsView::new(options, world));
            }
            OptionAction::Nothing => {}
            other => eprintln!("career: end-of-challenge option {other:?} is not ported"),
        }
    }

    /// `OptionSelect`'s Adjust Settings: the leaderboard kept (`mbResetLeaderboard` cleared) and
    /// FE state 39 pushed, which leaves FE state 8 (`OnLeave` closes its screen) and enters
    /// `FrontEndState_PartyPlayChangeGame` (`OnEnter` `0x82642ac8`).
    fn adjust_settings(&mut self) {
        let Some(party) = self.party.as_mut() else {
            eprintln!("career: Adjust Settings outside Party Play");
            return;
        };
        let Some(view) = self.hud.end_options.take() else { return };
        self.suspended_end_options = Some(view.options);
        self.end_options_ready = false;
        let change = ChangeGame::open(&mut party.backend);
        self.hud.change_game = Some(ChangeGame::snapshot(&party.backend));
        self.change_game = Some(change);
        self.change_game_ready = false;
        self.pausing.push(PausingState::ChangeGame);
    }

    /// FE state 39's `ReceiveInput` (`0x82677178`).
    fn change_game_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput) {
        let (Some(change), Some(party)) = (self.change_game.as_mut(), self.party.as_mut()) else { return };
        let action = change.update(input, &mut party.backend, party.challenge);
        self.hud.change_game_screen.append(&mut change.take_screen_states());
        self.hud.change_game = Some(ChangeGame::snapshot(&party.backend));
        match action {
            ChangeAction::None => {}
            ChangeAction::Back => self.leave_change_game(),
            ChangeAction::Restart => {
                self.system.restart_main_challenge(0, true);
                self.system.post(Event::Challenge(Message::new("cMsgHUDRetry", Vec::new())));
                self.pop_to_idle();
            }
            ChangeAction::Switch(location) => {
                self.system.post(Event::HudContinue);
                if let Some(party) = self.party.as_mut() {
                    party.challenge = location.key;
                }
                self.party_switch = Some(location);
                self.pop_to_idle();
            }
        }
    }

    /// FE state 39's `PopState`: its `OnLeave` closes the screen, then FE state 8's `OnEnter`
    /// opens its screen again over the kept leaderboard.
    fn leave_change_game(&mut self) {
        if self.pausing.last() == Some(&PausingState::ChangeGame) {
            self.pausing.pop();
        }
        self.change_game = None;
        self.change_game_ready = false;
        self.hud.change_game = None;
        if let Some(options) = self.suspended_end_options.take() {
            self.hud.end_options = Some(EndOptionsView::new(options, self.system.world()));
            self.end_options_play_once = true;
            self.end_options_ready = false;
            self.end_options_wait_outro = false;
        }
    }

    /// Change Game's next challenge once the running one has completed (its `hudcontinue` runs
    /// `challenge_cleanup` and `complete`) and its assets are unloaded. The original keeps one
    /// challenge system through `StateLoading`, whose asset manager unloads the completed
    /// challenge's hulls (their `DRegDMOArena` removing its challenge DMO groups) beside the next
    /// load; the port's next game builds a new system, so this one waits for
    /// `IsAllUnloadComplete` (0x82e0f400) first, as `GameModule::GameLoad` (0x8273be50) does
    /// before a new game.
    pub(super) fn take_party_switch(&mut self) -> Option<party_play::Location> {
        if self.main_challenge_active() || !self.system.is_all_unload_complete() {
            None
        } else {
            self.party_switch.take()
        }
    }

    /// `StateLoading::StartChallengeState` (0x8274f800)'s `StartMainChallenge` once
    /// `PartyPlayLoadState` found the challenge's assets loaded (`IsAssetLoadComplete`).
    fn start_loaded_party_challenge(&mut self) {
        if let Some(party) = self.party.as_mut()
            && party.loading
            && self.system.is_asset_load_complete(party.challenge)
        {
            party.loading = false;
            self.system.start_main_challenge(party.challenge);
        }
    }

    /// Whether the challenge's assets are in (`cChallengeAssetManager::IsAssetLoadComplete`).
    pub(super) fn challenge_assets_loaded(&self, challenge: u64) -> bool {
        self.system.is_asset_load_complete(challenge)
    }

    /// `cChallengeAssetManager::IsLoadingChallengeHulls`.
    pub(super) fn loading_challenge_hulls(&self) -> bool {
        crate::challenge::assets::is_loading_challenge_hulls(self.system.world())
    }

    /// `StateOnlineGame::ChallengeSkate`'s `StartMainChallenge` once every player has loaded,
    /// with the round's players' trigger history starting empty.
    pub(super) fn start_online_main_challenge(&mut self, challenge: u64) {
        self.previous.clear();
        self.system.start_main_challenge(challenge);
    }

    /// `StateOnlineGame::DisplayFinalResults` (`0x82772e58`): FE state 8 in its online display
    /// (`mIsOnlineDisplay`, screen 117) for the challenge.
    pub(super) fn open_online_results(&mut self, challenge: u64, win: bool, board: results::LeaderboardData, online: results::OnlineResults) {
        let options = EndChallengeOptions { win, throwdown: crate::challenge::results::throwdown::NONE, challenge, screen: crate::challenge::results::screen::ONLINE };
        let mut view = EndOptionsView::new(options, self.system.world());
        view.leaderboard = board;
        view.online = Some(online);
        self.hud.end_options = Some(view);
        self.end_options_play_once = true;
        if self.pausing.last() != Some(&PausingState::EndChallengeOptions) {
            self.pausing.push(PausingState::EndChallengeOptions);
        }
        self.end_options_ready = false;
        self.end_options_wait_outro = false;
    }

    /// `GameExitPostChallenge` in `DisplayFinalResults`: `cMsgPostChallengeDisplayExit` to the
    /// challenge system, then the state's exit pops the front end to state 0.
    pub(super) fn close_online_results(&mut self) {
        self.system.post(Event::Challenge(Message::new("cMsgPostChallengeDisplayExit", Vec::new())));
        self.hud.end_options = None;
        self.pausing.retain(|state| *state != PausingState::EndChallengeOptions);
    }

    /// FE state 8's `ReceiveInput` (`0x8267c798`) in the online display and its list's
    /// `UpdateInput`: up and down step the list; B, once the quit is enabled, asks to leave
    /// online (`eSk8FEPopup_Online_ConfirmQuitOnline`, Yes selected). Xbox LIVE's gamercard and
    /// player review (the bumpers) have no service here.
    fn online_results_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput) {
        let Some(online) = self.hud.end_options.as_mut().and_then(|view| view.online.as_mut()) else { return };
        for (pressed, forward) in [(input.up, false), (input.down, true)] {
            if pressed {
                let sound = online.step(forward);
                self.sounds.push(sound);
            }
        }
        if input.cancel && online.quit_enabled {
            self.dialog_owner = DialogOwner::LeaveOnline;
            self.hud.dialog_options = None;
            self.hud.dialog = Some(("ID_EANATION".to_owned(), "ID_ONLINE_CONFIRM_QUIT_ONLINE".to_owned(), 1));
            self.sounds.push("FE_SOUND_CORE_POPUP");
            if let Some(sound) = results::button_sound(3) {
                self.sounds.push(sound);
            }
        }
    }

    /// `OnUpdate` (`0x82658c18`): the post-event timer's seconds, and the quit button once the
    /// results are saved (`SetEnableOnlineQuitButton`).
    pub(super) fn update_online_results(&mut self, timer: i32, quit_enabled: bool) {
        if let Some(online) = self.hud.end_options.as_mut().and_then(|view| view.online.as_mut()) {
            online.timer = timer;
            online.quit_enabled = quit_enabled;
        }
    }

    /// `cLocationManager::GetSpawnLocation` of the location (`GetHashValue32` of its name) for an
    /// online player, a player past the sixth at the sixth's.
    pub(super) fn online_spawn_location(&self, location: &str, player: i32) -> Result<Option<[f32; 16]>, SelectedLiveError> {
        let locations = self.locations.lock().map_err(|_| error("location manager lock poisoned"))?;
        Ok(locations.online_spawn_location(hash_value32(location), player))
    }

    /// `cMsgOnlineSyncComplete` for the online challenge, delivered by the challenge's next update.
    pub(super) fn online_sync_complete(&mut self, challenge: u64) {
        let key = crate::challenge::system::Field::Key(challenge);
        self.system.post(Event::Challenge(Message::new("cMsgOnlineSyncComplete", vec![("ActivityKey", key)])));
    }

    /// FE state 8's online display as the challenge script opened it
    /// (`OpenOnlineContinueRetryDialog`), on the script's results: a round's
    /// (`kScreenMode_Round`) while the round results are FE state 27's active leaderboard
    /// (`Results_GetLeaderboardType`), the last as `Results_SetOnlineIsResultsForLastRound` left
    /// it.
    pub(super) fn show_online_script_results(&mut self, mut online: results::OnlineResults) {
        let leaderboards = &self.system.world().results;
        online.round = leaderboards.current_result_type != results::result_group::FINAL_RESULTS;
        online.last_round = leaderboards.online_is_results_for_last_round;
        let Some(view) = self.hud.end_options.as_mut().filter(|view| view.options.screen == crate::challenge::results::screen::ONLINE) else { return };
        let board = &view.leaderboard;
        let count = usize::try_from(board.num_players).unwrap_or(0).min(board.players.len());
        let mut by_rank: Vec<(i32, usize)> = board.players[..count].iter().enumerate().map(|(player, row)| (row.position, player)).collect();
        by_rank.sort_unstable();
        online.player_by_rank = by_rank.into_iter().map(|(_, player)| player).collect();
        view.online = Some(online);
    }

    /// `OnUpdate` (`0x82658c18`): FE state 27's timer (`mTimerID`), while it has one, in seconds
    /// rounded up (`mOnlineRoundResultsTimer`).
    pub(super) fn update_online_round_timer(&mut self) {
        let world = self.system.world();
        let id = world.results.timer_id;
        if id == -1 {
            return;
        }
        #[expect(clippy::cast_possible_truncation, reason = "(int)ceil(GetTime)")]
        let seconds = world.timers.time(id).ceil() as i32;
        if let Some(online) = self.hud.end_options.as_mut().and_then(|view| view.online.as_mut()) {
            online.round_timer = seconds;
        }
    }

    /// Whether FE state 8 shows the online results.
    pub(super) fn online_results_open(&self) -> bool {
        self.hud.end_options.as_ref().is_some_and(|view| view.options.screen == crate::challenge::results::screen::ONLINE)
    }

    /// `cMsgPlayerDisconnected` for a player of the online challenge, and its row marked
    /// disconnected (`HandlePlayerLeave` 0x8276aa70).
    pub(super) fn online_player_disconnected(&mut self, player: i32) {
        let world = self.system.world_mut();
        if let Some(online) = world.online.as_mut()
            && let Some(row) = usize::try_from(player).ok().and_then(|player| online.disconnected.get_mut(player))
        {
            *row = true;
        }
        #[expect(clippy::cast_precision_loss, reason = "the message's (float) field")]
        let index = player as f32;
        self.system.post(Event::Challenge(Message::new("cMsgPlayerDisconnected", vec![("PlayerIndex", crate::challenge::system::Field::Number(index))])));
    }

    /// The Party Play back end's selection.
    pub(super) fn party_backend(&self) -> Option<&party_play::BackEnd> {
        self.party.as_ref().map(|party| &party.backend)
    }

    /// `PopToState(0)` from FE state 39: its `OnLeave`, then FE state 8 dropped without another.
    fn pop_to_idle(&mut self) {
        self.pausing.retain(|state| !matches!(state, PausingState::ChangeGame | PausingState::EndChallengeOptions));
        self.change_game = None;
        self.change_game_ready = false;
        self.hud.change_game = None;
        self.suspended_end_options = None;
    }

    /// `FrontEndManager::OpenPopup` of a No/Yes template (`ID_COMMON_WARNING_TITLE`, default
    /// No, not cancellable); `FrontEndState_Popup::OnEnter` (`0x82658474`) sounds
    /// `core_popup`.
    fn open_dialog(&mut self, owner: DialogOwner, description: &str) {
        self.dialog_owner = owner;
        self.hud.dialog_options = None;
        self.hud.dialog = Some(("ID_COMMON_WARNING_TITLE".to_owned(), description.to_owned(), 0));
        self.sounds.push("FE_SOUND_CORE_POPUP");
    }

    /// FE state 8's `PopState`: `OnLeave` (`0x82658b48`) closes the screen, resets the
    /// leaderboard (`Reset` `0x82647c10`) and the throwdown type.
    fn leave_end_options(&mut self) {
        if let Some(at) = self.pausing.iter().rposition(|state| *state == PausingState::EndChallengeOptions) {
            self.pausing.remove(at);
        }
        self.hud.end_options = None;
        self.end_options_ready = false;
        self.system.world_mut().results.reset();
        // OnLeave 0x82658bbc..0x82658c08: SkipNIS(true), PopState, clear mOpenedByNIS.
        if std::mem::take(&mut self.end_options_opened_by_nis) {
            self.nis.leave_results_screen();
        }
    }

    /// `FrontEnd.OpenPhotoSelection(count)` (`0x82896cb0`) and `FrontEndState_PhotoChallenge::
    /// OnEnter` (`0x8265ff60`): FE state 8's win state set for after the pick, the state's
    /// picking (more than one photo) and count, the first photo shown (`SetScreenState(116,
    /// 0)`) with the HUD hidden. The killed-it screen's state 5 and the audio's
    /// `NotifyPickPhotoScreen` are not ported.
    fn open_photo_selection(&mut self, count: i32) {
        self.photo_end_win = true;
        self.hud.photo_select = Some(PhotoSelectView { selected: 0, enable_picking: count > 1, num_photos: count, requested: false });
        self.hud.photo_select_screen.push(0);
    }

    /// Whether `FrontEndState_PhotoChallenge` is up.
    pub(super) fn photo_select_open(&self) -> bool {
        self.hud.photo_select.is_some()
    }

    /// The challenge replay recorder's index (`DataSystem::DataMan`).
    pub(super) fn replay(&self) -> &crate::challenge::replay::ReplaySystem {
        self.system.replay()
    }

    /// `DataSysMan::SubmitFrame` into the replay recorder's index: this tick's frame number,
    /// whether it was stored, and the frames let go.
    pub(super) fn store_replay_frame(&mut self) -> (u32, bool, Vec<u32>) {
        self.system.store_replay_frame()
    }

    /// `FrontEndState_PhotoChallenge::ReceiveInput` (`0x82660048`): nothing once the pick is
    /// asked for; Y (keycode 4) picks the shown photo (`SetScreenState` -3 and -10, the
    /// `photo_pick` sound) and asks for its career-photo save, whose callback ends the state
    /// (`cSaveChallengePhotoCallback::OnComplete` `0x8266c180`); with more than one photo the
    /// right and left triggers (keycodes 7 and 10) step the photo, wrapping, send
    /// `cMsgNISPhotoSelect` to the scene, sound `photo_choose` and set -1 or -2; a changed
    /// photo is shown. Replaying challenges and challenges without a unique photo slot
    /// complete immediately; otherwise input stays blocked until the capture/save callback.
    pub(super) fn photo_select_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput) {
        let Some(mut view) = self.hud.photo_select else { return };
        if view.requested {
            return;
        }
        let before = view.selected;
        if input.option {
            self.hud.photo_select_screen.extend([-3, -10]);
            self.sounds.push("FE_SOUND_PHOTO_PICK");
            view.requested = true;
            self.hud.photo_select = Some(view);
            let world = self.system.world();
            let slot = world.activities.main.map_or(0, |main| {
                crate::challenge::photo_articles::unique_photo_slot(world, world.activities.activities[main].key)
            });
            if world.replaying_challenge || slot == 0 {
                self.photo_done();
            } else {
                self.hud.photo_save_slot = Some(slot);
            }
            return;
        }
        if view.enable_picking && (input.right_shift || input.left_shift) {
            let forward = input.right_shift;
            view.selected = if forward {
                if view.selected + 1 > view.num_photos - 1 { 0 } else { view.selected + 1 }
            } else if view.selected - 1 < 0 {
                view.num_photos - 1
            } else {
                view.selected - 1
            };
            self.nis.photo_select(usize::try_from(view.selected).unwrap_or(0));
            self.sounds.push("FE_SOUND_PHOTO_CHOOSE");
            self.hud.photo_select_screen.push(if forward { -1 } else { -2 });
        }
        if view.selected != before {
            self.hud.photo_select_screen.push(view.selected);
        }
        self.hud.photo_select = Some(view);
    }

    /// `FrontEndState_PhotoChallenge::PhotoDone` (`0x82660320`): FE state 8 replaces it
    /// (`SetState(8)`, offline, with the win state `OpenPhotoSelection` set; `OnLeave`
    /// `0x82807098` only re-enables picking) and `cMsgNISComplete` goes to the challenge
    /// system.
    fn photo_done(&mut self) {
        self.hud.photo_save_slot = None;
        self.hud.photo_select = None;
        let options = self.system.photo_end_challenge_options(self.photo_end_win);
        self.hud.end_options = Some(EndOptionsView::new(options, self.system.world()));
            self.end_options_play_once = true;
        self.pausing.push(PausingState::EndChallengeOptions);
        self.end_options_ready = false;
        self.end_options_wait_outro = false;
        self.system.post(Event::NisComplete);
    }

    /// `ConfirmRematchPopupCallback`'s yes: `cActivityManager::RestartMainChallenge(0, true)`
    /// (`0x82cd8880`), whose sub-controllers' `RestartChallenge` (music, DMO and LivingWorld
    /// operations, presentation groups) the port does not have, then `cMsgHUDRetry` and the
    /// state popped.
    fn confirm_rematch(&mut self) {
        self.system.restart_main_challenge(0, true);
        self.system.post(Event::Challenge(Message::new("cMsgHUDRetry", Vec::new())));
        self.leave_end_options();
    }

    /// `QuitPopupCallback`'s yes: `cMsgHUDContinue`, the state popped, then `QuittingToFE` and
    /// `EnterFE`.
    fn confirm_quit(&mut self) {
        self.system.post(Event::HudContinue);
        self.leave_end_options();
        self.quit_to_front_end = true;
    }

    /// The challenge system's map icons as the HUD minimap draws them
    /// (`MapIconManager::RenderMiniMapIcons` `0x826311f0`).
    pub(super) fn challenge_icons(&self) -> Vec<crate::ui::minimap::Icon> {
        crate::challenge::map_icons::minimap_icons(self.system.world())
    }

    /// `FrontEndState_ChallengeMap::OnEnter` (`0x826539e8`) and `OnLoaded` (`0x826879b8`), with the
    /// game's additional world location (`sGameInfo.additionalWorldLocation`).
    pub(super) fn open_challenge_map(&mut self, additional_world_location: Option<u64>) {
        let world = self.system.world_mut();
        world.additional_world_location = additional_world_location;
        self.challenge_map = Some(crate::challenge::stuff_menu::ChallengeMap::open(world));
    }

    /// `FrontEndState_ChallengeMap::OnLeave` (`0x8265ec98`).
    pub(super) fn close_challenge_map(&mut self) {
        if let Some(mut map) = self.challenge_map.take() {
            map.close();
        }
    }

    /// One front-end update of the open map: `OnUpdate` (`0x82687b38`), then the input the state
    /// receives while it has focus.
    pub(super) fn update_challenge_map(
        &mut self,
        input: Option<crate::challenge::stuff_menu::MapInput>,
    ) -> crate::challenge::stuff_menu::MapAction {
        let Some(map) = self.challenge_map.as_mut() else {
            return crate::challenge::stuff_menu::MapAction::None;
        };
        let world = self.system.world_mut();
        crate::challenge::stuff_menu::ChallengeMap::tick(world);
        let action = input.map_or(crate::challenge::stuff_menu::MapAction::None, |input| {
            map.update(world, input)
        });
        self.sounds.append(&mut map.take_sounds());
        action
    }

    pub(super) fn challenge_map_view(
        &self,
    ) -> Option<crate::challenge::stuff_menu::ChallengeMapView> {
        self.challenge_map
            .as_ref()
            .map(|map| map.view(self.system.world()))
    }

    /// `MapBackEndManager::TeleportToChallenge` (`0x8256dc10`)'s destination: the challenge's
    /// `Location` spawn point for the local player (`GetSpawnLocation(name, GetLocalPlayerID())`).
    pub(super) fn challenge_teleport_transform(&self, challenge: u64) -> Option<[f32; 16]> {
        let world = self.system.world();
        let location = crate::challenge::stuff_menu::challenge_location(world, challenge)?;
        let locations = self.locations.lock().ok()?;
        //8256de1c: Simulation::GetLocalPlayerID, including while spectating AI.
        locations.spawn_location(hash_value32(&location), crate::challenge::players::OFFLINE_LOCAL_PLAYER_ID)
    }

    /// `cTeleportManager::TeleportLocalPlayerStartChallenge` (`0x828a9678`): the challenge
    /// starts during the load (`sGameInfo.startChallengeDuringLoad` and `challengeKey`), whose
    /// `TeleportLocalToWorldLocation` goes to the GlobalData `Location` on the board.
    pub(super) fn prepare_start_challenge_travel(&mut self, key: u64) -> Option<[f32; 16]> {
        let transform = self.challenge_teleport_transform(key)?;
        self.system.start_challenge_during_load();
        self.map_challenge_load = Some(MapChallengeLoad::Teleport { key });
        Some(transform)
    }

    /// `MapBackEndManager::TeleportToChallenge8256dc10`: only a challenge with a Location
    /// reaches the replay/start flags. Reuse the authored offline player's spawn point.
    pub(super) fn prepare_challenge_map_travel(&mut self, key: u64, retry: bool) -> Option<[f32; 16]> {
        let transform = self.challenge_teleport_transform(key)?;
        if self.system.prepare_map_challenge(key, retry) {
            self.map_challenge_load = Some(MapChallengeLoad::Teleport { key });
        }
        Some(transform)
    }

    /// The host's queued teleport has reached physics and presentation. Keep the cover for
    /// `StartChallengeState`; ordinary map teleports can reveal immediately.
    pub(super) fn map_challenge_teleport_complete(&mut self) -> bool {
        if let Some(MapChallengeLoad::Teleport { key }) = self.map_challenge_load {
            self.map_challenge_load = Some(MapChallengeLoad::StartChallenge { key, ready_updates: 0 });
            true
        } else {
            false
        }
    }

    pub(super) fn map_challenge_loading(&self) -> bool {
        self.map_challenge_load.is_some()
    }

    fn start_loaded_map_challenge(&mut self) -> Result<(), SelectedLiveError> {
        if let Some(MapChallengeLoad::StartChallenge { key, ready_updates }) = self.map_challenge_load.as_mut()
            && self.system.update_map_challenge_load(*key, ready_updates).map_err(error)?
        {
            self.map_challenge_load = Some(MapChallengeLoad::WorldPrepare { rendered_frames: 0 });
        }
        Ok(())
    }

    /// `WorldPrepareState8274f668` reads the render tick, independently of NIS/simulation pause.
    pub(super) fn map_challenge_frame_presented(&mut self) {
        if let Some(MapChallengeLoad::WorldPrepare { rendered_frames }) = self.map_challenge_load.as_mut() {
            *rendered_frames = rendered_frames.saturating_add(1);
        }
    }

    /// `WorldPrepareState8274f6f8..90`: strictly more than five rendered frames, then the
    /// loading reset and release of the front-end cover. The NIS keeps its own loading flow.
    pub(super) fn finish_map_challenge_load(&mut self) -> bool {
        if matches!(self.map_challenge_load, Some(MapChallengeLoad::WorldPrepare { rendered_frames }) if rendered_frames > 5) {
            self.map_challenge_load = None;
            self.system.loading_reset();
            true
        } else {
            false
        }
    }

    /// A challenge's `challenges` text attribute, by activity name.
    fn challenge_text(&self, name: &str, key: u64) -> String {
        let db = &self.system.world().db;
        db.attribute(crate::challenge::keys::class::CHALLENGES, crate::challenge::keys::hash64(name), key)
            .and_then(|a| a.text(0))
            .unwrap_or_default()
            .to_owned()
    }

    /// The local player's active objective HAL key.
    fn active_objective(&self) -> Option<String> {
        let world = &self.system.world;
        let group = world.objectives.groups.iter().find(|g| g.player == world.local_player)?;
        let objective = group.objectives.iter().find(|o| o.state == Status::Active)?;
        objectives::hal_string(&world.db, objective.key)
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `FrontEndManager::StartGameModule` / `FrontEndState_CaC::Shutdown`:
    /// the career's challenge system with `SetEnableProgression(true)`. A
    /// continued career starts where `FrontEndState_MainMenu::EnterGame`
    /// places it.
    /// Party Play's challenge system (`Sk8ChallengeSystem` without progression): the party game
    /// module's `GameLoad`, `WorldPrepareState`'s `LoadingReset`, `StartInGame` without the phone
    /// list, then `PartyPlayLoadState`'s `LoadChallengeAssets` of the chosen challenge, which
    /// `StartChallengeState` starts once they are in.
    pub(super) fn start_party_play_challenge(&mut self, challenge: u64, backend: party_play::BackEnd) -> Result<(), SelectedLiveError> {
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("party play assets absent"))?;
        let almost_over = assets
            .vault
            .instance(PHYSICS_WIPEOUT, DEFAULT_COLLECTION)
            .try_scalar_word(WIPEOUT_ALMOST_OVER, 0)
            .map_err(error)?
            .map_or(0.0, f32::from_bits);
        let mut session = CareerSession::new(&assets.mounts, almost_over, self.host_calendar).map_err(error)?;
        session.set_audio(self.audio.interface.clone());
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let unlocks = store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(error)?;
        session.system.world_mut().unlockables.load(leaf_tuples(&unlocks));
        self.bind_challenge_players(&mut session, crate::challenge::players::GM_PARTY_PLAY)?;
        session.system.set_boot_flow_executed();
        session.system.game_load_party_play();
        session.system.loading_reset();
        session.system.start_in_game(true);
        session.system.load_challenge_assets(challenge);
        session.party = Some(PartyState { challenge, backend, loading: true });
        self.career = Some(session);
        Ok(())
    }

    /// `StateOnlineGame::ChallengeLoadState` (`0x82771a28`): the network game's challenge system
    /// (`GM_OnlineChallenge`, the `GameReset`'s players, `GameLoad` with signature 8) and the
    /// challenge's `LoadChallengeAssets`; `GameReset(Challenge)` starts it once every player has
    /// loaded.
    pub(super) fn start_online_challenge_load(
        &mut self,
        challenge: u64,
        players: Arc<dyn crate::challenge::players::PlayersBackEnd>,
        online: crate::challenge::players::OnlineChallenge,
    ) -> Result<(), SelectedLiveError> {
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("online challenge assets absent"))?;
        let almost_over = assets
            .vault
            .instance(PHYSICS_WIPEOUT, DEFAULT_COLLECTION)
            .try_scalar_word(WIPEOUT_ALMOST_OVER, 0)
            .map_err(error)?
            .map_or(0.0, f32::from_bits);
        let mut session = CareerSession::new(&assets.mounts, almost_over, self.host_calendar).map_err(error)?;
        session.set_audio(self.audio.interface.clone());
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let unlocks = store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(error)?;
        session.system.world_mut().unlockables.load(leaf_tuples(&unlocks));
        self.bind_challenge_players(&mut session, crate::challenge::players::GM_ONLINE_CHALLENGE)?;
        let world = session.system.world_mut();
        world.players = Some(players);
        world.local_player = online.local_player;
        world.online = Some(online);
        session.system.set_boot_flow_executed();
        session.system.game_load_network();
        session.system.loading_reset();
        session.system.start_in_game(true);
        session.system.load_online_challenge_assets(challenge);
        self.career = Some(session);
        Ok(())
    }

    /// `FrontEndState_MainMenu::StartSinglePlayer` (`0x82678c20`): the census spawns pedestrians
    /// and vehicles again (`EnablePedestrianSpawn(true)`, `EnableVehicleSpawn(true)` on
    /// `LW::Man::GetCensusMan`), whatever an earlier game's challenge left of them. No other game
    /// start sets them (`OnMenuSelectGame` 0x82681338 clears them in its marketing demo only), so a
    /// free skate after a quit game keeps that game's `LivingWorldManagement` spawners.
    pub(super) fn start_single_player_census(&self) -> Result<(), SelectedLiveError> {
        let world = &self.inner.world.as_ref().ok_or_else(|| error("world absent"))?.world;
        let world = world.lock().map_err(|_| error("world lock poisoned"))?;
        let mut living = world
            .grind_services
            .living_world()
            .lock()
            .map_err(|_| error("LivingWorld lock poisoned"))?;
        living.census.enable |= 0x20 | 0x08;
        Ok(())
    }

    pub(super) fn start_career(&mut self, continued: bool) -> Result<(), SelectedLiveError> {
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?;
        let almost_over = assets
            .vault
            .instance(PHYSICS_WIPEOUT, DEFAULT_COLLECTION)
            .try_scalar_word(WIPEOUT_ALMOST_OVER, 0)
            .map_err(error)?
            .map_or(0.0, f32::from_bits);
        let mut career = CareerSession::new(&assets.mounts, almost_over, self.host_calendar).map_err(error)?;
        career.set_audio(self.audio.interface.clone());
        self.bind_challenge_players(&mut career, crate::challenge::players::GM_SINGLE_PLAYER)?;
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let stats=store.load_leaf(crate::challenge::hom_save::SAVABLE,crate::challenge::hom_save::DATA).map_err(error)?;
        self.inner.player.as_ref().ok_or_else(||error("statistics player absent"))?.selected.owners.services.skaters.backend()
            .load_statistics(stats.as_ref(),&career.system.world().db).map_err(error)?;
        // UnlockablesBackEnd::LoadData 0x8255cd08 over the defaults Init added.
        let unlocks = store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(error)?;
        career.system.world_mut().unlockables.load(leaf_tuples(&unlocks));
        // The Sk8ChallengeSystem savable: Sk8ChallengeSystem::LoadData
        // 0x825a2ed8 → cChallengeDynamicHullManager::LoadData 0x82db4330.
        let hulls = store.load_leaf(dynamic_hull::SAVABLE, dynamic_hull::DATA).map_err(error)?;
        career.system.world_mut().dynamic_hulls.load(leaf_tuples(&hulls));
        // The profile's SignPost savable: cSignPostManager::LoadData 0x825e53d8.
        let leaf = |name: &str| store.load_leaf(signpost_progress::SAVABLE, name).map_err(error);
        let saved = [leaf(signpost_progress::DATA)?, leaf(signpost_progress::POSITION)?, leaf(signpost_progress::MESSAGES)?];
        signpost_progress::load(career.system.world_mut(), leaf_tuples(&saved[0]), leaf_tuples(&saved[1]), leaf_tuples(&saved[2]));
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            let signpost = &career.system.world().signpost;
            eprintln!(
                "career: SignPost {} ({} completed, {} available)",
                if saved[0].is_some() { "loaded" } else { "fresh" },
                signpost.completed.len(),
                signpost.available.len()
            );
        }
        // ProfileBackEnd::mSettings, loaded with the profile, holds mSubtitles.
        career.hud.subtitles = self.audio.settings.subtitles;
        career.system.set_enable_progression(true);
        // The career is entered from the main menu, whose front end state
        // marked the boot flow executed.
        career.system.set_boot_flow_executed();
        // The career game module's GameLoad, then the loading flow's
        // LoadingReset once the world is in.
        career.system.game_load();
        career.system.loading_reset();
        career.system.start_in_game(false);
        self.career = Some(career);
        if continued {
            self.place_continued_career()?;
        } else {
            self.place_new_career()?;
        }
        Ok(())
    }

    /// A new profile's `FrontEndState_MainMenu::StartSinglePlayer` (`0x82678c20`) set the first
    /// `eFELocationType_NewCareer` FE location's layout text 1 (`gamestart`'s `J_Gamestart`) as
    /// the location name (`SetLocationName` `0x82559d00`). After the editor,
    /// `GameModule::GameLoad` (`0x8273be50`) streams in its `GetSpawnLocation` and waits for the
    /// stream focus before `ResetGame(newGame, useLocationSpawner)` (`0x8273a680`) gives
    /// `PlayerUI::OnReset` (`0x828aadd0`) the name's hash; the game's first `PlayerUI::Update`
    /// (`0x828af9f8`) then has `ResetPlayer` (`0x828ad280`) spawn player 0 at
    /// `cLocationManager::GetSpawnLocation(hash, 0)` (`0x82a8dce8`), which that frame's
    /// simulation runs before the challenge system's first update. The port keeps its skater
    /// and moves it there on the board under the fade, as the map's travel does, and holds the
    /// career's updates until it has landed.
    fn place_new_career(&mut self) -> Result<(), SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?.mounts);
        let career = self.career.as_ref().ok_or_else(|| error("career absent"))?;
        let world = career.system.world();
        let location = crate::ui::freeskate_option::new_career_locations(&world.db, &world.unlockables, &mounts)
            .into_iter()
            .next()
            .ok_or_else(|| error("new career start location absent"))?;
        let transform = career
            .locations
            .lock()
            .map_err(|_| error("location manager lock poisoned"))?
            .spawn_location(hash_value32(&location.spawn), 0)
            .ok_or_else(|| error(format!("new career start {} absent", location.spawn)))?;
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: new career starts at {} {:?}", location.spawn, [transform[12], transform[13], transform[14]]);
        }
        self.travel_to_career_start(transform);
        Ok(())
    }

    /// The port's game load of a career start: the map's travel to the spawn, on the board, which
    /// streams it in under the fade, with the career's updates held until the skater has landed.
    fn travel_to_career_start(&mut self, transform: [f32; 16]) {
        // GameLoad 0x8273be50's SyncPurgeAdditionalWorldFile: a career sets no additional location
        // (only free skate, Party Play and online games call SetAdditionalLocation 0x8256bdc0), so a
        // world another game left loaded goes.
        self.map_travel_world = super::world_travel::TravelWorld::Main;
        self.map_travel = Some(transform);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = true;
        self.career_spawn_pending = true;
    }

    /// `FrontEndState_MainMenu::EnterGame` (`0x826788b0`) for a profile that
    /// is not new: the saved `ProgressionOrientation` when valid
    /// (`SetLocationStartUpPosition`), else the `fe_locations` start's
    /// location name (layout word 1, `SetLocationName`). `GameLoad` streams it in, and
    /// `ResetGame` hands the PlayerUI the startup transform (`OnResetPos` `0x828aa428`) or the
    /// name's hash, so the game's first `ResetPlayer` spawns player 0 there.
    fn place_continued_career(&mut self) -> Result<(), SelectedLiveError> {
        let career = self.career.as_ref().ok_or_else(|| error("career absent"))?;
        let world = career.system.world();
        let (valid, rows) = world.signpost.progress.orientation;
        let transform = if valid {
            let mut transform = [0.0; 16];
            for (i, row) in rows.iter().enumerate() {
                transform[i * 4..i * 4 + 3].copy_from_slice(row);
            }
            transform[15] = 1.0;
            Some(transform)
        } else {
            world
                .db
                .layout_text(class::FE_LOCATIONS, CONTINUE_START, 1)
                .and_then(|name| career.location_transforms.get(&hash_value32(name)).copied())
        };
        let transform = transform.ok_or_else(|| error("continued career start location absent"))?;
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: continue at {:?} (saved orientation {valid})", [transform[12], transform[13], transform[14]]);
        }
        self.travel_to_career_start(transform);
        Ok(())
    }

    /// The challenge code's `StatsBackEnd::UpdateStats` calls, applied with the frame's outputs
    /// and before a save writes the statistics.
    fn apply_stat_updates(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let updates = std::mem::take(&mut career.system.world_mut().stat_updates);
        if updates.is_empty() {
            return Ok(());
        }
        let owners = &self.inner.player.as_ref().ok_or_else(|| error("statistics player absent"))?.selected.owners;
        for (item, value, force) in updates {
            owners.services.skaters.backend().update_additive_statistic(item, value, force, &owners.cvars).map_err(error)?;
        }
        Ok(())
    }

    /// The autosave (the memcard save of every savable) for the ported
    /// savables: `UNLOCKABLES` (`UnlockablesBackEnd::SaveData` `0x8255ce38`),
    /// `SKATER_PROFILE`, whose `mNewProfile` the new career's
    /// `FrontEndState_CaC::Shutdown` cleared, `Sk8ChallengeSystem`
    /// (`Sk8ChallengeSystem::SaveData` `0x825a2ec0`, the dynamic hull
    /// states) and `SignPost` (`cSignPostManager::SaveData` `0x825cdef8`).
    /// The career's savables as they are now, written by another thread as the storage
    /// device writes the original's; every later profile access waits for it.
    fn save_career(&mut self) -> Result<(), SelectedLiveError> {
        self.apply_stat_updates()?;
        let in_additional_world = self.inner.terrain.as_ref().is_some_and(|terrain| terrain.additional_world().is_some());
        let asks = self.career.as_ref().is_some_and(|career| signpost_progress::asks_progression_orientation(career.system.world(), in_additional_world));
        if asks && let Some(rows) = self.local_suggested_respawn()? {
            let career = self.career.as_mut().ok_or_else(|| error("career absent"))?;
            signpost_progress::set_progression_orientation(career.system.world_mut(), rows);
        }
        let career = self.career.as_ref().ok_or_else(|| error("career absent"))?;
        let calendar=career.system.world().hom_local_calendar.ok_or_else(||error("statistics save requires host calendar"))?();
        let stats=self.inner.player.as_ref().ok_or_else(||error("statistics player absent"))?.selected.owners.services.skaters.backend().save_statistics(calendar).map_err(error)?;
        let leaves = signpost_progress::save(career.system.world());
        let unlocks = career.system.world().unlockables.save();
        let hulls = career.system.world().dynamic_hulls.save();
        let cash = signpost_progress::cash(career.system.world());
        let (oriented, orientation) = career.system.world().signpost.progress.orientation;
        let path = EditSkaterProfileStore::default_path().map_err(error)?;
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?.mounts);
        let settings = self.audio.settings;
        let music = self.audio.manager.as_ref().map(|manager| (manager.profile.clone(), manager.catalogue().to_vec()));
        let handed = HandedSave::hand_over();
        self.audio.new_profile = false;
        self.career_saves.start(move || {
            let saved = handed.write(path, |store| -> Result<(), String> {
                let mut data = match store.load_profile_data().map_err(|e| e.to_string())? {
                    Some(data) => data,
                    // ProfileBackEnd::ResetProfile 0x8256d8f0.
                    None => ProfileData::reset(
                        &localized(&mounts, "ID_DEFAULT_PLAYER_NAME").map_err(|e| e.to_string())?,
                        &localized(&mounts, "ID_DEFAULT_PLAYER_NICKNAME").map_err(|e| e.to_string())?,
                    ),
                };
                data.new_profile = false;
                let audio = AudioProfile {
                    settings: &settings,
                    music_player: music.as_ref().map(|(player, catalogue)| (player, catalogue.as_slice())),
                };
                store
                    .save_memcard(
                        &data,
                        Some(&audio),
                        &[
                            (unlockables::SAVABLE, &[], &unlocks),
                            (crate::challenge::hom_save::SAVABLE, &crate::challenge::hom_save::BEFORE, &stats),
                            (dynamic_hull::SAVABLE, &dynamic_hull::SAVABLES_BEFORE, &hulls),
                            (signpost_progress::SAVABLE, &signpost_progress::SAVABLES_BEFORE, &leaves),
                        ],
                    )
                    .map_err(|e| e.to_string())
            });
            match saved.map_err(|e| e.to_string()).and_then(|saved| saved) {
                Ok(()) => eprintln!("career: autosaved (cash {cash}, orientation {oriented} at {:?})", orientation[3]),
                Err(e) => eprintln!("career: autosave failed: {e}"),
            }
        });
        Ok(())
    }

    /// The career's `GetCash` and unlockables, which the editor's shop reads.
    pub(super) fn career_shop_funds(&self) -> Option<(i32, crate::challenge::unlockables::Unlockables)> {
        let world = self.career.as_ref()?.system.world();
        Some((signpost_progress::cash(world), world.unlockables.clone()))
    }

    /// The editor's close on the career: `FrontEndState_CaC::Shutdown` (`0x82659d28`) leaves
    /// the clothing unlockables (categories 15 and 16) no longer new, and each line the receipt
    /// bought leaves the cash (`OnMenuSelect`'s `SubtractCash(cost, 0)`, `0x826810b0`). The
    /// native writes neither until its next save; the port saves the record as the editor
    /// closes, so a purchase saves the career with it.
    #[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
    pub(super) fn close_editor_shop(&mut self, costs: &[i32]) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let world = career.system.world_mut();
        for category in [crate::edit_skater_shop::CLOTHING_UNLOCK, crate::edit_skater_shop::CLOTHING_SPONSOR] {
            world.unlockables.reset_all_new_unlocks_from_category(category);
        }
        if costs.is_empty() {
            return Ok(());
        }
        // OnMenuSelect 0x826810b0..0x826810f4: each line's cost leaves the cash, then adds to
        // statistic 9 (forced).
        for &cost in costs {
            signpost_progress::subtract_cash(world, cost, 0);
            world.stat_updates.push((9, cost.cast_unsigned() as f32, true));
        }
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: shop bought {costs:?}, cash {}", signpost_progress::cash(world));
        }
        self.save_career()
    }

    #[must_use]
    pub fn career_active(&self) -> bool {
        self.career.is_some()
    }

    /// A NIS's post-fx is active (`mICEPostfxActive`, from `Scene::Init` to `Scene::Purge`): the
    /// cutscene draws through PostFX whatever the setting.
    #[must_use]
    pub fn cutscene_postfx_active(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.hud.nis.ice_postfx.is_some())
    }

    /// The start hub's Restart Challenge (`RetrySelectedCallback` `0x8265ea20`):
    /// `RestartMainChallenge(0, false)`.
    pub(crate) fn career_restart_main_challenge(&mut self) {
        if let Some(career) = self.career.as_mut() {
            career.system.restart_main_challenge(0, false);
        }
    }

    /// Whether `FrontEndState_PressA`'s dialog is open.
    #[must_use]
    /// The sign-up poster is up and takes A.
    pub fn career_poster_ready(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.hud.poster.is_some() && !career.poster_blocked)
    }

    /// The first four players' types, names and peers, for a diagnostic run.
    pub fn diagnostic_player_rows(&self) -> Vec<(u32, String, i64)> {
        let Some(player) = self.inner.player.as_ref() else { return Vec::new() };
        let settings = &player.selected.owners.services.settings;
        (0..4)
            .map(|row| {
                (
                    settings.try_player_type(row).unwrap_or(0),
                    settings.try_player_name(row).unwrap_or_default(),
                    settings.try_peer_id(row).unwrap_or(-1),
                )
            })
            .collect()
    }

    /// Signs up the named challenge for a diagnostic run (`SKATE2_DIAGNOSTIC_CHALLENGE`).
    pub fn diagnostic_sign_up_challenge(&mut self, name: &str) -> bool {
        self.career.as_mut().is_some_and(|career| career.hud.poster.is_none()
            && !career.overlay_open() && career.system.diagnostic_sign_up(name))
    }

    /// The sweep declines unaffordable posters instead of entering the native fee-failure lock.
    pub fn career_poster_affordable(&self) -> bool {
        self.career.as_ref().and_then(|c| c.hud.poster.as_ref()).is_some_and(|p|
            (!p.betting || p.enough_for_min_bet) && (p.entry_fee_paid || p.cash >= p.entry_fee))
    }

    /// `ChallengeMusic.PlayMusic(trigger)` of the named challenge for a diagnostic run
    /// (`SKATE2_DIAGNOSTIC_MISSION_MUSIC`): the trigger reaches the audio system as the challenge
    /// audio manager's does.
    pub fn diagnostic_trigger_mission_music(&mut self, trigger: i32, name: &str) -> bool {
        let Some(career) = self.career.as_mut() else { return false };
        let activities = &career.system.world().activities.activities;
        let Some(challenge) = activities.iter().find(|activity| activity.name == name).map(|activity| activity.key) else { return false };
        career.system.world_mut().output(Output::TriggerMissionMusic { trigger, challenge });
        true
    }

    /// Abandons the main challenge for a diagnostic sweep (`SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP`).
    pub fn diagnostic_abandon_challenge(&mut self) -> bool {
        self.career.as_mut().is_some_and(|career| career.system.diagnostic_abandon())
    }

    /// Runs the named global activity for a diagnostic sweep (`+name`).
    pub fn diagnostic_run_global_activity(&mut self, name: &str) -> bool {
        self.career.as_mut().is_some_and(|career| career.system.diagnostic_run_global(name))
    }

    /// Sends the named teleporter's sign-up for a diagnostic sweep (`!name`).
    pub fn diagnostic_teleporter_sign_up(&mut self, name: &str) -> bool {
        self.career.as_mut().is_some_and(|career| career.system.diagnostic_teleporter_sign_up(name))
    }

    /// The main challenge's name and state.
    #[must_use]
    pub fn career_main_challenge(&self) -> Option<(String, Option<String>)> {
        let career = self.career.as_ref()?;
        Some((career.system.main_challenge_name()?, career.system.main_challenge_state()))
    }

    /// Shows the sign-up prompt for several challenges for a diagnostic run
    /// (`SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT`).
    pub fn diagnostic_sign_up_prompt(&mut self, names: &[&str]) -> bool {
        self.career.as_mut().is_some_and(|career| career.system.diagnostic_sign_up_prompt(names))
    }

    /// Forces the main challenge's active objectives complete for a diagnostic run
    /// (`SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES`).
    pub fn diagnostic_force_objectives(&mut self) -> Vec<String> {
        self.career.as_mut().map_or_else(Vec::new, |career| career.system.diagnostic_force_active_objectives())
    }

    /// Completes the main challenge's active objectives as their completion scripts would, for a
    /// diagnostic win (`SKATE2_DIAGNOSTIC_FILM_WIN`).
    pub fn diagnostic_complete_objectives(&mut self) -> Vec<String> {
        self.career.as_mut().map_or_else(Vec::new, |career| career.system.diagnostic_complete_active_objectives())
    }

    /// The main challenge's current state name.
    #[must_use]
    pub fn career_main_challenge_state(&self) -> Option<String> {
        self.career.as_ref().and_then(|career| career.system.main_challenge_state())
    }

    pub fn career_press_a_open(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.hud.press_a.is_some())
    }

    /// FE state 8 is open with its screen ready.
    #[must_use]
    /// Current result-page options for the opt-in controller diagnostic.
    pub fn diagnostic_end_options(&self) -> Option<[i32; 3]> {
        let view = self.career.as_ref()?.hud.end_options.as_ref()?;
        Some(std::array::from_fn(|slot| view.leaderboard.option(slot, view.options.win)))
    }

    pub fn career_end_options_ready(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.hud.end_options.is_some() && career.end_options_ready)
    }

    /// FE state 39 is open with its screen ready.
    #[must_use]
    pub fn career_change_game_ready(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.change_game.is_some() && career.change_game_ready)
    }

    /// `SkaterManager::GetLocalIndex` as the challenge system last set it.
    #[must_use]
    pub fn career_local_player(&self) -> Option<i32> {
        self.career.as_ref().map(|career| career.system.world().local_player)
    }

    /// Central messages and objective HUD screen states since the last call,
    /// and the current HUD state.
    pub fn take_career_hud(&mut self) -> Option<CareerHud> {
        let career = self.career.as_mut()?;
        let central_messages = std::mem::take(&mut career.hud.central_messages);
        let objective_screen = std::mem::take(&mut career.hud.objective_screen);
        let tutorial_screen = std::mem::take(&mut career.hud.tutorial_screen);
        let signup_screen = std::mem::take(&mut career.hud.signup_screen);
        let nis_screens = std::mem::take(&mut career.hud.nis_screens);
        let leaderboard_screen = std::mem::take(&mut career.hud.leaderboard_screen);
        let abd_screen = std::mem::take(&mut career.hud.abd_screen);
        let timer_screen = std::mem::take(&mut career.hud.timer_screen);
        let big_black_timer_screen=std::mem::take(&mut career.hud.big_black_timer_screen);
        let countdown_screen = std::mem::take(&mut career.hud.countdown_screen);
        let voting_screen = std::mem::take(&mut career.hud.voting_screen);
        let throwdown_screen = std::mem::take(&mut career.hud.throwdown_screen);
        let dpo_screen = std::mem::take(&mut career.hud.dpo_screen);
        let distance_screen = std::mem::take(&mut career.hud.distance_screen);
        let change_game_screen = std::mem::take(&mut career.hud.change_game_screen);
        let photo_select_screen = std::mem::take(&mut career.hud.photo_select_screen);
        let slappys_screen = std::mem::take(&mut career.hud.slappys_screen);
        let phone_data_dirty = std::mem::take(&mut career.hud.phone_data_dirty);
        Some(CareerHud {
            profile_name: self.inner.player.as_ref().and_then(|player| player.selected.owners.services.profile.name()).unwrap_or_default(),
            sponsor_labels: crate::challenge::sponsors::frontend_sponsor_labels(career.system.world()),
            hom_completed:career.system.world().hom.completed_count(),
            central_messages,
            objective_screen,
            tutorial_screen,
            signup_screen,
            nis_screens,
            leaderboard_screen,
            abd_screen,
            timer_screen,
            big_black_timer_screen,
            countdown_screen,
            voting_screen,
            throwdown_screen,
            dpo_screen,
            distance_screen,
            change_game_screen,
            photo_select_screen,
            slappys_screen,
            phone_data_dirty,
            // Options' ToggleSubtitles (0x8265c5e8) flips mSubtitles at once, in a challenge too.
            subtitles: self.audio.settings.subtitles,
            ..career.hud.clone()
        })
    }

    /// `PlayersBackEnd::IsPlayerStanceRegular` (`0x8255b500`) for the local
    /// player.
    pub(super) fn career_stance_regular(&self) -> Result<bool, SelectedLiveError> {
        let input = self.inner.bound_input.as_ref().ok_or_else(|| error("career input absent"))?;
        let player = &self.inner.player.as_ref().ok_or_else(|| error("career player absent"))?.selected;
        Ok(player.owners.services.settings.try_stance_byte(input.player_index()).map_err(error)? != 0)
    }

    /// `SkaterManager::GetLocalIndex`.
    fn skater_local_index(&self) -> Result<i32, SelectedLiveError> {
        let owner = self.inner.world.as_ref().ok_or_else(|| error("career world absent"))?;
        let local = owner
            .world
            .lock()
            .map_err(|_| error("world lock poisoned"))?
            .local_skater_identity
            .clone()
            .ok_or_else(|| error("SkaterManager local identity absent"))?;
        local.try_local_index().map_err(|e| error(format!("{e:?}")))
    }

    /// `SkaterManager::SetLocalIndex` 0x82600380.
    fn set_skater_local_index(&self, index: i32) -> Result<(), SelectedLiveError> {
        let owner = self.inner.world.as_ref().ok_or_else(|| error("career world absent"))?;
        let local = owner
            .world
            .lock()
            .map_err(|_| error("world lock poisoned"))?
            .local_skater_identity
            .clone()
            .ok_or_else(|| error("SkaterManager local identity absent"))?;
        local.try_set_local_index(index).map_err(|e| error(format!("{e:?}")))
    }

    /// Every skater's completed output this update, in the skater manager's index order;
    /// the one at `mLocalIndex` is local.
    pub(super) fn career_mission_frames(&self) -> Result<Vec<MissionFrame>, SelectedLiveError> {
        let local = self.skater_local_index()?;
        let player = self.inner.player.as_ref().ok_or_else(|| error("career player absent"))?;
        let primary = player.selected.actor.as_ref().ok_or_else(|| error("career Actor absent"))?;
        let actors: Vec<_> = std::iter::once(Arc::clone(primary))
            .chain(self.inner.party.iter().map(|skater| Arc::clone(&skater.actor)))
            .chain(self.inner.ai_skaters.iter().map(|skater| Arc::clone(&skater.actor)))
            .collect();
        let mut frames = Vec::with_capacity(actors.len());
        for actor in actors {
            let index = {
                let prefix = actor.prefix().map_err(error)?;
                let fields = prefix.fields().ok_or_else(|| error("career Actor fields absent"))?;
                i32::try_from(fields.player_index).map_err(error)?
            };
            let physical = actor.cached_physical_allocation().map_err(error)?;
            let output = physical.owner().phys_out().lock().map_err(|_| error("PhysOut lock poisoned"))?;
            frames.push(output.try_mission_frame(index, index == local, true).map_err(|e| error(format!("{e:?}")))?);
        }
        frames.sort_by_key(|frame| frame.player);
        Ok(frames)
    }

    /// The players back end the challenge natives read and write, and
    /// `CommonBackEnd::sGameInfo.gameMode`.
    fn bind_challenge_players(&self, career: &mut CareerSession, game_mode: i32) -> Result<(), SelectedLiveError> {
        let player = self.inner.player.as_ref().ok_or_else(|| error("challenge player absent"))?;
        let settings = Arc::clone(&player.selected.owners.services.settings);
        let world = career.system.world_mut();
        world.players = Some(Arc::clone(&settings) as Arc<dyn crate::challenge::players::PlayersBackEnd>);
        world.game_mode = game_mode;
        world.hom_stats=player.selected.owners.services.skaters.backend().hom_stats().map_err(error)?;
        world.ai_link = self.inner.ai.as_ref().map(|ai| crate::challenge::ai::AiLink {
            manager: Arc::clone(&ai.manager),
            paths: Arc::clone(&ai.paths),
            settings: Arc::clone(&settings),
            locations: Arc::clone(&career.locations) as Arc<dyn crate::challenge::ai::SpawnLocations>,
        });
        if let (Some(terrain), Some(owner)) = (self.inner.terrain.as_ref(), self.inner.world.as_ref())
            && let Ok(startup) = terrain.terrain_for(&owner.world)
        {
            world.host = Box::new(CareerHost {
                triggers: Arc::clone(&startup.triggers),
                locations: Arc::clone(&career.locations),
                photos: career.nis.photo_counter(),
                mounts: self.inner.assets.as_ref().map(|assets| Arc::clone(&assets.mounts)),
            });
        }
        self.bind_career_living_world(world)?;
        // The patterns are cTrickPatternManager statics; the new system's
        // cChallengeScoringManager (0x82da2178) enables every kind.
        world.scoring = Arc::clone(&self.inner.challenge_scoring);
        let mut scoring = world.scoring.lock().map_err(|_| error("challenge scoring lock poisoned"))?;
        scoring.current = None;
        scoring.enabled = [true; 14];
        scoring.hom_visuals = true;
        Ok(())
    }

    /// `LayerMan::GetAttribData` of `eFE_LocationTriggers` (layer 5) at a position's x/z, while the
    /// world painter's layers answer.
    fn career_location_key(&self, position: [f32; 3]) -> Option<u64> {
        let world = self.inner.world.as_ref()?;
        let terrain = self.inner.terrain.as_ref()?.terrain_for(&world.world).ok()?;
        let layers = terrain.layers.lock().ok()?;
        layers.try_attrib_key(5, [position[0], position[2]]).ok()
    }

    /// `LayerMan::GetAttribData` of `eSecurityGuardAreas` (layer 6) and `eServices` (layer 13) at a
    /// position's x/z, a miss answering key 0.
    fn career_challenge_layer_keys(&self, position: [f32; 3]) -> Option<(u64, u64)> {
        use crate::challenge::world_painter::layer;
        let world = self.inner.world.as_ref()?;
        let terrain = self.inner.terrain.as_ref()?.terrain_for(&world.world).ok()?;
        let layers = terrain.layers.lock().ok()?;
        let key = |id| layers.try_attrib_key(id, [position[0], position[2]]).unwrap_or(0);
        Some((key(layer::SECURITY_GUARD_AREAS), key(layer::SERVICES)))
    }

    /// `SKATE2_DIAGNOSTIC_DUMP_TRIGGERS`: the challenge message group's trigger volumes within
    /// `radius` of the local skater's tester (ids, transforms, volumes), and the skater's root
    /// matrix.
    pub fn diagnostic_dump_triggers(&self, radius: f32) -> Result<(), SelectedLiveError> {
        let Some(frame) = self.career.as_ref().and_then(|career| career.system.world().skaters.iter().find(|frame| frame.local).cloned()) else {
            return Ok(());
        };
        let [x, y, z] = frame.tester_base;
        let world = self.inner.world.as_ref().ok_or_else(|| error("career world absent"))?;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("career terrain absent"))?;
        let triggers = Arc::clone(&terrain.terrain_for(&world.world).map_err(|e| error(format!("{e:?}")))?.triggers);
        let triggers = triggers.lock().map_err(|_| error("trigger lock poisoned"))?;
        let bounds = OctreeBounds { minimum: [x - radius, y - radius, z - radius, 0.0], maximum: [x + radius, y + radius, z + radius, 0.0] };
        let indices = triggers.try_message_group_collisions(bounds, |_| true).map_err(|e| error(format!("{e:?}")))?;
        let root = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()).and_then(|actor| {
            let physical = actor.cached_physical_allocation().ok()?;
            let output = physical.owner().phys_out().lock().ok()?;
            Some(output.try_skeleton_output().ok()?.skeleton_root_matrix.columns().map(|column| column.xyz))
        });
        eprintln!("diagnostic triggers within {radius} of {:?}, root {root:?}:", frame.tester_base);
        for index in indices {
            let Some(record) = triggers.message_group_record(index) else { continue };
            let (guid, local) = record.trigger_guids().map_err(error)?;
            match record.trigger_bounding_volume().map_err(error)? {
                Some((volume, transform)) => eprintln!("  {guid:#x} {local:#x} transform {transform:?} volume {volume:?}"),
                None => eprintln!("  {guid:#x} {local:#x}: no model"),
            }
        }
        Ok(())
    }

    /// `cTriggerVolumeMessageGroup::Update` (`0x82e24020`) for the local
    /// tester: the challenge group entries its volume collides with.
    fn career_collisions(&self, frame: &MissionFrame, tuning: TesterTuning) -> Result<Vec<(u32, u64, u64)>, SelectedLiveError> {
        let tester = trigger_tester::generate(frame.tester_base, frame.tester_top, frame.tester_centre_of_mass, tuning);
        let tester_primitive = GpPrimitive::from_volume(&tester.volume, Some(tester.transform)).ok_or_else(|| error("tester volume"))?;
        let (minimum, maximum) = tester.bounds();
        let world = self.inner.world.as_ref().ok_or_else(|| error("career world absent"))?;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("career terrain absent"))?;
        let triggers = Arc::clone(&terrain.terrain_for(&world.world).map_err(|e| error(format!("{e:?}")))?.triggers);
        let triggers = triggers.lock().map_err(|_| error("trigger lock poisoned"))?;
        let mut failure = None;
        let indices = triggers
            .try_message_group_collisions(
                OctreeBounds { minimum: [minimum[0], minimum[1], minimum[2], 0.0], maximum: [maximum[0], maximum[1], maximum[2], 0.0] },
                |record| match record.trigger_bounding_volume() {
                    // IsColldingWithTriggerVolume 0x82e213b8: no model collides.
                    Ok(None) => true,
                    Ok(Some((volume, transform))) => GpPrimitive::from_volume(&volume, Some(transform))
                        .and_then(|trigger| rw_overlap(&tester_primitive, &trigger, 0.0))
                        .unwrap_or(false),
                    Err(e) => {
                        failure.get_or_insert(e);
                        false
                    }
                },
            )
            .map_err(|e| error(format!("{e:?}")))?;
        if let Some(e) = failure {
            return Err(error(e));
        }
        indices
            .into_iter()
            .map(|index| {
                let record = triggers.message_group_record(index).ok_or_else(|| error("trigger record absent"))?;
                let (guid, local) = record.trigger_guids().map_err(error)?;
                Ok((index, guid, local))
            })
            .collect()
    }

    /// One completed simulation update of the career.
    /// What the challenge system left for the HUD this update: the active objective, the mission
    /// title, the pro bio, the fade and the HUD binding, and every HUD screen's
    /// `SetScreenState` calls.
    fn collect_career_hud(&mut self) {
        let Some(career) = self.career.as_mut() else { return };
        let objective = career.active_objective();
        if objective != career.hud.objective && std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: objective {objective:?}");
        }
        career.hud.objective = objective;
        career.hud.mission_title = career.system.mission_title();
        career.hud.pro_info = career.system.pro_info();
        career.hud.fade = career.system.world().effects.fade_amount;
        career.hud.binding = career.system.world().hud.hud_binding();
        career.hud.start_hub = crate::challenge::start_hub::view(career.system.world());
        career.hud.can_show_phone_options=crate::challenge::contacts::can_show_options(career.system.world());
        career.hud.phone_service_icons=career.system.world().hud.group("phonelist")
            .and_then(|g|g.phonelist.as_ref()).map_or([false;3],|p|p.show_icon);
        career.hud.phone_alert_facts=crate::challenge::contacts::alert_facts(career.system.world());
        if let Some(phone)=career.system.world_mut().hud.group_mut("phonelist").and_then(|g|g.phonelist.as_mut()) {
            career.hud.phone_data_dirty|=std::mem::take(&mut phone.dirty);
        }
        career.hud.pro_film_pro_name=career.system.world().competitors.pro_film_pro
            .and_then(|key|career.system.world().db.layout_text(0x9c86_9217_c5ae_e9b4,key,18))
            .unwrap_or_default().to_owned();
        {
            use crate::challenge::{signpost_progress::points_by_name, sponsors};
            let world = career.system.world();
            career.hud.frontend_arguments = FrontendArguments {
                pools_found: points_by_name(world, "Pools Found"),
                tapes_given: points_by_name(world, "Circulate Pool"),
                sponsor_world: sponsors::sponsor_world(world, sponsors::signed_board_sponsor_key(world)),
                demo_world: sponsors::sponsor_world(world, sponsors::board_sponsor_key(world)),
                demo_sponsor: sponsors::demo_sponsor_hal_id(world),
            };
        }
        let states = std::mem::take(&mut career.system.world_mut().objective_hud.screen_states);
        career.hud.objective_screen.extend(states);
        let states = std::mem::take(&mut career.system.world_mut().tutorial_hud.screen_states);
        career.hud.tutorial_screen.extend(states);
        let states = std::mem::take(&mut career.system.world_mut().signup_hud.screen_states);
        career.hud.signup_screen.extend(states);
        let huds = &mut career.system.world_mut().turn_huds;
        career.hud.leaderboard_screen.append(&mut huds.leaderboard.screen_states);
        career.hud.abd_screen.append(&mut huds.abd.screen_states);
        career.hud.timer_screen.append(&mut huds.timer.screen_states);
        career.hud.big_black_timer_screen.append(&mut huds.big_black_timer.screen_states);
        career.hud.countdown_screen.append(&mut huds.countdown.screen_states);
        career.hud.voting_screen.append(&mut huds.voting.screen_states);
        career.hud.throwdown_screen.append(&mut huds.throwdown.screen_states);
        career.hud.dpo_screen.append(&mut huds.dpo.screen_states);
        career.hud.distance_screen.append(&mut huds.distance.screen_states);
    }

    /// `Sk8ChallengeSystem::GameUnload`'s `cActivityManager::GameUnload` (`0x82cda420`) as the
    /// career goes: the running activities stop, and the asset manager's hull unloads go to its
    /// loader, which the front end keeps updating ([`Self::try_update_unowned_challenge_hulls`]).
    pub(super) fn unload_career_activities(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        career.system.game_unload_activities();
        let outputs = std::mem::take(&mut career.system.world_mut().outputs);
        let (hulls, rest): (Vec<_>, Vec<_>) =
            outputs.into_iter().partition(|output| matches!(output, Output::LoadHull { .. } | Output::UnloadHull { .. }));
        career.system.world_mut().outputs = rest;
        // The online game keeps its challenge's hulls, GameModule::GameLoad's, through its rounds.
        let hulls = hulls.into_iter().filter(|output| !matches!(output, Output::UnloadHull { hull } if self.online_holds_hull(*hull))).collect();
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("career terrain absent"))?;
        queue_hull_requests(terrain.as_mut(), hulls)
    }

    /// The challenge system goes with its game. Every skater stops scoring by what it set: the
    /// scoring it shared outlives it here, and takes `ChallengeScoring::game_unload`.
    pub(super) fn drop_career(&mut self) -> Result<(), SelectedLiveError> {
        self.career = None;
        self.inner.challenge_scoring.lock().map_err(|_| error("challenge scoring lock poisoned"))?.game_unload();
        Ok(())
    }

    /// `HUDManager2` outlives the challenge system natively: once the system's game unloaded and
    /// its activities' groups went (`cChallengeHudTracker::ShutdownActivity` `0x82dac598`), the
    /// manager's next `Update` (`0x82cee4f0`) rebinds and hides every component they bound. The
    /// port's components go with the system, so that update runs before it goes, and the
    /// presentation takes its screen states afterwards ([`Self::take_ended_career_hud`]).
    pub(super) fn end_career_hud(&mut self) {
        let Some(career) = self.career.as_mut() else { return };
        career.system.update_hud();
        self.collect_career_hud();
        self.ended_career_hud = self.take_career_hud().map(|last| CareerHud {
            objective_screen: last.objective_screen,
            tutorial_screen: last.tutorial_screen,
            signup_screen: last.signup_screen,
            leaderboard_screen: last.leaderboard_screen,
            abd_screen: last.abd_screen,
            timer_screen: last.timer_screen,
            big_black_timer_screen: last.big_black_timer_screen,
            countdown_screen: last.countdown_screen,
            voting_screen: last.voting_screen,
            throwdown_screen: last.throwdown_screen,
            dpo_screen: last.dpo_screen,
            distance_screen: last.distance_screen,
            mission_title: last.mission_title,
            subtitles: last.subtitles,
            ..CareerHud::default()
        });
    }

    /// The screen states of the last HUD update of a challenge system that went outside a game
    /// unload, once ([`Self::end_career_hud`]); nothing else of its HUD is left.
    pub fn take_ended_career_hud(&mut self) -> Option<CareerHud> {
        self.ended_career_hud.take()
    }

    /// `AudioSystem::QuitChallenge` (`0x824ff478`), the audio system's `cMsgAbandonChallenge`
    /// handler, for the career's challenge.
    pub(super) fn quit_career_challenge_music(&self) {
        if let Some(audio) = self.career.as_ref().and_then(|career| career.audio.as_ref()) {
            audio.quit_challenge();
        }
    }

    /// `Sk8ChallengeSystem::GameUnload`'s `cChallengeDynamicHullManager::GameUnload`
    /// (`0x82da6470`) as the career or free-skate game goes: its loaded hulls' unloads go to the
    /// loader, whose later updates finish them (`GameModule::GameLoad` `0x8273be50` waits for
    /// `IsAllUnloaded` before the next game).
    pub(super) fn unload_career_dynamic_hulls(&mut self) -> Result<(), SelectedLiveError> {
        let Some(system) = self.career.as_mut().map(|career| &mut career.system).or(self.freeskate.hulls.as_mut()) else {
            return Ok(());
        };
        system.game_unload_dynamic_hulls();
        let outputs = std::mem::take(&mut system.world_mut().outputs);
        let (hulls, rest): (Vec<_>, Vec<_>) = outputs.into_iter().partition(is_dynamic_hull_request);
        system.world_mut().outputs = rest;
        self.dynamic_hulls_unloading = true;
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("career terrain absent"))?;
        queue_dynamic_hull_requests(terrain.as_mut(), hulls)
    }

    /// The free-skate game's challenge system for its dynamic hulls: the free-skate data, the
    /// global locations, the profile's unlockables (`UnlockablesBackEnd::LoadData` `0x8255cd08`)
    /// and dynamic hull states (`cChallengeDynamicHullManager::LoadData` `0x82db4330`), then
    /// `GameLoad(2)`.
    pub(super) fn free_skate_dynamic_hulls(&self) -> Result<ChallengeSystem, SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("free skate assets absent"))?.mounts);
        let (db, table) = self.freeskate.data.as_ref().ok_or_else(|| error("free skate data absent"))?;
        let global = mounts.read_path(GLOBAL_LUA).map_err(error)?;
        let mut system = ChallengeSystem::new_career(Arc::clone(db), &global).map_err(error)?;
        for location in &table.locations {
            system.world_mut().locations.insert(hash_value32(&location.name), location.transform);
        }
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let unlocks = store.load_leaf(unlockables::SAVABLE, unlockables::DATA).map_err(error)?;
        system.world_mut().unlockables.load(leaf_tuples(&unlocks));
        let hulls = store.load_leaf(dynamic_hull::SAVABLE, dynamic_hull::DATA).map_err(error)?;
        system.world_mut().dynamic_hulls.load(leaf_tuples(&hulls));
        system.game_load_free_skate_dynamic_hulls();
        Ok(system)
    }

    /// An online game's challenge system for its dynamic hulls: a network game's `GameLoad(8)`,
    /// which loads none of the career's gates, whatever the profile has opened.
    pub(super) fn online_dynamic_hulls(&self) -> Result<ChallengeSystem, SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("online assets absent"))?.mounts);
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("online data absent"))?;
        let global = mounts.read_path(GLOBAL_LUA).map_err(error)?;
        let mut system = ChallengeSystem::new_career(Arc::clone(db), &global).map_err(error)?;
        system.game_load_network_dynamic_hulls();
        Ok(system)
    }

    /// Free skate's dynamic hull manager `Update` (`0x82db4dd0`) at the local skater's tester,
    /// then its loader's; without one, the loader finishes a quit's unloads.
    fn try_update_free_skate_dynamic_hulls(&mut self) -> Result<(), SelectedLiveError> {
        if self.freeskate.hulls.is_none() {
            return self.try_update_unowned_dynamic_hulls();
        }
        self.try_update_unowned_challenge_hulls()?;
        let tester = self
            .career_mission_frames()?
            .into_iter()
            .find(|frame| frame.local)
            .and_then(|frame| frame.tester_valid.then_some(frame.tester_base));
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("free skate terrain absent"))?;
        let Some(system) = self.freeskate.hulls.as_mut() else { return Ok(()) };
        system.world_mut().dynamic_hulls.teleport_hold = self.map_travel.is_some();
        system.update_dynamic_hulls(tester);
        update_dynamic_hull_loader(system, terrain.as_mut())?;
        // Nothing else of the free-skate challenge system runs to take its other requests.
        system.take_outputs();
        Ok(())
    }

    /// The dynamic hull loader's update while no game's challenge system runs it (the front end's
    /// simulation, Party Play): `cActivityManager::Update` (`0x82cdba20`) updates every
    /// sub-controller in any game, so the manager's loader finishes the unloads a game's unload
    /// asked for.
    pub(super) fn try_update_unowned_dynamic_hulls(&mut self) -> Result<(), SelectedLiveError> {
        self.try_update_unowned_challenge_hulls()?;
        let Some(terrain) = self.inner.terrain.as_mut() else { return Ok(()) };
        if terrain.dynamic_hulls_unloaded() {
            return Ok(());
        }
        terrain.try_update_dynamic_hulls().map_err(error)?;
        Ok(())
    }

    /// The asset manager's `cPathedHullLoader::Update` while no game's challenge system runs it:
    /// `cChallengeAssetManager::Update` runs in any game, so the loader finishes the unloads the
    /// career's unload asked for.
    fn try_update_unowned_challenge_hulls(&mut self) -> Result<(), SelectedLiveError> {
        let Some(terrain) = self.inner.terrain.as_mut() else { return Ok(()) };
        if terrain.hulls_unloaded() {
            return Ok(());
        }
        terrain.try_update_hulls().map_err(error)?;
        Ok(())
    }

    /// `Simulation::PostUpdate` (`0x8289fd88`) while a NIS or the challenge holds the
    /// simulation paused: the front end's update, the challenge broadcaster's queue flushed (a
    /// photo outro's `cMsgNISChallengeNotify` reaches its challenge mid-scene), the highlight
    /// reels' update and the HUD, then what the challenge asked for; no simulation update.
    pub(super) fn try_update_career_paused(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let system = &mut career.system;
        // PostUpdate8289fd88 flushes challenge events even during NIS. Lua's
        // Random.GetRandomNumber8288adb8 still consumes the Simulation stream.
        system.world_mut().random = self.inner.bound_input.as_mut().map(|input| input.simulation_random_mut().clone());
        system.update_frontend();
        system.flush();
        system.post_update();
        system.update_hud();
        if let (Some(random), Some(input)) = (system.world_mut().random.take(), self.inner.bound_input.as_mut()) {
            *input.simulation_random_mut() = random;
        }
        self.apply_career_outputs()?;
        self.collect_career_hud();
        Ok(())
    }

    pub(super) fn try_update_career(&mut self) -> Result<(), SelectedLiveError> {
        if self.online_challenge_running() {
            return self.update_online_challenge();
        }
        self.career_step(None)
    }

    /// One update of the challenge system on the skaters' completed outputs. An online
    /// challenge gives every player's output of its tick and the local player's index: the
    /// system then reads the challenge's own random stream, and the local skater's simulation
    /// messages stay with it.
    pub(super) fn career_step(&mut self, online: Option<(Vec<MissionFrame>, i32, Option<MissionFrame>)>) -> Result<(), SelectedLiveError> {
        self.deliver_career_waypoint_groups()?;
        let challenge_messages = self.inner.try_take_challenge_messages().map_err(error)?;
        let Some(career) = self.career.as_mut() else { return self.try_update_free_skate_dynamic_hulls() };
        if online.is_none() {
            for message in challenge_messages {
                career.system.post(challenge_queue_event(message));
            }
        }
        // Flow::StateLoading updates before the challenge system. AI selection uses the same
        // Simulation RNG as the rest of this frame, including its queued Lua callbacks.
        if career.map_challenge_loading() {
            career.system.world_mut().random = self.inner.bound_input.as_mut().map(|input| input.simulation_random_mut().clone());
            career.start_loaded_map_challenge()?;
            if let (Some(random), Some(input)) = (career.system.world_mut().random.take(), self.inner.bound_input.as_mut()) {
                *input.simulation_random_mut() = random;
            }
        }
        career.start_loaded_party_challenge();
        let online_random = career.online_random.take();
        let hom_enabled=self.inner.player.as_ref().ok_or_else(||error("HoM player absent"))?.selected.owners.cvars.try_hom_challenge_enabled().map_err(error)?;
        let (frames, local_index, own) = match online {
            Some(online) => online,
            None => (self.career_mission_frames()?, self.skater_local_index()?, None),
        };
        // What only this console reads of its own skater, from its whole output.
        let own = own.or_else(|| frames.iter().find(|frame| frame.local).cloned());
        let regular = self.career_stance_regular()?;
        let tuning = self.career.as_ref().map(|c| c.tester).unwrap_or_default();
        let mut collisions = Vec::with_capacity(frames.len());
        for frame in &frames {
            collisions.push(if frame.tester_valid { Some(self.career_collisions(frame, tuning)?) } else { None });
        }
        let is_online = online_random.is_some();
        let random = online_random.or_else(|| self.inner.bound_input.as_mut().map(|input| input.simulation_random_mut().clone()));
        // Simulation::SimUpdate1 runs WorldPainter::LayerMan::Update (0x82ceb108) at the local
        // actor's x/z; a new eFE_LocationTriggers key reaches the objectives manager's
        // WorldPainterKeyChangeMsg (0x82d42d58) as CachedLocationKey.
        let per_player = self.online_round_per_player_objectives();
        let hom_location = own.as_ref().and_then(|frame| self.hom_location_name(frame.position));
        let location_key = own.as_ref().and_then(|frame| self.career_location_key(frame.position));
        let layer_keys = own.as_ref().and_then(|frame| self.career_challenge_layer_keys(frame.position));
        let memcard_active = self.career_saves.busy();
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let loading = self.map_travel.is_some() || career.map_challenge_loading();
        let CareerSession { system, mission, tuning: mission_tuning, previous, .. } = career;
        system.world_mut().flow_loading = loading;
        system.world_mut().memcard_active = memcard_active;
        system.world_mut().hom_enabled=hom_enabled;
        // PhysOutModule_MissionSystem::Update 0x82e313a0 runs on each skater's output.
        for frame in &frames {
            mission.update(frame, *mission_tuning, &mut |event| system.post(event));
        }
        let pairs = |entries: &[(u32, u64, u64)]| entries.iter().map(|&(_, guid, local)| (guid, local)).collect::<Vec<_>>();
        // An online freeskate activity's objectives each read their own player's state, from
        // every player's facts alike on every console (PC port: the original read each skater's
        // own output).
        system.world_mut().player_states = if per_player {
            frames
                .iter()
                .map(|frame| (frame.player, mission::player_state(frame, pairs(previous.get(&frame.player).map_or(&[][..], Vec::as_slice)))))
                .collect()
        } else {
            Vec::new()
        };
        if let Some(frame) = own.as_ref() {
            system.world_mut().player = mission::player_state(frame, pairs(previous.get(&frame.player).map_or(&[][..], Vec::as_slice)));
            // IsStanceTrickGesturesFlipped 0x82614648: (mCurrentStance != 0) ==
            // IsPlayerStanceRegular, mCurrentStance being mIsRidingSwitch.
            system.world_mut().stance_gestures_flipped = frame.is_riding_switch == regular;
        }
        system.world_mut().stance_regular = regular;
        if let Some((key,location))=hom_location {system.world_mut().hom_location=location;system.world_mut().hom_location_key=key;}
        if let Some(key) = location_key {
            system.world_mut().objectives.cached_location_key = Some(key);
        }
        // The same update's security guard area and service key changes reach the film and
        // service managers.
        if let Some((security_guard_areas, services)) = layer_keys {
            system.world_painter_update(security_guard_areas, services);
        }
        system.world_mut().local_player = local_index;
        system.world_mut().skaters.clone_from(&frames);
        // An online player's teleport is finished once its output says so.
        if let Some(online) = system.world_mut().online.as_mut() {
            for frame in frames.iter().filter(|frame| frame.is_teleport_finished) {
                if let Some(slot) = usize::try_from(frame.player).ok().and_then(|index| online.teleporting.get_mut(index)) {
                    *slot = false;
                }
            }
        }
        system.world_mut().tick = self.inner.selected_tick.as_ref().map_or(0, |tick| tick.presentation_tick());
        system.world_mut().random = random;
        system.world_mut().dynamic_hulls.teleport_hold = self.map_travel.is_some();
        system.update(loading, 1.0 / 60.0);
        // cChallengeAssetManager::Update ends with cPathedHullLoader::Update:
        // this update's hull requests, then each finished hull's OnComplete.
        let outputs = std::mem::take(&mut system.world_mut().outputs);
        let (hulls, rest): (Vec<_>, Vec<_>) =
            outputs.into_iter().partition(|output| matches!(output, Output::LoadHull { .. } | Output::UnloadHull { .. }));
        let (dynamic_hulls, rest): (Vec<_>, Vec<_>) = rest.into_iter().partition(is_dynamic_hull_request);
        system.world_mut().outputs = rest;
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("career terrain absent"))?;
        queue_hull_requests(terrain.as_mut(), hulls)?;
        for hull in terrain.try_update_hulls().map_err(error)? {
            crate::challenge::assets::on_complete(system.world_mut(), hull);
        }
        // cChallengeDynamicHullManager::Update (0x82db4dd0) ends with its own loader's update.
        queue_dynamic_hull_requests(terrain.as_mut(), dynamic_hulls)?;
        for hull in terrain.try_update_dynamic_hulls().map_err(error)? {
            system.dynamic_hull_load_complete(hull);
        }
        // cTriggerVolumeMessageGroup::Update 0x82e24020: each tester's entries, then exits.
        let mut local_entries = Vec::new();
        for (frame, current) in frames.iter().zip(collisions) {
            let Some(current) = current else { continue };
            let before_entries = previous.remove(&frame.player).unwrap_or_default();
            let before: Vec<u32> = before_entries.iter().map(|&(i, _, _)| i).collect();
            let now: Vec<u32> = current.iter().map(|&(i, _, _)| i).collect();
            let (entered, exited) = trigger_tester::enter_exit(&before, &now);
            for (enter, list, entries) in [(true, entered, &current), (false, exited, &before_entries)] {
                for index in list {
                    if let Some(&(_, guid, local)) = entries.iter().find(|(i, _, _)| *i == index) {
                        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                            eprintln!("career trigger: {} {local:#x} player {}", if enter { "enter" } else { "exit" }, frame.player);
                        }
                        system.post(Event::TriggerCollision { enter, volume: guid, player: frame.player });
                        system.post(Event::TriggerCollision { enter, volume: local, player: frame.player });
                        if enter && frame.local {
                            local_entries.push(local);
                        }
                    }
                }
            }
            if frame.local {
                system.world_mut().player.colliding_triggers = pairs(&current);
            }
            previous.insert(frame.player, current);
        }
        for volume in local_entries {
            self.online_local_entered(volume);
        }
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let system = &mut career.system;
        system.update_frontend();
        system.flush();
        system.post_update();
        system.update_hud();
        // Keep the SAME stream through EventSys::Flush (PostUpdate8289fd88).
        // AirCompleted/TimerExpired/TeleportTransitionFinished handlers can
        // consume it while selecting the next speaker, leader or AI path.
        if is_online {
            career.online_random = system.world_mut().random.take();
        } else if let (Some(random), Some(input)) = (system.world_mut().random.take(), self.inner.bound_input.as_mut()) {
            *input.simulation_random_mut() = random;
        }
        self.apply_career_outputs()?;
        self.apply_career_living_world()?;
        if let (Some(career), Some(ai)) = (self.career.as_ref(), self.inner.ai.as_mut()) {
            // AI::GetChallengeType 0x824bf008: the main challenge's ChallengeType, 52 without one.
            let world = career.system.world();
            ai.challenge_type = world.activities.main.map_or(52, |main| crate::challenge::signup::challenge_type(world, main));
            ai.respawn_locations = crate::challenge::ai::respawn_matrices(world);
            ai.respawn_location_set = world.ai_manager.respawn_location.is_some();
            ai.race_estimated_times = world.race.players.iter().map(|player| (player.racer_id, player.estimated_time_to_finish)).collect();
        }
        if let Some(career) = self.career.as_mut() {
            career.hud.result_info = career.system.submit_result_info();
        }
        self.collect_career_hud();
        let game_camera = self.game_camera_matrix();
        if let Some(career) = self.career.as_mut() {
            // The scene manager's frame; the native runs it per rendered frame
            // on the presentation thread (GameModule::Render 0x827320f0).
            career.nis.set_game_camera(game_camera);
            if career.nis.replay_clock().is_none() {
                let local = self.inner.local_index_presentation().map_err(error)?;
                career.nis.set_camera_anchor(local.and_then(|runtime| runtime.published_root()));
                career.nis.set_camera_local_hips(local.and_then(|runtime| runtime.published_hips_position()));
                career.nis.set_camera_local_hips_forward(local.and_then(|runtime| runtime.published_hips_forward()));
                career.nis.set_local_bounds(local.and_then(|runtime| runtime.published_bounds()));
            }
            career.set_local_player(
                self.inner
                    .player
                    .as_ref()
                    .and_then(|player| player.selected.owners.recipe_load.as_ref())
                    .and_then(|load| load.recipe().ok())
                    .map(|recipe| CareerSession::local_player_recipe(recipe)),
            );
            career.scene_source.set_players(self.inner.ai_skaters.iter().chain(self.inner.party.iter()).filter_map(|skater| {
                Some((i32::try_from(skater.player_index()).ok()?, CareerSession::local_player_recipe(skater.recipe_load.recipe().ok()?)))
            }));
            let scene_dt = career.nis.update_presentation_begin(1.0 / 60.0);
            if super::replay_session::update_replay_view(career, &self.replay_frames, &mut self.replay_clock, &mut self.replay_restorer)? {
                super::replay_session::restore_game_cloth(&mut self.inner, Some(career))?;
            }
            {
                let world_owner = self.inner.world.as_ref();
                let terrain = self.inner.terrain.as_ref();
                // Presentation::AddLineTest with kICECameraRadius over the selected world's
                // admitted ground (User_ICEWorldManager::HandleCollision).
                career.nis.update_presentation_end(scene_dt, &mut |start, end, radius| {
                    let owner = world_owner.ok_or("NIS camera line: world absent")?;
                    let ground = terrain
                        .ok_or("NIS camera line: terrain absent")?
                        .ground_pool_for(&owner.world)
                        .map_err(|e| format!("NIS camera line: {e}"))?;
                    let mut world = owner.world.lock().map_err(|_| "NIS camera line: world lock poisoned")?;
                    let line = crate::collision_line_collection::CollisionLineInput {
                        start: crate::skeleton_root_transforms::SkeletonRootVector::known(start),
                        end: crate::skeleton_root_transforms::SkeletonRootVector::known(end),
                        fatness: radius,
                    };
                    let hit = world
                        .try_world_line_test(
                            ground,
                            line,
                            "User_ICEWorldManager::HandleCollision",
                            &mut crate::collision_line_numeric::HostCollisionLineNumeric,
                        )
                        .map_err(|e| format!("NIS camera line: {e:?}"))?;
                    Ok(hit.map(|hit| hit.line_parameter))
                });
            }
            if let Some(assets) = self.inner.assets.as_ref() {
                career.present_scene_characters(&assets.mounts, &assets.vault, &assets.cvars);
            }
            career.publish_borrowed_skaters(self.inner.presentation_runtime.as_mut(), &mut self.inner.ai_skaters, &mut self.inner.party).map_err(error)?;
            let pauses = career.take_nis_events();
            let input = self.inner.bound_input.as_ref().ok_or_else(|| error("career input absent"))?;
            for paused in pauses {
                input.try_set_nis_pause(paused).map_err(error)?;
            }
        }
        Ok(())
    }

    /// Present the challenge system's requests.
    /// `cLocationManager` follows the registered simulation arenas.
    fn sync_locations(&mut self) -> Result<(), SelectedLiveError> {
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        let world = self.inner.world.as_ref().ok_or_else(|| error("career world absent"))?;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("career terrain absent"))?;
        let plans = &terrain.terrain_for(&world.world).map_err(|e| error(format!("{e:?}")))?.plans;
        career.locations.lock().map_err(|_| error("location manager lock poisoned"))?.sync(plans).map_err(error)
    }

    pub(super) fn apply_career_outputs(&mut self) -> Result<(), SelectedLiveError> {
        self.sync_locations()?;
        self.apply_stat_updates()?;
        let settings = self.inner.player.as_ref().map(|player| Arc::clone(&player.selected.owners.services.settings));
        let Some(career) = self.career.as_mut() else { return Ok(()) };
        // The audio system's cMsgSpeechEnded, through the challenge event broadcaster.
        let speech = career.audio.as_ref().map(|audio| audio.take_speech_messages()).unwrap_or_default();
        for message in speech {
            if let nis::audio::SpeechMessage::Ended { character, id } = message {
                career.present(HeldPresentation::Speech { character, id });
            }
        }
        let outputs = career.system.take_outputs();
        let party = career.system.world().game_mode == crate::challenge::players::GM_PARTY_PLAY;
        let online_local = career.system.world().online.as_ref().map(|online| online.local_player);
        let mut online_outputs = Vec::new();
        let mut end_points = Vec::new();
        let mut world_teleports = Vec::new();
        let mut start_teleports = Vec::new();
        let mut crowd_updates = 0;
        let mut markers = Vec::new();
        let mut marker_teleports = 0;
        let mut autosave = false;
        let mut sim_rates = Vec::new();
        let mut requests = Vec::new();
        let mut hulls = Vec::new();
        let mut dynamic_hulls = Vec::new();
        let mut ai_requests = Vec::new();
        for output in outputs {
            match output {
                // A challenge started outside the asset manager's update (a signpost answer,
                // a sign-up) queues its hulls too; the loader starts them on its next update.
                hull @ (Output::LoadHull { .. } | Output::UnloadHull { .. }) => hulls.push(hull),
                // A state set outside the manager's update (a progression script, a presentation
                // event, a purchase) unloads its old hull at the loader's next update.
                hull @ (Output::LoadDynamicHull { .. } | Output::UnloadDynamicHull { .. }) => dynamic_hulls.push(hull),
                Output::DynamicHullState { .. } => {}
                // PartyPlayUI handles cMsgSetPlayer; the other PlayerUIs do not subscribe.
                Output::SetPlayer { player } if party => requests.push(SkaterRequest::SetPlayer(player)),
                Output::SetPlayer { .. } => {}
                Output::SetLocalIndex { player } => requests.push(SkaterRequest::SetLocalIndex(player)),
                Output::PinPlayer { pin } => requests.push(SkaterRequest::Pin(pin)),
                Output::PinActor { player, pin } => requests.push(SkaterRequest::PinActor(player, pin)),
                Output::TeleportPlayer { location, pos_id, .. } if party => {
                    let locations = career.locations.lock().map_err(|_| error("location manager lock poisoned"))?;
                    let transform = locations.spawn_location(hash_value32(&location), pos_id);
                    if transform.is_none() {
                        eprintln!("career: teleport to unknown location {location} ({pos_id}), held {:?}", locations.counts());
                    }
                    drop(locations);
                    requests.extend(transform.map(SkaterRequest::Teleport));
                }
                Output::ModifiedCentralMessage(message) => career.hud.modified_central_messages.push(message.hal_id),
                Output::OpenPressADialog { player_turn, round } => {
                    let player = u32::try_from(player_turn - 1).ok();
                    let row = player.zip(settings.as_ref());
                    let controller = row.and_then(|(p, s)| s.try_controller_index(p).ok()).unwrap_or(-1);
                    let name = row.and_then(|(p, s)| s.try_player_name(p).ok()).unwrap_or_default();
                    career.hud.press_a = Some(PressADialog { player_turn, round, controller, name });
                }
                // ChallengeAudioManager::PlaySpeech (0x82da1b98): the audio system answers with
                // cMsgSpeechEnded; without one the line ends at once.
                Output::PlayNpcSpeech { character, id, param } => match career.audio.as_ref() {
                    Some(audio) => audio.play_mission_speech(character, id, param),
                    None => career.system.post(Event::SpeechEnded { character, id }),
                },
                Output::PlayCameraManCsisEvent { function } => {
                    if let Some(audio) = career.audio.as_ref() {
                        audio.play_camera_man_csis_event(function);
                    }
                }
                Output::ShowDialog { title, text, yes_no: true, .. } => {
                    career.dialog_owner = DialogOwner::Progression;
                    career.hud.dialog_options = None;
                    career.hud.dialog = Some((title, text, 0));
                    career.sounds.push("FE_SOUND_CORE_POPUP");
                }
                // FrontEnd.ShowYesNoPopup and ShowGenericAttributePopup: FrontEndManager::OpenPopup
                // (0x826147b8), default selection 0; FrontEndState_Popup::OnEnter (0x82658474)
                // sounds core_popup.
                Output::ShowPopup { id, title, description, options } => {
                    career.dialog_owner = DialogOwner::Script { id, generic: options.is_some() };
                    career.hud.dialog_options = options;
                    career.hud.dialog = Some((title, description, 0));
                    career.sounds.push("FE_SOUND_CORE_POPUP");
                }
                // FrontEnd.ClosePopup: StateManagerFrontEndBase::ClosePopup (0x82d68f40) closes
                // the popup without its callback.
                Output::ClosePopup { id } => {
                    if matches!(career.dialog_owner, DialogOwner::Script { id: open, .. } if open == id) && career.hud.dialog.is_some() {
                        career.hud.dialog = None;
                        career.hud.dialog_options = None;
                        career.dialog_owner = DialogOwner::default();
                    }
                }
                // OnEnter (0x82658880) forces the central HUD's outro and opens the screen.
                Output::OpenEndChallengeOptions(options) => {
                    career.hud.end_options = Some(EndOptionsView::new(options, career.system.world()));
            career.end_options_play_once = true;
                    career.pausing.push(PausingState::EndChallengeOptions);
                    career.end_options_ready = false;
                    career.end_options_wait_outro = false;
                    if online_local.is_some() && options.screen == crate::challenge::results::screen::ONLINE {
                        online_outputs.push(Output::OpenEndChallengeOptions(options));
                    }
                }
                // PopState of FE state 8's online display.
                Output::CloseOnlineContinueRetryDialog => {
                    if career.online_results_open() {
                        career.leave_end_options();
                    }
                }
                Output::ShowDialog { yes_no: false, .. } => career.system.post(Event::HudContinue),
                Output::CentralMessage(message) => career.hud.central_messages.push(message),
                // cTeleportManager::TeleportPlayer 0x828a90f8: the location's spawn point for the
                // player's position (GetSpawnLocation); online, a position past the sixth without
                // a spawn point of its own takes the sixth's.
                Output::TeleportPlayer { player, location, pos_id, set_session_marker } => {
                    let locations = career.locations.lock().map_err(|_| error("location manager lock poisoned"))?;
                    let hash = hash_value32(&location);
                    let transform = if online_local.is_some() { locations.online_spawn_location(hash, pos_id) } else { locations.spawn_location(hash, pos_id) };
                    if transform.is_none() {
                        eprintln!("career: teleport to unknown location {location} ({pos_id}), held {:?}", locations.counts());
                    }
                    drop(locations);
                    requests.extend(transform.map(|transform| SkaterRequest::TeleportPlayer(player, transform)));
                    if set_session_marker && let Some(transform) = transform {
                        markers.push(MarkerMessage::Set(Some(transform)));
                    }
                }
                Output::SetSessionMarker => markers.push(MarkerMessage::Set(None)),
                Output::ClearSessionMarker => markers.push(MarkerMessage::Clear),
                Output::TeleportToChallengeEndPoint { location } => end_points.push(location),
                Output::TeleportStartChallenge { challenge } => start_teleports.push(challenge),
                // Teleport.ToWorldFileLocation: TeleportLocalToWorldLocation (0x828a9cf0) teleports
                // to a location of the loaded world (the debug reference's BAM) as a challenge end
                // point does; another world's is an additional world location, which the port does
                // not load.
                Output::TeleportLocalToWorldLocation { world_location, world_file, location } => {
                    world_teleports.push((world_location, world_file, location));
                }
                Output::TeleportLocalPlayerToSessionMarker => marker_teleports += 1,
                Output::AutoSave => autosave = true,
                Output::SignUpPrompt { challenges } => {
                    if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
                        eprintln!("career: sign-up prompt {challenges:?} tester {:?}", career.system.world().player.tester_base);
                    }
                    career.hud.sign_up_prompt = challenges.iter().map(|c| career.challenge_text(c, CHALLENGE_TITLE)).collect();
                }
                // OpenPoster (0x82658240) pushes FE state 32, whose OnEnter
                // blocks its actions and opens screen 19.
                Output::OpenSignUpPoster { .. } => {
                    career.hud.poster = career.system.sign_up_poster_view();
                    career.poster_blocked = true;
                    career.poster_input_blocked = false;
                    career.poster_bet_input = (true, 0);
                    // challenge_info's SetMode(2) runs its BetLess for a betting challenge.
                    if let Some(view) = career.hud.poster.as_mut().filter(|view| view.betting) {
                        career.sounds.push(career.system.sign_up_bet_less());
                        view.bet = career.system.sign_up_bet_view();
                    }
                }
                Output::SetSimRate { sim_time, ticks } => sim_rates.push(crate::simulation_sim_rate::SetSimRate {
                    sim_time,
                    ticks_of_slow_motion: ticks,
                }),
                // FrontEndState_Popup::OnEnter (0x82658474) sounds core_popup.
                Output::SignUpChoice { options } => {
                    career.hud.sign_up_choice = Some((options.iter().map(|c| career.challenge_text(c, CHALLENGE_TITLE)).collect(), 0));
                    career.sounds.push("FE_SOUND_CORE_POPUP");
                }
                // The challenge system keeps the presentation's icons (World::map_icons).
                Output::ChallengeMapIcon { .. } => {}
                // NISManager::RunBasicNIS(scene, transition, false, true).
                Output::PlayNis { scene, transition, .. } => career.nis.run_basic_nis(&scene, transition, false, true),
                Output::RunNisDefinition { scene, ignore_breadcrumb, transition, hold_camera, skip_enabled, location, restore_streamer } => {
                    // GetRestorePosition (0x828a1418): the local skater's position.
                    let stream_position = match self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref()) {
                        Some(actor) if restore_streamer => Some(super::local_skater_position(self.inner.world.as_ref().map(|world| &world.world), actor)?),
                        _ => None,
                    };
                    let overrides = nis::manager::PlaybackOverrides { location, stream_position, player_indexes: Vec::new() };
                    career.nis.run_basic_nis_definition(&scene, ignore_breadcrumb, transition, hold_camera, skip_enabled, overrides);
                }
                Output::IgnoreTeleportToBreadcrumbOnNextNis => career.nis.ignore_teleport_to_breadcrumb_on_next_nis(),
                Output::ResetNisActorIds => career.nis.reset_actor_ids(),
                Output::ChallengeCamera { active, matrix } => career.challenge_camera = active.then_some(matrix),
                // CentralHUD screen states 6 and 7 are not presented yet.
                // CentralHUD screen states 6 and 7 run hudcentral's ShowCompleteMessage and
                // ShowFailedMessage, whose mcCongrats the shipped movie (d85e4167) lacks.
                Output::ChallengeResultMessage { .. } => {}
                Output::PreloadNisList(scenes) => career.nis.preload_nis_list(&scenes),
                Output::UnloadPreloadedNis => career.nis.unload_any_preloaded_niss(),
                Output::TriggerMissionMusic { trigger, challenge } => {
                    if let Some(audio) = career.audio.as_ref() {
                        // TriggerMissionMusic (0x82504f28) caches the spectator groups at a countdown.
                        if trigger == 0 {
                            let actor = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_deref());
                            let pods = self.inner.world.as_ref().zip(actor).map(|(owner, actor)| super::crowd_pod_infos(&owner.world, actor)).unwrap_or_default();
                            audio.cache_crowd_pod_infos(&pods);
                        }
                        let (challenge_type, race_tuning) = crate::challenge::audio_handlers::mission_music(career.system.world(), challenge);
                        audio.trigger_mission_music(trigger, challenge, challenge_type, race_tuning);
                    }
                }
                Output::QuitChallengeMusic => {
                    if let Some(audio) = career.audio.as_ref() {
                        audio.quit_challenge();
                    }
                }
                Output::RestartChallengeAudio { retry } => {
                    if let Some(audio) = career.audio.as_ref() {
                        audio.restart_challenge(retry);
                    }
                }
                // AudioSystem::PlayMissionSFX (0x82504e70): GetGlobalDataFromChallenge of no challenge
                // reads the default ChallengeType.
                Output::MissionAudio(crate::challenge::mission_audio::MissionAudio::Sfx { sfx, challenge }) => {
                    if let Some(audio) = career.audio.as_ref() {
                        let (challenge_type, race_tuning) = if challenge == 0 { (0, 0) } else { crate::challenge::audio_handlers::mission_music(career.system.world(), challenge) };
                        audio.play_mission_sfx(sfx, challenge, challenge_type, race_tuning);
                    }
                }
                // ContestPointsEarnedHandler 0x824f26a0, OneUpSuccessHandler 0x824ffc10,
                // OneUpFailureHandler 0x82500008, StartThrowdownChallengeHandler 0x824e7dc0,
                // StartTurnHandler 0x824ff828: these mission speech handlers are not ported.
                Output::MissionAudio(crate::challenge::mission_audio::MissionAudio::LeaderboardOverall { character }) => {
                    if let Some(audio) = career.audio.as_ref() {
                        audio.leaderboard_overall(character);
                    }
                }
                Output::MissionAudio(_) => {}
                Output::AddNisActorId(player) => career.nis.add_actor_id(player),
                // NIS.KillNIS (0x8288bd58): NISManager::KillNIS (0x828a2f88) sends cMsgNISUnloadAll
                // (SceneManagerImpl::ReceiveNISUnloadAllMsg 0x8286bf38) while IsInNIS.
                Output::KillNis => {
                    if career.nis.is_in_nis() {
                        career.nis.kill_nis();
                    }
                }
                // cChallengePauseManager::PauseSimulation (0x825b12f8) pushes
                // kPAUSE_STATE_CHALLENGE and UnpauseSimulation (0x825b12a0) pulls it.
                Output::ChallengePause { pause } => career.challenge_paused = pause,
                Output::RunReplayNis { scene, start, end, hold_camera, skip_enabled, location, stream_position, special_flags, .. } => {
                    let overrides = nis::manager::PlaybackOverrides { location, stream_position, player_indexes: Vec::new() };
                    career.nis.run_replay_nis(&scene, start, end, 0, hold_camera, skip_enabled, overrides, special_flags);
                }
                Output::TrackHomStatistics{bail,enabled}=>{
                    let owners=&self.inner.player.as_ref().ok_or_else(||error("HoM statistics player absent"))?.selected.owners;
                    owners.services.skaters.backend().track_hom_statistics(&bail,enabled,&owners.cvars,&owners.services.vault).map_err(error)?;
                }
                Output::ServiceStatistics{service,paid,pools}=>{
                    let owners=&self.inner.player.as_ref().ok_or_else(||error("service statistics player absent"))?.selected.owners;
                    owners.services.skaters.backend().record_service_statistics(service,paid,pools,&owners.cvars).map_err(error)?;
                }
                Output::GrantAchievement{achievement,time_delay}=>{
                    let owners=&self.inner.player.as_ref().ok_or_else(||error("achievement player absent"))?.selected.owners;
                    owners.services.skaters.backend().grant_achievement(achievement as usize,time_delay as u32).map_err(error)?;
                }
                Output::OpenHomResults => {
                    let seconds=self.inner.player.as_ref().ok_or_else(||error("HoM player absent"))?
                        .selected.owners.cvars.try_hom_animation_time().map_err(error)?;
                    career.hud.hom_results=Some(crate::challenge::hom_results::view(career.system.world(),seconds));
                    career.hom_results_ready=false;
                    if career.pausing.last()!=Some(&PausingState::HomResults) {career.pausing.push(PausingState::HomResults);}
                }
                Output::OpenPhotoSelection { count } => career.open_photo_selection(count),
                Output::EnterRealTime => career.nis.enter_real_time(),
                Output::OpenUnlockOverlay => {
                    career.hud.unlock_overlay = Some(career.system.unlock_overlay_view());
                    career.pausing.push(PausingState::Unlock);
                    career.unlock_ready = false;
                }
                // ReceiveSkate101HUDShowMsg (0x826285e8) asserts that FE
                // state 79 is not already on the stack. OnEnter (0x8269fd58)
                // sends cMsgPinPlayer(1) and opens screen 135.
                Output::ShowSkate101(view) => {
                    if career.hud.skate101.is_some() {
                        eprintln!("career: FE state 79 already on the stack (fe.cpp:1607)");
                    } else {
                        career.hud.skate101 = Some(view);
                        career.pausing.push(PausingState::Skate101);
                        career.skate101_ready = false;
                        eprintln!("career: cMsgPinPlayer(1) not applied");
                    }
                }
                Output::OpenSponsorOverlay { sponsor } => career.open_sponsor_overlay(sponsor),
                Output::OpenCoverageScreen => {
                    let male = settings.as_ref().and_then(|settings| settings.try_character_is_male(0).ok()).unwrap_or(true);
                    let profile = self.inner.player.as_ref().and_then(|player| player.selected.owners.services.profile.name());
                    career.open_slappys_blog(true, male, profile.unwrap_or_default());
                }
                // cSignPostUtil::HandleCaughtMessage: mMovieKey, then PushState(28).
                Output::PlayMovie { key, .. } => career.movie.push(key),
                Output::ResetTriggerVolumeState { volume } => {
                    #[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation, reason = "the native (float)(double) id")]
                    let matches = |id: u64| id as f64 as f32 == volume;
                    for entries in career.previous.values_mut() {
                        entries.retain(|&(_, guid, local)| !matches(guid) && !matches(local));
                    }
                }
                Output::ScriptError { context, error } => eprintln!("career script error in {context}: {error}"),
                output @ (Output::OnlineChallengeComplete { .. } | Output::OnlineChallengeCleanupDone | Output::OnlineRoundSyncPoint { .. }) => {
                    online_outputs.push(output);
                }
                Output::Ai(request) => ai_requests.push(request),
                // Presentation::ReceiveCrowdBufferChanged (0x827eefa8) runs CrowdPresentation::Update.
                Output::CrowdBufferChanged => crowd_updates += 1,
                Output::Unpresented { message, kind } => eprintln!("career: unpresented {kind} {message}"),
                other => eprintln!("career: {other:?}"),
            }
        }
        if !hulls.is_empty() {
            let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("career terrain absent"))?;
            queue_hull_requests(terrain.as_mut(), hulls)?;
        }
        if !dynamic_hulls.is_empty() {
            let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("career terrain absent"))?;
            queue_dynamic_hull_requests(terrain.as_mut(), dynamic_hulls)?;
        }
        if !sim_rates.is_empty() {
            // GameSimulationBroadcaster: dispatched at the next
            // SimController::Update1 (OnRequestSetSimRate).
            let world = self.inner.world_handle().ok_or_else(|| error("career world absent"))?;
            let world = world.lock().map_err(|_| error("world lock poisoned"))?;
            let mut rate = world.sim_rate.lock().map_err(|_| error("sim rate lock poisoned"))?;
            for message in sim_rates {
                rate.send(message).map_err(error)?;
            }
        }
        for location in end_points {
            self.teleport_to_challenge_end_point(&location)?;
        }
        for (world_location, world_file, location) in world_teleports {
            self.teleport_local_to_world_location(world_location, world_file.as_deref(), &location)?;
        }
        for challenge in start_teleports {
            self.teleport_local_player_start_challenge(challenge)?;
        }
        for _ in 0..crowd_updates {
            if let Err(failure) = self.update_career_crowd_presentation() {
                eprintln!("career: crowd presentation: {failure}");
            }
        }
        for _ in 0..marker_teleports {
            self.teleport_local_player_to_session_marker()?;
        }
        self.send_session_markers(markers)?;
        if online_local.is_some()
            && let Some(player) = requests.iter().rev().find_map(|request| match request {
                SkaterRequest::SetLocalIndex(player) => Some(*player),
                _ => None,
            })
        {
            self.online_switch_camera(player);
        }
        if online_local.is_some() {
            for request in &requests {
                if let &SkaterRequest::TeleportPlayer(player, transform) = request {
                    self.online_note_spot(player, transform);
                }
            }
        }
        let requests = match online_local {
            Some(local) => self.online_skater_requests(requests, local),
            None => requests,
        };
        if let Some(pin) = requests.iter().rev().find_map(|request| match request {
            SkaterRequest::Pin(pin) => Some(*pin),
            _ => None,
        }) && online_local.is_some()
        {
            self.online_local_pinned(pin);
        }
        self.apply_skater_requests(requests)?;
        self.apply_ai_requests(ai_requests)?;
        if autosave && let Err(e) = self.save_career() {
            eprintln!("career: autosave failed: {e}");
        }
        self.online_challenge_outputs(online_outputs)
    }

    /// An online challenge's skater requests: each console carries out those of its own
    /// player, whose skater is the local skater here, and leaves every other player's to that
    /// player's console.
    fn online_skater_requests(&self, requests: Vec<SkaterRequest>, local: i32) -> Vec<SkaterRequest> {
        let actor = self.inner.bound_input.as_ref().and_then(|input| i32::try_from(input.player_index()).ok()).unwrap_or(0);
        requests
            .into_iter()
            .filter_map(|request| match request {
                SkaterRequest::SetLocalIndex(player) => (player == local).then_some(SkaterRequest::SetLocalIndex(actor)),
                SkaterRequest::TeleportPlayer(player, transform) => (player == local).then_some(SkaterRequest::TeleportPlayer(actor, transform)),
                SkaterRequest::PinActor(player, pin) => (player == local).then_some(SkaterRequest::PinActor(actor, pin)),
                SkaterRequest::SetPlayer(_) => None,
                request @ (SkaterRequest::Pin(_) | SkaterRequest::Teleport(_)) => Some(request),
            })
            .collect()
    }

    /// The career's front-end input: the several-challenges popup and the
    /// poster take A/B (and the popup up/down); otherwise translated D-pad
    /// presses/releases go to the visible `SignUpHUD::UpdateInput`.
    pub(super) fn career_input(&mut self, input: crate::ui::pause_menu::PauseMenuInput, hud_dpad: [bool; 5]) {
        let controller = self.inner.bound_input.as_ref().map(|bound| bound.controller());
        let settings = self.inner.player.as_ref().map(|player| Arc::clone(&player.selected.owners.services.settings));
        let Some(career) = self.career.as_mut() else { return };
        if let Some(player_turn) = career.hud.press_a.as_ref().map(|dialog| dialog.player_turn) {
            // FrontEndState_PressA::ReceiveInput (0x826596b0): A from the controller of
            // player mPlayerTurn - 1 sends cMsgHUDComplete and pops the state.
            let turn = u32::try_from(player_turn - 1).ok();
            let turn_controller = turn.zip(settings).and_then(|(player, settings)| settings.try_controller_index(player).ok());
            let from_turn = turn_controller.and_then(|c| usize::try_from(c).ok()) == controller;
            if input.select && from_turn {
                career.hud.press_a = None;
                career.system.post(Event::HudComplete);
            }
            return;
        }
        if career.popup_input(input) {
            return;
        }
        if career.overlay_open() {
            career.overlay_input(input);
            return;
        }
        if let Some((options, selected)) = career.hud.sign_up_choice.as_mut() {
            // sk8popup's UpdateInput: B on this cancellable popup is Game.OnMenuSelect(0, -1)
            // without a sound; every other key goes to its menu_picker, which wraps, sounds a
            // move that changes the value and sounds A before Game.OnMenuSelect.
            let count = options.len();
            if input.up && count > 1 {
                *selected = (*selected + count - 1) % count;
                career.sounds.push("FE_SOUND_CORE_NAV_UP");
            }
            if input.down && count > 1 {
                *selected = (*selected + 1) % count;
                career.sounds.push("FE_SOUND_CORE_NAV_DOWN");
            }
            let chosen = *selected;
            // MultipleSignupsCallback (0x826b5430) pulls kPAUSE_STATE_FRONTEND either way.
            if input.select {
                career.sounds.push("FE_SOUND_CORE_A_BUTTON");
                career.hud.sign_up_choice = None;
                career.sign_up_choice_paused = false;
                career.system.sign_up_choose(Some(chosen));
            } else if input.cancel {
                career.hud.sign_up_choice = None;
                career.sign_up_choice_paused = false;
                career.system.sign_up_choose(None);
            }
            return;
        }
        if career.hud.poster.is_some() {
            // StateManagerFrontEnd::ReceiveInput (0x82656af8) gives the poster input while its
            // mLocalInputState allows it.
            if career.poster_input_blocked {
                return;
            }
            // FrontEndState_SignUpPoster::ReceiveInput (0x82657bf8) takes
            // A and B once OnIntroDone unblocked the actions; both pop the
            // state (PopState).
            if !career.poster_blocked {
                if input.select {
                    match career.system.sign_up_accept() {
                        PosterAccept::Accepted => {
                            career.poster_blocked = true;
                            career.hud.poster = None;
                        }
                        // The accept blocked the state's input, then core_error and
                        // OpenPopup(eSk8FEPopup_ChallengeInfo_FeeFailed).
                        PosterAccept::FeeFailed => {
                            career.poster_input_blocked = true;
                            career.sounds.push("FE_SOUND_CORE_ERROR");
                            career.dialog_owner = DialogOwner::FeeFailed;
                            career.hud.dialog_options = Some(vec!["ID_COMMON_OK".to_owned()]);
                            career.hud.dialog =
                                Some(("ID_COMMON_WARNING_TITLE".to_owned(), "ID_CHALLENGEINFO_FEE_FAILED".to_owned(), 0));
                            career.sounds.push("FE_SOUND_CORE_POPUP");
                        }
                        PosterAccept::NotEnoughForMinBet => career.sounds.push("FE_SOUND_CORE_ERROR"),
                        PosterAccept::Unported => {}
                    }
                } else if input.cancel {
                    career.system.sign_up_decline();
                    career.poster_blocked = true;
                    career.hud.poster = None;
                } else if input.misc {
                    career.poster_tutorial_video();
                }
            }
            // gFEStateInfo[32].mbInputToActionscript: the keys then reach the screen's UpdateInput.
            Self::poster_betting_input(career, input);
            return;
        }
        if hud_dpad[4] {
            career.system.sign_up_dpad_released();
        }
        if career.system.world().signup_hud.prop_bound {
            if career.system.world().signup_hud.visible {self.queue_prop_input(hud_dpad);}
            return;
        }
        if hud_dpad[0] {
            career.sign_up_choice_paused |= career.system.sign_up_dpad_up();
        }
    }

    /// The several-challenges popup's pause. `SignUpHUD::UpdateDPadUp` (`0x826bd450`) opens it
    /// in the front end's update and pushes `kPAUSE_STATE_FRONTEND` (`0x826bd738`), so the popup
    /// is up and no simulation update runs from that frame; `MultipleSignupsCallback`
    /// (`0x826b5430`) pulls it as the popup answers, before that frame's update. True while the
    /// popup holds the pause.
    pub(super) fn try_update_sign_up_choice_pause(&mut self) -> Result<bool, SelectedLiveError> {
        let Some(paused) = self.career.as_mut().and_then(CareerSession::take_sign_up_choice_pause_change) else {
            return Ok(self.career.as_ref().is_some_and(|career| career.sign_up_choice_paused));
        };
        self.inner.bound_input.as_mut().ok_or_else(|| error("career input absent"))?.try_set_frontend_pause(paused).map_err(error)?;
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            eprintln!("career: several-challenges popup {} kPAUSE_STATE_FRONTEND", if paused { "pushes" } else { "pulls" });
        }
        if paused {
            self.apply_career_outputs()?;
            self.collect_career_hud();
        }
        Ok(paused)
    }

    /// challenge_info's `UpdateInput` (`0x3960`) in the sign-up mode with the betting panel shown:
    /// right or left (`AptRight`/`AptLeft`, repeat pulses) after a release bets once and waits
    /// `mBettingRepsBeforeAutoSet` (5) pulses, then bets on every pulse; `AptRightReleased` or
    /// `AptLeftReleased` starts over. Each bet (`BetMore`/`BetLess` `0x3a7f`/`0x3ad9`) refreshes the
    /// panel.
    fn poster_betting_input(career: &mut CareerSession, input: crate::ui::pause_menu::PauseMenuInput) {
        if !career.hud.poster.as_ref().is_some_and(|view| view.betting) {
            return;
        }
        for (pressed, more) in [(input.left, false), (input.right, true)] {
            if !pressed {
                continue;
            }
            let (released, timer) = &mut career.poster_bet_input;
            if *released {
                *timer = 5;
                *released = false;
            } else if *timer > 0 {
                *timer -= 1;
                continue;
            }
            let sound = if more { career.system.sign_up_bet_more() } else { career.system.sign_up_bet_less() };
            career.sounds.push(sound);
            if let Some(view) = career.hud.poster.as_mut() {
                view.bet = career.system.sign_up_bet_view();
            }
        }
        if input.left_released || input.right_released {
            career.poster_bet_input = (true, 0);
        }
    }

    /// `FrontEndState_SignUpPoster::OnIntroDone` (`0x826581a0`), when the
    /// poster screen's intro reports `IntroComplete`: `cMsgHUDIntroDone` to
    /// the challenge system and the actions unblocked.
    pub fn acknowledge_career_poster_intro(&mut self) {
        let Some(career) = self.career.as_mut() else { return };
        if career.hud.poster.is_some() && career.poster_blocked {
            career.system.post(Event::HudIntroDone);
            career.poster_blocked = false;
        }
    }

    /// `FrontEndState_Unlock::OnScreenReady` when the overlay's intro reports
    /// `IntroComplete`: the screen ready, then `OnIntroDone` (`0x826423c0`)
    /// clears `mCashAnimDone` (and the unported `mAudioStopped`).
    pub fn acknowledge_career_unlock_intro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && career.hud.unlock_overlay.is_some()
        {
            career.unlock_ready = true;
            career.cash_anim_done = false;
        }
    }

    /// `FrontEndState_Skate101`'s `OnScreenReady` when its screen's intro
    /// reports `IntroComplete`; the state has no `OnIntroDone` of its own.
    pub fn acknowledge_career_skate101_intro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && career.hud.skate101.is_some()
        {
            career.skate101_ready = true;
        }
    }

    /// `FrontEndState_EndChallengeOptions`'s `OnScreenReady` when its screen's intro reports
    /// `IntroComplete`, then `OnIntroDone` (`0x82658fe8`): the final results' success or
    /// failure sound, once.
    pub fn acknowledge_career_end_options_intro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && let Some(view) = career.hud.end_options.as_ref()
            && !career.end_options_ready
        {
            career.end_options_ready = true;
            let sound = if career.system.world().results.current_result_type == results::result_group::ROUND_RESULTS {
                results::round_intro_sound(view.options.win)
            } else {
                results::intro_sound(view.options.win)
            };
            career.sounds.push(sound);
        }
    }

    /// FE state 39's `OnScreenReady` when its screen's intro reports `IntroComplete`.
    pub fn acknowledge_career_change_game_intro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && career.change_game.is_some()
        {
            career.change_game_ready = true;
        }
    }

    /// `FrontEndState_EndChallengeOptions::OnOutroDone` (`0x826590c8`) when its screen's
    /// outro reports `OutroComplete`: after Continue or Quit, `cMsgHUDContinue`.
    pub fn acknowledge_career_end_options_outro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && std::mem::take(&mut career.end_options_wait_outro)
        {
            career.system.post(Event::HudContinue);
        }
    }

    /// The career front-end states' sounds and the turn HUD's since the last call.
    pub(super) fn take_career_sounds(&mut self) -> Vec<&'static str> {
        let Some(career) = self.career.as_mut() else { return Vec::new() };
        let mut sounds = std::mem::take(&mut career.sounds);
        sounds.append(&mut career.system.world_mut().turn_huds.sounds);
        sounds
    }

    /// FE state 8's quit was confirmed since the last call.
    pub(super) fn take_career_quit_to_front_end(&mut self) -> bool {
        self.career.as_mut().is_some_and(|career| std::mem::take(&mut career.quit_to_front_end))
    }

    /// `FEUnlock.Cash_SetCashAnimDone` (`0x82624038`): the overlay's cash
    /// count is done, so its A is taken.
    pub fn acknowledge_career_cash_anim_done(&mut self) {
        if let Some(career) = self.career.as_mut() {
            career.cash_anim_done = true;
        }
    }

    /// The ICE camera a NIS shows and the stream focus it froze.
    pub(super) fn career_nis_camera(&self) -> (Option<NisCamera>, Option<FrozenFocus>) {
        self.career.as_ref().map_or((None, None), |career| (career.hud.nis.camera, career.hud.nis.frozen_focus))
    }

    /// A NIS holds the simulation paused (`kPAUSE_STATE_NIS`).
    #[must_use]
    pub fn career_nis_paused(&self) -> bool {
        self.career.as_ref().is_some_and(|c| c.nis.is_sim_paused())
    }

    /// The career holds the simulation paused: a NIS (`kPAUSE_STATE_NIS`) or the challenge's
    /// `Pause.PauseSimulation` (`kPAUSE_STATE_CHALLENGE`, `cChallengePauseManager::
    /// PauseSimulation` `0x825b12f8`), which a film's win holds from its highlight reel until
    /// the end-of-challenge options' Continue or Retry (`Pause.UnpauseSimulation`).
    #[must_use]
    pub fn career_simulation_paused(&self) -> bool {
        self.career.as_ref().is_some_and(|c| c.nis.is_sim_paused() || c.challenge_paused)
    }

    /// Diagnostics: the local player teleported on board to `position`, unrotated.
    pub fn diagnostic_teleport_local_player(&mut self, position: [f32; 3]) -> bool {
        let mut transform = [0.0; 16];
        for axis in 0..4 {
            transform[axis * 5] = 1.0;
        }
        transform[12..15].copy_from_slice(&position);
        self.teleport_local_player(transform).is_ok()
    }

    /// Diagnostics: visit an authored location without bypassing global activity/trigger updates.
    pub fn diagnostic_teleport_named_location(&mut self, name: &str) -> bool {
        let Some(career) = self.career.as_ref() else { return false };
        let world = career.system.world();
        let key = hash_value32(name);
        let Some(transform) = world.host.location_orientation(key).or_else(|| world.locations.get(&key).copied()) else { return false };
        self.teleport_local_player(transform).is_ok()
    }

    /// Diagnostics: while a race is tracked, the centre of the local player's next split-time
    /// gate (`cChallengeRaceManager`'s leg gates after the progress `Race.SetPlayerProgress` set).
    #[must_use]
    pub fn diagnostic_race_next_gate(&self) -> Option<[f32; 3]> {
        let world = self.career.as_ref()?.system.world();
        let race = &world.race;
        let player = race.players.iter().find(|player| player.racer_id == world.local_player).filter(|_| race.tracking)?;
        let gate = race.legs.get(usize::try_from(player.current_leg).ok()?)?.gates.get(usize::try_from(player.current_gate).ok()? + 1)?;
        Some([gate[0], gate[1], gate[2]])
    }

    /// Diagnostics: cash added to the career. False without one.
    pub fn diagnostic_add_cash(&mut self, cash: i32) -> bool {
        self.career.as_mut().map(|career| career.system.diagnostic_add_cash(cash)).is_some()
    }

    /// Diagnostics: the progression's `ActivateChallengeByTitle(title)`. False without a career
    /// or when the call fails.
    pub fn diagnostic_activate_challenge(&mut self, title: &str) -> bool {
        self.career.as_mut().is_some_and(|career| career.system.diagnostic_activate_challenge(title).is_ok())
    }

    /// Diagnostics: the map's travel to a point on the board, facing `yaw` radians about +y from
    /// +z: the fade, the destination's streams, `StateLoading::DynamicHullState`, then the
    /// teleport.
    pub fn diagnostic_travel(&mut self, position: [f32; 3], yaw: f32) {
        let (sin, cos) = yaw.sin_cos();
        let mut transform = [0.0; 16];
        transform[0] = cos;
        transform[2] = -sin;
        transform[5] = 1.0;
        transform[8] = sin;
        transform[10] = cos;
        transform[12..15].copy_from_slice(&position);
        transform[15] = 1.0;
        self.frontend_fade.cover();
        self.map_travel = Some(transform);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = true;
    }

    /// Diagnostics: an event on the career challenge system's queue, as the game's own
    /// messages arrive there.
    pub fn diagnostic_post_challenge_event(&mut self, event: Event) {
        if let Some(career) = self.career.as_mut() {
            career.system.post(event);
        }
    }

    /// `cSaveChallengePhotoCallback::OnComplete` 0x8266c180 and OnFail 0x8266c1d8
    /// both finish the photo state. A stale renderer completion cannot close another pick.
    pub fn acknowledge_career_photo_save(&mut self, slot: u32) {
        if let Some(career) = self.career.as_mut()
            && career.hud.photo_save_slot == Some(slot)
        {
            career.photo_done();
        }
    }

    /// `FrontEndState_PhotoChallenge` is up over a photo outro.
    #[must_use]
    pub fn career_photo_select_open(&self) -> bool {
        self.career.as_ref().is_some_and(CareerSession::photo_select_open)
    }

    /// The progression or confirmation popup is open.
    #[must_use]
    pub fn career_dialog_open(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.hud.dialog.is_some())
    }

    pub fn contacts_intro_complete(&mut self) {
        if let Some(contacts)=self.career.as_mut().and_then(|c|c.contacts.as_mut()) {contacts.intro_complete=true;}
    }
    /// A pausing career front-end state (the unlock overlay, Skate 101) holds
    /// the game paused; only the front end runs.
    #[must_use]
    pub fn career_overlay_open(&self) -> bool {
        self.career.as_ref().is_some_and(CareerSession::overlay_open)
    }

    /// The several-challenges popup holds `kPAUSE_STATE_FRONTEND`, from `SignUpHUD::UpdateDPadUp`
    /// (`0x826bd450`) until `SignUpHUD::MultipleSignupsCallback` (`0x826b5430`).
    #[must_use]
    pub fn career_sign_up_choice_paused(&self) -> bool {
        self.career.as_ref().is_some_and(|career| career.sign_up_choice_pushed)
    }

    /// The local skater's `BreadCrumbManager::GetSuggestedRespawnPosition`
    /// (`0x82ce2888`) as the four rows `SaveProgressionOrientation` writes;
    /// none without the Actor or its breadcrumbs.
    fn local_suggested_respawn(&self) -> Result<Option<[[f32; 3]; 4]>, SelectedLiveError> {
        Ok(self.local_suggested_breadcrumb()?.map(|crumb| crumb.transform.columns().map(|row| row.xyz)))
    }

    /// `cTeleportManager::TeleportRequiresLoadScreen` (`0x828a9420`) without a large load radius:
    /// `GameBackEnd::RequiresAdditionalWorldOperation` (`0x8255b898`, an additional world location
    /// to load or unload, which the port never has), `sGameInfo.startChallengeDuringLoad`, or
    /// `cGameAssetManager::IsAreaLoaded` (`0x824e0768`) failing for the simulation focus within
    /// `kMinSimRadiusForNoLoadScreenOnTelelport` (flags 2) or for the presentation focus within
    /// `kMinPresRadiusForNoLoadScreenOnTelelport` (flags 1; loaded while there is none).
    fn teleport_requires_load_screen(&self, position: [f32; 3]) -> Result<bool, SelectedLiveError> {
        let cvars = &self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?.cvars;
        let sim_radius = cvars.try_min_sim_radius_for_no_load_screen_on_teleport().map_err(|e| error(format!("{e:?}")))?;
        let pres_radius = cvars.try_min_pres_radius_for_no_load_screen_on_teleport().map_err(|e| error(format!("{e:?}")))?;
        let xml = cvars.try_enable_stream_xml().map_err(|e| error(format!("{e:?}")))?;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("career terrain absent"))?;
        let sim = terrain.try_area_loaded(position, sim_radius).map_err(|e| error(format!("{e:?}")))?;
        let pres = self.presentation_area.as_ref().is_none_or(|area| area.is_area_loaded(position, pres_radius, xml));
        let during_load = self.career.as_ref().is_some_and(|career| career.system.world().start_challenge_during_load);
        Ok(!sim || !pres || during_load)
    }

    /// `NISManager::TeleportToLastBreadCrumb` (`0x828a0db8`) for a story or go-to scene not told
    /// to skip it: `TeleportLocalToLastBreadcrumbRequiresLoadScreen` (`0x828a9350` →
    /// `0x828a9800`) tests the local skater's `GetSuggestedRespawnPosition` with
    /// `TeleportRequiresLoadScreen`, and without a load screen `TeleportLocalToLastBreadcrumb`
    /// (`0x828a93a8` → `TeleportToLastBreadcrumb` `0x828a9158`) has `TeleportPlayer` (`0x828a9888`)
    /// send `cMsgTeleport` with the same crumb, on the board unless the crumb is off it (the local
    /// skater has no AI controller). `RunBasicNIS` pauses the simulation right after, so the
    /// PlayerUI's teleport command runs as the NIS ends.
    pub(super) fn try_nis_breadcrumb_teleport(&mut self) -> Result<(), SelectedLiveError> {
        if !self.career.as_mut().is_some_and(|career| std::mem::take(&mut career.breadcrumb_teleport)) {
            return Ok(());
        }
        let Some(crumb) = self.local_suggested_breadcrumb()? else { return Ok(()) };
        let position = crumb.transform.columns()[3].xyz;
        let load_screen = self.teleport_requires_load_screen(position)?;
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            let facing = crumb.transform.columns()[2].xyz;
            let hips = self
                .inner
                .player
                .as_ref()
                .and_then(|player| player.selected.actor.as_ref())
                .and_then(|actor| actor.retained_breadcrumbs().ok().flatten())
                .and_then(|crumbs| crumbs.lock().ok().map(|crumbs| crumbs.last_known_actor_position.xyz));
            let ground = hips.map(|hips| self.wipeout_ground_line(hips)).unwrap_or_default();
            eprintln!("career: NIS breadcrumb {position:?} facing {facing:?} on board {} (hips {hips:?}; {ground}){}", !crumb.offboard,
                if load_screen { ", needs a load screen: not teleported" } else { "" });
        }
        if load_screen {
            return Ok(());
        }
        self.player_ui_queues()?.send_teleport(crumb.transform, !crumb.offboard);
        Ok(())
    }

    /// For the NIS breadcrumb trace: the wipeout spot's ground line in
    /// `_TestForCollisionConstraints` (`0x82ce19b0`), 10 m down from 0.1 m above the hips.
    fn wipeout_ground_line(&self, hips: [f32; 3]) -> String {
        use crate::skeleton_root_transforms::SkeletonRootVector;
        let line = || -> Result<String, String> {
            let world = &self.inner.world.as_ref().ok_or("career world absent")?.world;
            let terrain = self.inner.terrain.as_ref().ok_or("career terrain absent")?;
            let ground = terrain.ground_pool_for(world).map_err(|e| format!("{e:?}"))?;
            let mut physical = world.lock().map_err(|_| "world lock poisoned")?;
            let start = SkeletonRootVector::known([hips[0], hips[1], hips[2], 1.0])
                .zip(SkeletonRootVector::known([0.0, 0.1, 0.0, 0.0]), |a, b| a + b);
            let end = start.zip(SkeletonRootVector::known([0.0, -10.0, 0.0, 0.0]), |a, b| a + b);
            let input = crate::collision_line_collection::CollisionLineInput { start, end, fatness: 0.0 };
            let mut numeric = crate::collision_line_numeric::HostCollisionLineNumeric;
            let hit = physical
                .try_world_line_test(ground, input, "BreadCrumbManager::_TestForCollisionConstraints", &mut numeric)
                .map_err(|e| format!("{e:?}"))?;
            Ok(match hit {
                Some(hit) => format!(
                    "ground line from y {} hits y {} (drop {}, normal y {}, surface {})",
                    start.xyz[1],
                    hit.position.xyz[1],
                    start.xyz[1] - hit.position.xyz[1],
                    hit.normal.xyz[1],
                    (hit.surface_id >> 7) & 0x1f
                ),
                None => format!("ground line from y {} hits nothing", start.xyz[1]),
            })
        };
        line().unwrap_or_else(|e| format!("ground line failed: {e}"))
    }

    /// The local skater's `BreadCrumbManager::GetSuggestedRespawnPosition` (`0x82ce2888`), none
    /// without the Actor or its breadcrumbs.
    pub(super) fn local_suggested_breadcrumb(
        &self,
    ) -> Result<Option<crate::actor_construction::breadcrumbs::SkaterBreadCrumb>, SelectedLiveError> {
        use crate::actor_construction::breadcrumbs::LiveBreadCrumbSelectionSource;
        let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref()) else {
            return Ok(None);
        };
        let Some(breadcrumbs) = actor.retained_breadcrumbs().map_err(|e| error(format!("{e:?}")))? else {
            return Ok(None);
        };
        let world = &self.inner.world.as_ref().ok_or_else(|| error("career world absent"))?.world;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("career terrain absent"))?;
        let ground = terrain.ground_pool_for(world).map_err(|e| error(format!("{e:?}")))?;
        let cvars = &self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?.cvars;
        let mut physical = world.lock().map_err(|_| error("world lock poisoned"))?;
        let manager = breadcrumbs.lock().map_err(|_| error("breadcrumb lock poisoned"))?;
        let mut numeric = crate::collision_line_numeric::HostCollisionLineNumeric;
        let mut source = LiveBreadCrumbSelectionSource::new(&manager, actor, world, &mut physical, ground, cvars, &mut numeric);
        let crumb = manager.try_suggested_respawn(&mut source).map_err(|e| error(format!("{e:?}")))?;
        Ok(Some(crumb))
    }

    /// `cChallengeTeleportManager::TeleportPlayer`: the session marker's
    /// teleport flow to a location transform.
    /// The skater requests in the order the system made them: eEQGameSim messages to the
    /// PlayerUI (`cMsgSetPlayer`, `cMsgPinPlayer`, `cMsgTeleport`), the Actor flag of an
    /// AI-driven skater and `SetLocalIndex`.
    fn apply_skater_requests(&mut self, requests: Vec<SkaterRequest>) -> Result<(), SelectedLiveError> {
        for request in requests {
            match request {
                SkaterRequest::SetLocalIndex(player) => self.set_skater_local_index(player)?,
                SkaterRequest::TeleportPlayer(player, transform) => {
                    // TeleportPlayer828a9888 tests Actor::mAIController, not
                    // IsLocalPlayer. The human still receives cMsgTeleport
                    // when SetLocalIndex currently follows an AI competitor.
                    if !self.inner.try_teleport_ai_player(player, transform).map_err(error)?
                        && self.inner.bound_input.as_ref().is_some_and(|input| u32::try_from(player).ok() == Some(input.player_index()))
                    {
                        self.teleport_local_player(transform)?;
                    }
                }
                SkaterRequest::PinActor(player, pin) => {
                    let actor = std::iter::once(self.inner.player.as_ref().and_then(|p| p.selected.actor.as_ref()))
                        .flatten()
                        .chain(self.inner.party.iter().map(|skater| &skater.actor))
                        .chain(self.inner.ai_skaters.iter().map(|skater| &skater.actor))
                        .find(|actor| actor.prefix().ok().and_then(|p| p.fields().map(|f| f.player_index)) == u32::try_from(player).ok())
                        .map(Arc::clone);
                    if let Some(actor) = actor {
                        actor.try_set_pinned(pin).map_err(error)?;
                    }
                }
                SkaterRequest::SetPlayer(player) => self.player_ui_queues()?.send_set_player(player),
                SkaterRequest::Pin(pin) => self.player_ui_queues()?.send_pin(pin),
                SkaterRequest::Teleport(transform) => self
                    .player_ui_queues()?
                    .send_teleport(crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform), true),
            }
        }
        Ok(())
    }

    /// The path packages the stream reader has read, oldest first: `DoAddAsset` (`0x824d4458`),
    /// or `AssetMissing` (`0x824b1dc0`) for one that never arrives.
    fn deliver_path_packages(&mut self) -> Result<(), SelectedLiveError> {
        let Some(ai) = self.inner.ai.as_mut() else { return Ok(()) };
        while let Some(load) = ai.path_packages.first() {
            let Some(bytes) = load.bytes.get() else { break };
            let mut paths = ai.paths.lock().map_err(|_| error("AI path manager lock poisoned"))?;
            match bytes {
                Ok(bytes) => paths.try_add_binary_path_package(&load.file, bytes).map_err(error)?,
                Err(missing) => {
                    eprintln!("ai: static path package {} missing: {missing}", load.file);
                    paths.binary_path_package_missing();
                }
            }
            drop(paths);
            ai.path_packages.remove(0);
        }
        Ok(())
    }

    /// `ChallengeAIManager`'s requests of the AI skaters, in the order it made them.
    fn apply_ai_requests(&mut self, requests: Vec<crate::challenge::ai::AiRequest>) -> Result<(), SelectedLiveError> {
        use crate::{
            ai::path_controller::{BranchMode, GestureType, InfluenceFilter, TrickSwapMode},
            challenge::ai::AiRequest,
        };
        self.deliver_path_packages()?;
        for request in requests {
            if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                eprintln!("ai: challenge {request:?}");
            }
            match request {
                AiRequest::StartPath { player, path_id, branch, challenge_filter, dynamic_trick_selection } => {
                    let shared = self.career.as_ref().map(|career| career.system.world().ai_manager.multi_filter.clone());
                    let Some(ai) = self.inner.ai.as_mut() else { continue };
                    let universal = ai.paths.lock().map_err(|_| error("AI path manager lock poisoned"))?.universal_filter_id();
                    let Some(controller) = ai.controllers.get_mut(&player) else { continue };
                    controller.set_follow_path_behavior(universal);
                    controller.set_path_id(path_id);
                    controller.set_branch_mode(if branch { BranchMode::BranchRandom } else { BranchMode::DoNotBranch });
                    if challenge_filter {
                        if let Some(filter) = shared {
                            controller.set_filter(InfluenceFilter::Shared(filter));
                        }
                        controller.set_trick_swap_mode(if dynamic_trick_selection { TrickSwapMode::RandomSwap } else { TrickSwapMode::DoNotSwap });
                    }
                }
                AiRequest::FollowTrickPattern { player, pattern, path_id, filter } => {
                    let Some(ai) = self.inner.ai.as_mut() else { continue };
                    let universal = ai.paths.lock().map_err(|_| error("AI path manager lock poisoned"))?.universal_filter_id();
                    let Some(controller) = ai.controllers.get_mut(&player) else { continue };
                    controller.set_follow_path_behavior(universal);
                    if let Some(id) = path_id {
                        controller.set_trick_pattern(pattern);
                        controller.set_branch_mode(BranchMode::DoNotBranch);
                        controller.set_trick_swap_mode(TrickSwapMode::FollowTrickPattern);
                        controller.set_path_id(id);
                    } else {
                        controller.set_path_id(filter);
                    }
                }
                AiRequest::ClearBehavior { player } => {
                    if let Some(controller) = self.inner.ai.as_mut().and_then(|ai| ai.controllers.get_mut(&player)) {
                        controller.clear_behavior();
                        controller.external_reset();
                    }
                }
                AiRequest::SetChallengeRound { player, round } => {
                    if let Some(controller) = self.inner.ai.as_mut().and_then(|ai| ai.controllers.get_mut(&player)) {
                        controller.set_challenge_round(round);
                    }
                }
                AiRequest::Gesture { player, positive } => {
                    if let Some(controller) = self.inner.ai.as_mut().and_then(|ai| ai.controllers.get_mut(&player)) {
                        controller.trigger_gesture(if positive { GestureType::Positive } else { GestureType::Negative });
                    }
                }
                AiRequest::SwitchOffFollowMeRubberBanding { player } => {
                    if let Some(controller) = self.inner.ai.as_mut().and_then(|ai| ai.controllers.get_mut(&player)) {
                        controller.switch_off_follow_me_rubber_banding();
                    }
                }
                AiRequest::ExitAiBasedChallenge => self.inner.try_exit_ai_based_challenge().map_err(error)?,
                AiRequest::LoadPathPackage { file } => {
                    let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("AI paths: assets absent"))?.mounts);
                    let Some(ai) = self.inner.ai.as_mut() else { continue };
                    let bytes = Arc::new(std::sync::OnceLock::new());
                    let (read, path) = (Arc::clone(&bytes), file.clone());
                    let load = move || {
                        let _ = read.set(mounts.read_path(&path).map_err(|missing| format!("{missing:?}")));
                    };
                    if crate::diagnostics::traced_run() {
                        load();
                    } else {
                        crate::stream_reads::queue(load);
                    }
                    ai.path_packages.push(crate::selected_session_boot_owner::ai_skaters::PathPackageLoad { file, bytes });
                    self.deliver_path_packages()?;
                }
            }
        }
        Ok(())
    }

    /// `cChallengeTeleportManager`'s `cMsgSetSessionMarker`s, in order: `TeleportPlayerNow`
    /// (`0x82da6a00`) sends the location's spawn transform with the local Actor's
    /// `mLastSetFootForward`, `SetSessionMarker` (`0x82da6e40`) the Actor's
    /// `CalcRespawnTransform` with `GetFootForwardWithFakie`. Neither is sent without the Actor.
    fn send_session_markers(&mut self, markers: Vec<MarkerMessage>) -> Result<(), SelectedLiveError> {
        use crate::{
            actor_construction::breadcrumbs::calc_respawn_transform::try_calc_respawn_transform,
            reckoning_numeric::HostReckoningNumeric, skeleton_root_transforms::SkeletonRootMatrix,
        };
        for marker in markers {
            let MarkerMessage::Set(marker) = marker else {
                self.player_ui_queues()?.send_clear_session_marker();
                continue;
            };
            let Some(actor) = self.inner.player.as_ref().and_then(|player| player.selected.actor.clone()) else {
                return Ok(());
            };
            let (transform, foot_forward) = match marker {
                Some(transform) => {
                    (SkeletonRootMatrix::known(transform), actor.try_last_set_foot_forward().map_err(|e| error(format!("{e:?}")))?)
                }
                None => {
                    let cvars = &self.inner.assets.as_ref().ok_or_else(|| error("career assets absent"))?.cvars;
                    let physical = actor.cached_physical_allocation().map_err(|e| error(format!("{e:?}")))?;
                    let output = physical.owner().phys_out().lock().map_err(|_| error("PhysOut lock poisoned"))?;
                    let transform = try_calc_respawn_transform(&output, cvars, &mut HostReckoningNumeric)
                        .map_err(|e| error(format!("{e:?}")))?;
                    drop(output);
                    let right = actor.try_current_foot_forward_with_fakie().map_err(|e| error(format!("{e:?}")))?;
                    (transform, u32::from(!right))
                }
            };
            self.player_ui_queues()?.send_set_session_marker(transform, foot_forward);
        }
        Ok(())
    }

    fn player_ui_queues(&mut self) -> Result<&mut crate::session_marker::teleport::SessionMarkerTeleportQueues, SelectedLiveError> {
        Ok(&mut self.inner.session_marker.as_mut().ok_or_else(|| error("session marker absent"))?.teleport)
    }

    /// Diagnostics: `SKATE2_DIAGNOSTIC_RACE_GATES=<ticks>`: every so many completed ticks the
    /// local skater goes on its board to the next gate of its race leg (the next leg's first
    /// gate past the last, then the race's end), so a race runs to its finish.
    pub(super) fn diagnostic_race_gates(&mut self, player: i32) -> Result<(), SelectedLiveError> {
        let Some(interval) = std::env::var("SKATE2_DIAGNOSTIC_RACE_GATES").ok().and_then(|ticks| ticks.parse::<u32>().ok()).filter(|&ticks| ticks > 0) else {
            return Ok(());
        };
        if self.completed_frames % interval != 0 {
            return Ok(());
        }
        let Some(career) = self.career.as_ref() else { return Ok(()) };
        let race = &career.system.world().race;
        if !race.tracking {
            return Ok(());
        }
        let Some(info) = race.players.iter().find(|info| info.racer_id == player && !info.finished) else { return Ok(()) };
        let (leg, gate) = (info.current_leg as usize, info.current_gate as usize);
        let target = race
            .legs
            .get(leg)
            .and_then(|current| current.gates.get(gate + 1))
            .or_else(|| race.legs.get(leg + 1).and_then(|next| next.gates.get(1)))
            .copied()
            .unwrap_or(race.end_point);
        let mut transform = [0.0; 16];
        for axis in 0..4 {
            transform[axis * 5] = 1.0;
        }
        transform[12..15].copy_from_slice(&target[..3]);
        eprintln!("online: diagnostic race gate leg {leg} gate {gate} -> {:?}", &target[..3]);
        self.teleport_local_player(transform)
    }

    /// The `cMsgTeleport` an online round's sync point ends with: the local skater to the
    /// transform, off its board (`fIsOnBoard` 0), once the stream focus is there (the flow
    /// waited for the streamer at the spawn point before it reported the sync point).
    pub(super) fn online_sync_teleport(&mut self, transform: [f32; 16]) -> Result<(), SelectedLiveError> {
        let input = self.inner.bound_input.as_ref().ok_or_else(|| error("career input absent"))?;
        let player = &self.inner.player.as_ref().ok_or_else(|| error("career player absent"))?.selected;
        let regular = player.owners.services.settings.try_stance_byte(input.player_index()).map_err(error)?;
        let marker = self.inner.session_marker.as_mut().ok_or_else(|| error("career session marker absent"))?;
        marker.teleport.send_flow(crate::session_marker::player_ui::TeleportToPosition {
            transform: crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform),
            foot_forward: u32::from(regular != 0),
            on_board: false,
            session_marker: false,
        });
        Ok(())
    }

    /// `cTeleportManager::TeleportLocalPlayerToLastSessionMarker` (`0x828a9b18`): the game module's
    /// `SessionMarker::GetSessionMarker` (`0x828adc38`) as a session-marker
    /// `StateMessage_TeleportToPosition` without the large load radius. The native ignores the
    /// getter's result; without a valid marker its message carries an unset transform, which is
    /// not sent here.
    fn teleport_local_player_to_session_marker(&mut self) -> Result<(), SelectedLiveError> {
        let owner = self.inner.session_marker.as_mut().ok_or_else(|| error("career session marker absent"))?;
        if let Some((transform, foot_forward, on_board)) = owner.marker.as_ref().and_then(|marker| marker.get()) {
            owner.teleport.send_flow(crate::session_marker::player_ui::TeleportToPosition {
                transform,
                foot_forward,
                on_board,
                session_marker: true,
            });
        }
        Ok(())
    }

    fn teleport_local_player(&mut self, transform: [f32; 16]) -> Result<(), SelectedLiveError> {
        let input = self.inner.bound_input.as_ref().ok_or_else(|| error("career input absent"))?;
        let player = &self.inner.player.as_ref().ok_or_else(|| error("career player absent"))?.selected;
        let regular = player.owners.services.settings.try_stance_byte(input.player_index()).map_err(error)?;
        let marker = self.inner.session_marker.as_mut().ok_or_else(|| error("career session marker absent"))?;
        marker.teleport.send_flow(crate::session_marker::player_ui::TeleportToPosition {
            transform: crate::skeleton_root_transforms::SkeletonRootMatrix::known(transform),
            foot_forward: u32::from(regular != 0),
            on_board: true,
            session_marker: false,
        });
        Ok(())
    }
}

impl CareerSession {
    /// hom_results.UpdateInput10884 and OnMenuSelect826591e8.
    fn hom_results_input(&mut self,input:crate::ui::pause_menu::PauseMenuInput) {
        if !self.hom_results_ready {return;}
        let Some(view)=self.hud.hom_results.as_mut() else{return;};
        if !view.points_page {if input.select {view.points_page=true;}return;}
        if input.up {view.selection=(view.selection+2)%3;self.sounds.push("FE_SOUND_CORE_NAV_UP");}
        else if input.down {view.selection=(view.selection+1)%3;self.sounds.push("FE_SOUND_CORE_NAV_DOWN");}
        if input.select {
            self.sounds.push("FE_SOUND_CORE_A_BUTTON");
            if view.selection==0 {
                self.hud.hom_results=None;self.pausing.pop();self.system.post(Event::HudContinue);
                crate::challenge::hom_results::continue_results(self.system.world_mut());
            } else if view.selection==1 {self.open_hom_annals();}
            else {
                // PushState(12) leaves the results (OnLeave826486c0 closes screen 46).
                self.hud.hom_results=None;self.hom_results_ready=false;self.hom_replay_editor=true;
            }
        }
    }
}
impl<T,C> SelectedLiveSession<T,C> {
    pub fn acknowledge_career_hom_results_intro(&mut self) {
        if let Some(career)=self.career.as_mut() {career.hom_results_ready=true;}
    }
}

impl CareerSession {
    pub(super) fn take_hom_replay_editor(&mut self)->bool {std::mem::take(&mut self.hom_replay_editor)}
    /// The Replay Editor pops back to the results: OnEnter82642348 opens screen 46 anew.
    pub(super) fn reenter_hom_results(&mut self) {
        if self.pausing.last()==Some(&PausingState::HomResults) {self.system.world_mut().output(Output::OpenHomResults);}
    }
    pub(super) fn open_hom_annals(&mut self) {
        // PushState82d69cf0 does not keep the previous screen; pop re-enters it.
        match self.pausing.last() {
            Some(PausingState::HomResults)=>{self.hud.hom_results=None;self.hom_results_ready=false;},
            Some(PausingState::Blog)=>{self.leave_slappys_state();self.hud.slappys=None;},
            _=>{},
        }
        self.hud.hom_annals=Some(crate::challenge::hom_annals::view(self.system.world()));
        self.annals_ready=false;self.pausing.push(PausingState::AnnalsOfMeat);
    }
    /// ReceiveInput826a1ef8 and annalsofmeat.UpdateInput9ab0.
    fn hom_annals_input(&mut self,input:crate::ui::pause_menu::PauseMenuInput) {
        if !self.annals_ready {return;}
        let Some(view)=self.hud.hom_annals.as_mut() else{return;};
        if input.cancel {
            self.sounds.push("FE_SOUND_CORE_B_BUTTON");
            self.hud.hom_annals=None;self.pausing.pop();self.annals_ready=false;
            match self.pausing.last() {
                Some(PausingState::HomResults)=>self.system.world_mut().output(Output::OpenHomResults),
                Some(PausingState::Blog)=>self.enter_blog(),
                _=>{},
            }
        } else if input.left_shift||input.right_shift {
            // PrepareInput82d67808 restores mbAllowInput each input pass.
            self.sounds.push("FE_SOUND_CORE_TAB");
            if input.left_shift {view.tab=if view.tab==0 {2}else{view.tab-1};
                if view.tab==0 && view.stats.top.is_empty() {view.tab=2;}}
            else {view.tab=(view.tab+1)%3;if view.tab==0 && view.stats.top.is_empty() {view.tab=1;}}
        } else if view.tab==0 && view.stats.top.len()>1 {
            if input.left {view.bail_index=(view.bail_index+view.stats.top.len()-1)%view.stats.top.len();self.sounds.push("FE_SOUND_CORE_NAV_LEFT");}
            else if input.right {view.bail_index=(view.bail_index+1)%view.stats.top.len();self.sounds.push("FE_SOUND_CORE_NAV_RIGHT");}
        } else if view.tab==2 {
            if input.up {view.path_index=(view.path_index+3)%4;self.sounds.push("FE_SOUND_ANNALS_NAV");}
            else if input.down {view.path_index=(view.path_index+1)%4;self.sounds.push("FE_SOUND_ANNALS_NAV");}
        }
    }
}
impl<T,C> SelectedLiveSession<T,C> {
    pub fn acknowledge_career_hom_annals_intro(&mut self) {
        if let Some(career)=self.career.as_mut() {career.annals_ready=true;}
    }
    /// Install the host's local clock even before a career exists. A newly
    /// created career can save during the same frontend call that created it.
    pub fn set_career_calendar(&mut self, clock: fn() -> [u16; 6]) {
        self.host_calendar = Some(clock);
        if let Some(career) = self.career.as_mut() {
            career.system.world_mut().hom_local_calendar = Some(clock);
        }
    }
    /// FELocationManager.GetLocationKey8261bbb0 / GetLocationName82615b68.
    fn hom_location_name(&self,position:[f32;3])->Option<(u64,String)> {
        let world=self.inner.world.as_ref()?;
        let terrain=self.inner.terrain.as_ref()?.terrain_for(&world.world).ok()?;
        let layers=terrain.layers.lock().ok()?;
        let db=&self.career.as_ref()?.system.world().db;
        let triggers=crate::challenge::keys::hash64("worldpainter_fe_location_triggers");
        let mut key=layers.try_attrib_key(5,[position[0],position[2]]).ok()?;
        if key==0 || !db.has_collection(triggers,key) {key=layers.try_attrib_key(4,[position[0],position[2]]).ok()?;}
        if key==0 {key=u64::MAX;}
        Some((key,crate::challenge::hom_save::location_name(db,key)))
    }
}

impl<T,C> SelectedLiveSession<T,C> {
    /// Inspect the original HoM movies with actual recorded bail data; no score injection.
    pub fn diagnostic_hom_screen(&mut self,annals:bool) {
        if let Some(career)=self.career.as_mut() {
            if annals {career.open_hom_annals();}
            else {career.system.world_mut().output(Output::OpenHomResults);}
        }
    }
}
