//! CAC installed choices and native assembly dependencies. Corrected display
//! parts are distinct from saved choices, as in CASPartContainer825932f8.
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault};
use skate2_formats::{
    cac_parts::{CacPart, CacPartDb, SLOT_NAMES},
    recipe::customizations::{CharacterCustomizations, Colour},
    skater_profile::{CacPartId, CacRecord, RecipeData},
};
use std::sync::Arc;

/// PC port: Skate 3's `bottom` colour zone, for a linked Skate 3 data folder's colourizable
/// legs. Skate 2 has twelve zones and no colourizable legs, so a skater holds this one only
/// once a colour is picked, in its customizations alone: `colour_ids` and an outfit's colours
/// keep Skate 2's twelve.
pub(crate) const BOTTOM_COLOUR: usize = 12;
/// PC port: Apply Color's colour filters (`edit_skater_colour_filter.rs`), of a shirt (the outer
/// torso) and of pants with no colour of their own: customization colours bound to the garment each
/// was picked for, as a zone's colour is bound to its part, whose base the cloth shader multiplies
/// the garment's diffuse by. Neither is one of Skate 2's twelve zones, so `colour_ids` and an
/// outfit's colours hold neither.
pub(crate) const TOP_FILTER: usize = 13;
pub(crate) const BOTTOM_FILTER: usize = 14;
pub(crate) const MORPH_CLASS: u64 = 0x7f47_ca62_f410_0f41;
pub(crate) const COLOUR_ZONE_CLASS: u64 = 0xae1f_950c_a0d1_adf2;
pub(crate) const COLOUR_CLASS: u64 = 0x6d93_4d0e_38c9_5e19;
pub(crate) type RecordResult<T> = Result<T, Box<dyn std::error::Error>>;
const MISSING: u16 = u16::MAX;
// RawPartDataV6 fields starting at+1c, verified against PDB.
const ALTERNATE: usize = 0;
const ARM_REQUIRED: usize = 1;
const ARM_TYPE: usize = 2;
const EYEBROW_STYLE: usize = 9;
const FACIAL_STYLE: usize = 12;
const GENDER: usize = 13;
const SHADOW_COLOUR: usize = 14;
const SHADOW_STYLE: usize = 15;
const HAIR_TYPE: usize = 16;
const HAIR_STYLE: usize = 17;
const INNER_CUT: usize = 20;
const INNER_REQUIRED: usize = 21;
const LEG_REQUIRED: usize = 22;
const LEG_TYPE: usize = 23;
const MATERIAL_NAME: usize = 26;
const MODEL_NAME: usize = 27;
const OUTER_MATERIAL: usize = 28;
const OUTER_MODEL: usize = 29;
const SHADOW_REQUIRED: usize = 30;
const HAIR_REQUIRED: usize = 31;
const REMOVALS: usize = 32;
const SOCK_REQUIRED: usize = 33;
const SKIN_NORMAL: usize = 34;
const SKIN_TONE: usize = 35;
const SOCK_STYLE: usize = 36;
const WEAR_GROUP: usize = 38;
const WRIST: usize = 39;
fn truth(value: u8) -> bool {
    value != 0 && value != 255
}
fn id(p: &CacPart) -> CacPartId {
    CacPartId {
        model: p.model_id,
        material: p.material_id,
    }
}
fn part<'a>(db: &'a CacPartDb, ids: &[CacPartId; 26], slot: usize) -> Option<&'a CacPart> {
    let p = ids[slot];
    db.find(slot as u8, p.model, p.material)
}
fn index(db: &CacPartDb, text: &str) -> u16 {
    db.strings
        .iter()
        .position(|s| s == text)
        .and_then(|i| u16::try_from(i).ok())
        .unwrap_or(MISSING)
}
fn group(p: &CacPart) -> u16 {
    if p.attributes[WEAR_GROUP] == MISSING && p.slot == 18 {
        p.attributes[MATERIAL_NAME]
    } else {
        p.attributes[WEAR_GROUP]
    }
}

/// GetDefaultPartID82b68f78: first matching gender/default, then first
/// default wear style within the same gender/group; original part fallback.
pub(crate) fn default_part(db: &CacPartDb, slot: u8, is_male: bool) -> Option<&CacPart> {
    let gender = index(db, if is_male { "male" } else { "female" });
    let male = index(db, "male");
    let female = index(db, "female");
    let p = db.parts_in_slot(slot).find(|p| {
        truth(p.default)
            && (p.attributes[GENDER] == gender
                || (p.attributes[GENDER] != male && p.attributes[GENDER] != female))
    })?;
    let g = group(p);
    if g == MISSING {
        return Some(p);
    }
    Some(
        db.parts_in_slot(slot)
            .find(|q| {
                q.attributes[GENDER] == p.attributes[GENDER]
                    && group(q) == g
                    && truth(q.default_wear_style)
            })
            .unwrap_or(p),
    )
}
pub(crate) fn default_parts(db: &CacPartDb, is_male: bool) -> [CacPartId; 26] {
    std::array::from_fn(|slot| {
        default_part(db, slot as u8, is_male)
            .map(id)
            .unwrap_or_else(CacPartId::invalid)
    })
}

