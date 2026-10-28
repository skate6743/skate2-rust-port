//! The ENVIRONMENT state (`eMAINMAPSTATES` 2): `CSTATEMGR_Environment`, whose `EnterGamePlay`
//! 0x8254fc18 creates one `CSTATE_Environment` with `Setup(1)`: `Sk8::Audio::SFXObj_Ambience`
//! (0x58 bytes, TypeInfo 0x00020000, `CreateObject` 0x8254dc78), the location's ambience bed. The
//! beds are looped 5-channel EA-XMA streams in `data\audio\ambience.big`, played through
//! `SndPlayer1` with the resident `.snr` headers of `ambienceresident.big`, and
//! `C_main_ambience_crossfade` (`Main_Ambience_Crossfade.abk`) sounds while one bed hands over to
//! the next. `aud_wp_ambiences` (the key's collection) gives each bed's volume, id and fade times.

use std::sync::Arc;

use skate2_formats::big::{BigArchive, BigEntryKey};

use crate::aems::ClassObject;
use crate::math::fnmsub;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::{SfxCtx, PITCH_SCALE, VOLUME_SCALE};
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::PlayParams;
use crate::rwac::{PluginRef, VoiceId};
use crate::system::WorldAudioState;
use crate::vault::{class, Collection, Vault};

/// `Attrib::Gen::aud_wp_ambiences`' class.
const AUD_WP_AMBIENCES: u64 = 0x656A_AE7B_B40F_6C7F;
/// The `aud_wp_ambiences` layout words.
const VOLUME: u32 = 0;
const ID: u32 = 1;
const FADE_OUT: u32 = 2;
const FADE_IN: u32 = 3;
/// `GlobalAttributeCollection::aud_general_test_ambience` (+0x68) and its `RefSpec` that replaces
/// the ambience while it names a collection.
const TEST_AMBIENCE: Collection = Collection::new(class::AUD_GENERAL, 0xA7BC_D60F_F7EC_DE26);
const TEST_AMBIENCE_KEY: u64 = 0x51BA_D6E2_DB03_B4AE;
/// The front end's ambience.
const FE_AMBIENCE: u64 = 0xF646_CF81_5FDF_E13B;
/// The ambience every change to or from takes crossfade 25.
const CROSS_FADE_25: u64 = 0x0CA3_8082_91F3_515D;
/// `ambienceCrossFade` 0x82024d08: the crossfade between two ambiences, in either order.
const CROSS_FADES: [(u64, u64, i32); 24] = [
    (0x7049_9084_C7A3_A9A5, 0x6044_2B04_D22D_3F6D, 1),
    (0x6044_2B04_D22D_3F6D, 0x70BC_5DB7_9B79_ECFA, 2),
    (0x6044_2B04_D22D_3F6D, 0x2215_4395_33BB_51A4, 3),
    (0x70BC_5DB7_9B79_ECFA, 0x262E_B27C_F110_D5F4, 4),
    (0x70BC_5DB7_9B79_ECFA, 0x5254_54AE_EFF6_16EE, 5),
    (0x70BC_5DB7_9B79_ECFA, 0x2215_4395_33BB_51A4, 6),
    (0x5254_54AE_EFF6_16EE, 0x2215_4395_33BB_51A4, 7),
    (0x5254_54AE_EFF6_16EE, 0x8943_70BC_9E99_F90E, 8),
    (0x2215_4395_33BB_51A4, 0xF646_CF81_5FDF_E13B, 9),
    (0xF646_CF81_5FDF_E13B, 0x5677_DB4F_D468_1CEC, 10),
    (0x5254_54AE_EFF6_16EE, 0x78AE_0B9E_1F90_91F1, 11),
    (0x78AE_0B9E_1F90_91F1, 0xA4D6_01A3_23DB_650E, 12),
    (0x5254_54AE_EFF6_16EE, 0x8169_DB21_9036_52B8, 13),
    (0x2215_4395_33BB_51A4, 0x8169_DB21_9036_52B8, 14),
    (0x2215_4395_33BB_51A4, 0xCEED_3BB1_81EB_9F10, 15),
    (0xCEED_3BB1_81EB_9F10, 0x2626_744F_195B_0EA7, 16),
    (0xCEED_3BB1_81EB_9F10, 0x8169_DB21_9036_52B8, 17),
    (0xCEED_3BB1_81EB_9F10, 0x1195_8D29_7A37_10DF, 18),
    (0xCEED_3BB1_81EB_9F10, 0xE03C_3999_FE94_4219, 19),
    (0xCEED_3BB1_81EB_9F10, 0x31EE_126E_9F46_316F, 20),
    (0xE03C_3999_FE94_4219, 0xE316_CB22_314D_967F, 21),
    (0xE03C_3999_FE94_4219, 0x27E5_DE92_D436_CCBC, 22),
    (0x27E5_DE92_D436_CCBC, 0x78AE_0B9E_1F90_91F1, 23),
    (0x27E5_DE92_D436_CCBC, 0x8169_DB21_9036_52B8, 24),
];
const CROSS_FADE_CLASS: &str = "c_main_ambience_crossfade";
/// The `SFXObj_Ambience` objectID: state 2, instance 0, object 0.
const OBJECT_ID: u32 = 0x4002_0000;
/// `StreamPool::CreateInstance('Amb0', 1, 0x80000, 10, ...)`: one stream buffering 0x80000 bytes.
const STREAM_POOL: u32 = 0x416D_6230;
const STREAM_BUFFER_BYTES: u64 = 0x8_0000;
const RESIDENT_DATA: &str = "data\\audio/ambienceresident.big";
const STREAMS: &str = "data\\audio/ambience.big|";
/// `32767.0f`, the fade's Q15 scale.
const Q15: f32 = f32::from_bits(0x46FF_FE00);
/// The bed voice's plug-ins (`pAmbience`).
const VOICE_GAIN: u8 = 3;
const VOICE_RESAMPLE: u8 = 2;
const VOICE_LOW_PASS: u8 = 4;
const VOICE_SEND: u8 = 5;

