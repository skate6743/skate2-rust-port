//! `rw::audio::core::EALayer3Core` (0x2e0 bytes): one MPEG layer III granule of one or two channels
//! behind EA's one-byte granule header, decoded by `CMpegBase` / `CMpegLayer3Base` code.

#![allow(clippy::float_cmp)]

use std::sync::Arc;

use super::generated_transforms::{dct64, imdct12, imdct36};
use super::tables::{
    Sbi, ANTIALIAS_CA, ANTIALIAS_CS, DECWIN, HUFF_COUNT_0, HUFF_COUNT_1, HUFF_TABLE_1, HUFF_TABLE_10,
    HUFF_TABLE_11, HUFF_TABLE_12, HUFF_TABLE_13, HUFF_TABLE_15, HUFF_TABLE_16, HUFF_TABLE_2, HUFF_TABLE_24,
    HUFF_TABLE_3, HUFF_TABLE_5, HUFF_TABLE_6, HUFF_TABLE_7, HUFF_TABLE_8, HUFF_TABLE_9, IO, LINBITS,
    NUM_SFB_BLOCK, POW43_0_TO_31, POW43_SEGMENTS, PRETAB, REORDER_MAP, SAMPLE_RATES, SF_BAND_INDEX, SLEN,
    TAN12, TWO_TO_NEGATIVE_QUARTER_POWER, WIN,
};
use crate::math::fmadd;
use crate::rwac::decoder::DataRef;

/// Samples of a granule (`mFrameSamples`).
pub(super) const GRANULE: usize = 576;
/// `Decode`'s stack: both channels' `float[32][18]` spectra and the 600-float work buffer after
/// them, one block, so a big-values run past 576 lands in the next buffer as it does natively. Every
/// core of a decoder decodes on the same stack, so the decoder keeps one.
pub(super) const SCRATCH: usize = GRANULE + GRANULE + 600;
/// Where `Decode` starts the work buffer (`v43`).
const WORK: usize = 2 * GRANULE;
/// `HELPER_cos4_0`: the M/S stereo scale.
const COS4: f32 = f32::from_bits(0x3f35_04f3);

/// `slw` / `srw`: the shift amount is taken modulo 64, and 32 or more gives 0.
fn slw(x: u32, n: i32) -> u32 {
    let n = (n as u32) & 63;
    if n >= 32 { 0 } else { x << n }
}

fn srw(x: u32, n: i32) -> u32 {
    let n = (n as u32) & 63;
    if n >= 32 { 0 } else { x >> n }
}

/// `CMpegBase`'s bit reader: `mBufPtr`, `mBufPtrBase`, `mShiftReg`, `mShiftRegBits`.
#[derive(Clone)]
struct BitReader {
    bytes: Arc<[u8]>,
    /// `mBufPtr` and `mBufPtrBase` as indices into `bytes`.
    ptr: isize,
    base: isize,
    reg: u32,
    count: i32,
}

impl Default for BitReader {
    fn default() -> Self {
        Self { bytes: Arc::from(Vec::new()), ptr: 0, base: 0, reg: 0, count: 0 }
    }
}

impl BitReader {
    /// A byte of the buffer the pointer moves through (0 outside it).
    fn byte(&self, at: isize) -> u32 {
        usize::try_from(at).ok().and_then(|at| self.bytes.get(at)).map_or(0, |&b| u32::from(b))
    }

    fn set(&mut self, data: &DataRef) {
        self.bytes = Arc::clone(&data.bytes);
        self.ptr = data.offset as isize;
        self.base = self.ptr;
        self.reg = 0;
        self.count = 0;
    }

    /// One byte into the register below its valid bits.
    fn load_byte(&mut self) {
        let byte = self.byte(self.ptr);
        self.ptr += 1;
        self.reg |= slw(byte, 24 - self.count);
        self.count += 8;
    }

    /// `LoadBitRegister` 0x82c07d80: bytes in while 24 bits or fewer are held.
    fn load(&mut self) {
        while self.count <= 24 {
            self.load_byte();
        }
    }

    /// `GetBits` 0x82c07dd0.
    fn get(&mut self, n: i32) -> u32 {
        if self.count < n {
            loop {
                let before = self.count;
                self.load_byte();
                if before + 8 >= n {
                    break;
                }
            }
        }
        let value = self.reg;
        self.reg = slw(value, n);
        self.count -= n;
        srw(value, 32 - n)
    }

    fn consume(&mut self, n: i32) {
        self.reg = slw(self.reg, n);
        self.count -= n;
    }

    /// The bit position: `8 * (mBufPtr - mBufPtrBase) - mShiftRegBits`.
    fn position(&self) -> i32 {
        (8 * (self.ptr - self.base)) as i32 - self.count
    }

    /// `DecodeHuffman`'s refill: bytes in while 24 bits or fewer are held and the held bits,
    /// counted from `start`, stay short of `end`.
    fn load_until(&mut self, start: i32, end: i32) {
        while self.count <= 24 && self.count + start < end {
            self.load_byte();
        }
    }
}

/// `GranuleInfo` (0x18 bytes).
#[derive(Clone, Copy, Default)]
struct GranuleInfo {
    part2_3_length: u16,
    big_values: u16,
    scale_fac_compress: u16,
    global_gain: u8,
    window_switching: u8,
    block_type: u8,
    mixed_block: u8,
    region0_count: u8,
    region1_count: u8,
    table_select: [u8; 3],
    count1_table_select: u8,
    sub_block_gain: [u8; 3],
    pre_flag: u8,
    scale_fac_scale: u32,
}

impl GranuleInfo {
    fn short_block(&self) -> bool {
        self.window_switching != 0 && self.block_type == 2
    }
}

/// `Layer3ScaleFactors` (0x7c bytes) as its 62 words: `longBlock[23]` then `shortBlock[3][13]`.
#[derive(Clone, Copy)]
struct ScaleFactors([i16; 62]);

impl Default for ScaleFactors {
    fn default() -> Self {
        Self([0; 62])
    }
}

const SHORT: usize = 23;

fn short(window: usize, sfb: usize) -> usize {
    SHORT + 13 * window + sfb
}

