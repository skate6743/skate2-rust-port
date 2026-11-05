//! Original Biped82D7E2A8 surface normal and82D7E840 spring/lean stages.
//! Only these stages' written fields are retained here; all vectors keep VMX w.
mod math;
use math::*;
pub type Vector = [f32; 4];
const UP: Vector = [0.0, 1.0, 0.0, 0.0];

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub surface_normal: Vector, //544
    pub source_normal: Vector,  //560
    pub spring_normal: Vector,  //416
    pub spring_delta: Vector,   //432
    pub final_up: Vector,       //448
    pub lean: Vector,           //464
}
impl Default for State {
    fn default() -> Self {
        Self {
            surface_normal: UP,
            source_normal: UP,
            spring_normal: UP,
            spring_delta: [0.0; 4],
            final_up: UP,
            lean: [0.0; 4],
        }
    }
}
pub struct SurfaceInput {
    pub flags: u32,          //input176
    pub normal: Vector,      //input16
    pub origin: Vector,      //input0
    pub edge_point: Vector,  //input128
    pub edge_normal: Vector, //input144
}
pub struct SpringInput {
    pub velocity: Vector,       //Biped480
    pub up: Vector,             //Biped16
    pub right: Vector,          //Biped0
    pub forward: Vector,        //Biped32
    pub spring_right: Vector,   //Biped128
    pub spring_forward: Vector, //Biped160
    pub right_delta: f32,       //Biped696
    pub forward_delta: f32,     //Biped692
    pub suppress_lean: bool,    //input317
}
impl State {
    pub fn update_surface(&mut self, input: &SurfaceInput) {
        if input.flags & 1 == 0 {
            return;
        }
        self.source_normal = input.normal;
        let mut target = input.normal;
        if input.flags & 4 != 0 {
            let delta = sub(input.edge_point, input.origin);
            let right = normalize_or(cross(UP, delta), [0.0; 4]);
            let candidate = normalize_or(cross(delta, right), input.normal);
            let limit = 45.0 * f32::from_bits(0x3c8e_fa35);
            let first = clamp_angle(candidate, input.normal, limit);
            let second = clamp_angle(input.edge_normal, input.normal, limit);
            let lateral = scale(right, dot(right, input.normal));
            target = normalize_or(sub(input.normal, lateral), UP);
            if first[1] > target[1] {
                target = first;
            }
            if second[1] > target[1] {
                target = second;
            }
            let remaining = clamp(1.0 - dot(lateral, lateral), 0.0, 1.0);
            target = madd(target, square_root(remaining), lateral);
        }
        let angle = wrap_angle(unsigned_angle(self.surface_normal, target)).abs();
        if angle < f32::from_bits(0x3f49_0fdb) || target[1] > f32::from_bits(0x3f35_c28f) {
            self.surface_normal = normalize_or(
                madd(self.surface_normal, 0.5, scale(target, 0.5)),
                self.surface_normal,
            );
        }
    }

    pub fn update_spring(&mut self, input: &SpringInput) {
        let blend = clamp(length(input.velocity) * 0.125, 0.0, 1.0);
        let candidate = madd(self.surface_normal, blend, scale(UP, 1.0 - blend));
        let limited = clamp_angle(
            candidate,
            self.surface_normal,
            36.0 * f32::from_bits(0x3c8e_fa35),
        );
        let target = normalize_or(limited, input.up);
        let desired_delta = scale(sub(target, self.spring_normal), f32::from_bits(0x3e4c_cccd));
        let desired_delta = clamp_axis(
            desired_delta,
            input.spring_right,
            f32::from_bits(0xbccc_cccd),
            f32::from_bits(0x3ccc_cccd),
        );
        let acceleration = sub(desired_delta, self.spring_delta);
        let acceleration = clamp_axis(
            acceleration,
            input.spring_right,
            f32::from_bits(0xbbc4_9ba6),
            f32::from_bits(0x3bc4_9ba6),
        );
        let acceleration = clamp_axis(
            acceleration,
            input.spring_forward,
            f32::from_bits(0xbd4c_cccd),
            f32::from_bits(0x3d4c_cccd),
        );
        self.spring_delta = add(self.spring_delta, acceleration);
        self.spring_normal = normalize_or(
            add(self.spring_normal, self.spring_delta),
            self.spring_normal,
        );
        let lean_target = if input.suppress_lean {
            [0.0; 4]
        } else {
            madd(
                input.forward,
                input.forward_delta * f32::from_bits(0x3d23_d70a),
                scale(input.right, input.right_delta * f32::from_bits(0x3f33_3333)),
            )
        };
        self.lean = madd(
            lean_target,
            f32::from_bits(0x3d4c_cccd),
            scale(self.lean, f32::from_bits(0x3f73_3333)),
        );
        self.final_up = normalize_or(add(self.lean, self.spring_normal), self.spring_normal);
    }
}
#[cfg(test)]
mod tests;
