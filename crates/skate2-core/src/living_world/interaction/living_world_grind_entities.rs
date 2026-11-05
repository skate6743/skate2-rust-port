//! LivingWorld EntityMan. The same retained registry now admits typed payloads;
//! its spatial tree reuses the DMO/RAIN KD implementation and native map order.
use super::{
    dmo_locators::{LocatorError, LocatorKdTree},
    dmo_objects::EntityOrder,
};
use crate::reckoning_numeric::ReckoningNumeric;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityInitError {
    AlreadyConstructed,
    AlreadyInitialized,
    Allocation,
    UnsupportedCapacity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntityNearQueryError {
    MissingInitialization,
    MaxItemsExceedsNativeLimit,
    PopulatedTreeUnrecovered,
    PopulatedRosterUnrecovered,
}
impl std::fmt::Display for EntityInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld EntityMan initialization: {self:?}")
    }
}

pub(crate) struct EntityMan<const N: usize, E = ()> {
    members: HashMap<u32, E>,
    order: EntityOrder,
    spatial: Option<LocatorKdTree>,
}
pub(crate) trait SpatialEntity {
    fn core_position_xz(&self) -> [f32; 2];
    fn core_data_valid(&self) -> bool;
}
#[derive(Debug)]
pub(crate) enum EntityMutationError {
    MissingPayload,
    MissingInitialization,
    MapCapacity,
    InvalidCore(u32),
}
#[derive(Debug)]
pub(crate) enum EntitySpatialError<E> {
    MissingInitialization,
    Query(LocatorError<E>),
}
impl<const N: usize, E> EntityMan<N, E> {
    /// `EntityMan` constructors82d26b48/82d366d0/82e92c40/82e67710 create the empty map
    /// and null KD root. Host allocation failure is explicit.
    pub(crate) fn try_construct() -> Result<Box<Self>, EntityInitError> {
        // fixed_hash_map constructors82d253e8/82e8ded0 select the greatest
        // source-table prime <= N+1: table822852f0, policy82bd2278.
        let buckets = match N {
            24 => 23,
            40 => 41,
            // Prop constructor82e66f20 requests GetPrevBucketCountOnly(257).
            // Original prime table822852f0 includes257 (between241 and277).
            256 => 257,
            _ => return Err(EntityInitError::UnsupportedCapacity),
        };
        let mut members = HashMap::new();
        members
            .try_reserve(N)
            .map_err(|_| EntityInitError::Allocation)?;
        Ok(Box::new(Self {
            members,
            order: EntityOrder::with_bucket_count(buckets),
            spatial: None,
        }))
    }
    /// Init82d26bc0/82d36748/82e92cb8: `MemStore`80 bytes x24/50/40.
    /// Prop Init82e67788 uses the same tree with80-byte blocks x256.
    pub(crate) fn try_initialize(&mut self) -> Result<(), EntityInitError> {
        if self.spatial.is_some() {
            return Err(EntityInitError::AlreadyInitialized);
        }
        self.spatial = Some(LocatorKdTree::try_with_blocks(N).ok_or(EntityInitError::Allocation)?);
        Ok(())
    }
    /// Captures the source begin/end relation once, before later cvar reads.
    pub(crate) fn captured_member_count(&self) -> usize {
        self.members.len()
    }
    pub(crate) fn initialized(&self) -> bool {
        self.spatial.is_some()
    }

