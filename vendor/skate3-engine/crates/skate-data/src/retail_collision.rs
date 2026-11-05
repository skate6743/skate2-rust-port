//! Embedded RWCM v1 / RWCMSET1 archives. Cluster coordinates are already in
//! world space. Never weld them or replace their feature bytes with adjacency.
//! Layout reference: SK8R15 tools/vanilla_map_extraction/tools/retail_collision_mesh.py.
//! Its signed-16 interpretation is incorrect: native GetVertex82AC79D0
//! zero-extends the offsets before signed saturating addition to the base.

#[derive(Clone, Copy, Debug)]
pub struct RetailTriangle {
    pub points: [[f32; 3]; 3],
    pub edges: Option<[u8; 3]>,
    pub surface: u16,
    /// Unit flag 0x80: the unit carries a surface ID (`surface` is 0 otherwise).
    pub has_surface: bool,
    pub group: u16,
    pub one_sided: bool,
}

fn span(data: &[u8], at: usize, n: usize) -> Result<&[u8], String> {
    data.get(at..at.checked_add(n).ok_or("RWCM offset overflow")?)
        .ok_or_else(|| format!("Truncated RWCM at {at} (need {n})"))
}
fn be16(data: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_be_bytes(span(data, at, 2)?.try_into().unwrap()))
}
fn be32(data: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_be_bytes(span(data, at, 4)?.try_into().unwrap()))
}
fn le32(data: &[u8], at: &mut usize) -> Result<usize, String> {
    let value = u32::from_le_bytes(span(data, *at, 4)?.try_into().unwrap());
    *at += 4;
    Ok(value as usize)
}
fn id(data: &[u8], at: &mut usize, width: u8) -> Result<u16, String> {
    let value = match width {
        1 => u16::from(span(data, *at, 1)?[0]),
        2 => u16::from_le_bytes(span(data, *at, 2)?.try_into().unwrap()),
        _ => return Err(format!("Unsupported RWCM ID width {width}")),
    };
    *at += usize::from(width);
    Ok(value)
}

/// Visits complete clusters in archive order, retaining one decoded mesh at a
/// time. The callback can build its own spatial query index. Returns the number
/// of triangles visited.
///
/// Meshes in which no unit carries a surface ID are skipped. Across every
/// shipped district these are only 12-triangle boxes (zone/trigger/bounds
/// volumes); authored player positions lie inside some of them, e.g. the
/// SkateSchool area and DownTown's Kube Tower landing, so they are not solid.
/// Meshes that mix surfaced and surfaceless units are ordinary geometry and
/// are kept whole.
pub fn visit_clusters(
    data: &[u8],
    mut visit: impl FnMut(&str, &[RetailTriangle]) -> Result<(), String>,
) -> Result<usize, String> {
    if span(data, 0, 8)? != b"RWCMSET1" {
        return Err("Invalid RWCMSET1 magic".into());
    }
    let mut at = 8;
    let count = le32(data, &mut at)?;
    if count == 0 || count > data.len() / 105 {
        return Err("Invalid RWCM mesh count".into());
    }
    let mut total = 0;
    for _ in 0..count {
        let size = le32(data, &mut at)?;
        if size == 0 || size > 4096 {
            return Err("Invalid RWCM mesh name length".into());
        }
        let name =
            std::str::from_utf8(span(data, at, size)?).map_err(|_| "Invalid RWCM mesh name")?;
        at += size;
        let size = le32(data, &mut at)?;
        let mesh = span(data, at, size)?;
        at += size;
        total += mesh_clusters(mesh, &mut |triangles| visit(name, triangles))
            .map_err(|e| format!("RWCM mesh {name:?}: {e}"))?;
    }
    if at != data.len() {
        return Err("RWCM archive has trailing bytes".into());
    }
    Ok(total)
}

