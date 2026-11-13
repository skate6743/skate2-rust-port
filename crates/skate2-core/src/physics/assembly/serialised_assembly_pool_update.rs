//! Nonparallel `AssemblyPool` refresh and bounds publication.
//!
//! `BeginUpdate` (0x82e0dc98) updates all serialized volumes first;
//! `AssemblyPoolUpdateJob` (0x827cdf78) computes separate bounds for all
//! assemblies; `EndUpdate` (0x82e0bb10) publishes those bounds, then rebuilds
//! the active list from live Part state. Native worker scheduling is replaced
//! by synchronous execution under one immutable body-pool borrow.

use super::{
    PreparedSerialisedAssembly, SerialisedAssemblyCreatePartInput,
    SerialisedAssemblyLiveAssemblyInput, SerialisedAssemblyPackedBodyMotionWords,
    SerialisedAssemblyTemplate, SerialisedAssemblyUpdateError, SimulationPackedBodyWordImageDomain,
    SimulationPackedBodyWordProviderError, SimulationRigidBodyAddInertiaOwner,
    SimulationRigidBodyPool, SimulationRigidBodyPoolError, SimulationXenonAddressImageError,
    packed_body_motion,
};
use crate::{
    octree::AabbWords,
    serialised_assembly_bbox::{
        SerialisedAssemblyPartBBoxNumeric, try_calculate_primitive_part_bbox, try_union_part_bboxes,
    },
};
use skate2_formats::collision::VolumeType;

#[cfg(test)]
#[path = "serialised_assembly_pool_update_tests.rs"]
mod tests;

/// Live owner-qualified inputs to the existing serialized Update, in pool order.
/// Caller retains the original Part/material/flag sources, not newly guessed
/// defaults. Disjoint mutable borrows prevent duplicate assembly publication.
pub(crate) struct SerialisedAssemblyPoolUpdateInput<'a> {
    pub(crate) assembly: &'a mut PreparedSerialisedAssembly,
    pub(crate) template: &'a SerialisedAssemblyTemplate,
    pub(crate) inertia_owner: &'a SimulationRigidBodyAddInertiaOwner,
    pub(crate) parts: &'a [SerialisedAssemblyCreatePartInput],
    pub(crate) volume_flags: &'a [u32],
    pub(crate) live: SerialisedAssemblyLiveAssemblyInput<'a>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SerialisedAssemblyPoolUpdateError<N> {
    FrameUnavailable,
    ForeignImage {
        assembly: usize,
    },
    StaleVolumes {
        assembly: usize,
    },
    Update {
        assembly: usize,
        source: SerialisedAssemblyUpdateError<SimulationPackedBodyWordProviderError>,
    },
    BodyIndex {
        assembly: usize,
        volume: usize,
        index: u16,
    },
    Body {
        assembly: usize,
        source: SimulationRigidBodyPoolError,
    },
    Address {
        assembly: usize,
        source: SimulationXenonAddressImageError,
    },
    UnsupportedVolume {
        assembly: usize,
        volume: usize,
    },
    Numeric(N),
    Allocation,
}

