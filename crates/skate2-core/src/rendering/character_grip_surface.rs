//! PC port addition: a deck's griptape, for Edit Griptape's colour
//! (`edit_skater_record::GRIP_FILTER`).
//!
//! A deck mesh lays its faces out over its first texture coordinate set in islands that share no
//! vertex: the top face, the bottom face (one island or more) and the two rails. The griptape is
//! the top face: of the islands that lie flat in the mesh's bind space, where the board rests
//! under the skater, those on the higher side. Its colour goes over the rectangle of the first
//! set those islands span, which no other face reaches; a deck where one does has no griptape
//! here.
//!
//! The griptape is near black, so its colour is no filter over it as a garment's is: the diffuse
//! is first brought to a mean of one over the top face (`level`), which leaves the grain to take
//! the colour. See `docs/evidence/edit-skater-griptape-2026-11-16.md`.
use crate::texture::NativeTexture;
use skate2_formats::{
    material::{LinearTextureLevel, PIXEL_FORMAT_A8R8G8B8, PIXEL_FORMAT_DXT1, PIXEL_FORMAT_DXT5, PIXEL_FORMAT_LIN_DXT5},
    render::{NativeMeshGeometry, NativeVertexAttributeValues, VertexSemantic},
};

/// The customization colour that holds a deck's griptape colour.
pub const COLOUR_KEY: i32 = 15;

/// How much of an island's area looks up or down for it to lie flat.
const FLAT: f32 = 0.5;
/// The sample points along a triangle's edge the diffuse is read at.
const SAMPLES: usize = 6;
/// The least `level`: a diffuse darker than this over the top face is taken to be this bright.
const DARKEST: f32 = 1.0 / 4096.0;

type Uv = [f32; 2];

/// A deck mesh's griptape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripSurface {
    /// The rectangle of the first set the top face alone lies in: its least u and v, then its
    /// greatest.
    pub rectangle: [f32; 4],
    /// The mean over the top face of the diffuse squared, as the character programs read it.
    pub level: f32,
}

impl GripSurface {
    /// The griptape of a deck mesh drawn with `diffuse`.
    pub(crate) fn of(geometry: &NativeMeshGeometry, diffuse: &NativeTexture) -> Option<Self> {
        let uv = first_set(geometry)?;
        let triangles = triangles(geometry);
        let top = top_face(&geometry.positions, uv, &triangles)?;
        let diffuse = Diffuse::of(diffuse)?;
        let level = mean_square(uv, &triangles, &top.triangles, |uv| diffuse.at(uv))?;
        Some(Self { rectangle: top.rectangle, level: level.max(DARKEST) })
    }

    /// What the diffuse squared is multiplied by over the griptape for it to take `tint`, a
    /// colour filter's tint (`edit_skater_colour_filter`).
    #[must_use]
    pub fn gain(&self, tint: [f32; 3]) -> [f32; 3] {
        tint.map(|lane| if lane.is_finite() { lane.max(0.) / self.level } else { 0. })
    }
}

/// A mesh's top face: its triangles and the rectangle of the first set they span.
#[derive(Debug, PartialEq)]
struct TopFace {
    triangles: Vec<usize>,
    rectangle: [f32; 4],
}

