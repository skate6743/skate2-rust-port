//! The PLAYER state (`eMAINMAPSTATES` 1): `CSTATEMGR_Player`, `CSTATE_Player`, the
//! `SFXCTL_PlayerPhysics` / `SFXCTL_3DPlayerPos` / `SFXCTL_3DBoardPos` controllers and the skater
//! SFX objects. `Setup(0xFFF)` creates objects 0-11 in id order (`CreateSFXObjs` 0x82a7f130);
//! `SFXObj_PlayerSpeech` (7) needs the speech system and is not ported. TU3's `SFXObj_DropIn`
//! (its object 12) follows them for Skate 3 physics; `skate3_mix` gives the mixer its channels
//! and those of TU3's darkcatch sounds.

pub mod clothing;
pub mod contacts;
pub mod cracks;
pub mod drop_in;
pub mod hand_grabs;
pub mod offboard;
pub mod physics;
pub mod pos3d;
pub mod rail;
pub mod sense_of_speed;
pub mod skate3_mix;
pub mod skateboard;
pub mod treatments;
pub mod tricks;
pub mod wheels;

use crate::aems::Aems;
use crate::global_effects::GlobalEffects;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::presentation::SkaterData;
use crate::random::RwRandom;
use crate::rwac::{AudioCore, PluginRef};
use crate::splicer::{SpliceId, SpliceManager, SplicePlayParams, SpliceUpdateParams};
use crate::vault::Vault;
use crate::vmx::Vec3;
use crate::world::crowd::{CrowdEnv, CrowdManager};
use clothing::Clothing;
use contacts::Contacts;
use cracks::Cracks;
use drop_in::DropIn;
use hand_grabs::HandGrabs;
use offboard::OffBoard;
use physics::{PhysicsInputs, PlayerPhysics};
use pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use rail::Rail;
use sense_of_speed::SenseOfSpeed;
use skateboard::{GrainFiles, SkateBoard};
use treatments::Treatments;
use tricks::Tricks;
use wheels::Wheels;

/// `1/32767`, `1/4096` and the degrees per azimuth unit the SFX objects scale mixer outputs by.
pub const VOLUME_SCALE: f32 = f32::from_bits(0x3800_0100);
pub const PITCH_SCALE: f32 = f32::from_bits(0x3980_0000);
pub const AZIMUTH_SCALE: f32 = f32::from_bits(0x3bb4_00b4);

/// The `Audio_*` cvars the skater objects read (their registered defaults).
#[derive(Debug, Clone, Copy)]
pub struct Cvars {
    pub enable_sweetener: bool,
    pub debug_audio_material: u32,
    pub play_impacted_collision: bool,
    pub play_impactor_collision: bool,
    pub prioritize_hom_sounds: bool,
    pub prioritize_base_sounds: bool,
    pub collision_frame_decay: i32,
    pub skater_grunt_impact_level: i32,
    /// `kPlayDistortion`, `kPlayVolumeModulation`, `kTurnOffDryRolling`.
    pub play_distortion: bool,
    pub play_volume_modulation: bool,
    pub turn_off_dry_rolling: bool,
    /// `kPlayRattle`, `kPlayTone`, `kPlayWind`, `kPlayRocket` (`Audio_SenseOfSpeed`).
    pub play_rattle: bool,
    pub play_tone: bool,
    pub play_wind: bool,
    pub play_rocket: bool,
    /// `kClothingType` (`Audio.PCFoley` "Force Clothing Type", -1..=4).
    pub clothing_type: i32,
    /// `kEnablePCFootSteps`, `kFootStepOverride`, `kFootStepOverrideType` (an audio material) and
    /// `kFootStepShoeType` (0..=5) (`Audio_PCFoley`).
    pub enable_pc_foot_steps: bool,
    pub foot_step_override: bool,
    pub foot_step_override_type: u32,
    pub foot_step_shoe_type: i32,
    /// `kSlideSurface` (`Audio_Collision` "Set Slide Surface", -1..=6; -1 keeps the contact's).
    pub slide_surface: i32,
    /// `kEnableTreatments` (`Audio_Debug` "Enable Treaments").
    pub enable_treatments: bool,
    /// `kNoRailWhenUntagged` (`Audio_Debug` "No Rail Sounds When Not Tagged").
    pub no_rail_when_untagged: bool,
    /// Audio_HOM kNoHOMInReplay, registered false.
    pub no_hom_in_replay: bool,
}