/// `Attrib::Gen::aud_wp_ambiences(key)`.
const fn wp_ambience(key: u64) -> Collection {
    Collection::new(AUD_WP_AMBIENCES, key)
}

/// `SFXObj_Ambience::AmbienceInfo` (40 bytes).
struct AmbienceInfo {
    /// +0: the `.snr` header in the resident data.
    snr: DataRef,
    /// +4: the entry's name with its last letter made 's' (the `.sns` stream).
    filename: String,
    /// +0x24: `atol` of the name's first two characters.
    id: i32,
}

/// `mAmbienceState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmbienceState {
    None,
    Playing,
    FadeOut,
    FadeIn,
}

/// `Sk8::Audio::SFXObj_Ambience`.
struct Ambience {
    /// `m_pDMixIO`.
    io: Option<DMixIoId>,
    /// +0x24/+0x28 `mNumAmbiences`, `mpAmbienceInfo`; the resident data (+0x20) stays with their
    /// headers.
    infos: Vec<AmbienceInfo>,
    /// +0x30 `mAmbienceKey`.
    ambience_key: u64,
    /// +0x3C: the bed's voice (+0x38 `pAmbience` are its plug-ins).
    voice: Option<VoiceId>,
    /// +0x40.
    state: AmbienceState,
    /// +0x44 `mFadeOutTime` and +0x48 `mFadeInTime`.
    fade_out_time: f32,
    fade_in_time: f32,
    /// +0x4C `mpCrossFade`.
    cross_fade: Option<ClassObject>,
    /// +0x54 `mbWasFE`.
    was_fe: i32,
}

impl Ambience {
    /// The constructor 0x82545168: `LoadResidentData`, then the 'Amb0' stream pool
    /// (`mpAmbienceStreamPool` +0x1C). The object lives until `AudioSystem::Shutdown`, so the
    /// destructor 0x82529b60 that releases the voice and the pool never runs.
    fn new(io: Option<DMixIoId>, ctx: &mut SfxCtx<'_>) -> Self {
        let infos = load_resident_data(ctx);
        ctx.core.stream_pools.create(STREAM_POOL, 1, STREAM_BUFFER_BYTES);
        Self {
            io,
            infos,
            ambience_key: 0,
            voice: None,
            state: AmbienceState::None,
            fade_out_time: 0.0,
            fade_in_time: 0.0,
            cross_fade: None,
            was_fe: 0,
        }
    }