/// The islands of `triangles` (joined by the vertices they share) that lie flat on the higher
/// side of the mesh. None when the mesh has no two flat sides, or another face reaches into the
/// rectangle the top one spans.
fn top_face(positions: &[[f32; 3]], uv: &[Uv], triangles: &[[usize; 3]]) -> Option<TopFace> {
    let mut island: Vec<usize> = (0..positions.len()).collect();
    fn root(island: &mut [usize], mut vertex: usize) -> usize {
        while island[vertex] != vertex {
            island[vertex] = island[island[vertex]];
            vertex = island[vertex];
        }
        vertex
    }
    for triangle in triangles {
        let first = root(&mut island, triangle[0]);
        for &vertex in &triangle[1..] {
            let other = root(&mut island, vertex);
            island[other] = first;
        }
    }
    // For each island: its doubled area, how much of it looks up, and its height times its area.
    let mut sums: std::collections::HashMap<usize, [f64; 3]> = std::collections::HashMap::new();
    let islands: Vec<usize> = triangles.iter().map(|triangle| root(&mut island, triangle[0])).collect();
    for (triangle, island) in triangles.iter().zip(&islands) {
        let [a, b, c] = triangle.map(|vertex| positions[vertex].map(f64::from));
        let (ab, ac) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let normal = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
        let area = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let sum = sums.entry(*island).or_default();
        sum[0] += area;
        sum[1] += normal[1];
        sum[2] += area * (a[1] + b[1] + c[1]) / 3.;
    }
    // The two sides the flat islands look to, each with its area and its height times its area.
    let mut sides = [[0.0_f64; 2]; 2];
    let side_of = |sum: &[f64; 3]| (sum[0] > 0. && (sum[1] / sum[0]).abs() > f64::from(FLAT)).then_some(usize::from(sum[1] > 0.));
    for sum in sums.values() {
        if let Some(side) = side_of(sum) {
            sides[side][0] += sum[0];
            sides[side][1] += sum[2];
        }
    }
    if sides.iter().any(|side| side[0] <= 0.) {
        return None;
    }
    let higher = usize::from(sides[1][1] / sides[1][0] > sides[0][1] / sides[0][0]);
    let on_top = |island: &usize| sums.get(island).and_then(side_of) == Some(higher);
    let span = |triangle: &[usize; 3]| {
        let corners = triangle.map(|vertex| uv[vertex]);
        let lane = |lane: usize| corners.iter().map(move |corner| corner[lane]);
        [lane(0).fold(f32::INFINITY, f32::min), lane(1).fold(f32::INFINITY, f32::min), lane(0).fold(f32::NEG_INFINITY, f32::max), lane(1).fold(f32::NEG_INFINITY, f32::max)]
    };
    let mut top = TopFace { triangles: Vec::new(), rectangle: [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY] };
    for (index, (triangle, island)) in triangles.iter().zip(&islands).enumerate() {
        if on_top(island) {
            let span = span(triangle);
            top.rectangle = [top.rectangle[0].min(span[0]), top.rectangle[1].min(span[1]), top.rectangle[2].max(span[2]), top.rectangle[3].max(span[3])];
            top.triangles.push(index);
        }
    }
    let [left, up, right, down] = top.rectangle;
    if !(left < right && up < down) {
        return None;
    }
    let reached = triangles.iter().zip(&islands).filter(|(_, island)| !on_top(island)).any(|(triangle, _)| {
        let span = span(triangle);
        span[0] <= right && span[2] >= left && span[1] <= down && span[3] >= up
    });
    (!reached).then_some(top)
}

