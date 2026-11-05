//! Lossless stock physics-bone records. Core consumes typed values derived here.
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsBone {
    pub name: String,
    pub source_offset: u64,
    pub words: [u32; 28],
}
impl PhysicsBone {
    pub fn size(&self) -> [f32; 3] {
        [self.words[8], self.words[9], self.words[10]].map(f32::from_bits)
    }
    pub fn rotation(&self) -> [f32; 4] {
        [
            self.words[12],
            self.words[13],
            self.words[14],
            self.words[15],
        ]
        .map(f32::from_bits)
    }
    pub fn translation(&self) -> [f32; 3] {
        [self.words[16], self.words[17], self.words[18]].map(f32::from_bits)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsSkeleton {
    pub name: String,
    pub source_offset: u64,
    pub bones: Vec<PhysicsBone>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    source_sha256: String,
    skeletons: Vec<PhysicsSkeleton>,
}

impl PhysicsSkeleton {
    pub fn load(path: &Path, bank_sha256: &str, name: &str) -> Result<Self, String> {
        let linked = path
            .ancestors()
            .nth(3)
            .filter(|root| crate::asset_io::is_linked_root(root));
        let file = if let Some(root) = linked {
            // The linked OnBoard.abin is the very bank the animation data was read from.
            let bank = root.join("private/stock/data/anim/OnBoard.abin");
            let bytes = crate::asset_io::read_shared(&bank)
                .map_err(|error| format!("{}: {error}", bank.display()))?;
            File { version: 1, source_sha256: bank_sha256.to_owned(), skeletons: from_bank(&bytes)? }
        } else {
            let bytes = crate::asset_io::read(path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?
        };
        if file.version != 1 || file.source_sha256 != bank_sha256 {
            return Err(
                "Physics skeleton bank identity or format differs from animation data".into(),
            );
        }
        let mut identities = BTreeSet::new();
        if file.skeletons.iter().any(|s| !identities.insert(&s.name)) {
            return Err("Converted physics skeleton has unresolved duplicate names".into());
        }
        let selected = file
            .skeletons
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Required stock physics skeleton {name} is absent"))?;
        let mut names = BTreeSet::new();
        if selected.bones.is_empty() || selected.bones.len() > 255 {
            return Err("Invalid stock physical bone count".into());
        }
        for bone in &selected.bones {
            if !names.insert(&bone.name)
                || bone
                    .size()
                    .iter()
                    .chain(&bone.rotation())
                    .chain(&bone.translation())
                    .any(|v| !v.is_finite())
            {
                return Err(format!(
                    "Invalid or repeated stock physics bone {}",
                    bone.name
                ));
            }
        }
        Ok(selected)
    }
}

/// FastString words (base 38, six digits each), as the bank stores physics part names.
fn fast_name(data: &[u8], at: usize, words: usize) -> Result<String, String> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_";
    let mut result = String::new();
    for index in 0..words {
        let mut word = word_at(data, at + index * 4)?;
        let mut divisor = 38u32.pow(5);
        for _ in 0..6 {
            let digit = word / divisor;
            word %= divisor;
            if digit == 0 {
                if word != 0 {
                    return Err("Invalid FastString padding".into());
                }
                break;
            }
            let letter = ALPHABET.get(digit as usize - 1).ok_or("Invalid FastString digit")?;
            result.push(char::from(*letter));
            divisor /= 38;
        }
    }
    Ok(result)
}

fn word_at(data: &[u8], at: usize) -> Result<u32, String> {
    data.get(at..at + 4)
        .map(|bytes| u32::from_be_bytes(bytes.try_into().expect("4 bytes")))
        .ok_or_else(|| format!("Truncated animation bank at {at:#x}"))
}

/// The bank's physics skeleton records (kind 5), as the old asset pipeline's
/// `physics_skeleton.py` wrote them to `physics-skeletons.json`.
fn from_bank(data: &[u8]) -> Result<Vec<PhysicsSkeleton>, String> {
    let end = word_at(data, 0)? as usize;
    if word_at(data, 4)? != 1 || end > data.len() {
        return Err("Invalid animation bank".into());
    }
    let mut skeletons = Vec::new();
    let mut at = 48;
    while at < end {
        let size = word_at(data, at)? as usize;
        let kind = word_at(data, at + 4)?;
        if size < 48 || size % 16 != 0 || at + size > end {
            return Err("Invalid ABIN record".into());
        }
        if kind == 5 {
            let payload = (at + 55) & !15;
            let count = word_at(data, payload)? as usize;
            let start = payload + 16;
            if start + count * 112 > data.len() {
                return Err("Truncated physics skeleton record".into());
            }
            let mut bones = Vec::with_capacity(count);
            for index in 0..count {
                let pos = start + index * 112;
                let mut words = [0; 28];
                for (word, slot) in words.iter_mut().enumerate() {
                    *slot = word_at(data, pos + word * 4)?;
                }
                bones.push(PhysicsBone { name: fast_name(data, pos + 92, 5)?, source_offset: pos as u64, words });
            }
            skeletons.push(PhysicsSkeleton { name: fast_name(data, at + 16, 6)?, source_offset: at as u64, bones });
        }
        at += size;
    }
    if skeletons.is_empty() {
        return Err("No physics skeletons in animation bank".into());
    }
    Ok(skeletons)
}
