//! Authored weather geometry over the exact decoded map vertices and instance transforms.
//! Static BVH and admitted terrain/cover columns feed background rainwater preparation.

use crate::weather_runtime::{LOOK_PUDDLE_SIZE, WIND};

/// PC weather overlay: puddles are permitted through this inclusive slope.
pub(crate) const PUDDLE_MAX_SLOPE_DEGREES: f32 = 15.0;
use skate2_formats::render::{NativeMeshGeometry, NativeVertexAttributeValues, VertexSemantic};
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Copy)]
pub(crate) struct Triangle {
    pub(crate) points: [[f32; 3]; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) group: u64,
    wettable: bool,
    blocks_rain: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Patch {
    pub(crate) points: [[f32; 3]; 3],
    #[cfg(test)]
    pub(crate) normal: [f32; 3],
    /// Per-mesh conservative CDF of a continuous world-space field.
    pub(crate) curve: u32,
    /// Conservative support bound for coverage tests; the GPU evaluates the curve per pixel.
    #[cfg(test)]
    pub(crate) rank: f32,
    pub(crate) area: f64,
    pub(crate) group: u64,
}

struct Node {
    min: [f32; 3],
    max: [f32; 3],
    range: Range<usize>,
    children: Option<[usize; 2]>,
}

#[derive(Default)]
pub(crate) struct World {
    pub(crate) triangles: Vec<Triangle>,
    nodes: Vec<Node>,
    pub(crate) patches: Vec<Patch>,
    pub(crate) curves: Vec<crate::weather_puddles::Curve>,
}

pub(crate) fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
pub(crate) fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn normalized(a: [f32; 3]) -> [f32; 3] {
    let length = dot(a, a).sqrt();
    if length > 1e-10 {
        a.map(|v| v / length)
    } else {
        [0.0; 3]
    }
}
pub(crate) fn sky_direction() -> [f32; 3] {
    normalized(WIND.map(|v| -v))
}

impl World {
    #[cfg(test)]
    pub(crate) fn add_mesh(
        &mut self,
        geometry: &NativeMeshGeometry,
        matrix: Option<[f32; 16]>,
        group: u64,
        wettable: bool,
    ) {
        self.add_receiver_mesh(geometry, matrix, group, wettable, true);
    }

