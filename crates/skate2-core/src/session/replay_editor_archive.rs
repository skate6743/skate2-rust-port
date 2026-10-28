//! Saved footage for the PC presentation recorder. The edit track is the original
//! ICELITE format; the surrounding versioned container replaces console memcard
//! and compressed DataSystem ownership, retaining every recorded presentation.
#[path="replay_editor_archive/codec.rs"]
pub(crate) mod codec;
#[path="replay_editor_archive/characters.rs"]
pub(crate) mod characters;
#[path="replay_editor_archive/records.rs"]
mod records;
use codec::{Store,Reader,Writer,Result};
use characters::{Catalog,Loaded};
use super::super::replay_session::{ReplayFrame,RecordedWorld};
use crate::{presentation_restore::{PresentationSubmission,skater::SkaterFrame},
    challenge::replay::ReplaySystem,attribute_runtime::AttributeRuntimeVault,
    input_cvar_runtime::InputCvarReferences,SelectedPedestrianVisual,SelectedVehicleVisual};
use skate2_formats::{ice_camera::editor::EditableTake,mount::MountSet};
use std::{collections::BTreeMap,sync::Arc};

/// Version 2 adds the world location (`SerializeReplayDataToDataBuffer` writes
/// `sGameInfo.additionalWorldLocation`); a version 1 archive plays in the main world.
const MAGIC:&[u8;8]=b"SK2PCR02";
const MAGIC_V1:&[u8;8]=b"SK2PCR01";
pub(super) struct Archive {
    pub take:EditableTake,
    pub film_pack:bool,
    /// The `fe_locations` key of the additional world the footage was recorded in.
    pub world_location:Option<u64>,
    pub hull_states:Vec<u8>,
    pub index:ReplaySystem,
    pub frames:BTreeMap<u32,Arc<ReplayFrame>>,
}

