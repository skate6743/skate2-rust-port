//! General rigid-body dynamics island (Rapier). Independent of Skate3 BoardWorld.
//! Higher-level assemblies (cars, props, modes) compose these primitives in Lua.
use rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub use rapier3d;
pub mod definition_codec;
pub mod deformation;
pub mod model;
pub mod solid;
pub mod visual_contact;
pub use model::ModelColliderOptions;
pub use solid::{BodyDefinition, SolidBody, SolidCollider};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Box {
        half_extents: [f32; 3],
    },
    Sphere {
        radius: f32,
    },
    Capsule {
        half_height: f32,
        radius: f32,
    },
    /// Convex hull from authoring verts (e.g. resolved GLB object). Max 512 points.
    Convex {
        points: Vec<[f32; 3]>,
    },
    /// Package-relative GLB named object. Host resolves to Convex before spawn.
    Mesh {
        path: String,
        object: String,
    },
    /// Cook a compound solid from the existing render GLB. No collision asset.
    /// object selects a node/mesh in Scene0; empty selects the whole scene.
    Model {
        path: String,
        #[serde(default)]
        object: String,
        #[serde(default)]
        options: ModelColliderOptions,
    },
    /// Resolved convex parts in the SAME body-local frame. One rigid body/mass.
    Compound {
        hulls: Vec<Vec<[f32; 3]>>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    Dynamic,
    Kinematic,
    Static,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyDesc {
    #[serde(default)]
    pub deformation: Option<deformation::Options>,
    pub shape: Shape,
    pub body_type: BodyType,
    #[serde(default = "one_mass")]
    pub mass: f32,
    #[serde(default)]
    pub position: [f32; 3],
    /// Yaw around world Y, radians. Zero faces +Z.
    #[serde(default)]
    pub heading: f32,
    #[serde(default = "one_friction")]
    pub friction: f32,
    #[serde(default)]
    pub ccd: bool,
    /// Sensor/trigger: generates contacts without solid collision response.
    #[serde(default)]
    pub sensor: bool,
    /// Collision membership bits (default 0x0001).
    #[serde(default = "default_group")]
    pub membership: u32,
    /// Collision filter bits (default 0xFFFF).
    #[serde(default = "default_filter")]
    pub filter: u32,
    /// Original-engine assembly/contact classification (0..=20). This is
    /// metadata, NOT a Rapier layer mask. Mods select their own classification.
    #[serde(default)]
    pub contact_group: u32,
    /// Local centre of mass offset (body frame). Enables weight transfer under
    /// corner spring loads / longitudinal forces without any vehicle type.
    #[serde(default)]
    pub center_of_mass: [f32; 3],
    /// Chassis-local collider translation (legacy vehicle `collider_offset`).
    /// Mass properties use `center_of_mass - collider_offset` like the old vehicle SDK.
    #[serde(default)]
    pub collider_offset: [f32; 3],
    /// Optional inertia box half-extents; when set, mass properties use
    /// `I = m/3 * (yy+zz, xx+zz, xx+yy)` about `center_of_mass`.
    #[serde(default)]
    pub inertia_half_extents: Option<[f32; 3]>,
    #[serde(default = "default_lin_damp")]
    pub linear_damping: f32,
    #[serde(default = "default_ang_damp")]
    pub angular_damping: f32,
}

fn one_mass() -> f32 {
    1.
}
fn one_friction() -> f32 {
    0.7
}
fn default_group() -> u32 {
    0x0001
}
fn default_filter() -> u32 {
    0xFFFF
}
fn default_lin_damp() -> f32 {
    0.2
}
fn default_ang_damp() -> f32 {
    0.5
}

impl Default for BodyDesc {
    fn default() -> Self {
        Self {
            deformation: None,
            shape: Shape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            body_type: BodyType::Dynamic,
            mass: 1.,
            position: [0., 0., 0.],
            heading: 0.,
            friction: 0.7,
            ccd: false,
            sensor: false,
            membership: 0x0001,
            filter: 0xFFFF,
            contact_group: 0,
            center_of_mass: [0., 0., 0.],
            collider_offset: [0., 0., 0.],
            inertia_half_extents: None,
            linear_damping: 0.2,
            angular_damping: 0.5,
        }
    }
}

/// Generic ray spring-damper (Bullet/Rapier suspension formula, body-agnostic).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpringRayDesc {
    /// Attachment point in the body local frame.
    pub local_origin: [f32; 3],
    /// Cast direction in the body local frame (normalized by the host).
    #[serde(default = "neg_y")]
    pub local_direction: [f32; 3],
    pub rest_length: f32,
    pub max_travel: f32,
    /// Contact offset along the ray (e.g. wheel radius); subtracted from hit distance.
    #[serde(default)]
    pub contact_radius: f32,
    pub stiffness: f32,
    pub compression: f32,
    pub relaxation: f32,
    #[serde(default = "default_max_suspension_force")]
    pub max_force: f32,
    pub dt: f32,
}

