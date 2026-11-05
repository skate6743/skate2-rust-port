//! Skate 3's NavPower pedestrian navigation (`RWOBJECTTYPE_NAVPOWERDATA`, `0x00eb0027`) as a
//! MeshStore tile. Read as TU3 reads it: its Sim `RegArena` (`0x8285ac70`, kind 0x26) hands each
//! graph image of the record to NavPower; the image loader (`0x8293b620`) takes a big-endian
//! version 2 image whose words sum to its checksum and runs its sections, the NavGraph section
//! (`0x82937450`) a version 23 set of graphs, each `totalBytes` long; a graph (`0x82938158`,
//! `0x82938878`) holds convex areas from its header + 312, each 52 bytes and then its edges, 24
//! bytes each: the adjacent area as an offset from the graph header, then the edge's first point.
//!
//! PC port: Skate 2's pedestrians walk a Pathogen MeshStore, so each area becomes a fan of the
//! triangle cells it holds, laid out as Skate 2's NavMesh2 is: a cell's edge i runs from its point
//! i to i + 1, holds the cell and then the cell across it, and that cell is its neighbour i. The
//! area's cost multiplier (`0x82926640`) becomes the cells' cost: Skate 3's pedestrians keep to
//! multiplier 1 (`0x82c47d60`) as Skate 2's keep to cost 0; 15 is Skate 3's road (its largest
//! areas, in strips) and 10 the ground off the paths, Skate 2's 0.5 and 0.75. NavPower stitches
//! neighbouring tiles' graphs at their border edges (`0x82935460`), which stop about 0.14 m short
//! of their 100 m tile's side; here those edges' points move onto the side and the MeshStore links
//! each to the neighbouring tile's border edge it overlaps most.

use super::{Cell, NavAssetError, point_in_triangle_xz};
use crate::world_painter_layer_asset::{LayerAssetError, word};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};

/// Skate 3's Sim tiles: `cSim_<x>_<z>_high`, 100 m square.
const TILE: f32 = 100.0;
/// How far from its tile's side a border edge's points lie.
const SIDE_TOLERANCE: f32 = 0.5;
/// `(flags1 & 0xe000) == 0x2000`: an edge NavPower stitches to a neighbouring graph.
const EDGE_TYPE: u32 = 0xe000;
const BORDER_EDGE: u32 = 0x2000;
/// The cell lookup's square, in metres.
const GRID: f32 = 4.0;
const NO_CELL: u32 = u32::MAX;

pub(crate) struct NavPowerTile {
    points: Vec<[f32; 3]>,
    cells: Vec<TileCell>,
    edges: Vec<TileEdge>,
    /// Each edge's cell across it, which is also its cell's neighbour: `(slot << 29) | cell`.
    across: Vec<AtomicU32>,
    borders: Vec<Border>,
    min: [f32; 3],
    max: [f32; 3],
    tile: Option<[i32; 2]>,
    grid: Grid,
}
struct TileCell {
    points: [u32; 3],
    centroid: [f32; 3],
    cost: f32,
}
struct TileEdge {
    direction: [f32; 3],
    length: f32,
}
struct Border {
    edge: u32,
    /// The MeshStore's neighbour slot of the side: 1 -x, 2 +x, 3 -z, 4 +z.
    slot: usize,
    line: f32,
    span: [f32; 2],
}
struct Grid {
    origin: [f32; 2],
    columns: usize,
    rows: usize,
    starts: Vec<u32>,
    cells: Vec<u32>,
}

