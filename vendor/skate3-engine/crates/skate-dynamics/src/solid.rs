//! Immutable, solid-shape boundary for another physics solver. A convex hull is
//! never converted into a one-sided triangle shell here. All coordinates are
//! world-space unless explicitly marked local. One record represents ONE body.
use super::*;
use rapier3d::parry::query::{self, NonlinearRigidMotion, ShapeCastOptions};

/// Reserved membership used by native-player mirrors; independent of the
/// original game's numeric contact/assembly groups.
pub const PLAYER_MEMBERSHIP: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtraColliderDefinition {
    pub points: Vec<[f32; 3]>,
    pub translation: [f32; 3],
    pub friction: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyDefinition {
    #[serde(default)]
    pub deformation: Option<crate::deformation::Field>,
    pub body: BodyDesc,
    pub extras: Vec<ExtraColliderDefinition>,
}

#[derive(Clone)]
pub struct SolidCollider {
    pub shape: SharedShape,
    pub pose: Pose,
    pub friction: f32,
}

#[derive(Clone)]
pub struct SolidBody {
    pub id: u64,
    pub pose: Pose,
    pub center_of_mass: Vector,
    /// Principal inertia frame, not necessarily the visual/body frame.
    pub inertia_rotation: Rotation,
    pub inverse_mass: f32,
    pub inverse_inertia: Vector,
    pub linvel: Vector,
    pub angvel: Vector,
    pub contact_group: u32,
    pub colliders: Vec<SolidCollider>,
}

#[derive(Clone, Copy, Debug)]
pub struct SolidContact {
    pub point_a: Vector,
    pub point_b: Vector,
    /// Out of B, toward A: positive impulse acts on A along this vector.
    pub normal: Vector,
    pub distance: f32,
    pub time_of_impact: f32,
}

impl SolidBody {
    /// Exact separation/penetration, followed by a full-step translational AND
    /// rotational sweep. The native solver consumes current-frame witness
    /// points, so the speculative separation stays in its own bias equation.
    pub fn contact(
        &self,
        collider: &SolidCollider,
        shape_a: &dyn rapier3d::parry::shape::Shape,
        pose_a: Pose,
        com_a: Vector,
        linear_a: Vector,
        angular_a: Vector,
        dt: f32,
        skin: f32,
    ) -> Option<SolidContact> {
        if !(dt.is_finite() && dt > 0. && skin.is_finite() && skin >= 0.) {
            return None;
        }
        let bounds_a = shape_a.compute_aabb(&pose_a);
        let bounds_b = collider.shape.compute_aabb(&collider.pose);
        let radius_a = (bounds_a.maxs - bounds_a.mins).length() * 0.5;
        let radius_b = (bounds_b.maxs - bounds_b.mins).length() * 0.5;
        let center_a = (bounds_a.maxs + bounds_a.mins) * 0.5;
        let center_b = (bounds_b.maxs + bounds_b.mins) * 0.5;
        let travel = (linear_a - self.linvel).length() * dt
            + angular_a.length() * dt * ((center_a - com_a).length() + radius_a)
            + self.angvel.length() * dt * ((center_b - self.center_of_mass).length() + radius_b);
        if center_a.distance_squared(center_b) > (radius_a + radius_b + travel + skin).powi(2) {
            return None;
        }
        if let Ok(Some(c)) =
            query::contact(&pose_a, shape_a, &collider.pose, &*collider.shape, skin)
        {
            return valid_contact(SolidContact {
                point_a: c.point1,
                point_b: c.point2,
                normal: c.normal2,
                distance: c.dist,
                time_of_impact: 0.,
            });
        }
        let motion_a =
            NonlinearRigidMotion::new(pose_a, pose_a.inverse() * com_a, linear_a, angular_a);
        let motion_b = NonlinearRigidMotion::new(
            collider.pose,
            collider.pose.inverse() * self.center_of_mass,
            self.linvel,
            self.angvel,
        );
        let hit = query::cast_shapes_nonlinear(
            &motion_a,
            shape_a,
            &motion_b,
            &*collider.shape,
            0.,
            dt,
            true,
        )
        .ok()??;
        // The swept normal is transformed at the impact orientation; witness
        // points are transformed at the CURRENT pose for the native solver.
        let impact_rotation =
            Rotation::from_scaled_axis(self.angvel * hit.time_of_impact) * collider.pose.rotation;
        let normal = impact_rotation * hit.normal2;
        let point_a = pose_a * hit.witness1;
        let point_b = collider.pose * hit.witness2;
        valid_contact(SolidContact {
            point_a,
            point_b,
            normal,
            distance: (point_a - point_b).dot(normal),
            time_of_impact: hit.time_of_impact,
        })
    }
}

fn collect_solid_parts(
    shape: &SharedShape,
    pose: Pose,
    friction: f32,
    out: &mut Vec<SolidCollider>,
) {
    if let Some(compound) = shape.as_compound() {
        for (local_pose, part) in compound.shapes() {
            collect_solid_parts(part, pose * *local_pose, friction, out);
        }
    } else {
        out.push(SolidCollider {
            shape: shape.clone(),
            pose,
            friction,
        });
    }
}

fn valid_contact(mut c: SolidContact) -> Option<SolidContact> {
    if !c.point_a.is_finite()
        || !c.point_b.is_finite()
        || !c.normal.is_finite()
        || !c.distance.is_finite()
        || c.normal.length_squared() < 0.5
    {
        return None;
    }
    c.normal = c.normal.normalize();
    Some(c)
}

impl DynamicsWorld {
    pub fn local_center(&self, id: u64) -> Option<[f32; 3]> {
        self.bodies
            .get(*self.ids.get(&id)?)
            .map(|b| b.local_center_of_mass().to_array())
    }
    pub fn definition_revision(&self, id: u64) -> Option<usize> {
        self.meta.get(&id).map(|m| {
            m.geometry_revision
                .wrapping_mul(17)
                .wrapping_add(m.extra_hulls.len())
        })
    }

    /// Geometry is resolved locally before export; never send or execute Lua.
    pub fn body_definition(&self, id: u64) -> Option<BodyDefinition> {
        let meta = self.meta.get(&id)?;
        let mut body = meta.definition.clone();
        body.position = [0.; 3];
        body.heading = 0.;
        Some(BodyDefinition {
            deformation: self.deformation(id).cloned(),
            body,
            extras: meta
                .extra_hulls
                .iter()
                .map(|h| ExtraColliderDefinition {
                    points: h.points.clone(),
                    translation: h.translation,
                    friction: h.friction,
                })
                .collect(),
        })
    }

    pub fn spawn_replica(&mut self, definition: &BodyDefinition) -> Result<u64, String> {
        if definition.extras.len() > 16
            || matches!(
                definition.body.shape,
                Shape::Mesh { .. } | Shape::Model { .. }
            )
        {
            return Err("unsupported or oversized replica definition".into());
        }
        if let Some(field) = &definition.deformation {
            field.validate()?;
        }
        let runtime = match (&definition.deformation, &definition.body.deformation) {
            (Some(field), Some(options)) => Some(crate::deformation::Runtime::replica(
                field.clone(),
                options.clone(),
            )?),
            (None, None) => None,
            _ => return Err("replica deformation configuration/field mismatch".into()),
        };
        let mut desc = definition.body.clone();
        desc.deformation = None;
        // Ownership stays with the sender. The receiver does not resimulate
        // its engine, joints, Lua callbacks or local input.
        desc.body_type = BodyType::Dynamic;
        let id = self.spawn(desc)?;
        if let Some(runtime) = runtime {
            self.deformations.insert(id, runtime);
        }
        if let Some(meta) = self.meta.get_mut(&id) {
            meta.definition.deformation = definition.body.deformation.clone();
        }
        for extra in &definition.extras {
            if let Err(error) =
                self.add_convex_collider(id, &extra.points, extra.translation, extra.friction)
            {
                self.remove(id);
                return Err(error);
            }
        }
        let handle = *self.ids.get(&id).ok_or("replica body disappeared")?;
        if let Some(body) = self.bodies.get_mut(handle) {
            body.set_body_type(RigidBodyType::KinematicPositionBased, true);
        }
        // Keep authored physical mass for the native solver shadow. Rapier itself
        // remains owner-controlled kinematic; no peer reaction is committed here.
        if let Some(meta) = self.meta.get_mut(&id) {
            meta.definition.body_type = definition.body.body_type;
        }
        Ok(id)
    }

    /// Passive, finite-mass shadow of ONE native remote rigid body. All its
    /// collision parts share the original COM/inertia; mass is not duplicated
    /// per shape. The owner snapshot resets it each tick, but during the local
    /// Rapier step it can yield to an impact rather than acting as a stone wall.
    /// No resulting velocity is sent back as a second impulse to the peer.
    pub fn upsert_actor_shadow(
        &mut self,
        previous: Option<u64>,
        pose: Pose,
        inverse_mass: f32,
        inverse_inertia: [f32; 3],
        linear: [f32; 3],
        angular: [f32; 3],
        parts: &[SolidCollider],
    ) -> Result<u64, String> {
        if parts.is_empty()
            || parts.len() > 32
            || !inverse_mass.is_finite()
            || inverse_mass < 0.
            || inverse_inertia.iter().any(|v| !v.is_finite() || *v < 0.)
            || !pose.translation.is_finite()
            || !pose.rotation.is_finite()
            || linear.iter().chain(&angular).any(|v| !v.is_finite())
        {
            return Err("invalid native actor shadow".into());
        }
        let dynamic = inverse_mass > 0.;
        let mass = if dynamic { 1. / inverse_mass } else { 1. };
        let old = previous.filter(|id| {
            self.ids
                .get(id)
                .and_then(|h| self.bodies.get(*h))
                .is_some_and(|b| b.colliders().len() == parts.len() && b.is_dynamic() == dynamic)
        });
        let id = if let Some(id) = old {
            id
        } else {
            if let Some(id) = previous {
                self.remove(id);
            }
            let id = self.spawn(BodyDesc {
                shape: Shape::Sphere { radius: 0.1 },
                mass,
                body_type: if dynamic {
                    BodyType::Dynamic
                } else {
                    BodyType::Kinematic
                },
                ccd: true,
                linear_damping: 0.,
                angular_damping: 0.,
                membership: PLAYER_MEMBERSHIP,
                filter: !PLAYER_MEMBERSHIP,
                ..Default::default()
            })?;
            let handle = self.ids[&id];
            for part in &parts[1..] {
                let collider = ColliderBuilder::new(part.shape.clone())
                    .density(0.)
                    .collision_groups(InteractionGroups::new(
                        Group::from_bits_truncate(PLAYER_MEMBERSHIP),
                        Group::from_bits_truncate(!PLAYER_MEMBERSHIP),
                        InteractionTestMode::And,
                    ))
                    .build();
                self.colliders
                    .insert_with_parent(collider, handle, &mut self.bodies);
            }
            id
        };
        let handle = self.ids[&id];
        let handles = self.bodies[handle].colliders().to_vec();
        let principal =
            Vector::from_array(inverse_inertia.map(|x| if x > 0. { 1. / x } else { 0. }));
        for (index, (handle, part)) in handles.iter().zip(parts).enumerate() {
            let collider = &mut self.colliders[*handle];
            let old_bounds = collider.shape().compute_local_aabb();
            let new_bounds = part.shape.compute_local_aabb();
            if std::mem::discriminant(&collider.shape().as_typed_shape())
                != std::mem::discriminant(&part.shape.as_typed_shape())
                || old_bounds.mins != new_bounds.mins
                || old_bounds.maxs != new_bounds.maxs
            {
                collider.set_shape(part.shape.clone());
            }
            let local = pose.inverse() * part.pose;
            collider.set_position_wrt_parent(local);
            collider.set_friction(part.friction);
            // Only the first collider supplies mass. Express original native
            // inertia in that collider's frame so recombination returns the COM
            // and principal axes of the single native body.
            if index == 0 && dynamic {
                let properties = MassProperties::new(Vector::ZERO, mass, principal)
                    .transform_by(&local.inverse());
                collider.set_mass_properties(properties);
            } else {
                collider.set_density(0.);
            }
        }
        let body = &mut self.bodies[handle];
        body.set_gravity_scale(0., true);
        body.recompute_mass_properties_from_colliders(&self.colliders);
        body.set_position(pose, true);
        body.set_linvel(Vector::from_array(linear), true);
        body.set_angvel(Vector::from_array(angular), true);
        if !dynamic {
            self.kinematic_motion.insert(id, (linear, angular));
        }
        Ok(id)
    }

    /// Only solid objects whose layer masks admit native players are included.
    /// Caller also excludes its own actor mirrors and any occupied attachment.
    pub fn solid_bodies(&self) -> Vec<SolidBody> {
        let mut out = Vec::new();
        for (&id, meta) in &self.meta {
            let desc = &meta.definition;
            if desc.sensor
                || desc.membership & !PLAYER_MEMBERSHIP == 0
                || desc.filter & PLAYER_MEMBERSHIP == 0
            {
                continue;
            }
            let Some(body) = self.ids.get(&id).and_then(|h| self.bodies.get(*h)) else {
                continue;
            };
            if !body.is_enabled() {
                continue;
            }
            let props = &body.mass_properties().local_mprops;
            let mut colliders = Vec::new();
            for handle in body.colliders() {
                let Some(c) = self.colliders.get(*handle) else {
                    continue;
                };
                if c.is_sensor() || !c.is_enabled() {
                    continue;
                }
                let groups = c.collision_groups();
                if groups.memberships.bits() & 1 == 0
                    || groups.filter.bits() & PLAYER_MEMBERSHIP == 0
                {
                    continue;
                }
                // Flatten compounds for the native nonlinear support-shape
                // casts, but KEEP one body identity, COM and physical inertia.
                let pose = *body.position()
                    * c.position_wrt_parent()
                        .copied()
                        .unwrap_or_else(Pose::identity);
                collect_solid_parts(c.shared_shape(), pose, c.friction(), &mut colliders);
            }
            out.push(SolidBody {
                id,
                pose: *body.position(),
                center_of_mass: body.center_of_mass(),
                inertia_rotation: *body.rotation() * props.principal_inertia_local_frame,
                inverse_mass: if desc.body_type == BodyType::Dynamic {
                    props.inv_mass
                } else {
                    0.
                },
                inverse_inertia: if desc.body_type == BodyType::Dynamic {
                    props.inv_principal_inertia
                } else {
                    Vector::ZERO
                },
                linvel: body.linvel(),
                angvel: body.angvel(),
                contact_group: desc.contact_group,
                colliders,
            });
        }
        out
    }

    /// Feed the native solver's velocity change back exactly once. The caller
    /// passes only locally owned dynamic bodies, never another peer's replicas.
    pub fn add_velocity_delta(&mut self, id: u64, linear: [f32; 3], angular: [f32; 3]) -> bool {
        if linear.iter().chain(&angular).any(|v| !v.is_finite()) {
            return false;
        }
        let Some(body) = self.ids.get(&id).and_then(|h| self.bodies.get_mut(*h)) else {
            return false;
        };
        if !body.is_dynamic() {
            return false;
        }
        body.set_linvel(body.linvel() + Vector::from_array(linear), true);
        body.set_angvel(body.angvel() + Vector::from_array(angular), true);
        true
    }

    /// Split-impulse positional correction from another solver, at the COM.
    /// This is NOT an extra integration step and never changes velocity.
    pub fn correct_pose(&mut self, id: u64, displacement: [f32; 3], orientation: [f32; 3]) {
        if displacement
            .iter()
            .chain(&orientation)
            .any(|v| !v.is_finite())
        {
            return;
        }
        let Some(body) = self.ids.get(&id).and_then(|h| self.bodies.get_mut(*h)) else {
            return;
        };
        if !body.is_dynamic() {
            return;
        }
        let rotation = (Rotation::from_scaled_axis(Vector::from_array(orientation))
            * *body.rotation())
        .normalize();
        let translation = body.center_of_mass() + Vector::from_array(displacement)
            - rotation * body.local_center_of_mass();
        body.set_position(Pose::from_parts(translation, rotation), true);
    }

    /// Explicit velocity for a position-based mirror, integrated into a target
    /// on EACH Rapier substep. set_linvel alone is discarded by a position-based
    /// kinematic body and therefore cannot drive real contact response.
    pub fn set_kinematic_motion(&mut self, id: u64, linear: [f32; 3], angular: [f32; 3]) {
        if linear.iter().chain(&angular).any(|v| !v.is_finite()) {
            return;
        }
        let Some(body) = self.ids.get(&id).and_then(|h| self.bodies.get_mut(*h)) else {
            return;
        };
        if !body.is_kinematic() {
            return;
        }
        body.set_linvel(Vector::from_array(linear), true);
        body.set_angvel(Vector::from_array(angular), true);
        self.kinematic_motion.insert(id, (linear, angular));
    }

    pub(super) fn advance_kinematic_targets(&mut self, dt: f32) {
        for (id, (linear, angular)) in &self.kinematic_motion {
            let Some(body) = self.ids.get(id).and_then(|h| self.bodies.get_mut(*h)) else {
                continue;
            };
            let rotation = (Rotation::from_scaled_axis(Vector::from_array(*angular) * dt)
                * *body.rotation())
            .normalize();
            let translation = body.center_of_mass() + Vector::from_array(*linear) * dt
                - rotation * body.local_center_of_mass();
            body.set_next_kinematic_position(Pose::from_parts(translation, rotation));
        }
    }

    /// Full-height standing capsule clearance, including the actual static map
    /// and all solid mod colliders. Used for safe, upright detach placement.
    pub fn capsule_clear(
        &self,
        feet: [f32; 3],
        height: f32,
        radius: f32,
        skin: f32,
        exclude: &[u64],
    ) -> bool {
        if !(height >= 2. * radius && radius > 0. && skin >= 0.)
            || feet.iter().any(|v| !v.is_finite())
        {
            return false;
        }
        let capsule = SharedShape::capsule_y((height - 2. * radius) * 0.5, radius);
        let pose = Pose::translation(feet[0], feet[1] + height * 0.5, feet[2]);
        for (&id, handle) in &self.ids {
            if exclude.contains(&id) {
                continue;
            }
            let Some(body) = self.bodies.get(*handle) else {
                continue;
            };
            for handle in body.colliders() {
                let Some(collider) = self.colliders.get(*handle) else {
                    continue;
                };
                if collider.is_sensor() || !collider.is_enabled() {
                    continue;
                }
                let other = *body.position()
                    * collider
                        .position_wrt_parent()
                        .copied()
                        .unwrap_or_else(Pose::identity);
                match query::contact(&pose, &*capsule, &other, collider.shape(), skin) {
                    Ok(Some(c)) if c.dist < skin => return false,
                    Err(_) => return false, // Unknown geometry is NOT free space.
                    _ => {}
                }
            }
        }
        true
    }
}

/// Sweep a sphere against an immutable solid body set. Zero-radius lines use
/// a 0.1 mm sphere, including inside-solid starts. No triangle sidedness.
pub fn sweep_sphere(
    bodies: &[SolidBody],
    start: [f32; 3],
    end: [f32; 3],
    radius: f32,
) -> Option<(u64, SolidContact)> {
    let start = Vector::from_array(start);
    let delta = Vector::from_array(end) - start;
    if !start.is_finite() || !delta.is_finite() || !radius.is_finite() || radius < 0. {
        return None;
    }
    let shape = SharedShape::ball(radius.max(0.0001));
    let pose = Pose::from_translation(start);
    let mut nearest: Option<(u64, SolidContact)> = None;
    for body in bodies {
        for collider in &body.colliders {
            let options = ShapeCastOptions {
                max_time_of_impact: 1.,
                target_distance: 0.,
                stop_at_penetration: true,
                compute_impact_geometry_on_penetration: true,
            };
            let Ok(Some(hit)) = query::cast_shapes(
                &pose,
                delta,
                &*shape,
                &collider.pose,
                Vector::ZERO,
                &*collider.shape,
                options,
            ) else {
                continue;
            };
            let normal = collider.pose.rotation * hit.normal2;
            let point_b = collider.pose * hit.witness2;
            let Some(c) = valid_contact(SolidContact {
                point_a: start + delta * hit.time_of_impact - normal * radius,
                point_b,
                normal,
                distance: 0.,
                time_of_impact: hit.time_of_impact,
            }) else {
                continue;
            };
            if nearest
                .as_ref()
                .is_none_or(|(_, old)| c.time_of_impact < old.time_of_impact)
            {
                nearest = Some((body.id, c));
            }
        }
    }
    nearest
}

/// Exact collider surface triangulation for diagnostics/query normal refinement.
/// It does not feed the rigid-body narrow phase and never substitutes an AABB.
pub fn collider_triangles(collider: &SolidCollider) -> Vec<[[f32; 3]; 3]> {
    use rapier3d::parry::shape::TypedShape;
    let (vertices, indices) = match collider.shape.as_typed_shape() {
        TypedShape::Cuboid(s) => s.to_trimesh(),
        TypedShape::Ball(s) => s.to_trimesh(20, 12),
        TypedShape::Capsule(s) => s.to_trimesh(20, 12),
        TypedShape::ConvexPolyhedron(s) => s.to_trimesh(),
        _ => return Vec::new(),
    };
    indices
        .into_iter()
        .map(|triangle| triangle.map(|i| (collider.pose * vertices[i as usize]).to_array()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn box_world() -> (DynamicsWorld, u64) {
        let mut world = DynamicsWorld::default();
        let id = world
            .spawn(BodyDesc {
                shape: Shape::Box {
                    half_extents: [1., 1., 2.],
                },
                contact_group: 8,
                ..Default::default()
            })
            .unwrap();
        (world, id)
    }
    #[test]
    fn an_inside_capsule_is_not_a_missed_triangle_shell() {
        let (world, _) = box_world();
        let bodies = world.solid_bodies();
        let body = &bodies[0];
        let capsule = SharedShape::capsule_y(0.5, 0.3);
        let c = body
            .contact(
                &body.colliders[0],
                &*capsule,
                Pose::identity(),
                Vector::ZERO,
                Vector::ZERO,
                Vector::ZERO,
                1. / 60.,
                0.02,
            )
            .unwrap();
        assert!(c.distance < -0.1 && c.normal.is_finite());
    }
    #[test]
    fn fast_pass_through_is_swept_not_only_sampled() {
        let (mut world, id) = box_world();
        world.set_pose(id, [0., 0., 5.], [0., 0., 0., 1.]);
        world.set_linvel(id, [0., 0., -600.]);
        let body = world.solid_bodies().remove(0);
        let shape = SharedShape::ball(0.25);
        let c = body
            .contact(
                &body.colliders[0],
                &*shape,
                Pose::identity(),
                Vector::ZERO,
                Vector::ZERO,
                Vector::ZERO,
                1. / 60.,
                0.02,
            )
            .unwrap();
        assert!(c.time_of_impact > 0. && c.time_of_impact < 1. / 60.);
        assert!(c.normal.z < -0.9);
    }
    #[test]
    fn sensor_and_layer_filter_are_not_solid() {
        let mut world = DynamicsWorld::default();
        world
            .spawn(BodyDesc {
                sensor: true,
                ..Default::default()
            })
            .unwrap();
        world
            .spawn(BodyDesc {
                filter: 1,
                ..Default::default()
            })
            .unwrap();
        assert!(world.solid_bodies().is_empty());
    }
    #[test]
    fn compound_is_one_body_with_exact_local_offset() {
        let (mut world, id) = box_world();
        let points = vec![[-1., -1., -1.], [1., -1., -1.], [0., 1., -1.], [0., 0., 1.]];
        world
            .add_convex_collider(id, &points, [4., 0., 0.], 0.7)
            .unwrap();
        let bodies = world.solid_bodies();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].colliders.len(), 2);
        assert!((bodies[0].colliders[1].pose.translation.x - 4.).abs() < 1e-5);
        assert_eq!(bodies[0].contact_group, 8);
    }
    #[test]
    fn replica_preserves_hull_not_a_half_meter_box() {
        let (world, id) = box_world();
        let definition = world.body_definition(id).unwrap();
        let mut other = DynamicsWorld::default();
        let remote = other.spawn_replica(&definition).unwrap();
        assert_eq!(other.read(remote).unwrap().body_type, BodyType::Kinematic);
        assert_eq!(
            other.body_definition(remote).unwrap().body.shape,
            definition.body.shape
        );
        assert_eq!(
            other.solid_bodies()[0].inverse_mass,
            world.solid_bodies()[0].inverse_mass
        );
    }
    #[test]
    fn velocity_delta_is_not_applied_to_a_replica() {
        let (mut world, id) = box_world();
        let r = world
            .spawn_replica(&world.body_definition(id).unwrap())
            .unwrap();
        assert!(!world.add_velocity_delta(r, [10., 0., 0.], [0.; 3]));
        assert!(world.add_velocity_delta(id, [10., 0., 0.], [0.; 3]));
    }
    #[test]
    fn remote_actor_compound_keeps_one_mass_and_original_com() {
        let mut world = DynamicsWorld::default();
        let pose = Pose::translation(4., 5., 6.);
        let parts = [
            SolidCollider {
                shape: SharedShape::ball(0.3),
                pose: Pose::translation(4., 6., 6.),
                friction: 0.5,
            },
            SolidCollider {
                shape: SharedShape::capsule_y(0.4, 0.2),
                pose: Pose::translation(4., 4., 6.),
                friction: 0.5,
            },
        ];
        let id = world
            .upsert_actor_shadow(
                None,
                pose,
                1. / 80.,
                [0.5, 0.25, 0.2],
                [3., 0., 0.],
                [0.; 3],
                &parts,
            )
            .unwrap();
        let body = &world.bodies[world.ids[&id]];
        assert!(body.is_dynamic());
        assert_eq!(body.colliders().len(), 2);
        assert!((body.mass() - 80.).abs() < 0.001);
        assert!(body.center_of_mass().distance(pose.translation) < 0.0001);
        let retained = world
            .upsert_actor_shadow(
                Some(id),
                pose,
                1. / 80.,
                [0.5, 0.25, 0.2],
                [3., 0., 0.],
                [0.; 3],
                &parts,
            )
            .unwrap();
        assert_eq!(retained, id);
        assert!((world.bodies[world.ids[&id]].mass() - 80.).abs() < 0.001);
        assert!(
            world.solid_bodies().is_empty(),
            "native shadows must not be fed back as mod solids"
        );
    }
}
