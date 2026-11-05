use super::*;
use crate::character::{CharacterComponentAsset, CharacterLodAsset, CharacterModelAsset};
use crate::presentation::PresentationArena;
fn component(name: &str, id: u64) -> CharacterComponentAsset {
    CharacterComponentAsset {
        name: name.into(),
        models: vec![CharacterModelAsset {
            asset_id: id,
            lods: vec![CharacterLodAsset {
                level: 0,
                asset_id: id,
                arena_asset_id: id + 100,
                loaded_asset_id: id + 200,
                entry_path: "fixture".into(),
                material_instances: vec![], ropa_effects: None,
                    ropa_bindings:Vec::new(),
                arena: PresentationArena {
                    textures: Vec::new(),
                    id: 0,
                    materials: vec![],
                    meshes: vec![],
                    models: vec![],
                    instances: vec![PresentationInstance {
                        guid: id,
                        local_guid: id,
                        name: name.into(),
                        transform: [0.; 16],
                        bounding_box_min: [0.; 3],
                        bounding_box_max: [0.; 3],
                        render_model: None,
                    }],
                    anti_frustums: Vec::new(),
                    emitter_waypoints: Vec::new(),
                    visual_indicators: Vec::new(),
                },
            }],
        }],
    }
}
#[test]
fn clone_three_keeps_same_instance_and_native_hash_order_excludes_beard() {
    let scene = Arc::new(CharacterScene {
        customizations: None, schema_name: "fixture".into(),
        category: 0,
        components: vec![
            component("Beard", 1),
            component("Rostral", 2),
            component("Arm", 3),
        ],
        textures: vec![],
    });
    let mut finalized = FinalizedRecipeScene::new(Arc::clone(&scene));
    finalized.try_finalize(3).unwrap();
    assert!(Arc::ptr_eq(finalized.scene().unwrap(), &scene));
    let instances = finalized.instances(0).unwrap().collect::<Vec<_>>();
    assert_eq!(instances.len(), 2);
    assert!(instances.iter().all(|instance| instance.name != "Beard"));
    assert!(
        instances
            .windows(2)
            .all(|pair| rw_hash32_string(&pair[0].name) < rw_hash32_string(&pair[1].name))
    );
    for instance in instances {
        assert!(scene.components.iter().any(|component| std::ptr::eq(
            instance,
            &raw const component.models[0].lods[0].arena.instances[0]
        )));
    }
}
#[test]
fn mismatched_registry_id_and_missing_lod_zero_do_not_publish_ready() {
    let mut part = component("Arm", 1);
    part.models[0].lods[0].asset_id = 2;
    let mut finalized = FinalizedRecipeScene::new(Arc::new(CharacterScene {
        customizations: None, schema_name: "fixture".into(),
        category: 0,
        components: vec![part],
        textures: vec![],
    }));
    assert_eq!(
        finalized.try_finalize(3),
        Err(RecipeFinalizeError::MissingInstance(2))
    );
    assert!(matches!(
        finalized.scene(),
        Err(RecipeFinalizeError::NotReady)
    ));
    let mut part = component("Arm", 1);
    part.models[0].lods[0].level = 1;
    let mut finalized = FinalizedRecipeScene::new(Arc::new(CharacterScene {
        customizations: None, schema_name: "fixture".into(),
        category: 0,
        components: vec![part],
        textures: vec![],
    }));
    assert_eq!(
        finalized.try_finalize(3),
        Err(RecipeFinalizeError::EmptyLodZero)
    );
    assert_eq!(finalized.resolved.len(), 1);
    assert_eq!(finalized.lods[1].len(), 1);
    assert!(matches!(
        finalized.scene(),
        Err(RecipeFinalizeError::NotReady)
    ));
}

#[test]
fn later_missing_registry_instance_retains_resolved_prefix() {
    let mut first = component("Arm", 1);
    first.models.push(CharacterModelAsset {
        asset_id: 2,
        lods: component("unused", 2).models.remove(0).lods,
    });
    first.models[1].lods[0].asset_id = 3;
    let scene = Arc::new(CharacterScene {
        customizations: None, schema_name: "fixture".into(),
        category: 0,
        components: vec![first],
        textures: vec![],
    });
    let mut finalized = FinalizedRecipeScene::new(Arc::clone(&scene));
    assert_eq!(
        finalized.try_finalize(3),
        Err(RecipeFinalizeError::MissingInstance(3))
    );
    assert_eq!(finalized.resolved.len(), 1);
    let prior = finalized.resolved[0];
    assert_eq!(
        scene.components[prior.component].models[prior.model].asset_id,
        1
    );
    assert!(finalized.lods.is_empty());
    assert!(matches!(
        finalized.scene(),
        Err(RecipeFinalizeError::NotReady)
    ));
}