fn neg_y() -> [f32; 3] {
    [0., -1., 0.]
}
fn default_max_suspension_force() -> f32 {
    6000.
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SpringRayHit {
    pub in_contact: bool,
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub hard_point: [f32; 3],
    pub direction_ws: [f32; 3],
    pub suspension_length: f32,
    /// Clamped suspension force (newtons) after mass scaling — use as tire load.
    pub load: f32,
    pub relative_velocity: f32,
}

#[derive(Clone)]
struct ExtraHull {
    points: Vec<[f32; 3]>,
    translation: [f32; 3],
    friction: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct BodySnapshot {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // xyzw
    pub linvel: [f32; 3],
    pub angvel: [f32; 3],
    pub body_type: BodyType,
    /// Last `apply_force` this command frame (newtons). Zero if none.
    pub force: [f32; 3],
    /// Last `apply_torque` this command frame.
    pub torque: [f32; 3],
    pub mass: f32,
}

/// Reserved dynamics id for the static map trimesh (`set_ground`).
pub const GROUND_BODY_ID: u64 = 0;

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub body: u64,
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub toi: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ContactEvent {
    pub body_a: u64,
    pub body_b: u64,
    pub started: bool,
}

/// Neutral BoardWorld-facing export. Host converts to ContactPrimitive.
#[derive(Clone, Debug)]
pub enum ExportedShape {
    Box {
        half_extents: [f32; 3],
        rounding: f32,
    },
    Sphere {
        radius: f32,
    },
    Capsule {
        half_height: f32,
        radius: f32,
    },
    /// Local-space triangles in the body frame (capped by export).
    Triangles {
        tris: Vec<[[f32; 3]; 3]>,
    },
}

#[derive(Clone, Debug)]
pub struct ExportedVolume {
    pub body: u64,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub linvel: [f32; 3],
    pub angvel: [f32; 3],
    pub mass: f32,
    pub friction: f32,
    pub shape: ExportedShape,
}

pub const MAX_EXPORT_TRIANGLES: usize = 512;

#[derive(Clone)]
struct BodyMeta {
    geometry_revision: usize,
    definition: BodyDesc,
    shape: Shape,
    mass: f32,
    friction: f32,
    collider_offset: [f32; 3],
    /// Optional local-space collision mesh; when set, export prefers triangles.
    mesh: Option<Vec<[[f32; 3]; 3]>>,
    /// Extra convex hulls (e.g. tires) — exported to BoardWorld alongside the primary.
    extra_hulls: Vec<ExtraHull>,
    /// Force submitted since the last `begin_force_frame` (for SDK reads).
    commanded_force: [f32; 3],
    commanded_torque: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevoluteJointDesc {
    pub body_a: u64,
    pub body_b: u64,
    pub anchor_a: [f32; 3],
    pub anchor_b: [f32; 3],
    /// Local hinge axis on both bodies (unit vector preferred).
    #[serde(default = "axis_y")]
    pub axis: [f32; 3],
    /// Optional angular limits in radians.
    pub limits: Option<[f32; 2]>,
    #[serde(default = "true_fn")]
    pub contacts_enabled: bool,
}

fn axis_y() -> [f32; 3] {
    [0., 1., 0.]
}
fn true_fn() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum JointMotorDesc {
    Velocity {
        target_velocity: f32,
        #[serde(default = "one_factor")]
        factor: f32,
        max_force: Option<f32>,
    },
    Position {
        target_position: f32,
        stiffness: f32,
        damping: f32,
        max_force: Option<f32>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrismaticJointDesc {
    pub body_a: u64,
    pub body_b: u64,
    pub anchor_a: [f32; 3],
    pub anchor_b: [f32; 3],
    /// Sliding axis in body_a's local frame.
    #[serde(default = "neg_y")]
    pub axis: [f32; 3],
    /// Optional travel limits along the axis (min, max).
    pub limits: Option<[f32; 2]>,
    #[serde(default = "true_fn")]
    pub contacts_enabled: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointSpringDesc {
    pub target_position: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub max_force: Option<f32>,
}

fn one_factor() -> f32 {
    1.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JointKind {
    Revolute,
    Prismatic,
}

pub struct DynamicsWorld {
    deformations: BTreeMap<u64, deformation::Runtime>,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd: CCDSolver,
    gravity: Vector,
    ids: BTreeMap<u64, RigidBodyHandle>,
    reverse: HashMap<RigidBodyHandle, u64>,
    joints: BTreeMap<u64, ImpulseJointHandle>,
    joint_kinds: BTreeMap<u64, JointKind>,
    next: u64,
    next_joint: u64,
    ground: bool,
    contacts: Vec<ContactEvent>,
    prev_pairs: BTreeMap<(u64, u64), ()>,
    meta: HashMap<u64, BodyMeta>,
    kinematic_motion: BTreeMap<u64, ([f32; 3], [f32; 3])>,
}

impl Default for DynamicsWorld {
    fn default() -> Self {
        Self {
            deformations: BTreeMap::new(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::default(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),
            gravity: Vector::new(0., -9.81, 0.),
            ids: BTreeMap::new(),
            reverse: HashMap::new(),
            joints: BTreeMap::new(),
            joint_kinds: BTreeMap::new(),
            next: 1,
            next_joint: 1,
            ground: false,
            contacts: Vec::new(),
            prev_pairs: BTreeMap::new(),
            meta: HashMap::new(),
            kinematic_motion: BTreeMap::new(),
        }
    }
}

impl DynamicsWorld {
    pub fn has_ground(&self) -> bool {
        self.ground
    }

    pub fn set_ground(
        &mut self,
        triangles: impl Iterator<Item = [[f32; 3]; 3]>,
    ) -> Result<(), String> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for triangle in triangles {
            let n = vertices.len() as u32;
            vertices.extend(triangle.map(|p| Vector::new(p[0], p[1], p[2])));
            // Preserve author winding; callers must face normals toward the playable side.
            indices.push([n, n + 1, n + 2]);
        }
        if vertices.is_empty() {
            return Err("dynamics ground has no triangles".into());
        }
        let collider = ColliderBuilder::trimesh_with_flags(
            vertices,
            indices,
            TriMeshFlags::FIX_INTERNAL_EDGES,
        )
        .map_err(|e| e.to_string())?
        .friction(1.)
        .collision_groups(InteractionGroups::new(
            Group::ALL,
            Group::ALL,
            InteractionTestMode::And,
        ))
        .build();
        let body = self.bodies.insert(RigidBodyBuilder::fixed().build());
        self.colliders
            .insert_with_parent(collider, body, &mut self.bodies);
        // Raycasts map collider parents back through `reverse`; without this entry
        // every spring_ray misses the map trimesh and tire drive stays at 0 m/s.
        self.ids.insert(GROUND_BODY_ID, body);
        self.reverse.insert(body, GROUND_BODY_ID);
        if self.next <= GROUND_BODY_ID {
            self.next = GROUND_BODY_ID + 1;
        }
        self.ground = true;
        Ok(())
    }

    pub fn spawn(&mut self, desc: BodyDesc) -> Result<u64, String> {
        self.validate(&desc)?;
        let deformation = desc
            .deformation
            .clone()
            .map(|o| deformation::Runtime::new(&desc, o))
            .transpose()?;
        let collider = self.collider(&desc)?;
        // Mass comes from the collider (see collider()); do not also call
        // additional_mass or density+mass stack and inflate inertia.
        let builder = match desc.body_type {
            BodyType::Dynamic => RigidBodyBuilder::dynamic()
                .ccd_enabled(desc.ccd)
                .linear_damping(desc.linear_damping)
                .angular_damping(desc.angular_damping),
            BodyType::Kinematic => RigidBodyBuilder::kinematic_position_based(),
            BodyType::Static => RigidBodyBuilder::fixed(),
        };
        let body = builder
            .translation(Vector::new(
                desc.position[0],
                desc.position[1],
                desc.position[2],
            ))
            .rotation(Vector::Y * desc.heading)
            .build();
        let handle = self.bodies.insert(body);
        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);
        let id = self.next;
        self.next += 1;
        if let Some(runtime) = deformation {
            self.deformations.insert(id, runtime);
        }
        self.ids.insert(id, handle);
        self.reverse.insert(handle, id);
        if let Some(body) = self.bodies.get_mut(handle) {
            body.recompute_mass_properties_from_colliders(&self.colliders);
        }
        self.meta.insert(
            id,
            BodyMeta {
                geometry_revision: 0,
                definition: desc.clone(),
                shape: desc.shape.clone(),
                mass: desc.mass.max(0.01),
                friction: desc.friction,
                collider_offset: desc.collider_offset,
                mesh: None,
                extra_hulls: Vec::new(),
                commanded_force: [0., 0., 0.],
                commanded_torque: [0., 0., 0.],
            },
        );
        Ok(id)
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let Some(handle) = self.ids.remove(&id) else {
            return false;
        };
        self.reverse.remove(&handle);
        self.meta.remove(&id);
        self.deformations.remove(&id);
        self.kinematic_motion.remove(&id);
        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
        true
    }

    /// Extra convex collider on an existing body (e.g. high-friction tires).
    pub fn add_convex_collider(
        &mut self,
        id: u64,
        points: &[[f32; 3]],
        translation: [f32; 3],
        friction: f32,
    ) -> Result<(), String> {
        let handle = *self.ids.get(&id).ok_or("unknown body")?;
        if self.meta.get(&id).is_none_or(|m| m.extra_hulls.len() >= 16)
            || translation.iter().any(|v| !v.is_finite() || v.abs() > 100.)
            || points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1000.)
        {
            return Err("extra collider exceeds finite geometry/body limits".into());
        }
        if points.len() < 4 || points.len() > 512 {
            return Err("convex needs 4..=512 points".into());
        }
        if !friction.is_finite() || !(0. ..=2.).contains(&friction) {
            return Err("friction must be in 0..2".into());
        }
        let pts: Vec<Vector> = points
            .iter()
            .map(|p| Vector::new(p[0], p[1], p[2]))
            .collect();
        // Mass stays on the primary collider; extras must not add density.
        let builder = ColliderBuilder::convex_hull(&pts)
            .ok_or_else(|| "convex hull failed (degenerate points)".to_string())?
            .friction(friction)
            .density(0.)
            .sensor(self.meta[&id].definition.sensor)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(self.meta[&id].definition.membership),
                Group::from_bits_truncate(self.meta[&id].definition.filter),
                InteractionTestMode::And,
            ))
            .translation(Vector::new(translation[0], translation[1], translation[2]));
        self.colliders
            .insert_with_parent(builder.build(), handle, &mut self.bodies);
        if let Some(meta) = self.meta.get_mut(&id) {
            meta.extra_hulls.push(ExtraHull {
                points: points.to_vec(),
                translation,
                friction,
            });
        }
        Ok(())
    }

    /// Optional local-space trimesh for BoardWorld export (capped at export time).
    pub fn set_collision_mesh(&mut self, id: u64, tris: Vec<[[f32; 3]; 3]>) -> bool {
        let Some(meta) = self.meta.get_mut(&id) else {
            return false;
        };
        if tris.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return false;
        }
        meta.mesh = if tris.is_empty() { None } else { Some(tris) };
        true
    }

    /// Update a player mirror without destroying its contact manifold each tick.
    pub fn upsert_kinematic_proxy(
        &mut self,
        id: Option<u64>,
        shape: Shape,
        position: [f32; 3],
        rotation_xyzw: [f32; 4],
        friction: f32,
    ) -> Result<u64, String> {
        let desc = BodyDesc {
            shape: shape.clone(),
            body_type: BodyType::Kinematic,
            mass: 1.,
            position,
            friction,
            membership: solid::PLAYER_MEMBERSHIP,
            filter: u32::MAX,
            ..Default::default()
        };
        self.validate(&desc)?;
        let reusable = id.filter(|id| {
            self.meta
                .get(id)
                .is_some_and(|m| m.shape == shape && m.friction == friction)
        });
        let id = if let Some(id) = reusable {
            id
        } else {
            if let Some(id) = id {
                self.remove(id);
            }
            self.spawn(desc)?
        };
        if !self.set_pose(id, position, rotation_xyzw) {
            return Err("invalid kinematic proxy pose".into());
        }
        Ok(id)
    }

    /// Export dynamics colliders for injection into BoardWorld as Attached volumes.
    pub fn export_boardworld_volumes(&self) -> Vec<ExportedVolume> {
        let mut out = Vec::new();
        for (&id, meta) in &self.meta {
            let Some(snap) = self.read(id) else {
                continue;
            };
            if !snap
                .position
                .iter()
                .chain(snap.linvel.iter())
                .chain(snap.angvel.iter())
                .all(|v| v.is_finite())
                || !snap.rotation.iter().all(|v| v.is_finite())
            {
                continue;
            }
            let shape = if let Some(mesh) = &meta.mesh {
                let mut tris = mesh.clone();
                if tris.len() > MAX_EXPORT_TRIANGLES {
                    tris.truncate(MAX_EXPORT_TRIANGLES);
                }
                ExportedShape::Triangles { tris }
            } else {
                match &meta.shape {
                    Shape::Box { half_extents } => ExportedShape::Box {
                        half_extents: *half_extents,
                        rounding: 0.,
                    },
                    Shape::Sphere { radius } => ExportedShape::Sphere { radius: *radius },
                    Shape::Capsule {
                        half_height,
                        radius,
                    } => ExportedShape::Capsule {
                        half_height: *half_height,
                        radius: *radius,
                    },
                    Shape::Convex { points } => {
                        // Real hull faces, not a bounding box: BoardWorld gets the
                        // authored silhouette (body-local triangles, world-transformed
                        // by the consumer).
                        if let Some(vol) = self.convex_hull_volume(
                            id,
                            meta,
                            snap,
                            points,
                            meta.collider_offset,
                            meta.friction,
                        ) {
                            out.push(vol);
                        }
                        for hull in &meta.extra_hulls {
                            if let Some(vol) = self.convex_hull_volume(
                                id,
                                meta,
                                snap,
                                &hull.points,
                                hull.translation,
                                hull.friction,
                            ) {
                                out.push(vol);
                            }
                        }
                        continue;
                    }
                    Shape::Compound { hulls } => {
                        let mut tris = Vec::new();
                        for points in hulls {
                            if let Some(part) =
                                hull_triangles(points, meta.collider_offset, usize::MAX)
                            {
                                tris.extend(part);
                            }
                        }
                        ExportedShape::Triangles { tris }
                    }
                    // Unresolved file references are not physics shapes.
                    Shape::Mesh { .. } | Shape::Model { .. } => continue,
                }
            };
            out.push(ExportedVolume {
                body: id,
                position: snap.position,
                rotation: snap.rotation,
                linvel: snap.linvel,
                angvel: snap.angvel,
                mass: meta.mass,
                friction: meta.friction,
                shape,
            });
            for hull in &meta.extra_hulls {
                if let Some(vol) = self.convex_hull_volume(
                    id,
                    meta,
                    snap,
                    &hull.points,
                    hull.translation,
                    hull.friction,
                ) {
                    out.push(vol);
                }
            }
        }
        out
    }

    /// Convex hull → body-local triangle soup for BoardWorld (never a bounding box).
    fn convex_hull_volume(
        &self,
        id: u64,
        meta: &BodyMeta,
        snap: BodySnapshot,
        points: &[[f32; 3]],
        local_translation: [f32; 3],
        friction: f32,
    ) -> Option<ExportedVolume> {
        let tris = hull_triangles(points, local_translation, MAX_EXPORT_TRIANGLES)?;
        Some(ExportedVolume {
            body: id,
            position: snap.position,
            rotation: snap.rotation,
            linvel: snap.linvel,
            angvel: snap.angvel,
            mass: meta.mass,
            friction,
            shape: ExportedShape::Triangles { tris },
        })
    }

    /// Geometric overlap with externally solved shapes, including sensors.
    /// This query never creates a second simulation body or applies impulses.
    pub fn overlaps_shapes(&self, id: u64, shapes: &[(SharedShape, Pose)]) -> bool {
        let Some(body) = self.ids.get(&id).and_then(|h| self.bodies.get(*h)) else {
            return false;
        };
        body.colliders()
            .iter()
            .filter_map(|h| self.colliders.get(*h))
            .any(|c| {
                let collider_pose = c
                    .position_wrt_parent()
                    .map_or(*c.position(), |local| *body.position() * *local);
                let bounds = c.shape().compute_aabb(&collider_pose).loosened(0.025);
                c.is_enabled()
                    && shapes.iter().any(|(shape, pose)| {
                        use rapier3d::parry::bounding_volume::BoundingVolume;
                        if !bounds.intersects(&shape.compute_aabb(pose)) {
                            return false;
                        }
                        if c.is_sensor() {
                            rapier3d::parry::query::intersection_test(
                                &collider_pose,
                                c.shape(),
                                pose,
                                &**shape,
                            )
                            .unwrap_or(false)
                        } else {
                            // Native BoardStep uses a 2.5cm contact prediction margin.
                            rapier3d::parry::query::contact(
                                &collider_pose,
                                c.shape(),
                                pose,
                                &**shape,
                                0.025,
                            )
                            .ok()
                            .flatten()
                            .is_some()
                        }
                    })
            })
    }

    pub fn read(&self, id: u64) -> Option<BodySnapshot> {
        let handle = *self.ids.get(&id)?;
        let body = self.bodies.get(handle)?;
        let meta = self.meta.get(&id)?;
        let q = *body.rotation();
        Some(BodySnapshot {
            position: [
                body.translation().x,
                body.translation().y,
                body.translation().z,
            ],
            rotation: [q.x, q.y, q.z, q.w],
            linvel: [body.linvel().x, body.linvel().y, body.linvel().z],
            angvel: [body.angvel().x, body.angvel().y, body.angvel().z],
            body_type: if body.is_dynamic() {
                BodyType::Dynamic
            } else if body.is_kinematic() {
                BodyType::Kinematic
            } else {
                BodyType::Static
            },
            force: meta.commanded_force,
            torque: meta.commanded_torque,
            mass: meta.mass,
        })
    }

    /// Start a force command frame: clear prior commanded forces and Rapier user forces.
    /// Call once per fixed tick before applying new `sdk.physics.force` commands.
    pub fn begin_force_frame(&mut self) {
        for meta in self.meta.values_mut() {
            meta.commanded_force = [0., 0., 0.];
            meta.commanded_torque = [0., 0., 0.];
        }
        self.clear_user_forces();
    }

    pub fn set_pose(&mut self, id: u64, position: [f32; 3], rotation_xyzw: [f32; 4]) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        if position.iter().any(|v| !v.is_finite()) || rotation_xyzw.iter().any(|v| !v.is_finite()) {
            return false;
        }
        let raw = Rotation::from_xyzw(
            rotation_xyzw[0],
            rotation_xyzw[1],
            rotation_xyzw[2],
            rotation_xyzw[3],
        );
        if raw.length_squared() < 1e-8 {
            return false;
        }
        let rot = raw.normalize();
        body.set_translation(Vector::new(position[0], position[1], position[2]), true);
        body.set_rotation(rot, true);
        true
    }

    pub fn apply_force(&mut self, id: u64, force: [f32; 3], point: Option<[f32; 3]>) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        if !body.is_dynamic() {
            return false;
        }
        // Rapier user forces persist across steps until reset; replace rather than
        // stack so each sdk.physics.force call is "this frame's force".
        body.reset_forces(true);
        let f = Vector::new(force[0], force[1], force[2]);
        match point {
            Some(p) => body.add_force_at_point(f, Vector::new(p[0], p[1], p[2]), true),
            None => body.add_force(f, true),
        }
        if let Some(meta) = self.meta.get_mut(&id) {
            meta.commanded_force = force;
        }
        true
    }

    pub fn apply_impulse(&mut self, id: u64, impulse: [f32; 3], point: Option<[f32; 3]>) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        if !body.is_dynamic() {
            return false;
        }
        let j = Vector::new(impulse[0], impulse[1], impulse[2]);
        if j.length_squared() > 1e-12 {
            body.wake_up(true);
        }
        match point {
            Some(p) => body.apply_impulse_at_point(j, Vector::new(p[0], p[1], p[2]), true),
            None => body.apply_impulse(j, true),
        }
        true
    }

    pub fn apply_torque(&mut self, id: u64, torque: [f32; 3]) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        if !body.is_dynamic() {
            return false;
        }
        body.reset_torques(true);
        body.add_torque(Vector::new(torque[0], torque[1], torque[2]), true);
        if let Some(meta) = self.meta.get_mut(&id) {
            meta.commanded_torque = torque;
        }
        true
    }

    pub fn apply_torque_impulse(&mut self, id: u64, torque_impulse: [f32; 3]) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        if !body.is_dynamic() {
            return false;
        }
        body.apply_torque_impulse(
            Vector::new(torque_impulse[0], torque_impulse[1], torque_impulse[2]),
            true,
        );
        true
    }

    pub fn set_linvel(&mut self, id: u64, linvel: [f32; 3]) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        body.set_linvel(Vector::new(linvel[0], linvel[1], linvel[2]), true);
        true
    }

    pub fn set_angvel(&mut self, id: u64, angvel: [f32; 3]) -> bool {
        let Some(&handle) = self.ids.get(&id) else {
            return false;
        };
        let Some(body) = self.bodies.get_mut(handle) else {
            return false;
        };
        body.set_angvel(Vector::new(angvel[0], angvel[1], angvel[2]), true);
        true
    }

    pub fn add_revolute_joint(&mut self, desc: RevoluteJointDesc) -> Result<u64, String> {
        let a = *self
            .ids
            .get(&desc.body_a)
            .ok_or_else(|| format!("unknown body {}", desc.body_a))?;
        let b = *self
            .ids
            .get(&desc.body_b)
            .ok_or_else(|| format!("unknown body {}", desc.body_b))?;
        if desc.body_a == desc.body_b {
            return Err("joint bodies must differ".into());
        }
        let axis = Vector::new(desc.axis[0], desc.axis[1], desc.axis[2]);
        if !(axis.length() > 1e-6) {
            return Err("joint axis must be non-zero".into());
        }
        let mut builder = RevoluteJointBuilder::new(axis.normalize())
            .local_anchor1(Vector::new(
                desc.anchor_a[0],
                desc.anchor_a[1],
                desc.anchor_a[2],
            ))
            .local_anchor2(Vector::new(
                desc.anchor_b[0],
                desc.anchor_b[1],
                desc.anchor_b[2],
            ))
            .contacts_enabled(desc.contacts_enabled);
        if let Some([min, max]) = desc.limits {
            if !min.is_finite() || !max.is_finite() || min > max {
                return Err("invalid revolute limits".into());
            }
            builder = builder.limits([min, max]);
        }
        let handle = self.impulse_joints.insert(a, b, builder.build(), true);
        let id = self.next_joint;
        self.next_joint += 1;
        self.joints.insert(id, handle);
        self.joint_kinds.insert(id, JointKind::Revolute);
        Ok(id)
    }

    pub fn add_prismatic_joint(&mut self, desc: PrismaticJointDesc) -> Result<u64, String> {
        let a = *self
            .ids
            .get(&desc.body_a)
            .ok_or_else(|| format!("unknown body {}", desc.body_a))?;
        let b = *self
            .ids
            .get(&desc.body_b)
            .ok_or_else(|| format!("unknown body {}", desc.body_b))?;
        if desc.body_a == desc.body_b {
            return Err("joint bodies must differ".into());
        }
        let axis = Vector::new(desc.axis[0], desc.axis[1], desc.axis[2]);
        if !(axis.length() > 1e-6) {
            return Err("joint axis must be non-zero".into());
        }
        let mut builder = PrismaticJointBuilder::new(axis.normalize())
            .local_anchor1(Vector::new(
                desc.anchor_a[0],
                desc.anchor_a[1],
                desc.anchor_a[2],
            ))
            .local_anchor2(Vector::new(
                desc.anchor_b[0],
                desc.anchor_b[1],
                desc.anchor_b[2],
            ))
            .contacts_enabled(desc.contacts_enabled);
        if let Some([min, max]) = desc.limits {
            if !min.is_finite() || !max.is_finite() || min > max {
                return Err("invalid prismatic limits".into());
            }
            builder = builder.limits([min, max]);
        }
        let handle = self.impulse_joints.insert(a, b, builder.build(), true);
        let id = self.next_joint;
        self.next_joint += 1;
        self.joints.insert(id, handle);
        self.joint_kinds.insert(id, JointKind::Prismatic);
        Ok(id)
    }

    pub fn set_joint_motor(&mut self, joint_id: u64, motor: JointMotorDesc) -> bool {
        if self.joint_kinds.get(&joint_id) != Some(&JointKind::Revolute) {
            return false;
        }
        let Some(&handle) = self.joints.get(&joint_id) else {
            return false;
        };
        let Some(joint) = self.impulse_joints.get_mut(handle, true) else {
            return false;
        };
        let Some(revolute) = joint.data.as_revolute_mut() else {
            return false;
        };
        let (max_force, active) = match motor {
            JointMotorDesc::Velocity {
                target_velocity,
                factor,
                max_force,
            } => {
                if !target_velocity.is_finite()
                    || !factor.is_finite()
                    || factor < 0.
                    || max_force.is_some_and(|force| !force.is_finite() || force < 0.)
                {
                    return false;
                }
                revolute.set_motor_velocity(target_velocity, factor);
                (max_force, factor > 0.)
            }
            JointMotorDesc::Position {
                target_position,
                stiffness,
                damping,
                max_force,
            } => {
                if !target_position.is_finite()
                    || !stiffness.is_finite()
                    || stiffness < 0.
                    || !damping.is_finite()
                    || damping < 0.
                    || max_force.is_some_and(|force| !force.is_finite() || force < 0.)
                {
                    return false;
                }
                revolute.set_motor_position(target_position, stiffness, damping);
                (max_force, stiffness > 0. || damping > 0.)
            }
        };
        if let Some(max) = max_force {
            revolute.set_motor_max_force(max);
        }
        if active || max_force.is_some_and(|force| force > 0.) {
            let (a, b) = (joint.body1(), joint.body2());
            if let Some(body) = self.bodies.get_mut(a) {
                body.wake_up(true);
            }
            if let Some(body) = self.bodies.get_mut(b) {
                body.wake_up(true);
            }
        }
        true
    }

    /// Current revolute angle in radians, measured by Rapier's joint frames.
    pub fn revolute_angle(&self, joint_id: u64) -> Option<f32> {
        if self.joint_kinds.get(&joint_id) != Some(&JointKind::Revolute) {
            return None;
        }
        let handle = *self.joints.get(&joint_id)?;
        let joint = self.impulse_joints.get(handle)?;
        let revolute = joint.data.as_revolute()?;
        let body_a = self.bodies.get(joint.body1())?;
        let body_b = self.bodies.get(joint.body2())?;
        Some(revolute.angle(&body_a.position().rotation, &body_b.position().rotation))
    }

    pub fn set_joint_spring(&mut self, joint_id: u64, spring: JointSpringDesc) -> bool {
        if self.joint_kinds.get(&joint_id) != Some(&JointKind::Prismatic) {
            return false;
        }
        let Some(&handle) = self.joints.get(&joint_id) else {
            return false;
        };
        let Some(joint) = self.impulse_joints.get_mut(handle, true) else {
            return false;
        };
        if !spring.target_position.is_finite()
            || !spring.stiffness.is_finite()
            || spring.stiffness < 0.
            || !spring.damping.is_finite()
            || spring.damping < 0.
            || spring.max_force.is_some_and(|f| !f.is_finite() || f < 0.)
        {
            return false;
        }
        let Some(prismatic) = joint.data.as_prismatic_mut() else {
            return false;
        };
        prismatic.set_motor_position(spring.target_position, spring.stiffness, spring.damping);
        if let Some(max) = spring.max_force {
            prismatic.set_motor_max_force(max);
        }
        true
    }

    pub fn remove_joint(&mut self, joint_id: u64) -> bool {
        let Some(handle) = self.joints.remove(&joint_id) else {
            return false;
        };
        self.joint_kinds.remove(&joint_id);
        self.impulse_joints.remove(handle, true);
        true
    }

    pub fn raycast(
        &mut self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_toi: f32,
    ) -> Option<RayHit> {
        self.raycast_excluding(origin, direction, max_toi, None)
    }

    pub fn raycast_excluding(
        &mut self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_toi: f32,
        exclude: Option<u64>,
    ) -> Option<RayHit> {
        let dir = Vector::new(direction[0], direction[1], direction[2]);
        let len = dir.length();
        if !(len > 1e-6) || !max_toi.is_finite() || max_toi <= 0. {
            return None;
        }
        let ray = Ray::new(Vector::new(origin[0], origin[1], origin[2]), dir / len);
        let exclude_handle = exclude.and_then(|id| self.ids.get(&id).copied());
        let filter = match exclude_handle {
            Some(h) => QueryFilter::default().exclude_rigid_body(h),
            None => QueryFilter::default(),
        };
        let query = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        );
        let (collider, intersection) = query.cast_ray_and_get_normal(&ray, max_toi, true)?;
        let body = self.colliders.get(collider)?.parent()?;
        let id = *self.reverse.get(&body)?;
        let point = ray.point_at(intersection.time_of_impact);
        Some(RayHit {
            body: id,
            point: [point.x, point.y, point.z],
            normal: [
                intersection.normal.x,
                intersection.normal.y,
                intersection.normal.z,
            ],
            toi: intersection.time_of_impact,
        })
    }

    /// Suspension ray: `solid = false` so anchors flush with terrain still register contact.
    fn spring_raycast_excluding(
        &mut self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_toi: f32,
        exclude: Option<u64>,
    ) -> Option<RayHit> {
        let dir = Vector::new(direction[0], direction[1], direction[2]);
        let len = dir.length();
        if !(len > 1e-6) || !max_toi.is_finite() || max_toi <= 0. {
            return None;
        }
        let ray = Ray::new(Vector::new(origin[0], origin[1], origin[2]), dir / len);
        let exclude_handle = exclude.and_then(|id| self.ids.get(&id).copied());
        let filter = match exclude_handle {
            Some(h) => QueryFilter::default().exclude_rigid_body(h),
            None => QueryFilter::default(),
        };
        let query = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        );
        let (collider, intersection) = query.cast_ray_and_get_normal(&ray, max_toi, true)?;
        let body = self.colliders.get(collider)?.parent()?;
        let id = *self.reverse.get(&body)?;
        let point = ray.point_at(intersection.time_of_impact);
        Some(RayHit {
            body: id,
            point: [point.x, point.y, point.z],
            normal: [
                intersection.normal.x,
                intersection.normal.y,
                intersection.normal.z,
            ],
            toi: intersection.time_of_impact,
        })
    }

    pub fn velocity_at(&self, id: u64, point: [f32; 3]) -> Option<[f32; 3]> {
        let handle = *self.ids.get(&id)?;
        let body = self.bodies.get(handle)?;
        let v = body.velocity_at_point(Vector::new(point[0], point[1], point[2]));
        Some([v.x, v.y, v.z])
    }

    /// Inverse mass along a world-space direction at a world point (for slip solves).
    pub fn effective_inv_mass(&self, id: u64, point: [f32; 3], direction: [f32; 3]) -> Option<f32> {
        let handle = *self.ids.get(&id)?;
        let body = self.bodies.get(handle)?;
        if !body.is_dynamic() {
            return Some(0.);
        }
        let dir = Vector::new(direction[0], direction[1], direction[2]);
        let len = dir.length();
        if !(len > 1e-6) {
            return None;
        }
        let dir = dir / len;
        let p = Vector::new(point[0], point[1], point[2]);
        let lever = (p - body.center_of_mass()).cross(dir);
        let inv = body.mass_properties().local_mprops.inv_mass
            + lever.dot(body.mass_properties().effective_world_inv_inertia * lever);
        Some(inv)
    }

    /// World torque impulse for a body-local angular acceleration over `dt`
    /// (ground-stability / air-control style assists without a vehicle type).
    pub fn local_ang_accel_impulse(
        &self,
        id: u64,
        local_accel: [f32; 3],
        dt: f32,
    ) -> Option<[f32; 3]> {
        use rapier3d::utils::AngularInertiaOps;
        if !(dt.is_finite() && dt > 0.) || local_accel.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let handle = *self.ids.get(&id)?;
        let body = self.bodies.get(handle)?;
        if !body.is_dynamic() {
            return None;
        }
        let rotation = *body.rotation();
        let accel = Vector::new(local_accel[0], local_accel[1], local_accel[2]);
        let angular_impulse =
            body.mass_properties().effective_world_inv_inertia.inverse() * (rotation * accel * dt);
        Some([angular_impulse.x, angular_impulse.y, angular_impulse.z])
    }

    /// Bullet/Rapier ray spring-damper. Applies an impulse at the contact point and
    /// returns load/contact data so Lua can compose tire friction / assists.
    pub fn spring_ray(&mut self, id: u64, desc: SpringRayDesc) -> Option<SpringRayHit> {
        if !(desc.dt.is_finite() && desc.dt > 0.)
            || !(desc.rest_length.is_finite() && desc.rest_length > 0.)
            || !(desc.max_travel.is_finite() && desc.max_travel >= 0.)
            || !(desc.contact_radius.is_finite() && desc.contact_radius >= 0.)
            || !(desc.stiffness.is_finite() && desc.stiffness > 0.)
            || !(desc.compression.is_finite() && desc.compression >= 0.)
            || !(desc.relaxation.is_finite() && desc.relaxation >= 0.)
            || !(desc.max_force.is_finite() && desc.max_force > 0.)
        {
            return None;
        }
        let handle = *self.ids.get(&id)?;
        let (hard_point, direction_ws, mass, raylen) = {
            let body = self.bodies.get(handle)?;
            if !body.is_dynamic() {
                return None;
            }
            let local_dir = Vector::new(
                desc.local_direction[0],
                desc.local_direction[1],
                desc.local_direction[2],
            );
            let local_len = local_dir.length();
            if !(local_len > 1e-6) {
                return None;
            }
            let local_dir = local_dir / local_len;
            let local_origin = Vector::new(
                desc.local_origin[0],
                desc.local_origin[1],
                desc.local_origin[2],
            );
            let pose = *body.position();
            (
                pose * local_origin,
                pose.rotation * local_dir,
                body.mass().max(0.01),
                // Match Rapier/Bullet raycast vehicles: rest + travel + wheel radius.
                desc.rest_length + desc.max_travel + desc.contact_radius,
            )
        };
        // Bias the origin upward so a flush/embedded anchor still finds ground.
        let origin = hard_point - direction_ws * desc.max_travel;
        let hit = self.spring_raycast_excluding(
            [origin.x, origin.y, origin.z],
            [direction_ws.x, direction_ws.y, direction_ws.z],
            raylen + desc.max_travel,
            Some(id),
        );

        let mut result = SpringRayHit {
            in_contact: false,
            point: [
                hard_point.x + direction_ws.x * raylen,
                hard_point.y + direction_ws.y * raylen,
                hard_point.z + direction_ws.z * raylen,
            ],
            normal: [-direction_ws.x, -direction_ws.y, -direction_ws.z],
            hard_point: [hard_point.x, hard_point.y, hard_point.z],
            direction_ws: [direction_ws.x, direction_ws.y, direction_ws.z],
            suspension_length: desc.rest_length,
            load: 0.,
            relative_velocity: 0.,
        };

        let Some(hit) = hit else {
            return Some(result);
        };
        // Origin was biased upward by max_travel along the ray.
        let hit_distance = hit.toi - desc.max_travel;
        let mut suspension_length = hit_distance - desc.contact_radius;
        let min_len = desc.rest_length - desc.max_travel;
        let max_len = desc.rest_length + desc.max_travel;
        suspension_length = suspension_length.clamp(min_len, max_len);
        let normal = Vector::new(hit.normal[0], hit.normal[1], hit.normal[2]);
        let contact = Vector::new(hit.point[0], hit.point[1], hit.point[2]);
        let denominator = normal.dot(direction_ws);
        let chassis_vel = self.bodies.get(handle)?.velocity_at_point(contact);
        let proj_vel = normal.dot(chassis_vel);
        let (rel_vel, clipped_inv) = if denominator >= -0.1 {
            (0., 1. / 0.1)
        } else {
            let inv = -1. / denominator;
            (proj_vel * inv, inv)
        };
        let length_diff = desc.rest_length - suspension_length;
        let mut force = desc.stiffness * length_diff * clipped_inv;
        let damp = if rel_vel < 0. {
            desc.compression
        } else {
            desc.relaxation
        };
        force -= damp * rel_vel;
        // Match Rapier/Bullet raycast suspension: force is already in newtons.
        let mut load = force.max(0.);
        // At full droop the spring force can be ~0 even though the wheel is grounded.
        // Floor to one corner's static share so tire slip has grip when `in_contact`.
        let static_per_wheel = mass * 9.81 / 4.;
        if load < static_per_wheel {
            load = static_per_wheel;
        }
        if load > desc.max_force {
            load = desc.max_force;
        }
        result.in_contact = true;
        result.point = hit.point;
        result.normal = hit.normal;
        result.suspension_length = suspension_length;
        result.load = load;
        result.relative_velocity = rel_vel;
        let impulse = normal * load * desc.dt;
        self.bodies
            .get_mut(handle)?
            .apply_impulse_at_point(impulse, contact, true);
        Some(result)
    }

    pub fn drain_contacts(&mut self) -> Vec<ContactEvent> {
        std::mem::take(&mut self.contacts)
    }

    /// Current touching body pairs from the last `step` (not edge events).
    pub fn active_contact_pairs(&self) -> Vec<(u64, u64)> {
        self.prev_pairs.keys().copied().collect()
    }

    /// Raycast that only returns a hit on the static map trimesh (`GROUND_BODY_ID`).
    pub fn raycast_ground(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_toi: f32,
    ) -> Option<RayHit> {
        let &ground = self.ids.get(&GROUND_BODY_ID)?;
        let ground_body = self.bodies.get(ground)?;
        let dir = Vector::new(direction[0], direction[1], direction[2]);
        let len = dir.length();
        if !(len > 1e-6) || !max_toi.is_finite() || max_toi <= 0. {
            return None;
        }
        let ray = Ray::new(Vector::new(origin[0], origin[1], origin[2]), dir / len);
        // Query the ground collider directly. This works immediately after
        // `set_ground`; broad-phase queries are not populated until the first step.
        let intersection = ground_body
            .colliders()
            .iter()
            .filter_map(|handle| {
                let collider = self.colliders.get(*handle)?;
                collider
                    .shape()
                    .cast_ray_and_get_normal(collider.position(), &ray, max_toi, true)
            })
            .min_by(|a, b| a.time_of_impact.total_cmp(&b.time_of_impact))?;
        let point = ray.point_at(intersection.time_of_impact);
        Some(RayHit {
            body: GROUND_BODY_ID,
            point: [point.x, point.y, point.z],
            normal: [
                intersection.normal.x,
                intersection.normal.y,
                intersection.normal.z,
            ],
            toi: intersection.time_of_impact,
        })
    }

    /// Raycast that skips any hit whose body id is in `exclude`.
    pub fn raycast_excluding_bodies(
        &mut self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_toi: f32,
        exclude: &[u64],
    ) -> Option<RayHit> {
        if exclude.is_empty() {
            return self.raycast(origin, direction, max_toi);
        }
        let dir = Vector::new(direction[0], direction[1], direction[2]);
        let len = dir.length();
        if !(len > 1e-6) || !max_toi.is_finite() || max_toi <= 0. {
            return None;
        }
        let dir = dir / len;
        let mut origin = Vector::new(origin[0], origin[1], origin[2]);
        let mut remaining = max_toi;
        for _ in 0..16 {
            let hit = self.raycast(
                [origin.x, origin.y, origin.z],
                [dir.x, dir.y, dir.z],
                remaining,
            )?;
            if !exclude.contains(&hit.body) {
                return Some(hit);
            }
            let advance = hit.toi + 0.15;
            if advance >= remaining - 1e-4 {
                return None;
            }
            remaining -= advance;
            origin = Vector::new(
                hit.point[0] + dir.x * 0.15,
                hit.point[1] + dir.y * 0.15,
                hit.point[2] + dir.z * 0.15,
            );
        }
        None
    }

    pub fn step(&mut self, dt: f32) {
        if !(dt.is_finite() && dt > 0.) {
            return;
        }
        let mut integration = IntegrationParameters::default();
        let steps = ((dt / 0.008_334).ceil() as usize).clamp(1, 16);
        integration.dt = dt / steps as f32;
        for _ in 0..steps {
            self.advance_kinematic_targets(integration.dt);
            self.pipeline.step(
                self.gravity,
                &integration,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd,
                &(),
                &(),
            );
            self.deform_impacts(integration.dt);
        }
        // Rapier keeps user forces/torques until reset; clear after the frame so
        // sdk.physics.force must be re-issued each fixed tick (sandbox WASD pattern).
        self.clear_user_forces();
        self.collect_contacts();
    }

    fn clear_user_forces(&mut self) {
        for (_, body) in self.bodies.iter_mut() {
            if body.is_dynamic() {
                body.reset_forces(false);
                body.reset_torques(false);
            }
        }
    }

    pub fn iter_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.ids.keys().copied()
    }

    fn collect_contacts(&mut self) {
        let mut current = BTreeMap::new();
        let mut push_pair = |c1: ColliderHandle, c2: ColliderHandle| {
            let Some(ca) = self.colliders.get(c1) else {
                return;
            };
            let Some(cb) = self.colliders.get(c2) else {
                return;
            };
            let (Some(pa), Some(pb)) = (ca.parent(), cb.parent()) else {
                return;
            };
            let (Some(&a), Some(&b)) = (self.reverse.get(&pa), self.reverse.get(&pb)) else {
                return;
            };
            let key = if a < b { (a, b) } else { (b, a) };
            current.insert(key, ());
        };
        for pair in self.narrow_phase.contact_pairs() {
            push_pair(pair.collider1, pair.collider2);
        }
        for (c1, c2, intersecting) in self.narrow_phase.intersection_pairs() {
            if intersecting {
                push_pair(c1, c2);
            }
        }
        for key in current.keys() {
            if !self.prev_pairs.contains_key(key) {
                self.contacts.push(ContactEvent {
                    body_a: key.0,
                    body_b: key.1,
                    started: true,
                });
            }
        }
        for key in self.prev_pairs.keys() {
            if !current.contains_key(key) {
                self.contacts.push(ContactEvent {
                    body_a: key.0,
                    body_b: key.1,
                    started: false,
                });
            }
        }
        self.prev_pairs = current;
    }

    fn validate(&self, desc: &BodyDesc) -> Result<(), String> {
        if desc.contact_group > 20 {
            return Err("contact_group must be in 0..=20".into());
        }
        if !desc.mass.is_finite() || desc.mass <= 0. {
            return Err("mass must be finite and > 0".into());
        }
        if !desc.friction.is_finite() || !(0. ..=2.).contains(&desc.friction) {
            return Err("friction must be in 0..2".into());
        }
        if !desc.heading.is_finite() {
            return Err("heading must be finite".into());
        }
        if !desc.linear_damping.is_finite()
            || desc.linear_damping < 0.
            || !desc.angular_damping.is_finite()
            || desc.angular_damping < 0.
        {
            return Err("damping must be finite and >= 0".into());
        }
        if desc
            .center_of_mass
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 10.)
        {
            return Err("center_of_mass out of range".into());
        }
        if desc
            .collider_offset
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 10.)
        {
            return Err("collider_offset out of range".into());
        }
        if let Some(half) = desc.inertia_half_extents {
            if half.iter().any(|v| !v.is_finite() || *v <= 0. || *v > 10.) {
                return Err("inertia_half_extents invalid".into());
            }
        }
        if desc
            .position
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 100_000.)
        {
            return Err("position out of range".into());
        }
        match &desc.shape {
            Shape::Box { half_extents } => {
                if half_extents
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0. || *v > 100.)
                {
                    return Err("box half_extents invalid".into());
                }
            }
            Shape::Sphere { radius } | Shape::Capsule { radius, .. } => {
                if !radius.is_finite() || *radius <= 0. || *radius > 100. {
                    return Err("radius invalid".into());
                }
            }
            Shape::Convex { points } => {
                if points.len() < 4 || points.len() > 512 {
                    return Err("convex needs 4..=512 points".into());
                }
                if points
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v.abs() > 1000.)
                {
                    return Err("convex points invalid".into());
                }
            }
            Shape::Compound { hulls } => model::validate_hulls(hulls)?,
            Shape::Model {
                path,
                object,
                options,
            } => {
                options.validate()?;
                if path.is_empty()
                    || path.len() > 256
                    || path.contains("..")
                    || path.contains('\\')
                    || path.contains(':')
                    || object.len() > 120
                {
                    return Err("model path/object invalid".into());
                }
            }
            Shape::Mesh { path, object } => {
                if path.is_empty()
                    || path.len() > 256
                    || path.contains("..")
                    || path.contains('\\')
                    || object.is_empty()
                    || object.len() > 120
                {
                    return Err("mesh path/object invalid".into());
                }
            }
        }
        if let Shape::Capsule { half_height, .. } = &desc.shape {
            if !half_height.is_finite() || *half_height <= 0. || *half_height > 100. {
                return Err("capsule half_height invalid".into());
            }
        }
        Ok(())
    }

    fn collider(&self, desc: &BodyDesc) -> Result<Collider, String> {
        let builder = match &desc.shape {
            Shape::Box { half_extents } => {
                ColliderBuilder::cuboid(half_extents[0], half_extents[1], half_extents[2])
            }
            Shape::Sphere { radius } => ColliderBuilder::ball(*radius),
            Shape::Capsule {
                half_height,
                radius,
            } => ColliderBuilder::capsule_y(*half_height, *radius),
            Shape::Convex { points } => {
                let pts: Vec<Vector> = points
                    .iter()
                    .map(|p| Vector::new(p[0], p[1], p[2]))
                    .collect();
                ColliderBuilder::convex_hull(&pts)
                    .ok_or_else(|| "convex hull failed (degenerate points)".to_string())?
            }
            Shape::Compound { hulls } => ColliderBuilder::new(model::compound_shape(hulls)?),
            Shape::Mesh { .. } | Shape::Model { .. } => {
                return Err("model/mesh shape must be resolved by the host before spawn".into());
            }
        };
        let membership = Group::from_bits_truncate(desc.membership);
        let filter = Group::from_bits_truncate(desc.filter);
        let offset = Vector::new(
            desc.collider_offset[0],
            desc.collider_offset[1],
            desc.collider_offset[2],
        );
        let builder = builder
            .translation(offset)
            .friction(desc.friction)
            .sensor(desc.sensor)
            .collision_groups(InteractionGroups::new(
                membership,
                filter,
                InteractionTestMode::And,
            ));
        let builder = if desc.body_type == BodyType::Dynamic {
            let mass = desc.mass.max(0.01);
            let use_props = desc.inertia_half_extents.is_some()
                || desc.center_of_mass.iter().any(|v| *v != 0.)
                || desc.collider_offset.iter().any(|v| *v != 0.);
            if use_props {
                let [x, y, z] = desc
                    .inertia_half_extents
                    .unwrap_or_else(|| inertia_half_from_shape(&desc.shape));
                let inertia =
                    Vector::new(y * y + z * z, x * x + z * z, x * x + y * y) * (mass / 3.);
                let com = Vector::new(
                    desc.center_of_mass[0] - desc.collider_offset[0],
                    desc.center_of_mass[1] - desc.collider_offset[1],
                    desc.center_of_mass[2] - desc.collider_offset[2],
                );
                let props = MassProperties::new(com, mass, inertia);
                builder.mass_properties(props)
            } else {
                builder.mass(mass)
            }
        } else {
            builder.density(0.)
        };
        Ok(builder.build())
    }
}

