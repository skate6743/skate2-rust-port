//! Contact projection/classification82D81F80. No synthesized floor samples.
use super::{Input, Vector, dot, length, scale, sub};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactSample {
    pub position: Vector,
    pub normal: Vector,
    pub forward_distance: f32,
    pub height: f32,
    pub flags: u32,
    pub sort_distance: f32,
}
#[derive(Clone, Debug, Default)]
pub struct Samples {
    pub ground: Vec<ContactSample>,   //10832, capacity64
    pub other: Vec<ContactSample>,    //14944
    pub original_ground_count: usize, //14932
}
impl Samples {
    pub fn insert(
        &mut self,
        input: Input,
        position: Vector,
        normal: Vector,
        category: u32,
        sort_override: f32,
        provenance: u32,
    ) -> bool {
        let delta = sub(position, input.position);
        let forward = dot(input.forward, delta);
        let height = dot(input.up, delta);
        let high = height >= 0.79000002 && forward >= 0.;
        let normal = sub(normal, scale(input.right, dot(normal, input.right)));
        let magnitude = length(normal);
        let normal = if magnitude >= 0.001 {
            scale(normal, reciprocal(magnitude))
        } else if height < 0. {
            input.forward
        } else {
            scale(input.forward, -1.)
        };
        let ground = category == 1 || high;
        if ground && self.ground.len() == 64 || !ground && forward < 0. {
            return false;
        }
        let mut flags = match provenance {
            1 => 1,
            2 => 4,
            _ => 0,
        };
        if high {
            flags |= 8;
        }
        if category == 2 {
            flags |= 0x10;
        }
        let sample = ContactSample {
            position,
            normal,
            forward_distance: forward,
            height,
            flags,
            sort_distance: if sort_override >= 0. {
                sort_override
            } else {
                forward
            },
        };
        if ground {
            self.ground.push(sample);
            self.original_ground_count = self.ground.len();
        } else {
            self.other.push(sample);
        }
        true
    }
}
fn reciprocal(value: f32) -> f32 {
    let mut r = crate::physics::native_arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(value, 1.), r);
    }
    r
}
