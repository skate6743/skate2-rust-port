//! The game audio runtime: `skate2_audio::system::AudioSystem` behind a mutex. The simulation tick
//! runs `AudioSystem::Update` with the session's camera and the skaters' `PhysOut`s, the
//! marker UI's `Audio.PlaySound` calls queue menu sounds, and a cpal output stream pulls 256-frame
//! RWAC blocks (48 kHz, folded to stereo by RWAC's 6-to-2 map). Devices running at another rate get
//! a linear-interpolation resample; nothing else is changed.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use skate2_audio::grain_player::GrainData;
use skate2_audio::player::pos3d::CameraMatrix;
use skate2_audio::player::skateboard::GrainFiles;
use skate2_audio::player::PlayerFrame;
use skate2_audio::presentation::{audio_material, multiplier_flags, CollisionInfo, PhysOutView};
use skate2_audio::system::{AemsFiles, AudioSystem, FeSound, GameAudioState, WorldAudioState};
use skate2_audio::vault::{AttribSource, Vault};
use skate2_audio::world::pedestrian::{PedStateOut, PedestrianData};
use skate2_audio::world::dynamic_object::{DmoSwapFiles, DynamicData};
use skate2_audio::world::traffic::VehicleData;
use skate2_core::challenge::attrib_db::AttribDb;
use skate2_core::ui::music_player::MusicRequest;
use skate2_core::ui::FeSoundRequest;
use skate2_core::{AudioSkater, SkaterAudioPhysOut};
use skate2_formats::attrib::string_hash64;
use skate2_formats::big::BigEntryKey;
use skate2_formats::mount::{MountSet, MountedBig};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool,Ordering};
use std::time::Instant;

/// `Attrib::Gen::fe`'s class.
const FE_CLASS: u64 = 0x5831_CB95_F3E9_0598;
const HOM_SPLICE: u64 = 0x845A_1005_2522_A2FB;
const LOGO_TYPE: u64 = 0xA408_0FB6_5880_E3C2;
const FLANGE: u64 = 0xBF45_D439_FAC7_1A2E;
const HOM_EXPIRE_TIME: u64 = 0x9DAD_777A_FEA4_E73C;
const MIX_RATE: u32 = 48_000;

/// The game root, derived from the Boot archive the way `marker_ui::Assets::load` does.
fn game_root(mounts: &MountSet) -> Result<PathBuf, String> {
    let boot = mounts
        .archives()
        .iter()
        .find(|a| a.data_path().file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("miscboot.big")))
        .ok_or("audio requires the miscboot mount")?;
    boot.data_path().parent().and_then(Path::parent).and_then(Path::parent).map(Path::to_path_buf).ok_or_else(|| "invalid Boot archive path".to_owned())
}

/// The splicer banks `LoadSkateboardData` 0x824ee4b8 (`splicerskateboard.big`) and
/// `LoadCollisionData` 0x824ee348 (`splicercollision.big`) load, in their order, with each bank's
/// `SPLICE_TYPE`.
const SKATEBOARD_BANKS: [(&str, usize); 20] = [
    ("CellPhone_Rings.bnk", 27),
    ("Foot_Scrapes.bnk", 17),
    ("L4_Lands_Ollies.bnk", 20),
    ("sk82_cloth.bnk", 23),
    ("Sk82_Whsh_Bys.bnk", 21),
    ("sk8_foley_rattle.bnk", 24),
    ("SK8_Foot_Catch.bnk", 16),
    ("Sk8_Lands_Hrd_Cnc.bnk", 8),
    ("Sk8_Lands_Hrd_Wd.bnk", 9),
    ("Sk8_Lands_Sft_Cnc.bnk", 10),
    ("Sk8_Lands_Sft_Wd.bnk", 11),
    ("SK8_Ollie_Scrapes.bnk", 14),
    ("SK8_Ollies_Cnc.bnk", 12),
    ("SK8_Ollies_Wood.bnk", 13),
    ("SKATE_foot_pushes.bnk", 0),
    ("Splice_dirtfeet.bnk", 3),
    ("Splice_grassfeet.bnk", 4),
    ("Splice_Hands.bnk", 19),
    ("Splice_Smoothconc.bnk", 1),
    ("Splice_woodfoot.bnk", 2),
];
const COLLISION_BANKS: [(&str, usize); 2] = [("Skate_Collisions.bnk", 5), ("Skate_Metal.bnk", 6)];

/// `skater.vlt` as the audio code reads it (`Attrib::Instance::GetAttributePointer`, `mLayoutPtr`).
struct VaultSource(AttribDb);

impl AttribSource for VaultSource {
    fn attribute(&self, class: u64, collection: u64, key: u64, index: u32) -> Option<Vec<u8>> {
        self.0.attribute(class, collection, key)?.value_bytes(index as usize)
    }

    fn layout_word(&self, class: u64, collection: u64, index: u32) -> u32 {
        self.0.layout_word(class, collection, index)
    }

    fn has_collection(&self, class: u64, collection: u64) -> bool {
        self.0.has_collection(class, collection)
    }

    fn layout_text(&self, class: u64, collection: u64, index: u32) -> Option<String> {
        self.0.layout_text(class, collection, index).map(str::to_owned)
    }

    fn text(&self, class: u64, collection: u64, key: u64, index: u32) -> Option<String> {
        self.0.attribute(class, collection, key)?.text(index as usize).map(str::to_owned)
    }

    fn length(&self, class: u64, collection: u64, key: u64) -> u32 {
        self.0.attribute(class, collection, key).map_or(0, |attribute| attribute.len() as u32)
    }

    fn leaf_collections(&self, class: u64) -> Vec<u64> {
        self.0.leaf_collections(class)
    }
}

/// The `fe` collections and the Apt globals `FrontEndManager::RegisterAudioEnums` 0x8261b820 makes
/// from them: `FE_SOUND_<NAME>` holds `name`, and `Audio.PlaySound` keys it with
/// `Attrib::StringToLowerCaseKey`.
fn fe_sounds(db: &AttribDb) -> (HashMap<u64, FeSound>, HashMap<String, u64>) {
    let mut sounds = HashMap::new();
    let mut globals = HashMap::new();
    for collection in db.leaf_collections(FE_CLASS) {
        let Some(name) = db.layout_text(FE_CLASS, collection, 0).map(str::to_owned) else { continue };
        let attribute = |key: u64| db.attribute(FE_CLASS, collection, key);
        let sound = FeSound {
            volume: f32::from_bits(db.layout_word(FE_CLASS, collection, 1)),
            menu_splice: db.layout_word(FE_CLASS, collection, 2) as i32,
            fe_splice: db.layout_word(FE_CLASS, collection, 3) as i32,
            hom_splice: attribute(HOM_SPLICE).and_then(|a| a.u32(0)).unwrap_or(0),
            logo_type: attribute(LOGO_TYPE).and_then(|a| a.i32(0)).unwrap_or(0),
            flange: attribute(FLANGE).and_then(|a| a.bool(0)).unwrap_or(false),
            hom_expire_time: attribute(HOM_EXPIRE_TIME).and_then(|a| a.i32(0)).unwrap_or(0),
            name: name.clone(),
        };
        globals.insert(format!("FE_SOUND_{}", name.to_uppercase()), string_hash64(&name.to_lowercase()));
        sounds.insert(collection, sound);
    }
    (sounds, globals)
}

/// The banks of one `data\audio` splicer archive.
fn splicer_banks(root: &Path, archive: &str, banks: &[(&str, usize)]) -> Result<Vec<(String, usize, Arc<[u8]>)>, String> {
    let big = MountedBig::open(root.join("data/audio").join(archive), false).map_err(|e| format!("{archive}: {e}"))?;
    banks
        .iter()
        .map(|&(name, slot)| big.read_path(name).map(|bytes| (name.to_owned(), slot, Arc::from(bytes))).map_err(|e| format!("{name}: {e}")))
        .collect()
}

