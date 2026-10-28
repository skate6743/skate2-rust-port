//! `Sk8::Input::RumbleManager`, the `SkaterManager`'s (its constructor 0x825fa948): fifteen rumble
//! events, each a `rumble` collection of the skater vault, which `UpdateRumbleEvents` 0x825f64e0
//! runs over the local skater's `PhysOut` after every simulation tick (`SkaterManager::PostUpdate`
//! 0x825f8b88). An event that comes out moving is a `cMsgRumbleChannel` on the event's own channel,
//! which `PlayerUI::ReceiveRumbleChannelMsg` 0x828aa268 adds to the player's pad
//! (`Vibration::AddRumbleEvent`).
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    physical_player_phys_out::construct::rumble::SkaterRumblePhysOut,
    physics::{FilteredPhysicsState, PointGraph},
};
use std::sync::Arc;

const RUMBLE_CLASS: u64 = 0xE34B_3BAC_EEAF_190B;
const RUMBLE_PATTERN_CLASS: u64 = 0xE9CC_3355_7E9E_AA02;
const RUMBLE_SURFACE_CLASS: u64 = 0x4114_98F1_1DA4_6A57;
/// The classes the skater vault selects for the rumble manager.
pub(crate) const CLASSES: [u64; 3] = [RUMBLE_CLASS, RUMBLE_PATTERN_CLASS, RUMBLE_SURFACE_CLASS];

/// `eRumbleEvent` order: each event's `rumble` collection, as the constructor binds them.
const EVENTS: [(EventKind, u64); 15] = [
    (EventKind::SkeletonImpact, 0x9287_2872_C048_AAE9),
    (EventKind::DeckImpact, 0xC446_E594_5A25_7C86),
    (EventKind::Landing, 0xCF13_B9BE_D1D8_C9E8),
    (EventKind::PowerSlide, 0xD5FF_258A_904D_B509),
    (EventKind::SpeedWobble, 0x8441_E460_D4B9_077C),
    (EventKind::Push, 0x0611_0C5B_D81C_DF44),
    (EventKind::Wheels, 0xA65F_C8B7_41DC_CD99),
    (EventKind::Trucks, 0x27CD_E290_C328_67A9),
    (EventKind::Deck, 0xE6D5_1E08_1F06_42EA),
    (EventKind::FootBrake, 0x354D_D4C2_4DC5_0BD4),
    (EventKind::DeckBrake, 0x3492_821D_FA12_1A9C),
    (EventKind::FootSteps, 0xAFBF_4FFC_30F7_25F3),
    (EventKind::Pumping, 0x65DC_24B1_6689_CF8A),
    (EventKind::MoveObject, 0x17D8_14B0_4362_DC66),
    (EventKind::Carving, 0x24BF_A70D_2AAB_E02E),
];
/// `rumble::use_pattern_rumble`.
const USE_PATTERN_RUMBLE: u64 = 0x3AEF_41AC_6A05_94BE;
/// The scale `SkeletonImpactRumbleEvent::PostProcess` 0x825f2180 gives a skater not wiping out.
const SKELETON_IMPACT_SCALE: u64 = 0xA28E_E2F8_DF85_F6D8;
/// The scale `PumpingRumbleEvent::PostProcess` 0x825f28b0 gives a pump the skater did not mean.
const PUMPING_SCALE: u64 = 0xB43F_2EA4_E2F2_550F;
/// `MoveObjectRumbleEvent::GenerateInputValue` 0x825f2948's mass range and its curve.
const MOVE_OBJECT_MASS_MIN: u64 = 0x1501_B713_3633_1431;
const MOVE_OBJECT_MASS_MAX: u64 = 0xF26E_9D3D_8984_DE4F;
const MOVE_OBJECT_CURVE: u64 = 0xBB39_3AC5_5416_8CDB;

