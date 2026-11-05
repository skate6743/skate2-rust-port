//! Shader `cTextureMan` kind-0 table transitions; retained texture ownership is external.
//!
//! This does not construct the aliased `renderengine::Texture` product. `T` is
//! an identity token supplied by the retained processed-asset owner; the owner
//! must keep that exact texture allocation alive until `remove_texture` runs.

use std::collections::BTreeMap;

/// Source fixed-vector length and constructor loop bound at `82e1b534`.
pub const SHADER_TEXTURE_SLOT_COUNT: usize = 5120;

/// One native `TextureTable` slot, with the three independently mutable links.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShaderTextureSlot<T> {
    pub active: Option<T>,
    pub high: Option<T>,
    pub low: Option<T>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShaderTextureAddError {
    NoFreeSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShaderTextureRemoveTransition {
    Missing,
    Retained,
    Freed,
}

/// The low-resolution bit of a texture ID: bit 0, as Skate 2's `FindTexture` clears it. PC port: a
/// linked Skate 3 data folder's textures spend bit 0 on their IDs and mark the low-resolution
/// texture with bit 62 instead (Skate 3's `TextureMan` slot lookup `sub_8280AC58` keys by the ID
/// with it clear, its arena registration 0x8279ee58 tests it), so two neighbouring textures stay
/// distinct. Which game a texture is comes from where it was loaded: its IDs say nothing of it.
#[must_use]
pub const fn low_resolution_bit(skate3: bool) -> u64 {
    if skate3 { 1 << 62 } else { 1 }
}

/// Table keys use the full 64-bit TOC texture ID with only its low-resolution bit clear (PC port:
/// apart for each game, whose IDs share no meaning).
///
/// Native constructor `82e1b400` pushes free indexes `0..5120` ascending;
/// `AddTexture` pops the last, so the first new key receives slot 5119.
#[derive(Debug)]
pub struct ShaderTextureTable<T> {
    slots: Vec<Option<ShaderTextureSlot<T>>>,
    free_indices: Vec<usize>,
    by_normalized_id: BTreeMap<(bool, u64), usize>,
    null_texture: Option<T>,
}

impl<T: Copy + Eq> ShaderTextureTable<T> {
    /// `cTextureMan` constructor `82e1b400`; null texture starts raw null.
    pub fn new() -> Self {
        Self {
            slots: vec![None; SHADER_TEXTURE_SLOT_COUNT],
            free_indices: (0..SHADER_TEXTURE_SLOT_COUNT).collect(),
            by_normalized_id: BTreeMap::new(),
            null_texture: None,
        }
    }

    /// Source `BeginFrame82e13498` replaces the live missing-texture pointer.
    pub fn begin_frame(&mut self, missing_texture: Option<T>) {
        self.null_texture = missing_texture;
    }

    /// Source `FindTexture82e1a090`: `clrrdi` clears bit 0 of the full ID.
    pub fn find(&self, texture_asset_id: u64) -> Option<(usize, ShaderTextureSlot<T>)> {
        self.find_from(texture_asset_id, false)
    }

    /// [`Self::find`] among one game's textures (PC port: a linked Skate 3 data folder's clear
    /// their own low-resolution bit, [`low_resolution_bit`]).
    pub fn find_from(&self, texture_asset_id: u64, skate3: bool) -> Option<(usize, ShaderTextureSlot<T>)> {
        let index = *self.by_normalized_id.get(&(skate3, texture_asset_id & !low_resolution_bit(skate3)))?;
        Some((
            index,
            self.slots[index].expect("mapped texture slot exists"),
        ))
    }

    /// Source `AddTexture82e1b058`: new slot sets all three pointers; an odd
    /// existing ID replaces low only, and an even one replaces high and active.
    pub fn add_texture(
        &mut self,
        texture_asset_id: u64,
        texture: T,
    ) -> Result<usize, ShaderTextureAddError> {
        self.add_texture_from(texture_asset_id, texture, false)
    }

    /// [`Self::add_texture`] for one game's texture.
    pub fn add_texture_from(
        &mut self,
        texture_asset_id: u64,
        texture: T,
        skate3: bool,
    ) -> Result<usize, ShaderTextureAddError> {
        let low_resolution = low_resolution_bit(skate3);
        let key = (skate3, texture_asset_id & !low_resolution);
        if let Some(&index) = self.by_normalized_id.get(&key) {
            let slot = self.slots[index]
                .as_mut()
                .expect("mapped texture slot exists");
            if texture_asset_id & low_resolution != 0 {
                slot.low = Some(texture);
            } else {
                slot.high = Some(texture);
                slot.active = Some(texture);
            }
            return Ok(index);
        }
        let index = self
            .free_indices
            .pop()
            .ok_or(ShaderTextureAddError::NoFreeSlot)?;
        assert!(
            self.slots[index].is_none(),
            "free texture index is occupied"
        );
        self.by_normalized_id.insert(key, index);
        self.slots[index] = Some(ShaderTextureSlot {
            active: Some(texture),
            high: Some(texture),
            low: Some(texture),
        });
        Ok(index)
    }

    /// Source `RemTexture82e1b548`: replace a matching pointer or raw null
    /// with the *current* BeginFrame null texture. A prior nonzero null-texture
    /// pointer does not satisfy the raw-null check after BeginFrame changes it.
    pub fn remove_texture(
        &mut self,
        texture_asset_id: u64,
        texture: T,
    ) -> ShaderTextureRemoveTransition {
        let key = (false, texture_asset_id & !low_resolution_bit(false));
        let Some(&index) = self.by_normalized_id.get(&key) else {
            return ShaderTextureRemoveTransition::Missing;
        };
        let slot = self.slots[index]
            .as_mut()
            .expect("mapped texture slot exists");
        if slot.low == Some(texture) || slot.low.is_none() {
            slot.low = self.null_texture;
        }
        if slot.high == Some(texture) || slot.high.is_none() {
            slot.high = self.null_texture;
            slot.active = slot.low;
        }
        if slot.high != self.null_texture || slot.low != self.null_texture {
            return ShaderTextureRemoveTransition::Retained;
        }
        self.by_normalized_id.remove(&key);
        self.slots[index] = None;
        self.free_indices.push(index);
        ShaderTextureRemoveTransition::Freed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_slot_order_full_width_keys_and_even_odd_updates() {
        let mut table = ShaderTextureTable::new();
        let first = 0x1111_2222_3333_4444;
        let other = 0x9999_aaaa_3333_4444;
        assert_eq!(table.add_texture(first, 10), Ok(5119));
        assert_eq!(table.add_texture(other | 1, 20), Ok(5118));
        assert_eq!(table.add_texture(first | 1, 11), Ok(5119));
        assert_eq!(
            table.find(first),
            Some((
                5119,
                ShaderTextureSlot {
                    active: Some(10),
                    high: Some(10),
                    low: Some(11)
                }
            ))
        );
        assert_eq!(table.add_texture(first, 12), Ok(5119));
        assert_eq!(
            table.find(first | 1),
            Some((
                5119,
                ShaderTextureSlot {
                    active: Some(12),
                    high: Some(12),
                    low: Some(11)
                }
            ))
        );
        assert_eq!(table.find(other).unwrap().1.high, Some(20));
    }

    #[test]
    fn nonzero_null_texture_survives_unrelated_removal_after_begin_frame_change() {
        let mut table = ShaderTextureTable::new();
        let key = 0x1234_5678_9abc_def0;
        table.begin_frame(Some(99));
        assert_eq!(table.add_texture(key, 10), Ok(5119));
        assert_eq!(table.add_texture(key | 1, 11), Ok(5119));
        assert_eq!(
            table.remove_texture(key, 10),
            ShaderTextureRemoveTransition::Retained
        );
        assert_eq!(
            table.find(key).unwrap().1,
            ShaderTextureSlot {
                active: Some(11),
                high: Some(99),
                low: Some(11)
            }
        );
        table.begin_frame(Some(100));
        assert_eq!(
            table.remove_texture(key, 12),
            ShaderTextureRemoveTransition::Retained
        );
        assert_eq!(table.find(key).unwrap().1.high, Some(99));
        assert_eq!(
            table.remove_texture(key | 1, 11),
            ShaderTextureRemoveTransition::Retained
        );
        assert_eq!(table.find(key).unwrap().1.low, Some(100));
        assert_eq!(table.find(key).unwrap().1.high, Some(99));
        assert_eq!(
            table.remove_texture(key, 99),
            ShaderTextureRemoveTransition::Freed
        );
        assert!(table.find(key).is_none());
        assert_eq!(table.add_texture(0xfedc_ba98_7654_3210, 12), Ok(5119));
    }

    #[test]
    fn raw_null_pointer_is_replaced_by_live_begin_frame_null_texture() {
        let mut table = ShaderTextureTable::new();
        let key = 0x1234_5678_9abc_def0;
        assert_eq!(table.add_texture(key, 10), Ok(5119));
        assert_eq!(table.add_texture(key | 1, 11), Ok(5119));
        assert_eq!(
            table.remove_texture(key, 10),
            ShaderTextureRemoveTransition::Retained
        );
        assert_eq!(table.find(key).unwrap().1.high, None);
        table.begin_frame(Some(99));
        assert_eq!(
            table.remove_texture(key, 12),
            ShaderTextureRemoveTransition::Retained
        );
        assert_eq!(table.find(key).unwrap().1.high, Some(99));
        assert_eq!(table.find(key).unwrap().1.active, Some(11));
    }

    #[test]
    fn exhausted_free_list_does_not_change_existing_registration() {
        let mut table = ShaderTextureTable::new();
        for index in 0..SHADER_TEXTURE_SLOT_COUNT {
            assert_eq!(
                table.add_texture((index as u64 + 1) << 1, index),
                Ok(SHADER_TEXTURE_SLOT_COUNT - 1 - index)
            );
        }
        assert_eq!(
            table.add_texture(0x1_0000, SHADER_TEXTURE_SLOT_COUNT),
            Err(ShaderTextureAddError::NoFreeSlot)
        );
        assert_eq!(table.find(2).unwrap().1.active, Some(0));
        assert!(table.find(0x1_0000).is_none());
        assert_eq!(
            table.remove_texture(0x1_0000, SHADER_TEXTURE_SLOT_COUNT),
            ShaderTextureRemoveTransition::Missing
        );
    }
}
