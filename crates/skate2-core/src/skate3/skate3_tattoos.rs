//! PC port: Skate 2's tattoos on a linked Skate 3 data folder's arms and legs.
//!
//! Skate 2's `cacstamp_skin` program draws a tattoo where the skin's alpha map names a zone and
//! the mesh's second texture coordinate set is positive: the zone picks the skater's choice, and
//! that set places the choice's cell of the tattoo sheet (the material's `decal` map) on the skin.
//! Skate 3 remade its arms and legs. Their materials name no alpha map, their second set is laid
//! out for Skate 3's own decal, and their `decal` map is Skate 3's: under Skate 3's clothes a
//! skater's tattoos were not drawn.
//!
//! The body under them is the same, so each of Skate 3's arms and legs takes its tattoos from
//! Skate 2's limbs of its slot and gender. Every vertex is projected to the nearest point of those
//! limbs' surface: it takes the zone their alpha map names there, and that point's second set.
//! Skate 3 reshaped the limbs, though, and where the two surfaces part the nearest points of
//! neighbouring vertices cross, which folds a tattoo drawn from the projected set alone. A limb's
//! own second set lays the same tattooed skin out without folds: for each zone, the affine map
//! that takes the limb's own coordinates nearest the projected ones carries that layout onto
//! Skate 2's sheet, and the limb is drawn with it. A limb with no second set of its own keeps the
//! projected one.
//!
//! The limb's materials get an alpha map of their own, the zones drawn over the limb's first set,
//! and Skate 2's tattoo sheet as their decal. Skate 2's program then draws them as it draws its
//! own.
use super::tattoo_transfer::{Affine, Nearest, Surface, SurfaceTriangle, Uv, Zone, ZoneMap, ZonePainter, weighted};
use crate::{
    character::{CharacterComponentAsset, CharacterScene, CharacterTextureAsset},
    presentation::{PresentationMaterial, PresentationMaterialChannel, PresentationMaterialValue},
    texture::NativeTexture,
};
use skate2_formats::{
    cac_parts::{CacPartDb, SLOT_NAMES},
    hash::rw_hash64_string,
    material::{
        LinearTextureLevel, PIXEL_FORMAT_A8R8G8B8, PIXEL_FORMAT_DXT1, PIXEL_FORMAT_DXT5, PIXEL_FORMAT_LIN_DXT5, TextureEndian, TextureType,
    },
    mount::MountSet,
    recipe::CharacterCustomizations,
    render::{NativeMeshGeometry, NativeVertexAttribute, NativeVertexAttributeValues, VertexSemantic},
    skater_profile::CacPartId,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

/// The arm and leg slots (`SLOT_NAMES`).
const LIMB_SLOTS: [u8; 2] = [0, 1];
/// How far from Skate 2's limbs a vertex still takes their tattoos, in metres.
const REACH: f32 = 0.02;
/// The drawn alpha map's width and height in texels.
const MAP_SIZE: usize = 256;
/// A zone's own layout is carried onto the sheet when this many of its vertices agree on the
/// map, at least half of them, within this median miss (in sheet cells).
const FIT_PAIRS: usize = 12;
const FIT_MISS: f32 = 0.06;
/// The least miss a vertex is left out of a zone's map for.
const FIT_FLOOR: f32 = 0.01;
/// The sheet coordinate a carried layout stops at, short of the next cell.
const SHEET_EDGE: f32 = 0.998;

/// Skate 2's limbs of a slot and gender: their surface, the alpha maps it reads and the tattoo
/// sheet.
struct Reference {
    surface: Surface,
    maps: Vec<ZoneMap>,
    sheet: CharacterTextureAsset,
}

/// What [`fit_limbs`] gave a limb's model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FitStats {
    vertices: usize,
    /// Vertices within reach of Skate 2's limbs.
    near: usize,
    /// Vertices that took a place on the tattoo sheet.
    tattooed: usize,
    /// Zones whose layout is the limb's own, carried onto the sheet.
    carried: usize,
    /// Alpha map texels drawn from the limb's triangles, and those two zones claimed.
    drawn: usize,
    conflicts: usize,
}