/// `RumbleDataMapper`'s three `rumble_surface` collections (its constructor 0x825f2e40).
const GRIND_SURFACES: u64 = 0x540A_6FA5_8191_02EE;
const PHYSICS_SURFACES: u64 = 0x0429_3D0F_2ADC_FBB4;
const AUDIO_SURFACES: u64 = 0x9631_4169_5224_2947;
/// `rumble_surface::physics_*` by `pegasus::tPhysicsSurfaceType`: undefined, smooth, rough, slow,
/// slippery, veryslow, unrideable, donotalign.
const PHYSICS_SURFACE_KEYS: [u64; 8] = [
    0xE981_DBBD_F67E_CAF3,
    0xBFE8_C265_A2A6_D17D,
    0x22B4_F2FC_54FE_B29B,
    0xA5EC_4468_351C_7BC5,
    0xFAE0_C518_7130_0C04,
    0x4F3A_DA04_CC30_A3C6,
    0x6027_1025_8D5C_8E9C,
    0xA7A0_F366_2068_AD7D,
];
/// `rumble_surface::audio_*` by rolling material: plywood, roughasphalt, roughconcrete,
/// smoothasphalt, smoothconcrete, woodramp, aggregate, grass, dirt, metal, manhole, drain, grate.
const AUDIO_SURFACE_KEYS: [u64; 13] = [
    0x5DEC_62F9_E5E5_32B6,
    0x448B_4190_EB67_753C,
    0x11B3_0179_2AA3_27D2,
    0x66A8_B24B_61FD_7894,
    0xEE74_DBD9_0352_604C,
    0xEBA3_C725_94D3_EF76,
    0x949B_9058_C735_828C,
    0xE2CA_9346_FBF5_538D,
    0x33B0_7FFA_F42E_4D48,
    0x67BF_6E8B_BAE7_7B7E,
    0x383B_4361_0B3E_7C36,
    0xF57A_CDC7_FF10_5337,
    0x9D81_A321_9935_1A39,
];
/// `rumble_surface::grind_*` by grind material: roughconcrete, smoothconcrete, roughwood,
/// smoothwood, rail1, rail2, rail3, rail4, coping, car, plastic, braidedbnch, perforatedbnch, rail5,
/// untagged.
const GRIND_SURFACE_KEYS: [u64; 15] = [
    0x98E2_15BA_1887_238B,
    0xC907_4F3D_0B68_C324,
    0x6535_508B_9BBD_9567,
    0xDDF8_3883_AEA8_0D48,
    0x1772_3819_2805_D0A6,
    0x55F2_D345_6F0E_9A73,
    0xFBF8_AF1E_08EE_6371,
    0xB940_6169_7CE7_51A8,
    0x64B4_E771_B55F_B8B1,
    0x4657_7EB4_C476_3897,
    0x416D_21D9_CDFF_5761,
    0xB4F3_1371_32B4_F6EE,
    0x6EF6_9C4E_6C04_18EF,
    0x3F18_199E_444B_8785,
    0x63DC_E27A_F4C0_C3C1,
];
/// `grind_untagged`: what `GetGrindSurfaceRumble` reads for an untagged surface.
const UNTAGGED_GRIND: usize = 14;
/// `RumbleDataMapper::GetSurfacePatternKey` 0x825ee5c0: each `pegasus::tSurfacePatternType`'s
/// `rumble_pattern` collection; any other type reads the first's (the class's default).
const PATTERN_KEYS: [u64; 16] = [
    0xD7ED_BD36_2D7D_2152,
    0x705F_4C5A_AB64_3C59,
    0x5CA0_64A0_949F_1CFD,
    0x4910_F5FE_8404_9C8E,
    0x2F8A_CE8D_2E0E_683C,
    0xF04E_7991_7BF5_C29E,
    0x0AEF_A1D0_D1B6_E360,
    0xB6B8_4FED_1FA3_E3F1,
    0xC626_A329_A594_C121,
    0x3522_236C_4655_8DE6,
    0xF69A_81BF_FE8F_DBB6,
    0x216A_369A_0184_462D,
    0x1B7A_30F0_1706_0546,
    0xDCF7_8E93_3147_BE02,
    0x62B3_59CF_3449_71C7,
    0xB145_B686_A909_80EB,
];
/// `GlobalAttributeCollection::aud_general_mapping`: `aud_general` "mapping", whose array of
/// 48-byte records `SurfaceMapping::GetRollingMaterial` 0x8251ba48 (word 1) and
/// `GetGrindMaterial` 0x8251bcd0 (word 4) read by audio material.
const AUD_GENERAL_CLASS: u64 = 0xC183_1BDB_6CB1_B1EA;
const AUD_GENERAL_MAPPING: u64 = 0xC489_459A_0C07_D154;
const MAPPING: u64 = 0x4CA6_0755_8B1C_F440;
const MAPPING_TYPE: u64 = 0xFEFA_9F11_5D3D_1860;
/// `eAudioMaterialType_Max`, and the last record `SurfaceMapping` reads (a material at or past it
/// reads that one).
const MATERIAL_MAX: i32 = 132;
const LAST_MAPPED_MATERIAL: u32 = 92;
/// `tRumbleInfo::operator==`'s tolerance, `SMALL_FLOAT` (2^-16).
const SMALL_FLOAT: f32 = f32::from_bits(0x3780_0000);
/// The step `SkaterManager::PostUpdate` passes.
pub(crate) const STEP: f32 = 1.0 / 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventKind {
    SkeletonImpact,
    DeckImpact,
    Landing,
    PowerSlide,
    SpeedWobble,
    Push,
    Wheels,
    Trucks,
    Deck,
    FootBrake,
    DeckBrake,
    FootSteps,
    Pumping,
    MoveObject,
    Carving,
}

