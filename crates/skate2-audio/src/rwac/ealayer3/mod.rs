//! `rw::audio::core::EaLayer3DecBase`: EA's MPEG layer III streams, decoded in software on the 360.
//! `SndPlayer1::sDecoderGuidLut` gives EAAC codec 5 'EL31' (`EaLayer31Dec`, version 0), 6 'L32P'
//! (`EaLayer32PcmDec`, version 1) and 7 'L32S' (`EaLayer32SpikeDec`, version 2); all three share
//! this base. A request's data is a run of granules, one `EALayer3Core` per channel pair:
//!
//! * version 0: a byte (0xEE when a PCM block follows the granule), the granule (`EALayer3Core`);
//! * versions 1 and 2: an `EaLayer32Block` header, an optional granule, optional PCM.
//!
//! Its `DecoderDesc` (0x83012ea4) has a 576-sample block, so `Decoder::Decode` runs its block-based
//! branch: each `DecodeEvent` fills the decoder's own sample buffer ("Decoder block storage") with
//! one granule, which `Decode` copies out as asked. The decoder adds 1105 samples of latency to a
//! new feed and drops them.

mod core;
mod generated_transforms;
pub mod mpeg;
mod tables;

use self::core::{Layer3Core, GRANULE, SCRATCH};
use crate::rwac::decoder::DataRef;

/// `mLatency` of a new feed: the decoder's delay in samples.
const LATENCY: i32 = 1105;
/// `__real_38000000`: `DecodeEvent` scales the 16-bit range to [-1, 1).
const OUTPUT_SCALE: f32 = f32::from_bits(0x3800_0000);

/// `Decoder::RequestDesc` (0x18 bytes).
#[derive(Clone, Default)]
struct Request {
    src: Option<DataRef>,
    seek_data: Option<DataRef>,
    decoder_skip: i32,
    num_samples: i32,
    feed_type: u8,
}

/// `EaLayer32Block` (0x20 bytes) of a version 1/2 block.
#[derive(Default)]
struct Layer32Block {
    granule_offset: usize,
    pcm_data_offset: usize,
    pcm_samples: i32,
    offset_samples: i32,
    /// `BlockOffsetMode`: 0 ignore, 1 preserve, 2 mute.
    offset_mode: u32,
}

impl Layer32Block {
    /// `EaLayer32Block::Read` 0x82c08418: the block's size, and its layout. Bit 15 of the first
    /// half-word announces the 32-bit word of offset mode (2 bits), offset samples (10), PCM
    /// samples (10) and granule size (10).
    fn read(data: &[u8]) -> (i32, Self) {
        let byte = |at: usize| data.get(at).copied().unwrap_or(0);
        let head = u16::from_be_bytes([byte(0), byte(1)]);
        let size = i32::from(head & 0xFFF);
        if head & 0x8000 == 0 {
            return (size, Self { granule_offset: 2, ..Self::default() });
        }
        let word = u32::from_be_bytes([byte(2), byte(3), byte(4), byte(5)]);
        let granule_size = (word & 0x3FF) as usize;
        let block = Self {
            granule_offset: if granule_size != 0 { 6 } else { 0 },
            pcm_data_offset: granule_size + 6,
            pcm_samples: ((word >> 10) & 0x3FF) as i32,
            offset_samples: ((word >> 20) & 0x3FF) as i32,
            offset_mode: (word >> 30) & 3,
        };
        (size, block)
    }
}

/// A big-endian `s16` as `fcfid` + `frsp` convert it.
fn s16(data: &[u8], at: usize) -> f32 {
    let byte = |at: usize| data.get(at).copied().unwrap_or(0);
    f32::from(i16::from_be_bytes([byte(at), byte(at + 1)]))
}

