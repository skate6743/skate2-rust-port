//! Solid Rapier/native solver boundary. Native bone/board primitives remain
//! native; convex objects are tested with their actual Parry support shapes.
use super::{Skate3PhysicsState, Skate3ControllerState};
use crate::skate3_native::math_host::*;
use skate3_core::{math::Vector3, physics::{
    board_step::{BoardCollision, CollisionBody}, board_world::BoardWorldVolume,
    contact::{RetailContactInput, RetailContactMaterial, combine_contact_materials},
    world_contact::ContactPrimitive,
}};
use skate_dynamics::{SolidBody, rapier3d::{prelude::{SharedShape, Pose, Rotation, Vector}}};

pub(crate) fn vector(v: Vector3) -> Vector { Vector::new(v.x, v.y, v.z) }

/// TU3's assembly pair culling (8277AF90) and then each primitive pair's by its part group
/// (8277A6C0), between a skater volume and a host body of `group` (Skate 3 numbering).
fn culled(volume: &BoardWorldVolume, group: u32, physics: &Skate3PhysicsState, skater: &Skate3ControllerState) -> bool {
    let assembly = match volume.body {
        CollisionBody::Board(_) => physics.board.collision_group(),
        CollisionBody::Attached(_) => skater.skeleton_collision.assembly_group,
        CollisionBody::StaticWorld => return true,
    };
    let allowed = |own| super::collision_groups::pair_allowed_online(own, group, physics.network_active);
    !(allowed(assembly) && allowed(volume.collision_group))
}
fn native(v: Vector) -> Vector3 { Vector3::new(v.x, v.y, v.z) }

pub(crate) fn shape(primitive: ContactPrimitive) -> Option<(SharedShape, Pose)> {
    let (shape, center, rotation) = match primitive {
        ContactPrimitive::Sphere(s) => (SharedShape::ball(s.radius), s.center, Quat::IDENTITY),
        ContactPrimitive::Capsule { center, axis, half_length, radius } => {
            let axis = Vec3::new(axis.x, axis.y, axis.z).try_normalize()?;
            (SharedShape::capsule_y(half_length, radius), center, Quat::from_rotation_arc(Vec3::Y, axis))
        }
        ContactPrimitive::RoundedBox { center, basis, half_extents, radius } => (
            SharedShape::round_cuboid(half_extents.x, half_extents.y, half_extents.z, radius),
            center, Quat::from_mat3(&Mat3::from_cols_array_2d(&basis.columns)).normalize(),
        ),
        ContactPrimitive::Triangle(_) => return None,
    };
    Some((shape, Pose::from_parts(vector(center), Rotation::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w))))
}

pub(crate) fn append(
    contacts: &mut Vec<BoardCollision>, volumes: &[BoardWorldVolume],
    bodies: &[(usize, SolidBody)], physics: &Skate3PhysicsState, skater: &Skate3ControllerState,
) {
    let dt = physics.settings.step.simulation.time_step;
    for a in volumes {
        // The board's kicktail triangles meet only PC Object Dropper boards (as
        // fat triangles): a wedged or nose-on board otherwise slid into the deck.
        let (kicktail, (shape_a, pose_a)) = match a.primitive {
            ContactPrimitive::Triangle(t) => {
                let [p, q, r] = t.vertices.map(vector);
                (true, (SharedShape::round_triangle(p, q, r, t.fatness), Pose::IDENTITY))
            }
            primitive => match shape(primitive) {
                Some(shape) => (false, shape),
                None => continue,
            },
        };
        let native_body = match a.body {
            CollisionBody::Board(id) => physics.board.bodies().get(id.index()),
            CollisionBody::Attached(index) => skater.skeleton.bodies().get(index),
            CollisionBody::StaticWorld => None,
        };
        let Some(native_body) = native_body else { continue };
        for (index, body) in bodies {
            // Mesh bodies contact through append_meshes.
            if physics.world_proxies.meshes.iter().any(|(mesh, _)| mesh == index) { continue; }
            if kicktail { continue; }
            if culled(a, body.contact_group, physics, skater) { continue; }
            for collider in &body.colliders {
                let Some(c) = body.contact(collider, &*shape_a, pose_a,
                    vector(native_body.rates.position), vector(native_body.rates.linear_velocity),
                    vector(native_body.rates.angular_velocity), dt, 0.025) else { continue };
                let material = combine_contact_materials(a.material, RetailContactMaterial {
                    static_friction: collider.friction,
                    dynamic_friction: collider.friction * 0.75, restitution: 0.,
                });
                contacts.push(BoardCollision {
                    body_a: a.body, body_b: CollisionBody::Attached(*index),
                    contact: RetailContactInput {
                        position_on_a: native(c.point_a), position_on_b: native(c.point_b),
                        normal: native(c.normal), restitution: material.restitution,
                        static_friction: material.static_friction,
                        dynamic_friction: material.dynamic_friction, tag: 0,
                    },
                });
            }
        }
    }
}

