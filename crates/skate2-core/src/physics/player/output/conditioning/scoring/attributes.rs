//! Scoring's original generated attribute access paths. Tuning remains in Vault.
#[cfg(feature = "skate3-runtime")]
#[path = "skate3_hom.rs"]
pub(super) mod skate3_hom;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    physics::PointGraph,
};

#[derive(Debug)]
pub(crate) enum ScoreError {
    Access(crate::physical_player_phys_out::PhysOutAccessError),
    Cvar(crate::input_cvar_runtime::InputCvarError),
    AirReset(super::air_collector::AirResetError),
    Attribute(AttributeRuntimeError),
    Numeric(crate::reckoning_numeric::ReckoningHostNumericError),
    Invalid(&'static str),
    Missing(&'static str),
    Poisoned,
    #[cfg(feature = "skate3-runtime")]
    NativeCatalog(String),
}
impl From<AttributeRuntimeError> for ScoreError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Attribute(value)
    }
}
impl std::fmt::Display for ScoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ScoreError {}

pub(super) fn word(instance: &ScoringAttributes, key: u64) -> Result<u32, ScoreError> {
    #[cfg(feature = "skate3-runtime")]
    if instance.skate3_enabled() {
        return instance.source_word(key);
    }
    let Some(data) = instance.try_get_data(key, 0)? else {
        return Ok(0);
    };
    Ok(u32::from_be_bytes(
        data.bytes()
            .get(..4)
            .ok_or(ScoreError::Invalid("scoring word width"))?
            .try_into()
            .unwrap(),
    ))
}
pub(super) fn layout_word(
    instance: &ScoringAttributes,
    offset: usize,
) -> Result<u32, ScoreError> {
    #[cfg(feature = "skate3-runtime")]
    if instance.skate3_enabled() && instance.class == skate3_hom::CLASS {
        return skate3_hom::data().map_err(ScoreError::NativeCatalog)?.bone_word(offset)
            .ok_or(ScoreError::Invalid("Skate 3 HoM bone layout offset"));
    }
    let Some(data) = instance.try_cached_layout_field(offset, 4)? else {
        return Ok(0);
    };
    Ok(u32::from_be_bytes(
        data.bytes()
            .try_into()
            .map_err(|_| ScoreError::Invalid("scoring layout word"))?,
    ))
}

/// Generated GetAttributePointer(key,0), followed by DefaultDataArea(4).
pub(super) fn scalar(instance: &ScoringAttributes, key: u64) -> Result<f32, ScoreError> {
    #[cfg(feature = "skate3-runtime")]
    if instance.skate3_enabled() { return word(instance, key).map(f32::from_bits); }
    let Some(data) = instance.try_get_data(key, 0)? else {
        return Ok(0.0);
    };
    let bytes = data
        .bytes()
        .get(..4)
        .ok_or(ScoreError::Invalid("scoring scalar width"))?;
    Ok(f32::from_be_bytes(bytes.try_into().unwrap()))
}

/// PointNegGraphData8 has sixteen metadata bytes, then eight X and eight Y.
/// Native scoring calls PointGraphEval directly; metadata is not a second curve.
pub(super) fn curve(
    instance: &ScoringAttributes,
    offset: usize,
    x: f32,
) -> Result<f32, ScoreError> {
    #[cfg(feature = "skate3-runtime")]
    if instance.skate3_enabled() {
        if instance.class == skate3_hom::CLASS {
            return skate3_hom::data().map_err(ScoreError::NativeCatalog)?.curve(offset,x)
                .ok_or(ScoreError::Invalid("Skate 3 HoM curve offset"));
        }
        let data = super::skate3_catalog::data().map_err(ScoreError::NativeCatalog)?;
        return match instance.class {
            0x546c_36b6_5603_8e04 => {
                let offset = u16::try_from(offset).map_err(|_| ScoreError::Invalid("Skate 3 collector curve offset"))?;
                data.collector.try_curve(offset, x).ok_or(ScoreError::Invalid("Skate 3 collector curve offset"))
            },
            0x3492_15e2_e817_703c => match offset {
                180 => Ok(data.announcement.evaluate(x)),
                260 => Ok(data.repetition.evaluate(x)),
                _ => Err(ScoreError::Invalid("Skate 3 general curve offset")),
            },
            _ => Err(ScoreError::Invalid("Skate 3 scoring class")),
        };
    }
    let Some(data) = instance.try_cached_layout_field(offset + 16, 64)? else {
        return Ok(PointGraph::<8>::default().evaluate(x));
    };
    let b = data.bytes();
    if b.len() != 64 {
        return Err(ScoreError::Invalid("scoring curve width"));
    }
    let values: [f32; 16] =
        std::array::from_fn(|i| f32::from_be_bytes(b[i * 4..i * 4 + 4].try_into().unwrap()));
    Ok(PointGraph::<8> {
        x: values[..8].try_into().unwrap(),
        y: values[8..].try_into().unwrap(),
    }
    .evaluate(x))
}

