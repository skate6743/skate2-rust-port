//! Snd9's AEMS runtime: compiled sound "modules" in `.abk` (`ABKC`) banks, instantiated when the
//! Csis class they subscribe to is created and interpreted by the "Aems Modules" timer
//! (`TIMERSTAGE_BEFORECOMMANDS`, every sixth mix frame). PLAYER nodes start Skate's sample
//! players (`SkateSamplePlayerFactory`), which mix to the mastering sub mix.
//!
//! Instances keep the native byte layout (big-endian, header included) so the bytecode's routes
//! and state offsets apply unchanged. Pointer fields hold this port's identifiers instead:
//! static-data pointers stay bank offsets (the static fixups are not applied), a
//! `PLAYERSTATE.pModuleBank` is the bank index plus one, `pSamplePlayer` and `Class*` are arena
//! indices plus one, and a `DESTROYSTATE` names its instance by index plus one.

mod interpret;
mod player;

pub use player::SamplePlayer;

use crate::csis::{Client, Csis, DescKind, Handle, InterfaceId};
use crate::rwac::{AudioCore, PluginRef};
use std::collections::HashMap;
use std::sync::Arc;

/// The id of the "Aems Modules" timer among the core's `TIMERSTAGE_BEFORECOMMANDS` timers.
pub const TIMER_ID: u32 = 0x4145_4D53;

/// The instance header (`MODULEINSTANCE`): the interpreted data starts after it.
const INSTANCE_HEADER: usize = 0x18;

pub(crate) fn rd32(data: &[u8], at: usize) -> i32 {
    data.get(at..at.wrapping_add(4)).map_or(0, |b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn wr32(data: &mut [u8], at: usize, value: i32) {
    if let Some(b) = data.get_mut(at..at.wrapping_add(4)) {
        b.copy_from_slice(&value.to_be_bytes());
    }
}

pub(crate) fn rd16(data: &[u8], at: usize) -> u16 {
    data.get(at..at.wrapping_add(2)).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]))
}

pub(crate) fn wr16(data: &mut [u8], at: usize, value: u16) {
    if let Some(b) = data.get_mut(at..at.wrapping_add(2)) {
        b.copy_from_slice(&value.to_be_bytes());
    }
}

pub(crate) fn rd8(data: &[u8], at: usize) -> u8 {
    data.get(at).copied().unwrap_or(0)
}

pub(crate) fn wr8(data: &mut [u8], at: usize, value: u8) {
    if let Some(b) = data.get_mut(at) {
        *b = value;
    }
}

pub(crate) fn rdf(data: &[u8], at: usize) -> f32 {
    f32::from_bits(rd32(data, at) as u32)
}

pub(crate) fn wrf(data: &mut [u8], at: usize, value: f32) {
    wr32(data, at, value.to_bits() as i32);
}

/// `iSNDrandom` 0x82c228c8: the six-word add-with-carry generator over `SNDIrandseed`, which
/// starts at zero. Unlike `rw::math::Random`, a word's carry is "the sum is below the old word",
/// which misses the carry when the added value wraps exactly.
#[derive(Debug, Clone, Default)]
pub struct SndRandom {
    seed: [u32; 6],
}

impl SndRandom {
    pub fn next_u32(&mut self) -> u32 {
        let s = &mut self.seed;
        let old5 = s[5];
        let s4 = s[4].wrapping_add(old5);
        let carry4 = u32::from(s4 < old5 || s4 < s[4]);
        let s3 = s[3].wrapping_add(s4).wrapping_add(carry4);
        let carry3 = u32::from(s3 < s[3]);
        let s2 = s[2].wrapping_add(s3).wrapping_add(carry3);
        let carry2 = u32::from(s2 < s[2]);
        let s1 = s[1].wrapping_add(s2).wrapping_add(carry2);
        let carry1 = u32::from(s1 < s[1]);
        let s0 = s[0].wrapping_add(s1).wrapping_add(carry1);
        *s = [s0, s1, s2, s3, s4, old5.wrapping_add(1)];
        if s[5] != 0 {
            return s0;
        }
        for index in (1..5).rev() {
            s[index] = s[index].wrapping_add(1);
            if s[index] != 0 {
                return s0;
            }
        }
        s[0] = s0.wrapping_add(1);
        s[0]
    }
}

