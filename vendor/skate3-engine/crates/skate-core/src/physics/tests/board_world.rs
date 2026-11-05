use super::*;
use crate::{
    math::Basis3,
    physics::{
        board::{BODY_COUNT, RETAIL_TRUCK_Y_POSITION, RETAIL_WHEEL_RADIUS},
        board_runtime::BoardMotion,
        board_step::BoardStepSettings,
        collision::TriangleFeature,
        drive_frames::{
            AuthoredTransformInputs, RetailAffineTransform, authored_body_transforms,
            default_truck_transforms,
        },
        drive_parameters::{RetailTruckDriveSettings, retail_truck_drive_dynamics},
        rigid_body::{
            RetailBodyMassProperties, RetailInertiaDynamics, RetailLocalMassFrame,
            RetailSimulationStep,
        },
    },
};

fn material(a: f32, b: f32, c: f32) -> RetailContactMaterial {
    RetailContactMaterial {
        static_friction: a,
        dynamic_friction: b,
        restitution: c,
    }
}
fn triangle() -> WorldTriangle {
    WorldTriangle::from_vertices(
        [
            Vector3::new(-10.0, 0.0, -10.0),
            Vector3::new(0.0, 0.0, 10.0),
            Vector3::new(10.0, 0.0, -10.0),
        ],
        material(0.8, 0.6, 0.2),
        42,
        TriangleFeature::ONE_SIDED | 0xe0,
        [1.0; 3],
        0.0,
    )
    .unwrap()
}
fn config() -> WheelWorldSettings {
    WheelWorldSettings {
        radius: RETAIL_WHEEL_RADIUS,
        material: material(0.2, 0.1, 0.8),
        query: WorldContactSettings {
            volume_padding: 0.0,
            maximum_separating_distance: 0.0,
            edge_cos_bend_normal_threshold: -1.0,
            convexity_epsilon: 0.0,
            is_object: false,
        },
        retention: ContactRetentionSettings {
            capacity: 100,
            duplicate_distance_squared: 0.000001,
            deferred_reduction: false,
        },
    }
}
fn board(gap: f32, gravity: Vector3) -> BoardRuntime {
    let mass = RetailBodyMassProperties {
        local_mass_frame: RetailLocalMassFrame::IDENTITY,
        dynamics: RetailInertiaDynamics {
            inverse_tensor: Vector3::new(3.0, 3.0, 3.0),
            inverse_mass: 0.5,
            spherical: 0.0,
            maximum_linear_velocity: 1000.0,
            maximum_angular_velocity: 1000.0,
            linear_drag: 0.0,
            angular_drag: 0.0,
        },
    };
    BoardRuntime::new(
        [mass; BODY_COUNT],
        authored_body_transforms(AuthoredTransformInputs::STOCK),
        RetailAffineTransform {
            translation: Vector3::new(
                0.0,
                RETAIL_WHEEL_RADIUS - RETAIL_TRUCK_Y_POSITION + gap,
                0.0,
            ),
            ..RetailAffineTransform::IDENTITY
        },
        RetailSimulationStep::fixed_60_hz(30, 0.001, gravity),
        BoardMotion::Active,
    )
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 2e-6, "{a} != {b}");
}

#[test]
fn finite_world_geometry_produces_four_ordered_wheel_contacts_with_combined_materials() {
    let mut world = BoardWorld::new(vec![triangle()]);
    let board = board(0.0, Vector3::ZERO);
    let contacts = world.query(&board, config());
    assert_eq!(contacts.len(), 4);
    for (index, collision) in contacts.iter().enumerate() {
        assert_eq!(collision.body_a, CollisionBody::Board(BodyId::ORDER[index]));
        assert_eq!(collision.body_b, CollisionBody::StaticWorld);
        close(collision.contact.normal.y, 1.0);
        close(collision.contact.position_on_a.y, 0.0);
        close(collision.contact.position_on_b.y, 0.0);
        assert_eq!(collision.contact.static_friction, 0.8);
        assert_eq!(collision.contact.dynamic_friction, 0.6);
        assert_eq!(collision.contact.restitution, 0.2);
        assert_eq!(collision.contact.tag, 42);
    }
}

#[test]
fn contact_query_uses_live_poses_and_rejects_air_remote_geometry_and_backfaces() {
    let mut world = BoardWorld::new(vec![triangle()]);
    let mut board = board(0.0, Vector3::ZERO);
    assert_eq!(world.query(&board, config()).len(), 4);
    for body in board.bodies_mut() {
        body.rates.position.y += 1.0;
    }
    assert!(world.query(&board, config()).is_empty());
    for body in board.bodies_mut() {
        body.rates.position.y -= 1.0;
        body.rates.position.x += 30.0;
    }
    assert!(world.query(&board, config()).is_empty());
    for body in board.bodies_mut() {
        body.rates.position.x -= 30.0;
        body.rates.position.y -= RETAIL_WHEEL_RADIUS * 1.5;
    }
    assert!(world.query(&board, config()).is_empty());
}

