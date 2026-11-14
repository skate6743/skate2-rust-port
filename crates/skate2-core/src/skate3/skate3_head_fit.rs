//! PC port: Skate 2's head under a linked Skate 3 data folder's tops.
//!
//! Skate 3 remade its heads and made its tops on them; Skate 2's female head stands out of a Skate
//! 3 tee at the top of the shoulder, by the collar. The skater keeps its own head (Skate 3's is
//! another face), so where a Skate 3 top lies over the head's skin, that skin goes just under the
//! top: each head vertex out of the top's surface (the surface along its normal, inward, within
//! reach) or less than a margin under it (outward) moves in along its normal to the margin under
//! the surface. Skin the top does not cover stays where it is. A top is Skate 3's when its model
//! names one of Skate 3's helper bones ([`super::is_helper_bone`]).
use crate::character::CharacterComponentAsset;
use skate2_formats::render::{NativeMeshGeometry, NativeVertexAttributeValues, VertexSemantic};
use std::collections::HashMap;

/// How far under the top's surface the skin goes, in metres.
const MARGIN: f32 = 0.004;
/// How far out of the top's surface a vertex is still the top's to cover, in metres.
const REACH: f32 = 0.025;

/// What [`fit_vertices`] moved.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FitStats {
    pub candidates: usize,
    /// Vertices out of the top's surface.
    pub out: usize,
    /// Vertices less than the margin under it.
    pub under: usize,
    /// The farthest a vertex moved, in model units.
    pub farthest: f32,
}

/// Fits every head (`Rostral`) level of detail under the Skate 3 tops (`OuterTorso`,
/// `InnerTorso`) of `components`: each against the tops' level of detail of the same level, else
/// their nearest.
pub fn fit_heads(components: &mut [CharacterComponentAsset]) {
    let diagnostic = std::env::var("SKATE2_DIAGNOSTIC_S3_HEAD_FIT").ok();
    if diagnostic.as_deref() == Some("off") {
        return;
    }
    let tops: Vec<Vec<(u32, Vec<[[f32; 3]; 3]>)>> = components
        .iter()
        .filter(|component| ["OuterTorso", "InnerTorso"].iter().any(|name| component.name.eq_ignore_ascii_case(name)))
        .flat_map(|component| &component.models)
        .filter_map(|model| {
            let levels: Vec<(u32, Vec<[[f32; 3]; 3]>)> = model
                .lods
                .iter()
                .filter(|lod| lod.arena.models.iter().any(|m| m.bone_names.iter().any(|bone| super::is_helper_bone(bone))))
                .map(|lod| (u32::from(lod.level), lod.arena.meshes.iter().flat_map(|mesh| triangles(&mesh.geometry)).collect()))
                .collect();
            (!levels.is_empty()).then_some(levels)
        })
        .collect();
    if tops.is_empty() {
        return;
    }
    for component in components.iter_mut().filter(|component| component.name.eq_ignore_ascii_case("Rostral")) {
        for model in &mut component.models {
            for lod in &mut model.lods {
                let level = u32::from(lod.level);
                let occluders: Vec<[[f32; 3]; 3]> = tops
                    .iter()
                    .filter_map(|levels| levels.iter().min_by_key(|(top_level, _)| top_level.abs_diff(level)))
                    .flat_map(|(_, triangles)| triangles.iter().copied())
                    .collect();
                let scale = model_units_per_metre(&lod.arena.models);
                for mesh in &mut lod.arena.meshes {
                    let Some(normals) = normals(&mesh.geometry) else { continue };
                    let stats = fit_vertices(&mut mesh.geometry.positions, &normals, &occluders, MARGIN * scale, REACH * scale);
                    if diagnostic.is_some() {
                        eprintln!(
                            "s3 head fit: {} model {:#018x} lod {level}: {} top triangles, {} vertices near, {} out, {} under, farthest {:.1} mm",
                            component.name,
                            model.asset_id,
                            occluders.len(),
                            stats.candidates,
                            stats.out,
                            stats.under,
                            stats.farthest / scale * 1000.,
                        );
                    }
                }
            }
        }
    }
}

