//! `NFSMixMaster`, `NFSMixMap` and `NFSMixMapState` (0x829cae10-0x829d2a28). The map
//! (`MixMapSK8.mxb`) describes, per mixer state (`eMAINMAPSTATES`), curves on game inputs, mix
//! controls, 3D controls, event envelopes, sub channels and master channels; each state is
//! instantiated once per game state object (`IDynamicMixer::GetStateCount`). Game objects write
//! DMixIO input blocks and read the packed words the master channels mix into their output blocks.
//!
//! Native pointers become [`Source`] values resolved the way `GetObjectPtr` 0x829cc1f0 resolves
//! them. Every array keeps the native allocation order, which is also the processing order, and
//! instances index the flat `NFSMixMapState` block the way `m_pFirstInstance[n]` does (an instance
//! past a state's copies reads the next state's first instance). The map bytes stay mutable because
//! the native code patches records in place: `AddScaleIDs` writes the expanded scale count into each
//! record's top byte and `CreateEvtMixCtls` bumps zero-length envelope stages to one frame.

// Negated float comparisons keep the PowerPC branch outcomes for unordered (NaN) operands.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::shape::{
    azim_shape_output, cents_from_pitch_mult, curve_output, db_to_q15, fctiwz, float_curve_output, int_pitch_mult_from_cents,
    pitch_mult_from_cents, q15_to_db,
};
use super::tables::{DOPPLER_SMOOTHING_FACTOR, F_DT_FRAME_LOCK};

const BLOCK_WORDS: usize = 16;
/// `__real_41855557` (16.66667): envelope stage lengths are stored in frames.
const MS_PER_FRAME: f32 = f32::from_bits(0x4185_5557);
/// `__real_418553f8` (16.666): stage lengths below one frame transition instantly.
const INSTANT_MS: f32 = f32::from_bits(0x4185_53f8);

/// `eEnvelopeStage`.
const STAGE_OFF: u32 = 0;
const STAGE_ATTACK: u32 = 1;
const STAGE_DECAY: u32 = 2;
const STAGE_SUSTAIN: u32 = 3;
const STAGE_RELEASE: u32 = 4;

/// The game side of `Nicotine::IDynamicMixer`.
pub trait MixerHost {
    /// `GetStateCount` (`AudioSystem` 0x824edb90): the state manager's `m_CurNumStates`.
    fn state_count(&mut self, state: usize) -> i32;
    /// `ConnectDMixIO` (`AudioSystem` 0x824eda60): hands the block to the game object `id` names;
    /// false when no such object exists, and the block's output is then turned off.
    fn connect_dmixio(&mut self, id: u32) -> bool;
}

/// `Nicotine::DMixIO::DMX_PRESET_TYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetType {
    Volume = 0,
    Pitch = 1,
    Frequency = 2,
    Azimuth = 3,
    Depth = 4,
}

/// A `Nicotine::DMixIO`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DMixIoId(usize);

#[derive(Debug, Clone, Copy)]
struct DMixIo {
    id: u32,
    input: Option<usize>,
    output: Option<usize>,
}

/// What an `int *` from `GetObjectPtr` points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// `DUMMYINPUT`, never written.
    Dummy,
    /// A word of the DMixIO input blocks (`m_pDynMixInputBlocks`, contiguous).
    Input(usize),
    CurveQ15(usize),
    MixCtlDb(usize),
    MasterOutput(usize),
    SubOutput(usize),
    ThreeDDb(usize),
    ThreeDQ15(usize),
    /// `bHACKINIT`: the `st3DMixCtlProc` itself, for master channel 3D lists.
    ThreeDProc(usize),
    EvtDb(usize),
    EvtQ15(usize),
}

/// `stCurveDataProc`.
#[derive(Debug, Clone, Copy)]
struct Curve {
    input_id: i32,
    input: Source,
    db: i32,
    q15: i32,
}

/// The two halves of every `st*Proc`.
#[derive(Debug, Clone, Copy, Default)]
struct Proc {
    shared: usize,
    unique: usize,
}

/// `stMixCtlSharedData`.
#[derive(Debug, Clone, Copy, Default)]
struct MixCtlShared {
    params: usize,
    object_id: i32,
    offset: i32,
    ratio: i32,
}

/// `stMixCtlUniqueData`.
#[derive(Debug, Clone, Copy, Default)]
struct MixCtlUnique {
    curve: Option<usize>,
    scales: Option<usize>,
    cmp_db: i32,
}

/// `stEvtMixCtlSharedData`.
#[derive(Debug, Clone, Copy, Default)]
struct EvtShared {
    params: usize,
    offset: i32,
    ratio: i32,
}

/// `stEvtMixCtlUniqueData`.
#[derive(Debug, Clone, Copy)]
struct EvtUnique {
    stage: u32,
    elapsed: f32,
    q_start: i32,
    ms_start: f32,
    trigger_id: i32,
    trigger: Source,
    scales: Option<usize>,
    output: i32,
    qoutput: i32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

/// `st3DMixCtlSharedData`.
#[derive(Debug, Clone, Copy, Default)]
struct ThreeDShared {
    params: usize,
    state_params: usize,
    cur_cam: i32,
    prev_cam: i32,
    ms_since_cam: i32,
}

/// `st3DMixCtlUniqueData`.
#[derive(Debug, Clone, Copy)]
struct ThreeDUnique {
    inputs_id: i32,
    inputs: Option<usize>,
    azimuth: i32,
    db_rolloff: i32,
    q15_rolloff: i32,
    doppler_cents: i32,
    prev_dist: f32,
    prev_delta_dist: f32,
    min_max: [[f32; 2]; 4],
}

/// `stMixChSharedData`.
#[derive(Debug, Clone, Copy, Default)]
struct SubShared {
    params: usize,
    num_inputs: i32,
}

/// `stMixChUniqueData`.
#[derive(Debug, Clone, Copy, Default)]
struct SubUnique {
    inputs: Option<usize>,
    output: i32,
}

/// `stMasterMixChSharedData`.
#[derive(Debug, Clone, Copy, Default)]
struct MasterShared {
    params: usize,
    num_inputs: i32,
    presets: usize,
}

/// `stMasterMixChUniqueData`.
#[derive(Debug, Clone, Copy, Default)]
struct MasterUnique {
    output_id: i32,
    inputs: Option<usize>,
    output: i32,
    three_d: Option<usize>,
    outputs: usize,
}

/// `NFSMixMapState`: one instance of a mixer state.
#[derive(Debug, Clone, Copy, Default)]
struct MapState {
    state_index: usize,
    object_index: i32,
    ref_count: i32,
    first: usize,
    header: usize,
    mix_ctls: usize,
    mix_ctl_count: usize,
    three_d: usize,
    three_d_count: usize,
    events: usize,
    event_count: usize,
    subs: usize,
    sub_count: usize,
    masters: usize,
    master_count: usize,
    out_arrays: usize,
}

/// `NFSMixMap` with the `NFSMixMaster` that owns it.
pub struct MixMap {
    data: Vec<u8>,
    map_type: i32,
    header_states: usize,
    state_ref_count: [i32; 25],
    curve_totals: [[i32; 2]; 16],
    scale_id_count: usize,
    states: Vec<MapState>,
    state_procs: Vec<Option<usize>>,
    curves: Vec<Curve>,
    scale_ids: Vec<i32>,
    scales: Vec<Source>,
    mix_shared: Vec<MixCtlShared>,
    mix_unique: Vec<MixCtlUnique>,
    mix_procs: Vec<Proc>,
    evt_shared: Vec<EvtShared>,
    evt_unique: Vec<EvtUnique>,
    evt_procs: Vec<Proc>,
    three_d_shared: Vec<ThreeDShared>,
    three_d_unique: Vec<ThreeDUnique>,
    three_d_procs: Vec<Proc>,
    sub_shared: Vec<SubShared>,
    sub_unique: Vec<SubUnique>,
    sub_procs: Vec<Proc>,
    master_shared: Vec<MasterShared>,
    master_unique: Vec<MasterUnique>,
    master_procs: Vec<Proc>,
    sub_input_ids: Vec<i32>,
    sub_inputs: Vec<Source>,
    master_input_ids: Vec<i32>,
    master_inputs: Vec<Source>,
    output_words: Vec<i32>,
    input_words: Vec<i32>,
    dmixio: Vec<DMixIo>,
    prev_cam: i32,
    cur_cam: i32,
    delta_time: f32,
    ms_delta_time: f32,
    delta_time_ratio: [f32; 2],
}

fn clamp_db(value: i32) -> i32 {
    value.clamp(-10000, 0)
}

impl MixMap {
    /// `IDynamicMixer::InitMap` 0x829c9f48: `CreateMainMainMap`, then `NFSMixMaster::InitMixMap`
    /// 0x829caf30 for `num_states` mixer states (`MAP_CREATE_PARAMS::NumMixStates`).
    pub fn new(data: Vec<u8>, num_states: usize, host: &mut dyn MixerHost) -> Self {
        let mut map = Self {
            data,
            map_type: 0,
            header_states: 0,
            state_ref_count: [0; 25],
            curve_totals: [[0; 2]; 16],
            scale_id_count: 0,
            states: Vec::new(),
            state_procs: Vec::new(),
            curves: Vec::new(),
            scale_ids: Vec::new(),
            scales: Vec::new(),
            mix_shared: Vec::new(),
            mix_unique: Vec::new(),
            mix_procs: Vec::new(),
            evt_shared: Vec::new(),
            evt_unique: Vec::new(),
            evt_procs: Vec::new(),
            three_d_shared: Vec::new(),
            three_d_unique: Vec::new(),
            three_d_procs: Vec::new(),
            sub_shared: Vec::new(),
            sub_unique: Vec::new(),
            sub_procs: Vec::new(),
            master_shared: Vec::new(),
            master_unique: Vec::new(),
            master_procs: Vec::new(),
            sub_input_ids: Vec::new(),
            sub_inputs: Vec::new(),
            master_input_ids: Vec::new(),
            master_inputs: Vec::new(),
            output_words: Vec::new(),
            input_words: Vec::new(),
            dmixio: Vec::new(),
            prev_cam: 0,
            cur_cam: 0,
            delta_time: 0.0,
            ms_delta_time: 0.0,
            delta_time_ratio: [1.0, 1.0],
        };
        map.map_type = map.word(0);
        map.header_states = usize::try_from(map.word(4)).unwrap_or(0);
        map.pre_process(host);
        map.allocate_mixer_memory();
        let copies: Vec<i32> = (0..num_states).map(|state| map.map_state_copies(state)).collect();
        for (state, &count) in copies.iter().enumerate() {
            for object in 0..count {
                map.create_main_map_state(state, count, object);
            }
        }
        for state in 0..num_states {
            let mut object = 0;
            while object < map.state_ref_count.get(state).copied().unwrap_or(0) {
                if let Some(instance) = map.instance(state, object) {
                    map.create_sub_mix_channels(instance);
                    map.create_master_mix_channels(instance);
                }
                object += 1;
            }
        }
        map.setup_state_proc_arrays();
        map.connect_mix_map(host);
        map
    }