/// `data\audio\grains.big`'s `.grain` files, which the skateboards' `LoadDataForSurface` loads by
/// their `GrainFile` name, and the loose `data\audio` samples and seek tables (`.snr`, `.sek`)
/// the wheels load by name (`AudioAssetManager::SyncLoad`, `FileLoader::SyncLoad`).
fn grain_files(root: &Path) -> Result<GrainFiles, String> {
    let big = MountedBig::open(root.join("data/audio/grains.big"), false).map_err(|e| format!("grains.big: {e}"))?;
    let mut files = GrainFiles::new();
    for entry in &big.archive().entries {
        let BigEntryKey::Name(name) = &entry.key else { continue };
        let bytes = big.read_entry(entry).map_err(|e| format!("{name}: {e}"))?;
        files.insert(name.to_lowercase(), GrainData { bytes: Arc::from(bytes) });
    }
    let audio = skate2_formats::host_path::on_disk(&root.join("data/audio"));
    for entry in std::fs::read_dir(&audio).map_err(|e| format!("data/audio: {e}"))? {
        let path = entry.map_err(|e| format!("data/audio: {e}"))?.path();
        let loose = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("snr") || e.eq_ignore_ascii_case("sek"));
        let Some(name) = path.file_name().and_then(|n| n.to_str()).filter(|_| loose && path.is_file()) else { continue };
        let bytes = std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
        files.insert(name.to_lowercase(), GrainData { bytes: Arc::from(bytes) });
    }
    Ok(files)
}

/// `LoadCsisEventSystem` 0x824edc20's interfaces.
const AEMS_INTERFACES: [(&str, &str); 8] = [
    ("skateboard.big", "SK8_AEMS_skateboard.csi"),
    ("pedestrian.big", "SK8_AEMS_Foley.csi"),
    ("emitters2.big", "Sk8_Emitters_Project.csi"),
    ("rolling.big", "SK8_AEMS_rolling.csi"),
    ("vehicle.big", "AEMS_TRAFFIC.csi"),
    ("crowds.big", "SK8_AEMS_Crowds.csi"),
    ("dynamicobject.big", "Sk8_AEMS_MoveableObjects.csi"),
    ("moments.big", "Sk8_moments.csi"),
];

/// The AEMS banks in `AudioAssetManager::SyncLoad` order: `Initialize`, `LoadInGameData`
/// (`LoadSkateboardData` 0x824ee4b8, `LoadFoleyData` 0x824f0bf0), then the
/// `SFXObj_SkateBoard` constructor's patch banks.
const AEMS_BANKS: [(&str, &str); 20] = [
    ("emitters2.big", "emitter_utility.abk"),
    ("emitters2.big", "Main_Ambience_Crossfade.abk"),
    ("skateboard.big", "board_scrapes.abk"),
    ("skateboard.big", "Brd_Squeaks.abk"),
    ("skateboard.big", "FOOT_DRAG.abk"),
    ("skateboard.big", "GRINDS.abk"),
    ("skateboard.big", "Rolling_Rattles.abk"),
    ("skateboard.big", "Seams_Bank.abk"),
    ("skateboard.big", "sense_of_speed.abk"),
    ("skateboard.big", "Sk8_Air_Flip_Tricks.abk"),
    ("skateboard.big", "Treatments.abk"),
    ("skateboard.big", "WHEEL_SKID_BANK.abk"),
    ("pedestrian.big", "Bodyslide.abk"),
    ("pedestrian.big", "Foley_Cloth.abk"),
    ("pedestrian.big", "fstep_livingworld.abk"),
    ("pedestrian.big", "fstep_skateshoe1_sm.abk"),
    ("rolling.big", "PatchBank_Objects.abk"),
    ("rolling.big", "PatchBank_RocksBounce.abk"),
    ("rolling.big", "PatchBank_Rolling_Surfaces.abk"),
    ("rolling.big", "PatchBank_SpiderCracks.abk"),
];

/// The banks `SFXObj_Dynamic`'s constructor 0x82533670 loads from `data\audio\dynamicobject.big`.
const DMO_BANKS: [(&str, &str); 4] = [
    ("dynamicobject.big", "DMO_Starts.abk"),
    ("dynamicobject.big", "Dynamics_Objects_Roll.abk"),
    ("dynamicobject.big", "Dynamics_Objects_Slide_A.abk"),
    ("dynamicobject.big", "Dynamics_Objects_Slide_B.abk"),
];

/// `AudioSystem::LoadMomentsData` 0x824f0e50's banks (`SecondaryLoad`): the modules of
/// `Sk8_moments.csi`'s `fade_to_white`, `killedit_*` and `ownedit_logo`.
const MOMENTS_BANKS: [(&str, &str); 3] = [
    ("moments.big", "fade_to_white.abk"),
    ("moments.big", "killed_it.abk"),
    ("moments.big", "owned_it.abk"),
];

/// `GetHOMBank` 0x824e6a08's banks (`sHOMBankNames`), in their order.
fn hom_banks(root: &Path) -> Result<Vec<Arc<[u8]>>, String> {
    let big = MountedBig::open(root.join("data/audio/splicercollision.big"), false).map_err(|e| format!("splicercollision.big: {e}"))?;
    skate2_audio::main_objects::HOM_BANK_NAMES.iter().map(|&name| big.read_path(name).map(Arc::from).map_err(|e| format!("{name}: {e}"))).collect()
}

/// `AudioSystem::LoadVehiclesData` 0x824fd868's banks in its `AsyncLoadAt` order: the engines
/// (`sEngineNames`), `Traffic_Horn.abk` and `Traffic_Skid.abk` of `vehicle.big`
/// (`mpEnginesBigFileHdr`), then `car_alarms.abk` of `emitters2.big` (`mpEmittersBigFileHdr`).
/// `SecondaryLoad` asks for them and `Update` loads them once no crowd banks are loaded.
const VEHICLE_BANKS: [(&str, &str); 11] = [
    ("vehicle.big", "C00_heavy01.abk"),
    ("vehicle.big", "C01_family01.abk"),
    ("vehicle.big", "C03_sports01.abk"),
    ("vehicle.big", "C04_taxi01.abk"),
    ("vehicle.big", "C05_truck01.abk"),
    ("vehicle.big", "C06_sports02.abk"),
    ("vehicle.big", "C07_family02.abk"),
    ("vehicle.big", "C08_family03.abk"),
    ("vehicle.big", "Traffic_Horn.abk"),
    ("vehicle.big", "Traffic_Skid.abk"),
    ("emitters2.big", "car_alarms.abk"),
];

/// The banks `AudioSystem::LoadCrowdsData` 0x824fdcc8 can load from `crowds.big`
/// (`mpCrowdsBigFileHdr`): every name of `sCrowdNamesThree` to `sCrowdNamesArenaOhhs`.
const CROWD_BANKS: [(&str, &str); 11] = [
    ("crowds.big", "crowds_threes_a.abk"),
    ("crowds.big", "crowds_threes_b.abk"),
    ("crowds.big", "crowds_fives_a.abk"),
    ("crowds.big", "crowds_fives_b.abk"),
    ("crowds.big", "crowds_tens_a.abk"),
    ("crowds.big", "crowds_tens_b.abk"),
    ("crowds.big", "crowds_hundreds_a.abk"),
    ("crowds.big", "crowds_hundreds_b.abk"),
    ("crowds.big", "arena_ambience.abk"),
    ("crowds.big", "arena_cheers.abk"),
    ("crowds.big", "arena_ohhs.abk"),
];