impl NavPowerTile {
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, NavAssetError> {
        let mut builder = Builder::default();
        let count = word(bytes, 4)? as usize;
        let images = word(bytes, 8)? as usize;
        for index in 0..count {
            builder.image(bytes, word(bytes, images + index * 4)? as usize)?;
        }
        Ok(builder.finish())
    }
    pub(crate) fn cell(&self, id: u32) -> Result<Cell, NavAssetError> {
        let cell = self.tile_cell(id)?;
        Ok(Cell { id, centroid: cell.centroid, fixed_cost: cell.cost })
    }
    pub(crate) fn cell_edges(&self, id: u32) -> Result<[u32; 3], NavAssetError> {
        self.tile_cell(id)?;
        Ok([id * 3, id * 3 + 1, id * 3 + 2])
    }
    pub(crate) fn cell_neighbours(&self, id: u32) -> Result<[u32; 3], NavAssetError> {
        self.tile_cell(id)?;
        Ok(std::array::from_fn(|side| self.across[id as usize * 3 + side].load(Ordering::Relaxed)))
    }
    pub(crate) fn cell_address(&self, id: u32) -> Result<usize, NavAssetError> {
        Ok(std::ptr::from_ref(self.tile_cell(id)?) as usize)
    }
    pub(crate) fn cell_points(&self, id: u32) -> Result<[[f32; 4]; 3], NavAssetError> {
        let cell = self.tile_cell(id)?;
        Ok(cell.points.map(|point| self.point(point)))
    }
    pub(crate) fn edge_cells(&self, id: u32) -> Result<[u32; 2], NavAssetError> {
        self.tile_edge(id)?;
        Ok([id / 3, self.across[id as usize].load(Ordering::Relaxed)])
    }
    pub(crate) fn edge_points(&self, id: u32) -> Result<[[f32; 4]; 2], NavAssetError> {
        self.tile_edge(id)?;
        let cell = &self.cells[id as usize / 3];
        let side = id as usize % 3;
        Ok([self.point(cell.points[side]), self.point(cell.points[(side + 1) % 3])])
    }
    pub(crate) fn edge_address(&self, id: u32) -> Result<usize, NavAssetError> {
        Ok(std::ptr::from_ref(self.tile_edge(id)?) as usize)
    }
    pub(crate) fn edge_line(&self, id: u32) -> Result<([f32; 4], f32), NavAssetError> {
        let edge = self.tile_edge(id)?;
        let [x, y, z] = edge.direction;
        Ok(([x, y, z, 0.], edge.length))
    }
    pub(crate) fn contains_xz(&self, p: [f32; 3]) -> bool {
        p[0] >= self.min[0] && self.max[0] >= p[0] && p[2] >= self.min[2] && self.max[2] >= p[2]
    }
    /// Skate 2's `GetMeshCell` choice over the cells of the point's square: of those whose
    /// triangle holds the point, the one whose centroid is nearest it in height.
    pub(crate) fn get_cell(&self, p: [f32; 3], y_tolerance: f32) -> Option<Cell> {
        if !self.contains_xz(p) {
            return None;
        }
        let mut best: Option<Cell> = None;
        let mut distance = f32::MAX;
        for &id in self.grid.candidates(p) {
            let cell = &self.cells[id as usize];
            let points = cell.points.map(|point| self.points[point as usize]);
            if !point_in_triangle_xz(p, points) {
                continue;
            }
            let delta = (cell.centroid[1] - p[1]).abs();
            if best.is_none() || delta < distance {
                best = Some(Cell { id, centroid: cell.centroid, fixed_cost: cell.cost });
                distance = delta;
            }
        }
        if distance < y_tolerance { best } else { None }
    }
    /// The MeshStore's neighbour slot of the other tile, when it is beside this one.
    pub(crate) fn slot_of(&self, other: &Self) -> Option<usize> {
        let ([x, z], [other_x, other_z]) = (self.tile?, other.tile?);
        Some(match (other_x - x, other_z - z) {
            (-1, 0) => 1,
            (1, 0) => 2,
            (0, -1) => 3,
            (0, 1) => 4,
            _ => return None,
        })
    }
    /// Links the two tiles' border edges on their shared side, each way.
    pub(crate) fn stitch(&self, slot: usize, other: &Self) {
        let back = if slot & 1 != 0 { slot + 1 } else { slot - 1 };
        self.link(slot, other, back);
        other.link(back, self, slot);
    }
    fn link(&self, slot: usize, other: &Self, back: usize) {
        for border in self.borders.iter().filter(|border| border.slot == slot) {
            let mut best = None;
            let mut most = 0.0;
            for candidate in other.borders.iter().filter(|candidate| candidate.slot == back && candidate.line == border.line) {
                let overlap = border.span[1].min(candidate.span[1]) - border.span[0].max(candidate.span[0]);
                if overlap > most {
                    most = overlap;
                    best = Some(candidate.edge / 3);
                }
            }
            if let Some(cell) = best {
                #[expect(clippy::cast_possible_truncation, reason = "slots 1 to 4")]
                self.across[border.edge as usize].store((slot as u32) << 29 | cell, Ordering::Relaxed);
            }
        }
    }
    fn tile_cell(&self, id: u32) -> Result<&TileCell, NavAssetError> {
        #[expect(clippy::cast_possible_truncation, reason = "cells are counted below 2^29")]
        let count = self.cells.len() as u32;
        self.cells.get(id as usize).ok_or(NavAssetError::Index { index: id, count })
    }
    fn tile_edge(&self, id: u32) -> Result<&TileEdge, NavAssetError> {
        #[expect(clippy::cast_possible_truncation, reason = "edges are counted below 2^31")]
        let count = self.edges.len() as u32;
        self.edges.get(id as usize).ok_or(NavAssetError::Index { index: id, count })
    }
    fn point(&self, id: u32) -> [f32; 4] {
        let [x, y, z] = self.points[id as usize];
        [x, y, z, 0.]
    }
}