/// `mHuffTables` as `HuffmanTableSetup` 0x82c099d8 fills them: tables 0, 4 and 14 are empty, 16 to 23
/// share table 16 and 24 to 31 table 24.
fn huff_table(select: u8) -> Option<&'static [i16]> {
    Some(match select {
        1 => &HUFF_TABLE_1,
        2 => &HUFF_TABLE_2,
        3 => &HUFF_TABLE_3,
        5 => &HUFF_TABLE_5,
        6 => &HUFF_TABLE_6,
        7 => &HUFF_TABLE_7,
        8 => &HUFF_TABLE_8,
        9 => &HUFF_TABLE_9,
        10 => &HUFF_TABLE_10,
        11 => &HUFF_TABLE_11,
        12 => &HUFF_TABLE_12,
        13 => &HUFF_TABLE_13,
        15 => &HUFF_TABLE_15,
        16..=23 => &HUFF_TABLE_16,
        24..=31 => &HUFF_TABLE_24,
        _ => return None,
    })
}

/// `SToPowerOf4over3` 0x82c09bf0: `|x|^(4/3)` for x of 32 and more, linear on the segment of x's bit
/// length (`fmuls` then `fadds`).
fn pow43(x: i16) -> f32 {
    let mut v = i32::from(x);
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    let segment = (14 - (v.count_ones() as i32 & 0x3F)).min(8) as usize;
    let [base, slope] = POW43_SEGMENTS[segment];
    base + slope * f32::from(x)
}

/// `sfBandIndex[sfreq].l[index]` as the native read sees it: past `l[22]` it reads the `s` bytes
/// that follow in `SBI`.
fn long_bound(sbi: &Sbi, index: usize) -> i32 {
    if index < 23 {
        i32::from(sbi.l[index])
    } else {
        let at = 2 * (index - 23);
        i32::from(i16::from_be_bytes([sbi.s[at], sbi.s[at + 1]]))
    }
}

/// `TAN12_0[is_pos]`: `io_0` follows the 16 ratios, so a 5-bit LSF scale factor that an MPEG-1
/// granule keeps (`scfsi`) reads its factors.
fn tan12(is_pos: u32) -> f32 {
    let index = is_pos as usize;
    match index {
        0..=15 => TAN12[index],
        16..=79 => IO[(index - 16) / 32][(index - 16) % 32],
        _ => 0.0,
    }
}

/// `fnmsubs`: `-(a * c - b)`, negated after the rounding, so an exact zero is -0.
fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -fmadd(a, c, -b)
}

/// `ScaleSamples` 0x82be2e28.
fn scale(samples: &mut [f32], gain: f32) {
    for sample in samples {
        *sample *= gain;
    }
}

/// `rw::audio::core::EALayer3Core`.
pub(super) struct Layer3Core {
    bits: BitReader,
    /// `mOpened`, `mHeader`.
    opened: bool,
    header: u32,
    /// `mChannels`: 1, or 2 unless the mode is mono.
    channels: u8,
    samp_freq: i32,
    /// `cFrameSize`: the last granule's bytes.
    pub(super) frame_size: i32,
    /// `mFrameSamples`.
    pub(super) frame_samples: u16,
    reset_synth_required: bool,
    /// `mBandOffset`: each channel's synthesis buffer position.
    band_offset: [u8; 2],
    mpeg25: bool,
    lsf: bool,
    sfreq: u8,
    /// `mVersion`: 1 for an MPEG-1 granule.
    version: u8,
    samp_freq_idx: u8,
    mode: u8,
    mode_ext: u8,
    /// `mGranuleInfo[channel][granule]`.
    granule_info: [[GranuleInfo; 2]; 2],
    scale_factors: [ScaleFactors; 2],
    /// `mpTemp->sideInfo.ch[channel].scfsi`.
    scfsi: [[u8; 4]; 2],
    /// `mpLoadedTableSelect`: a window-switched granule loads only the first two.
    loaded_table_select: [Option<&'static [i16]>; 3],
    /// `mpPrevBlockX4` ("MP3HybridHistory"): each channel's IMDCT overlap.
    prev_block: Vec<[[[f32; 4]; 18]; 8]>,
    /// `mpPolySynthHistoryF` ("PolySynthHistoryF"): each channel's two dct64 buffers.
    synth_history: Vec<[[f32; 288]; 2]>,
    /// PC port (custom songs): the granules are a standard MPEG stream's (`mpeg::Mp3Stream`), whose
    /// part 3 can end in stuffing bits of any length.
    pub(super) standard_stream: bool,
}

impl Layer3Core {
    /// `EaLayer3DecBase::CreateInstance` 0x82c167e8's per-core setup: zeroed, `HuffmanTableSetup`,
    /// and zeroed history for `channels` channels.
    pub(super) fn new(channels: usize) -> Self {
        Self {
            bits: BitReader::default(),
            opened: false,
            header: 0,
            channels: 0,
            samp_freq: 0,
            frame_size: 0,
            frame_samples: 0,
            reset_synth_required: false,
            band_offset: [0; 2],
            mpeg25: false,
            lsf: false,
            sfreq: 0,
            version: 0,
            samp_freq_idx: 0,
            mode: 0,
            mode_ext: 0,
            granule_info: [[GranuleInfo::default(); 2]; 2],
            scale_factors: [ScaleFactors::default(); 2],
            scfsi: [[0; 4]; 2],
            loaded_table_select: [None; 3],
            prev_block: vec![[[[0.0; 4]; 18]; 8]; channels],
            synth_history: vec![[[0.0; 288]; 2]; channels],
            standard_stream: false,
        }
    }

    /// `Open` 0x82c13a30: `CMpegBase::ProcessHeader` 0x82c0de98 gets the granule's first byte, which
    /// can never carry the 0xFFE sync, so it returns at once; the synthesis is reset.
    pub(super) fn open(&mut self, data: &DataRef) {
        if self.opened {
            self.header = 0;
            self.opened = false;
        }
        self.bits.set(data);
        self.opened = true;
        self.reset_synth_required = true;
        self.band_offset = [1, 1];
        self.frame_samples = GRANULE as u16;
    }

    /// `mChannels` of the last granule header.
    pub(super) fn channels(&self) -> u8 {
        self.channels
    }

    /// The next granule of the same feed: only the bit reader moves.
    pub(super) fn resume(&mut self, data: &DataRef) {
        self.bits.set(data);
    }