fn big_file(root: &Path, big: &str, name: &str) -> Result<Vec<u8>, String> {
    let archive = MountedBig::open(root.join("data/audio").join(big), false).map_err(|e| format!("{big}: {e}"))?;
    let entry = archive
        .archive()
        .entries
        .iter()
        .find(|entry| matches!(&entry.key, BigEntryKey::Name(n) if n.eq_ignore_ascii_case(name)))
        .ok_or_else(|| format!("{big}: no {name}"))?;
    archive.read_entry(entry).map_err(|e| format!("{name}: {e}"))
}

/// `Sk8EmitterSystem::LoadGameFiles` 0x824e9e00's data files, in its order.
const EMITTER_DATA_FILES: [&str; 5] = ["sfx.ems", "reverb.ems", "music.ems", "speakers.ems", "crowds.ems"];

fn aems_files(root: &Path) -> Result<AemsFiles, String> {
    let mut files = AemsFiles::default();
    for name in EMITTER_DATA_FILES {
        let bytes = std::fs::read(skate2_formats::host_path::on_disk(&root.join("data/audio").join(name))).map_err(|e| format!("{name}: {e}"))?;
        files.emitters.data_files.push((format!("data\\audio\\{name}"), bytes));
    }
    // The banks the emitter system's swap slots load from `data\audio\emitters.big`.
    let emitters = MountedBig::open(root.join("data/audio/emitters.big"), false).map_err(|e| format!("emitters.big: {e}"))?;
    for entry in &emitters.archive().entries {
        if let BigEntryKey::Name(name) = &entry.key {
            let bytes = emitters.read_entry(entry).map_err(|e| format!("{name}: {e}"))?;
            files.emitters.banks.push((name.to_string(), Arc::from(bytes)));
        }
    }
    for (big, name) in AEMS_INTERFACES {
        files.interfaces.push((name.to_owned(), big_file(root, big, name)?));
    }
    for (big, name) in AEMS_BANKS {
        files.banks.push((name.to_owned(), Arc::from(big_file(root, big, name)?)));
    }
    for (big, name) in DMO_BANKS {
        files.banks.push((name.to_owned(), Arc::from(big_file(root, big, name)?)));
    }
    for (big, name) in MOMENTS_BANKS {
        files.banks.push((name.to_owned(), Arc::from(big_file(root, big, name)?)));
    }
    for (big, name) in VEHICLE_BANKS.iter().chain(&CROWD_BANKS) {
        files.asset_files.push((name.to_string(), Arc::from(big_file(root, big, name)?)));
    }
    Ok(files)
}

/// The `SFXObj_Moving` swap banks by bare name: `data\audio\splicerdmo.big` holds only the
/// `aud_dynamicobjectrandom` splice banks, so its `big|Moveable_*` paths never resolve.
fn dmo_swap_files(mounts: &MountSet) -> Result<DmoSwapFiles, String> {
    let root = game_root(mounts)?;
    let big = MountedBig::open(root.join("data/audio/splicerdmo.big"), false).map_err(|e| format!("splicerdmo.big: {e}"))?;
    let mut files = DmoSwapFiles::default();
    for entry in &big.archive().entries {
        if let BigEntryKey::Name(name) = &entry.key {
            let bytes = big.read_entry(entry).map_err(|e| format!("{name}: {e}"))?;
            files.0.insert(name.to_string(), Arc::from(bytes));
        }
    }
    Ok(files)
}

/// `AudioSystem::EnterWorld` with the front-end banks of `data\audio\splicerfe.big`, the skater
/// and collision splicer banks, `data\audio\MixMapSK8.mxb`, `skater.vlt` and its `fe` collections.
fn load_system(mounts: &MountSet) -> Result<(AudioSystem, HashMap<String, u64>), String> {
    let root = game_root(mounts)?;
    let banks = MountedBig::open(root.join("data/audio/splicerfe.big"), false).map_err(|e| e.to_string())?;
    let fe_bank: Arc<[u8]> = banks.read_path("Sk82_FE.bnk").map_err(|e| format!("Sk82_FE.bnk: {e}"))?.into();
    let menu_bank: Arc<[u8]> = banks.read_path("sk8_menu.bnk").map_err(|e| format!("sk8_menu.bnk: {e}"))?.into();
    let mut world_banks = splicer_banks(&root, "splicerskateboard.big", &SKATEBOARD_BANKS)?;
    world_banks.extend(splicer_banks(&root, "splicercollision.big", &COLLISION_BANKS)?);
    let mix_map = std::fs::read(skate2_formats::host_path::on_disk(&root.join("data/audio/MixMapSK8.mxb"))).map_err(|e| format!("MixMapSK8.mxb: {e}"))?;
    let grains = grain_files(&root)?;
    let aems = aems_files(&root)?;
    // `skater.vlt` and the challenge Vaults the contests' audio reads `challenges` from.
    let db = AttribDb::load_career(mounts).map_err(|e| e.to_string())?;
    let (sounds, globals) = fe_sounds(&db);
    let mut system = AudioSystem::new();
    system.set_hom_banks(hom_banks(&root)?);
    // `rw::core::filesys` for the streamed ambience beds and `ambienceresident.big`.
    system.set_stream_files(Arc::new(skate2_audio::rwac::stream_pool::GameStreamFiles::new(root)));
    system.enter_world(Vault::new(Some(Box::new(VaultSource(db)))), fe_bank, menu_bank, world_banks, mix_map, sounds, grains, aems)?;
    Ok((system, globals))
}

