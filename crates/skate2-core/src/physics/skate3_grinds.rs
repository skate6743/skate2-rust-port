//! Retained Skate 2 spline payloads and source occurrences for the S3 runtime.
use crate::skate3_native::{HostGrindAsset, HostGrindSegment};
use crate::{
    grind_data_runtime::GrindSplineSource, physical_player_assembly::PhysicalAssemblyWorld,
};

#[derive(Default)]
pub(super) struct HostGrinds {
    sources: Vec<GrindSplineSource>,
    cached: Vec<CachedSource>,
    /// Dropped props' splines, by entity and placement, in entity order.
    props: Vec<CachedProp>,
    /// Ticks before a MOVING prop may rebuild the grind world again.
    prop_cooldown: u32,
    initialized: bool,
    next_handle: u64,
}

/// Dropped props are dynamic: one pushed along re-places its rails at most
/// this often (and once more when it comes to rest), not every tick.
const MOVING_PROP_REBUILD_TICKS: u32 = 6;

struct CachedProp {
    key: (u32, [f32; 16]),
    /// `None`: the prop's splines hold no segment.
    asset: Option<HostGrindAsset>,
}
impl HostGrinds {
    /// Reuse immutable payloads by retained native owner and dictionary entry.
    /// Asset/segment order and first-encounter monotonic header IDs stay intact.
    fn assets(&mut self, sources: &[GrindSplineSource]) -> Result<Vec<HostGrindAsset>, String> {
        let mut assets = Vec::with_capacity(sources.len());
        for source in sources {
            let index = if let Some(index) = self
                .cached
                .iter()
                .position(|cached| source.is_same_source(&cached.source))
            {
                index
            } else {
                let segments = convert_source(source, &mut self.next_handle)?;
                self.cached.push(CachedSource {
                    source: source.clone(),
                    segments,
                });
                self.cached.len() - 1
            };
            assets.push(HostGrindAsset {
                segments: self.cached[index].segments.clone(),
            });
        }
        self.cached.retain(|cached| {
            sources
                .iter()
                .any(|source| source.is_same_source(&cached.source))
        });
        Ok(assets)
    }

    pub(super) fn sync(
        &mut self,
        world: &PhysicalAssemblyWorld,
        physics: &mut crate::skate3_native::physics::Skate3PhysicsState,
        skater: &mut crate::skate3_native::physics::Skate3ControllerState,
    ) -> Result<(), String> {
        let props = self.prop_assets(world)?;
        let owner = world
            .grind_services
            .static_data()
            .lock()
            .map_err(|_| "Host grind data lock")?;
        if self.initialized && props.is_none() && owner.matches_skate3_sources(&self.sources)? {
            return Ok(());
        }
        if let Some(props) = props {
            self.props = props;
        }
        let sources = owner.skate3_sources()?;
        drop(owner);
        #[cfg(test)]
        if !self.initialized && std::env::var_os("SKATE2_AUDIT_GRIND_STREAMING").is_some() {
            audit_cached_sources(&sources)?;
        }
        let conversion_started = std::time::Instant::now();
        let mut assets = self.assets(&sources)?;
        assets.extend(self.props.iter().filter_map(|prop| prop.asset.clone()));
        let conversion_ms = conversion_started.elapsed().as_secs_f64() * 1000.;
        let provider_started = std::time::Instant::now();
        let provider = std::sync::Arc::new(
            crate::skate3_native::grind_world::StaticProvider::from_host(&assets)?,
        );
        crate::diagnostics::test_log::event(
            "skate3_grind_refresh",
            format_args!(
                "sources={} props={} segments={} conversion_ms={conversion_ms:.3} provider_ms={:.3}",
                sources.len(),
                self.props.len(),
                assets
                    .iter()
                    .map(|asset| asset.segments.len())
                    .sum::<usize>(),
                provider_started.elapsed().as_secs_f64() * 1000.
            ),
        );
        skater
            .trajectory
            .bind_grind_world(std::sync::Arc::clone(&provider));
        physics.grind_world = provider;
        self.sources = sources;
        self.initialized = true;
        Ok(())
    }
}

struct CachedSource {
    source: GrindSplineSource,
    segments: Vec<HostGrindSegment>,
}

