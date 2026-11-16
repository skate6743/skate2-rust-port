//! The geometry and zone maps of [`super::tattoos`]: the nearest point of a surface, a zone map's
//! texels, and a zone map drawn over a mesh's texture coordinates.
use std::collections::HashMap;

pub type Uv = [f32; 2];

/// A tattoo zone as `cacstamp_skin` reads it from its alpha map: 0 for none, 1 to 4 for the red
/// levels (over 0.2, 0.4, 0.6 and 0.8) and 5 to 8 for the green ones, read where red is zero.
pub type Zone = u8;

/// The zone an alpha map's texel names.
#[must_use]
pub fn zone_of(red: u8, green: u8) -> Zone {
    let level = |value: u8| -> u8 {
        let value = f32::from(value) / 255.;
        if value > 0.8 {
            4
        } else if value > 0.6 {
            3
        } else if value > 0.4 {
            2
        } else if value > 0.2 {
            1
        } else {
            0
        }
    };
    if red != 0 {
        level(red)
    } else {
        match level(green) {
            0 => 0,
            level => level + 4,
        }
    }
}

/// A zone's texel (blue, green, red, alpha), mid-way through its level.
#[must_use]
pub fn zone_texel(zone: Zone) -> [u8; 4] {
    const LEVELS: [u8; 4] = [77, 128, 179, 230];
    match zone {
        1..=4 => [0, 0, LEVELS[usize::from(zone - 1)], 255],
        5..=8 => [0, LEVELS[usize::from(zone - 5)], 0, 255],
        _ => [0, 0, 0, 255],
    }
}

/// An alpha map's zones, row by row from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneMap {
    pub width: usize,
    pub height: usize,
    pub zones: Vec<Zone>,
}

impl ZoneMap {
    /// The zone of the texel `uv` lies in.
    #[must_use]
    pub fn at(&self, uv: Uv) -> Zone {
        if self.width == 0 || self.height == 0 || !uv[0].is_finite() || !uv[1].is_finite() {
            return 0;
        }
        let texel = |value: f32, size: usize| ((value * size as f32).floor().max(0.) as usize).min(size - 1);
        self.zones[texel(uv[1], self.height) * self.width + texel(uv[0], self.width)]
    }

    /// An uncompressed map: four bytes a texel (blue, green, red, alpha), `row_pitch` bytes a row.
    #[must_use]
    pub fn from_bgra(width: usize, height: usize, row_pitch: usize, data: &[u8]) -> Option<Self> {
        let mut zones = Vec::with_capacity(width * height);
        for y in 0..height {
            let row = data.get(y * row_pitch..y * row_pitch + width * 4)?;
            zones.extend(row.chunks_exact(4).map(|texel| zone_of(texel[2], texel[1])));
        }
        Some(Self { width, height, zones })
    }

    /// A block-compressed map: `block_bytes` a block, its colour block (two 5:6:5 colours and
    /// sixteen two-bit choices) `colour_offset` bytes in.
    #[must_use]
    pub fn from_blocks(width: usize, height: usize, row_pitch: usize, block_bytes: usize, colour_offset: usize, data: &[u8]) -> Option<Self> {
        let mut zones = vec![0; width * height];
        for block_y in 0..height.div_ceil(4) {
            for block_x in 0..width.div_ceil(4) {
                let start = block_y * row_pitch + block_x * block_bytes + colour_offset;
                let block = data.get(start..start + 8)?;
                let colours = [u16::from_le_bytes([block[0], block[1]]), u16::from_le_bytes([block[2], block[3]])];
                let red_green = |colour: u16| [f32::from((colour >> 11) & 31) * 255. / 31., f32::from((colour >> 5) & 63) * 255. / 63.];
                let (first, second) = (red_green(colours[0]), red_green(colours[1]));
                let mix = |a: f32, b: f32| [first[0] * a + second[0] * b, first[1] * a + second[1] * b];
                // A colour block alone orders its colours for a third, transparent choice.
                let palette = if colours[0] > colours[1] || colour_offset != 0 {
                    [first, second, mix(2. / 3., 1. / 3.), mix(1. / 3., 2. / 3.)]
                } else {
                    [first, second, mix(0.5, 0.5), [0., 0.]]
                };
                let choices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
                for texel in 0..16 {
                    let (x, y) = (block_x * 4 + texel % 4, block_y * 4 + texel / 4);
                    if x < width && y < height {
                        let [red, green] = palette[((choices >> (2 * texel)) & 3) as usize];
                        zones[y * width + x] = zone_of(red.round() as u8, green.round() as u8);
                    }
                }
            }
        }
        Some(Self { width, height, zones })
    }
}