    pub(crate) fn get(&self, id: u32) -> Option<&E> {
        self.members.get(&id)
    }
    pub(crate) fn get_mut(&mut self, id: u32) -> Option<&mut E> {
        self.members.get_mut(&id)
    }
    /// Borrow one payload for an update and the other live payloads for queries.
    /// Membership, source iteration order and KD entries stay untouched. The
    /// caller supplies its current fields for queries of the updating entity.
    pub(crate) fn split_entity_mut(&mut self, id: u32) -> Option<(&mut E, Vec<(u32, &E)>)> {
        let mut selected = None;
        let mut others = Vec::with_capacity(self.members.len().saturating_sub(1));
        for (&key, value) in &mut self.members {
            if key == id {
                selected = Some(value);
            } else {
                others.push((key, &*value));
            }
        }
        selected.map(|value| (value, others))
    }
    pub(crate) fn ids(&self) -> Vec<u32> {
        self.order.ids()
    }
    /// Direct native inter-entity callbacks may mutate a peer during the
    /// selected entity's update. Keep every payload in the SAME roster and
    /// borrow disjoint entries; do not defer calls or clone peer state.
    /// The KD tree of the members' positions (`GetEntitiesWithinRange`).
    pub(crate) fn spatial(&self) -> Option<&LocatorKdTree> {
        self.spatial.as_ref()
    }
    pub(crate) fn split_entity_and_peers_mut(&mut self, id: u32)
        -> Option<(&mut E, Vec<(u32, &mut E)>)> {
        let mut selected = None;
        let mut peers = Vec::with_capacity(self.members.len().saturating_sub(1));
        for (&key, value) in &mut self.members {
            if key == id { selected = Some(value); }
            else { peers.push((key, value)); }
        }
        selected.map(|entity| (entity, peers))
    }
    /// The spatial index is a distinct retained field: source callback queries
    /// it while mutating payloads, without rebuilding or cloning its ordering.
    pub(crate) fn split_entity_and_peers_and_spatial_mut(&mut self,id:u32)
        ->Option<(&mut E,Vec<(u32,&mut E)>,&LocatorKdTree)>{
        let spatial=self.spatial.as_ref()?;
        let mut selected=None;
        let mut peers=Vec::with_capacity(self.members.len().saturating_sub(1));
        for (&key,value) in &mut self.members{
            if key==id{selected=Some(value);}else{peers.push((key,value));}
        }
        selected.map(|entity|(entity,peers,spatial))
    }
    /// RemoveEntity82d26d48 changes ONLY the map. Source clears/rebuilds
    /// spatial lookups at its separate scheduling boundary, not here.
    pub(crate) fn remove(&mut self, id: u32) -> Option<E> {
        let removed = self.members.remove(&id);
        if removed.is_some() {
            self.order.remove(id);
        }
        removed
    }
    pub(crate) fn clear_spatial_lookups(&mut self) -> Result<(), EntityMutationError> {
        self.spatial
            .as_mut()
            .ok_or(EntityMutationError::MissingInitialization)?
            .reset();
        Ok(())
    }
    pub(crate) fn find_near<M: ReckoningNumeric>(
        &self,
        point: [f32; 2],
        radius: f32,
        max_items: u32,
        numeric: &mut M,
    ) -> Result<Vec<(u32, f32)>, EntitySpatialError<M::Error>> {
        self.spatial
            .as_ref()
            .ok_or(EntitySpatialError::MissingInitialization)?
            .find_near_numeric(point, radius, max_items, numeric)
            .map(|found| {
                found
                    .into_iter()
                    .map(|(id, distance)| (id as u32, distance))
                    .collect()
            })
            .map_err(EntitySpatialError::Query)
    }

    /// Native `KDTree<uint,Vector2,2>::FindNear`82d28c00 stores zero to the
    /// result count before testing its root. The constructor-null-root path
    /// leaves both output arrays untouched. A populated tree needs traversal;
    /// a published entity roster with a null tree is not accepted as an empty
    /// first-build scene because that could hide a spawned pedestrian.
    pub(crate) fn try_find_near_constructor_empty(
        &self,
        _point: [f32; 2],
        _radius: f32,
        max_items: u32,
        result_count: &mut u32,
        _entity_ids: &mut [u32],
        _distances: &mut [f32],
    ) -> Result<(), EntityNearQueryError> {
        if self.spatial.is_none() {
            return Err(EntityNearQueryError::MissingInitialization);
        }
        *result_count = 0; // 82d28c34, before max-items and root checks.
        if max_items > 256 {
            return Err(EntityNearQueryError::MaxItemsExceedsNativeLimit);
        }
        if !self.spatial.as_ref().expect("initialized tree").is_empty() {
            return Err(EntityNearQueryError::PopulatedTreeUnrecovered);
        }
        if !self.members.is_empty() {
            return Err(EntityNearQueryError::PopulatedRosterUnrecovered);
        }
        Ok(())
    }
    /// The same `FindNear`82d28c00 prefix for a caller whose point and radius
    /// come from producers the constructor-empty route never reads: the count
    /// store precedes both, and every populated state stays an explicit error.
    pub(crate) fn try_constructor_empty_count(
        &self,
        max_items: u32,
    ) -> Result<u32, EntityNearQueryError> {
        let mut count = 0;
        self.try_find_near_constructor_empty(
            [0.0; 2],
            0.0,
            max_items,
            &mut count,
            &mut [],
            &mut [],
        )?;
        Ok(count)
    }
}

