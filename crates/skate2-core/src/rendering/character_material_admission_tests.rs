use super::*;
use crate::attribute_runtime::tests::{
    collection, fixture_text_pointer, graph, set_dependency_bytes,
};
use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};
#[test]
fn native_suffix_assertion_and_checked_scratch_boundary_are_admitted_explicitly() {
    assert!(validate_material_name("character.default_skin").is_ok());
    assert!(validate_material_name("character.x").is_err());
    assert!(validate_material_name("character.\0suffix").is_err());
    assert!(validate_material_name(&format!("{}.cloth", "a".repeat(247))).is_err());
}
#[test]
fn diffuse_projection_ignores_metadata_and_retains_last_scope_write() {
    use crate::presentation::{PresentationMaterial, PresentationMaterialChannel};
    let channel = |guid| PresentationMaterialChannel {
        shader_input: "diffuse".into(),
        flags: 0,
        image_channel: 0,
        value: PresentationMaterialValue::Raster {
            guid,
            stream_name: String::new(),
            resident_texture: None,
        },
    };
    let mut material = PresentationMaterial {
        flags: 0,
        effect: None,
        channels: vec![channel(10), channel(11), channel(12), channel(13)],
    };
    assert_eq!(diffuse_guid(&material).unwrap(), 13);
    material.channels.last_mut().unwrap().flags = 2;
    assert!(
        diffuse_guid(&material).is_err(),
        "late scalar does not revive earlier raster"
    );
    material.channels.truncate(2);
    assert!(
        diffuse_guid(&material).is_err(),
        "metadata is not a sampled texture"
    );
}
fn ropa_vault() -> Arc<AttributeRuntimeVault> {
    let class = 0x91bc_a669_3efc_9aa7;
    let mut vault = graph(class, vec![]);
    collection(&mut vault, class, 0x2275_b900_5016_6969, 0, Some(0), vec![]);
    let mut bytes = vec![0; 256];
    bytes[32..46].copy_from_slice(b"tutorial_male\0");
    set_dependency_bytes(&mut vault, 1, bytes);
    fixture_text_pointer(&mut vault, 1, 0, 32);
    Arc::new(vault)
}
fn empty_mesh_scene() -> Arc<CharacterScene> {
    use crate::character::{CharacterComponentAsset, CharacterLodAsset, CharacterModelAsset};
    use crate::presentation::{PresentationArena, PresentationInstance, PresentationModel};
    let bounds = skate2_formats::render::AxisAlignedBox {
        minimum: [0.; 4],
        maximum: [0.; 4],
    };
    Arc::new(CharacterScene {
        customizations: None,
        schema_name: "tutorial_male".into(),
        category: 0,
        textures: vec![],
        components: vec![CharacterComponentAsset {
            name: "Arm".into(),
            models: vec![CharacterModelAsset {
                asset_id: 1,
                lods: vec![CharacterLodAsset {
                    level: 0,
                    asset_id: 2,
                    arena_asset_id: 3,
                    loaded_asset_id: 3,
                    entry_path: "fixture".into(),
                    material_instances: vec![],
                    ropa_effects: None,
                    ropa_bindings:Vec::new(),
                    arena: PresentationArena {
                        textures: Vec::new(),
                        id: 3,
                        materials: vec![],
                        meshes: vec![],
                        models: vec![PresentationModel {
                            dictionary_index: 0,
                            bounding_box: bounds,
                            island_bounding_boxes: Vec::new(),
                            island_areas: Vec::new(),
                            total_bone_count: 0,
                            named_bone_count: 0,
                            inverse_bind_pose: vec![],
                            bone_names: vec![],
                            meshes: vec![],
                        }],
                        instances: vec![PresentationInstance {
                            guid: 2,
                            local_guid: 2,
                            name: "Arm".into(),
                            transform: [0.; 16],
                            bounding_box_min: [0.; 3],
                            bounding_box_max: [0.; 3],
                            render_model: Some(0),
                        }],
                        anti_frustums: Vec::new(),
                        emitter_waypoints: Vec::new(),
                        visual_indicators: Vec::new(),
                    },
                }],
            }],
        }],
    })
}
#[test]
fn material_binding_rejects_foreign_palette_and_scene_before_mesh_reads() {
    let scene = empty_mesh_scene();
    let mut finalized = FinalizedRecipeScene::new(Arc::clone(&scene));
    finalized.try_finalize(3).unwrap();
    let hierarchy = crate::character::CharacterHierarchy {
        bone_names: vec!["Hips".into()],
    };
    let first = CharacterPalette::new(Arc::clone(&scene), &hierarchy, 0).unwrap();
    let second = CharacterPalette::new(scene, &hierarchy, 0).unwrap();
    let materials = CharacterMaterialAdmission::try_new(
        &finalized,
        &first,
        ropa_vault(),
        InputCvarReferences::new_static_initialized(),
    )
    .unwrap();
    assert!(
        matches!(
            materials.view(&first, 0, 0),
            Err(CharacterMaterialError::Missing(
                "constructed presentation entity"
            ))
        ),
        "even a non-ropa view must wait for its actual header"
    );
    assert!(matches!(
        materials.view(&second, 0, 0),
        Err(CharacterMaterialError::ForeignOwner)
    ));
    let foreign = CharacterPalette::new(empty_mesh_scene(), &hierarchy, 0).unwrap();
    assert!(matches!(
        CharacterMaterialAdmission::try_new(
            &finalized,
            &foreign,
            ropa_vault(),
            InputCvarReferences::new_static_initialized()
        ),
        Err(CharacterMaterialError::ForeignOwner)
    ));
}
#[test]
fn constructor_consumes_full_header_peer_and_locality_not_low_word_or_backend() {
    let mut scene = empty_mesh_scene();
    Arc::get_mut(&mut scene).unwrap().schema_name = "other".into();
    let mut finalized = FinalizedRecipeScene::new(Arc::clone(&scene));
    finalized.try_finalize(3).unwrap();
    let palette = CharacterPalette::new(
        scene,
        &crate::character::CharacterHierarchy {
            bone_names: vec!["Hips".into()],
        },
        0,
    )
    .unwrap();
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut header = [0; 208];
    header[192..196].copy_from_slice(&0x8000_u32.to_be_bytes());
    for (peer, allowed) in [(i64::from(u32::MAX), true), (-1, false)] {
        header[160..168].copy_from_slice(&peer.to_be_bytes());
        let mut material =
            CharacterMaterialAdmission::try_new(&finalized, &palette, ropa_vault(), refs.clone())
                .unwrap();
        let result = material.try_construct_from_header(&header);
        if allowed {
            result.unwrap();
            assert_eq!(material.cloth_disabled(), Some(true));
        } else {
            assert!(matches!(
                result,
                Err(CharacterMaterialError::EnabledClothPending)
            ));
        }
    }
}
#[test]
fn ropa_schema_exclusion_and_nonlocal_gate_do_not_sample_unlinked_cvar() {
    let vault = ropa_vault();
    let refs = InputCvarReferences::new_static_initialized();
    for (schema, local, disabled) in [
        ("tutorial_male", true, false),
        ("other", false, false),
        ("other", true, true),
    ] {
        let mut state = ropa::PresentationRopaState::default();
        state
            .try_construct(schema, local, disabled, &vault, &refs)
            .unwrap();
        assert_eq!(state.cloth_disabled(), Some(true));
        assert_eq!(state.vertex_x().unwrap().to_bits(), 1_f32.to_bits());
    }
}
#[test]
fn enabled_cloth_stops_before_x_store_and_cvar_false_admits_actual_constructor_store() {
    let vault = ropa_vault();
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut enabled = ropa::PresentationRopaState::default();
    assert!(matches!(
        enabled.try_construct("other", true, false, &vault, &refs),
        Err(ropa::PresentationRopaError::EnabledClothPending)
    ));
    assert_eq!(enabled.cloth_disabled(), Some(false));
    assert!(enabled.vertex_x().is_err());
    manager
        .try_deserialize_startup(Some(b"[Config]\r\ncv_EnableRopa=0\r\n"))
        .unwrap();
    let mut disabled = ropa::PresentationRopaState::default();
    disabled
        .try_construct("other", true, false, &vault, &refs)
        .unwrap();
    assert_eq!(disabled.cloth_disabled(), Some(false));
    assert_eq!(disabled.vertex_x().unwrap().to_bits(), 1_f32.to_bits());
}
#[test]
fn a_frozen_copy_is_its_own_entity_without_cloth() {
    let scene = empty_mesh_scene();
    let mut finalized = FinalizedRecipeScene::new(Arc::clone(&scene));
    finalized.try_finalize(3).unwrap();
    let hierarchy = crate::character::CharacterHierarchy {
        bone_names: vec!["Hips".into()],
    };
    let palette = CharacterPalette::new(scene, &hierarchy, 0).unwrap();
    let materials = CharacterMaterialAdmission::try_new(
        &finalized,
        &palette,
        ropa_vault(),
        InputCvarReferences::new_static_initialized(),
    )
    .unwrap();
    let copy = materials.try_frozen_copy().unwrap();
    assert_eq!(materials.cloth_disabled(), None, "the source is not constructed");
    assert_eq!(copy.cloth_disabled(), Some(true));
    assert_eq!(copy.ropa.vertex_x().unwrap().to_bits(), 1_f32.to_bits());
    assert!(copy.cloth.is_empty());
    assert!(
        matches!(copy.view(&palette, 0, 0), Err(CharacterMaterialError::Missing("mesh binding"))),
        "the copy binds the source's materials for the same palette"
    );
}