impl Default for Cvars {
    fn default() -> Self {
        Self {
            enable_sweetener: true,
            debug_audio_material: crate::presentation::MATERIAL_MAX,
            play_impacted_collision: true,
            play_impactor_collision: true,
            prioritize_hom_sounds: true,
            prioritize_base_sounds: false,
            collision_frame_decay: 5,
            skater_grunt_impact_level: 0,
            play_distortion: true,
            play_volume_modulation: true,
            turn_off_dry_rolling: false,
            play_rattle: true,
            play_tone: false,
            play_wind: true,
            play_rocket: true,
            clothing_type: 1,
            enable_pc_foot_steps: true,
            foot_step_override: false,
            foot_step_override_type: 0,
            foot_step_shoe_type: 0,
            slide_surface: -1,
            enable_treatments: true,
            no_rail_when_untagged: false,
            no_hom_in_replay: false,
        }
    }
}

/// `AudioSystem::PlayCollisionSound`'s `tCollisionInfo` (48 bytes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionInfo {
    pub material: [u32; 2],
    pub severity: [u32; 2],
    pub position: Vec3,
    pub volume: [i32; 2],
    pub dmo_collision: bool,
}

/// What an SFX object reaches through singletons, its state and its `m_pDMixIO`.
pub struct SfxCtx<'a> {
    pub core: &'a mut AudioCore,
    /// Csis and AEMS.
    pub aems: &'a mut Aems,
    pub splicer: &'a mut SpliceManager,
    pub map: &'a mut MixMap,
    pub vault: &'a Vault,
    /// PC port: a linked Skate 3 data folder's attributes (TU3's darkcatch sounds), none unlinked.
    pub skate3: Option<&'a Vault>,
    pub rng: &'a mut RwRandom,
    pub master_sub_mix: PluginRef,
    /// The `GlobalEffects` singleton: the reverb and flange sub mixes, the flange pan.
    pub global_effects: &'a mut GlobalEffects,
    pub cvars: Cvars,
    /// `CSTATE_Base::t_DeltaTime`.
    pub delta_time: f32,
    /// `CSTATE_Player::mStateData.local`.
    pub local: bool,
    /// `VDPresentationState::mOutData.bBail` / `fBailSeverity`.
    pub bail: bool,
    pub bail_severity: f32,
    /// `VDPresentationState::mOutData.bOutOfBounds` / `fOutOfBoundsAmount`.
    pub out_of_bounds: bool,
    pub out_of_bounds_amount: f32,
    /// `AudioSystem::mIsHallOfMeat`.
    pub hall_of_meat: bool,
    /// `AudioSystem::mbIsTimesOneFive`, `mbIsTimesTwo`, `mbIsTimesThree`.
    pub times: [bool; 3],
    /// `AudioSystem::mbInOwnTheSpot` and `mChallengeType`.
    pub in_own_the_spot: bool,
    pub challenge_type: i32,
    /// `MicrophoneMan` microphone 0 (the camera) position.
    pub camera_position: Vec3,
    /// `AudioSystem::GetNPCWheelType`.
    pub npc_wheel_type: u32,
    /// `AudioSystem::PlayCollisionSound` requests, applied to the collision manager in order.
    pub collisions: &'a mut Vec<CollisionInfo>,
    /// `AudioSystem::PlayCrowdSound`'s crowd manager (`mStateManagers[7]`) and what it reads,
    /// during the passes that can cheer.
    pub crowds: Option<&'a mut CrowdManager>,
    pub crowd_env: Option<CrowdEnv<'a>>,
    /// `AudioSystem::mCondScore - mCondPrevScore`.
    pub cond_score_delta: i32,
}