/// `rw::audio::core::Decoder` with a 576-sample block and the `EaLayer3DecBase` instance data.
pub struct EaLayer3Decoder {
    pub num_channels: u8,
    slots: Vec<Request>,
    feed_slot: usize,
    prepare_slot: usize,
    decode_slot: usize,
    decode_slot_samples_decoded: i32,
    /// `mDecodedSamplesAvailable` and the block buffer's `mNumSamples`.
    decoded_samples_available: u16,
    block_samples: u16,
    /// "Decoder block storage": 576 samples per channel.
    block: Vec<f32>,
    /// `mVersion`: 0 'EL31', 1 'L32P', 2 'L32S'.
    version: u8,
    /// `mppEaLayer3Core`: one core per channel pair.
    cores: Vec<Layer3Core>,
    /// `EALayer3Core::Decode`'s stack buffers, which every core's granule reuses.
    scratch: Box<[f32; SCRATCH]>,
    /// `mpEncodedSample`.
    encoded: Option<DataRef>,
    remaining_samples: i32,
    latency: i32,
    skip_samples: i32,
    new_feed: bool,
}

impl EaLayer3Decoder {
    /// `DecoderRegistry::DecoderFactory` 0x82be3ee8 with `EaLayer3DecBase::CreateInstance` 0x82c167e8:
    /// `(channels + 1) / 2` cores, the last one mono for an odd count.
    #[must_use]
    pub fn new(version: u8, num_channels: u8, max_slots: usize) -> Self {
        let total = i32::from(num_channels);
        let cores = (0..(total + 1) / 2).map(|index| Layer3Core::new(if total / 2 - index != 0 { 2 } else { 1 })).collect();
        Self {
            num_channels,
            slots: vec![Request::default(); max_slots],
            feed_slot: 0,
            prepare_slot: 0,
            decode_slot: 0,
            decode_slot_samples_decoded: 0,
            decoded_samples_available: 0,
            block_samples: 0,
            block: vec![0.0; GRANULE * usize::from(num_channels)],
            version,
            cores,
            scratch: Box::new([0.0; SCRATCH]),
            encoded: None,
            remaining_samples: 0,
            latency: LATENCY,
            skip_samples: 0,
            new_feed: false,
        }
    }

    /// `Decoder::Feed` 0x82bdcb68 (`Decoder::vftable`'s feed event is empty): queues a request and
    /// returns its slot; a full queue returns 0.
    #[allow(clippy::too_many_arguments)]
    pub fn feed(
        &mut self,
        src: DataRef,
        num_samples: i32,
        feed_type: u8,
        decoder_skip: i32,
        seek_data: Option<DataRef>,
        _seek_data_version: u8,
        _num_bytes: i32,
    ) -> u8 {
        let slot = self.feed_slot;
        if self.slots[slot].num_samples != 0 {
            return 0;
        }
        self.slots[slot] = Request { src: Some(src), seek_data, decoder_skip, num_samples, feed_type };
        if self.feed_slot == self.decode_slot {
            self.decode_slot_samples_decoded = decoder_skip;
        }
        self.feed_slot += 1;
        if self.feed_slot >= self.slots.len() {
            self.feed_slot = 0;
        }
        slot as u8
    }

    /// `Decoder::GetSamplesRemaining` 0x82bf61a0.
    #[must_use]
    pub fn samples_remaining(&self, slot: u8) -> i32 {
        let slot = usize::from(slot);
        let desc = &self.slots[slot];
        if desc.num_samples == 0 {
            0
        } else if slot == self.decode_slot {
            desc.num_samples - self.decode_slot_samples_decoded
        } else {
            desc.num_samples - desc.decoder_skip
        }
    }

    /// `Decoder::AdvanceDecodeState` 0x82bdcc28.
    fn advance_decode_state(&mut self, samples: i32) {
        self.decode_slot_samples_decoded += samples;
        if self.decode_slot_samples_decoded == self.slots[self.decode_slot].num_samples {
            self.slots[self.decode_slot].num_samples = 0;
            self.decode_slot += 1;
            if self.decode_slot >= self.slots.len() {
                self.decode_slot = 0;
            }
            self.decode_slot_samples_decoded = self.slots[self.decode_slot].decoder_skip;
        }
    }

    /// A block sample, as the copy out reads it (nothing outside the storage).
    fn block_sample(&self, at: isize) -> f32 {
        usize::try_from(at).ok().and_then(|at| self.block.get(at)).copied().unwrap_or(0.0)
    }

