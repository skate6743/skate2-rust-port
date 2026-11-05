//! Shared scalar types. Rendering conversions stay outside the core.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl Default for Vector3 {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Basis3 {
    pub columns: [[f32; 3]; 3],
}
