//! `Csis` (csiscore): the systems of function, class and global-variable descriptors that `.csi`
//! (`MOIR`) files declare, the handles bound to them by CRC and name, and the client lists that
//! connect the game's calls to the AEMS modules subscribed to them.
//!
//! Handles keep the native `{mpPrivate, mKey}` pair: `private` is the descriptor's index plus one
//! (0 is a null pointer) and `key` the descriptor's `crcAndKey` word, so a stale or unbound handle
//! fails exactly as natively (-3, -6).

/// `Csis::Result` values.
pub const OK: i32 = 0;
pub const ERROR_ALLOC: i32 = -1;
pub const ERROR_STALE: i32 = -3;
pub const ERROR_NO_CLIENTS: i32 = -4;
pub const ERROR_NOT_FOUND: i32 = -5;
pub const ERROR_UNBOUND: i32 = -6;

/// `Csis::InterfaceHandleData` (`FunctionHandle`, `ClassHandle`, `GlobalVariableHandle`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Handle {
    pub private: u32,
    pub key: i32,
}

/// `Csis::InterfaceId`.
#[derive(Debug, Clone, Copy)]
pub struct InterfaceId<'a> {
    pub name: &'a str,
    pub system_crc: i16,
    pub interface_crc: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescKind {
    Function,
    Class,
    Global,
}

/// A subscriber: an AEMS module's class constructor client, or a state inside a module instance
/// (`instance`, byte offset of the state in the instance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    ModuleConstructor { bank: usize, module: usize },
    Destructor { instance: usize, state: usize },
    Global { instance: usize, state: usize },
    ClassData { instance: usize, state: usize },
    Function { instance: usize, state: usize },
}

/// `CsisDef::FunctionDesc` (functions and classes) and `CsisDef::GlobalVariableDesc`.
#[derive(Debug, Clone)]
pub struct Desc {
    pub kind: DescKind,
    pub name: String,
    pub crc: i16,
    /// `u.key` as a word: `crc << 16 | key`, -1 once unsubscribed.
    pub key_word: i32,
    /// `clients`, head first (constructor clients for a class).
    pub clients: Vec<Client>,
    /// `curVal` (globals).
    pub value: i32,
}

/// `CsisDef::SystemDesc`.
#[derive(Debug, Clone)]
struct SystemDesc {
    name: String,
    crc: i16,
    functions: std::ops::Range<usize>,
    classes: std::ops::Range<usize>,
    globals: std::ops::Range<usize>,
}

/// `Csis::ClassData`.
#[derive(Debug, Clone)]
pub struct ClassData {
    pub desc: usize,
    pub ref_count: i32,
    /// `mUpdateClients` and `mDestroyClients`, head first.
    pub update_clients: Vec<Client>,
    pub destroy_clients: Vec<Client>,
}

#[derive(Debug, Default)]
pub struct Csis {
    pub descs: Vec<Desc>,
    systems: Vec<SystemDesc>,
    /// `gSystems`, head first (indices into `systems`).
    order: Vec<usize>,
    /// `gUniqueKeyId`.
    unique_key: i16,
    /// `Class*` arena: a class pointer is its index plus one.
    pub classes: Vec<Option<ClassData>>,
}

fn be16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn c_string(bytes: &[u8], at: usize) -> Option<String> {
    let tail = bytes.get(at..)?;
    let end = tail.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&tail[..end]).into_owned())
}

