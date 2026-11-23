//! PC port addition (custom songs): a standard MPEG audio layer III stream (an MP3 file's frames)
//! through the game's own layer III decoder.
//!
//! EA's layer III streams hold the same granules a standard frame does, repacked: each granule
//! alone behind a one-byte header (version, sample rate, mode and mode extension, two bits each),
//! its number, its side information (a second granule's led by the frame's scale factor selection)
//! and each channel's part 2 and 3 bits, with no bit reservoir. [`Mp3Stream`] reads standard
//! frames (MPEG-1, MPEG-2 and MPEG-2.5), keeps the reservoir, and hands [`Layer3Core`] each granule
//! repacked so.

use std::sync::Arc;

use super::core::{Layer3Core, GRANULE, SCRATCH};
use crate::rwac::decoder::DataRef;

/// Layer III bit rates in kbit/s by the header's index: MPEG-1, then MPEG-2 and MPEG-2.5.
const BIT_RATES: [[u32; 15]; 2] =
    [[0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320], [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160]];
/// MPEG-1 sample rates by the header's index; MPEG-2 halves them and MPEG-2.5 quarters them.
const SAMPLE_RATES: [u32; 3] = [44100, 48000, 32000];
/// How far back a frame's main data can begin, in bytes (`main_data_begin` is nine bits).
const RESERVOIR: usize = 511;

/// A frame header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Header {
    /// The header's version bits: 3 MPEG-1, 2 MPEG-2, 0 MPEG-2.5.
    version: u8,
    /// A 16-bit CRC follows the header.
    crc: bool,
    rate_index: u8,
    sample_rate: u32,
    mode: u8,
    mode_ext: u8,
    /// The whole frame's bytes.
    length: usize,
}

impl Header {
    /// The layer III frame header at the start of `bytes`: the 11-bit sync, a version that is not
    /// reserved, layer III, and a bit rate (free format is not read) and sample rate.
    fn parse(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..4)?;
        if b[0] != 0xFF || b[1] & 0xE0 != 0xE0 {
            return None;
        }
        let version = (b[1] >> 3) & 3;
        if version == 1 || (b[1] >> 1) & 3 != 1 {
            return None;
        }
        let bit_rate_index = usize::from(b[2] >> 4);
        let rate_index = (b[2] >> 2) & 3;
        if bit_rate_index == 0 || bit_rate_index == 15 || rate_index == 3 {
            return None;
        }
        let lsf = version != 3;
        let bit_rate = BIT_RATES[usize::from(lsf)][bit_rate_index] * 1000;
        let sample_rate = SAMPLE_RATES[usize::from(rate_index)] >> match version {
            3 => 0,
            2 => 1,
            _ => 2,
        };
        let padding = usize::from((b[2] >> 1) & 1);
        let length = (if lsf { 72 } else { 144 }) * bit_rate as usize / sample_rate as usize + padding;
        Some(Self { version, crc: b[1] & 1 == 0, rate_index, sample_rate, mode: b[3] >> 6, mode_ext: (b[3] >> 4) & 3, length })
    }

    /// MPEG-2 and MPEG-2.5: one granule a frame.
    const fn lsf(&self) -> bool {
        self.version != 3
    }

    const fn channels(&self) -> usize {
        if self.mode == 3 { 1 } else { 2 }
    }

    /// The side information's bytes.
    const fn side_info(&self) -> usize {
        match (self.lsf(), self.channels()) {
            (false, 1) => 17,
            (false, _) => 32,
            (true, 1) => 9,
            (true, _) => 17,
        }
    }

    /// Where the side information starts in the frame.
    const fn side_info_at(&self) -> usize {
        if self.crc { 6 } else { 4 }
    }

    /// The same stream: version and sample rate.
    fn follows(&self, other: &Self) -> bool {
        self.version == other.version && self.sample_rate == other.sample_rate
    }
}

/// `count` bits (at most 32) of `bytes` from bit `at`, most significant first; bits past the end
/// read 0.
fn bits(bytes: &[u8], at: usize, count: usize) -> u32 {
    let mut value = 0u32;
    for bit in at..at + count {
        let set = bytes.get(bit / 8).is_some_and(|byte| byte & (0x80 >> (bit % 8)) != 0);
        value = (value << 1) | u32::from(set);
    }
    value
}

/// A granule being repacked, most significant bit first.
#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}

impl BitWriter {
    fn push(&mut self, bit: bool) {
        if self.bits % 8 == 0 {
            self.bytes.push(0);
        }
        if bit && let Some(last) = self.bytes.last_mut() {
            *last |= 0x80 >> (self.bits % 8);
        }
        self.bits += 1;
    }

    fn put(&mut self, value: u32, count: usize) {
        for bit in (0..count).rev() {
            self.push((value >> bit) & 1 != 0);
        }
    }

    /// `count` bits of `source` from bit `at`.
    fn copy(&mut self, source: &[u8], at: usize, count: usize) {
        for bit in at..at + count {
            self.push(source.get(bit / 8).is_some_and(|byte| byte & (0x80 >> (bit % 8)) != 0));
        }
    }
}