/// `sndsintbl` 0x83016248: a quarter sine, 0..65535 over 257 entries.
const SIN_TABLE: [u16; 257] = [
    0, 402, 804, 1206, 1608, 2010, 2412, 2814, 3215, 3617, 4018, 4420, 4821, 5222, 5622, 6023,
    6423, 6823, 7223, 7623, 8022, 8421, 8819, 9218, 9616, 10013, 10410, 10807, 11204, 11600, 11995, 12390,
    12785, 13179, 13573, 13966, 14359, 14751, 15142, 15533, 15923, 16313, 16702, 17091, 17479, 17866, 18253, 18638,
    19024, 19408, 19792, 20175, 20557, 20938, 21319, 21699, 22078, 22456, 22833, 23210, 23586, 23960, 24334, 24707,
    25079, 25450, 25820, 26189, 26557, 26925, 27291, 27656, 28020, 28383, 28745, 29105, 29465, 29824, 30181, 30538,
    30893, 31247, 31600, 31952, 32302, 32651, 32999, 33346, 33692, 34036, 34379, 34721, 35061, 35400, 35738, 36074,
    36409, 36743, 37075, 37406, 37736, 38064, 38390, 38716, 39039, 39362, 39682, 40002, 40319, 40636, 40950, 41263,
    41575, 41885, 42194, 42501, 42806, 43110, 43412, 43712, 44011, 44308, 44603, 44897, 45189, 45480, 45768, 46055,
    46340, 46624, 46906, 47186, 47464, 47740, 48015, 48288, 48558, 48828, 49095, 49360, 49624, 49886, 50146, 50403,
    50660, 50914, 51166, 51416, 51665, 51911, 52155, 52398, 52639, 52877, 53114, 53348, 53581, 53811, 54040, 54266,
    54491, 54713, 54933, 55152, 55368, 55582, 55794, 56004, 56212, 56417, 56621, 56822, 57022, 57219, 57414, 57606,
    57797, 57986, 58172, 58356, 58538, 58718, 58895, 59070, 59243, 59414, 59583, 59749, 59913, 60075, 60235, 60392,
    60547, 60700, 60850, 60998, 61144, 61288, 61429, 61568, 61705, 61839, 61971, 62100, 62228, 62353, 62475, 62596,
    62714, 62829, 62942, 63053, 63162, 63268, 63371, 63473, 63571, 63668, 63762, 63854, 63943, 64030, 64115, 64197,
    64276, 64353, 64428, 64501, 64571, 64638, 64703, 64766, 64826, 64884, 64939, 64992, 65043, 65091, 65136, 65179,
    65220, 65258, 65294, 65327, 65358, 65386, 65412, 65436, 65457, 65475, 65491, 65505, 65516, 65524, 65531, 65534,
    65535,
];

/// `iSNDsin` 0x82c22aa0: a 1024-step period from the quarter table.
#[must_use]
pub fn snd_sin(angle: i32) -> i32 {
    let quadrant = (angle >> 8) & 3;
    let index = (angle & 0xFF) as usize;
    match quadrant {
        0 => i32::from(SIN_TABLE[index]),
        1 => i32::from(SIN_TABLE[256 - index]),
        2 => -i32::from(SIN_TABLE[index]),
        _ => -i32::from(SIN_TABLE[256 - index]),
    }
}

/// `AemsDef::Module`, parsed; its state lives here rather than in the bank bytes.
#[derive(Debug, Clone)]
pub struct Module {
    /// Offset of the record in the bank (the class handle is at +4).
    pub offset: usize,
    pub id: i32,
    pub cur_instances: i16,
    pub max_instances: i16,
    pub num_globals: u16,
    pub num_functions: u16,
    pub num_players: u8,
    pub class_destructor_present: bool,
    pub class_data_present: bool,
    pub num_class_controllers: u8,
    /// Bank offsets of the code and of the instance template (header included).
    pub pcode: usize,
    pub pdata: usize,
    pub data_size: usize,
    pub destroy_offset: usize,
    /// Instance offsets of the PLAYER and ControlClass states.
    pub player_offsets: Vec<usize>,
    pub class_controller_offsets: Vec<usize>,
    /// `moduleInstance`, head first.
    pub instances: Vec<usize>,
}

/// `AemsDef::ModuleBank`: the resident part (with the bound handles written in) and the file.
pub struct Bank {
    pub name: String,
    pub handle: i32,
    pub resident: Vec<u8>,
    pub file: Arc<[u8]>,
    /// `pSnd10SampleBankHeader`: offset of the `S10A` header in `file`.
    pub sample_bank: Option<usize>,
    pub stream_path: Option<String>,
    pub stream_offset: i32,
    pub modules: Vec<Module>,
}

