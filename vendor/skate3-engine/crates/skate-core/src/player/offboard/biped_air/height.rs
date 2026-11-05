//! Original TU3 82D2FF50 and 82D2FDD0, recovered from disassembly.
use super::{DT, Vector};
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    a[2].mul_add(b[2], a[1].mul_add(b[1], a[0] * b[0]))
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn madd(a: Vector, b: f32, c: Vector) -> Vector {
    core::array::from_fn(|i| a[i].mul_add(b, c[i]))
}
pub(super) fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
///Translations returned by82BE3220 for bones15/19, in that order.
pub fn projected_height(bone15: Vector, bone19: Vector, reference: Vector, up: Vector) -> f32 {
    let a = dot(up, sub(bone15, reference));
    let b = dot(up, sub(bone19, reference));
    select(a - b, b, a)
}
#[derive(Clone, Copy, Debug)]
pub struct HeightInput {
    pub bone15: Vector,
    pub bone19: Vector,
    ///82BE3170(skeleton,1) translation.
    pub bone1: Vector,
    pub up_544: Vector,
    pub velocity_608: Vector,
}
impl super::State {
    pub fn correct_height(&mut self, input: HeightInput) {
        let height = projected_height(
            input.bone15,
            input.bone19,
            self.result.position_272,
            input.up_544,
        );
        let limit = (dot(input.up_544, input.velocity_608) * DT).abs() + 0.05;
        let delta = (height + 0.79) - self.height_432;
        let lower = select(-limit - delta, -limit, delta);
        self.height_432 += select(limit - lower, lower, limit);
        self.body_target_416 = madd(input.up_544, self.height_432, self.result.position_272);
        let predicted = madd(self.result.velocity_288, DT, input.bone1);
        let lowered = madd(input.up_544, -0.4, predicted);
        self.body_offset_436 = dot(sub(lowered, self.body_target_416), input.up_544);
    }
}