    fn word(&self, at: usize) -> i32 {
        self.data.get(at..at.wrapping_add(4)).map_or(0, |b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn set_word(&mut self, at: usize, value: i32) {
        if let Some(bytes) = self.data.get_mut(at..at.wrapping_add(4)) {
            bytes.copy_from_slice(&value.to_be_bytes());
        }
    }

    fn state_header(&self, state: usize) -> Option<usize> {
        let table = usize::try_from(self.word(8)).ok()?;
        usize::try_from(self.word(table + 4 * state)).ok()
    }

    fn ref_count(&self, state: i32) -> i32 {
        usize::try_from(state).ok().and_then(|state| self.state_ref_count.get(state)).copied().unwrap_or(0)
    }

    /// `GetMapStateCopies` 0x829cc100.
    fn map_state_copies(&self, state: usize) -> i32 {
        if state >= self.header_states { 0 } else { self.state_ref_count[state] }
    }

    /// `m_pStateProcs[state]->m_pFirstInstance[object]`.
    fn instance(&self, state: usize, object: i32) -> Option<usize> {
        let first = (*self.state_procs.get(state)?)?;
        let index = first.checked_add(usize::try_from(object).ok()?)?;
        (index < self.states.len()).then_some(index)
    }

    /// `PreProcessMixMap` 0x829cf288: the counts that size the curve and scale arrays.
    fn pre_process(&mut self, host: &mut dyn MixerHost) {
        for state in 0..self.header_states.min(25) {
            self.state_ref_count[state] = if self.state_header(state).is_some() { host.state_count(state) } else { 0 };
        }
        let mut scale_params = 0i32;
        for state in 0..self.header_states.min(25) {
            let Some(header) = self.state_header(state) else { continue };
            let copies = self.state_ref_count[state];
            let mix_offset = self.word(header + 4);
            if mix_offset != -1 {
                let base = header + mix_offset as usize;
                let mut record = base + 16;
                let mut seen: Vec<i32> = Vec::new();
                for _ in 0..self.word(base).max(0) {
                    let input = self.word(record);
                    let scale_count = (self.word(record + 4) >> 16) & 0xF;
                    if !seen.contains(&input) {
                        self.curve_totals[((input >> 24) & 0xF) as usize][0] += copies;
                    }
                    seen.push(input);
                    let mut scales = 0;
                    for k in 0..scale_count as usize {
                        let scale_state = (self.word(record + 8 + 4 * k) >> 16) & 0xFF;
                        scales += if scale_state as usize == state { 1 } else { self.ref_count(scale_state) };
                    }
                    scale_params += copies * scales;
                    record += 4 * (scale_count as usize + 2);
                }
            }
            let event_offset = self.word(header + 0x18);
            if event_offset != -1 {
                let base = header + event_offset as usize;
                let mut record = base + 16;
                let mut scales = 0;
                for _ in 0..self.word(base).max(0) {
                    let count = (self.word(record + 4) >> 16) & 0xF;
                    for k in 0..count as usize {
                        let scale_state = (self.word(record + 24 + 4 * k) >> 16) & 0xFF;
                        scales += if scale_state as usize == state { 1 } else { self.ref_count(scale_state) };
                    }
                    record += 4 * (count as usize + 6);
                }
                scale_params += copies * scales;
            }
        }
        let curves: i32 = self.curve_totals[..10].iter().map(|total| total[0]).sum();
        self.curves = vec![Curve { input_id: 0, input: Source::Dummy, db: 0, q15: 0 }; usize::try_from(curves).unwrap_or(0)];
        self.scale_ids = vec![0; usize::try_from(scale_params).unwrap_or(0)];
    }

    /// `AllocateMixerMemory` 0x829cf838 (the arrays that need more than growing).
    fn allocate_mixer_memory(&mut self) {
        self.state_procs = vec![None; self.header_states];
    }

    /// `NFSMixMapState::Initialize` 0x829d1390.
    fn initialize_state(&mut self, instance: usize, state: usize, object: i32) {
        self.states[instance].state_index = state;
        self.states[instance].object_index = object;
        if object == 0 {
            self.states[instance].ref_count = 1;
        } else {
            let first = self.states[instance].first;
            self.states[first].ref_count += 1;
            let count = self.states[first].ref_count;
            for k in 0..object as usize {
                if let Some(entry) = self.states.get_mut(first + k) {
                    entry.ref_count = count;
                }
            }
        }
    }

    /// `CreateMainMapState` 0x829cc128 with `AddMixState` 0x829d29b8.
    fn create_main_map_state(&mut self, state: usize, _copies: i32, object: i32) {
        let first = if let Some(first) = self.state_procs[state] {
            first
        } else {
            let first = self.states.len();
            self.states.push(MapState::default());
            self.state_procs[state] = Some(first);
            self.initialize_state(first, state, object);
            first
        };
        self.states[first].first = first;
        if object == 0 {
            self.initialize_state(first, state, 0);
        } else {
            let next = self.states.len();
            self.states.push(MapState { first, ..MapState::default() });
            self.initialize_state(next, state, object);
        }
        let Some(header) = self.state_header(state) else { return };
        let instance = first + object as usize;
        self.states[instance].header = header;
        self.create_mix_ctls(instance);
        self.create_3d_mix_ctls(instance);
        self.create_evt_mix_ctls(instance);
    }

    /// `GetCurveDataPtr` 0x829cbce0: one curve per distinct input id, grouped by curve table.
    fn curve_data_ptr(&mut self, id: i32) -> usize {
        let table = ((id >> 24) & 0xF) as usize;
        let start: i32 = self.curve_totals[..table].iter().map(|total| total[0]).sum();
        let start = start as usize;
        let assigned = self.curve_totals[table][1] as usize;
        if let Some(found) = (start..start + assigned).find(|&index| self.curves.get(index).is_some_and(|curve| curve.input_id == id)) {
            return found;
        }
        let index = start + assigned;
        self.curve_totals[table][1] += 1;
        let curve = Curve { input_id: id, input: Source::Dummy, db: 0, q15: 0x7FFF };
        if index < self.curves.len() {
            self.curves[index] = curve;
        } else {
            self.curves.resize(index + 1, curve);
        }
        index
    }

    fn push_scale_id(&mut self, offset: usize, id: i32) {
        let index = self.scale_id_count + offset;
        if index < self.scale_ids.len() {
            self.scale_ids[index] = id;
        } else {
            self.scale_ids.resize(index + 1, 0);
            self.scale_ids[index] = id;
        }
    }

    /// Both `AddScaleIDs` overloads (0x829cbda8 mix controls, 0x829cbea0 events): ids in the
    /// owner's state take its instance, ids of other states expand to one per copy of that state.
    fn add_scale_ids(&mut self, record: usize, first_id: usize, instance: i32, event: bool) -> Option<usize> {
        if event && self.scale_id_count == self.scale_ids.len() {
            return None;
        }
        let swing = self.word(record + 4);
        if swing & 0x1F_0000 == 0 {
            return None;
        }
        let start = self.scale_id_count;
        let owner_state = (self.word(record) >> 16) & 0xFF;
        let mut added = 0usize;
        for k in 0..((swing >> 16) & 0x1F) as usize {
            let id = self.word(first_id + 4 * k);
            let state = (id >> 16) & 0xFF;
            if state == owner_state {
                self.push_scale_id(added, (instance << 11) | id);
                added += 1;
            } else {
                for copy in 0..self.ref_count(state) {
                    self.push_scale_id(added, (copy << 11) | id);
                    added += 1;
                }
            }
        }
        let swing = self.word(record + 4);
        self.set_word(record + 4, ((added as i32) << 24) | (swing & 0x00FF_FFFF));
        self.scale_id_count += added;
        Some(start)
    }

    /// `AssignMixCtlDataPtrs` 0x829cbfd8: later instances share the last shared entry with the
    /// same `MIXCTLOBJID`.
    fn assign_mix_ctl_data_ptrs(&mut self, record: usize, object: i32, index: i32) -> Proc {
        let shared = if object == 0 {
            self.mix_shared.push(MixCtlShared::default());
            self.mix_shared.len() - 1
        } else {
            let input = self.word(record);
            let key = ((input >> 16) & 0xE000) | (self.map_type << 8) | (input & 0x0FFF_0000) | index;
            if let Some(found) = self.mix_shared.iter().rposition(|shared| shared.object_id == key) {
                found
            } else {
                self.mix_shared.push(MixCtlShared::default());
                self.mix_shared.len() - 1
            }
        };
        self.mix_unique.push(MixCtlUnique::default());
        Proc { shared, unique: self.mix_unique.len() - 1 }
    }

    /// `nOffset`/`nRatio` of mix and event controls: a negative swing scales toward silence, a
    /// positive one adds a millibel offset.
    fn offset_and_ratio(swing: i32) -> (i32, i32) {
        if swing & 0x8000 != 0 {
            (0, 0x7FFF - db_to_q15((swing as u32 | 0xFFFF_0000) as i32))
        } else {
            let offset = swing & 0x7FFF;
            (offset, 0x7FFF - db_to_q15(-offset))
        }
    }

    /// `CreateMixCtls` 0x829d1548.
    fn create_mix_ctls(&mut self, instance: usize) {
        let header = self.states[instance].header;
        let offset = self.word(header + 4);
        self.states[instance].mix_ctl_count = 0;
        if offset < 0 {
            return;
        }
        let base = header + offset as usize;
        if self.word(base) <= 0 {
            return;
        }
        self.states[instance].mix_ctls = self.mix_procs.len();
        let object = self.states[instance].object_index;
        let mut record = base + 16;
        let mut index = 0;
        while index < self.word(base) {
            let input = self.word(record);
            let swing_copy = self.word(record + 4);
            let curve = self.curve_data_ptr(((input as u32 & 0xFFFF_07FF) | ((object as u32) << 11)) as i32);
            let scales = self.add_scale_ids(record, record + 8, object, false);
            let proc = self.assign_mix_ctl_data_ptrs(record, object, index);
            self.mix_procs.push(proc);
            let (offset, ratio) = Self::offset_and_ratio(self.word(record + 4));
            let shared = &mut self.mix_shared[proc.shared];
            shared.object_id = (self.map_type << 8) | ((input >> 16) & 0xE000) | (input & 0x0FFF_0000) | index;
            shared.params = record;
            shared.offset = offset;
            shared.ratio = ratio;
            self.mix_unique[proc.unique] = MixCtlUnique { curve: Some(curve), scales, cmp_db: 0 };
            self.states[instance].mix_ctl_count += 1;
            index += 1;
            record += 4 * ((swing_copy >> 16) & 0x1F) as usize + 8;
        }
    }

    /// `Create3DMixCtls` 0x829d2388.
    fn create_3d_mix_ctls(&mut self, instance: usize) {
        let header = self.states[instance].header;
        let offset = self.word(header + 8);
        self.states[instance].three_d_count = 0;
        if offset < 0 {
            return;
        }
        let base = header + offset as usize;
        if self.word(base) <= 0 {
            return;
        }
        self.states[instance].three_d = self.three_d_procs.len();
        let object = self.states[instance].object_index;
        let first = self.states[instance].first;
        let mut record = base + 16;
        let mut index = 0;
        while index < self.word(base) {
            let shared = if object == 0 {
                self.three_d_shared.push(ThreeDShared { params: record, ..ThreeDShared::default() });
                self.three_d_shared.len() - 1
            } else {
                self.three_d_procs[self.states[first].three_d + index as usize].shared
            };
            self.three_d_shared[shared].state_params = record + 4;
            let params = self.three_d_shared[shared].params;
            let state_params = record + 4;
            let mut min_max = [[0.0f32; 2]; 4];
            for (quadrant, pair) in min_max.iter_mut().enumerate() {
                let word = self.word(state_params + 8 + 4 * quadrant);
                *pair = [(word & 0x7FFF) as f32, ((word >> 16) & 0x7FFF) as f32];
            }
            let input_id = self.word(params);
            self.three_d_unique.push(ThreeDUnique {
                inputs_id: (((object as u32) << 11) & 0x1FFF_F800 | 0x6000_0000 | (input_id as u32 & 0x1FFF_FFFF)) as i32,
                inputs: None,
                azimuth: 0,
                db_rolloff: 0,
                q15_rolloff: 0x7FFF,
                doppler_cents: 0,
                prev_dist: 1.0,
                prev_delta_dist: 1.0,
                min_max,
            });
            self.three_d_procs.push(Proc { shared, unique: self.three_d_unique.len() - 1 });
            self.states[instance].three_d_count += 1;
            index += 1;
            record += 24 * ((self.word(record) >> 24) & 0xF) as usize + 4;
        }
    }

    /// `CreateEvtMixCtls` 0x829d1e90.
    fn create_evt_mix_ctls(&mut self, instance: usize) {
        let header = self.states[instance].header;
        let offset = self.word(header + 0x18);
        self.states[instance].event_count = 0;
        if offset < 0 {
            return;
        }
        let base = header + offset as usize;
        if self.word(base) <= 0 {
            return;
        }
        self.states[instance].events = self.evt_procs.len();
        let object = self.states[instance].object_index;
        let first = self.states[instance].first;
        let mut record = base + 16;
        let mut index = 0usize;
        loop {
            let shared = if object == 0 {
                self.evt_shared.push(EvtShared { params: record, ..EvtShared::default() });
                self.evt_shared.len() - 1
            } else {
                self.evt_procs[self.states[first].events + index].shared
            };
            let params = self.evt_shared[shared].params;
            let bump = |map: &mut Self, at: usize| {
                let value = map.word(at);
                if value & 0xFFF == 0 {
                    map.set_word(at, value | 1);
                }
            };
            match (self.word(params) >> 24) & 0xF {
                0 | 2 => {
                    bump(self, params + 0xC);
                    bump(self, params + 0x14);
                }
                1 => {
                    bump(self, params + 0xC);
                    bump(self, params + 0x10);
                    bump(self, params + 0x14);
                }
                _ => {}
            }
            let (offset, ratio) = Self::offset_and_ratio(self.word(params + 4));
            self.evt_shared[shared].offset = offset;
            self.evt_shared[shared].ratio = ratio;
            let trigger_id = (object << 11) | self.word(params + 8);
            let scales = self.add_scale_ids(params, params + 24, object, true);
            self.evt_unique.push(EvtUnique {
                stage: STAGE_OFF,
                elapsed: 0.0,
                q_start: 0,
                ms_start: 0.0,
                trigger_id,
                trigger: Source::Dummy,
                scales,
                output: 0,
                qoutput: 0,
                attack: 0.0,
                decay: 0.0,
                sustain: 0.0,
                release: 0.0,
            });
            self.evt_procs.push(Proc { shared, unique: self.evt_unique.len() - 1 });
            self.states[instance].event_count += 1;
            index += 1;
            record += 4 * ((self.word(params + 4) >> 16) & 0xF) as usize + 24;
            if index as i32 >= self.word(base) {
                break;
            }
        }
    }

    /// `CreateSubMixChannels` 0x829d1b90.
    fn create_sub_mix_channels(&mut self, instance: usize) {
        let header = self.states[instance].header;
        let offset = self.word(header + 0xC);
        self.states[instance].sub_count = 0;
        if offset < 0 {
            return;
        }
        let base = header + offset as usize;
        if self.word(base) <= 0 {
            return;
        }
        self.states[instance].subs = self.sub_procs.len();
        let object = self.states[instance].object_index;
        let first = self.states[instance].first;
        let mut record = base + 16;
        let mut index = 0;
        while index < self.word(base) {
            let id = self.word(record);
            let shared = if object == 0 {
                self.sub_shared.push(SubShared { params: record, num_inputs: (id >> 16) & 0xFF });
                self.sub_shared.len() - 1
            } else {
                self.sub_procs[self.states[first].subs + index as usize].shared
            };
            self.sub_unique.push(SubUnique { inputs: None, output: 0 });
            self.sub_procs.push(Proc { shared, unique: self.sub_unique.len() - 1 });
            self.states[instance].sub_count += 1;
            index += 1;
            record += 4 * ((id >> 16) & 0xFF) as usize + 8;
        }
    }

    /// `CreateMasterMixChannels` 0x829d1cd8: consecutive channels of one SFX object share its
    /// 16-word output array.
    fn create_master_mix_channels(&mut self, instance: usize) {
        let header = self.states[instance].header;
        let offset = self.word(header + 0x10);
        self.states[instance].master_count = 0;
        if offset < 0 {
            return;
        }
        let base = header + offset as usize;
        let arrays = self.output_words.len();
        let unique_objects = usize::try_from(self.word(base + 4)).unwrap_or(0);
        self.output_words.resize(arrays + BLOCK_WORDS * unique_objects, 0);
        self.states[instance].out_arrays = arrays;
        if self.word(base) <= 0 {
            return;
        }
        self.states[instance].masters = self.master_procs.len();
        let object = self.states[instance].object_index;
        let first = self.states[instance].first;
        let mut record = base + 16;
        let mut previous_object = 0;
        let mut array_offset = 0usize;
        let mut index = 0;
        while index < self.word(base) {
            let id = self.word(record);
            let shared = if object == 0 {
                self.master_shared.push(MasterShared { params: record, num_inputs: (id >> 16) & 0xFF, presets: 0 });
                self.master_shared.len() - 1
            } else {
                self.master_procs[self.states[first].masters + index as usize].shared
            };
            let sfx_object = self.word(self.master_shared[shared].params + 8);
            let shared_array = (previous_object == sfx_object).then(|| (arrays + array_offset).checked_sub(BLOCK_WORDS)).flatten();
            let outputs = shared_array.unwrap_or_else(|| {
                let outputs = arrays + array_offset;
                array_offset += BLOCK_WORDS;
                outputs
            });
            if outputs + BLOCK_WORDS > self.output_words.len() {
                self.output_words.resize(outputs + BLOCK_WORDS, 0);
            }
            self.master_unique.push(MasterUnique {
                output_id: sfx_object | (object << 11),
                inputs: None,
                output: -10000,
                three_d: None,
                outputs,
            });
            self.master_procs.push(Proc { shared, unique: self.master_unique.len() - 1 });
            previous_object = sfx_object;
            self.states[instance].master_count += 1;
            index += 1;
            record += 4 * (((id >> 16) & 0xFF) as usize + 3);
        }
    }

    /// The expanded input ids of a channel input list: own-state ids take the instance, other
    /// states' ids one entry per copy (`InitializeSubChannels`/`InitializeMasterChannels`).
    fn expand_inputs(&self, ids: &[i32], state: usize, object: i32, count: i32, out: &mut Vec<i32>) {
        let mut done = 0;
        let mut list = ids.iter();
        while done < count {
            let Some(&raw) = list.next() else { break };
            let id = (raw as u32 & 0xFFFF_07FF) as i32;
            let id_state = (raw >> 16) & 0xFF;
            if id_state as usize == state {
                out.push((object << 11) | id);
                done += 1;
            } else {
                let copies = self.ref_count(id_state);
                for copy in 0..copies.max(0) {
                    out.push((copy << 11) | id);
                }
                done += copies;
            }
        }
    }

    /// `InitializeSubChannels` 0x829d2610.
    fn initialize_sub_channels(&mut self, instance: usize) {
        let state = self.states[instance];
        for proc in &self.sub_procs.clone()[state.subs..state.subs + state.sub_count] {
            let params = self.sub_shared[proc.shared].params;
            let listed = ((self.word(params) >> 16) & 0xFF) as usize;
            let ids: Vec<i32> = (0..listed).map(|k| self.word(params + 8 + 4 * k)).collect();
            let count: i32 =
                ids.iter().map(|&id| if ((id >> 16) & 0xFF) as usize == state.state_index { 1 } else { self.ref_count((id >> 16) & 0xFF) }).sum();
            let start = self.sub_input_ids.len();
            let mut expanded = Vec::new();
            self.expand_inputs(&ids, state.state_index, state.object_index, count, &mut expanded);
            expanded.resize(usize::try_from(count).unwrap_or(0), 0);
            self.sub_input_ids.extend(expanded);
            self.sub_unique[proc.unique].inputs = Some(start);
            self.sub_shared[proc.shared].num_inputs = count;
        }
    }

    /// `InitializeMasterChannels` 0x829d2780: 3D inputs (listed first) go after the others.
    fn initialize_master_channels(&mut self, instance: usize) {
        let state = self.states[instance];
        let mut presets = state.header.wrapping_add(self.word(state.header + 0x14) as usize);
        for proc in &self.master_procs.clone()[state.masters..state.masters + state.master_count] {
            let params = self.master_shared[proc.shared].params;
            self.master_shared[proc.shared].presets = presets;
            let preset_count = (self.word(presets) & 0xFF) as usize;
            let listed = ((self.word(params) >> 16) & 0xFF) as usize;
            let ids: Vec<i32> = (0..listed).map(|k| self.word(params + 12 + 4 * k)).collect();
            let mut normal = 0;
            let mut three_d = 0;
            for &id in &ids {
                if id as u32 & 0xE000_0000 == 0x8000_0000 {
                    three_d += 1;
                } else {
                    let id_state = (id >> 16) & 0xFF;
                    normal += if id_state as usize == state.state_index { 1 } else { self.ref_count(id_state) };
                }
            }
            let start = self.master_input_ids.len();
            let mut expanded = Vec::new();
            self.expand_inputs(&ids[three_d.min(ids.len())..], state.state_index, state.object_index, normal, &mut expanded);
            expanded.resize(usize::try_from(normal).unwrap_or(0), 0);
            for &id in &ids[..three_d.min(ids.len())] {
                expanded.push(((id as u32 & 0xFFFF_07FF) | ((state.object_index as u32) << 11)) as i32);
            }
            self.master_input_ids.extend(expanded);
            self.master_unique[proc.unique].inputs = Some(start);
            self.master_unique[proc.unique].three_d = (three_d > 0).then_some(start + normal as usize);
            self.master_shared[proc.shared].num_inputs = ((three_d as i32) << 16) | normal;
            presets += 4 * (preset_count + 1);
        }
    }

    /// `SetupStateProcArrays` 0x829cc820.
    fn setup_state_proc_arrays(&mut self) {
        for state in 0..self.header_states {
            let Some(first) = self.state_procs[state] else { continue };
            let mut object = 0;
            while object < self.states[first].ref_count {
                let instance = first + object as usize;
                if instance < self.states.len() {
                    self.initialize_sub_channels(instance);
                    self.initialize_master_channels(instance);
                }
                object += 1;
            }
        }
    }

    fn find_dmixio(&self, key: u32) -> Option<usize> {
        self.dmixio.iter().position(|io| io.id == key)
    }

    fn allocate_input_block(&mut self) -> usize {
        let block = self.input_words.len() / BLOCK_WORDS;
        self.input_words.resize(self.input_words.len() + BLOCK_WORDS, 0);
        block
    }

    fn turn_off_output(&mut self, io: usize) {
        if let Some(output) = self.dmixio[io].output {
            self.output_words[output + 15] = 0;
        }
    }

    /// `SETSFXID` 0x829cb738: the first output array registered for an object becomes its block.
    fn set_sfx_id(&mut self, id: i32, outputs: usize, host: &mut dyn MixerHost) -> bool {
        let key = id as u32 & 0xE0FF_FFF0;
        if let Some(io) = self.find_dmixio(key) {
            if self.dmixio[io].output.is_none() {
                self.dmixio[io].output = Some(outputs);
            }
            return true;
        }
        self.dmixio.push(DMixIo { id: key, input: None, output: Some(outputs) });
        if !host.connect_dmixio(key) {
            self.turn_off_output(self.dmixio.len() - 1);
        }
        true
    }

    /// `GetObjectPtr` 0x829cc1f0.
    fn object_ptr(&mut self, id: i32, used_db: bool, hack_init: bool, host: &mut dyn MixerHost) -> Source {
        let id = id as u32;
        let state = ((id >> 16) & 0xFF) as usize;
        let object = ((id >> 11) & 0x1F) as i32;
        let index = (id & 0xFF) as usize;
        match id & 0xE000_0000 {
            0x4000_0000 | 0x6000_0000 => {
                let key = id & 0xE0FF_FFF0;
                let io = if let Some(io) = self.find_dmixio(key) {
                    io
                } else {
                    let block = self.allocate_input_block();
                    self.dmixio.push(DMixIo { id: key, input: Some(block), output: None });
                    let io = self.dmixio.len() - 1;
                    if !host.connect_dmixio(key) {
                        self.turn_off_output(io);
                    }
                    io
                };
                let block = if let Some(block) = self.dmixio[io].input {
                    block
                } else {
                    let block = self.allocate_input_block();
                    self.dmixio[io].input = Some(block);
                    block
                };
                Source::Input(block * BLOCK_WORDS + (id & 0xF) as usize)
            }
            0x2000_0000 => {
                let Some(instance) = self.instance(state, object) else { return Source::Dummy };
                let entry = self.states[instance];
                if id & 0x1000_0000 == 0 {
                    if index < entry.master_count { Source::MasterOutput(self.master_procs[entry.masters + index].unique) } else { Source::Dummy }
                } else if index < entry.sub_count {
                    Source::SubOutput(self.sub_procs[entry.subs + index].unique)
                } else {
                    Source::Dummy
                }
            }
            0x8000_0000 => {
                let Some(instance) = self.instance(state, object) else { return Source::Dummy };
                let entry = self.states[instance];
                if index >= entry.three_d_count {
                    return Source::Dummy;
                }
                let proc = entry.three_d + index;
                let unique = self.three_d_procs[proc].unique;
                if hack_init {
                    Source::ThreeDProc(proc)
                } else if used_db {
                    Source::ThreeDDb(unique)
                } else {
                    Source::ThreeDQ15(unique)
                }
            }
            0x0000_0000 => {
                let Some(instance) = self.instance(state, object) else { return Source::Dummy };
                let entry = self.states[instance];
                if index >= entry.mix_ctl_count {
                    return Source::Dummy;
                }
                let unique = self.mix_procs[entry.mix_ctls + index].unique;
                if used_db {
                    Source::MixCtlDb(unique)
                } else {
                    self.mix_unique[unique].curve.map_or(Source::Dummy, Source::CurveQ15)
                }
            }
            0xA000_0000 => {
                let Some(instance) = self.instance(state, object) else { return Source::Dummy };
                let entry = self.states[instance];
                if index >= entry.event_count {
                    return Source::Dummy;
                }
                let unique = self.evt_procs[entry.events + index].unique;
                if used_db { Source::EvtDb(unique) } else { Source::EvtQ15(unique) }
            }
            _ => Source::Dummy,
        }
    }

    /// `ConnectMixMap` 0x829cc518.
    fn connect_mix_map(&mut self, host: &mut dyn MixerHost) {
        for index in 0..self.master_procs.len() {
            let unique = self.master_unique[self.master_procs[index].unique];
            let connected = self.set_sfx_id(unique.output_id, unique.outputs, host);
            let outputs = &mut self.output_words[unique.outputs..unique.outputs + BLOCK_WORDS];
            outputs[..15].fill(0);
            outputs[15] = i32::from(connected);
        }
        for index in 0..self.curves.len() {
            let id = self.curves[index].input_id;
            self.curves[index].input = self.object_ptr(id, false, false, host);
        }
        self.scales = Vec::with_capacity(self.scale_ids.len());
        for index in 0..self.scale_ids.len() {
            let source = self.object_ptr(self.scale_ids[index], false, false, host);
            self.scales.push(source);
        }
        for index in 0..self.three_d_procs.len() {
            let unique = self.three_d_procs[index].unique;
            let source = self.object_ptr(self.three_d_unique[unique].inputs_id, false, false, host);
            self.three_d_unique[unique].inputs = match source {
                Source::Input(word) => Some(word),
                _ => None,
            };
        }
        for index in 0..self.evt_procs.len() {
            let Proc { shared, unique } = self.evt_procs[index];
            let trigger = self.object_ptr(self.evt_unique[unique].trigger_id, false, false, host);
            let params = self.evt_shared[shared].params;
            let (p0, p1, p2) = (self.word(params + 0xC), self.word(params + 0x10), self.word(params + 0x14));
            let entry = &mut self.evt_unique[unique];
            entry.trigger = trigger;
            entry.attack = (p0 & 0xFFF) as f32 * MS_PER_FRAME;
            entry.decay = ((p1 >> 16) & 0xFFF) as f32 * MS_PER_FRAME;
            entry.sustain = (p1 & 0xFFF) as f32 * MS_PER_FRAME;
            entry.release = (p2 & 0xFFF) as f32 * MS_PER_FRAME;
        }
        self.sub_inputs = Vec::with_capacity(self.sub_input_ids.len());
        for index in 0..self.sub_input_ids.len() {
            let source = self.object_ptr(self.sub_input_ids[index], true, false, host);
            self.sub_inputs.push(source);
        }
        self.master_inputs = Vec::with_capacity(self.master_input_ids.len());
        for index in 0..self.master_input_ids.len() {
            let source = self.object_ptr(self.master_input_ids[index], true, true, host);
            self.master_inputs.push(source);
        }
    }

    fn input_word(&self, index: usize) -> i32 {
        self.input_words.get(index).copied().unwrap_or(0)
    }

    fn read(&self, source: Source) -> i32 {
        match source {
            Source::Dummy | Source::ThreeDProc(_) => 0,
            Source::Input(index) => self.input_word(index),
            Source::CurveQ15(index) => self.curves[index].q15,
            Source::MixCtlDb(index) => self.mix_unique[index].cmp_db,
            Source::MasterOutput(index) => self.master_unique[index].output,
            Source::SubOutput(index) => self.sub_unique[index].output,
            Source::ThreeDDb(index) => self.three_d_unique[index].db_rolloff,
            Source::ThreeDQ15(index) => self.three_d_unique[index].q15_rolloff,
            Source::EvtDb(index) => self.evt_unique[index].output,
            Source::EvtQ15(index) => self.evt_unique[index].qoutput,
        }
    }

    fn scale_product(&self, start: Option<usize>, count: usize) -> i32 {
        let mut scale = 0x7FFFi32;
        if let Some(start) = start {
            for source in self.scales.iter().skip(start).take(count) {
                scale = self.read(*source).wrapping_mul(scale) >> 15;
            }
        }
        scale
    }

    /// `NFSMixMap::ProcessMixMap` 0x829d0df8.
    pub fn process(&mut self, dt: f32, cam_state: i32) {
        self.delta_time_ratio[1] = self.delta_time_ratio[0];
        self.delta_time_ratio[0] = dt / F_DT_FRAME_LOCK;
        self.delta_time = dt;
        self.ms_delta_time = dt * 1000.0;
        self.prev_cam = self.cur_cam;
        self.cur_cam = cam_state;
        for index in 0..self.curves.len() {
            let curve = self.curves[index];
            let q15 = curve_output((curve.input_id >> 24) & 0xF, self.read(curve.input));
            self.curves[index].q15 = q15;
            self.curves[index].db = q15_to_db(q15);
        }
        for index in 0..self.mix_procs.len() {
            let Proc { shared, unique } = self.mix_procs[index];
            let control = self.mix_shared[shared];
            let curve_q15 = self.mix_unique[unique].curve.map_or(0, |curve| self.curves[curve].q15);
            let q15 = 0x7FFF - (0x7FFFi32.wrapping_sub(curve_q15).wrapping_mul(control.ratio) >> 15);
            let value = control.offset.wrapping_add(q15_to_db(q15));
            let count = ((self.word(control.params + 4) >> 24) & 0xFF) as usize;
            let scale = self.scale_product(self.mix_unique[unique].scales, count);
            self.mix_unique[unique].cmp_db = scale.wrapping_mul(value) >> 15;
        }
        self.update_3d_mix_ctls();
        self.update_evt_mix_ctls();
        self.update_sub_channels();
        self.update_master_channels();
        self.mix_master_channels();
    }

    /// `Update3DMixCtls` 0x829cd9f0.
    fn update_3d_mix_ctls(&mut self) {
        if self.cur_cam != self.prev_cam {
            for shared in 0..self.three_d_shared.len() {
                let params = self.three_d_shared[shared].params;
                let count = ((self.word(params) >> 24) & 0xF) as usize;
                let mut cam = self.cur_cam;
                loop {
                    let found = (0..count).map(|k| params + 4 + 24 * k).find(|&at| (self.word(at) >> 24) & 0xF == cam);
                    if let Some(at) = found {
                        let entry = &mut self.three_d_shared[shared];
                        entry.state_params = at;
                        entry.prev_cam = self.prev_cam;
                        entry.cur_cam = self.cur_cam;
                        entry.ms_since_cam = 0;
                        break;
                    }
                    if cam == 0 {
                        break;
                    }
                    cam = 0;
                }
            }
        }
        for index in 0..self.three_d_procs.len() {
            let Proc { shared, unique } = self.three_d_procs[index];
            let base = self.three_d_unique[unique].inputs.unwrap_or(usize::MAX - BLOCK_WORDS);
            let input = |map: &Self, k: usize| map.input_word(base + k);
            if input(self, 15) & 1 == 0 {
                let entry = &mut self.three_d_unique[unique];
                entry.db_rolloff = -10000;
                entry.q15_rolloff = 0;
                entry.azimuth = 0;
                entry.doppler_cents = 0;
                continue;
            }
            let state_params = self.three_d_shared[shared].state_params;
            let info = self.word(state_params);
            let curves = self.word(state_params + 4);
            let distance_source = (info >> 12) & 0xF;
            let distance = match distance_source {
                0 => f32::from_bits(input(self, 1) as u32),
                1 => f32::from_bits(input(self, 0) as u32),
                _ => -1.0,
            };
            let azimuth = match (info >> 8) & 0xF {
                0 => input(self, 3),
                1 => input(self, 2),
                _ => 0,
            };
            self.three_d_unique[unique].azimuth = azimuth;
            let (table, mix, near, far) = match (azimuth >> 14) & 3 {
                0 => (curves >> 28, azimuth, 0, 1),
                1 => (curves >> 16, azimuth.wrapping_sub(0x4000), 1, 2),
                2 => (curves >> 24, azimuth.wrapping_sub(0x8000), 2, 3),
                _ => (curves >> 20, azimuth.wrapping_sub(0xC000), 3, 0),
            };
            let min_max = self.three_d_unique[unique].min_max;
            let [near_min, near_max] = min_max[near];
            let [far_min, far_max] = min_max[far];
            if distance > near_max && distance > far_max {
                let entry = &mut self.three_d_unique[unique];
                entry.db_rolloff = -10000;
                entry.q15_rolloff = 0;
                entry.doppler_cents = 0;
                continue;
            }
            let mut near_distance = distance;
            let mut far_distance = distance;
            if near_distance < near_min {
                near_distance = near_min;
            }
            if far_distance < far_min {
                far_distance = far_min;
            }
            if near_distance > near_max {
                near_distance = near_max;
            }
            if far_distance > far_max {
                far_distance = far_max;
            }
            let near_ratio = fctiwz((near_distance - near_min) / (near_max - near_min) * 32767.0);
            let far_ratio = fctiwz((far_distance - far_min) / (far_max - far_min) * 32767.0);
            let q15 = azim_shape_output(table & 0xF, [near_ratio, far_ratio], mix);
            self.three_d_unique[unique].q15_rolloff = q15;
            self.three_d_unique[unique].db_rolloff = q15_to_db(q15);
            let speed = curves & 0xFFFF;
            if speed == 0 {
                continue;
            }
            let speed = speed as f32;
            let flags = input(self, 15);
            let (velocity, flag, now) = if distance_source == 1 {
                (input(self, 13), 0x8000_0000u32, f32::from_bits(input(self, 0) as u32))
            } else {
                (input(self, 14), 0x4000_0000u32, f32::from_bits(input(self, 1) as u32))
            };
            let cents = if flags as u32 & flag == 0 {
                let mut denominator = f32::from_bits(velocity as u32) + speed;
                if !(denominator > 0.0) {
                    denominator = speed;
                }
                cents_from_pitch_mult(speed / denominator) as f32
            } else {
                if let Some(word) = self.input_words.get_mut(base + 15) {
                    *word = (flags as u32 & !flag) as i32;
                }
                0.0
            };
            let entry = &mut self.three_d_unique[unique];
            if entry.prev_delta_dist == 0.0 {
                entry.prev_delta_dist = 1.0;
            }
            entry.prev_delta_dist = if now > entry.prev_dist { now - entry.prev_dist } else { entry.prev_dist - now };
            entry.prev_dist = now;
            let previous = entry.doppler_cents;
            entry.doppler_cents = fctiwz((cents - previous as f32) * DOPPLER_SMOOTHING_FACTOR).wrapping_add(previous);
        }
    }

    /// `UpdateEvtMixCtls` 0x829ce770.
    fn update_evt_mix_ctls(&mut self) {
        for index in 0..self.evt_procs.len() {
            let Proc { shared, unique } = self.evt_procs[index];
            let params = self.evt_shared[shared].params;
            let id = self.word(params);
            let trigger = self.read(self.evt_unique[unique].trigger);
            if self.evt_unique[unique].stage == STAGE_OFF && trigger == 0 {
                let entry = &mut self.evt_unique[unique];
                entry.elapsed = 0.0;
                entry.q_start = 0;
                entry.ms_start = 0.0;
                entry.stage = STAGE_OFF;
                entry.qoutput = 0;
                entry.output = if (id >> 9) & 1 != 0 { -10000 } else { 0 };
                continue;
            }
            let ms = self.ms_delta_time;
            let (p0, p1, p2) = (self.word(params + 0xC), self.word(params + 0x10), self.word(params + 0x14));
            let entry = &mut self.evt_unique[unique];
            entry.elapsed += ms;
            match (id >> 24) & 0xF {
                0 => entry.update_ar(id, p0, p2, trigger),
                1 => entry.update_asr(id, p0, p2, trigger),
                3 => entry.update_adsr(id, p0, p1, p2, trigger),
                _ => {}
            }
            let swing = self.word(params + 4) as i16 as i32;
            let EvtShared { offset, ratio, .. } = self.evt_shared[shared];
            let db_output = (id >> 9) & 1 != 0;
            let qoutput = self.evt_unique[unique].qoutput;
            self.evt_unique[unique].output = if db_output {
                q15_to_db(qoutput)
            } else if swing > 0 {
                offset.wrapping_add(q15_to_db((qoutput.wrapping_mul(ratio) >> 15) - ratio + 0x7FFF))
            } else {
                q15_to_db(0x7FFF - (qoutput.wrapping_mul(ratio) >> 15))
            };
            let Some(scales) = self.evt_unique[unique].scales else { continue };
            let count = ((self.word(params + 4) >> 24) & 0xFF) as usize;
            let scale = self.scale_product(Some(scales), count);
            let output = self.evt_unique[unique].output;
            self.evt_unique[unique].output =
                if db_output { q15_to_db(db_to_q15(output).wrapping_mul(scale) >> 15) } else { output.wrapping_mul(scale) >> 15 };
        }
    }

    /// `UpdateSubChannels` 0x829cc8c8.
    fn update_sub_channels(&mut self) {
        for index in 0..self.sub_procs.len() {
            let Proc { shared, unique } = self.sub_procs[index];
            let Some(start) = self.sub_unique[unique].inputs else { continue };
            self.sub_unique[unique].output = 0;
            let count = (self.sub_shared[shared].num_inputs & 0xFF) as usize;
            for k in 0..count {
                let value = self.sub_inputs.get(start + k).map_or(0, |&source| self.read(source));
                self.sub_unique[unique].output = self.sub_unique[unique].output.wrapping_add(value);
            }
            let swing = self.word(self.sub_shared[shared].params + 4);
            let lower = (swing as u32 | 0xFFFF_0000) as i32;
            let upper = (swing >> 16) & 0x7FFF;
            let entry = &mut self.sub_unique[unique];
            if entry.output > upper {
                entry.output = upper;
            }
            if entry.output < lower {
                entry.output = lower;
            }
        }
    }

    /// `UpdateMasterChannels` 0x829cc990.
    fn update_master_channels(&mut self) {
        for index in 0..self.master_procs.len() {
            let Proc { shared, unique } = self.master_procs[index];
            let outputs = self.master_unique[unique].outputs;
            if self.output_words[outputs + 15] & 1 == 0 {
                self.master_unique[unique].output = -10000;
                continue;
            }
            self.master_unique[unique].output = (self.word(self.master_shared[shared].params + 4) >> 16) as i16 as i32;
            let Some(start) = self.master_unique[unique].inputs else { continue };
            let count = (self.master_shared[shared].num_inputs & 0xFF) as usize;
            for k in 0..count {
                let value = self.master_inputs.get(start + k).map_or(0, |&source| self.read(source));
                self.master_unique[unique].output = self.master_unique[unique].output.wrapping_add(value);
            }
        }
    }

    fn pack_output(&mut self, outputs: usize, slot: i32, value: i32) {
        let word = outputs + (slot >> 1) as usize;
        let shift = (slot & 1) * 16;
        let keep = 0xFFFFu32 << (((slot - 1) & 1) * 16);
        if let Some(target) = self.output_words.get_mut(word) {
            *target = (((value as u32 & 0xFFFF) << shift) | (*target as u32 & keep)) as i32;
        }
    }

    /// `MixMasterChannels` 0x829cca58: each preset entry packs one 16-bit output.
    fn mix_master_channels(&mut self) {
        for index in 0..self.master_procs.len() {
            let Proc { shared, unique } = self.master_procs[index];
            let MasterShared { presets, num_inputs, .. } = self.master_shared[shared];
            let MasterUnique { outputs, output, three_d, .. } = self.master_unique[unique];
            let head = self.word(presets);
            let kind = (head >> 24) & 0xF;
            if self.output_words[outputs + 15] & 1 == 0 {
                let value = match kind {
                    1 => 0,
                    2 => 25000,
                    _ => -10000,
                };
                let slot = (self.word(presets + 4) >> 26) & 0x1F;
                self.pack_output(outputs, slot, value);
                continue;
            }
            for k in 0..(head & 0x1F) as usize {
                let entry = self.word(presets + 4 + 4 * k);
                let slot = (entry >> 26) & 0x1F;
                let index_3d = (entry >> 21) & 0x1F;
                let offset = entry as i16 as i32;
                let count_3d = (num_inputs >> 16) & 0x1F;
                let value = if count_3d > 0 && index_3d < count_3d {
                    let proc = three_d.and_then(|start| self.master_inputs.get(start + index_3d as usize)).and_then(|source| match source {
                        Source::ThreeDProc(proc) => Some(*proc),
                        _ => None,
                    });
                    let control = proc.map(|proc| self.three_d_unique[self.three_d_procs[proc].unique]);
                    match control {
                        Some(control) if entry < 0 => control.azimuth & 0xFFFF,
                        Some(control) => match kind {
                            0 | 4 => db_to_q15(clamp_db(control.db_rolloff.wrapping_add(output).wrapping_add(offset))),
                            1 => {
                                let cents = control.doppler_cents.wrapping_add(output).wrapping_add(offset);
                                if cents > 2400 {
                                    2400
                                } else if cents < -4800 {
                                    0
                                } else {
                                    cents
                                }
                            }
                            2 => clamp_db(output.wrapping_add(offset)),
                            _ => output,
                        },
                        None => 0,
                    }
                } else {
                    let value = output.wrapping_add(offset);
                    match kind {
                        0 | 4 => db_to_q15(clamp_db(value)),
                        1 => value.clamp(-4800, 2400),
                        2 => fctiwz(pitch_mult_from_cents(clamp_db(value)) * 25000.0),
                        _ => value.clamp(0, 25000),
                    }
                };
                self.pack_output(outputs, slot, value);
            }
        }
    }

    /// The DMixIO linked to game object `id` (`SndBase::objectID` with the object or control kind
    /// in the top bits), as `ConnectDMixIO` hands it out.
    pub fn dmixio(&self, id: u32) -> Option<DMixIoId> {
        self.find_dmixio(id & 0xE0FF_FFF0).map(DMixIoId)
    }

    /// `DMixIO::SetDMixInput(int, int)` 0x829ca318.
    pub fn set_input(&mut self, io: DMixIoId, index: usize, value: i32) {
        if let Some(block) = self.dmixio[io.0].input {
            if let Some(word) = self.input_words.get_mut(block * BLOCK_WORDS + index) {
                *word = value;
            }
        }
    }

    /// `DMixIO::GetDMixInput` 0x829ca368.
    pub fn input(&self, io: DMixIoId, index: usize) -> i32 {
        self.dmixio[io.0].input.map_or(0, |block| self.input_word(block * BLOCK_WORDS + index))
    }

    /// `memset(m_pDMixInputBlock, 0, 64)` (`CSTATE_Base::Detach`).
    pub fn clear_input(&mut self, io: DMixIoId) {
        if let Some(block) = self.dmixio[io.0].input {
            self.input_words[block * BLOCK_WORDS..(block + 1) * BLOCK_WORDS].fill(0);
        }
    }

    /// `DMixIO::GetDMixOutput` 0x829ca388.
    pub fn output(&self, io: DMixIoId, index: i32, kind: PresetType) -> i32 {
        let Some(outputs) = self.dmixio[io.0].output else { return 0 };
        let word = self.output_words.get(outputs + (index >> 1) as usize).copied().unwrap_or(0);
        let value = word >> ((16 * index) & 0x10);
        match kind {
            PresetType::Volume | PresetType::Frequency | PresetType::Depth => value & 0x7FFF,
            PresetType::Pitch => int_pitch_mult_from_cents(value as u16),
            PresetType::Azimuth => i32::from(value as u16),
        }
    }

    /// `TurnOnMixOutput` 0x829ca420 / `TurnOffMixOutput` 0x829ca408.
    pub fn set_output_enabled(&mut self, io: DMixIoId, enabled: bool) {
        if let Some(outputs) = self.dmixio[io.0].output {
            self.output_words[outputs + 15] = i32::from(enabled);
        }
    }

    pub fn has_output(&self, io: DMixIoId) -> bool {
        self.dmixio[io.0].output.is_some()
    }
}

/// PC port: a state's master mix channel and preset blocks, measured.
struct MasterLayout {
    header: usize,
    /// The master block: its four-word head, then the channel records up to `masters_end`.
    masters: usize,
    masters_end: usize,
    presets: usize,
    presets_end: usize,
    /// Each channel's `objectID`, in order.
    objects: Vec<u32>,
}

impl MasterLayout {
    fn read(data: &[u8], state: usize) -> Option<Self> {
        let word = |at: usize| -> Option<u32> {
            let bytes = data.get(at..at.checked_add(4)?)?;
            Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        };
        let offset = |at: usize| -> Option<usize> {
            let bytes = data.get(at..at.checked_add(4)?)?;
            usize::try_from(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])).ok()
        };
        if state >= offset(4)? {
            return None;
        }
        let header = offset(offset(8)?.checked_add(4 * state)?)?;
        let masters = header.checked_add(offset(header.checked_add(0x10)?)?)?;
        let presets = header.checked_add(offset(header.checked_add(0x14)?)?)?;
        let count = offset(masters)?;
        let mut masters_end = masters.checked_add(16)?;
        let mut presets_end = presets;
        let mut objects = Vec::new();
        for _ in 0..count {
            let inputs = ((word(masters_end)? >> 16) & 0xFF) as usize;
            objects.push(word(masters_end.checked_add(8)?)?);
            masters_end = masters_end.checked_add(4 * (inputs + 3))?;
            presets_end = presets_end.checked_add(4 * ((word(presets_end)? & 0xFF) as usize + 1))?;
        }
        (masters_end <= data.len() && presets_end <= data.len()).then_some(Self { header, masters, masters_end, presets, presets_end, objects })
    }
}

/// PC port: the `objectID` each master mix channel of state `state` names, in order; `None` when
/// the map has no such state or its master channels do not read whole.
#[must_use]
pub fn master_objects(data: &[u8], state: usize) -> Option<Vec<u32>> {
    MasterLayout::read(data, state).map(|layout| layout.objects)
}

/// PC port: `data` with `records` appended to state `state`'s master mix channels and `presets`
/// to its preset groups: each record as its words (id, swing, `objectID`, inputs), each group as
/// its words (head, entries). The state's next block follows its presets, so both blocks are
/// rebuilt at the end of the map and the state header names them anew; the channel count, and
/// the count of output arrays (one for each run of channels of one SFX object), grow with them.
/// `None` when the state does not read whole or a record or group is not one.
#[must_use]
pub fn with_master_channels(data: &[u8], state: usize, records: &[&[u32]], presets: &[&[u32]]) -> Option<Vec<u8>> {
    let layout = MasterLayout::read(data, state)?;
    if records.len() != presets.len() {
        return None;
    }
    for record in records {
        if record.len() != ((record.first()? >> 16) & 0xFF) as usize + 3 {
            return None;
        }
    }
    for group in presets {
        if group.len() != (group.first()? & 0xFF) as usize + 1 {
            return None;
        }
    }
    let head = data.get(layout.masters..layout.masters + 16)?;
    let head_word = |k: usize| u32::from_be_bytes([head[4 * k], head[4 * k + 1], head[4 * k + 2], head[4 * k + 3]]);
    let mut arrays = head_word(1);
    let mut previous = layout.objects.last().copied();
    for record in records {
        if previous != Some(record[2]) {
            arrays += 1;
        }
        previous = Some(record[2]);
    }
    let count = u32::try_from(layout.objects.len() + records.len()).ok()?;
    let mut out = data.to_vec();
    out.resize(out.len().next_multiple_of(4), 0);
    let new_masters = out.len();
    for value in [count, arrays, head_word(2), head_word(3)] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out.extend_from_slice(data.get(layout.masters + 16..layout.masters_end)?);
    for value in records.iter().flat_map(|record| record.iter()) {
        out.extend_from_slice(&value.to_be_bytes());
    }
    let new_presets = out.len();
    out.extend_from_slice(data.get(layout.presets..layout.presets_end)?);
    for value in presets.iter().flat_map(|group| group.iter()) {
        out.extend_from_slice(&value.to_be_bytes());
    }
    let masters_offset = u32::try_from(new_masters - layout.header).ok()?;
    let presets_offset = u32::try_from(new_presets - layout.header).ok()?;
    out.get_mut(layout.header + 0x10..layout.header + 0x14)?.copy_from_slice(&masters_offset.to_be_bytes());
    out.get_mut(layout.header + 0x14..layout.header + 0x18)?.copy_from_slice(&presets_offset.to_be_bytes());
    Some(out)
}

impl EvtUnique {
    fn switch_off(&mut self) {
        self.elapsed = 0.0;
        self.ms_start = 0.0;
        self.q_start = 0;
        self.stage = STAGE_OFF;
        self.qoutput = 0;
        self.output = 0;
    }

