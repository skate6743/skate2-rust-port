//! Completed pedestrian palettes in the shared object/material/shadow passes.
use crate::{gpu_renderer::{GpuObject,GpuRenderer,GpuTexture,RenderResult,CameraRelativeClip,FrameEnvironment,
    character::{CharacterPoseRows,CharacterShaderFamily,CharacterGhostState}},
    render_environment::EnvironmentLighting};
use skate2_core::{SelectedPedestrianVisual,character::palette::CharacterPalette,presentation::PresentationMaterialValue,
    presentation_restore::PresentationRestorer};
use skate2_formats::mount::MountSet;
use std::{collections::{BTreeMap,BTreeSet},sync::Arc};
type EntityKey=(u32,u64,usize);
type Key=(EntityKey,u8,usize,usize);
#[derive(Clone,Copy)]
enum Parameters { Plain([[f32;4];8]), Stamp([[f32;4];7]), Surface([[f32;4];7]) }
impl Parameters {
    fn gain(self)->f32 {match self{Self::Plain(rows)=>rows[0][1],Self::Stamp(rows)|Self::Surface(rows)=>rows[0][1]}}
    fn lighting(self)->[[f32;4];7] {match self{Self::Plain(rows)=>std::array::from_fn(|i|rows[i]),Self::Stamp(rows)|Self::Surface(rows)=>rows}}
    fn set_plain_factors(self,pose:&mut CharacterPoseRows<'_>){
        if let Self::Plain(rows)=self {
            // Final defaultlivingworld PS c11.w/c13.w: m_params[5].w/[7].w.
            let values=pose.living_world.as_mut().expect("pedestrian parameters");
            values[4][2]=rows[5][3];values[4][3]=rows[7][3];
        }
    }
}
pub(crate) struct PedestrianRender{
    mounts:Arc<MountSet>,environment:Arc<EnvironmentLighting>,
    retained:Vec<(Key,Parameters)>,
    /// Unselected LODs of still-resident entities. Reusing their GPU allocations
    /// leaves SwitchLOD's selection and every pose/material update unchanged.
    inactive_lod_objects:BTreeMap<Key,(GpuObject,Parameters)>,
    opacity:BTreeMap<EntityKey,skate2_core::LivingWorldOpacityState>,
    /// Uploaded recipe textures by original asset ID, shared by every mesh and LOD.
    textures:BTreeMap<u64,(String,GpuTexture)>,
    /// A crowd imposter target's no-sim pedestrian keeps the LOD it was made with: it is in
    /// no render view that would switch it.
    fixed_lod:Option<u8>,
}
/// The retained upload for this asset ID, unless the ID now names another texture.
fn cached_texture(cache:&mut BTreeMap<u64,(String,GpuTexture)>,id:u64,name:&str,
    upload:impl FnOnce()->RenderResult<GpuTexture>)->RenderResult<GpuTexture>{
    if let Some((cached,texture))=cache.get(&id) { if cached==name { return Ok(texture.clone()); } }
    let texture=upload()?;
    cache.insert(id,(name.to_owned(),texture.clone()));
    Ok(texture)
}
impl PedestrianRender{
    pub(crate) fn new(mounts:Arc<MountSet>,environment:Arc<EnvironmentLighting>)->Self{
        Self{mounts,environment,retained:Vec::new(),inactive_lod_objects:BTreeMap::new(),
            opacity:BTreeMap::new(),textures:BTreeMap::new(),fixed_lod:None}
    }
    pub(crate) fn crowd_target(mounts:Arc<MountSet>,environment:Arc<EnvironmentLighting>)->Self{
        Self{fixed_lod:Some(0),..Self::new(mounts,environment)}
    }
    /// Objects remain in GpuScene for normal sorting and shadow admission.
    /// The next frame recovers only this consumer's suffix, preserving buffers.
    /// A visual `drawn` refuses stays out of every pass.
    #[expect(clippy::too_many_arguments, reason = "one frame's pedestrian inputs")]
    pub(crate) fn update(&mut self,renderer:&GpuRenderer,objects:&mut Vec<GpuObject>,
        prior:Vec<GpuObject>,visuals:&[SelectedPedestrianVisual],camera:&CameraRelativeClip,
        frame:FrameEnvironment,restore:Option<&PresentationRestorer>,
        drawn:impl Fn(&SelectedPedestrianVisual)->bool)->RenderResult<()>{
        if prior.len()!=self.retained.len(){return Err("pedestrian GPU object membership".into());}
        let mut previous=std::mem::take(&mut self.inactive_lod_objects);
        previous.extend(self.retained.drain(..).zip(prior)
            .map(|((key,gain),object)|(key,(object,gain))));
        let live=visuals.iter().map(|v|(v.entity_id,v.recipe_id,Arc::as_ptr(&v.palette) as usize)).collect::<BTreeSet<_>>();
        previous.retain(|(entity,_,_,_),_|live.contains(entity));
        self.opacity.retain(|key,_|live.contains(key));
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static TRACE_FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let trace = *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some())
            && TRACE_FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 300 == 0;
        for visual in visuals{
            // LivingWorldPresentation::DoSimRestore 0x82813a90's local-to-world, else the tick's.
            let restored=restore.and_then(|restore|restore.pedestrian(visual.entity_id));
            let placement=restored.map_or_else(||visual.placement(),|transform|visual.placement_at(transform));
            let selected_lod=match self.fixed_lod {Some(lod)=>lod,None=>visual.select_geometry_lod(camera.camera,&placement)?};
            if restored.is_some(){visual.publish_at(&placement)?;}
            let entity_key=(visual.entity_id,visual.recipe_id,Arc::as_ptr(&visual.palette) as usize);
            let opacity=self.opacity.entry(entity_key).or_insert_with(||
                skate2_core::LivingWorldOpacityState::with_spawn_opacity(visual.spawn_opacity));
            opacity.try_update(visual.opacity_settings,visual.opacity_parameters,visual.simulation_center_velocity,
                placement.bounds_center,camera.camera)?;
            let ghost=CharacterGhostState{opacity:opacity.opacity,ghosted:opacity.ghosted};
            let hidden=!drawn(visual);
            let palette=visual.palette.lock().map_err(|_|"pedestrian GPU palette lock")?;
            let scene=palette.scene();
            for (part_index,part) in palette.skinning().parts.iter().enumerate(){
                let lod=&scene.components[part.component_index].models[part.model_index].lods[part.lod_index];
                let model=lod.arena.instances.iter().find_map(|i|i.render_model)
                    .and_then(|index|lod.arena.models.get(index)).ok_or("pedestrian RModel absent")?;
                for (mesh_index,link) in model.meshes.iter().enumerate(){
                    let key=(entity_key,selected_lod,part_index,mesh_index);
                    let mesh=lod.arena.meshes.get(link.mesh_index).ok_or("pedestrian mesh absent")?;
                    let rows=palette.mesh_rows(part_index,mesh_index)?;
                    if trace && part_index == 0 && mesh_index == 0 {
                        let relative: [f32;3] = std::array::from_fn(|i| placement.position[i]-camera.camera[i]);
                        let clip: [f32;4] = std::array::from_fn(|row| (0..3).fold(camera.clip_from_camera[12+row],
                            |sum,axis| camera.clip_from_camera[axis*4+row].mul_add(relative[axis],sum)));
                        std::eprintln!("pedestrian draw id={} lod={} opacity={} ghosted={} camera={:?} clip={:?} palette_prefix={:?}",
                            visual.entity_id,selected_lod,ghost.opacity,ghost.ghosted,camera.camera,clip,rows.rows.get(..3));
                    }
                    let mut constants=[visual.colours[0],visual.colours[1],frame.fog_k1,frame.fog_colour,[0.;4]];
                    constants[4][1]=self.environment.frame_character_attributes(frame)?[2][2];
                    let mut pose=CharacterPoseRows{rows:rows.rows,source_generation:rows.generation,
                        world_to_clip:&camera.clip_from_camera,camera_origin:camera.camera,
                        // cLivingWorldPresEntity ctor82807a20 sets +220 i_vtx_params.x=1
                        // (82807ae8); BindConstants8280b4c8 binds it to every current-LOD model.
                        ropa_vertex_params_x:Some(1.),
                        ghost,living_world:Some(constants)};
                    let stamp_inputs=|material|->RenderResult<crate::gpu_renderer::character::stamp::Inputs>{
                        Ok(crate::gpu_renderer::character::stamp::Inputs{material,irradiance:visual.irradiance,
                            attributes:self.environment.frame_character_attributes(frame)?,
                            light:frame.light_direction,csm:frame.shadows.globals(),camera:camera.camera,
                            // Skate 2's program blends to its far colour (diffuse*.3*saturate(world
                            // shadow+.1)) over i_material_lod; Skate 3's, which a district's look
                            // follows, binds no material LOD.
                            material_lod:if frame.skate3_district {0.}else{visual.material_lod(camera.camera,placement.position)?}})
                    };
                    let (mut object,parameters)=if let Some(retained)=previous.remove(&key){retained}else{
                        let material=mesh.material.and_then(|i|lod.arena.materials.get(i))
                            .ok_or("pedestrian material absent")?;
                        let effect=material.effect.as_ref().ok_or("pedestrian material metadata absent")?;
                        let material_key=effect.attribute_class_id.zip(effect.material_variant_id)
                            .ok_or("pedestrian material Vault key absent")?;
                        let bindings=crate::load_material_attribute_bindings(&self.mounts,&BTreeSet::from([material_key]))?;
                        let binding=bindings.get(&material_key).ok_or("pedestrian effect binding absent")?;
                        let (family,parameters)=match binding.manifest_name.as_str(){
                            "defaultcharacter"|"defaulthair"|"defaulthair_ropa"|"defaultcharacter_ropa"=>(match binding.manifest_name.as_str(){
                                "defaulthair"=>CharacterShaderFamily::Hair,
                                "defaulthair_ropa"=>CharacterShaderFamily::HairRopa,
                                "defaultcharacter_ropa"=>CharacterShaderFamily::Ropa,
                                _=>CharacterShaderFamily::Character},Parameters::Surface(
                                self.environment.material_parameter_array(material_key.0,material_key.1)
                                .and_then(|rows|rows.get(..7)).ok_or("pedestrian character seven material rows absent")?.try_into()?)),
                            "defaultlivingworld"=>(CharacterShaderFamily::LivingWorld,Parameters::Plain(
                                self.environment.material_parameter_array(material_key.0,material_key.1)
                                .and_then(|rows|rows.get(..8)).ok_or("pedestrian livingworld eight material rows absent")?.try_into()?)),
                            "livingworld_stamp"=>(CharacterShaderFamily::LivingWorldStamp,Parameters::Stamp(
                                self.environment.material_parameter_array(material_key.0,material_key.1)
                                .and_then(|rows|rows.get(..7)).ok_or("pedestrian stamp seven material rows absent")?.try_into()?)),
                            other=>return Err(format!("pedestrian effect {other} requires its original GPU implementation").into()),
                        };
                        let guid=material.channels.get(2..).unwrap_or_default().iter().rev()
                            .find(|c|c.shader_input=="diffuse").and_then(|c|match &c.value{
                                PresentationMaterialValue::Raster{guid,..}=>Some(*guid),_=>None})
                            .ok_or("pedestrian original diffuse binding absent")?;
                        let texture=scene.textures.iter().find(|t|t.source_asset_id==guid)
                            .ok_or("pedestrian original diffuse texture absent")?;
                        let texture=cached_texture(&mut self.textures,guid,&texture.texture.name,
                            ||renderer.upload_native_texture(&texture.texture))?;
                        let alpha=if family.is_hair() {
                            let guid=material.channels.get(2..).unwrap_or_default().iter().rev()
                                .find(|c|c.shader_input=="alpha").and_then(|c|match &c.value {
                                    PresentationMaterialValue::Raster{guid,..}=>Some(*guid),_=>None})
                                .ok_or("pedestrian hair original alpha binding absent")?;
                            let texture=scene.textures.iter().find(|t|t.source_asset_id==guid)
                                .ok_or("pedestrian hair original alpha texture absent")?;
                            Some(cached_texture(&mut self.textures,guid,&texture.texture.name,
                                ||renderer.upload_native_texture(&texture.texture))?)
                        }else{None};
                        pose.living_world.as_mut().expect("parameters")[4][0]=parameters.gain();
                        parameters.set_plain_factors(&mut pose);
                        let mut object=renderer.create_character_object(&mesh.geometry,u16::try_from(mesh.skin_palette.len())?,
                            family,&pose,&texture,alpha.as_ref())?;
                        if let Parameters::Plain(_)=parameters {
                            renderer.bind_living_world_lighting(&mut object,&texture,&stamp_inputs(parameters.lighting())?)?;
                        }
                        if let Parameters::Stamp(rows)|Parameters::Surface(rows)=parameters {
                            let normal=material.channels.get(2..).unwrap_or_default().iter().rev()
                                .find(|c|c.shader_input=="normal").and_then(|c|match &c.value{
                                    PresentationMaterialValue::Raster{guid,..}=>Some(*guid),_=>None})
                                .ok_or("pedestrian original normal binding absent")?;
                            let normal=scene.textures.iter().find(|t|t.source_asset_id==normal)
                                .ok_or("pedestrian original normal texture absent")?;
                            let normal=cached_texture(&mut self.textures,normal.source_asset_id,&normal.texture.name,
                                ||renderer.upload_native_texture(&normal.texture))?;
                            match parameters {
                                Parameters::Surface(_)=>renderer.bind_character_surface(&mut object,&mesh.geometry,&normal,alpha.as_ref(),&stamp_inputs(rows)?)?,
                                _=>renderer.bind_character_stamp(&mut object,&normal,&stamp_inputs(rows)?)?,
                            }
                        }
                        renderer.set_character_draw_order(&mut object,u8::try_from(binding.draw_order)?)?;
                        (object,parameters)
                    };
                    pose.living_world.as_mut().expect("parameters")[4][0]=parameters.gain();
                    parameters.set_plain_factors(&mut pose);
                    if trace && part_index == 0 && mesh_index == 0 {
                        let distance=(0..3).map(|i|(placement.position[i]-camera.camera[i]).powi(2)).sum::<f32>().sqrt();
                        let (kind,rows)=match parameters {Parameters::Plain(r)=>("plain",r[..8].to_vec()),
                            Parameters::Stamp(r)=>("stamp",r.to_vec()),Parameters::Surface(r)=>("surface",r.to_vec())};
                        std::eprintln!("pedestrian program id={} lod={selected_lod} distance={distance:.1} kind={kind} district={} m0={:?} m5={:?} last={:?}",
                            visual.entity_id,frame.skate3_district,rows[0],rows[5],rows.last());
                    }
                    renderer.update_character_object(&mut object,&pose)?;
                    renderer.set_character_shadow_bounds(&mut object,placement.position,placement.world_bounds)?;
                    renderer.update_character_stamp(&object,&stamp_inputs(parameters.lighting())?)?;
                    object.hidden=hidden;
                    objects.push(object);self.retained.push((key,parameters));
                }
            }
        }
        self.inactive_lod_objects=previous;
        Ok(())
    }
}

