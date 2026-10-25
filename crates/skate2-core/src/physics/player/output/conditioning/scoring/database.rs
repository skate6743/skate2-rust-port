//! TrickDatabase::ConstructFromAttribSys82dfb490 and TrickScoringSettings::Set82de9a70.
use super::{
    scorable::TrickSettings,
    scoring_attributes::ScoreError,
    scoring_catalog::{SCORABLES, info},
};
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault};
use std::{collections::BTreeMap, sync::Arc};
pub(crate) const TRICK_CLASS: u64 = 0x6918_4699_84a8_c596;

pub(super) struct TrickDatabase {
    vault: Arc<AttributeRuntimeVault>,
    indices: BTreeMap<u64, u32>,
    animation_ids: BTreeMap<crate::play_animation::FastString36, i32>,
}
impl TrickDatabase {
    pub(super) fn matches_vault(&self, vault: &Arc<AttributeRuntimeVault>) -> bool {
        Arc::ptr_eq(&self.vault, vault)
    }
    pub(super) fn load(vault: &Arc<AttributeRuntimeVault>) -> Result<Self, ScoreError> {
        let keys = vault.leaf_collections_sorted(TRICK_CLASS)?;
        // AnimScorableIDInitializer82dfd0f8 inserts all269 catalog names using
        // the same FastString encoding as animation, not a new string hash.
        let mut animation_ids = BTreeMap::new();
        for info in SCORABLES {
            let name = crate::play_animation::FastString36::encode(info.name)
                .map_err(|_| ScoreError::Invalid("scorable animation name"))?;
            animation_ids.entry(name).or_insert(info.id);
        }
        Ok(Self {
            vault: vault.clone(),
            indices: keys
                .into_iter()
                .enumerate()
                .map(|(i, k)| (k, i as u32))
                .collect(),
            animation_ids,
        })
    }
    pub(super) fn animation_id(&self, name: &crate::play_animation::FastString36) -> i32 {
        self.animation_ids.get(name).copied().unwrap_or(-1)
    }
    /// IsGrindGrab82defc10 reads the original trick layout byte148.
    pub(super) fn is_grind_grab(&self, id: i32) -> Result<bool, ScoreError> {
        let Some(item) = info(id).filter(|i| i.trick_type == 5) else {
            return Ok(false);
        };
        let key = skate2_formats::attrib::string_hash64(item.name);
        let attr = self.vault.instance(TRICK_CLASS, key);
        if attr.is_null() {
            return Ok(false);
        }
        Ok(attr
            .try_cached_layout_field(148, 1)?
            .is_some_and(|b| b.bytes()[0] != 0))
    }
    pub(super) fn lookup_id(
        &self,
        id: i32,
        settings: &mut TrickSettings,
    ) -> Result<(), ScoreError> {
        self.lookup(info(id).map_or("INVALID", |i| i.name), settings)
    }
    pub(super) fn lookup_index(
        &self,
        index: i32,
        settings: &mut TrickSettings,
    ) -> Result<(), ScoreError> {
        settings.is_valid = false;
        let Some((&key, &index)) = self.indices.iter().find(|(_, i)| **i as i32 == index) else {
            return Ok(());
        };
        self.lookup_key(key, index, settings)
    }
    /// The trick whose Vault collection key is `key`; invalid for a key not in the database.
    pub(super) fn lookup_attrib_key(
        &self,
        key: u64,
        settings: &mut TrickSettings,
    ) -> Result<(), ScoreError> {
        settings.is_valid = false;
        let Some(&index) = self.indices.get(&key) else {
            return Ok(());
        };
        self.lookup_key(key, index, settings)
    }
    pub(super) fn lookup(
        &self,
        name: &str,
        settings: &mut TrickSettings,
    ) -> Result<(), ScoreError> {
        let key = skate2_formats::attrib::string_hash64(&name.to_ascii_lowercase());
        settings.is_valid = false;
        let Some(&index) = self.indices.get(&key) else {
            return Ok(());
        };
        self.lookup_key(key, index, settings)
    }
    fn lookup_key(
        &self,
        key: u64,
        index: u32,
        settings: &mut TrickSettings,
    ) -> Result<(), ScoreError> {
        let attr = self.vault.instance(TRICK_CLASS, key);
        if attr.is_null() {
            return Ok(());
        }
        settings.is_valid = true;
        settings.trick_index = index;
        let layout = attr.try_cached_layout_field(8, 184)?;
        let zero = [0u8; 184];
        let b = layout.as_ref().map_or(zero.as_slice(), |v| v.bytes());
        let word = |off: usize| u32::from_be_bytes(b[off - 8..off - 4].try_into().unwrap());
        settings.scorable_id = word(136) as i32;
        settings.base_score = word(160) as i32;
        settings.trick_type = word(112) as i32;
        settings.fakie_variant_id = word(152) as i32;
        if settings.trick_type as u32 >= 14 {
            return Err(ScoreError::Invalid("TrickType"));
        }
        settings.trick_name_hal = layout_text(&attr, 0)?;
        settings.difficulty = word(116) as i32;
        settings.trick_tutorial_type =
            attr.try_scalar_word(0xb766_4cb6_ff57_98ea, 0)?.unwrap_or(0) as i32;
        settings.trick_description_hal = layout_text(&attr, 4)?;
        settings.substitute_trick_group = word(124);
        settings.min_trick_time = f32::from_bits(word(140));
        settings.trick_book_sorting_index = word(128);
        settings.trick_gesture = word(156) as i32;
        settings.trick_attrib_key = key;
        settings.max_impending_trick_frames = word(144);
        // Private.GetLength reads the header's second halfword, layout+10.
        let count = u16::from_be_bytes(b[2..4].try_into().unwrap()) as usize;
        for (i, t) in settings.skater_intent_tuples.iter_mut().enumerate() {
            let fields = if i < count {
                std::array::from_fn(|j| word(16 + i * 16 + j * 4))
            } else {
                [0; 4]
            };
            t.skater_intent = fields[0] as i32;
            t.value = f32::from_bits(fields[1]);
            t.start_frame = fields[2] as i32;
            t.end_frame = fields[3] as i32;
        }
        for (key, inputs) in [
            (0x03ac_c9f2_0080_7b87, &mut settings.trick_inputs),
            (0x6211_c49d_1f9b_0367, &mut settings.trick_inputs_goofy),
        ] {
            for (i, input) in inputs.iter_mut().enumerate() {
                let data = attr.try_plain_array_data(key, i as u32, 0xb8bc_832e_41d2_0873, 12)?;
                let zero = [0; 12];
                let b = data.as_ref().map_or(zero.as_slice(), |d| d.bytes());
                input.gesture = i32::from_be_bytes(b[0..4].try_into().unwrap());
                input.button = i32::from_be_bytes(b[4..8].try_into().unwrap());
                // Source does NOT copy connector or SwitchVariant in Set.
            }
        }
        Ok(())
    }
}
fn layout_text(
    attr: &AttributeRuntimeInstance,
    offset: u32,
) -> Result<Option<Arc<str>>, ScoreError> {
    let Some(data) = attr.try_cached_layout_text(offset)? else {
        return Ok(None);
    };
    let bytes = data.bytes();
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    Ok(Some(
        std::str::from_utf8(bytes)
            .map_err(|_| ScoreError::Invalid("trick localized string ID"))?
            .into(),
    ))
}
