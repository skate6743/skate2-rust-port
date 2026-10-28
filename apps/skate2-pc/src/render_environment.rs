//! Original environment-lighting inputs read from the shipped `skater.vlt`.
//!
//! Reference (debug `sk82_na_m.xex`):
//! - `GraphicsMan::Init` 0x8273f6e8 binds `g_envattributes` to the
//!   `material_envattributes/default` layout array: element count at +0,
//!   16-byte float4 data from +16.
//! - `RenderMan::CreateFresnelLookupTexture` 0x82833a68 and
//!   `UpdateFresnelLookupTexture` 0x8282a528 fill a 256x1 A8R8G8B8 table from
//!   `g_envattributes[5].xyz` (exponent, facing, edging).
//! - `RenderMan::UpdateRenderParams` 0x82852780 reads the world
//!   `render_locations` collection chosen by
//!   `LooseWorldPresentation::GetWorldRenderLocation` 0x827d0cc8 (no explicit
//!   location: key 0xec763034105b866b), normalizes its sun vector into
//!   `g_vLightDir`, and packs the blended fog (`m_FogAttribs` output) into
//!   `g_FogK1` / `g_FogColour` (`GlobalShaderData` +0x140 / +0x130).
//! - `RenderMan::UpdateWorldPainterAttribs` 0x82848ba8 blends `material_fog`
//!   and `material_sky` collections chosen by the WorldPainter keys, or by the
//!   location's `material_fog`/`material_sky` references without them. The fog
//!   layout is colour +0, power +16, start +20, strength +24, end +28.
//! - `cMaterialScope::Resolve` 0x82e17a98 binds each material's `m_params`
//!   from its own class/variant collection; the sky blend writes into
//!   `material_sky/default`'s `m_params` storage.
//! - With the location's sun-map flag and `kEnableSunDirectionMap` (default
//!   1), `UpdateRenderParams` replaces the Vault sun with
//!   `SunLightMan::GetSunPosition` 0x82851f20 sampled at the local skater's
//!   hips (`SkaterPresentation::GetHipsPos` 0x827e3ef0), or at the camera
//!   without one, before the normalize. See `SunDirectionMap`.
//! - `PostFX::PostFX` 0x82839748 binds `m_CSMAttribs` to a `rendering`
//!   collection whose values `RenderShadows_CSM` reads (`CsmAttributes`).

use crate::{
    AttributeCollectionMap, attribute_direct_pointer_u64, attribute_string,
    gpu_renderer::RenderResult,
    render_shadows::{CsmAttributes, WorldShadowAttributes},
    resolve_attribute_location,
};
use skate2_formats::{
    attrib::{
        AttributeExport, AttributeExportKind, AttributePointerLocation, AttributePointerValue,
        AttributeVaultMetadata, string_hash64,
    },
    hash::rw_hash64_string,
    material::{TEXTURE_TYPE_ID, Texture},
    mount::MountSet,
    pegasus::{TABLE_OF_CONTENTS_TYPE_ID, TableOfContents},
    render::GPU_BUFFER_RESOURCE_TYPE_ID,
    rw4::Arena as Rw4Arena,
};
use skate2_core::challenge::attrib_db::{AttribDb, Attribute};
use std::collections::{BTreeMap, BTreeSet};

/// `GetWorldRenderLocation` default when neither a location nor the marquee
/// viewer is active.
const WORLD_LOCATION_KEY: u64 = 0xec76_3034_105b_866b;
const SUN_VECTOR_KEY: u64 = 0x1c70_4903_08c5_34f2;
/// Layout byte 3: `UpdateRenderParams` then asks `SunLightMan` for the sun.
const SUN_MAP_FLAG_KEY: u64 = 0x9b49_7535_feaf_c4b5;
/// `default_sh_probe`: nine float3 coefficients.
const DEFAULT_SH_PROBE_KEY: u64 = 0xf45d_fcab_6ec1_95cd;
const FOG_COLOUR_KEY: u64 = 0x6168_551a_6d8b_1750;
const FOG_POWER_KEY: u64 = 0x9807_5c22_d130_b9b1;
const FOG_START_KEY: u64 = 0xff6a_0d91_f34f_f75a;
const FOG_STRENGTH_KEY: u64 = 0xf493_615b_931e_d5b5;
const FOG_END_KEY: u64 = 0x23cf_effd_8097_9581;
/// `1/255` as the native `fnmsubs` operand (0x3b808081).
const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// `RenderMan::RenderMan` 0x82854268 starts the fog and sky blends on the
/// `default` collection (`StringHash64("default")`).
pub(crate) const DEFAULT_COLLECTION_KEY: u64 = 0xd7ed_bd36_2d7d_2152;
/// The `rendering` collection `SunLightMan::SetTexturePointers` 0x8284e3f0
/// and `LoadWorldMapAsset` 0x82851d10 read; the same hash as the
/// `render_locations` sun vector key.
const SUN_MAP_COLLECTION_KEY: u64 = 0x1c70_4903_08c5_34f2;
/// Its texture name, looked up as `RwHash64String("<name>.Texture")`.
const SUN_MAP_TEXTURE_KEY: u64 = 0xe1d4_c5c7_44e8_4dd8;
/// The world (x, z) rectangle the map covers.
const SUN_MAP_WORLD_MIN_KEY: u64 = 0xfb5b_9bfe_a84b_1155;
const SUN_MAP_WORLD_MAX_KEY: u64 = 0x71b0_a3a1_89b8_bd3f;
/// Boot texture arena (`miscboot.big`) holding the map texture.
const SUN_MAP_ARENA: &str = "data\\content\\textures\\globaltextures.rx2";
/// renderengine `PIXELFORMAT_LIN_A8R8G8B8`: `LoadWorldMapImage` 0x8284f968
/// locks the untiled surface and reads it directly.
const PIXEL_FORMAT_LINEAR_A8R8G8B8: u32 = 405_274_758;
/// `GetSunPosition` decode factor, 2/255 (`fmsubs` operand 0x3c008081).
const TWO_OVER_255: f32 = f32::from_bits(0x3c00_8081);
/// `PostFX::m_CSMAttribs`: `rendering` collection 0xb9a92367e7ac68b9.
const CSM_COLLECTION_KEY: u64 = 0xb9a9_2367_e7ac_68b9;
/// Cascade 0 half-extent (`RenderShadows_CSM` 0x82856018).
const CSM_SCALE_KEY: u64 = 0xc1e5_1168_dd06_bd38;
/// Cascade centre offset along the camera axis (0x82856048..6090).
const CSM_OFFSET_KEY: u64 = 0x62c0_3852_c5f4_d2ab;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FogAttributes {
    pub(crate) colour: [f32; 4],
    pub(crate) power: f32,
    pub(crate) start: f32,
    pub(crate) strength: f32,
    pub(crate) end: f32,
}

