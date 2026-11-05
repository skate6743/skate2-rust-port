use super::*;
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId, collision::Sphere, contact::RetailContactMaterial,
        skeleton_body::SkeletonCollisionSettings, world_contact::ContactPrimitive,
    },
};

fn mode() -> SkeletonCollisionMode {
    let mut mode = SkeletonCollisionMode::new_normal(
        SkeletonCollisionSettings {
            enabled: true,
            normal_material: material(),
            compliant: [false; 24],
            priority: [0.0; 24],
            effect_time: 0.0,
        },
        true,
    );
    mode.select_driven(3).unwrap();
    mode
}
fn material() -> RetailContactMaterial {
    RetailContactMaterial {
        static_friction: 0.5,
        dynamic_friction: 0.3,
        restitution: 0.0,
    }
}
fn volume(body: CollisionBody, x: f32) -> BoardWorldVolume {
    BoardWorldVolume {
        body,
        primitive: ContactPrimitive::Sphere(Sphere {
            center: Vector3::new(x, 0.0, 0.0),
            radius: 0.5,
        }),
        linear_velocity: Vector3::ZERO,
        material: material(),
    }
}

#[test]
fn board_pairs_use_actual_biped_part_groups() {
    let mode = mode();
    assert_eq!(mode.assembly_group, 6);
    let board = [volume(CollisionBody::Board(BodyId::Deck), 0.0)];
    // The same overlapping geometry must not collide merely because its
    // containing assembly is6. Carried board4 excludes all these part groups.
    for part in [3, 7, 15, 19, 16, 20, 23, 24, 25] {
        let rider = [volume(CollisionBody::Attached(part), 0.25)];
        let mut contacts = Vec::new();
        append(&mut contacts, &board, &rider, 4, &mode).unwrap();
        assert!(contacts.is_empty(), "carried board contacted part{part}");
    }
    // Released board7 can contact the original leg group17, but not torso18
    // or controller20. Do not disable all board/rider collision as a fix.
    for (part, expected) in [(15, true), (19, true), (23, false), (24, false)] {
        let mut contacts = Vec::new();
        append(
            &mut contacts,
            &board,
            &[volume(CollisionBody::Attached(part), 0.25)],
            7,
            &mode,
        )
        .unwrap();
        assert_eq!(!contacts.is_empty(), expected, "released board part{part}");
    }
}

#[test]
fn assembly_gate_and_world_query_group_are_distinct() {
    let mut mode = mode();
    let board = [volume(CollisionBody::Board(BodyId::Deck), 0.0)];
    let rider = [volume(CollisionBody::Attached(15), 0.25)];
    mode.parts[15].volume_group = 1; // World-query group is not VolumeData212.
    let mut contacts = Vec::new();
    append(&mut contacts, &board, &rider, 7, &mode).unwrap();
    assert!(!contacts.is_empty());
    contacts.clear();
    mode.assembly_group = 1;
    append(&mut contacts, &board, &rider, 7, &mode).unwrap();
    assert!(contacts.is_empty());
}

#[test]
fn original_single_player_table_keeps_special_groups_and_symmetry() {
    for a in 0..21 {
        for b in 0..21 {
            assert_eq!(group_pair_allowed(a, b), group_pair_allowed(b, a));
        }
    }
    assert!(!group_pair_allowed(4, 5));
    assert!(!group_pair_allowed(4, 17));
    assert!(group_pair_allowed(7, 17));
    assert!(group_pair_allowed(5, 20));
    assert!(!group_pair_allowed(7, 20));
    assert!(group_pair_allowed(11, 11));
    assert!(!group_pair_allowed(11, 12));
    assert!(group_pair_allowed(12, 12));
    assert!(group_pair_allowed(13, 13));
    assert!(!group_pair_allowed(21, 0));
}
