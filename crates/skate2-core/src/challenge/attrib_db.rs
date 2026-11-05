//! Multi-Vault `Attrib::Database` view for the challenge and progression data.
//!
//! Class definitions come from `skater.vlt`; collections from every loaded
//! Vault (`ChallengeCore`, `progressioncore`) join the same class tables, and
//! a parent may live in another Vault. Lookups follow
//! `Collection::GetNode` (`0x8297b658`): the collection, then its parents,
//! then the class layout table when the original collection has a layout.
//! `Collection::GetData` (`0x8297b7e8`) resolves the node in its container's
//! layout; `Attribute::GetLength` (`0x8297a0a0`) uses the instance layout.

use skate2_formats::{
    attrib::{
        AttributeDefinition, AttributeExportKind, AttributePointerLocation, AttributePointerValue,
        AttributeVaultMetadata, string_hash64,
    },
    mount::MountSet,
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct AttribDbError(pub String);

impl fmt::Display for AttribDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Attrib database: {}", self.0)
    }
}

impl std::error::Error for AttribDbError {}

type Result<T> = std::result::Result<T, AttribDbError>;

/// The Vaults the challenge system reads, in native load order.
pub const CAREER_VAULTS: [&str; 3] = [
    "data\\db\\skater.vlt",
    "data\\db\\ChallengeCore.vlt",
    "data\\db\\progressioncore.vlt",
];

/// A byte position inside one Vault buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Location {
    vault: u16,
    dependency: u16,
    offset: u32,
}

impl Location {
    /// The location `by` bytes further into the same buffer.
    #[must_use]
    pub const fn offset_by(self, by: u32) -> Self {
        Self { offset: self.offset + by, ..self }
    }
}

#[derive(Clone)]
struct Vault {
    buffers: Vec<Arc<[u8]>>,
    pointers: BTreeMap<AttributePointerLocation, AttributePointerValue>,
}

/// PC port (`crate::skate3`): another database's node, its slot moved into a vault adopted here
/// ([`AttribDb::adopt_vault`]), for [`AttribDb::graft`].
#[derive(Clone, Copy, Debug)]
pub struct ForeignNode(Node);

impl ForeignNode {
    /// Whether its value lives in its own collection's layout (`Node` flag 0x10), which a graft
    /// onto another collection's layout cannot carry.
    #[must_use]
    pub fn is_layout(&self) -> bool {
        self.0.flags & 0x10 != 0
    }
}

/// PC port (`crate::skate3`): values made for [`AttribDb::graft`] rather than read from a vault,
/// which [`AttribDb::synthesize`] keeps in a vault of their own: each value's pointer slot, the
/// value, and the texts its `char*`s name, with the relocations between them.
#[derive(Clone, Debug, Default)]
pub struct Synthesis {
    bytes: Vec<u8>,
    /// (slot, target): the pointer at `slot` points at `target`, both in `bytes`.
    pointers: Vec<(u32, u32)>,
    /// Each value's node flags and slot.
    nodes: Vec<(u8, u32)>,
}