impl FogAttributes {
    /// `(g_FogK1, g_FogColour)` in `UpdateRenderParams` lane order: K1 is
    /// (1/(end-start), (-1/(end-start))*start, power, 0) and the colour lanes
    /// are the fog RGB and -1, each scaled by strength.
    pub(crate) fn constants(&self) -> ([f32; 4], [f32; 4]) {
        let range = self.end - self.start;
        let k1 = [1.0 / range, (-1.0 / range) * self.start, self.power, 0.0];
        let colour = [
            self.colour[0] * self.strength,
            self.colour[1] * self.strength,
            self.colour[2] * self.strength,
            -self.strength,
        ];
        (k1, colour)
    }
}

/// `SunLightMan::WorldMap`: every texel's (r, g, b) bytes and the (x, z) to
/// (s, t) transform `LoadWorldMapAsset` builds from the world rectangle.
#[derive(Clone, Debug)]
pub(crate) struct SunDirectionMap {
    width: u32,
    height: u32,
    /// `NormalPixel<unsigned char>`, row-major.
    pixels: Vec<[u8; 3]>,
    /// `mWorldTransform` row 0 x and row 1 y: 1 / (max - min).
    scale: [f32; 2],
    /// Row 2 (x, y): min / (min - max).
    offset: [f32; 2],
}

impl SunDirectionMap {
    /// `SetTexturePointers` names the texture; `LoadWorldMapAsset` and the
    /// A8R8G8B8 `LoadWorldMapImage` overload 0x8284e578 read texel bytes 1, 2
    /// and 3 as r, g and b straight from the locked surface.
    fn load(mounts: &MountSet, name: &str, min: [f32; 2], max: [f32; 2]) -> RenderResult<Self> {
        let bytes = mounts.read_path(SUN_MAP_ARENA)?;
        let resources = Rw4Arena::split_file_resources(&bytes)?;
        let arena = Rw4Arena::parse(&resources)?;
        let guid = rw_hash64_string(&format!("{name}.Texture"));
        let mut object = None;
        for entry in &arena.dictionary {
            if entry.type_id != TABLE_OF_CONTENTS_TYPE_ID {
                continue;
            }
            let bytes = arena
                .object_bytes(&resources, entry)
                .ok_or("sun map TOC range is invalid")?;
            if let Some(item) = TableOfContents::parse(bytes)?
                .entries
                .into_iter()
                .find(|item| item.guid == guid)
            {
                object = Some(item.object);
            }
        }
        let object =
            object.ok_or_else(|| format!("{name}.Texture is absent from {SUN_MAP_ARENA}"))?;
        let entry = arena
            .local_entry(object)
            .filter(|entry| entry.type_id == TEXTURE_TYPE_ID)
            .ok_or("sun map TOC entry is not a texture")?;
        let texture = Texture::parse(
            arena
                .object_bytes(&resources, entry)
                .ok_or("sun map texture range is invalid")?,
        )?;
        if texture.pixel_format != PIXEL_FORMAT_LINEAR_A8R8G8B8 {
            return Err(format!(
                "sun map format {:#x}: only the LIN_A8R8G8B8 LoadWorldMapImage path is ported",
                texture.pixel_format
            )
            .into());
        }
        let data = arena
            .local_entry(texture.data)
            .filter(|entry| entry.type_id == GPU_BUFFER_RESOURCE_TYPE_ID)
            .and_then(|entry| arena.object_bytes(&resources, entry))
            .ok_or("sun map texel payload link is invalid")?;
        // Linear fetch constant: dword 0 bits 22-30 hold the pitch in
        // 32-texel units; `Texture::Lock` returns that stride.
        let pitch = usize::try_from(((texture.fetch_words[0] >> 22) & 0x1ff) * 32 * 4)?;
        let width = texture.width.max(1);
        let height = texture.height.max(1);
        let row_bytes = usize::try_from(width)? * 4;
        let mut pixels = Vec::with_capacity(usize::try_from(width * height)?);
        for row in 0..usize::try_from(height)? {
            let start = row * pitch;
            let texels = data
                .get(start..start + row_bytes)
                .ok_or("sun map payload is shorter than its surface")?;
            pixels.extend(
                texels
                    .chunks_exact(4)
                    .map(|texel| [texel[1], texel[2], texel[3]]),
            );
        }
        Ok(Self {
            width,
            height,
            pixels,
            scale: [1.0 / (max[0] - min[0]), 1.0 / (max[1] - min[1])],
            offset: [min[0] / (min[0] - max[0]), min[1] / (min[1] - max[1])],
        })
    }

    /// `GetSunPosition` after its map checks: (x, z) through the transform
    /// (the zero lanes of its `vmaddfp` chain add nothing), `SampleBilinear`,
    /// then each channel `c * 2/255 - 1` fused (`fmsubs`).
    pub(crate) fn sample(&self, position: [f32; 3]) -> [f32; 3] {
        let s = (self.scale[0] * position[0]) + self.offset[0];
        let t = (self.scale[1] * position[2]) + self.offset[1];
        self.sample_bilinear(s, t)
            .map(|channel| channel.mul_add(TWO_OVER_255, -1.0))
    }