    /// `ProcessEALayer3Header` 0x82c09880: version (2 bits), sample rate (2), mode (2), mode
    /// extension (2).
    fn process_header(&mut self, header: u32) {
        let version_index = (header & 0xFF) >> 6;
        self.mode_ext = (header & 3) as u8;
        self.mpeg25 = version_index == 0;
        self.mode = ((header >> 2) & 3) as u8;
        self.lsf = version_index != 3;
        self.version = u8::from(header & 0x40 != 0);
        let rate = ((header >> 4) & 3) as u8;
        if self.mpeg25 {
            self.sfreq = rate + 6;
            self.samp_freq_idx = rate + 6;
        } else {
            self.samp_freq_idx = 3 * u8::from(self.lsf) + rate;
            self.sfreq = if self.lsf { 3 } else { 0 } + rate;
        }
        self.channels = u8::from(self.mode != 3) + 1;
        self.samp_freq = i32::from(SAMPLE_RATES.get(usize::from(self.samp_freq_idx)).copied().unwrap_or(0));
    }

    fn band_index(&self) -> &'static Sbi {
        &SF_BAND_INDEX[usize::from(self.sfreq).min(8)]
    }

    /// `GetSideInfo` 0x82c084f0. A window-switched granule of block type 0 stops reading (false).
    fn side_info(&mut self, gr: usize) -> bool {
        if self.lsf {
            for ch in 0..usize::from(self.channels) {
                let gi = &mut self.granule_info[ch][0];
                gi.part2_3_length = self.bits.get(12) as u16;
                gi.big_values = self.bits.get(9) as u16;
                gi.global_gain = self.bits.get(8) as u8;
                gi.scale_fac_compress = self.bits.get(9) as u16;
                gi.window_switching = self.bits.get(1) as u8;
                if gi.window_switching != 0 {
                    gi.block_type = self.bits.get(2) as u8;
                    gi.mixed_block = self.bits.get(1) as u8;
                    gi.table_select[0] = self.bits.get(5) as u8;
                    gi.table_select[1] = self.bits.get(5) as u8;
                    gi.sub_block_gain[0] = self.bits.get(3) as u8;
                    gi.sub_block_gain[1] = self.bits.get(3) as u8;
                    gi.sub_block_gain[2] = self.bits.get(3) as u8;
                    if gi.block_type == 0 {
                        return false;
                    }
                    if gi.block_type != 2 || gi.mixed_block != 0 {
                        gi.region0_count = 7;
                        gi.region1_count = 13;
                    } else {
                        gi.region0_count = 8;
                    }
                } else {
                    gi.table_select[0] = self.bits.get(5) as u8;
                    gi.table_select[1] = self.bits.get(5) as u8;
                    gi.table_select[2] = self.bits.get(5) as u8;
                    gi.region0_count = self.bits.get(4) as u8;
                    gi.region1_count = self.bits.get(3) as u8;
                    gi.block_type = 0;
                }
                gi.scale_fac_scale = self.bits.get(1);
                gi.count1_table_select = self.bits.get(1) as u8;
            }
            return true;
        }
        if gr == 1 {
            self.bits.load();
            for ch in 0..usize::from(self.channels) {
                let reg = self.bits.reg;
                self.scfsi[ch] = [(reg >> 31) as u8, ((reg >> 30) & 1) as u8, ((reg >> 29) & 1) as u8, ((reg >> 28) & 1) as u8];
                self.bits.consume(4);
            }
        }
        for ch in 0..usize::from(self.channels) {
            let part2_3_length = self.bits.get(12) as u16;
            self.bits.load();
            let reg = self.bits.reg;
            let gi = &mut self.granule_info[ch][gr];
            gi.part2_3_length = part2_3_length;
            gi.big_values = (reg >> 23) as u16;
            gi.global_gain = (reg >> 15) as u8;
            gi.scale_fac_compress = ((reg >> 11) & 0xF) as u16;
            gi.window_switching = u8::from(reg & 0x400 != 0);
            self.bits.consume(22);
            self.bits.load();
            let reg = self.bits.reg;
            let gi = &mut self.granule_info[ch][gr];
            if gi.window_switching != 0 {
                gi.block_type = (reg >> 30) as u8;
                gi.mixed_block = u8::from(reg & 0x2000_0000 != 0);
                gi.table_select[0] = ((reg >> 24) & 0x1F) as u8;
                gi.table_select[1] = ((reg >> 19) & 0x1F) as u8;
                gi.sub_block_gain[0] = ((reg >> 16) & 7) as u8;
                gi.sub_block_gain[1] = ((reg >> 13) & 7) as u8;
                gi.sub_block_gain[2] = ((reg >> 10) & 7) as u8;
                if gi.block_type == 0 {
                    return false;
                }
                if gi.block_type != 2 || gi.mixed_block != 0 {
                    gi.region0_count = 7;
                    gi.region1_count = 13;
                } else {
                    gi.region0_count = 8;
                    gi.region1_count = 12;
                }
            } else {
                gi.table_select[0] = (reg >> 27) as u8;
                gi.table_select[1] = ((reg >> 22) & 0x1F) as u8;
                gi.table_select[2] = ((reg >> 17) & 0x1F) as u8;
                gi.region0_count = ((reg >> 13) & 0xF) as u8;
                gi.block_type = 0;
                gi.region1_count = ((reg >> 10) & 7) as u8;
            }
            gi.pre_flag = u8::from(reg & 0x200 != 0);
            gi.scale_fac_scale = (reg >> 8) & 1;
            gi.count1_table_select = u8::from(reg & 0x80 != 0);
            self.bits.consume(25);
        }
        true
    }

    /// A scale factor of `bits` bits (0 reads nothing).
    fn scale_bits(&mut self, bits: u8) -> i16 {
        if bits == 0 { 0 } else { self.bits.get(i32::from(bits)) as i16 }
    }