/// SetInstalledPartID82b695d0 removes the requested part from owned inventory,
/// moves the previous installed part into inventory, then installs the choice.
pub(crate) fn wear_part(
    record: &mut CacRecord,
    db: &CacPartDb,
    choice: &CacPart,
) -> RecordResult<()> {
    let slot = usize::from(choice.slot);
    if slot >= 26
        || db
            .find(choice.slot, choice.model_id, choice.material_id)
            .is_none()
    {
        return Err("unknown CAC part".into());
    }
    let next = id(choice);
    let invalid = CacPartId::invalid();
    if next == invalid {
        return Ok(());
    }
    let mut owned = record.owned_parts[slot].clone();
    if let Some(i) = owned.iter().position(|&x| x == next) {
        owned.remove(i);
    }
    let previous = record.installed_parts[slot];
    if previous != invalid && previous != next {
        owned.push(previous);
    }
    if record.owned_parts.iter().map(Vec::len).sum::<usize>() - record.owned_parts[slot].len()
        + owned.len()
        >= 6000
    {
        return Err("CAC inventory capacity reached".into());
    }
    record.owned_parts[slot] = owned;
    record.installed_parts[slot] = next;
    record.rebuild_unused_owned_slots(invalid)?;
    Ok(())
}
/// CanRemoveItem82642e30 allows the assembler's nullable slots AND OUTER_TORSO.
pub(crate) fn can_remove_part(slot: u8) -> bool {
    matches!(
        slot,
        2 | 4 | 6 | 9 | 10 | 14 | 16 | 17 | 18 | 20 | 21 | 22 | 23 | 24 | 25
    )
}
/// Removals use ExchangePart, not invalid WearPart.
pub(crate) fn remove_part(record: &mut CacRecord, db: &CacPartDb,
    undo: &CacRecord, slot: u8) -> RecordResult<()> {
    if !can_remove_part(slot) {
        return Err("CAC slot cannot be empty".into());
    }
    // CASSimManager::RemovePart82598cc0..8ea8: OUTER_TORSO is required
    // by assembly even when the player chooses Remove. Install the first
    // skinned torso of the current arms' gender, using their skin/normal.
    // With no arms, use the undo-point torso; failed selection restores it.
    let replacement = if slot == 23 {
        let reference = part(db, &record.installed_parts, 0)
            .or_else(|| part(db, &undo.installed_parts, 23))
            .ok_or("CAC shirt removal has no arm or undo torso")?;
        db.parts_in_slot(23)
            .find(|p| truth(p.skinned) && p.attributes[GENDER] == reference.attributes[GENDER])
            .map(|p| alternate_skin(db, p,
                reference.attributes[SKIN_TONE], reference.attributes[SKIN_NORMAL]))
            .or_else(|| part(db, &undo.installed_parts, 23))
    } else { None };
    exchange_empty_part(record, slot)?;
    if let Some(replacement) = replacement { wear_part(record, db, replacement)?; }
    Ok(())
}

/// ExchangePart82b6bfd8 accepts an invalid ID for every slot. The nullable
/// restriction belongs to the player's Remove command, not InitializePlayer
/// 8258fc48..fcb4, which exchanges slots with no authored gender default.
fn exchange_empty_part(record: &mut CacRecord, slot: u8) -> RecordResult<()> {
    let slot = usize::from(slot);
    if slot >= record.installed_parts.len() { return Err("invalid CAC part slot".into()); }
    let previous = record.installed_parts[slot];
    let invalid = CacPartId::invalid();
    let already_owned = record.owned_parts.iter().flatten().any(|&p| p == previous)
        || record
            .installed_parts
            .iter()
            .enumerate()
            .any(|(i, &p)| i != slot && p == previous);
    if previous != invalid && !already_owned {
        if record.owned_parts.iter().map(Vec::len).sum::<usize>() + 1 >= 6000 {
            return Err("CAC inventory capacity reached".into());
        }
        record.owned_parts[slot].push(previous);
    }
    record.installed_parts[slot] = invalid;
    record.rebuild_unused_owned_slots(invalid)?;
    Ok(())
}

fn word(instance: &AttributeRuntimeInstance, offset: usize) -> RecordResult<u32> {
    // Generated CAC wrappers bind DefaultDataArea on null cached layouts.
    let Some(data) = instance.try_cached_layout_field(offset, 4)? else {
        return Ok(0);
    };
    Ok(u32::from_be_bytes(
        data.bytes().try_into().expect("four bytes"),
    ))
}
fn rgb(instance: &AttributeRuntimeInstance, offset: usize) -> RecordResult<[u32; 3]> {
    Ok([
        word(instance, offset)?,
        word(instance, offset + 4)?,
        word(instance, offset + 8)?,
    ])
}

/// GetDefaultColourForZone8258e368 passes layout+24 to the generated
/// ClassRefSpec_cac_colour constructor8258e1d0. It reads the collection key
/// at +0 and fixes the class to cac_colour; +8 is a cached pointer, not a key.
/// The immutable Vault already owns the referenced collection.
fn default_colour(
    vault: &Arc<AttributeRuntimeVault>,
    zone: &AttributeRuntimeInstance,
) -> RecordResult<AttributeRuntimeInstance> {
    let data = zone
        .try_cached_layout_field(24, 8)?
        .ok_or("CAC default colour reference absent")?;
    let key = u64::from_be_bytes(data.bytes().try_into()?);
    let colour = vault.instance(COLOUR_CLASS, key);
    if colour.is_null() {
        return Err(format!("CAC default colour {key:016x} unbound").into());
    }
    Ok(colour)
}

/// CASMorphParams825916c8/GetDefaultSetting8258dd70: complete sorted class
/// scan, last out-of-range row supplies fallback, then per-zone overrides.
pub(crate) fn morph_defaults(
    vault: &Arc<AttributeRuntimeVault>,
    male: bool,
) -> RecordResult<[u32; 21]> {
    let keys = vault.collections_sorted(MORPH_CLASS)?;
    let offset = if male { 20 } else { 32 };
    let mut fallback = 0.5f32.to_bits();
    for &key in &keys {
        let i = vault.instance(MORPH_CLASS, key);
        if word(&i, 4)? > 20 {
            fallback = word(&i, offset)?;
        }
    }
    let mut values = [fallback; 21];
    for key in keys {
        let i = vault.instance(MORPH_CLASS, key);
        let zone = word(&i, 4)? as usize;
        if zone < 21 {
            values[zone] = word(&i, offset)?;
        }
    }
    Ok(values)
}