/// Per-ScoreModule selection shared by cloned distance/time graph addresses.
/// The underlying host collection remains untouched for other skaters and modes.
#[derive(Clone, Debug)]
pub(super) struct ScoringAttributes {
    native: AttributeRuntimeInstance,
    class: u64,
    enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl ScoringAttributes {
    pub(super) fn new(native: AttributeRuntimeInstance, class: u64) -> Self {
        Self { native, class, enabled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)) }
    }
    #[cfg(feature = "skate3-runtime")]
    pub(super) fn select_skate3(&self, enabled: bool) {
        self.enabled.store(enabled, std::sync::atomic::Ordering::Relaxed);
    }
    #[cfg(feature = "skate3-runtime")]
    pub(super) fn skate3_enabled(&self) -> bool { self.enabled.load(std::sync::atomic::Ordering::Relaxed) }
    #[cfg(feature = "skate3-runtime")]
    fn source_word(&self, key: u64) -> Result<u32, ScoreError> {
        // Parse authored scalar words once, not through JSON/name lookups
        // during every physics tick. Curve samples are likewise retained by ScoringData.
        type Words = std::collections::BTreeMap<(u64, u64), u32>;
        static WORDS: std::sync::OnceLock<Words> = std::sync::OnceLock::new();
        let words = crate::skate3_native::physics::player::linked_once(&WORDS, || {
            let data = crate::skate3_native::physics::player::stock_collections()?;
            let mut words = Words::new();
            for (class, name) in [(0x546c_36b6_5603_8e04, "Hash_546C36B656038E04"),
                (0x3492_15e2_e817_703c, "Hash_349215E2E817703C"),
                (skate3_hom::CLASS, "Hash_9E4C471BC6EAB615")] {
                let row = data.entries().iter().find(|r| r.class_name == name && r.key == "default")
                    .ok_or_else(|| format!("Missing embedded scoring default {name}"))?;
                for (field, value) in &row.fields {
                    if value.data.len() != 8 { continue; }
                    let field_key = if let Some(hash) = field.strip_prefix("Hash_") {
                        u64::from_str_radix(hash, 16).map_err(|e| e.to_string())?
                    } else { skate2_formats::attrib::string_hash64(field) };
                    let value = data.words::<1>(name, "default", field)?[0];
                    words.insert((class, field_key), value);
                }
            }
            Ok(words)
        }).map_err(ScoreError::NativeCatalog)?;
        words.get(&(self.class, key)).copied().ok_or_else(||
            ScoreError::NativeCatalog(format!("Missing Skate 3 scoring field {:016X}/{key:016X}", self.class)))
    }
}
impl std::ops::Deref for ScoringAttributes {
    type Target = AttributeRuntimeInstance;
    fn deref(&self) -> &Self::Target { &self.native }
}

// Raw lis/ori/insrdi operands:82df25f0/2640,82dedd68/ddb8,82dea488/668.
pub(super) const LINE_RATE: u64 = 0xfe02_a452_31f7_b06a;
pub(super) const LINE_MAX: u64 = 0x5747_8337_accf_ff18;
pub(super) const MOMENTUM_RATE: u64 = 0x8840_7506_dc97_80ed;
pub(super) const MOMENTUM_MAX: u64 = 0x2876_7a5f_961c_7129;
pub(super) const MOMENTUM_THRESHOLDS: [u64; 3] = [
    0x8298_87d8_c24a_5b8a,
    0xfb84_08a1_5c17_a9d4,
    0x30aa_e071_a786_8771,
];
pub(super) const MOMENTUM_MULTIPLIERS: [u64; 3] = [
    0x4752_0563_64ff_91fe,
    0xf499_3a13_ed7b_44c1,
    0xdc0c_3853_f6c5_fc2e,
];
pub(super) const MOMENTUM_ALLOW_DROP: u64 = 0xe361_97bf_c8ea_1cad;
