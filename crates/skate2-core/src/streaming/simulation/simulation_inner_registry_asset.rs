//! Retained native inner `cAsset82d61a78` wrapper over an existing arena record.
//! The shared resident registry owns only a weak reference; the processed asset
//! list owns the wrapper, and the wrapper keeps its original plan bytes alive.
use crate::simulation_normal_arena_plan::{
    SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRegistryAssetKind,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnerAssetError {
    MissingOperation(usize),
    NotRegistryOperation,
    MissingDictionary(u32),
    ObjectType { expected: u32, actual: u32 },
    ObjectBytes,
}

pub(crate) struct SimulationInnerRegistryAsset {
    pub(crate) id: u64,
    pub(crate) kind: SimulationRegistryAssetKind,
    pub(crate) plan: Arc<SimulationNormalArenaPlan>,
    pub(crate) dictionary_index: u32,
}
impl SimulationInnerRegistryAsset {
    /// Actual ID/type/data-pointer tuple supplied by the six recovered ID-only
    /// `Process*` callbacks before normal `RegArena` reaches their manager calls.
    pub(crate) fn try_from_operation(
        plan: Arc<SimulationNormalArenaPlan>,
        index: usize,
    ) -> Result<Arc<Self>, InnerAssetError> {
        let operation = plan
            .operations()
            .get(index)
            .ok_or(InnerAssetError::MissingOperation(index))?;
        let SimulationNormalOperation::RegistryAsset {
            kind,
            asset_id,
            source_dictionary_index,
            raw_object_type,
            ..
        } = *operation
        else {
            return Err(InnerAssetError::NotRegistryOperation);
        };
        let decoded = &plan.static_plan().asset().decoded;
        let entry = decoded
            .arena
            .dictionary
            .get(source_dictionary_index as usize)
            .ok_or(InnerAssetError::MissingDictionary(source_dictionary_index))?;
        if entry.type_id != raw_object_type {
            return Err(InnerAssetError::ObjectType {
                expected: raw_object_type,
                actual: entry.type_id,
            });
        }
        decoded
            .arena
            .object_bytes(&decoded.resources.resources, entry)
            .ok_or(InnerAssetError::ObjectBytes)?;
        Ok(Arc::new(Self {
            id: asset_id,
            kind,
            plan,
            dictionary_index: source_dictionary_index,
        }))
    }

    pub(crate) fn bytes(&self) -> Result<&[u8], InnerAssetError> {
        let decoded = &self.plan.static_plan().asset().decoded;
        let entry = decoded
            .arena
            .dictionary
            .get(self.dictionary_index as usize)
            .ok_or(InnerAssetError::MissingDictionary(self.dictionary_index))?;
        decoded
            .arena
            .object_bytes(&decoded.resources.resources, entry)
            .ok_or(InnerAssetError::ObjectBytes)
    }

    /// The record's raw arena object type, of which a registry kind may admit more than one.
    pub(crate) fn object_type(&self) -> Result<u32, InnerAssetError> {
        let decoded = &self.plan.static_plan().asset().decoded;
        decoded
            .arena
            .dictionary
            .get(self.dictionary_index as usize)
            .map(|entry| entry.type_id)
            .ok_or(InnerAssetError::MissingDictionary(self.dictionary_index))
    }
}
