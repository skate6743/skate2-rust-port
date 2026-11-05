//! Runtime clip metadata read directly from the stock Andale bank.
//! Clock/attribute evaluation belongs to core/game; this loader preserves bits
//! and record order and rejects malformed or ambiguous source data.
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
mod blend_space;
mod native;
pub use blend_space::{BlendSimplexMetadata, BlendSpaceMetadata};
mod selection_space;
pub use selection_space::{
    SelectionCandidateMetadata, SelectionParameterMetadata, SelectionSpaceMetadata,
};

/// Bank-relative record offsets are meaningful only with this source identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BankSource {
    pub source_bank: String,
    pub source_sha256: String,
    pub source_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipAttribute {
    pub name: String,
    pub type_id: u8,
    pub begin_bits: u32,
    pub end_bits: u32,
    /// Raw aligned payload, including opaque curve data and padding. Consumers
    /// select the lanes written by Attribute::Init for this type.
    pub payload_words: Vec<u32>,
    pub source_offset: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipMetadata {
    pub name: String,
    pub source_offset: u64,
    pub fps_bits: u32,
    pub frames_bits: u32,
    pub base_speed_bits: u32,
    pub flags_word: u32,
    pub attributes: Vec<ClipAttribute>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseBlendMetadata {
    pub name: String,
    pub source_offset: u64,
    pub parameter: String,
    pub children: Vec<String>,
}
pub enum TreeMetadata<'a> {
    Clip(&'a ClipMetadata),
    BlendSpace(&'a BlendSpaceMetadata),
    PhaseBlend(&'a PhaseBlendMetadata),
    Selector(&'a SelectorMetadata),
    SelectionSpace(&'a SelectionSpaceMetadata),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectorMetadata {
    pub name: String,
    pub source_offset: u64,
    pub parameter: String,
    pub default: String,
    pub children: Vec<String>,
    pub values: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnsupportedTree {
    name: String,
    source_offset: u64,
    type_id: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    source_bank: String,
    source_sha256: String,
    source_bytes: u64,
    clips: Vec<ClipMetadata>,
    #[serde(default)]
    phase_blends: Vec<PhaseBlendMetadata>,
    #[serde(default)]
    blend_spaces: Vec<BlendSpaceMetadata>,
    #[serde(default)]
    selectors: Vec<SelectorMetadata>,
    #[serde(default)]
    selection_spaces: Vec<SelectionSpaceMetadata>,
    unsupported_trees: Vec<UnsupportedTree>,
}

#[derive(Debug)]
pub struct AnimationMetadata {
    pub source_bank: String,
    pub source_sha256: String,
    clips: BTreeMap<String, Vec<ClipMetadata>>,
    phase_blends: BTreeMap<String, Vec<PhaseBlendMetadata>>,
    blend_spaces: BTreeMap<String, Vec<BlendSpaceMetadata>>,
    selectors: BTreeMap<String, Vec<SelectorMetadata>>,
    selection_spaces: BTreeMap<String, Vec<SelectionSpaceMetadata>>,
    unsupported: BTreeMap<String, Vec<(u64, u32)>>,
    sources: Vec<BankSource>,
    origins: BTreeMap<String, usize>,
}

impl AnimationMetadata {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = crate::asset_io::read_to_string(path)
            .map_err(|e| format!("Cannot read animation metadata {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let file: File = serde_json::from_str(text).map_err(|e| e.to_string())?;
        Self::from_file(file)
    }

    fn from_file(file: File) -> Result<Self, String> {
        if file.version != 1 {
            return Err(format!(
                "Unsupported animation metadata version {}",
                file.version
            ));
        }
        if file.source_bank.is_empty()
            || file.source_sha256.len() != 64
            || !file.source_sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || file.source_bytes < 48
        {
            return Err("Invalid animation bank source identity".into());
        }
        let mut clips: BTreeMap<String, Vec<ClipMetadata>> = BTreeMap::new();
        for clip in file.clips {
            validate_name(&clip.name, 36)?;
            let fps = f32::from_bits(clip.fps_bits);
            let frames = f32::from_bits(clip.frames_bits);
            let base = f32::from_bits(clip.base_speed_bits);
            if !fps.is_finite()
                || fps <= 0.0
                || !frames.is_finite()
                || frames < 1.0
                || !base.is_finite()
                || base <= 0.0
                || clip.source_offset >= file.source_bytes
            {
                return Err(format!("{}: invalid clip header", clip.name));
            }
            let mut previous = clip.source_offset;
            for attribute in &clip.attributes {
                validate_name(&attribute.name, 30)?;
                if !f32::from_bits(attribute.begin_bits).is_finite()
                    || !f32::from_bits(attribute.end_bits).is_finite()
                    || attribute.source_offset <= previous
                    || attribute.source_offset >= file.source_bytes
                {
                    return Err(format!("{}: invalid attribute record/order", clip.name));
                }
                let minimum = match attribute.type_id {
                    0 => 1,
                    1 => 4,
                    3 => 6,
                    _ => 0,
                };
                if attribute.payload_words.len() < minimum {
                    return Err(format!(
                        "{}: truncated {} payload",
                        clip.name, attribute.name
                    ));
                }
                previous = attribute.source_offset;
            }
            clips.entry(clip.name.clone()).or_default().push(clip);
        }
        let mut phase_blends: BTreeMap<String, Vec<PhaseBlendMetadata>> = BTreeMap::new();
        for tree in file.phase_blends {
            validate_name(&tree.name, 36)?;
            validate_name(&tree.parameter, 30)?;
            if tree.children.len() < 2 {
                return Err(format!("{}: PhaseBlend needs two children", tree.name));
            }
            if tree.source_offset >= file.source_bytes {
                return Err(format!("{}: tree offset outside bank", tree.name));
            }
            for child in &tree.children {
                validate_name(child, 36)?;
            }
            phase_blends
                .entry(tree.name.clone())
                .or_default()
                .push(tree);
        }
        let mut blend_spaces: BTreeMap<String, Vec<BlendSpaceMetadata>> = BTreeMap::new();
        for tree in file.blend_spaces {
            tree.validate(file.source_bytes)?;
            blend_spaces
                .entry(tree.name.clone())
                .or_default()
                .push(tree);
        }
        let mut selectors: BTreeMap<String, Vec<SelectorMetadata>> = BTreeMap::new();
        for tree in file.selectors {
            validate_name(&tree.name, 36)?;
            validate_name(&tree.parameter, 30)?;
            validate_name(&tree.default, 36)?;
            if tree.children.len() != tree.values.len() {
                return Err(format!("{}: selector key/child count mismatch", tree.name));
            }
            if tree.source_offset >= file.source_bytes {
                return Err(format!("{}: tree offset outside bank", tree.name));
            }
            for child in &tree.children {
                validate_name(child, 36)?;
            }
            for value in &tree.values {
                validate_name(value, 30)?;
            }
            selectors.entry(tree.name.clone()).or_default().push(tree);
        }
        let mut selection_spaces: BTreeMap<String, Vec<SelectionSpaceMetadata>> = BTreeMap::new();
        for tree in file.selection_spaces {
            tree.validate(file.source_bytes)?;
            selection_spaces
                .entry(tree.name.clone())
                .or_default()
                .push(tree);
        }
        let mut unsupported: BTreeMap<String, Vec<(u64, u32)>> = BTreeMap::new();
        for tree in file.unsupported_trees {
            validate_name(&tree.name, 36)?;
            if tree.source_offset >= file.source_bytes {
                return Err(format!("{}: tree offset outside bank", tree.name));
            }
            unsupported
                .entry(tree.name)
                .or_default()
                .push((tree.source_offset, tree.type_id));
        }
        let origins = clips
            .keys()
            .chain(phase_blends.keys())
            .chain(blend_spaces.keys())
            .chain(selectors.keys())
            .chain(selection_spaces.keys())
            .chain(unsupported.keys())
            .map(|name| (name.clone(), 0))
            .collect();
        let sources = vec![BankSource {
            source_bank: file.source_bank.clone(),
            source_sha256: file.source_sha256.clone(),
            source_bytes: file.source_bytes,
        }];
        Ok(Self {
            source_bank: file.source_bank,
            source_sha256: file.source_sha256,
            clips,
            phase_blends,
            blend_spaces,
            selectors,
            selection_spaces,
            unsupported,
            sources,
            origins,
        })
    }

    /// Combine disjoint animation namespaces without comparing offsets from
    /// different files. Same-bank duplicates still obey native last-record wins.
    /// All collision checks precede mutation; the primary rig identity remains.
    pub fn merge(&mut self, other: Self) -> Result<(), String> {
        if let Some(name) = other
            .origins
            .keys()
            .find(|name| self.origins.contains_key(*name))
        {
            return Err(format!("Animation bank merge collides at {name}"));
        }
        let base = self.sources.len();
        self.origins.extend(
            other
                .origins
                .into_iter()
                .map(|(name, index)| (name, base + index)),
        );
        self.sources.extend(other.sources);
        self.clips.extend(other.clips);
        self.phase_blends.extend(other.phase_blends);
        self.blend_spaces.extend(other.blend_spaces);
        self.selectors.extend(other.selectors);
        self.selection_spaces.extend(other.selection_spaces);
        self.unsupported.extend(other.unsupported);
        Ok(())
    }

    pub fn sources(&self) -> &[BankSource] {
        &self.sources
    }

    pub fn source_for(&self, name: &str) -> Option<&BankSource> {
        self.origins
            .get(&name.to_ascii_uppercase())
            .map(|&index| &self.sources[index])
    }

    /// The serialized names use FastString's canonical uppercase alphabet.
    /// The API accepts only that alphabet plus lowercase ASCII, so uppercase
    /// lookup is equivalent to native FastString encoding for accepted inputs.
    pub fn clip(&self, name: &str) -> Result<&ClipMetadata, String> {
        let name = name.to_ascii_uppercase();
        validate_name(&name, 36)?;
        // SetDBContent82D1B428..434 updates its separate clip map on every
        // clip record; repeated keys overwrite with the later bank record.
        self.clips
            .get(&name)
            .and_then(|clips| clips.iter().max_by_key(|c| c.source_offset))
            .ok_or_else(|| format!("Missing direct clip {name}"))
    }
    pub fn tree(&self, name: &str) -> Result<TreeMetadata<'_>, String> {
        let name = name.to_ascii_uppercase();
        validate_name(&name, 36)?;
        let clips = self.clips.get(&name).map(Vec::as_slice).unwrap_or(&[]);
        let trees = self
            .unsupported
            .get(&name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let phase_blends = self
            .phase_blends
            .get(&name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let blend_spaces = self
            .blend_spaces
            .get(&name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let selectors = self.selectors.get(&name).map(Vec::as_slice).unwrap_or(&[]);
        let selection_spaces = self
            .selection_spaces
            .get(&name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        // TU382D1B438..45C inserts into the tree map in file order and writes
        // the record pointer even when map[]82D27320 finds an existing key.
        let last = clips
            .iter()
            .map(|c| c.source_offset)
            .chain(phase_blends.iter().map(|t| t.source_offset))
            .chain(blend_spaces.iter().map(|t| t.source_offset))
            .chain(selectors.iter().map(|t| t.source_offset))
            .chain(selection_spaces.iter().map(|t| t.source_offset))
            .chain(trees.iter().map(|t| t.0))
            .max();
        if let Some(clip) = clips.iter().find(|c| Some(c.source_offset) == last) {
            return Ok(TreeMetadata::Clip(clip));
        }
        if let Some(tree) = phase_blends.iter().find(|t| Some(t.source_offset) == last) {
            return Ok(TreeMetadata::PhaseBlend(tree));
        }
        if let Some(tree) = blend_spaces.iter().find(|t| Some(t.source_offset) == last) {
            return Ok(TreeMetadata::BlendSpace(tree));
        }
        if let Some(tree) = selectors.iter().find(|t| Some(t.source_offset) == last) {
            return Ok(TreeMetadata::Selector(tree));
        }
        if let Some(tree) = selection_spaces
            .iter()
            .find(|t| Some(t.source_offset) == last)
        {
            return Ok(TreeMetadata::SelectionSpace(tree));
        }
        if let Some((_, kind)) = trees.iter().find(|t| Some(t.0) == last) {
            return Err(format!(
                "Animation {name} is tree type {kind}; tree evaluation is required"
            ));
        }
        Err(format!("Missing animation {name} in {}", self.source_bank))
    }

    pub fn clip_count(&self) -> usize {
        self.clips.values().map(Vec::len).sum()
    }
}

fn validate_name(name: &str, length: usize) -> Result<(), String> {
    if name.is_empty()
        || name.len() > length
        || !name
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
    {
        return Err(format!("Invalid canonical FastString name {name:?}"));
    }
    Ok(())
}