/// Gives the arms and legs of `scene` whose skin names no alpha map (Skate 3's) Skate 2's
/// tattoos. `SKATE2_DIAGNOSTIC_S3_TATTOOS=off` leaves them as they are; any other value logs
/// what each limb took.
pub fn fit_limbs(mounts: &MountSet, category: &str, scene: &mut CharacterScene) {
    let diagnostic = std::env::var("SKATE2_DIAGNOSTIC_S3_TATTOOS").ok();
    if diagnostic.as_deref() == Some("off") {
        return;
    }
    let Some(customizations) = scene.customizations.clone() else { return };
    for slot in LIMB_SLOTS {
        let name = SLOT_NAMES[usize::from(slot)];
        let Some(component) = scene.components.iter().position(|component| component.name.eq_ignore_ascii_case(name)) else { continue };
        let Some(material) = unzoned_skin(&scene.components[component]) else { continue };
        let Some(reference) = reference(mounts, category, slot, material, &customizations) else { continue };
        let mut textures = Vec::new();
        for model in &mut scene.components[component].models {
            let Some((stats, zones)) = fit_model(model, &reference) else { continue };
            if diagnostic.is_some() {
                eprintln!(
                    "s3 tattoos: {name} model {:#018x}: {} of {} vertices near Skate 2's ({} triangles), {} on the sheet, {} zones in the limb's own layout, {} map texels drawn, {} claimed twice",
                    model.asset_id,
                    stats.near,
                    stats.vertices,
                    reference.surface.triangles().len(),
                    stats.tattooed,
                    stats.carried,
                    stats.drawn,
                    stats.conflicts,
                );
            }
            textures.push(zones);
        }
        if textures.is_empty() {
            continue;
        }
        textures.push(reference.sheet.clone());
        for texture in textures {
            if !scene.textures.iter().any(|held| held.source_asset_id == texture.source_asset_id) {
                scene.textures.push(texture);
            }
        }
    }
}

/// The skin material of a limb whose model materials name a decal map and no alpha map.
fn unzoned_skin(component: &CharacterComponentAsset) -> Option<u64> {
    let lod = component.models.iter().flat_map(|model| &model.lods).find(|lod| lod.arena.materials.iter().any(lacks_zones))?;
    lod.material_instances.first().and_then(|instance| instance.materials.first()).map(|material| material.asset_id)
}

fn lacks_zones(material: &PresentationMaterial) -> bool {
    raster(material, "decal").is_some() && raster(material, "alpha").is_none()
}

/// The texture a material's channel samples, as `character_material_admission` reads it.
fn raster(material: &PresentationMaterial, name: &str) -> Option<u64> {
    let channel = material.channels.iter().skip(2).rfind(|channel| channel.shader_input == name)?;
    match &channel.value {
        PresentationMaterialValue::Raster { guid, .. } if channel.flags & 2 == 0 => Some(*guid),
        _ => None,
    }
}

/// A limb mesh projected onto the reference: for each vertex the sheet coordinate and zone of
/// the nearest point, and the mesh's own second set where it places the vertex.
struct Projected {
    level: usize,
    mesh: usize,
    finest: bool,
    sheet: Vec<Uv>,
    zones: Vec<Zone>,
    own: Option<Vec<Option<Uv>>>,
}

