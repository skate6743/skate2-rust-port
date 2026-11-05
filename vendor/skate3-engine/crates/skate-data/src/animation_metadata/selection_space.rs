//! Type11 layout from original 82D1BEA0, 82D26CC0 and 82D26FF8.
use super::validate_name;
use crate::abin::{Error, Reader, RecordHeader, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionParameterMetadata {
    pub name: String,
    pub mode: u32,
    pub weight_bits: u32,
    pub minimum_bits: u32,
    pub maximum_bits: u32,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionCandidateMetadata {
    pub child: String,
    pub value_bits: Vec<u32>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionSpaceMetadata {
    pub name: String,
    pub source_offset: u64,
    pub parameters: Vec<SelectionParameterMetadata>,
    pub candidates: Vec<SelectionCandidateMetadata>,
}
impl SelectionSpaceMetadata {
    pub(super) fn validate(&self, bytes: u64) -> std::result::Result<(), String> {
        validate_name(&self.name, 36)?;
        if self.source_offset >= bytes || self.parameters.len() > 10 || self.candidates.is_empty() {
            return Err(format!("{}: invalid selection space dimensions", self.name));
        }
        for p in &self.parameters {
            validate_name(&p.name, 30)?;
            if [p.weight_bits, p.minimum_bits, p.maximum_bits]
                .into_iter()
                .any(|v| !f32::from_bits(v).is_finite())
            {
                return Err(format!("{}: nonfinite selection metric", self.name));
            }
        }
        for c in &self.candidates {
            validate_name(&c.child, 36)?;
            if c.value_bits.len() != self.parameters.len()
                || c.value_bits.iter().any(|v| !f32::from_bits(*v).is_finite())
            {
                return Err(format!("{}: invalid selection candidate", self.name));
            }
        }
        Ok(())
    }
}
pub(super) fn read(r: Reader<'_>, h: &RecordHeader) -> Result<SelectionSpaceMetadata> {
    let p = h.payload_offset;
    let count = r.u32(p)? as usize;
    if count > 10 {
        return Err(Error::new(
            p,
            "selection parameter array exceeds native capacity",
        ));
    }
    let parameters_at = r.relative(p, r.u32(p + 4)?, count * 40)?.start;
    let candidates_count = r.u32(p + 8)? as usize;
    let candidates_at = r.relative(p, r.u32(p + 12)?, 4)?.start;
    let stride = r.u32(candidates_at)? as usize;
    let size = stride
        .checked_mul(candidates_count)
        .ok_or_else(|| Error::new(p, "selection candidate size overflow"))?;
    r.take(candidates_at, size)?;
    if stride < 32 {
        return Err(Error::new(
            candidates_at,
            "selection candidate header too small",
        ));
    }
    let parameters = (0..count)
        .map(|i| {
            let at = parameters_at + i * 40;
            Ok(SelectionParameterMetadata {
                name: r.name(at, 6)?,
                mode: r.u32(at + 24)?,
                weight_bits: r.u32(at + 28)?,
                minimum_bits: r.u32(at + 32)?,
                maximum_bits: r.u32(at + 36)?,
            })
        })
        .collect::<Result<_>>()?;
    let candidates = (0..candidates_count)
        .map(|i| {
            let at = candidates_at + i * stride;
            let child_reader = r.bounded(at..at + stride)?;
            let values = child_reader
                .relative(at, child_reader.u32(at + 28)?, count * 4)?
                .start;
            Ok(SelectionCandidateMetadata {
                child: child_reader.name(at + 4, 6)?,
                value_bits: (0..count)
                    .map(|j| child_reader.u32(values + j * 4))
                    .collect::<Result<_>>()?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(SelectionSpaceMetadata {
        name: h.name.clone(),
        source_offset: h.offset as u64,
        parameters,
        candidates,
    })
}
