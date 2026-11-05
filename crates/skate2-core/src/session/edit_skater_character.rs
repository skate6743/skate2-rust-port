//! Native CAS master component/model/material selection.
//! CharacterEntity::CreateCopyFromComponents82d13ad0 and RebindComponentToMaterial82d08fd8.
use skate2_formats::{cac_parts::{CacPartDb,SLOT_NAMES},recipe::CharacterRecipe,skater_profile::CacPartId};
use crate::{character::{CharacterHierarchy,palette::CharacterPalette,material_admission::CharacterMaterialAdmission},
    player::mounted_recipe::finalize::FinalizedRecipeScene};
use std::sync::Arc;
pub type EditSkaterCharacterResult<T> = Result<T,Box<dyn std::error::Error>>;

pub fn compose_recipe(db:&CacPartDb,parts:&[CacPartId;26],
    customizations:skate2_formats::recipe::CharacterCustomizations)->EditSkaterCharacterResult<CharacterRecipe> {
    let mut recipe=db.master.clone();
    recipe.components.clear();recipe.clone_type=2;recipe.packed_size=0;
    recipe.has_customizations=true;recipe.customizations=Some(customizations);
    let invalid=CacPartId::invalid();
    for (slot,id) in parts.iter().enumerate() {
        let Some(component)=db.master.components.iter().find(|p|p.name==SLOT_NAMES[slot]) else {continue};
        if *id==invalid {continue;}
        let mut selected=component.clone();
        let model=component.models.iter().find(|m|m.asset_id==id.model)
            .ok_or_else(||format!("CAS {} model {:016x} absent from master",component.name,id.model))?;
        let mut model=model.clone();
        for lod in &mut model.lods {
            for instance in &mut lod.material_instances {
                // Native uses the selected material if present, otherwise the
                // first original ordered material (parts can share geometry).
                let selected_material=instance.materials.iter().any(|m|m.asset_id==id.material);
                let mut material=instance.materials.iter().find(|m|m.asset_id==id.material)
                    .or_else(||instance.materials.iter().min_by_key(|m|m.asset_id)).cloned();
                if selected_material {
                    if let (Some(material),Some(custom))=(&mut material,&recipe.customizations){
                        apply_texture_customizations(&db.master,material,id,custom)?;
                    }
                }
                instance.materials=material.into_iter().collect();
            }
        }
        selected.models=vec![model];recipe.components.push(selected);
    }
    recipe.components.sort_by_key(|p|skate2_formats::hash::rw_hash32_string(&p.name));
    if recipe.components.is_empty(){return Err("CAS assembled recipe contains no components".into());}
    Ok(recipe)
}

/// PC port: a saved skater can wear parts whose files are absent, as Skate 3 clothes are once the
/// Skate 3 data is unlinked. Each component with a missing model or texture takes its slot's
/// native default part (`GetDefaultPartID` 0x82b68f78) for this load only; the saved record keeps
/// the original parts. A slot with no loadable default is left empty. Returns the slots replaced.
pub(crate) fn fill_missing_parts(mounts:&skate2_formats::mount::MountSet,recipe:&mut CharacterRecipe,
    is_male:bool,category:&str)->EditSkaterCharacterResult<Vec<String>> {
    let template=CharacterRecipe{components:Vec::new(),..recipe.clone()};
    let present=|component:&skate2_formats::recipe::CharacterComponent| {
        let single=CharacterRecipe{components:vec![component.clone()],..template.clone()};
        single.asset_requests(category).iter().all(|request|mounts.contains_path(&request.entry_path))
    };
    if recipe.components.iter().all(|component|present(component)) {return Ok(Vec::new());}
    static DB:std::sync::OnceLock<Arc<CacPartDb>>=std::sync::OnceLock::new();
    let db=match DB.get() {
        Some(db)=>Arc::clone(db),
        None=>{
            let db=Arc::new(CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH)?)?);
            Arc::clone(DB.get_or_init(||db))
        }
    };
    let customizations=recipe.customizations.clone();
    let mut replaced=Vec::new();
    let mut components=Vec::with_capacity(recipe.components.len());
    for component in std::mem::take(&mut recipe.components) {
        if present(&component) {components.push(component);continue;}
        replaced.push(component.name.clone());
        let Some(slot)=SLOT_NAMES.iter().position(|name|name.eq_ignore_ascii_case(&component.name)) else {continue};
        let Some(part)=crate::edit_skater_record::default_part(&db,slot as u8,is_male) else {continue};
        let parts:[CacPartId;26]=std::array::from_fn(|i| if i==slot {
            CacPartId{model:part.model_id,material:part.material_id}
        } else {CacPartId::invalid()});
        let Some(customizations)=customizations.clone() else {continue};
        let Ok(default)=compose_recipe(&db,&parts,customizations) else {continue};
        if let Some(fallback)=default.components.into_iter().next() && present(&fallback) {
            components.push(fallback);
        }
    }
    components.sort_by_key(|p|skate2_formats::hash::rw_hash32_string(&p.name));
    recipe.components=components;
    if recipe.components.is_empty(){return Err("CAS recipe has no loadable components".into());}
    Ok(replaced)
}

