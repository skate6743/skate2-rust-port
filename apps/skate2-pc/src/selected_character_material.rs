//! Bind selected character materials to the shared native material shader paths.
use crate::gpu_renderer::{
    CameraRelativeClip, GpuObject, GpuRenderer, RenderResult,
    character::{CharacterGhostState, CharacterShaderFamily},
    character_palette::{CharacterMeshAdmission, SelectedCharacterGpuBinding},
};
use skate2_core::character::{
    material_admission::CharacterMaterialAdmission, palette::CharacterPalette,
};
use skate2_formats::mount::MountSet;
use std::collections::BTreeSet;

/// Builds real GPU objects; caller retains returned bindings beside stable object
/// indices in its existing window-owned object vector.
pub(crate) fn create_objects(
    renderer: &GpuRenderer,
    mounts: &MountSet,
    palette: &CharacterPalette,
    materials: &CharacterMaterialAdmission,
    camera: &CameraRelativeClip,
    ghost: CharacterGhostState,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
) -> RenderResult<Vec<(SelectedCharacterGpuBinding, GpuObject)>> {
    // `CreateEffectListFromMaterialKey` 0x82e203f8: each material's
    // `DrawOrder` RefSpec GUID is the first `InstanceSortKey` byte.
    let keys = materials
        .meshes()
        .map(|(part, mesh)| Ok(materials.view(palette, part, mesh)?.material_key))
        .collect::<RenderResult<BTreeSet<_>>>()?;
    let bindings = crate::load_material_attribute_bindings(mounts, &keys)?;
    let mut output = Vec::new();
    for (part, mesh) in materials.meshes() {
        let view = materials.view(palette, part, mesh)?;
        let family = family(view.effect)?;
        let order = bindings
            .get(&view.material_key)
            .ok_or("character material has no attribute binding")?
            .draw_order;
        let draw_order = u8::try_from(order)
            .map_err(|_| format!("character draw order {order} is not a sort-key byte"))?;
        let texture = renderer.upload_native_texture(view.texture)?;
        // PC port: a linked Skate 3 data folder's clothing can leave out the alpha map its
        // program samples (`character_material_admission`): hair drawn whole, CAS skin, face and
        // shift with no tattoo regions.
        let alpha = match view.alpha {
            Some(alpha) => Some(renderer.upload_native_texture(alpha)?),
            None if matches!(view.effect, "defaulthair" | "defaulthair_ropa" | "cac_hair" | "cac_hair_ropa") => {
                Some(renderer.upload_native_texture(&crate::render_inspection::solid_texture("hair alpha", [255; 4]))?)
            }
            None if matches!(view.effect, "cacstamp_skin" | "cacstamp_face" | "cacstamp_shift") => {
                Some(renderer.upload_native_texture(&crate::render_inspection::solid_texture("CAS alpha", [0; 4]))?)
            }
            None => None,
        };
        let object = renderer.create_selected_character_object(
            palette,
            materials,
            CharacterMeshAdmission {
                part,
                mesh,
                family,
                draw_order,
            },
            camera,
            &texture,
            if family.is_hair() {
                alpha.as_ref()
            } else {
                None
            },
            ghost,
            living_world_constants(&view, environment, frame)?,
        )?;
        let (binding, mut object) = object.into_parts();
        if family == CharacterShaderFamily::LivingWorld {
            renderer.bind_living_world_lighting(&mut object, &texture, &cas_inputs(&view, environment, frame, irradiance, camera)?)?;
        } else if let Some(mode) = crate::gpu_renderer::character::cas::mode(view.effect) {
            if std::env::var_os("SKATE2_DIAGNOSTIC_CAS_TEXTURES").is_some() {
                log_cas_textures(&view);
            }
            // PC port: a linked Skate 3 data folder's clothing can leave a map out
            // (`character_material_admission`): a flat normal (the CAS programs read its alpha
            // and green), no decal or dirt, and skin's tattoo layer white.
            let normal = match view.normal {
                Some(t) => renderer.upload_native_texture(t)?,
                None => renderer.upload_native_texture(&crate::render_inspection::solid_texture("CAS normal", [128, 128, 128, 128]))?,
            };
            let decal = match view.decal {
                Some(t) => Some(renderer.upload_native_texture(t)?),
                None if mode == 1. => Some(renderer.upload_native_texture(&crate::render_inspection::solid_texture("CAS tattoo", [255; 4]))?),
                None => Some(renderer.upload_native_texture(&crate::render_inspection::solid_texture("CAS decal", [0; 4]))?),
            };
            let decal2 = match view.decal2 {
                Some(t) => Some(renderer.upload_native_texture(t)?),
                None => Some(renderer.upload_native_texture(&crate::render_inspection::solid_texture("CAS dirt", [0; 4]))?),
            };
            let inputs = cas_inputs(&view, environment, frame, irradiance, camera)?;
            renderer.bind_cas_surface(
                &mut object,
                &view.geometry.geometry,
                &normal,
                alpha.as_ref(),
                decal.as_ref(),
                decal2.as_ref(),
                &inputs,
                cas_custom(&view, mode),
            )?;
        } else {
            let normal=renderer.upload_native_texture(view.normal.ok_or("character normal channel absent")?)?;
            renderer.bind_character_surface(&mut object, &view.geometry.geometry, &normal, alpha.as_ref(),
                &cas_inputs(&view, environment, frame, irradiance, camera)?)?;
        }
        renderer.set_character_fog(&object, frame)?;
        output.push((binding, object));
    }
    Ok(output)
}
pub(crate) fn update_object(
    renderer: &GpuRenderer,
    binding: &SelectedCharacterGpuBinding,
    object: &mut GpuObject,
    palette: &CharacterPalette,
    materials: &CharacterMaterialAdmission,
    camera: &CameraRelativeClip,
    ghost: CharacterGhostState,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
) -> RenderResult<()> {
    let (part, mesh) = binding.part_mesh();
    let view = materials.view(palette, part, mesh)?;
    renderer.update_selected_character_binding(
        binding,
        object,
        palette,
        materials,
        camera,
        ghost,
        living_world_constants(&view, environment, frame)?,
    )?;
    if let Some(mode) = crate::gpu_renderer::character::cas::mode(view.effect) {
        renderer.update_character_cas_constants(
            object,
            &cas_inputs(&view, environment, frame, irradiance, camera)?,
            cas_custom(&view, mode),
        )?;
    } else {
        renderer.update_character_stamp(object,
            &cas_inputs(&view, environment, frame, irradiance, camera)?)?;
    }
    renderer.set_character_fog(object, frame)?;
    Ok(())
}
/// Diagnostic (`SKATE2_DIAGNOSTIC_CAS_TEXTURES`): a CAS material's maps, each with its format, size
/// and the mean of its largest level's alpha and alpha squared (DXT5 decoded, opaque for DXT1).
fn log_cas_textures(view: &skate2_core::character::material_admission::CharacterMaterialView<'_>) {
    fn describe(texture: Option<&skate2_core::texture::NativeTexture>) -> String {
        let Some(texture) = texture else { return "none".into() };
        let alphas: Vec<f64> = texture.linear_levels.iter().find(|level| level.level == 0).map_or_else(Vec::new, |level| {
            match texture.pixel_format {
                skate2_formats::material::PIXEL_FORMAT_DXT5 | skate2_formats::material::PIXEL_FORMAT_LIN_DXT5 => {
                    level.data.chunks_exact(16).flat_map(dxt5_alphas).collect()
                }
                skate2_formats::material::PIXEL_FORMAT_A8R8G8B8 => level.data.chunks_exact(4).map(|texel| f64::from(texel[3]) / 255.).collect(),
                _ => vec![1.0],
            }
        });
        let count = alphas.len().max(1) as f64;
        let mean = alphas.iter().sum::<f64>() / count;
        let squares = alphas.iter().map(|alpha| alpha * alpha).sum::<f64>() / count;
        format!("{} {:#x} {}x{} alpha {mean:.3} squared {squares:.4}", texture.name, texture.pixel_format, texture.width, texture.height)
    }
    /// A DXT5 block's sixteen alphas (0 to 1).
    fn dxt5_alphas(block: &[u8]) -> [f64; 16] {
        let (a0, a1) = (f64::from(block[0]), f64::from(block[1]));
        let palette = if a0 > a1 {
            [a0, a1, (6. * a0 + a1) / 7., (5. * a0 + 2. * a1) / 7., (4. * a0 + 3. * a1) / 7., (3. * a0 + 4. * a1) / 7., (2. * a0 + 5. * a1) / 7., (a0 + 6. * a1) / 7.]
        } else {
            [a0, a1, (4. * a0 + a1) / 5., (3. * a0 + 2. * a1) / 5., (2. * a0 + 3. * a1) / 5., (a0 + 4. * a1) / 5., 0., 255.]
        };
        let bits = u64::from_le_bytes([block[2], block[3], block[4], block[5], block[6], block[7], 0, 0]);
        std::array::from_fn(|texel| palette[((bits >> (3 * texel)) & 7) as usize] / 255.)
    }
    eprintln!(
        "cas textures: {} material {:#018x}/{:#018x} diffuse [{}] normal [{}] alpha [{}] decal [{}] decal2 [{}]",
        view.effect,
        view.material_key.0,
        view.material_key.1,
        describe(Some(view.texture)),
        describe(view.normal),
        describe(view.alpha),
        describe(view.decal),
        describe(view.decal2),
    );
}