    /// `Decoder::Decode` 0x82bdccb0, block-based branch: what is left of the last block first, then
    /// a `DecodeEvent` per block, each copied out up to `count` samples.
    pub fn decode(&mut self, out: &mut [&mut [f32]], count: i32) -> i32 {
        let channels = usize::from(self.num_channels);
        let mut done = 0i32;
        if self.decoded_samples_available != 0 {
            done = i32::from(self.decoded_samples_available).min(count);
            for (ch, channel) in out.iter_mut().enumerate().take(channels) {
                let base = (GRANULE * ch) as isize + self.block_samples as isize - self.decoded_samples_available as isize;
                for i in 0..done as usize {
                    channel[i] = self.block_sample(base + i as isize);
                }
            }
            self.decoded_samples_available -= done as u16;
            self.advance_decode_state(done);
        }
        if done < count {
            loop {
                let slot = self.decode_slot;
                if self.slots[slot].num_samples == 0 {
                    break;
                }
                let decoded = self.decode_event();
                let mut available = self.slots[slot].num_samples - self.decode_slot_samples_decoded;
                if decoded < available {
                    available = decoded;
                }
                self.decoded_samples_available = available as u16;
                self.block_samples = available as u16;
                let take = i32::from(self.decoded_samples_available).min(count - done);
                for (ch, channel) in out.iter_mut().enumerate().take(channels) {
                    for i in 0..take as usize {
                        channel[done as usize + i] = self.block_sample((GRANULE * ch + i) as isize);
                    }
                }
                done += take;
                self.decode_slot_samples_decoded += take;
                self.decoded_samples_available = self.decoded_samples_available.wrapping_sub(take as u16);
                if self.decode_slot_samples_decoded == self.slots[slot].num_samples {
                    self.slots[slot].num_samples = 0;
                    self.decode_slot += 1;
                    if self.decode_slot >= self.slots.len() {
                        self.decode_slot = 0;
                    }
                    self.decode_slot_samples_decoded = self.slots[self.decode_slot].decoder_skip;
                }
                if done >= count {
                    return done;
                }
            }
        }
        done
    }

    /// `EaLayer3DecBase::SkipBlocks` 0x82c12e18: a seek's `decoderSkip` drops whole granules by
    /// the seek data's sizes (version 0: one size per granule; versions 1/2: one per ten blocks,
    /// the rest by their headers).
    fn skip_blocks(&mut self) {
        let skip = self.skip_samples;
        if skip <= 0 {
            return;
        }
        if !self.new_feed || skip >= 47 || self.version == 1 {
            self.latency = 0;
        } else {
            self.latency -= 576;
        }
        let (blocks, skipped) = if !self.new_feed || self.version == 1 {
            (skip / 576, 576 * (skip / 576))
        } else {
            let blocks = (skip + LATENCY) / 576;
            (blocks, if blocks > 1 { 576 * blocks - LATENCY } else { 0 })
        };
        self.skip_samples = skip - skipped;
        let tens = if self.version == 0 { blocks } else { (blocks - 1) / 10 };
        let seek = &self.slots[self.decode_slot];
        let mut offset = 0i32;
        if let Some(seek) = seek.seek_data.as_ref().filter(|_| seek.num_samples != 0) {
            let table = seek.slice();
            for k in 0..tens.max(0) as usize {
                offset += i32::from(i16::from_be_bytes([table.get(2 * k).copied().unwrap_or(0), table.get(2 * k + 1).copied().unwrap_or(0)]));
            }
        }
        if let Some(encoded) = self.encoded.clone() {
            let data = encoded.slice();
            for _ in 0..(blocks - 10 * tens).max(0) {
                for _ in 0..self.cores.len() {
                    let at = offset as usize;
                    let size = i32::from(u16::from_be_bytes([data.get(at).copied().unwrap_or(0), data.get(at + 1).copied().unwrap_or(0)]) & 0xFFF);
                    offset += size;
                }
            }
            self.encoded = Some(encoded.at(offset as usize));
        }
    }