/// Model units in a metre: the inverse bind poses' scale (their columns' length).
fn model_units_per_metre(models: &[crate::presentation::PresentationModel]) -> f32 {
    let lengths: Vec<f32> = models
        .iter()
        .flat_map(|model| &model.inverse_bind_pose)
        .map(|m| (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt())
        .filter(|length| length.is_finite() && *length > 0.)
        .collect();
    if lengths.is_empty() {
        return 1.;
    }
    // The bind pose maps model units to metres: a metre is 1 / scale units.
    lengths.len() as f32 / lengths.iter().sum::<f32>()
}

/// A mesh's triangles: its draws' triangle lists.
fn triangles(geometry: &NativeMeshGeometry) -> Vec<[[f32; 3]; 3]> {
    let mut out = Vec::new();
    for draw in &geometry.draws {
        let start = draw.start_index as usize;
        let Some(indices) = geometry.indices.get(start..start + draw.index_count as usize) else { continue };
        for corners in indices.chunks_exact(3) {
            let vertex = |index: u32| {
                usize::try_from(i64::from(index) + i64::from(draw.base_vertex_index)).ok().and_then(|vertex| geometry.positions.get(vertex).copied())
            };
            if let (Some(a), Some(b), Some(c)) = (vertex(corners[0]), vertex(corners[1]), vertex(corners[2])) {
                out.push([a, b, c]);
            }
        }
    }
    out
}

/// A mesh's vertex normals, one per position.
fn normals(geometry: &NativeMeshGeometry) -> Option<Vec<[f32; 3]>> {
    let attribute = geometry.attributes.iter().find(|attribute| attribute.semantic == VertexSemantic::Normal(0))?;
    let normals: Vec<[f32; 3]> = match &attribute.values {
        NativeVertexAttributeValues::Float3(values) => values.clone(),
        NativeVertexAttributeValues::Float4(values) => values.iter().map(|v| [v[0], v[1], v[2]]).collect(),
        _ => return None,
    };
    (normals.len() == geometry.positions.len()).then_some(normals)
}

/// Moves each of `positions` out of the `occluders`' surface (along `-normal`, within `reach`) or
/// less than `margin` under it (along `normal`) in along its normal to `margin` under the surface.
/// Vertices at the same place move together, along their normals' mean, so no seam opens.
pub fn fit_vertices(positions: &mut [[f32; 3]], normals: &[[f32; 3]], occluders: &[[[f32; 3]; 3]], margin: f32, reach: f32) -> FitStats {
    let mut stats = FitStats::default();
    if occluders.is_empty() || positions.len() != normals.len() {
        return stats;
    }
    let grid = Grid::new(occluders, reach.max(margin));
    let mut places: HashMap<[u32; 3], Vec<usize>> = HashMap::new();
    for (index, position) in positions.iter().enumerate() {
        if grid.near(*position) {
            places.entry(position.map(f32::to_bits)).or_default().push(index);
        }
    }
    for vertices in places.values() {
        stats.candidates += vertices.len();
        let at = positions[vertices[0]];
        let sum = vertices.iter().fold([0.; 3], |sum, &index| add(sum, normals[index]));
        let Some(normal) = normalize(sum) else { continue };
        let push = match grid.hit(at, scale(normal, -1.), reach) {
            Some(depth) => {
                stats.out += vertices.len();
                depth + margin
            }
            None => match grid.hit(at, normal, margin) {
                Some(cover) => {
                    stats.under += vertices.len();
                    margin - cover
                }
                None => continue,
            },
        };
        stats.farthest = stats.farthest.max(push);
        for &index in vertices {
            positions[index] = add(positions[index], scale(normal, -push));
        }
    }
    stats
}

/// The occluders bucketed by cell, each in every cell its box (grown by the reach) touches.
struct Grid<'a> {
    triangles: &'a [[[f32; 3]; 3]],
    cell: f32,
    cells: HashMap<[i32; 3], Vec<u32>>,
}