fn put_vehicle(v:&SelectedVehicleVisual,w:&mut Writer,c:&mut Catalog) {
    v.entity_id.put(w);v.model_key.put(w);v.recipe_id.put(w);c.scene(&v.scene).put(w);
    v.pose_name.put(w);v.transform.put(w);v.wheels.put(w);v.colours.put(w);v.irradiance.put(w);
    v.opacity_parameters.put(w);v.opacity_settings.put(w);v.simulation_center_velocity.put(w);
}
fn get_vehicle(r:&mut Reader<'_>,c:&Catalog)->Result<SelectedVehicleVisual> {
    let entity_id=Store::get(r)?;let model_key=Store::get(r)?;let recipe_id=Store::get(r)?;
    let scene=Arc::clone(c.scenes.get(usize::get(r)?).ok_or("replay vehicle scene")?);
    Ok(SelectedVehicleVisual {entity_id,model_key,recipe_id,scene,pose_name:Store::get(r)?,transform:Store::get(r)?,
        wheels:Store::get(r)?,colours:Store::get(r)?,irradiance:Store::get(r)?,opacity_parameters:Store::get(r)?,
        opacity_settings:Store::get(r)?,simulation_center_velocity:Store::get(r)?})
}
fn put_pedestrian(v:&SelectedPedestrianVisual,w:&mut Writer,c:&mut Catalog)->Result<()> {
    v.spawn_opacity.put(w);v.entity_id.put(w);v.recipe_id.put(w);v.model_key.put(w);
    c.borrowed_palette(&*v.palette.lock().map_err(|_|"replay pedestrian palette lock")?).put(w);
    v.colours.put(w);v.irradiance.put(w);v.position.put(w);v.material_lod_distance.put(w);
    v.geometry_lod_squared.put(w);v.hierarchy.as_ref().put(w);v.pose.put(w);v.transform.put(w);
    v.local_bounds.put(w);v.bounds_center.put(w);v.world_bounds.put(w);
    v.opacity_parameters.put(w);v.opacity_settings.put(w);v.simulation_center_velocity.put(w);Ok(())
}
fn get_pedestrian(r:&mut Reader<'_>,c:&mut Loaded)->Result<SelectedPedestrianVisual> {
    let spawn_opacity=Store::get(r)?;let entity_id=Store::get(r)?;let recipe_id=Store::get(r)?;let model_key=Store::get(r)?;
    let index=usize::get(r)?;
    let palette=c.pedestrian_palette(index)?;
    let colours=Store::get(r)?;let irradiance=Store::get(r)?;let position=Store::get(r)?;let material_lod_distance=Store::get(r)?;
    let geometry_lod_squared=Store::get(r)?;
    let hierarchy:Arc<crate::character::CharacterHierarchy>=Arc::new(Store::get(r)?);
    let pose:Option<Arc<[crate::animation::AnimationMatrix]>>=Store::get(r)?;
    if pose.as_ref().is_none_or(|pose|pose.len()!=hierarchy.bone_names.len()) {
        return Err("invalid replay pedestrian hierarchy pose".into());
    }
    let transform=Store::get(r)?;
    Ok(SelectedPedestrianVisual {spawn_opacity,entity_id,recipe_id,model_key,palette,
        colours,irradiance,position,material_lod_distance,geometry_lod_squared,hierarchy,pose,transform,
        local_bounds:Store::get(r)?,bounds_center:Store::get(r)?,world_bounds:Store::get(r)?,
        opacity_parameters:Store::get(r)?,opacity_settings:Store::get(r)?,simulation_center_velocity:Store::get(r)?})
}
fn put_frame(frame:&ReplayFrame,w:&mut Writer,c:&mut Catalog)->Result<()> {
    let v=&frame.visual;v.tick.put(w);v.sim_rate.put(w);v.camera.put(w);v.living_world.put(w);
    v.skaters.len().put(w);
    for skater in &v.skaters {
        c.rig(&skater.rig).put(w);c.palette(&skater.palette).put(w);skater.header.put(w);
        skater.pose.put(w);skater.twist_ratios.put(w);skater.materials.is_some().put(w);
    }
    frame.local_skater.put(w);frame.players_recipe.put(w);frame.audio.put(w);
    frame.world.dynamic_objects.put(w);frame.world.vehicles.len().put(w);
    for v in &frame.world.vehicles {put_vehicle(v,w,c);}
    frame.world.pedestrians.len().put(w);
    for p in &frame.world.pedestrians {put_pedestrian(p,w,c)?;}
    frame.world.traffic_lights.put(w);Ok(())
}
fn get_frame(r:&mut Reader<'_>,c:&mut Loaded)->Result<ReplayFrame> {
    let tick=Store::get(r)?;let sim_rate=Store::get(r)?;let camera=Store::get(r)?;let living_world=Store::get(r)?;
    let count=usize::get(r)?;if count==0||count>256 {return Err("replay skater count".into());}
    let mut skaters=Vec::with_capacity(count);
    for _ in 0..count {
        let rig=Arc::clone(c.catalog.rigs.get(usize::get(r)?).ok_or("replay rig reference")?);
        let index=usize::get(r)?;
        let mut palette=Arc::clone(c.catalog.palettes.get(index).ok_or("replay palette reference")?);
        if !palette.has_deck_bone() {
            let mut recorded=(*palette).clone();recorded.set_deck_bone(rig.board);
            palette=Arc::new(recorded);c.catalog.palettes[index]=Arc::clone(&palette);
        }
        let header:[u8;208]=Store::get(r)?;let pose:Vec<skate2_formats::andale::AndaleSqt>=Store::get(r)?;
        if pose.len()!=rig.bind.len() {return Err("replay skater pose length".into());}
        let twist_ratios=Store::get(r)?;
        let materials=if bool::get(r)? {Some(c.material(index,&header)?)} else {None};
        skaters.push(SkaterFrame {rig,palette,header,pose,twist_ratios,materials});
    }
    let local_skater=usize::get(r)?;let players_recipe:Vec<bool>=Store::get(r)?;
    if local_skater>=skaters.len()||players_recipe.len()!=skaters.len() {return Err("replay player roster".into());}
    let audio=Store::get(r)?;let dynamic_objects=Store::get(r)?;
    let count=usize::get(r)?;if count>4096 {return Err("replay vehicle count".into());}
    let vehicles=(0..count).map(|_|get_vehicle(r,&c.catalog)).collect::<Result<Vec<_>>>()?;
    let count=usize::get(r)?;if count>4096 {return Err("replay pedestrian count".into());}
    let pedestrians=(0..count).map(|_|get_pedestrian(r,c)).collect::<Result<Vec<_>>>()?;
    Ok(ReplayFrame {visual:PresentationSubmission {camera,skaters,living_world,tick,sim_rate},audio,
        world:RecordedWorld {dynamic_objects,vehicles,pedestrians,traffic_lights:Store::get(r)?},local_skater,players_recipe})
}