pub struct Instance {
    pub bank: usize,
    pub module: usize,
    pub data: Vec<u8>,
}

/// `Snd9::CalculateAemsTimerFrequency` state.
#[derive(Debug, Clone, Copy)]
struct TimerState {
    min_update_rate: f32,
    previous_time_to_next_call: f32,
    skip: u32,
    multiplier: u32,
    /// `gAemsTimerPeriod` (milliseconds).
    period: f32,
}

/// The Csis registry and the AEMS banks, instances and sample players.
pub struct Aems {
    pub csis: Csis,
    pub banks: Vec<Option<Bank>>,
    bank_counter: i32,
    pub instances: Vec<Option<Instance>>,
    /// `sndaems.timerclient`, head first.
    timer_list: Vec<usize>,
    random: SndRandom,
    timer: TimerState,
    timer_registered: bool,
    pub players: Vec<Option<SamplePlayer>>,
    /// `Snd9::Aems::GetDrySubMix`: the mastering sub mix.
    pub dry_sub_mix: Option<PluginRef>,
    /// `GlobalEffects::mpEnvSendSubPlugInPtrs[0]`, the reverb sub mix every sample player's wet
    /// send feeds (none without `GlobalEffects`).
    pub env_send_sub_mix: Option<PluginRef>,
    /// The game's `Csis::g<Class>Handle` globals, by class name.
    class_handles: HashMap<String, Handle>,
    /// Timer passes, for the `SKATE2_TRACE_AUDIO` summary.
    trace_passes: u32,
    /// The `RemoveModuleBankHandler` commands `BeginRemoveModuleBank` posted, by bank handle.
    pending_bank_removals: Vec<i32>,
}

/// A game-side Csis class object (`Csis::<Class>` = `{Class* mpClass; <Class>Struct mData}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassObject {
    pub class: u32,
    pub data: Vec<i32>,
}

impl Aems {
    /// `Snd9::Aems::SetMinimumUpdateRate(30.0)` as `AudioCore::InitializeAudioCore` sets it.
    #[must_use]
    pub fn new(dry_sub_mix: Option<PluginRef>) -> Self {
        Self {
            csis: Csis::new(),
            banks: Vec::new(),
            bank_counter: 0,
            instances: Vec::new(),
            timer_list: Vec::new(),
            random: SndRandom::default(),
            timer: TimerState { min_update_rate: 30.0, previous_time_to_next_call: 0.0, skip: 0, multiplier: 0, period: 0.0 },
            timer_registered: false,
            players: Vec::new(),
            dry_sub_mix,
            env_send_sub_mix: None,
            class_handles: HashMap::new(),
            trace_passes: 0,
            pending_bank_removals: Vec::new(),
        }
    }