/// Transfers the reference's tattoos onto every level of detail of `model`, and returns what it
/// took with the alpha map drawn for it. None, and the model untouched, when no vertex of its
/// finest level lies in a zone.
fn fit_model(model: &mut crate::character::CharacterModelAsset, reference: &Reference) -> Option<(FitStats, CharacterTextureAsset)> {
    let mut levels: Vec<usize> = (0..model.lods.len()).collect();
    levels.sort_by_key(|level| model.lods[*level].level);
    let mut stats = FitStats::default();
    let mut painter = ZonePainter::new(MAP_SIZE);
    let mut projected: Vec<Projected> = Vec::new();
    for (rank, &level) in levels.iter().enumerate() {
        let arena = &model.lods[level].arena;
        for (index, mesh) in arena.meshes.iter().enumerate() {
            if !mesh.material.and_then(|material| arena.materials.get(material)).is_some_and(lacks_zones) {
                continue;
            }
            let Some(first) = uv(&mesh.geometry, 0) else { continue };
            let nearest: Vec<Option<Nearest>> = mesh.geometry.positions.iter().map(|position| reference.surface.nearest(*position)).collect();
            let corners = |nearest: &Nearest| reference.surface.triangles()[nearest.triangle];
            let sheet: Vec<Uv> = nearest.iter().map(|nearest| nearest.as_ref().map_or([0.; 2], |nearest| weighted(corners(nearest).sheet_uv, nearest.weights))).collect();
            let zones: Vec<Zone> = nearest
                .iter()
                .map(|nearest| {
                    nearest.as_ref().map_or(0, |nearest| {
                        let triangle = corners(nearest);
                        reference.maps[triangle.zone_map].at(weighted(triangle.zone_uv, nearest.weights))
                    })
                })
                .collect();
            if rank == 0 {
                for triangle in triangle_indices(&mesh.geometry) {
                    painter.paint(triangle.map(|vertex| first[vertex]), shared_zone(triangle.map(|vertex| zones[vertex])));
                }
                stats.vertices += nearest.len();
                stats.near += nearest.iter().flatten().count();
            }
            let own = uv(&mesh.geometry, 1).map(|own| {
                let parked = parked(own);
                own.iter().map(|uv| (Some(*uv) != parked).then_some(*uv)).collect()
            });
            projected.push(Projected { level, mesh: index, finest: rank == 0, sheet, zones, own });
        }
    }
    stats.drawn = painter.drawn();
    stats.conflicts = painter.conflicts;
    let texels = painter.finish()?;
    let mut pairs: HashMap<Zone, Vec<(Uv, Uv)>> = HashMap::new();
    for mesh in projected.iter().filter(|mesh| mesh.finest) {
        let Some(own) = &mesh.own else { continue };
        for ((own, sheet), zone) in own.iter().zip(&mesh.sheet).zip(&mesh.zones) {
            if let Some(own) = own
                && *zone != 0
                && sheet[0] > 0.
                && sheet[1] > 0.
            {
                pairs.entry(*zone).or_default().push((*own, *sheet));
            }
        }
    }
    let carried: HashMap<Zone, Affine> = pairs
        .iter()
        .filter_map(|(zone, pairs)| {
            let (map, kept, miss) = Affine::fit_agreeing(pairs, FIT_FLOOR)?;
            (kept >= FIT_PAIRS && kept * 2 >= pairs.len() && miss <= FIT_MISS).then_some((*zone, map))
        })
        .collect();
    stats.carried = carried.len();
    let guid = rw_hash64_string(&format!("skate 3 tattoo zones {:016x}", model.asset_id));
    for mesh in projected {
        let sheet: Vec<Uv> = match &mesh.own {
            Some(own) => own
                .iter()
                .zip(&mesh.sheet)
                .zip(&mesh.zones)
                .map(|((own, sheet), zone)| match (own, carried.get(zone)) {
                    (Some(own), Some(map)) => map.apply(*own).map(|value| value.clamp(0., SHEET_EDGE)),
                    // The limb's own layout leaves this vertex bare.
                    (None, Some(_)) => [0.; 2],
                    (_, None) => *sheet,
                })
                .collect(),
            None => mesh.sheet,
        };
        if mesh.finest {
            stats.tattooed += sheet.iter().filter(|uv| uv[0] > 0. && uv[1] > 0.).count();
        }
        set_uv(&mut model.lods[mesh.level].arena.meshes[mesh.mesh].geometry, 1, sheet);
    }
    for lod in &mut model.lods {
        for material in lod.arena.materials.iter_mut().filter(|material| lacks_zones(material)) {
            for channel in material.channels.iter_mut().skip(2).filter(|channel| channel.shader_input == "decal") {
                if let PresentationMaterialValue::Raster { guid, resident_texture, .. } = &mut channel.value {
                    *guid = reference.sheet.source_asset_id;
                    *resident_texture = None;
                }
            }
            material.channels.push(PresentationMaterialChannel {
                shader_input: "alpha".to_owned(),
                flags: 0,
                image_channel: u16::MAX,
                value: PresentationMaterialValue::Raster { guid, stream_name: String::new(), resident_texture: None },
            });
        }
    }
    let name = format!("skate 3 tattoo zones {:#018x}", model.asset_id);
    Some((
        stats,
        CharacterTextureAsset { source_asset_id: guid, loaded_asset_id: guid, entry_path: name.clone(), texture: zone_texture(guid, name, texels) },
    ))
}

