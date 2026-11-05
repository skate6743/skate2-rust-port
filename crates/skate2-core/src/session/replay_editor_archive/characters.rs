//! CharacterRecipeLoader::SaveRecipes/LoadRecipes equivalent for the PC
//! presentation archive: retain recipes/remaps, reload the original game assets.
use super::codec::{Result,Store,Writer,Reader};
use crate::{character::{CharacterScene,CharacterSkinningState,palette::CharacterPalette,material_admission::CharacterMaterialAdmission},
    presentation_restore::skater::SkaterRig,attribute_runtime::AttributeRuntimeVault,input_cvar_runtime::InputCvarReferences};
use skate2_formats::{recipe::{CharacterRecipe,CharacterComponent,CharacterModel,CharacterLod,CharacterCustomizations},mount::MountSet};
use std::sync::{Arc,Mutex};

/// A look as it leaves or enters this console (a recipe message, a replay file). Stamps keep no
/// texture path: the original's user-generated stamps were custom graphics a console downloaded
/// from the path a recipe named, and the port resolves stamps by asset id alone and loads no
/// user-generated one. Colours and morphs that are no number read as neutral ones.
fn shared(mut custom:CharacterCustomizations)->CharacterCustomizations {
    custom.stamps.retain(|stamp|!stamp.user_generated);
    for stamp in &mut custom.stamps {stamp.path.clear();}
    for colour in &mut custom.colours {
        for bits in colour.base_rgb_bits.iter_mut().chain(&mut colour.highlight_rgb_bits) {*bits=bounded(*bits,1.0,0.0,8.0);}
    }
    for bits in &mut custom.morph_bits {*bits=bounded(*bits,0.5,-4.0,4.0);}
    custom
}
fn bounded(bits:u32,fallback:f32,min:f32,max:f32)->u32 {
    let value=f32::from_bits(bits);
    if value.is_finite() {value.clamp(min,max).to_bits()} else {fallback.to_bits()}
}

/// About twice the largest original look (Create a Skater's 19 components of up to 2 LODs, 38 in
/// all; a pro's 11): a message or a shared replay names the models and textures this console
/// reads in full, so a larger recipe is refused before any of them loads.
const MOST_COMPONENTS:usize=32;
const MOST_MODELS:usize=4;
const MOST_LODS:usize=4;
const MOST_LODS_IN_ALL:usize=64;
const MOST_TEXTURES:usize=512;

fn admissible(recipe:&CharacterRecipe)->Result<()> {
    let models=||recipe.components.iter().flat_map(|part|&part.models);
    let lods=||models().flat_map(|model|&model.lods);
    let textures:usize=lods().flat_map(|lod|&lod.material_instances).flat_map(|instance|&instance.materials)
        .map(|material|material.shader_parameters.len()).sum();
    if recipe.components.len()>MOST_COMPONENTS||recipe.components.iter().any(|part|part.models.len()>MOST_MODELS)
        ||models().any(|model|model.lods.len()>MOST_LODS)||lods().count()>MOST_LODS_IN_ALL||textures>MOST_TEXTURES {
        return Err("replay character larger than any original look".into());
    }
    Ok(())
}

/// The most scenes, palettes and rigs a replay file's catalog holds.
const REPLAY_LIMITS:[usize;3]=[4096,16384,4096];

