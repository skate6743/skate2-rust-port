use super::super::{
    SimulationDmoAssemblyPartOwner, resident_model_catalog::tests::placed_locator_plan,
};
use super::*;
use crate::{
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBBoxMinMaxNumeric,
    },
    serialised_assembly_template::tests::body_only_template_for_test,
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationXenonAddressImage,
    },
};
use skate2_formats::pegasus::ObjectId;

const PLACEMENT: [f32; 16] = [
    1.0, 0.0, 0.0, 3.0, 0.0, 1.0, 0.0, 5.0, 0.0, 0.0, 1.0, 7.0, 11.0, 13.0, 17.0, 19.0,
];

pub(crate) fn fixture(
    capacity: usize,
    part_count: u32,
) -> (
    SourceBackend,
    SimulationManagerContactDomain,
    Arc<SimulationDmoResolvedLocatorPartOwner>,
) {
    let template = body_only_template_for_test(part_count);
    let plan = Arc::clone(template.owner());
    let asset = Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
        Arc::clone(&plan),
        vec![template],
    ));
    let mut backend = SourceBackend::try_new(capacity, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    let binding = Arc::new(
        SimulationArenaXenonAddressBinding::try_new(Arc::clone(&plan), [0x2000, 0, 0, 0, 0])
            .unwrap(),
    );
    backend
        .try_bind_source_arenas(
            SimulationXenonAddressImage::try_new(
                0x1000_0080,
                u32::try_from(capacity).unwrap(),
                0,
                0,
            )
            .unwrap(),
            vec![(plan, binding)],
        )
        .unwrap();
    // Controlled fixture bypasses catalog traversal but uses the same exact
    // validated asset/template binder and parsed source locator occurrence.
    let parts =
        SimulationDmoAssemblyPartOwner::try_new_from_validated_route(asset, ObjectId(3), &backend)
            .unwrap();
    let locators = placed_locator_plan(&[(0x1234, &[(1, 2, 3)])], PLACEMENT);
    let owner = Arc::new(SimulationDmoResolvedLocatorPartOwner {
        locator: locators.try_locator_occurrence(0, 0).unwrap(),
        parts,
    });
    (backend, manager, owner)
}

#[test]
fn fresh_candidate_uses_locator_and_definition_not_serialized_header() {
    let (_backend, _manager, owner) = fixture(3, 2);
    assert!(owner.parts.template().culling_table().is_some());
    assert_ne!(owner.parts.template().assembly().group_id, 0);
    let candidate = SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap();
    assert_eq!(candidate.world_index(), 0xffff);
    assert_eq!(candidate.requested_state(), None);
    assert_eq!(candidate.group, 0);
    assert_eq!(candidate.part_user_data, [0, 0]);
    assert!(candidate.culling_table.is_none());
    assert!(!candidate.is_attached());
    for matrix in candidate.parts.part_transforms() {
        assert_eq!(matrix.map(f32::to_bits), PLACEMENT.map(f32::to_bits));
    }
}

#[test]
fn admitted_occurrences_share_manager_ids_and_move_with_their_publication() {
    let (mut backend, mut manager, owner) = fixture(5, 1);
    let mut pool = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut handles = Vec::new();
    for _ in 0..3 {
        handles.push(
            pool.try_insert(
                &mut manager,
                &mut backend,
                SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
                4,
                &mut HostSerialisedAssemblyBBoxNumeric,
            )
            .unwrap(),
        );
    }
    let ids = handles
        .iter()
        .map(|handle| pool.candidate(handle).unwrap().world_index())
        .collect::<Vec<_>>();
    assert_eq!(ids[1], ids[0].wrapping_add(1));
    assert_eq!(ids[2], ids[1].wrapping_add(1));
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
    let moved_bodies = pool
        .publication(&handles[2])
        .unwrap()
        .rigid_bodies()
        .to_vec();
    for member in &pool.members {
        let (_, publication) = member.content.source().unwrap();
        assert_eq!(publication.header().group, 0);
        assert!(publication.header().culling_table.is_none());
        assert_eq!(member.bounds, publication.header().aabb);
    }
    let removed = pool
        .try_remove_serialized(&mut backend, &handles[1])
        .unwrap();
    assert!(!removed.is_attached());
    assert_eq!(removed.world_index(), ids[1]);
    assert_eq!(pool.len(), 2);
    assert!(pool.handles().nth(1).unwrap().same_as(&handles[2]));
    assert_eq!(
        pool.publication(&handles[2]).unwrap().rigid_bodies(),
        moved_bodies
    );
    assert!(matches!(
        pool.try_remove_serialized(&mut backend, &handles[1]),
        Err(SimulationDmoPoolError::MissingMember)
    ));
    for handle in [&handles[0], &handles[2]] {
        pool.try_remove_serialized(&mut backend, handle).unwrap();
    }
    assert_eq!(backend.rigid_body_snapshot().counts.free, 4);
}

