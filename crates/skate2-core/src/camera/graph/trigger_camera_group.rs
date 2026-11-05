//! Retained standard trigger groups; camera queries use native group2.
//! Uses the existing RW octree and retained normal-plan admission, not a miss stub.
use crate::{
    grind_data_octree::{GrindDataOctree, GrindOctreeError, OctreeBounds},
    selected_simulation_asset_services::SelectedSimulationAssetServices,
    simulation_normal_arena_plan::{
        SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordHandle,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use skate2_formats::collision::{Volume, VolumeType};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CameraTriggerError {
    AlreadyInitialized,
    MissingInitialization,
    MissingAdmission,
    PreviousAdmissionFailure,
    InvalidBounds,
    Allocation,
    TruncatedRecord,
    PendingCollision { index: usize },
    Tree(GrindOctreeError),
}

struct CameraQueryGroup {
    /// Tree constructor82e22e40 leaves this payload unwritten until Init.
    collision_volume: Option<Volume>,
    tree: Option<GrindDataOctree>,
    bounds: Option<OctreeBounds>,
    entry_fill_index: u32,
    free_indices: Vec<u32>,
    invalid_entry: Option<SimulationRecordHandle>,
    records: Vec<Option<SimulationRecordHandle>>,
}
impl CameraQueryGroup {
    fn construct() -> Self {
        Self {
            collision_volume: None,
            tree: None,
            bounds: None,
            entry_fill_index: 0,
            free_indices: Vec::new(),
            invalid_entry: None,
            records: Vec::new(),
        }
    }
    fn initialize(&mut self, bounds: OctreeBounds) -> Result<(), CameraTriggerError> {
        // TriggerTree.Init82e226e0 publishes BoxVolume before managed tree Init.
        // BoxVolume.Initialize82adcc38 uses vTableArray[4], represented by the
        // existing Volume type discriminator rather than an executable pointer.
        self.collision_volume = Some(Volume {
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            vtable_offset: VolumeType::Box as u32,
            shape_words: [1.0_f32.to_bits(); 3],
            radius: 0.0,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        });
        if self.tree.is_some() {
            return Err(CameraTriggerError::AlreadyInitialized);
        }
        // Ordered scalar comparisons preserve the native NaN admission gate.
        if (0..3).any(|i| bounds.minimum[i] > bounds.maximum[i]) {
            return Err(CameraTriggerError::InvalidBounds);
        }
        self.bounds = Some(bounds);
        self.tree = Some(GrindDataOctree::try_new(256, bounds).map_err(CameraTriggerError::Tree)?);
        Ok(())
    }
}

pub(crate) struct CameraTriggerGroup {
    assets: Option<Arc<SelectedSimulationAssetServices>>,
    bounds: Option<OctreeBounds>,
    group: Option<CameraQueryGroup>,
    other_groups: [Option<CameraQueryGroup>; 2],
    local_instances: std::collections::BTreeMap<u64, SimulationRecordHandle>,
    admitted_plans: Option<Vec<Arc<SimulationNormalArenaPlan>>>,
    admission_failed: bool,
}
impl CameraTriggerGroup {
    /// GetTriggerInstance82e21cc0 -> GetTriggerVolumeFromID82e21b78.
    /// The native API compares float-converted GUIDs and scans group0,1,2
    /// in retained entry order; a u64 map lookup has different semantics.
    pub(crate) fn try_find_volume_id(&self, volume_id: f32)
        -> Result<Option<SimulationRecordHandle>, CameraTriggerError> {
        if self.admission_failed { return Err(CameraTriggerError::PreviousAdmissionFailure); }
        for group in [&self.other_groups[0], &self.other_groups[1], &self.group] {
            let group = group.as_ref().ok_or(CameraTriggerError::MissingInitialization)?;
            for record in group.records.iter().take(group.entry_fill_index as usize).flatten() {
                let bytes = record.bytes();
                for offset in [176, 184] {
                    let guid = u64::from_be_bytes(bytes.get(offset..offset + 8)
                        .ok_or(CameraTriggerError::TruncatedRecord)?.try_into().unwrap());
                    // Native __u64tod followed by frsp: retain both roundings.
                    if volume_id == crate::skate3::id_float(guid) { return Ok(Some(record.clone())); }
                }
            }
        }
        Ok(None)
    }
    pub(crate) const fn construct() -> Self {
        Self {
            assets: None,
            bounds: None,
            group: None,
            other_groups: [None, None],
            local_instances: std::collections::BTreeMap::new(),
            admitted_plans: None,
            admission_failed: false,
        }
    }
    /// Standard group tree initialization from Manager.Init82e23500.
    /// Caller supplies bounds produced by the SAME retained stream-file owners.
    pub(crate) fn try_initialize(
        &mut self,
        assets: Arc<SelectedSimulationAssetServices>,
        mut bounds: OctreeBounds,
    ) -> Result<(), CameraTriggerError> {
        if self.group.is_some() || self.other_groups.iter().any(Option::is_some) {
            return Err(CameraTriggerError::AlreadyInitialized);
        }
        self.assets = Some(assets);
        self.bounds = Some(bounds);
        if bounds.minimum[..3].iter().all(|v| *v == 0.0)
            && bounds.maximum[..3].iter().all(|v| *v == 0.0)
        {
            // Native zero-box branch starts [-1,+1], scales extent by499.5,
            // then subtracts/adds. W is explicitly zero before these operations.
            bounds.minimum = [-1.0, -1.0, -1.0, 0.0];
            bounds.maximum = [1.0, 1.0, 1.0, 0.0];
            for i in 0..4 {
                let expansion = (bounds.maximum[i] - bounds.minimum[i]) * 499.5;
                bounds.minimum[i] -= expansion;
                bounds.maximum[i] += expansion;
            }
            self.bounds = Some(bounds);
        }
        // Native stores group pointer before its Init; failure retains it.
        for slot in &mut self.other_groups {
            *slot = Some(CameraQueryGroup::construct());
            slot.as_mut().unwrap().initialize(bounds)?;
        }
        self.group = Some(CameraQueryGroup::construct());
        self.group.as_mut().unwrap().initialize(bounds)?;
        Ok(())
    }

    /// Actual `AddTriggerVolume82e24810` routes type+216 before group insertion.
    /// Insert the retained record and its bounds into the selected native group.
    pub(crate) fn try_register(
        &mut self,
        record: &SimulationRecordHandle,
    ) -> Result<(), CameraTriggerError> {
        if self.admission_failed {
            return Err(CameraTriggerError::PreviousAdmissionFailure);
        }
        self.admission_failed = true;
        let bytes = record.bytes();
        let word = |offset: usize| -> Result<u32, CameraTriggerError> {
            Ok(u32::from_be_bytes(
                bytes
                    .get(offset..offset + 4)
                    .ok_or(CameraTriggerError::TruncatedRecord)?
                    .try_into()
                    .unwrap(),
            ))
        };
        let kind = word(216)?;
        let group_index = match kind {
            1 => 1,
            2 => 2,
            _ => 0,
        };
        let bounds = OctreeBounds {
            minimum: [
                f32::from_bits(word(64)?),
                f32::from_bits(word(68)?),
                f32::from_bits(word(72)?),
                0.0,
            ],
            maximum: [
                f32::from_bits(word(80)?),
                f32::from_bits(word(84)?),
                f32::from_bits(word(88)?),
                0.0,
            ],
        };
        let group = if group_index == 2 {
            self.group.as_mut()
        } else {
            self.other_groups[group_index].as_mut()
        }
        .ok_or(CameraTriggerError::MissingInitialization)?;
        let tree = group
            .tree
            .as_mut()
            .ok_or(CameraTriggerError::MissingInitialization)?;
        // cManagedOctTree.Insert82e21860: allocate index, store pointer, insert hull.
        let index = if let Some(index) = group.free_indices.pop() {
            index
        } else {
            let index = group.entry_fill_index;
            group.entry_fill_index = index.wrapping_add(1);
            index
        };
        if index >= 256 {
            return Err(CameraTriggerError::Allocation);
        }
        if index as usize == group.records.len() {
            group.records.push(Some(record.clone()));
        } else {
            group.records[index as usize] = Some(record.clone());
        }
        tree.try_insert(index as usize, bounds)
            .map_err(CameraTriggerError::Tree)?;
        let local_id = u64::from_be_bytes(
            bytes
                .get(184..192)
                .ok_or(CameraTriggerError::TruncatedRecord)?
                .try_into()
                .unwrap(),
        );
        // Native manager map assignment follows successful group insertion.
        self.local_instances.insert(local_id, record.clone());
        self.admission_failed = false;
        Ok(())
    }

    /// RemoveTriggerVolume82e22cb8 visits all three groups, using pointer
    /// identity, then erases the local-ID map entry. Managed Remove82e232b0
    /// returns its index to the LIFO free list before unlinking the octree.
    pub(crate) fn try_remove(
        &mut self,
        record: &SimulationRecordHandle,
    ) -> Result<(), CameraTriggerError> {
        if self.admission_failed {
            return Err(CameraTriggerError::PreviousAdmissionFailure);
        }
        self.admission_failed = true;
        for group in self
            .other_groups
            .iter_mut()
            .chain(std::iter::once(&mut self.group))
        {
            let group = group
                .as_mut()
                .ok_or(CameraTriggerError::MissingInitialization)?;
            if let Some(index) = group.records.iter().position(|actual| {
                actual
                    .as_ref()
                    .is_some_and(|actual| actual.is_same_backing_record(record))
            }) {
                group.records[index] = group.invalid_entry.clone();
                group.free_indices.push(index as u32);
                group
                    .tree
                    .as_mut()
                    .ok_or(CameraTriggerError::MissingInitialization)?
                    .try_remove(index)
                    .map_err(CameraTriggerError::Tree)?;
            }
        }
        let bytes = record.bytes();
        let id = u64::from_be_bytes(
            bytes
                .get(184..192)
                .ok_or(CameraTriggerError::TruncatedRecord)?
                .try_into()
                .unwrap(),
        );
        self.local_instances.remove(&id);
        self.admission_failed = false;
        Ok(())
    }

    /// Finalize only after the normal adapter has visited these SAME plans.
    /// Retain them and reject records not registered through that adapter.
    pub(crate) fn try_finish_admission(
        &mut self,
        plans: &[Arc<SimulationNormalArenaPlan>],
    ) -> Result<(), CameraTriggerError> {
        if self.admission_failed {
            return Err(CameraTriggerError::PreviousAdmissionFailure);
        }
        if self.group.as_ref().and_then(|g| g.tree.as_ref()).is_none() {
            return Err(CameraTriggerError::MissingInitialization);
        }
        for plan in plans {
            for (index, operation) in plan.operations().iter().enumerate() {
                if matches!(operation, SimulationNormalOperation::TriggerInstance { .. }) {
                    let record = plan
                        .record_handle(index)
                        .ok_or(CameraTriggerError::TruncatedRecord)?;
                    if !self
                        .group
                        .iter()
                        .chain(self.other_groups.iter().flatten())
                        .any(|group| {
                            group
                                .records
                                .iter()
                                .flatten()
                                .any(|actual| actual.is_same_backing_record(&record))
                        })
                    {
                        return Err(CameraTriggerError::MissingAdmission);
                    }
                }
            }
        }
        let mut retained = Vec::new();
        retained
            .try_reserve_exact(plans.len())
            .map_err(|_| CameraTriggerError::Allocation)?;
        retained.extend(plans.iter().cloned());
        self.admitted_plans = Some(retained);
        Ok(())
    }

    /// The challenge message group (`StandardTriggerGroups[0]`):
    /// `cManagedOctTree::GetCollisions` (`0x82e238b8`), the entries whose
    /// bounds meet `bounds` and that `valid` accepts, ascending.
    pub(crate) fn try_message_group_collisions(
        &self,
        bounds: OctreeBounds,
        mut valid: impl FnMut(&SimulationRecordHandle) -> bool,
    ) -> Result<Vec<u32>, CameraTriggerError> {
        if self.admission_failed {
            return Err(CameraTriggerError::PreviousAdmissionFailure);
        }
        let group = self.other_groups[0]
            .as_ref()
            .ok_or(CameraTriggerError::MissingInitialization)?;
        let tree = group
            .tree
            .as_ref()
            .ok_or(CameraTriggerError::MissingInitialization)?;
        let mut query = tree.try_query(bounds).map_err(CameraTriggerError::Tree)?;
        let mut hits = Vec::new();
        while let Some(index) = query.try_next().map_err(CameraTriggerError::Tree)? {
            if let Some(Some(record)) = group.records.get(index)
                && valid(record)
            {
                hits.push(index as u32);
            }
        }
        hits.sort_unstable();
        Ok(hits)
    }

    /// An entry of the challenge message group.
    pub(crate) fn message_group_record(&self, index: u32) -> Option<&SimulationRecordHandle> {
        self.other_groups[0].as_ref()?.records.get(index as usize)?.as_ref()
    }

    /// Camera `IsSubjectInVolume82e38460` supplies Subject root translation.
    pub(crate) fn try_subject_box_is_empty(
        &self,
        position: SkeletonRootVector,
    ) -> Result<bool, CameraTriggerError> {
        if self.admission_failed {
            return Err(CameraTriggerError::PreviousAdmissionFailure);
        }
        if self.admitted_plans.is_none() {
            return Err(CameraTriggerError::MissingAdmission);
        }
        let tree = self
            .group
            .as_ref()
            .and_then(|g| g.tree.as_ref())
            .ok_or(CameraTriggerError::MissingInitialization)?;
        let extent = f32::from_bits(0x3e4c_cccd);
        // The reused RW query reads XYZ only. An unknown W is scratch here,
        // never published back to the retained Subject or reported as known.
        let w = position.w.unwrap_or(0.0);
        let position = position.xyz;
        let bounds = OctreeBounds {
            minimum: [
                position[0] - extent,
                position[1] - extent,
                position[2] - extent,
                w,
            ],
            maximum: [
                position[0] + extent,
                position[1] + extent,
                position[2] + extent,
                w,
            ],
        };
        let mut query = tree.try_query(bounds).map_err(CameraTriggerError::Tree)?;
        if let Some(index) = query.try_next().map_err(CameraTriggerError::Tree)? {
            return Err(CameraTriggerError::PendingCollision { index });
        }
        Ok(true)
    }
}

#[cfg(test)]
#[path = "trigger_camera_group_tests.rs"]
mod tests;