/// The core's `PhysOut` copy as the audio presentation reads it.
fn phys_out_view(p: &SkaterAudioPhysOut) -> PhysOutView {
    let info = &p.averaged_skeleton_collision_info;
    PhysOutView {
        centre_of_mass: p.centre_of_mass,
        centre_of_mass_velocity: p.centre_of_mass_velocity,
        hips_mat_y: p.hips_mat_y,
        foot_l_vel_deck_space: p.foot_l_vel_deck_space,
        foot_r_vel_deck_space: p.foot_r_vel_deck_space,
        foot_left_vel: p.foot_left_vel,
        foot_right_vel: p.foot_right_vel,
        torso_angular_vel: p.torso_angular_vel,
        right_foot_angular_vel: p.right_foot_angular_vel,
        left_foot_angular_vel: p.left_foot_angular_vel,
        absorption_compression: p.absorption_compression,
        average_limb_speed: p.average_limb_speed,
        is_wipeout_over: p.is_wipeout_over,
        is_left_foot_on_deck: p.is_left_foot_on_deck,
        is_right_foot_on_deck: p.is_right_foot_on_deck,
        is_grabbing_left: p.is_grabbing_left,
        is_grabbing_right: p.is_grabbing_right,
        effective_transform_y: p.effective_transform_y,
        deck_nose_dir: p.deck_nose_dir,
        deck_position: p.deck_position,
        wheel_pos: p.wheel_pos,
        deck_velocity: p.deck_velocity,
        skateboard_surface_speed: p.skateboard_surface_speed,
        deck_tilt: p.deck_tilt,
        wheel_hardness: p.wheel_hardness,
        time_in_known_air: p.time_in_known_air,
        traj_collision_time: p.traj_collision_time,
        jump_height: p.jump_height,
        foot_plant_surface_id: p.foot_plant_surface_id,
        hand_plant_surface_id: p.hand_plant_surface_id,
        is_skateboard_airborne: p.is_skateboard_airborne,
        is_starting_to_jump: p.is_starting_to_jump,
        is_foot_planting: p.is_foot_planting,
        is_left_foot_planting: p.is_left_foot_planting,
        is_right_foot_planting: p.is_right_foot_planting,
        hand_plant_grabbing: p.hand_plant_grabbing,
        num_wheels_in_contact: p.num_wheels_in_contact,
        front_truck_audio_surface_type: p.front_truck_audio_surface_type as u32,
        back_truck_audio_surface_type: p.back_truck_audio_surface_type as u32,
        deck_audio_surface_type: p.deck_audio_surface_type as u32,
        deck_slide_speed: p.deck_slide_speed,
        wheel_audio_surface_type: p.wheel_audio_surface_type.map(|surface| surface as u32),
        wheel_surface_pattern: p.wheel_surface_pattern.map(|pattern| pattern as u32),
        is_front_truck_in_contact: p.is_front_truck_in_contact,
        is_back_truck_in_contact: p.is_back_truck_in_contact,
        is_deck_in_contact: p.is_deck_in_contact,
        physics_state: p.physics_state,
        is_foot_braking: p.is_foot_braking,
        is_manual_braking: p.is_manual_braking,
        is_physically_pushing: p.is_physically_pushing,
        is_generally_pushing: p.is_generally_pushing,
        is_generally_pushing_right_foot: p.is_generally_pushing_right_foot,
        is_wiping_out: p.is_wiping_out,
        is_manual: p.is_manual,
        is_reverting: p.is_reverting,
        turn: p.turn,
        ground_up_vector: p.ground_up_vector,
        deck_local_xyz_angular_speed: p.deck_local_xyz_angular_speed,
        wheel_slide: p.wheel_slide,
        jump_fraction: p.jump_fraction,
        grind_type: p.grind_type,
        grind_impact_speed: p.grind_impact_speed,
        averaged_deck_contact_force: p.averaged_deck_contact_force,
        grind_audio_surface_type: p.grind_audio_surface_type as u32,
        off_board_jump_landing: p.off_board_jump_landing,
        hippy_jump_strength: p.hippy_jump_strength,
        wheel_impact_strength: p.wheel_impact_strength,
        wheel_landed: p.wheel_landed,
        averaged_skeleton_collision_info: CollisionInfo {
            contact_force: info.contact_force,
            slide_speed: info.slide_speed,
            audio_surface_type: info.audio_surface_type.map(|surface| surface as u32),
            is_groin_contact: info.is_groin_contact,
            is_face_contact: info.is_face_contact,
            vehicle_impact: info.vehicle_impact,
            skater_impact: info.skater_impact,
            pedestrian_impact: info.pedestrian_impact,
        },
        is_grinding: p.is_grinding,
        is_on_stairs: p.is_on_stairs,
        is_walking_up: p.is_walking_up,
        is_walking_down: p.is_walking_down,
        push_mongo: p.push_mongo,
        line_score: p.line_score,
        wipedout_trick_score: p.wipedout_trick_score,
        banked_point_change_this_frame: p.banked_point_change_this_frame,
        audio_banked_point_change: p.audio_banked_point_change,
        raw_scorable_id: p.raw_scorable_id,
        left_foot_surface_id: p.left_foot_surface_id,
        right_foot_surface_id: p.right_foot_surface_id,
        is_grabbing_object: p.is_grabbing_object,
        left_foot_down: p.left_foot_down,
        right_foot_down: p.right_foot_down,
        is_unmounting: p.is_unmounting,
        is_air_dismount: p.is_air_dismount,
        is_holding_skateboard: p.is_holding_skateboard,
        is_jumping: p.is_jumping,
        is_starting_standing_jump: p.is_starting_standing_jump,
    }
}

/// What the session publishes for the audio each tick: the completed frame count, the game
/// camera's Z axis and position, and the skaters' `PhysOut`s when the frame completed.
pub(crate) struct AudioWorldInput {
    pub(crate) security: skate2_core::SecurityPresentation,
    pub(crate) game_speed: f32,
    pub(crate) is_rewinding: bool,
    pub(crate) frame: u32,
    pub(crate) camera_at: [f32; 3],
    pub(crate) camera_position: [f32; 3],
    pub(crate) skaters: Option<Vec<AudioSkater>>,
    /// The front end's simulation: its AI skaters replace the ones retained from the last
    /// completed gameplay frame, and `CSTATEMGR_Player::UpdateParams` 0x82556c60 detaches its
    /// states as the local skater's index leaves.
    pub(crate) front_end: bool,
    /// `GlobalData.reverbKey` (`SelectedLiveSession::audio_reverb_key`); no additional world is
    /// loaded, so `SetAdditionalLocationKey` 0x824f1b30 leaves `GetReverbKey` on it.
    pub(crate) reverb_key: u64,
    /// `GlobalData.ambienceKey` (`SelectedLiveSession::audio_ambience_key`), which `GetAmbienceKey`
    /// 0x824f1cb8 returns for the same reason.
    pub(crate) ambience_key: u64,
    /// `GlobalData.emitterKey` (`SelectedLiveSession::audio_emitter_key`): the location's random
    /// emitters, which `AudioSystem::GetEmitterKey` 0x824f1cf0 returns outside races.
    pub(crate) emitter_key: u64,
    /// The out-of-boundary amount `VDWriter::WriteData` recorded for the frame, if any.
    pub(crate) out_of_boundary_amount: Option<f32>,
    /// The `cMsgFadeAmount` it recorded (`SelectedLiveSession::audio_fade_amount`).
    pub(crate) fade_amount: Option<f32>,
    /// `AudioPresentation::DoSimSubmit`'s pedestrians (`SelectedLiveSession::audio_pedestrians`).
    pub(crate) pedestrians: Vec<skate2_core::AudioPedestrian>,
    /// `AudioPresentation::DoSimSubmit`'s cars (`SelectedLiveSession::audio_vehicles`).
    pub(crate) vehicles: Vec<skate2_core::AudioVehicle>,
    /// `AudioPresentation::DoSimSubmit`'s dynamic objects
    /// (`SelectedLiveSession::audio_dynamic_objects`).
    pub(crate) dynamic_objects: Vec<skate2_core::AudioDynamicObject>,
    /// `GlobalData.localRaceRanking` (`SelectedLiveSession::audio_local_race_ranking`).
    pub(crate) local_race_ranking: i32,
}

/// `AudioPresentation::DoSimSubmit`'s `PedestrianData` of a pedestrian the core found.
fn pedestrian_data(pedestrian: &skate2_core::AudioPedestrian) -> PedestrianData {
    let out = PedStateOut {
        state: pedestrian.state,
        length: pedestrian.length,
        topic: pedestrian.topic,
        target_greet_character: pedestrian.target_greet_character,
        flags: pedestrian.flags,
    };
    PedestrianData::from_state_out(pedestrian.id, out, pedestrian.audio_character, pedestrian.distance, pedestrian.local_to_world)
}

/// `AudioPresentation::DoSimSubmit`'s `VehicleData` of a car the core found.
fn vehicle_data(vehicle: &skate2_core::AudioVehicle) -> VehicleData {
    VehicleData {
        acceleration: vehicle.acceleration,
        speed: vehicle.speed,
        distance_to_camera: vehicle.distance,
        vehicle_id: vehicle.id,
        horn_state: vehicle.horn_state,
        is_skidding: vehicle.skidding,
        engine_key: vehicle.engine_key,
        matrix: vehicle.matrix,
    }
}