/// Where a second set parks the vertices it places nowhere: the one coordinate a tenth of them
/// or more share (Skate 3's limbs keep their bare skin at one corner).
fn parked(own: &[Uv]) -> Option<Uv> {
    let mut counts: HashMap<[u32; 2], usize> = HashMap::new();
    for uv in own {
        *counts.entry(uv.map(f32::to_bits)).or_default() += 1;
    }
    counts.into_iter().filter(|(_, count)| count * 10 >= own.len()).max_by_key(|(_, count)| *count).map(|(uv, _)| uv.map(f32::from_bits))
}

/// The zone a triangle's corners agree on: two of them, else the first that names one.
fn shared_zone(zones: [Zone; 3]) -> Zone {
    let named: Vec<Zone> = zones.into_iter().filter(|zone| *zone != 0).collect();
    match named[..] {
        [a, b, c] if b == c && a != b => b,
        [first, ..] => first,
        [] => 0,
    }
}

/// A drawn alpha map as a texture: uncompressed, one level.
fn zone_texture(guid: u64, name: String, texels: Vec<u8>) -> NativeTexture {
    let size = MAP_SIZE as u32;
    NativeTexture {
        dictionary_index: 0,
        guid,
        name,
        texture_type: TextureType::TwoDimensional,
        endian: TextureEndian::None,
        width: size,
        height: size,
        depth: 1,
        mip_levels: 1,
        pixel_format: PIXEL_FORMAT_A8R8G8B8,
        packed_mips: false,
        mip_data_offset: 0,
        encoded_data: Vec::new(),
        linear_levels: vec![LinearTextureLevel {
            level: 0,
            face: 0,
            width: size,
            height: size,
            width_in_blocks: size,
            height_in_blocks: size,
            bytes_per_block: 4,
            row_pitch: size * 4,
            data: texels,
        }],
    }
}

/// An alpha map's zones, from its top level.
fn zone_map(texture: &NativeTexture) -> Option<ZoneMap> {
    let level = texture.linear_levels.iter().find(|level| level.level == 0 && level.face == 0)?;
    let (width, height, pitch) = (level.width as usize, level.height as usize, level.row_pitch as usize);
    match texture.pixel_format {
        PIXEL_FORMAT_A8R8G8B8 => ZoneMap::from_bgra(width, height, pitch, &level.data),
        PIXEL_FORMAT_DXT1 => ZoneMap::from_blocks(width, height, pitch, 8, 0, &level.data),
        PIXEL_FORMAT_DXT5 | PIXEL_FORMAT_LIN_DXT5 => ZoneMap::from_blocks(width, height, pitch, 16, 8, &level.data),
        _ => None,
    }
}

fn uv(geometry: &NativeMeshGeometry, set: u8) -> Option<&[Uv]> {
    let attribute = geometry.attributes.iter().find(|attribute| attribute.semantic == VertexSemantic::TextureCoordinate(set))?;
    match &attribute.values {
        NativeVertexAttributeValues::Float2(values) if values.len() == geometry.positions.len() => Some(values),
        _ => None,
    }
}

