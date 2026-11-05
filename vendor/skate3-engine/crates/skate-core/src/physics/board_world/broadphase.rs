//! ZIP world broadphase helpers; recovered narrow-phase stays in its existing modules.
use super::{BoardWorld, ContactPrimitive, Vector3, WorldTriangle, query_metadata::Bounds};

// Positive magnitude of thin_triangle's existing 0xb727_c5ac tolerance.
pub(super) const THIN_MARGIN: f32 = f32::from_bits(0x3727_c5ac);

impl BoardWorld {
    /// Conservative cluster culling, retaining canonical traversal order and
    /// every triangle in intersecting clusters. Narrow-phase remains unchanged.
    pub fn candidate_ranges(&self, bounds: Option<Bounds>) -> Vec<std::ops::Range<usize>> {
        let (Some(metadata), Some(bounds)) = (&self.query_metadata, bounds.filter(|b| b.valid()))
        else {
            return vec![0..self.triangles.len()];
        };
        let scale = [
            bounds.min.x,
            bounds.min.y,
            bounds.min.z,
            bounds.max.x,
            bounds.max.y,
            bounds.max.z,
        ]
        .into_iter()
        .map(f32::abs)
        .fold(1., f32::max);
        let bounds = bounds.expanded(
            self.maximum_fatness + self.maximum_triangle_margin + scale * (8. * f32::EPSILON),
        );
        self.query_index
            .query(bounds, &metadata.meshes)
            .into_iter()
            .map(|i| metadata.meshes[i].triangle_range.clone())
            .collect()
    }

    pub fn line_candidates(
        &self,
        start: Vector3,
        end: Vector3,
        radius: f32,
    ) -> impl Iterator<Item = (usize, &WorldTriangle)> {
        let bounds = self.line_candidate_bounds(start, end, radius);
        self.candidate_ranges(bounds)
            .into_iter()
            .flatten()
            .filter(move |&i| {
                self.query_metadata.is_none()
                    || bounds.is_none_or(|b| self.triangle_bounds[i].overlaps(b))
            })
            .map(|i| (i, &self.triangles[i]))
    }

    /// Bounds shared with adapters that retain mesh pool/group/identity filtering.
    /// Includes the current thin-leaf endpoint and barycentric margins.
    pub fn line_candidate_bounds(
        &self,
        start: Vector3,
        end: Vector3,
        radius: f32,
    ) -> Option<Bounds> {
        if !radius.is_finite() || radius < 0. {
            return None;
        }
        let span = (end.x - start.x)
            .abs()
            .max((end.y - start.y).abs())
            .max((end.z - start.z).abs());
        Bounds::from_points([start, end]).map(|b| {
            conservative_bounds(
                b,
                radius + self.maximum_fatness + self.maximum_triangle_margin + span * THIN_MARGIN,
            )
        })
    }

    /// ZIP hierarchy results in canonical mesh order, for current metadata consumers.
    /// Pool, matching group, rejection mask and identity stay caller-owned.
    pub fn candidate_mesh_indices(
        &self,
        bounds: Option<Bounds>,
    ) -> Result<Vec<usize>, &'static str> {
        let metadata = self.query_metadata()?;
        let Some(bounds) = bounds.filter(|b| b.valid()) else {
            return Ok((0..metadata.meshes.len()).collect());
        };
        Ok(self.query_index.query(
            conservative_bounds(bounds, self.maximum_fatness + self.maximum_triangle_margin),
            &metadata.meshes,
        ))
    }
}

pub(super) fn primitive_bounds(primitive: ContactPrimitive) -> Option<Bounds> {
    let (center, radius) = match primitive {
        ContactPrimitive::Sphere(s) => (s.center, s.radius),
        ContactPrimitive::Capsule {
            center,
            axis,
            half_length,
            radius,
        } => {
            if !radius.is_finite() {
                return None;
            }
            // 82AD97A0: centre +- fma(|axis|, half length +68, radius +80).
            let extent = |a: f32| a.abs().mul_add(half_length, radius.abs());
            let e = Vector3::new(extent(axis.x), extent(axis.y), extent(axis.z));
            return Bounds::from_points([
                Vector3::new(center.x - e.x, center.y - e.y, center.z - e.z),
                Vector3::new(center.x + e.x, center.y + e.y, center.z + e.z),
            ]);
        }
        ContactPrimitive::RoundedBox {
            center,
            basis,
            half_extents,
            radius,
        } => {
            if !radius.is_finite() {
                return None;
            }
            // 82AD9558: per world axis, |axis 1| * h1, then fma |axis 0| * h0,
            // then fma |axis 2| * h2, then + radius (+80) as a separate add.
            let [c0, c1, c2] = basis.columns;
            let (h0, h1, h2) = (
                half_extents.x.abs(),
                half_extents.y.abs(),
                half_extents.z.abs(),
            );
            let extent: [f32; 3] = std::array::from_fn(|axis| {
                c2[axis]
                    .abs()
                    .mul_add(h2, c0[axis].abs().mul_add(h0, c1[axis].abs() * h1))
                    + radius.abs()
            });
            return Bounds::from_points([
                Vector3::new(
                    center.x - extent[0],
                    center.y - extent[1],
                    center.z - extent[2],
                ),
                Vector3::new(
                    center.x + extent[0],
                    center.y + extent[1],
                    center.z + extent[2],
                ),
            ]);
        }
        ContactPrimitive::Triangle(t) => {
            if !t.fatness.is_finite() {
                return None;
            }
            return Bounds::from_points(t.vertices).map(|b| b.expanded(t.fatness.abs()));
        }
    };
    if !radius.is_finite() {
        return None;
    }
    Bounds::from_points([center]).map(|b| b.expanded(radius.abs()))
}