/// A triangle of the surface tattoos are taken from: its corners, and at each the alpha map's
/// coordinate and the tattoo sheet's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceTriangle {
    pub positions: [[f32; 3]; 3],
    pub zone_uv: [Uv; 3],
    pub sheet_uv: [Uv; 3],
    /// Which of the surface's zone maps `zone_uv` reads.
    pub zone_map: usize,
}

/// The nearest point of a [`Surface`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nearest {
    pub triangle: usize,
    /// The point's weights of the triangle's corners.
    pub weights: [f32; 3],
    pub distance: f32,
}

/// Triangles, found by the cell a point lies in: each is listed in every cell within `reach` of
/// it, so a point's cell lists every triangle within `reach` of the point.
#[derive(Clone, Debug, Default)]
pub struct Surface {
    triangles: Vec<SurfaceTriangle>,
    reach: f32,
    cells: HashMap<[i32; 3], Vec<u32>>,
}

impl Surface {
    #[must_use]
    pub fn new(triangles: Vec<SurfaceTriangle>, reach: f32) -> Self {
        let mut cells: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
        for (index, triangle) in triangles.iter().enumerate() {
            let corner = |pick: fn(f32, f32) -> f32, grow: f32| -> [i32; 3] {
                std::array::from_fn(|axis| {
                    let bound = triangle.positions.iter().map(|p| p[axis]).fold(triangle.positions[0][axis], pick);
                    ((bound + grow) / reach).floor() as i32
                })
            };
            let (low, high) = (corner(f32::min, -reach), corner(f32::max, reach));
            for x in low[0]..=high[0] {
                for y in low[1]..=high[1] {
                    for z in low[2]..=high[2] {
                        cells.entry([x, y, z]).or_default().push(index as u32);
                    }
                }
            }
        }
        Self { triangles, reach, cells }
    }

    #[must_use]
    pub fn triangles(&self) -> &[SurfaceTriangle] {
        &self.triangles
    }

    /// The surface's nearest point to `point`, when one lies within the reach.
    #[must_use]
    pub fn nearest(&self, point: [f32; 3]) -> Option<Nearest> {
        let cell: [i32; 3] = std::array::from_fn(|axis| (point[axis] / self.reach).floor() as i32);
        let mut best: Option<Nearest> = None;
        for &index in self.cells.get(&cell)? {
            let triangle = &self.triangles[index as usize];
            let weights = nearest_weights(point, triangle.positions);
            let nearest: [f32; 3] = std::array::from_fn(|axis| (0..3).map(|corner| weights[corner] * triangle.positions[corner][axis]).sum());
            let distance = length(sub(point, nearest));
            if distance <= self.reach && best.is_none_or(|best| distance < best.distance) {
                best = Some(Nearest { triangle: index as usize, weights, distance });
            }
        }
        best
    }
}