fn colour_target(db: &CacPartDb, ids: &[CacPartId; 26], slot: usize) -> CacPartId {
    let invalid = CacPartId::invalid();
    let target = match slot {
        0 => 18,
        1 => 2,
        2 => 3,
        3 => 0,
        4 => 1,
        10 => 17,
        5 => {
            for s in [23, 22] {
                if let Some(p) = part(db, ids, s) {
                    if truth(p.skinned) {
                        return id(p);
                    }
                }
            }
            return invalid;
        }
        9 => {
            return part(db, ids, 23)
                .filter(|p| !truth(p.skinned) && truth(p.colourizable))
                .map(id)
                .unwrap_or(invalid);
        }
        _ => return invalid,
    };
    part(db, ids, target)
        .filter(|p| truth(p.colourizable))
        .map(id)
        .unwrap_or(invalid)
}

/// CASPlayerInitializer8258fb90 plus CACBackEnd::InitPlayer82573bb0.
/// Recipe bytes are produced later by the caller from these original choices.
pub(crate) fn fresh_record(
    db: &CacPartDb,
    vault: &Arc<AttributeRuntimeVault>,
    is_male: bool,
) -> RecordResult<CacRecord> {
    let installed = default_parts(db, is_male);
    let mut custom = CharacterCustomizations {
        is_male,
        colour_capacity: 12,
        stamp_capacity: 3,
        embedded_capacity: 6,
        morph_bits: morph_defaults(vault, is_male)?.to_vec(),
        embedded: (0u8..6).map(|key| [key, 8]).collect(),
        ..CharacterCustomizations::default()
    };
    let zones = vault.collections_sorted(COLOUR_ZONE_CLASS)?;
    let mut colour_ids = [u64::MAX; 12];
    for slot in 0..12 {
        let native_slot = if matches!(slot, 3..=5) { 2 } else { slot };
        let mut zone = None;
        for &key in &zones {
            let row = vault.instance(COLOUR_ZONE_CLASS, key);
            if word(&row, 44)? as usize == native_slot {
                zone = Some(row);
            }
        }
        let zone = zone.ok_or("CAC default colour zone absent")?;
        let colour = default_colour(vault, &zone)?;
        let key = colour
            .collection_key()
            .ok_or("CAC default colour unbound")?;
        let target = colour_target(db, &installed, slot);
        custom.colours.push(Colour {
            key: slot as i32,
            base_rgb_bits: rgb(&colour, 32)?,
            highlight_rgb_bits: rgb(&colour, 0)?,
            model_id: target.model,
            material_id: target.material,
        });
        colour_ids[slot] = key;
    }
    let mut suffix = fresh_closet();
    suffix.extend_from_slice(&custom.encode()?);
    let data = RecipeData {
        recipe: Vec::new(),
        tuning_bits: [0.3f32.to_bits(), 0.7f32.to_bits()],
        hat_type: 0,
        stance: 0,
        style: 0,
        gestures: [0, 1, 2, 3],
        is_male,
    };
    let mut record = CacRecord::new(data, CacPartId::invalid(), suffix);
    record.installed_parts = installed;
    record.colour_ids = colour_ids;
    record.skin_normal[..7].copy_from_slice(b"average");
    Ok(record)
}

/// CACRecord ctor82b6ba60 creates two initialized outfits, never zero outfits.
/// Unused thumbnail/path tails have no logical value; host fills them with0.
fn fresh_closet() -> Vec<u8> {
    fn u32out(b: &mut Vec<u8>, x: u32) {
        b.extend_from_slice(&x.to_be_bytes());
    }
    fn idout(b: &mut Vec<u8>, x: CacPartId) {
        b.extend_from_slice(&x.model.to_be_bytes());
        b.extend_from_slice(&x.material.to_be_bytes());
    }
    let mut out = Vec::new();
    u32out(&mut out, 1);
    u32out(&mut out, u32::MAX);
    u32out(&mut out, u32::MAX);
    u32out(&mut out, 2);
    let invalid = CacPartId::invalid();
    for _ in 0..2 {
        u32out(&mut out, 3);
        out.extend_from_slice(&[0; 15000]);
        u32out(&mut out, 0);
        for _ in 0..12 {
            out.extend_from_slice(&[0; 24]);
            idout(&mut out, invalid);
            out.extend_from_slice(&u64::MAX.to_be_bytes());
        }
        for _ in 0..6 {
            u32out(&mut out, 8);
        }
        out.push(1);
        for _ in 0..26 {
            idout(&mut out, invalid);
        }
        out.extend_from_slice(&[0; 84]);
        for _ in 0..3 {
            out.push(0);
            u32out(&mut out, u32::MAX);
            idout(&mut out, invalid);
            idout(&mut out, invalid);
            out.extend_from_slice(&[0; 256]);
        }
        u32out(&mut out, 1);
        u32out(&mut out, 0);
        out.extend_from_slice(&[0; 64]);
    }
    out
}