/// A NIS `ColorizedSkaterPresEntity` (`0x827e5e20`): its recipe's meshes take the
/// pedestrian effects with the entity's `i_colorize_red`/`i_colorize_blue`
/// (`BindConstants` `0x827dd1a0`) over the skater binds: full opacity and its own
/// SH. Nothing binds its `i_material_lod`, and `cScope::DispatchParamsHelper`
/// (`0x82e14aa0`) skips an unbound parameter, leaving the register as the last
/// draw set it; zero stands in for that.
pub(crate) struct ColorizedObjects{parameters:Vec<Parameters>}
/// What a colorized scene character's draw takes this frame.
#[derive(Clone,Copy)]
pub(crate) struct ColorizedInputs{pub(crate) colours:[[f32;4];2],pub(crate) irradiance:[[f32;4];9],
    pub(crate) position:[f32;3],pub(crate) bounds:[[f32;3];2]}
fn colorized_pose<'a>(environment:&EnvironmentLighting,frame:FrameEnvironment,rows:&'a [[f32;4]],generation:u64,
    inputs:ColorizedInputs,camera:&'a CameraRelativeClip)->RenderResult<CharacterPoseRows<'a>>{
    let mut constants=[inputs.colours[0],inputs.colours[1],frame.fog_k1,frame.fog_colour,[0.;4]];
    constants[4][1]=environment.frame_character_attributes(frame)?[2][2];
    Ok(CharacterPoseRows{rows,source_generation:generation,world_to_clip:&camera.clip_from_camera,
        camera_origin:camera.camera,ropa_vertex_params_x:Some(1.),
        ghost:CharacterGhostState{opacity:1.,ghosted:false},living_world:Some(constants)})
}
fn colorized_stamp(environment:&EnvironmentLighting,frame:FrameEnvironment,inputs:ColorizedInputs,
    camera:&CameraRelativeClip,material:[[f32;4];7])->RenderResult<crate::gpu_renderer::character::stamp::Inputs>{
    Ok(crate::gpu_renderer::character::stamp::Inputs{material,irradiance:inputs.irradiance,
        attributes:environment.frame_character_attributes(frame)?,light:frame.light_direction,
        csm:frame.shadows.globals(),camera:camera.camera,material_lod:0.})
}
impl ColorizedObjects{
    pub(crate) fn len(&self)->usize{self.parameters.len()}
    /// Each mesh's object, appended to `objects` in the order `update` takes them.
    #[expect(clippy::too_many_arguments,reason="one scene character's renderer, Vault, frame and draw inputs")]
    pub(crate) fn create(renderer:&GpuRenderer,mounts:&MountSet,environment:&EnvironmentLighting,frame:FrameEnvironment,
        palette:&CharacterPalette,inputs:ColorizedInputs,camera:&CameraRelativeClip,objects:&mut Vec<GpuObject>)->RenderResult<Self>{
        let mut textures=BTreeMap::new();
        let mut parameters=Vec::new();
        let scene=palette.scene();
        for (part_index,part) in palette.skinning().parts.iter().enumerate(){
            let lod=&scene.components[part.component_index].models[part.model_index].lods[part.lod_index];
            let model=lod.arena.instances.iter().find_map(|i|i.render_model)
                .and_then(|index|lod.arena.models.get(index)).ok_or("colorized RModel absent")?;
            for (mesh_index,link) in model.meshes.iter().enumerate(){
                let mesh=lod.arena.meshes.get(link.mesh_index).ok_or("colorized mesh absent")?;
                let rows=palette.mesh_rows(part_index,mesh_index)?;
                let mut pose=colorized_pose(environment,frame,rows.rows,rows.generation,inputs,camera)?;
                let material=mesh.material.and_then(|i|lod.arena.materials.get(i)).ok_or("colorized material absent")?;
                let effect=material.effect.as_ref().ok_or("colorized material metadata absent")?;
                let material_key=effect.attribute_class_id.zip(effect.material_variant_id)
                    .ok_or("colorized material Vault key absent")?;
                let bindings=crate::load_material_attribute_bindings(mounts,&BTreeSet::from([material_key]))?;
                let binding=bindings.get(&material_key).ok_or("colorized effect binding absent")?;
                let rows=|count:usize|environment.material_parameter_array(material_key.0,material_key.1)
                    .and_then(|rows|rows.get(..count)).ok_or("colorized material rows absent");
                let (family,parameter)=match binding.manifest_name.as_str(){
                    "defaultcharacter"=>(CharacterShaderFamily::Character,Parameters::Surface(rows(7)?.try_into()?)),
                    "defaultcharacter_ropa"=>(CharacterShaderFamily::Ropa,Parameters::Surface(rows(7)?.try_into()?)),
                    "defaulthair"=>(CharacterShaderFamily::Hair,Parameters::Surface(rows(7)?.try_into()?)),
                    "defaulthair_ropa"=>(CharacterShaderFamily::HairRopa,Parameters::Surface(rows(7)?.try_into()?)),
                    "defaultlivingworld"=>(CharacterShaderFamily::LivingWorld,Parameters::Plain(rows(8)?.try_into()?)),
                    "livingworld_stamp"=>(CharacterShaderFamily::LivingWorldStamp,Parameters::Stamp(rows(7)?.try_into()?)),
                    other=>return Err(format!("colorized effect {other} requires its original GPU implementation").into()),
                };
                let raster=|input:&str|material.channels.get(2..).unwrap_or_default().iter().rev()
                    .find(|c|c.shader_input==input).and_then(|c|match &c.value{
                        PresentationMaterialValue::Raster{guid,..}=>Some(*guid),_=>None});
                let mut upload=|guid:u64|->RenderResult<GpuTexture>{
                    let texture=scene.textures.iter().find(|t|t.source_asset_id==guid)
                        .ok_or("colorized original texture absent")?;
                    cached_texture(&mut textures,guid,&texture.texture.name,||renderer.upload_native_texture(&texture.texture))
                };
                let texture=upload(raster("diffuse").ok_or("colorized original diffuse binding absent")?)?;
                let alpha=if family.is_hair() {
                    Some(upload(raster("alpha").ok_or("colorized hair original alpha binding absent")?)?)
                }else{None};
                pose.living_world.as_mut().expect("parameters")[4][0]=parameter.gain();
                parameter.set_plain_factors(&mut pose);
                let mut object=renderer.create_character_object(&mesh.geometry,u16::try_from(mesh.skin_palette.len())?,
                    family,&pose,&texture,alpha.as_ref())?;
                if let Parameters::Plain(_)=parameter {
                    renderer.bind_living_world_lighting(&mut object,&texture,
                        &colorized_stamp(environment,frame,inputs,camera,parameter.lighting())?)?;
                }
                if let Parameters::Stamp(material_rows)|Parameters::Surface(material_rows)=parameter {
                    let normal=upload(raster("normal").ok_or("colorized original normal binding absent")?)?;
                    let stamp=colorized_stamp(environment,frame,inputs,camera,material_rows)?;
                    match parameter {
                        Parameters::Surface(_)=>renderer.bind_character_surface(&mut object,&mesh.geometry,&normal,alpha.as_ref(),&stamp)?,
                        _=>renderer.bind_character_stamp(&mut object,&normal,&stamp)?,
                    }
                }
                renderer.set_character_draw_order(&mut object,u8::try_from(binding.draw_order)?)?;
                renderer.update_character_object(&mut object,&pose)?;
                renderer.set_character_shadow_bounds(&mut object,inputs.position,inputs.bounds)?;
                renderer.update_character_stamp(&object,&colorized_stamp(environment,frame,inputs,camera,parameter.lighting())?)?;
                objects.push(object);parameters.push(parameter);
            }
        }
        Ok(Self{parameters})
    }
    /// The objects `create` built, posed and lit for this frame.
    #[expect(clippy::too_many_arguments,reason="one scene character's renderer, frame and draw inputs")]
    pub(crate) fn update(&self,renderer:&GpuRenderer,environment:&EnvironmentLighting,frame:FrameEnvironment,
        palette:&CharacterPalette,inputs:ColorizedInputs,camera:&CameraRelativeClip,
        prior:&mut impl Iterator<Item=GpuObject>,objects:&mut Vec<GpuObject>)->RenderResult<()>{
        let mut parameters=self.parameters.iter();
        let scene=palette.scene();
        for (part_index,part) in palette.skinning().parts.iter().enumerate(){
            let lod=&scene.components[part.component_index].models[part.model_index].lods[part.lod_index];
            let model=lod.arena.instances.iter().find_map(|i|i.render_model)
                .and_then(|index|lod.arena.models.get(index)).ok_or("colorized RModel absent")?;
            for mesh_index in 0..model.meshes.len(){
                let parameter=*parameters.next().ok_or("colorized object parameters absent")?;
                let mut object=prior.next().ok_or("colorized object absent")?;
                let rows=palette.mesh_rows(part_index,mesh_index)?;
                let mut pose=colorized_pose(environment,frame,rows.rows,rows.generation,inputs,camera)?;
                pose.living_world.as_mut().expect("parameters")[4][0]=parameter.gain();
                parameter.set_plain_factors(&mut pose);
                renderer.update_character_object(&mut object,&pose)?;
                renderer.set_character_shadow_bounds(&mut object,inputs.position,inputs.bounds)?;
                renderer.update_character_stamp(&object,&colorized_stamp(environment,frame,inputs,camera,parameter.lighting())?)?;
                objects.push(object);
            }
        }
        Ok(())
    }
}