    /// `GetAmbienceKey` 0x8252a058: the front end's ambience while `mbIsFE` and for 30 updates
    /// after it (and on while loading at the last one), else `AudioSystem::GetAmbienceKey`; a set
    /// `aud_general_test_ambience` replaces either.
    fn get_ambience_key(&mut self, world_key: u64, is_fe: i32, loading: bool, vault: &Vault) -> u64 {
        let mut key = world_key;
        if is_fe > 0 {
            key = FE_AMBIENCE;
            self.was_fe = 30;
        } else if self.was_fe > 1 {
            key = FE_AMBIENCE;
            self.was_fe -= 1;
        } else if self.was_fe == 1 {
            if loading {
                key = FE_AMBIENCE;
            } else {
                self.was_fe = 0;
            }
        }
        let test = vault.bytes(TEST_AMBIENCE, TEST_AMBIENCE_KEY, 0, 0x18);
        let test = u64::from_be_bytes(test[8..16].try_into().expect("eight bytes"));
        if test != 0 {
            key = test;
        }
        key
    }

    /// `UpdateParams` 0x8254dd30: follows the key (`UpdateAmbience` while it holds,
    /// `ChangeAmbience` when it moves), then sets mixer input 0 to the fade: the fade-out's
    /// progress, the fade-in's remainder, 0 while playing and 0x7FFF without a bed.
    fn update_params(&mut self, t: f32, key: u64, ctx: &mut SfxCtx<'_>) {
        if key == self.ambience_key {
            self.update_ambience(t, ctx);
        } else {
            self.change_ambience(key, t, ctx);
        }
        let collection = wp_ambience(self.ambience_key);
        let fade = match self.state {
            AmbienceState::FadeOut => fctiwz(self.fade_out_time / ctx.vault.layout_f32(collection, FADE_OUT) * Q15),
            AmbienceState::FadeIn => 0x7FFF_i32.wrapping_sub(fctiwz(self.fade_in_time / ctx.vault.layout_f32(collection, FADE_IN) * Q15)),
            AmbienceState::Playing => 0,
            AmbienceState::None => 0x7FFF,
        };
        ctx.set_input(self.io, 0, fade.clamp(0, 0x7FFF));
    }

    /// `ChangeAmbience` 0x825452d0, one step per update while the key differs: a playing bed starts
    /// fading out (with the pair's crossfade sound when there is a new key), or without a key of
    /// its own just stops; the fade-out ends after `fadeOut` seconds with `StopAmbience`; with no
    /// bed the new key's bed starts fading in and the key is taken; a fade-in carries on.
    fn change_ambience(&mut self, key: u64, t: f32, ctx: &mut SfxCtx<'_>) {
        match self.state {
            AmbienceState::Playing => {
                if self.ambience_key == 0 {
                    self.state = AmbienceState::None;
                    return;
                }
                self.state = AmbienceState::FadeOut;
                self.fade_out_time = 0.0;
                if key != 0 {
                    let id = cross_fade_id(self.ambience_key, key).clamp(1, 25);
                    self.cross_fade = Some(ctx.aems.create_object(CROSS_FADE_CLASS, vec![0, 0x7FFF, 0, 0, 0x1000, 25000, 0, 0, 0, id]));
                }
            }
            AmbienceState::FadeOut => {
                self.fade_out_time += t;
                if self.fade_out_time > ctx.vault.layout_f32(wp_ambience(self.ambience_key), FADE_OUT) {
                    self.stop_ambience(ctx);
                    self.state = AmbienceState::None;
                }
            }
            AmbienceState::None => {
                if key != 0 {
                    self.state = AmbienceState::FadeIn;
                    self.fade_in_time = 0.0;
                    let id = ctx.vault.layout_word(wp_ambience(key), ID) as i32;
                    self.play_ambience(id, ctx);
                }
                self.ambience_key = key;
            }
            AmbienceState::FadeIn => self.update_ambience(t, ctx),
        }
    }

    /// `UpdateAmbience` 0x8252a220: a fade-in runs `fadeIn` seconds, then the crossfade sound is
    /// released and the bed plays on; a fade-out turns back into a fade-in from where it stood.
    fn update_ambience(&mut self, t: f32, ctx: &mut SfxCtx<'_>) {
        let collection = wp_ambience(self.ambience_key);
        match self.state {
            AmbienceState::FadeIn => {
                self.fade_in_time += t;
                if self.fade_in_time > ctx.vault.layout_f32(collection, FADE_IN) {
                    if let Some(cross_fade) = self.cross_fade.take() {
                        ctx.aems.release_object(cross_fade);
                    }
                    self.state = AmbienceState::Playing;
                }
            }
            AmbienceState::FadeOut => {
                self.state = AmbienceState::FadeIn;
                let fade_in = ctx.vault.layout_f32(collection, FADE_IN);
                self.fade_in_time = fnmsub(self.fade_out_time / ctx.vault.layout_f32(collection, FADE_OUT), fade_in, fade_in);
            }
            AmbienceState::None | AmbienceState::Playing => {}
        }
    }