    fn progress(&self, length: f32) -> f32 {
        let span = length - self.ms_start;
        let mut t = self.elapsed - self.ms_start;
        if span > 0.0 {
            t /= span;
        }
        t
    }

    fn rise(&mut self, shape: i32) {
        let t = self.progress(self.attack);
        let start = self.q_start;
        self.qoutput = fctiwz(float_curve_output(shape, t) * (0x7FFF - start) as f32).wrapping_add(start);
    }

    /// Release (or decay toward zero) from a mirrored curve; AR uses the curve value itself, the
    /// other envelopes one minus it.
    fn fall(&mut self, shape: i32, inverted: bool) {
        let t = self.progress(self.release);
        let start = self.q_start;
        let curve = float_curve_output(shape, 1.0 - t);
        let amount = if inverted { 1.0 - curve } else { curve };
        self.qoutput = start.wrapping_sub(fctiwz(amount * start as f32));
    }

    /// Moves to `stage`, keeping the elapsed share of `from` as the share of `to` left to run.
    fn rescale(&mut self, stage: u32, from: f32, to: f32) {
        self.stage = stage;
        let elapsed = (from - self.elapsed) / from * to;
        self.q_start = self.qoutput;
        self.ms_start = elapsed;
        self.elapsed = elapsed;
    }