    /// `new Csis::<Class>(...)`: `Class::CreateInstance(&g<Class>Handle, &mData, &mpClass)`, binding
    /// the handle (`ClassHandle::Set(&<Class>Id)`) and retrying when it fails.
    pub fn create_object(&mut self, name: &str, data: Vec<i32>) -> ClassObject {
        let mut handle = self.class_handles.get(name).copied().unwrap_or_default();
        let class = self.class_create_named(&mut handle, name, &data);
        self.class_handles.insert(name.to_owned(), handle);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} aems create {name} class {class} instances {} data {data:?}", self.instance_count());
        }
        ClassObject { class, data }
    }

    /// PC port: [`Self::create_object`] of the class `name` the interface subscribed as `system`
    /// declares, not the first loaded interface's class of that name.
    pub fn create_object_in(&mut self, system: &str, name: &str, data: Vec<i32>) -> ClassObject {
        let key = format!("{system}|{name}");
        let mut handle = self.class_handles.get(&key).copied().unwrap_or_default();
        let (result, mut class) = self.class_create(&mut handle, &data);
        if result < 0 {
            if let Some((system_crc, interface_crc)) = self.csis.find_in(system, DescKind::Class, name) {
                self.csis.set_handle(&mut handle, DescKind::Class, InterfaceId { name, system_crc, interface_crc });
            }
            class = self.class_create(&mut handle, &data).1;
        }
        self.class_handles.insert(key, handle);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} aems create {system} {name} class {class} instances {} data {data:?}", self.instance_count());
        }
        ClassObject { class, data }
    }

    /// `Csis::Class::SetMemberData(mpClass, &mData)` when the class exists.
    pub fn set_member_data(&mut self, object: &ClassObject) {
        if object.class != 0 {
            self.send_parameters(object.class, &object.data);
        }
    }

    /// `Class::Release(mpClass)` and `Csis::System::Free`.
    pub fn release_object(&mut self, object: ClassObject) {
        if object.class != 0 {
            self.class_release(object.class);
        }
    }

    /// `Csis::System::Subscribe` of a loaded `.csi`.
    pub fn subscribe_interface(&mut self, name: &str, file: &[u8]) -> bool {
        self.csis.subscribe(name, file).is_some()
    }

    /// PC port: a second game's `.csi` behind every loaded one ([`Csis::subscribe_last`]).
    pub fn subscribe_interface_last(&mut self, name: &str, file: &[u8]) -> bool {
        self.csis.subscribe_last(name, file).is_some()
    }

    /// PC port: whether an interface was subscribed under `name`.
    #[must_use]
    pub fn has_interface(&self, name: &str) -> bool {
        self.csis.has_system(name)
    }

    /// `SNDAEMS_addmodulebank` 0x82c1b7d0 and `SNDAEMSI_resolvemodulebank` 0x82c1b478: bind the
    /// interface handles, subscribe each module's constructor client to its class, point the
    /// PLAYER states at the bank and start the timer with the first bank. Returns the handle.
    pub fn add_module_bank(&mut self, core: &mut AudioCore, name: &str, file: Arc<[u8]>) -> Option<i32> {
        if file.get(..4)? != b"ABKC" {
            return None;
        }
        self.bank_counter = self.bank_counter.wrapping_add(1);
        if self.bank_counter < 0 {
            self.bank_counter = 1;
        }
        let handle = self.bank_counter;
        let resident_size = rd32(&file, 0x18) as usize;
        let module_offset = rd32(&file, 0x1C) as usize;
        let sfx_offset = rd32(&file, 0x20);
        let interface_offset = rd32(&file, 0x38) as usize;
        let num_modules = rd16(&file, 0x0A);
        let mut resident = file.get(..resident_size)?.to_vec();
        let bank_index = self.banks.len();
        // Interface references: {handleOffset, IDOffset, type}, ID = {systemCrc, interfaceCrc, name}.
        let num_refs = rd32(&file, interface_offset).max(0) as usize;
        for index in 0..num_refs {
            let at = interface_offset + 4 + 12 * index;
            let handle_at = rd32(&file, at) as usize;
            let id_at = rd32(&file, at + 4) as usize;
            let kind = match rd8(&file, at + 8) {
                0 => DescKind::Global,
                1 => DescKind::Class,
                _ => DescKind::Function,
            };
            let name_end = file.get(id_at + 4..)?.iter().position(|&b| b == 0)?;
            let interface_name = String::from_utf8_lossy(&file[id_at + 4..id_at + 4 + name_end]).into_owned();
            let id = InterfaceId { name: &interface_name, system_crc: rd16(&file, id_at) as i16, interface_crc: rd16(&file, id_at + 2) as i16 };
            let mut bound = Handle { private: rd32(&resident, handle_at) as u32, key: rd32(&resident, handle_at + 4) };
            self.csis.set_handle(&mut bound, kind, id);
            wr32(&mut resident, handle_at, bound.private as i32);
            wr32(&mut resident, handle_at + 4, bound.key);
        }
        let mut modules = Vec::with_capacity(usize::from(num_modules));
        let mut at = module_offset;
        for index in 0..usize::from(num_modules) {
            let num_players = rd8(&file, at + 0x24);
            let num_class_controllers = rd8(&file, at + 0x27);
            let pdata = rd32(&file, at + 0x2C) as usize;
            let player_offsets: Vec<usize> = (0..usize::from(num_players)).map(|i| rd32(&file, at + 0x3C + 4 * i) as usize).collect();
            let class_controller_offsets: Vec<usize> =
                (0..usize::from(num_class_controllers)).map(|i| rd32(&file, at + 0x3C + 4 * (usize::from(num_players) + i)) as usize).collect();
            for &player in &player_offsets {
                wr32(&mut resident, pdata + player, bank_index as i32 + 1);
            }
            let mut class_handle = Handle { private: rd32(&resident, at + 4) as u32, key: rd32(&resident, at + 8) };
            self.csis.subscribe_client(&mut class_handle, Client::ModuleConstructor { bank: bank_index, module: index });
            wr32(&mut resident, at + 4, class_handle.private as i32);
            wr32(&mut resident, at + 8, class_handle.key);
            modules.push(Module {
                offset: at,
                id: rd32(&file, at),
                cur_instances: rd16(&file, at + 0x1C) as i16,
                max_instances: rd16(&file, at + 0x1E) as i16,
                num_globals: rd16(&file, at + 0x20),
                num_functions: rd16(&file, at + 0x22),
                num_players,
                class_destructor_present: rd8(&file, at + 0x25) != 0,
                class_data_present: rd8(&file, at + 0x26) != 0,
                num_class_controllers,
                pcode: rd32(&file, at + 0x28) as usize,
                pdata,
                data_size: rd32(&file, at + 0x30) as usize,
                destroy_offset: rd32(&file, at + 0x34) as usize,
                player_offsets,
                class_controller_offsets,
                instances: Vec::new(),
            });
            at += 0x3C + 4 * (usize::from(num_players) + usize::from(num_class_controllers));
        }
        let first = !self.banks.iter().any(Option::is_some);
        self.banks.push(Some(Bank {
            name: name.to_owned(),
            handle,
            resident,
            file,
            sample_bank: (sfx_offset != 0).then_some(sfx_offset as usize),
            stream_path: None,
            stream_offset: 0,
            modules,
        }));
        if first && !self.timer_registered {
            core.add_before_commands_timer(TIMER_ID);
            self.timer_registered = true;
        }
        Some(handle)
    }

    /// `Snd9::Aems::BeginRemoveModuleBank` 0x82c18f50: each module's constructor client leaves
    /// its class and its instances are destroyed (head first); the bank leaves the list when the
    /// posted `RemoveModuleBankHandler` 0x82c18e80 runs on the next audio frame. -10 when no
    /// bank has the handle.
    pub fn begin_remove_module_bank(&mut self, core: &mut AudioCore, handle: i32) -> i32 {
        let Some(bank_index) = self.banks.iter().position(|bank| bank.as_ref().is_some_and(|bank| bank.handle == handle)) else { return -10 };
        let modules = self.banks[bank_index].as_ref().map_or(0, |bank| bank.modules.len());
        for module_index in 0..modules {
            let Some((offset, instances)) = self.module(bank_index, module_index).map(|module| (module.offset, module.instances.clone())) else { continue };
            let Some(bank) = self.banks[bank_index].as_mut() else { continue };
            let mut class_handle = Handle { private: rd32(&bank.resident, offset + 4) as u32, key: rd32(&bank.resident, offset + 8) };
            self.csis.unsubscribe_client(&mut class_handle, Client::ModuleConstructor { bank: bank_index, module: module_index });
            if let Some(bank) = self.banks[bank_index].as_mut() {
                wr32(&mut bank.resident, offset + 4, class_handle.private as i32);
                wr32(&mut bank.resident, offset + 8, class_handle.key);
            }
            for slot in instances {
                self.destroy_instance(core, slot);
            }
        }
        self.pending_bank_removals.push(handle);
        crate::csis::OK
    }

    /// `Snd9::Aems::IsModuleBankRemoved` 0x82c19090: no bank in the list has the handle.
    #[must_use]
    pub fn is_module_bank_removed(&self, handle: i32) -> bool {
        !self.banks.iter().flatten().any(|bank| bank.handle == handle)
    }

    /// `Snd9::RemoveModuleBankHandler` 0x82c18e80 for each posted removal: the bank leaves the
    /// list, and the timer stops when no bank is left.
    fn remove_pending_banks(&mut self, core: &mut AudioCore) {
        for handle in std::mem::take(&mut self.pending_bank_removals) {
            if let Some(bank) = self.banks.iter_mut().find(|bank| bank.as_ref().is_some_and(|bank| bank.handle == handle)) {
                *bank = None;
            }
        }
        if self.timer_registered && !self.banks.iter().any(Option::is_some) {
            core.remove_before_commands_timer(TIMER_ID);
            self.timer_registered = false;
        }
    }

    /// `Csis::Class::CreateInstanceFast` 0x829b2ad0: a new `ClassData` (refcount 1), every
    /// constructor client in order, then the member data to the update clients.
    pub fn class_create(&mut self, handle: &mut Handle, member_data: &[i32]) -> (i32, u32) {
        let desc = match self.csis.validate(handle) {
            Ok(desc) => desc,
            Err(error) => return (error, 0),
        };
        let class = self.csis.alloc_class(desc);
        let clients = self.csis.descs[desc].clients.clone();
        for client in clients {
            if let Client::ModuleConstructor { bank, module } = client {
                self.create_module_instance(class, bank, module);
            }
        }
        self.send_parameters(class, member_data);
        (crate::csis::OK, class)
    }

    /// The game's `if (CreateInstance(&handle, ...) < 0) { handle.Set(&Id); CreateInstance(...); }`.
    pub fn class_create_named(&mut self, handle: &mut Handle, name: &str, member_data: &[i32]) -> u32 {
        let (result, class) = self.class_create(handle, member_data);
        if result >= 0 {
            return class;
        }
        if let Some((system_crc, interface_crc)) = self.csis.find(DescKind::Class, name) {
            self.csis.set_handle(handle, DescKind::Class, InterfaceId { name, system_crc, interface_crc });
        }
        self.class_create(handle, member_data).1
    }

    /// `Csis::ClassData::SendParameters` 0x829b1f78 (`SetMemberData`).
    pub fn send_parameters(&mut self, class: u32, member_data: &[i32]) {
        let Some(data) = self.csis.class(class) else { return };
        for client in data.update_clients.clone() {
            self.call_function_client(client, member_data);
        }
    }

    /// `Csis::Class::ReleaseFast` 0x829b2358: the destructor clients, then the creator's
    /// reference.
    pub fn class_release(&mut self, class: u32) {
        let Some(data) = self.csis.class(class) else { return };
        for client in data.destroy_clients.clone() {
            if let Client::Destructor { instance, state } = client {
                if let Some(instance) = self.instances.get_mut(instance).and_then(Option::as_mut) {
                    wr32(&mut instance.data, state + 0x10, 1);
                }
            }
        }
        self.csis.drop_class_ref(class);
    }

    /// `Csis::Function::CallFast` 0x829b2910.
    pub fn function_call(&mut self, handle: &mut Handle, parameters: &[i32]) -> i32 {
        let desc = match self.csis.validate(handle) {
            Ok(desc) => desc,
            Err(error) => return error,
        };
        let clients = self.csis.descs[desc].clients.clone();
        if clients.is_empty() {
            return crate::csis::ERROR_NO_CLIENTS;
        }
        for client in clients {
            self.call_function_client(client, parameters);
        }
        crate::csis::OK
    }

    /// `Csis::GlobalVariable::SetFast` 0x829b2c88: clients hear only a change.
    pub fn global_set(&mut self, handle: &mut Handle, value: i32) -> i32 {
        let desc = match self.csis.validate(handle) {
            Ok(desc) => desc,
            Err(error) => return error,
        };
        if self.csis.descs[desc].value == value {
            return crate::csis::OK;
        }
        self.csis.descs[desc].value = value;
        for client in self.csis.descs[desc].clients.clone() {
            self.call_function_client(client, &[value]);
        }
        crate::csis::OK
    }

    /// The `FunctionClient` callbacks: `SNDAEMSI_SetGlobalVariable` 0x82c18b78,
    /// `SNDAEMSI_SetClassData` 0x82c18b98 and `CsisFunctionCallback` 0x82c18bd8.
    fn call_function_client(&mut self, client: Client, parameters: &[i32]) {
        let parameter = |i: usize| parameters.get(i).copied().unwrap_or(0);
        match client {
            Client::Global { instance, state } => {
                if let Some(instance) = self.instances.get_mut(instance).and_then(Option::as_mut) {
                    wr32(&mut instance.data, state + 0x18, parameter(0));
                }
            }
            Client::ClassData { instance, state } => {
                if let Some(instance) = self.instances.get_mut(instance).and_then(Option::as_mut) {
                    let count = usize::from(rd8(&instance.data, state + 0x10));
                    for i in 0..count {
                        wr32(&mut instance.data, state + 0x14 + 4 * i, parameter(i));
                    }
                }
            }
            Client::Function { instance, state } => {
                if let Some(instance) = self.instances.get_mut(instance).and_then(Option::as_mut) {
                    let count = usize::from(rd8(&instance.data, state + 0x18));
                    for i in 0..count {
                        wr32(&mut instance.data, state + 0x1C + 4 * i, parameter(i));
                    }
                    wr8(&mut instance.data, state + 0x19, 1);
                }
            }
            Client::ModuleConstructor { .. } | Client::Destructor { .. } => {}
        }
    }

    fn module(&self, bank: usize, module: usize) -> Option<&Module> {
        self.banks.get(bank)?.as_ref()?.modules.get(module)
    }

    /// `SNDAEMSI_CreateModuleInstance` 0x82c18c20: copy the template, fill the destroy state,
    /// link the instance (head of the module's list and of the timer list), then subscribe its
    /// states: the class destructor, the globals (which hear their value at once), the class
    /// data and the functions.
    fn create_module_instance(&mut self, class: u32, bank_index: usize, module_index: usize) {
        let Some(module) = self.module(bank_index, module_index) else { return };
        if module.cur_instances >= module.max_instances {
            return;
        }
        let (pdata, size, destroy) = (module.pdata, module.data_size, module.destroy_offset);
        let (destructor, num_globals, class_data, num_functions) =
            (module.class_destructor_present, module.num_globals, module.class_data_present, module.num_functions);
        let Some(bank) = self.banks[bank_index].as_ref() else { return };
        let Some(template) = bank.resident.get(pdata..pdata + size) else { return };
        let mut data = template.to_vec();
        let slot = self.instances.iter().position(Option::is_none).unwrap_or(self.instances.len());
        wr32(&mut data, destroy, module_index as i32);
        wr32(&mut data, destroy + 4, slot as i32 + 1);
        wr32(&mut data, destroy + 8, class as i32);
        let instance = Instance { bank: bank_index, module: module_index, data };
        if slot == self.instances.len() {
            self.instances.push(Some(instance));
        } else {
            self.instances[slot] = Some(instance);
        }
        if let Some(module) = self.banks[bank_index].as_mut().and_then(|b| b.modules.get_mut(module_index)) {
            module.instances.insert(0, slot);
        }
        self.timer_list.insert(0, slot);
        let mut state = INSTANCE_HEADER;
        if destructor {
            self.csis.subscribe_class(class, Client::Destructor { instance: slot, state }, true);
            state += 0x14;
        }
        for _ in 0..num_globals {
            let mut handle = self.instance_handle(slot, state);
            if self.csis.subscribe_client(&mut handle, Client::Global { instance: slot, state }) == crate::csis::OK {
                let desc = handle.private as usize - 1;
                let value = self.csis.descs[desc].value;
                self.call_function_client(Client::Global { instance: slot, state }, &[value]);
            }
            self.set_instance_handle(slot, state, handle);
            state += 0x1C;
        }
        if class_data {
            self.csis.subscribe_class(class, Client::ClassData { instance: slot, state }, false);
            let outputs = self.instances[slot].as_ref().map_or(0, |i| usize::from(rd8(&i.data, state + 0x10)));
            state += 4 * (outputs + 5);
        }
        for _ in 0..num_functions {
            let mut handle = self.instance_handle(slot, state);
            self.csis.subscribe_client(&mut handle, Client::Function { instance: slot, state });
            self.set_instance_handle(slot, state, handle);
            let parameters = self.instances[slot].as_ref().map_or(0, |i| usize::from(rd8(&i.data, state + 0x18)));
            state += 4 * (parameters + 7);
        }
        if let Some(module) = self.banks[bank_index].as_mut().and_then(|b| b.modules.get_mut(module_index)) {
            module.cur_instances += 1;
        }
    }

    fn instance_handle(&self, instance: usize, at: usize) -> Handle {
        self.instances[instance].as_ref().map_or(Handle::default(), |i| Handle { private: rd32(&i.data, at) as u32, key: rd32(&i.data, at + 4) })
    }

    fn set_instance_handle(&mut self, instance: usize, at: usize, handle: Handle) {
        if let Some(instance) = self.instances[instance].as_mut() {
            wr32(&mut instance.data, at, handle.private as i32);
            wr32(&mut instance.data, at + 4, handle.key);
        }
    }

    /// `SNDAEMSI_updatedestroy` 0x82c174f8 for a triggered DESTROY: unlink, unsubscribe the states
    /// in creation order, release every sample player and controlled class, free.
    fn destroy_instance(&mut self, core: &mut AudioCore, slot: usize) {
        let Some(instance) = self.instances[slot].as_ref() else { return };
        let (bank_index, module_index) = (instance.bank, instance.module);
        let Some(module) = self.module(bank_index, module_index).cloned() else { return };
        let class = rd32(&instance.data, module.destroy_offset + 8) as u32;
        if let Some(module) = self.banks[bank_index].as_mut().and_then(|b| b.modules.get_mut(module_index)) {
            module.instances.retain(|&i| i != slot);
        }
        self.timer_list.retain(|&i| i != slot);
        let mut state = INSTANCE_HEADER;
        if module.class_destructor_present {
            self.csis.unsubscribe_class(class, Client::Destructor { instance: slot, state }, true);
            state += 0x14;
        }
        for _ in 0..module.num_globals {
            let mut handle = self.instance_handle(slot, state);
            self.csis.unsubscribe_client(&mut handle, Client::Global { instance: slot, state });
            self.set_instance_handle(slot, state, handle);
            state += 0x1C;
        }
        if module.class_data_present {
            self.csis.unsubscribe_class(class, Client::ClassData { instance: slot, state }, false);
            let outputs = self.instances[slot].as_ref().map_or(0, |i| usize::from(rd8(&i.data, state + 0x10)));
            state += 4 * (outputs + 5);
        }
        for _ in 0..module.num_functions {
            let mut handle = self.instance_handle(slot, state);
            self.csis.unsubscribe_client(&mut handle, Client::Function { instance: slot, state });
            self.set_instance_handle(slot, state, handle);
            let parameters = self.instances[slot].as_ref().map_or(0, |i| usize::from(rd8(&i.data, state + 0x18)));
            state += 4 * (parameters + 7);
        }
        for &player in &module.player_offsets {
            let id = self.instances[slot].as_ref().map_or(0, |i| rd32(&i.data, player + 8));
            if id != 0 {
                self.release_player(core, id as u32);
                if let Some(instance) = self.instances[slot].as_mut() {
                    wr32(&mut instance.data, player + 8, 0);
                }
            }
        }
        for &controller in &module.class_controller_offsets {
            let controlled = self.instances[slot].as_ref().map_or(0, |i| rd32(&i.data, controller + 8));
            if controlled != 0 {
                self.class_release(controlled as u32);
            }
        }
        if let Some(module) = self.banks[bank_index].as_mut().and_then(|b| b.modules.get_mut(module_index)) {
            module.cur_instances -= 1;
        }
        self.instances[slot] = None;
    }

    /// `SNDAEMSI_timerupdate` 0x82c1b3f0: every `n`-th call (`CalculateAemsTimerFrequency`
    /// 0x82c19118), every instance in timer-list order.
    pub fn timer_update(&mut self, core: &mut AudioCore, time_to_next_call: f32) {
        // The frame's commands run after its timers; a removed bank has no instances left, so
        // unlinking it first changes nothing.
        if !self.pending_bank_removals.is_empty() {
            self.remove_pending_banks(core);
        }
        if time_to_next_call != self.timer.previous_time_to_next_call {
            self.calculate_timer_frequency(time_to_next_call);
        }
        self.timer.skip = self.timer.skip.wrapping_sub(1);
        if self.timer.skip != 0 {
            return;
        }
        self.timer.skip = self.timer.multiplier;
        for slot in self.timer_list.clone() {
            if self.instances.get(slot).is_some_and(Option::is_some) {
                self.interpret(core, slot);
            }
        }
        self.trace_passes = self.trace_passes.wrapping_add(1);
        if self.trace_passes % 32 == 0 {
            if let Some(elapsed) = crate::splicer::trace_elapsed() {
                let modules: Vec<String> = self
                    .instances
                    .iter()
                    .flatten()
                    .map(|i| {
                        let bank = self.banks[i.bank].as_ref().map_or("?", |b| b.name.as_str());
                        let module = self.banks[i.bank].as_ref().map_or(0, |b| b.modules[i.module].id);
                        format!("{bank}#{module}")
                    })
                    .collect();
                eprintln!("{elapsed:9.3} aems players {} instances {modules:?}", self.players.iter().flatten().count());
            }
        }
    }

    fn calculate_timer_frequency(&mut self, tick: f32) {
        let limit = 1.0 / self.timer.min_update_rate;
        self.timer.previous_time_to_next_call = tick;
        let mut sum = 0.0f32;
        let mut count = 0u32;
        loop {
            sum += tick;
            count += 1;
            if !(sum + tick < limit) {
                break;
            }
        }
        self.timer.multiplier = count;
        self.timer.skip = count;
        self.timer.period = (count as f32 * tick) * 1000.0;
    }

    /// Instances alive (for diagnostics).
    #[must_use]
    pub fn instance_count(&self) -> usize {
        self.instances.iter().flatten().count()
    }
}
