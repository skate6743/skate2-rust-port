//! Retail trigger volumes: the volume shape, the tracked-entity query shape and
//! the per-entity enter/exit bookkeeping of the trigger manager's groups.
//!
//! TU3 evidence (reference only; notes `triggers-volumes-re.md`):
//! - Each group update (`82DD70E8`) gives every tracked entity a query
//!   **cylinder** (`82DD80B8` → `82ADA4E0`, RW volume type 5) from three points
//!   the skater entity reports (vtable +12 / +16 / +20): its **feet** (ground
//!   point), **head** and **hips**. Measured in the recomp: head 1.61-1.65 m and
//!   hips 0.97-1.00 m above the feet for a standing skater, feet on the ground
//!   plane. Radius 0.34 m, axis `feet - hips` (pointing down), half-height
//!   `0.5·|head - feet| + 0.05`, centred `half-height - 0.02` above the feet:
//!   the cylinder runs from 2 cm below the feet to 8 cm above the head.
//! - Candidates come from an AABB tree of the volumes' bounds, then each one is
//!   tested against the volume's real shape (`82DD8498` → `82AD3CD8`, RW
//!   volume-volume overlap, tolerance 0): the oriented box linked from the
//!   item. The bounds contain the box, so the narrow test alone decides.
//! - The new "inside" set is diffed with last frame's (`82557600`): entered
//!   volumes are posted first, then exited ones, per entity in slot order.
//! - Removing a volume (`82DD7018`) drops it from every entity's set without an
//!   exit message; removing an entity (`82DD6D20`) posts exits for its volumes.
use std::collections::BTreeMap;

pub type Point = [f32; 3];

/// A trigger volume's real shape: a box with orthonormal axes. `axes[i]` is the
/// box's local axis `i` in world space; `fatness` rounds it (RW volume fatness).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientedBox {
    pub center: Point,
    pub axes: [Point; 3],
    pub half_extents: Point,
    pub fatness: f32,
}

impl OrientedBox {
    pub fn axis_aligned(min: Point, max: Point) -> Self {
        Self {
            center: [0, 1, 2].map(|i| (min[i] + max[i]) * 0.5),
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            half_extents: [0, 1, 2].map(|i| (max[i] - min[i]).abs() * 0.5),
            fatness: 0.,
        }
    }

    /// Bounds of the box (including fatness).
    pub fn aabb(&self) -> (Point, Point) {
        let extent = [0, 1, 2].map(|c| {
            (0..3)
                .map(|a| self.axes[a][c].abs() * self.half_extents[a])
                .sum::<f32>()
                + self.fatness
        });
        (
            [0, 1, 2].map(|c| self.center[c] - extent[c]),
            [0, 1, 2].map(|c| self.center[c] + extent[c]),
        )
    }

    pub fn contains_point(&self, point: Point) -> bool {
        let d = sub(point.map(f64::from), self.center.map(f64::from));
        let outside = (0..3).map(|a| {
            (dot(d, self.axes[a].map(f64::from)).abs() - f64::from(self.half_extents[a])).max(0.)
        });
        outside.map(|v| v * v).sum::<f64>().sqrt() <= f64::from(self.fatness)
    }

    /// Finite, non-negative extents and an orthonormal basis.
    pub fn is_valid(&self) -> bool {
        let finite = self
            .center
            .iter()
            .chain(self.half_extents.iter())
            .chain(self.axes.iter().flatten())
            .chain(std::iter::once(&self.fatness))
            .all(|v| v.is_finite());
        let a = self.axes.map(|v| v.map(f64::from));
        finite
            && self.half_extents.iter().all(|&h| h >= 0.)
            && self.fatness >= 0.
            && a.iter().all(|v| (dot(*v, *v) - 1.).abs() < 1e-3)
            && dot(a[0], a[1]).abs() < 1e-3
            && dot(a[0], a[2]).abs() < 1e-3
            && dot(a[1], a[2]).abs() < 1e-3
    }

    fn support(&self, d: [f64; 3]) -> [f64; 3] {
        let mut p = self.center.map(f64::from);
        for a in 0..3 {
            let axis = self.axes[a].map(f64::from);
            let s = if dot(d, axis) >= 0. { 1. } else { -1. };
            p = add(p, scale(axis, s * f64::from(self.half_extents[a])));
        }
        p
    }
}

/// A solid cylinder: `axis` is a unit vector, the caps lie `half_height` from
/// the centre along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylinder {
    pub center: Point,
    pub axis: Point,
    pub half_height: f32,
    pub radius: f32,
}

/// How a tracked entity's three points become its query cylinder. Defaults are
/// the TU3 constants; a mod or a custom body may supply its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryShape {
    pub radius: f32,
    pub length_scale: f32,
    pub length_pad: f32,
    /// How far the cylinder's end reaches past the feet point.
    pub foot_pad: f32,
}

impl QueryShape {
    pub const RETAIL: Self = Self {
        radius: 0.34,
        length_scale: 0.5,
        length_pad: 0.05,
        foot_pad: 0.02,
    };