/// The mean of `texel` squared (its three lanes') over the `chosen` triangles of the first set,
/// each read at a lattice of points and counted by the area it spans there.
fn mean_square(uv: &[Uv], triangles: &[[usize; 3]], chosen: &[usize], texel: impl Fn(Uv) -> Option<[f32; 3]>) -> Option<f32> {
    let (mut sum, mut weight) = (0.0_f64, 0.0_f64);
    for &index in chosen {
        let [a, b, c] = triangles[index].map(|vertex| uv[vertex]);
        let area = f64::from(((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs());
        let (mut squares, mut points) = (0.0_f64, 0_u32);
        for i in 0..SAMPLES {
            for j in 0..SAMPLES - i {
                let (u, v) = ((i as f32 + 1. / 3.) / SAMPLES as f32, (j as f32 + 1. / 3.) / SAMPLES as f32);
                let at = [a[0] + (b[0] - a[0]) * u + (c[0] - a[0]) * v, a[1] + (b[1] - a[1]) * u + (c[1] - a[1]) * v];
                if let Some(colour) = texel(at) {
                    squares += colour.iter().map(|lane| f64::from(lane * lane)).sum::<f64>() / 3.;
                    points += 1;
                }
            }
        }
        if points > 0 {
            sum += area * squares / f64::from(points);
            weight += area;
        }
    }
    (weight > 0.).then(|| (sum / weight) as f32)
}

/// A diffuse map's largest level, read texel by texel.
struct Diffuse<'a> {
    level: &'a LinearTextureLevel,
    /// The bytes of a block and where its colour block starts; None for four bytes a texel
    /// (blue, green, red, alpha).
    blocks: Option<(usize, usize)>,
}

impl<'a> Diffuse<'a> {
    fn of(texture: &'a NativeTexture) -> Option<Self> {
        let level = texture.linear_levels.iter().find(|level| level.level == 0 && level.face == 0)?;
        let blocks = match texture.pixel_format {
            PIXEL_FORMAT_A8R8G8B8 => None,
            PIXEL_FORMAT_DXT1 => Some((8, 0)),
            PIXEL_FORMAT_DXT5 | PIXEL_FORMAT_LIN_DXT5 => Some((16, 8)),
            _ => return None,
        };
        Some(Self { level, blocks })
    }

    /// The colour (red, green, blue; 0 to 1) of the texel `uv` lies in, the map repeating.
    fn at(&self, uv: Uv) -> Option<[f32; 3]> {
        let (width, height, pitch) = (self.level.width as usize, self.level.height as usize, self.level.row_pitch as usize);
        if width == 0 || height == 0 || !uv[0].is_finite() || !uv[1].is_finite() {
            return None;
        }
        let texel = |value: f32, size: usize| ((value.rem_euclid(1.) * size as f32) as usize).min(size - 1);
        let (x, y) = (texel(uv[0], width), texel(uv[1], height));
        let Some((block_bytes, colour_offset)) = self.blocks else {
            let texel = self.level.data.get(y * pitch + x * 4..y * pitch + x * 4 + 4)?;
            return Some([texel[2], texel[1], texel[0]].map(|lane| f32::from(lane) / 255.));
        };
        let start = (y / 4) * pitch + (x / 4) * block_bytes + colour_offset;
        block_colour(self.level.data.get(start..start + 8)?, colour_offset != 0, (y % 4) * 4 + x % 4)
    }
}

/// Texel `texel` (0 to 15, row by row) of a colour block: two 5:6:5 colours, then sixteen two-bit
/// choices. A colour block alone (`four` false) orders its colours for a third, black choice.
fn block_colour(block: &[u8], four: bool, texel: usize) -> Option<[f32; 3]> {
    let colours = [u16::from_le_bytes([*block.first()?, *block.get(1)?]), u16::from_le_bytes([*block.get(2)?, *block.get(3)?])];
    let rgb = |colour: u16| [f32::from((colour >> 11) & 31) / 31., f32::from((colour >> 5) & 63) / 63., f32::from(colour & 31) / 31.];
    let (first, second) = (rgb(colours[0]), rgb(colours[1]));
    let mix = |a: f32, b: f32| std::array::from_fn(|lane| first[lane] * a + second[lane] * b);
    let choices = u32::from_le_bytes([*block.get(4)?, *block.get(5)?, *block.get(6)?, *block.get(7)?]);
    Some(match ((choices >> (2 * texel)) & 3, colours[0] > colours[1] || four) {
        (0, _) => first,
        (1, _) => second,
        (2, true) => mix(2. / 3., 1. / 3.),
        (3, true) => mix(1. / 3., 2. / 3.),
        (2, false) => mix(0.5, 0.5),
        _ => [0.; 3],
    })
}

fn first_set(geometry: &NativeMeshGeometry) -> Option<&[Uv]> {
    let attribute = geometry.attributes.iter().find(|attribute| attribute.semantic == VertexSemantic::TextureCoordinate(0))?;
    match &attribute.values {
        NativeVertexAttributeValues::Float2(values) if values.len() == geometry.positions.len() => Some(values),
        _ => None,
    }
}

/// A mesh's triangles as vertex indices.
fn triangles(geometry: &NativeMeshGeometry) -> Vec<[usize; 3]> {
    let mut out = Vec::new();
    for draw in &geometry.draws {
        let start = draw.start_index as usize;
        let Some(indices) = geometry.indices.get(start..start + draw.index_count as usize) else { continue };
        for corners in indices.chunks_exact(3) {
            let vertex = |index: u32| usize::try_from(i64::from(index) + i64::from(draw.base_vertex_index)).ok().filter(|vertex| *vertex < geometry.positions.len());
            if let (Some(a), Some(b), Some(c)) = (vertex(corners[0]), vertex(corners[1]), vertex(corners[2])) {
                out.push([a, b, c]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A board two units long: a top face and a bottom face of two triangles each, a rail of two
    /// more, every face an island of its own in the first set. `flip` winds every triangle the
    /// other way round.
    fn board(flip: bool, rail_u: f32) -> (Vec<[f32; 3]>, Vec<Uv>, Vec<[usize; 3]>) {
        let mut positions = Vec::new();
        let mut uv = Vec::new();
        let mut triangles = Vec::new();
        let mut quad = |corners: [[f32; 3]; 4], uvs: [Uv; 4]| {
            let first = positions.len();
            positions.extend(corners);
            uv.extend(uvs);
            for triangle in [[0, 1, 2], [0, 2, 3]] {
                let [a, b, c] = triangle.map(|corner| first + corner);
                triangles.push(if flip { [a, c, b] } else { [a, b, c] });
            }
        };
        // Top (y 0.1), wound to look up.
        quad([[0., 0.1, 0.], [0., 0.1, 2.], [1., 0.1, 2.], [1., 0.1, 0.]], [[0.1, 0.1], [0.1, 0.4], [0.8, 0.4], [0.8, 0.1]]);
        // Bottom (y 0), wound to look down.
        quad([[0., 0., 0.], [1., 0., 0.], [1., 0., 2.], [0., 0., 2.]], [[0.1, 0.6], [0.8, 0.6], [0.8, 0.9], [0.1, 0.9]]);
        // A rail along x 1, looking sideways.
        quad([[1., 0., 0.], [1., 0.1, 0.], [1., 0.1, 2.], [1., 0., 2.]], [[rail_u, 0.1], [rail_u + 0.05, 0.1], [rail_u + 0.05, 0.9], [rail_u, 0.9]]);
        (positions, uv, triangles)
    }

    #[test]
    fn the_top_face_is_the_flat_side_that_lies_higher_however_the_mesh_is_wound() {
        for flip in [false, true] {
            let (positions, uv, triangles) = board(flip, 0.9);
            let top = top_face(&positions, &uv, &triangles).unwrap();
            assert_eq!(top.triangles, [0, 1], "wound {flip}");
            assert_eq!(top.rectangle, [0.1, 0.1, 0.8, 0.4]);
        }
    }

    #[test]
    fn a_face_that_reaches_into_the_top_faces_rectangle_leaves_no_griptape() {
        let (positions, uv, triangles) = board(false, 0.5);
        assert_eq!(top_face(&positions, &uv, &triangles), None);
        let (positions, uv, triangles) = board(false, 0.9);
        assert_eq!(top_face(&positions, &uv, &triangles[..2]), None, "one flat side alone");
    }

    #[test]
    fn the_level_is_the_mean_of_the_diffuse_squared_over_the_chosen_triangles() {
        let (_, uv, triangles) = board(false, 0.9);
        // A quarter over the top face, one everywhere else.
        let texel = |uv: Uv| Some(if uv[1] < 0.5 { [0.5; 3] } else { [1.; 3] });
        assert!((mean_square(&uv, &triangles, &[0, 1], texel).unwrap() - 0.25).abs() < 1e-6);
        assert!((mean_square(&uv, &triangles, &[2, 3], texel).unwrap() - 1.).abs() < 1e-6);
        assert_eq!(mean_square(&uv, &triangles, &[], texel), None);
        let surface = GripSurface { rectangle: [0.; 4], level: 0.25 };
        assert_eq!(surface.gain([1., 0.5, 0.]), [4., 2., 0.]);
    }

    #[test]
    fn a_colour_block_reads_its_two_colours_and_the_ones_between() {
        // Pure red and pure blue; texel 0 the first, 1 the second, 2 and 3 between them.
        let block = [0x00, 0xf8, 0x1f, 0x00, 0b1110_0100, 0, 0, 0];
        assert_eq!(block_colour(&block, false, 0), Some([1., 0., 0.]));
        assert_eq!(block_colour(&block, false, 1), Some([0., 0., 1.]));
        let between = block_colour(&block, false, 2).unwrap();
        assert!((between[0] - 2. / 3.).abs() < 1e-6 && (between[2] - 1. / 3.).abs() < 1e-6);
        // The colours the other way round, alone: the third choice halves them, the fourth is black.
        let block = [0x1f, 0x00, 0x00, 0xf8, 0b1110_0100, 0, 0, 0];
        assert_eq!(block_colour(&block, false, 2), Some([0.5, 0., 0.5]));
        assert_eq!(block_colour(&block, false, 3), Some([0.; 3]));
        assert!(block_colour(&block, true, 3).unwrap()[0] > 0.6, "a block after an alpha block has four colours");
    }
}
