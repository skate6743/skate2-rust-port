//! Compact, bounded definitions for the existing fragmented APPLICATION stream.
//! Scalar metadata is JSON; hull vertices are lossless little-endian f32 values.
//! This is an in-memory wire encoding, NOT a collision JSON/file requirement.
use crate::{
    BodyDefinition, Shape,
    model::{MAX_BODY_POINTS, MAX_COMPOUND_HULLS, MAX_HULL_POINTS},
};
const MAGIC: &[u8; 4] = b"SBD4";
const MAX_HEADER: usize = 8_192;
pub const MAX_DEFINITION_BYTES: usize = 640 * 192;

fn visit_points(
    definition: &mut BodyDefinition,
    mut visit: impl FnMut(&mut Vec<[f32; 3]>) -> Result<(), String>,
) -> Result<(), String> {
    if definition.extras.len() > 16 {
        return Err("too many extra colliders".into());
    }
    match &mut definition.body.shape {
        Shape::Convex { points } => visit(points)?,
        Shape::Compound { hulls } => {
            if hulls.is_empty() || hulls.len() > MAX_COMPOUND_HULLS {
                return Err("bad compound part count".into());
            }
            for points in hulls {
                visit(points)?;
            }
        }
        Shape::Mesh { .. } | Shape::Model { .. } => {
            return Err("cannot replicate unresolved asset paths".into());
        }
        _ => {}
    }
    for extra in &mut definition.extras {
        visit(&mut extra.points)?;
    }
    Ok(())
}

pub fn encode(definition: &BodyDefinition) -> Result<Vec<u8>, String> {
    let mut header = definition.clone();
    let mut groups = Vec::new();
    let mut total = 0usize;
    visit_points(&mut header, |points| {
        if !(4..=MAX_HULL_POINTS).contains(&points.len())
            || points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1000.)
        {
            return Err("invalid hull vertices in body definition".into());
        }
        total += points.len();
        if total > MAX_BODY_POINTS {
            return Err("body definition vertex budget exceeded".into());
        }
        groups.push(std::mem::take(points));
        Ok(())
    })?;
    let offsets = if let Some(field) = &mut header.deformation {
        field.validate()?;
        std::mem::take(&mut field.offsets)
    } else {
        Vec::new()
    };
    let metadata = serde_json::to_vec(&header).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_HEADER {
        return Err("body definition metadata budget exceeded".into());
    }
    let mut bytes = Vec::with_capacity(8 + metadata.len() + 12 * total + 2 * groups.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&metadata);
    for points in groups {
        bytes.extend_from_slice(&(points.len() as u16).to_le_bytes());
        for point in points {
            for value in point {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    for offset in offsets {
        for value in offset {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    if bytes.len() > MAX_DEFINITION_BYTES {
        return Err("body definition byte budget exceeded".into());
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<BodyDefinition, String> {
    if bytes.len() < 8 || bytes.len() > MAX_DEFINITION_BYTES || &bytes[..4] != MAGIC {
        return Err("unsupported or oversized body definition".into());
    }
    let length = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| "bad header")?) as usize;
    if length > MAX_HEADER || length > bytes.len() - 8 {
        return Err("bad metadata length".into());
    }
    let mut definition: BodyDefinition =
        serde_json::from_slice(&bytes[8..8 + length]).map_err(|e| e.to_string())?;
    let mut at = 8 + length;
    let mut total = 0usize;
    visit_points(&mut definition, |points| {
        if !points.is_empty() {
            return Err("inline geometry is not permitted in compact definitions".into());
        }
        let count_bytes = bytes.get(at..at + 2).ok_or("truncated hull count")?;
        let count = u16::from_le_bytes([count_bytes[0], count_bytes[1]]) as usize;
        at += 2;
        if !(4..=MAX_HULL_POINTS).contains(&count) || total + count > MAX_BODY_POINTS {
            return Err("received hull exceeds vertex budget".into());
        }
        let payload = bytes
            .get(at..at + count * 12)
            .ok_or("truncated hull vertices")?;
        total += count;
        points.reserve(count);
        for point in payload.chunks_exact(12) {
            let mut values = [0.; 3];
            for (axis, value) in values.iter_mut().enumerate() {
                *value = f32::from_le_bytes(
                    point[axis * 4..axis * 4 + 4]
                        .try_into()
                        .map_err(|_| "bad vertex")?,
                );
                if !value.is_finite() || value.abs() > 1000. {
                    return Err("invalid received vertex".into());
                }
            }
            points.push(values);
        }
        at += payload.len();
        Ok(())
    })?;
    if let Some(field) = &mut definition.deformation {
        if !field.offsets.is_empty() || field.resolution.iter().any(|n| !(2..=25).contains(n)) {
            return Err("bad deformation header".into());
        }
        let count = field.resolution.iter().product::<usize>();
        if count > 2048 {
            return Err("deformation field budget exceeded".into());
        }
        let payload = bytes
            .get(at..at + count * 12)
            .ok_or("truncated deformation field")?;
        field.offsets = payload
            .chunks_exact(12)
            .map(|p| {
                std::array::from_fn(|a| f32::from_le_bytes(p[a * 4..a * 4 + 4].try_into().unwrap()))
            })
            .collect();
        field.validate()?;
        at += payload.len();
    }
    if at != bytes.len() {
        return Err("trailing body definition bytes".into());
    }
    Ok(definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyDesc, DynamicsWorld};
    #[test]
    fn compound_round_trip_and_truncation() {
        let hull = vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let definition = BodyDefinition {
            deformation: None,
            body: BodyDesc {
                shape: Shape::Compound {
                    hulls: vec![hull.clone(), hull],
                },
                ..Default::default()
            },
            extras: vec![],
        };
        let bytes = encode(&definition).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(&definition).unwrap(),
            serde_json::to_value(&decoded).unwrap()
        );
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err(), "accepted truncation {end}");
        }
        let mut excess = bytes.clone();
        excess.push(0);
        assert!(decode(&excess).is_err());
        assert!(DynamicsWorld::default().spawn_replica(&decoded).is_ok());
    }
    #[test]
    fn rejects_asset_paths_and_oversized_data() {
        let definition = BodyDefinition {
            deformation: None,
            body: BodyDesc {
                shape: Shape::Model {
                    path: "model.glb".into(),
                    object: String::new(),
                    options: Default::default(),
                },
                ..Default::default()
            },
            extras: vec![],
        };
        assert!(encode(&definition).is_err());
        assert!(decode(&vec![0; MAX_DEFINITION_BYTES + 1]).is_err());
    }
}