/// The returned ascending ordinals are `EndUpdate`'s active list, not island
/// assignments or ground filtering. All-static but non-frozen parts are active.
///
/// Checked host failures may leave earlier `VolumeData` updates visible, but
/// invalidate every input's bounds stamp and publish no new Header AABBs until
/// all bounds succeed. This is explicit host failure policy, not native fault
/// rollback. Pool/image borrows prevent mutation DURING this call; the caller
/// must enter its collision phase before any later same-frame body mutation.
pub(crate) fn try_update_serialised_assembly_pool<N: SerialisedAssemblyPartBBoxNumeric>(
    inputs: &mut [SerialisedAssemblyPoolUpdateInput<'_>],
    pool: &SimulationRigidBodyPool,
    image: &SimulationPackedBodyWordImageDomain,
    numeric: &mut N,
) -> Result<Vec<usize>, SerialisedAssemblyPoolUpdateError<N::Error>> {
    // Retain the legacy checked-failure prefix before allocating the adapter.
    for input in inputs.iter_mut() {
        input.assembly.bounds_frame = None;
    }
    pool.frame_identity()
        .ok_or(SerialisedAssemblyPoolUpdateError::FrameUnavailable)?;
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(inputs.len())
        .map_err(|_| SerialisedAssemblyPoolUpdateError::Allocation)?;
    for input in inputs {
        entries.push(AssemblyPoolUpdateEntry::Source {
            input: SerialisedAssemblyPoolUpdateInput {
                assembly: &mut *input.assembly,
                template: input.template,
                inertia_owner: input.inertia_owner,
                parts: input.parts,
                volume_flags: input.volume_flags,
                live: SerialisedAssemblyLiveAssemblyInput {
                    group: input.live.group,
                    culling_table: input.live.culling_table,
                },
            },
            image,
        });
    }
    try_update_assembly_pool_entries(&mut entries, pool, numeric).map_err(|error| match error {
        AssemblyPoolEntryUpdateError::Source(error) => error,
        AssemblyPoolEntryUpdateError::Character { .. } => unreachable!("source-only inputs"),
    })
}

use crate::{
    dynamic_assembly_session::character_publication::{
        CharacterCollisionPublication, CharacterPublicationError,
    },
    living_world_grind_services::character_models::collision_assembly::{
        CharacterCollisionAssembly, CharacterCollisionBackend,
    },
};

/// Both storage routes feed the SAME BeginUpdate/job/EndUpdate ordering.
/// Character definitions are constructed by the game; they have no serialized
/// arena/image identity. Each source route still requires its original image.
pub(crate) enum AssemblyPoolUpdateEntry<'a> {
    Source {
        input: SerialisedAssemblyPoolUpdateInput<'a>,
        image: &'a SimulationPackedBodyWordImageDomain,
    },
    Character {
        assembly: &'a mut CharacterCollisionPublication,
        owner: &'a CharacterCollisionAssembly,
        backend: &'a CharacterCollisionBackend,
    },
}

#[derive(Debug)]
pub(crate) enum AssemblyPoolEntryUpdateError<N> {
    Source(SerialisedAssemblyPoolUpdateError<N>),
    Character {
        assembly: usize,
        source: CharacterPublicationError,
    },
}

