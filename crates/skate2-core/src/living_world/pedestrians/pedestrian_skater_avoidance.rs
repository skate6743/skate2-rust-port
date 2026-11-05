//! Skater modifier82e973f0, closest-point condition82e6c648 and handler82e8f5d0.
use super::{EntityOwner, SkaterModifier, GeneralModifier};
use crate::{
    living_world_chase::TargetableId,
    living_world_mesh_store::MeshStore,
    pedestrian_intent_packet::{IntentStage, queue::IntentQueue},
    pedestrian_navigation::rig::NavRig,
    play_animation::FastString30,
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe, try_signed_angle},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector,
};

pub(crate) struct SkaterAvoidanceTuning {
    pub(crate) avoid_npc_skaters: bool,
    pub(crate) predict_time: f32,
    pub(crate) outer_radius: f32,
    pub(crate) inner_radius: f32,
    pub(crate) forward_angle: f32,
    pub(crate) big_avoid_speed: f32,
    pub(crate) influence_radius: f32,
    pub(crate) minimum_start_speed: f32,
    pub(crate) allow_avoid: bool,
    pub(crate) nav_mesh_only: bool,
    pub(crate) large_avoid_distance: f32,
    pub(crate) small_avoid_distance: f32,
    pub(crate) nav_mesh_y_tolerance: f32,
}
/// Values read from actual target Entity and IActor::GetPhysOut. No inferred
/// zero/bail defaults: callers must read PhysOut_Filtered and Scoring2 owners.
pub(crate) struct SkaterAvoidanceTarget {
    pub(crate) owner: EntityOwner,
    pub(crate) targetable: TargetableId,
    pub(crate) position: [f32;4],
    pub(crate) velocity: [f32;4],
    pub(crate) filtered_physics_state: u32,
    pub(crate) hom_bail_in_progress: bool,
}
/// Mandatory returned side effect of successful large avoidance. Original
/// ILWMoodable event3 uses perpetrator/victim IDs, victim position and1/1.
/// Caller must deliver it to the existing mood owner before publication.
pub(crate) struct SkaterAvoidanceMoodEvent {
    pub(crate) perpetrator: TargetableId,
    pub(crate) victim: TargetableId,
    pub(crate) position: [f32;4],
}

