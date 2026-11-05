use super::{Edge, EdgeSearch, EdgeSelection, Frame, QueryContext, math::*};
use crate::math::Vector3;
/// 82D32168..321C0 and82D32808. Inputs are the live Biped frame and state1040.
pub fn edge_search(
    frame: Frame,
    context: QueryContext,
    state_vector_1040: Vector3,
    processed_2488: u32,
) -> EdgeSearch {
    let narrow_forward =
        processed_2488 & 0x0800_0000 != 0 || !(dot(state_vector_1040, state_vector_1040) >= 1.);
    let center = if narrow_forward {
        madd(frame.forward, 0.75, frame.position)
    } else {
        frame.position
    };
    let extent = madd(
        abs(frame.forward),
        0.75,
        madd(abs(frame.up), 0.2, scale(abs(frame.right), 0.5)),
    );
    EdgeSearch {
        min: sub(center, extent),
        max: add(center, extent),
        frame,
        context,
        narrow_forward,
    }
}
///82E09D28: finite-segment closest point, including the tiny-edge retain branch.
pub fn closest_point(point: Vector3, edge: Edge) -> Vector3 {
    let mut d = sub(edge.end, edge.start);
    let l = length(d);
    if l > f32::from_bits(0x3780_0000) {
        d = scale(d, reciprocal(l));
    }
    let t = dot(d, sub(point, edge.start)).min(l).max(0.);
    madd(d, t, edge.start)
}
///82D32808: input candidates MUST retain provider order and its shared cap40.
///Both distances must improve; this is not a nearest-distance sort.
pub fn select_edge(search: EdgeSearch, candidates: &[Edge]) -> Option<EdgeSelection> {
    let mut best_vertical = 0.2;
    let mut best_horizontal = 1.5;
    let mut selected = None;
    for &edge in candidates.iter().take(40) {
        let closest = closest_point(search.frame.position, edge);
        let delta = sub(closest, search.frame.position);
        if search.narrow_forward && 0. > dot(search.frame.forward, delta) {
            continue;
        }
        let vertical = dot(search.frame.up, delta);
        let horizontal_distance = length(sub(delta, scale(search.frame.up, vertical)));
        // Native scalar bge rejects ordered >=; unordered does not take bge.
        if !(vertical.abs() >= best_vertical) && !(horizontal_distance >= best_horizontal) {
            best_vertical = vertical.abs();
            best_horizontal = horizontal_distance;
            selected = Some(EdgeSelection { edge, closest });
        }
    }
    selected
}