    #[expect(
        clippy::many_single_char_names,
        reason = "conventional components in the inverse-transpose cofactor equations"
    )]
    pub(crate) fn add_receiver_mesh(
        &mut self,
        geometry: &NativeMeshGeometry,
        matrix: Option<[f32; 16]>,
        group: u64,
        wettable: bool,
        blocks_rain: bool,
    ) {
        let normals = geometry
            .attributes
            .iter()
            .find(|a| a.semantic == VertexSemantic::Normal(0));
        for draw in &geometry.draws {
            let start = draw.start_index as usize;
            let Some(indices) = geometry
                .indices
                .get(start..start.saturating_add(draw.index_count as usize))
            else {
                continue;
            };
            for indices in indices.chunks_exact(3) {
                let mut points = [[0.0; 3]; 3];
                let mut reference = [0.0; 3];
                let mut valid = true;
                for (corner, &index) in indices.iter().enumerate() {
                    let Ok(index) =
                        usize::try_from(i64::from(index) + i64::from(draw.base_vertex_index))
                    else {
                        valid = false;
                        break;
                    };
                    let Some(&position) = geometry.positions.get(index) else {
                        valid = false;
                        break;
                    };
                    points[corner] = matrix.map_or(position, |m| {
                        std::array::from_fn(|i| {
                            m[i] * position[0]
                                + m[4 + i] * position[1]
                                + m[8 + i] * position[2]
                                + m[12 + i]
                        })
                    });
                    let n = normals.and_then(|attribute| match &attribute.values {
                        NativeVertexAttributeValues::Float3(v) => v.get(index).copied(),
                        NativeVertexAttributeValues::Float4(v) => {
                            v.get(index).map(|n| [n[0], n[1], n[2]])
                        }
                        _ => None,
                    });
                    if let Some(n) = n {
                        let n = matrix.map_or(n, |m| {
                            // Cofactors are the inverse-transpose up to a common determinant;
                            // only the orientation sign is needed to align the exact face normal.
                            let a = [m[0], m[1], m[2]];
                            let b = [m[4], m[5], m[6]];
                            let c = [m[8], m[9], m[10]];
                            let det = dot(a, cross(b, c)).signum();
                            let (x, y, z) = (cross(b, c), cross(c, a), cross(a, b));
                            std::array::from_fn(|i| (x[i] * n[0] + y[i] * n[1] + z[i] * n[2]) * det)
                        });
                        for i in 0..3 {
                            reference[i] += n[i];
                        }
                    }
                }
                if !valid || !points.iter().flatten().all(|v| v.is_finite()) {
                    continue;
                }
                let mut normal =
                    normalized(cross(sub(points[1], points[0]), sub(points[2], points[0])));
                if dot(normal, normal) < 0.5 {
                    continue;
                }
                if dot(normal, reference) < 0.0 {
                    normal = normal.map(|v| -v);
                }
                self.triangles.push(Triangle {
                    points,
                    normal,
                    group,
                    wettable,
                    blocks_rain,
                });
            }
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "area sums use f64; native vertex coordinates and upward-rounded ranks use f32"
    )]
    pub(crate) fn finish(self)->Self { self.finish_while(||true).unwrap_or_default() }
    pub(crate) fn release_build_scratch(&mut self) {self.patches=Vec::new();}
    /// The runtime wet field is global; only receiver/cover geometry is needed.
    pub(crate) fn finish_geometry_while(mut self,alive:impl Fn()->bool)->Option<Self> {
        if !alive(){return None;}
        if !self.triangles.is_empty(){self.split(0..self.triangles.len());}
        alive().then_some(self)
    }
    pub(crate) fn finish_while(mut self,alive:impl Fn()->bool)->Option<Self> {
        if !alive() {return None;}
        if self.triangles.is_empty() {
            return Some(self);
        }
        self.split(0..self.triangles.len());
        if !alive() {return None;}
        let mut work=0usize;
        let total_area: f64 = self
            .triangles
            .iter()
            .filter(|t| t.wettable)
            .map(|t| area(t.points))
            .sum();
        // Bound work for very large resident scenes; small levels keep sub-metre patches.
        let edge = (total_area / 80_000.0)
            .sqrt()
            .max(f64::from(LOOK_PUDDLE_SIZE) * 0.23) as f32;
        // Small floor pieces need enough patches to represent a sub-40% pool;
        // otherwise a two-triangle slab has ranks 50%/100% and can never puddle.
        let mut flat_areas = BTreeMap::<u64, f64>::new();
        for triangle in &self.triangles {
            if triangle.wettable && triangle.normal[1] >= PUDDLE_MAX_SLOPE_DEGREES.to_radians().cos() {
                *flat_areas.entry(triangle.group).or_default() += area(triangle.points);
            }
        }
        let sky = sky_direction();
        let mut groups = BTreeMap::<u64, Vec<usize>>::new();
        for triangle in &self.triangles {
            if !triangle.wettable || triangle.normal[1]<PUDDLE_MAX_SLOPE_DEGREES.to_radians().cos()
                || dot(triangle.normal,sky)<=0.005 {continue;}
            let edge = if triangle.normal[1] >= PUDDLE_MAX_SLOPE_DEGREES.to_radians().cos() {
                flat_areas
                    .get(&triangle.group)
                    .map_or(edge, |area| edge.min((area / 128.0).sqrt() as f32))
            } else {
                edge
            };
            let mut stack = vec![(triangle.points, 0)];
            while let Some((p, depth)) = stack.pop() {
                work+=1;if work%1024==0 && !alive() {return None;}
                let edges = [
                    dot(sub(p[1], p[0]), sub(p[1], p[0])),
                    dot(sub(p[2], p[1]), sub(p[2], p[1])),
                    dot(sub(p[0], p[2]), sub(p[0], p[2])),
                ];
                let longest = (0..3)
                    .max_by(|&a, &b| edges[a].total_cmp(&edges[b]))
                    .unwrap_or(0);
                if edges[longest] > edge * edge && depth < 12 {
                    let (a, b, c) = (longest, (longest + 1) % 3, (longest + 2) % 3);
                    let middle = std::array::from_fn(|i| (p[a][i] + p[b][i]) * 0.5);
                    stack.push(([p[a], middle, p[c]], depth + 1));
                    stack.push(([middle, p[b], p[c]], depth + 1));
                    continue;
                }
                // Host large-city adaptation: this CDF budgets the whole flat mesh.
                // The original exact GPU cover query rejects sheltered puddle pixels.
                // Retain all cover triangles in the BVH, not millions of CPU patch rays.
                let index = self.patches.len();
                self.patches.push(Patch {
                    points: p,
                    #[cfg(test)]
                    normal: triangle.normal,
                    curve: crate::weather_puddles::NO_CURVE,
                    #[cfg(test)]
                    rank: 2.0,
                    area: area(p),
                    group: triangle.group,
                });
                if triangle.normal[1] >= PUDDLE_MAX_SLOPE_DEGREES.to_radians().cos() {
                    groups.entry(triangle.group).or_default().push(index);
                }
            }
        }
        for indices in groups.values() {
            if !alive() {return None;}
            let coarse=indices.iter().any(|&i| {
                let p=self.patches[i].points;
                (0..3).any(|j|dot(sub(p[(j+1)%3],p[j]),sub(p[(j+1)%3],p[j]))>(LOOK_PUDDLE_SIZE*0.35).powi(2))
            });
            let samples=||indices.iter().map(|&i|{let patch=&self.patches[i];(patch.points,patch.area)});
            let curve=if coarse {crate::weather_puddles::Curve::sampled(samples())}
                else {crate::weather_puddles::Curve::build(samples())};
            let index = u32::try_from(self.curves.len()).expect("resident mesh curve index");
            for &i in indices {
                self.patches[i].curve = index;
                #[cfg(test)]
                {
                    self.patches[i].rank=curve.rank(if coarse {crate::weather_puddles::centre_value(self.patches[i].points)}
                        else {crate::weather_puddles::lower_bound(self.patches[i].points)});
                }
            }
            self.curves.push(curve);
        }
        Some(self)
    }

    /// GPU query-only BVH. There are no weather vertex/index buffers: depth supplies
    /// the overlay position, while this data admits materials and rejects shelter.
    pub(crate) fn overlay_index(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        if self.nodes.is_empty() || !self.triangles.iter().any(|t| t.wettable) {
            return None;
        }
        let mut escapes = vec![0_usize; self.nodes.len()];
        for i in (0..self.nodes.len()).rev() {
            escapes[i] = self.nodes[i].children.map_or(i + 1, |c| escapes[c[1]]);
        }
        let mut nodes = Vec::with_capacity(self.nodes.len() * 32);
        for (i, n) in self.nodes.iter().enumerate() {
            let first = if n.children.is_some() {
                u32::MAX
            } else {
                let first = u32::try_from(n.range.start).ok()?;
                if first >= (1 << 28) {
                    return None;
                }
                first | (u32::try_from(n.range.len() - 1).ok()? << 28)
            };
            nodes.extend(n.min.into_iter().flat_map(f32::to_ne_bytes));
            nodes.extend(first.to_ne_bytes());
            nodes.extend(n.max.into_iter().flat_map(f32::to_ne_bytes));
            nodes.extend(u32::try_from(escapes[i]).ok()?.to_ne_bytes());
        }
        let mut group_curves = BTreeMap::new();
        for patch in &self.patches {
            if patch.curve != crate::weather_puddles::NO_CURVE {
                group_curves.insert(patch.group, patch.curve);
            }
        }
        let mut triangles = Vec::with_capacity(self.triangles.len() * 64);
        for t in &self.triangles {
            triangles.extend(t.points[0].into_iter().flat_map(f32::to_ne_bytes));
            triangles
                .extend((u32::from(t.wettable) | (u32::from(t.blocks_rain) << 1)).to_ne_bytes());
            triangles.extend(
                sub(t.points[1], t.points[0])
                    .into_iter()
                    .flat_map(f32::to_ne_bytes),
            );
            triangles.extend(
                group_curves
                    .get(&t.group)
                    .copied()
                    .unwrap_or(crate::weather_puddles::NO_CURVE)
                    .to_ne_bytes(),
            );
            triangles.extend(
                sub(t.points[2], t.points[0])
                    .into_iter()
                    .flat_map(f32::to_ne_bytes),
            );
            triangles.extend(0_u32.to_ne_bytes());
            triangles.extend(t.normal.into_iter().chain([0.0]).flat_map(f32::to_ne_bytes));
        }
        Some((nodes, triangles))
    }

    fn split(&mut self, range: Range<usize>) -> usize {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for t in &self.triangles[range.clone()] {
            for p in t.points {
                for i in 0..3 {
                    min[i] = min[i].min(p[i]);
                    max[i] = max[i].max(p[i]);
                }
            }
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            min,
            max,
            range: range.clone(),
            children: None,
        });
        if range.len() > 8 {
            let axis = (0..3)
                .max_by(|&a, &b| (max[a] - min[a]).total_cmp(&(max[b] - min[b])))
                .unwrap_or(0);
            self.triangles[range.clone()].select_nth_unstable_by(range.len() / 2, |a, b| {
                a.points
                    .iter()
                    .map(|p| p[axis])
                    .sum::<f32>()
                    .total_cmp(&b.points.iter().map(|p| p[axis]).sum::<f32>())
            });
            let mid = range.start + range.len() / 2;
            let a = self.split(range.start..mid);
            let b = self.split(mid..range.end);
            self.nodes[index].children = Some([a, b]);
        }
        index
    }


    /// Rasterise exact transformed map triangles for the local visual-water model.
    /// Roof cover and receiving terrain are independent, including bowl slopes.
    pub(crate) fn rain_columns(&self, origin:[f32;2], step:f32, size:usize, ceiling:f32)
        -> Vec<crate::weather_hydrology::Ground> {
        use crate::weather_hydrology::Ground;
        let mut out=vec![Ground::default();size*size];
        let mut cover=vec![f32::NEG_INFINITY;size*size];
        let edge=(size-1) as f32;
        for triangle in &self.triangles {
            if !triangle.blocks_rain && !triangle.wettable {continue;}
            let [a,b,c]=triangle.points;
            let det=(b[2]-c[2])*(a[0]-c[0])+(c[0]-b[0])*(a[2]-c[2]);
            if det.abs()<1e-7 {continue;}
            let min_x=(a[0].min(b[0]).min(c[0])-origin[0])/step;
            let max_x=(a[0].max(b[0]).max(c[0])-origin[0])/step;
            let min_z=(a[2].min(b[2]).min(c[2])-origin[1])/step;
            let max_z=(a[2].max(b[2]).max(c[2])-origin[1])/step;
            if max_x<0. || max_z<0. || min_x>edge || min_z>edge {continue;}
            let x0=min_x.ceil().clamp(0.,edge) as usize;let x1=max_x.floor().clamp(0.,edge) as usize;
            let z0=min_z.ceil().clamp(0.,edge) as usize;let z1=max_z.floor().clamp(0.,edge) as usize;
            for z in z0..=z1 {for x in x0..=x1 {
                let wx=origin[0]+x as f32*step;let wz=origin[1]+z as f32*step;
                let u=((b[2]-c[2])*(wx-c[0])+(c[0]-b[0])*(wz-c[2]))/det;
                let v=((c[2]-a[2])*(wx-c[0])+(a[0]-c[0])*(wz-c[2]))/det;
                if u< -0.0001 || v< -0.0001 || u+v>1.0001 {continue;}
                let y=u*a[1]+v*b[1]+(1.-u-v)*c[1];let i=z*size+x;
                if triangle.blocks_rain {cover[i]=cover[i].max(y);}
                if triangle.wettable && triangle.normal[1].abs()>0.00001 && y<=ceiling
                    && (!out[i].valid || y>out[i].height) {
                    out[i]=Ground{height:y,valid:true,exposed:false};
                }
            }}
        }
        for (cell,roof) in out.iter_mut().zip(cover) {
            cell.exposed=cell.valid && roof<=cell.height+0.12;
        }
        out
    }

    /// Only upward-facing, admitted wet surfaces can produce board spray.
    pub(crate) fn wet_contact(&self,origin:[f32;3],limit:f32)->Option<[f32;3]> {
        if self.nodes.is_empty(){return None;}let mut best=limit;
        self.query(0,origin,[0.,-1.,0.],&mut best,true);
        (best<limit).then_some([origin[0],origin[1]-best,origin[2]])
    }

    /// Same static cover geometry as rain streaks, sampled over a small sky cone.
    pub(crate) fn listener_exposure(&self, position: [f32; 3]) -> f32 {
        let sky = sky_direction();
        let mut clear = 0_u16;
        for (x, z) in [
            (0.0, 0.0),
            (0.30, 0.0),
            (-0.30, 0.0),
            (0.0, 0.30),
            (0.0, -0.30),
        ] {
            let direction = normalized([sky[0] + x, sky[1], sky[2] + z]);
            if self.ray_distance(position, direction, 10_000.0).is_none() {
                clear += 1;
            }
        }
        f32::from(clear) / 5.0
    }

    pub(crate) fn ray_distance(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        limit: f32,
    ) -> Option<f32> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best = limit;
        self.query(0, origin, direction, &mut best,false);
        (best < limit).then_some(best)
    }

    #[expect(
        clippy::many_single_char_names,
        reason = "standard two-sided Moller-Trumbore ray/triangle coordinates"
    )]
    fn query(&self, index: usize, origin: [f32; 3], direction: [f32; 3], best: &mut f32,wet:bool) {
        let node = &self.nodes[index];
        let (mut enter, mut leave) = (0.0_f32, *best);
        for i in 0..3 {
            if direction[i].abs() < 1e-9 {
                if origin[i] < node.min[i] || origin[i] > node.max[i] {
                    return;
                }
            } else {
                let a = (node.min[i] - origin[i]) / direction[i];
                let b = (node.max[i] - origin[i]) / direction[i];
                enter = enter.max(a.min(b));
                leave = leave.min(a.max(b));
                if leave < enter {
                    return;
                }
            }
        }
        if let Some([a, b]) = node.children {
            self.query(a, origin, direction, best,wet);
            self.query(b, origin, direction, best,wet);
        } else {
            for t in &self.triangles[node.range.clone()] {
                if !t.blocks_rain || (wet && (!t.wettable || t.normal[1]<PUDDLE_MAX_SLOPE_DEGREES.to_radians().cos())) {
                    continue;
                }
                let (e1, e2) = (sub(t.points[1], t.points[0]), sub(t.points[2], t.points[0]));
                let h = cross(direction, e2);
                let det = dot(e1, h);
                if det.abs() < 1e-8 {
                    continue;
                }
                let s = sub(origin, t.points[0]);
                let u = dot(s, h) / det;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(direction, q) / det;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let distance = dot(e2, q) / det;
                if distance > 0.0001 && distance < *best {
                    *best = distance;
                }
            }
        }
    }
}