/// A coordinate at a [`Nearest`] point: the corners' weighted by its weights.
#[must_use]
pub fn weighted(corners: [Uv; 3], weights: [f32; 3]) -> Uv {
    std::array::from_fn(|axis| (0..3).map(|corner| corners[corner][axis] * weights[corner]).sum())
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

/// The weights of a triangle's corners at its nearest point to `point` (Ericson, Real-Time
/// Collision Detection 5.1.5).
#[must_use]
pub fn nearest_weights(point: [f32; 3], [a, b, c]: [[f32; 3]; 3]) -> [f32; 3] {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(point, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return [1., 0., 0.];
    }
    let bp = sub(point, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return [0., 1., 0.];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return [1. - v, v, 0.];
    }
    let cp = sub(point, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return [0., 0., 1.];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let w = d2 / (d2 - d6);
        return [1. - w, 0., w];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return [0., 1. - w, w];
    }
    let total = va + vb + vc;
    if total == 0. || !total.is_finite() {
        return [1., 0., 0.];
    }
    let (v, w) = (vb / total, vc / total);
    [1. - v - w, v, w]
}

/// An affine map of the plane: `[[a, b, c], [d, e, f]]` takes `(x, y)` to
/// `(a x + b y + c, d x + e y + f)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine(pub [[f64; 3]; 2]);

impl Affine {
    #[must_use]
    pub fn apply(&self, uv: Uv) -> Uv {
        let (x, y) = (f64::from(uv[0]), f64::from(uv[1]));
        self.0.map(|row| (row[0] * x + row[1] * y + row[2]) as f32)
    }

    /// The map taking each pair's first coordinate nearest its second (least squares), or none
    /// when the first coordinates lie on a line.
    #[must_use]
    pub fn fit(pairs: &[(Uv, Uv)]) -> Option<Self> {
        let mut normal = [[0_f64; 3]; 3];
        let mut right = [[0_f64; 3]; 2];
        for (from, to) in pairs {
            let basis = [f64::from(from[0]), f64::from(from[1]), 1.];
            for row in 0..3 {
                for column in 0..3 {
                    normal[row][column] += basis[row] * basis[column];
                }
                for (axis, sum) in right.iter_mut().enumerate() {
                    sum[row] += basis[row] * f64::from(to[axis]);
                }
            }
        }
        let minor = |r0: usize, r1: usize, c0: usize, c1: usize| normal[r0][c0] * normal[r1][c1] - normal[r0][c1] * normal[r1][c0];
        let cofactors = [
            [minor(1, 2, 1, 2), -minor(1, 2, 0, 2), minor(1, 2, 0, 1)],
            [-minor(0, 2, 1, 2), minor(0, 2, 0, 2), -minor(0, 2, 0, 1)],
            [minor(0, 1, 1, 2), -minor(0, 1, 0, 2), minor(0, 1, 0, 1)],
        ];
        let determinant: f64 = (0..3).map(|column| normal[0][column] * cofactors[0][column]).sum();
        let scale = normal[0][0].max(normal[1][1]).max(normal[2][2]);
        if !determinant.is_finite() || determinant.abs() <= scale * scale * scale * 1e-12 {
            return None;
        }
        // The normal matrix is symmetric: its inverse is its cofactors over the determinant.
        Some(Self(right.map(|sum| std::array::from_fn(|row| (0..3).map(|column| cofactors[row][column] * sum[column]).sum::<f64>() / determinant))))
    }

    /// [`Affine::fit`] over the pairs that agree: six rounds, each keeping the pairs within three
    /// times the median miss (and no less than `floor`) of the last fit. Returns the map, the
    /// pairs kept and their median miss.
    #[must_use]
    pub fn fit_agreeing(pairs: &[(Uv, Uv)], floor: f32) -> Option<(Self, usize, f32)> {
        let mut kept: Vec<(Uv, Uv)> = pairs.to_vec();
        let mut found = None;
        for _ in 0..6 {
            let map = Self::fit(&kept)?;
            let miss = |pair: &(Uv, Uv)| {
                let at = map.apply(pair.0);
                ((at[0] - pair.1[0]).powi(2) + (at[1] - pair.1[1]).powi(2)).sqrt()
            };
            let mut misses: Vec<f32> = kept.iter().map(miss).collect();
            misses.sort_by(f32::total_cmp);
            let median = *misses.get(misses.len() / 2)?;
            found = Some((map, kept.len(), median));
            let limit = (median * 3.).max(floor);
            let next: Vec<(Uv, Uv)> = pairs.iter().copied().filter(|pair| miss(pair) <= limit).collect();
            if next.len() == kept.len() {
                break;
            }
            kept = next;
        }
        found
    }
}

/// A zone map drawn over a mesh's texture coordinates: each triangle's zone over the texels it
/// covers and those within `MARGIN` texels of it, then every other texel its nearest drawn one's.
#[derive(Clone, Debug)]
pub struct ZonePainter {
    size: usize,
    zones: Vec<Zone>,
    /// Texels two triangles gave different zones.
    pub conflicts: usize,
}

/// How far out of a triangle its zone is drawn, in texels.
const MARGIN: f32 = 0.75;

impl ZonePainter {
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self { size, zones: vec![0; size * size], conflicts: 0 }
    }

    pub fn paint(&mut self, triangle: [Uv; 3], zone: Zone) {
        if zone == 0 || triangle.iter().flatten().any(|value| !value.is_finite()) {
            return;
        }
        let size = self.size as f32;
        let corners = triangle.map(|uv| [uv[0] * size, uv[1] * size]);
        let bound = |axis: usize, pick: fn(f32, f32) -> f32| corners.iter().map(|corner| corner[axis]).fold(corners[0][axis], pick);
        let texel = |value: f32| (value.floor().max(0.) as usize).min(self.size - 1);
        let (left, right) = (texel(bound(0, f32::min) - 1.), texel(bound(0, f32::max) + 1.));
        let (top, bottom) = (texel(bound(1, f32::min) - 1.), texel(bound(1, f32::max) + 1.));
        for y in top..=bottom {
            for x in left..=right {
                if distance_2d([x as f32 + 0.5, y as f32 + 0.5], corners) <= MARGIN {
                    let drawn = &mut self.zones[y * self.size + x];
                    if *drawn != 0 && *drawn != zone {
                        self.conflicts += 1;
                    }
                    *drawn = zone;
                }
            }
        }
    }

    /// The texels drawn so far.
    #[must_use]
    pub fn drawn(&self) -> usize {
        self.zones.iter().filter(|zone| **zone != 0).count()
    }

    /// The finished map's texels (blue, green, red, alpha), or none with nothing drawn.
    #[must_use]
    pub fn finish(mut self) -> Option<Vec<u8>> {
        let mut front: Vec<usize> = (0..self.zones.len()).filter(|texel| self.zones[*texel] != 0).collect();
        if front.is_empty() {
            return None;
        }
        while !front.is_empty() {
            let mut next = Vec::new();
            for texel in front {
                let (x, y) = (texel % self.size, texel / self.size);
                let neighbours = [
                    (x > 0).then(|| texel - 1),
                    (x + 1 < self.size).then(|| texel + 1),
                    (y > 0).then(|| texel - self.size),
                    (y + 1 < self.size).then(|| texel + self.size),
                ];
                for neighbour in neighbours.into_iter().flatten() {
                    if self.zones[neighbour] == 0 {
                        self.zones[neighbour] = self.zones[texel];
                        next.push(neighbour);
                    }
                }
            }
            front = next;
        }
        Some(self.zones.iter().flat_map(|zone| zone_texel(*zone)).collect())
    }
}