#[derive(Default)]
pub(crate) struct Catalog {
    pub scenes:Vec<Arc<CharacterScene>>,pub palettes:Vec<Arc<CharacterPalette>>,pub rigs:Vec<Arc<SkaterRig>>,
}
impl Catalog {
    pub fn scene(&mut self,scene:&Arc<CharacterScene>)->usize {
        if let Some(i)=self.scenes.iter().position(|s|Arc::ptr_eq(s,scene)) {return i;}
        let i=self.scenes.len();self.scenes.push(Arc::clone(scene));i
    }
    pub fn palette(&mut self,palette:&Arc<CharacterPalette>)->usize {
        if let Some(i)=self.palettes.iter().position(|p|p.identity()==palette.identity()&&p.skinning()==palette.skinning()) {return i;}
        self.scene(palette.scene());let i=self.palettes.len();self.palettes.push(Arc::clone(palette));i
    }
    pub fn borrowed_palette(&mut self,palette:&CharacterPalette)->usize {
        if let Some(i)=self.palettes.iter().position(|p|p.identity()==palette.identity()&&p.skinning()==palette.skinning()) {return i;}
        self.palette(&Arc::new(palette.clone()))
    }
    pub fn rig(&mut self,rig:&Arc<SkaterRig>)->usize {
        if let Some(i)=self.rigs.iter().position(|r|Arc::ptr_eq(r,rig)) {return i;}
        let i=self.rigs.len();self.rigs.push(Arc::clone(rig));i
    }
    pub fn put(&self,w:&mut Writer)->Result<()> {
        self.scenes.len().put(w);
        for scene in &self.scenes {
            let lod=scene.components.iter().flat_map(|p|&p.models).flat_map(|m|&m.lods).next().ok_or("empty replay character")?;
            let path=lod.entry_path.replace('/',"\\");
            let category=path.split('\\').nth(2).ok_or("replay character asset namespace")?;
            category.to_owned().put(w);
            // Only allocation hints differ; geometry, materials and CAC values
            // come from the actual retained recipe. Clone3 is the gameplay copy.
            let recipe=CharacterRecipe {version:7,schema_name:scene.schema_name.clone(),category:scene.category,memory_category:15,clone_type:3,
                components:scene.components.iter().map(|part|CharacterComponent {name:part.name.clone(),models:part.models.iter().map(|model|CharacterModel {
                    asset_id:model.asset_id,lods:model.lods.iter().map(|lod|CharacterLod {level:lod.level,asset_id:lod.asset_id,arena_asset_id:lod.arena_asset_id,
                        material_instances:lod.material_instances.clone()}).collect()}).collect()}).collect(),
                has_customizations:scene.customizations.is_some(),customizations:scene.customizations.clone().map(shared),packed_size:0};
            recipe.encode().map_err(|e|e.to_string())?.put(w);
        }
        self.palettes.len().put(w);
        for palette in &self.palettes {
            self.scenes.iter().position(|s|Arc::ptr_eq(s,palette.scene())).ok_or("replay palette scene absent")?.put(w);
            palette.skinning().put(w);
        }
        self.rigs.len().put(w);
        for rig in &self.rigs {
            rig.skeleton.put(w);rig.bind.put(w);rig.inverse_bind.put(w);rig.hips.put(w);rig.board.put(w);
            rig.twist_bones.put(w);rig.local_bounds.put(w);
        }
        Ok(())
    }
    pub fn get(r:&mut Reader<'_>,mounts:&MountSet,vault:&Arc<AttributeRuntimeVault>)->Result<Self> {
        Self::get_at_most(r,mounts,vault,REPLAY_LIMITS)
    }
    /// A catalog of at most `scenes` looks, `palettes` palettes and `rigs` rigs, each count
    /// checked before the first of its entries is built.
    pub fn get_at_most(r:&mut Reader<'_>,mounts:&MountSet,vault:&Arc<AttributeRuntimeVault>,
        [scenes,palettes,rigs]:[usize;3])->Result<Self> {
        let mut result=Self::default();
        let count=usize::get(r)?;if count>scenes {return Err("replay character count".into());}
        for _ in 0..count {
            let category=String::get(r)?;
            if !matches!(category.as_str(),"marquee"|"livingworld"|"vehicle"|"createacharacter"|"test") {return Err("replay character namespace".into());}
            let mut recipe=CharacterRecipe::parse(&Vec::<u8>::get(r)?).map_err(|e|e.to_string())?;
            admissible(&recipe)?;
            if category=="createacharacter" {
                let male=recipe.customizations.as_ref().is_none_or(|c|c.is_male);
                crate::edit_skater_character::fill_missing_parts(mounts,&mut recipe,male,&category).map_err(|e|e.to_string())?;
            }
            recipe.customizations=recipe.customizations.map(shared);
            result.scenes.push(Arc::new(crate::mounted_character_scene::own_character_scene_category(mounts,&recipe,&category).map_err(|e|e.to_string())?));
        }
        let count=usize::get(r)?;if count>palettes {return Err("replay palette count".into());}
        for _ in 0..count {
            let scene=Arc::clone(result.scenes.get(usize::get(r)?).ok_or("replay scene reference")?);
            let skinning=CharacterSkinningState::get(r)?;
            for part in &skinning.parts {
                let lod=scene.components.get(part.component_index).and_then(|p|p.models.get(part.model_index))
                    .and_then(|m|m.lods.get(part.lod_index)).ok_or("replay skinning part outside recipe")?;
                if part.bone_remap.iter().any(|&i|i>=skinning.hierarchy_inverse_bind_pose.len())||lod.level!=skinning.lod_level {
                    return Err("replay hierarchy remap".into());
                }
            }
            result.palettes.push(Arc::new(CharacterPalette::from_replay_skinning(scene,skinning).map_err(|e|e.to_string())?));
        }
        let count=usize::get(r)?;if count>rigs {return Err("replay skater rig count".into());}
        for _ in 0..count {
            let rig=SkaterRig {skeleton:Store::get(r)?,bind:Store::get(r)?,inverse_bind:Store::get(r)?,hips:Store::get(r)?,board:Store::get(r)?,
                twist_bones:Store::get(r)?,local_bounds:Store::get(r)?,
                rendering_skaters:crate::actor_face_playback::presentation::frame::selected_runtime::render_opacity::rendering_skaters(vault)};
            if rig.bind.len()!=rig.skeleton.parents.len()||rig.inverse_bind.len()!=rig.bind.len() {return Err("replay skater hierarchy lengths".into());}
            let count=rig.bind.len();
            if rig.hips>=count||rig.board>=count||rig.twist_bones.iter().any(|&bone|bone>=count)
                ||rig.skeleton.trajectory_bone.is_some_and(|bone|bone>=count)
                ||rig.skeleton.parts.iter().any(|part|part.bones.start>part.bones.end||part.bones.end>count)
                ||rig.skeleton.parents.iter().enumerate().any(|(bone,&parent)|parent < -1
                    || (parent>=0&&parent as usize>=bone&&rig.skeleton.trajectory_bone!=Some(parent as usize))) {
                return Err("invalid replay skater hierarchy".into());
            }
            result.rigs.push(Arc::new(rig));
        }
        Ok(result)
    }
}
pub(crate) struct Loaded {
    pub catalog:Catalog,
    pub materials:Vec<Option<Arc<Mutex<CharacterMaterialAdmission>>>>,
    pub pedestrian_palettes:Vec<Option<Arc<Mutex<CharacterPalette>>>>,
    pub vault:Arc<AttributeRuntimeVault>,pub cvars:InputCvarReferences,
}
impl Loaded {
    pub fn pedestrian_palette(&mut self,index:usize)->Result<Arc<Mutex<CharacterPalette>>> {
        let palette=self.catalog.palettes.get(index).ok_or("replay pedestrian palette index")?;
        let entry=self.pedestrian_palettes.get_mut(index).ok_or("replay pedestrian palette owner")?;
        Ok(Arc::clone(entry.get_or_insert_with(||Arc::new(Mutex::new(palette.as_ref().clone())))))
    }
    pub fn material(&mut self,index:usize,header:&[u8])->Result<Arc<Mutex<CharacterMaterialAdmission>>> {
        let current=self.materials.get_mut(index).ok_or("replay material palette index")?;
        if let Some(material)=current {return Ok(Arc::clone(material));}
        let palette=&self.catalog.palettes[index];
        let mut finalized=crate::player::mounted_recipe::finalize::FinalizedRecipeScene::new(Arc::clone(palette.scene()));
        finalized.try_finalize(3).map_err(|e|e.to_string())?;
        let mut material=CharacterMaterialAdmission::try_new(&finalized,palette,Arc::clone(&self.vault),self.cvars.clone()).map_err(|e|e.to_string())?;
        material.try_construct_from_header(header).map_err(|e|e.to_string())?;
        let material=Arc::new(Mutex::new(material));*current=Some(Arc::clone(&material));Ok(material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(components:usize,lods:usize)->CharacterRecipe {
        CharacterRecipe {version:7,schema_name:"test".into(),category:0,memory_category:15,clone_type:3,
            components:(0..components).map(|part|CharacterComponent {name:format!("part{part}"),models:vec![CharacterModel {asset_id:part as u64,
                lods:(0..lods).map(|level|CharacterLod {level:level as u8,asset_id:level as u64,arena_asset_id:(part*8+level) as u64,material_instances:vec![]}).collect()}]}).collect(),
            has_customizations:false,customizations:None,packed_size:0}
    }

    #[test]
    fn a_look_the_size_of_the_original_ones_loads_and_a_larger_one_is_refused() {
        assert!(admissible(&recipe(19,2)).is_ok());
        assert!(admissible(&recipe(MOST_COMPONENTS+1,1)).is_err());
        assert!(admissible(&recipe(4,MOST_LODS+1)).is_err());
        assert!(admissible(&recipe(MOST_COMPONENTS,3)).is_err());
    }
}