/// `tRumbleInfo`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RumbleInfo {
    high: f32,
    low: f32,
    decay: f32,
}

impl RumbleInfo {
    /// `operator==(0.0f)`: both frequencies within `SMALL_FLOAT` of zero (unordered reads equal).
    fn is_still(self) -> bool {
        !(self.high.abs() > SMALL_FLOAT) && !(self.low.abs() > SMALL_FLOAT)
    }
}

/// A rumble the manager sends (`cMsgRumbleChannel`), on its event's channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RumbleChannel {
    pub(crate) low: f32,
    pub(crate) high: f32,
    pub(crate) decay: f32,
    pub(crate) channel: u32,
}

/// A `rumble` collection's layout (`Attrib::Gen::rumble`, 0xB0 bytes).
#[derive(Clone, Debug, Default)]
struct EventLayout {
    /// +0x00, read when +0x99 is set.
    low_curve: PointGraph<8>,
    /// +0x40, read when +0x9A is set.
    high_curve: PointGraph<8>,
    /// +0x80 and +0x8C.
    scale: RumbleInfo,
    offset: RumbleInfo,
    /// +0x98: the surface's rumble scales the event's.
    use_surface: bool,
    use_low_curve: bool,
    use_high_curve: bool,
    /// +0xA0 and +0xA4: the input range mapped to 0..1.
    input_min: f32,
    input_max: f32,
    /// +0xA8.
    enabled: bool,
    /// +0xAC: the rumble's decay rate.
    decay: f32,
}

#[derive(Clone, Debug)]
struct RumbleEvent {
    kind: EventKind,
    layout: EventLayout,
    /// `use_pattern_rumble`.
    use_pattern: bool,
    /// The skeleton impact and pumping events' `PostProcess` scale.
    scale: f32,
    /// The move-object event's mass range and curve.
    mass_min: f32,
    mass_max: f32,
    mass_curve: PointGraph<8>,
}

/// A `rumble_pattern` collection's layout (0x108 bytes).
#[derive(Clone, Debug, Default)]
struct PatternLayout {
    /// +0x00 and +0x80: the low and high frequency over one pass of the pattern.
    low: PointGraph<16>,
    high: PointGraph<16>,
    /// +0x100: the distance one pass covers; +0x104: the pattern's strength.
    length: f32,
    scale: f32,
}

/// `RumbleDataMapper`: the surface rumbles of the three `rumble_surface` collections, and the
/// `SurfaceMapping` records that turn an audio material into a rolling or grind material.
#[derive(Clone, Debug, Default)]
struct SurfaceRumbles {
    physics: [RumbleInfo; 8],
    audio: [RumbleInfo; 13],
    grind: [RumbleInfo; 15],
    /// Records 0..=92: each audio material's rolling and grind material.
    materials: Vec<(u32, u32)>,
}

impl SurfaceRumbles {
    /// `AudioSystem::GetAudioMaterial` 0x824e67c0 and the `SurfaceMapping` record it reads.
    fn materials(&self, surface: i32) -> (u32, u32) {
        let material = surface.wrapping_sub(1);
        let material = if surface == 0 || !(0..=MATERIAL_MAX).contains(&material) { MATERIAL_MAX } else { material };
        let record = u32::try_from(material).map_or(LAST_MAPPED_MATERIAL, |m| m.min(LAST_MAPPED_MATERIAL));
        usize::try_from(record).ok().and_then(|record| self.materials.get(record)).copied().unwrap_or_default()
    }
    /// `GetPhysicsSurfaceRumble` 0x825ee4a0.
    fn physics(&self, surface: u32) -> RumbleInfo {
        usize::try_from(surface).ok().and_then(|surface| self.physics.get(surface)).copied().unwrap_or_default()
    }
    /// `GetAudioSurfaceRumble` 0x825f2f38: an undefined surface does not rumble.
    fn audio(&self, surface: i32) -> RumbleInfo {
        if surface == 0 {
            return RumbleInfo::default();
        }
        let (rolling, _) = self.materials(surface);
        usize::try_from(rolling).ok().and_then(|rolling| self.audio.get(rolling)).copied().unwrap_or_default()
    }
    /// `GetGrindSurfaceRumble` 0x825f30d8: an undefined surface grinds as untagged.
    fn grind(&self, surface: i32) -> RumbleInfo {
        if surface == 0 {
            return self.grind[UNTAGGED_GRIND];
        }
        let (_, grind) = self.materials(surface);
        usize::try_from(grind).ok().and_then(|grind| self.grind.get(grind)).copied().unwrap_or_default()
    }
}