/// geotest::ClosestPointToRay82e54a28. The existing segment closest-point
/// helper clamps endpoints and cannot implement this unbounded ray graph.
/// Shares reciprocal estimates/refinement, with original Y then X then Z FMA.
pub(crate) fn closest_point_to_ray<N: ReckoningNumeric>(position:[f32;4],
    heading:[f32;4], query:[f32;4], numeric:&mut N)
    -> Result<(f32,[f32;4]),N::Error> {
    let squared = numeric.dot3(heading,heading)?;
    let reciprocal = refine_drive_reciprocal(squared,numeric.reciprocal_estimate(squared)?);
    let y = (query[1]-position[1]) * heading[1];
    let xy = (query[0]-position[0]).mul_add(heading[0],y);
    let xyz = (query[2]-position[2]).mul_add(heading[2],xy);
    let proportion = reciprocal * xyz;
    Ok((proportion,std::array::from_fn(|i| heading[i].mul_add(proportion,position[i]))))
}
impl GeneralModifier {
    /// Condition_ShouldAvoid82e768c8 preserves cached vector/distance when
    /// the predicted time gate fails. Unlike Skater, time is not retained.
    pub(crate) fn should_avoid<N: ReckoningNumeric>(&mut self, my_position:[f32;4],
        target_position:[f32;4], target_velocity:[f32;4], outer_radius:f32,
        numeric:&mut N) -> Result<bool,N::Error> {
        let (time,point) = closest_point_to_ray(target_position,target_velocity,my_position,numeric)?;
        if !(self.predict_time > time) || time <= 0. { return Ok(false); }
        self.closest_point_to_me = std::array::from_fn(|i| my_position[i]-point[i]);
        self.squared_distance_to_closest = numeric.dot3(self.closest_point_to_me,self.closest_point_to_me)?;
        Ok(self.squared_distance_to_closest < outer_radius*outer_radius)
    }
    /// HandleBurdened_Stop82e770f0 (debug overlay has no simulation effect).
    pub(crate) fn handle_burdened_stop(&mut self) {
        self.flags.active = true; self.flags.speed = true; self.speed = 0.;
    }
    /// HandleBurdened_Avoid82e8f340 uses the same two retained packets as
    /// Skater avoidance; it performs no separate navigation mesh test.
    pub(crate) fn handle_burdened_avoid<N:ReckoningNumeric>(&mut self,
        facing:[f32;4],inner_radius:f32,front_half_angle:f32,
        queue:&mut IntentQueue,numeric:&mut N)->Result<(),String> where N::Error:std::fmt::Debug{
        let jump=!(self.squared_distance_to_closest>=inner_radius*inner_radius);
        if !jump&&queue.packet(queue.avoid_step)?.in_queue{return Ok(());}
        let avoid=[self.closest_point_to_me[0],0.,self.closest_point_to_me[2],0.];
        let facing=[facing[0],0.,facing[2],0.];
        let angle=try_signed_angle(facing,avoid,[0.,1.,0.,0.],numeric).map_err(|e|format!("general avoid angle: {e:?}"))?;
        let direction=avoid_direction(angle,front_half_angle);
        self.recommended_packet=Some(create_avoid_packet(queue,jump,direction)?);
        self.flags.active=true;self.flags.intent=true;Ok(())
    }
}
pub(super) fn avoid_direction(angle:f32,half:f32)->&'static str{
    if angle<half||angle>std::f32::consts::TAU-half{"Forward"}
    else if angle>=std::f32::consts::PI-half{
        if angle>=half+std::f32::consts::PI{"Right"}else{"Back"}
    }else{"Left"}
}
/// Native IntentGen CreateJump/StepAvoidIntentPacket share these retained IDs.
pub(crate) fn create_avoid_packet(queue:&mut IntentQueue,jump:bool,direction:&str)->Result<u64,String>{
    let id=if jump{queue.avoid_jump}else{queue.avoid_step};
    let mut stage=IntentStage::default();
    for name in ["Avoid",if jump{"Jump"}else{"Step"},direction]{
        stage.add_intent(FastString30::encode(name).expect("native avoid literal"))?;
    }
    stage.rotate_to_target=false;queue.packet_mut(id)?.replace_single_stage(stage);Ok(id)
}
impl SkaterModifier {
    /// EvaluateNavModifiers82e80110..120 writes these five values after its
    /// enabled gate, even when no local skater exists. MinimumSpeed is untouched.
    pub(crate) fn refresh_tuning(&mut self, tuning:&SkaterAvoidanceTuning) {
        self.predict_time = tuning.predict_time;
        self.inner_radius = tuning.inner_radius;
        self.outer_radius = tuning.outer_radius;
        self.forward_angle = tuning.forward_angle;
        self.big_avoid_speed = tuning.big_avoid_speed;
    }
    pub(crate) fn should_avoid<N: ReckoningNumeric>(&mut self, my_position:[f32;4],
        target_position:[f32;4], target_velocity:[f32;4], numeric:&mut N)
        -> Result<bool,N::Error> {
        let (time,point) = closest_point_to_ray(target_position,target_velocity,my_position,numeric)?;
        self.time_until_closest = time; self.closest_point_on_path = point;
        if time >= self.predict_time || time <= 0. { return Ok(false); }
        self.closest_point_to_me = std::array::from_fn(|i| my_position[i]-point[i]);
        self.squared_distance_to_closest = numeric.dot3(self.closest_point_to_me,self.closest_point_to_me)?;
        Ok(self.squared_distance_to_closest < self.outer_radius*self.outer_radius)
    }
    pub(crate) fn recommend<N: ReckoningNumeric>(&mut self,
        my_id:TargetableId, my_position:[f32;4], my_facing:[f32;4],
        target:SkaterAvoidanceTarget, tuning:&SkaterAvoidanceTuning,
        rig:&NavRig, mesh:&MeshStore, queue:&mut IntentQueue, numeric:&mut N)
        -> Result<Option<SkaterAvoidanceMoodEvent>,String> where N::Error:std::fmt::Debug {
        self.target_entity = Some(target.owner);
        self.active = false; self.has_intent = false;
        if !self.enabled || queue.packet(queue.avoid_jump)?.in_queue { return Ok(None); }
        let delta = [target.position[0]-my_position[0],0.,target.position[2]-my_position[2],0.];
        let squared = numeric.dot3(delta,delta).map_err(|e|format!("skater avoid distance: {e:?}"))?;
        // Native bgt/blt leave unordered values on the continuation path.
        if squared - tuning.influence_radius*tuning.influence_radius > 0. { return Ok(None); }
        let target_speed_squared = numeric.dot3(target.velocity,target.velocity)
            .map_err(|e|format!("skater avoid velocity: {e:?}"))?;
        if target_speed_squared-tuning.minimum_start_speed*tuning.minimum_start_speed < 0.
            || !tuning.allow_avoid { return Ok(None); }
        if !self.should_avoid(my_position,target.position,target.velocity,numeric)
            .map_err(|e|format!("skater avoid closest: {e:?}"))? { return Ok(None); }

        let target_speed_squared = numeric.dot3(target.velocity,target.velocity)
            .map_err(|e|format!("skater avoid handler velocity: {e:?}"))?;
        let large = target.filtered_physics_state != 6 && !target.hom_bail_in_progress
            && !(target_speed_squared <= self.big_avoid_speed*self.big_avoid_speed)
            && !(self.squared_distance_to_closest >= self.inner_radius*self.inner_radius);
        if !large && queue.packet(queue.avoid_step)?.in_queue { return Ok(None); }
        let avoid = [self.closest_point_to_me[0],0.,self.closest_point_to_me[2],0.];
        let facing = [my_facing[0],0.,my_facing[2],0.];
        let angle = try_signed_angle(facing,avoid,[0.,1.,0.,0.],numeric)
            .map_err(|e|format!("skater avoid angle: {e:?}"))?;
        let direction = avoid_direction(angle,self.forward_angle);
        if tuning.nav_mesh_only {
            let distance = if large { tuning.large_avoid_distance } else { tuning.small_avoid_distance };
            let direction = try_normalize_safe(avoid,[0.;4],numeric)
                .map_err(|e|format!("skater avoid normalize: {e:?}"))?;
            let position:[f32;4] = std::array::from_fn(|i| direction[i].mul_add(distance,my_position[i]));
            let cell = mesh.try_get_cell_with_tolerance(SkeletonRootVector::known(position),tuning.nav_mesh_y_tolerance)
                .map_err(|e|format!("skater avoid mesh: {e:?}"))?;
            if !cell.is_valid() { return Ok(None); }
            let candidate_cost = cell.cost_and_centroid().ok_or("skater avoid candidate mesh absent")?.0;
            // `GetCost` reads the rig's cell unchecked (`0x82e8fa40`); PC port: off the mesh
            // (Skate 3's has gaps) it costs most.
            let current_cost = rig.mesh_info.cell.cost_and_centroid().map_or(f32::MAX, |(cost, _)| cost);
            if candidate_cost > current_cost { return Ok(None); }
        }
        let id = create_avoid_packet(queue,large,direction)?;
        self.recommended_packet = Some(id); self.active = true; self.has_intent = true;
        Ok(large.then_some(SkaterAvoidanceMoodEvent { perpetrator:target.targetable,
            victim:my_id, position:my_position }))
    }
}
