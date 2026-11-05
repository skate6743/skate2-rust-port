//! Bounded, impact-driven plasticity shared by render meshes and solid colliders.
//! A low-resolution material lattice is sampled against the actual obstacle.
//! Resting bodies have no deformation work; no per-frame mesh uploads or VHACD.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// Plastic yield: contact impulse divided by this body's mass, in m/s.
    pub yield_speed: f32,
    /// Metres of permanent displacement per m/s above yield.
    pub compliance: f32,
    pub radius: f32,
    pub max_displacement: f32,
    pub max_step: f32,
    pub cooldown: f32,
    pub resolution: [usize; 3],
}
impl Default for Options {
    fn default() -> Self {
        Self {
            yield_speed: 2.,
            compliance: 0.045,
            radius: 1.2,
            max_displacement: 0.55,
            max_step: 0.22,
            cooldown: 0.10,
            resolution: [9, 5, 17],
        }
    }
}
impl Options {
    pub fn validate(&self) -> Result<(), String> {
        if !self.yield_speed.is_finite()
            || !(0.5..=100.).contains(&self.yield_speed)
            || !self.compliance.is_finite()
            || !(0.001..=0.2).contains(&self.compliance)
            || !self.radius.is_finite()
            || !(0.1..=5.).contains(&self.radius)
            || !self.max_displacement.is_finite()
            || !(0.01..=2.).contains(&self.max_displacement)
            || !self.max_step.is_finite()
            || !(0.001..=self.max_displacement).contains(&self.max_step)
            || !self.cooldown.is_finite()
            || !(0.05..=2.).contains(&self.cooldown)
            || self.resolution.iter().any(|n| !(2..=25).contains(n))
            || self.resolution.iter().product::<usize>() > 2048
        {
            return Err("invalid bounded deformation options".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub resolution: [usize; 3],
    pub offsets: Vec<[f32; 3]>,
    pub revision: u64,
}
impl Field {
    pub fn validate(&self) -> Result<(), String> {
        if self.resolution.iter().any(|n| !(2..=25).contains(n)) {
            return Err("invalid deformation resolution".into());
        }
        let count = self.resolution.iter().product::<usize>();
        if count > 2048
            || self.offsets.len() != count
            || self
                .min
                .iter()
                .chain(&self.max)
                .any(|v| !v.is_finite() || v.abs() > 1000.)
            || (0..3).any(|i| self.max[i] - self.min[i] < 0.001)
            || self
                .offsets
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 2.)
        {
            return Err("invalid deformation field".into());
        }
        Ok(())
    }
    fn index(&self, xyz: [usize; 3]) -> usize {
        (xyz[2] * self.resolution[1] + xyz[1]) * self.resolution[0] + xyz[0]
    }
    pub fn rest(&self, i: usize) -> Vector {
        let [nx, ny, _] = self.resolution;
        let xyz = [i % nx, (i / nx) % ny, i / (nx * ny)];
        Vector::from_array(std::array::from_fn(|a| {
            self.min[a]
                + (self.max[a] - self.min[a]) * xyz[a] as f32 / (self.resolution[a] - 1) as f32
        }))
    }
    /// Cached once per render vertex; no searches or Lua tables during updates.
    pub fn weights(&self, p: [f32; 3]) -> ([usize; 8], [f32; 8]) {
        let v: [f32; 3] = std::array::from_fn(|a| {
            ((p[a] - self.min[a]) / (self.max[a] - self.min[a])).clamp(0., 1.)
                * (self.resolution[a] - 1) as f32
        });
        let base: [usize; 3] = std::array::from_fn(|a| (v[a] as usize).min(self.resolution[a] - 2));
        let frac: [f32; 3] = std::array::from_fn(|a| v[a] - base[a] as f32);
        let ids =
            std::array::from_fn(|i| self.index(std::array::from_fn(|a| base[a] + ((i >> a) & 1))));
        let weights = std::array::from_fn(|i| {
            (0..3)
                .map(|a| {
                    if (i >> a) & 1 == 0 {
                        1. - frac[a]
                    } else {
                        frac[a]
                    }
                })
                .product()
        });
        (ids, weights)
    }
    pub fn displacement(&self, p: [f32; 3]) -> Vector {
        let (ids, weights) = self.weights(p);
        (0..8).fold(Vector::ZERO, |v, i| {
            v + Vector::from_array(self.offsets[ids[i]]) * weights[i]
        })
    }
    /// Radial falloff bounds the work, but only rays hitting actual obstacle
    /// geometry cause displacement. A narrow post cannot produce a wall dent.
    fn dent(
        &mut self,
        pose: Pose,
        point: Vector,
        inward: Vector,
        depth: f32,
        obstacle: &Collider,
        options: &Options,
    ) -> bool {
        let outward = -inward;
        let local_inward = pose.rotation.inverse() * inward;
        let mut changed = false;
        for i in 0..self.offsets.len() {
            let old = Vector::from_array(self.offsets[i]);
            let current = pose * (self.rest(i) + old);
            let delta = current - point;
            let lateral = delta - inward * delta.dot(inward);
            let distance = lateral.length() / options.radius;
            if distance >= 1. || delta.dot(inward).abs() > options.radius {
                continue;
            }
            let ray = Ray::new(current - outward * options.radius, outward);
            let Some(hit) =
                obstacle
                    .shape()
                    .cast_ray(obstacle.position(), &ray, options.radius + depth, true)
            else {
                continue;
            };
            let amount = (options.radius + depth - hit).clamp(0., depth)
                * (1. - distance * distance).powi(2);
            if amount < 0.0005 {
                continue;
            }
            let next = (old + local_inward * amount).clamp_length_max(options.max_displacement);
            if (next - old).length_squared() > 0.000_000_25 {
                self.offsets[i] = next.to_array();
                changed = true;
            }
        }
        changed
    }
}

#[derive(Clone)]
pub(super) struct Runtime {
    options: Options,
    pub field: Field,
    rest: Vec<Vec<[f32; 3]>>,
    current: Vec<Vec<[f32; 3]>>,
    shapes: Vec<SharedShape>,
    remaining: f32,
}
impl Runtime {
    pub(super) fn replica(field: Field, options: Options) -> Result<Self, String> {
        options.validate()?;
        field.validate()?;
        if field.resolution != options.resolution {
            return Err("deformation resolution mismatch".into());
        }
        Ok(Self {
            options,
            field,
            rest: Vec::new(),
            current: Vec::new(),
            shapes: Vec::new(),
            remaining: 0.,
        })
    }
    pub fn new(desc: &BodyDesc, options: Options) -> Result<Self, String> {
        options.validate()?;
        if desc.sensor {
            return Err("sensors cannot deform".into());
        }
        type Templates = Vec<(Shape, f32, [usize; 3], Runtime)>;
        static CACHE: std::sync::OnceLock<std::sync::Mutex<Templates>> = std::sync::OnceLock::new();
        let cache = CACHE.get_or_init(Default::default);
        if let Ok(entries) = cache.lock() {
            if let Some((_, _, _, template)) = entries.iter().find(|(shape, radius, res, _)| {
                shape == &desc.shape && *radius == options.radius && *res == options.resolution
            }) {
                let mut runtime = template.clone();
                runtime.options = options;
                return Ok(runtime);
            }
        }
        let mut rest = match &desc.shape {
            Shape::Compound { hulls } => hulls.clone(),
            Shape::Convex { points } => vec![points.clone()],
            Shape::Box { half_extents: h } => vec![
                (0..8)
                    .map(|i| {
                        std::array::from_fn(|a| h[a] * if (i >> a) & 1 == 0 { -1. } else { 1. })
                    })
                    .collect(),
            ],
            _ => {
                return Err("deformation requires box, convex or resolved compound geometry".into());
            }
        };
        // Refine only at creation. Large convex pieces otherwise bridge across
        // a localized concave dent. Plane splits preserve the original solid and
        // give the damaged compound enough local support, without runtime VHACD.
        refine(&mut rest, options.radius * 0.5);
        let mut min = Vector::splat(f32::MAX);
        let mut max = Vector::splat(-f32::MAX);
        for p in rest.iter().flatten() {
            let p = Vector::from_array(*p);
            min = min.min(p);
            max = max.max(p);
        }
        if (max - min).min_element() < 0.001 {
            return Err("deformation requires volumetric geometry".into());
        }
        let count = options.resolution.iter().product();
        let field = Field {
            min: min.to_array(),
            max: max.to_array(),
            resolution: options.resolution,
            offsets: vec![[0.; 3]; count],
            revision: 0,
        };
        let shapes = rest
            .iter()
            .map(|points| {
                SharedShape::convex_hull(
                    &points
                        .iter()
                        .copied()
                        .map(Vector::from_array)
                        .collect::<Vec<_>>(),
                )
                .ok_or("invalid deformable hull".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let runtime = Self {
            options,
            field,
            current: rest.clone(),
            rest,
            shapes,
            remaining: 0.,
        };
        if let Ok(mut entries) = cache.lock() {
            if entries.len() >= 8 {
                entries.remove(0);
            }
            entries.push((
                desc.shape.clone(),
                runtime.options.radius,
                runtime.options.resolution,
                runtime.clone(),
            ));
        }
        Ok(runtime)
    }
    fn rebuild(
        &self,
        field: &Field,
    ) -> Result<(Vec<Vec<[f32; 3]>>, Vec<SharedShape>, SharedShape), String> {
        let mut hulls = Vec::with_capacity(self.rest.len());
        let mut shapes = self.shapes.clone();
        for (i, rest) in self.rest.iter().enumerate() {
            let points: Vec<_> = rest
                .iter()
                .map(|p| (Vector::from_array(*p) + field.displacement(*p)).to_array())
                .collect();
            if points.iter().zip(&self.current[i]).any(|(a, b)| {
                Vector::from_array(*a).distance_squared(Vector::from_array(*b)) > 1e-10
            }) {
                shapes[i] = SharedShape::convex_hull(
                    &points
                        .iter()
                        .copied()
                        .map(Vector::from_array)
                        .collect::<Vec<_>>(),
                )
                .ok_or("deformation collapsed a convex part")?;
            }
            hulls.push(points);
        }
        let compound = SharedShape::compound(
            shapes
                .iter()
                .cloned()
                .map(|s| (Pose::identity(), s))
                .collect(),
        );
        Ok((hulls, shapes, compound))
    }
}

fn refine(hulls: &mut Vec<Vec<[f32; 3]>>, target: f32) {
    let mut exhausted = std::collections::BTreeSet::new();
    while hulls.len() < model::MAX_COMPOUND_HULLS {
        let candidate = hulls
            .iter()
            .enumerate()
            .filter(|(i, _)| !exhausted.contains(i))
            .filter_map(|(i, p)| {
                let mut lo = Vector::splat(f32::MAX);
                let mut hi = -lo;
                for &v in p {
                    let v = Vector::from_array(v);
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
                let size = hi - lo;
                let axis = (0..3).max_by(|&a, &b| size[a].total_cmp(&size[b]))?;
                (size[axis] > target).then_some((i, axis, (lo[axis] + hi[axis]) * 0.5, size[axis]))
            })
            .max_by(|a, b| a.3.total_cmp(&b.3));
        let Some((i, axis, cut, _)) = candidate else {
            break;
        };
        let Some(shape) = SharedShape::convex_hull(
            &hulls[i]
                .iter()
                .copied()
                .map(Vector::from_array)
                .collect::<Vec<_>>(),
        ) else {
            exhausted.insert(i);
            continue;
        };
        let Some(poly) = shape.as_convex_polyhedron() else {
            exhausted.insert(i);
            continue;
        };
        let (vertices, triangles) = poly.to_trimesh();
        let mut sides: [Vec<Vector>; 2] = std::array::from_fn(|side| {
            vertices
                .iter()
                .copied()
                .filter(|p| {
                    if side == 0 {
                        p[axis] <= cut
                    } else {
                        p[axis] >= cut
                    }
                })
                .collect()
        });
        let mut edges = std::collections::BTreeSet::new();
        for tri in triangles {
            for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                if !edges.insert((a.min(b), a.max(b))) {
                    continue;
                }
                let a = vertices[a as usize];
                let b = vertices[b as usize];
                if (a[axis] - cut) * (b[axis] - cut) < 0. {
                    let p = a + (b - a) * ((cut - a[axis]) / (b[axis] - a[axis]));
                    sides[0].push(p);
                    sides[1].push(p);
                }
            }
        }
        let parts: Option<Vec<Vec<[f32; 3]>>> = sides
            .into_iter()
            .map(|points| {
                let shape = SharedShape::convex_hull(&points)?;
                Some(
                    shape
                        .as_convex_polyhedron()?
                        .to_trimesh()
                        .0
                        .into_iter()
                        .map(|v| v.to_array())
                        .collect(),
                )
            })
            .collect();
        let Some(mut parts) = parts else {
            exhausted.insert(i);
            continue;
        };
        let total: usize = hulls.iter().map(Vec::len).sum::<usize>() - hulls[i].len()
            + parts.iter().map(Vec::len).sum::<usize>();
        if total > model::MAX_BODY_POINTS {
            break;
        }
        hulls[i] = parts.pop().unwrap();
        hulls.push(parts.pop().unwrap());
    }
}

impl DynamicsWorld {
    /// Damage-only replica revision: retain body/collider handles and interpolation.
    /// Other definition changes use the existing full replacement path.
    pub fn update_replica_deformation(
        &mut self,
        id: u64,
        definition: &BodyDefinition,
    ) -> Result<bool, String> {
        let Some(old) = self.body_definition(id) else {
            return Ok(false);
        };
        let Some(field) = &definition.deformation else {
            return Ok(false);
        };
        let Some(options) = definition.body.deformation.clone() else {
            return Err("deformation field without options".into());
        };
        let mut compare = definition.clone();
        compare.body.shape = old.body.shape.clone();
        compare.deformation = old.deformation.clone();
        if serde_json::to_vec(&compare).map_err(|e| e.to_string())?
            != serde_json::to_vec(&old).map_err(|e| e.to_string())?
        {
            return Ok(false);
        }
        self.validate(&definition.body)?;
        let runtime = Runtime::replica(field.clone(), options)?;
        let new = self.collider(&definition.body)?;
        let body = self
            .bodies
            .get(self.ids[&id])
            .ok_or("missing replica body")?;
        if !body.is_kinematic() {
            return Err("only replicas accept remote deformation".into());
        }
        let handle = *body.colliders().first().ok_or("missing replica collider")?;
        let collider = self
            .colliders
            .get_mut(handle)
            .ok_or("missing replica collider")?;
        collider.set_shape(new.shared_shape().clone());
        self.deformations.insert(id, runtime);
        let meta = self.meta.get_mut(&id).unwrap();
        meta.shape = definition.body.shape.clone();
        meta.definition = definition.body.clone();
        meta.mesh = None;
        meta.geometry_revision = meta.geometry_revision.wrapping_add(1);
        Ok(true)
    }
    pub fn deformation(&self, id: u64) -> Option<&Field> {
        self.deformations.get(&id).map(|r| &r.field)
    }
    pub fn collider_offset(&self, id: u64) -> Option<[f32; 3]> {
        self.meta.get(&id).map(|m| m.collider_offset)
    }
    pub(super) fn deform_impacts(&mut self, dt: f32) {
        if self.deformations.is_empty() {
            return;
        }
        let mut impacts = Vec::new();
        for (&id, runtime) in &mut self.deformations {
            runtime.remaining = (runtime.remaining - dt).max(0.);
            if runtime.remaining > 0. {
                continue;
            }
            let Some(body) = self.ids.get(&id).and_then(|h| self.bodies.get(*h)) else {
                continue;
            };
            if !body.is_dynamic() || body.is_sleeping() {
                continue;
            }
            let Some(&own) = body.colliders().first() else {
                continue;
            };
            let mut strongest: Option<(f32, ColliderHandle, Vector, Vector)> = None;
            for pair in self.narrow_phase.contact_pairs_with(own) {
                let first = pair.collider1 == own;
                let other = if first {
                    pair.collider2
                } else {
                    pair.collider1
                };
                if self.colliders.get(other).is_none_or(|c| c.is_sensor()) {
                    continue;
                }
                let mut impulse = 0.;
                let mut point = Vector::ZERO;
                let mut normal = Vector::ZERO;
                for manifold in pair.solver_manifolds() {
                    let magnitude: f32 = manifold.points.iter().map(|p| p.data.impulse).sum();
                    let Some(contact) = manifold.data.solver_contacts.first() else {
                        continue;
                    };
                    let (a, b) = manifold
                        .data
                        .solver_contact_world_points(contact, &self.bodies);
                    impulse += magnitude;
                    point += (a + b) * (0.5 * magnitude);
                    normal += manifold.data.normal * magnitude;
                }
                let severity = impulse / self.meta[&id].mass;
                if severity <= runtime.options.yield_speed
                    || normal.length_squared() < 1e-8
                    || strongest.as_ref().is_some_and(|s| s.0 >= severity)
                {
                    continue;
                }
                strongest = Some((
                    severity,
                    other,
                    point / impulse,
                    normal.normalize() * if first { -1. } else { 1. },
                ));
            }
            if let Some(impact) = strongest {
                impacts.push((id, own, impact));
            }
        }
        // Preserve the solved frame for both objects in a crash.
        let impacts: Vec<_> = impacts
            .into_iter()
            .filter_map(|(id, own, impact)| {
                Some((id, own, impact, self.colliders.get(impact.1)?.clone()))
            })
            .collect();
        for (id, own, (severity, _, point, inward), obstacle) in impacts {
            let Some(collider) = self.colliders.get(own) else {
                continue;
            };
            let runtime = self.deformations.get_mut(&id).unwrap();
            let mut field = runtime.field.clone();
            let depth = ((severity - runtime.options.yield_speed) * runtime.options.compliance)
                .min(runtime.options.max_step);
            if !field.dent(
                *collider.position(),
                point,
                inward,
                depth,
                &obstacle,
                &runtime.options,
            ) {
                continue;
            }
            let Ok((hulls, shapes, shape)) = runtime.rebuild(&field) else {
                continue;
            };
            let Some(collider) = self.colliders.get_mut(own) else {
                continue;
            };
            // Atomic commit. Keep authored mass/inertia and the same rigid-body
            // identity, velocities and joints; geometry updates inject no impulse.
            let mass = collider.mass_properties();
            collider.set_shape(shape);
            collider.set_mass_properties(mass);
            field.revision = field.revision.wrapping_add(1);
            runtime.field = field;
            runtime.current = hulls.clone();
            runtime.shapes = shapes;
            runtime.remaining = runtime.options.cooldown;
            let meta = self.meta.get_mut(&id).unwrap();
            meta.shape = Shape::Compound { hulls };
            meta.definition.shape = meta.shape.clone();
            meta.mesh = None;
            meta.geometry_revision = meta.geometry_revision.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> Options {
        Options {
            resolution: [9, 5, 9],
            ..Default::default()
        }
    }
    fn block() -> BodyDesc {
        BodyDesc {
            shape: Shape::Box {
                half_extents: [1., 0.5, 1.],
            },
            deformation: Some(options()),
            mass: 1000.,
            linear_damping: 0.,
            angular_damping: 0.,
            ..Default::default()
        }
    }
    #[test]
    fn obstacle_geometry_distinguishes_wall_from_narrow_post() {
        let r = Runtime::new(&block(), options()).unwrap();
        let wall = ColliderBuilder::cuboid(4., 4., 0.2)
            .translation(Vector::new(0., 0., 1.2))
            .build();
        let post = ColliderBuilder::cuboid(0.13, 4., 0.2)
            .translation(Vector::new(0., 0., 1.2))
            .build();
        let mut broad = r.field.clone();
        let mut narrow = r.field.clone();
        for (f, c) in [(&mut broad, &wall), (&mut narrow, &post)] {
            assert!(f.dent(Pose::identity(), Vector::Z, -Vector::Z, 0.2, c, &options()));
        }
        let center = [0., 0., 1.];
        let side = [0.75, 0., 1.];
        assert!(narrow.displacement(center).z < -0.15);
        assert!(narrow.displacement(side).length() < 0.0001);
        assert!(broad.displacement(side).z < -0.02);
        assert!(broad.displacement([0., 0., -1.]).length() < 0.0001);
    }
    #[test]
    fn impacts_change_solid_shape_and_replicated_mesh_field() {
        let mut w = DynamicsWorld::default();
        w.gravity = Vector::ZERO;
        w.spawn(BodyDesc {
            body_type: BodyType::Static,
            shape: Shape::Box {
                half_extents: [4., 4., 0.2],
            },
            position: [0., 0., 1.4],
            ..Default::default()
        })
        .unwrap();
        let id = w.spawn(block()).unwrap();
        w.set_linvel(id, [0., 0., 15.]);
        let before = w.body_definition(id).unwrap();
        let start = std::time::Instant::now();
        let mut worst = std::time::Duration::ZERO;
        for _ in 0..90 {
            let t = std::time::Instant::now();
            w.step(1. / 120.);
            worst = worst.max(t.elapsed());
        }
        eprintln!(
            "90 collision ticks {:?}; worst tick {:?}",
            start.elapsed(),
            worst
        );
        let after = w.body_definition(id).unwrap();
        let field = after.deformation.as_ref().unwrap();
        assert!(
            field.revision > 0,
            "actual Rapier impact must dent the field"
        );
        assert_ne!(before.body.shape, after.body.shape);
        assert!(field.offsets.iter().flatten().any(|v| v.abs() > 0.001));
        let handle = w.ids[&id];
        assert!((w.bodies[handle].mass() - 1000.).abs() < 0.01);
        let solid = w.solid_bodies().into_iter().find(|b| b.id == id).unwrap();
        assert!(
            solid
                .colliders
                .iter()
                .any(|c| c.shape.as_convex_polyhedron().is_some())
        );
        let bytes = crate::definition_codec::encode(&after).unwrap();
        let decoded = crate::definition_codec::decode(&bytes).unwrap();
        let mut remote = DynamicsWorld::default();
        let other = remote.spawn_replica(&decoded).unwrap();
        assert_eq!(remote.deformation(other).unwrap().offsets, field.offsets);
        assert_eq!(
            remote.body_definition(other).unwrap().body.shape,
            after.body.shape
        );
        remote.step(1. / 120.);
        assert_eq!(remote.deformation(other).unwrap().revision, field.revision);
        assert!((remote.solid_bodies()[0].inverse_mass - 0.001).abs() < 1e-7);
        for cut in [0, 7, bytes.len() - 1] {
            assert!(crate::definition_codec::decode(&bytes[..cut]).is_err());
        }
    }
    #[test]
    fn resting_contacts_do_not_accumulate_damage() {
        let mut w = DynamicsWorld::default();
        w.spawn(BodyDesc {
            body_type: BodyType::Static,
            shape: Shape::Box {
                half_extents: [5., 0.5, 5.],
            },
            position: [0., -1., 0.],
            ..Default::default()
        })
        .unwrap();
        let id = w.spawn(block()).unwrap();
        for _ in 0..300 {
            w.step(1. / 120.);
        }
        assert_eq!(w.deformation(id).unwrap().revision, 0);
        assert!(matches!(
            w.body_definition(id).unwrap().body.shape,
            Shape::Box { .. }
        ));
    }
    #[test]
    fn damage_is_bounded_and_invalid_input_rejected() {
        let mut r = Runtime::new(&block(), options()).unwrap();
        let wall = ColliderBuilder::cuboid(4., 4., 0.2)
            .translation(Vector::new(0., 0., 1.2))
            .build();
        for _ in 0..100 {
            r.field.dent(
                Pose::identity(),
                Vector::Z,
                -Vector::Z,
                0.22,
                &wall,
                &options(),
            );
        }
        assert!(
            r.field
                .offsets
                .iter()
                .all(|p| Vector::from_array(*p).length() <= 0.551)
        );
        assert!(
            Options {
                resolution: [usize::MAX; 3],
                ..options()
            }
            .validate()
            .is_err()
        );
        let mut f = r.field.clone();
        f.offsets[0][0] = f32::NAN;
        assert!(f.validate().is_err());
        f = r.field;
        f.offsets.clear();
        assert!(f.validate().is_err());
    }
}

#[cfg(test)]
mod fidelity_tests {
    use super::*;
    #[test]
    fn concave_post_dent_changes_the_solid_surface_and_rotates_with_body() {
        let options = Options {
            resolution: [9, 5, 9],
            ..Default::default()
        };
        let desc = BodyDesc {
            shape: Shape::Box {
                half_extents: [1., 0.5, 1.],
            },
            ..Default::default()
        };
        let runtime = Runtime::new(&desc, options.clone()).unwrap();
        let mut field = runtime.field.clone();
        let obstacle = ColliderBuilder::cuboid(0.13, 4., 0.2)
            .translation(Vector::new(0., 0., 1.2))
            .build();
        assert!(field.dent(
            Pose::identity(),
            Vector::Z,
            -Vector::Z,
            0.2,
            &obstacle,
            &options
        ));
        let (_, _, shape) = runtime.rebuild(&field).unwrap();
        let hit = shape
            .cast_local_ray(&Ray::new(Vector::new(0., 0., 3.), -Vector::Z), 5., true)
            .unwrap();
        let visual = 1. + field.displacement([0., 0., 1.]).z;
        assert!(
            (3. - hit - visual).abs() < 0.015,
            "collider {} vs mesh {}",
            3. - hit,
            visual
        );
        let side = shape
            .cast_local_ray(&Ray::new(Vector::new(0.75, 0., 3.), -Vector::Z), 5., true)
            .unwrap();
        assert!(
            (side - 2.).abs() < 0.001,
            "undamaged side must keep its collision surface"
        );
        let pose = Pose::from_parts(
            Vector::new(8., 3., -4.),
            Rotation::from_scaled_axis(Vector::Y * 1.1),
        );
        let mut rotated = runtime.field.clone();
        let mut obstacle = obstacle;
        obstacle.set_position(pose * *obstacle.position());
        assert!(rotated.dent(
            pose,
            pose * Vector::Z,
            pose.rotation * (-Vector::Z),
            0.2,
            &obstacle,
            &options
        ));
        for (a, b) in field.offsets.iter().zip(rotated.offsets) {
            assert!(Vector::from_array(*a).distance(Vector::from_array(b)) < 0.0001);
        }
    }
    #[test]
    fn damage_update_preserves_remote_body_handle_and_accepts_late_join() {
        let desc = BodyDesc {
            deformation: Some(Options::default()),
            ..Default::default()
        };
        let mut local = DynamicsWorld::default();
        let id = local.spawn(desc).unwrap();
        let mut definition = local.body_definition(id).unwrap();
        let mut remote = DynamicsWorld::default();
        let peer = remote.spawn_replica(&definition).unwrap();
        let original_handle = remote.ids[&peer];
        let field = definition.deformation.as_mut().unwrap();
        field.revision = 1;
        field.offsets.iter_mut().for_each(|p| p[2] = -0.1);
        let runtime = &local.deformations[&id];
        let (hulls, _, _) = runtime.rebuild(field).unwrap();
        definition.body.shape = Shape::Compound { hulls };
        assert!(
            remote
                .update_replica_deformation(peer, &definition)
                .unwrap()
        );
        assert_eq!(remote.ids[&peer], original_handle);
        assert_eq!(remote.deformation(peer).unwrap().revision, 1);
        let mut joiner = DynamicsWorld::default();
        let joined = joiner.spawn_replica(&definition).unwrap();
        assert_eq!(
            joiner.deformation(joined).unwrap().offsets,
            remote.deformation(peer).unwrap().offsets
        );
        assert_eq!(
            remote.body_definition(peer).unwrap().body.shape,
            joiner.body_definition(joined).unwrap().body.shape
        );
    }
}
