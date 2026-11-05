//! Direct stock Andale bank storage and format descriptions.
//!
//! Source: TU3 DataBase::SetDBContent82D1B340, Clip::Init827B8AB0,
//! FetchSys::BatchFetch82D20560/82D20788 and codec identifier82D17A60.
//! This module owns bytes and bounds checks; codec arithmetic belongs to core.
mod bank;
mod reader;
pub use bank::Bank;
pub use reader::Reader;
use std::{fmt, ops::Range};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub offset: usize,
    pub message: String,
}
impl Error {
    pub fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ABIN at {:#x}: {}", self.offset, self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    Raw,
    Vbr,
    Other(u32),
}
impl Codec {
    ///82D17A60 reverses the disk word and compares RD\0\0 and VBR\0.
    pub fn from_word(word: u32) -> Self {
        match word {
            0x0000_4452 => Self::Raw,
            0x0052_4256 => Self::Vbr,
            v => Self::Other(v),
        }
    }
}
#[derive(Clone, Debug)]
pub struct RecordHeader {
    pub name: String,
    pub name_words: [u32; 6],
    pub offset: usize,
    pub size: usize,
    pub payload_offset: usize,
    pub type_id: u32,
    pub codec_word: u32,
    pub guid: u32,
}
impl RecordHeader {
    pub fn codec(&self) -> Codec {
        Codec::from_word(self.codec_word)
    }
    pub fn range(&self) -> Range<usize> {
        self.offset..self.offset + self.size
    }
}
#[derive(Clone, Debug)]
pub struct Record {
    pub header: RecordHeader,
    pub data: RecordData,
}
#[derive(Clone, Debug)]
pub enum RecordData {
    Clip(Clip),
    Pose(Pose),
    Hierarchy(Hierarchy),
    PhysicsPose(PhysicsPose),
    Opaque,
}
#[derive(Clone, Debug)]
pub struct Clip {
    pub fps_bits: u32,
    pub frame_count_bits: u32,
    pub base_speed_bits: u32,
    pub flags: u32,
    pub loop_translation_words: [u32; 4],
    pub loop_rotation_words: [u32; 4],
    pub attribute_offset: usize,
    pub attribute_count: u8,
    pub parts: Vec<PartEntry>,
}
impl Clip {
    pub fn looping(&self) -> bool {
        self.flags & 0x1000_0000 != 0
    }
    pub fn channel_animation(&self) -> bool {
        self.flags & 0x2000_0000 != 0
    }
    pub fn phase_controlled(&self) -> bool {
        self.flags & 0x4000_0000 != 0
    }
}
#[derive(Clone, Debug)]
pub struct Pose {
    pub parts: Vec<PartEntry>,
}
#[derive(Clone, Debug)]
pub struct PhysicsPose {
    pub bone_count: u32,
    ///24 authored112-byte PhysicsParamBoneData records in the stock banks.
    pub records: Range<usize>,
}
#[derive(Clone, Debug)]
pub struct PartEntry {
    pub table_index: usize,
    pub table_offset: usize,
    pub raw_word: u32,
    pub encoded_id: u8,
    ///An authored zero offset remains an empty entry; no table compaction.
    pub part: Option<AnimationPart>,
}
/// Common original AnimationPart header for RAW and VBR. Relative words and
/// absolute byte ranges coexist so consumers never accidentally mix bases.
#[derive(Clone, Debug)]
pub struct AnimationPart {
    pub offset: usize,
    pub header_word: u32,
    pub compressed_size: u32,
    pub compression_header_relative: u32,
    pub compressed_data_relative: u32,
    ///Header bits6..11 plus1, verified against all stock hierarchy parts.
    pub channel_count: u16,
    ///Packed header bits12..31 give the byte count at compression_header_relative.
    pub compression_header: Range<usize>,
    pub compressed_data: Range<usize>,
    ///Original channel-animation weight words begin at part+16.
    pub channel_weights: Option<Range<usize>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyPart {
    pub name: String,
    pub bone_count: u32,
    pub sqt_offset: i32,
    ///Retained original words; no runtime part-id rewriting is performed.
    pub flags: u16,
    pub encoded_id: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hierarchy {
    pub offset: usize,
    pub bone_count: u16,
    pub has_trajectory: bool,
    pub parents: Vec<i32>,
    pub mirrors: Vec<i32>,
    pub bone_names: Vec<String>,
    pub parts: Vec<HierarchyPart>,
}
impl Hierarchy {
    pub fn compatible_with(&self, other: &Self) -> bool {
        self.bone_count == other.bone_count
            && self.has_trajectory == other.has_trajectory
            && self.parents == other.parents
            && self.mirrors == other.mirrors
            && self.bone_names == other.bone_names
            && self.parts == other.parts
    }
}