fn mesh_clusters(
    data: &[u8],
    visit: &mut impl FnMut(&[RetailTriangle]) -> Result<(), String>,
) -> Result<usize, String> {
    span(data, 0, 96)?;
    let size = be32(data, 80)? as usize;
    if size < 96 || size > data.len() {
        return Err("Invalid RWCM mesh size".into());
    }
    let data = &data[..size];
    let table = be32(data, 52)? as usize;
    let count = be32(data, 64)? as usize;
    let granularity = f32::from_bits(be32(data, 56)?);
    if !granularity.is_finite() || granularity <= 0. {
        return Err("Invalid RWCM granularity".into());
    }
    if table < 96 || count == 0 || count > data.len() / 4 {
        return Err("Invalid RWCM cluster table".into());
    }
    span(data, table, count * 4)?;
    let one_sided = be16(data, 60)? & 0x10 != 0;
    let mut previous_end = table + count * 4;
    let mut total = 0;
    let mut clusters = Vec::with_capacity(count);
    for i in 0..count {
        let offset = be32(data, table + i * 4)? as usize;
        if offset < previous_end {
            return Err("Overlapping RWCM clusters".into());
        }
        let size = be16(data, offset + 8)? as usize;
        if size < 16 {
            return Err("Invalid RWCM cluster size".into());
        }
        let cluster = span(data, offset, size)?;
        previous_end = offset + size;
        let triangles = decode_cluster(cluster, granularity, data[62], data[63], one_sided)?;
        total += triangles.len();
        clusters.push(triangles);
    }
    if total != be32(data, 40)? as usize {
        return Err("RWCM triangle count mismatch".into());
    }
    if !clusters.iter().flatten().any(|t| t.has_surface) {
        return Ok(0); // volume, not collision (see visit_clusters)
    }
    for triangles in &clusters {
        visit(triangles)?;
    }
    Ok(total)
}

fn decode_cluster(
    data: &[u8],
    granularity: f32,
    group_width: u8,
    surface_width: u8,
    one_sided: bool,
) -> Result<Vec<RetailTriangle>, String> {
    let unit_count = usize::from(be16(data, 0)?);
    let unit_start = (usize::from(be16(data, 4)?) + 1) * 16;
    let units = span(data, unit_start, usize::from(be16(data, 2)?))?;
    let vertex_data = span(data, 0, unit_start)?;
    let mut vertices = Vec::with_capacity(usize::from(data[10]));
    for i in 0..usize::from(data[10]) {
        let mut p = [0.; 3];
        for axis in 0..3 {
            p[axis] = match data[12] {
                0 => f32::from_bits(be32(vertex_data, 16 + i * 16 + axis * 4)?),
                1 => {
                    let base = be32(vertex_data, 16 + axis * 4)? as i32;
                    // vmrghh with a zero vector produces unsigned halfwords;
                    // vaddsws then adds these to the signed base with saturation.
                    let delta = be16(vertex_data, 28 + i * 6 + axis * 2)?;
                    let integer = base.saturating_add(i32::from(delta));
                    // Float conversion precedes multiplication in GetVertex.
                    integer as f32 * granularity
                }
                2 => (be32(vertex_data, 16 + i * 12 + axis * 4)? as i32) as f32 * granularity,
                mode => return Err(format!("Unsupported RWCM vertex compression {mode}")),
            };
            if !p[axis].is_finite() {
                return Err("Non-finite RWCM vertex".into());
            }
        }
        vertices.push(p);
    }
    let mut at = 0;
    let mut triangles = Vec::new();
    for _ in 0..unit_count {
        let flags = span(units, at, 1)?[0];
        at += 1;
        if flags & 15 != 1 {
            return Err(format!(
                "Unsupported RWCM unit type {}; triangle units are required",
                flags & 15
            ));
        }
        let indices = span(units, at, 3)?;
        at += 3;
        let mut points = [[0.; 3]; 3];
        for i in 0..3 {
            points[i] = *vertices
                .get(usize::from(indices[i]))
                .ok_or("RWCM missing vertex")?;
        }
        let edges = if flags & 0x20 != 0 {
            let e = span(units, at, 3)?.try_into().unwrap();
            at += 3;
            Some(e)
        } else {
            None
        };
        let group = if flags & 0x40 != 0 {
            id(units, &mut at, group_width)?
        } else {
            0
        };
        let surface = if flags & 0x80 != 0 {
            id(units, &mut at, surface_width)?
        } else {
            0
        };
        triangles.push(RetailTriangle {
            points,
            edges,
            surface,
            has_surface: flags & 0x80 != 0,
            group,
            one_sided,
        });
    }
    if at != units.len() {
        return Err("RWCM unit stream has trailing bytes".into());
    }
    Ok(triangles)
}
