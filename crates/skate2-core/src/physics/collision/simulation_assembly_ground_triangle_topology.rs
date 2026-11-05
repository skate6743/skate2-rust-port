//! Private clustered-mesh GP construction for the Assembly/ground producer.
//!
//! The newer `ClusteredMeshCluster::UnitGetOverlappingGPInstances` at
//! `0x82acfdd0..0x82ad0cc0` and the older free overload used by Assembly at
//! `0x827c98f0..0x827ca7b8` materialize at most two records. This module keeps
//! their shared point/edge/tag math and their distinct dispatch/cull/scalar
//! schedules private until the complete ground Contact producer owns them.
//! The old path expands only referenced XYZ values; native vector W lanes have
//! no proven semantic meaning.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for exact clustered-mesh GP construction"
    )
)]

use crate::narrow_phase::{GpPrimitive, GpPrimitiveType};
use skate2_formats::collision::{ClusteredMesh, CollisionError, MeshCluster, MeshUnit, UnitType};

const FLAG_TRIANGLE_DEFAULT: u32 = 0x01e0;
const FLAG_TRIANGLE_BASE_WITH_EDGE_COS: u32 = 0x0101;
const EXPLICIT_EDGE_FLAG: u8 = 0x20;
const CLUSTER_ONE_SIDED_FLAG: u16 = 0x10;
const EDGE_COS_NUMERATOR: f32 = f32::from_bits(0x411d_e9e7);
const EDGE_COS_MINIMUM_NORMAL_SQUARED: f32 = f32::from_bits(0x0080_0000);

#[derive(Clone, Copy, Debug, PartialEq)]
struct AssemblyGroundTriangleTopology {
    points: [[f32; 3]; 3],
    edge_data: [f32; 3],
    flags: u32,
    volume_tag: u32,
    user_tag: u32,
    source: AssemblyGroundTriangleSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AssemblyGroundTriangleSource {
    cluster_index: usize,
    unit_stream_offset: usize,
    primitive_in_unit: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct QuadDiagonal {
    cosine: f32,
    convex: bool,
}

/// Xenon-dependent reductions and reciprocal-root estimate used by
/// `ComputeEdgeCos`. Keeping these injected distinguishes the exact recovered
/// schedule from the host approximation used for ordinary finite execution.
pub(super) trait AssemblyGroundNumeric {
    fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32;
    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32;
    fn reciprocal_seed(&mut self, value: f32) -> f32;

    fn decode_edge_scalar(&mut self, edge: u8) -> f32 {
        decode_serialized_edge(edge)
    }
}

pub(super) struct HostAssemblyGroundNumeric;

impl AssemblyGroundNumeric for HostAssemblyGroundNumeric {
    fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
        left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
    }

    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
        value.sqrt().recip()
    }