    fn jump(&mut self, stage: u32, from: f32, to: f32) {
        self.stage = stage;
        if from < INSTANT_MS {
            self.q_start = self.qoutput;
            self.ms_start = to;
            self.elapsed = to;
        } else {
            self.rescale(stage, from, to);
        }
    }

    fn enter(&mut self, stage: u32, q_start: i32) {
        self.elapsed = 0.0;
        self.stage = stage;
        self.ms_start = 0.0;
        self.q_start = q_start;
    }

    /// `UpdateAREvent` 0x829cd7c8.
    fn update_ar(&mut self, id: i32, p0: i32, p2: i32, trigger: i32) {
        let retrigger = (id >> 10) & 1 != 0;
        loop {
            if self.stage <= STAGE_ATTACK {
                self.stage = STAGE_ATTACK;
                if self.elapsed < self.attack {
                    if self.attack == 0.0 {
                        self.qoutput = 0x7FFF;
                    } else {
                        self.rise((p0 >> 12) & 0xF);
                    }
                    return;
                }
                self.enter(STAGE_RELEASE, 0x7FFF);
                continue;
            }
            if !(self.elapsed < self.release) {
                self.switch_off();
                return;
            }
            if trigger != 0 && retrigger {
                self.jump(STAGE_ATTACK, self.release, self.attack);
                continue;
            }
            self.fall((p2 >> 12) & 0xF, false);
            return;
        }
    }