impl Grid {
    fn candidates(&self, p: [f32; 3]) -> &[u32] {
        let column = square(p[0] - self.origin[0], self.columns);
        let row = square(p[2] - self.origin[1], self.rows);
        let index = row * self.columns + column;
        match (self.starts.get(index), self.starts.get(index + 1)) {
            (Some(&start), Some(&end)) => &self.cells[start as usize..end as usize],
            _ => &[],
        }
    }
}
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to the grid")]
fn square(offset: f32, count: usize) -> usize {
    ((offset / GRID).floor().max(0.) as usize).min(count.saturating_sub(1))
}

#[derive(Default)]
struct Builder {
    points: Vec<[f32; 3]>,
    point_index: HashMap<[u32; 3], u32>,
    cells: Vec<([u32; 3], f32)>,
    across: Vec<u32>,
    border_edges: Vec<(u32, [u32; 2])>,
    bounds: Option<([f32; 3], [f32; 3])>,
}
impl Builder {
    /// `0x8293b620`: an image TU3 would not load (another version, a byte-swapped or loaded
    /// image, a wrong checksum) adds nothing.
    fn image(&mut self, bytes: &[u8], at: usize) -> Result<(), NavAssetError> {
        if word(bytes, at)? == 0 || word(bytes, at + 4)? != 2 || word(bytes, at + 16)? & 1 != 0 {
            return Ok(());
        }
        let size = word(bytes, at + 8)? as usize;
        let body = at + 24;
        let mut sum = 0u32;
        for offset in (0..size / 4 * 4).step_by(4) {
            sum = sum.wrapping_add(word(bytes, body + offset)?);
        }
        if sum != word(bytes, at + 12)? {
            return Ok(());
        }
        let mut section = body;
        while section + 12 <= body + size {
            let (id, length) = (word(bytes, section)?, word(bytes, section + 4)? as usize);
            if id == 0x0001_0000 && length != 0 && word(bytes, section + 8)? == 0 {
                self.graph_set(bytes, section + 12)?;
            }
            section += 12 + length;
        }
        Ok(())
    }
    /// `0x82937450`: a set of version 23, its graphs one after another. Skate 3's districts hold
    /// layer 0 only, the one taken here.
    fn graph_set(&mut self, bytes: &[u8], at: usize) -> Result<(), NavAssetError> {
        if word(bytes, at)? >> 24 == 0 || word(bytes, at + 4)? != 23 {
            return Ok(());
        }
        let mut graph = at + 12;
        for _ in 0..word(bytes, at + 8)? {
            if word(bytes, graph + 4)? == 0 {
                self.graph(bytes, graph)?;
            }
            let total = word(bytes, graph + 12)? as usize;
            if total == 0 {
                break;
            }
            graph += total;
        }
        Ok(())
    }
    /// `0x82938158`: the graph's bounds at +32, its areas from +312 for `areaBytes` (+8).
    fn graph(&mut self, bytes: &[u8], graph: usize) -> Result<(), NavAssetError> {
        let float = |at: usize| word(bytes, at).map(f32::from_bits);
        let min = [float(graph + 32)?, float(graph + 36)?, float(graph + 40)?];
        let max = [float(graph + 44)?, float(graph + 48)?, float(graph + 52)?];
        self.bounds = Some(match self.bounds {
            Some((low, high)) => (std::array::from_fn(|i| low[i].min(min[i])), std::array::from_fn(|i| high[i].max(max[i]))),
            None => (min, max),
        });
        let start = graph + 312;
        let end = start + word(bytes, graph + 8)? as usize;
        if end > bytes.len() {
            return Err(LayerAssetError::Truncated { required: end, actual: bytes.len() }.into());
        }
        // Each area's offset, first cell and points.
        let mut areas: Vec<(u32, u32, Vec<u32>)> = Vec::new();
        let mut at = start;
        while at < end {
            let count = (word(bytes, at + 40)? & 0x7f) as usize;
            let mut points = Vec::with_capacity(count);
            for edge in 0..count {
                let edge = at + 52 + edge * 24;
                points.push(self.point([word(bytes, edge + 4)?, word(bytes, edge + 8)?, word(bytes, edge + 12)?]));
            }
            #[expect(clippy::cast_possible_truncation, reason = "a graph is far smaller than 4 GB")]
            let offset = (at - graph) as u32;
            #[expect(clippy::cast_possible_truncation, reason = "cells are counted below 2^29")]
            let first = self.cells.len() as u32;
            #[expect(clippy::cast_possible_truncation, reason = "an area has at most 127 edges")]
            let fans = count.saturating_sub(2) as u32;
            if count >= 3 {
                let cost = cell_cost(cost_multiplier(word(bytes, at + 44)?));
                for fan in 1..count - 1 {
                    self.cells.push(([points[0], points[fan], points[fan + 1]], cost));
                    self.across.extend([NO_CELL; 3]);
                }
                // The fan's diagonals: a cell's last edge is the next cell's first.
                for cell in first..first + fans - 1 {
                    self.across[cell as usize * 3 + 2] = cell + 1;
                    self.across[(cell as usize + 1) * 3] = cell;
                }
            }
            areas.push((offset, first, points));
            at += 52 + count * 24;
        }
        let by_offset: HashMap<u32, usize> = areas.iter().enumerate().map(|(index, area)| (area.0, index)).collect();
        for (offset, first, points) in &areas {
            let count = points.len();
            if count < 3 {
                continue;
            }
            let area = graph + *offset as usize;
            for edge in 0..count {
                let record = area + 52 + edge * 24;
                let own = polygon_edge(*first, count, edge);
                let adjacent = word(bytes, record)?;
                if adjacent == 0 {
                    if word(bytes, record + 16)? & EDGE_TYPE == BORDER_EDGE {
                        self.border_edges.push((own, [points[edge], points[(edge + 1) % count]]));
                    }
                    continue;
                }
                let Some(&other) = by_offset.get(&adjacent) else { continue };
                let (_, other_first, other_points) = &areas[other];
                let other_count = other_points.len();
                if other_count < 3 {
                    continue;
                }
                // NavPower's neighbouring areas share each edge, reversed.
                let back = (0..other_count).find(|&k| {
                    word(bytes, graph + adjacent as usize + 52 + k * 24).is_ok_and(|value| value == *offset)
                        && other_points[k] == points[(edge + 1) % count]
                        && other_points[(k + 1) % other_count] == points[edge]
                });
                if let Some(k) = back {
                    self.across[own as usize] = polygon_edge(*other_first, other_count, k) / 3;
                }
            }
        }
        Ok(())
    }
    fn point(&mut self, bits: [u32; 3]) -> u32 {
        let points = &mut self.points;
        *self.point_index.entry(bits).or_insert_with(|| {
            points.push(bits.map(f32::from_bits));
            #[expect(clippy::cast_possible_truncation, reason = "points are counted below 2^32")]
            let id = points.len() as u32 - 1;
            id
        })
    }
    fn finish(mut self) -> NavPowerTile {
        #[expect(clippy::cast_possible_truncation, reason = "tile coordinates")]
        let tile = self.bounds.map(|(min, max)| [0, 2].map(|axis| ((min[axis] + max[axis]) * 0.5 / TILE).floor() as i32));
        let mut borders = Vec::new();
        if let Some(tile) = tile {
            for &(edge, [a, b]) in &self.border_edges {
                let mut side = None;
                let mut nearest = SIDE_TOLERANCE;
                for (axis_index, axis) in [0, 2].into_iter().enumerate() {
                    for high in [false, true] {
                        #[expect(clippy::cast_precision_loss, reason = "tile coordinates")]
                        let line = (tile[axis_index] + i32::from(high)) as f32 * TILE;
                        let far = (self.points[a as usize][axis] - line).abs().max((self.points[b as usize][axis] - line).abs());
                        if far <= nearest {
                            nearest = far;
                            side = Some((axis_index, axis, high, line));
                        }
                    }
                }
                let Some((axis_index, axis, high, line)) = side else { continue };
                self.points[a as usize][axis] = line;
                self.points[b as usize][axis] = line;
                let along = 2 - axis;
                let (from, to) = (self.points[a as usize][along], self.points[b as usize][along]);
                borders.push(Border { edge, slot: 1 + axis_index * 2 + usize::from(high), line, span: [from.min(to), from.max(to)] });
            }
        }
        let points = self.points;
        let cells: Vec<TileCell> = self.cells.iter().map(|&(ids, cost)| {
            let [a, b, c] = ids.map(|id| points[id as usize]);
            TileCell { points: ids, centroid: std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.), cost }
        }).collect();
        let edges = cells.iter().flat_map(|cell| (0..3).map(|side| {
            let (p, q) = (points[cell.points[side] as usize], points[cell.points[(side + 1) % 3] as usize]);
            let delta: [f32; 3] = std::array::from_fn(|i| q[i] - p[i]);
            let length = delta[2].mul_add(delta[2], delta[0].mul_add(delta[0], delta[1] * delta[1])).sqrt();
            let direction = if length > 0. { delta.map(|d| d / length) } else { [0.; 3] };
            TileEdge { direction, length }
        })).collect();
        let (mut min, mut max) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for point in &points {
            for i in 0..3 {
                min[i] = min[i].min(point[i]);
                max[i] = max[i].max(point[i]);
            }
        }
        let grid = Grid::build(&points, &cells, min, max);
        NavPowerTile { points, cells, edges, across: self.across.into_iter().map(AtomicU32::new).collect(), borders, min, max, tile, grid }
    }
}