    fn reciprocal_seed(&mut self, value: f32) -> f32 {
        value.recip()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AssemblyGroundTriangle {
    primitive: GpPrimitive,
    // Companion lanes retain full line-query vectors without a second XYZ copy.
    point_w: [f32; 3],
    normal_w: f32,
    volume_tag: u32,
    user_tag: u32,
    source: AssemblyGroundTriangleSource,
}

impl AssemblyGroundTriangle {
    pub(super) fn points_full4(&self) -> [[f32; 4]; 3] {
        std::array::from_fn(|index| {
            let [x, y, z] = self.primitive.points[index];
            [x, y, z, self.point_w[index]]
        })
    }

    pub(super) fn normal_full4(&self) -> [f32; 4] {
        let [x, y, z] = self.primitive.face_normals[0];
        [x, y, z, self.normal_w]
    }

    pub(super) const fn primitive(&self) -> &GpPrimitive {
        &self.primitive
    }

    pub(super) const fn volume_tag(&self) -> u32 {
        self.volume_tag
    }

    pub(super) const fn user_tag(&self) -> u32 {
        self.user_tag
    }

    pub(super) const fn source(&self) -> AssemblyGroundTriangleSource {
        self.source
    }
}

impl AssemblyGroundTriangleSource {
    pub(super) const fn cluster_index(self) -> usize {
        self.cluster_index
    }

    pub(super) const fn unit_stream_offset(self) -> usize {
        self.unit_stream_offset
    }

    pub(super) const fn primitive_in_unit(self) -> u8 {
        self.primitive_in_unit
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct UnitGpBatch {
    records: [Option<AssemblyGroundTriangle>; 2],
    count: usize,
}

impl UnitGpBatch {
    const fn empty() -> Self {
        Self {
            records: [None, None],
            count: 0,
        }
    }

    fn as_slice(&self) -> &[Option<AssemblyGroundTriangle>] {
        &self.records[..self.count]
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &AssemblyGroundTriangle> {
        self.as_slice().iter().map(|record| {
            record
                .as_ref()
                .expect("the published GP batch prefix is initialized")
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AssemblyGroundQueryBox {
    pub(super) minimum: [f32; 3],
    pub(super) maximum: [f32; 3],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum UnitTopologyError {
    VertexDecode(CollisionError),
    InvalidVertexCount {
        unit_type: UnitType,
        actual: usize,
        expected: usize,
    },
    InvalidEdgeCount {
        actual: usize,
        expected: usize,
    },
    VertexOutsideCluster {
        index: u8,
    },
    MissingQuadDiagonal,
    UnsupportedTriangleListCount {
        count: usize,
    },
    AssemblyNativeAssertedPrimitiveCount {
        unit_type: UnitType,
        count: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnitGpOverload {
    NewMember,
    OldAssemblyFree,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UnitTopologyBatch {
    records: [Option<AssemblyGroundTriangleTopology>; 2],
    count: usize,
}

impl UnitTopologyBatch {
    const fn empty() -> Self {
        Self {
            records: [None, None],
            count: 0,
        }
    }

    fn as_slice(&self) -> &[Option<AssemblyGroundTriangleTopology>] {
        &self.records[..self.count]
    }
}

/// Reconstructs the exact signed PowerPC `slw/extsw/fcfid/frsp` denominator.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    reason = "the narrowing/sign extension and f64-to-f32 round are the native instruction path"
)]
fn serialized_edge_denominator(edge: u8) -> f32 {
    let shift = u32::from(edge & 0x1f);
    let word = 8_u32.wrapping_shl(shift);
    let signed = i64::from(word as i32);
    (signed as f64) as f32
}

fn decode_serialized_edge(edge: u8) -> f32 {
    1.0 - EDGE_COS_NUMERATOR / serialized_edge_denominator(edge)
}

/// `rw::collision::ComputeEdgeCos` at `0x827c81c0..0x827c8378`.
fn compute_edge_cos<N: AssemblyGroundNumeric>(
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
    v3: [f32; 3],
    numeric: &mut N,
) -> QuadDiagonal {
    let d10 = subtract(v1, v0);
    let d20 = subtract(v2, v0);
    let d23 = subtract(v2, v3);
    let d13 = subtract(v1, v3);
    let edge = subtract(v2, v1);
    let normal_0 = source_cross(d10, d20);
    let normal_1 = source_cross(d23, d13);
    let hinge = source_cross(normal_0, normal_1);

    // Native freezes convexity before either degenerate-normal guard.
    let convex_scalar = numeric.sum_three_products(edge, hinge);
    let convex_flag = if convex_scalar > 0.0 { 0x20 } else { 0 };
    let length_0 = numeric.sum_three_products(normal_0, normal_0);
    let length_1 = numeric.sum_three_products(normal_1, normal_1);
    if length_0 <= EDGE_COS_MINIMUM_NORMAL_SQUARED || length_1 <= EDGE_COS_MINIMUM_NORMAL_SQUARED {
        return QuadDiagonal {
            cosine: 1.0,
            convex: convex_flag != 0,
        };
    }

    let numerator = numeric.sum_three_products(normal_0, normal_1);
    let seed_0 = numeric.reciprocal_sqrt_seed(length_0);
    let seed_1 = numeric.reciprocal_sqrt_seed(length_1);
    let reciprocal_root_0 = refine_reciprocal_sqrt_twice(length_0, seed_0);
    let reciprocal_root_1 = refine_reciprocal_sqrt_twice(length_1, seed_1);
    let partial = reciprocal_root_0 * numerator;
    QuadDiagonal {
        cosine: partial * reciprocal_root_1,
        convex: convex_flag != 0,
    }
}

fn subtract(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] - right[axis])
}

/// Source-shaped cross schedule: one rounded multiply, one fused negative
/// multiply-subtract, then the final YZX permutation.
fn source_cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    let partial = [left[0] * right[1], left[1] * right[2], left[2] * right[0]];
    let permuted = [
        (-left[1]).mul_add(right[0], partial[0]),
        (-left[2]).mul_add(right[1], partial[1]),
        (-left[0]).mul_add(right[2], partial[2]),
    ];
    [permuted[1], permuted[2], permuted[0]]
}

fn refine_reciprocal_sqrt_twice(value: f32, estimate: f32) -> f32 {
    let square_0 = estimate * estimate;
    let half_0 = estimate * 0.5;
    let residual_0 = (-value).mul_add(square_0, 1.0);
    let refined_1 = half_0.mul_add(residual_0, estimate);
    let square_1 = refined_1 * refined_1;
    let half_1 = refined_1 * 0.5;
    let residual_1 = (-value).mul_add(square_1, 1.0);
    half_1.mul_add(residual_1, refined_1)
}

fn pack_explicit_edge_flags(edges: [u8; 3], cluster_flags: u16) -> u32 {
    let [first, second, third] = edges;
    FLAG_TRIANGLE_BASE_WITH_EDGE_COS
        | (u32::from(cluster_flags) & u32::from(CLUSTER_ONE_SIDED_FLAG))
        | (u32::from(first) & 0x20)
        | ((u32::from(second) & 0x20) << 1)
        | ((u32::from(third) & 0x20) << 2)
        | ((u32::from(first) & 0x40) << 3)
        | ((u32::from(second) & 0x40) << 4)
        | ((u32::from(third) & 0x40) << 5)
}

#[allow(
    clippy::too_many_lines,
    reason = "keeps safe-host validation and the two native materialization arms contiguous"
)]
fn materialize_unit_topology(
    cluster_index: usize,
    cluster_flags: u16,
    unit: &MeshUnit,
    resolved_vertices: &[[f32; 3]],
    diagonal: Option<QuadDiagonal>,
) -> Result<UnitTopologyBatch, UnitTopologyError> {
    let primitive_count = validate_unit_layout(unit)?;
    if unit.unit_type == UnitType::OldTriangle || primitive_count == 0 {
        return Ok(UnitTopologyBatch::empty());
    }

    // Validate every referenced vertex before constructing either quad sibling.
    let points = unit
        .vertex_indices
        .iter()
        .map(|&index| {
            resolved_vertices
                .get(usize::from(index))
                .copied()
                .ok_or(UnitTopologyError::VertexOutsideCluster { index })
        })
        .collect::<Result<Vec<_>, _>>()?;

    materialize_supported_unit_topology(
        cluster_index,
        cluster_flags,
        unit,
        &points,
        primitive_count,
        diagonal,
        0,
        UnitGpOverload::NewMember,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the supported native arm has distinct point, tag, and scalar-order inputs"
)]
fn materialize_supported_unit_topology(
    cluster_index: usize,
    cluster_flags: u16,
    unit: &MeshUnit,
    points: &[[f32; 3]],
    primitive_count: usize,
    diagonal: Option<QuadDiagonal>,
    volume_tag: u32,
    overload: UnitGpOverload,
) -> Result<UnitTopologyBatch, UnitTopologyError> {
    materialize_supported_unit_topology_with_decoder(
        cluster_index,
        cluster_flags,
        unit,
        points,
        primitive_count,
        diagonal,
        volume_tag,
        overload,
        &mut decode_serialized_edge,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the supported native arm has distinct point, tag, scalar-order, and decoder inputs"
)]
fn materialize_supported_unit_topology_with_decoder(
    cluster_index: usize,
    cluster_flags: u16,
    unit: &MeshUnit,
    points: &[[f32; 3]],
    primitive_count: usize,
    diagonal: Option<QuadDiagonal>,
    volume_tag: u32,
    overload: UnitGpOverload,
    decode: &mut impl FnMut(u8) -> f32,
) -> Result<UnitTopologyBatch, UnitTopologyError> {
    debug_assert!(primitive_count == 1 || primitive_count == 2);
    let explicit_edges = unit.flags & EXPLICIT_EDGE_FLAG != 0;

    if primitive_count == 1 {
        let (edge_data, flags) = if explicit_edges {
            let [edge_0, edge_1, edge_2] = unit.edge_data[..3].try_into().unwrap();
            // Native scalar division order is e0, e2, e1; stores restore the
            // serialized geometric slot order.
            let decoded_0 = decode(edge_0);
            let decoded_2 = decode(edge_2);
            let decoded_1 = decode(edge_1);
            (
                [decoded_0, decoded_1, decoded_2],
                pack_explicit_edge_flags([edge_0, edge_1, edge_2], cluster_flags),
            )
        } else {
            ([0.0; 3], FLAG_TRIANGLE_DEFAULT)
        };
        return Ok(UnitTopologyBatch {
            records: [
                Some(AssemblyGroundTriangleTopology {
                    points: [points[0], points[1], points[2]],
                    edge_data,
                    flags,
                    volume_tag,
                    user_tag: (u32::from(unit.surface_id.unwrap_or(0)) << 16)
                        | u32::from(unit.group_id.unwrap_or(0)),
                    source: AssemblyGroundTriangleSource {
                        cluster_index,
                        unit_stream_offset: unit.stream_offset,
                        primitive_in_unit: 0,
                    },
                }),
                None,
            ],
            count: 1,
        });
    }

    let user_tag =
        (u32::from(unit.surface_id.unwrap_or(0)) << 16) | u32::from(unit.group_id.unwrap_or(0));
    let source = |primitive_in_unit| AssemblyGroundTriangleSource {
        cluster_index,
        unit_stream_offset: unit.stream_offset,
        primitive_in_unit,
    };

    let (first_edge_data, first_flags, second_edge_data, second_flags) = if explicit_edges {
        let diagonal = diagonal.ok_or(UnitTopologyError::MissingQuadDiagonal)?;
        let [edge_0, edge_1, edge_2, edge_3] = unit.edge_data[..4].try_into().unwrap();
        // The two overloads store identical geometric slots but request the
        // four external scalar divisions in different native orders.
        let decoded =
            decode_explicit_quad_edges([edge_0, edge_1, edge_2, edge_3], overload, decode);
        let diagonal_convex = if diagonal.convex { 0x20 } else { 0 };
        let first_middle = (edge_1 & 0x40) | diagonal_convex;
        let second_middle = (edge_2 & 0x40) | diagonal_convex;
        (
            [decoded[0], diagonal.cosine, decoded[2]],
            pack_explicit_edge_flags([edge_0, first_middle, edge_2], cluster_flags),
            [decoded[3], diagonal.cosine, decoded[1]],
            pack_explicit_edge_flags([edge_3, second_middle, edge_1], cluster_flags),
        )
    } else {
        (
            [0.0; 3],
            FLAG_TRIANGLE_DEFAULT,
            [0.0; 3],
            FLAG_TRIANGLE_DEFAULT,
        )
    };

    Ok(UnitTopologyBatch {
        records: [
            Some(AssemblyGroundTriangleTopology {
                points: [points[0], points[1], points[2]],
                edge_data: first_edge_data,
                flags: first_flags,
                volume_tag,
                user_tag,
                source: source(0),
            }),
            Some(AssemblyGroundTriangleTopology {
                points: [points[3], points[2], points[1]],
                edge_data: second_edge_data,
                flags: second_flags,
                volume_tag,
                user_tag,
                source: source(1),
            }),
        ],
        count: 2,
    })
}

fn decode_explicit_quad_edges(
    edges: [u8; 4],
    overload: UnitGpOverload,
    mut decode: impl FnMut(u8) -> f32,
) -> [f32; 4] {
    let scalar_order = match overload {
        // `0x82ad083c..0x82ad0944`.
        UnitGpOverload::NewMember => [3, 0, 2, 1],
        // `0x827ca30c..0x827ca418`.
        UnitGpOverload::OldAssemblyFree => [2, 3, 0, 1],
    };
    let mut decoded = [0.0; 4];
    for index in scalar_order {
        decoded[index] = decode(edges[index]);
    }
    decoded
}

fn validate_unit_layout(unit: &MeshUnit) -> Result<usize, UnitTopologyError> {
    let primitive_count = match unit.unit_type {
        UnitType::OldTriangle => return Ok(0),
        UnitType::Triangle => {
            validate_vertex_count(unit, 3)?;
            1
        }
        UnitType::Quad => {
            validate_vertex_count(unit, 4)?;
            2
        }
        UnitType::TriangleList => {
            let count = unit.vertex_indices.len().checked_sub(2).ok_or(
                UnitTopologyError::InvalidVertexCount {
                    unit_type: unit.unit_type,
                    actual: unit.vertex_indices.len(),
                    expected: 2,
                },
            )?;
            if count > 2 {
                return Err(UnitTopologyError::UnsupportedTriangleListCount { count });
            }
            count
        }
    };

    let expected_vertices = primitive_count + 2;
    validate_vertex_count(unit, expected_vertices)?;
    let explicit_edges = unit.flags & EXPLICIT_EDGE_FLAG != 0;
    let expected_edges = if explicit_edges { expected_vertices } else { 0 };
    if unit.edge_data.len() != expected_edges {
        return Err(UnitTopologyError::InvalidEdgeCount {
            actual: unit.edge_data.len(),
            expected: expected_edges,
        });
    }
    Ok(primitive_count)
}

/// Old free-overload dispatch at `0x827c9964..0x827c99c4` followed by the
/// `N == 1` / `N == 2` gates at `0x827c9a54` and `0x827c9f3c`.
fn validate_old_assembly_unit_layout(unit: &MeshUnit) -> Result<usize, UnitTopologyError> {
    let primitive_count = match unit.unit_type {
        UnitType::OldTriangle => 0,
        UnitType::Triangle => 1,
        UnitType::Quad => 2,
        UnitType::TriangleList => unit.vertex_indices.len().checked_sub(2).ok_or(
            UnitTopologyError::InvalidVertexCount {
                unit_type: unit.unit_type,
                actual: unit.vertex_indices.len(),
                expected: 2,
            },
        )?,
    };
    if primitive_count != 1 && primitive_count != 2 {
        return Err(UnitTopologyError::AssemblyNativeAssertedPrimitiveCount {
            unit_type: unit.unit_type,
            count: primitive_count,
        });
    }

    validate_vertex_count(unit, primitive_count + 2)?;
    let expected_edges = if unit.flags & EXPLICIT_EDGE_FLAG != 0 {
        primitive_count + 2
    } else {
        0
    };
    if unit.edge_data.len() != expected_edges {
        return Err(UnitTopologyError::InvalidEdgeCount {
            actual: unit.edge_data.len(),
            expected: expected_edges,
        });
    }
    Ok(primitive_count)
}

fn validate_vertex_count(unit: &MeshUnit, expected: usize) -> Result<(), UnitTopologyError> {
    if unit.vertex_indices.len() == expected {
        Ok(())
    } else {
        Err(UnitTopologyError::InvalidVertexCount {
            unit_type: unit.unit_type,
            actual: unit.vertex_indices.len(),
            expected,
        })
    }
}

/// Constructs the semantic fields written by accepted native GP records.
///
/// `local_vertices` are already decompressed with the cluster's proven mode;
/// keeping that step outside this seam avoids inventing W-lane meaning. The
/// transform is mandatory, as it is on all three recovered callers.
fn build_unit_gp_with_numeric<N: AssemblyGroundNumeric>(
    cluster_index: usize,
    cluster_flags: u16,
    unit: &MeshUnit,
    local_vertices: &[[f32; 3]],
    query_box: AssemblyGroundQueryBox,
    transform: [f32; 16],
    numeric: &mut N,
) -> Result<UnitGpBatch, UnitTopologyError> {
    let primitive_count = validate_unit_layout(unit)?;
    if unit.unit_type == UnitType::OldTriangle || primitive_count == 0 {
        return Ok(UnitGpBatch::empty());
    }

    let local_points = unit
        .vertex_indices
        .iter()
        .map(|&index| {
            local_vertices
                .get(usize::from(index))
                .copied()
                .ok_or(UnitTopologyError::VertexOutsideCluster { index })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !unit_points_overlap_box(&local_points, query_box) {
        return Ok(UnitGpBatch::empty());
    }

    build_supported_unit_gp_with_numeric(
        cluster_index,
        cluster_flags,
        unit,
        &local_points,
        primitive_count,
        transform,
        0,
        UnitGpOverload::NewMember,
        numeric,
    )
}

/// Selected vertex access shared by decoded old fixtures and lazy gameplay
/// cluster views. No unreferenced vertex or normal table is inspected here.
pub(super) trait ModernGpVertexRead {
    fn vertex_position(
        &self,
        mesh: &ClusteredMesh,
        index: u8,
    ) -> Result<[f32; 3], UnitTopologyError>;
}
impl ModernGpVertexRead for MeshCluster {
    fn vertex_position(
        &self,
        mesh: &ClusteredMesh,
        index: u8,
    ) -> Result<[f32; 3], UnitTopologyError> {
        self.vertices
            .get(usize::from(index))
            .map(|vertex| vertex.position(mesh))
            .ok_or(UnitTopologyError::VertexOutsideCluster { index })
    }
}

impl ModernGpVertexRead for skate2_formats::collision::line_cursor::MeshLineClusterView<'_> {
    fn vertex_position(
        &self,
        mesh: &ClusteredMesh,
        index: u8,
    ) -> Result<[f32; 3], UnitTopologyError> {
        skate2_formats::collision::line_cursor::MeshLineClusterView::vertex_position(
            self, mesh, index,
        )
        .map_err(UnitTopologyError::VertexDecode)
    }
}

/// Modern member overload82acfdd0, used by gameplay mesh line queries.
/// Decode only referenced XYZ lanes; forward placement supplies point W.
/// The caller retains the parsed unit's byte size for its native range cursor.
pub(super) fn build_new_unit_gp_with_numeric<N: AssemblyGroundNumeric, V: ModernGpVertexRead>(
    mesh: &ClusteredMesh,
    cluster_index: usize,
    cluster: &V,
    unit: &MeshUnit,
    query_box: AssemblyGroundQueryBox,
    transform: [f32; 16],
    numeric: &mut N,
) -> Result<UnitGpBatch, UnitTopologyError> {
    let primitive_count = validate_unit_layout(unit)?;
    if unit.unit_type == UnitType::OldTriangle || primitive_count == 0 {
        return Ok(UnitGpBatch::empty());
    }
    let local_points = unit
        .vertex_indices
        .iter()
        .map(|&index| cluster.vertex_position(mesh, index))
        .collect::<Result<Vec<_>, _>>()?;
    if !unit_points_overlap_box(&local_points, query_box) {
        return Ok(UnitGpBatch::empty());
    }
    build_supported_unit_gp_with_numeric(
        cluster_index,
        mesh.cluster_flags,
        unit,
        &local_points,
        primitive_count,
        transform,
        0,
        UnitGpOverload::NewMember,
        numeric,
    )
}

/// Geometry-only projection of the old Assembly unit constructor. These
/// fields are copied unchanged from topology by `build_gp_record`; the host's
/// Skate3 scene does not consume native2 face frames, edge directions or GP
/// dimensions. Preserve the exact old dispatch, transform and edge arithmetic
/// without constructing those discarded contact-solver fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AssemblyGroundFeatures {
    pub(super) points: [[f32; 3]; 3],
    pub(super) edge_data: [f32; 3],
    pub(super) flags: u32,
}

pub(super) fn build_old_assembly_unit_features(
    mesh: &ClusteredMesh,
    cluster_index: usize,
    cluster: &MeshCluster,
    unit: &MeshUnit,
    transform: [f32; 16],
) -> Result<Vec<AssemblyGroundFeatures>, UnitTopologyError> {
    let primitive_count = validate_old_assembly_unit_layout(unit)?;
    let mut points = [[0.; 3]; 4];
    for (slot, &index) in unit.vertex_indices.iter().enumerate() {
        let position = cluster.vertices.get(usize::from(index))
            .map(|vertex|vertex.position(mesh))
            .ok_or(UnitTopologyError::VertexOutsideCluster { index })?;
        points[slot] = transform_point(transform, position);
    }
    let mut numeric = HostAssemblyGroundNumeric;
    let diagonal = if primitive_count == 2 && unit.flags & EXPLICIT_EDGE_FLAG != 0 {
        Some(compute_edge_cos(points[0], points[1], points[2], points[3], &mut numeric))
    } else { None };
    let topology = materialize_supported_unit_topology(
        cluster_index, mesh.cluster_flags, unit, &points[..unit.vertex_indices.len()],
        primitive_count, diagonal, 0, UnitGpOverload::OldAssemblyFree,
    )?;
    Ok(topology.records[..topology.count].iter().map(|record| {
        let record = record.as_ref().expect("published topology prefix");
        AssemblyGroundFeatures { points:record.points,edge_data:record.edge_data,flags:record.flags }
    }).collect())
}

/// Assembly's older free overload at `0x827c98f0..0x827ca7b8`.
///
/// Unlike the newer member overload, this path has no model-space AABB and
/// therefore performs no pre-transform cull. Only referenced cluster vertices
/// are expanded, in serialized unit-index order. Unsupported primitive counts
/// are surfaced as a typed host counterpart to the native assertion path.
pub(super) fn build_old_assembly_unit_gp_with_numeric<N: AssemblyGroundNumeric>(
    mesh: &ClusteredMesh,
    cluster_index: usize,
    cluster: &MeshCluster,
    unit: &MeshUnit,
    transform: [f32; 16],
    volume_ref: u32,
    numeric: &mut N,
) -> Result<UnitGpBatch, UnitTopologyError> {
    let primitive_count = validate_old_assembly_unit_layout(unit)?;
    let local_points = unit
        .vertex_indices
        .iter()
        .map(|&index| {
            cluster
                .vertices
                .get(usize::from(index))
                .map(|vertex| vertex.position(mesh))
                .ok_or(UnitTopologyError::VertexOutsideCluster { index })
        })
        .collect::<Result<Vec<_>, _>>()?;

    build_supported_unit_gp_with_numeric(
        cluster_index,
        mesh.cluster_flags,
        unit,
        &local_points,
        primitive_count,
        transform,
        volume_ref,
        UnitGpOverload::OldAssemblyFree,
        numeric,
    )
}

pub(super) fn build_old_assembly_unit_gp(
    mesh: &ClusteredMesh,
    cluster_index: usize,
    cluster: &MeshCluster,
    unit: &MeshUnit,
    transform: [f32; 16],
    volume_ref: u32,
) -> Result<UnitGpBatch, UnitTopologyError> {
    let mut numeric = HostAssemblyGroundNumeric;
    build_old_assembly_unit_gp_with_numeric(
        mesh,
        cluster_index,
        cluster,
        unit,
        transform,
        volume_ref,
        &mut numeric,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the shared supported arm preserves both overloads' explicit native inputs"
)]
fn build_supported_unit_gp_with_numeric<N: AssemblyGroundNumeric>(
    cluster_index: usize,
    cluster_flags: u16,
    unit: &MeshUnit,
    local_points: &[[f32; 3]],
    primitive_count: usize,
    transform: [f32; 16],
    volume_tag: u32,
    overload: UnitGpOverload,
    numeric: &mut N,
) -> Result<UnitGpBatch, UnitTopologyError> {
    debug_assert!(primitive_count == 1 || primitive_count == 2);

    let transformed = local_points
        .iter()
        .map(|&point| transform_point(transform, point))
        .collect::<Vec<_>>();
    let point_w: Vec<f32> = local_points
        .iter()
        .map(|point| {
            let x = transform[3].mul_add(point[0], transform[15]);
            let xy = transform[7].mul_add(point[1], x);
            transform[11].mul_add(point[2], xy)
        })
        .collect();
    let first_points = [transformed[0], transformed[1], transformed[2]];
    let first_point_w = [point_w[0], point_w[1], point_w[2]];
    let (first_raw_normal, first_normal_squared) =
        build_raw_face_normal(first_points, first_point_w, numeric);
    let second_raw = (primitive_count == 2).then(|| {
        let points = [transformed[3], transformed[2], transformed[1]];
        let (raw, squared) =
            build_raw_face_normal(points, [point_w[3], point_w[2], point_w[1]], numeric);
        (points, raw, squared)
    });
    let first_face_seed = numeric.reciprocal_sqrt_seed(first_normal_squared);
    let second_face_seed = second_raw
        .as_ref()
        .map(|(_, _, squared)| numeric.reciprocal_sqrt_seed(*squared));
    let first_normal = finish_face_normal(first_raw_normal, first_normal_squared, first_face_seed);
    let second = match (second_raw, second_face_seed) {
        (Some((points, raw, squared)), Some(seed)) => {
            Some((points, finish_face_normal(raw, squared, seed)))
        }
        (None, None) => None,
        _ => unreachable!("the second face and estimate are staged together"),
    };

    // Both quad face normals precede the optional shared-diagonal call.
    let diagonal = if primitive_count == 2 && unit.flags & EXPLICIT_EDGE_FLAG != 0 {
        Some(compute_edge_cos(
            transformed[0],
            transformed[1],
            transformed[2],
            transformed[3],
            numeric,
        ))
    } else {
        None
    };

    let mut remapped = unit.clone();
    remapped.vertex_indices = (0..transformed.len())
        .map(|index| {
            u8::try_from(index).expect("a native unit has at most four materialized vertices")
        })
        .collect();
    let topology = {
        let mut decode = |edge| numeric.decode_edge_scalar(edge);
        materialize_supported_unit_topology_with_decoder(
            cluster_index,
            cluster_flags,
            &remapped,
            &transformed,
            primitive_count,
            diagonal,
            volume_tag,
            overload,
            &mut decode,
        )?
    };
    let first = topology.records[0].map(|record| {
        build_gp_record(
            record,
            first_point_w,
            first_normal,
            [0, 1, 2],
            [1, 0, 2],
            numeric,
        )
    });
    let second = match (topology.records[1], second) {
        (Some(record), Some((points, normal))) => {
            debug_assert_eq!(record.points, points);
            Some(build_gp_record(
                record,
                [point_w[3], point_w[2], point_w[1]],
                normal,
                [0, 2, 1],
                [0, 1, 2],
                numeric,
            ))
        }
        (None, None) => None,
        _ => unreachable!("topology and face-normal counts agree"),
    };
    Ok(UnitGpBatch {
        records: [first, second],
        count: topology.count,
    })
}

fn unit_points_overlap_box(points: &[[f32; 3]], query: AssemblyGroundQueryBox) -> bool {
    let mut minimum = points[0];
    let mut maximum = points[0];
    for point in &points[1..] {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(point[axis]);
            maximum[axis] = maximum[axis].max(point[axis]);
        }
    }
    (0..3).all(|axis| !(minimum[axis] > query.maximum[axis] || query.minimum[axis] > maximum[axis]))
}

fn transform_point(transform: [f32; 16], point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        let x = transform[row].mul_add(point[0], transform[12 + row]);
        let y = transform[4 + row].mul_add(point[1], x);
        transform[8 + row].mul_add(point[2], y)
    })
}

fn build_raw_face_normal<N: AssemblyGroundNumeric>(
    points: [[f32; 3]; 3],
    point_w: [f32; 3],
    numeric: &mut N,
) -> ([f32; 4], f32) {
    let d10 = subtract(points[1], points[0]);
    let d20 = subtract(points[2], points[0]);
    let raw = source_cross(d10, d20);
    let d10_w = point_w[1] - point_w[0];
    let d20_w = point_w[2] - point_w[0];
    let raw_w = (-d10_w).mul_add(d20_w, d10_w * d20_w);
    let squared = numeric.sum_three_products(raw, raw);
    ([raw[0], raw[1], raw[2], raw_w], squared)
}

fn finish_face_normal(raw: [f32; 4], squared: f32, seed: f32) -> [f32; 4] {
    let inverse_length = refine_reciprocal_sqrt_twice(squared, seed);
    raw.map(|component| component * inverse_length)
}

fn build_gp_record<N: AssemblyGroundNumeric>(
    topology: AssemblyGroundTriangleTopology,
    point_w: [f32; 3],
    face_normal: [f32; 4],
    edge_reduction_order: [usize; 3],
    dimension_seed_order: [usize; 3],
    numeric: &mut N,
) -> AssemblyGroundTriangle {
    let [a, b, c] = topology.points;
    let edge_vectors = [subtract(c, a), subtract(b, c), subtract(a, b)];
    let mut squared_lengths = [0.0; 3];
    for index in edge_reduction_order {
        squared_lengths[index] =
            numeric.sum_three_products(edge_vectors[index], edge_vectors[index]);
    }
    let mut reciprocal_sqrt_seeds = [0.0; 3];
    for index in edge_reduction_order {
        reciprocal_sqrt_seeds[index] = numeric.reciprocal_sqrt_seed(squared_lengths[index]);
    }
    let inverse_lengths: [f32; 3] = std::array::from_fn(|index| {
        refine_reciprocal_sqrt_once(squared_lengths[index], reciprocal_sqrt_seeds[index])
    });
    let mut reciprocal_seeds = [0.0; 3];
    for index in dimension_seed_order {
        reciprocal_seeds[index] = numeric.reciprocal_seed(inverse_lengths[index]);
    }
    let edge_directions = std::array::from_fn(|index| {
        edge_vectors[index].map(|component| component * inverse_lengths[index])
    });
    let dimensions = std::array::from_fn(|index| {
        refine_reciprocal_twice(inverse_lengths[index], reciprocal_seeds[index])
    });
    AssemblyGroundTriangle {
        primitive: GpPrimitive {
            primitive_type: GpPrimitiveType::Triangle,
            points: topology.points,
            face_normals: [
                [face_normal[0], face_normal[1], face_normal[2]],
                [0.0; 3],
                [0.0; 3],
            ],
            face_normal_count: 1,
            edge_directions,
            edge_direction_count: 3,
            dimensions,
            fatness: 0.0,
            edge_data: topology.edge_data,
            flags: topology.flags,
        },
        point_w,
        normal_w: face_normal[3],
        volume_tag: topology.volume_tag,
        user_tag: topology.user_tag,
        source: topology.source,
    }
}

fn refine_reciprocal_sqrt_once(value: f32, estimate: f32) -> f32 {
    let square = estimate * estimate;
    let half = estimate * 0.5;
    let residual = (-value).mul_add(square, 1.0);
    half.mul_add(residual, estimate)
}

fn refine_reciprocal_twice(value: f32, estimate: f32) -> f32 {
    let residual_0 = (-value).mul_add(estimate, 1.0);
    let refined_1 = estimate.mul_add(residual_0, estimate);
    let residual_1 = (-value).mul_add(refined_1, 1.0);
    refined_1.mul_add(residual_1, refined_1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::collision::{ClusterVertex, VertexCompression};

    fn line_transform() -> [f32; 16] {
        let mut transform = IDENTITY;
        transform[3] = 1.1;
        transform[7] = -0.7;
        transform[11] = 0.3;
        transform[15] = 0.0;
        transform
    }

    #[test]
    fn modern_line_records_retain_full_point_and_normal_w_without_extra_numeric_calls() {
        let mesh = clustered_mesh(1.0, 0);
        let cluster = uncompressed_cluster();
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let mut numeric = RecordingNumeric::default();
        let batch = build_new_unit_gp_with_numeric(
            &mesh,
            7,
            &cluster,
            &source,
            broad_query(),
            line_transform(),
            &mut numeric,
        )
        .unwrap();
        let record = batch.iter().next().unwrap();
        assert_eq!(
            record.points_full4().map(|p| p[3].to_bits()),
            [0, 0x3f8c_cccd, 0xbf33_3333]
        );
        // Independent Python/ucrt fmaf: rounded a*b minus exact a*b leaves
        // this nonzero residual. Canonical normal W=0 would lose source bits.
        assert_eq!(
            record.normal_full4().map(f32::to_bits),
            [0, 0, 0x3f80_0000, 0x32c2_8f5c]
        );
        assert_eq!(record.source().cluster_index(), 7);
        assert_eq!(record.user_tag(), 0xabcd_1234);
        assert_eq!(numeric.reductions.len(), 4);
        assert_eq!(numeric.seed_values.len(), 4);
        assert_eq!(numeric.reciprocal_values.len(), 3);
        let mut original_numeric = RecordingNumeric::default();
        let original = build_unit_gp_with_numeric(
            7,
            0,
            &source,
            &vertices(),
            broad_query(),
            line_transform(),
            &mut original_numeric,
        )
        .unwrap();
        assert_eq!(batch, original);
        assert_eq!(numeric.events, original_numeric.events);
    }

    #[test]
    fn quad_line_lanes_follow_second_primitive_order_and_same_nonideal_face_estimate() {
        let mesh = clustered_mesh(1.0, 0);
        let cluster = uncompressed_cluster();
        let source = unit(UnitType::Quad, 2, &[0, 1, 2, 3], &[]);
        let batch = build_new_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &source,
            broad_query(),
            line_transform(),
            &mut FixedEstimateNumeric,
        )
        .unwrap();
        let records: Vec<_> = batch.iter().collect();
        assert_eq!(
            records[0].points_full4().map(|p| p[3].to_bits()),
            [0, 0x3f8c_cccd, 0xbf33_3333]
        );
        assert_eq!(
            records[1].points_full4().map(|p| p[3].to_bits()),
            [0x3ecc_ccce, 0xbf33_3333, 0x3f8c_cccd]
        );
        for record in records {
            assert_eq!(
                record.normal_full4().map(f32::to_bits),
                [0, 0, 0x3f7d_3ecc, 0x32c0_7768]
            );
        }
    }

    #[test]
    fn modern_selected_vertices_ignore_serialized_w_and_unreferenced_bad_points() {
        let mesh = clustered_mesh(1.0, 0);
        let mut cluster = uncompressed_cluster();
        for vertex in &mut cluster.vertices {
            if let ClusterVertex::Uncompressed(point) = vertex {
                point[3] = f32::NAN;
            }
        }
        cluster.vertices[3] = ClusterVertex::Uncompressed([f32::NAN; 4]);
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let batch = build_new_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &source,
            broad_query(),
            line_transform(),
            &mut HostAssemblyGroundNumeric,
        )
        .unwrap();
        assert!(
            batch
                .iter()
                .next()
                .unwrap()
                .points_full4()
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        let mut numeric = RecordingNumeric::default();
        let separated = AssemblyGroundQueryBox {
            minimum: [3.0; 3],
            maximum: [4.0; 3],
        };
        let culled = build_new_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &source,
            separated,
            [f32::NAN; 16],
            &mut numeric,
        )
        .unwrap();
        assert_eq!(culled, UnitGpBatch::empty());
        assert!(numeric.events.is_empty());
    }

    #[test]
    fn line_point_w_uses_the_same_fused_forward_graph_as_xyz() {
        let mesh = clustered_mesh(1.0, 0);
        let mut cluster = uncompressed_cluster();
        cluster.vertices[0] = ClusterVertex::Uncompressed([
            f32::from_bits(0x43ee_3db1),
            f32::from_bits(0xc2d2_de1c),
            f32::from_bits(0xc454_755c),
            f32::NAN,
        ]);
        let mut transform = IDENTITY;
        transform[3] = f32::from_bits(0xc3c9_4647);
        transform[7] = f32::from_bits(0xc334_73f5);
        transform[11] = f32::from_bits(0x42c2_a7bf);
        transform[15] = f32::from_bits(0x420e_c6ef);
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let batch = build_new_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &source,
            AssemblyGroundQueryBox {
                minimum: [-1000.0; 3],
                maximum: [1000.0; 3],
            },
            transform,
            &mut HostAssemblyGroundNumeric,
        )
        .unwrap();
        assert_eq!(
            batch.iter().next().unwrap().points_full4()[0][3].to_bits(),
            0xc879_7887
        );
    }

    #[test]
    fn line_normal_w_keeps_exceptional_zero_products_while_xyz_remains_finite() {
        let mesh = clustered_mesh(1.0, 0);
        let cluster = uncompressed_cluster();
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let mut transform = IDENTITY;
        transform[3] = f32::INFINITY;
        let batch = build_new_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &source,
            broad_query(),
            transform,
            &mut HostAssemblyGroundNumeric,
        )
        .unwrap();
        let record = batch.iter().next().unwrap();
        assert!(record.points_full4()[0][3].is_nan());
        assert!(record.normal_full4()[3].is_nan());
        assert_eq!(
            record.primitive().face_normals[0].map(f32::to_bits),
            [0, 0, 0x3f80_0000]
        );
    }

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    fn unit(unit_type: UnitType, flags: u8, indices: &[u8], edges: &[u8]) -> MeshUnit {
        MeshUnit {
            stream_offset: 0x34,
            unit_type,
            flags,
            vertex_indices: indices.to_vec(),
            edge_data: edges.to_vec(),
            group_id: Some(0x1234),
            surface_id: Some(0xabcd),
        }
    }