    /// `SunLightMan::SampleBilinear` 0x82826fc0 in its register order.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "native fcfid/frsp and fctiwz over 64x64 map coordinates"
    )]
    fn sample_bilinear(&self, s: f32, t: f32) -> [f32; 3] {
        // fsel pairs: s <= 0 (either zero) gives 0, then s > 1 or NaN gives 1.
        let clamp = |value: f32| {
            let value = if -value >= 0.0 { 0.0 } else { value };
            if 1.0 - value >= 0.0 { value } else { 1.0 }
        };
        let (width, height) = (self.width as i32, self.height as i32);
        let fx = clamp(s) * self.width as f32;
        let fy = clamp(t) * self.height as f32;
        // fctiwz truncation; the lower corner clamps to the last texel and the
        // upper one to min(corner + 1, last).
        let x0 = (fx as i32).min(width - 1);
        let y0 = (fy as i32).min(height - 1);
        let x1 = if x0 + 1 < width - 1 {
            x0 + 1
        } else {
            width - 1
        };
        let y1 = if y0 + 1 < height - 1 {
            y0 + 1
        } else {
            height - 1
        };
        let frac_x = fx - x0 as f32;
        let frac_y = fy - y0 as f32;
        let inverse_x = 1.0 - frac_x;
        let inverse_y = 1.0 - frac_y;
        let texel = |x: i32, y: i32| self.pixels[(y * width + x) as usize].map(f32::from);
        let [p00, p01, p10, p11] = [texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1)];
        let r = p01[0].mul_add(frac_x, p00[0] * inverse_x).mul_add(
            inverse_y,
            p11[0].mul_add(frac_x, p10[0] * inverse_x) * frac_y,
        );
        let g = p01[1].mul_add(frac_x, p00[1] * inverse_x).mul_add(
            inverse_y,
            p11[1].mul_add(frac_x, p10[1] * inverse_x) * frac_y,
        );
        let b = p11[2].mul_add(frac_x, p10[2] * inverse_x).mul_add(
            frac_y,
            p00[2].mul_add(inverse_x, p01[2] * frac_x) * inverse_y,
        );
        [r, g, b]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EnvironmentLighting {
    pub(crate) postfx: std::sync::Arc<crate::render_postfx::Assets>,
    pub(crate) envattributes: [[f32; 4]; 7],
    pub(crate) sun_vector: [f32; 3],
    /// UpdateRenderParams82852d20..e0c, rendering/default.
    pub(crate) view_light_direction: [f32; 4],
    pub(crate) view_light_response: [f32; 4],
    pub(crate) view_light_minimum: f32,
    pub(crate) view_light_backlit: f32,
    pub(crate) sun_direction_map_requested: bool,
    pub(crate) sun_map: SunDirectionMap,
    /// `PostFX::m_CSMAttribs` values `RenderShadows_CSM` reads.
    pub(crate) csm: CsmAttributes,
    pub(crate) world_shadow: WorldShadowAttributes,
    /// The same frustum collection's island area rule values.
    pub(crate) scene_cull: skate2_core::presentation_scene_culling::IslandAreaCullSettings,
    pub(crate) proxy_draw_order: i32,
    /// The location's `material_fog` / `material_sky` reference collections,
    /// the keys `UpdateWorldPainterAttribs` uses without WorldPainter data.
    pub(crate) location_fog_key: u64,
    pub(crate) location_sky_key: u64,
    /// Every `material_fog` collection, keyed by collection key.
    pub(crate) fogs: BTreeMap<u64, FogAttributes>,
    /// Every `material_sky` collection's two `m_params` elements.
    pub(crate) skies: BTreeMap<u64, [[f32; 4]; 2]>,
    material_params: BTreeMap<(u64, u64), Vec<[f32; 4]>>,
    /// The classes whose definitions declare `m_params`.
    m_params_classes: Vec<u64>,
    pub(crate) character_attributes: Vec<[f32; 4]>,
    pub(crate) use_world_sh_probes: bool,
    /// `use_worldpainter`, the location's layout byte 0: `UpdateWorldPainterAttribs` 0x82848c08
    /// takes the WorldPainter keys only when it is set, else the location's references.
    pub(crate) use_worldpainter: bool,
    pub(crate) default_irradiance: [[f32; 4]; 9],
    pub(crate) vehicle_lod_squared: [f32; 3],
    /// Materials whose `m_params` resolve to `material_sky/default`'s storage,
    /// which the WorldPainter sky blend overwrites every frame.
    sky_blended: BTreeSet<(u64, u64)>,
}

pub(crate) struct VaultReader<'a> {
    buffers: [&'a [u8]; 2],
    pointers: &'a BTreeMap<AttributePointerLocation, AttributePointerValue>,
    classes: &'a BTreeMap<u64, &'a AttributeExport>,
    collections: &'a AttributeCollectionMap<'a>,
}

impl VaultReader<'_> {
    pub(crate) fn collection_keys(&self, class: u64) -> Vec<u64> {
        self.collections.keys().filter_map(|&(c, key)| (c == class).then_some(key)).collect()
    }
    /// Where one attribute's value lives, following explicit entries through
    /// the parent chain before static/layout defaults (`Collection::GetNode`).
    fn location(&self, class_id: u64, collection_id: u64, key: u64) -> RenderResult<(u16, u64)> {
        let class = self
            .classes
            .get(&class_id)
            .ok_or("render Vault class absent")?;
        let (export, collection) = self
            .collections
            .get(&(class_id, collection_id))
            .ok_or("render Vault collection absent")?;
        let definition = class
            .class_definitions(&self.buffers, self.pointers)?
            .into_iter()
            .find(|definition| definition.key == key)
            .ok_or("render Vault attribute not declared by its class")?;
        Ok(resolve_attribute_location(
            class,
            export,
            collection,
            &definition,
            self.collections,
            self.pointers,
        )?
        .ok_or("render Vault attribute has no value")?)
    }

    pub(crate) fn bytes(
        &self,
        class_id: u64,
        collection_id: u64,
        key: u64,
        length: usize,
    ) -> RenderResult<&[u8]> {
        let (dependency, offset) = self.location(class_id, collection_id, key)?;
        let start = usize::try_from(offset)?;
        self.buffers
            .get(usize::from(dependency))
            .and_then(|bytes| bytes.get(start..start.checked_add(length)?))
            .ok_or_else(|| "render Vault attribute outside its buffer".into())
    }

    pub(crate) fn floats<const N: usize>(
        &self,
        class_id: u64,
        collection_id: u64,
        key: u64,
    ) -> RenderResult<[f32; N]> {
        let bytes = self.bytes(class_id, collection_id, key, N * 4)?;
        let mut values = [0.0; N];
        for (value, word) in values.iter_mut().zip(bytes.chunks_exact(4)) {
            *value = f32::from_be_bytes(word.try_into()?);
        }
        Ok(values)
    }

    /// `Attrib` array: count at +0, element size at +4, float4 data at +16.
    fn float4_array(
        &self,
        class_id: u64,
        collection_id: u64,
        key: u64,
    ) -> RenderResult<Vec<[f32; 4]>> {
        let header = self.bytes(class_id, collection_id, key, 16)?;
        let count = usize::from(u16::from_be_bytes([header[0], header[1]]));
        if u16::from_be_bytes([header[4], header[5]]) != 16 {
            return Err("render Vault array is not float4".into());
        }
        let bytes = self.bytes(class_id, collection_id, key, 16 + count * 16)?;
        bytes[16..]
            .chunks_exact(16)
            .map(|element| {
                let mut value = [0.0; 4];
                for (lane, word) in value.iter_mut().zip(element.chunks_exact(4)) {
                    *lane = f32::from_be_bytes(word.try_into()?);
                }
                Ok(value)
            })
            .collect()
    }

    /// A reference attribute: class key +0, collection key +8.
    pub(crate) fn reference(&self, class_id: u64, collection_id: u64, key: u64) -> RenderResult<(u64, u64)> {
        let bytes = self.bytes(class_id, collection_id, key, 16)?;
        Ok((
            u64::from_be_bytes(bytes[0..8].try_into()?),
            u64::from_be_bytes(bytes[8..16].try_into()?),
        ))
    }

    /// A string attribute: the value is a pointer to the text.
    pub(crate) fn string(
        &self,
        class_id: u64,
        collection_id: u64,
        key: u64,
    ) -> RenderResult<String> {
        let (dependency, offset) = self.location(class_id, collection_id, key)?;
        let (dependency, offset) = attribute_direct_pointer_u64(self.pointers, dependency, offset)
            .ok_or("render Vault string attribute has no pointer")?;
        attribute_string(&self.buffers, dependency, offset)
            .ok_or_else(|| "render Vault string attribute is not text".into())
    }

    fn fog(&self, class_id: u64, collection_id: u64) -> RenderResult<FogAttributes> {
        let [power] = self.floats::<1>(class_id, collection_id, FOG_POWER_KEY)?;
        let [start] = self.floats::<1>(class_id, collection_id, FOG_START_KEY)?;
        let [strength] = self.floats::<1>(class_id, collection_id, FOG_STRENGTH_KEY)?;
        let [end] = self.floats::<1>(class_id, collection_id, FOG_END_KEY)?;
        Ok(FogAttributes {
            colour: self.floats::<4>(class_id, collection_id, FOG_COLOUR_KEY)?,
            power,
            start,
            strength,
            end,
        })
    }
}

