//! Common force leaves82D40080/82D3FFC0/82D40130/82D401F8.
//! These consume manager output; they do not duplicate its assistance update.
use super::{V, scale};
use crate::{physics::native_arithmetic::dot3, point_graph::PointGraph};

pub fn pin(normal: V, gravity_relief: f32) -> Option<V> {
    if !(gravity_relief > 0.) {
        return None;
    }
    let mut force = scale(normal, -40.);
    force[1] = 0.;
    Some(force)
}

pub fn coping(upmost: V, velocity: V, gravity_relief: f32) -> Option<V> {
    (gravity_relief > 0. && dot3(upmost, velocity) < 0.).then_some([0., 120., 0., 0.])
}

pub fn exit_lean(high_side: V, angle_1500: f32, graph: &PointGraph<4>) -> Option<V> {
    (angle_1500 > 0.).then(|| scale(high_side, graph.evaluate(angle_1500) * -65.))
}

///Unlike conditional pin/coping, original always calls AddForce, even at0.
pub fn truck_compensation(normal: V, pitch_1508: f32) -> V {
    scale(normal, pitch_1508.abs() * -600.)
}