#[expect(
    clippy::many_single_char_names,
    reason = "expanded double-precision triangle cross product"
)]
fn area(p: [[f32; 3]; 3]) -> f64 {
    let a = p[0].map(f64::from);
    let b = p[1].map(f64::from);
    let c = p[2].map(f64::from);
    let e: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
    let f: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
    ((e[1] * f[2] - e[2] * f[1]).powi(2)
        + (e[2] * f[0] - e[0] * f[2]).powi(2)
        + (e[0] * f[1] - e[1] * f[0]).powi(2))
    .sqrt()
        * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::render::{DrawIndexedParameters, NativeVertexAttribute};
    fn plane(y: f32, size: f32) -> NativeMeshGeometry {
        NativeMeshGeometry {
            positions: vec![
                [-size, y, -size],
                [-size, y, size],
                [size, y, size],
                [size, y, -size],
            ],
            attributes: vec![NativeVertexAttribute {
                semantic: VertexSemantic::Normal(0),
                original_format: 0,
                values: NativeVertexAttributeValues::Float3(vec![[0.0, 1.0, 0.0]; 4]),
            }],
            indices: vec![0, 1, 2, 0, 2, 3],
            draws: vec![DrawIndexedParameters {
                primitive_type: 4,
                base_vertex_index: 0,
                start_index: 0,
                index_count: 6,
            }],
            position_format: 0,
        }
    }
    #[test]
    fn runtime_geometry_path_keeps_contact_and_cover_without_unused_patch_work(){
        let mut world=World::default();world.add_mesh(&plane(0.,5.),None,1,true);
        let world=world.finish_geometry_while(||true).unwrap();
        assert!(world.patches.is_empty() && world.curves.is_empty());
        assert!(world.overlay_index().is_some());
        assert_eq!(world.wet_contact([0.,0.2,0.],0.5),Some([0.,0.,0.]));
        assert!(World::default().finish_geometry_while(||false).is_none());
    }
    #[test]
    fn area_rank_guarantees_coverage_cap_on_each_mesh() {
        let mut world = World::default();
        world.add_mesh(&plane(0.0, 12.0), None, 1, true);
        let world = world.finish();
        let total: f64 = world.patches.iter().map(|p| p.area).sum();
        assert!(total > 570.0);
        for fraction in [
            0.0,
            0.1,
            0.2,
            0.306,
            0.35,
            crate::weather_runtime::MAX_PUDDLE_COVERAGE,
        ] {
            let covered: f64 = world
                .patches
                .iter()
                .filter(|p| p.rank <= fraction)
                .map(|p| p.area)
                .sum();
            assert!(covered / total <= f64::from(fraction) + 1e-8);
            assert!(covered / total < 0.7);
        }
    }
    #[test]
    fn small_floor_pieces_can_form_puddles_with_soft_edges_inside_the_area_cap() {
        let mut world = World::default();
        world.add_mesh(&plane(0.0, 0.1), None, 1, true);
        let world = world.finish();
        let total: f64 = world.patches.iter().map(|p| p.area).sum();
        let selected: f64 = world
            .patches
            .iter()
            .filter(|p| p.rank <= 0.306)
            .map(|p| p.area)
            .sum();
        assert!(selected / total > 0.25 && selected / total <= 0.306);
        assert!(
            world
                .patches
                .iter()
                .all(|p| p.curve != crate::weather_puddles::NO_CURVE)
        );
    }
    #[test]
    fn cover_bvh_rejects_floor_below_roof_while_cdf_budgets_all_flat_area() {
        let mut world = World::default();
        world.add_mesh(&plane(0.0, 5.0), None, 1, true);
        world.add_mesh(&plane(4.0, 2.0), None, 2, true);
        let world = world.finish();
        assert!(
            world
                .ray_distance([0.0, 1.0, 0.0], sky_direction(), 100.0)
                .is_some()
        );
        assert!(
            world
                .ray_distance([0.0, 4.1, 0.0], sky_direction(), 100.0)
                .is_none()
        );
        assert!(world.patches.iter().any(|p|p.group==1));
        assert!(world.patches.iter().any(|p| p.group == 2 && p.rank <= 0.35));
    }
    #[test]
    fn continuous_puddle_contours_do_not_jump_at_the_original_floor_diagonal() {
        let mut world = World::default();
        world.add_mesh(&plane(0.0, 12.0), None, 1, true);
        let world = world.finish();
        let curve = &world.curves[0];
        let mut wet_area = 0.0;
        for patch in &world.patches {
            let centre =
                std::array::from_fn(|axis| patch.points.iter().map(|p| p[axis]).sum::<f32>() / 3.0);
            let rank = curve.rank(crate::weather_puddles::field(centre));
            assert!(rank + 0.00001 >= patch.rank);
            if rank < 0.306 {
                wet_area += patch.area;
            }
        }
        let total: f64 = world.patches.iter().map(|p| p.area).sum();
        assert!(wet_area / total > 0.05 && wet_area / total <= 0.306);
        for index in 0..200_u16 {
            let coordinate = f32::from(index) * 0.1 - 10.0;
            let left =
                crate::weather_puddles::field([coordinate + 0.0001, 0.0, coordinate - 0.0001]);
            let right =
                crate::weather_puddles::field([coordinate - 0.0001, 0.0, coordinate + 0.0001]);
            assert!(
                (curve.rank(left) - curve.rank(right)).abs() < 0.01,
                "puddle mask jumped across a triangle edge"
            );
        }
    }
    #[test]
    fn listener_cover_is_derived_from_the_same_roof_that_blocks_rain() {
        let mut world = World::default();
        world.add_mesh(&plane(4.0, 5.0), None, 1, true);
        let world = world.finish();
        assert_eq!(world.listener_exposure([0.0, 1.0, 0.0]), 0.0);
        assert_eq!(world.listener_exposure([0.0, 5.0, 0.0]), 1.0);
        assert_eq!(world.listener_exposure([10.0, 1.0, 0.0]), 1.0);
    }
    #[test]
    fn steep_surfaces_keep_cover_triangles_without_puddle_subdivision() {
        let mut mesh = plane(0.0, 4.0);
        for p in &mut mesh.positions {
            p[1] = p[0] * 15.1_f32.to_radians().tan();
        }
        let mut world = World::default();
        world.add_mesh(&mesh, None, 1, true);
        let world = world.finish();
        assert!(world.patches.is_empty());
        assert_eq!(world.triangles.len(),2);
        assert!(world.patches.iter().all(|p| p.rank > 1.0));
    }
    #[test]
    fn puddles_include_the_fifteen_degree_boundary() {
        for angle in [0.0_f32, 5.0, 14.9, 15.0] {
            let mut mesh = plane(0.0, 4.0);
            for p in &mut mesh.positions { p[1] = p[0] * angle.to_radians().tan(); }
            let mut world = World::default();
            world.add_mesh(&mesh, None, 1, true);
            let world = world.finish();
            assert!(world.patches.iter().any(|p| p.rank < 0.35), "angle={angle}");
            let (nodes, triangles) = world.overlay_index().unwrap();
            assert_eq!(nodes.len() % 32, 0);
            assert_eq!(triangles.len(), world.triangles.len() * 64);
        }
    }

    #[test]
    fn large_city_floor_keeps_continuous_puddles_instead_of_collapsed_minima(){
        let mut world=World::default();world.add_mesh(&plane(0.,2000.),None,1,true);let world=world.finish();
        let curve=&world.curves[0];let old=crate::weather_puddles::Curve::build(world.patches.iter().map(|p|(p.points,p.area)));
        let mut covered=0;let mut old_covered=0;
        for x in 0..64 {for z in 0..64 {
            let field=crate::weather_puddles::field([-1983.+x as f32*62.4,0.,-1991.+z as f32*62.3]);
            covered+=usize::from(curve.rank(field)<0.699);old_covered+=usize::from(old.rank(field)<0.699);
        }}
        let fraction=covered as f32/4096.;assert!(fraction>0.5&&fraction<0.85,"coverage={fraction}");
        assert!(old_covered<covered/3,"negative control must expose the coarse-minimum failure");
    }

}
