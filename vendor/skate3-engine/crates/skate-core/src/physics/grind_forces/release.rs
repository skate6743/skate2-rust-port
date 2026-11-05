//! Exit nudge82D3F850 / S2 82D85330. Family dispatch remains separate.
use super::{V, dot3, scale, sub};

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub position: V,
    pub point: V,
    pub across: V,
    pub normal: V,
    pub velocity: V,
    pub geometry_kind: u32,
    pub high_side: V,
    pub force_across: bool,
    /// Original r7. It is independent of r8/force_across; slides suppress lift.
    pub enable_lift: bool,
    pub strength: f32,
    pub speed_limit: f32,
    pub lift: f32,
}

/// None means no AddForce call, including no accumulator wakeup.
pub fn force(input: Input) -> Option<V> {
    let outward = if input.force_across || input.geometry_kind == 0 {
        scale(
            input.across,
            if dot3(input.across, sub(input.position, input.point)) > 0.0 {
                1.0
            } else {
                -1.0
            },
        )
    } else {
        scale(input.high_side, -1.0)
    };
    if !(dot3(input.velocity, outward) < input.speed_limit) {
        return None;
    }
    let lateral = scale(outward, input.strength);
    if !input.enable_lift {
        return Some(lateral);
    }
    let upward = input.normal[1].max(0.0) * input.lift;
    Some(core::array::from_fn(|i| {
        input.normal[i].mul_add(upward, lateral[i])
    }))
}