    /// `GetScaleFactors` 0x82c089a8 (MPEG-1).
    fn scale_factors(&mut self, ch: usize, gr: usize) {
        let gi = self.granule_info[ch][gr];
        let slen0 = SLEN[0].get(usize::from(gi.scale_fac_compress)).copied().unwrap_or(0);
        let slen1 = SLEN[1].get(usize::from(gi.scale_fac_compress)).copied().unwrap_or(0);
        if gi.short_block() {
            if gi.mixed_block != 0 {
                for sfb in 0..8 {
                    self.scale_factors[ch].0[sfb] = self.scale_bits(slen0);
                }
                for sfb in 3..6 {
                    for window in 0..3 {
                        self.scale_factors[ch].0[short(window, sfb)] = self.scale_bits(slen0);
                    }
                }
            } else {
                for sfb in 0..6 {
                    for window in 0..3 {
                        self.scale_factors[ch].0[short(window, sfb)] = self.scale_bits(slen0);
                    }
                }
            }
            for sfb in 6..12 {
                for window in 0..3 {
                    self.scale_factors[ch].0[short(window, sfb)] = self.scale_bits(slen1);
                }
            }
            for window in 0..3 {
                self.scale_factors[ch].0[short(window, 12)] = 0;
            }
            return;
        }
        let scfsi = self.scfsi[ch];
        let parts = [(0..6, slen0), (6..11, slen0), (11..16, slen1), (16..21, slen1)];
        for (part, (bands, bits)) in parts.into_iter().enumerate() {
            if scfsi[part] == 0 || gr == 0 {
                for sfb in bands {
                    self.scale_factors[ch].0[sfb] = self.scale_bits(bits);
                }
            }
        }
        self.scale_factors[ch].0[21] = 0;
        self.scale_factors[ch].0[22] = 0;
    }

    /// `GetLsfScaleData` 0x82c093c0: the four partitions' bit lengths from `scalefac_compress`
    /// (intensity-coded right channel or not) and their scale factors in order.
    fn lsf_scale_data(&mut self, ch: usize, gr: usize) -> [u8; 45] {
        let mode_ext = self.mode_ext;
        let gi = self.granule_info[ch][gr];
        let sfc = u32::from(gi.scale_fac_compress);
        let block = if gi.block_type == 2 {
            if gi.mixed_block == 0 { 1 } else if gi.mixed_block == 1 { 2 } else { 0 }
        } else {
            0
        };
        let intensity_right = (mode_ext == 1 || mode_ext == 3) && ch == 1;
        let mut slen = [0u32; 4];
        let mut table = 0usize;
        if intensity_right {
            let v = sfc >> 1;
            if v < 180 {
                self.granule_info[ch][gr].pre_flag = 0;
                slen = [v / 36, (v % 36) / 6, (v % 36) % 6, 0];
                table = 3;
            } else if v < 244 {
                self.granule_info[ch][gr].pre_flag = 0;
                slen = [((v + 12) >> 4) & 3, ((v - 4) >> 2) & 3, v & 3, 0];
                table = 4;
            } else if v < 255 {
                self.granule_info[ch][gr].pre_flag = 0;
                slen = [(v - 244) / 3, (v - 244) % 3, 0, 0];
                table = 5;
            }
        } else if sfc < 400 {
            self.granule_info[ch][gr].pre_flag = 0;
            slen = [(sfc >> 4) / 5, (sfc >> 4) % 5, (sfc >> 2) & 3, sfc & 3];
        } else if sfc < 500 {
            self.granule_info[ch][gr].pre_flag = 0;
            let v = (sfc - 400) >> 2;
            slen = [v / 5, v % 5, sfc & 3, 0];
            table = 1;
        } else if sfc < 512 {
            self.granule_info[ch][gr].pre_flag = 1;
            let v = sfc - 500;
            slen = [v / 3, v % 3, 0, 0];
            table = 2;
        }
        let mut buffer = [0u8; 45];
        let counts = NUM_SFB_BLOCK[table][block];
        let mut at = 0;
        for part in 0..4 {
            for _ in 0..counts[part] {
                buffer[at] = self.scale_bits(slen[part] as u8) as u8;
                at += 1;
            }
        }
        buffer
    }

    /// `GetLsfScaleFactors` 0x82c096f0.
    fn lsf_scale_factors(&mut self, ch: usize, gr: usize) {
        let buffer = self.lsf_scale_data(ch, gr);
        let gi = self.granule_info[ch][gr];
        let sf = &mut self.scale_factors[ch].0;
        if gi.short_block() {
            if gi.mixed_block != 0 {
                for sfb in 0..8 {
                    sf[sfb] = i16::from(buffer[sfb]);
                }
                for sfb in 3..12 {
                    for window in 0..3 {
                        sf[short(window, sfb)] = i16::from(buffer[8 + 3 * (sfb - 3) + window]);
                    }
                }
            } else {
                for sfb in 0..12 {
                    for window in 0..3 {
                        sf[short(window, sfb)] = i16::from(buffer[3 * sfb + window]);
                    }
                }
            }
            for window in 0..3 {
                sf[short(window, 12)] = 0;
            }
        } else {
            for sfb in 0..21 {
                sf[sfb] = i16::from(buffer[sfb]);
            }
            sf[21] = 0;
            sf[22] = 0;
        }
    }