    /// `EaLayer3DecBase::DecodeEvent` 0x82c15e00: starts the next prepared request when the last is
    /// used up, then decodes granules (each core's in turn) until one has samples past the latency
    /// and skip, scaled by 2^-15 into the block buffer. The number of samples, at most what the
    /// request has left.
    fn decode_event(&mut self) -> i32 {
        if self.remaining_samples <= 0 {
            let slot = self.prepare_slot;
            let desc = self.slots[slot].clone();
            if desc.num_samples != 0 {
                self.prepare_slot += 1;
                if self.prepare_slot >= self.slots.len() {
                    self.prepare_slot = 0;
                }
            }
            if desc.feed_type == 0 {
                self.new_feed = true;
                self.latency = LATENCY;
                self.skip_samples = 0;
            }
            self.encoded.clone_from(&desc.src);
            self.skip_samples = desc.decoder_skip;
            self.remaining_samples = desc.num_samples - desc.decoder_skip;
            if desc.decoder_skip > 0 {
                self.skip_blocks();
            }
        }
        let channels = usize::from(self.num_channels);
        let mut frames = 0;
        let mut latency_used = 0;
        let mut skip_used = 0;
        loop {
            let start = self.encoded.clone();
            for core in 0..self.cores.len() {
                let core_channels = if i32::from(self.num_channels) / 2 - core as i32 != 0 { 2 } else { 1 };
                let src = self.encoded.clone();
                let consumed = match &src {
                    Some(src) => self.decode_granule(core, src, 2 * core, core_channels, &mut frames, &mut latency_used, &mut skip_used),
                    None => -1,
                };
                let Some(src) = src.filter(|_| consumed >= 0) else {
                    for ch in 0..channels {
                        self.block[GRANULE * ch..GRANULE * (ch + 1)].fill(0.0);
                    }
                    self.encoded = start;
                    return -1;
                };
                self.encoded = Some(src.at(consumed as usize));
            }
            self.new_feed = false;
            if self.latency > 0 {
                self.latency -= latency_used;
            }
            if self.skip_samples > 0 {
                self.skip_samples -= skip_used;
            }
            for ch in 0..channels {
                for sample in self.block[GRANULE * ch..].iter_mut().take(frames.max(0) as usize) {
                    *sample *= OUTPUT_SCALE;
                }
            }
            if frames > 0 {
                let remaining = self.remaining_samples;
                if remaining < frames {
                    frames = remaining;
                }
                if remaining >= 0 {
                    self.remaining_samples = remaining - frames;
                }
                return frames;
            }
        }
    }

    /// `EaLayer3DecBase::DecodeGranule` 0x82c15aa8: one core's granule from `src` into the block
    /// channels from `first`, then the latency and skip taken off the front. The bytes used.
    #[allow(clippy::too_many_arguments)]
    fn decode_granule(
        &mut self,
        core: usize,
        src: &DataRef,
        first: usize,
        channels: usize,
        frames: &mut i32,
        latency_used: &mut i32,
        skip_used: &mut i32,
    ) -> i32 {
        let data = src.slice();
        let (mut used, block, granule, pcm_block) = if self.version != 0 {
            let (size, block) = Layer32Block::read(data);
            let granule = (block.granule_offset != 0).then(|| src.at(block.granule_offset));
            (size, Some(block), granule, false)
        } else {
            (1, None, Some(src.at(1)), data.first() == Some(&0xEE))
        };
        if let Some(granule) = &granule {
            let core_decoder = &mut self.cores[core];
            if self.new_feed {
                core_decoder.open(granule);
            } else {
                core_decoder.resume(granule);
            }
            let mut outputs: Vec<&mut [f32]> = self.block.chunks_mut(GRANULE).skip(first).take(channels).collect();
            core_decoder.decode(&mut self.scratch, &mut outputs);
        }
        let mut offset = 0;
        let mut mute = 0;
        if let Some(block) = &block {
            let mut count = 576;
            match block.offset_mode {
                0 => {
                    count = 576 - block.offset_samples;
                    if block.granule_offset == 0 {
                        count = block.pcm_samples;
                    }
                }
                1 => {
                    if block.granule_offset == 0 {
                        count = block.pcm_samples;
                    }
                }
                2 if block.granule_offset == 0 => count = 2 * block.pcm_samples,
                _ => {}
            }
            *frames = count;
            match block.offset_mode {
                0 => offset = block.offset_samples,
                2 => mute = block.offset_samples,
                _ => {}
            }
        } else {
            *frames = 576;
            *latency_used = 0;
            if self.latency > 0 {
                if self.latency >= *frames {
                    *latency_used = *frames;
                    *frames = 0;
                } else {
                    *latency_used = self.latency;
                    *frames -= self.latency;
                    offset = self.latency;
                }
            }
        }
        if self.skip_samples > 0 {
            if self.skip_samples >= *frames {
                *skip_used = *frames;
                *frames = 0;
            } else {
                *skip_used = self.skip_samples;
                *frames -= self.skip_samples;
                offset += self.skip_samples;
            }
        }
        if let Some(block) = &block {
            if block.pcm_samples > 0 {
                let pcm = &data[block.pcm_data_offset.min(data.len())..];
                self.decode_pcm(first, channels, block.offset_samples, pcm, block.pcm_samples);
            }
        } else {
            used += self.cores[core].frame_size;
            if pcm_block {
                let at = (*latency_used) as usize;
                used += self.decode_special_block(&data[(used as usize).min(data.len())..], first, at, core);
            }
        }
        if *frames > 0 && offset > 0 {
            for ch in first..first + channels {
                let base = GRANULE * ch;
                let count = *frames as usize;
                let from = base + offset as usize;
                if from + count <= self.block.len() {
                    self.block.copy_within(from..from + count, base);
                }
            }
        }
        if mute - offset > 0 {
            for ch in first..first + channels {
                let base = GRANULE * ch;
                let end = (base + (mute - offset) as usize).min(self.block.len());
                self.block[base..end].fill(0.0);
            }
        }
        used
    }