/// DoSwitchGender82590430 preserves both native Closet outfits and inventory.
pub(crate) fn switch_gender(
    record: &mut CacRecord,
    db: &CacPartDb,
    vault: &Arc<AttributeRuntimeVault>,
) -> RecordResult<()> {
    let old = usize::from(!record.data.is_male);
    store_outfit(record, old)?;
    let next = 1 - old;
    let start = 16 + next * 17060;
    let outfit = record
        .closet_and_customizations
        .get(start..start + 17060)
        .ok_or("CAC gender closet must contain two outfits")?
        .to_vec();
    let id = outfit_id(&outfit, 15609 + 3 * 16);
    if id != CacPartId::invalid() {
        restore_outfit(record, &outfit)?;
    } else {
        let fresh = fresh_record(db, vault, !record.data.is_male)?;
        let (mut custom, _) =
            CharacterCustomizations::parse_prefix(record.customizations_bytes()?)?;
        let (defaults, _) = CharacterCustomizations::parse_prefix(fresh.customizations_bytes()?)?;
        custom.is_male = defaults.is_male;
        custom.colours = defaults.colours;
        custom.morph_bits = defaults.morph_bits;
        record.data.is_male = fresh.data.is_male;
        record.data.tuning_bits = fresh.data.tuning_bits;
        for (slot, &id) in fresh.installed_parts.iter().enumerate() {
            if let Some(part) = db.find(slot as u8, id.model, id.material) {
                wear_part(record, db, part)?;
            } else {
                exchange_empty_part(record, slot as u8)?;
            }
        }
        record.colour_ids = fresh.colour_ids;
        record.skin_normal = fresh.skin_normal;
        record.replace_customizations(&custom.encode()?)?;
        store_outfit(record, next)?;
    }
    let offset = if record.data.is_male { 4 } else { 8 };
    record.closet_and_customizations[offset..offset + 4]
        .copy_from_slice(&(next as u32).to_be_bytes());
    Ok(())
}
fn outfit_word(b: &[u8], n: usize) -> u32 {
    u32::from_be_bytes(b[n..n + 4].try_into().unwrap())
}
fn outfit_qword(b: &[u8], n: usize) -> u64 {
    u64::from_be_bytes(b[n..n + 8].try_into().unwrap())
}
fn outfit_id(b: &[u8], n: usize) -> CacPartId {
    CacPartId {
        model: outfit_qword(b, n),
        material: outfit_qword(b, n + 8),
    }
}
/// Outfit::Set82b6ab70 / Serialize82b64b68: packed size17060, no SIMD padding.
fn store_outfit(record: &mut CacRecord, index: usize) -> RecordResult<()> {
    let (c, _) = CharacterCustomizations::parse_prefix(record.customizations_bytes()?)?;
    if c.morph_bits.len() != 21 {
        return Err("CAC outfit requires21 morphs".into());
    }
    let start = 16 + index * 17060;
    let b = record
        .closet_and_customizations
        .get_mut(start..start + 17060)
        .ok_or("CAC gender closet must contain two outfits")?;
    b.fill(0);
    b[0..4].copy_from_slice(&3u32.to_be_bytes());
    let size = (record.thumbnail_size as usize)
        .min(15000)
        .min(record.thumbnail.len());
    b[4..4 + size].copy_from_slice(&record.thumbnail[..size]);
    b[15004..15008].copy_from_slice(&(size as u32).to_be_bytes());
    for slot in 0..12 {
        let n = 15008 + slot * 48;
        let colour = c.colours.iter().find(|c| c.key == slot as i32);
        let (id, key) = if let Some(colour) = colour {
            for axis in 0..3 {
                b[n + axis * 4..n + axis * 4 + 4]
                    .copy_from_slice(&colour.base_rgb_bits[axis].to_be_bytes());
                b[n + 12 + axis * 4..n + 16 + axis * 4]
                    .copy_from_slice(&colour.highlight_rgb_bits[axis].to_be_bytes());
            }
            (
                CacPartId {
                    model: colour.model_id,
                    material: colour.material_id,
                },
                record.colour_ids[slot],
            )
        } else {
            (CacPartId::invalid(), u64::MAX)
        };
        b[n + 24..n + 32].copy_from_slice(&id.model.to_be_bytes());
        b[n + 32..n + 40].copy_from_slice(&id.material.to_be_bytes());
        b[n + 40..n + 48].copy_from_slice(&key.to_be_bytes());
    }
    for slot in 0..6 {
        let value = c
            .embedded
            .iter()
            .find(|v| v[0] == slot as u8)
            .map_or(8, |v| v[1]);
        b[15584 + slot * 4..15588 + slot * 4].copy_from_slice(&(value as u32).to_be_bytes());
    }
    b[15608] = u8::from(record.data.is_male);
    for (slot, id) in record.installed_parts.iter().enumerate() {
        let n = 15609 + slot * 16;
        b[n..n + 8].copy_from_slice(&id.model.to_be_bytes());
        b[n + 8..n + 16].copy_from_slice(&id.material.to_be_bytes());
    }
    for (slot, bits) in c.morph_bits.iter().enumerate() {
        b[16025 + slot * 4..16029 + slot * 4].copy_from_slice(&bits.to_be_bytes());
    }
    for slot in 0..3 {
        let n = 16109 + slot * 293;
        let stamp = c.stamps.iter().find(|s| s.key == slot as u8);
        let invalid = CacPartId::invalid();
        b[n + 1..n + 5].copy_from_slice(&stamp.map_or(u32::MAX, |s| s.key as u32).to_be_bytes());
        for (j, id) in [
            stamp.map_or(invalid.model, |s| s.model_id),
            stamp.map_or(invalid.material, |s| s.material_id),
            invalid.model,
            stamp.map_or(invalid.material, |s| s.stamp_material_id),
        ]
        .into_iter()
        .enumerate()
        {
            b[n + 5 + j * 8..n + 13 + j * 8].copy_from_slice(&id.to_be_bytes());
        }
        if let Some(stamp) = stamp {
            b[n] = u8::from(stamp.user_generated);
            let path = stamp.path.as_bytes();
            let len = path.len().min(255);
            b[n + 37..n + 37 + len].copy_from_slice(&path[..len]);
        }
    }
    b[16988..16992].copy_from_slice(&record.data.stance.to_be_bytes());
    b[16992..16996].copy_from_slice(&record.data.style.to_be_bytes());
    b[16996..17060].copy_from_slice(&record.skin_normal);
    let offset = if record.data.is_male { 4 } else { 8 };
    record.closet_and_customizations[offset..offset + 4]
        .copy_from_slice(&(index as u32).to_be_bytes());
    Ok(())
}
/// SwitchToOutfit82b6b1a8 leaves physics, gestures and owned inventory intact.
fn restore_outfit(record: &mut CacRecord, b: &[u8]) -> RecordResult<()> {
    use skate2_formats::recipe::customizations::TextureStamp;
    let (mut c, _) = CharacterCustomizations::parse_prefix(record.customizations_bytes()?)?;
    record.thumbnail = b[4..15004].to_vec();
    record.thumbnail_size = outfit_word(b, 15004).min(15000);
    for slot in 0..12 {
        let n = 15008 + slot * 48;
        let key = outfit_qword(b, n + 40);
        if key as u32 == u32::MAX {
            continue;
        }
        let id = outfit_id(b, n + 24);
        let colour = Colour {
            key: slot as i32,
            base_rgb_bits: std::array::from_fn(|axis| outfit_word(b, n + axis * 4)),
            highlight_rgb_bits: std::array::from_fn(|axis| outfit_word(b, n + 12 + axis * 4)),
            model_id: id.model,
            material_id: id.material,
        };
        if let Some(old) = c.colours.iter_mut().find(|v| v.key == slot as i32) {
            *old = colour;
        } else {
            c.colours.push(colour);
        }
        record.colour_ids[slot] = key;
    }
    c.embedded = (0..6)
        .map(|slot| [slot as u8, outfit_word(b, 15584 + slot * 4) as u8])
        .collect();
    c.is_male = b[15608] != 0;
    record.data.is_male = c.is_male;
    record.installed_parts = std::array::from_fn(|slot| outfit_id(b, 15609 + slot * 16));
    c.morph_bits = (0..21)
        .map(|slot| outfit_word(b, 16025 + slot * 4))
        .collect();
    c.stamps.clear();
    for slot in 0..3 {
        let n = 16109 + slot * 293;
        let key = outfit_word(b, n + 1);
        if key == u32::MAX {
            continue;
        }
        let path = &b[n + 37..n + 293];
        let len = path.iter().position(|&v| v == 0).unwrap_or(path.len());
        c.stamps.push(TextureStamp {
            key: key as u8,
            user_generated: b[n] != 0,
            model_id: outfit_qword(b, n + 5),
            material_id: outfit_qword(b, n + 13),
            stamp_material_id: outfit_qword(b, n + 29),
            path: String::from_utf8(path[..len].to_vec())?,
        });
    }
    record.data.stance = outfit_word(b, 16988);
    record.data.style = outfit_word(b, 16992);
    record.skin_normal.copy_from_slice(&b[16996..17060]);
    record.replace_customizations(&c.encode()?)?;
    Ok(())
}