pub(super) fn encode(take:&EditableTake,film_pack:bool,world_location:Option<u64>,hull_states:Vec<u8>,
    index:&ReplaySystem,frames:&BTreeMap<u32,Arc<ReplayFrame>>)->Result<Vec<u8>> {
    let mut catalog=Catalog::default();let mut payload=Writer::default();
    index.num_frames_tracked().put(&mut payload);
    for frame in index.frames() {
        frame.put(&mut payload);
        put_frame(frames.get(&frame).ok_or("save replay frame missing")?,&mut payload,&mut catalog)?;
    }
    let mut output=Writer(MAGIC.to_vec());take.serialize().put(&mut output);
    film_pack.put(&mut output);world_location.unwrap_or(u64::MAX).put(&mut output);
    hull_states.put(&mut output);catalog.put(&mut output)?;
    output.0.extend_from_slice(&payload.0);
    let crc=skate2_formats::skater_profile::native_crc(&output.0);crc.put(&mut output);
    Ok(output.0)
}
pub(super) fn decode(data:&[u8],mounts:&MountSet,vault:&Arc<AttributeRuntimeVault>,cvars:&InputCvarReferences)->Result<Archive> {
    let length=data.len().checked_sub(4).ok_or("truncated replay header")?;
    let crc=u32::from_be_bytes(data[length..].try_into().map_err(|_|"replay checksum length")?);
    if skate2_formats::skater_profile::native_crc(&data[..length])!=crc {return Err("replay checksum mismatch".into());}
    let mut r=Reader::new(&data[..length]);
    let magic=r.bytes(8)?;
    let located=if magic==MAGIC {true} else if magic==MAGIC_V1 {false} else {return Err("unsupported replay archive version".into())};
    let take=EditableTake::parse(&Vec::<u8>::get(&mut r)?)?;
    let film_pack=bool::get(&mut r)?;
    let world_location=if located {Some(u64::get(&mut r)?).filter(|&key|key!=u64::MAX)} else {None};
    let hull_states=Vec::<u8>::get(&mut r)?;
    if hull_states.len()<12 || u32::from_be_bytes(hull_states[..4].try_into().unwrap())!=3
        || u32::from_be_bytes(hull_states[8..12].try_into().unwrap()) as usize>(hull_states.len()-12)/12 {
        return Err("invalid replay dynamic hull states".into());
    }
    if !take.length().is_finite()||take.length()<1.5 {
        return Err("invalid replay editing parameters".into());
    }
    let catalog=Catalog::get(&mut r,mounts,vault)?;
    let materials=vec![None;catalog.palettes.len()];
    let pedestrian_palettes=vec![None;catalog.palettes.len()];
    let mut catalog=Loaded {catalog,materials,pedestrian_palettes,vault:Arc::clone(vault),cvars:cvars.clone()};
    let count=usize::get(&mut r)?;if count==0||count>crate::challenge::replay::MAX_FRAMES {return Err("replay frame count".into());}
    let mut index=ReplaySystem::default();let mut frames=BTreeMap::new();
    for _ in 0..count {
        let number=u32::get(&mut r)?;if !index.store_frame(number).0 {return Err("replay frame order".into());}
        frames.insert(number,Arc::new(get_frame(&mut r,&mut catalog)?));
    }
    // Pins can retain old footage across gaps longer than the ring's normal
    // duration. Validate against the saved frame span, without a time cap.
    let span=(index.last_frame_number_written()-index.oldest_frame_number_written()) as f32*(1.0/60.0);
    if take.length()!=span.max(1.5) {return Err("replay edit track does not match recorded frame span".into());}
    if r.offset!=r.data.len() {return Err("trailing replay data".into());}
    Ok(Archive {take,film_pack,world_location,hull_states,index,frames})
}