impl SfxCtx<'_> {
    /// `AudioSystem::PlayCrowdSound` 0x824e69e0: `CSTATEMGR_Crowd::PlayCrowdSound` at once.
    pub fn play_crowd_sound(&mut self, skater_id: i32, score: i32, wiped_out_score: i32, local: bool) {
        let Some(env) = self.crowd_env else { return };
        let Some(crowds) = self.crowds.take() else { return };
        let inputs = env.inputs(skater_id, self.vault);
        crowds.play_crowd_sound(skater_id, score, wiped_out_score, local, &inputs, env.banks_loaded, self);
        self.crowds = Some(crowds);
    }

    pub fn set_input(&mut self, io: Option<DMixIoId>, index: usize, value: i32) {
        if let Some(io) = io {
            self.map.set_input(io, index, value);
        }
    }

    /// `SndBase::GetDMixOutput`.
    #[must_use]
    pub fn output(&self, io: Option<DMixIoId>, index: i32, kind: PresetType) -> i32 {
        io.map_or(0, |io| self.map.output(io, index, kind))
    }

    /// `new Splicer::Splice(type, index, subMix)` and `Play` with the zero-volume update every
    /// object passes (`mfDelta` = the state's `t_DeltaTime`).
    pub fn new_splice(&mut self, bank: i32, index: i32, sub_mix: PluginRef) -> Option<SpliceId> {
        let bank = usize::from(bank as i8 as u8);
        let splice = self.splicer.create_splice(self.core, bank, index, sub_mix)?;
        let update = SpliceUpdateParams { vol: 0.0, pitch: 1.0, az: 0.0, delta: self.delta_time, spread: 1.0, time_stretch: 1.0 };
        self.splicer.play_splice(self.core, splice, SplicePlayParams::default(), &update);
        Some(splice)
    }

    /// `delete splice` (with the `~Splice` release).
    pub fn release(&mut self, splice: &mut Option<SpliceId>) {
        if let Some(id) = splice.take() {
            self.splicer.release_splice(self.core, id);
        }
    }

    #[must_use]
    pub fn is_playing(&self, splice: SpliceId) -> bool {
        self.splicer.is_playing(splice)
    }

    pub fn update_splice(&mut self, splice: SpliceId, mut update: SpliceUpdateParams) {
        self.splicer.update_splice(self.core, splice, &mut update);
    }
}

/// `CSTATE_Player::mStateData` (`SkaterAudioStateData`) fields the ported objects read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkaterAudioStateData {
    pub skater_id: i32,
    pub array_index: i32,
    pub local: bool,
    pub character_type: u32,
}

/// One `CSTATE_Player` with its controllers and objects.
pub struct PlayerState {
    pub instance: u32,
    pub attached: bool,
    pub state_data: SkaterAudioStateData,
    pub wheel_type: u32,
    pub cur_time: f32,
    pub delta_time: f32,
    pub physics: PlayerPhysics,
    pub player_pos: Obj3d,
    pub board_pos: Obj3d,
    pub skateboard: SkateBoard,
    pub contacts: Contacts,
    contacts_io: Option<DMixIoId>,
    pub wheels: Wheels,
    pub rail: Rail,
    pub cracks: Cracks,
    pub tricks: Tricks,
    pub clothing: Clothing,
    pub treatments: Treatments,
    pub sense_of_speed: SenseOfSpeed,
    pub offboard: OffBoard,
    pub hand_grabs: HandGrabs,
    /// TU3's `SFXObj_DropIn` (its PLAYER object 12).
    pub drop_in: DropIn,
    /// Whether the position controllers' `Ref3D` pointers are assigned (`InitSFX` assigns them,
    /// `Detach` clears position and direction).
    refs_assigned: bool,
}

/// The objectID of a PLAYER-state object or controller.
#[must_use]
pub fn object_id(instance: u32, sfx: u32) -> u32 {
    (1 << 16) | (instance << 11) | (sfx << 4)
}