fn family(effect: &str) -> RenderResult<CharacterShaderFamily> {
    match effect {
        "defaultcharacter" | "cacstamp_cloth" | "cacstamp_skin" | "cacstamp_face"
        | "cacstamp_shift" => Ok(CharacterShaderFamily::Character),
        "defaulthair" | "cac_hair" | "cac_hair_ropa" => Ok(CharacterShaderFamily::Hair),
        "defaulthair_ropa" => Ok(CharacterShaderFamily::HairRopa),
        "defaultcharacter_ropa" | "cacstamp_cloth_ropa" => Ok(CharacterShaderFamily::Ropa),
        "defaultlivingworld" => Ok(CharacterShaderFamily::LivingWorld),
        other => Err(format!("unsupported actual character material effect {other:?}").into()),
    }
}

/// `i_colorize_red`/`i_colorize_blue` of a skater mesh with the pedestrian effect: a plain
/// `SkaterPresEntity` binds neither (`BindConstants` 0x827dcf50) and `cScope::DispatchParamsHelper`
/// 0x82e14aa0 uploads only bound params, so the native register keeps the previous draw's
/// value. The port uses the `ColorizedSkaterPresEntity` constructor's colour (0x827e5e20).
const UNBOUND_COLORIZE: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

/// The `defaultlivingworld` constants of a skater mesh: the colorize pair, the fog, then the
/// material gain, the exposure and the `m_params[5].w`/`[7].w` factors.
fn living_world_constants(
    view: &skate2_core::character::material_admission::CharacterMaterialView<'_>,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
) -> RenderResult<Option<[[f32; 4]; 5]>> {
    if view.effect != "defaultlivingworld" {
        return Ok(None);
    }
    let rows = environment
        .material_parameter_array(view.material_key.0, view.material_key.1)
        .and_then(|rows| rows.get(..8))
        .ok_or("livingworld eight material rows absent")?;
    let exposure = environment.frame_character_attributes(frame)?[2][2];
    Ok(Some([
        UNBOUND_COLORIZE,
        UNBOUND_COLORIZE,
        frame.fog_k1,
        frame.fog_colour,
        [rows[0][1], exposure, rows[5][3], rows[7][3]],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_mapping_has_no_substitution() {
        assert_eq!(
            family("defaultcharacter").unwrap(),
            CharacterShaderFamily::Character
        );
        assert_eq!(family("defaulthair").unwrap(), CharacterShaderFamily::Hair);
        assert_eq!(
            family("defaultcharacter_ropa").unwrap(),
            CharacterShaderFamily::Ropa
        );
        assert!(family("default").is_err());
        assert!(family("marquee_cloth").is_err());
    }
}

fn cas_custom(
    view: &skate2_core::character::material_admission::CharacterMaterialView<'_>,
    mode: f32,
) -> [[f32; 4]; 5] {
    [
        view.colours[0],
        view.colours[1],
        view.tattoos[0],
        view.tattoos[1],
        [mode, 0., skin_specular_mask(view, mode), 0.],
    ]
}

/// The specular mask `cacstamp_skin` reads from its diffuse's alpha (squared), as Skate 2's skin
/// textures hold it on average (`SKATE2_DIAGNOSTIC_CAS_TEXTURES`'s mean of alpha squared over its
/// arms' and legs' diffuse maps).
const SKATE2_SKIN_SPECULAR_MASK: f32 = 0.038;

/// PC port: the specular mask a skin material takes in place of its diffuse's alpha, or 0 for its
/// own. A linked Skate 3 data folder's arms and legs (`crate::skate3`) keep no mask in their
/// diffuse (DXT1, opaque), which Skate 2's program reads as the whole highlight on every texel:
/// they take Skate 2's skin's, so they shine as Skate 2's arms and legs do.
fn skin_specular_mask(view: &skate2_core::character::material_admission::CharacterMaterialView<'_>, mode: f32) -> f32 {
    use skate2_formats::material::{PIXEL_FORMAT_DXT1, PIXEL_FORMAT_R5G6B5};
    let skin = mode == 1.;
    // SKATE2_DIAGNOSTIC_SKIN_MASK_OFF: every skin takes its mask from its diffuse alpha, for
    // comparison.
    if std::env::var_os("SKATE2_DIAGNOSTIC_SKIN_MASK_OFF").is_some() {
        return 0.;
    }
    if skin && matches!(view.texture.pixel_format, PIXEL_FORMAT_DXT1 | PIXEL_FORMAT_R5G6B5) { SKATE2_SKIN_SPECULAR_MASK } else { 0. }
}
fn cas_inputs(
    view: &skate2_core::character::material_admission::CharacterMaterialView<'_>,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
    camera: &CameraRelativeClip,
) -> RenderResult<crate::gpu_renderer::character::stamp::Inputs> {
    Ok(crate::gpu_renderer::character::stamp::Inputs {
        material: environment
            .material_parameter_array(view.material_key.0, view.material_key.1)
            .and_then(|rows| rows.get(..7))
            .ok_or("CAS character seven material rows absent")?
            .try_into()?,
        irradiance,
        attributes: environment.frame_character_attributes(frame)?,
        light: frame.light_direction,
        csm: frame.shadows.globals(),
        camera: camera.camera,
        material_lod: 0.,
    })
}