/// Element `index` of a database attribute as big-endian floats; `None` when it is shorter.
pub(crate) fn attribute_floats<const N: usize>(attribute: &Attribute<'_>, index: usize) -> Option<[f32; N]> {
    let bytes = attribute.value_bytes(index)?;
    (bytes.len() >= N * 4).then(|| std::array::from_fn(|lane| f32::from_be_bytes(bytes[lane * 4..lane * 4 + 4].try_into().expect("four bytes"))))
}

pub(crate) fn with_vault<T>(
    mounts: &MountSet,
    read: impl FnOnce(&VaultReader<'_>) -> RenderResult<T>,
) -> RenderResult<T> {
    let vlt = mounts.read_path("data\\db\\skater.vlt")?;
    let bin = mounts.read_path("data\\db\\skater.bin")?;
    let vault = AttributeVaultMetadata::parse(&vlt)?;
    let pointers = vault.resolve_pointers(&[vlt.len(), bin.len()])?;
    let classes = vault
        .exports
        .iter()
        .filter(|export| export.kind() == AttributeExportKind::Class)
        .map(|export| (export.object_id, export))
        .collect::<BTreeMap<_, _>>();
    let collections = vault
        .exports
        .iter()
        .filter(|export| export.kind() == AttributeExportKind::Collection)
        .map(|export| {
            let collection = export.collection_load_data(&vlt)?;
            Ok((
                (collection.class_id, collection.collection_id),
                (export, collection),
            ))
        })
        .collect::<Result<AttributeCollectionMap<'_>, skate2_formats::attrib::AttributeChunkError>>(
        )?;
    let reader = VaultReader {
        buffers: [vlt.as_slice(), bin.as_slice()],
        pointers: &pointers,
        classes: &classes,
        collections: &collections,
    };

    read(&reader)
}