    /// `EALayer3Core::DecodeHuffman` 0x82c12f98: the big values and count1 quadruples into
    /// `scratch[at..]` as `±2^((globalGain - 210) / 4) * |x|^(4/3)`; values of 32 and more go through
    /// `SToPowerOf4over3` 32 at a time.
    #[allow(clippy::too_many_lines)]
    fn decode_huffman(&mut self, scratch: &mut [f32; SCRATCH], ch: usize, gr: usize, at: usize, part2_start: i32) {
        let gi = self.granule_info[ch][gr];
        let end = i32::from(gi.part2_3_length) + part2_start;
        let (region1, region2) = if gi.short_block() {
            (36, 576)
        } else {
            let sbi = self.band_index();
            let r0 = usize::from(gi.region0_count);
            (long_bound(sbi, r0 + 1), long_bound(sbi, usize::from(gi.region1_count) + 2 + r0))
        };
        let big = 2 * i32::from(gi.big_values);
        let limits = [region1.min(big), region2.min(big), big];
        let gain = TWO_TO_NEGATIVE_QUARTER_POWER[usize::from(255 - gi.global_gain)];
        let out = scratch;
        let standard_stream = self.standard_stream;
        let bits = &mut self.bits;
        let mut pending_index = [0i16; 32];
        let mut pending_value = [0i16; 32];
        let mut pending = 0usize;
        let flush = |out: &mut [f32; SCRATCH], index: &[i16; 32], value: &[i16; 32], count: usize| {
            for k in 0..count {
                let slot = at + index[k] as u16 as usize;
                if let Some(sample) = out.get_mut(slot) {
                    // PC port: `SToPowerOf4over3` is linear between powers of two, which EA's
                    // streams are made for; a standard stream's values take the power itself.
                    *sample *= if standard_stream { f32::from(value[k]).powf(4.0 / 3.0) } else { pow43(value[k]) };
                }
            }
        };
        let mut i = 0i32;
        for (region, &limit) in limits.iter().enumerate() {
            let Some(table) = self.loaded_table_select[region] else {
                for j in i.max(0)..limit {
                    if let Some(sample) = out.get_mut(at + j as usize) {
                        *sample = 0.0;
                    }
                }
                i = limit;
                continue;
            };
            let linbits = i32::from(LINBITS[usize::from(gi.table_select[region]).min(31)]);
            while i < limit {
                let start = bits.position();
                bits.load_until(start, end);
                let entry = table.get((bits.reg >> 24) as usize).copied().unwrap_or(0);
                let value = if entry < 0 {
                    bits.consume(8);
                    let mut node = (-i32::from(entry)) as usize;
                    let mut j = table.get(node).copied().unwrap_or(0);
                    while j < 0 {
                        let one = (bits.reg as i32) < 0;
                        node += 1;
                        if one {
                            node = node.wrapping_add((-i32::from(j)) as usize);
                        }
                        bits.consume(1);
                        j = table.get(node).copied().unwrap_or(0);
                    }
                    i32::from(j)
                } else {
                    bits.consume(i32::from(entry >> 8));
                    i32::from(entry & 0xFF)
                };
                let mut x = value >> 4;
                let mut y = value & 0xF;
                for (offset, magnitude) in [(0usize, &mut x), (1, &mut y)] {
                    if offset == 1 {
                        bits.load_until(start, end);
                    }
                    if *magnitude == 15 && linbits != 0 {
                        let reg = bits.reg;
                        bits.reg = slw(reg, linbits);
                        bits.count -= linbits;
                        *magnitude = srw(reg, 32 - linbits) as i32 + 15;
                    }
                    let slot = at + i as usize + offset;
                    if *magnitude == 0 {
                        if let Some(sample) = out.get_mut(slot) {
                            *sample = 0.0;
                        }
                        continue;
                    }
                    let signed = if bits.reg & 0x8000_0000 == 0 { gain } else { -gain };
                    if let Some(sample) = out.get_mut(slot) {
                        *sample = signed;
                    }
                    if *magnitude >= 32 {
                        pending_index[pending] = (i + offset as i32) as i16;
                        pending_value[pending] = *magnitude as i16;
                        pending += 1;
                        if pending >= 32 {
                            flush(out, &pending_index, &pending_value, pending);
                            pending = 0;
                        }
                    } else if let Some(sample) = out.get_mut(slot) {
                        *sample *= POW43_0_TO_31[*magnitude as usize];
                    }
                    bits.consume(1);
                }
                i += 2;
            }
        }
        flush(out, &pending_index, &pending_value, pending);
        let (entries, shifter): (&[[u8; 2]], i32) =
            if gi.count1_table_select == 0 { (&HUFF_COUNT_0, 26) } else { (&HUFF_COUNT_1, 28) };
        let mut position = bits.position();
        while position < end && i < 576 {
            bits.load_until(position, end);
            let [flags, length] = entries.get(srw(bits.reg, shifter) as usize).copied().unwrap_or([0, 0]);
            bits.consume(i32::from(length));
            for (k, flag) in [8u8, 4, 2, 1].into_iter().enumerate() {
                let slot = at + i as usize + k;
                let value = if flags & flag != 0 {
                    let value = if bits.reg & 0x8000_0000 == 0 { gain } else { -gain };
                    bits.consume(1);
                    value
                } else {
                    0.0
                };
                if let Some(sample) = out.get_mut(slot) {
                    *sample = value;
                }
            }
            i += 4;
            position = bits.position();
        }
        if position > end {
            let rewind = (bits.count - end + position) as u32;
            bits.ptr -= (rewind >> 3) as isize;
            bits.count = (rewind & 7) as i32;
            if bits.count != 0 {
                bits.reg = slw(bits.byte(bits.ptr - 1), 32 - bits.count);
            } else if standard_stream {
                // PC port: no bits held; the next channel's begin at the byte.
                bits.reg = 0;
            }
            i -= 4;
        }
        let mut position = bits.position();
        // PC port: a standard stream's stuffing is passed a few bits at a time, which the register
        // holds whole; EA's granules end within a register's reach.
        while standard_stream && end - position > 16 {
            bits.get(16);
            position += 16;
        }
        if position < end {
            bits.get(end - position);
        }
        for j in i..576 {
            if let Some(sample) = usize::try_from(at as i32 + j).ok().and_then(|slot| out.get_mut(slot)) {
                *sample = 0.0;
            }
        }
    }

    /// `CMpegLayer3Base::Dequantize` 0x82c0a050: each band's scale factor gain
    /// (`gTwoToNegativeQuarterPower + 45`, so a gain of 1 leaves the band alone).
    fn dequantize(&self, scratch: &mut [f32; SCRATCH], ch: usize, gr: usize, at: usize) {
        let gi = self.granule_info[ch][gr];
        let sf = self.scale_factors[ch].0;
        let sbi = self.band_index();
        let table = &TWO_TO_NEGATIVE_QUARTER_POWER[45..];
        let shift = gi.scale_fac_scale;
        let long_bands = if gi.short_block() {
            if gi.mixed_block == 0 { 0 } else if self.sfreq >= 3 { 6 } else { 8 }
        } else {
            22
        };
        let samples = &mut scratch[at..at + GRANULE];
        for sfb in 0..long_bands {
            let mut factor = i32::from(sf[sfb]);
            if gi.pre_flag != 0 {
                factor += i32::from(PRETAB[sfb]);
            }
            let gain = table[(2 * (factor << shift)) as usize];
            if gain != 1.0 {
                let (from, to) = (sbi.l[sfb] as usize, sbi.l[sfb + 1] as usize);
                scale(&mut samples[from..to], gain);
            }
        }
        let first_short = if long_bands == 22 { 13 } else if long_bands != 0 { 3 } else { 0 };
        for sfb in first_short..13 {
            let width = usize::from(sbi.s[sfb + 1] - sbi.s[sfb]);
            let mut from = 3 * usize::from(sbi.s[sfb]);
            for window in 0..3 {
                let index = 2 * ((i32::from(sf[short(window, sfb)]) << shift) + 4 * i32::from(gi.sub_block_gain[window]));
                let gain = table[index as usize];
                if gain != 1.0 {
                    scale(&mut samples[from..from + width], gain);
                }
                from += width;
            }
        }
    }