/// `WheelsRumbleEvent::mRumblePattern`: the pattern the wheels last rolled on (its key starts at
/// the class default's, the first) and how far along it they are.
#[derive(Clone, Copy, Debug, Default)]
struct WheelsPattern {
    pattern: usize,
    time: f32,
}

/// What `UpdateRumbleEvents` gathers for every event (`CommonEventData`) besides the `PhysOut`.
#[derive(Clone, Copy)]
struct Common {
    /// `mNumWheelsInContact > 0`.
    wheels_on_ground: bool,
    /// Either foot on the deck.
    feet_on_deck: bool,
}

/// The manager.
#[derive(Clone, Debug)]
pub(crate) struct RumbleManager {
    events: Vec<RumbleEvent>,
    surfaces: SurfaceRumbles,
    patterns: Vec<PatternLayout>,
    wheels: WheelsPattern,
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    bytes.get(offset..offset + 4).map_or(0, |w| u32::from_be_bytes([w[0], w[1], w[2], w[3]]))
}
fn float(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(word(bytes, offset))
}
fn flag(bytes: &[u8], offset: usize) -> bool {
    bytes.get(offset).is_some_and(|byte| *byte != 0)
}
/// An `N`-point graph: its x values, then its y values.
fn graph<const N: usize>(bytes: &[u8], offset: usize) -> PointGraph<N> {
    PointGraph {
        x: std::array::from_fn(|i| float(bytes, offset + 4 * i)),
        y: std::array::from_fn(|i| float(bytes, offset + 4 * (N + i))),
    }
}
fn info(bytes: &[u8], offset: usize) -> RumbleInfo {
    RumbleInfo { high: float(bytes, offset), low: float(bytes, offset + 4), decay: float(bytes, offset + 8) }
}
/// A collection's layout, or `DefaultDataArea`'s zeros without one.
fn layout(instance: &AttributeRuntimeInstance, size: usize) -> Result<Vec<u8>, AttributeRuntimeError> {
    let mut bytes = instance.try_cached_layout(size)?.map(|data| data.bytes().to_vec()).unwrap_or_default();
    bytes.resize(size, 0);
    Ok(bytes)
}
/// `GetAttributePointer`'s value, or `DefaultDataArea`'s zeros.
fn attribute(instance: &AttributeRuntimeInstance, key: u64) -> Result<Vec<u8>, AttributeRuntimeError> {
    Ok(instance.try_get_data(key, 0)?.map(|data| data.bytes().to_vec()).unwrap_or_default())
}
fn infos<const N: usize>(instance: &AttributeRuntimeInstance, keys: [u64; N]) -> Result<[RumbleInfo; N], AttributeRuntimeError> {
    let mut out = [RumbleInfo::default(); N];
    for (slot, key) in out.iter_mut().zip(keys) {
        *slot = info(&attribute(instance, key)?, 0);
    }
    Ok(out)
}