impl<'a> Grid<'a> {
    fn new(triangles: &'a [[[f32; 3]; 3]], reach: f32) -> Self {
        let cell = (reach * 2.).max(1e-4);
        let mut cells: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (index, triangle) in triangles.iter().enumerate() {
            let low: [i32; 3] = std::array::from_fn(|axis| ((triangle.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min) - reach) / cell).floor() as i32);
            let high: [i32; 3] = std::array::from_fn(|axis| ((triangle.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max) + reach) / cell).floor() as i32);
            for x in low[0]..=high[0] {
                for y in low[1]..=high[1] {
                    for z in low[2]..=high[2] {
                        cells.entry([x, y, z]).or_default().push(index as u32);
                    }
                }
            }
        }
        Self { triangles, cell, cells }
    }

    fn key(&self, point: [f32; 3]) -> [i32; 3] {
        point.map(|value| (value / self.cell).floor() as i32)
    }

    fn near(&self, point: [f32; 3]) -> bool {
        self.cells.contains_key(&self.key(point))
    }

    /// The nearest occluder along `direction` from `origin`, no farther than `reach`.
    fn hit(&self, origin: [f32; 3], direction: [f32; 3], reach: f32) -> Option<f32> {
        self.cells
            .get(&self.key(origin))?
            .iter()
            .filter_map(|&index| ray_triangle(origin, direction, &self.triangles[index as usize]))
            .filter(|&t| t <= reach)
            .min_by(f32::total_cmp)
    }
}

/// Möller–Trumbore, either face: the distance along `direction` (unit) to the triangle.
fn ray_triangle(origin: [f32; 3], direction: [f32; 3], triangle: &[[f32; 3]; 3]) -> Option<f32> {
    let edge1 = sub(triangle[1], triangle[0]);
    let edge2 = sub(triangle[2], triangle[0]);
    let p = cross(direction, edge2);
    let det = dot(edge1, p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inverse = 1. / det;
    let s = sub(origin, triangle[0]);
    let u = dot(s, p) * inverse;
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = cross(s, edge1);
    let v = dot(direction, q) * inverse;
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = dot(edge2, q) * inverse;
    (t > 1e-6).then_some(t)
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f32; 3], by: f32) -> [f32; 3] {
    a.map(|value| value * by)
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn normalize(a: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(a, a).sqrt();
    (length > 1e-6).then(|| scale(a, 1. / length))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square of two triangles in the plane z = 0, a metre across.
    fn plane() -> Vec<[[f32; 3]; 3]> {
        vec![
            [[-0.5, -0.5, 0.], [0.5, -0.5, 0.], [0.5, 0.5, 0.]],
            [[-0.5, -0.5, 0.], [0.5, 0.5, 0.], [-0.5, 0.5, 0.]],
        ]
    }

    #[test]
    fn skin_out_of_the_top_or_just_under_it_goes_the_margin_under() {
        let up = [0., 0., 1.];
        let mut positions = vec![[0.1, 0., 0.005], [0.2, 0., -0.002], [0.3, 0., 0.05], [0.0, 0.1, -0.01]];
        let normals = vec![up; 4];
        let stats = fit_vertices(&mut positions, &normals, &plane(), 0.004, 0.025);
        // Out by 5 mm: in to 4 mm under.
        assert!((positions[0][2] + 0.004).abs() < 1e-6, "{:?}", positions[0]);
        // 2 mm under: in to 4 mm under.
        assert!((positions[1][2] + 0.004).abs() < 1e-6, "{:?}", positions[1]);
        // 5 cm out, past the reach: where it was.
        assert_eq!(positions[2], [0.3, 0., 0.05]);
        // Already 10 mm under: where it was.
        assert_eq!(positions[3], [0.0, 0.1, -0.01]);
        assert_eq!((stats.out, stats.under), (1, 1));
    }

    #[test]
    fn vertices_at_one_place_move_together_and_skin_away_from_the_top_stays() {
        let mut positions = vec![[0., 0., 0.003], [0., 0., 0.003], [2., 2., 0.003]];
        let normals = vec![[0.6, 0., 0.8], [-0.6, 0., 0.8], [0., 0., 1.]];
        fit_vertices(&mut positions, &normals, &plane(), 0.004, 0.025);
        assert_eq!(positions[0], positions[1]);
        assert!((positions[0][2] + 0.004).abs() < 1e-6, "{:?}", positions[0]);
        // Off the top's edge: no surface along its normal.
        assert_eq!(positions[2], [2., 2., 0.003]);
    }
}