impl Csis {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `Csis::System::Subscribe` 0x829b20c0: the descriptors follow the 40-byte header
    /// (12-byte function and class descriptors, 16-byte globals); each takes the next unique key
    /// (wrapping to 1 past 0x7FFF), and the system is pushed on the head of `gSystems`.
    pub fn subscribe(&mut self, name: &str, file: &[u8]) -> Option<usize> {
        if file.get(..4)? != b"MOIR" {
            return None;
        }
        let num_functions = usize::from(be16(file, 0x0A)?);
        let num_classes = usize::from(be16(file, 0x0C)?);
        let num_globals = usize::from(be16(file, 0x0E)?);
        let crc = be16(file, 0x10)? as i16;
        let next_key = |csis: &mut Self| {
            let mut key = csis.unique_key.wrapping_add(1);
            if key < 0 {
                key = 1;
            }
            csis.unique_key = key;
            key
        };
        let mut offset = 40;
        let mut add = |csis: &mut Self, kind: DescKind, count: usize| -> Option<std::ops::Range<usize>> {
            let start = csis.descs.len();
            for _ in 0..count {
                let (name_at, crc_at, value) = match kind {
                    DescKind::Global => (be32(file, offset + 8)?, offset + 12, be32(file, offset + 4)? as i32),
                    _ => (be32(file, offset + 4)?, offset + 8, 0),
                };
                let desc_crc = be16(file, crc_at)? as i16;
                let key = next_key(csis);
                csis.descs.push(Desc {
                    kind,
                    name: c_string(file, name_at as usize)?,
                    crc: desc_crc,
                    key_word: (i32::from(desc_crc) << 16) | i32::from(key as u16),
                    clients: Vec::new(),
                    value,
                });
                offset += if kind == DescKind::Global { 16 } else { 12 };
            }
            Some(start..csis.descs.len())
        };
        let functions = add(self, DescKind::Function, num_functions)?;
        let classes = add(self, DescKind::Class, num_classes)?;
        let globals = add(self, DescKind::Global, num_globals)?;
        self.systems.push(SystemDesc { name: name.to_owned(), crc, functions, classes, globals });
        let index = self.systems.len() - 1;
        self.order.insert(0, index);
        Some(index)
    }

    #[must_use]
    /// PC port: [`Self::subscribe`] with the system behind every other, so a lookup by name keeps
    /// finding the classes subscribed before it (a second game's interface).
    pub fn subscribe_last(&mut self, name: &str, file: &[u8]) -> Option<usize> {
        let index = self.subscribe(name, file)?;
        self.order.retain(|&system| system != index);
        self.order.push(index);
        Some(index)
    }

    /// PC port: whether a system was subscribed under `name`.
    #[must_use]
    pub fn has_system(&self, name: &str) -> bool {
        self.systems.iter().any(|system| system.name == name)
    }

    /// PC port: [`Self::find`] within the system subscribed under `system`.
    #[must_use]
    pub fn find_in(&self, system: &str, kind: DescKind, name: &str) -> Option<(i16, i16)> {
        let system = self.systems.iter().find(|candidate| candidate.name == system)?;
        let range = match kind {
            DescKind::Function => system.functions.clone(),
            DescKind::Class => system.classes.clone(),
            DescKind::Global => system.globals.clone(),
        };
        range.into_iter().find(|&index| self.descs[index].name == name).map(|index| (system.crc, self.descs[index].crc))
    }

    pub fn system_name(&self, index: usize) -> Option<&str> {
        self.systems.get(index).map(|s| s.name.as_str())
    }

    /// `Csis::SetHandle` 0x829b25e0 / 0x829b26f0 (`SetFast`, `Set`): the systems whose CRC
    /// matches are searched first, then every system; a descriptor matches on its CRC and name.
    pub fn set_handle(&self, handle: &mut Handle, kind: DescKind, id: InterfaceId<'_>) -> i32 {
        for try_unmatched in [false, true] {
            for &system in &self.order {
                let system = &self.systems[system];
                if system.crc != id.system_crc && !try_unmatched {
                    continue;
                }
                let range = match kind {
                    DescKind::Function => system.functions.clone(),
                    DescKind::Class => system.classes.clone(),
                    DescKind::Global => system.globals.clone(),
                };
                for index in range {
                    let desc = &self.descs[index];
                    if desc.crc == id.interface_crc && desc.name == id.name {
                        *handle = Handle { private: index as u32 + 1, key: desc.key_word };
                        return OK;
                    }
                }
            }
        }
        ERROR_NOT_FOUND
    }