    /// `UpdateASREvent` 0x829cd4a8.
    fn update_asr(&mut self, id: i32, p0: i32, p2: i32, trigger: i32) {
        let triggered = trigger != 0;
        let retrigger = (id >> 10) & 1 != 0;
        let hold = (id >> 8) & 1 != 0;
        loop {
            match self.stage {
                STAGE_OFF | STAGE_ATTACK => {
                    self.stage = STAGE_ATTACK;
                    if hold && !triggered {
                        self.jump(STAGE_RELEASE, self.attack, self.release);
                        continue;
                    }
                    if self.elapsed < self.attack {
                        if self.attack == 0.0 {
                            self.qoutput = 0x7FFF;
                        } else {
                            self.rise((p0 >> 12) & 0xF);
                        }
                        return;
                    }
                    self.enter(STAGE_SUSTAIN, 0x7FFF);
                }
                STAGE_SUSTAIN => {
                    if !(!triggered && hold) && (hold || !(self.elapsed > self.sustain)) {
                        self.qoutput = 0x7FFF;
                        if hold {
                            self.elapsed = 0.0;
                        }
                        return;
                    }
                    self.enter(STAGE_RELEASE, 0x7FFF);
                }
                _ => {
                    if !(self.elapsed < self.release) {
                        self.switch_off();
                        return;
                    }
                    if triggered && retrigger {
                        self.jump(STAGE_ATTACK, self.release, self.attack);
                        continue;
                    }
                    self.fall((p2 >> 12) & 0xF, true);
                    return;
                }
            }
        }
    }