    /// The retail entity's points: `feet` anchors the cylinder (one end lies
    /// `foot_pad` past it), the axis runs `feet - hips` (straight down when
    /// they coincide; retail would normalise a zero vector) and the length
    /// follows `|head - feet|`, so the cylinder reaches past the head.
    pub fn cylinder(&self, feet: Point, head: Point, hips: Point) -> Option<Cylinder> {
        let [f, h, c] = [feet, head, hips].map(|p| p.map(f64::from));
        if !f
            .iter()
            .chain(h.iter())
            .chain(c.iter())
            .all(|v| v.is_finite())
        {
            return None;
        }
        let down = sub(f, c);
        let len = dot(down, down).sqrt();
        let axis = if len > 1e-6 {
            scale(down, 1. / len)
        } else {
            [0., -1., 0.]
        };
        let span = dot(sub(h, f), sub(h, f)).sqrt();
        let half = f64::from(self.length_scale) * span + f64::from(self.length_pad);
        let center = sub(f, scale(axis, half - f64::from(self.foot_pad)));
        Some(Cylinder {
            center: center.map(|v| v as f32),
            axis: axis.map(|v| v as f32),
            half_height: half as f32,
            radius: self.radius,
        })
    }
}

impl Default for QueryShape {
    fn default() -> Self {
        Self::RETAIL
    }
}

impl Cylinder {
    fn support(&self, d: [f64; 3]) -> [f64; 3] {
        let axis = self.axis.map(f64::from);
        let along = dot(d, axis);
        let s = if along >= 0. { 1. } else { -1. };
        let mut p = add(
            self.center.map(f64::from),
            scale(axis, s * f64::from(self.half_height)),
        );
        let radial = sub(d, scale(axis, along));
        let r = dot(radial, radial).sqrt();
        if r > 1e-12 {
            p = add(p, scale(radial, f64::from(self.radius) / r));
        }
        p
    }

    pub fn aabb(&self) -> (Point, Point) {
        let extent = [0, 1, 2].map(|c| {
            let a = self.axis[c];
            a.abs() * self.half_height + self.radius * (1. - a * a).max(0.).sqrt()
        });
        (
            [0, 1, 2].map(|c| self.center[c] - extent[c]),
            [0, 1, 2].map(|c| self.center[c] + extent[c]),
        )
    }
}

/// Retail's narrow test: the query cylinder overlaps the box (tolerance 0).
pub fn overlaps(query: &Cylinder, volume: &OrientedBox) -> bool {
    // Broad phase like retail's AABB tree: disjoint bounds cannot overlap.
    let ((qa, qb), (va, vb)) = (query.aabb(), volume.aabb());
    if (0..3).any(|i| qb[i] < va[i] || vb[i] < qa[i]) {
        return false;
    }
    distance(query, volume) <= f64::from(volume.fatness)
}

/// Separation between the shapes, 0 when they overlap (GJK on the Minkowski
/// difference with exact cylinder and box support mappings).
pub fn distance(query: &Cylinder, volume: &OrientedBox) -> f64 {
    let support = |d: [f64; 3]| sub(query.support(d), volume.support(neg(d)));
    let mut v = sub(query.center.map(f64::from), volume.center.map(f64::from));
    if dot(v, v) < 1e-18 {
        return 0.;
    }
    let mut simplex: Vec<[f64; 3]> = Vec::with_capacity(4);
    for _ in 0..64 {
        let w = support(neg(v));
        let vv = dot(v, v);
        // No progress towards the origin: v is (within rounding) the closest point.
        if vv - dot(v, w) <= 1e-10 * vv.max(1.) {
            return vv.sqrt();
        }
        simplex.push(w);
        let (closest, reduced) = closest_on_simplex(&simplex);
        simplex = reduced;
        v = closest;
        if simplex.len() == 4 || dot(v, v) < 1e-14 {
            return 0.;
        }
    }
    dot(v, v).sqrt()
}

/// Closest point of the simplex to the origin and the smallest sub-simplex holding it.
fn closest_on_simplex(s: &[[f64; 3]]) -> ([f64; 3], Vec<[f64; 3]>) {
    match s.len() {
        1 => (s[0], s.to_vec()),
        2 => segment(s[0], s[1]),
        3 => triangle(s[0], s[1], s[2]),
        _ => tetrahedron(s[0], s[1], s[2], s[3]),
    }
}

fn segment(a: [f64; 3], b: [f64; 3]) -> ([f64; 3], Vec<[f64; 3]>) {
    let ab = sub(b, a);
    let t = -dot(a, ab);
    if t <= 0. {
        return (a, vec![a]);
    }
    let denom = dot(ab, ab);
    if t >= denom {
        return (b, vec![b]);
    }
    (add(a, scale(ab, t / denom)), vec![a, b])
}