impl HostGrinds {
    /// The living world's moving objects' embedded grind splines at their current
    /// (physical) target transforms; `None` when nothing needs rebuilding. A
    /// prop kept in place keeps its converted segments and owner handles; a
    /// moved one is re-placed, rate-limited while the same set keeps moving.
    fn prop_assets(&mut self, world: &PhysicalAssemblyWorld) -> Result<Option<Vec<CachedProp>>, String> {
        let living = world
            .grind_services
            .living_world()
            .lock()
            .map_err(|_| "Host living world lock")?;
        let objects = living
            .dmo_objects
            .as_deref()
            .map_or_else(Vec::new, |dmos| dmos.grindable_objects());
        let objects: Vec<_> = objects
            .into_iter()
            .filter(|object| !object.grind_splines.is_empty())
            .map(|object| ((object.entity_id, object.target_transform), object))
            .collect();
        let same = |a: &(u32, [f32; 16]), b: &(u32, [f32; 16])| {
            a.0 == b.0 && crate::simulation_assembly_backend::constraint_session::skate3_world::same_prop_placement(&a.1, &b.1)
        };
        self.prop_cooldown = self.prop_cooldown.saturating_sub(1);
        let same_set = self.props.len() == objects.len()
            && self.props.iter().zip(&objects).all(|(cached, (key, _))| cached.key.0 == key.0);
        if same_set
            && (self.prop_cooldown > 0
                || self.props.iter().zip(&objects).all(|(cached, (key, _))| same(&cached.key, key)))
        {
            return Ok(None);
        }
        self.prop_cooldown = MOVING_PROP_REBUILD_TICKS;
        let mut previous = std::mem::take(&mut self.props);
        let mut props = Vec::with_capacity(objects.len());
        for (key, object) in objects {
            if let Some(index) = previous.iter().position(|cached| same(&cached.key, &key)) {
                props.push(previous.swap_remove(index));
                continue;
            }
            let asset = convert_prop(
                &object.target_transform,
                object.grind_splines.values(),
                &mut self.next_handle,
            )?;
            props.push(CachedProp { key, asset });
        }
        Ok(Some(props))
    }
}

/// Segment coefficients `a, b, c` are directions and take only the rotation;
/// `d` is the segment start (`SplineEval` at 0) and takes the full placement,
/// as `QueryGrindSplinesWithBoundingBox` transforms its endpoints. The
/// weight lanes stay authored. Each spline gets a fresh owner handle.
fn convert_prop<'a>(
    transform: &[f32; 16],
    splines: impl Iterator<Item = &'a crate::living_world_grind_services::dmo_models::DmoGrindSpline>,
    next_handle: &mut u64,
) -> Result<Option<HostGrindAsset>, String> {
    let rotate = |v: &[f32]| -> [f32; 4] {
        [0, 1, 2, 3].map(|lane| {
            if lane == 3 {
                v[3]
            } else {
                transform[lane] * v[0] + transform[4 + lane] * v[1] + transform[8 + lane] * v[2]
            }
        })
    };
    let mut segments = Vec::new();
    for spline in splines {
        let data = spline.source.data();
        let header = data
            .spline_at_object_offset(spline.header_offset)
            .ok_or("Prop grind spline lacks its header")?;
        let (Some(head), Some(tail)) = (header.head_offset, header.tail_offset) else {
            continue;
        };
        let first = data
            .segment_index_at_object_offset(head)
            .ok_or("Prop grind spline has an invalid first segment")?;
        let last = data
            .segment_index_at_object_offset(tail)
            .ok_or("Prop grind spline has an invalid last segment")?;
        let owner = next_handle.checked_add(1).ok_or("Grind header ID exhausted")?;
        *next_handle = owner;
        for (local, segment) in data
            .segments
            .get(first..=last)
            .ok_or("Prop grind spline segments out of range")?
            .iter()
            .enumerate()
        {
            let basis = &segment.basis_matrix;
            let mut start = rotate(&basis[12..16]);
            for lane in 0..3 {
                start[lane] += transform[12 + lane];
            }
            let (minimum, maximum) = crate::world::transform_collision_aabb(
                std::array::from_fn(|i| segment.bounding_box_min[i]),
                std::array::from_fn(|i| segment.bounding_box_max[i]),
                *transform,
            );
            segments.push(HostGrindSegment {
                coefficients: [rotate(&basis[0..4]), rotate(&basis[4..8]), rotate(&basis[8..12]), start],
                minimum,
                maximum,
                owner,
                spline_guids: [header.guid, header.local_guid],
                segment_index: u32::try_from(local).map_err(|error| error.to_string())?,
            });
        }
    }
    Ok((!segments.is_empty()).then_some(HostGrindAsset { segments }))
}

fn convert_source(
    source: &GrindSplineSource,
    next_handle: &mut u64,
) -> Result<Vec<HostGrindSegment>, String> {
    let mut owners = std::collections::BTreeMap::new();
    let mut segments = Vec::with_capacity(source.data().segments.len());
    for (index, segment) in source.data().segments.iter().enumerate() {
        let offset = segment
            .spline_offset
            .ok_or("Host grind segment lacks its native header")?;
        let header = source
            .spline_header(offset)
            .ok_or("Host grind segment has an invalid header")?;
        let first_index = source
            .data()
            .segment_index_at_object_offset(
                header
                    .head_offset
                    .ok_or("Host grind header lacks its first segment")?,
            )
            .ok_or("Host grind header has an invalid first segment")?;
        let local_index = index
            .checked_sub(first_index)
            .ok_or("Host grind segment precedes its header")?;
        let owner = match owners.entry(offset) {
            std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let id = next_handle
                    .checked_add(1)
                    .ok_or("Grind header ID exhausted")?;
                *next_handle = id;
                entry.insert(id);
                id
            }
        };
        segments.push(HostGrindSegment {
            coefficients: std::array::from_fn(|c| {
                std::array::from_fn(|r| segment.basis_matrix[c * 4 + r])
            }),
            minimum: std::array::from_fn(|i| segment.bounding_box_min[i]),
            maximum: std::array::from_fn(|i| segment.bounding_box_max[i]),
            owner,
            spline_guids: [header.guid, header.local_guid],
            segment_index: u32::try_from(local_index).map_err(|error| error.to_string())?,
        });
    }
    Ok(segments)
}