    fn vertices() -> [[f32; 3]; 4] {
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ]
    }

    fn broad_query() -> AssemblyGroundQueryBox {
        AssemblyGroundQueryBox {
            minimum: [-10.0; 3],
            maximum: [10.0; 3],
        }
    }

    fn clustered_mesh(granularity: f32, cluster_flags: u16) -> ClusteredMesh {
        ClusteredMesh {
            bounding_box_min: [0.0; 4],
            bounding_box_max: [0.0; 4],
            aggregate_vtable_offset: 0,
            tag_bit_count: 0,
            volume_count: 0,
            aggregate_padding: 0,
            kd_tree_offset: 0,
            cluster_data_offset: 0,
            vertex_compression_granularity: granularity,
            cluster_flags,
            group_id_size: 2,
            surface_id_size: 2,
            cluster_count: 1,
            maximum_cluster_count: 1,
            unit_count: 1,
            maximum_unit_count: 1,
            allocation_size: 0,
            default_group_id: 0,
            default_surface_id: 0,
            default_edge_angle: 0,
        }
    }

    fn cluster(vertices: Vec<ClusterVertex>, compression: VertexCompression) -> MeshCluster {
        MeshCluster {
            unit_count: 1,
            unit_data_size: 0,
            unit_data_start: 0,
            normal_start: 0,
            total_size: 0,
            vertex_count: u8::try_from(vertices.len()).unwrap(),
            normal_count: 0,
            compression,
            vertices,
            normals: Vec::new(),
            unit_data: Vec::new(),
        }
    }

    fn uncompressed_cluster() -> MeshCluster {
        cluster(
            vertices()
                .into_iter()
                .map(|[x, y, z]| ClusterVertex::Uncompressed([x, y, z, 0.0]))
                .collect(),
            VertexCompression::Uncompressed,
        )
    }

    fn records(batch: &UnitTopologyBatch) -> Vec<AssemblyGroundTriangleTopology> {
        batch
            .as_slice()
            .iter()
            .map(|record| record.expect("initialized topology prefix"))
            .collect()
    }

    #[derive(Default)]
    struct RecordingNumeric {
        reductions: Vec<([f32; 3], [f32; 3])>,
        seed_values: Vec<f32>,
        reciprocal_values: Vec<f32>,
        edge_scalars: Vec<u8>,
        events: Vec<NumericEvent>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum NumericEvent {
        Reduction,
        ReciprocalSqrt(u32),
        Reciprocal(u32),
        EdgeScalar(u8),
    }

    impl AssemblyGroundNumeric for RecordingNumeric {
        fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
            self.reductions.push((left, right));
            self.events.push(NumericEvent::Reduction);
            left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
        }

        fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
            self.seed_values.push(value);
            self.events
                .push(NumericEvent::ReciprocalSqrt(value.to_bits()));
            value.sqrt().recip()
        }

        fn reciprocal_seed(&mut self, value: f32) -> f32 {
            self.reciprocal_values.push(value);
            self.events.push(NumericEvent::Reciprocal(value.to_bits()));
            value.recip()
        }

        fn decode_edge_scalar(&mut self, edge: u8) -> f32 {
            self.edge_scalars.push(edge);
            self.events.push(NumericEvent::EdgeScalar(edge));
            decode_serialized_edge(edge)
        }
    }

    struct QueuedNumeric {
        reductions: std::vec::IntoIter<f32>,
        seed_calls: usize,
    }

    struct FixedEstimateNumeric;

    impl AssemblyGroundNumeric for FixedEstimateNumeric {
        fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
            left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
        }

        fn reciprocal_sqrt_seed(&mut self, _value: f32) -> f32 {
            0.75
        }

        fn reciprocal_seed(&mut self, _value: f32) -> f32 {
            1.25
        }
    }

    impl AssemblyGroundNumeric for QueuedNumeric {
        fn sum_three_products(&mut self, _left: [f32; 3], _right: [f32; 3]) -> f32 {
            self.reductions.next().expect("queued reduction")
        }

        fn reciprocal_sqrt_seed(&mut self, _value: f32) -> f32 {
            self.seed_calls += 1;
            1.0
        }

        fn reciprocal_seed(&mut self, value: f32) -> f32 {
            value.recip()
        }
    }

    #[test]
    fn gp_triangle_uses_native_face_cyclic_edges_dimensions_and_counts() {
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let mut numeric = HostAssemblyGroundNumeric;
        let batch = build_unit_gp_with_numeric(
            4,
            0xffff,
            &source,
            &vertices(),
            broad_query(),
            IDENTITY,
            &mut numeric,
        )
        .unwrap();
        let result = batch.as_slice()[0].expect("one triangle");

        assert_eq!(result.primitive.primitive_type, GpPrimitiveType::Triangle);
        assert_eq!(
            result.primitive.points,
            [vertices()[0], vertices()[1], vertices()[2]]
        );
        assert_eq!(
            result.primitive.face_normals[0].map(f32::to_bits),
            [0, 0, 0x3f80_0000]
        );
        assert_eq!(
            result
                .primitive
                .edge_directions
                .map(|edge| edge.map(f32::to_bits)),
            [
                [0, 0x3f80_0000, 0],
                [0x3f35_04f3, 0xbf35_04f3, 0],
                [0xbf80_0000, 0, 0],
            ]
        );
        assert_eq!(
            result.primitive.dimensions.map(f32::to_bits),
            [0x3f80_0000, 0x3fb5_04f3, 0x3f80_0000]
        );
        assert_eq!(result.primitive.face_normal_count, 1);
        assert_eq!(result.primitive.edge_direction_count, 3);
        assert_eq!(result.primitive.fatness.to_bits(), 0);
        assert_eq!(result.primitive.flags, FLAG_TRIANGLE_DEFAULT);
        assert_eq!(result.source.cluster_index, 4);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "the single ordered event vector freezes the complete quad callback chronology"
    )]
    fn gp_quad_builds_both_normals_before_diagonal_and_both_edge_frames() {
        let source = unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &[0; 4]);
        let mut numeric = RecordingNumeric::default();
        let batch = build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &vertices(),
            broad_query(),
            IDENTITY,
            &mut numeric,
        )
        .unwrap();
        let records = batch
            .as_slice()
            .iter()
            .map(|record| record.expect("quad record"))
            .collect::<Vec<_>>();

        assert_eq!(
            records[0].primitive.face_normals[0].map(f32::to_bits),
            [0, 0, 0x3f80_0000]
        );
        assert_eq!(
            records[1].primitive.face_normals[0].map(f32::to_bits),
            [0, 0, 0x3f80_0000]
        );
        assert_eq!(
            records[1].primitive.points,
            [vertices()[3], vertices()[2], vertices()[1]]
        );
        assert_eq!(
            records[1]
                .primitive
                .edge_directions
                .map(|edge| edge.map(f32::to_bits)),
            [
                [0, 0xbf80_0000, 0],
                [0xbf35_04f3, 0x3f35_04f3, 0],
                [0x3f80_0000, 0, 0],
            ]
        );
        // Two face seeds, two ComputeEdgeCos seeds, then six edge seeds.
        assert_eq!(numeric.seed_values.len(), 10);
        assert_eq!(numeric.reciprocal_values.len(), 6);
        let reduction = NumericEvent::Reduction;
        let rsqrt_one = NumericEvent::ReciprocalSqrt(0x3f80_0000);
        let rsqrt_two = NumericEvent::ReciprocalSqrt(0x4000_0000);
        let reciprocal_one = NumericEvent::Reciprocal(0x3f80_0000);
        let reciprocal_diagonal = NumericEvent::Reciprocal(0x3f35_04f3);
        assert_eq!(
            numeric.events,
            [
                reduction,
                reduction,
                rsqrt_one,
                rsqrt_one,
                reduction,
                reduction,
                reduction,
                reduction,
                rsqrt_one,
                rsqrt_one,
                NumericEvent::EdgeScalar(0),
                NumericEvent::EdgeScalar(0),
                NumericEvent::EdgeScalar(0),
                NumericEvent::EdgeScalar(0),
                reduction,
                reduction,
                reduction,
                rsqrt_one,
                rsqrt_two,
                rsqrt_one,
                reciprocal_diagonal,
                reciprocal_one,
                reciprocal_one,
                reduction,
                reduction,
                reduction,
                rsqrt_one,
                rsqrt_one,
                rsqrt_two,
                reciprocal_one,
                reciprocal_diagonal,
                reciprocal_one,
            ]
        );
        assert_eq!(
            numeric
                .reciprocal_values
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            [
                0x3f35_04f3,
                0x3f80_0000,
                0x3f80_0000,
                0x3f80_0000,
                0x3f35_04f3,
                0x3f80_0000,
            ]
        );
        assert_eq!(
            [records[0].primitive.flags, records[1].primitive.flags],
            [0x101, 0x101]
        );
    }

    #[test]
    fn quad_second_face_reduces_and_seeds_cyclic_slots_zero_two_one() {
        let asymmetric = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [5.0, 7.0, 0.0],
        ];
        let source = unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &[0; 4]);
        let mut numeric = RecordingNumeric::default();
        build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &asymmetric,
            broad_query(),
            IDENTITY,
            &mut numeric,
        )
        .unwrap();

        // Face 0, face 1, and ComputeEdgeCos consume six reductions. The
        // first GP record consumes the next three; record 1 begins at nine.
        assert_eq!(
            numeric.reductions[9..12]
                .iter()
                .map(|(left, _)| *left)
                .collect::<Vec<_>>(),
            vec![[-3.0, -7.0, 0.0], [5.0, 4.0, 0.0], [-2.0, 3.0, 0.0]]
        );
        assert_eq!(
            numeric.seed_values[7..10]
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            [58.0_f32.to_bits(), 41.0_f32.to_bits(), 13.0_f32.to_bits()]
        );
        assert_eq!(
            numeric.reciprocal_values[3..6]
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            [58.0_f32, 13.0_f32, 41.0_f32].map(|squared| {
                refine_reciprocal_sqrt_once(squared, squared.sqrt().recip()).to_bits()
            })
        );
    }

    #[test]
    fn gp_cull_is_inclusive_and_precedes_transform_and_numeric_work() {
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let separated = AssemblyGroundQueryBox {
            minimum: [2.0, -1.0, -1.0],
            maximum: [3.0, 2.0, 1.0],
        };
        let mut into_query = IDENTITY;
        into_query[12] = 2.0;
        let mut numeric = RecordingNumeric::default();
        let rejected = build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &vertices(),
            separated,
            into_query,
            &mut numeric,
        )
        .unwrap();
        assert_eq!(rejected, UnitGpBatch::empty());
        assert!(numeric.reductions.is_empty());
        assert!(numeric.seed_values.is_empty());

        let touching = AssemblyGroundQueryBox {
            minimum: [1.0, -1.0, -1.0],
            maximum: [2.0, 2.0, 1.0],
        };
        let mut numeric = RecordingNumeric::default();
        let accepted = build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &vertices(),
            touching,
            IDENTITY,
            &mut numeric,
        )
        .unwrap();
        assert_eq!(accepted.count, 1);
        assert!(!numeric.reductions.is_empty());

        let unordered = AssemblyGroundQueryBox {
            minimum: [f32::NAN; 3],
            maximum: [f32::NAN; 3],
        };
        assert!(unit_points_overlap_box(&vertices()[..3], unordered));
    }

    #[test]
    fn affine_point_conversion_keeps_translation_then_x_y_z_fused_order() {
        let transform = [
            1.25, -0.5, 0.75, 0.0, -2.0, 0.25, 1.5, 0.0, 0.125, -1.25, 2.0, 0.0, 3.0, -4.0, 5.0,
            1.0,
        ];
        assert_eq!(
            transform_point(transform, [0.3, -0.7, 1.1]).map(f32::to_bits),
            [0x409d_3333, 0xc0b6_6667, 0x40cc_0000]
        );

        let mut discriminator = [0.0; 16];
        discriminator[0] = f32::from_bits(0xc3c9_4647);
        discriminator[4] = f32::from_bits(0xc334_73f5);
        discriminator[8] = f32::from_bits(0x42c2_a7bf);
        discriminator[12] = f32::from_bits(0x420e_c6ef);
        let point = [
            f32::from_bits(0x43ee_3db1),
            f32::from_bits(0xc2d2_de1c),
            f32::from_bits(0xc454_755c),
        ];
        assert_eq!(
            transform_point(discriminator, point)[0].to_bits(),
            0xc879_7887
        );
    }

    #[test]
    fn nonideal_estimates_expose_face_edge_and_dimension_refinement_counts() {
        let source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        let mut numeric = FixedEstimateNumeric;
        let record = build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &vertices(),
            broad_query(),
            IDENTITY,
            &mut numeric,
        )
        .unwrap()
        .as_slice()[0]
            .expect("triangle");
        assert_eq!(
            record.primitive.face_normals[0].map(f32::to_bits),
            [0, 0, 0x3f7d_3ecc]
        );
        assert_eq!(
            record
                .primitive
                .edge_directions
                .map(|edge| edge.map(f32::to_bits)),
            [
                [0, 0x3f6a_0000, 0],
                [0x3f34_0000, 0xbf34_0000, 0],
                [0xbf6a_0000, 0, 0],
            ]
        );
        assert_eq!(
            record.primitive.dimensions.map(f32::to_bits),
            [0x3f8b_f9f0, 0x3fb6_015b, 0x3f8b_f9f0]
        );
    }

    #[test]
    fn degenerate_gp_face_and_edge_still_request_estimates_without_zero_shortcut() {
        let source = unit(UnitType::Triangle, 1, &[0, 0, 2], &[]);
        let mut numeric = RecordingNumeric::default();
        let result = build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &vertices(),
            broad_query(),
            IDENTITY,
            &mut numeric,
        );
        assert!(result.is_ok());
        assert_eq!(
            numeric
                .seed_values
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            [0, 0x3f80_0000, 0x3f80_0000, 0]
        );
    }

    #[test]
    fn compute_edge_cos_preserves_orientation_and_coplanar_result() {
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [1.0, 0.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];
        let mut numeric = HostAssemblyGroundNumeric;
        let coplanar = compute_edge_cos(v0, v1, v2, [1.0, 1.0, 0.0], &mut numeric);
        assert_eq!(coplanar.cosine.to_bits(), 0x3f80_0000);
        assert!(!coplanar.convex);

        let below = compute_edge_cos(v0, v1, v2, [1.0, 1.0, -1.0], &mut numeric);
        let above = compute_edge_cos(v0, v1, v2, [1.0, 1.0, 1.0], &mut numeric);
        assert!(below.convex);
        assert!(!above.convex);
        assert_eq!(below.cosine.to_bits(), above.cosine.to_bits());
    }

    #[test]
    fn compute_edge_cos_guards_after_three_reductions_and_before_estimates() {
        let mut numeric = RecordingNumeric::default();
        let result = compute_edge_cos(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            &mut numeric,
        );
        assert_eq!(result.cosine.to_bits(), 0x3f80_0000);
        assert!(!result.convex);
        assert_eq!(numeric.reductions.len(), 3);
        assert!(numeric.seed_values.is_empty());

        let boundary = f32::from_bits(0x2000_0000);
        let mut numeric = RecordingNumeric::default();
        let result = compute_edge_cos(
            [0.0, 0.0, 0.0],
            [boundary, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            &mut numeric,
        );
        assert_eq!(
            numeric.reductions[1].0.map(f32::to_bits),
            [0, 0, boundary.to_bits()]
        );
        assert_eq!(
            numeric.reductions[1].0[2]
                .mul_add(numeric.reductions[1].1[2], 0.0)
                .to_bits(),
            EDGE_COS_MINIMUM_NORMAL_SQUARED.to_bits()
        );
        assert_eq!(result.cosine.to_bits(), 0x3f80_0000);
        assert!(numeric.seed_values.is_empty());

        let mut queued = QueuedNumeric {
            reductions: vec![1.0, EDGE_COS_MINIMUM_NORMAL_SQUARED, 1.0].into_iter(),
            seed_calls: 0,
        };
        let result = compute_edge_cos([0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3], &mut queued);
        assert!(result.convex);
        assert_eq!(result.cosine.to_bits(), 0x3f80_0000);
        assert_eq!(queued.seed_calls, 0);
    }

    #[test]
    fn compute_edge_cos_reduces_numerator_then_requests_both_seeds() {
        let mut numeric = RecordingNumeric::default();
        let result = compute_edge_cos(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, -1.0],
            &mut numeric,
        );
        assert_eq!(numeric.reductions.len(), 4);
        assert_eq!(numeric.seed_values.len(), 2);
        assert_eq!(numeric.seed_values[0].to_bits(), 0x3f80_0000);
        assert_eq!(numeric.seed_values[1].to_bits(), 0x4040_0000);
        assert!(result.convex);
    }

    #[test]
    fn compute_edge_cos_nonideal_seed_freezes_both_two_round_refinements() {
        let mut numeric = FixedEstimateNumeric;
        let result = compute_edge_cos(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, -1.0],
            &mut numeric,
        );
        assert!(result.convex);
        assert_eq!(result.cosine.to_bits(), 0x3f0d_ac88);
    }

    #[test]
    fn source_cross_retains_rounded_multiply_then_fused_subtract() {
        let left = [
            f32::from_bits(0),
            f32::from_bits(0x4443_8772),
            f32::from_bits(0x4443_dc25),
        ];
        let right = [
            f32::from_bits(0),
            f32::from_bits(0xc41b_05d6),
            f32::from_bits(0xc432_c985),
        ];
        assert_eq!(source_cross(left, right)[0].to_bits(), 0xc78f_9ab5);
    }

    #[test]
    fn serialized_edge_decode_preserves_signed_shift_boundary_and_golden_bits() {
        let cases = [
            (0, 0xbe6f_4f38),
            (1, 0x3ec4_2c32),
            (2, 0x3f31_0b0c),
            (3, 0x3f58_8586),
            (13, 0x3f7f_f621),
            (27, 0x3f80_0000),
            (28, 0x3f80_0000),
        ];
        for (exponent, expected) in cases {
            assert_eq!(decode_serialized_edge(exponent).to_bits(), expected);
        }
        assert_eq!(serialized_edge_denominator(27).to_bits(), 0x4e80_0000);
        assert_eq!(serialized_edge_denominator(28).to_bits(), 0xcf00_0000);
        assert_eq!(serialized_edge_denominator(29).to_bits(), 0x0000_0000);
    }

    #[test]
    fn explicit_triangle_restores_slots_flags_tags_and_source_identity() {
        let batch = materialize_unit_topology(
            7,
            0x10,
            &unit(UnitType::Triangle, 0x21, &[0, 1, 2], &[0x60, 0x21, 0x42]),
            &vertices(),
            None,
        )
        .unwrap();
        let result = records(&batch);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].points, vertices()[..3]);
        assert_eq!(
            result[0].edge_data.map(f32::to_bits),
            [0xbe6f_4f38, 0x3ec4_2c32, 0x3f31_0b0c]
        );
        assert_eq!(result[0].flags, 0x0b71);
        assert_eq!(result[0].volume_tag, 0);
        assert_eq!(result[0].user_tag, 0xabcd_1234);
        assert_eq!(
            result[0].source,
            AssemblyGroundTriangleSource {
                cluster_index: 7,
                unit_stream_offset: 0x34,
                primitive_in_unit: 0,
            }
        );
    }

    #[test]
    fn explicit_triangle_flag_roles_and_cluster_mask_are_exact() {
        for (edge_index, bit, expected) in [
            (0, 0x20, 0x121),
            (1, 0x20, 0x141),
            (2, 0x20, 0x181),
            (0, 0x40, 0x301),
            (1, 0x40, 0x501),
            (2, 0x40, 0x901),
        ] {
            let mut edges = [0_u8; 3];
            edges[edge_index] = bit;
            let record = records(
                &materialize_unit_topology(
                    0,
                    0,
                    &unit(UnitType::Triangle, 0x20, &[0, 1, 2], &edges),
                    &vertices(),
                    None,
                )
                .unwrap(),
            )[0];
            assert_eq!(record.flags, expected);
        }

        let plain = pack_explicit_edge_flags([0; 3], 0);
        assert_eq!(pack_explicit_edge_flags([0; 3], 0x10), plain | 0x10);
        assert_eq!(pack_explicit_edge_flags([0; 3], 0xffff), plain | 0x10);
    }

    #[test]
    fn quad_one_hot_flags_discriminate_every_external_and_synthetic_slot() {
        let expectations = [
            (
                0x20,
                [
                    [0x121, 0x101],
                    [0x101, 0x181],
                    [0x181, 0x101],
                    [0x101, 0x121],
                ],
            ),
            (
                0x40,
                [
                    [0x301, 0x101],
                    [0x501, 0x901],
                    [0x901, 0x501],
                    [0x101, 0x301],
                ],
            ),
        ];
        for (bit, expected_by_edge) in expectations {
            for (edge_index, expected) in expected_by_edge.into_iter().enumerate() {
                let mut edges = [0_u8; 4];
                edges[edge_index] = bit;
                let result = records(
                    &materialize_unit_topology(
                        0,
                        0,
                        &unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &edges),
                        &vertices(),
                        Some(QuadDiagonal {
                            cosine: 1.0,
                            convex: false,
                        }),
                    )
                    .unwrap(),
                );
                assert_eq!([result[0].flags, result[1].flags], expected);
            }
        }

        let result = records(
            &materialize_unit_topology(
                0,
                0,
                &unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &[0; 4]),
                &vertices(),
                Some(QuadDiagonal {
                    cosine: 1.0,
                    convex: true,
                }),
            )
            .unwrap(),
        );
        assert_eq!([result[0].flags, result[1].flags], [0x141, 0x141]);
    }

    #[test]
    fn no_edge_route_uses_literal_defaults_and_ignores_cluster_flags() {
        for (unit_type, indices) in [
            (UnitType::Triangle, &[0, 1, 2][..]),
            (UnitType::Quad, &[0, 1, 2, 3][..]),
        ] {
            for cluster_flags in [0, 0x10, 0xffff] {
                let result = records(
                    &materialize_unit_topology(
                        0,
                        cluster_flags,
                        &unit(unit_type, unit_type as u8, indices, &[]),
                        &vertices(),
                        None,
                    )
                    .unwrap(),
                );
                for record in result {
                    assert_eq!(record.flags, FLAG_TRIANGLE_DEFAULT);
                    assert_eq!(record.edge_data.map(f32::to_bits), [0; 3]);
                }
            }
        }
    }

    #[test]
    fn quad_mapping_shared_diagonal_and_synthetic_flags_are_exact() {
        let source = unit(
            UnitType::Quad,
            0x22,
            &[0, 1, 2, 3],
            &[0x60, 0x41, 0x62, 0x23],
        );
        for (convex, expected_flags) in [(true, [0x0ff1, 0x0d71]), (false, [0x0fb1, 0x0d31])] {
            let result = records(
                &materialize_unit_topology(
                    9,
                    0x10,
                    &source,
                    &vertices(),
                    Some(QuadDiagonal {
                        cosine: f32::from_bits(0x3f12_3456),
                        convex,
                    }),
                )
                .unwrap(),
            );
            assert_eq!(
                result[0].points,
                [vertices()[0], vertices()[1], vertices()[2]]
            );
            assert_eq!(
                result[1].points,
                [vertices()[3], vertices()[2], vertices()[1]]
            );
            assert_eq!(result[0].flags, expected_flags[0]);
            assert_eq!(result[1].flags, expected_flags[1]);
            assert_eq!(result[0].edge_data[1].to_bits(), 0x3f12_3456);
            assert_eq!(result[1].edge_data[1].to_bits(), 0x3f12_3456);
            assert_eq!(result[0].edge_data[0].to_bits(), 0xbe6f_4f38);
            assert_eq!(result[0].edge_data[2].to_bits(), 0x3f31_0b0c);
            assert_eq!(result[1].edge_data[0].to_bits(), 0x3f58_8586);
            assert_eq!(result[1].edge_data[2].to_bits(), 0x3ec4_2c32);
            assert_eq!(result[0].source.primitive_in_unit, 0);
            assert_eq!(result[1].source.primitive_in_unit, 1);
            assert_eq!(result[0].user_tag, result[1].user_tag);
        }
    }

    #[test]
    fn native_count_dispatch_keeps_old_and_empty_list_inert_and_quarantines_long_lists() {
        let old = unit(UnitType::OldTriangle, 0, &[0, 1, 2], &[]);
        assert_eq!(
            materialize_unit_topology(0, 0, &old, &vertices(), None),
            Ok(UnitTopologyBatch::empty())
        );
        let empty = unit(UnitType::TriangleList, 3, &[0, 1], &[]);
        assert_eq!(
            materialize_unit_topology(0, 0, &empty, &vertices(), None),
            Ok(UnitTopologyBatch::empty())
        );
        let long = unit(UnitType::TriangleList, 3, &[0, 1, 2, 3, 0], &[]);
        assert_eq!(
            materialize_unit_topology(0, 0, &long, &vertices(), None),
            Err(UnitTopologyError::UnsupportedTriangleListCount { count: 3 })
        );

        let triangle = unit(UnitType::Triangle, 0x21, &[0, 1, 2], &[0x60, 0x21, 0x42]);
        let list_one = unit(
            UnitType::TriangleList,
            0x23,
            &[0, 1, 2],
            &[0x60, 0x21, 0x42],
        );
        assert_eq!(
            materialize_unit_topology(2, 0x10, &triangle, &vertices(), None),
            materialize_unit_topology(2, 0x10, &list_one, &vertices(), None)
        );

        let quad = unit(
            UnitType::Quad,
            0x22,
            &[0, 1, 2, 3],
            &[0x60, 0x41, 0x62, 0x23],
        );
        let list_two = unit(
            UnitType::TriangleList,
            0x23,
            &[0, 1, 2, 3],
            &[0x60, 0x41, 0x62, 0x23],
        );
        let diagonal = Some(QuadDiagonal {
            cosine: 0.75,
            convex: true,
        });
        assert_eq!(
            materialize_unit_topology(2, 0x10, &quad, &vertices(), diagonal),
            materialize_unit_topology(2, 0x10, &list_two, &vertices(), diagonal)
        );
    }

    #[test]
    fn direct_unit_ids_pack_only_present_tag_halves() {
        let mut source = unit(UnitType::Triangle, 1, &[0, 1, 2], &[]);
        source.group_id = None;
        source.surface_id = None;
        assert_eq!(
            records(&materialize_unit_topology(0, 0, &source, &vertices(), None).unwrap())[0]
                .user_tag,
            0
        );
        source.group_id = Some(0x1234);
        assert_eq!(
            records(&materialize_unit_topology(0, 0, &source, &vertices(), None).unwrap())[0]
                .user_tag,
            0x0000_1234
        );
        source.group_id = None;
        source.surface_id = Some(0xabcd);
        assert_eq!(
            records(&materialize_unit_topology(0, 0, &source, &vertices(), None).unwrap())[0]
                .user_tag,
            0xabcd_0000
        );
    }

    #[test]
    fn old_assembly_dispatch_surfaces_native_asserted_counts_before_numeric_work() {
        let mesh = clustered_mesh(1.0, 0);
        let cluster = uncompressed_cluster();
        for (source, count) in [
            (unit(UnitType::OldTriangle, 0, &[255, 255, 255], &[]), 0),
            (unit(UnitType::TriangleList, 3, &[255, 255], &[]), 0),
            (
                unit(UnitType::TriangleList, 3, &[255, 255, 255, 255, 255], &[]),
                3,
            ),
        ] {
            let mut numeric = RecordingNumeric::default();
            assert_eq!(
                build_old_assembly_unit_gp_with_numeric(
                    &mesh,
                    0,
                    &cluster,
                    &source,
                    IDENTITY,
                    0,
                    &mut numeric,
                ),
                Err(UnitTopologyError::AssemblyNativeAssertedPrimitiveCount {
                    unit_type: source.unit_type,
                    count,
                })
            );
            assert!(numeric.events.is_empty());
        }
    }

    #[test]
    fn old_assembly_type_and_list_dispatch_share_supported_arms() {
        let mesh = clustered_mesh(1.0, 0x10);
        let cluster = uncompressed_cluster();
        for (direct, list) in [
            (
                unit(UnitType::Triangle, 0x21, &[0, 1, 2], &[0x60, 0x21, 0x42]),
                unit(
                    UnitType::TriangleList,
                    0x23,
                    &[0, 1, 2],
                    &[0x60, 0x21, 0x42],
                ),
            ),
            (
                unit(
                    UnitType::Quad,
                    0x22,
                    &[0, 1, 2, 3],
                    &[0x60, 0x41, 0x62, 0x23],
                ),
                unit(
                    UnitType::TriangleList,
                    0x23,
                    &[0, 1, 2, 3],
                    &[0x60, 0x41, 0x62, 0x23],
                ),
            ),
        ] {
            let direct =
                build_old_assembly_unit_gp(&mesh, 3, &cluster, &direct, IDENTITY, 0x7654_3210)
                    .unwrap();
            let list = build_old_assembly_unit_gp(&mesh, 3, &cluster, &list, IDENTITY, 0x7654_3210)
                .unwrap();
            assert_eq!(direct, list);
        }
    }

    #[test]
    fn old_assembly_has_no_local_query_cull_and_preserves_tags_and_source() {
        let mut mesh = clustered_mesh(1.0, 0);
        mesh.bounding_box_min = [100.0, 100.0, 100.0, 0.0];
        mesh.bounding_box_max = [101.0, 101.0, 101.0, 0.0];
        let cluster = uncompressed_cluster();
        let source = unit(UnitType::Quad, 2, &[0, 1, 2, 3], &[]);
        let mut far_transform = IDENTITY;
        far_transform[12] = 10_000.0;
        far_transform[13] = -20_000.0;
        let batch =
            build_old_assembly_unit_gp(&mesh, 7, &cluster, &source, far_transform, 0xfeed_beef)
                .unwrap();
        let records = batch.iter().copied().collect::<Vec<_>>();

        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .all(|record| record.primitive().points[0][0] >= 10_000.0)
        );
        assert!(
            records
                .iter()
                .all(|record| record.volume_tag() == 0xfeed_beef)
        );
        assert!(
            records
                .iter()
                .all(|record| record.user_tag() == 0xabcd_1234)
        );
        assert_eq!(records[0].source().cluster_index(), 7);
        assert_eq!(records[0].source().unit_stream_offset(), 0x34);
        assert_eq!(records[0].source().primitive_in_unit(), 0);
        assert_eq!(records[1].source().primitive_in_unit(), 1);
    }

    #[test]
    fn old_assembly_explicit_quad_scalar_calls_are_e2_e3_e0_e1() {
        let mut old_calls = Vec::new();
        let old =
            decode_explicit_quad_edges([10, 20, 30, 40], UnitGpOverload::OldAssemblyFree, |edge| {
                old_calls.push(edge);
                f32::from(edge)
            });
        assert_eq!(old_calls, [30, 40, 10, 20]);
        assert_eq!(
            old.map(f32::to_bits),
            [10.0, 20.0, 30.0, 40.0].map(f32::to_bits)
        );

        let mut new_calls = Vec::new();
        let new = decode_explicit_quad_edges([10, 20, 30, 40], UnitGpOverload::NewMember, |edge| {
            new_calls.push(edge);
            f32::from(edge)
        });
        assert_eq!(new_calls, [40, 10, 30, 20]);
        assert_eq!(new.map(f32::to_bits), old.map(f32::to_bits));

        let mesh = clustered_mesh(1.0, 0);
        let cluster = uncompressed_cluster();
        let mut numeric = RecordingNumeric::default();
        build_old_assembly_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &[10, 20, 30, 40]),
            IDENTITY,
            0,
            &mut numeric,
        )
        .unwrap();
        assert_eq!(numeric.edge_scalars, [30, 40, 10, 20]);

        let mut numeric = RecordingNumeric::default();
        build_old_assembly_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &unit(UnitType::Triangle, 0x21, &[0, 1, 2], &[10, 20, 30]),
            IDENTITY,
            0,
            &mut numeric,
        )
        .unwrap();
        assert_eq!(numeric.edge_scalars, [10, 30, 20]);
    }

    #[test]
    fn both_overloads_pass_serialized_quad_order_to_diagonal_computation() {
        let points = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, -1.0],
        ];
        let source = unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &[0; 4]);
        let mesh = clustered_mesh(1.0, 0);
        let cluster = cluster(
            points
                .into_iter()
                .map(|[x, y, z]| ClusterVertex::Uncompressed([x, y, z, 0.0]))
                .collect(),
            VertexCompression::Uncompressed,
        );

        let expected_diagonal_reductions = [
            ([-1.0, 1.0, 0.0], [-1.0, 1.0, 0.0]),
            ([0.0, 0.0, 1.0], [0.0, 0.0, 1.0]),
            ([1.0, 1.0, 1.0], [1.0, 1.0, 1.0]),
            ([0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
        ];
        let mut old_numeric = RecordingNumeric::default();
        let old = build_old_assembly_unit_gp_with_numeric(
            &mesh,
            0,
            &cluster,
            &source,
            IDENTITY,
            0,
            &mut old_numeric,
        )
        .unwrap();
        assert_eq!(old_numeric.reductions[2..6], expected_diagonal_reductions);
        assert_eq!(
            old.iter()
                .map(|record| record.primitive.flags)
                .collect::<Vec<_>>(),
            [0x141, 0x141]
        );

        let mut numeric = RecordingNumeric::default();
        let new = build_unit_gp_with_numeric(
            0,
            0,
            &source,
            &points,
            broad_query(),
            IDENTITY,
            &mut numeric,
        )
        .unwrap();
        assert_eq!(numeric.reductions[2..6], expected_diagonal_reductions);
        assert_eq!(
            new.iter()
                .map(|record| record.primitive.flags)
                .collect::<Vec<_>>(),
            [0x141, 0x141]
        );
    }

    #[test]
    fn old_assembly_inherits_only_cluster_one_sided_flag() {
        let mesh = clustered_mesh(1.0, 0xffff);
        let cluster = uncompressed_cluster();
        let source = unit(UnitType::Triangle, 0x21, &[0, 1, 2], &[0; 3]);
        let record = build_old_assembly_unit_gp(&mesh, 0, &cluster, &source, IDENTITY, 0)
            .unwrap()
            .iter()
            .next()
            .copied()
            .unwrap();
        assert_eq!(record.primitive().flags, 0x111);
    }

    #[test]
    fn old_assembly_resolves_vertices_in_serialized_index_order() {
        let mesh = clustered_mesh(1.0, 0);
        let cluster = cluster(
            vec![
                ClusterVertex::Uncompressed([10.0, 0.0, 0.0, 0.0]),
                ClusterVertex::Uncompressed([20.0, 1.0, 0.0, 0.0]),
                ClusterVertex::Uncompressed([30.0, 2.0, 0.0, 0.0]),
                ClusterVertex::Uncompressed([40.0, 4.0, 1.0, 0.0]),
            ],
            VertexCompression::Uncompressed,
        );
        let source = unit(UnitType::Quad, 2, &[2, 0, 3, 1], &[]);
        let batch = build_old_assembly_unit_gp(&mesh, 4, &cluster, &source, IDENTITY, 0).unwrap();
        let records = batch.iter().collect::<Vec<_>>();
        assert_eq!(
            records[0].primitive().points,
            [[30.0, 2.0, 0.0], [10.0, 0.0, 0.0], [40.0, 4.0, 1.0],]
        );
        assert_eq!(
            records[1].primitive().points,
            [[20.0, 1.0, 0.0], [40.0, 4.0, 1.0], [10.0, 0.0, 0.0],]
        );
        assert_eq!(records[0].source().primitive_in_unit(), 0);
        assert_eq!(records[1].source().primitive_in_unit(), 1);
    }

    #[test]
    fn old_assembly_expands_u16_and_s32_vertices_before_affine_transform() {
        let u16_mesh = clustered_mesh(0.5, 0);
        let u16_cluster = cluster(
            vec![
                ClusterVertex::Unsigned16 {
                    quantized_origin: [i32::MAX - 1, -5, 10],
                    value: [10, 2, 4],
                },
                ClusterVertex::Unsigned16 {
                    quantized_origin: [i32::MAX - 1, -5, 10],
                    value: [9, 4, 4],
                },
                ClusterVertex::Unsigned16 {
                    quantized_origin: [i32::MAX - 1, -5, 10],
                    value: [8, 2, 6],
                },
            ],
            VertexCompression::Unsigned16,
        );
        let source = unit(UnitType::Triangle, 0, &[0, 1, 2], &[]);
        let u16_record =
            build_old_assembly_unit_gp(&u16_mesh, 0, &u16_cluster, &source, IDENTITY, 0)
                .unwrap()
                .iter()
                .next()
                .copied()
                .unwrap();
        assert_eq!(
            u16_record
                .primitive()
                .points
                .map(|point| point.map(f32::to_bits)),
            [
                [0x4e80_0000, 0xbfc0_0000, 0x40e0_0000],
                [0x4e80_0000, 0xbf00_0000, 0x40e0_0000],
                [0x4e80_0000, 0xbfc0_0000, 0x4100_0000],
            ]
        );

        let s32_mesh = clustered_mesh(0.25, 0);
        let s32_cluster = cluster(
            vec![
                ClusterVertex::Signed32([3, -4, 5]),
                ClusterVertex::Signed32([4, -4, 5]),
                ClusterVertex::Signed32([3, -3, 5]),
            ],
            VertexCompression::Signed32,
        );
        let mut transform = IDENTITY;
        transform[12] = 2.0;
        transform[13] = 3.0;
        transform[14] = -1.0;
        let s32_record =
            build_old_assembly_unit_gp(&s32_mesh, 0, &s32_cluster, &source, transform, 0)
                .unwrap()
                .iter()
                .next()
                .copied()
                .unwrap();
        assert_eq!(
            s32_record.primitive().points[0].map(f32::to_bits),
            [0x4030_0000, 0x4000_0000, 0x3e80_0000]
        );
    }

    #[test]
    fn malformed_host_values_refuse_before_publishing_a_partial_batch() {
        let missing_edge = unit(UnitType::Triangle, 0x21, &[0, 1, 2], &[0, 1]);
        assert_eq!(
            materialize_unit_topology(0, 0, &missing_edge, &vertices(), None),
            Err(UnitTopologyError::InvalidEdgeCount {
                actual: 2,
                expected: 3,
            })
        );
        let bad_quad = unit(UnitType::Quad, 0x22, &[0, 1, 2, 9], &[0; 4]);
        assert_eq!(
            materialize_unit_topology(
                0,
                0,
                &bad_quad,
                &vertices(),
                Some(QuadDiagonal {
                    cosine: 1.0,
                    convex: false,
                }),
            ),
            Err(UnitTopologyError::VertexOutsideCluster { index: 9 })
        );
        let missing_diagonal = unit(UnitType::Quad, 0x22, &[0, 1, 2, 3], &[0; 4]);
        assert_eq!(
            materialize_unit_topology(0, 0, &missing_diagonal, &vertices(), None),
            Err(UnitTopologyError::MissingQuadDiagonal)
        );

        let mesh = clustered_mesh(1.0, 0);
        let cluster = uncompressed_cluster();
        let mut numeric = RecordingNumeric::default();
        assert_eq!(
            build_old_assembly_unit_gp_with_numeric(
                &mesh,
                0,
                &cluster,
                &unit(UnitType::Triangle, 1, &[0, 1, 9], &[]),
                IDENTITY,
                0,
                &mut numeric,
            ),
            Err(UnitTopologyError::VertexOutsideCluster { index: 9 })
        );
        assert!(numeric.events.is_empty());
    }
}