impl EnvironmentLighting {
    pub(crate) fn frame_character_attributes(&self,frame:crate::gpu_renderer::FrameEnvironment)->RenderResult<[[f32;4];3]> {
        let mut rows: [[f32;4];3]=self.character_attributes.get(..3).ok_or("character attributes absent")?.try_into()?;
        // Tonemap82829248 writes g_charattributes[2].z alongside env[2].x.
        if let Some(exposure)=frame.exposure {rows[2][2]=exposure;}
        Ok(rows)
    }
    /// CasCamera82587634..78 chooses PostFX.m_FEAttribsCSM. The FE branch of
    /// RenderShadows_CSM uses its original CSM scale/offset/depth fields.
    pub(crate) fn load_frontend_csm(mounts: &MountSet, key: u64) -> RenderResult<CsmAttributes> {
        with_vault(mounts, |reader| {
            let class = string_hash64("rendering");
            let attributes = CsmAttributes {
                scale: reader.floats::<1>(class, key, CSM_SCALE_KEY)?[0],
                offset: reader.floats::<3>(class, key, CSM_OFFSET_KEY)?,
                depth_range: reader.floats::<1>(class, key, string_hash64("nis_depth_precision"))?[0],
            };
            if !(attributes.scale > 0. && attributes.scale.is_finite()
                && attributes.depth_range.is_finite()
                && attributes.offset.iter().all(|v|v.is_finite())) {
                return Err(format!("invalid original frontend CSM {key:016x}: {attributes:?}").into());
            }
            Ok(attributes)
        })
    }
    pub(crate) fn load(mounts: &MountSet) -> RenderResult<Self> {
        Self::load_location(mounts, WORLD_LOCATION_KEY)
    }
    pub(crate) fn load_location(mounts: &MountSet, location_key: u64) -> RenderResult<Self> {
        with_vault(mounts, |reader| {
            let collections = reader.collections;
            let classes = reader.classes;
            let pointers = reader.pointers;
            let envattributes: [[f32; 4]; 7] = reader
                .float4_array(
                    string_hash64("material_envattributes"),
                    DEFAULT_COLLECTION_KEY,
                    string_hash64("g_envattributes"),
                )?
                .try_into()
                .map_err(|_| "g_envattributes must hold seven float4 values")?;

            let location_class = string_hash64("render_locations");
            let use_world_sh_probes = reader.bytes(location_class, location_key, string_hash64("use_world_sh_probes"), 1)?[0] != 0;
            let use_worldpainter = reader.bytes(location_class, location_key, string_hash64("use_worldpainter"), 1)?[0] != 0;
            let probe = reader.bytes(
                location_class,
                location_key,
                0xf45d_fcab_6ec1_95cd,
                8 + 9 * 12,
            )?;
            if u16::from_be_bytes(probe[2..4].try_into()?) != 9
                || u16::from_be_bytes(probe[4..6].try_into()?) != 12
            {
                return Err(
                    "render location default probe must contain nine float3 coefficients".into(),
                );
            }
            let mut default_irradiance = [[0.; 4]; 9];
            for (row, bytes) in default_irradiance
                .iter_mut()
                .zip(probe[8..].chunks_exact(12))
            {
                for (lane, raw) in row[..3].iter_mut().zip(bytes.chunks_exact(4)) {
                    *lane = f32::from_be_bytes(raw.try_into()?);
                }
            }
            let character_attributes = reader.float4_array(
                string_hash64("material_charattributes"),
                DEFAULT_COLLECTION_KEY,
                string_hash64("g_charattributes"),
            )?;
            let sun_vector = reader.floats::<3>(location_class, location_key, SUN_VECTOR_KEY)?;
            let sun_direction_map_requested =
                reader.bytes(location_class, location_key, SUN_MAP_FLAG_KEY, 1)?[0] != 0;
            let rendering_class = string_hash64("rendering");
            let view_light_direction = reader.floats::<4>(
                rendering_class,
                DEFAULT_COLLECTION_KEY,
                0xd900_aef7_ea4d_75a2,
            )?;
            let view_light_response = reader.floats::<4>(
                rendering_class,
                DEFAULT_COLLECTION_KEY,
                0x222c_ba2d_46ea_6839,
            )?;
            let view_light_minimum = reader.floats::<1>(
                rendering_class,
                DEFAULT_COLLECTION_KEY,
                0x0279_cce0_02b4_6bc4,
            )?[0];
            let view_light_backlit = reader.floats::<1>(
                rendering_class,
                DEFAULT_COLLECTION_KEY,
                0x9cdb_53d1_64e5_da30,
            )?[0];
            // SceneRenderView ctor8284ff20: rendering/51416546b54f421d.
            // GetLODDistancesFromAttrib82821b74..c78: this full64 float array,
            // squared element by element. LOD2 is stored but not a fourth level.
            let lod_data = reader.bytes(
                rendering_class,
                0x5141_6546_b54f_421d,
                0x5d69_75b4_2dac_e3a7,
                20,
            )?;
            if u16::from_be_bytes([lod_data[0], lod_data[1]]) < 3
                || u16::from_be_bytes([lod_data[4], lod_data[5]]) != 4
            {
                return Err("vehicle LOD distances must contain three floats".into());
            }
            let vehicle_lod_squared = std::array::from_fn(|i| {
                let value = f32::from_be_bytes(
                    lod_data[8 + i * 4..12 + i * 4]
                        .try_into()
                        .expect("four bytes"),
                );
                value * value
            });

            let sun_map = SunDirectionMap::load(
                mounts,
                &reader.string(rendering_class, SUN_MAP_COLLECTION_KEY, SUN_MAP_TEXTURE_KEY)?,
                reader.floats::<2>(
                    rendering_class,
                    SUN_MAP_COLLECTION_KEY,
                    SUN_MAP_WORLD_MIN_KEY,
                )?,
                reader.floats::<2>(
                    rendering_class,
                    SUN_MAP_COLLECTION_KEY,
                    SUN_MAP_WORLD_MAX_KEY,
                )?,
            )?;
            let [scale] = reader.floats::<1>(rendering_class, CSM_COLLECTION_KEY, CSM_SCALE_KEY)?;
            // Layout +0x24, which `RenderShadows_CSM` 0x82855cbc reads as the range.
            let [depth_range] = reader.floats::<1>(
                rendering_class,
                CSM_COLLECTION_KEY,
                string_hash64("nis_depth_precision"),
            )?;
            let csm = CsmAttributes {
                scale,
                offset: reader.floats::<3>(rendering_class, CSM_COLLECTION_KEY, CSM_OFFSET_KEY)?,
                depth_range,
            };
            // ViewMan ctor8282aed8 binds every camera's Frustum to this
            // collection, NOT the CSM collection (its minShadowPixelSize differs).
            let frustum_collection = 0x5141_6546_b54f_421d;
            let world_shadow = WorldShadowAttributes {
                scale: reader.floats::<1>(
                    rendering_class,
                    CSM_COLLECTION_KEY,
                    0x9c87_0676_8cfc_bb4a,
                )?[0],
                offset: reader.floats::<3>(
                    rendering_class,
                    CSM_COLLECTION_KEY,
                    0x0fb8_d6f7_8561_5ebe,
                )?,
                cull: skate2_core::presentation_shadow_culling::WorldShadowCullSettings {
                    minimum_pixels: reader.floats::<1>(
                        rendering_class,
                        frustum_collection,
                        string_hash64("minShadowPixelSize"),
                    )?[0],
                    extra_radius: reader.floats::<1>(
                        rendering_class,
                        frustum_collection,
                        string_hash64("extraRadius"),
                    )?[0],
                },
                enabled: reader.bytes(
                    location_class,
                    location_key,
                    string_hash64("use_world_onto_object_shadow"),
                    1,
                )?[0]
                    != 0,
            };
            // `Frustum::UpdateCullParams` 0x82853970 reads the island area rule
            // from the same collection.
            let frustum_float = |name: &str| -> RenderResult<f32> {
                Ok(reader.floats::<1>(rendering_class, frustum_collection, string_hash64(name))?[0])
            };
            let scene_cull = skate2_core::presentation_scene_culling::IslandAreaCullSettings {
                min_dist: frustum_float("minDist")?,
                min_area: frustum_float("minAreaCull")?,
                optimal_dist: frustum_float("optimalCullDist")?,
            };
            // WorldPresentation827e9d10's first-mesh proxy-order exclusion.
            let proxy_draw_order = i32::from_be_bytes(
                reader
                    .bytes(
                        string_hash64("pass"),
                        0xc948_0baf_95b5_238f,
                        string_hash64("GUID"),
                        4,
                    )?
                    .try_into()
                    .map_err(|_| "proxy pass order width")?,
            );
            let postfx = std::sync::Arc::new(crate::render_postfx::Assets::read(reader, mounts, location_key)?);
            let fog_class = string_hash64("material_fog");
            let sky_class = string_hash64("material_sky");
            // A missing reference reads `Attrib::DefaultDataArea` (zeroes): key 0,
            // which blends back to the defaults.
            let location_key = |class| {
                reader
                    .reference(location_class, location_key, class)
                    .map_or(0, |(_, collection)| collection)
            };
            let location_fog_key = location_key(fog_class);
            let location_sky_key = location_key(sky_class);

            let m_params = string_hash64("m_params");
            let mut fogs = BTreeMap::new();
            let mut skies = BTreeMap::new();
            for &(class_id, collection_id) in collections.keys() {
                if class_id == fog_class {
                    fogs.insert(collection_id, reader.fog(class_id, collection_id)?);
                } else if class_id == sky_class {
                    // `ChangeCollection` 0x8283c038 reads elements 0 and 1; a
                    // missing element is `Attrib::DefaultDataArea` (zeroes).
                    let params = reader.float4_array(class_id, collection_id, m_params)?;
                    let mut pair = [[0.0; 4]; 2];
                    for (destination, source) in pair.iter_mut().zip(&params) {
                        *destination = *source;
                    }
                    skies.insert(collection_id, pair);
                }
            }
            if !fogs.contains_key(&DEFAULT_COLLECTION_KEY)
                || !skies.contains_key(&DEFAULT_COLLECTION_KEY)
            {
                return Err("material_fog/material_sky default collections absent".into());
            }

            let mut declares_m_params = BTreeMap::new();
            for (&class_id, class) in classes {
                let declared = class
                    .class_definitions(&reader.buffers, &pointers)?
                    .iter()
                    .any(|definition| definition.key == m_params);
                declares_m_params.insert(class_id, declared);
            }
            let m_params_classes = declares_m_params.iter().filter(|(_, declared)| **declared).map(|(&class, _)| class).collect();
            let sky_storage = reader.location(sky_class, DEFAULT_COLLECTION_KEY, m_params)?;
            let mut material_params = BTreeMap::new();
            let mut sky_blended = BTreeSet::new();
            for &(class_id, collection_id) in collections.keys() {
                if declares_m_params.get(&class_id) != Some(&true) {
                    continue;
                }
                let params = reader.float4_array(class_id, collection_id, m_params)?;
                if params.is_empty() {
                    continue;
                }
                if reader.location(class_id, collection_id, m_params)? == sky_storage {
                    sky_blended.insert((class_id, collection_id));
                }
                material_params.insert((class_id, collection_id), params);
            }
            Ok(Self {
                postfx,
                envattributes,
                use_world_sh_probes,
                use_worldpainter,
                default_irradiance,
                sun_vector,
                view_light_direction,
                view_light_response,
                view_light_minimum,
                view_light_backlit,
                sun_direction_map_requested,
                sun_map,
                csm,
                world_shadow,
                scene_cull,
                proxy_draw_order,
                location_fog_key,
                location_sky_key,
                fogs,
                skies,
                material_params,
                m_params_classes,
                character_attributes,
                vehicle_lod_squared,
                sky_blended,
            })
        })
    }

