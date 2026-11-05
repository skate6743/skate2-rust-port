//! DMO Recommend82e975a8 / Condition82e7e470 / Handler82e8fc28.
use super::{DynamicObjectModifier,EntityOwner,skater_avoidance::{SkaterAvoidanceTuning,create_avoid_packet}};
use crate::{living_world_chase::TargetableId,living_world_pedestrian_state::PedestrianStateStorage,
 pedestrian_intent_packet::queue::IntentQueue,pedestrian_navigation::rig::NavRig,living_world_mesh_store::MeshStore,
 reckoning_numeric::{ReckoningNumeric,try_normalize_safe,try_signed_angle,cross_full4},skeleton_root_transforms::SkeletonRootVector};
pub(crate) struct DmoAvoidanceTuning{
 pub(crate) influence_radius:f32,pub(crate) minimum_start_speed:f32,pub(crate) allow_avoid:bool,
 pub(crate) corner_distance:f32,pub(crate) max_jump_distance:f32,
}
pub(crate) struct DmoTarget{
 pub(crate) id:u32,pub(crate) position:[f32;4],pub(crate) center:[f32;4],pub(crate) velocity:[f32;4],
 pub(crate) corners:[[f32;4];4],
}
fn flat(v:[f32;4])->[f32;4]{[v[0],0.,v[2],0.]}
fn sub(a:[f32;4],b:[f32;4])->[f32;4]{std::array::from_fn(|i|a[i]-b[i])}
impl DynamicObjectModifier{
 pub(crate) fn recommend<N:ReckoningNumeric>(&mut self,position:[f32;4],facing:[f32;4],target:DmoTarget,
  tuning:&DmoAvoidanceTuning,avoid:&SkaterAvoidanceTuning,rig:&NavRig,mesh:&MeshStore,
  queue:&mut IntentQueue,state:&mut PedestrianStateStorage,numeric:&mut N)->Result<(),String> where N::Error:std::fmt::Debug{
  self.target_entity=Some(EntityOwner::LivingWorld(target.id));self.active=false;self.has_intent=false;
  if !self.enabled{return Ok(());}
  let delta=flat(sub(target.position,position));
  if numeric.dot3(delta,delta).map_err(err)?-tuning.influence_radius*tuning.influence_radius>0.{return Ok(());}
  if numeric.dot3(target.velocity,target.velocity).map_err(err)?-tuning.minimum_start_speed*tuning.minimum_start_speed<0.
   ||!tuning.allow_avoid{return Ok(());}
  let velocity=flat(target.velocity);let center=flat(target.center);let me=flat(position);
  let normal=try_normalize_safe(flat(cross_full4(velocity,[0.,1.,0.,0.])),[0.;4],numeric).map_err(err)?;
  let(mut low,mut high)=(0.,0.);let(mut low_corner,mut high_corner)=([0.;4],[0.;4]);
  for corner in target.corners{
   let projection=numeric.dot3(flat(sub(corner,center)),normal).map_err(err)?;
   if projection<=0.{if projection<low{low=projection;low_corner=corner;}}
   else if projection>high{high=projection;high_corner=corner;}
  }
  let projection=numeric.dot3(sub(me,center),normal).map_err(err)?;
  self.entity_normal_projection=projection;
  if projection-tuning.corner_distance>=high||tuning.corner_distance+projection<=low{return Ok(());}
  let corner=if projection<=0.{self.closest_corner_normal_projection=low;self.avoid_direction=normal.map(|v|-v);low_corner}
   else{self.closest_corner_normal_projection=high;self.avoid_direction=normal;high_corner};
  if !(numeric.dot3(flat(sub(me,corner)),velocity).map_err(err)?>0.){return Ok(());}
  if self.closest_corner_normal_projection.abs()-projection.abs()>=tuning.max_jump_distance{
   // Raw82e900dc..f0 preserves BOTH words of target ID. The decompiler's
   // invented assignment type=1 aliases r7's boolean into the r8 load.
   for want in &mut state.wants{want.active=false;}
   state.wants[10].target=TargetableId{kind:2,id:target.id as i32};
   state.wants[10].active=true;state.wants[10].explicit=false;
   return Ok(());
  }
  let angle=try_signed_angle(flat(facing),flat(self.avoid_direction),[0.,1.,0.,0.],numeric).map_err(err)?;
        let direction=if angle<self.forward_angle||angle>std::f32::consts::TAU-self.forward_angle{"Forward"}
            else if angle<std::f32::consts::PI-self.forward_angle{"Left"}
            else if angle<self.forward_angle+std::f32::consts::PI{"Back"}else{"Right"};
  if avoid.nav_mesh_only{
   let normalized=try_normalize_safe(flat(self.avoid_direction),[0.;4],numeric).map_err(err)?;
   let candidate=std::array::from_fn(|i|normalized[i].mul_add(avoid.large_avoid_distance,position[i]));
   let cell=mesh.try_get_cell_with_tolerance(SkeletonRootVector::known(candidate),avoid.nav_mesh_y_tolerance).map_err(err)?;
   if !cell.is_valid(){return Ok(());}
   let cost=cell.cost_and_centroid().ok_or("DMO avoid candidate mesh absent")?.0;
   // GetCost reads the rig's cell unchecked; PC port: off the mesh (Skate 3's has gaps) it costs most.
   let current=rig.mesh_info.cell.cost_and_centroid().map_or(f32::MAX,|(cost,_)|cost);
   if !(cost<=current){return Ok(());}
  }
  self.recommended_packet=Some(create_avoid_packet(queue,true,direction)?);self.active=true;self.has_intent=true;Ok(())
 }
}
fn err(e:impl std::fmt::Debug)->String{format!("DMO avoidance: {e:?}")}