/// The length of an ID3v2 tag at the start of `data` (its ten-byte header, its body and a footer
/// when it has one); 0 without a tag.
fn id3v2_length(data: &[u8]) -> usize {
    let Some(head) = data.get(..10).filter(|head| head.starts_with(b"ID3")) else { return 0 };
    let size = head[6..10].iter().fold(0usize, |size, &byte| (size << 7) | usize::from(byte & 0x7F));
    10 + size + if head[5] & 0x10 != 0 { 10 } else { 0 }
}

/// An MP3 file's sound, a frame at a time.
pub struct Mp3Stream {
    data: Arc<[u8]>,
    /// Where the next frame is looked for.
    position: usize,
    core: Layer3Core,
    scratch: Box<[f32; SCRATCH]>,
    /// The main data of the frames read, back as far as a frame can reach.
    reservoir: Vec<u8>,
    /// The decoder has taken a granule (its first resets the synthesis).
    started: bool,
    /// The first frame's header, which every later frame follows.
    first: Header,
    pub sample_rate: u32,
    pub channels: u8,
}

impl Mp3Stream {
    /// The stream of `data`: past an ID3v2 tag, the first frame another of the same stream
    /// follows (or that ends the file), and past it when it is the encoder's Xing, Info or VBRI
    /// frame, which holds no sound. `None` for data without one.
    #[must_use]
    pub fn open(data: Arc<[u8]>) -> Option<Self> {
        let start = id3v2_length(&data).min(data.len());
        let (mut position, first) = Self::sync(&data, start)?;
        let side = position + first.side_info_at() + first.side_info();
        let tag = |at: usize| data.get(at..at + 4);
        if matches!(tag(side), Some(b"Xing" | b"Info")) || tag(position + 36) == Some(&b"VBRI"[..]) {
            position += first.length;
        }
        let mut core = Layer3Core::new(2);
        core.standard_stream = true;
        Some(Self {
            data,
            position,
            core,
            scratch: Box::new([0.0; SCRATCH]),
            reservoir: Vec::new(),
            started: false,
            first,
            sample_rate: first.sample_rate,
            channels: first.channels() as u8,
        })
    }

    /// The first frame at or after `from` that the next frame of the same stream follows, or
    /// that reaches the end of the data.
    fn sync(data: &[u8], from: usize) -> Option<(usize, Header)> {
        (from..data.len().saturating_sub(3)).find_map(|at| {
            let header = Header::parse(&data[at..])?;
            if header.length < header.side_info_at() + header.side_info() {
                return None;
            }
            let next = at + header.length;
            let followed = match data.get(next..) {
                Some(rest) if rest.len() >= 4 => Header::parse(rest).is_some_and(|other| other.follows(&header)),
                Some(_) => true,
                None => false,
            };
            followed.then_some((at, header))
        })
    }

    /// The next frame's header and where it starts: at the position when a frame of the stream
    /// is there, else the next one found (a tag or damage between frames is passed over).
    fn next_frame(&self) -> Option<(usize, Header)> {
        let here = self.data.get(self.position..).and_then(Header::parse).filter(|header| header.follows(&self.first));
        let (at, header) = match here {
            Some(header) => (self.position, header),
            None => {
                let (at, header) = Self::sync(&self.data, self.position)?;
                if !header.follows(&self.first) {
                    return None;
                }
                (at, header)
            }
        };
        (at + header.side_info_at() + header.side_info() <= self.data.len()).then_some((at, header))
    }