fn colour_index(
    record: &CacRecord,
    db: &CacPartDb,
    vault: &Arc<AttributeRuntimeVault>,
    slot: usize,
) -> RecordResult<u16> {
    let row = vault.instance(COLOUR_CLASS, record.colour_ids[slot]);
    let name = row
        .try_cached_layout_text(52)?
        .ok_or("CAC colour name absent")?;
    let text = std::str::from_utf8(name.bytes())?.trim_end_matches('\0');
    Ok(index(db, text))
}

/// CASAssembler::Correct825932f8. The record retains requested hair/garments;
/// corrected arms/legs/rostral and skinned torso are published as native
/// CorrectColours8258ded0 does. All matching scans preserve authored order.
pub(crate) fn assemble_parts(
    record: &mut CacRecord,
    db: &CacPartDb,
    vault: &Arc<AttributeRuntimeVault>,
    going_into_game: bool,
    correct_rostral: bool,
) -> RecordResult<[CacPartId; 26]> {
    let invalid = CacPartId::invalid();
    let mut ids = record.installed_parts;
    let hair_colour = colour_index(record, db, vault, 1)?;
    let skin = colour_index(record, db, vault, 2)?;
    let end = record
        .skin_normal
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(64);
    let normal = index(
        db,
        if end == 0 {
            "average"
        } else {
            std::str::from_utf8(&record.skin_normal[..end])?
        },
    );
    let mut custom = CharacterCustomizations::parse_prefix(record.customizations_bytes()?)?.0;
    // Hair requirement: none=0, hat=1, full=2.
    let required = part(db, &ids, 18)
        .map(|p| p.attributes[HAIR_REQUIRED])
        .unwrap_or_else(|| index(db, "full"));
    let old_hair = part(db, &ids, 2);
    ids[2] = invalid;
    if required != index(db, "none") {
        if let Some(hair) = old_hair {
            let hat_hair = hair.attributes[HAIR_TYPE] == index(db, "hat");
            let need_full = required == index(db, "full");
            let chosen = if hat_hair == !need_full {
                Some(hair)
            } else {
                db.parts_in_slot(2)
                    .find(|p| {
                        p.attributes[HAIR_STYLE] == hair.attributes[HAIR_STYLE]
                            && p.attributes[GENDER] == hair.attributes[GENDER]
                            && p.attributes[HAIR_TYPE] == required
                    })
                    .or_else(|| db.parts_in_slot(2).next())
            };
            if let Some(p) = chosen {
                ids[2] = id(p);
                apply_colour(&mut custom, 1, ids[2]);
            }
        }
    }
    if correct_rostral {
        if let Some(rostral) = part(db, &ids, 3) {
            let shadow = part(db, &ids, 2)
                .map(|p| p.attributes[SHADOW_REQUIRED])
                .unwrap_or_else(|| index(db, "0"));
            // GetRostralID82b65538 deliberately keeps LAST matching row.
            if let Some(p) = db
                .parts_in_slot(3)
                .filter(|p| {
                    p.attributes[GENDER] == rostral.attributes[GENDER]
                        && p.attributes[SKIN_TONE] == skin
                        && p.attributes[SHADOW_STYLE] == shadow
                        && p.attributes[SHADOW_COLOUR] == hair_colour
                        && p.attributes[FACIAL_STYLE] == rostral.attributes[FACIAL_STYLE]
                        && p.attributes[EYEBROW_STYLE] == rostral.attributes[EYEBROW_STYLE]
                })
                .last()
            {
                ids[3] = id(p);
            }
        }
    }
    let mut outer = part(db, &ids, 23);
    let mut inner = part(db, &ids, 22);
    if outer.is_none() {
        if let Some(p) = inner {
            let material = if p.attributes[OUTER_MATERIAL] == MISSING {
                p.attributes[MATERIAL_NAME]
            } else {
                p.attributes[OUTER_MATERIAL]
            };
            outer = db
                .parts_in_slot(23)
                .find(|q| {
                    q.attributes[MODEL_NAME] == p.attributes[OUTER_MODEL]
                        && q.attributes[MATERIAL_NAME] == material
                })
                .or_else(|| default_part(db, 23, p.attributes[GENDER] == index(db, "male")));
        }
    }
    let outer = outer.ok_or("CAS assembly requires outer torso")?;
    if outer.attributes[INNER_REQUIRED] == MISSING {
        inner = None;
    } else {
        if inner.is_none() {
            inner = default_part(db, 22, outer.attributes[GENDER] == index(db, "male"));
        }
        if let Some(p) = inner {
            if p.attributes[INNER_CUT] != outer.attributes[INNER_REQUIRED] {
                inner = db.parts_in_slot(22).find(|q| {
                    q.material_id == p.material_id
                        && q.attributes[INNER_CUT] == outer.attributes[INNER_REQUIRED]
                });
            }
        }
    }
    let arm_for = |torso: &CacPart, torso_slot: u8| -> Option<&CacPart> {
        if torso.attributes[ARM_REQUIRED] == MISSING {
            return None;
        }
        limb_for(db, 0, torso_slot, torso, |p| {
            p.attributes[GENDER] == torso.attributes[GENDER]
                && p.attributes[ARM_TYPE] == torso.attributes[ARM_REQUIRED]
                && p.attributes[SKIN_TONE] == skin
                && p.attributes[SKIN_NORMAL] == normal
        })
    };
    let arm = inner.and_then(|p| arm_for(p, 22)).or_else(|| arm_for(outer, 23));
    ids[23] = id(alternate_skin(db, outer, skin, normal));
    ids[22] = inner
        .map(|p| id(alternate_skin(db, p, skin, normal)))
        .unwrap_or(invalid);
    ids[0] = arm.map(id).unwrap_or(invalid);
    // Skate 3's `CASAssembler::Correct` (0x8253f648) then clears the jewellery under a torso whose
    // `IsNecklaceRemoved` is set (0x82540430), a flag only a linked Skate 3 data folder's parts
    // carry.
    if [22, 23].into_iter().any(|slot| part(db, &ids, slot).is_some_and(|p| truth(p.necklace_removed))) {
        ids[25] = invalid;
    }
    if ids[21] != invalid {
        if let Some(arm) = part(db, &ids, 0) {
            if arm.attributes[WRIST] != index(db, "yes") {
                ids[21] = invalid;
            }
        }
    }
    let pants = part(db, &ids, 15);
    let sock = part(db, &ids, 17);
    ids[1] = invalid;
    if let Some(pants) = pants {
        if pants.attributes[LEG_REQUIRED] == MISSING {
            ids[17] = invalid;
        } else {
            // GetLegPartForPants82b67b58 does NOT compare normalIndex.
            let leg = limb_for(db, 1, 15, pants, |p| {
                p.attributes[GENDER] == pants.attributes[GENDER]
                    && p.attributes[LEG_TYPE] == pants.attributes[LEG_REQUIRED]
                    && p.attributes[SKIN_TONE] == skin
                    && match sock {
                        Some(sock) => p.attributes[SOCK_REQUIRED] == sock.attributes[SOCK_STYLE],
                        None => {
                            p.attributes[SOCK_REQUIRED] == MISSING
                                || p.attributes[SOCK_REQUIRED] == index(db, "none")
                        }
                    }
            });
            ids[1] = leg.map(id).unwrap_or(invalid);
        }
    }
    for slot in [5, 11, 13] {
        if let Some(p) = part(db, &ids, slot) {
            if truth(p.used_in_game) != going_into_game {
                if let Some(q) = db.parts_in_slot(slot as u8).find(|q| {
                    q.attributes[MODEL_NAME] == p.attributes[MODEL_NAME]
                        && q.attributes[MATERIAL_NAME] == p.attributes[ALTERNATE]
                }) {
                    ids[slot] = id(q);
                }
            }
        }
    }
    for slot in 0..26 {
        if let Some(p) = part(db, &ids, slot) {
            if p.attributes[REMOVALS] != MISSING {
                for name in db.string(p.attributes[REMOVALS])?.split(',') {
                    let target = SLOT_NAMES
                        .iter()
                        .position(|&s| s == name)
                        .ok_or("unknown CAS removal component")?;
                    ids[target] = invalid;
                }
            }
        }
    }
    for slot in [0, 1, 3] {
        record.installed_parts[slot] = ids[slot];
    }
    for (colour, slot) in [(2, 3), (3, 0), (4, 1)] {
        apply_colour(&mut custom, colour, ids[slot]);
    }
    if let Some(p) = part(db, &ids, 23) {
        if truth(p.colourizable) && !truth(p.skinned) {
            apply_colour(&mut custom, 9, ids[23]);
        }
        if truth(p.skinned) {
            apply_colour(&mut custom, 5, ids[23]);
            record.installed_parts[23] = ids[23];
        } else if part(db, &ids, 22).is_some_and(|p| truth(p.skinned)) {
            apply_colour(&mut custom, 5, ids[22]);
            record.installed_parts[22] = ids[22];
        }
    }
    if part(db, &ids, 18).is_some_and(|p| truth(p.colourizable)) {
        apply_colour(&mut custom, 0, ids[18]);
    }
    if part(db, &ids, 15).is_some_and(|p| truth(p.colourizable)) {
        apply_colour(&mut custom, BOTTOM_COLOUR as i32, ids[15]);
    }
    // CorrectSkateboard825900d0 repoints deck stamp only when logoable.
    if part(db, &ids, 11).is_some_and(|p| truth(p.logoable)) {
        for stamp in &mut custom.stamps {
            if stamp.key == 2 {
                stamp.model_id = ids[11].model;
                stamp.material_id = ids[11].material;
            }
        }
    }
    // PC port: a colour filter goes with its garment: one bound to another part than the one its
    // slot wears is gone.
    retain_worn_filters(&mut custom, &record.installed_parts);
    // CorrectStamps82585cd0 checks installed choices, not corrected display IDs.
    custom.stamps.retain(|stamp| {
        let slot = match stamp.key {
            0 => 18,
            1 => 23,
            2 => 11,
            _ => return true,
        };
        part(db, &record.installed_parts, slot).is_some_and(|p| truth(p.logoable))
    });
    record.replace_customizations(&custom.encode()?)?;
    Ok(ids)
}