    /// `EaLayer3DecBase::DecodeSpecialBlock` 0x82c08230: the PCM after a version 0 granule
    /// flagged 0xEE: a sample count, an offset, then each channel's samples. The bytes used.
    fn decode_special_block(&mut self, data: &[u8], first: usize, at: usize, core: usize) -> i32 {
        let word = |k: usize| u32::from_be_bytes([0, 1, 2, 3].map(|b| data.get(k + b).copied().unwrap_or(0))) as i32;
        let count = word(0);
        let offset = word(4);
        let channels = usize::from(self.cores[core].channels());
        let mut read = 8;
        for ch in 0..channels {
            let base = GRANULE * (first + ch) + at;
            for i in 0..count.max(0) as usize {
                let value = s16(data, read);
                read += 2;
                if let Some(sample) = (base as i64 + i64::from(offset) + i as i64)
                    .try_into()
                    .ok()
                    .and_then(|slot: usize| self.block.get_mut(slot))
                {
                    *sample = value;
                }
            }
        }
        2 * (channels as i32 * count + 4)
    }

    /// `EaLayer3DecBase::DecodePcm` 0x82c07ef0: big-endian `s16` PCM, interleaved for two channels
    /// (any other count fills the first channel), from `offset` on.
    fn decode_pcm(&mut self, first: usize, channels: usize, offset: i32, data: &[u8], samples: i32) {
        let samples = samples.max(0) as usize;
        let stereo = channels == 2;
        for i in 0..samples {
            let base = offset.max(0) as usize + i;
            if stereo {
                let left = s16(data, 4 * i);
                let right = s16(data, 4 * i + 2);
                if let Some(sample) = self.block.get_mut(GRANULE * first + base) {
                    *sample = left;
                }
                if let Some(sample) = self.block.get_mut(GRANULE * (first + 1) + base) {
                    *sample = right;
                }
            } else if let Some(sample) = self.block.get_mut(GRANULE * first + base) {
                *sample = s16(data, 2 * i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: u32) -> u32 {
            ((self.next() >> 33) % u64::from(n)) as u32
        }
    }

    #[derive(Default)]
    struct Bits {
        bytes: Vec<u8>,
        count: usize,
    }

    impl Bits {
        fn put(&mut self, value: u32, n: u32) {
            for i in (0..n).rev() {
                if self.count % 8 == 0 {
                    self.bytes.push(0);
                }
                if (value >> i) & 1 != 0 {
                    *self.bytes.last_mut().expect("byte") |= 0x80 >> (self.count % 8);
                }
                self.count += 1;
            }
        }
    }

    /// A granule of random side info and main data (MPEG-1, MPEG-2 and MPEG-2.5 at 11/12 kHz;
    /// long, start/stop, short and mixed blocks; joint stereo with intensity and M/S).
    fn granule(rng: &mut Rng, stereo: bool) -> Vec<u8> {
        let mut b = Bits::default();
        let version = [3, 3, 2, 0][rng.below(4) as usize];
        let rate = if version == 0 { rng.below(2) } else { rng.below(3) };
        let mode = if stereo { [0, 1, 1, 2][rng.below(4) as usize] } else { 3 };
        b.put(version << 6 | rate << 4 | mode << 2 | rng.below(4), 8);
        let lsf = version != 3;
        let gr = if lsf { 0 } else { rng.below(2) };
        b.put(gr, 1);
        let channels = if mode == 3 { 1 } else { 2 };
        if !lsf && gr == 1 {
            for _ in 0..channels {
                b.put(rng.below(16), 4);
            }
        }
        let mut lengths = Vec::new();
        for _ in 0..channels {
            let length = rng.below(1200);
            lengths.push(length);
            b.put(length, 12);
            b.put(rng.below(289), 9);
            b.put(rng.below(256), 8);
            if lsf {
                b.put(rng.below(512), 9);
            } else {
                b.put(rng.below(16), 4);
            }
            let switched = rng.below(10) < 4;
            b.put(u32::from(switched), 1);
            if switched {
                b.put([1, 2, 2, 3][rng.below(4) as usize], 2);
                b.put(u32::from(rng.below(10) < 3), 1);
                b.put(rng.below(32), 5);
                b.put(rng.below(32), 5);
                for _ in 0..3 {
                    b.put(rng.below(8), 3);
                }
            } else {
                for _ in 0..3 {
                    b.put(rng.below(32), 5);
                }
                let region0 = rng.below(16);
                b.put(region0, 4);
                b.put(rng.below(8.min(21 - region0)), 3);
            }
            if !lsf {
                b.put(rng.below(2), 1);
            }
            b.put(rng.below(2), 1);
            b.put(rng.below(2), 1);
        }
        for length in lengths {
            for _ in 0..length {
                b.put((rng.next() >> 63) as u32, 1);
            }
        }
        b.bytes
    }

    fn pcm(rng: &mut Rng, count: u32) -> Vec<u8> {
        (0..count).flat_map(|_| ((rng.below(60000) as i32 - 30000) as i16).to_be_bytes()).collect()
    }

    fn chunk(out: &mut Vec<u8>, payload: &[u8], samples: u32, last: bool) {
        let head = (payload.len() as u32 + 8) | if last { 0x8000_0000 } else { 0 };
        out.extend_from_slice(&head.to_be_bytes());
        out.extend_from_slice(&samples.to_be_bytes());
        out.extend_from_slice(payload);
    }

    /// Version 0, two channels: 4 granules per chunk, some followed by a PCM block (0xEE).
    fn stream_v0(rng: &mut Rng) -> Vec<u8> {
        let mut out = Vec::new();
        for index in 0..3 {
            let mut payload = Vec::new();
            for _ in 0..4 {
                let special = index > 0 && rng.below(4) == 0;
                payload.push(if special { 0xEE } else { 0 });
                payload.extend(granule(rng, true));
                if special {
                    let count = rng.below(40);
                    let offset = rng.below(200);
                    payload.extend_from_slice(&count.to_be_bytes());
                    payload.extend_from_slice(&offset.to_be_bytes());
                    payload.extend(pcm(rng, 2 * count));
                }
            }
            chunk(&mut out, &payload, 576 * 4 - if index == 0 { 1105 } else { 0 }, index == 2);
        }
        out
    }

    /// Version 1, one channel: plain granule blocks and blocks with offsets and PCM.
    fn stream_v2(rng: &mut Rng) -> Vec<u8> {
        let mut out = Vec::new();
        for index in 0..3 {
            let mut payload = Vec::new();
            let mut samples = 0;
            for _ in 0..5 {
                if rng.below(2) == 0 {
                    let granule = granule(rng, false);
                    payload.extend_from_slice(&(((2 + granule.len()) & 0xFFF) as u16).to_be_bytes());
                    payload.extend(granule);
                    samples += 576;
                } else {
                    let has_granule = rng.below(10) < 6;
                    let granule = if has_granule { granule(rng, false) } else { Vec::new() };
                    let mode = rng.below(3);
                    let pcm_samples = rng.below(120);
                    let offset = rng.below(200);
                    let word = mode << 30 | offset << 20 | pcm_samples << 10 | (granule.len() as u32 & 0x3FF);
                    let data = pcm(rng, pcm_samples);
                    let size = (6 + granule.len() + data.len()) & 0xFFF;
                    payload.extend_from_slice(&(0x8000 | size as u16).to_be_bytes());
                    payload.extend_from_slice(&word.to_be_bytes());
                    payload.extend(granule);
                    payload.extend(data);
                    samples += match (mode, has_granule) {
                        (0, true) => 576 - offset,
                        (0 | 1, false) => pcm_samples,
                        (2, false) => 2 * pcm_samples,
                        _ => 576,
                    };
                }
            }
            chunk(&mut out, &payload, samples.max(1), index == 2);
        }
        out
    }

    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xCBF2_9CE4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3))
    }

    /// Decodes `stream` as `SndPlayer1` feeds it: one request per chunk, at most 256 samples per
    /// `Decode`. The samples interleaved, as big-endian bits.
    fn decode(stream: &[u8], version: u8, channels: usize) -> Vec<u8> {
        let mut data = stream.to_vec();
        data.extend(std::iter::repeat_n(0, 4096));
        let bytes: Arc<[u8]> = Arc::from(data);
        let mut decoder = EaLayer3Decoder::new(version, channels as u8, 20);
        let mut out = Vec::new();
        let mut pos = 0;
        let mut first = true;
        while pos + 8 <= stream.len() {
            let head = u32::from_be_bytes(stream[pos..pos + 4].try_into().expect("word"));
            let samples = i32::from_be_bytes(stream[pos + 4..pos + 8].try_into().expect("word"));
            let size = (head & 0x7FFF_FFFF) as usize;
            let src = DataRef { bytes: Arc::clone(&bytes), offset: pos + 8 };
            let handle = decoder.feed(src, samples, u8::from(!first), 0, None, 0, size as i32 - 8);
            first = false;
            while decoder.samples_remaining(handle) != 0 {
                let count = decoder.samples_remaining(handle).min(256);
                let mut buffers = vec![[0.0f32; 256]; channels];
                let mut outputs: Vec<&mut [f32]> = buffers.iter_mut().map(|b| &mut b[..]).collect();
                let got = decoder.decode(&mut outputs, count) as usize;
                for i in 0..got {
                    for buffer in &buffers {
                        out.extend_from_slice(&buffer[i].to_bits().to_be_bytes());
                    }
                }
            }
            pos += size;
        }
        out
    }

    /// The hashes come from the debug executable's own decoder (`Decoder::Feed`/`Decode` with
    /// `EaLayer3DecBase` 'EL31' and 'L32P') run on the same streams in a PPC interpreter.
    #[test]
    fn decodes_bit_exact_with_the_native_decoder() {
        let v0 = stream_v0(&mut Rng(0x2545_F491_4F6C_DD1D));
        assert_eq!(fnv1a(&v0), 0x3c38_4523_6caa_274a);
        let out = decode(&v0, 0, 2);
        assert_eq!(out.len() / 4, 2 * 5807);
        assert_eq!(fnv1a(&out), 0xd11d_1cf6_4661_8a19);
        let v2 = stream_v2(&mut Rng(0x9E37_79B9_7F4A_7C15));
        assert_eq!(fnv1a(&v2), 0x7cdb_5e50_db03_d014);
        let out = decode(&v2, 1, 1);
        assert_eq!(out.len() / 4, 6987);
        assert_eq!(fnv1a(&out), 0xf192_c440_254a_6eaf);
    }

    #[test]
    fn version_2_block_headers() {
        let (size, block) = Layer32Block::read(&[0x42, 0x10]);
        assert_eq!(size, 0x210);
        assert_eq!((block.granule_offset, block.pcm_samples, block.offset_mode), (2, 0, 0));
        let word: u32 = 2 << 30 | 53 << 20 | 36 << 10 | 509;
        let mut data = vec![0x82, 0x93];
        data.extend_from_slice(&word.to_be_bytes());
        let (size, block) = Layer32Block::read(&data);
        assert_eq!(size, 0x293);
        assert_eq!((block.granule_offset, block.pcm_data_offset), (6, 515));
        assert_eq!((block.offset_mode, block.offset_samples, block.pcm_samples), (2, 53, 36));
    }
}
