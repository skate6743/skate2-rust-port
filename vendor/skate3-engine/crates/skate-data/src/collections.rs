//! Typed access to converted stock skater XML values, without engine objects.
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Deserialize)]
pub struct Field {
    #[serde(rename = "type")]
    pub type_name: String,
    pub data: String,
    /// An array field's elements (`data` holds the count): the converter's `array`, with the
    /// text elements resolved (`text_items`).
    #[serde(default)]
    pub array: Option<FieldArray>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct FieldArray {
    #[serde(default)]
    pub text_items: Vec<String>,
    /// A non-text array's elements, each its big-endian bytes as hex.
    #[serde(default)]
    pub raw_items: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Collection {
    #[serde(rename = "class")]
    pub class_name: String,
    pub key: String,
    pub parent: String,
    pub fields: BTreeMap<String, Field>,
    pub source: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Collections {
    version: u32,
    collections: Vec<Collection>,
    /// (numeric class, numeric key) -> index of the FIRST matching entry, i.e.
    /// exactly what a linear scan in vector order finds. Built on first lookup,
    /// reset by every mutation. A full scan per lookup hashed every entry's
    /// names and cost seconds per skater/camera load.
    #[serde(skip)]
    index: std::sync::OnceLock<std::collections::HashMap<(String, String), usize>>,
}

impl Collections {
    /// Replace one profile with a child of a base profile, without editing assets.
    pub fn override_profile(
        &mut self,
        class: &str,
        target: &str,
        base: &str,
        fields: BTreeMap<String, Field>,
    ) -> Result<(), String> {
        let id = crate::attrib_hash::numeric_name(class);
        let base_id = crate::attrib_hash::numeric_name(base);
        if !self.collections.iter().any(|c| {
            crate::attrib_hash::numeric_name(&c.class_name) == id
                && crate::attrib_hash::numeric_name(&c.key) == base_id
        }) {
            return Err("Missing base profile".into());
        }
        let target_id = crate::attrib_hash::numeric_name(target);
        self.collections.retain(|c| {
            !(crate::attrib_hash::numeric_name(&c.class_name) == id
                && crate::attrib_hash::numeric_name(&c.key) == target_id)
        });
        self.collections.push(Collection {
            class_name: class.into(),
            key: target.into(),
            parent: base.into(),
            fields,
            source: "host custom profile".into(),
            sha256: String::new(),
        });
        self.index = std::sync::OnceLock::new();
        Ok(())
    }

    fn entry(&self, class_id: &str, key_id: &str) -> Option<&Collection> {
        let index = self.index.get_or_init(|| {
            let mut index = std::collections::HashMap::with_capacity(self.collections.len());
            for (position, item) in self.collections.iter().enumerate() {
                index
                    .entry((
                        crate::attrib_hash::numeric_name(&item.class_name),
                        crate::attrib_hash::numeric_name(&item.key),
                    ))
                    .or_insert(position);
            }
            index
        });
        index
            .get(&(class_id.to_owned(), key_id.to_owned()))
            .map(|&position| &self.collections[position])
    }

    pub fn entries(&self) -> &[Collection] {
        &self.collections
    }

    pub fn load(asset_root: &Path) -> Result<Self, String> {
        let data = if crate::asset_io::is_linked_root(asset_root) {
            Self::linked(asset_root)?
        } else {
            let path = asset_root.join("private/stock/skater-collections.json");
            let bytes = crate::asset_io::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?
        };
        if data.version != 1 {
            return Err(format!(
                "Unsupported skater collections version {}",
                data.version
            ));
        }
        let mut identities = std::collections::BTreeSet::new();
        for item in &data.collections {
            if !identities.insert((
                crate::attrib_hash::numeric_name(&item.class_name),
                crate::attrib_hash::numeric_name(&item.key),
            )) {
                return Err(format!(
                    "Duplicate collection {}/{}",
                    item.class_name, item.key
                ));
            }
        }
        Ok(data)
    }

    /// The linked game's skater database (`data/db/skaterschema` + `skatercollections`).
    fn linked(asset_root: &Path) -> Result<Self, String> {
        let read = |name: &str| {
            let path = asset_root.join("private/stock/data/db").join(name);
            crate::asset_io::read_shared(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        let collections = crate::vault::convert(
            &read("skaterschema.vlt")?,
            &read("skaterschema.bin")?,
            &read("skatercollections.vlt")?,
            &read("skatercollections.bin")?,
        )?;
        Ok(Self { version: 1, collections, index: std::sync::OnceLock::new() })
    }

    pub fn field(&self, class: &str, key: &str, name: &str) -> Result<&Field, String> {
        // Converted vaults can retain numeric identities when debug names were
        // unavailable. Both spellings identify the same native AttribSys entry.
        let class_id = crate::attrib_hash::numeric_name(class);
        let field_id = crate::attrib_hash::numeric_name(name);
        let mut current = key;
        for _ in 0..=self.collections.len() {
            // Equal names have equal numeric names, so matching on numeric
            // names alone is the same predicate the original scan used.
            let current_id = crate::attrib_hash::numeric_name(current);
            let item = self
                .entry(&class_id, &current_id)
                .ok_or_else(|| format!("Missing stock collection {class}/{current}"))?;
            if let Some(field) = item.fields.get(name).or_else(|| {
                item.fields
                    .iter()
                    .find(|(key, _)| crate::attrib_hash::numeric_name(key) == field_id)
                    .map(|(_, field)| field)
            }) {
                return Ok(field);
            }
            if item.parent.is_empty() {
                return Err(format!("Missing stock field {class}/{key}/{name}"));
            }
            current = &item.parent;
        }
        Err(format!("Cyclic stock collection inheritance {class}/{key}"))
    }

    pub fn float(&self, class: &str, key: &str, name: &str) -> Result<f32, String> {
        let field = self.field(class, key, name)?;
        if field.type_name != "EA::Reflection::Float" {
            return Err(format!("Expected float at {class}/{key}/{name}"));
        }
        let words = decode_words::<1>(&field.data)?;
        let value = f32::from_bits(words[0]);
        if !value.is_finite() {
            return Err(format!("Non-finite stock float {class}/{key}/{name}"));
        }
        Ok(value)
    }

    pub fn integer(&self, class: &str, key: &str, name: &str) -> Result<u32, String> {
        let field = self.field(class, key, name)?;
        if !matches!(
            field.type_name.as_str(),
            "EA::Reflection::Int32" | "EA::Reflection::UInt32"
        ) {
            return Err(format!("Expected integer at {class}/{key}/{name}"));
        }
        Ok(decode_words::<1>(&field.data)?[0])
    }

    pub fn boolean(&self, class: &str, key: &str, name: &str) -> Result<bool, String> {
        let field = self.field(class, key, name)?;
        if field.type_name != "EA::Reflection::Bool" {
            return Err(format!("Expected boolean at {class}/{key}/{name}"));
        }
        // Attributes store the byte followed by padding; layout bools may
        // contain only the byte. Reading a big-endian u32 gives the wrong bit.
        match field.data.get(..2) {
            Some("00") => Ok(false),
            Some("01") => Ok(true),
            _ => Err(format!("Invalid stock boolean {class}/{key}/{name}")),
        }
    }

    /// A non-text array field's elements as bytes (empty for a non-array field).
    pub fn raw_items(&self, class: &str, key: &str, name: &str) -> Result<Vec<Vec<u8>>, String> {
        let field = self.field(class, key, name)?;
        let Some(array) = &field.array else { return Ok(Vec::new()) };
        array
            .raw_items
            .iter()
            .map(|item| {
                (0..item.len() / 2)
                    .map(|i| u8::from_str_radix(&item[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
                    .collect()
            })
            .collect()
    }

    pub fn words<const N: usize>(
        &self,
        class: &str,
        key: &str,
        name: &str,
    ) -> Result<[u32; N], String> {
        decode_words(&self.field(class, key, name)?.data)
    }
}

fn decode_words<const N: usize>(text: &str) -> Result<[u32; N], String> {
    let hex: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if hex.len() != N * 8 || !hex.is_ascii() {
        return Err(format!(
            "Expected {N} big-endian words, found {} bytes of hex",
            hex.len()
        ));
    }
    let mut words = [0; N];
    for (i, word) in words.iter_mut().enumerate() {
        *word = u32::from_str_radix(&hex[i * 8..i * 8 + 8], 16)
            .map_err(|e| format!("Invalid collection payload: {e}"))?;
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_export_names_resolve_readable_manual_settings() {
        // Identities and payload from the prepared vault; no asset IO or gameplay.
        let data: Collections = serde_json::from_value(serde_json::json!({
            "version": 1, "collections": [{
                "class": "anim_motion", "key": crate::attrib_hash::numeric_name("manual"),
                "parent": "", "source": "fixture", "sha256": "",
                "fields": { "Hash_A353CE1670D3AA40": { "type": "EA::Reflection::Float", "data": "3D23D70A" } }
            }]
        })).unwrap();
        assert_eq!(
            data.float("anim_motion", "manual", "manual_clamp_vel")
                .unwrap()
                .to_bits(),
            0x3d23d70a
        );
        assert!(data.field("anim_motion", "manual", "missing").is_err());
    }

    #[test]
    fn readable_names_resolve_numeric_queries_through_inheritance() {
        let data: Collections = serde_json::from_value(serde_json::json!({
            "version": 1, "collections": [
                {"class":"example", "key":"child", "parent":crate::attrib_hash::numeric_name("base"),
                 "source":"fixture", "sha256":"", "fields":{}},
                {"class":"example", "key":"base", "parent":"", "source":"fixture", "sha256":"",
                 "fields":{"value":{"type":"EA::Reflection::Float", "data":"3F800000"}}}
            ]
        })).unwrap();
        assert_eq!(
            data.float(
                &crate::attrib_hash::numeric_name("example"),
                &crate::attrib_hash::numeric_name("child"),
                &crate::attrib_hash::numeric_name("value")
            )
            .unwrap(),
            1.0
        );
    }

    /// The original linear scan, kept as the reference for the index.
    fn field_linear<'a>(
        data: &'a Collections,
        class: &str,
        key: &str,
        name: &str,
    ) -> Result<&'a Field, String> {
        let class_id = crate::attrib_hash::numeric_name(class);
        let field_id = crate::attrib_hash::numeric_name(name);
        let mut current = key;
        for _ in 0..=data.collections.len() {
            let current_id = crate::attrib_hash::numeric_name(current);
            let item = data
                .collections
                .iter()
                .find(|c| {
                    (c.class_name == class
                        || crate::attrib_hash::numeric_name(&c.class_name) == class_id)
                        && (c.key == current
                            || crate::attrib_hash::numeric_name(&c.key) == current_id)
                })
                .ok_or_else(|| format!("Missing stock collection {class}/{current}"))?;
            if let Some(field) = item.fields.get(name).or_else(|| {
                item.fields
                    .iter()
                    .find(|(key, _)| crate::attrib_hash::numeric_name(key) == field_id)
                    .map(|(_, f)| f)
            }) {
                return Ok(field);
            }
            if item.parent.is_empty() {
                return Err(format!("Missing stock field {class}/{key}/{name}"));
            }
            current = &item.parent;
        }
        Err(format!("Cyclic stock collection inheritance {class}/{key}"))
    }

    fn same(data: &Collections, class: &str, key: &str, name: &str) {
        match (
            data.field(class, key, name),
            field_linear(data, class, key, name),
        ) {
            (Ok(a), Ok(b)) => assert!(std::ptr::eq(a, b), "{class}/{key}/{name}: different field"),
            (Err(a), Err(b)) => assert_eq!(a, b, "{class}/{key}/{name}"),
            (a, b) => panic!("{class}/{key}/{name}: indexed {a:?} vs linear {b:?}"),
        }
    }

    fn row(class: &str, key: &str, parent: &str, value: &str) -> serde_json::Value {
        serde_json::json!({"class": class, "key": key, "parent": parent, "source": "fixture", "sha256": "",
            "fields": {"value": {"type": "EA::Reflection::Float", "data": value}}})
    }

    #[test]
    fn index_matches_linear_scan_for_spellings_inheritance_and_errors() {
        let n = crate::attrib_hash::numeric_name;
        let data: Collections =
            serde_json::from_value(serde_json::json!({"version": 1, "collections": [
                row("example", "child", &n("base"), "00000001"),
                row("example", "base", "", "00000002"),
                row(&n("other"), "base", "", "00000003"),
                row("example", "loop_a", "loop_b", "00000004"),
                row("example", "loop_b", "loop_a", "00000005"),
            ]}))
            .unwrap();
        for class in ["example", &n("example"), "other", &n("other"), "missing"] {
            for key in ["child", "base", &n("base"), "0x0", "loop_a", "missing"] {
                for name in ["value", &n("value"), "absent"] {
                    same(&data, class, key, name);
                }
            }
        }
    }

    #[test]
    fn index_keeps_the_first_duplicate_like_the_scan() {
        // load() rejects duplicates, but a directly deserialized vault may not.
        let n = crate::attrib_hash::numeric_name;
        let data: Collections =
            serde_json::from_value(serde_json::json!({"version": 1, "collections": [
                row("example", "dup", "", "00000001"),
                row(&n("example"), &n("dup"), "", "00000002"),
            ]}))
            .unwrap();
        same(&data, "example", "dup", "value");
        assert_eq!(
            data.field("example", "dup", "value").unwrap().data,
            "00000001"
        );
    }

    #[test]
    fn override_profile_resets_the_index() {
        let mut data: Collections =
            serde_json::from_value(serde_json::json!({"version": 1, "collections": [
                row("physics_mode", "easy", "", "00000001"),
                row("physics_mode", "test", "", "00000002"),
            ]}))
            .unwrap();
        assert_eq!(
            data.field("physics_mode", "test", "value").unwrap().data,
            "00000002"
        ); // builds the index
        let mut fields = BTreeMap::new();
        fields.insert(
            "value".to_owned(),
            Field {
                type_name: "EA::Reflection::Float".into(),
                data: "00000009".into(),
                array: None,
            },
        );
        data.override_profile("physics_mode", "test", "easy", fields)
            .unwrap();
        assert_eq!(
            data.field("physics_mode", "test", "value").unwrap().data,
            "00000009"
        );
        same(&data, "physics_mode", "test", "value");
        same(&data, "physics_mode", "easy", "value");
    }

    /// Every (entry, field name) in the real converted vault, including names
    /// only reachable through inheritance, resolves identically.
    #[test]
    #[ignore = "requires SKATE3_ASSET_ROOT pointing to converted stock assets"]
    fn index_matches_linear_scan_on_private_collections() {
        let root = std::env::var_os("SKATE3_ASSET_ROOT").expect("set SKATE3_ASSET_ROOT");
        let data = Collections::load(std::path::Path::new(&root)).unwrap();
        let mut names_by_class: BTreeMap<String, std::collections::BTreeSet<String>> =
            BTreeMap::new();
        for item in &data.collections {
            names_by_class
                .entry(crate::attrib_hash::numeric_name(&item.class_name))
                .or_default()
                .extend(item.fields.keys().cloned());
        }
        let mut checked = 0usize;
        for item in &data.collections {
            for name in &names_by_class[&crate::attrib_hash::numeric_name(&item.class_name)] {
                same(&data, &item.class_name, &item.key, name);
                checked += 1;
            }
        }
        eprintln!(
            "compared {checked} lookups over {} entries",
            data.collections.len()
        );
        assert!(checked > 0);
    }
}