impl Grid {
    fn build(points: &[[f32; 3]], cells: &[TileCell], min: [f32; 3], max: [f32; 3]) -> Self {
        let origin = [min[0], min[2]];
        let count = |extent: f32| if extent.is_finite() && extent >= 0. { square(extent, usize::MAX) + 1 } else { 0 };
        let (columns, rows) = (count(max[0] - min[0]), count(max[2] - min[2]));
        let mut buckets = vec![Vec::new(); columns * rows];
        for (id, cell) in cells.iter().enumerate() {
            let corners = cell.points.map(|point| points[point as usize]);
            let low = |axis: usize| corners.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
            let high = |axis: usize| corners.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
            for row in square(low(2) - origin[1], rows)..=square(high(2) - origin[1], rows) {
                for column in square(low(0) - origin[0], columns)..=square(high(0) - origin[0], columns) {
                    #[expect(clippy::cast_possible_truncation, reason = "cells are counted below 2^29")]
                    buckets[row * columns + column].push(id as u32);
                }
            }
        }
        let mut starts = Vec::with_capacity(buckets.len() + 1);
        let mut ids = Vec::new();
        for bucket in buckets {
            #[expect(clippy::cast_possible_truncation, reason = "candidate lists are far below 2^32")]
            starts.push(ids.len() as u32);
            ids.extend(bucket);
        }
        #[expect(clippy::cast_possible_truncation, reason = "candidate lists are far below 2^32")]
        starts.push(ids.len() as u32);
        Self { origin, columns, rows, starts, cells: ids }
    }
}