/// `AudioPresentation::DoSimSubmit`'s `DynamicData` of a dynamic object the core found, with
/// `AudioSystem::GetAudioMaterial` of its strongest contact's surface. Without a contact the
/// record's material stays unwritten (0 here); `SFXObj_Dynamic` reads it only for an impact.
fn dynamic_data(object: &skate2_core::AudioDynamicObject) -> DynamicData {
    DynamicData {
        object_id: object.id,
        object_vel: object.velocity,
        grabbed_object: object.grabbed,
        dynamic_type: object.characteristic_key,
        angular_vel: object.angular_velocity,
        collision_force: object.contact.map_or(0.0, |(force, _)| force),
        on_ground: object.contact.is_some(),
        collision_material: object.contact.map_or(0, |(_, surface)| audio_material(surface)),
        on_side: object.on_side,
        bounds_center: Some(object.bounds_center),
    }
}

fn lock(system: &Mutex<AudioSystem>) -> MutexGuard<'_, AudioSystem> {
    system.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The audio clock the scenes read (`AudioCore::GetSystemTime` with `kNISUsesTheAudioClock`).
/// The console's RWAC renders one 256-frame block at a time as the audio hardware takes it, so its
/// time follows the hardware steadily; the device here takes many frames per callback, which the
/// core renders at once. The clock is the time of the frames handed to the device before its last
/// callback, plus the wall time since that callback (at most its length), which follows the device
/// as steadily.
#[derive(Default)]
struct DeviceClock {
    state: Mutex<DeviceClockState>,
}

#[derive(Default)]
struct DeviceClockState {
    /// Seconds of frames handed before the last callback, and that callback's.
    handed: f64,
    callback: f64,
    at: Option<Instant>,
}

impl DeviceClock {
    fn lock(&self) -> MutexGuard<'_, DeviceClockState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A callback handing the device `seconds` of frames.
    fn callback(&self, seconds: f64) {
        let mut state = self.lock();
        state.handed += state.callback;
        state.callback = seconds;
        state.at = Some(Instant::now());
    }

    fn now(&self) -> f64 {
        let state = self.lock();
        state.handed + state.at.map_or(0.0, |at| at.elapsed().as_secs_f64().min(state.callback))
    }
}

/// Converts the 48 kHz stereo mix to the device's rate and channel count.
struct Output {
    system: Arc<Mutex<AudioSystem>>,
    exporting:Arc<AtomicBool>,
    pending: VecDeque<f32>,
    block: Vec<f32>,
    step: f64,
    position: f64,
    previous: [f32; 2],
    current: [f32; 2],
    clock: Arc<DeviceClock>,
    device_rate: f64,
}

impl Output {
    fn next_mix_frame(&mut self) -> [f32; 2] {
        if self.pending.len() < 2 {
            self.block.clear();
            let mut system=lock(&self.system);
            if self.exporting.load(Ordering::Acquire){return [0.;2];}
            system.render_stereo(&mut self.block);
            self.pending.extend(self.block.iter().copied());
        }
        [self.pending.pop_front().unwrap_or(0.0), self.pending.pop_front().unwrap_or(0.0)]
    }

    fn next_frame(&mut self) -> [f32; 2] {
        if self.step == 1.0 {
            return self.next_mix_frame();
        }
        while self.position >= 1.0 {
            self.position -= 1.0;
            self.previous = self.current;
            self.current = self.next_mix_frame();
        }
        let t = self.position as f32;
        self.position += self.step;
        [self.previous[0] + (self.current[0] - self.previous[0]) * t, self.previous[1] + (self.current[1] - self.previous[1]) * t]
    }

    fn fill<T: cpal::SizedSample + cpal::FromSample<f32>>(&mut self, data: &mut [T], channels: usize) {
        self.clock.callback((data.len() / channels.max(1)) as f64 / self.device_rate);
        if self.exporting.load(Ordering::Acquire) {
            self.pending.clear();self.previous=[0.;2];self.current=[0.;2];self.position=1.;
            data.fill(T::from_sample(0.));return;
        }
        for frame in data.chunks_mut(channels) {
            let [left, right] = self.next_frame();
            for (channel, sample) in frame.iter_mut().enumerate() {
                let value = match (channels, channel) {
                    (1, _) => (left + right) * 0.5,
                    (_, 0) => left,
                    (_, 1) => right,
                    _ => 0.0,
                };
                *sample = T::from_sample(value.clamp(-1.0, 1.0));
            }
        }
    }
}

fn build_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(device: &cpal::Device, config: cpal::StreamConfig, mut output: Output) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels);
    device
        .build_output_stream(config, move |data: &mut [T], _: &cpal::OutputCallbackInfo| output.fill(data, channels), |error| eprintln!("audio output: {error}"), None)
        .map_err(|e| e.to_string())
}

/// `SKATE2_AUDIO_DUMP`: instead of the device, the mix rendered in step with the simulation into
/// a file of 48 kHz stereo little-endian f32 frames.
struct Dump {
    file: std::io::BufWriter<std::fs::File>,
    simulated: f64,
    mixed: f64,
    block: Vec<f32>,
}

enum AudioOutput {
    Device { _stream: cpal::Stream },
    Dump(Dump),
}

pub(crate) struct GameAudio {
    system: Arc<Mutex<AudioSystem>>,
    exporting:Arc<AtomicBool>,
    export_pending:VecDeque<f32>,
    globals: HashMap<String, u64>,
    /// The last skaters the session published: `AudioPresentation` keeps them on frames that
    /// publish none (a paused simulation).
    skaters: Vec<AudioSkater>,
    dmo_swap_files: Arc<DmoSwapFiles>,
    output: AudioOutput,
    /// The device's clock; none for the dump, whose mix the update renders.
    clock: Option<Arc<DeviceClock>>,
}

/// The game's `IAudioInterface` over the audio system: the scenes' speech, posts, music and
/// playback notices, the challenges' mission speech and its messages, and the audio clock (the
/// device's, else the core's).
struct GameAudioInterface {
    system: Arc<Mutex<AudioSystem>>,
    clock: Option<Arc<DeviceClock>>,
}

impl skate2_core::nis::audio::AudioInterface for GameAudioInterface {
    fn system_time(&self) -> f64 {
        self.clock.as_ref().map_or_else(|| lock(&self.system).system_time(), |clock| clock.now())
    }

    fn queue_nis_post(&self, id: i32) {
        lock(&self.system).queue_nis_post(id);
    }

    fn play_nis_post(&self) {
        lock(&self.system).play_nis_post();
    }

    fn is_post_queued(&self) -> bool {
        lock(&self.system).is_post_queued()
    }

    fn queue_nis_speech(&self, character: i32, id: i32) {
        trace_speech(format_args!("queue NIS speech {character} {id}"));
        lock(&self.system).queue_nis_speech(character, id);
    }

    fn play_nis_speech(&self, character: i32, id: i32) {
        trace_speech(format_args!("play NIS speech {character} {id}"));
        lock(&self.system).play_nis_speech(character, id);
    }

    fn is_speech_queued(&self, character: i32, id: i32) -> bool {
        lock(&self.system).is_speech_queued(character, id)
    }

    fn notify_nis_playback_start(&self, scene_type: i32, special_flags: i32, is_replay_nis: bool, scene_length: f32) {
        lock(&self.system).notify_nis_playback_start(scene_type, special_flags, is_replay_nis, scene_length);
    }

    fn notify_nis_playback_finish(&self) {
        lock(&self.system).notify_nis_playback_finish();
    }

    fn leaderboard_overall(&self, character: i32) {
        trace_speech(format_args!("leaderboard overall {character}"));
        lock(&self.system).leaderboard_overall(character);
    }
    fn play_mission_speech(&self, character: i32, id: i32, param: i32) {
        trace_speech(format_args!("play mission speech {character} {id} {param}"));
        lock(&self.system).play_mission_speech(character, id, param);
    }