pub(crate) fn try_update_assembly_pool_entries<N: SerialisedAssemblyPartBBoxNumeric>(
    entries: &mut [AssemblyPoolUpdateEntry<'_>],
    pool: &SimulationRigidBodyPool,
    numeric: &mut N,
) -> Result<Vec<usize>, AssemblyPoolEntryUpdateError<N::Error>> {
    use AssemblyPoolEntryUpdateError as E;
    use AssemblyPoolUpdateEntry as Entry;
    for entry in entries.iter_mut() {
        match entry {
            Entry::Source { input, .. } => input.assembly.bounds_frame = None,
            Entry::Character { assembly, .. } => assembly.invalidate_refresh(),
        }
    }
    let frame = pool.frame_identity().ok_or(E::Source(
        SerialisedAssemblyPoolUpdateError::FrameUnavailable,
    ))?;
    for (ordinal, entry) in entries.iter_mut().enumerate() {
        match entry {
            Entry::Source { input, image } => {
                input
                    .assembly
                    .try_update_from_xenon(
                        input.template,
                        pool,
                        input.inertia_owner,
                        image,
                        input.parts,
                        input.volume_flags,
                        input.live,
                    )
                    .map_err(|source| {
                        E::Source(SerialisedAssemblyPoolUpdateError::Update {
                            assembly: ordinal,
                            source,
                        })
                    })?;
            }
            Entry::Character {
                assembly,
                owner,
                backend,
            } => {
                if backend.authority() != pool.authority() {
                    return Err(E::Character {
                        assembly: ordinal,
                        source: CharacterPublicationError::ForeignBackend,
                    });
                }
                assembly
                    .try_update_volumes(owner, backend)
                    .map_err(|source| E::Character {
                        assembly: ordinal,
                        source,
                    })?;
            }
        }
    }
    let mut bounds = Vec::new();
    bounds
        .try_reserve_exact(entries.len())
        .map_err(|_| E::Source(SerialisedAssemblyPoolUpdateError::Allocation))?;
    for (ordinal, entry) in entries.iter().enumerate() {
        bounds.push(match entry {
            Entry::Source { input, image } => {
                try_stage_assembly_bounds(input.assembly, ordinal, pool, image, numeric)
                    .map_err(E::Source)?
            }
            Entry::Character {
                assembly,
                owner,
                backend,
            } => assembly
                .try_stage_bounds(owner, backend)
                .map_err(|source| E::Character {
                    assembly: ordinal,
                    source,
                })?,
        });
    }
    let mut active = Vec::new();
    active
        .try_reserve_exact(entries.len())
        .map_err(|_| E::Source(SerialisedAssemblyPoolUpdateError::Allocation))?;
    // Source EndUpdate publishes ALL header bounds before rescanning live Parts.
    for (entry, bound) in entries.iter_mut().zip(bounds) {
        match entry {
            Entry::Source { input, .. } => input.assembly.header.aabb = bound,
            Entry::Character { assembly, .. } => assembly.publish_unstamped_bounds(bound),
        }
    }
    collect_active_entries::<N::Error>(entries, pool, &mut active)?;
    for entry in entries {
        match entry {
            Entry::Source { input, .. } => input.assembly.bounds_frame = Some(frame),
            Entry::Character { assembly, .. } => assembly.stamp_bounds(frame),
        }
    }
    Ok(active)
}

/// `EndUpdate` (0x82e0bb10) after a `BeginUpdate` that returned before the base update (a
/// full `DynamicPool` mesh collection): no bounds job ran, so copying the retained bounds over
/// the Header changes nothing, and every record keeps what the last Update (or Insert's Create)
/// wrote. Those are this frame's records; the active list is rebuilt from live Part state.
pub(crate) fn try_end_assembly_pool_entries_unrefreshed<N>(
    entries: &mut [AssemblyPoolUpdateEntry<'_>],
    pool: &SimulationRigidBodyPool,
) -> Result<Vec<usize>, AssemblyPoolEntryUpdateError<N>> {
    use AssemblyPoolEntryUpdateError as E;
    use AssemblyPoolUpdateEntry as Entry;
    let frame = pool.frame_identity().ok_or(E::Source(
        SerialisedAssemblyPoolUpdateError::FrameUnavailable,
    ))?;
    let mut active = Vec::new();
    active
        .try_reserve_exact(entries.len())
        .map_err(|_| E::Source(SerialisedAssemblyPoolUpdateError::Allocation))?;
    collect_active_entries::<N>(entries, pool, &mut active)?;
    for entry in entries {
        match entry {
            Entry::Source { input, .. } => {
                input.assembly.volume_frame = Some(frame);
                input.assembly.bounds_frame = Some(frame);
            }
            Entry::Character { assembly, .. } => assembly.retain_for_frame(frame),
        }
    }
    Ok(active)
}

/// `EndUpdate` 0x82e0bbb4..bc38's active list: each assembly with a Part that is not frozen.
fn collect_active_entries<N>(
    entries: &[AssemblyPoolUpdateEntry<'_>],
    pool: &SimulationRigidBodyPool,
    active: &mut Vec<usize>,
) -> Result<(), AssemblyPoolEntryUpdateError<N>> {
    use AssemblyPoolEntryUpdateError as E;
    use AssemblyPoolUpdateEntry as Entry;
    for (ordinal, entry) in entries.iter().enumerate() {
        let mut all_frozen = true;
        let mut visit = |body| -> Result<(), AssemblyPoolEntryUpdateError<N>> {
            let state = pool.try_raw_state(body).map_err(|source| {
                E::Source(SerialisedAssemblyPoolUpdateError::Body {
                    assembly: ordinal,
                    source,
                })
            })?;
            all_frozen &= state & 2 != 0;
            Ok(())
        };
        match entry {
            Entry::Source { input, .. } => {
                for part in input.parts {
                    visit(part.body)?;
                }
            }
            Entry::Character { assembly, .. } => {
                for &body in assembly.rigid_bodies() {
                    visit(body)?;
                }
            }
        }
        if !all_frozen {
            active.push(ordinal);
        }
    }
    Ok(())
}

/// Job output is separate from Header until every assembly has completed.
fn try_stage_assembly_bounds<N: SerialisedAssemblyPartBBoxNumeric>(
    assembly: &PreparedSerialisedAssembly,
    ordinal: usize,
    pool: &SimulationRigidBodyPool,
    image: &SimulationPackedBodyWordImageDomain,
    numeric: &mut N,
) -> Result<AabbWords, SerialisedAssemblyPoolUpdateError<N::Error>> {
    if !image.is_same_pool(pool) || !assembly.matches_ground_image(image) {
        return Err(SerialisedAssemblyPoolUpdateError::ForeignImage { assembly: ordinal });
    }
    if assembly.volume_frame != pool.frame_identity() || assembly.volume_frame.is_none() {
        return Err(SerialisedAssemblyPoolUpdateError::StaleVolumes { assembly: ordinal });
    }
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(assembly.volume_data.records().len())
        .map_err(|_| SerialisedAssemblyPoolUpdateError::Allocation)?;
    for (volume_ordinal, volume) in assembly.volume_data.records().iter().enumerate() {
        // Source resolves the table entry BEFORE testing Aggregate, but does
        // not dereference an Aggregate's selected body or numeric fields.
        let body = *assembly
            .rigid_bodies
            .get(usize::from(volume.rigid_body_index))
            .ok_or(SerialisedAssemblyPoolUpdateError::BodyIndex {
                assembly: ordinal,
                volume: volume_ordinal,
                index: volume.rigid_body_index,
            })?;
        if volume.volume_type == VolumeType::Aggregate {
            entries.push((AabbWords::from_f32([0.0; 4], [0.0; 4]), 0));
            continue;
        }
        let snapshot = pool
            .try_serialised_assembly_snapshot(body)
            .map_err(|source| SerialisedAssemblyPoolUpdateError::Body {
                assembly: ordinal,
                source,
            })?;
        let topology = pool
            .try_packed_word_topology_snapshot(body)
            .map_err(|source| SerialisedAssemblyPoolUpdateError::Body {
                assembly: ordinal,
                source,
            })?;
        let words = SerialisedAssemblyPackedBodyMotionWords {
            right_link: image
                .rigid_body_node_address(topology.right_link)
                .map_err(|source| SerialisedAssemblyPoolUpdateError::Address {
                    assembly: ordinal,
                    source,
                })?,
            left_link: image
                .rigid_body_node_address(topology.left_link)
                .map_err(|source| SerialisedAssemblyPoolUpdateError::Address {
                    assembly: ordinal,
                    source,
                })?,
        };
        let bbox = try_calculate_primitive_part_bbox(
            &volume.volume,
            &volume.transform,
            packed_body_motion(snapshot, words),
            numeric,
        )
        .map_err(SerialisedAssemblyPoolUpdateError::Numeric)?
        .ok_or(SerialisedAssemblyPoolUpdateError::UnsupportedVolume {
            assembly: ordinal,
            volume: volume_ordinal,
        })?;
        entries.push((bbox, 1));
    }
    // Includes zero/all-Aggregate lists, unlike Create's nonempty requirement.
    try_union_part_bboxes(&entries, numeric).map_err(SerialisedAssemblyPoolUpdateError::Numeric)
}