/// Diagnostic (`SKATE2_DIAGNOSTIC_CAS_LIMBS`): every arm and leg part, and the arm and leg kinds
/// the shirts and pants require, each with the database it came from.
pub(crate) fn log_limbs(db: &CacPartDb) {
    if std::env::var_os("SKATE2_DIAGNOSTIC_CAS_LIMBS").is_none() {
        return;
    }
    let text = |p: &CacPart, attribute: usize| db.string(p.attributes[attribute]).unwrap_or("?").to_owned();
    for slot in [0_u8, 1] {
        for index in db.slot_range(slot) {
            let p = &db.parts[index];
            eprintln!(
                "cas limbs: slot {slot} #{index} s3 {} gender {} type {} tone {} normal {} sock {} wrist {} model {} material {}",
                db.is_skate3(index),
                text(p, GENDER),
                text(p, if slot == 0 { ARM_TYPE } else { LEG_TYPE }),
                text(p, SKIN_TONE),
                text(p, SKIN_NORMAL),
                text(p, SOCK_REQUIRED),
                text(p, WRIST),
                text(p, MODEL_NAME),
                text(p, MATERIAL_NAME),
            );
        }
    }
    for slot in [15_u8, 22, 23] {
        let mut kinds = std::collections::BTreeMap::new();
        for index in db.slot_range(slot) {
            let p = &db.parts[index];
            let need = text(p, if slot == 15 { LEG_REQUIRED } else { ARM_REQUIRED });
            *kinds.entry((db.is_skate3(index), text(p, GENDER), need)).or_insert(0) += 1;
        }
        for ((skate3, gender, need), count) in kinds {
            eprintln!("cas limbs: slot {slot} s3 {skate3} gender {gender} requires {need}: {count}");
        }
    }
    for slot in [22_u8, 23] {
        let necklace_off: std::collections::BTreeSet<String> =
            db.slot_range(slot).map(|index| &db.parts[index]).filter(|p| truth(p.necklace_removed)).map(|p| text(p, MODEL_NAME)).collect();
        eprintln!("cas limbs: slot {slot} takes the necklace off under {} models: {necklace_off:?}", necklace_off.len());
    }
}