    /// The handle check every `*Fast` call starts with: the descriptor, or the error (a stale
    /// handle is cleared to `{0, -3}`).
    pub fn validate(&self, handle: &mut Handle) -> Result<usize, i32> {
        if handle.key < 0 {
            return Err(handle.key);
        }
        if handle.private == 0 {
            return Err(ERROR_UNBOUND);
        }
        let index = handle.private as usize - 1;
        if self.descs.get(index).map(|d| d.key_word) != Some(handle.key) {
            *handle = Handle { private: 0, key: ERROR_STALE };
            return Err(ERROR_STALE);
        }
        Ok(index)
    }

    /// `Function::SubscribeFast` 0x829b29c8 (also classes' constructor clients): head insertion.
    pub fn subscribe_client(&mut self, handle: &mut Handle, client: Client) -> i32 {
        match self.validate(handle) {
            Ok(index) => {
                self.descs[index].clients.insert(0, client);
                OK
            }
            Err(error) => error,
        }
    }

    /// `Function::UnsubscribeFast` 0x829b2a40 / `GlobalVariable::UnsubscribeFast` 0x829b2e08.
    pub fn unsubscribe_client(&mut self, handle: &mut Handle, client: Client) -> i32 {
        match self.validate(handle) {
            Ok(index) => {
                let clients = &mut self.descs[index].clients;
                if let Some(at) = clients.iter().position(|c| *c == client) {
                    clients.remove(at);
                }
                OK
            }
            Err(error) => error,
        }
    }

    /// `ClassData` allocation for `Class::CreateInstanceFast` (refcount 1, no clients).
    pub fn alloc_class(&mut self, desc: usize) -> u32 {
        let data = ClassData { desc, ref_count: 1, update_clients: Vec::new(), destroy_clients: Vec::new() };
        if let Some(slot) = self.classes.iter().position(Option::is_none) {
            self.classes[slot] = Some(data);
            return slot as u32 + 1;
        }
        self.classes.push(Some(data));
        self.classes.len() as u32
    }

    #[must_use]
    pub fn class(&self, class: u32) -> Option<&ClassData> {
        self.classes.get((class as usize).checked_sub(1)?)?.as_ref()
    }

    pub fn class_mut(&mut self, class: u32) -> Option<&mut ClassData> {
        self.classes.get_mut((class as usize).checked_sub(1)?)?.as_mut()
    }

    /// The refcount drop shared by `ReleaseFast` and the `Unsubscribe*Fast` calls: the class is
    /// freed at zero.
    pub fn drop_class_ref(&mut self, class: u32) {
        let Some(data) = self.class_mut(class) else { return };
        data.ref_count -= 1;
        if data.ref_count == 0 {
            self.classes[class as usize - 1] = None;
        }
    }

    /// `Class::SubscribeDestructorFast` 0x829b2430 / `SubscribeMemberDataFast` 0x829b2508.
    pub fn subscribe_class(&mut self, class: u32, client: Client, destructor: bool) {
        if let Some(data) = self.class_mut(class) {
            let list = if destructor { &mut data.destroy_clients } else { &mut data.update_clients };
            list.insert(0, client);
            data.ref_count += 1;
        }
    }

    /// `Class::UnsubscribeDestructorFast` 0x829b2470 / `UnsubscribeMemberDataFast` 0x829b2548.
    pub fn unsubscribe_class(&mut self, class: u32, client: Client, destructor: bool) {
        if let Some(data) = self.class_mut(class) {
            let list = if destructor { &mut data.destroy_clients } else { &mut data.update_clients };
            if let Some(at) = list.iter().position(|c| *c == client) {
                list.remove(at);
            }
        }
        self.drop_class_ref(class);
    }

    /// The handle a game-side `ClassHandle::Set(&handle, &Id)` binds, for an interface found by
    /// name in any system.
    #[must_use]
    pub fn find(&self, kind: DescKind, name: &str) -> Option<(i16, i16)> {
        for &system in &self.order {
            let system = &self.systems[system];
            let range = match kind {
                DescKind::Function => system.functions.clone(),
                DescKind::Class => system.classes.clone(),
                DescKind::Global => system.globals.clone(),
            };
            for index in range {
                if self.descs[index].name == name {
                    return Some((system.crc, self.descs[index].crc));
                }
            }
        }
        None
    }
}