/// One element of a synthesized `Attrib::Array`.
#[derive(Clone, Copy, Debug)]
pub enum Element<'a> {
    /// Bytes as they are, zero-filled or cut to the element size.
    Bytes(&'a [u8]),
    /// A `char*` (`tLocationID`, `EA::Reflection::Text`) to the text.
    Text(&'a str),
}

impl Synthesis {
    fn offset(&self) -> u32 {
        u32::try_from(self.bytes.len()).expect("a small synthesis")
    }

    fn align(&mut self, to: usize) {
        while !self.bytes.len().is_multiple_of(to) {
            self.bytes.push(0);
        }
    }

    /// A zeroed pointer word, aligned.
    fn slot(&mut self) -> u32 {
        self.align(4);
        let at = self.offset();
        self.bytes.extend([0; 4]);
        at
    }

    fn text(&mut self, text: &str) -> u32 {
        let at = self.offset();
        self.bytes.extend(text.as_bytes());
        self.bytes.push(0);
        at
    }

    /// A single `char*` value; the vault keeps a word-sized value inline (`Node` flag 0x40), its
    /// slot holding the pointer. The index of its node.
    pub fn text_value(&mut self, text: &str) -> usize {
        let slot = self.slot();
        let target = self.text(text);
        self.pointers.push((slot, target));
        self.nodes.push((0x80 | 0x40, slot));
        self.nodes.len() - 1
    }

    /// An `Attrib::Array` of `size`-byte elements (its header: capacity, count, element size and
    /// flags, each a big-endian `u16`). The index of its node.
    pub fn array(&mut self, size: u16, elements: &[Element<'_>]) -> usize {
        let slot = self.slot();
        self.align(16);
        let array = self.offset();
        let count = u16::try_from(elements.len()).expect("a small array");
        for half in [count, count, size, 0] {
            self.bytes.extend(half.to_be_bytes());
        }
        let mut texts = Vec::new();
        for element in elements {
            let at = self.offset();
            let mut bytes = vec![0; usize::from(size)];
            match element {
                Element::Bytes(value) => {
                    let len = value.len().min(bytes.len());
                    bytes[..len].copy_from_slice(&value[..len]);
                }
                Element::Text(text) => texts.push((at, *text)),
            }
            self.bytes.extend(bytes);
        }
        for (at, text) in texts {
            let target = self.text(text);
            self.pointers.push((at, target));
        }
        self.pointers.push((slot, array));
        self.nodes.push((0x80 | 0x02, slot));
        self.nodes.len() - 1
    }

    /// A word-sized value as it is, inline. The index of its node.
    pub fn word(&mut self, word: [u8; 4]) -> usize {
        let slot = self.slot();
        self.bytes[slot as usize..slot as usize + 4].copy_from_slice(&word);
        self.nodes.push((0x80 | 0x40, slot));
        self.nodes.len() - 1
    }

    /// A value of `bytes` behind its slot, each `(offset, text)` of `texts` a `char*` in it to
    /// the text. The index of its node.
    pub fn value(&mut self, bytes: &[u8], texts: &[(u32, String)]) -> usize {
        let slot = self.slot();
        self.align(16);
        let value = self.offset();
        self.bytes.extend(bytes);
        self.texts(value, texts);
        self.pointers.push((slot, value));
        self.nodes.push((0x80, slot));
        self.nodes.len() - 1
    }

    /// An `Attrib::Array` of `count` elements of `size` bytes, `bytes` as they are and each
    /// `(offset, text)` of `texts` a `char*` in them to the text. The index of its node.
    pub fn raw_array(&mut self, count: u16, size: u16, bytes: &[u8], texts: &[(u32, String)]) -> usize {
        let slot = self.slot();
        self.align(16);
        let array = self.offset();
        for half in [count, count, size, 0] {
            self.bytes.extend(half.to_be_bytes());
        }
        self.bytes.extend(bytes);
        self.texts(array + 8, texts);
        self.pointers.push((slot, array));
        self.nodes.push((0x80 | 0x02, slot));
        self.nodes.len() - 1
    }

    /// An `Attrib::Blob`: its size and a pointer to `bytes`. The index of its node.
    pub fn blob(&mut self, bytes: &[u8]) -> usize {
        let slot = self.slot();
        self.align(16);
        let value = self.offset();
        self.bytes.extend(u32::try_from(bytes.len()).expect("a small blob").to_be_bytes());
        self.bytes.extend([0; 4]);
        self.align(16);
        let data = self.offset();
        self.bytes.extend(bytes);
        self.pointers.push((value + 4, data));
        self.pointers.push((slot, value));
        self.nodes.push((0x80, slot));
        self.nodes.len() - 1
    }

    /// The texts of `char*`s at offsets from `base`.
    fn texts(&mut self, base: u32, texts: &[(u32, String)]) {
        for (offset, text) in texts {
            let target = self.text(text);
            self.pointers.push((base + offset, target));
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Node {
    flags: u8,
    value: u32,
    /// Where a stored pointer's word lives (relocated through `PtrN`).
    slot: Option<Location>,
}

struct Class {
    definitions: HashMap<u64, AttributeDefinition>,
    layout_nodes: HashMap<u64, Node>,
    static_data: Option<Location>,
}

struct Collection {
    parent: u64,
    layout: Option<Location>,
    nodes: HashMap<u64, Node>,
}

pub struct AttribDb {
    vaults: Vec<Vault>,
    classes: HashMap<u64, Class>,
    collections: HashMap<(u64, u64), Collection>,
    sorted: HashMap<u64, Vec<u64>>,
    type_names: HashMap<u64, String>,
    /// Each class's `Attrib::ClassLoadData::mCollectionReserve`.
    collection_reserve: HashMap<u64, u32>,
    /// Each class's collections in load order (Vault order, then export order).
    loaded: HashMap<u64, Vec<u64>>,
    /// The downloadable content's unlock lists (category, `GetStringHash` id), read with its
    /// vaults by [`AttribDb::load_career`].
    dlc_unlocks: Vec<(u32, u32)>,
}

/// One resolved attribute, as `Attrib::Attribute`.
#[derive(Clone, Copy)]
pub struct Attribute<'a> {
    db: &'a AttribDb,
    pub definition: &'a AttributeDefinition,
    node: Node,
    container: (u64, u64),
    original: (u64, u64),
    class: u64,
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b[..4].try_into().expect("four bytes"))
}
fn be64(b: &[u8]) -> u64 {
    u64::from_be_bytes(b[..8].try_into().expect("eight bytes"))
}

impl AttribDb {
    /// No vaults loaded yet.
    pub(crate) fn empty() -> Self {
        Self {
            vaults: Vec::new(),
            classes: HashMap::new(),
            collections: HashMap::new(),
            sorted: HashMap::new(),
            type_names: HashMap::new(),
            collection_reserve: HashMap::new(),
            loaded: HashMap::new(),
            dlc_unlocks: Vec::new(),
        }
    }

    /// The challenge system's database: [`CAREER_VAULTS`], then each vault of the mounted
    /// downloadable content (`cVaultManager::StartDLCLoad` `0x82ab8b50` in the DLC flow), with
    /// that content's unlock lists.
    pub fn load_career(mounts: &MountSet) -> Result<Self> {
        let dlc = crate::dlc::vaults(mounts);
        let paths: Vec<&str> = CAREER_VAULTS.iter().copied().chain(dlc.iter().map(String::as_str)).collect();
        let mut db = Self::load(mounts, &paths)?;
        db.dlc_unlocks = crate::dlc::unlocks(mounts);
        Ok(db)
    }

    /// The downloadable content's unlocks, in the order its lists name them.
    #[must_use]
    pub fn dlc_unlocks(&self) -> &[(u32, u32)] {
        &self.dlc_unlocks
    }

    pub fn load(mounts: &MountSet, paths: &[&str]) -> Result<Self> {
        let mut db = Self::empty();
        for path in paths {
            db.read_vault(mounts, path)?;
        }
        db.sort_collections();
        Ok(db)
    }

    /// One Vault and its dependencies, read from `mounts` beside it.
    fn read_vault(&mut self, mounts: &MountSet, path: &str) -> Result<()> {
        let vlt = mounts.read_path(path).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
        let directory = path.rfind(['/', '\\']).map_or("", |end| &path[..end]);
        let metadata = AttributeVaultMetadata::parse(&vlt).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
        let mut buffers: Vec<Arc<[u8]>> = vec![Arc::from(vlt)];
        for dependency in &metadata.dependencies {
            let dep_path = format!("{directory}\\{}", dependency.name);
            let bytes = mounts.read_path(&dep_path).map_err(|e| AttribDbError(format!("{dep_path}: {e}")))?;
            buffers.push(Arc::from(bytes));
        }
        self.add_vault(path, &metadata, buffers)
    }

    fn sort_collections(&mut self) {
        self.sorted.clear();
        for &(class, collection) in self.collections.keys() {
            self.sorted.entry(class).or_default().push(collection);
        }
        for keys in self.sorted.values_mut() {
            keys.sort_unstable();
        }
    }

    /// PC port (`crate::skate3`): one more Vault after [`AttribDb::load`], read as it reads each.
    ///
    /// # Errors
    ///
    /// The Vault or a dependency is unreadable or malformed.
    pub fn load_more(&mut self, mounts: &MountSet, path: &str) -> Result<()> {
        self.read_vault(mounts, path)?;
        self.sort_collections();
        Ok(())
    }

    /// How many Vaults the database holds, in load order.
    #[must_use]
    pub fn vault_count(&self) -> u16 {
        u16::try_from(self.vaults.len()).unwrap_or(u16::MAX)
    }

    /// PC port (`crate::skate3`): takes in `source`'s Vault `index` (its buffers and
    /// relocations), whose nodes [`AttribDb::foreign_node`] then reads here; its index here.
    pub fn adopt_vault(&mut self, source: &AttribDb, index: u16) -> Option<u16> {
        let vault = source.vaults.get(usize::from(index))?.clone();
        let here = u16::try_from(self.vaults.len()).ok()?;
        self.vaults.push(vault);
        Some(here)
    }

    /// PC port (`crate::skate3`): the keys a class defines.
    #[must_use]
    pub fn definition_keys(&self, class: u64) -> Vec<u64> {
        let mut keys: Vec<u64> = self.classes.get(&class).map(|class| class.definitions.keys().copied().collect()).unwrap_or_default();
        keys.sort_unstable();
        keys
    }

    /// PC port (`crate::skate3`): `source`'s definitions of `class` that this database's class
    /// lacks (the whole class when it has none) and none of the classes `unless_defined_by`
    /// defines here, with their type names, read as this database's own. None of them has a
    /// layout node here, so a collection without the value reads none; and a field another of a
    /// challenge's data providers defines keeps being read there.
    pub fn merge_definitions(&mut self, source: &AttribDb, class: u64, unless_defined_by: &[u64]) {
        let Some(theirs) = source.classes.get(&class) else { return };
        let taken: Vec<u64> = theirs
            .definitions
            .keys()
            .copied()
            .filter(|&key| unless_defined_by.iter().any(|&other| self.definition(other, key).is_some()))
            .collect();
        let ours = self.classes.entry(class).or_insert_with(|| Class { definitions: HashMap::new(), layout_nodes: HashMap::new(), static_data: None });
        for (&key, definition) in &theirs.definitions {
            if ours.definitions.contains_key(&key) || taken.contains(&key) {
                continue;
            }
            ours.definitions.insert(key, definition.clone());
            if let Some(name) = source.type_names.get(&definition.type_id) {
                self.type_names.entry(definition.type_id).or_insert_with(|| name.clone());
            }
        }
    }

    /// PC port (`crate::skate3`): whether this database's `class` and `source`'s `source_class`
    /// both define `key`, alike: the same type, element size and arrayness.
    #[must_use]
    pub fn defines_alike(&self, class: u64, source: &AttribDb, source_class: u64, key: u64) -> bool {
        let (Some(ours), Some(theirs)) = (self.definition(class, key), source.definition(source_class, key)) else { return false };
        ours.type_id == theirs.type_id && ours.size == theirs.size && ours.flags & 1 == theirs.flags & 1
    }

    /// PC port (`crate::skate3`): a new collection `(class, key)` of `nodes` alone, with no
    /// parent or layout. False when `key` is taken.
    pub fn create(&mut self, class: u64, key: u64, nodes: &[(u64, ForeignNode)]) -> bool {
        if self.collections.contains_key(&(class, key)) {
            return false;
        }
        let nodes = nodes.iter().map(|&(key, ForeignNode(node))| (key, node)).collect();
        self.collections.insert((class, key), Collection { parent: 0, layout: None, nodes });
        self.loaded.entry(class).or_default().push(key);
        let sorted = self.sorted.entry(class).or_default();
        if let Err(at) = sorted.binary_search(&key) {
            sorted.insert(at, key);
        }
        true
    }

    /// PC port (`crate::skate3`): the value `(class, collection)` reads for `key` (through its
    /// parents and layout, as [`AttribDb::attribute`]) made again in `synthesis`, each `char*`
    /// in it (a relocated word naming a text) with its text. The index of its node; `None`
    /// without a value.
    pub fn copy_value(&self, class: u64, collection: u64, key: u64, synthesis: &mut Synthesis) -> Option<usize> {
        let attribute = self.attribute(class, collection, key)?;
        if attribute.is_array() {
            let base = attribute.array_base()?;
            let header = self.bytes(base, 8)?;
            let count = u16::from_be_bytes([header[2], header[3]]);
            let size = u16::from_be_bytes([header[4], header[5]]);
            let pad = u32::from((u16::from_be_bytes([header[6], header[7]]) >> 12) & 8);
            let start = base.offset_by(8 + pad);
            let len = usize::from(count) * usize::from(size);
            let bytes = self.bytes(start, len)?.to_vec();
            let texts = self.texts_in(start, len);
            return Some(synthesis.raw_array(count, size, &bytes, &texts));
        }
        let at = attribute.data(0)?;
        if attribute.is_inline() {
            if let Some(text) = self.text_at(at) {
                return Some(synthesis.text_value(text));
            }
            let word = self.bytes(at, 4)?;
            return Some(synthesis.word([word[0], word[1], word[2], word[3]]));
        }
        let size = usize::from(attribute.definition.size);
        let bytes = self.bytes(at, size)?.to_vec();
        let texts = self.texts_in(at, size);
        Some(synthesis.value(&bytes, &texts))
    }

    /// PC port (`crate::skate3`): the `Attrib::Blob` `(class, collection)` reads for `key`.
    #[must_use]
    pub fn blob_of(&self, class: u64, collection: u64, key: u64) -> Option<&[u8]> {
        self.attribute(class, collection, key)?.blob()
    }

    /// The texts of the relocated words in `len` bytes from `start`, by offset.
    fn texts_in(&self, start: Location, len: usize) -> Vec<(u32, String)> {
        (0..len)
            .step_by(4)
            .filter_map(|offset| {
                let offset = u32::try_from(offset).ok()?;
                let word = start.offset_by(offset);
                self.pointer_at(word)?;
                Some((offset, self.text_at(word)?.to_owned()))
            })
            .collect()
    }

    /// PC port (`crate::skate3`): the node `(class, collection)` resolves for `key` through its
    /// parents (not the class layout), its slot moved by `adopted` (this database's Vault
    /// indices to the adopting one's); `None` without one or when its Vault was not adopted.
    #[must_use]
    pub fn foreign_node(&self, class: u64, collection: u64, key: u64, adopted: &HashMap<u16, u16>) -> Option<ForeignNode> {
        let mut current = collection;
        loop {
            let entry = self.collections.get(&(class, current))?;
            if let Some(&node) = entry.nodes.get(&key) {
                let mut node = node;
                if let Some(slot) = node.slot.as_mut() {
                    slot.vault = *adopted.get(&slot.vault)?;
                }
                return Some(ForeignNode(node));
            }
            if entry.parent == 0 {
                return None;
            }
            current = entry.parent;
        }
    }

    /// PC port (`crate::skate3`): a [`Synthesis`]'s values as nodes of a vault of their own, in
    /// the order it made them, for [`AttribDb::graft`].
    pub fn synthesize(&mut self, synthesis: Synthesis) -> Vec<ForeignNode> {
        let vault = u16::try_from(self.vaults.len()).expect("few vaults");
        let pointers = synthesis
            .pointers
            .iter()
            .map(|&(slot, target)| {
                (
                    AttributePointerLocation { dependency_index: 0, offset: slot },
                    AttributePointerValue::Direct { dependency_index: 0, offset: u64::from(target) },
                )
            })
            .collect();
        self.vaults.push(Vault { buffers: vec![Arc::from(synthesis.bytes)], pointers });
        synthesis
            .nodes
            .iter()
            .map(|&(flags, slot)| ForeignNode(Node { flags, value: 0, slot: Some(Location { vault, dependency: 0, offset: slot }) }))
            .collect()
    }

    /// PC port (`crate::skate3`): a new collection `(class, key)` beside `template`, under its
    /// parent and with its layout: its own nodes but `drop`, then `nodes` over them. False when
    /// `template` is absent or `key` is taken.
    pub fn graft(&mut self, class: u64, key: u64, template: u64, drop: &[u64], nodes: &[(u64, ForeignNode)]) -> bool {
        if self.collections.contains_key(&(class, key)) {
            return false;
        }
        let Some(base) = self.collections.get(&(class, template)) else { return false };
        let mut own = base.nodes.clone();
        for key in drop {
            own.remove(key);
        }
        for &(key, ForeignNode(node)) in nodes {
            own.insert(key, node);
        }
        let collection = Collection { parent: base.parent, layout: base.layout, nodes: own };
        self.collections.insert((class, key), collection);
        self.loaded.entry(class).or_default().push(key);
        let sorted = self.sorted.entry(class).or_default();
        if let Err(at) = sorted.binary_search(&key) {
            sorted.insert(at, key);
        }
        true
    }

    fn add_vault(&mut self, path: &str, metadata: &AttributeVaultMetadata, buffers: Vec<Arc<[u8]>>) -> Result<()> {
        let index = u16::try_from(self.vaults.len()).expect("few vaults");
        let sizes: Vec<usize> = buffers.iter().map(|b| b.len()).collect();
        let pointers = metadata.resolve_pointers(&sizes).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
        let slices: Vec<&[u8]> = buffers.iter().map(AsRef::as_ref).collect();
        let vlt = &buffers[0];
        let direct = |offset: u32| match pointers.get(&AttributePointerLocation { dependency_index: 0, offset }) {
            Some(AttributePointerValue::Direct { dependency_index, offset }) => {
                Some(Location { vault: index, dependency: *dependency_index, offset: u32::try_from(*offset).ok()? })
            }
            _ => None,
        };
        for export in &metadata.exports {
            match export.kind() {
                AttributeExportKind::Database => {
                    let data = export.database_load_data(vlt).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
                    // Type names: `type_count` packed NUL-terminated strings.
                    if let Some(names) = direct(export.data_offset + 12) {
                        let buffer = &buffers[usize::from(names.dependency)];
                        let mut at = names.offset as usize;
                        for _ in 0..data.type_count {
                            let Some(len) = buffer.get(at..).and_then(|b| b.iter().position(|&c| c == 0)) else { break };
                            let name = String::from_utf8_lossy(&buffer[at..at + len]).into_owned();
                            self.type_names.insert(string_hash64(&name), name);
                            at += len + 1;
                        }
                    }
                }
                AttributeExportKind::Class => {
                    let header = export.class_load_data(vlt).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
                    let definitions =
                        export.class_definitions(&slices, &pointers).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
                    let mut class = Class {
                        definitions: HashMap::new(),
                        layout_nodes: HashMap::new(),
                        static_data: direct(export.data_offset + 24),
                    };
                    for definition in definitions {
                        // ClassPrivate ctor 0x82978d50: layout nodes for flag-2,
                        // non-static (flag 8) definitions.
                        if definition.flags & 2 != 0 && definition.flags & 8 == 0 {
                            let flags = 0x90
                                | if definition.flags & 1 != 0 { 2 } else { 0 }
                                | if definition.flags & 0x20 != 0 { 8 } else { 0 };
                            class
                                .layout_nodes
                                .insert(definition.key, Node { flags, value: u32::from(definition.offset), slot: None });
                        }
                        class.definitions.insert(definition.key, definition);
                    }
                    self.collection_reserve
                        .insert(header.class_id, header.collection_reserve);
                    self.classes.insert(header.class_id, class);
                }
                AttributeExportKind::Collection => {
                    let data = export.collection_load_data(vlt).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
                    let mut nodes = HashMap::with_capacity(data.entries.len());
                    for (i, entry) in data.entries.iter().enumerate() {
                        let relative = 48 + u32::from(data.types_length) * 8 + u32::try_from(i).expect("entry") * 16 + 8;
                        nodes.insert(
                            entry.key,
                            Node {
                                flags: entry.node_flags | 0x80,
                                value: entry.data_pointer,
                                slot: Some(Location { vault: index, dependency: 0, offset: export.data_offset + relative }),
                            },
                        );
                    }
                    self.collections.insert(
                        (data.class_id, data.collection_id),
                        Collection { parent: data.parent_id, layout: direct(export.data_offset + 40), nodes },
                    );
                    self.loaded
                        .entry(data.class_id)
                        .or_default()
                        .push(data.collection_id);
                }
                AttributeExportKind::Unknown(_) => {}
            }
        }
        self.vaults.push(Vault { buffers, pointers });
        Ok(())
    }

    /// Type name of a definition (`Database::GetIndexedTypeDesc`).
    #[must_use]
    pub fn type_name(&self, type_id: u64) -> Option<&str> {
        self.type_names.get(&type_id).map(String::as_str)
    }

    /// `Attrib::FindCollection`.
    #[must_use]
    pub fn has_collection(&self, class: u64, collection: u64) -> bool {
        self.collections.contains_key(&(class, collection))
    }

    /// The keys a collection's own nodes hold, not its parents'.
    pub(crate) fn own_keys(&self, class: u64, collection: u64) -> Vec<u64> {
        let mut keys: Vec<u64> = self.collections.get(&(class, collection)).map(|c| c.nodes.keys().copied().collect()).unwrap_or_default();
        keys.sort_unstable();
        keys
    }

    /// PC port (`crate::skate3`): the keys a collection and its parents hold, each once.
    pub(crate) fn inherited_keys(&self, class: u64, collection: u64) -> Vec<u64> {
        let mut keys = Vec::new();
        let mut current = collection;
        while let Some(entry) = self.collections.get(&(class, current)) {
            keys.extend(entry.nodes.keys().copied());
            if entry.parent == 0 {
                break;
            }
            current = entry.parent;
        }
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// `AttribSysUtils::GetCollections` (`0x82ab9510`): sorted keys.
    #[must_use]
    pub fn collections(&self, class: u64) -> &[u64] {
        self.sorted.get(&class).map_or(&[], Vec::as_slice)
    }

    /// `Attrib::Class::GetFirstCollection`/`GetNextCollection` (`0x8297a508`/`0x8297a518`), the
    /// walk `AttribSysUtils::GetCollections` (`0x82ab9510`) makes: the class's collection table in
    /// slot order. The table is a `VecHashMap` of `mCollectionReserve` slots (`ClassPrivate`
    /// `0x82978d50`) that each loaded collection joins (`InternalAdd` `0x82977ae0`).
    #[must_use]
    pub fn table_order(&self, class: u64) -> Vec<u64> {
        let reserve = self.collection_reserve.get(&class).copied().unwrap_or(0);
        let mut table = CollectionTable::new(usize::try_from(reserve).unwrap_or(0));
        for &key in self.loaded.get(&class).map_or(&[][..], Vec::as_slice) {
            table.add(key);
        }
        table.slots.into_iter().flatten().collect()
    }

    /// `AttribSysUtils::GetLeafCollections` (`0x82ab9710`): the sorted keys
    /// that are no collection's parent.
    #[must_use]
    pub fn leaf_collections(&self, class: u64) -> Vec<u64> {
        let all = self.collections(class);
        let mut parents: Vec<u64> = all.iter().filter_map(|&k| self.parent(class, k)).collect();
        parents.sort_unstable();
        all.iter().copied().filter(|k| parents.binary_search(k).is_err()).collect()
    }

    #[must_use]
    pub fn definition(&self, class: u64, key: u64) -> Option<&AttributeDefinition> {
        self.classes.get(&class)?.definitions.get(&key)
    }

    #[must_use]
    pub fn parent(&self, class: u64, collection: u64) -> Option<u64> {
        self.collections.get(&(class, collection)).map(|c| c.parent)
    }

    /// `Instance::Get(key)`: the attribute or `None` (`Attribute::IsValid`).
    #[must_use]
    pub fn attribute(&self, class: u64, collection: u64, key: u64) -> Option<Attribute<'_>> {
        let class_data = self.classes.get(&class)?;
        let definition = class_data.definitions.get(&key)?;
        let original = self.collections.get(&(class, collection))?;
        let mut current_key = collection;
        let mut current = original;
        loop {
            if let Some(node) = current.nodes.get(&key) {
                return Some(Attribute {
                    db: self,
                    definition,
                    node: *node,
                    container: (class, current_key),
                    original: (class, collection),
                    class,
                });
            }
            if current.parent == 0 {
                break;
            }
            current_key = current.parent;
            current = self.collections.get(&(class, current_key))?;
        }
        let node = class_data.layout_nodes.get(&key)?;
        original.layout?;
        Some(Attribute { db: self, definition, node: *node, container: (class, collection), original: (class, collection), class })
    }

    fn buffer(&self, location: Location) -> &[u8] {
        &self.vaults[usize::from(location.vault)].buffers[usize::from(location.dependency)]
    }

    #[must_use]
    pub fn bytes(&self, location: Location, len: usize) -> Option<&[u8]> {
        let start = usize::try_from(location.offset).ok()?;
        self.buffer(location).get(start..start.checked_add(len)?)
    }

    /// The relocated pointer stored at `slot`, if any.
    #[must_use]
    pub fn pointer_at(&self, slot: Location) -> Option<Location> {
        match self.vaults[usize::from(slot.vault)]
            .pointers
            .get(&AttributePointerLocation { dependency_index: slot.dependency, offset: slot.offset })?
        {
            AttributePointerValue::Direct { dependency_index, offset } => {
                Some(Location { vault: slot.vault, dependency: *dependency_index, offset: u32::try_from(*offset).ok()? })
            }
            _ => None,
        }
    }

    /// NUL-terminated text at a relocated `char*` slot.
    #[must_use]
    pub fn text_at(&self, slot: Location) -> Option<&str> {
        let target = self.pointer_at(slot)?;
        let buffer = self.buffer(target);
        let start = usize::try_from(target.offset).ok()?;
        let end = buffer[start..].iter().position(|&b| b == 0)? + start;
        std::str::from_utf8(&buffer[start..end]).ok()
    }

    fn offset(location: Location, by: u32) -> Location {
        Location { offset: location.offset + by, ..location }
    }

    fn collection_layout(&self, key: (u64, u64)) -> Option<Location> {
        self.collections.get(&key)?.layout
    }

    /// `Attrib::Instance::mLayoutPtr` word `index`; a missing collection or
    /// layout reads `DefaultDataArea` (zero).
    #[must_use]
    pub fn layout_word(&self, class: u64, collection: u64, index: u32) -> u32 {
        self.collection_layout((class, collection))
            .and_then(|at| self.bytes(Self::offset(at, index * 4), 4))
            .map_or(0, be32)
    }

    /// The byte at `offset` of a collection's layout (0 when missing).
    #[must_use]
    pub fn layout_byte(&self, class: u64, collection: u64, offset: u32) -> u8 {
        self.collection_layout((class, collection)).and_then(|at| self.bytes(Self::offset(at, offset), 1)).map_or(0, |b| b[0])
    }

    /// The `char*` at layout word `index`.
    #[must_use]
    pub fn layout_text(&self, class: u64, collection: u64, index: u32) -> Option<&str> {
        self.text_at(Self::offset(self.collection_layout((class, collection))?, index * 4))
    }
}

/// `VecHashMap<u64, Collection, Class::TablePolicy, 1, 96>`: the slot of each key.
struct CollectionTable {
    slots: Vec<Option<u64>>,
    entries: usize,
    /// `mWorstCollision`.
    worst: usize,
}

impl CollectionTable {
    fn new(size: usize) -> Self {
        Self {
            slots: vec![None; size],
            entries: 0,
            worst: 0,
        }
    }

    /// The size a full table grows to (`InternalAdd`, `Add`).
    const fn grown(size: usize) -> usize {
        if size == 0 {
            1
        } else {
            (20 * size / 16 + 3) & !3
        }
    }

    /// `RebuildTable` (`0x829778f8`): a table of the requested size that `CopyFromOldTable`
    /// (`0x829770f8`) refills from the old one in slot order, one slot larger again while a key
    /// still lands more than 96 slots past its own.
    fn rebuild(&mut self, requested: usize) {
        let mut size = requested;
        loop {
            let old = std::mem::replace(&mut self.slots, vec![None; size]);
            self.entries = 0;
            self.worst = 0;
            for key in old.into_iter().flatten() {
                self.internal_add(key);
            }
            if self.worst <= 96 {
                break;
            }
            size += 1;
        }
    }

    /// `InternalAdd` (`0x82977ae0`): a full table grows first; the key goes to its low word
    /// modulo the size, then to the next free slot, unless a slot it passes holds the same key.
    fn internal_add(&mut self, key: u64) -> bool {
        if self.entries == self.slots.len() {
            self.rebuild(Self::grown(self.slots.len()));
        }
        let size = self.slots.len();
        let mut slot = usize::try_from(key & 0xffff_ffff).unwrap_or(0) % size;
        let mut distance = 0;
        while let Some(present) = self.slots[slot] {
            if present == key {
                return false;
            }
            distance += 1;
            slot = (slot + 1) % size;
        }
        self.slots[slot] = Some(key);
        self.worst = self.worst.max(distance);
        self.entries += 1;
        true
    }

    /// `Add` (`0x8297a690`): a probe longer than 96 grows the table.
    fn add(&mut self, key: u64) {
        self.internal_add(key);
        if self.worst > 96 {
            self.rebuild(Self::grown(self.slots.len()));
        }
    }
}

impl<'a> Attribute<'a> {
    #[must_use]
    pub const fn is_array(&self) -> bool {
        self.node.flags & 2 != 0
    }

    /// Whether the value is the node's own word (`Node` flag 0x40).
    #[must_use]
    pub const fn is_inline(&self) -> bool {
        self.node.flags & 0x40 != 0
    }

    /// An array's `Attrib::Array` header.
    #[must_use]
    pub fn array_base(&self) -> Option<Location> {
        if !self.is_array() {
            return None;
        }
        self.base(self.db.collection_layout(self.container))
    }

    #[must_use]
    pub fn type_name(&self) -> Option<&'a str> {
        self.db.type_name(self.definition.type_id)
    }

    /// `Node::GetPointer`/`GetArray` against `layout`.
    fn base(&self, layout: Option<Location>) -> Option<Location> {
        let node = self.node;
        if node.flags & 0x10 != 0 {
            return Some(AttribDb::offset(layout?, node.value));
        }
        if node.flags & 0x20 != 0 {
            return Some(AttribDb::offset(self.db.classes.get(&self.class)?.static_data?, node.value));
        }
        self.db.pointer_at(node.slot?)
    }

    /// `Attribute::GetLength`.
    #[must_use]
    pub fn len(&self) -> usize {
        if !self.is_array() {
            return 1;
        }
        let Some(array) = self.base(self.db.collection_layout(self.original)) else { return 0 };
        self.db.bytes(array, 8).map_or(0, |h| usize::from(u16::from_be_bytes([h[2], h[3]])))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `Collection::GetData(key, index)`: `None` is a null pointer.
    #[must_use]
    pub fn data(&self, index: usize) -> Option<Location> {
        let layout = self.db.collection_layout(self.container);
        if !self.is_array() {
            if self.node.flags & 0x40 != 0 {
                // Inline: the node's own word, serialized at its entry slot.
                return self.node.slot;
            }
            return self.base(layout);
        }
        let array = self.base(layout)?;
        let header = self.db.bytes(array, 8)?;
        let count = usize::from(u16::from_be_bytes([header[2], header[3]]));
        let size = u32::from(u16::from_be_bytes([header[4], header[5]]));
        let pad = u32::from((u16::from_be_bytes([header[6], header[7]]) >> 12) & 8);
        if index >= count {
            return None;
        }
        Some(AttribDb::offset(array, 8 + pad + size * u32::try_from(index).ok()?))
    }

    /// Raw value bytes (`size` = the definition's element size).
    #[must_use]
    pub fn value_bytes(&self, index: usize) -> Option<Vec<u8>> {
        let size = usize::from(self.definition.size);
        self.db.bytes(self.data(index)?, size).map(<[u8]>::to_vec)
    }

    #[must_use]
    pub fn u32(&self, index: usize) -> Option<u32> {
        let b = self.value_bytes(index)?;
        (b.len() >= 4).then(|| be32(&b))
    }
    #[must_use]
    pub fn i32(&self, index: usize) -> Option<i32> {
        self.u32(index).map(u32::cast_signed)
    }
    #[must_use]
    pub fn i16(&self, index: usize) -> Option<i16> {
        let b = self.value_bytes(index)?;
        (b.len() >= 2).then(|| i16::from_be_bytes([b[0], b[1]]))
    }
    #[must_use]
    pub fn f32(&self, index: usize) -> Option<f32> {
        self.u32(index).map(f32::from_bits)
    }
    #[must_use]
    pub fn u64(&self, index: usize) -> Option<u64> {
        let b = self.value_bytes(index)?;
        (b.len() >= 8).then(|| be64(&b))
    }
    #[must_use]
    pub fn bool(&self, index: usize) -> Option<bool> {
        Some(self.value_bytes(index)?.first().copied()? != 0)
    }

    /// `EA::Reflection::Text`: a relocated `char*`.
    #[must_use]
    pub fn text(&self, index: usize) -> Option<&'a str> {
        self.db.text_at(self.data(index)?)
    }

    /// `Attrib::RefSpec`: (class key, collection key).
    #[must_use]
    pub fn ref_spec(&self, index: usize) -> Option<(u64, u64)> {
        let b = self.db.bytes(self.data(index)?, 16)?;
        Some((be64(&b[..8]), be64(&b[8..16])))
    }

    /// `Attrib::Blob`: (u32 size, relocated data pointer).
    #[must_use]
    pub fn blob(&self) -> Option<&'a [u8]> {
        let at = self.data(0)?;
        let size = usize::try_from(be32(self.db.bytes(at, 4)?)).ok()?;
        let target = self.db.pointer_at(AttribDb::offset(at, 4))?;
        self.db.bytes(target, size)
    }

    /// A relocated pointer field inside this attribute's value
    /// (e.g. `tLuaScript`).
    #[must_use]
    pub fn text_field(&self, index: usize) -> Option<&'a str> {
        self.db.text_at(self.data(index)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::mount::{MountedBig, SKATE2_ARCHIVES};

    pub(crate) fn mounted_career_db() -> AttribDb {
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\db\\db.big").unwrap();
        let mut mounts = MountSet::new();
        mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        AttribDb::load(&mounts, &CAREER_VAULTS).unwrap()
    }

    /// A synthesized value reads back through its node as a vault's would: an inline `char*`,
    /// an array of `char*`s, an array of records (cut and zero-filled to the element size) and
    /// an empty array.
    #[test]
    fn synthesized_values_read_back_through_their_nodes() {
        let mut synthesis = Synthesis::default();
        let start = synthesis.text_value("_1up_dwtn_02_startlocator_round_1");
        let starts = synthesis.array(4, &[Element::Text("round_1"), Element::Text("round_2")]);
        let records = synthesis.array(24, &[Element::Bytes(&[1, 2, 3]), Element::Bytes(&[9; 30])]);
        let none = synthesis.array(16, &[]);
        let mut db = AttribDb::empty();
        let made = db.synthesize(synthesis);
        let slot = |index: usize| made[index].0.slot.expect("a slot");
        assert_eq!(made[start].0.flags & 0x40, 0x40, "inline");
        assert_eq!(db.text_at(slot(start)), Some("_1up_dwtn_02_startlocator_round_1"));
        let array = |index: usize| {
            assert_eq!(made[index].0.flags & 0x02, 0x02, "an array");
            let at = db.pointer_at(slot(index)).expect("the array");
            let header = db.bytes(at, 8).expect("its header").to_vec();
            (at, u16::from_be_bytes([header[2], header[3]]), u16::from_be_bytes([header[4], header[5]]))
        };
        let (at, count, size) = array(starts);
        assert_eq!((count, size), (2, 4));
        assert_eq!(db.text_at(at.offset_by(8)), Some("round_1"));
        assert_eq!(db.text_at(at.offset_by(12)), Some("round_2"));
        let (at, count, size) = array(records);
        assert_eq!((count, size), (2, 24));
        let mut first = vec![0; 24];
        first[..3].copy_from_slice(&[1, 2, 3]);
        assert_eq!(db.bytes(at.offset_by(8), 24), Some(&first[..]));
        assert_eq!(db.bytes(at.offset_by(32), 24), Some(&[9; 24][..]));
        assert_eq!(array(none).1, 0);
    }

    /// A raw word, a value and an array with `char*`s in them, and a blob read back as a vault's.
    #[test]
    fn synthesized_raw_values_keep_their_texts() {
        let mut synthesis = Synthesis::default();
        let word = synthesis.word([0, 0, 0, 32]);
        let value = synthesis.value(&[1, 2, 3, 4, 0, 0, 0, 0, 5, 6], &[(4, "scene".to_owned())]);
        let array = synthesis.raw_array(2, 4, &[0; 8], &[(0, "round_1".to_owned()), (4, "round_2".to_owned())]);
        let blob = synthesis.blob(b"\x1bLua chunk");
        let mut db = AttribDb::empty();
        let made = db.synthesize(synthesis);
        let slot = |index: usize| made[index].0.slot.expect("a slot");
        assert_eq!(db.bytes(slot(word), 4), Some(&[0, 0, 0, 32][..]));
        let at = db.pointer_at(slot(value)).expect("the value");
        assert_eq!(db.bytes(at, 4), Some(&[1, 2, 3, 4][..]));
        assert_eq!(db.text_at(at.offset_by(4)), Some("scene"));
        assert_eq!(db.bytes(at.offset_by(8), 2), Some(&[5, 6][..]));
        let at = db.pointer_at(slot(array)).expect("the array");
        assert_eq!(db.text_at(at.offset_by(8)), Some("round_1"));
        assert_eq!(db.text_at(at.offset_by(12)), Some("round_2"));
        let at = db.pointer_at(slot(blob)).expect("the blob");
        assert_eq!(db.bytes(at, 4), Some(&10_u32.to_be_bytes()[..]));
        let data = db.pointer_at(at.offset_by(4)).expect("its data");
        assert_eq!(db.bytes(data, 10), Some(&b"\x1bLua chunk"[..]));
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn reads_first_mission_and_start_state_machine() {
        let db = mounted_career_db();
        let challenges = string_hash64("challenges");
        let mission = string_hash64("m_1a_01_goto_slappys");
        assert!(db.has_collection(challenges, mission));
        let objectives = db.attribute(challenges, mission, string_hash64("Objectives")).unwrap();
        assert!(objectives.len() >= 3);
        let (class, first) = objectives.ref_spec(0).unwrap();
        assert_eq!(class, string_hash64("challenge_objective"));
        assert_eq!(first, string_hash64("m_1a_01_goto_slappys_push"));

        let machines = string_hash64("progression_statemachines");
        let start = db
            .collections(machines)
            .iter()
            .copied()
            .find(|&c| db.attribute(machines, c, string_hash64("Name")).and_then(|a| a.text(0)) == Some("Start State Machine"))
            .unwrap();
        assert_eq!(db.attribute(machines, start, string_hash64("Starting")).unwrap().bool(0), Some(true));
        let states = db.attribute(machines, start, string_hash64("States")).unwrap();
        assert_eq!(states.len(), 6);
        assert_eq!(db.type_name(string_hash64("EA::Reflection::Text")), Some("EA::Reflection::Text"));
    }
}