impl PlayerState {
    fn new(instance: u32, vault: &Vault, map: Option<&MixMap>, grains: &GrainFiles) -> Self {
        let io = |kind: u32, sfx: u32| map.and_then(|map| map.dmixio(kind | object_id(instance, sfx)));
        let mut physics = PlayerPhysics { io: io(0x6000_0000, 0), ..PlayerPhysics::default() };
        physics.reset();
        let mut player_pos = Obj3d::new(vault);
        player_pos.io = io(0x6000_0000, 1);
        let mut board_pos = Obj3d::new(vault);
        board_pos.io = io(0x6000_0000, 2);
        let mut skateboard = SkateBoard::new(vault, grains, 4 * instance);
        skateboard.io = io(0x4000_0000, 0);
        let mut wheels = Wheels::new(vault, grains);
        wheels.io = io(0x4000_0000, 2);
        let mut rail = Rail::default();
        rail.io = io(0x4000_0000, 3);
        let mut cracks = Cracks::default();
        cracks.io = io(0x4000_0000, 4);
        let mut tricks = Tricks::default();
        tricks.io = io(0x4000_0000, 5);
        let mut clothing = Clothing::default();
        clothing.io = io(0x4000_0000, 6);
        let mut treatments = Treatments::default();
        treatments.io = io(0x4000_0000, 8);
        let mut sense_of_speed = SenseOfSpeed::new(vault, grains, sense_of_speed::ROCKET_TIMER_BASE + instance);
        sense_of_speed.io = io(0x4000_0000, 9);
        let mut offboard = OffBoard::new(vault);
        offboard.io = io(0x4000_0000, 10);
        let mut hand_grabs = HandGrabs::default();
        hand_grabs.io = io(0x4000_0000, 11);
        let mut drop_in = DropIn::default();
        drop_in.io = io(0x4000_0000, skate3_mix::DROP_IN_SFX);
        let mut contacts = Contacts::new(vault);
        contacts.skate3_io = io(0x4000_0000, skate3_mix::DARKCATCH_SFX);
        Self {
            instance,
            attached: false,
            state_data: SkaterAudioStateData { skater_id: -1, array_index: -1, local: false, character_type: 0 },
            wheel_type: physics::WHEEL_HARD,
            cur_time: 0.0,
            delta_time: 0.0,
            physics,
            player_pos,
            board_pos,
            skateboard,
            contacts,
            contacts_io: io(0x4000_0000, 1),
            wheels,
            rail,
            cracks,
            tricks,
            clothing,
            treatments,
            sense_of_speed,
            offboard,
            hand_grabs,
            drop_in,
            refs_assigned: false,
        }
    }

    /// `CSTATE_Player::Attach` 0x825515d8: the state data, `mWheelType = Hard`, then
    /// `CSTATE_Base::Attach` (controllers' `InitSFX`, objects' `SetupLoadData`).
    fn attach(&mut self, data: SkaterAudioStateData, ctx: &mut SfxCtx<'_>) {
        self.state_data = data;
        self.wheel_type = physics::WHEEL_HARD;
        self.attached = true;
        self.physics.init_sfx(ctx.map);
        self.refs_assigned = true;
        for io in [self.player_pos.io, self.board_pos.io].into_iter().flatten() {
            ctx.map.set_output_enabled(io, true);
        }
        self.skateboard.init_sfx(&self.physics, data.local, ctx);
        for io in [self.contacts_io, self.contacts.skate3_io].into_iter().flatten() {
            ctx.map.set_output_enabled(io, true);
        }
        self.wheels.init_sfx(ctx);
        self.rail.init_sfx(&self.physics, ctx);
        self.cracks.init_sfx(&self.physics, ctx);
        self.tricks.init_sfx(ctx);
        self.clothing.init_sfx(&self.physics, ctx);
        self.treatments.init_sfx(&self.physics, ctx);
        self.sense_of_speed.init_sfx(&self.physics, ctx);
        self.offboard.init_sfx(&self.physics, ctx);
        self.hand_grabs.init_sfx(&self.physics, ctx);
        self.drop_in.init_sfx(ctx);
    }

    /// `CSTATE_Player::Detach` 0x82551630 / `CSTATE_Base::Detach` 0x82a7e350.
    fn detach(&mut self, ctx: &mut SfxCtx<'_>) {
        self.attached = false;
        self.skateboard.destroy(ctx);
        disconnect(self.skateboard.io, ctx.map);
        self.contacts.destroy(ctx);
        disconnect(self.contacts_io, ctx.map);
        disconnect(self.contacts.skate3_io, ctx.map);
        self.wheels.destroy(ctx);
        disconnect(self.wheels.io, ctx.map);
        self.rail.destroy(ctx);
        disconnect(self.rail.io, ctx.map);
        self.cracks.destroy(ctx);
        disconnect(self.cracks.io, ctx.map);
        self.tricks.destroy(ctx);
        disconnect(self.tricks.io, ctx.map);
        self.clothing.destroy(ctx);
        disconnect(self.clothing.io, ctx.map);
        self.treatments.destroy(ctx);
        disconnect(self.treatments.io, ctx.map);
        self.sense_of_speed.destroy(ctx);
        disconnect(self.sense_of_speed.io, ctx.map);
        self.offboard.destroy(ctx);
        disconnect(self.offboard.io, ctx.map);
        self.hand_grabs.destroy(ctx);
        disconnect(self.hand_grabs.io, ctx.map);
        self.drop_in.destroy(ctx);
        disconnect(self.drop_in.io, ctx.map);
        self.physics.reset();
        self.refs_assigned = false;
        for io in [self.physics.io, self.player_pos.io, self.board_pos.io].into_iter().flatten() {
            ctx.map.clear_input(io);
        }
        self.state_data.skater_id = -1;
        self.state_data.local = false;
        self.state_data.array_index = -1;
    }