    /// `ProcessUpdate` 0x8252a310: the bed's gain (`volume` times mixer volume 0), resample
    /// (mixer pitch 2) and low-pass frequency (mixer frequency 3); during a fade the crossfade
    /// sound's members 0 and 8 follow mixer volume 1.
    fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        if self.state == AmbienceState::None {
            return;
        }
        let level = ctx.output(self.io, 0, PresetType::Volume) as f32 * VOLUME_SCALE;
        let volume = ctx.vault.layout_f32(wp_ambience(self.ambience_key), VOLUME) * level;
        if matches!(self.state, AmbienceState::FadeIn | AmbienceState::FadeOut) {
            if let Some(cross_fade) = &mut self.cross_fade {
                let level = ctx.output(self.io, 1, PresetType::Volume).clamp(0, 0x7FFF);
                cross_fade.data[0] = level;
                cross_fade.data[8] = level;
                ctx.aems.set_member_data(cross_fade);
            }
        }
        let Some(voice) = self.voice else { return };
        ctx.core.set_attribute(PluginRef { voice, index: VOICE_GAIN }, 0, volume);
        let pitch = ctx.output(self.io, 2, PresetType::Pitch) as f32 * PITCH_SCALE;
        ctx.core.set_attribute(PluginRef { voice, index: VOICE_RESAMPLE }, 0, pitch);
        let frequency = ctx.output(self.io, 3, PresetType::Frequency) as f32;
        ctx.core.set_attribute(PluginRef { voice, index: VOICE_LOW_PASS }, 0, frequency);
    }

    /// `PlayAmbience` 0x82529c90: a stage 0 voice (`SndPlayer1`, `Rechannel`, `Resample`, `Gain`,
    /// `LowPassIir2`, `Send`, each of six channels) playing the bed's stream
    /// (`data\audio\ambience.big|<name>.sns` with its resident header, pool 'Amb0', expelled when
    /// done) into the master sub mix. The id it also keeps (+0x50) is never read.
    fn play_ambience(&mut self, id: i32, ctx: &mut SfxCtx<'_>) {
        let Some(info) = get_ambience_info(&self.infos, id) else { return };
        let six = |kind: PlugInKind| PlugInConfig { kind, output_channels: 6 };
        self.voice = ctx.core.create_voice(
            0,
            "Ambience",
            &[
                six(PlugInKind::SndPlayer1 { max_requests: 1.0 }),
                six(PlugInKind::Rechannel),
                six(PlugInKind::Resample),
                six(PlugInKind::Gain),
                six(PlugInKind::LowPassIir2),
                six(PlugInKind::Send),
            ],
        );
        let Some(voice) = self.voice else { return };
        // PC port: the bed starts silent, where its fade-in starts, until `ProcessUpdate` sets its
        // gain. The original played it at the Gain's default of 1.0 until then, an update at full
        // volume whatever the profile's; the PC's first frames come far apart, and with a profile
        // at no volume that was a blast of ambience at boot.
        ctx.core.set_attribute(PluginRef { voice, index: VOICE_GAIN }, 0, 0.0);
        let params = PlayParams {
            stream_path: Some(format!("{STREAMS}{}", info.filename).replace('/', "\\")),
            ram_sample: Some(info.snr.clone()),
            stream_pool: STREAM_POOL,
            expel_mode: 1.0,
            ..PlayParams::default()
        };
        ctx.core.play(PluginRef { voice, index: 0 }, params);
        ctx.core.connect_send(PluginRef { voice, index: VOICE_SEND }, Some(ctx.master_sub_mix));
    }

    /// `StopAmbience` 0x82529fa8: the player's stop event, then the voice's release.
    fn stop_ambience(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(voice) = self.voice.take() {
            ctx.core.stop(PluginRef { voice, index: 0 });
            ctx.core.release_voice(voice);
        }
    }
}