    /// `UpdateADSREvent` 0x829cd058.
    fn update_adsr(&mut self, id: i32, p0: i32, p1: i32, p2: i32, trigger: i32) {
        let triggered = trigger != 0;
        let retrigger = (id >> 10) & 1 != 0;
        let hold = (id >> 8) & 1 != 0;
        let level = p2 >> 16;
        loop {
            match self.stage {
                STAGE_OFF | STAGE_ATTACK => {
                    self.stage = STAGE_ATTACK;
                    if hold && !triggered {
                        self.jump(STAGE_RELEASE, self.attack, self.release);
                        continue;
                    }
                    if self.elapsed < self.attack {
                        if self.attack > INSTANT_MS {
                            self.rise((p0 >> 12) & 0xF);
                        } else {
                            self.qoutput = 0x7FFF;
                        }
                        return;
                    }
                    self.enter(STAGE_DECAY, 0x7FFF);
                }
                STAGE_DECAY => {
                    if !triggered && hold {
                        self.rescale(STAGE_RELEASE, self.decay, self.release);
                        continue;
                    }
                    if !(self.elapsed > self.decay) {
                        let t = self.progress(self.decay);
                        let start = self.q_start;
                        let curve = float_curve_output((p1 >> 12) & 0xF, 1.0 - t);
                        self.qoutput = fctiwz((1.0 - curve) * level.wrapping_sub(start) as f32).wrapping_add(start);
                        return;
                    }
                    self.enter(STAGE_SUSTAIN, level);
                }
                STAGE_SUSTAIN => {
                    if !(!triggered && hold) && (hold || !(self.elapsed > self.sustain)) {
                        self.qoutput = level;
                        if hold {
                            self.elapsed = 0.0;
                        }
                        return;
                    }
                    self.enter(STAGE_RELEASE, level);
                }
                _ => {
                    if !(self.elapsed < self.release) {
                        self.switch_off();
                        return;
                    }
                    if triggered && retrigger {
                        self.jump(STAGE_ATTACK, self.release, self.attack);
                        continue;
                    }
                    self.fall((p2 >> 12) & 0xF, true);
                    return;
                }
            }
        }
    }
}