pub(super) fn conservative_bounds(bounds: Bounds, padding: f32) -> Bounds {
    let scale = [
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ]
    .into_iter()
    .map(f32::abs)
    .fold(1., f32::max);
    bounds.expanded(padding + scale * (8. * f32::EPSILON))
}

/// Data 0x820849C8: the fixed 1/60 step the world query predicts over.
pub const VOLUME_QUERY_STEP: f32 = f32::from_bits(0x3C88_8889);
/// Data 0x821659FC: scale applied to the swept box about its centre.
pub const VOLUME_QUERY_SCALE: f32 = 1.05;

/// Body motion the per-volume query box sweeps over (body +32, +48, +144, +160).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VolumeMotion {
    pub linear_velocity: Vector3,
    pub angular_velocity: Vector3,
    pub force_acceleration: Vector3,
    pub torque_acceleration: Vector3,
}

impl VolumeMotion {
    pub fn of(rates: &crate::physics::rigid_body::RetailBodyRates) -> Self {
        Self {
            linear_velocity: rates.linear_velocity,
            angular_velocity: rates.angular_velocity,
            force_acceleration: rates.force_acceleration,
            torque_acceleration: rates.torque_acceleration,
        }
    }
}

fn v3(f: impl Fn(usize) -> f32) -> Vector3 {
    Vector3::new(f(0), f(1), f(2))
}

fn axis(v: Vector3, i: usize) -> f32 {
    [v.x, v.y, v.z][i]
}

/// Step = rate * dt + clamp(dot(rate * dt, accel * dt^2), 0, 1) * accel * dt^2.
fn motion_step(rate: Vector3, acceleration: Vector3, dt: f32) -> Vector3 {
    let first = v3(|i| axis(rate, i) * dt);
    let second = v3(|i| axis(acceleration, i) * dt * dt);
    let factor = (0..3)
        .map(|i| axis(first, i) * axis(second, i))
        .sum::<f32>()
        .max(0.)
        .min(1.);
    v3(|i| axis(rate, i).mul_add(dt, axis(acceleration, i) * factor * dt * dt))
}

/// 82777E70: the box every world triangle's vertex box is tested against in
/// 8277BC58 (BE94..BF38) before the pair query. Primitive bounds from the
/// shape's bounds slot (+4, flag 1; sphere 82ADD738, capsule 82AD97A0,
/// triangle 82ADDC40, box 82AD9558: shape plus radius/fatness +80 only, no
/// +112 padding, no separation term), padded by the largest extent
/// difference times min(|angular step|, 1), unioned with itself moved by the
/// linear step, then scaled about its centre.
pub fn volume_query_bounds(
    primitive: ContactPrimitive,
    motion: VolumeMotion,
    step: f32,
    scale: f32,
) -> Option<Bounds> {
    let b = primitive_bounds(primitive)?;
    let e = v3(|i| axis(b.max, i) - axis(b.min, i));
    let extent_pad = (e.x - e.y)
        .abs()
        .max((e.y - e.z).abs())
        .max((e.z - e.x).abs());
    let angular = motion_step(motion.angular_velocity, motion.torque_acceleration, step);
    // vmsum3fp128, vrsqrtefp128 and two Newton refinements, length = x * y,
    // zero for a zero step: the same sequence as board_motion_output::length.
    let length = crate::physics::board_motion_output::length(angular);
    let pad = extent_pad * length.min(1.);
    let linear = motion_step(motion.linear_velocity, motion.force_acceleration, step);
    let min = v3(|i| axis(b.min, i) - pad);
    let max = v3(|i| axis(b.max, i) + pad);
    let min = v3(|i| axis(min, i).min(axis(min, i) + axis(linear, i)));
    let max = v3(|i| axis(max, i).max(axis(max, i) + axis(linear, i)));
    let centre = v3(|i| (axis(max, i) + axis(min, i)) * 0.5);
    let half = v3(|i| (axis(max, i) - axis(centre, i)) * scale);
    let out = Bounds {
        min: v3(|i| axis(centre, i) - axis(half, i)),
        max: v3(|i| axis(centre, i) + axis(half, i)),
    };
    out.valid().then_some(out)
}