    /// The fog, sky, bloom, auto-exposure and colour cube collections of every vault `db` holds
    /// that the game's own lack: the downloadable content's (`cVaultManager::StartDLCLoad`
    /// 0x82ab8b50 adds them to their classes), which its locations' references name.
    pub(crate) fn with_vault_collections(&self, mounts: &MountSet, db: &AttribDb) -> RenderResult<Self> {
        let mut environment = self.clone();
        let fog_class = string_hash64("material_fog");
        for &key in db.collections(fog_class) {
            if environment.fogs.contains_key(&key) {
                continue;
            }
            let float = |attribute| db.attribute(fog_class, key, attribute).and_then(|value| value.f32(0));
            let colour = db.attribute(fog_class, key, FOG_COLOUR_KEY).and_then(|value| attribute_floats::<4>(&value, 0));
            if let (Some(colour), Some(power), Some(start), Some(strength), Some(end)) =
                (colour, float(FOG_POWER_KEY), float(FOG_START_KEY), float(FOG_STRENGTH_KEY), float(FOG_END_KEY))
            {
                environment.fogs.insert(key, FogAttributes { colour, power, start, strength, end });
            }
        }
        let sky_class = string_hash64("material_sky");
        for &key in db.collections(sky_class) {
            if environment.skies.contains_key(&key) {
                continue;
            }
            // `ChangeCollection` 0x8283c038 reads elements 0 and 1; a missing one is zeroes.
            let params = db.attribute(sky_class, key, string_hash64("m_params"));
            let element = |index| params.as_ref().and_then(|params| attribute_floats::<4>(params, index)).unwrap_or([0.0; 4]);
            environment.skies.insert(key, [element(0), element(1)]);
        }
        // The content's world materials' `m_params` (`cMaterialScope::Resolve` 0x82e17a98), and
        // whether each resolves to `material_sky/default`'s storage, which the sky blend writes.
        let m_params = string_hash64("m_params");
        let sky_storage = db.attribute(sky_class, DEFAULT_COLLECTION_KEY, m_params).and_then(|params| params.data(0));
        for &class in &self.m_params_classes {
            for &key in db.collections(class) {
                if environment.material_params.contains_key(&(class, key)) {
                    continue;
                }
                let Some(params) = db.attribute(class, key, m_params) else { continue };
                let values: Vec<[f32; 4]> = (0..params.len()).filter_map(|index| attribute_floats::<4>(&params, index)).collect();
                if values.is_empty() {
                    continue;
                }
                if class == sky_class && sky_storage.is_some() && params.data(0) == sky_storage {
                    environment.sky_blended.insert((class, key));
                }
                environment.material_params.insert((class, key), values);
            }
        }
        let mut postfx = (*environment.postfx).clone();
        postfx.add_vault_collections(mounts, db)?;
        environment.postfx = std::sync::Arc::new(postfx);
        Ok(environment)
    }

    /// `cMsgSetWorldRenderLocation` (`LooseWorldPresentation::SetWorldRenderLocation` 0x827d53a8)
    /// with a world location's key: its `render_locations` collection, which
    /// `RenderMan::UpdateRenderParams` 0x82852780 changes to (and then
    /// `SHLightingMan::UpdateDefaultLightProbe`): the sun vector and sun-map flag, the default SH
    /// probe, the probe, shadow and WorldPainter flags, and the fog, sky, auto-exposure, bloom and
    /// colour cube references. Only downloadable content defines world locations' collections
    /// (`dlc_parkade`, ...); without one, `GetWorldRenderLocation` 0x827d0cc8 keeps the default
    /// location: `None`.
    pub(crate) fn with_world_render_location(&self, db: &AttribDb, key: u64) -> RenderResult<Option<Self>> {
        let class = string_hash64("render_locations");
        if !db.has_collection(class, key) {
            return Ok(None);
        }
        let value = |attribute: u64| {
            db.attribute(class, key, attribute)
                .ok_or_else(|| format!("render location {key:016x}: attribute {attribute:016x} absent"))
        };
        let flag = |name: &str| -> RenderResult<bool> {
            value(string_hash64(name))?.bool(0).ok_or_else(|| format!("render location {key:016x}: {name} unreadable").into())
        };
        let mut environment = self.clone();
        environment.sun_vector = attribute_floats::<3>(&value(SUN_VECTOR_KEY)?, 0)
            .ok_or_else(|| format!("render location {key:016x}: sun vector unreadable"))?;
        environment.sun_direction_map_requested =
            value(SUN_MAP_FLAG_KEY)?.bool(0).ok_or_else(|| format!("render location {key:016x}: sun map flag unreadable"))?;
        environment.use_worldpainter = flag("use_worldpainter")?;
        environment.use_world_sh_probes = flag("use_world_sh_probes")?;
        environment.world_shadow.enabled = flag("use_world_onto_object_shadow")?;
        let probe = value(DEFAULT_SH_PROBE_KEY)?;
        if probe.len() != 9 {
            return Err(format!("render location {key:016x}: the default probe must contain nine float3 coefficients").into());
        }
        for (index, row) in environment.default_irradiance.iter_mut().enumerate() {
            let [r, g, b] = attribute_floats::<3>(&probe, index)
                .ok_or_else(|| format!("render location {key:016x}: default probe coefficient {index} unreadable"))?;
            *row = [r, g, b, 0.0];
        }
        // A missing reference reads `Attrib::DefaultDataArea` (zeroes): key 0, which blends back to
        // the defaults.
        let reference = |attribute: u64| {
            db.attribute(class, key, attribute).and_then(|value| value.ref_spec(0)).map_or(0, |(_, collection)| collection)
        };
        environment.location_fog_key = reference(string_hash64("material_fog"));
        environment.location_sky_key = reference(string_hash64("material_sky"));
        let mut postfx = (*environment.postfx).clone();
        postfx.location = [
            reference(string_hash64("colour_cube")),
            reference(crate::render_postfx::DEFAULT_EXPOSURE),
            reference(crate::render_postfx::DEFAULT_BLOOM),
        ];
        environment.postfx = std::sync::Arc::new(postfx);
        Ok(Some(environment))
    }