// The pre-cache adapter is retained only as an independent real-data oracle.
#[cfg(test)]
#[derive(Default)]
struct ReferenceGrinds {
    handles: Vec<(GrindSplineSource, u32, u64)>,
    next_handle: u64,
}
#[cfg(test)]
impl ReferenceGrinds {
    fn handle(&mut self, source: &GrindSplineSource, offset: u32) -> Result<u64, String> {
        if let Some((_, _, id)) = self
            .handles
            .iter()
            .find(|(old, at, _)| *at == offset && source.is_same_source(old))
        {
            return Ok(*id);
        }
        let id = self
            .next_handle
            .checked_add(1)
            .ok_or("Grind header ID exhausted")?;
        self.next_handle = id;
        self.handles.push((source.clone(), offset, id));
        Ok(id)
    }
    fn assets(&mut self, sources: &[GrindSplineSource]) -> Result<Vec<HostGrindAsset>, String> {
        let mut assets = Vec::new();
        for source in sources {
            let mut segments = Vec::new();
            for (index, segment) in source.data().segments.iter().enumerate() {
                let offset = segment
                    .spline_offset
                    .ok_or("Host grind segment lacks its native header")?;
                let header = source
                    .spline_header(offset)
                    .ok_or("Host grind segment has an invalid header")?;
                let first = source
                    .data()
                    .segment_at_object_offset(
                        header
                            .head_offset
                            .ok_or("Host grind header lacks its first segment")?,
                    )
                    .ok_or("Host grind header has an invalid first segment")?;
                let first_index = source
                    .data()
                    .segments
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, first))
                    .expect("segment belongs to the source");
                let local_index = index
                    .checked_sub(first_index)
                    .ok_or("Host grind segment precedes its header")?;
                segments.push(HostGrindSegment {
                    coefficients: std::array::from_fn(|c| {
                        std::array::from_fn(|r| segment.basis_matrix[c * 4 + r])
                    }),
                    minimum: std::array::from_fn(|i| segment.bounding_box_min[i]),
                    maximum: std::array::from_fn(|i| segment.bounding_box_max[i]),
                    owner: self.handle(source, offset)?,
                    spline_guids: [header.guid, header.local_guid],
                    segment_index: u32::try_from(local_index).map_err(|e| e.to_string())?,
                });
            }
            assets.push(HostGrindAsset { segments });
        }
        self.handles
            .retain(|(old, _, _)| sources.iter().any(|source| source.is_same_source(old)));
        Ok(assets)
    }
}

#[cfg(test)]
fn audit_cached_sources(sources: &[GrindSplineSource]) -> Result<(), String> {
    let mut cached = HostGrinds::default();
    let mut reference = ReferenceGrinds::default();
    let mut reversed = sources.to_vec();
    reversed.reverse();
    let mut duplicated = sources.to_vec();
    duplicated.extend_from_slice(sources);
    let sequences = [
        sources.to_vec(),
        reversed,
        sources.iter().skip(1).cloned().collect(),
        sources.to_vec(),
        duplicated,
        Vec::new(),
        sources.to_vec(),
    ];
    for (step, current) in sequences.iter().enumerate() {
        let start = std::time::Instant::now();
        let actual = cached.assets(current)?;
        let cached_ms = start.elapsed().as_secs_f64() * 1000.;
        let start = std::time::Instant::now();
        let expected = reference.assets(current)?;
        let reference_ms = start.elapsed().as_secs_f64() * 1000.;
        assert_eq!(
            actual.len(),
            expected.len(),
            "grind source count at step {step}"
        );
        for (asset, oracle) in actual.iter().zip(&expected) {
            assert_eq!(asset.segments.len(), oracle.segments.len());
            for (segment, oracle) in asset.segments.iter().zip(&oracle.segments) {
                assert_eq!(
                    segment.coefficients.map(|v| v.map(f32::to_bits)),
                    oracle.coefficients.map(|v| v.map(f32::to_bits))
                );
                assert_eq!(
                    segment.minimum.map(f32::to_bits),
                    oracle.minimum.map(f32::to_bits)
                );
                assert_eq!(
                    segment.maximum.map(f32::to_bits),
                    oracle.maximum.map(f32::to_bits)
                );
                assert_eq!(
                    (segment.owner, segment.spline_guids, segment.segment_index),
                    (oracle.owner, oracle.spline_guids, oracle.segment_index)
                );
            }
        }
        assert_eq!(cached.next_handle, reference.next_handle);
        crate::diagnostics::test_log::event(
            "grind_cache_audit",
            format_args!(
                "step={step} sources={} segments={} cached_ms={cached_ms:.3} reference_ms={reference_ms:.3} bitwise_equal=true",
                current.len(),
                actual
                    .iter()
                    .map(|asset| asset.segments.len())
                    .sum::<usize>()
            ),
        );
    }
    Ok(())
}