#[test]
fn end_update_after_a_full_mesh_collection_publishes_the_retained_records() {
    let (mut backend, mut manager, owner) = fixture(3, 1);
    let mut pool = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let handle = pool
        .try_insert(
            &mut manager,
            &mut backend,
            SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
            4,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap();
    let created = pool.publication(&handle).unwrap().header().aabb;
    // BeginUpdate met a full mesh collection and returned before the base update.
    pool.base_update_skipped = true;
    backend.begin_dynamic_update_frame(1. / 60.);
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    assert!(!pool.publication(&handle).unwrap().has_bounds_for_frame(frame));
    let active = pool
        .try_end_update(&backend, &mut HostSerialisedAssemblyBBoxNumeric)
        .unwrap();
    assert!(!pool.base_update_skipped);
    let publication = pool.publication(&handle).unwrap();
    assert!(publication.has_bounds_for_frame(frame));
    assert_eq!(publication.header().aabb, created);
    // EndUpdate's own active list: the assembly is active unless every Part is frozen.
    let body = publication.rigid_bodies()[0];
    let frozen = backend.collision_body_pool().try_raw_state(body).unwrap() & 2 != 0;
    assert_eq!(active, if frozen { Vec::new() } else { vec![0] });
}

#[test]
fn preflight_refuses_foreign_backend_before_world_id_or_attachment() {
    let (backend, manager, owner) = fixture(2, 1);
    assert!(matches!(
        SimulationDmoAssemblyPool::try_new(&manager, &backend, true),
        Err(SimulationDmoPoolError::UnsupportedParallel)
    ));
    let (mut other, mut other_manager, _) = fixture(2, 1);
    let mut pool = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let failure = pool
        .try_insert(
            &mut other_manager,
            &mut other,
            SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
            4,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap_err();
    assert!(matches!(
        failure.error,
        SimulationDmoPoolError::ForeignBackend
    ));
    assert_eq!(failure.candidate.world_index(), 0xffff);
    assert_eq!(failure.candidate.requested_state(), None);
    assert!(!failure.candidate.is_attached());
    assert_eq!(pool.len(), 0);
}

#[test]
fn body_exhaustion_keeps_issued_id_and_original_candidate() {
    let (mut backend, mut manager, owner) = fixture(1, 1);
    let mut pool = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let first = pool
        .try_insert(
            &mut manager,
            &mut backend,
            SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
            4,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap_err();
    assert!(matches!(
        first.error,
        SimulationDmoPoolError::Attach(SimulationDmoAssemblyPartAttachError::PartUnavailable {
            ordinal: 0
        })
    ));
    assert!(!first.candidate.is_attached());
    assert_eq!(first.candidate.requested_state(), Some(4));
    let first_id = first.candidate.world_index();
    let second = pool
        .try_insert(
            &mut manager,
            &mut backend,
            first.candidate,
            4,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap_err();
    assert_eq!(second.candidate.world_index(), first_id.wrapping_add(1));
    assert_eq!(pool.len(), 0);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 0);
}

struct RefuseNumeric;
impl SerialisedAssemblyBBoxMinMaxNumeric for RefuseNumeric {
    type Error = &'static str;
    fn minimum(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Err("numeric")
    }
    fn maximum(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Err("numeric")
    }
}
impl SerialisedAssemblyPartBBoxNumeric for RefuseNumeric {
    fn sum_three_products(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        Err("numeric")
    }
    fn reciprocal_square_root_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        Err("numeric")
    }
}

#[test]
fn create_refusal_retains_attached_candidate_until_explicit_source_detach() {
    let (mut backend, mut manager, owner) = fixture(2, 1);
    let mut pool = SimulationDmoAssemblyPool::try_new(&manager, &backend, false).unwrap();
    let mut failure = pool
        .try_insert(
            &mut manager,
            &mut backend,
            SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap(),
            4,
            &mut RefuseNumeric,
        )
        .unwrap_err();
    assert!(matches!(
        failure.error,
        SimulationDmoPoolError::Create(SerialisedAssemblyCreateError::Numeric("numeric"))
    ));
    assert!(failure.candidate.is_attached());
    assert_eq!(pool.len(), 0);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 0);
    let issued = failure.candidate.world_index();
    failure.candidate.try_detach(&mut backend).unwrap();
    assert!(!failure.candidate.is_attached());
    assert_eq!(failure.candidate.world_index(), issued);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
    let handle = pool
        .try_insert(
            &mut manager,
            &mut backend,
            failure.candidate,
            4,
            &mut HostSerialisedAssemblyBBoxNumeric,
        )
        .unwrap();
    assert_eq!(
        pool.candidate(&handle).unwrap().world_index(),
        issued.wrapping_add(1)
    );
    pool.try_remove_serialized(&mut backend, &handle).unwrap();
}
