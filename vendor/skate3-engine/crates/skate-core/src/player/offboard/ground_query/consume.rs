use super::{
    ConsumeInput, Frame, GroundAdjustment, GroundGeometry, GroundQueryPacket, LineHit, math::*,
};
use crate::math::Vector3;
///82C20C08 fields directly consumed by82D31620; calls82C21558/82C1E170.
pub fn interpret_hits(packet: &GroundQueryPacket, hits: [Option<LineHit>; 7]) -> GroundGeometry {
    let h = |i: usize| hits[i].is_some();
    let mut side = scale(
        cross(Vector3::new(0., 1., 0.), packet.tangent),
        inverse_root(dot(
            cross(Vector3::new(0., 1., 0.), packet.tangent),
            cross(Vector3::new(0., 1., 0.), packet.tangent),
        )),
    );
    let kind = if (h(0) && h(1)) || h(4) {
        3
    } else if h(0) {
        if hits[2].is_some_and(|v| !(v.fraction >= 0.65)) {
            2
        } else {
            1
        }
    } else if h(1) {
        side = scale(side, -1.);
        if hits[3].is_some_and(|v| !(v.fraction >= 0.65)) {
            2
        } else {
            1
        }
    } else {
        0
    };
    let flag28 = hits[0].is_some_and(|v| v.packed_surface & 0x0f80 == 0x0400)
        || hits[1].is_some_and(|v| v.packed_surface & 0x0f80 == 0x0400);
    let (flag26, flag27) = if let Some(hit) = hits[6] {
        let delta = sub(hit.position, packet.center);
        let d = dot(hit.face_normal, safe_unit(packet.tangent, Vector3::ZERO));
        let rejected = if d > 0. {
            sub(hit.face_normal, scale(packet.tangent, d))
        } else {
            hit.face_normal
        };
        let normal = safe_unit(rejected, Vector3::ZERO);
        (
            !(delta.y > -1.),
            0.9 > normal.y && dot(horizontal(delta), normal) > 0.,
        )
    } else {
        (true, false)
    };
    GroundGeometry {
        frame: Frame {
            right: side,
            up: packet.up,
            forward: packet.tangent,
            position: packet.center,
        },
        kind,
        flag26,
        flag27,
        flag28,
    }
}
///82D31620 including its PreUpdate resets82D30EAC..82D30F0C. None means the
///previous query was not pending; contact fallback still executes.
pub fn consume_geometry(input: ConsumeInput, geometry: Option<GroundGeometry>) -> GroundAdjustment {
    let mut out = GroundAdjustment {
        state_752: false,
        state_753: false,
        state_754: false,
        frame_768: Frame::IDENTITY,
        input_up_416: input.previous_input_up_416,
    };
    let mut nearby = false;
    if let Some(g) = geometry {
        let delta = sub(g.frame.position, input.frame_80.position);
        let dh = horizontal(delta);
        nearby = 0.3 > delta.y.abs()
            && f32::from_bits(0x3d23_d70b) > dot(dh, dh)
            && dot(g.frame.forward, input.frame_80.forward).abs() > 0.9;
        if !g.flag28 && input.reach_364 > length(dh) + 0.7 {
            out.state_752 = g.kind == 2 && g.frame.up.y > 0.98;
            out.state_754 = g.flag26 || g.flag27;
        }
        if g.kind == 0 || g.kind == 1 {
            out.state_753 = !(dot(delta, g.frame.right).abs() > 0.06)
                && !(input.contact_flags_368 & 0x10 != 0 && input.contact_flags_368 & 0x20 != 0);
        }
        if out.state_752 || out.state_753 {
            out.frame_768 = g.frame;
        }
    }
    if !out.state_753 && input.contact_flags_368 & 1 != 0 && input.contact_flags_368 & 0x38 == 0 {
        let h = dot(
            sub(input.frame_80.position, input.contact_position_192),
            input.frame_80.up,
        );
        // Native bge/ble leave unordered values on the accepting path.
        if !(h >= 0.06) && !(h <= -0.02) {
            out.state_753 = true;
            out.frame_768 = input.frame_80;
            out.frame_768.position = input.contact_position_192;
        }
    }
    if nearby || out.state_753 {
        let x = horizontal(out.frame_768.right);
        let x_length = length(x);
        if !(x_length <= 0.001) {
            let x = scale(x, reciprocal(x_length));
            let z = sub(
                out.frame_768.forward,
                scale(x, dot(out.frame_768.forward, x)),
            );
            let z_length = length(z);
            if !(z_length <= 0.001) {
                let z = scale(z, reciprocal(z_length));
                out.frame_768.right = x;
                out.frame_768.forward = z;
                out.frame_768.up = cross(z, x);
            }
        }
        if 0. > out.frame_768.up.y {
            out.frame_768.up = scale(out.frame_768.up, -1.);
            out.frame_768.right = scale(out.frame_768.right, -1.);
        }
        out.input_up_416 = out.frame_768.up;
    }
    out
}