/// Moving bodies with an authored mesh: the native GroundPipeline triangle
/// dispatcher, GP edge features, packed surfaces and contact retention that
/// static ground uses, run on the mesh at the body's current placement, with
/// each volume's motion taken relative to the body. Rows join the shared
/// solve against the body (Attached), so the board rolls up a kicker with no
/// box seams AND pushes it. Only meshes near the volumes are queried.
pub(crate) fn append_meshes(
    contacts: &mut Vec<BoardCollision>, volumes: &[BoardWorldVolume], physics: &Skate3PhysicsState,
    skater: &Skate3ControllerState, query: skate3_core::physics::collision::WorldContactSettings,
) {
    use skate3_core::physics::board_world::{BoardWorld, VOLUME_QUERY_SCALE, VOLUME_QUERY_STEP, volume_query_bounds};
    let meshes = &physics.world_proxies.meshes;
    if volumes.is_empty() || meshes.is_empty() { return; }
    let padding = query.volume_padding.max(0.) + query.maximum_separating_distance.max(0.) + 0.1;
    for (index, mesh) in meshes {
        let group = physics.world_proxies.groups.get(index).copied().unwrap_or(0);
        let relative: Vec<_> = volumes.iter().filter(|volume| !culled(volume, group, physics, skater)).map(|volume| {
            let mut volume = *volume;
            let v = &mut volume.motion.linear_velocity;
            *v = Vector3::new(v.x - mesh.velocity.x, v.y - mesh.velocity.y, v.z - mesh.velocity.z);
            volume
        }).collect();
        // Unbounded volume boxes keep the mesh: the per-triangle test decides.
        if relative.is_empty() { continue; }
        let near = relative.iter().any(|volume| {
            volume_query_bounds(volume.primitive, volume.motion, VOLUME_QUERY_STEP, VOLUME_QUERY_SCALE)
                .is_none_or(|bounds| bounds.expanded(padding).overlaps(mesh.bounds))
        });
        if !near { continue; }
        let mut world = BoardWorld::new(mesh.triangles.triangles().to_vec());
        contacts.extend(world.query_primitives(&relative, query, physics.retention).iter().map(|row| {
            BoardCollision { body_b: CollisionBody::Attached(*index), ..*row }
        }));
    }
}

