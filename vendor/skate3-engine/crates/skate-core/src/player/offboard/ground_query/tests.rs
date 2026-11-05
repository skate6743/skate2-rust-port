use super::*;
fn v(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}
fn context() -> QueryContext {
    QueryContext {
        selection_flags_2948: 0,
        matching_id_2952: -1,
    }
}
fn input() -> ConsumeInput {
    ConsumeInput {
        frame_80: Frame::IDENTITY,
        contact_position_192: Vector3::ZERO,
        contact_flags_368: 0,
        reach_364: 1.,
        previous_input_up_416: v(0.1, 0.9, 0.),
    }
}
fn hit(position: Vector3) -> LineHit {
    LineHit {
        position,
        face_normal: v(0., 1., 0.),
        fraction: 0.5,
        packed_surface: 0,
    }
}
#[test]
fn edge_selection_requires_both_distances_to_improve_in_order() {
    let search = edge_search(Frame::IDENTITY, context(), v(0., 0., 2.), 0);
    let edges =
        [v(0.5, 0.1, 0.), v(0.1, 0.15, 0.), v(0.6, 0.05, 0.)].map(|p| Edge { start: p, end: p });
    let selected = select_edge(search, &edges).unwrap();
    assert_eq!(selected.closest, edges[0].start);
    let reversed = select_edge(search, &[edges[1], edges[0]]).unwrap();
    assert_eq!(reversed.closest, edges[1].start);
}
#[test]
fn biped_packet_has_seventh_downward_line_and_rounded_sixth_line() {
    let selected = EdgeSelection {
        edge: Edge {
            start: v(0., 0., -1.),
            end: v(0., 0., 1.),
        },
        closest: Vector3::ZERO,
    };
    let packet = prepare_packet(Frame::IDENTITY, context(), selected, 0.25).unwrap();
    assert_eq!(packet.lines[0].start, v(0.09, 0.04, 0.));
    assert_eq!(packet.lines[5].radius, 0.001);
    assert_eq!(packet.lines[6].start, v(0., 0., 0.2));
    assert_eq!(packet.lines[6].end, v(0., -6., 0.2));
    assert_eq!(GroundQueryPacket::SOURCE_POOL_MASK, 7);
}
#[test]
fn no_query_preserves_existing_contact_up_but_contact_fallback_runs() {
    let i = input();
    let out = consume_geometry(i, None);
    assert!(!out.state_753);
    assert_eq!(out.input_up_416, i.previous_input_up_416);
    let out = consume_geometry(
        ConsumeInput {
            contact_flags_368: 1,
            ..i
        },
        None,
    );
    assert!(out.state_753);
    assert_eq!(out.input_up_416, v(0., 1., 0.));
    let out = consume_geometry(
        ConsumeInput {
            contact_flags_368: 9,
            ..i
        },
        None,
    );
    assert!(!out.state_753);
}
#[test]
fn reach_and_material_flag_gate_step_without_gating_side_contact() {
    let g = GroundGeometry {
        frame: Frame::IDENTITY,
        kind: 2,
        flag26: true,
        flag27: false,
        flag28: false,
    };
    let out = consume_geometry(
        ConsumeInput {
            reach_364: 0.7,
            ..input()
        },
        Some(g),
    );
    assert!(!out.state_752);
    assert!(!out.state_754);
    let out = consume_geometry(input(), Some(g));
    assert!(out.state_752 && out.state_754);
    let out = consume_geometry(input(), Some(GroundGeometry { flag28: true, ..g }));
    assert!(!out.state_752);
    let out = consume_geometry(
        input(),
        Some(GroundGeometry {
            kind: 1,
            flag28: true,
            ..g
        }),
    );
    assert!(out.state_753);
}
#[test]
fn seventh_hit_height_and_first_hit_material_reach_consumed_flags() {
    let packet = prepare_packet(
        Frame::IDENTITY,
        context(),
        EdgeSelection {
            edge: Edge {
                start: v(0., 0., -1.),
                end: v(0., 0., 1.),
            },
            closest: Vector3::ZERO,
        },
        0.25,
    )
    .unwrap();
    let mut hits = [None; 7];
    hits[0] = Some(LineHit {
        packed_surface: 0x400,
        ..hit(Vector3::ZERO)
    });
    hits[2] = Some(hit(Vector3::ZERO));
    hits[6] = Some(hit(v(0., -1., 0.)));
    let g = interpret_hits(&packet, hits);
    assert_eq!(g.kind, 2);
    assert!(g.flag28 && g.flag26);
    assert!(!g.flag27);
    hits[6] = Some(hit(v(0., -0.9, 0.)));
    assert!(!interpret_hits(&packet, hits).flag26);
}
