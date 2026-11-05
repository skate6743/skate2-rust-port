//! CACPresEntity CreateBlendJob827e89d0 / Morph_VMX827d4720.
//! Morph private RX2 resource copies before the shared vertex decoder consumes
//! them. Source SHORT4N/HEND3N packing is retained; host rsqrt seed is explicit.
use skate2_formats::{
    mount::MountSet,
    pegasus::ObjectId,
    recipe::CharacterCustomizations,
    render::{
        self, MeshHelper, OptimizedRenderMeshData, RenderBlendShapeData, VertexBuffer,
        VertexDescriptor,
    },
    rw4::{Arena, DictionaryEntry},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(crate) struct CharacterMorph {
    keywords: [String; 21],
    weights: Vec<f32>,
}
impl CharacterMorph {
    pub(crate) fn load(
        mounts: &MountSet,
        custom: Option<&CharacterCustomizations>,
    ) -> Result<Self> {
        let mut result = Self {
            keywords: std::array::from_fn(|_| String::new()),
            weights: custom.map_or_else(Vec::new, |c| {
                c.morph_bits.iter().copied().map(f32::from_bits).collect()
            }),
        };
        if result.weights.is_empty() {
            return Ok(result);
        }
        if result.weights.len() != 21 {
            return Err("CAS morph requires original21 zones".into());
        }
        const CLASS: u64 = 0x7f47_ca62_f410_0f41;
        let vault = crate::attribute_runtime::AttributeRuntimeVault::try_load_mounted(
            mounts,
            "data/db/skater.vlt",
            &[CLASS],
        )
        .map_err(|error| error.to_string())?;
        for key in vault.collections_sorted(CLASS)? {
            let instance = vault.instance(CLASS, key);
            let Some(data) = instance.try_cached_layout_field(4, 4)? else {
                continue;
            };
            let zone = u32::from_be_bytes(data.bytes().try_into()?) as usize;
            if zone < 21 {
                if let Some(text) = instance.try_cached_layout_text(0)? {
                    result.keywords[zone] = std::str::from_utf8(text.bytes())?
                        .trim_end_matches('\0')
                        .to_owned();
                }
            }
        }
        Ok(result)
    }
    pub(crate) fn apply(&self, arena: &Arena, resources: &mut [Vec<u8>; 5]) -> Result<()> {
        if self.weights.is_empty() {
            return Ok(());
        }
        let mut visited = std::collections::BTreeSet::new();
        for entry in arena
            .dictionary
            .iter()
            .filter(|e| e.type_id == render::R_OPTI_MESH_DATA_TYPE_ID)
        {
            let raw = arena
                .object_bytes(resources, entry)
                .ok_or("CAS optimized mesh bytes")?;
            let mesh = OptimizedRenderMeshData::parse(raw)?;
            if mesh.blend_shapes.is_empty() {
                continue;
            }
            if mesh.blend_shapes.len() > 21 {
                return Err("CAS blend targets exceed21".into());
            }
            let mut ordered = Vec::new();
            for target in &mesh.blend_shapes {
                let bytes = raw
                    .get(target.name_offset as usize..)
                    .ok_or("CAS target name offset")?;
                let end = bytes
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("CAS target name terminator")?;
                let mut name = std::str::from_utf8(&bytes[..end])?;
                if name.starts_with("global_fat") {
                    name = "fat";
                }
                let Some(zone) = self.keywords.iter().position(|prefix| !prefix.is_empty() && name.starts_with(prefix)) else {
                    // PC port: a linked Skate 3 data folder's clothing (`crate::skate3`) carries
                    // targets for body shapes Skate 2 has no zone for (`thin_...`); they stay at
                    // the base shape.
                    if crate::skate3::linked().is_some() {
                        crate::skate3::note_unmatched_morph(name);
                        continue;
                    }
                    return Err(format!("unknown original CAS morph target {name}").into());
                };
                ordered.push((zone, target.blend_shape));
            }
            ordered.sort_by_key(|(zone, _)| *zone);
            if ordered.windows(2).any(|p| p[0].0 == p[1].0) {
                return Err("duplicate CAS target priority".into());
            }
            let source = stream(
                arena,
                resources,
                mesh.vertex_descriptor.ok_or("CAS vertex descriptor")?,
                mesh.mesh_helper.ok_or("CAS mesh helper")?,
            )?;
            if !visited.insert(source.data.0) {
                continue;
            }
            let destination =
                object_entry(arena, source.data, render::GPU_BUFFER_RESOURCE_TYPE_ID)?;
            let mut output = arena
                .object_bytes(resources, destination)
                .ok_or("CAS source stream")?
                .to_vec();
            if source.stride == 0 || source.size % source.stride != 0 {
                return Err("CAS morph stream remainder".into());
            }
            if source.size > output.len() {
                return Err("CAS source stream outside resource".into());
            }
            let count = source.size / source.stride;
            let mut targets = Vec::new();
            for (index, (_, id)) in ordered.iter().enumerate() {
                // Native sorted target buffers consume the contiguous multipliers
                // in order; they do not index multipliers by the dictionary order.
                let blend = RenderBlendShapeData::parse(object(
                    arena,
                    resources,
                    *id,
                    render::R_BLEND_SHAPE_DATA_TYPE_ID,
                )?)?;
                let target = stream(arena, resources, blend.vertex_descriptor, blend.mesh_helper)?;
                let bytes = object(
                    arena,
                    resources,
                    target.data,
                    render::GPU_BUFFER_RESOURCE_TYPE_ID,
                )?;
                if target.stride == 0 || target.size % target.stride != 0 {
                    return Err("CAS target stream remainder".into());
                }
                if target.size > bytes.len() {
                    return Err("CAS target stream outside resource".into());
                }
                if target.size / target.stride < count {
                    return Err("CAS target shorter than source".into());
                }
                targets.push((self.weights[index], target, bytes));
            }
            for vertex in 0..count {
                let offset = vertex * source.stride;
                let mut position = read_position(&output[offset..])?;
                let mut normal = read_normal(&output[offset + source.normal..])?;
                for (weight, target, bytes) in &targets {
                    if f64::from(*weight) <= 0.001 {
                        continue;
                    }
                    let start = vertex * target.stride;
                    let delta = read_position(&bytes[start..])?;
                    let n = read_normal(&bytes[start + target.normal..])?;
                    for lane in 0..3 {
                        position[lane] = delta[lane].mul_add(*weight, position[lane]);
                        normal[lane] = n[lane].mul_add(*weight, normal[lane]);
                    }
                }
                for lane in 0..3 {
                    let value = (position[lane] * 32767.).floor();
                    if !value.is_finite()
                        || f64::from(value) < f64::from(i32::MIN)
                        || f64::from(value) > f64::from(i32::MAX)
                    {
                        return Err("CAS morph position fctiwz exceptional result".into());
                    }
                    output[offset + lane * 2..offset + lane * 2 + 2]
                        .copy_from_slice(&(value as i32 as i16).to_be_bytes());
                }
                use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};
                let mut numeric = HostReckoningNumeric;
                let v = [normal[0], normal[1], normal[2], 0.];
                let square = numeric.dot3(v, v)?;
                if square <= 0. {
                    return Err("CAS morph zero normal".into());
                }
                let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt(
                    square,
                    numeric.reciprocal_sqrt_estimate(square)?,
                );
                let n = normal.map(|v| v * inverse);
                let x = (n[0] * 1023.) as i32 as u32 & 0x7ff;
                let y = (n[1] * 1023.) as i32 as u32 & 0x7ff;
                let z = (n[2] * 511.) as i32 as u32;
                output[offset + source.normal..offset + source.normal + 4]
                    .copy_from_slice(&(((z << 11) | y) << 11 | x).to_be_bytes());
            }
            resources[destination.resource_index][destination.object_range.clone()]
                .copy_from_slice(&output);
        }
        Ok(())
    }
}
struct Stream {
    data: ObjectId,
    stride: usize,
    normal: usize,
    size: usize,
}
fn stream(
    arena: &Arena,
    resources: &[Vec<u8>; 5],
    descriptor: ObjectId,
    helper: ObjectId,
) -> Result<Stream> {
    let descriptor = VertexDescriptor::parse(object(
        arena,
        resources,
        descriptor,
        render::VERTEX_DESCRIPTOR_TYPE_ID,
    )?)?;
    let helper = MeshHelper::parse(object(
        arena,
        resources,
        helper,
        render::MESH_HELPER_TYPE_ID,
    )?)?;
    let position = descriptor.position_element().ok_or("CAS morph position")?;
    let normal = descriptor
        .elements
        .iter()
        .find(|e| e.usage == 3 && e.usage_index == 0)
        .ok_or("CAS morph normal")?;
    if position.stream != 0
        || normal.stream != 0
        || position.offset != 0
        || position.format != render::VERTEX_FORMAT_SHORT4N
        || normal.format != render::VERTEX_FORMAT_HEND3N
    {
        return Err("CAS Morph_VMX requires source SHORT4N and HEND3N stream0".into());
    }
    let stride = *descriptor.stream_strides()?.get(&0).ok_or("CAS stride")? as usize;
    let buffer = VertexBuffer::parse(object(
        arena,
        resources,
        *helper.vertex_buffers.first().ok_or("CAS stream0")?,
        render::VERTEX_BUFFER_TYPE_ID,
    )?)?;
    Ok(Stream {
        data: buffer.data,
        stride,
        normal: normal.offset as usize,
        size: buffer.buffer_size as usize,
    })
}
fn object_entry(arena: &Arena, id: ObjectId, kind: u32) -> Result<&DictionaryEntry> {
    let entry = arena.local_entry(id).ok_or("CAS resource object absent")?;
    if entry.type_id != kind {
        return Err("CAS resource type mismatch".into());
    }
    Ok(entry)
}
fn object<'a>(
    arena: &Arena,
    resources: &'a [Vec<u8>; 5],
    id: ObjectId,
    kind: u32,
) -> Result<&'a [u8]> {
    arena
        .object_bytes(resources, object_entry(arena, id, kind)?)
        .ok_or_else(|| "CAS resource range".into())
}
fn read_position(bytes: &[u8]) -> Result<[f32; 3]> {
    if bytes.len() < 8 {
        return Err("CAS short position".into());
    }
    Ok(std::array::from_fn(|i| {
        f32::from(i16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]])) * (1. / 32767.)
    }))
}
fn read_normal(bytes: &[u8]) -> Result<[f32; 3]> {
    let bits = u32::from_be_bytes(bytes.get(..4).ok_or("CAS short normal")?.try_into()?);
    Ok([
        ((bits << 21) as i32 >> 21) as f32 * (1. / 1023.),
        ((bits << 10) as i32 >> 21) as f32 * (1. / 1023.),
        (bits as i32 >> 22) as f32 * (1. / 511.),
    ])
}
