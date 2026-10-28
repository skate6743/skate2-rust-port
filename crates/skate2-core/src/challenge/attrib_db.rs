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

struct Vault {
    buffers: Vec<Arc<[u8]>>,
    pointers: BTreeMap<AttributePointerLocation, AttributePointerValue>,
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
            let vlt = mounts.read_path(path).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
            let directory = path.rfind(['/', '\\']).map_or("", |end| &path[..end]);
            let metadata = AttributeVaultMetadata::parse(&vlt).map_err(|e| AttribDbError(format!("{path}: {e}")))?;
            let mut buffers: Vec<Arc<[u8]>> = vec![Arc::from(vlt)];
            for dependency in &metadata.dependencies {
                let dep_path = format!("{directory}\\{}", dependency.name);
                let bytes = mounts.read_path(&dep_path).map_err(|e| AttribDbError(format!("{dep_path}: {e}")))?;
                buffers.push(Arc::from(bytes));
            }
            db.add_vault(path, &metadata, buffers)?;
        }
        for (&(class, collection), _) in &db.collections {
            db.sorted.entry(class).or_default().push(collection);
        }
        for keys in db.sorted.values_mut() {
            keys.sort_unstable();
        }
        Ok(db)
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