/// The part slot whose garment colour filter `key` is for.
pub(crate) fn filter_part_slot(key: usize) -> Option<usize> {
    match key {
        TOP_FILTER => Some(23),
        BOTTOM_FILTER => Some(15),
        _ => None,
    }
}
/// The colour filter of a part slot's garments.
pub(crate) fn part_filter(slot: u8) -> Option<usize> {
    match slot {
        23 => Some(TOP_FILTER),
        15 => Some(BOTTOM_FILTER),
        _ => None,
    }
}
/// Colour filter `key`'s colour: its base and the garment it is bound to.
pub(crate) fn filter(custom: &CharacterCustomizations, key: usize) -> Option<&Colour> {
    custom.colours.iter().find(|colour| usize::try_from(colour.key).ok() == Some(key))
}
/// Puts colour filter `key` on `target` with `base` (no highlight: the cloth shader reads none), or
/// takes it off with `None`.
pub(crate) fn set_filter(custom: &mut CharacterCustomizations, key: usize, base: Option<[u32; 3]>, target: CacPartId) {
    custom.colours.retain(|colour| usize::try_from(colour.key).ok() != Some(key));
    if let Some(base) = base {
        custom.colour_capacity = custom.colour_capacity.max(BOTTOM_FILTER as u32 + 1);
        custom.colours.push(Colour {
            key: key as i32,
            base_rgb_bits: base,
            highlight_rgb_bits: [0; 3],
            model_id: target.model,
            material_id: target.material,
        });
    }
}
/// Takes off each colour filter bound to another part than the one its slot wears; true when one
/// went.
pub(crate) fn retain_worn_filters(custom: &mut CharacterCustomizations, installed: &[CacPartId; 26]) -> bool {
    let before = custom.colours.len();
    custom.colours.retain(|colour| {
        usize::try_from(colour.key).ok().and_then(filter_part_slot).is_none_or(|slot| {
            installed[slot] == CacPartId { model: colour.model_id, material: colour.material_id }
        })
    });
    custom.colours.len() != before
}
/// The bottom zone's colour, added the first time one is picked; the caller sets its colour and
/// binds it to the worn legs.
pub(crate) fn ensure_bottom_colour(custom: &mut CharacterCustomizations) {
    let key = BOTTOM_COLOUR as i32;
    if !custom.colours.iter().any(|colour| colour.key == key) {
        custom.colour_capacity = custom.colour_capacity.max(BOTTOM_COLOUR as u32 + 1);
        custom.colours.push(Colour { key, base_rgb_bits: [0; 3], highlight_rgb_bits: [0; 3], model_id: 0, material_id: 0 });
    }
}
/// The first arm or leg part of `slot` that `matches` takes (`GetArmPartForTorso`,
/// `GetLegPartForPants`). PC port: for a garment of `garment_slot` that a linked Skate 3 data
/// folder's clothing joined (`Skate3Data::clothing_for`), Skate 3's own arms or legs come first:
/// Skate 2's, the first of each kind, do not meet Skate 3's clothes (a female shirt's sleeves).
/// Skate 2's stay the fallback, and Skate 2's clothes keep theirs.
fn limb_for<'a>(
    db: &'a CacPartDb,
    slot: u8,
    garment_slot: u8,
    garment: &CacPart,
    matches: impl Fn(&CacPart) -> bool,
) -> Option<&'a CacPart> {
    // SKATE2_DIAGNOSTIC_SKATE2_LIMBS: every garment takes the first that fits, Skate 2's first.
    let skate3_first = std::env::var_os("SKATE2_DIAGNOSTIC_SKATE2_LIMBS").is_none();
    (skate3_first && db.is_skate3_part(garment_slot, garment.model_id, garment.material_id))
        .then(|| db.skate3_parts_in_slot(slot).find(|p| matches(p)))
        .flatten()
        .or_else(|| db.parts_in_slot(slot).find(|p| matches(p)))
}

