//! Bind stock child geometry to the live solver part transforms.
use super::settings::PhysicsSettings;
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        board_step::CollisionBody,
        board_world::{BoardWorldVolume, VolumeMotion},
        collision::Sphere,
        contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
        mass::{DeckShape, MassShape},
        world_contact::{ContactPrimitive, transform_triangle_volume},
    },
};

pub(crate) fn world_volumes(
    board: &BoardRuntime,
    settings: &PhysicsSettings,
) -> Vec<BoardWorldVolume> {
    let poses = board.part_transforms();
    let mut volumes = Vec::new();
    // SerializeVolumes preserves part order, then compound child order.
    for id in BodyId::ORDER {
        let body = &board.bodies()[id.index()];
        if body.state_flags == 1 {
            continue;
        }
        let pose = poses[id.index()];
        let mut add = |primitive, material: RetailContactMaterial| {
            volumes.push(BoardWorldVolume {
                collision_group: board.collision_group(),
                body: CollisionBody::Board(id),
                primitive,
                material,
                motion: VolumeMotion::of(&body.rates),
            });
        };
        match id {
            BodyId::Deck => {
                for child in &settings.deck_geometry.children {
                    if !child.collision_enabled {
                        continue;
                    }
                    let world = compose(pose, child.transform);
                    let primitive = match child.shape {
                        DeckShape::Sphere { radius } => ContactPrimitive::Sphere(Sphere {
                            center: world.translation,
                            radius,
                        }),
                        DeckShape::Capsule {
                            radius,
                            half_length,
                        } => capsule(world, radius, half_length),
                        DeckShape::RoundedBox {
                            half_extents,
                            radius,
                        } => ContactPrimitive::RoundedBox {
                            center: world.translation,
                            basis: world.basis,
                            half_extents,
                            radius,
                        },
                        DeckShape::Triangle {
                            vertices,
                            fatness,
                            edge_cosines,
                            volume_flags,
                        } => ContactPrimitive::Triangle(transform_triangle_volume(
                            vertices,
                            fatness,
                            edge_cosines,
                            volume_flags,
                            world.basis,
                            world.translation,
                        )),
                    };
                    add(primitive, settings.deck_material);
                }
            }
            BodyId::FrontTruck | BodyId::BackTruck => {
                if settings.truck_collisions {
                    let MassShape::Capsule {
                        radius,
                        half_length,
                    } = settings.truck_shape
                    else {
                        unreachable!("TU382C0A6F8 constructs a capsule for each truck");
                    };
                    add(capsule(pose, radius, half_length), settings.truck_material);
                }
            }
            _ => add(
                ContactPrimitive::Sphere(Sphere {
                    center: pose.translation,
                    radius: settings.wheel_radius,
                }),
                settings.wheel_material,
            ),
        }
    }
    volumes
}

fn capsule(pose: RetailAffineTransform, radius: f32, half_length: f32) -> ContactPrimitive {
    let z = pose.basis.columns[2];
    ContactPrimitive::Capsule {
        center: pose.translation,
        axis: Vector3::new(z[0], z[1], z[2]),
        half_length,
        radius,
    }
}

// Same multiply/add order as the Volume-to-world vertex path82ADE468.
fn transform(basis: Basis3, point: Vector3, origin: Vector3) -> Vector3 {
    let translation = [origin.x, origin.y, origin.z];
    let component = |row| {
        point.z.mul_add(
            basis.columns[2][row],
            point.y.mul_add(
                basis.columns[1][row],
                point.x.mul_add(basis.columns[0][row], translation[row]),
            ),
        )
    };
    Vector3::new(component(0), component(1), component(2))
}
fn compose(parent: RetailAffineTransform, child: RetailAffineTransform) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: Basis3 {
            columns: child.basis.columns.map(|c| {
                // CreateGP82AD8758 starts basis composition with a product;
                // only position composition includes an initial translation.
                core::array::from_fn(|row| {
                    c[2].mul_add(
                        parent.basis.columns[2][row],
                        c[1].mul_add(
                            parent.basis.columns[1][row],
                            c[0] * parent.basis.columns[0][row],
                        ),
                    )
                })
            }),
        },
        translation: transform(parent.basis, child.translation, parent.translation),
    }
}