impl RumbleManager {
    /// The constructor's events and `RumbleDataMapper`, read once from the vault (its values never
    /// change); a missing collection or attribute reads `DefaultDataArea`'s zeros.
    pub(crate) fn try_new(vault: &Arc<AttributeRuntimeVault>) -> Result<Self, AttributeRuntimeError> {
        let mut events = Vec::with_capacity(EVENTS.len());
        for (kind, collection) in EVENTS {
            let instance = vault.instance(RUMBLE_CLASS, collection);
            let bytes = layout(&instance, 0xB0)?;
            let event_layout = EventLayout {
                low_curve: graph(&bytes, 0x00),
                high_curve: graph(&bytes, 0x40),
                scale: info(&bytes, 0x80),
                offset: info(&bytes, 0x8C),
                use_surface: flag(&bytes, 0x98),
                use_low_curve: flag(&bytes, 0x99),
                use_high_curve: flag(&bytes, 0x9A),
                input_min: float(&bytes, 0xA0),
                input_max: float(&bytes, 0xA4),
                enabled: flag(&bytes, 0xA8),
                decay: float(&bytes, 0xAC),
            };
            let scale = match kind {
                EventKind::SkeletonImpact => float(&attribute(&instance, SKELETON_IMPACT_SCALE)?, 0),
                EventKind::Pumping => float(&attribute(&instance, PUMPING_SCALE)?, 0),
                _ => 0.0,
            };
            let (mass_min, mass_max, mass_curve) = if kind == EventKind::MoveObject {
                (
                    float(&attribute(&instance, MOVE_OBJECT_MASS_MIN)?, 0),
                    float(&attribute(&instance, MOVE_OBJECT_MASS_MAX)?, 0),
                    graph(&attribute(&instance, MOVE_OBJECT_CURVE)?, 0),
                )
            } else {
                (0.0, 0.0, PointGraph::default())
            };
            events.push(RumbleEvent {
                kind,
                layout: event_layout,
                use_pattern: flag(&attribute(&instance, USE_PATTERN_RUMBLE)?, 0),
                scale,
                mass_min,
                mass_max,
                mass_curve,
            });
        }
        let mapping = vault.instance(AUD_GENERAL_CLASS, AUD_GENERAL_MAPPING);
        let mut materials = Vec::new();
        for record in 0..=LAST_MAPPED_MATERIAL {
            let field = |range| -> Result<u32, AttributeRuntimeError> {
                Ok(mapping.try_array_field_data(MAPPING, record, MAPPING_TYPE, 48, range)?.map_or(0, |data| word(data.bytes(), 0)))
            };
            materials.push((field(4..8)?, field(16..20)?));
        }
        let surfaces = SurfaceRumbles {
            physics: infos(&vault.instance(RUMBLE_SURFACE_CLASS, PHYSICS_SURFACES), PHYSICS_SURFACE_KEYS)?,
            audio: infos(&vault.instance(RUMBLE_SURFACE_CLASS, AUDIO_SURFACES), AUDIO_SURFACE_KEYS)?,
            grind: infos(&vault.instance(RUMBLE_SURFACE_CLASS, GRIND_SURFACES), GRIND_SURFACE_KEYS)?,
            materials,
        };
        let mut patterns = Vec::with_capacity(PATTERN_KEYS.len());
        for key in PATTERN_KEYS {
            let bytes = layout(&vault.instance(RUMBLE_PATTERN_CLASS, key), 0x108)?;
            patterns.push(PatternLayout {
                low: graph(&bytes, 0x00),
                high: graph(&bytes, 0x80),
                length: float(&bytes, 0x100),
                scale: float(&bytes, 0x104),
            });
        }
        Ok(Self { events, surfaces, patterns, wheels: WheelsPattern::default() })
    }

    /// `UpdateRumbleEvents`: every event over this tick's `PhysOut`, in event order; those that
    /// are not still go out on their event's channel.
    pub(crate) fn update(&mut self, phys: &SkaterRumblePhysOut, dt: f32) -> Vec<RumbleChannel> {
        let common = Common {
            wheels_on_ground: phys.num_wheels_in_contact > 0,
            feet_on_deck: phys.is_left_foot_on_deck || phys.is_right_foot_on_deck,
        };
        let mut rumbles = Vec::new();
        for (channel, event) in (0u32..).zip(&self.events) {
            let rumble = process(event, &self.surfaces, &self.patterns, &mut self.wheels, phys, common, dt);
            if !rumble.is_still() {
                rumbles.push(RumbleChannel { low: rumble.low, high: rumble.high, decay: rumble.decay, channel });
            }
        }
        rumbles
    }
}

/// Each event's `PreCondition`.
fn precondition(kind: EventKind, phys: &SkaterRumblePhysOut, common: Common) -> bool {
    match kind {
        EventKind::SkeletonImpact | EventKind::SpeedWobble | EventKind::Pumping => true,
        EventKind::DeckImpact => common.feet_on_deck,
        // A wheel that landed this tick and not the last.
        EventKind::Landing => {
            common.feet_on_deck && (0..4).any(|wheel| phys.wheel_landed[wheel] && !phys.prev_wheel_landed[wheel])
        }
        EventKind::PowerSlide | EventKind::Wheels => common.feet_on_deck && common.wheels_on_ground,
        EventKind::Push => phys.is_physically_pushing,
        EventKind::Trucks => common.feet_on_deck && (phys.is_front_truck_in_contact || phys.is_back_truck_in_contact),
        EventKind::Deck => common.feet_on_deck && phys.is_deck_in_contact,
        EventKind::FootBrake => phys.is_foot_braking,
        EventKind::DeckBrake => phys.is_manual_braking,
        EventKind::FootSteps => {
            !phys.is_grabbing_object && (phys.left_foot_just_landed || phys.right_foot_just_landed)
        }
        EventKind::MoveObject => phys.is_grabbing_object,
        EventKind::Carving => {
            common.feet_on_deck
                && common.wheels_on_ground
                && phys.is_carving
                && !(f64::from(phys.speed_wobble_effect) > 0.01)
        }
    }
}

