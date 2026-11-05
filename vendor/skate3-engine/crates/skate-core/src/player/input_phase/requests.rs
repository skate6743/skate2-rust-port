use super::types::RawVector;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundHistoryRequest {
    pub previous_position: RawVector,
    pub current_position: RawVector,
    pub previous_filtered_delta: RawVector,
    /// Processed +2604. The native block also consumes constants at
    /// `0x82072818` and `0x8231A844`; the service owns those exact values.
    pub timestep_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundHistoryResult {
    pub delta: RawVector,
    pub filtered_delta: RawVector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrepareJumpRequest {
    pub skateboard_vector: RawVector,
    pub previous_velocity: RawVector,
}
