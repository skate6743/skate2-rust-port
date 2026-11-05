//! Physical body indices and stock geometry used by the TU3 board assembly.
pub const BODY_COUNT: usize = 7;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BodyId {
    RightFrontWheel = 0,
    LeftFrontWheel = 1,
    RightBackWheel = 2,
    LeftBackWheel = 3,
    FrontTruck = 4,
    BackTruck = 5,
    Deck = 6,
}
impl BodyId {
    pub const ORDER: [Self; BODY_COUNT] = [
        Self::RightFrontWheel,
        Self::LeftFrontWheel,
        Self::RightBackWheel,
        Self::LeftBackWheel,
        Self::FrontTruck,
        Self::BackTruck,
        Self::Deck,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }
}

pub const RETAIL_WHEEL_RADIUS: f32 = f32::from_bits(0x3CFD_F3B6);
pub const RETAIL_TRUCK_Y_POSITION: f32 = f32::from_bits(0xBD67_6C8B);
pub const RETAIL_TRUCK_Z_POSITION_BACK: f32 = f32::from_bits(0xBD54_FDF4);
pub const RETAIL_TRUCK_Z_POSITION_FRONT: f32 = f32::from_bits(0xBD54_FDF4);
pub const RETAIL_TRUCK_ROTATION_AXIS_ANGLE_DEGREES: f32 = f32::from_bits(0x41F8_0000);
pub const RETAIL_DECK_MID_LENGTH: f32 = f32::from_bits(0x3F17_0A3D);
