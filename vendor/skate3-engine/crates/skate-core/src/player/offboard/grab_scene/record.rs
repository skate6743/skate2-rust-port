//!82585DD8/82585F58 authored spline initialization, not a raw guest pointer.
use super::{Descriptor, Frame, Record, Vector, math::*};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub id: u32,
    pub points: Vec<Vector>,
    pub approach_vectors: Vec<Vector>,
    pub word_60: u32,
}

///Actual first-part data. None is native assembly with no first part; the
///native default coefficients apply only to that proven branch.
#[derive(Clone, Debug)]
pub struct AssemblyData {
    pub identity: u32,
    pub first_part: Option<PartData>,
}
#[derive(Clone, Debug)]
pub struct PartData {
    pub identity: u32,
    pub rates: Option<RatesData>,
    pub coefficients_0_to_36: [f32; 10],
}
#[derive(Clone, Debug)]
pub struct RatesData {
    pub identity: u32,
    pub vector_48: Vector,
    pub transform_position_48: Vector,
}
pub struct RecordInput<'a> {
    pub descriptor: Descriptor,
    pub geometry: Arc<Geometry>,
    pub frame: Frame,
    pub object_vector_128: Vector,
    pub assembly: Option<&'a AssemblyData>,
    pub word_272: u32,
}
impl Record {
    pub fn from_geometry(i: RecordInput<'_>) -> Result<Self, &'static str> {
        if i.geometry.id == 0
            || i.geometry.points.is_empty()
            || i.geometry.points.len() > 255
            || i.geometry.approach_vectors.len() > 255
        {
            return Err("Grab geometry needs a real identity and native byte-sized point counts");
        }
        if i.geometry
            .points
            .iter()
            .chain(&i.geometry.approach_vectors)
            .flatten()
            .chain(i.frame.iter().flatten())
            .chain(i.object_vector_128.iter())
            .any(|v| !v.is_finite())
        {
            return Err("Nonfinite authored grab geometry/frame");
        }
        let mut r = Self([0; 72], i.geometry);
        for j in 0..4 {
            r.set_vector(j * 16, i.frame[j]);
        }
        r.0[47] = i.descriptor.kind;
        r.0[48] = i.descriptor.id;
        r.0[49] = r.1.id;
        r.0[50] = 0x80000000;
        r.0[51] = r.1.word_60;
        r.0[68] = i.word_272;
        let start = point(i.frame, r.1.points[0]);
        let end = point(i.frame, *r.1.points.last().unwrap());
        let approach =
            r.1.approach_vectors
                .first()
                .copied()
                .unwrap_or([0., 1., 0., 0.]);
        r.set_vector(64, start);
        r.set_vector(80, end);
        r.set_vector(96, direction(i.frame, approach));
        r.set_vector(128, i.object_vector_128);
        r.set_vector(224, [1.; 4]);
        for (offset, value) in [(240, 1.), (244, 1.), (248, f32::MAX), (252, f32::MAX)] {
            r.0[offset / 4] = value.to_bits();
        }
        if let Some(a) = i.assembly {
            if a.identity == 0 {
                return Err("Zero assembly identity is reserved for native null");
            }
            r.0[52] = a.identity;
            if let Some(p) = &a.first_part {
                if p.identity == 0 {
                    return Err("Zero part identity is reserved for native null");
                }
                r.0[53] = p.identity;
                for j in 0..10 {
                    r.0[56 + j] = p.coefficients_0_to_36[j].to_bits();
                }
                if let Some(rates) = &p.rates {
                    if rates.identity == 0 {
                        return Err("Zero rates identity is reserved for native null");
                    }
                    r.0[54] = rates.identity;
                    r.0[50] |= 0x40000000;
                    r.set_vector(144, rates.vector_48);
                    r.set_vector(160, rates.transform_position_48);
                }
            }
        }
        let delta = sub(end, start);
        let straight = length(delta);
        let inv = reciprocal(straight);
        r.set_vector(112, delta.map(|v| v * inv));
        r.0[45] = (straight * 0.5).to_bits();
        let mut total = 0.0;
        for pair in r.1.points.windows(2) {
            total += length(sub(pair[1], pair[0]));
        }
        r.0[44] = total.to_bits();
        Ok(r)
    }
    pub fn set_vector(&mut self, offset: usize, value: Vector) {
        self.0[offset / 4..offset / 4 + 4].copy_from_slice(&value.map(f32::to_bits));
    }
}