    fn play_camera_man_csis_event(&self, function: &'static str) {
        trace_speech(format_args!("play camera man CSIS event {function}"));
        lock(&self.system).play_camera_man_csis_event(function);
    }

    fn trigger_mission_music(&self, trigger: i32, challenge: u64, challenge_type: i32, race_tuning: u64) {
        trace_speech(format_args!("trigger mission music {trigger} for {challenge:#x} (type {challenge_type}, race tuning {race_tuning:#x})"));
        lock(&self.system).trigger_mission_music(trigger, skate2_audio::system::MissionMusic { challenge, challenge_type, race_tuning });
    }

    fn play_mission_sfx(&self, sfx: i32, challenge: u64, challenge_type: i32, race_tuning: u64) {
        trace_speech(format_args!("play mission SFX {sfx} for {challenge:#x} (type {challenge_type})"));
        lock(&self.system).play_mission_sfx(sfx, skate2_audio::system::MissionMusic { challenge, challenge_type, race_tuning });
    }

    fn cache_crowd_pod_infos(&self, pods: &[([f32; 4], i32)]) {
        let pods: Vec<_> = pods.iter().map(|&(pod_pos, num_spectators)| skate2_audio::world::crowd::CrowdPodInfo { pod_pos, num_spectators }).collect();
        lock(&self.system).cache_crowd_pod_infos(&pods);
    }

    fn restart_challenge(&self, retry: bool) {
        trace_speech(format_args!("restart challenge (retry {retry})"));
        lock(&self.system).restart_challenge(retry);
    }

    fn quit_challenge(&self) {
        trace_speech(format_args!("quit challenge"));
        lock(&self.system).quit_challenge();
    }

    fn take_speech_messages(&self) -> Vec<skate2_core::nis::audio::SpeechMessage> {
        use skate2_audio::speech::SpeechMessage as Audio;
        use skate2_core::nis::audio::SpeechMessage as Game;
        lock(&self.system)
            .take_speech_messages()
            .into_iter()
            .inspect(|message| trace_speech(format_args!("{message:?}")))
            .map(|message| match message {
                Audio::Started { character, id } => Game::Started { character, id },
                Audio::Ended { character, id } => Game::Ended { character, id },
                Audio::CallStarted { character, id } => Game::CallStarted { character, id },
                Audio::CallEnded { character, id } => Game::CallEnded { character, id },
            })
            .collect()
    }
}

