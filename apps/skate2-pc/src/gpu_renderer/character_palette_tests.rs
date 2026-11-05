use super::*;
use skate2_core::{character::*, presentation::*};
use skate2_formats::render::{AxisAlignedBox, NativeMeshGeometry};
use std::sync::Arc;

fn identity() -> [f32; 16] {
    let mut m = [0.; 16];
    for i in [0, 5, 10, 15] {
        m[i] = 1.;
    }
    m
}
fn scene() -> Arc<CharacterScene> {
    let bounds = AxisAlignedBox {
        minimum: [0.; 4],
        maximum: [1.; 4],
    };
    Arc::new(CharacterScene {
        customizations: None,
        schema_name: "test".into(),
        category: 0,
        textures: vec![],
        components: vec![CharacterComponentAsset {
            name: "body".into(),
            models: vec![CharacterModelAsset {
                asset_id: 1,
                lods: vec![CharacterLodAsset {
                    level: 0,
                    asset_id: 2,
                    arena_asset_id: 3,
                    loaded_asset_id: 3,
                    entry_path: "test.rx2".into(),
                    material_instances: vec![],
                    ropa_effects: None,
                    ropa_bindings:Vec::new(),
                    arena: PresentationArena {
                        textures: Vec::new(),
                        id: 3,
                        materials: vec![],
                        meshes: vec![PresentationMesh {
                            dictionary_index: 0,
                            bounding_box: bounds,
                            material: None,
                            skin_palette: vec![0],
                            geometry: NativeMeshGeometry {
                                positions: vec![[2., 3., 4.]],
                                position_format: 0,
                                attributes: vec![],
                                indices: vec![],
                                draws: vec![],
                            },
                        }],
                        models: vec![PresentationModel {
                            dictionary_index: 1,
                            bounding_box: bounds,
                            island_bounding_boxes: Vec::new(),
                            island_areas: Vec::new(),
                            total_bone_count: 1,
                            named_bone_count: 1,
                            inverse_bind_pose: vec![identity()],
                            bone_names: vec!["hips".into()],
                            meshes: vec![PresentationModelMesh {
                                mesh_index: 0,
                                material: None,
                            }],
                        }],
                        instances: vec![PresentationInstance {
                            guid: 1,
                            local_guid: 1,
                            name: "body".into(),
                            transform: identity(),
                            bounding_box_min: [0.; 3],
                            bounding_box_max: [1.; 3],
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
fn adapter_borrows_exact_published_rows_and_same_original_geometry() {
    let scene = scene();
    let mut palette = CharacterPalette::new(
        Arc::clone(&scene),
        &CharacterHierarchy {
            bone_names: vec!["hips".into()],
        },
        0,
    )
    .unwrap();
    let admission = CharacterMeshAdmission {
        part: 0,
        mesh: 0,
        family: CharacterShaderFamily::Ropa,
        draw_order: 9,
    };
    let clip = CameraRelativeClip {
        clip_from_camera: identity(),
        camera: [0.0; 3],
    };
    let visible = CharacterGhostState {
        opacity: 1.0,
        ghosted: false,
    };
    assert!(pose_rows(&palette, admission, &clip, None, None, visible, None).is_err());
    let mut matrix = identity();
    matrix[12] = 17.;
    palette.update_bone_transforms(&[matrix]).unwrap();
    palette.pack_for_upload(identity()).unwrap();
    let pose = pose_rows(&palette, admission, &clip, None, None, visible, None).unwrap();
    assert_eq!(pose.ghost, visible);
    assert_eq!(
        pose.rows.as_ptr(),
        palette.mesh_rows(0, 0).unwrap().rows.as_ptr()
    );
    assert_eq!(pose.rows[0][3].to_bits(), 17_f32.to_bits());
    assert!(pose.ropa_vertex_params_x.is_none());
    assert!(std::ptr::eq(
        geometry(&palette, 0, 0).unwrap(),
        &raw const scene.components[0].models[0].lods[0].arena.meshes[0]
    ));
}
#[test]
fn gpu_binding_rejects_different_palette_even_when_scene_is_shared() {
    let scene = scene();
    let hierarchy = CharacterHierarchy {
        bone_names: vec!["hips".into()],
    };
    let first = CharacterPalette::new(Arc::clone(&scene), &hierarchy, 0).unwrap();
    let second = CharacterPalette::new(scene, &hierarchy, 0).unwrap();
    validate_palette(&first.identity(), &first).unwrap();
    assert!(validate_palette(&first.identity(), &second).is_err());
}

#[test]
fn moved_object_binding_requires_its_exact_allocation_token() {
    let original = Arc::new(());
    let retained = Arc::clone(&original);
    let other = Arc::new(());
    validate_object_identity(&retained, Some(&original)).unwrap();
    assert!(validate_object_identity(&retained, Some(&other)).is_err());
    assert!(validate_object_identity(&retained, None).is_err());
}
