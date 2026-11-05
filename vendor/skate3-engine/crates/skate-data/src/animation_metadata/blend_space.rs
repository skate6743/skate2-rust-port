//! Andale type6: construction82D1B8D0, layout82D22DA0, simplex82D23898.
use super::validate_name;
use crate::abin::{Error, Reader, RecordHeader, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlendSimplexMetadata {
    pub children: Vec<usize>,
    /// Spatial coordinates only; the serialized matrix also has a final column.
    pub vertex_bits: Vec<Vec<u32>>,
    pub normal_bits: Vec<Vec<u32>>,
    pub scale_bits: Vec<u32>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlendSpaceMetadata {
    pub name: String,
    pub source_offset: u64,
    pub parameters: Vec<String>,
    pub children: Vec<String>,
    pub simplexes: Vec<BlendSimplexMetadata>,
}
impl BlendSpaceMetadata {
    pub(super) fn validate(&self, bytes: u64) -> std::result::Result<(), String> {
        validate_name(&self.name, 36)?;
        let d = self.parameters.len();
        if self.source_offset >= bytes
            || d == 0
            || d > 4
            || self.children.len() < d + 1
            || self.simplexes.is_empty()
        {
            return Err(format!(
                "{}: invalid BlendSpace dimensions/offset",
                self.name
            ));
        }
        for name in &self.parameters {
            validate_name(name, 30)?;
        }
        for name in &self.children {
            validate_name(name, 36)?;
        }
        for s in &self.simplexes {
            if s.children.len() != d + 1
                || s.scale_bits.len() != d + 1
                || s.vertex_bits.len() != d + 1
                || s.normal_bits.len() != d + 1
                || s.children.iter().any(|&i| i >= self.children.len())
                || s.vertex_bits
                    .iter()
                    .chain(&s.normal_bits)
                    .any(|v| v.len() != d)
                || s.vertex_bits
                    .iter()
                    .chain(&s.normal_bits)
                    .flatten()
                    .chain(&s.scale_bits)
                    .any(|&v| !f32::from_bits(v).is_finite())
            {
                return Err(format!("{}: invalid BlendSpace simplex", self.name));
            }
        }
        Ok(())
    }
}
pub(super) fn read(r: Reader<'_>, h: &RecordHeader) -> Result<BlendSpaceMetadata> {
    let p = h.payload_offset;
    let count = r.u32(p)? as usize;
    let names_size = count
        .checked_mul(24)
        .ok_or_else(|| Error::new(p, "BlendSpace count overflow"))?;
    let names = r.relative(p, r.u32(p + 4)?, names_size)?.start;
    let q = r.relative(p, r.u32(p + 12)?, 44)?.start;
    let space = r.bounded(r.relative(q, 0, r.u32(q)? as usize)?)?;
    let d = space.u32(q + 28)? as usize;
    if d == 0 || d > 4 || d > r.u32(p + 16)? as usize {
        return Err(Error::new(q + 28, "invalid BlendSpace dimension"));
    }
    let params = space.relative(q, space.u32(q + 32)?, d * 20)?.start;
    let simplex_count = space.u32(q + 36)? as usize;
    // Every record must at least contain its header; bound the count before allocating.
    space.relative(
        q,
        space.u32(q + 40)?,
        simplex_count
            .checked_mul(20)
            .ok_or_else(|| Error::new(q, "BlendSpace simplex count overflow"))?,
    )?;
    let mut at = space.relative(q, space.u32(q + 40)?, 20)?.start;
    let mut simplexes = Vec::new();
    for _ in 0..simplex_count {
        let size = space.u32(at)? as usize;
        let s = space.bounded(space.relative(at, 0, size)?)?;
        if s.u32(at + 16)? as usize != d + 1 {
            return Err(Error::new(at + 16, "BlendSpace matrix dimension mismatch"));
        }
        s.take(at + 20, (d + 1) * (d + 1) * 4)?;
        let indices = s.relative(at, s.u32(at + 4)?, (d + 1) * 4)?.start;
        let scales = s.relative(at, s.u32(at + 8)?, (d + 1) * 4)?.start;
        let normals = s
            .relative(at, s.u32(at + 12)?, (d + 1) * (d + 1) * 4)?
            .start;
        let mut simplex = BlendSimplexMetadata {
            children: Vec::new(),
            vertex_bits: Vec::new(),
            normal_bits: Vec::new(),
            scale_bits: Vec::new(),
        };
        for i in 0..=d {
            let normal = normals + i * (d + 1) * 4;
            if s.u32(normal)? as usize != d {
                return Err(Error::new(normal, "BlendSpace normal dimension mismatch"));
            }
            simplex.children.push(s.u32(indices + i * 4)? as usize);
            simplex.scale_bits.push(s.u32(scales + i * 4)?);
            simplex.vertex_bits.push(
                (0..d)
                    .map(|j| s.u32(at + 20 + (i * (d + 1) + j) * 4))
                    .collect::<Result<_>>()?,
            );
            simplex.normal_bits.push(
                (0..d)
                    .map(|j| s.u32(normal + 4 + j * 4))
                    .collect::<Result<_>>()?,
            );
        }
        simplexes.push(simplex);
        at += size;
    }
    Ok(BlendSpaceMetadata {
        name: h.name.clone(),
        source_offset: h.offset as u64,
        parameters: (0..d)
            .map(|i| space.name(params + i * 20, 5))
            .collect::<Result<_>>()?,
        children: (0..count)
            .map(|i| r.name(names + i * 24, 6))
            .collect::<Result<_>>()?,
        simplexes,
    })
}
