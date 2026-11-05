//! Named trigger volumes of a map: the `<Map>.triggers` sidecar written by the
//! setup pipeline (`tools/asset_pipeline/map_volumes.py`) or a `TVOL`
//! extension (schema 1, same JSON) embedded in a custom `.skate` package.
//!
//! Retail source: the 0x00EB0019 volume sets of each district's simulation
//! streams. A volume is a named oriented box with the retail instance id
//! (what challenge scripts address), the link GUID and its trigger group.
use crate::skate_map::SkateMap;
use serde::{Deserialize, Serialize};
use skate_core::triggers::OrientedBox;
use std::path::{Path, PathBuf};

pub const FORMAT: &str = "skate3rust-trigger-volumes";
pub const VERSION: u32 = 1;
pub const EXTENSION_TAG: [u8; 4] = *b"TVOL";
pub const EXTENSION_SCHEMA: u32 = 1;
pub const SIDECAR_EXTENSION: &str = "triggers";
/// Retail groups hold up to 256 volumes each (AddVolume 82DD7668).
pub const MAX_VOLUMES: usize = 4096;
const MAX_ID: usize = 128;
const MAX_NAME: usize = 512;

/// The trigger manager's three groups (`sub_82DD7AE0`). A retail item's group
/// word (+216) picks one (`82DD7C58`: 1 Stairs, 2 Camera, else Challenge); every
/// shipped item holds 0. Only Challenge tracks bodies: Stairs and Camera are
/// built without entity slots (group descriptors `0x82FCA108`), so they are
/// position/name lookups that never post enter/exit.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum TriggerGroup {
    #[default]
    Challenge,
    Stairs,
    Camera,
}

impl TriggerGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Challenge => "challenge",
            Self::Stairs => "stairs",
            Self::Camera => "camera",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "challenge" => Some(Self::Challenge),
            "stairs" => Some(Self::Stairs),
            "camera" => Some(Self::Camera),
            _ => None,
        }
    }
    /// Whether the group gives tracked bodies enter/exit events (retail: only
    /// the Challenge group has entity slots).
    pub fn tracks_bodies(self) -> bool {
        self == Self::Challenge
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum TriggerShape {
    Box {
        center: [f32; 3],
        #[serde(default = "identity_axes")]
        axes: [[f32; 3]; 3],
        half_extents: [f32; 3],
        #[serde(default)]
        fatness: f32,
    },
}

fn identity_axes() -> [[f32; 3]; 3] {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
}

impl TriggerShape {
    pub fn oriented_box(&self) -> OrientedBox {
        match *self {
            Self::Box {
                center,
                axes,
                half_extents,
                fatness,
            } => OrientedBox {
                center,
                axes,
                half_extents,
                fatness,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerVolumeRecord {
    /// Unique within the map. Retail: the instance id as 16 hex digits.
    pub id: String,
    /// Short name (`tut_sksc_reset_vol01`).
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_guid: Option<String>,
    #[serde(default)]
    pub group: TriggerGroup,
    pub shape: TriggerShape,
    /// Retail bounds (the broad phase); derived from the shape when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aabb: Option<Bounds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arena: Option<String>,
}

impl TriggerVolumeRecord {
    pub fn instance(&self) -> Option<u64> {
        self.instance_id.as_deref().and_then(parse_hex)
    }
    pub fn link(&self) -> Option<u64> {
        self.link_guid.as_deref().and_then(parse_hex)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerVolumeFile {
    pub format: String,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub volumes: Vec<TriggerVolumeRecord>,
}

fn parse_hex(text: &str) -> Option<u64> {
    let digits = text.strip_prefix("0x").unwrap_or(text);
    (!digits.is_empty() && digits.len() <= 16)
        .then(|| u64::from_str_radix(digits, 16).ok())
        .flatten()
}

pub fn validate_record(volume: &TriggerVolumeRecord) -> Result<(), String> {
    if volume.id.is_empty() || volume.id.len() > MAX_ID || volume.id.chars().any(char::is_control) {
        return Err(format!("Invalid trigger volume id {:?}", volume.id));
    }
    if volume.name.len() > MAX_NAME
        || volume
            .full_name
            .as_ref()
            .is_some_and(|n| n.len() > 4 * MAX_NAME)
    {
        return Err(format!("Trigger volume {} has an overlong name", volume.id));
    }
    for (what, value) in [
        ("instance_id", &volume.instance_id),
        ("link_guid", &volume.link_guid),
    ] {
        if value.as_deref().is_some_and(|v| parse_hex(v).is_none()) {
            return Err(format!(
                "Trigger volume {}: {what} is not a 64-bit hex id",
                volume.id
            ));
        }
    }
    if !volume.shape.oriented_box().is_valid() {
        return Err(format!("Trigger volume {} has an invalid box", volume.id));
    }
    if let Some(b) = &volume.aabb {
        if !b.min.iter().chain(b.max.iter()).all(|v| v.is_finite())
            || (0..3).any(|i| b.min[i] > b.max[i])
        {
            return Err(format!("Trigger volume {} has invalid bounds", volume.id));
        }
    }
    Ok(())
}

pub fn parse(bytes: &[u8]) -> Result<TriggerVolumeFile, String> {
    let file: TriggerVolumeFile =
        serde_json::from_slice(bytes).map_err(|e| format!("Trigger volumes: {e}"))?;
    if file.format != FORMAT {
        return Err(format!("Trigger volumes: unknown format {:?}", file.format));
    }
    if file.version != VERSION {
        return Err(format!(
            "Trigger volumes: version {} is not supported (reader supports {VERSION})",
            file.version
        ));
    }
    if file.volumes.len() > MAX_VOLUMES {
        return Err(format!(
            "Trigger volumes: {} volumes exceed the {MAX_VOLUMES} limit",
            file.volumes.len()
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for volume in &file.volumes {
        validate_record(volume)?;
        if !ids.insert(volume.id.as_str()) {
            return Err(format!("Trigger volumes: duplicate id {}", volume.id));
        }
    }
    Ok(file)
}

pub fn sidecar_path(map_path: &Path) -> PathBuf {
    map_path.with_extension(SIDECAR_EXTENSION)
}

/// Where a map's volumes came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Sidecar(PathBuf),
    Embedded,
    None,
}

/// The map's volumes: the sidecar next to the package wins over an embedded
/// `TVOL` extension (so a converted or edited sidecar can replace it); no
/// source means no volumes.
pub fn load_for_map(
    map_path: Option<&Path>,
    map: Option<&SkateMap>,
) -> Result<(Vec<TriggerVolumeRecord>, Origin), String> {
    if let Some(path) = map_path.map(sidecar_path).filter(|p| p.is_file()) {
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file = parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        return Ok((file.volumes, Origin::Sidecar(path)));
    }
    if let Some(extension) = map.and_then(|m| m.extensions.iter().find(|e| e.tag == EXTENSION_TAG))
    {
        if extension.schema != EXTENSION_SCHEMA {
            return Err(format!("TVOL schema {} is not supported", extension.schema));
        }
        return Ok((parse(&extension.payload)?.volumes, Origin::Embedded));
    }
    Ok((Vec::new(), Origin::None))
}
