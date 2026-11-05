//! NoRoadWanderTargetIterator82e90100/FindValidTarget82e8cfd8. Authored
//! navigation cells and avoid volumes choose targets; this is not locomotion.
use crate::{
    living_world_mesh_store::MeshStore,
    reckoning_numeric::{ReckoningNumeric, cross_full4, try_normalize_safe},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct NoRoadWanderTuning {
    pub(crate) timer_speed: f32,
    pub(crate) enabled: bool,
    pub(crate) far_attempts: i32,
    pub(crate) far_distance: f32,
    pub(crate) far_angle: f32,
    pub(crate) close_attempts: i32,
    pub(crate) close_distance: f32,
    pub(crate) close_angle: f32,
    pub(crate) fail_distance: f32,
    pub(crate) y_tolerance: f32,
}

/// Existing Manager world-avoid trigger membership. It must be supplied by
/// the real retained trigger owner; absent data cannot mean a permitted target.
pub(crate) trait WanderAvoidVolumes {
    fn contains(&self, point: [f32; 4]) -> Result<bool, String>;
}

/// Selected ChallengeLivingWorldManager::AvoidVolumeList (ctor825baa18).
/// Only authored AvoidArea operations add IDs; streamed triggers alone do not.
pub(crate) struct ChallengeAvoidVolumes {
    ids: Vec<u64>,
}
impl ChallengeAvoidVolumes {
    pub(crate) const fn new() -> Self { Self { ids: Vec::new() } }

    /// ExecuteLivingWorldOperation825bde28 tail, AFTER the area's census cull.
    /// Raw cmpld at825bdea8 compares the complete GUID, unlike the decompiler.
    pub(crate) fn record_after_avoid_area_cull(&mut self, id: u64) {
        if !self.ids.contains(&id) { self.ids.push(id); }
    }

    /// GameUnload825b6080 clears the retained list without replacing its owner.
    pub(crate) fn game_unload(&mut self) { self.ids.clear(); }

    /// GetAvoidVolumes825c0e48 resolves all registered volumes in list order
    /// before the caller tests points. Missing streamed volumes are skipped.
    pub(crate) fn resolve(&self, triggers: &crate::trigger_camera_group::CameraTriggerGroup)
        -> Result<ResolvedAvoidVolumes, String> {
        let mut volumes = Vec::with_capacity(self.ids.len());
        for &id in &self.ids {
            if let Some(volume) = triggers.try_find_volume_id(crate::skate3::id_float(id))
                .map_err(|e| format!("world avoid trigger lookup: {e:?}"))? {
                volumes.push(volume);
            }
        }
        Ok(ResolvedAvoidVolumes { volumes })
    }
}

pub(crate) struct ResolvedAvoidVolumes {
    volumes: Vec<crate::simulation_normal_arena_plan::SimulationRecordHandle>,
}
impl ResolvedAvoidVolumes {
    /// CreateVolumeNavRigLineTestBatch82e86adc visits the same resolved
    /// ChallengeLivingWorldManager list, preserving order and capacity stop.
    pub(crate) fn add_to_pedestrian_query_batch(&self,
        batch: &mut crate::pedestrian_volume_batches::PedestrianVolumeBatches,
    ) -> Result<(), String> {
        for volume in &self.volumes {
            let (transform, radius) = volume.trigger_volume_geometry()?;
            if !batch.add_volume(crate::collision_volume_line::VolumeData {
                transform, dimensions: [radius[0], radius[1], radius[2], 0.], mask: 0,
            }) { break; }
        }
        Ok(())
    }
}
impl WanderAvoidVolumes for ResolvedAvoidVolumes {
    fn contains(&self, point: [f32; 4]) -> Result<bool, String> {
        for volume in &self.volumes {
            if volume.trigger_contains_point(point)? { return Ok(true); }
        }
        Ok(false)
    }
}

pub(crate) struct NoRoadWanderTargetIterator {
    pub(crate) target: [f32; 4],
    pub(crate) last_target_failed: bool,
    pub(crate) last_target_timeout: bool,
    pub(crate) cost_threshold: f32,
}
impl NoRoadWanderTargetIterator {
    pub(crate) fn new() -> Self {
        Self {
            target: [0.; 4],
            last_target_failed: false,
            last_target_timeout: false,
            cost_threshold: 0.,
        }
    }
    /// OnPathingFailure82e6cdd8: the caller decides when to increment afterward.
    pub(crate) fn on_pathing_failure(&mut self, case: i32) -> bool {
        match case {
            1 | 2 => {
                self.last_target_failed = true;
                true
            }
            4 => {
                self.last_target_timeout = true;
                true
            }
            _ => false,
        }
    }
    fn candidate(
        mesh: &MeshStore,
        avoid: &impl WanderAvoidVolumes,
        position: [f32; 4],
        y_tolerance: f32,
        cost_threshold: f32,
    ) -> Result<Option<[f32; 4]>, String> {
        let cell = mesh
            .try_get_cell_with_tolerance(SkeletonRootVector::known(position), y_tolerance)
            .map_err(|e| format!("pedestrian target: {e}"))?;
        let Some((cost, centroid)) = cell.cost_and_centroid() else {
            return Ok(None);
        };
        // Ordered native Cost > threshold; preserve its NaN branch behavior.
        if cost > cost_threshold || avoid.contains(centroid)? {
            return Ok(None);
        }
        Ok(Some(centroid))
    }
    /// Source tests straight ahead first, then alternates -step,+step,-2step,
    /// +2step, including maxAttempts angular candidates after the first probe.
    fn find_valid_target(
        mesh: &MeshStore,
        avoid: &impl WanderAvoidVolumes,
        start: [f32; 4],
        direction: [f32; 4],
        distance: f32,
        angle: f32,
        attempts: u32,
        y_tolerance: f32,
        cost_threshold: f32,
    ) -> Result<Option<[f32; 4]>, String> {
        let point = std::array::from_fn(|i| direction[i].mul_add(distance, start[i]));
        if let Some(found) = Self::candidate(mesh, avoid, point, y_tolerance, cost_threshold)? {
            return Ok(Some(found));
        }
        let step = (angle * f32::from_bits(0x3c0efa35)) / (attempts >> 1) as f32;
        for attempt in 1..=attempts {
            let unsigned = ((attempt + 1) >> 1) as f32 * step;
            let angle = if attempt & 1 != 0 {
                unsigned * (-1.)
            } else {
                unsigned
            };
            let (sin, cos) = try_sin_cos(angle * 0.5, &mut HostReckoningSinCosNumeric)
                .map_err(|e| format!("pedestrian target sine: {e:?}"))?;
            let quaternion = [0., sin, 0., cos];
            let cross = cross_full4(quaternion, direction);
            let inner = std::array::from_fn(|i| cos.mul_add(direction[i], cross[i]));
            let outer = cross_full4(quaternion, inner);
            let rotated: [f32; 4] = std::array::from_fn(|i| outer[i].mul_add(2., direction[i]));
            let point = std::array::from_fn(|i| rotated[i].mul_add(distance, start[i]));
            if let Some(found) = Self::candidate(mesh, avoid, point, y_tolerance, cost_threshold)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
    /// Returns the RestartTargetTimer argument only when the source emits it.
    /// The failed-search forward target is authored behavior (distance cvar),
    /// not a substitute for the separate mesh path solver's admission.
    pub(crate) fn increment<N: ReckoningNumeric>(
        &mut self,
        mesh: &MeshStore,
        avoid: &impl WanderAvoidVolumes,
        current_position: [f32; 4],
        facing: [f32; 4],
        cost_threshold: f32,
        tuning: NoRoadWanderTuning,
        numeric: &mut N,
    ) -> Result<Option<f32>, String>
    where
        N::Error: std::fmt::Debug,
    {
        let direction = if self.last_target_timeout {
            let delta: [f32; 4] = std::array::from_fn(|i| current_position[i] - self.target[i]);
            try_normalize_safe(delta, [0.; 4], numeric)
                .map_err(|e| format!("pedestrian target normalize: {e:?}"))?
        } else {
            facing
        };
        let mut found = None;
        let mut duration = None;
        if tuning.far_attempts > 0 && !self.last_target_failed {
            found = Self::find_valid_target(
                mesh,
                avoid,
                current_position,
                direction,
                tuning.far_distance,
                tuning.far_angle,
                tuning.far_attempts as u32,
                tuning.y_tolerance,
                cost_threshold,
            )?;
            if found.is_some() {
                duration = Some(tuning.far_distance / tuning.timer_speed);
            }
        }
        if found.is_none() && tuning.close_attempts > 0 {
            found = Self::find_valid_target(
                mesh,
                avoid,
                current_position,
                direction,
                tuning.close_distance,
                tuning.close_angle,
                tuning.close_attempts as u32,
                tuning.y_tolerance,
                cost_threshold,
            )?;
            if found.is_some() {
                duration = Some(tuning.close_distance / tuning.timer_speed);
            }
        }
        self.target = found.unwrap_or_else(|| {
            std::array::from_fn(|i| direction[i].mul_add(tuning.fail_distance, current_position[i]))
        });
        self.last_target_failed = false;
        self.last_target_timeout = false;
        Ok(duration)
    }
}

/// Ephemeral frame adapter; the iterator remains on its concrete pedestrian.
pub(crate) struct NoRoadIteratorFrame<'a, A: WanderAvoidVolumes> {
    pub(crate) iterator: &'a mut NoRoadWanderTargetIterator,
    pub(crate) mesh: &'a MeshStore,
    pub(crate) avoid: &'a A,
    pub(crate) facing: [f32; 4],
    pub(crate) tuning: NoRoadWanderTuning,
    pub(crate) restart_timer: &'a mut dyn FnMut(f32) -> Result<(), String>,
}
impl<N: ReckoningNumeric, A: WanderAvoidVolumes> super::mesh::NavigationTargetIterator<N>
    for NoRoadIteratorFrame<'_, A>
where
    N::Error: std::fmt::Debug,
{
    fn has_target(&self) -> bool {
        true
    }
    fn target(&self) -> Result<[f32; 4], String> {
        Ok(self.iterator.target)
    }
    fn increment(&mut self, position: [f32; 4], _dt: f32, numeric: &mut N) -> Result<(), String> {
        if let Some(duration) = self.iterator.increment(
            self.mesh,
            self.avoid,
            position,
            self.facing,
            self.iterator.cost_threshold,
            self.tuning,
            numeric,
        )? {
            (self.restart_timer)(duration)?;
        }
        Ok(())
    }
    fn reset_cost_threshold(&mut self) {
        self.iterator.cost_threshold = 0.;
    }
    fn cost_threshold(&mut self) -> Result<f32, String> {
        Ok(self.iterator.cost_threshold)
    }
    fn allow_incomplete_path(&self) -> bool {
        false
    }
    fn allow_off_mesh(&self) -> bool {
        false
    }
    fn alternate_target(&self) -> Option<[f32; 4]> {
        None
    }
    fn on_pathing_failure(&mut self, case: i32, _position: [f32; 4]) -> Result<bool, String> {
        Ok(self.iterator.on_pathing_failure(case))
    }
}