/// How far `point` is from a triangle in the plane: 0 inside it.
fn distance_2d(point: Uv, [a, b, c]: [Uv; 3]) -> f32 {
    let side = |from: Uv, to: Uv| (to[0] - from[0]) * (point[1] - from[1]) - (to[1] - from[1]) * (point[0] - from[0]);
    let sides = [side(a, b), side(b, c), side(c, a)];
    if sides.iter().all(|side| *side >= 0.) || sides.iter().all(|side| *side <= 0.) {
        return 0.;
    }
    let edge = |from: Uv, to: Uv| {
        let (along, to_point) = ([to[0] - from[0], to[1] - from[1]], [point[0] - from[0], point[1] - from[1]]);
        let squared = along[0] * along[0] + along[1] * along[1];
        let t = if squared > 0. { ((along[0] * to_point[0] + along[1] * to_point[1]) / squared).clamp(0., 1.) } else { 0. };
        let offset = [to_point[0] - along[0] * t, to_point[1] - along[1] * t];
        (offset[0] * offset[0] + offset[1] * offset[1]).sqrt()
    };
    edge(a, b).min(edge(b, c)).min(edge(c, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_alpha_texel_names_its_zone_as_the_program_reads_it() {
        assert_eq!(zone_of(0, 0), 0);
        assert_eq!(zone_of(51, 255), 0, "red at 0.2 is under the first level, and green is not read");
        assert_eq!(zone_of(52, 0), 1);
        assert_eq!(zone_of(103, 0), 2);
        assert_eq!(zone_of(154, 0), 3);
        assert_eq!(zone_of(205, 0), 4);
        assert_eq!(zone_of(0, 77), 5);
        assert_eq!(zone_of(0, 230), 8);
        for zone in 0..=8 {
            let [_, green, red, _] = zone_texel(zone);
            assert_eq!(zone_of(red, green), zone);
        }
    }

    #[test]
    fn the_nearest_point_of_a_triangle_is_inside_on_an_edge_or_at_a_corner() {
        let triangle = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        assert_eq!(nearest_weights([0.25, 0.25, 3.], triangle), [0.5, 0.25, 0.25]);
        assert_eq!(nearest_weights([-1., -1., 0.], triangle), [1., 0., 0.]);
        assert_eq!(nearest_weights([2., -1., 0.], triangle), [0., 1., 0.]);
        assert_eq!(nearest_weights([0.5, -1., 0.], triangle), [0.5, 0.5, 0.]);
        assert_eq!(nearest_weights([1., 1., 0.], triangle), [0., 0.5, 0.5]);
    }

    #[test]
    fn a_surface_gives_its_nearest_triangle_within_reach() {
        let triangle = |z: f32, sheet: f32| SurfaceTriangle {
            positions: [[0., 0., z], [0.1, 0., z], [0., 0.1, z]],
            zone_uv: [[0.1, 0.1]; 3],
            sheet_uv: [[sheet, 0.], [sheet + 1., 0.], [sheet, 1.]],
            zone_map: 0,
        };
        let surface = Surface::new(vec![triangle(0., 0.), triangle(0.01, 10.)], 0.02);
        let near = surface.nearest([0.025, 0.025, 0.008]).unwrap();
        assert_eq!(near.triangle, 1);
        assert!((near.distance - 0.002).abs() < 1e-6);
        let sheet = weighted(surface.triangles()[1].sheet_uv, near.weights);
        assert!((sheet[0] - 10.25).abs() < 1e-5 && (sheet[1] - 0.25).abs() < 1e-5);
        assert_eq!(surface.nearest([0.025, 0.025, -0.019]).unwrap().triangle, 0);
        assert!(surface.nearest([0.025, 0.025, 0.031]).is_none(), "past the reach");
        assert!(surface.nearest([5., 5., 5.]).is_none());
    }

    #[test]
    fn an_affine_map_is_fitted_through_the_pairs_that_agree() {
        let map = Affine([[-4., 0.25, 1.75], [0.5, 3., -2.]]);
        let grid: Vec<Uv> = (0..5).flat_map(|x| (0..5).map(move |y| [0.1 + 0.05 * x as f32, 0.7 + 0.06 * y as f32])).collect();
        let mut pairs: Vec<(Uv, Uv)> = grid.iter().map(|from| (*from, map.apply(*from))).collect();
        let fitted = Affine::fit(&pairs).unwrap();
        for (row, expected) in fitted.0.iter().zip(map.0) {
            for (value, expected) in row.iter().zip(expected) {
                assert!((value - expected).abs() < 1e-3, "{fitted:?}");
            }
        }
        // Three pairs far off do not pull the fit.
        for pair in pairs.iter_mut().take(3) {
            pair.1 = [pair.1[0] + 0.6, pair.1[1] - 0.4];
        }
        let (agreeing, kept, median) = Affine::fit_agreeing(&pairs, 0.01).unwrap();
        assert_eq!(kept, 22);
        assert!(median < 1e-3);
        let at = agreeing.apply([0.2, 0.8]);
        let expected = map.apply([0.2, 0.8]);
        assert!((at[0] - expected[0]).abs() < 1e-3 && (at[1] - expected[1]).abs() < 1e-3);
        assert!(Affine::fit(&[([0., 0.], [0., 0.]), ([1., 1.], [1., 1.]), ([2., 2.], [2., 2.])]).is_none(), "points on a line");
    }

    #[test]
    fn zone_maps_read_plain_and_block_texels() {
        // Two texels: red 0.3 (zone 1), then green 0.5 with no red (zone 6).
        let plain = ZoneMap::from_bgra(2, 1, 8, &[0, 0, 77, 255, 0, 128, 0, 255]).unwrap();
        assert_eq!(plain.zones, [1, 6]);
        assert_eq!(plain.at([0.2, 0.5]), 1);
        assert_eq!(plain.at([0.9, 0.5]), 6);
        assert_eq!(plain.at([7., -3.]), 6, "coordinates clamp to the map");
        // A block of red 16/31 (0.52: zone 2) with its last texel the other colour, red 31/31.
        let first = 16_u16 << 11;
        let second = 31_u16 << 11;
        let mut block = Vec::new();
        block.extend(second.to_le_bytes());
        block.extend(first.to_le_bytes());
        block.extend(0x1555_5555_u32.to_le_bytes());
        let blocks = ZoneMap::from_blocks(4, 4, 8, 8, 0, &block).unwrap();
        assert_eq!(blocks.zones[..15], [2; 15]);
        assert_eq!(blocks.zones[15], 4);
    }

    #[test]
    fn a_painted_zone_map_covers_its_triangles_and_fills_the_rest_from_the_nearest() {
        let mut painter = ZonePainter::new(8);
        painter.paint([[0., 0.], [0.4, 0.], [0., 0.4]], 1);
        painter.paint([[1., 1.], [0.6, 1.], [1., 0.6]], 2);
        painter.paint([[0.5, 0.5], [0.6, 0.5], [0.5, 0.6]], 0);
        assert_eq!(painter.conflicts, 0);
        assert!(painter.drawn() > 0);
        let texels = painter.finish().unwrap();
        assert_eq!(texels.len(), 8 * 8 * 4);
        let zone = |x: usize, y: usize| zone_of(texels[(y * 8 + x) * 4 + 2], texels[(y * 8 + x) * 4 + 1]);
        assert_eq!(zone(0, 0), 1);
        assert_eq!(zone(7, 7), 2);
        assert_eq!(zone(7, 0), 1.max(zone(7, 0)), "every texel has a zone");
        assert!((0..8).all(|y| (0..8).all(|x| zone(x, y) != 0)));
        assert!(ZonePainter::new(4).finish().is_none());
    }
}
