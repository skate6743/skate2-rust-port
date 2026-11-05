//! PhaseBlend attribute intersection82D161B0 and Scale/AddWeighted82D16370/428.
use super::output::attributes::{AnimationAttribute, AttributeName};

/// Bone-name mapping supplied by the stock hierarchy. A native excluded bone
/// (mirror index -1) maps to itself; missing names remain a data error.
#[derive(Clone, Debug)]
pub struct AttributeMirror(pub Vec<(AttributeName, AttributeName)>);
impl AttributeMirror {
    /// Attribute::Mirror82530F48 changes payload identity, never the event key.
    pub fn apply(&self, attribute: &mut AnimationAttribute) -> Result<(), String> {
        match attribute.kind {
            1 => {
                let x = attribute.payload.0[0].ok_or("Uninitialized mirrored vector attribute")?;
                attribute.payload.0[0] = Some(x ^ 0x8000_0000);
            }
            3 => {
                let mut name = [0; 5];
                for (i, word) in name.iter_mut().enumerate() {
                    *word =
                        attribute.payload.0[i].ok_or("Uninitialized mirrored bone reference")?;
                }
                let mirrored = self
                    .0
                    .iter()
                    .find(|(source, _)| source.0 == name)
                    .map(|(_, target)| target)
                    .ok_or("Animation event bone is absent from its hierarchy")?;
                for (target, source) in attribute.payload.0.iter_mut().zip(mirrored.0) {
                    *target = Some(source);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn blend(
    left: &mut AnimationAttribute,
    right: &AnimationAttribute,
    weight: f32,
) -> Result<(), String> {
    let left_weight = 1.0 - weight;
    if left.status & 2 != 0 {
        left.begin_time = -1.0;
        left.end_time = -1.0;
    } else {
        // Time fields use separate multiply/add instructions, not fused math.
        left.begin_time = right.begin_time * weight + left.begin_time * left_weight;
        left.end_time = right.end_time * weight + left.end_time * left_weight;
    }
    let lanes: &[usize] = match left.kind {
        0 | 2 => &[0],
        1 => &[0, 1, 2, 3],
        3 => &[5],
        _ => &[],
    };
    for &i in lanes {
        let l = f32::from_bits(left.payload.0[i].ok_or("Uninitialized blended attribute payload")?);
        let r =
            f32::from_bits(right.payload.0[i].ok_or("Uninitialized blended attribute payload")?);
        left.payload.0[i] = Some(r.mul_add(weight, l * left_weight).to_bits());
    }
    Ok(())
}

///82D16370. Keep reference names and event flags; scale only numeric payload.
pub fn scale(attribute: &mut AnimationAttribute, weight: f32) -> Result<(), String> {
    if attribute.status & 2 != 0 {
        attribute.begin_time = -1.;
        attribute.end_time = -1.;
    } else {
        attribute.begin_time *= weight;
        attribute.end_time *= weight;
    }
    for &i in numeric_lanes(attribute.kind) {
        let value =
            f32::from_bits(attribute.payload.0[i].ok_or("Uninitialized weighted attribute")?);
        attribute.payload.0[i] = Some((value * weight).to_bits());
    }
    Ok(())
}
///82D16428. Time adds are separate instructions; numeric payload uses FMA.
pub fn add_weighted(
    left: &mut AnimationAttribute,
    right: &AnimationAttribute,
    weight: f32,
) -> Result<(), String> {
    if left.status & 2 == 0 {
        left.begin_time += right.begin_time * weight;
        left.end_time += right.end_time * weight;
    }
    for &i in numeric_lanes(left.kind) {
        let a = f32::from_bits(left.payload.0[i].ok_or("Uninitialized weighted attribute")?);
        let b = f32::from_bits(right.payload.0[i].ok_or("Uninitialized weighted attribute")?);
        let value = b.mul_add(weight, a);
        left.payload.0[i] = Some(value.to_bits());
    }
    Ok(())
}
fn numeric_lanes(kind: u8) -> &'static [usize] {
    match kind {
        0 | 2 => &[0],
        1 => &[0, 1, 2, 3],
        3 => &[5],
        _ => &[],
    }
}

/// Native uses a forward-only intersection, retaining left order/metadata.
/// It does not append right-only attributes or replace the left reference name.
pub fn intersection(
    left: Vec<AnimationAttribute>,
    right: &[AnimationAttribute],
    weight: f32,
) -> Result<Vec<AnimationAttribute>, String> {
    let mut result = Vec::new();
    let mut next = 0;
    for mut attribute in left {
        while next < right.len() && right[next].name.0 < attribute.name.0 {
            next += 1;
        }
        let Some(other) = right.get(next) else {
            break;
        };
        if other.name == attribute.name {
            blend(&mut attribute, other, weight)?;
            result.push(attribute);
        }
    }
    Ok(result)
}