fn set_uv(geometry: &mut NativeMeshGeometry, set: u8, values: Vec<Uv>) {
    let semantic = VertexSemantic::TextureCoordinate(set);
    match geometry.attributes.iter_mut().find(|attribute| attribute.semantic == semantic) {
        Some(attribute) => attribute.values = NativeVertexAttributeValues::Float2(values),
        None => {
            let original_format =
                geometry.attributes.iter().find(|attribute| attribute.semantic == VertexSemantic::TextureCoordinate(0)).map_or(0, |attribute| attribute.original_format);
            geometry.attributes.push(NativeVertexAttribute { semantic, original_format, values: NativeVertexAttributeValues::Float2(values) });
        }
    }
}

/// A mesh's triangles as vertex indices.
fn triangle_indices(geometry: &NativeMeshGeometry) -> Vec<[usize; 3]> {
    let mut out = Vec::new();
    for draw in &geometry.draws {
        let start = draw.start_index as usize;
        let Some(indices) = geometry.indices.get(start..start + draw.index_count as usize) else { continue };
        for corners in indices.chunks_exact(3) {
            let vertex = |index: u32| usize::try_from(i64::from(index) + i64::from(draw.base_vertex_index)).ok().filter(|vertex| *vertex < geometry.positions.len());
            if let (Some(a), Some(b), Some(c)) = (vertex(corners[0]), vertex(corners[1]), vertex(corners[2])) {
                out.push([a, b, c]);
            }
        }
    }
    out
}

type ReferenceKey = (String, u8, bool, u64);

/// The reference for a slot, gender and skin, read once.
fn reference(mounts: &MountSet, category: &str, slot: u8, material: u64, customizations: &CharacterCustomizations) -> Option<Arc<Reference>> {
    static REFERENCES: OnceLock<Mutex<HashMap<ReferenceKey, Option<Arc<Reference>>>>> = OnceLock::new();
    let references = REFERENCES.get_or_init(Mutex::default);
    let key = (category.to_owned(), slot, customizations.is_male, material);
    if let Some(found) = references.lock().ok()?.get(&key) {
        return found.clone();
    }
    // Read with the lock released: reading a limb builds a scene of its own.
    let read = match read_reference(mounts, category, slot, material, customizations) {
        Ok(reference) => reference.map(Arc::new),
        Err(error) => {
            eprintln!("skate 3 clothing: Skate 2's {} tattoos were not read: {error}", SLOT_NAMES[usize::from(slot)]);
            None
        }
    };
    references.lock().ok()?.insert(key, read.clone());
    read
}

fn skate2_parts(mounts: &MountSet) -> Result<Arc<CacPartDb>, Box<dyn std::error::Error>> {
    static PARTS: OnceLock<Arc<CacPartDb>> = OnceLock::new();
    if let Some(parts) = PARTS.get() {
        return Ok(Arc::clone(parts));
    }
    let parts = Arc::new(CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH)?)?);
    Ok(Arc::clone(PARTS.get_or_init(|| parts)))
}