/// Each event's `GenerateInputValue`.
fn input_value(event: &RumbleEvent, phys: &SkaterRumblePhysOut) -> f32 {
    match event.kind {
        EventKind::SkeletonImpact => phys.skeleton_contact_force.iter().fold(0.0, |sum, force| force + sum),
        EventKind::DeckImpact => phys.averaged_deck_contact_force,
        // The impacts of the wheels that landed this tick.
        EventKind::Landing => {
            let mut sum = 0.0f32;
            for wheel in 0..4 {
                if phys.wheel_landed[wheel] && !phys.prev_wheel_landed[wheel] {
                    sum = if wheel == 0 { phys.wheel_impact_strength[0] } else { phys.wheel_impact_strength[wheel] + sum };
                }
            }
            sum
        }
        EventKind::PowerSlide => phys.wheel_slide,
        EventKind::SpeedWobble => 0.0,
        // The length of the deck's empirical acceleration.
        EventKind::Push => {
            let [x, y, z] = phys.emperical_deck_acceleration;
            let square = x.mul_add(x, y.mul_add(y, z * z));
            if square == 0.0 { 0.0 } else { square.sqrt() }
        }
        EventKind::Wheels | EventKind::Trucks | EventKind::Deck | EventKind::FootBrake | EventKind::DeckBrake => {
            phys.skateboard_surface_speed
        }
        // How fast the skater's centre of mass comes down.
        EventKind::FootSteps => {
            let falling = phys.centre_of_mass_velocity_y * -1.0;
            if falling > 0.0 { falling } else { 0.0 }
        }
        EventKind::Pumping => phys.pumping_acceleration,
        // The object's movement, weighted by its mass on the curve.
        EventKind::MoveObject => {
            let mass = phys.object_mass * 0.01;
            let motion = (phys.anim_obj_angular_speed.abs() + phys.anim_obj_mov_x.abs()) + phys.anim_obj_mov_z.abs();
            let along = (mass - event.mass_min) / (event.mass_max - event.mass_min);
            event.mass_curve.evaluate(along) * motion
        }
        EventKind::Carving => phys.carving_slide,
    }
}

/// Each event's `GetSurfaceRumble`.
fn surface_rumble(kind: EventKind, phys: &SkaterRumblePhysOut, surfaces: &SurfaceRumbles) -> RumbleInfo {
    match kind {
        EventKind::PowerSlide | EventKind::FootBrake | EventKind::DeckBrake => surfaces.physics(phys.physics_surface_type),
        EventKind::Wheels => surfaces.audio(phys.wheel_audio_surface_type),
        EventKind::Trucks => surfaces.grind(if phys.is_front_truck_in_contact {
            phys.front_truck_audio_surface_type
        } else {
            phys.back_truck_audio_surface_type
        }),
        EventKind::Deck => surfaces.grind(phys.deck_audio_surface_type),
        _ => RumbleInfo::default(),
    }
}

/// `WheelsRumbleEvent::GetPatternRumble` 0x825f2640 (the other events have none): the pattern of
/// the surface under the first wheel, restarted when it changes, advanced by the distance rolled.
fn pattern_rumble(
    kind: EventKind,
    phys: &SkaterRumblePhysOut,
    patterns: &[PatternLayout],
    wheels: &mut WheelsPattern,
    dt: f32,
) -> RumbleInfo {
    if kind != EventKind::Wheels {
        return RumbleInfo::default();
    }
    let pattern = u32::try_from(phys.wheel_surface_pattern)
        .ok()
        .and_then(|pattern| usize::try_from(pattern).ok())
        .filter(|&pattern| pattern < PATTERN_KEYS.len())
        .unwrap_or(0);
    if pattern != wheels.pattern {
        wheels.pattern = pattern;
        wheels.time = 0.0;
    }
    patterns.get(pattern).map_or_else(RumbleInfo::default, |layout| {
        pattern_step(layout, &mut wheels.time, phys.skateboard_surface_speed * dt)
    })
}