/// Last-mile constraint for animation-driven walking. Native walking may write
/// its root without integrating a Rapier body; a full-height capsule sweep keeps
/// that displacement from bypassing the same solid obstacle used by BoardStep.
/// Only mod solids participate. Stock floor and movement algorithms are intact.
fn constrained_position(solids:&[(usize,SolidBody)],start:Vec3,target:Vec3) -> Vec3 {
    use skate_dynamics::rapier3d::parry::query::{self,ShapeCastOptions};
    if solids.is_empty() || !start.is_finite() || !target.is_finite() { return target; }
    const SKIN:f32=0.015;
    let capsule=SharedShape::capsule_y(0.55,0.28);
    let mut position=Vector::from_array(start.to_array());
    let mut remaining=Vector::from_array((target-start).to_array());
    // Deep overlap recovery, including a body spawned around a walking actor.
    // It uses the closed convex volume, not triangle front faces.
    for _ in 0..8 {
        let pose=Pose::from_translation(position);
        let mut deepest:Option<(f32,Vector)>=None;
        for (_,body) in solids {
            for collider in &body.colliders {
                if let Ok(Some(c))=query::contact(&pose,&*capsule,&collider.pose,&*collider.shape,SKIN) {
                    if c.dist<0. && deepest.as_ref().is_none_or(|(d,_)|c.dist<*d) {
                        deepest=Some((c.dist,c.normal2));
                    }
                }
            }
        }
        let Some((distance,normal))=deepest else { break };
        if !normal.is_finite() || !distance.is_finite() { break; }
        position+=normal*(-distance+SKIN);
    }
    for _ in 0..5 {
        if remaining.length_squared()<1e-10 { break; }
        let pose=Pose::from_translation(position);
        let mut nearest:Option<(f32,Vector)>=None;
        for (_,body) in solids {
            for collider in &body.colliders {
                let options=ShapeCastOptions { max_time_of_impact:1.,target_distance:SKIN,
                    stop_at_penetration:true,compute_impact_geometry_on_penetration:true };
                if let Ok(Some(hit))=query::cast_shapes(&pose,remaining,&*capsule,&collider.pose,Vector::ZERO,&*collider.shape,options) {
                    let normal=collider.pose.rotation*hit.normal2;
                    // Contacts behind us cannot block movement away/tangential.
                    if normal.dot(remaining) >= -1e-7 { continue; }
                    if nearest.as_ref().is_none_or(|(t,_)|hit.time_of_impact<*t) {
                        nearest=Some((hit.time_of_impact,normal));
                    }
                }
            }
        }
        let Some((time,normal))=nearest else { position+=remaining; break };
        position+=remaining*time.clamp(0.,1.)+normal*0.001;
        remaining*=1.-time.clamp(0.,1.);
        remaining-=normal*remaining.dot(normal).min(0.);
    }
    Vec3::from_array(position.to_array())
}

pub(super) fn constrain_ground(
    owner:&mut super::biped_ground::Owner,
    result:&mut skate3_core::player::offboard::controller::GroundResult,
    previous:[f32;4],solids:&[(usize,SolidBody)],
) {
    let start=Vec3::new(previous[0],previous[1],previous[2]);
    let requested=Vec3::new(result.position[0],result.position[1],result.position[2]);
    let delta=constrained_position(solids,start,requested)-requested;
    if delta.length_squared()<1e-12 || !delta.is_finite() { return; }
    let shift=|v:&mut [f32;4]| { v[0]+=delta.x;v[1]+=delta.y;v[2]+=delta.z; };
    shift(&mut result.position);
    shift(&mut result.physical_frame[3]);
    shift(&mut result.animation_frame[3]);
    let state=&mut owner.controller.state;
    shift(&mut state.position_368);
    shift(&mut state.motion.frame_0[3]);
    shift(&mut state.motion.published_frame_64[3]);
    shift(&mut state.frame_output.frame[3]);
    shift(&mut state.correction_target_592);
    owner.result=Some(*result);
    // Do not replace the actor's velocity with obstacle velocity here: native
    // contact reports must see the real relative impact speed for wipeout.
}

#[cfg(any())]
mod tests {
    use super::*;
    fn obstacle() -> Vec<(usize,SolidBody)> {
        let mut world=skate_dynamics::DynamicsWorld::default();
        world.spawn(skate_dynamics::BodyDesc { shape:skate_dynamics::Shape::Box { half_extents:[0.5,2.,2.] },..Default::default() }).unwrap();
        world.solid_bodies().into_iter().enumerate().collect()
    }
    #[test] fn a_walk_step_cannot_cross_the_whole_object() {
        let end=constrained_position(&obstacle(),Vec3::new(-3.,0.,0.),Vec3::new(3.,0.,0.));
        assert!(end.x < -0.77);
    }
    #[test] fn walking_away_is_not_blocked() {
        let end=constrained_position(&obstacle(),Vec3::new(-0.80,0.,0.),Vec3::new(-2.,0.,0.));
        assert!((end.x+2.).abs()<1e-4);
    }
    #[test] fn deep_inside_start_is_recovered() {
        let end=constrained_position(&obstacle(),Vec3::ZERO,Vec3::ZERO);
        assert!(end.x.abs() > 0.77);
    }
}