/// `LoadResidentData` 0x8252a530: `SyncLoad("data\\audio/ambienceresident.big")` and an
/// `AmbienceInfo` per table entry.
fn load_resident_data(ctx: &SfxCtx<'_>) -> Vec<AmbienceInfo> {
    let Some(files) = ctx.core.stream_pools.files() else { return Vec::new() };
    let Some(size) = files.size(RESIDENT_DATA) else { return Vec::new() };
    let mut bytes = vec![0u8; size as usize];
    if !files.read_at(RESIDENT_DATA, 0, &mut bytes) {
        return Vec::new();
    }
    let Ok(table) = BigArchive::parse(&bytes) else { return Vec::new() };
    let bytes: Arc<[u8]> = Arc::from(bytes);
    table
        .entries
        .iter()
        .filter_map(|entry| {
            let BigEntryKey::Name(name) = &entry.key else { return None };
            let mut filename = name.clone();
            filename.pop();
            filename.push('s');
            let id = atol(name.get(..2).unwrap_or(name));
            Some(AmbienceInfo { snr: DataRef { bytes: Arc::clone(&bytes), offset: entry.offset as usize }, filename, id })
        })
        .collect()
}

/// The CRT `atol`: optional blanks and sign, then decimal digits.
fn atol(text: &str) -> i32 {
    let text = text.trim_start_matches([' ', '\t', '\n', '\r', '\x0B', '\x0C']);
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let value = digits.bytes().take_while(u8::is_ascii_digit).fold(0i32, |value, digit| value.wrapping_mul(10).wrapping_add(i32::from(digit - b'0')));
    if negative { value.wrapping_neg() } else { value }
}

/// `GetAmbienceInfo` 0x825159e0: the bed with this id, else the first one.
fn get_ambience_info(infos: &[AmbienceInfo], id: i32) -> Option<&AmbienceInfo> {
    infos.iter().find(|info| info.id == id).or_else(|| infos.first())
}

/// `GetAmbienceCrossFadeID` 0x82515a40: 25 to or from [`CROSS_FADE_25`], the pair's entry in
/// either order, else 6.
fn cross_fade_id(from: u64, to: u64) -> i32 {
    if from == CROSS_FADE_25 || to == CROSS_FADE_25 {
        return 25;
    }
    CROSS_FADES.iter().find(|&&(a, b, _)| (from == a && to == b) || (from == b && to == a)).map_or(6, |&(_, _, id)| id)
}

/// `CSTATEMGR_Environment` and its one state. `GameModule::OneOffLoad` 0x8273add8 calls
/// `AudioSystem::EnterWorld` once at boot, whose `InitializeMixer` 0x824ed830 enters every state
/// manager's gameplay; only `AudioSystem::Shutdown` 0x824e64b8 leaves it. The ambience therefore
/// plays in the front end too, with or without a world.
pub struct EnvironmentManager {
    io: Option<DMixIoId>,
    ambience: Option<Ambience>,
    /// The presentation's `GlobalData.ambienceKey` (`AudioSystem::GetAmbienceKey` 0x824f1cb8),
    /// which keeps its last value through updates without a world.
    presentation_key: u64,
}

impl EnvironmentManager {
    #[must_use]
    pub fn new(_vault: &Vault, map: Option<&MixMap>) -> Self {
        Self { io: map.and_then(|map| map.dmixio(OBJECT_ID)), ambience: None, presentation_key: 0 }
    }

    /// `CSTATEMGR_Base::UpdateParams`. The first one enters gameplay (`CreateState`, `Setup(1)`,
    /// then `Attach`, whose `LoadData` runs `InitSFX`: the mixer output on). `is_fe` is
    /// `AudioSystem::mbIsFE` and `loading` `AudioSystem::IsLoading`.
    pub fn update_params(&mut self, t: f32, world: Option<&WorldAudioState>, is_fe: i32, loading: bool, ctx: &mut SfxCtx<'_>) {
        if let Some(world) = world {
            self.presentation_key = world.ambience_key;
        }
        if self.ambience.is_none() {
            self.ambience = Some(Ambience::new(self.io, ctx));
            if let Some(io) = self.io {
                ctx.map.set_output_enabled(io, true);
            }
        }
        if let Some(ambience) = &mut self.ambience {
            let key = ambience.get_ambience_key(self.presentation_key, is_fe, loading, ctx.vault);
            ambience.update_params(t, key, ctx);
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate`.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(ambience) = &mut self.ambience {
            ambience.process_update(ctx);
        }
    }
}