// Ericson, Real-Time Collision Detection 5.1.5, with p = origin.
fn triangle(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> ([f64; 3], Vec<[f64; 3]>) {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = neg(a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return (a, vec![a]);
    }
    let bp = neg(b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return (b, vec![b]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return (add(a, scale(ab, v)), vec![a, b]);
    }
    let cp = neg(c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return (c, vec![c]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let w = d2 / (d2 - d6);
        return (add(a, scale(ac, w)), vec![a, c]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (add(b, scale(sub(c, b), w)), vec![b, c]);
    }
    let denom = 1. / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (add(a, add(scale(ab, v), scale(ac, w))), vec![a, b, c])
}

fn tetrahedron(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> ([f64; 3], Vec<[f64; 3]>) {
    // Origin outside the plane of a face (on the side away from the fourth vertex)?
    let outside = |p: [f64; 3], q: [f64; 3], r: [f64; 3], s: [f64; 3]| {
        let n = cross(sub(q, p), sub(r, p));
        let sign_o = dot(neg(p), n);
        let sign_s = dot(sub(s, p), n);
        sign_o * sign_s < 0.
    };
    let faces = [(a, b, c, d), (a, c, d, b), (a, d, b, c), (b, d, c, a)];
    let volume = dot(sub(b, a), cross(sub(c, a), sub(d, a)));
    let size = [b, c, d]
        .iter()
        .map(|p| dot(sub(*p, a), sub(*p, a)))
        .fold(0., f64::max);
    // A flat tetrahedron contains nothing: keep the nearest face instead.
    let flat = volume.abs() <= 1e-9 * size.max(1e-12).powf(1.5);
    let mut best: Option<([f64; 3], Vec<[f64; 3]>)> = None;
    for (p, q, r, s) in faces {
        if flat || outside(p, q, r, s) {
            let candidate = triangle(p, q, r);
            if best
                .as_ref()
                .is_none_or(|(x, _)| dot(candidate.0, candidate.0) < dot(*x, *x))
            {
                best = Some(candidate);
            }
        }
    }
    best.unwrap_or(([0.; 3], vec![a, b, c, d]))
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn neg(a: [f64; 3]) -> [f64; 3] {
    [-a[0], -a[1], -a[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Entered,
    Exited,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriggerEvent<B, V> {
    pub body: B,
    pub volume: V,
    pub transition: Transition,
}

/// Which volumes each tracked body is inside, and the transitions per update.
#[derive(Clone, Debug)]
pub struct Tracker<B: Ord + Clone, V: Ord + Clone> {
    inside: BTreeMap<B, Vec<V>>,
}

impl<B: Ord + Clone, V: Ord + Clone> Default for Tracker<B, V> {
    fn default() -> Self {
        Self {
            inside: BTreeMap::new(),
        }
    }
}

impl<B: Ord + Clone, V: Ord + Clone> Tracker<B, V> {
    /// Volumes the body is currently inside, in volume order.
    pub fn inside(&self, body: &B) -> &[V] {
        self.inside.get(body).map_or(&[], Vec::as_slice)
    }

    pub fn bodies(&self) -> impl Iterator<Item = (&B, &[V])> {
        self.inside.iter().map(|(b, v)| (b, v.as_slice()))
    }

    /// Forget everything without events (a world change replaces all volumes).
    pub fn clear(&mut self) {
        self.inside.clear();
    }

    /// One group update. `bodies` in slot order, `volumes` in registration
    /// order. Bodies no longer listed are removed with exit events; volumes no
    /// longer listed are forgotten silently, as in retail.
    pub fn update(
        &mut self,
        bodies: &[(B, Cylinder)],
        volumes: &[(V, OrientedBox)],
    ) -> Vec<TriggerEvent<B, V>> {
        let mut events = Vec::new();
        let listed: Vec<&B> = bodies.iter().map(|(b, _)| b).collect();
        let removed: Vec<B> = self
            .inside
            .keys()
            .filter(|b| !listed.contains(b))
            .cloned()
            .collect();
        for body in removed {
            for volume in self.inside.remove(&body).unwrap_or_default() {
                if volumes.iter().any(|(v, _)| *v == volume) {
                    events.push(TriggerEvent {
                        body: body.clone(),
                        volume,
                        transition: Transition::Exited,
                    });
                }
            }
        }
        for (body, query) in bodies {
            let now: Vec<V> = volumes
                .iter()
                .filter(|(_, shape)| overlaps(query, shape))
                .map(|(v, _)| v.clone())
                .collect();
            let before: Vec<V> = self
                .inside
                .get(body)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|v| volumes.iter().any(|(id, _)| id == v))
                .collect();
            for volume in now.iter().filter(|v| !before.contains(v)) {
                events.push(TriggerEvent {
                    body: body.clone(),
                    volume: volume.clone(),
                    transition: Transition::Entered,
                });
            }
            for volume in before.iter().filter(|v| !now.contains(v)) {
                events.push(TriggerEvent {
                    body: body.clone(),
                    volume: volume.clone(),
                    transition: Transition::Exited,
                });
            }
            self.inside.insert(body.clone(), now);
        }
        events
    }
}