/// `RumblePattern::GetRumble` 0x825f2568: one step along the pattern, wrapping past its end.
fn pattern_step(layout: &PatternLayout, time: &mut f32, step: f32) -> RumbleInfo {
    let step = if -step >= 0.0 { 0.0 } else { step };
    let length = layout.length;
    if !(length > 0.0) {
        return RumbleInfo::default();
    }
    *time = (if length - step >= 0.0 { step } else { length }) + *time;
    if *time > length {
        *time -= length;
    }
    let along = *time / length;
    let low = layout.low.evaluate(along);
    let high = layout.high.evaluate(along);
    RumbleInfo { high: high * layout.scale, low: layout.scale * low, decay: 0.0 }
}

/// Each event's `PostProcess`.
fn post_process(event: &RumbleEvent, phys: &SkaterRumblePhysOut, rumble: &mut RumbleInfo) {
    let scaled = match event.kind {
        EventKind::SkeletonImpact => {
            phys.filtered_physics_state != FilteredPhysicsState::Wipeout && !phys.physics_requests_wipeout
        }
        EventKind::Pumping => !phys.is_intentional_pump,
        _ => false,
    };
    if scaled {
        rumble.high *= event.scale;
        rumble.low *= event.scale;
    }
}

/// `ProcessRumbleEvent` 0x825f2b10: an enabled event whose condition holds maps its input over the
/// layout's range (through its curves when they are on), scaled by the surface's rumble and the
/// pattern's when the event takes them, then by the layout's scale and offset, then post-processed.
fn process(
    event: &RumbleEvent,
    surfaces: &SurfaceRumbles,
    patterns: &[PatternLayout],
    wheels: &mut WheelsPattern,
    phys: &SkaterRumblePhysOut,
    common: Common,
    dt: f32,
) -> RumbleInfo {
    let layout = &event.layout;
    let mut rumble = RumbleInfo::default();
    if !layout.enabled || !precondition(event.kind, phys, common) {
        return rumble;
    }
    let input = input_value(event, phys);
    rumble.decay = layout.decay;
    if input > layout.input_min {
        let along = if input >= layout.input_max {
            1.0
        } else {
            (input - layout.input_min) / (layout.input_max - layout.input_min)
        };
        rumble.high = if layout.use_high_curve { layout.high_curve.evaluate(along) } else { along };
        rumble.low = if layout.use_low_curve { layout.low_curve.evaluate(along) } else { along };
        if layout.use_surface || event.use_pattern {
            let (mut surface_high, mut surface_low, mut decay) = (0.0, 0.0, 0.0);
            if layout.use_surface {
                let surface = surface_rumble(event.kind, phys, surfaces);
                surface_high = rumble.high * surface.high;
                surface_low = surface.low * rumble.low;
                decay = rumble.decay;
            }
            let (mut pattern_high, mut pattern_low) = (0.0, 0.0);
            if event.use_pattern {
                let pattern = pattern_rumble(event.kind, phys, patterns, wheels, dt);
                pattern_high = rumble.high * pattern.high;
                pattern_low = pattern.low * rumble.low;
            }
            // Without the surface the rumble keeps no decay rate.
            rumble = RumbleInfo { high: pattern_high + surface_high, low: pattern_low + surface_low, decay };
        }
    }
    rumble.high = rumble.high * layout.scale.high + layout.offset.high;
    rumble.low = layout.scale.low * rumble.low + layout.offset.low;
    post_process(event, phys, &mut rumble);
    rumble
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp<const N: usize>() -> PointGraph<N> {
        #[expect(clippy::cast_precision_loss, reason = "small test indices")]
        let step = |i: usize| i as f32 / (N - 1) as f32;
        PointGraph { x: std::array::from_fn(step), y: std::array::from_fn(step) }
    }

    fn one_event(kind: EventKind, layout: EventLayout) -> RumbleManager {
        let event = RumbleEvent {
            kind,
            layout,
            use_pattern: false,
            scale: 0.0,
            mass_min: 0.0,
            mass_max: 0.0,
            mass_curve: PointGraph::default(),
        };
        RumbleManager {
            events: vec![event],
            surfaces: SurfaceRumbles::default(),
            patterns: vec![PatternLayout::default(); 16],
            wheels: WheelsPattern::default(),
        }
    }

    fn rolling() -> SkaterRumblePhysOut {
        SkaterRumblePhysOut {
            is_left_foot_on_deck: true,
            num_wheels_in_contact: 4,
            skateboard_surface_speed: 10.0,
            ..SkaterRumblePhysOut::default()
        }
    }

    fn layout() -> EventLayout {
        EventLayout {
            scale: RumbleInfo { high: 1.0, low: 1.0, decay: 0.0 },
            input_max: 20.0,
            enabled: true,
            decay: 10.0,
            ..EventLayout::default()
        }
    }

    #[test]
    fn an_enabled_event_maps_its_input_over_its_range_with_its_decay() {
        let mut manager = one_event(EventKind::Wheels, layout());
        let rumbles = manager.update(&rolling(), STEP);
        assert_eq!(rumbles, [RumbleChannel { low: 0.5, high: 0.5, decay: 10.0, channel: 0 }]);
    }

    #[test]
    fn a_disabled_event_or_an_unmet_condition_sends_nothing() {
        let mut manager = one_event(EventKind::Wheels, EventLayout { enabled: false, ..layout() });
        assert!(manager.update(&rolling(), STEP).is_empty());
        let mut manager = one_event(EventKind::Wheels, layout());
        let airborne = SkaterRumblePhysOut { num_wheels_in_contact: 0, ..rolling() };
        assert!(manager.update(&airborne, STEP).is_empty());
    }

    #[test]
    fn an_input_at_its_minimum_rests() {
        let mut manager = one_event(EventKind::Wheels, EventLayout { input_min: 10.0, ..layout() });
        assert!(manager.update(&rolling(), STEP).is_empty());
    }

    #[test]
    fn a_surface_scales_the_rumble_and_an_undefined_one_stops_it() {
        let mut manager = one_event(EventKind::Wheels, EventLayout { use_surface: true, ..layout() });
        manager.surfaces.audio[3] = RumbleInfo { high: 0.5, low: 0.25, decay: 15.0 };
        manager.surfaces.materials = vec![(0, 0); 93];
        manager.surfaces.materials[4] = (3, 0);
        let on_material_four = SkaterRumblePhysOut { wheel_audio_surface_type: 5, ..rolling() };
        let rumbles = manager.update(&on_material_four, STEP);
        assert_eq!(rumbles, [RumbleChannel { low: 0.125, high: 0.25, decay: 10.0, channel: 0 }]);
        let undefined = SkaterRumblePhysOut { wheel_audio_surface_type: 0, ..rolling() };
        assert!(manager.update(&undefined, STEP).is_empty());
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "exact native float results")]
    fn a_pattern_advances_by_the_distance_rolled_and_wraps() {
        let layout = PatternLayout { low: ramp(), high: ramp(), length: 2.0, scale: 1.0 };
        let mut time = 0.0;
        let first = pattern_step(&layout, &mut time, 0.5);
        assert_eq!((first.low, first.high, first.decay), (0.25, 0.25, 0.0));
        pattern_step(&layout, &mut time, 1.75);
        assert_eq!(time, 0.25);
        assert_eq!(pattern_step(&layout, &mut time, -1.0).low, 0.125);
        assert_eq!(pattern_step(&PatternLayout::default(), &mut time, 1.0), RumbleInfo::default());
    }

    #[test]
    fn surface_materials_follow_the_audio_material_of_the_surface() {
        let mut surfaces = SurfaceRumbles { materials: vec![(0, 0); 93], ..SurfaceRumbles::default() };
        surfaces.materials[92] = (9, 9);
        surfaces.grind[9] = RumbleInfo { high: 1.0, low: 1.0, decay: 0.1 };
        surfaces.grind[UNTAGGED_GRIND] = RumbleInfo { high: 0.0, low: 0.0, decay: 15.0 };
        assert_eq!(surfaces.grind(500), surfaces.grind[9]);
        assert_eq!(surfaces.grind(-3), surfaces.grind[9]);
        assert_eq!(surfaces.grind(0), surfaces.grind[UNTAGGED_GRIND]);
        assert_eq!(surfaces.physics(9), RumbleInfo::default());
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "exact native float results")]
    fn a_skeleton_impact_is_scaled_unless_the_skater_wipes_out() {
        let mut manager = one_event(EventKind::SkeletonImpact, layout());
        manager.events[0].scale = 0.5;
        let hit = SkaterRumblePhysOut { skeleton_contact_force: [5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], ..rolling() };
        assert_eq!(manager.update(&hit, STEP)[0].high, 0.25);
        let wiping = SkaterRumblePhysOut { filtered_physics_state: FilteredPhysicsState::Wipeout, ..hit };
        assert_eq!(manager.update(&wiping, STEP)[0].high, 0.5);
    }
}
