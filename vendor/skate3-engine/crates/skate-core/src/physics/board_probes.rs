//! Deck and wall probes in TU3 82C07788/82C079E0, with the actual optional
//! wall-line producer82C01F10. Wheel probes remain in board_ground.
use super::{
    board_ground::WheelLine,
    board_motion_output::{dot, inverse_length_squared},
};
use crate::math::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardProbeHit {
    pub point: Vector3,
    pub normal: Vector3,
    pub surface_tag: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardProbeState {
    pub start: Vector3,
    pub point: Vector3,
    pub normal: Vector3,
    pub surface_tag: u32,
    pub hit: bool,
}
impl Default for BoardProbeState {
    fn default() -> Self {
        // SkateboardBody ctor82C06614..82C0675C clears both complete records.
        Self {
            start: Vector3::ZERO,
            point: Vector3::ZERO,
            normal: Vector3::ZERO,
            surface_tag: 0,
            hit: false,
        }
    }
}
impl BoardProbeState {
    pub fn start(&mut self, position: Vector3) {
        self.start = position;
        self.hit = false;
    }
    /// A miss only clears the hit byte. Previously published point, normal and
    /// surface deliberately remain, unlike a disabled optional query.
    pub fn publish(&mut self, hit: Option<BoardProbeHit>) {
        self.hit = hit.is_some();
        if let Some(hit) = hit {
            self.point = hit.point;
            self.normal = hit.normal;
            self.surface_tag = hit.surface_tag;
        }
    }
    pub fn disable(&mut self) {
        *self = Self::default();
    }
}

pub const DECK_PROBE_RADIUS: f32 = f32::from_bits(0x3e19_999a);
/// Fifth query always runs from the physical deck origin100m down, radius0.15.
/// Native start is copied to body7968, and result to7984/8000/8016/8020.
pub fn deck_probe(position: Vector3) -> WheelLine {
    WheelLine {
        start: position,
        end: Vector3::new(
            position.x,
            position.y + f32::from_bits(0xc2c8_0000),
            position.z,
        ),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WallLineInput {
    /// Processed2508: current physical state (100 or101 admitted).
    pub state: u32,
    /// Processed464,544,112,2752.
    pub contact_normal: Vector3,
    pub skater_up: Vector3,
    pub deck_position: Vector3,
    pub time: f32,
}
/// Source82C01F10 clears endpoints208/224 on every call, then enables byte291
/// only for the three physical gates below. No guessed grounded condition.
pub fn wall_probe(input: WallLineInput) -> Option<WheelLine> {
    if !matches!(input.state, 100 | 101)
        || !(input.contact_normal.y.abs() < 0.5)
        || !(dot(input.skater_up, input.contact_normal) > f32::from_bits(0x3f35_c28f))
        || !(input.time > f32::from_bits(0x3c23_d70a))
    {
        return None;
    }
    let normal = input.contact_normal;
    let start = Vector3::new(
        normal.x.mul_add(0.05, input.deck_position.x),
        normal.y.mul_add(0.05, input.deck_position.y),
        normal.z.mul_add(0.05, input.deck_position.z),
    );
    let first = cross(Vector3::new(0., 1., 0.), normal);
    let down = cross(first, normal);
    let squared = dot(down, down);
    let inverse = inverse_length_squared(squared, 2);
    let magnitude = if squared == 0. { 0. } else { squared * inverse };
    let down = if magnitude > f32::from_bits(0x3586_37bd) {
        Vector3::new(down.x * inverse, down.y * inverse, down.z * inverse)
    } else {
        Vector3::ZERO
    };
    Some(WheelLine {
        start,
        end: Vector3::new(
            down.x.mul_add(4., start.x),
            down.y.mul_add(4., start.y),
            down.z.mul_add(4., start.z),
        ),
    })
}
fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wall_probe_gates_and_miss_retention_follow_source() {
        let mut input = WallLineInput {
            state: 100,
            contact_normal: Vector3::new(1., 0., 0.),
            skater_up: Vector3::new(1., 0., 0.),
            deck_position: Vector3::new(2., 3., 4.),
            time: 0.02,
        };
        let line = wall_probe(input).unwrap();
        assert_eq!(line.start, Vector3::new(2.05, 3., 4.));
        assert_eq!(line.end, Vector3::new(2.05, -1., 4.));
        input.state = 200;
        assert!(wall_probe(input).is_none());
        let mut output = BoardProbeState::default();
        output.start(line.start);
        output.publish(Some(BoardProbeHit {
            point: line.end,
            normal: Vector3::new(0., 1., 0.),
            surface_tag: 7,
        }));
        output.start(line.start);
        output.publish(None);
        assert!(!output.hit);
        assert_eq!(output.point, line.end);
        assert_eq!(output.surface_tag, 7);
        output.disable();
        assert_eq!(output, BoardProbeState::default());
    }
}