/// The fan cell and side of an area's edge: its first edge is the first cell's first, its last
/// the last cell's last, the others each cell's second.
#[expect(clippy::cast_possible_truncation, reason = "an area has at most 127 edges")]
fn polygon_edge(first: u32, count: usize, edge: usize) -> u32 {
    let (cell, side) = match edge {
        0 => (0, 0),
        _ if edge == count - 1 => (count as u32 - 3, 2),
        _ => (edge as u32 - 1, 1),
    };
    (first + cell) * 3 + side
}

/// `0x82926640`: the larger of the area's two 4-bit cost multipliers in its second flags word.
const fn cost_multiplier(flags: u32) -> u32 {
    let (a, b) = ((flags >> 12) & 0xf, (flags >> 8) & 0xf);
    if a > b { a } else { b }
}

/// PC port: Skate 2's cell cost of a multiplier (see the module).
const fn cell_cost(multiplier: u32) -> f32 {
    match multiplier {
        0 | 1 => 0.0,
        2..=10 => 0.75,
        _ => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One record of one image of one set holding the given graphs' area bytes.
    fn record(graphs: &[(f32, Vec<u8>)]) -> Vec<u8> {
        let mut set = Vec::new();
        set.extend(0xffff_ffffu32.to_be_bytes());
        set.extend(23u32.to_be_bytes());
        #[expect(clippy::cast_possible_truncation, reason = "test")]
        set.extend((graphs.len() as u32).to_be_bytes());
        for (offset_x, areas) in graphs {
            let mut graph = vec![0u8; 312];
            #[expect(clippy::cast_possible_truncation, reason = "test")]
            let (area_bytes, total) = (areas.len() as u32, 312 + areas.len() as u32);
            graph[8..12].copy_from_slice(&area_bytes.to_be_bytes());
            graph[12..16].copy_from_slice(&total.to_be_bytes());
            for (i, value) in [offset_x + 0.14, 0., 0.14, offset_x + 99.86, 0., 99.86].into_iter().enumerate() {
                graph[32 + i * 4..36 + i * 4].copy_from_slice(&value.to_be_bytes());
            }
            graph.extend(areas);
            set.extend(graph);
        }
        let mut body = Vec::new();
        body.extend(0x0001_0000u32.to_be_bytes());
        #[expect(clippy::cast_possible_truncation, reason = "test")]
        body.extend((set.len() as u32).to_be_bytes());
        body.extend(0u32.to_be_bytes());
        body.extend(set);
        let sum = body.chunks(4).fold(0u32, |sum, w| sum.wrapping_add(u32::from_be_bytes(w.try_into().unwrap())));
        let mut bytes = vec![0u8; 0x40];
        bytes[4..8].copy_from_slice(&1u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&0x30u32.to_be_bytes());
        bytes[0x30..0x34].copy_from_slice(&0x40u32.to_be_bytes());
        bytes.extend(0xffff_ffffu32.to_be_bytes());
        bytes.extend(2u32.to_be_bytes());
        #[expect(clippy::cast_possible_truncation, reason = "test")]
        bytes.extend((body.len() as u32).to_be_bytes());
        bytes.extend(sum.to_be_bytes());
        bytes.extend([0u8; 8]);
        bytes.extend(body);
        bytes
    }
    /// An area of the given points (clockwise seen from above, as NavPower's are), each edge's
    /// adjacent area offset and type flags.
    fn area(points: &[[f32; 3]], edges: &[(u32, u32)], flags2: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; 52];
        #[expect(clippy::cast_possible_truncation, reason = "test")]
        bytes[40..44].copy_from_slice(&(points.len() as u32).to_be_bytes());
        bytes[44..48].copy_from_slice(&flags2.to_be_bytes());
        for (point, &(adjacent, flags)) in points.iter().zip(edges) {
            bytes.extend(adjacent.to_be_bytes());
            for value in point {
                bytes.extend(value.to_be_bytes());
            }
            bytes.extend(flags.to_be_bytes());
            bytes.extend(0u32.to_be_bytes());
        }
        bytes
    }

    #[test]
    fn areas_become_fans_of_skate2_cells() {
        // A square (offset 312) beside a triangle (offset 312 + 52 + 4 * 24) across its edge 1.
        let square = area(&[[0.14, 0., 0.14], [0.14, 0., 10.], [10., 0., 10.], [10., 0., 0.14]],
            &[(0, BORDER_EDGE), (0, 0), (312 + 148, 0), (0, 0)], 0x1100);
        let triangle = area(&[[10., 0., 0.14], [10., 0., 10.], [20., 0., 5.]], &[(312, 0), (0, 0), (0, 0)], 0xf100);
        let tile = NavPowerTile::read(&record(&[(0., [square, triangle].concat())])).unwrap();
        assert_eq!(tile.cells.len(), 3);
        assert_eq!(tile.cell_neighbours(0).unwrap(), [NO_CELL, NO_CELL, 1], "fan cells share their diagonal");
        assert_eq!(tile.cell_neighbours(1).unwrap(), [0, 2, NO_CELL], "the square's third edge is the triangle's first");
        assert_eq!(tile.cell_neighbours(2).unwrap(), [1, NO_CELL, NO_CELL]);
        assert_eq!(tile.edge_cells(4).unwrap(), [1, 2]);
        assert_eq!((tile.cell(0).unwrap().fixed_cost, tile.cell(2).unwrap().fixed_cost), (0.0, 0.5));
        let [p, q] = tile.edge_points(0).unwrap();
        assert_eq!((p[0], q[0]), (0.0, 0.0), "the border edge moves onto the tile's side");
        assert_eq!(tile.borders.len(), 1);
        assert_eq!(tile.borders[0].slot, 1);
        let (direction, length) = tile.edge_line(0).unwrap();
        assert!((direction[2] - 1.0).abs() < 1e-6 && (length - (10.0 - 0.14)).abs() < 1e-5);
        assert_eq!(tile.get_cell([15., 3., 5.], f32::MAX).map(|cell| cell.id), Some(2));
        assert_eq!(tile.get_cell([25., 0., 5.], f32::MAX).map(|cell| cell.id), None);
        assert!(tile.get_cell([15., 3., 5.], 2.).is_none(), "outside the height tolerance");
    }

    #[test]
    fn border_edges_link_across_neighbouring_tiles() {
        let west = area(&[[99.86, 0., 0.14], [90., 0., 0.14], [90., 0., 50.], [99.86, 0., 50.]],
            &[(0, 0), (0, 0), (0, 0), (0, BORDER_EDGE)], 0x1100);
        let east = area(&[[100.14, 0., 20.], [110., 0., 20.], [110., 0., 0.14], [100.14, 0., 0.14]],
            &[(0, 0), (0, 0), (0, 0), (0, BORDER_EDGE)], 0x1100);
        let west = NavPowerTile::read(&record(&[(0., west)])).unwrap();
        let east = NavPowerTile::read(&record(&[(100., east)])).unwrap();
        let slot = west.slot_of(&east).unwrap();
        assert_eq!((slot, east.slot_of(&west)), (2, Some(1)));
        west.stitch(slot, &east);
        assert_eq!(west.cell_neighbours(1).unwrap()[2], 2 << 29 | 1, "west's border edge reaches east's cell");
        assert_eq!(east.cell_neighbours(1).unwrap()[2], 1 << 29 | 1);
        assert_eq!(west.edge_points(5).unwrap()[0][0], 100.0);
    }

    /// With `SKATE3_DATA`, every NavPower record of the three districts' Sim collections reads
    /// into cells whose neighbours point back at them, and tiles side by side link.
    #[test]
    fn skate3_district_navigation_reads_and_links() {
        use skate2_formats::{mount::{ArchiveSource, MountSet, MountedBig}, rw4::Arena, stream::{CollectionMap, StreamFile, decode_arena}};
        let Some(root) = std::env::var_os("SKATE3_DATA").map(std::path::PathBuf::from) else { return };
        for district in ["DIST_University", "DIST_DownTown", "DIST_Industrial"] {
            let mut mounts = MountSet::new();
            let path = root.join("data").join("content").join(format!("world{district}.big"));
            mounts.push(MountedBig::open_source(ArchiveSource::File(path)).expect("district archive mounts"));
            let base = format!("data\\content\\world\\stream\\{district}\\{district}_Sim");
            let map = CollectionMap::parse(&mounts.read_path(&format!("{base}.xsm")).unwrap()).unwrap();
            let mut tiles = Vec::new();
            for collection in &map.collections {
                let path = format!("data\\content\\world\\stream\\{district}\\{}", collection.filename);
                let path = if mounts.contains_path(&path) { path } else { format!("{path}.xsf") };
                let bytes = mounts.read_path(&path).unwrap();
                for asset in &StreamFile::parse(&bytes, collection.asset_count).unwrap().assets {
                    let resources = decode_arena(&bytes[asset.data_range.clone()]).unwrap().resources;
                    let arena = Arena::parse(&resources).unwrap();
                    for entry in arena.dictionary.iter().filter(|entry| entry.type_id == 0x00eb_0027) {
                        let record = arena.object_bytes(&resources, entry).unwrap();
                        tiles.push(NavPowerTile::read(record).unwrap_or_else(|e| panic!("{path}: {e:?}")));
                    }
                }
            }
            let (mut cells, mut borders) = (0, 0);
            for tile in &tiles {
                #[expect(clippy::cast_possible_truncation, reason = "test")]
                for id in 0..tile.cells.len() as u32 {
                    for (side, across) in tile.cell_neighbours(id).unwrap().into_iter().enumerate() {
                        if across == NO_CELL {
                            continue;
                        }
                        let back = tile.cell_neighbours(across).unwrap();
                        assert!(back.contains(&id), "{district}: cell {id} side {side} reaches {across}, which does not reach back");
                    }
                }
                cells += tile.cells.len();
                borders += tile.borders.len();
            }
            let mut linked = 0;
            for (index, tile) in tiles.iter().enumerate() {
                for other in &tiles[index + 1..] {
                    if let Some(slot) = tile.slot_of(other) {
                        tile.stitch(slot, other);
                    }
                }
            }
            for tile in &tiles {
                linked += tile.borders.iter().filter(|border| tile.across[border.edge as usize].load(Ordering::Relaxed) != NO_CELL).count();
            }
            eprintln!("{district}: {} tiles, {cells} cells, {borders} border edges, {linked} linked", tiles.len());
            assert!(cells > 0 && linked * 10 >= borders * 9, "{district}: border edges link to their neighbours");
        }
    }

    #[test]
    fn images_tu3_would_not_load_add_nothing() {
        let square = area(&[[1., 0., 1.], [1., 0., 2.], [2., 0., 2.]], &[(0, 0); 3], 0x1100);
        let mut bytes = record(&[(0., square)]);
        bytes[0x40 + 12] ^= 1;
        assert!(NavPowerTile::read(&bytes).unwrap().cells.is_empty(), "a wrong checksum");
        assert!(NavPowerTile::read(&bytes[..0x40]).is_err(), "a truncated record");
    }
}