#[test]
fn approaching_velocity_padding_and_world_fatness_control_real_contact_acceptance() {
    let mut world = BoardWorld::new(vec![triangle()]);
    let mut board = board(0.01, Vector3::ZERO);
    let mut settings = config();
    settings.query.maximum_separating_distance = 0.05;
    assert!(world.query(&board, settings).is_empty());
    for body in board.bodies_mut() {
        body.rates.linear_velocity.y = -1.0;
    }
    assert_eq!(world.query(&board, settings).len(), 4);
    settings.query.maximum_separating_distance = 0.004;
    assert!(world.query(&board, settings).is_empty());
    // Still approaching, but the limit alone (0.004) no longer covers the
    // 0.01 gap: the volume padding (+112) and the triangle fatness must.
    settings.query.volume_padding = 0.02;
    assert_eq!(world.query(&board, settings).len(), 4);
    // 8277BC58 BE94..BF38 tests the triangle's bare vertex box against the
    // 82777E70 volume box (shape bounds swept along the velocity, no +112
    // padding): a rising volume 0.01 above the plane is culled first.
    for body in board.bodies_mut() {
        body.rates.linear_velocity.y = 1.0;
    }
    assert!(world.query(&board, settings).is_empty());
    for body in board.bodies_mut() {
        body.rates.linear_velocity.y = -1.0;
    }
    settings.query.volume_padding = 0.0;
    let mut thick = triangle();
    thick.triangle.fatness = 0.02;
    let mut thick_world = BoardWorld::new(vec![thick]);
    let contacts = thick_world.query(&board, settings);
    assert_eq!(contacts.len(), 4);
    close(contacts[0].contact.position_on_b.y, 0.02);
}

#[test]
fn duplicate_triangles_and_explicit_contact_capacity_do_not_multiply_solver_rows() {
    let board = board(0.0, Vector3::ZERO);
    let mut world = BoardWorld::new(vec![triangle(), triangle()]);
    assert_eq!(world.query(&board, config()).len(), 4);
    assert_eq!(world.dropped_contacts(), 4);
    let mut settings = config();
    settings.retention.capacity = 3;
    assert_eq!(world.query(&board, settings).len(), 3);
    assert!(world.dropped_contacts() > 0);
    settings.retention.capacity = 100;
    settings.retention.deferred_reduction = true;
    let mut repeated = BoardWorld::new(vec![triangle(); 8]);
    assert_eq!(repeated.query(&board, settings).len(), 4);
}

#[test]
fn world_contacts_feed_supporting_reactions_into_the_persistent_board_solver() {
    let gravity = Vector3::new(0.0, -9.8, 0.0);
    let mut supported = board(0.0, gravity);
    let mut falling = board(0.0, gravity);
    let mut world = BoardWorld::new(vec![triangle()]);
    let collisions = world.query(&supported, config());
    assert_eq!(collisions.len(), 4);
    let settings = BoardStepSettings {
        simulation: RetailSimulationStep::fixed_60_hz(30, 0.001, gravity),
        iterations: 25,
        base_truck_transforms: default_truck_transforms(),
        truck_dynamics: retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK),
        force_point_y_offset: 0.0,
    };
    supported.advance(collisions, [0.0; 2], settings);
    falling.advance(&[], [0.0; 2], settings);
    let momentum = |board: &BoardRuntime| {
        board
            .bodies()
            .iter()
            .map(|body| body.rates.linear_velocity.y / body.inertia.inverse_mass)
            .sum::<f32>()
    };
    assert!(momentum(&supported) > momentum(&falling) + 0.01);
    assert_eq!(
        supported.hook().body.rates.world_inverse_inertia,
        Basis3 {
            columns: [[0.0; 3]; 3]
        }
    );
}

#[test]
fn host_geometry_rejects_degenerate_triangles() {
    assert!(
        WorldTriangle::from_vertices(
            [Vector3::ZERO; 3],
            material(0.0, 0.0, 0.0),
            0,
            0,
            [0.0; 3],
            0.0
        )
        .is_none()
    );
}

#[test]
fn attached_volume_contact_reaches_its_actual_solver_body() {
    use crate::physics::board_step::AttachedStep;
    let mut board = board(0.0, Vector3::ZERO);
    let mut attached = board.bodies()[0];
    attached.rates.position = Vector3::new(0.0, RETAIL_WHEEL_RADIUS, 0.0);
    attached.rates.linear_velocity.y = -1.0;
    let before = attached.rates.linear_velocity.y;
    let volume = BoardWorldVolume {
        collision_group: 0,
        body: CollisionBody::Attached(0),
        primitive: ContactPrimitive::Sphere(Sphere {
            center: attached.rates.position,
            radius: RETAIL_WHEEL_RADIUS,
        }),
        motion: crate::physics::board_world::VolumeMotion {
            linear_velocity: attached.rates.linear_velocity,
            ..Default::default()
        },
        material: material(0.0, 0.0, 0.0),
    };
    let mut world = BoardWorld::new(vec![triangle(), triangle()]);
    let config = config();
    let contacts = world.query_primitives(&[volume], config.query, config.retention);
    assert_eq!(
        contacts.len(),
        1,
        "retention must preserve attached body identity"
    );
    assert_eq!(contacts[0].body_a, CollisionBody::Attached(0));
    let settings = BoardStepSettings {
        simulation: RetailSimulationStep::fixed_60_hz(30, 0.001, Vector3::ZERO),
        iterations: 25,
        base_truck_transforms: default_truck_transforms(),
        truck_dynamics: retail_truck_drive_dynamics(RetailTruckDriveSettings::STOCK),
        force_point_y_offset: 0.0,
    };
    board.advance_attached(
        contacts,
        [0.0; 2],
        settings,
        AttachedStep {
            bodies: vec![&mut attached],
            contacts: &mut [],
            joints: &mut [],
            drives: &mut [],
        },
    );
    assert!(attached.rates.linear_velocity.y > before + 0.5);
    assert!(
        board.contact_reports().is_empty(),
        "skater contacts are not board observations"
    );
}
