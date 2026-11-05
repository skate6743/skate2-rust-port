use skate_core::animation::output::{
    attributes::{
        AnimationAttribute, AttributeName, AttributePayload, MotionGraphAttribute, PacketAttributes,
    },
    motion_graph_packet::{self, GestureData, MotionGraphPacket, TrickData, TrickIdentifier},
};
fn attribute(kind: u8, seed: u32) -> AnimationAttribute {
    AnimationAttribute {
        payload: AttributePayload(std::array::from_fn(|i| Some(seed + i as u32))),
        begin_time: 2.0,
        end_time: 3.0,
        name: AttributeName([seed; 5]),
        status: 9,
        kind,
        sequence_id: 17,
    }
}
#[test]
fn assignment_copies_only_kind_selected_union_lanes_and_all_metadata() {
    for kind in [0, 1, 2, 3, 4, 255] {
        let source = attribute(kind, 100);
        let mut destination = attribute(3, 200);
        destination.copy_from(&source);
        let count = match kind {
            0 | 2 => 1,
            1 => 4,
            3 => 6,
            _ => 0,
        };
        for lane in 0..6 {
            assert_eq!(
                destination.payload.0[lane],
                Some(if lane < count { 100 } else { 200 } + lane as u32)
            );
        }
        assert_eq!(destination.kind, kind);
        assert_eq!(destination.name, source.name);
        assert_eq!(destination.begin_time, 2.0);
        assert_eq!(destination.end_time, 3.0);
        assert_eq!(destination.status, 9);
        assert_eq!(destination.sequence_id, 17);
    }
}
#[test]
fn motion_graph_conversion_preserves_scalar_bits_without_fabricating_other_lanes() {
    for bits in [0x80000000, 0x7fc12345, 0x3f800000] {
        let source = MotionGraphAttribute {
            name: AttributeName([10, 20, 30, 40, 50]),
            value: f32::from_bits(bits),
        };
        let result = source.to_animation();
        assert_eq!(result.payload.0, [Some(bits), None, None, None, None, None]);
        assert_eq!(result.name, source.name);
        assert_eq!((result.begin_time, result.end_time), (-1.0, -1.0));
        assert_eq!((result.status, result.kind, result.sequence_id), (6, 0, -1));
    }
}
#[test]
fn merge_resets_active_length_preserves_duplicates_and_keeps_mg_before_tree() {
    let name = AttributeName([1, 2, 3, 4, 5]);
    let mg = [
        MotionGraphAttribute { name, value: 0.0 },
        MotionGraphAttribute { name, value: 2.0 },
    ];
    let mut tree = [attribute(0, 7), attribute(1, 8)];
    tree[0].name = name;
    let mut list = PacketAttributes::default();
    list.append(&attribute(3, 500));
    list.replace_from(&mg, &tree);
    assert_eq!(list.entries().len(), 4);
    assert_eq!(list.entries()[0].name, name);
    assert_eq!(list.entries()[1].name, name);
    assert_eq!(list.entries()[2].name, name);
    assert_eq!(list.entries()[0].payload.0[0], Some(0));
    assert_eq!(list.entries()[1].payload.0[0], Some(2.0f32.to_bits()));
    assert_eq!(list.entries()[2].payload.0[0], Some(7));
    assert_eq!(list.entries()[3].kind, 1);
    list.replace_from(&[], &[]);
    assert!(list.entries().is_empty());
    list.append(&attribute(0, 900));
    assert_eq!(
        list.entries()[0].payload.0,
        [
            Some(900),
            Some(501),
            Some(502),
            Some(503),
            Some(504),
            Some(505)
        ]
    );
}
fn motion(seed: u32) -> MotionGraphPacket {
    MotionGraphPacket {
        trick: TrickData {
            identifiers: [TrickIdentifier([seed; 6]), TrickIdentifier([seed + 1; 6])],
            word48: seed + 2,
            scalars52_56: [f32::from_bits(0x80000000), f32::from_bits(0x7fc12345)],
        },
        gesture: GestureData {
            gesture: seed + 3,
            flag_bytes: [1, 2, 0xaa, 0xbb],
        },
        wipeout_gesture: [4.0, -5.0],
    }
}
#[test]
fn concrete_motion_graph_publication_preserves_source_including_gesture_and_trick() {
    let source = motion(10);
    let mut destination = motion(90);
    motion_graph_packet::publish(&source, &mut destination);
    assert_eq!(destination.trick.identifiers, source.trick.identifiers);
    assert_eq!(destination.trick.word48, 12);
    assert_eq!(
        destination.trick.scalars52_56.map(f32::to_bits),
        [0x80000000, 0x7fc12345]
    );
    assert_eq!(destination.gesture, source.gesture);
    assert_eq!(destination.wipeout_gesture, [4.0, -5.0]);
    assert_eq!(source.gesture.gesture, 13);
    assert_eq!(source.wipeout_gesture, [4.0, -5.0]);
    destination.gesture.gesture = 37;
    motion_graph_packet::publish(&source, &mut destination);
    assert_eq!(destination.gesture.gesture, 13);
}