    fn player_refs(&self) -> ObjRefs {
        if !self.refs_assigned {
            return ObjRefs { vel: Some(self.physics.velocity), ..ObjRefs::default() };
        }
        ObjRefs { dir: Some(self.physics.forward), pos: Some(self.physics.position), vel: Some(self.physics.velocity) }
    }

    fn board_refs(&self) -> ObjRefs {
        if !self.refs_assigned {
            return ObjRefs { vel: Some(self.physics.board_velocity), ..ObjRefs::default() };
        }
        ObjRefs { dir: Some(self.physics.board_forward), pos: Some(self.physics.board_position), vel: Some(self.physics.board_velocity) }
    }
}

/// What the PLAYER manager reads from `Presentation` each update.
#[derive(Debug, Clone, Default)]
pub struct PlayerFrame {
    /// `AudioPresentation::mInterpolatedSkater[0..mCount]`.
    pub skaters: Vec<SkaterData>,
    pub game_speed: f32,
    pub is_rewinding: bool,
}

/// `CSTATEMGR_Player`.
pub struct PlayerManager {
    pub states: Vec<PlayerState>,
    previous_local: i32,
    cur_time: f32,
    pub shared: Pos3dShared,
}

impl PlayerManager {
    /// `CSTATEMGR_Player::EnterGamePlay` 0x8254fde0: two states, each with every player object.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>, grains: &GrainFiles) -> Self {
        Self { states: (0..2).map(|instance| PlayerState::new(instance, vault, map, grains)).collect(), previous_local: -1, cur_time: 0.0, shared: Pos3dShared::default() }
    }

    /// The `TIMERSTAGE_BEFORECOMMANDS` timer `id` of a skateboard's grain player (four per state)
    /// or of a state's rocket grain player (from `ROCKET_TIMER_BASE`).
    pub fn grain_timer(&mut self, id: u32, core: &mut AudioCore, rng: &mut RwRandom, period: f32) {
        if let Some(state) = id.checked_sub(sense_of_speed::ROCKET_TIMER_BASE).and_then(|index| self.states.get_mut(index as usize)) {
            state.sense_of_speed.rocket_timer(core, rng, period);
        } else if let Some(state) = self.states.get_mut(id as usize / 4) {
            state.skateboard.grain_timer(id as usize % 4, core, rng, period);
        }
    }

    /// `CSTATEMGR_Player::UpdateParams` 0x82556c60: the states first, then attach/detach.
    pub fn update_params(&mut self, t: f32, frame: &PlayerFrame, mics: &MicrophoneMan, inputs: PhysicsInputs, ctx: &mut SfxCtx<'_>) {
        self.cur_time += t;
        for index in 0..self.states.len() {
            self.update_state(index, t, frame, mics, inputs, ctx);
        }
        let local = frame.skaters.iter().rposition(|skater| skater.local).map_or(-1, |index| index as i32);
        if local != self.previous_local {
            for state in &mut self.states {
                if state.attached {
                    state.detach(ctx);
                }
            }
            self.previous_local = local;
        }
        for (index, skater) in frame.skaters.iter().enumerate() {
            if skater.ghosted || !(skater.local || in_range(skater.skater_pos, mics)) {
                continue;
            }
            let player_index = skater.player_index as i32;
            if let Some(state) = self.states.iter_mut().find(|state| state.attached && state.state_data.skater_id == player_index) {
                state.state_data.array_index = index as i32;
                state.state_data.local = skater.local;
                continue;
            }
            if let Some(state) = self.states.iter_mut().find(|state| !state.attached) {
                let data = SkaterAudioStateData { skater_id: player_index, array_index: index as i32, local: skater.local, character_type: skater.character };
                state.attach(data, ctx);
            } else if skater.local {
                let state = &mut self.states[0];
                state.detach(ctx);
                let data = SkaterAudioStateData { skater_id: player_index, array_index: index as i32, local: true, character_type: skater.character };
                state.attach(data, ctx);
            }
        }
    }

    /// `CSTATEMGR_Player::GetNPCWheelType` 0x8254fee0: the opposite of the first local state's
    /// `mWheelType`, soft without a local state.
    #[must_use]
    pub fn npc_wheel_type(&self) -> u32 {
        self.states.iter().find(|state| state.state_data.local).map_or(physics::WHEEL_SOFT, |state| u32::from(state.wheel_type == physics::WHEEL_HARD))
    }

    /// `CSTATE_Player::UpdateParams` 0x825587b8 and `CSTATE_Base::UpdateParams` 0x82a7e0e0.
    fn update_state(&mut self, index: usize, t: f32, frame: &PlayerFrame, mics: &MicrophoneMan, inputs: PhysicsInputs, ctx: &mut SfxCtx<'_>) {
        let npc_wheel_type = self.npc_wheel_type();
        let state = &mut self.states[index];
        let array_index = state.state_data.array_index;
        if array_index != -1 {
            let skater = usize::try_from(array_index).ok().and_then(|index| frame.skaters.get(index));
            let out_of_range = match skater {
                None => true,
                Some(skater) => !skater.local && !in_range(skater.skater_pos, mics),
            };
            if out_of_range {
                state.detach(ctx);
            }
        }
        state.delta_time = t;
        state.cur_time += t;
        if !state.attached {
            return;
        }
        let Some(skater) = usize::try_from(state.state_data.array_index).ok().and_then(|index| frame.skaters.get(index)) else { return };
        ctx.delta_time = t;
        ctx.local = state.state_data.local;
        ctx.npc_wheel_type = npc_wheel_type;
        let inputs = PhysicsInputs { local: state.state_data.local, npc_wheel_type, ..inputs };
        state.wheel_type = state.physics.update_params(t, skater, inputs, ctx.vault, mics, ctx.map);
        let player_refs = state.player_refs();
        state.player_pos.update_params(t, player_refs, mics, &mut self.shared, ctx.map);
        let board_refs = state.board_refs();
        state.board_pos.update_params(t, board_refs, mics, &mut self.shared, ctx.map);
        state.skateboard.update_params(t, &state.physics, ctx);
        let io = state.contacts_io;
        state.contacts.update_params(t, &state.physics, skater, state.state_data.array_index, io, ctx);
        state.rail.update_params(t, &state.physics, ctx);
        state.cracks.update_params(&state.physics, ctx);
        state.tricks.update_params(t, &state.physics, ctx);
        state.clothing.update_params(t, &state.physics, ctx);
        state.treatments.update_params(t, &state.physics, ctx);
        state.sense_of_speed.update_params(t, &state.physics, ctx);
        state.offboard.update_params(t, &state.physics, ctx);
        state.hand_grabs.update_params(t, &state.physics, ctx);
        state.drop_in.update_params(&state.physics, ctx);
    }

    /// `CSTATEMGR_Base::ProcessUpdate` 0x82a7dc40.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            ctx.delta_time = state.delta_time;
            ctx.local = state.state_data.local;
            state.skateboard.process_update(&state.physics, ctx);
            let io = state.contacts_io;
            state.contacts.process_update(&state.physics, io, ctx);
            state.wheels.process_update(&state.physics, ctx);
            state.rail.process_update(&state.physics, ctx);
            state.cracks.process_update(&state.physics, ctx);
            state.tricks.process_update(&state.physics, ctx);
            state.clothing.process_update(&state.physics, ctx);
            state.treatments.process_update(&state.physics, ctx);
            state.sense_of_speed.process_update(&state.physics, ctx);
            state.offboard.process_update(&state.physics, ctx);
            state.hand_grabs.process_update(&state.physics, ctx);
            state.drop_in.process_update(ctx);
        }
    }
}

/// `CSTATE_Base::Detach`'s per-object `TurnOffMixOutput` and input clear.
fn disconnect(io: Option<DMixIoId>, map: &mut MixMap) {
    if let Some(io) = io {
        map.set_output_enabled(io, false);
        map.clear_input(io);
    }
}

/// `CSTATE_Player::IsInRange` 0x825561f8: within 30 m of the camera microphone.
fn in_range(position: Vec3, mics: &MicrophoneMan) -> bool {
    crate::vmx::length(crate::vmx::sub(position, mics.microphones[0].cur_position)) < 30.0
}