    /// `m_params[0]` bound by `cMaterialScope::Resolve` for this material key.
    pub(crate) fn material_params(&self, class_id: u64, variant_id: u64) -> Option<[f32; 4]> {
        self.material_params
            .get(&(class_id, variant_id))
            .and_then(|params| params.first())
            .copied()
    }

    /// `m_params[0..2]` bound by `cMaterialScope::Resolve` for this material
    /// key; None unless the collection holds both elements.
    pub(crate) fn material_params_pair(
        &self,
        class_id: u64,
        variant_id: u64,
    ) -> Option<[[f32; 4]; 2]> {
        let params = self.material_params.get(&(class_id, variant_id))?;
        Some([*params.first()?, *params.get(1)?])
    }
    pub(crate) fn material_parameter_array(&self, class: u64, variant: u64) -> Option<&[[f32; 4]]> {
        self.material_params
            .get(&(class, variant))
            .map(Vec::as_slice)
    }

    /// `m_params[0..2]` for a sky material, and whether that storage is the
    /// one the WorldPainter sky blend writes (`material_sky/default`).
    pub(crate) fn sky_material_params(
        &self,
        class_id: u64,
        variant_id: u64,
    ) -> Option<([[f32; 4]; 2], bool)> {
        let params = self.material_params.get(&(class_id, variant_id))?;
        let params = [*params.first()?, *params.get(1)?];
        Some((params, self.sky_blended.contains(&(class_id, variant_id))))
    }

    /// `g_vLightDir` from the Vault sun vector alone, with w zeroed.
    pub(crate) fn light_direction(&self) -> [f32; 4] {
        normalized_light(self.sun_vector)
    }

    /// Native g_ViewDotLight: camera matrix axis2 normalized, dot authored
    /// direction, response.x + dot*response.y; then minimum/backlit/zero.
    /// Raw VMX128 operands verified (IDA incorrectly labels VA63 as VA95).
    pub(crate) fn view_dot_light(&self, camera_axis: [f32; 3]) -> [f32; 4] {
        let view = normalized_light(camera_axis);
        let d = self.view_light_direction;
        let dot = d[2].mul_add(view[2], d[1].mul_add(view[1], d[0] * view[0]));
        [
            dot.mul_add(self.view_light_response[1], self.view_light_response[0]),
            self.view_light_minimum,
            self.view_light_backlit,
            0.0,
        ]
    }

    /// `g_vLightDir` as `UpdateRenderParams` builds it each frame: the sun
    /// map sampled at `position` (the local skater's hips, or the camera
    /// without one) when the location requests it, else the Vault vector.
    pub(crate) fn light_direction_at(&self, position: [f32; 3]) -> [f32; 4] {
        normalized_light(if self.sun_direction_map_requested {
            self.sun_map.sample(position)
        } else {
            self.sun_vector
        })
    }

    /// `g_envattributes[2]`; the shaders read `.x` as exposure.
    pub(crate) fn exposure(&self) -> [f32; 4] {
        self.envattributes[2]
    }

    /// 256 RGBA8 texels of the native A8R8G8B8 fresnel table (R=G=B, A=0xff
    /// for in-range values), computed in `UpdateFresnelLookupTexture` order.
    pub(crate) fn fresnel_lookup(&self) -> Vec<u8> {
        let [exponent, facing, edging, _] = self.envattributes[5];
        let inverse_edging = 1.0 / edging;
        let mut texels = Vec::with_capacity(256 * 4);
        for index in 0..256_u16 {
            let base = (-f32::from(index)).mul_add(INV_255, 1.0);
            let blended = (edging - facing).mul_add(base.powf(exponent), facing);
            let scaled = (blended * 255.0) * inverse_edging;
            // fctidz truncation, then the native `value * 0x10101 - 0x1000000`.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "native fctidz keeps the low word of the truncated integer"
            )]
            let byte = scaled.trunc() as i64 as u32;
            let word = byte.wrapping_mul(0x0001_0101).wrapping_sub(0x0100_0000);
            let [alpha, red, green, blue] = word.to_be_bytes();
            texels.extend_from_slice(&[red, green, blue, alpha]);
        }
        texels
    }
}