    /// `I_Stereo_K_Values` 0x82c0a2a8.
    fn k_values(is_pos: u32, io_type: usize, i: usize, k: &mut [[f32; 576]; 2]) {
        if is_pos == 0 {
            k[1][i] = 1.0;
            k[0][i] = 1.0;
        } else if is_pos & 1 != 0 {
            k[0][i] = IO[io_type][((is_pos + 1) >> 1) as usize];
            k[1][i] = 1.0;
        } else {
            k[0][i] = 1.0;
            k[1][i] = IO[io_type][(is_pos >> 1) as usize];
        }
    }

    /// The intensity position of line `i`: `is_pos` from the right channel's scale factor, then its
    /// LSF `k` values or MPEG-1 ratio.
    fn set_intensity(&self, state: &mut Intensity, i: usize, value: i16) {
        state.is_pos[i] = value as u32;
        if state.is_pos[i] != 7 {
            if self.lsf {
                Self::k_values(state.is_pos[i], state.io_type, i, &mut state.k);
            } else {
                state.is_ratio[i] = tan12(state.is_pos[i]);
            }
        }
    }

    /// Copies line `from`'s intensity position to line `to`.
    fn copy_intensity(&self, state: &mut Intensity, to: usize, from: usize) {
        state.is_pos[to] = state.is_pos[from];
        if self.lsf {
            state.k[0][to] = state.k[0][from];
            state.k[1][to] = state.k[1][from];
        } else {
            state.is_ratio[to] = state.is_ratio[from];
        }
    }

    /// `CMpegLayer3Base::Stereo` 0x82c0a350 on both spectra (`scratch[0..1152]`, channel 1 at 576).
    #[allow(clippy::too_many_lines)]
    fn stereo(&self, scratch: &mut [f32; SCRATCH], gr: usize) {
        let i_stereo = self.mode == 1 && self.mode_ext & 1 != 0;
        let ms_stereo = self.mode == 1 && self.mode_ext & 2 != 0;
        if !i_stereo {
            if ms_stereo {
                let (left, right) = scratch.split_at_mut(GRANULE);
                for i in 0..GRANULE {
                    let (l, r) = (left[i], right[i]);
                    left[i] = (l + r) * COS4;
                    right[i] = (l - r) * COS4;
                }
            }
            return;
        }
        let gi = self.granule_info[0][gr];
        let mut state = Intensity { is_pos: [7; 576], is_ratio: [0.0; 576], k: [[0.0; 576]; 2], io_type: usize::from(gi.scale_fac_compress & 1) };
        let sf = self.scale_factors[1].0;
        let sbi = self.band_index();
        if gi.short_block() {
            let mixed = gi.mixed_block != 0;
            let mut max_sfb = 0;
            for window in 0..3 {
                let words = SHORT + 13 * window;
                let mut sfbcnt: i32 = if mixed { 2 } else { -1 };
                let lowest = if mixed { 3 } else { 0 };
                let mut sfb: i32 = 12;
                while sfb >= lowest {
                    let lo = i32::from(sbi.s[sfb as usize]);
                    let mut lines = i32::from(sbi.s[sfb as usize + 1]) - lo;
                    let mut i = (window as i32 + 1) * lines + 3 * lo - 1;
                    while lines > 0 {
                        if scratch[GRANULE + i as usize] != 0.0 {
                            sfbcnt = sfb;
                            sfb = -10;
                            lines = -10;
                        }
                        lines -= 1;
                        i -= 1;
                    }
                    sfb -= 1;
                }
                let mut sfb = (sfbcnt + 1) as usize;
                if mixed && sfb > max_sfb {
                    max_sfb = sfb;
                }
                while sfb < 12 {
                    let lo = usize::from(sbi.s[sfb]);
                    let width = usize::from(sbi.s[sfb + 1]) - lo;
                    let start = width * window + 3 * lo;
                    for i in start..start + width {
                        self.set_intensity(&mut state, i, sf[words + sfb]);
                    }
                    sfb += 1;
                }
                let s10 = usize::from(sbi.s[10]);
                let s11 = usize::from(sbi.s[11]);
                let width = usize::from(sbi.s[12]) - s11;
                let from = (s11 - s10) * window + 3 * s10;
                let to = width * window + 3 * s11;
                for i in to..to + width {
                    self.copy_intensity(&mut state, i, from);
                }
            }
            if mixed && max_sfb <= 3 {
                let mut sb: i32 = 2;
                let mut ss: i32 = 17;
                let mut row: i32 = 34;
                let mut last: i32 = -1;
                loop {
                    if scratch[(18 * row + ss) as usize] != 0.0 {
                        last = 18 * sb + ss;
                        break;
                    }
                    ss -= 1;
                    if ss < 0 {
                        sb -= 1;
                        row -= 1;
                        ss = 17;
                    }
                    if row < 32 {
                        break;
                    }
                }
                let mut sfb = 0usize;
                while i32::from(sbi.l[sfb]) <= last {
                    sfb += 1;
                }
                let mut i = sbi.l[sfb] as usize;
                while sfb < 8 {
                    for _ in 0..(sbi.l[sfb + 1] - sbi.l[sfb]).max(0) {
                        self.set_intensity(&mut state, i, sf[sfb]);
                        i += 1;
                    }
                    sfb += 1;
                }
            }
        } else {
            let mut sb: i32 = 31;
            let mut ss: i32 = 17;
            let mut row: i32 = 63;
            let mut last: i32 = 0;
            loop {
                if scratch[(18 * row + ss) as usize] != 0.0 {
                    last = 18 * sb + ss;
                    break;
                }
                ss -= 1;
                if ss < 0 {
                    sb -= 1;
                    row -= 1;
                    ss = 17;
                }
                if row < 32 {
                    break;
                }
            }
            let mut sfb = 0usize;
            while i32::from(sbi.l[sfb]) <= last {
                sfb += 1;
            }
            let mut i = sbi.l[sfb] as usize;
            while sfb < 21 {
                for _ in 0..(sbi.l[sfb + 1] - sbi.l[sfb]).max(0) {
                    self.set_intensity(&mut state, i, sf[sfb]);
                    i += 1;
                }
                sfb += 1;
            }
            let from = sbi.l[20] as usize;
            for _ in 0..(576 - i32::from(sbi.l[21])).max(0) {
                if i >= 576 {
                    break;
                }
                self.copy_intensity(&mut state, i, from);
                i += 1;
            }
        }
        let (left, right) = scratch.split_at_mut(GRANULE);
        for i in 0..GRANULE {
            if state.is_pos[i] == 7 {
                if ms_stereo {
                    let (l, r) = (left[i], right[i]);
                    right[i] = (l - r) * COS4;
                    left[i] = (r + l) * COS4;
                }
            } else if self.lsf {
                let l = left[i];
                let r = state.k[1][i] * l;
                left[i] = state.k[0][i] * l;
                right[i] = r;
            } else {
                let ratio = state.is_ratio[i];
                let part = left[i] / (ratio + 1.0);
                right[i] = part;
                left[i] = ratio * part;
            }
        }
    }