/// Reads Skate 2's limbs of `slot` for the skater's gender, one scene for each model, the finest
/// level of each that places tattoos. None when none of them names an alpha map and a tattoo
/// sheet.
fn read_reference(
    mounts: &MountSet,
    category: &str,
    slot: u8,
    material: u64,
    customizations: &CharacterCustomizations,
) -> Result<Option<Reference>, Box<dyn std::error::Error>> {
    let parts = skate2_parts(mounts)?;
    let mut triangles = Vec::new();
    let mut maps: Vec<(u64, ZoneMap)> = Vec::new();
    let mut sheet = None;
    let mut reach = REACH;
    for limb in crate::edit_skater_record::limb_models(&parts, slot, customizations.is_male, material) {
        let worn: [CacPartId; 26] = std::array::from_fn(|index| if index == usize::from(slot) { limb } else { CacPartId::invalid() });
        let Ok(recipe) = crate::edit_skater_character::compose_recipe(&parts, &worn, customizations.clone()) else { continue };
        if !recipe.asset_requests(category).iter().all(|request| mounts.contains_path(&request.entry_path)) {
            continue;
        }
        let scene = crate::mounted_character_scene::own_character_scene_category(mounts, &recipe, category)?;
        for model in scene.components.iter().flat_map(|component| &component.models) {
            let Some(lod) = model.lods.iter().min_by_key(|lod| lod.level) else { continue };
            reach = REACH * super::head_fit::model_units_per_metre(&lod.arena.models);
            for mesh in &lod.arena.meshes {
                let Some(skin) = mesh.material.and_then(|material| lod.arena.materials.get(material)) else { continue };
                let texture = |name: &str| raster(skin, name).and_then(|guid| scene.textures.iter().find(|texture| texture.source_asset_id == guid));
                // A limb with no second set places no tattoo (Skate 2's forearm-down arm): over
                // the same skin as one that does, it would take the vertices nearest it.
                let (Some(alpha), Some(decal), Some(zone_uv), Some(sheet_uv)) =
                    (texture("alpha"), texture("decal"), uv(&mesh.geometry, 0), uv(&mesh.geometry, 1))
                else {
                    continue;
                };
                let zone_map = match maps.iter().position(|(guid, _)| *guid == alpha.source_asset_id) {
                    Some(held) => held,
                    None => {
                        let Some(map) = zone_map(&alpha.texture) else { continue };
                        maps.push((alpha.source_asset_id, map));
                        maps.len() - 1
                    }
                };
                sheet.get_or_insert_with(|| decal.clone());
                triangles.extend(triangle_indices(&mesh.geometry).into_iter().map(|triangle| SurfaceTriangle {
                    positions: triangle.map(|vertex| mesh.geometry.positions[vertex]),
                    zone_uv: triangle.map(|vertex| zone_uv[vertex]),
                    sheet_uv: triangle.map(|vertex| sheet_uv[vertex]),
                    zone_map,
                }));
            }
        }
    }
    let Some(sheet) = sheet else { return Ok(None) };
    if triangles.is_empty() {
        return Ok(None);
    }
    Ok(Some(Reference { surface: Surface::new(triangles, reach), maps: maps.into_iter().map(|(_, map)| map).collect(), sheet }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangle_takes_the_zone_two_corners_share_or_the_first_named() {
        assert_eq!(shared_zone([0, 0, 0]), 0);
        assert_eq!(shared_zone([0, 2, 0]), 2);
        assert_eq!(shared_zone([1, 2, 2]), 2);
        assert_eq!(shared_zone([1, 1, 2]), 1);
        assert_eq!(shared_zone([1, 2, 3]), 1);
        assert_eq!(shared_zone([0, 2, 1]), 2);
    }

    #[test]
    fn a_second_set_parks_its_bare_vertices_at_one_coordinate() {
        let mut own = vec![[0., 1.]; 30];
        own.extend((0..20).map(|vertex| [0.2 + vertex as f32 * 0.01, 0.7]));
        assert_eq!(parked(&own), Some([0., 1.]));
        let spread: Vec<Uv> = (0..50).map(|vertex| [vertex as f32 * 0.01, 0.5]).collect();
        assert_eq!(parked(&spread), None);
    }

    #[test]
    fn a_drawn_alpha_map_is_one_uncompressed_level() {
        let texture = zone_texture(7, "zones".to_owned(), vec![0; MAP_SIZE * MAP_SIZE * 4]);
        assert_eq!((texture.width, texture.height, texture.pixel_format), (256, 256, PIXEL_FORMAT_A8R8G8B8));
        assert_eq!(texture.linear_levels[0].row_pitch, 1024);
        assert_eq!(zone_map(&texture).map(|map| map.zones.len()), Some(MAP_SIZE * MAP_SIZE));
    }
}
