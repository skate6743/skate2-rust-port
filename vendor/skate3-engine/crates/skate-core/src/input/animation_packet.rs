//! Complete TU3 82DB5BE0: AnimOutPhysIn -> ProcessedPhysIn publication.
//! Called by physical input stage 82DB4048 before board/attribute processing.
//! Inputs are already produced by animation; this is not a gamepad mapping.

/// Source packet fields. Byte flags use their low bit, including noncanonical
/// byte values. Matrix/vector words are copied without numerical conversion.
pub struct AnimationPacketFields {
    /// Packet +10368 -> processed +2468 bit 20.
    pub stance_byte: u8,
    /// Packet +10384 / +10388.
    pub timestep: f32,
    pub scalar_10388: f32,
    /// Packet +10375, +10496, +10784 -> processed +2468 bits 3, 2, 1.
    pub flags_10375_10496_10784: [u8; 3],
    pub vector_10480: [u32; 4],
    pub matrix_10704: [[u32; 4]; 4],
    pub byte_10768: u8,
    /// Packet +10788 / +10792.
    pub truck_tightness: f32,
    pub scalar_10792: f32,
    /// Packet +10371 -> processed +2476 bit 20.
    pub flag_10371: u8,
}

/// All fields written by 82DB5BE0. Seed flag words from the preceding native
/// reset/publication stages: this helper preserves their unrelated bits.
pub struct ProcessedPacketFields {
    pub flags_2468: u32,
    pub flags_2476: u32,
    pub timestep: f32,
    pub scalar_2668: f32,
    pub vector_1520: [u32; 4],
    pub matrix_1536: [[u32; 4]; 4],
    pub byte_1600: u8,
    pub truck_tightness: f32,
    pub scalar_2764: f32,
}

/// Publish the complete function's writes into distinct source/destination
/// records. The source packet must remain valid through physical processing.
pub fn publish(source: &AnimationPacketFields, output: &mut ProcessedPacketFields) {
    replace_bit(&mut output.flags_2468, 20, source.stance_byte);
    output.timestep = source.timestep;
    output.scalar_2668 = source.scalar_10388;
    replace_bit(&mut output.flags_2468, 3, source.flags_10375_10496_10784[0]);
    replace_bit(&mut output.flags_2468, 2, source.flags_10375_10496_10784[1]);
    output.vector_1520 = source.vector_10480;
    replace_bit(&mut output.flags_2468, 1, source.flags_10375_10496_10784[2]);
    output.matrix_1536 = source.matrix_10704;
    output.byte_1600 = source.byte_10768;
    output.truck_tightness = source.truck_tightness;
    output.scalar_2764 = source.scalar_10792;
    replace_bit(&mut output.flags_2476, 20, source.flag_10371);
}

fn replace_bit(word: &mut u32, bit: u32, byte: u8) {
    *word = (*word & !(1 << bit)) | (u32::from(byte & 1) << bit);
}