/// PC port: installed parts the database lacks (Skate 3 clothes with the Skate 3 data unlinked)
/// show as their slots' native defaults; the record keeps the original ids.
pub(crate) fn wearable_parts(db:&CacPartDb,parts:[CacPartId;26],is_male:bool)->[CacPartId;26] {
    std::array::from_fn(|slot| {
        let id=parts[slot];
        let known=id==CacPartId::invalid() || db.master.components.iter()
            .find(|c|c.name==SLOT_NAMES[slot]).is_none_or(|c|c.models.iter().any(|m|m.asset_id==id.model));
        if known {return id;}
        crate::edit_skater_record::default_part(db,slot as u8,is_male)
            .map_or_else(CacPartId::invalid,|part|CacPartId{model:part.model_id,material:part.material_id})
    })
}

/// CreateCopyFromComponents82d13f34..406c matches both model/material, then
/// resolves the FIRST record with the matching key. Raw PPC confirms both
/// comparisons are full64-bit, despite Hex-Rays splitting the registers.
fn apply_texture_customizations(master:&CharacterRecipe,material:&mut skate2_formats::recipe::CharacterMaterial,
    part:&CacPartId,custom:&skate2_formats::recipe::CharacterCustomizations)->EditSkaterCharacterResult<()> {
    let Some(part_stamp)=custom.stamps.iter().find(|s|s.model_id==part.model&&s.material_id==part.material) else{return Ok(());};
    let Some(stamp)=custom.stamps.iter().find(|s|s.key==part_stamp.key) else{return Ok(());};
    if stamp.user_generated {return Err("CAS user-generated stamp texture owner is not loaded".into());}
    // GetStampableTextureFromMaterialVariant82d093d0 searches Misc, models in
    // asset order, FIRST LOD only, then original material-instance order.
    let Some(misc)=master.components.iter().find(|c|c.name=="Misc") else{return Ok(());};
    let mut models=misc.models.iter().collect::<Vec<_>>();models.sort_by_key(|m|m.asset_id);
    for model in models {
        let Some(lod)=model.lods.iter().min_by_key(|l|l.level) else{continue;};
        for instance in &lod.material_instances {
            if let Some(source)=instance.materials.iter().find(|m|m.asset_id==stamp.stamp_material_id){
                if let Some(texture)=source.shader_parameters.iter().find(|p|p.channel_name=="decal"){
                    if let Some(channel)=material.shader_parameters.iter_mut().find(|p|p.channel_name=="decal"){
                        channel.asset_id=texture.asset_id;
                    }
                }
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Owned scene/texture copies are the host implementation of clone2. Mutating
/// their material bindings cannot modify a retained master or gameplay recipe.
pub struct EditSkaterCharacter {
    pub recipe:Arc<CharacterRecipe>,
    pub palette:CharacterPalette,
    pub materials:CharacterMaterialAdmission,
    pub(crate) finalized:Arc<FinalizedRecipeScene>,
}
impl EditSkaterCharacter {
    pub(crate) fn load(mounts:&skate2_formats::mount::MountSet,recipe:CharacterRecipe,
        hierarchy:&CharacterHierarchy,vault:Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars:crate::input_cvar_runtime::InputCvarReferences)->EditSkaterCharacterResult<Self> {
        let scene=crate::mounted_character_scene::own_character_scene_category(mounts,&recipe,"createacharacter")?;
        let scene=Arc::new(scene);
        let mut finalized=FinalizedRecipeScene::new(Arc::clone(&scene));
        finalized.try_finalize(2).map_err(|e|format!("CAS finalize: {e:?}"))?;
        let palette=CharacterPalette::new(scene,hierarchy,0)?;
        let mut materials=CharacterMaterialAdmission::try_new(&finalized,&palette,vault,cvars)?;
        materials.try_construct_cas()?;
        Ok(Self{recipe:Arc::new(recipe),palette,materials,finalized:Arc::new(finalized)})
    }
}