    /// `CMpegLayer3Base::Reorder` 0x82c0bdf0: a short block's windows interleaved line by line
    /// (`from` into `to`); a mixed block keeps its first 36 lines.
    fn reorder(&self, scratch: &mut [f32; SCRATCH], ch: usize, gr: usize, from: usize, to: usize) {
        let gi = self.granule_info[ch][gr];
        if !gi.short_block() {
            return;
        }
        let sfreq = usize::from(self.sfreq).min(8);
        let sbi = self.band_index();
        let s = scratch;
        if gi.mixed_block != 0 {
            for i in 0..36 {
                s[to + i] = s[from + i];
            }
            for sfb in 3..13 {
                let lo = usize::from(sbi.s[sfb]);
                let width = usize::from(sbi.s[sfb + 1]) - lo;
                let start = 3 * lo;
                for j in 0..width {
                    s[to + start + 3 * j] = s[from + start + j];
                    s[to + start + 3 * j + 1] = s[from + start + width + j];
                    s[to + start + 3 * j + 2] = s[from + start + 2 * width + j];
                }
            }
            return;
        }
        for band in 0..3 {
            for j in 0..4 {
                for window in 0..3 {
                    s[to + 12 * band + 3 * j + window] = s[from + 12 * band + 4 * window + j];
                }
            }
        }
        let mut src = 36;
        let mut dst = 36;
        for &width in &REORDER_MAP[sfreq] {
            let width = usize::from(width);
            let pairs = if width == 0 { 0 } else { ((width - 1) >> 1) + 1 };
            for pair in 0..pairs {
                let b = src + 2 * pair;
                let o = dst + 6 * pair;
                s[to + o] = s[from + b];
                s[to + o + 1] = s[from + b + width];
                s[to + o + 2] = s[from + b + 2 * width];
                s[to + o + 3] = s[from + b + 1];
                s[to + o + 4] = s[from + b + width + 1];
                s[to + o + 5] = s[from + b + 2 * width + 1];
            }
            src += 2 * pairs + 2 * width;
            dst += 6 * pairs;
        }
    }

    /// `CMpegLayer3Base::AntiAlias` 0x82c0c2b8: the butterflies across subband boundaries (only the
    /// first of a mixed block, none for a short block); each output is `fmsubs` of a rounded
    /// product.
    fn anti_alias(&self, scratch: &mut [f32; SCRATCH], ch: usize, gr: usize, at: usize) {
        let gi = self.granule_info[ch][gr];
        let boundaries = if gi.window_switching == 0 {
            31
        } else if gi.block_type == 2 && gi.mixed_block == 0 {
            return;
        } else if gi.mixed_block != 0 && gi.block_type == 2 {
            1
        } else {
            31
        };
        let x = &mut scratch[at..at + GRANULE];
        for sb in 1..=boundaries {
            let boundary = 18 * sb;
            for i in 0..8 {
                let lo = x[boundary - 1 - i];
                let hi = x[boundary + i];
                let (cs, ca) = (ANTIALIAS_CS[i], ANTIALIAS_CA[i]);
                x[boundary - 1 - i] = fmadd(lo, cs, -(hi * ca));
                x[boundary + i] = fmadd(hi, cs, -(lo * -ca));
            }
        }
    }

    /// `CMpegLayer3Base::Hybrid` 0x82c0ccf0: each X4 band's lanes through `Imdct36X1` (`win_0` of
    /// the block type) or `Imdct12X1`, then `OverlapAddX4` with the channel's previous block. A mixed
    /// block's first band has two long and two short lanes.
    fn hybrid(&mut self, ch: usize, gr: usize, bands: &mut [[[f32; 4]; 18]; 8]) {
        let gi = self.granule_info[ch][gr];
        let mut first = 0;
        let mut imdct = [[0.0f32; 4]; 36];
        if gi.window_switching != 0 && gi.mixed_block != 0 {
            imdct36(&bands[0], 0, &mut imdct, &WIN[0]);
            imdct36(&bands[0], 1, &mut imdct, &WIN[0]);
            imdct12(&bands[0], 2, &mut imdct);
            imdct12(&bands[0], 3, &mut imdct);
            overlap_add(&mut bands[0], &imdct, &mut self.prev_block[ch][0]);
            first = 1;
        }
        for (band, prev) in bands.iter_mut().zip(self.prev_block[ch].iter_mut()).skip(first) {
            for lane in 0..4 {
                if gi.block_type == 2 {
                    imdct12(band, lane, &mut imdct);
                } else {
                    imdct36(band, lane, &mut imdct, &WIN[usize::from(gi.block_type)]);
                }
            }
            overlap_add(band, &imdct, prev);
        }
    }

    /// `CMpegBase::PolySynth` 0x82c0dba8: one block of 32 subband samples through `dct64` into the
    /// channel's history and the `decwin_0` windowing, 32 output samples (mpg123's `synth_1to1`
    /// order of fused operations; the backward half's last term reads `window[0]`).
    fn poly_synth(&mut self, ch: usize, out: &mut [f32], band: &[f32; 32]) {
        let bo = (u32::from(self.band_offset[ch]).wrapping_sub(1) & 0xF) as usize;
        self.band_offset[ch] = bo as u8;
        let odd = bo & 1;
        let bo1 = bo + (odd ^ 1);
        let [buf0, buf1] = &mut self.synth_history[ch];
        if odd == 1 {
            dct64(&mut buf1[(bo + 1) & 0xF..], &mut buf0[bo..], band);
        } else {
            dct64(&mut buf0[bo..], &mut buf1[bo + 1..], band);
        }
        let b0: &[f32; 288] = if odd == 1 { buf0 } else { buf1 };
        let w = &DECWIN;
        let mut wb = 16 - bo1;
        let mut hb = 0;
        for sample in out.iter_mut().take(16) {
            let mut sum = fmadd(w[wb], b0[hb], -(w[wb + 1] * b0[hb + 1]));
            for k in 2..16 {
                sum = if k & 1 == 0 { fmadd(w[wb + k], b0[hb + k], sum) } else { fnmsubs(w[wb + k], b0[hb + k], sum) };
            }
            *sample = sum;
            wb += 32;
            hb += 16;
        }
        let mut sum = w[wb] * b0[hb];
        for k in (2..16).step_by(2) {
            sum = fmadd(w[wb + k], b0[hb + k], sum);
        }
        out[16] = sum;
        wb = wb + 2 * bo1 - 32;
        for sample in out.iter_mut().skip(17).take(15) {
            hb -= 16;
            let mut sum = -(w[wb - 1] * b0[hb]);
            for k in 1..15 {
                sum = fnmsubs(w[wb - 1 - k], b0[hb + k], sum);
            }
            *sample = fnmsubs(b0[hb + 15], w[wb], sum);
            wb -= 32;
        }
    }

