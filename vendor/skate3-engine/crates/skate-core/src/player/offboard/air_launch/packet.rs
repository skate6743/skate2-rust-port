use super::Vector;
/// Native118-byte logical packet. W lanes and the two trailing bytes are real
/// fields; this host struct makes no promise about the C ABI or native padding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Packet {
    pub velocity_0: Vector,
    pub secondary_velocity_16: Vector,
    pub position_32: Vector,
    pub up_48: Vector,
    pub forward_64: Vector,
    pub board_position_80: Vector,
    pub scalar_96: f32,
    pub scalar_100: f32,
    pub scalar_104: f32,
    pub kind_108: u32,
    pub kind_112: u32,
    pub has_board_position_116: bool,
    pub flag_117: bool,
}
impl Packet {
    ///82D2DB98 does not write104; the caller supplies its retained value.
    /// The launch producer subsequently overwrites104 in every mode.
    pub fn initialized(retained_104: f32) -> Self {
        Self {
            velocity_0: [0.0; 4],
            secondary_velocity_16: [0.0; 4],
            position_32: [0.0; 4],
            up_48: [0.0, 1.0, 0.0, 0.0],
            forward_64: [0.0; 4],
            board_position_80: [0.0; 4],
            scalar_96: 0.0,
            scalar_100: 0.0,
            scalar_104: retained_104,
            kind_108: 1,
            kind_112: 0,
            has_board_position_116: false,
            flag_117: false,
        }
    }
}