fn apply_colour(custom: &mut CharacterCustomizations, key: i32, target: CacPartId) {
    if let Some(c) = custom.colours.iter_mut().find(|c| c.key == key) {
        c.model_id = target.model;
        c.material_id = target.material;
    }
}
fn alternate_skin<'a>(db: &'a CacPartDb, p: &'a CacPart, skin: u16, normal: u16) -> &'a CacPart {
    // GetAlternateSkinColourPartID82b66d40 handles skinned OUTER only,
    // not INNER; its matching scan keeps the last result.
    if p.slot != 23 || !truth(p.skinned) {
        return p;
    }
    db.parts_in_slot(p.slot)
        .filter(|q| {
            q.model_id == p.model_id
                && q.attributes[GENDER] == p.attributes[GENDER]
                && q.attributes[SKIN_TONE] == skin
                && q.attributes[SKIN_NORMAL] == normal
        })
        .last()
        .unwrap_or(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_runtime::tests::{
        add_fixture_class, collection, graph, set_dependency_bytes,
    };

    #[test]
    fn editor_empty_exchange_keeps_inventory() {
        let invalid = CacPartId::invalid();
        let mut record = CacRecord::new(skate2_formats::skater_profile::RecipeData {
            recipe: Vec::new(), tuning_bits: [0;2], hat_type: 0, stance: 0,
            style: 0, gestures: [0;4], is_male: true,
        }, invalid, Vec::new());
        let shirt = CacPartId { model: 123, material: 456 };
        record.installed_parts[23] = shirt;
        assert!(can_remove_part(23));
        exchange_empty_part(&mut record, 23).unwrap();
        assert_eq!(record.installed_parts[23], invalid);
        assert_eq!(record.owned_parts[23], vec![shirt]);
        exchange_empty_part(&mut record, 23).unwrap();
        assert_eq!(record.owned_parts[23], vec![shirt]);
        let before = record.clone();
        assert!(!can_remove_part(0));
        assert!(!can_remove_part(26));
        assert!(exchange_empty_part(&mut record, 26).is_err());
        assert_eq!(record, before);
    }

    #[test]
    fn default_colour_uses_typed_key_not_general_refspec_cache() {
        let mut vault = graph(COLOUR_ZONE_CLASS, vec![]);
        add_fixture_class(&mut vault, COLOUR_CLASS, vec![]);
        let key = 0x405b_fb7d_8bb7_96da;
        collection(&mut vault, COLOUR_ZONE_CLASS, 10, 0, Some(0), vec![]);
        collection(&mut vault, COLOUR_CLASS, key, 0, Some(64), vec![]);
        let mut bytes = vec![0; 128];
        bytes[24..32].copy_from_slice(&key.to_be_bytes());
        // ClassRefSpec cache/padding at +8..16 is zero in the shipped asset.
        set_dependency_bytes(&mut vault, 1, bytes);
        let vault = Arc::new(vault);
        let zone = vault.instance(COLOUR_ZONE_CLASS, 10);
        assert_eq!(default_colour(&vault, &zone).unwrap().collection_key(), Some(key));
        assert!(default_colour(&vault, &vault.instance(COLOUR_ZONE_CLASS, 99)).is_err());
    }

    /// PC port: Apply Color's filter is one per garment slot, bound to the garment it was put on:
    /// a slot wearing another part leaves it behind, a zone's colour stays, and `None` takes it off.
    #[test]
    fn a_colour_filter_goes_with_its_garment() {
        let tee = CacPartId { model: 1, material: 2 };
        let other = CacPartId { model: 3, material: 4 };
        let zone = Colour { key: 9, base_rgb_bits: [5; 3], highlight_rgb_bits: [5; 3], model_id: 7, material_id: 8 };
        let mut custom = CharacterCustomizations { colour_capacity: 12, colours: vec![zone.clone()], ..Default::default() };
        set_filter(&mut custom, TOP_FILTER, Some([1, 2, 3]), tee);
        set_filter(&mut custom, TOP_FILTER, Some([4, 5, 6]), tee);
        assert_eq!(custom.colours.len(), 2, "one filter a slot");
        assert_eq!(filter(&custom, TOP_FILTER).map(|c| (c.base_rgb_bits, c.highlight_rgb_bits)), Some(([4, 5, 6], [0; 3])));
        assert!(custom.colour_capacity as usize > BOTTOM_FILTER);
        let mut installed = [CacPartId::invalid(); 26];
        installed[23] = tee;
        assert!(!retain_worn_filters(&mut custom, &installed), "the tee is worn");
        installed[23] = other;
        assert!(retain_worn_filters(&mut custom, &installed), "another shirt is worn");
        assert_eq!(custom.colours, vec![zone], "the zone's colour stays bound as it was");
        set_filter(&mut custom, BOTTOM_FILTER, Some([7; 3]), other);
        set_filter(&mut custom, BOTTOM_FILTER, None, other);
        assert!(filter(&custom, BOTTOM_FILTER).is_none());
        assert_eq!((part_filter(23), part_filter(15), part_filter(5)), (Some(TOP_FILTER), Some(BOTTOM_FILTER), None));
        assert_eq!((filter_part_slot(TOP_FILTER), filter_part_slot(BOTTOM_FILTER), filter_part_slot(9)), (Some(23), Some(15), None));
    }
}