/// The sun vector normalized with w zeroed.
fn normalized_light([x, y, z]: [f32; 3]) -> [f32; 4] {
    let inverse_length = 1.0 / z.mul_add(z, y.mul_add(y, x * x)).sqrt();
    [
        x * inverse_length,
        y * inverse_length,
        z * inverse_length,
        0.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2x2 map over x, z in [0, 100]: +x, +y, +z and a grey texel.
    fn sun_map() -> SunDirectionMap {
        SunDirectionMap {
            width: 2,
            height: 2,
            pixels: vec![
                [255, 128, 128],
                [128, 255, 128],
                [128, 128, 255],
                [64, 64, 64],
            ],
            scale: [1.0 / 100.0, 1.0 / 100.0],
            offset: [0.0, 0.0],
        }
    }

    fn default_fog() -> FogAttributes {
        FogAttributes {
            colour: [0.067_308, 0.133_73, 0.269_231, 1.0],
            power: 1.4,
            start: 50.0,
            strength: 1.0,
            end: 300.0,
        }
    }

    fn lighting(envattributes: [[f32; 4]; 7]) -> EnvironmentLighting {
        EnvironmentLighting {
            postfx: std::sync::Arc::new(crate::render_postfx::Assets {
                cubes: BTreeMap::new(), blooms: BTreeMap::new(), exposures: BTreeMap::new(),
                location: [0;3], matrix: [[0.;4];3],
                bloom_sample_offset: 0., hom: Default::default(), watermark_position: [0.;2], deck_ao: [0.; 4],
            }),
            use_world_sh_probes: true,
            use_worldpainter: true,
            default_irradiance:[[0.;4];9],
            envattributes,
            sun_vector: [-0.41, 0.4, 0.8],
            view_light_direction: [0., 0., 1., 0.],
            view_light_response: [1., 0.5, 0., 0.],
            view_light_minimum: 0.1,
            view_light_backlit: 0.2,
            sun_direction_map_requested: true,
            sun_map: sun_map(),
            csm: CsmAttributes {
                scale: 1.8,
                offset: [1.0, 1.0, 1.0],
                depth_range: 5.1,
            },
            world_shadow: WorldShadowAttributes {
                scale: 1.5,
                offset: [0.0; 3],
                cull: skate2_core::presentation_shadow_culling::WorldShadowCullSettings {
                    minimum_pixels: 0.0,
                    extra_radius: 5.0,
                },
                enabled: true,
            },
            scene_cull: skate2_core::presentation_scene_culling::IslandAreaCullSettings {
                min_dist: 120.0,
                min_area: 0.2,
                optimal_dist: 900.0,
            },
            proxy_draw_order: 0,
            location_fog_key: DEFAULT_COLLECTION_KEY,
            location_sky_key: DEFAULT_COLLECTION_KEY,
            fogs: BTreeMap::from([(DEFAULT_COLLECTION_KEY, default_fog())]),
            skies: BTreeMap::new(),
            material_params: BTreeMap::new(),
            m_params_classes: Vec::new(),
            character_attributes: Vec::new(),
            vehicle_lod_squared: [0.; 3],
            sky_blended: BTreeSet::new(),
        }
    }

    #[test]
    fn default_collection_key_is_the_attrib_hash_of_default() {
        assert_eq!(string_hash64("default"), DEFAULT_COLLECTION_KEY);
    }

    #[test]
    fn fresnel_table_spans_edging_to_facing() {
        let mut attributes = [[0.0; 4]; 7];
        attributes[5] = [8.0, 1.0, 5.0, 1.5];
        let table = lighting(attributes).fresnel_lookup();
        assert_eq!(table.len(), 1024);
        // u = 0 is grazing: (1 + 4 * 1^8) * 255 / 5 = 255.
        assert_eq!(&table[0..4], &[255, 255, 255, 255]);
        // u = 1 is facing: 1 * 255 / 5 = 51.
        assert_eq!(&table[1020..1024], &[51, 51, 51, 255]);
        assert!(
            table
                .chunks_exact(4)
                .all(|texel| texel[0] == texel[1] && texel[1] == texel[2])
        );
    }

    #[test]
    fn fog_constants_follow_native_lane_packing() {
        let (k1, colour) = default_fog().constants();
        assert!((k1[0] - 0.004).abs() < 1e-7);
        assert!((k1[1] + 0.2).abs() < 1e-6);
        assert_eq!(k1[2].to_bits(), 1.4_f32.to_bits());
        assert_eq!(colour[3].to_bits(), (-1.0_f32).to_bits());
    }

    #[test]
    fn sun_map_decodes_texel_bytes_and_clamps_to_its_rectangle() {
        let map = sun_map();
        // Lower-left texel exactly: (255, 128, 128) * 2/255 - 1.
        let corner = map.sample([0.0, 5.0, 0.0]);
        assert_eq!(corner[0], 255.0_f32.mul_add(TWO_OVER_255, -1.0));
        assert_eq!(corner[1], 128.0_f32.mul_add(TWO_OVER_255, -1.0));
        // Outside the rectangle clamps to the far corner, where both upper
        // indices collapse onto the last texel.
        assert_eq!(map.sample([-50.0, 0.0, -50.0]), corner);
        assert_eq!(
            map.sample([500.0, 0.0, 500.0]),
            [64.0_f32.mul_add(TWO_OVER_255, -1.0); 3]
        );
        // The map replaces the Vault vector only when the location asks.
        let mut lighting = lighting([[0.0; 4]; 7]);
        assert_ne!(
            lighting.light_direction_at([0.0; 3]),
            lighting.light_direction()
        );
        lighting.sun_direction_map_requested = false;
        assert_eq!(
            lighting.light_direction_at([0.0; 3]),
            lighting.light_direction()
        );
    }

    #[test]
    fn light_direction_is_unit_length_with_zero_w() {
        let direction = lighting([[0.0; 4]; 7]).light_direction();
        let length_squared = direction[..3].iter().map(|lane| lane * lane).sum::<f32>();
        assert!((length_squared - 1.0).abs() < 1e-6);
        assert_eq!(direction[3].to_bits(), 0);
    }

    #[test]
    fn view_lighting_uses_normalized_camera_z_and_retains_authored_lanes() {
        let light = lighting([[0.; 4]; 7]);
        assert_eq!(light.view_dot_light([0., 0., 20.]), [1.5, 0.1, 0.2, 0.]);
        assert_eq!(light.view_dot_light([0., 0., -20.]), [0.5, 0.1, 0.2, 0.]);
        assert_eq!(light.view_dot_light([20., 0., 0.]), [1.0, 0.1, 0.2, 0.]);
    }

    /// The downloadable content's render locations on the shipped vaults: the Parkade's own sun
    /// with the sun map and the WorldPainter off and its references among the content's
    /// collections, Maloof without one (the default location stays), and the city's default
    /// location keeping both on.
    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT and SKATE2_DLC_DIR"]
    fn downloadable_content_render_locations() {
        use skate2_formats::{mount::{ArchiveSource, MountedBig, SKATE2_ARCHIVES}, stfs::StfsPackage};
        let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES.iter() {
            mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
        }
        for path in std::fs::read_dir(std::env::var_os("SKATE2_DLC_DIR").unwrap()).unwrap().flatten().map(|entry| entry.path()) {
            let Ok(package) = StfsPackage::open(&path) else { continue };
            for entry in package.root_files() {
                mounts.push_dlc(MountedBig::open_source(ArchiveSource::Package(package.file(entry).unwrap())).unwrap());
            }
        }
        let city = EnvironmentLighting::load(&mounts).unwrap();
        assert!(city.use_worldpainter && city.sun_direction_map_requested);
        let vaults = skate2_core::dlc::vaults(&mounts);
        let paths: Vec<&str> = std::iter::once("data\\db\\skater.vlt").chain(vaults.iter().map(String::as_str)).collect();
        let db = AttribDb::load(&mounts, &paths).unwrap();
        let extended = city.with_vault_collections(&mounts, &db).unwrap();
        let parkade = extended.with_world_render_location(&db, string_hash64("dlc_parkade")).unwrap().unwrap();
        assert_eq!(parkade.sun_vector, [0x3ed0_9a24, 0x3f12_f976, 0x3f35_cfff].map(f32::from_bits));
        assert!(!parkade.sun_direction_map_requested && !parkade.use_worldpainter);
        assert!(parkade.use_world_sh_probes && parkade.world_shadow.enabled);
        assert_eq!(parkade.location_fog_key, 0xbe87_1da5_89aa_e0e9);
        assert!(parkade.fogs.contains_key(&parkade.location_fog_key), "the content's fog joins the blends");
        assert!(parkade.skies.contains_key(&parkade.location_sky_key));
        assert!(parkade.postfx.exposures.contains_key(&parkade.postfx.location[1]));
        assert!(parkade.postfx.blooms.contains_key(&parkade.postfx.location[2]));
        // dlc_default's probe, which the Parkade inherits across the vaults.
        assert_eq!(parkade.default_irradiance[0][0], f32::from_bits(0x3dcc_cccd));
        assert!(extended.with_world_render_location(&db, string_hash64("dlc_maloof_money_cup")).unwrap().is_none());
        for name in ["dlc_elementary_school", "dlc_community_center", "dlc_rob_fantasy_park"] {
            assert!(extended.with_world_render_location(&db, string_hash64(name)).unwrap().is_some(), "{name}");
        }
    }
}