impl<const N: usize, E: SpatialEntity> EntityMan<N, E> {
    /// AddEntity82d26dc0/82e67910/82e92ee8: map publication precedes the spatial insertion.
    /// A duplicate key keeps the previous payload, but still inserts the
    /// passed entity's coordinate, as the source ignores DoInsertValue.success.
    /// The KD insertion's full-store false is also ignored by the native void
    /// callback. Removal retires nodes at the next scheduled spatial rebuild.
    pub(crate) fn add(&mut self, id: u32, entity: E) -> Result<(), EntityMutationError> {
        self.add_retained(id, &mut Some(entity))
    }
    /// Same AddEntity; transfer payload ownership only at map publication.
    /// A failed prefix or duplicate keeps the caller's original value. Needed
    /// for entities whose admitted physics bodies cannot be cleaned up by Drop.
    pub(crate) fn add_retained(
        &mut self,
        id: u32,
        entity: &mut Option<E>,
    ) -> Result<(), EntityMutationError> {
        let tree = self
            .spatial
            .as_mut()
            .ok_or(EntityMutationError::MissingInitialization)?;
        let position = entity
            .as_ref()
            .ok_or(EntityMutationError::MissingPayload)?
            .core_position_xz();
        if !self.members.contains_key(&id) {
            if self.members.len() == N {
                return Err(EntityMutationError::MapCapacity);
            }
            self.members
                .insert(id, entity.take().expect("checked payload"));
            self.order.insert(id);
        }
        // Prop82e6797c returns immediately after Insert, without testing r3;
        // vehicle82d26e2c and pedestrian82e92f54 have the same contract.
        // Paused replay/menu streaming can exhaust old nodes while the live
        // map still has room. Preserve the admitted entity and continue.
        tree.insert(u64::from(id), position);
        Ok(())
    }
    /// UpdateSpatialLookups82d26c28/82e92dc8 does NOT clear first. Caller
    /// must retain the original separate ClearSpatialLookups phase.
    pub(crate) fn update_spatial_lookups(&mut self) -> Result<(), EntityMutationError> {
        let tree = self
            .spatial
            .as_mut()
            .ok_or(EntityMutationError::MissingInitialization)?;
        for id in self.order.ids() {
            let entity = self.members.get(&id).expect("entity map order");
            if !entity.core_data_valid() {
                return Err(EntityMutationError::InvalidCore(id));
            }
            // Native82e678d8/82d26d10/82e92eb0 continue the map traversal
            // regardless of Insert's result, just like AddEntity.
            tree.insert(u64::from(id), entity.core_position_xz());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityInitError, EntityMan, EntityNearQueryError};
    struct Entity([f32; 2]);
    impl super::SpatialEntity for Entity {
        fn core_position_xz(&self) -> [f32; 2] {
            self.0
        }
        fn core_data_valid(&self) -> bool {
            true
        }
    }
    #[test]
    fn removal_leaves_spatial_membership_until_the_explicit_rebuild_phase() {
        let mut owner = EntityMan::<24, Entity>::try_construct().unwrap();
        owner.try_initialize().unwrap();
        owner.add(3, Entity([0., 0.])).unwrap();
        owner.add(26, Entity([1., 0.])).unwrap();
        assert_eq!(owner.ids(), vec![26, 3]); // collision in original23 buckets
        owner.remove(3).unwrap();
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        let found = owner.find_near([0., 0.], 2., 24, &mut numeric).unwrap();
        assert_eq!(
            found.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![3, 26]
        );
        owner.clear_spatial_lookups().unwrap();
        owner.update_spatial_lookups().unwrap();
        let found = owner.find_near([0., 0.], 2., 24, &mut numeric).unwrap();
        assert_eq!(
            found.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![26]
        );
        assert!(owner.initialized()); // initialized does not mean empty
    }
    #[test]
    fn prop_spatial_phases_recycle_nodes_across_more_than_256_culls() {
        let mut owner = EntityMan::<256, Entity>::try_construct().unwrap();
        owner.try_initialize().unwrap();
        owner.add(1, Entity([0., 0.])).unwrap();
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        for id in 2..1026 {
            owner.add(id, Entity([1., 0.])).unwrap();
            owner.remove(id).unwrap();
            // RemoveEntity intentionally leaves the old KD entry until the
            // Manager's boundary. Rebuild also publishes moved live props.
            owner.get_mut(1).unwrap().0 = [id as f32, 0.];
            owner.clear_spatial_lookups().unwrap();
            owner.update_spatial_lookups().unwrap();
            assert_eq!(owner.captured_member_count(), 1);
            let found = owner.find_near([id as f32, 0.], 0.5, 256, &mut numeric).unwrap();
            assert_eq!(found.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [1]);
            assert!(owner.find_near([1., 0.], 0.5, 256, &mut numeric).unwrap().is_empty());
        }
    }

    #[test]
    fn constructor_and_memstore_init_have_distinct_lifetimes() {
        let mut owner = EntityMan::<24>::try_construct().unwrap();
        assert_eq!(owner.captured_member_count(), 0);
        assert!(!owner.initialized());
        owner.try_initialize().unwrap();
        assert!(owner.initialized());
        assert_eq!(owner.captured_member_count(), 0);
        assert_eq!(
            owner.try_initialize(),
            Err(EntityInitError::AlreadyInitialized)
        );
    }

    #[test]
    fn native_null_root_query_zeros_only_count_and_rejects_unrecovered_admission() {
        fn query(
            owner: &EntityMan<40>,
            max_items: u32,
            count: &mut u32,
            ids: &mut [u32; 20],
            distances: &mut [f32; 20],
        ) -> Result<(), EntityNearQueryError> {
            owner.try_find_near_constructor_empty(
                [0.0, 0.0],
                10.0,
                max_items,
                count,
                ids,
                distances,
            )
        }
        let mut owner = EntityMan::<40>::try_construct().unwrap();
        let mut count = 9;
        let mut ids = [12; 20];
        let mut distances = [3.0; 20];
        assert_eq!(
            query(&owner, 20, &mut count, &mut ids, &mut distances),
            Err(EntityNearQueryError::MissingInitialization)
        );
        assert_eq!(count, 9);
        owner.try_initialize().unwrap();
        assert_eq!(
            query(&owner, 257, &mut count, &mut ids, &mut distances),
            Err(EntityNearQueryError::MaxItemsExceedsNativeLimit)
        );
        assert_eq!(count, 0);
        assert_eq!(
            query(&owner, 20, &mut count, &mut ids, &mut distances),
            Ok(())
        );
        assert_eq!(ids, [12; 20]);
        assert_eq!(distances, [3.0; 20]);
        owner.spatial.as_mut().unwrap().insert(13, [0.; 2]);
        assert_eq!(
            query(&owner, 20, &mut count, &mut ids, &mut distances),
            Err(EntityNearQueryError::PopulatedTreeUnrecovered)
        );
        owner.clear_spatial_lookups().unwrap();
        owner.members.insert(13, ());
        assert_eq!(
            query(&owner, 20, &mut count, &mut ids, &mut distances),
            Err(EntityNearQueryError::PopulatedRosterUnrecovered)
        );
    }
}