    /// One frame's samples (1152 a channel, 576 for MPEG-2 and MPEG-2.5), interleaved 16-bit in
    /// the stream's channels, onto `out`. False at the end of the stream.
    pub fn decode_frame(&mut self, out: &mut Vec<i16>) -> bool {
        let Some((at, header)) = self.next_frame() else { return false };
        let data = Arc::clone(&self.data);
        let end = (at + header.length).min(data.len());
        let frame = &data[at..end];
        self.position = at + header.length;
        let side_at = header.side_info_at();
        let Some(side) = frame.get(side_at..side_at + header.side_info()) else { return false };
        let main = frame.get(side_at + header.side_info()..).unwrap_or(&[]);
        let (lsf, channels) = (header.lsf(), header.channels());
        let main_data_begin = bits(side, 0, if lsf { 8 } else { 9 }) as usize;
        // A frame that reaches back past what was read (the first after a cut) has no sound.
        let reachable = main_data_begin <= self.reservoir.len();
        let mut main_bit = 8 * self.reservoir.len().saturating_sub(main_data_begin);
        self.reservoir.extend_from_slice(main);
        // After main_data_begin: the private bits, then MPEG-1's scale factor selection.
        let mut side_bit = match (lsf, channels) {
            (false, 1) => 9 + 5,
            (false, _) => 9 + 3,
            (true, 1) => 8 + 1,
            (true, _) => 8 + 2,
        };
        let scfsi_at = side_bit;
        if !lsf {
            side_bit += 4 * channels;
        }
        let granule_bits = if lsf { 63 } else { 59 };
        let granules: u32 = if lsf { 1 } else { 2 };
        for granule in 0..granules {
            let mut writer = BitWriter::default();
            writer.put(u32::from(header.version) << 6 | u32::from(header.rate_index) << 4 | u32::from(header.mode) << 2 | u32::from(header.mode_ext), 8);
            writer.put(granule, 1);
            if !lsf && granule == 1 {
                writer.copy(side, scfsi_at, 4 * channels);
            }
            let mut lengths = [0usize; 2];
            for length in lengths.iter_mut().take(channels) {
                *length = bits(side, side_bit, 12) as usize;
                writer.copy(side, side_bit, granule_bits);
                side_bit += granule_bits;
            }
            for &length in lengths.iter().take(channels) {
                writer.copy(&self.reservoir, main_bit, length);
                main_bit += length;
            }
            let mut pcm = [[0.0f32; GRANULE]; 2];
            if reachable {
                let granule = DataRef { bytes: Arc::from(writer.bytes), offset: 0 };
                if self.started {
                    self.core.resume(&granule);
                } else {
                    self.core.open(&granule);
                    self.started = true;
                }
                let (left, right) = pcm.split_at_mut(1);
                let mut outputs: Vec<&mut [f32]> = vec![&mut left[0][..], &mut right[0][..]];
                self.core.decode(&mut self.scratch, &mut outputs);
            }
            let sample = |value: f32| value.round().clamp(-32768.0, 32767.0) as i16;
            for index in 0..GRANULE {
                let left = pcm[0][index];
                let right = if channels == 2 { pcm[1][index] } else { left };
                if self.channels == 2 {
                    out.push(sample(left));
                    out.push(sample(right));
                } else {
                    out.push(sample((left + right) * 0.5));
                }
            }
        }
        let keep = self.reservoir.len().saturating_sub(RESERVOIR);
        self.reservoir.drain(..keep);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 128 kbit/s 44.1 kHz joint stereo MPEG-1 frame header, unpadded.
    const FRAME: [u8; 4] = [0xFF, 0xFB, 0x90, 0x64];

    #[test]
    fn a_frame_header_gives_its_stream_and_length() {
        let header = Header::parse(&FRAME).unwrap();
        assert_eq!((header.version, header.sample_rate, header.mode, header.mode_ext, header.length), (3, 44100, 1, 2, 417));
        assert!(!header.crc && !header.lsf());
        assert_eq!((header.channels(), header.side_info(), header.side_info_at()), (2, 32, 4));
        // MPEG-2 at 22.05 kHz, 64 kbit/s, mono, padded, with a CRC.
        let low = Header::parse(&[0xFF, 0xF2, 0x82, 0xC0]).unwrap();
        assert_eq!((low.version, low.sample_rate, low.length, low.channels(), low.side_info(), low.side_info_at()), (2, 22050, 209, 1, 9, 6));
        // Layer II, a free-format bit rate and a reserved sample rate are not frames.
        assert_eq!(Header::parse(&[0xFF, 0xFD, 0x90, 0x64]), None);
        assert_eq!(Header::parse(&[0xFF, 0xFB, 0x00, 0x64]), None);
        assert_eq!(Header::parse(&[0xFF, 0xFB, 0x9C, 0x64]), None);
    }

    #[test]
    fn bits_copy_across_bytes() {
        let source = [0b1010_1100, 0b0101_0011];
        assert_eq!(bits(&source, 3, 7), 0b011_0001);
        assert_eq!(bits(&source, 12, 8), 0b0011_0000, "bits past the end read 0");
        let mut writer = BitWriter::default();
        writer.put(0b101, 3);
        writer.copy(&source, 3, 7);
        assert_eq!((writer.bits, writer.bytes.as_slice()), (10, &[0b1010_1100, 0b0100_0000][..]));
    }

    #[test]
    fn the_stream_starts_past_a_tag_and_the_encoder_frame() {
        let mut data = b"ID3\x04\0\0\0\0\0\x05tagxx".to_vec();
        let frame = |tag: &[u8]| {
            let mut frame = vec![0u8; 417];
            frame[..4].copy_from_slice(&FRAME);
            frame[36..36 + tag.len()].copy_from_slice(tag);
            frame
        };
        data.extend(frame(b"Info"));
        data.extend(frame(b""));
        data.extend(frame(b""));
        let mut stream = Mp3Stream::open(Arc::from(data)).unwrap();
        assert_eq!((stream.position, stream.sample_rate, stream.channels), (15 + 417, 44100, 2));
        // Two silent frames of 1152 stereo samples, then the end.
        let mut out = Vec::new();
        assert!(stream.decode_frame(&mut out));
        assert!(stream.decode_frame(&mut out));
        assert!(!stream.decode_frame(&mut out));
        assert_eq!(out.len(), 2 * 2 * 1152);
        assert!(out.iter().all(|&sample| sample == 0));
        assert!(Mp3Stream::open(Arc::from(vec![0u8; 4096])).is_none());
    }
}
