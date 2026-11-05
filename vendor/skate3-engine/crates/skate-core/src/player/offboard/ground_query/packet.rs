use super::{EdgeSelection, Frame, GroundQueryPacket, Line, QueryContext, math::*};
use crate::math::Vector3;
///82D321D0..32320 then82C20728: all seven Biped lines, in source order.
pub fn prepare_packet(
    frame: Frame,
    context: QueryContext,
    selected: EdgeSelection,
    deck_half_wheelbase: f32,
) -> Option<GroundQueryPacket> {
    let edge = selected.edge;
    let closest = selected.closest;
    let reverse = safe_unit(sub(edge.start, edge.end), frame.right);
    let forward = safe_unit(
        sub(frame.forward, scale(reverse, dot(reverse, frame.forward))),
        frame.forward,
    );
    let seventh_start = madd(forward, 0.2, closest);
    let delta = sub(edge.end, edge.start);
    let raw_up = cross(delta, cross(Vector3::new(0., 1., 0.), delta));
    let up = safe_unit(raw_up, raw_up);
    let tangent = safe_unit(delta, delta);
    if f32::from_bits(0x3727_c5ac) > length(raw_up) {
        return None;
    }
    let projection = madd(tangent, dot(sub(closest, edge.start), tangent), edge.start);
    let center = sub(closest, sub(closest, projection));
    let side = cross(up, tangent);
    let short_up = scale(up, 0.04);
    let near = scale(side, 0.09);
    let far = scale(side, deck_half_wheelbase);
    let far_up = scale(up, deck_half_wheelbase * f32::from_bits(0x3f87_ae14));
    let line = |c: Vector3, d: Vector3, radius| Line {
        start: add(c, d),
        end: sub(c, d),
        radius,
    };
    Some(GroundQueryPacket {
        context,
        center,
        up,
        tangent,
        lines: [
            line(add(center, near), short_up, 0.),
            line(sub(center, near), short_up, 0.),
            line(add(center, far), far_up, 0.),
            line(sub(center, far), far_up, 0.),
            line(center, short_up, 0.),
            line(add(center, scale(up, 0.09)), near, 0.001),
            Line {
                start: seventh_start,
                end: add(seventh_start, Vector3::new(0., -6., 0.)),
                radius: 0.,
            },
        ],
    })
}