    /// `EALayer3Core::Decode` 0x82c13aa8: one granule into `outputs` (576 samples per channel).
    pub(super) fn decode(&mut self, scratch: &mut [f32; SCRATCH], outputs: &mut [&mut [f32]]) -> i32 {
        self.bits.base = self.bits.ptr;
        let header = self.bits.get(8);
        self.process_header(header);
        let gr = self.bits.get(1) as usize;
        self.side_info(gr);
        let channels = usize::from(self.channels);
        let mut buffer = [0usize, GRANULE];
        let mut work = WORK;
        for (ch, &at) in buffer.iter().enumerate().take(channels) {
            let part2_start = self.bits.position();
            if self.version == 1 {
                self.scale_factors(ch, gr);
            } else {
                self.lsf_scale_factors(ch, gr);
            }
            let gi = self.granule_info[ch][gr];
            let regions = if gi.window_switching == 0 { 3 } else { 2 };
            for region in 0..regions {
                self.loaded_table_select[region] = huff_table(gi.table_select[region]);
            }
            self.decode_huffman(scratch, ch, gr, at, part2_start);
            self.dequantize(scratch, ch, gr, at);
        }
        if channels > 1 {
            self.stereo(scratch, gr);
        }
        for ch in 0..channels.min(2).min(self.prev_block.len()) {
            let gi = self.granule_info[ch][gr];
            if gi.short_block() {
                self.reorder(scratch, ch, gr, buffer[ch], work);
            } else {
                std::mem::swap(&mut buffer[ch], &mut work);
            }
            self.anti_alias(scratch, ch, gr, work);
            let mut bands = [[[0.0f32; 4]; 18]; 8];
            reorder_for_vectoring(&scratch[work..work + GRANULE], &mut bands);
            self.hybrid(ch, gr, &mut bands);
            if self.reset_synth_required {
                self.synth_history.fill([[0.0; 288]; 2]);
                self.reset_synth_required = false;
            }
            frequency_inversion(&mut bands);
            let mut subbands = [[0.0f32; 32]; 18];
            reorder_for_poly_synth(&bands, &mut subbands);
            for (block, band) in subbands.iter().enumerate() {
                if let Some(out) = outputs.get_mut(ch) {
                    let mut pcm = [0.0f32; 32];
                    self.poly_synth(ch, &mut pcm, band);
                    if let Some(slot) = out.get_mut(32 * block..32 * block + 32) {
                        slot.copy_from_slice(&pcm);
                    }
                }
            }
            write_x4(&bands, &mut scratch[buffer[ch]..buffer[ch] + GRANULE]);
            write_rows(&subbands, &mut scratch[work..work + GRANULE]);
        }
        let tail = -self.bits.count & 7;
        if tail != 0 {
            self.bits.get(8 - tail);
        }
        self.frame_size = (self.bits.position() as u32 >> 3) as i32;
        0
    }
}

/// `Stereo`'s stack arrays: `is_pos` (7 = none), `is_ratio` (MPEG-1) and `k` (LSF).
struct Intensity {
    is_pos: [u32; 576],
    is_ratio: [f32; 576],
    k: [[f32; 576]; 2],
    io_type: usize,
}

/// `OverlapAddX4` 0x82c0cb00: the band is the IMDCT's first half plus the previous block, which
/// becomes its second half.
fn overlap_add(band: &mut [[f32; 4]; 18], imdct: &[[f32; 4]; 36], prev: &mut [[f32; 4]; 18]) {
    for lane in 0..4 {
        for i in 0..18 {
            band[i][lane] = imdct[i][lane] + prev[i][lane];
            prev[i][lane] = imdct[18 + i][lane];
        }
    }
}

/// `ReorderForVectoring` 0x82c070d0: `[32][18]` into eight bands of four subband lanes.
fn reorder_for_vectoring(source: &[f32], bands: &mut [[[f32; 4]; 18]; 8]) {
    for (group, band) in bands.iter_mut().enumerate() {
        for (ss, row) in band.iter_mut().enumerate() {
            for (lane, value) in row.iter_mut().enumerate() {
                *value = source[18 * (4 * group + lane) + ss];
            }
        }
    }
}

/// `FrequencyInversionX4` 0x82c07838: odd subbands' odd samples times -1.
fn frequency_inversion(bands: &mut [[[f32; 4]; 18]; 8]) {
    for band in bands.iter_mut() {
        for row in band.iter_mut().skip(1).step_by(2) {
            row[1] *= -1.0;
            row[3] *= -1.0;
        }
    }
}

/// `HELPER_ReorderForFPolySynth` 0x82b93510: back to `[18][32]`, one row per synthesis call.
fn reorder_for_poly_synth(bands: &[[[f32; 4]; 18]; 8], rows: &mut [[f32; 32]; 18]) {
    for (ss, row) in rows.iter_mut().enumerate() {
        for (sb, value) in row.iter_mut().enumerate() {
            *value = bands[sb / 4][ss][sb % 4];
        }
    }
}

/// The X4 bands left in a spectrum buffer, as the native in-place arrays hold them afterwards.
fn write_x4(bands: &[[[f32; 4]; 18]; 8], out: &mut [f32]) {
    for (i, value) in bands.iter().flatten().flatten().enumerate() {
        out[i] = *value;
    }
}

fn write_rows(rows: &[[f32; 32]; 18], out: &mut [f32]) {
    for (i, value) in rows.iter().flatten().enumerate() {
        out[i] = *value;
    }
}