impl GameAudio {
    /// `AudioSystem::EnterWorld`, then the output stream.
    pub(crate) fn start(mounts: &MountSet) -> Result<Self, String> {
        let (system, globals) = load_system(mounts)?;
        let system = Arc::new(Mutex::new(system));
        let dmo_swap_files = Arc::new(dmo_swap_files(mounts)?);
        if let Some(path) = std::env::var_os("SKATE2_AUDIO_DUMP") {
            let file = std::fs::File::create(&path).map_err(|e| format!("SKATE2_AUDIO_DUMP: {e}"))?;
            eprintln!("audio: {} fe sounds, mix dumped to {}", globals.len(), PathBuf::from(&path).display());
            let dump = Dump { file: std::io::BufWriter::new(file), simulated: 0.0, mixed: 0.0, block: Vec::new() };
            return Ok(Self { system, globals, skaters: Vec::new(), dmo_swap_files, output: AudioOutput::Dump(dump), clock: None,
                exporting:Arc::new(AtomicBool::new(false)),export_pending:VecDeque::new() });
        }

        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config = supported.config();
        let step = f64::from(MIX_RATE) / f64::from(config.sample_rate);
        let clock = Arc::new(DeviceClock::default());
        let exporting=Arc::new(AtomicBool::new(false));
        let output = Output {
            system: Arc::clone(&system),
            exporting:Arc::clone(&exporting),
            pending: VecDeque::new(),
            block: Vec::new(),
            step: if config.sample_rate == MIX_RATE { 1.0 } else { step },
            position: 1.0,
            previous: [0.0; 2],
            current: [0.0; 2],
            clock: Arc::clone(&clock),
            device_rate: f64::from(config.sample_rate),
        };
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32>(&device, config, output)?,
            cpal::SampleFormat::I16 => build_stream::<i16>(&device, config, output)?,
            cpal::SampleFormat::U16 => build_stream::<u16>(&device, config, output)?,
            // Linux's ALSA devices may offer only these.
            cpal::SampleFormat::I8 => build_stream::<i8>(&device, config, output)?,
            cpal::SampleFormat::I24 => build_stream::<cpal::I24>(&device, config, output)?,
            cpal::SampleFormat::I32 => build_stream::<i32>(&device, config, output)?,
            cpal::SampleFormat::I64 => build_stream::<i64>(&device, config, output)?,
            cpal::SampleFormat::U8 => build_stream::<u8>(&device, config, output)?,
            cpal::SampleFormat::U24 => build_stream::<cpal::U24>(&device, config, output)?,
            cpal::SampleFormat::U32 => build_stream::<u32>(&device, config, output)?,
            cpal::SampleFormat::U64 => build_stream::<u64>(&device, config, output)?,
            cpal::SampleFormat::F64 => build_stream::<f64>(&device, config, output)?,
            format => return Err(format!("unsupported output sample format {format:?}")),
        };
        stream.play().map_err(|e| e.to_string())?;
        eprintln!(
            "audio: {} fe sounds, output {} Hz x{} ({:?})",
            globals.len(),
            config.sample_rate,
            config.channels,
            supported.sample_format()
        );
        Ok(Self { system, globals, skaters: Vec::new(), dmo_swap_files, output: AudioOutput::Device { _stream: stream }, clock: Some(clock),exporting,export_pending:VecDeque::new() })
    }

    /// The audio system as the career's scenes and challenge audio manager call it
    /// (`IAudioInterface`).
    pub(crate) fn audio_interface(&self) -> Arc<dyn skate2_core::nis::audio::AudioInterface> {
        Arc::new(GameAudioInterface { system: Arc::clone(&self.system), clock: self.clock.clone() })
    }

    /// `IAudioInterface::PlayNISSfx`.
    pub(crate) fn play_nis_sfx(&self, id: i32) {
        lock(&self.system).play_nis_sfx(id);
    }

    /// `AudioSystem::PauseForMovie`.
    pub(crate) fn pause_for_movie(&self) {
        lock(&self.system).pause_for_movie();
    }

    /// `AudioSystem::ResumeAfterMovie`.
    pub(crate) fn resume_after_movie(&self) {
        lock(&self.system).resume_after_movie();
    }

    /// `AudioSystem::GetMovieVolume`.
    pub(crate) fn movie_volume(&self) -> f32 {
        lock(&self.system).movie_volume()
    }

    /// A movie's `AudioRenderer_Rwac` (`Init`) over the decoded sound its reader fills.
    pub(crate) fn movie_init(&self, sample_rate: u32, channels: u8, sound: skate2_audio::movie::MovieSound) -> bool {
        lock(&self.system).movie_init(sample_rate, channels, sound)
    }

    /// The movie renderer's `Begin`.
    pub(crate) fn movie_begin(&self) {
        lock(&self.system).movie_begin();
    }

    /// The movie renderer's `SetVolume`.
    pub(crate) fn movie_set_volume(&self, volume: f32) {
        lock(&self.system).movie_set_volume(volume);
    }

    /// The movie renderer's `GetRenderingPosition` in milliseconds.
    pub(crate) fn movie_position(&self) -> Option<f32> {
        lock(&self.system).movie_rendering_position()
    }

    /// The movie renderer's `Stop` and destruction.
    pub(crate) fn movie_shutdown(&self) {
        lock(&self.system).movie_shutdown();
    }

    /// One front-end sound request. The movies' `Audio.PlaySound(_global.<global>)`
    /// (`AudioCmdTranslator::sMethod_PlaySound` 0x8261dfb8) is a one-shot play; only `FE_SOUND_*`
    /// globals exist, and other names are no-ops natively.
    pub(crate) fn request(&self, request: FeSoundRequest) {
        match request {
            FeSoundRequest::PlayKey(key)=>{lock(&self.system).global_fe_play_sound(key,false);return;},
            FeSoundRequest::LoopKey(key)=>{lock(&self.system).global_fe_play_sound(key,true);return;},
            FeSoundRequest::StopKey(key)=>{lock(&self.system).global_fe_stop_sound(key);return;},
            _=>{}
        }
        let (verb, global) = match request {
            FeSoundRequest::PlayKey(_)|FeSoundRequest::LoopKey(_)|FeSoundRequest::StopKey(_)=>unreachable!(),
            FeSoundRequest::Play(global) => ("play", global),
            FeSoundRequest::Loop(global) => ("loop", global),
            FeSoundRequest::Stop(global) => ("stop", global),
        };
        let key = self.globals.get(global).copied();
        if let Some(elapsed) = skate2_audio::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} audio {verb} {global}{}", if key.is_some() { "" } else { " (no fe collection)" });
        }
        if let Some(key) = key {
            let mut system = lock(&self.system);
            match request {
                FeSoundRequest::PlayKey(_)|FeSoundRequest::LoopKey(_)|FeSoundRequest::StopKey(_)=>unreachable!(),
                FeSoundRequest::Play(_) => system.global_fe_play_sound(key, false),
                FeSoundRequest::Loop(_) => system.global_fe_play_sound(key, true),
                FeSoundRequest::Stop(_) => system.global_fe_stop_sound(key),
            }
        }
    }

    /// `GetMusicPlayerSongIDFromIndex` for each music player song.
    pub(crate) fn music_player_song_ids(&self) -> Vec<u32> {
        lock(&self.system).music_player_song_ids()
    }

    /// The profile's current playlist, then the front end's `MusicSystem` calls in order.
    pub(crate) fn music_player(&self, playlist: (&[u32], i32), requests: Vec<MusicRequest>) {
        let mut system = lock(&self.system);
        system.set_music_player_playlist(playlist.0, playlist.1);
        for request in requests {
            match request {
                MusicRequest::SetDefaultMode(mode) => system.set_music_default_mode(mode),
                MusicRequest::IPodTrigger { trigger, song } => system.ipod_trigger(trigger, song),
            }
        }
    }

    /// `GetCurrentMusicPlayerSongID` and the playlist index the update left.
    pub(crate) fn music_player_song(&self) -> (u32, i32) {
        let system = lock(&self.system);
        (system.current_music_player_song_id(), system.music_player_song_index())
    }

    /// The song the music system's `UpdateChyron` displayed since the last call.
    pub(crate) fn take_chyron(&self) -> Option<u32> {
        lock(&self.system).take_chyron()
    }

    /// One game frame of `AudioSystem::Update`: the world (camera, the local skater's
    /// `FillSkaterData` as player 0) when the session has one, else the front-end branch.
    pub(crate) fn update(&mut self, dt: f32, state: GameAudioState, world: Option<AudioWorldInput>) {
        if let Some(world) = world.as_ref() {
            if world.front_end {
                self.skaters = world.skaters.clone().unwrap_or_default();
            } else if let Some(skaters) = world.skaters.clone() {
                self.skaters = skaters;
            }
        }
        let mut system = lock(&self.system);
        system.state = state;
        let world = world.map(|world| {
            let [x, y, z] = world.camera_at;
            let [px, py, pz] = world.camera_position;
            let skaters = self
                .skaters
                .iter()
                .map(|skater| {
                    system.fill_skater_data(
                        &phys_out_view(&skater.phys_out),
                        skater.local,
                        i32::from_ne_bytes(skater.player_index.to_ne_bytes()),
                        skater.can_see_player,
                        skater.ai_audio.as_ref(),
                    )
                })
                .collect();
            // DoSimSubmit keeps the last local skater's `PhysOut` for the global bail and
            // multiplier data.
            let local = self.skaters.iter().rev().find(|skater| skater.local).map(|skater| &skater.phys_out);
            WorldAudioState {
                chase: (world.security.chasers, world.security.flags),
                bodyguard: world.security.bodyguard,
                frame: world.frame,
                sim_tick: world.frame,
                camera: CameraMatrix { at: [x, y, z, 0.0], position: [px, py, pz, 1.0] },
                player: PlayerFrame { skaters, game_speed: world.game_speed, is_rewinding: world.is_rewinding },
                hom_bail: local.is_some_and(|s| s.hom_bail_in_progress && s.hom_effect_enabled),
                // `VDReader::ReadData` 0x827fdd78: the bail record `VDWriter::_PrepareBailData`
                // 0x827fcd40 writes unless the wipeout duration is at most 0 (a NaN records).
                bail: local.is_some_and(|skater| skater.overall_wipeout_duration > 0.0 || skater.overall_wipeout_duration.is_nan()),
                bail_severity: local.map_or(0.0, |skater| skater.contact_force_total_during_wipeout),
                out_of_bounds: world.out_of_boundary_amount.is_some(),
                out_of_bounds_amount: world.out_of_boundary_amount.unwrap_or(0.0),
                fade_info: world.fade_amount,
                times: multiplier_flags(local.map(|skater| skater.momentum_meter_multiplier)),
                local_race_ranking: world.local_race_ranking,
                reverb_key: world.reverb_key,
                ambience_key: world.ambience_key,
                emitter_key: world.emitter_key,
                pedestrians: world.pedestrians.iter().map(pedestrian_data).collect(),
                vehicles: world.vehicles.iter().map(vehicle_data).collect(),
                dynamic_objects: world.dynamic_objects.iter().map(dynamic_data).collect(),
                dmo_swap_files: Arc::clone(&self.dmo_swap_files),
                ..WorldAudioState::default()
            }
        });
        if std::env::var_os("SKATE2_TRACE_REPLAY_AUDIO").is_some()
            && let Some(world) = &world
        {
            let forces: f32 = self.skaters.iter().filter(|s| s.local)
                .flat_map(|s| s.phys_out.averaged_skeleton_collision_info.contact_force).sum();
            eprintln!("audio frame={} replay={} speed={} rewind={} bail={} hom={} contacts={forces} skaters={}",
                world.frame, system.state.presentation.is_some_and(|p|p.0), world.player.game_speed,
                world.player.is_rewinding, world.bail, world.hom_bail, world.player.skaters.len());
        }
        system.world = world;
        system.update(dt);
        if let AudioOutput::Dump(dump) = &mut self.output && !self.exporting.load(Ordering::Acquire) {
            use std::io::Write;
            dump.simulated += f64::from(dt);
            while dump.mixed < dump.simulated {
                dump.block.clear();
                system.render_stereo(&mut dump.block);
                let bytes: Vec<u8> = dump.block.iter().flat_map(|x| x.to_le_bytes()).collect();
                if dump.file.write_all(&bytes).is_err() {
                    break;
                }
                dump.mixed += 256.0 / f64::from(MIX_RATE);
            }
            let _ = dump.file.flush();
        }
    }

    pub(crate) fn exporting(&self)->bool {self.exporting.load(Ordering::Acquire)}
    pub(crate) fn begin_export(&mut self) {
        self.exporting.store(true,Ordering::Release);
        self.export_pending.clear();
        let mut system=lock(&self.system);
        // The confirmation click and frontend loops do not belong to the clip.
        for key in self.globals.values() {system.global_fe_stop_sound(*key);}
    }
    pub(crate) fn end_export(&mut self) {
        self.export_pending.clear();self.exporting.store(false,Ordering::Release);
    }
    /// Exactly 800 stereo samples per 60 Hz video frame, using the ordinary
    /// recorded skater/world audio and RWAC mixer, independent of device rate
    /// and the time spent rendering or streaming.
    pub(crate) fn export_frame(&mut self,state:GameAudioState,world:Option<AudioWorldInput>)->Vec<f32> {
        self.update(1./60.,state,world);
        let samples=crate::media_encoder::AUDIO_FRAMES*2;
        let mut system=lock(&self.system);
        while self.export_pending.len()<samples {
            let mut block=Vec::new();system.render_stereo(&mut block);
            self.export_pending.extend(block);
        }
        self.export_pending.drain(..samples).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::mount::SKATE2_ARCHIVES;

    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn pause_menu_sound_renders() {
        let game = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter().take(4) {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        let (mut system, globals) = load_system(&mounts).unwrap();
        let unresolved: Vec<_> = globals.iter().filter(|(_, key)| system.fe_sound(**key).is_none()).map(|(name, _)| name.clone()).collect();
        assert!(unresolved.is_empty(), "globals without an fe collection: {unresolved:?}");
        // `pause_menu` itself has Volume 0.0 in skater.vlt; the press sound is the audible one.
        let key = globals["FE_SOUND_PAUSE_MENU_PRESS"];
        let sound = system.fe_sound(key).unwrap().clone();
        eprintln!("FE_SOUND_PAUSE_MENU_PRESS = {sound:?}");
        system.state = GameAudioState { hud_state: skate2_audio::system::hud_state::FREE_SKATE, ..GameAudioState::default() };
        system.global_fe_play_sound(key, false);
        let mut stereo = Vec::new();
        let mut mixed = 0.0f64;
        for tick in 0..180 {
            system.update(1.0 / 60.0);
            while mixed < f64::from(tick + 1) / 60.0 {
                system.render_stereo(&mut stereo);
                mixed += 256.0 / 48000.0;
            }
        }
        let peak = stereo.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        eprintln!("rendered {} frames, peak {peak}", stereo.len() / 2);
        if let Some(path) = std::env::var_os("SKATE2_AUDIO_WAV") {
            write_wav(&path, &stereo);
        }
        assert!(peak > 0.0);
    }

    /// `Goto_m_1a_01_goto_slappys_outro_1`'s start (`ScenePlayer::AudioQueueForStart`): the
    /// scene's post and the cameraman's line, both 5008, buffer in the 'Spch' stream pool of a
    /// system made as the game makes it.
    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn a_scenes_post_and_line_buffer() {
        let game = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter().take(4) {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        let (mut system, _) = load_system(&mounts).unwrap();
        system.queue_nis_post(5008);
        system.queue_nis_speech(40, 5008);
        let mut stereo = Vec::new();
        let mut ticks = 0;
        while !(system.is_post_queued() && system.is_speech_queued(40, 5008)) {
            system.update(1.0 / 60.0);
            stereo.clear();
            system.render_stereo(&mut stereo);
            std::thread::sleep(std::time::Duration::from_millis(1));
            ticks += 1;
            assert!(ticks < 5000, "never buffered");
        }
        eprintln!("buffered after {ticks} ticks");
    }

    fn write_wav(path: &std::ffi::OsStr, stereo: &[f32]) {
        let data: Vec<u8> = stereo.iter().flat_map(|x| ((x.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()).collect();
        let mut wav = Vec::new();
        for chunk in [&b"RIFF"[..], &(36 + data.len() as u32).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes(), &1u16.to_le_bytes(), &2u16.to_le_bytes(),
            &48000u32.to_le_bytes(), &(48000u32 * 4).to_le_bytes(), &4u16.to_le_bytes(), &16u16.to_le_bytes(), b"data", &(data.len() as u32).to_le_bytes(), &data] {
            wav.extend_from_slice(chunk);
        }
        std::fs::write(path, wav).unwrap();
    }

    /// A local skater pushes for a second, rolls, is airborne for half a second and lands on all
    /// four wheels: the PLAYER state attaches, `SFXObj_Contacts` plays the push and the landing and
    /// `SFXObj_SkateBoard` rolls on smooth asphalt (the unmapped wheel material) through its grains.
    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn skating_sounds_render() {
        let game = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter().take(4) {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        let (mut system, _) = load_system(&mounts).unwrap();
        let mut stereo = Vec::new();
        let mut mixed = 0.0f64;
        for tick in 0u32..240 {
            let mut view = PhysOutView {
                num_wheels_in_contact: 4,
                effective_transform_y: [0.0, 1.0, 0.0, 0.0],
                deck_nose_dir: [0.0, 0.0, 1.0, 0.0],
                ground_up_vector: [0.0, 1.0, 0.0, 0.0],
                hips_mat_y: [0.0, 1.0, 0.0, 0.0],
                skateboard_surface_speed: 4.0,
                deck_velocity: [0.0, 0.0, 4.0, 0.0],
                centre_of_mass: [0.0, 1.0, 0.0, 1.0],
                deck_position: [0.0, 0.1, 0.0, 1.0],
                ..PhysOutView::default()
            };
            match tick {
                0..60 => {
                    view.is_physically_pushing = tick % 40 < 20;
                    view.is_generally_pushing = view.is_physically_pushing;
                    view.is_generally_pushing_right_foot = true;
                }
                120..150 => {
                    view.is_skateboard_airborne = true;
                    view.num_wheels_in_contact = 0;
                    view.time_in_known_air = (tick - 120) as f32 / 60.0;
                }
                150 => {
                    view.wheel_landed = [true; 4];
                    view.wheel_impact_strength = [50.0; 4];
                }
                _ => {}
            }
            let skater = system.fill_skater_data(&view, true, 0, false, None);
            system.world = Some(WorldAudioState {
                frame: tick + 1,
                sim_tick: tick + 1,
                camera: CameraMatrix { at: [0.0, 0.0, 1.0, 0.0], position: [0.0, 1.5, -4.0, 1.0] },
                player: PlayerFrame { skaters: vec![skater], game_speed: 1.0, is_rewinding: false },
                ..WorldAudioState::default()
            });
            system.update(1.0 / 60.0);
            while mixed < f64::from(tick + 1) / 60.0 {
                system.render_stereo(&mut stereo);
                mixed += 256.0 / 48000.0;
            }
        }
        let peak = |from: f32, to: f32| {
            let range = (from * 48000.0) as usize * 2..((to * 48000.0) as usize * 2).min(stereo.len());
            stereo[range].iter().fold(0.0f32, |m, x| m.max(x.abs()))
        };
        let (push, rolling, landing) = (peak(0.0, 1.0), peak(1.5, 1.95), peak(2.5, 3.5));
        eprintln!("push peak {push}, rolling peak {rolling}, landing peak {landing}");
        if let Some(path) = std::env::var_os("SKATE2_AUDIO_WAV") {
            write_wav(&path, &stereo);
        }
        assert!(push > 0.0, "no push sound");
        assert!(rolling > 0.0, "no rolling sound");
        assert!(landing > 0.0, "no landing sound");
    }
}

/// `SKATE2_TRACE_SPEECH`: the game's speech requests and the audio system's speech messages.
fn trace_speech(line: std::fmt::Arguments<'_>) {
    if std::env::var_os("SKATE2_TRACE_SPEECH").is_some() {
        eprintln!("speech: {line}");
    }
}