/// Triangulated convex hull of `points + translation`, in the body frame.
/// Decimates the input until the face count fits `max_tris`, so a dense GLB hull
/// still exports its real silhouette rather than collapsing to a primitive.
fn hull_triangles(
    points: &[[f32; 3]],
    translation: [f32; 3],
    max_tris: usize,
) -> Option<Vec<[[f32; 3]; 3]>> {
    use rapier3d::parry::shape::ConvexPolyhedron;
    if points.len() < 4 || points.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    for stride in [1usize, 2, 3, 5, 8, 13] {
        let pts: Vec<Vector> = points
            .iter()
            .step_by(stride)
            .map(|p| {
                Vector::new(
                    p[0] + translation[0],
                    p[1] + translation[1],
                    p[2] + translation[2],
                )
            })
            .collect();
        if pts.len() < 4 {
            break;
        }
        let Some(poly) = ConvexPolyhedron::from_convex_hull(&pts) else {
            continue;
        };
        let (verts, indices) = poly.to_trimesh();
        if indices.is_empty() || indices.len() > max_tris {
            continue;
        }
        let tris: Vec<[[f32; 3]; 3]> = indices
            .iter()
            .filter_map(|[a, b, c]| {
                let p = [
                    *verts.get(*a as usize)?,
                    *verts.get(*b as usize)?,
                    *verts.get(*c as usize)?,
                ];
                // Degenerate faces produce NaN normals downstream; drop them here.
                let area = (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
                if !area.is_finite() || area < 1e-7 {
                    return None;
                }
                Some([
                    [p[0].x, p[0].y, p[0].z],
                    [p[1].x, p[1].y, p[1].z],
                    [p[2].x, p[2].y, p[2].z],
                ])
            })
            .collect();
        if !tris.is_empty() {
            return Some(tris);
        }
    }
    None
}

fn inertia_half_from_shape(shape: &Shape) -> [f32; 3] {
    match shape {
        Shape::Box { half_extents } => *half_extents,
        Shape::Sphere { radius } => [*radius, *radius, *radius],
        Shape::Capsule {
            half_height,
            radius,
        } => [*radius, *half_height + *radius, *radius],
        Shape::Convex { points } => {
            let mut mn = [f32::MAX; 3];
            let mut mx = [f32::MIN; 3];
            for p in points {
                for i in 0..3 {
                    mn[i] = mn[i].min(p[i]);
                    mx[i] = mx[i].max(p[i]);
                }
            }
            [
                ((mx[0] - mn[0]) * 0.5).max(0.05),
                ((mx[1] - mn[1]) * 0.5).max(0.05),
                ((mx[2] - mn[2]) * 0.5).max(0.05),
            ]
        }
        Shape::Compound { hulls } => {
            let mut min = [f32::MAX; 3];
            let mut max = [f32::MIN; 3];
            for p in hulls.iter().flatten() {
                for axis in 0..3 {
                    min[axis] = min[axis].min(p[axis]);
                    max[axis] = max[axis].max(p[axis]);
                }
            }
            std::array::from_fn(|axis| ((max[axis] - min[axis]) * 0.5).max(0.001))
        }
        Shape::Mesh { .. } | Shape::Model { .. } => [0.5, 0.5, 0.5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_shapes_detect_sensor_intersection_and_solid_contact_margin() {
        let mut world = DynamicsWorld::default();
        let sensor = world
            .spawn(BodyDesc {
                sensor: true,
                ..box_desc(BodyType::Static, [0., 40., 0.], [1., 0.5, 1.])
            })
            .unwrap();
        let solid = world
            .spawn(box_desc(BodyType::Static, [0., 40., 0.], [1., 0.5, 1.]))
            .unwrap();
        let shape = SharedShape::ball(0.1);
        let near = [(
            shape.clone(),
            Pose::from_translation(Vector::new(0., 40.61, 0.)),
        )];
        assert!(!world.overlaps_shapes(sensor, &near));
        assert!(world.overlaps_shapes(solid, &near));
        let inside = [(shape, Pose::from_translation(Vector::new(0., 40.55, 0.)))];
        assert!(world.overlaps_shapes(sensor, &inside));
        assert!(!world.overlaps_shapes(sensor, &[]));
        assert!(world.set_pose(sensor, [10., 40., 0.], [0., 0., 0., 1.]));
        assert!(!world.overlaps_shapes(sensor, &inside));
    }

    fn box_desc(body_type: BodyType, position: [f32; 3], half: [f32; 3]) -> BodyDesc {
        BodyDesc {
            shape: Shape::Box { half_extents: half },
            body_type,
            mass: 10.,
            position,
            friction: 0.8,
            ccd: true,
            ..Default::default()
        }
    }

    /// Both triangles wound so normals face +Y (playable side).
    fn flat_ground(world: &mut DynamicsWorld) {
        let tris = [
            [[-40., 0., -40.], [40., 0., -40.], [40., 0., 40.]],
            [[-40., 0., -40.], [-40., 0., 40.], [40., 0., 40.]],
        ];
        world.set_ground(tris.into_iter()).unwrap();
    }

    fn speed(snap: &BodySnapshot) -> f32 {
        (snap.linvel[0].powi(2) + snap.linvel[1].powi(2) + snap.linvel[2].powi(2)).sqrt()
    }

    fn settle_crate(world: &mut DynamicsWorld, mass: f32) -> u64 {
        // Offset from the diagonal seam so contact is unambiguous.
        let id = world
            .spawn(BodyDesc {
                mass,
                friction: 0.85,
                ccd: true,
                ..box_desc(BodyType::Dynamic, [1., 3., 1.], [0.5, 0.5, 0.5])
            })
            .unwrap();
        for _ in 0..180 {
            world.step(1. / 60.);
        }
        let snap = world.read(id).unwrap();
        assert!(
            snap.position[1] > 0.35 && snap.position[1] < 1.25 && speed(&snap) < 0.75,
            "crate should rest on ground, y={} speed={}",
            snap.position[1],
            speed(&snap)
        );
        id
    }

    #[test]
    fn box_falls_onto_ground_plane() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        let id = settle_crate(&mut world, 10.);
        let snap = world.read(id).unwrap();
        assert!(
            (snap.position[1] - 0.5).abs() < 0.35,
            "rest height should be ~half-extent, y={}",
            snap.position[1]
        );
    }

    #[test]
    fn ground_triangles_support_origin_diagonal() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        let id = world
            .spawn(box_desc(BodyType::Dynamic, [0., 2., 0.], [0.5, 0.5, 0.5]))
            .unwrap();
        for _ in 0..180 {
            world.step(1. / 60.);
        }
        let snap = world.read(id).unwrap();
        assert!(
            snap.position[1] > 0.35 && snap.position[1] < 1.25,
            "must not fall through seam at origin, y={}",
            snap.position[1]
        );
    }

    #[test]
    fn revolute_motor_spins_body() {
        let mut world = DynamicsWorld::default();
        let base = world
            .spawn(box_desc(BodyType::Static, [0., 1., 0.], [0.2, 0.2, 0.2]))
            .unwrap();
        let rotor = world
            .spawn(BodyDesc {
                mass: 2.,
                friction: 0.1,
                ccd: false,
                ..box_desc(BodyType::Dynamic, [0.6, 1., 0.], [0.5, 0.1, 0.1])
            })
            .unwrap();
        let joint = world
            .add_revolute_joint(RevoluteJointDesc {
                body_a: base,
                body_b: rotor,
                anchor_a: [0.3, 0., 0.],
                anchor_b: [-0.3, 0., 0.],
                axis: [0., 0., 1.],
                limits: None,
                contacts_enabled: false,
            })
            .unwrap();
        assert!(world.set_joint_motor(
            joint,
            JointMotorDesc::Velocity {
                target_velocity: 4.,
                factor: 20.,
                max_force: Some(50.),
            }
        ));
        for _ in 0..120 {
            world.step(1. / 60.);
        }
        let snap = world.read(rotor).unwrap();
        let spin =
            (snap.angvel[0].powi(2) + snap.angvel[1].powi(2) + snap.angvel[2].powi(2)).sqrt();
        assert!(spin > 0.5, "motor should spin rotor, angvel={spin}");
    }

    #[test]
    fn sensor_reports_contact() {
        let mut world = DynamicsWorld::default();
        let _static_box = world
            .spawn(box_desc(BodyType::Static, [0., 0., 0.], [1., 1., 1.]))
            .unwrap();
        let sensor = world
            .spawn(BodyDesc {
                shape: Shape::Sphere { radius: 0.5 },
                body_type: BodyType::Dynamic,
                mass: 1.,
                position: [0., 3., 0.],
                friction: 0.5,
                ccd: true,
                sensor: true,
                ..Default::default()
            })
            .unwrap();
        let mut saw = false;
        for _ in 0..180 {
            world.step(1. / 60.);
            for c in world.drain_contacts() {
                if c.started && (c.body_a == sensor || c.body_b == sensor) {
                    saw = true;
                }
            }
        }
        assert!(saw, "sensor should report contact with static box");
    }

    #[test]
    fn export_boardworld_volumes_primitives() {
        let mut world = DynamicsWorld::default();
        let box_id = world
            .spawn(box_desc(BodyType::Dynamic, [1., 2., 3.], [0.5, 0.4, 0.3]))
            .unwrap();
        let sphere_id = world
            .spawn(BodyDesc {
                shape: Shape::Sphere { radius: 0.25 },
                body_type: BodyType::Kinematic,
                mass: 2.,
                position: [0., 1., 0.],
                friction: 0.5,
                ..Default::default()
            })
            .unwrap();
        assert!(world.set_collision_mesh(
            box_id,
            vec![
                [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                [[0., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            ],
        ));
        let exports = world.export_boardworld_volumes();
        assert_eq!(exports.len(), 2);
        let box_export = exports.iter().find(|e| e.body == box_id).unwrap();
        assert!(matches!(box_export.shape, ExportedShape::Triangles { .. }));
        let sphere_export = exports.iter().find(|e| e.body == sphere_id).unwrap();
        assert!(matches!(
            sphere_export.shape,
            ExportedShape::Sphere { radius } if (radius - 0.25).abs() < 1e-6
        ));
        world.remove(box_id);
        assert_eq!(world.export_boardworld_volumes().len(), 1);
    }

    /// Convex bodies (and their extra hulls) must reach BoardWorld as real hull
    /// faces; a bounding box would make an authored shape feel like a crate.
    #[test]
    fn spring_ray_finds_ground_within_max_travel() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        let id = world
            .spawn(BodyDesc {
                shape: Shape::Box {
                    half_extents: [0.5, 0.2, 1.],
                },
                position: [0., 0.55, 0.],
                mass: 1560.,
                ..Default::default()
            })
            .unwrap();
        world.step(1. / 120.);
        let direct = world.raycast_excluding([0., 0.8, 0.], [0., -1., 0.], 1.5, Some(id));
        assert!(direct.is_some(), "ground ray must hit: {direct:?}");
        let hit = world
            .spring_ray(
                id,
                SpringRayDesc {
                    local_origin: [0., 0., 0.],
                    local_direction: [0., -1., 0.],
                    rest_length: 0.25,
                    max_travel: 0.25,
                    contact_radius: 0.34,
                    stiffness: 30.,
                    compression: 4.,
                    relaxation: 4.,
                    max_force: 6000.,
                    dt: 1. / 120.,
                },
            )
            .expect("spring_ray");
        assert!(
            hit.in_contact,
            "suspension must reach ground within rest+travel+radius (direct={direct:?}, hit={hit:?})"
        );
        assert!(hit.load > 0., "grounded wheel must carry load");
    }

    #[test]
    fn spring_ray_at_full_droop_still_carries_static_load() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        // Body high enough that suspension sits at max droop (length_diff <= 0).
        let id = world
            .spawn(BodyDesc {
                shape: Shape::Box {
                    half_extents: [0.5, 0.2, 1.],
                },
                position: [0., 1.2, 0.],
                mass: 1560.,
                ..Default::default()
            })
            .unwrap();
        world.step(1. / 120.);
        let hit = world
            .spring_ray(
                id,
                SpringRayDesc {
                    local_origin: [0., 0., 0.],
                    local_direction: [0., -1., 0.],
                    rest_length: 0.25,
                    max_travel: 1.,
                    contact_radius: 0.34,
                    stiffness: 30.,
                    compression: 4.,
                    relaxation: 4.,
                    max_force: 6000.,
                    dt: 1. / 120.,
                },
            )
            .expect("spring_ray");
        assert!(hit.in_contact, "ray must still hit ground at droop");
        let min = 1560. * 9.81 / 4.;
        assert!(
            hit.load >= min * 0.99,
            "droop contact must floor load, got {}",
            hit.load
        );
    }

    #[test]
    fn convex_body_and_extra_hulls_export_hull_triangles() {
        let mut world = DynamicsWorld::default();
        let wedge = vec![
            [-1., -0.25, -2.],
            [1., -0.25, -2.],
            [1., -0.25, 2.],
            [-1., -0.25, 2.],
            [-0.6, 0.5, -0.5],
            [0.6, 0.5, -0.5],
            [0.6, 0.4, 1.2],
            [-0.6, 0.4, 1.2],
        ];
        let id = world
            .spawn(BodyDesc {
                shape: Shape::Convex {
                    points: wedge.clone(),
                },
                body_type: BodyType::Dynamic,
                mass: 1560.,
                position: [0., 2., 0.],
                friction: 0.05,
                center_of_mass: [0., -0.24, -0.37],
                inertia_half_extents: Some([1.02, 0.61, 2.43]),
                ..Default::default()
            })
            .unwrap();
        let tire: Vec<[f32; 3]> = vec![
            [-0.1, -0.34, -0.34],
            [0.1, -0.34, -0.34],
            [0.1, 0.34, -0.34],
            [-0.1, 0.34, -0.34],
            [-0.1, 0., 0.34],
            [0.1, 0., 0.34],
        ];
        world
            .add_convex_collider(id, &tire, [0.8, -0.08, -1.35], 1.3)
            .unwrap();

        let exports = world.export_boardworld_volumes();
        assert_eq!(exports.len(), 2, "chassis hull plus one tire hull");
        for export in &exports {
            let ExportedShape::Triangles { tris } = &export.shape else {
                panic!("convex must export hull triangles, got {:?}", export.shape);
            };
            assert!(tris.len() >= 4 && tris.len() <= MAX_EXPORT_TRIANGLES);
            assert!(tris.iter().flatten().flatten().all(|v| v.is_finite()));
        }
        // The tire hull keeps its collider offset in the body frame.
        let tire_export = exports
            .iter()
            .find_map(|e| match &e.shape {
                ExportedShape::Triangles { tris } if tris.iter().flatten().all(|v| v[2] < -0.5) => {
                    Some(e)
                }
                _ => None,
            })
            .expect("offset tire hull present");
        assert!((tire_export.friction - 1.3).abs() < 1e-6);
    }

    #[test]
    fn horizontal_force_stops_accelerating_after_release() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        let id = settle_crate(&mut world, 25.);

        for _ in 0..45 {
            world.begin_force_frame();
            // Match host: one force command per tick. Use impulse-scale force so
            // friction cannot hide a regression.
            assert!(world.apply_force(id, [500., 0., 0.], None));
            assert_eq!(world.read(id).unwrap().force, [500., 0., 0.]);
            world.step(1. / 60.);
        }
        let after_push = world.read(id).unwrap();
        let push_speed = after_push.linvel[0].abs();
        assert!(
            push_speed > 0.4,
            "push should move the box in +X, vx={} force={:?}",
            after_push.linvel[0],
            after_push.force
        );
        assert!(
            after_push.position[1] > 0.3,
            "must stay on ground while pushed, y={}",
            after_push.position[1]
        );

        for _ in 0..120 {
            world.begin_force_frame();
            assert_eq!(world.read(id).unwrap().force, [0., 0., 0.]);
            world.step(1. / 60.);
        }
        let coast = world.read(id).unwrap();
        let coast_vx = coast.linvel[0].abs();
        assert!(
            coast_vx <= push_speed + 0.15,
            "released force must not keep accelerating: push_vx={push_speed} coast_vx={coast_vx}"
        );
        assert!(
            coast.position[1] > 0.3,
            "must remain on ground after coast, y={}",
            coast.position[1]
        );
    }

    #[test]
    fn repeating_same_force_each_tick_stays_bounded() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        let id = settle_crate(&mut world, 25.);
        let mut peak_vx = 0f32;
        for _ in 0..180 {
            world.begin_force_frame();
            assert!(world.apply_force(id, [500., 0., 0.], None));
            world.step(1. / 60.);
            peak_vx = peak_vx.max(world.read(id).unwrap().linvel[0].abs());
        }
        assert!(
            peak_vx < 40.,
            "constant force must not runaway; peak |vx|={peak_vx}"
        );
        let end = world.read(id).unwrap();
        assert!(
            end.position[1] > 0.3 && end.position.iter().all(|v| v.is_finite()),
            "must stay on ground, y={}",
            end.position[1]
        );
    }

    #[test]
    fn commanded_force_visible_on_read_until_next_frame() {
        let mut world = DynamicsWorld::default();
        let id = world
            .spawn(box_desc(BodyType::Dynamic, [0., 5., 0.], [0.5, 0.5, 0.5]))
            .unwrap();
        world.begin_force_frame();
        assert!(world.apply_force(id, [12., -3., 4.], None));
        let snap = world.read(id).unwrap();
        assert_eq!(snap.force, [12., -3., 4.]);
        assert_eq!(snap.mass, 10.);
        world.step(1. / 60.);
        // Survives step so the next fixed snapshot can report what was applied.
        assert_eq!(world.read(id).unwrap().force, [12., -3., 4.]);
        world.begin_force_frame();
        assert_eq!(world.read(id).unwrap().force, [0., 0., 0.]);
    }

    #[test]
    fn idle_box_does_not_drift_on_flat_ground() {
        let mut world = DynamicsWorld::default();
        flat_ground(&mut world);
        let id = settle_crate(&mut world, 25.);
        let mid = world.read(id).unwrap();
        for _ in 0..300 {
            world.step(1. / 60.);
        }
        let end = world.read(id).unwrap();
        let drift = ((end.position[0] - mid.position[0]).powi(2)
            + (end.position[2] - mid.position[2]).powi(2))
        .sqrt();
        assert!(
            drift < 0.35 && speed(&end) < 0.35 && end.position[1] > 0.3,
            "idle crate drifted drift={drift} speed={} y={}",
            speed(&end),
            end.position[1]
        );
    }

    #[test]
    fn force_then_zero_frames_clears_user_force() {
        let mut world = DynamicsWorld::default();
        let id = world
            .spawn(BodyDesc {
                mass: 10.,
                friction: 0.,
                ccd: false,
                ..box_desc(BodyType::Dynamic, [0., 10., 0.], [0.25, 0.25, 0.25])
            })
            .unwrap();
        assert!(world.apply_force(id, [100., 0., 0.], None));
        world.step(1. / 60.);
        let vx_after = world.read(id).unwrap().linvel[0];
        assert!(
            vx_after > 0.1,
            "first step should gain +X velocity, vx={vx_after}"
        );
        let mut prev = vx_after;
        for i in 0..30 {
            world.begin_force_frame();
            world.step(1. / 60.);
            let vx = world.read(id).unwrap().linvel[0];
            assert!(
                vx <= prev + 0.05,
                "without re-applying force, vx must not keep rising (step {i}): prev={prev} vx={vx}"
            );
            prev = vx;
        }
    }

    #[test]
    fn rapier_add_force_without_reset_would_runaway() {
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let mut impulse_joints = ImpulseJointSet::new();
        let mut multibody_joints = MultibodyJointSet::new();
        let mut islands = IslandManager::new();
        let mut broad = DefaultBroadPhase::new();
        let mut narrow = NarrowPhase::new();
        let mut ccd = CCDSolver::new();
        let mut pipeline = PhysicsPipeline::new();
        let gravity = Vector::new(0., 0., 0.);
        let integration = IntegrationParameters::default();
        let handle = bodies.insert(
            RigidBodyBuilder::dynamic()
                .linear_damping(0.)
                .angular_damping(0.)
                .build(),
        );
        colliders.insert_with_parent(
            ColliderBuilder::ball(0.5).mass(1.).build(),
            handle,
            &mut bodies,
        );
        let mut peak = 0f32;
        for _ in 0..60 {
            bodies
                .get_mut(handle)
                .unwrap()
                .add_force(Vector::new(10., 0., 0.), true);
            pipeline.step(
                gravity,
                &integration,
                &mut islands,
                &mut broad,
                &mut narrow,
                &mut bodies,
                &mut colliders,
                &mut impulse_joints,
                &mut multibody_joints,
                &mut ccd,
                &(),
                &(),
            );
            peak = peak.max(bodies.get(handle).unwrap().linvel().x.abs());
        }
        assert!(
            peak > 50.,
            "control: raw Rapier add_force stacks (peak vx={peak}); DynamicsWorld must clear"
        );
    }
}
