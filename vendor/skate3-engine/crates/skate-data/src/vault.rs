//! Skate 3's big-endian, 64-bit AttribSys schema and collections (`skaterschema` and
//! `skatercollections`, `.vlt` + `.bin`) as `Collections`, read from the linked game's bytes.
//! The same conversion the old asset pipeline's `vlt.py` wrote to `skater-collections.json`;
//! names without a debug string keep their numeric `Hash_` spelling.
use crate::collections::{Collection, Field, FieldArray};
use std::collections::{BTreeMap, HashMap};

const POINTERS: u32 = 0x5074_724e;
const EXPORTS: u32 = 0x4578_704e;
const END: u32 = 0x456e_6443;
const CLASS_EXPORT: u64 = 0x2A78_95AC_4A87_6152;
const COLLECTION_EXPORT: u64 = 0xAD30_3B8F_42B3_307E;
const TEXT: &str = "EA::Reflection::Text";

/// The old pipeline's debug names for hashes the vaults hold no string for.
const NAMES: &str = include_str!("vault_names.txt");

fn take(data: &[u8], at: usize, size: usize) -> Result<&[u8], String> {
    at.checked_add(size)
        .and_then(|end| data.get(at..end))
        .ok_or_else(|| format!("Truncated AttribSys record at {at:#x}, size {size}"))
}
fn u16_at(data: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_be_bytes(take(data, at, 2)?.try_into().expect("2 bytes")))
}
fn u32_at(data: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_be_bytes(take(data, at, 4)?.try_into().expect("4 bytes")))
}
fn u64_at(data: &[u8], at: usize) -> Result<u64, String> {
    Ok(u64::from_be_bytes(take(data, at, 8)?.try_into().expect("8 bytes")))
}
fn at(value: u32) -> usize {
    value as usize
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

/// A `.vlt` with its `.bin`, pointer fixups applied, and the `.vlt`'s exports
/// (id, kind, size, offset).
struct Vault {
    vlt: Vec<u8>,
    bin: Vec<u8>,
    exports: Vec<(u64, u64, u32, u32)>,
}

fn vault(vlt: &[u8], bin: &[u8]) -> Result<Vault, String> {
    let mut vault = Vault { vlt: vlt.to_vec(), bin: bin.to_vec(), exports: Vec::new() };
    let mut chunk = 0;
    while chunk < vault.vlt.len() {
        let tag = u32_at(&vault.vlt, chunk)?;
        let size = at(u32_at(&vault.vlt, chunk + 4)?);
        if size < 8 {
            return Err("Invalid AttribSys chunk length".into());
        }
        take(&vault.vlt, chunk, size)?;
        let (body, length) = (chunk + 8, size - 8);
        match tag {
            POINTERS => {
                let mut into_vlt = false;
                for record in (body..body + length).step_by(16) {
                    let offset = at(u32_at(&vault.vlt, record)?);
                    let kind = u16_at(&vault.vlt, record + 4)?;
                    let index = u16_at(&vault.vlt, record + 6)?;
                    let dest = u64_at(&vault.vlt, record + 8)?;
                    match kind {
                        0 => break,
                        2 => into_vlt = index == 0,
                        1 | 3 => {
                            let target = if into_vlt { &mut vault.vlt } else { &mut vault.bin };
                            take(target, offset, 4)?;
                            let value = if kind == 1 { 0 } else { dest as u32 };
                            target[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
                        }
                        4 => {}
                        _ => return Err(format!("Unknown VLT pointer kind {kind}")),
                    }
                }
            }
            EXPORTS => {
                let count = u64_at(&vault.vlt, body)?;
                if count > (length as u64).saturating_sub(8) / 24 {
                    return Err("Invalid VLT export count".into());
                }
                for index in 0..count as usize {
                    let record = body + 8 + index * 24;
                    vault.exports.push((
                        u64_at(&vault.vlt, record)?,
                        u64_at(&vault.vlt, record + 8)?,
                        u32_at(&vault.vlt, record + 16)?,
                        u32_at(&vault.vlt, record + 20)?,
                    ));
                }
            }
            _ => {}
        }
        chunk += size;
        if tag == END {
            break;
        }
    }
    Ok(vault)
}

/// `vlt.py`'s `[A-Za-z_][A-Za-z0-9_:./ -]{1,180}\x00` matches: every short NUL-terminated
/// identifier-like string.
fn strings(data: &[u8], into: &mut Vec<String>) {
    let first = |byte: u8| byte.is_ascii_alphabetic() || byte == b'_';
    let rest = |byte: u8| byte.is_ascii_alphanumeric() || b"_:./ -".contains(&byte);
    let mut start = 0;
    while start < data.len() {
        if !first(data[start]) {
            start += 1;
            continue;
        }
        let run = data[start + 1..].iter().take_while(|&&byte| rest(byte)).count();
        let end = start + 1 + run;
        if data.get(end) != Some(&0) {
            // Every start inside this run meets the same terminator.
            start = end.max(start + 1);
        } else if (1..=180).contains(&run) {
            into.push(String::from_utf8_lossy(&data[start..end]).into_owned());
            start = end + 1;
        } else {
            start += 1;
        }
    }
}

struct Names(HashMap<u64, String>);
impl Names {
    fn name(&self, key: u64) -> String {
        if key == 0 {
            return String::new();
        }
        self.0.get(&key).cloned().unwrap_or_else(|| format!("Hash_{key:016X}"))
    }
    fn learn(&mut self, text: &str) {
        self.0.insert(crate::attrib_hash::hash(text), text.to_owned());
    }
}

fn text_at(bin: &[u8], pointer: usize) -> Result<String, String> {
    let tail = bin.get(pointer..).ok_or_else(|| format!("VLT text at {pointer:#x} outside its bin"))?;
    let end = tail.iter().position(|&byte| byte == 0).ok_or_else(|| format!("Unterminated VLT text at {pointer:#x}"))?;
    std::str::from_utf8(&tail[..end]).map(str::to_owned).map_err(|error| error.to_string())
}

/// A schema field: (type, offset, size, maximum, flags, alignment).
type Definition = (u64, u16, u16, u16, u8, u8);

fn array_items(data: &[u8], pos: usize, size: u16, alignment: u8) -> Result<Vec<Vec<u8>>, String> {
    let capacity = u16_at(data, pos)?;
    let count = u16_at(data, pos + 2)?;
    let stride = u16_at(data, pos + 4)?;
    if count > capacity || stride != size || stride == 0 || alignment > 16 {
        return Err(format!("Invalid VLT array at {pos:#x}"));
    }
    let boundary = 1usize << alignment;
    let mut cursor = pos + 8;
    let mut items = Vec::new();
    for index in 0..capacity {
        cursor = (cursor + boundary - 1) & !(boundary - 1);
        let raw = take(data, cursor, usize::from(stride))?;
        if index < count {
            items.push(raw.to_vec());
        }
        cursor += usize::from(stride);
    }
    Ok(items)
}

/// The skater database's collections, as `Collections::load` serves them for linked data.
pub fn convert(schema_vlt: &[u8], schema_bin: &[u8], collections_vlt: &[u8], collections_bin: &[u8])
    -> Result<Vec<Collection>, String> {
    let schema = vault(schema_vlt, schema_bin)?;
    let data = vault(collections_vlt, collections_bin)?;
    let mut found: Vec<String> = NAMES.lines().map(str::to_owned).collect();
    strings(&schema.bin, &mut found);
    strings(&data.bin, &mut found);
    let mut names = Names(HashMap::new());
    for text in &found {
        names.learn(text);
    }
    let mut classes: HashMap<u64, Vec<(u64, Definition)>> = HashMap::new();
    for &(_, kind, _, offset) in &schema.exports {
        if kind != CLASS_EXPORT {
            continue;
        }
        let record = at(offset);
        let key = u64_at(&schema.vlt, record)?;
        let count = at(u32_at(&schema.vlt, record + 12)?);
        let definitions = at(u32_at(&schema.vlt, record + 16)?);
        let mut fields = Vec::with_capacity(count);
        for index in 0..count {
            let field = definitions + index * 24;
            let definition = (
                u64_at(&schema.bin, field + 8)?,
                u16_at(&schema.bin, field + 16)?,
                u16_at(&schema.bin, field + 18)?,
                u16_at(&schema.bin, field + 20)?,
                take(&schema.bin, field + 22, 1)?[0],
                take(&schema.bin, field + 23, 1)?[0],
            );
            let key = u64_at(&schema.bin, field)?;
            // A repeated definition replaces the first in place, as a dict assignment does.
            match fields.iter_mut().find(|(existing, _)| *existing == key) {
                Some((_, slot)) => *slot = definition,
                None => fields.push((key, definition)),
            }
        }
        classes.insert(key, fields);
    }
    let source = crate::sha256::digest(collections_vlt);
    let mut rows = Vec::new();
    for &(_, kind, _, offset) in &data.exports {
        if kind != COLLECTION_EXPORT {
            continue;
        }
        let record = at(offset);
        let key = u64_at(&data.vlt, record)?;
        let class = u64_at(&data.vlt, record + 8)?;
        let parent = u64_at(&data.vlt, record + 16)?;
        let count = at(u32_at(&data.vlt, record + 32)?);
        let types_length = usize::from(u16_at(&data.vlt, record + 38)?);
        let layout = at(u32_at(&data.vlt, record + 40)?);
        let fields = classes.get(&class).ok_or_else(|| format!("Collection of unknown class {class:016X}"))?;
        let mut out: BTreeMap<String, Field> = BTreeMap::new();
        let mut value = |names: &mut Names, field_key: u64, bytes: &[u8], pos: usize, inline: bool| -> Result<(), String> {
            let &(type_id, _, size, _, flags, alignment) = &fields
                .iter()
                .find(|(key, _)| *key == field_key)
                .ok_or_else(|| format!("Unknown schema field {field_key:x}"))?
                .1;
            let type_name = names.name(type_id);
            let length = if inline { 4 } else { usize::from(size) };
            let field = if type_name == TEXT && flags & 1 == 0 {
                let pointer = at(u32_at(bytes, pos)?);
                let raw = if pointer == 0 { String::new() } else { text_at(&data.bin, pointer)? };
                if raw.is_ascii() {
                    names.learn(&raw);
                }
                Field { type_name, data: raw, array: None }
            } else {
                let raw = take(bytes, pos, length)?;
                let array = if flags & 1 != 0 {
                    let items = array_items(bytes, pos, size, alignment)?;
                    let (text_items, raw_items) = if type_name == TEXT {
                        let text = items
                            .iter()
                            .map(|item| {
                                let pointer = at(u32::from_be_bytes(item.get(..4).ok_or("Short VLT text item")?.try_into().expect("4 bytes")));
                                if pointer == 0 { Ok(String::new()) } else { text_at(&data.bin, pointer) }
                            })
                            .collect::<Result<_, String>>()?;
                        (text, Vec::new())
                    } else {
                        (Vec::new(), items.iter().map(|item| hex(item)).collect())
                    };
                    Some(FieldArray { text_items, raw_items })
                } else {
                    None
                };
                Field { type_name, data: hex(raw), array }
            };
            out.insert(names.name(field_key), field);
            Ok(())
        };
        for &(field_key, (_, field_offset, _, _, flags, _)) in fields {
            if flags & 2 != 0 && layout != 0 {
                value(&mut names, field_key, &data.bin, layout + usize::from(field_offset), false)?;
            }
        }
        let entries = record + 48 + types_length * 8;
        for index in 0..count {
            let pos = entries + index * 16;
            let field_key = u64_at(&data.vlt, pos)?;
            let pointer = at(u32_at(&data.vlt, pos + 8)?);
            let &(_, (_, _, size, _, flags, _)) = fields
                .iter()
                .find(|(key, _)| *key == field_key)
                .ok_or_else(|| format!("Unknown schema field {field_key:x}"))?;
            if size <= 4 && flags & 1 == 0 {
                value(&mut names, field_key, &data.vlt, pos + 8, true)?;
            } else {
                value(&mut names, field_key, &data.bin, pointer, false)?;
            }
        }
        rows.push((names.name(class), key, parent, out));
    }
    Ok(rows
        .into_iter()
        .map(|(class_name, key, parent, fields)| Collection {
            class_name,
            key: names.name(key),
            parent: names.name(parent),
            fields,
            source: "skatercollections.vlt".into(),
            sha256: source.clone(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn strings_match_the_pipeline_pattern() {
        let mut found = Vec::new();
        super::strings(b"\0Normal\0x\0_a:b/c\0123\0Easy Mode\0toolong!\0", &mut found);
        assert_eq!(found, ["Normal", "_a:b/c", "Easy Mode"]);
    }
}
