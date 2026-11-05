//! Retained `GrindData` asset lifecycle,82cfb640/82cfd190/82cfb8b8.
//! Arena-backed sources feed the independently reviewed native octree.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained query metadata awaits grind investigation and live frame scheduling"
    )
)]
use crate::grind_data_octree::{GrindDataOctree, GrindOctreeError, OctreeBounds};
use crate::{
    simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationNormalOperation},
    simulation_static_arena_plan::SimulationStaticOperation,
};
use skate2_formats::pegasus::{
    PegasusError, SPLINE_DATA_TYPE_ID, SplineData, SplineSegment, TABLE_OF_CONTENTS_TYPE_ID,
    TableOfContents,
};
use std::{fmt, sync::Arc};

/// VMX exceptional min/max and dot-reduction rounding remain explicit numeric
/// boundaries. Bounds union executes all four lanes; query extent uses dot3.
pub(crate) trait GrindDataNumeric {
    type Error;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error>;
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error>;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error>;
}

/// Existing selected host VMX policy: ordered compare selects the right operand
/// on ties/unordered; dot3 uses rounded products and left-associated sums.
/// This is an explicit numeric boundary, not exceptional-value Xenon parity.
pub(crate) struct HostGrindDataNumeric;
impl GrindDataNumeric for HostGrindDataNumeric {
    type Error = std::convert::Infallible;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(std::array::from_fn(|lane| {
            if left[lane] < right[lane] {
                left[lane]
            } else {
                right[lane]
            }
        }))
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(std::array::from_fn(|lane| {
            if left[lane] > right[lane] {
                left[lane]
            } else {
                right[lane]
            }
        }))
    }
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(crate::reckoning_numeric::host_dot3(left, right))
    }
}

#[derive(Debug)]
pub(crate) enum GrindDataError<N> {
    Uninitialized,
    AlreadyInitialized,
    Source(GrindDataSourceError),
    Octree(GrindOctreeError),
    Numeric(N),
    SegmentCount,
}
impl<N: fmt::Debug> fmt::Display for GrindDataError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninitialized => f.write_str("GrindData Init required"),
            Self::AlreadyInitialized => f.write_str("GrindData bootstrap already initialized"),
            Self::Source(e) => e.fmt(f),
            Self::Octree(e) => write!(f, "grind octree: {e:?}"),
            Self::Numeric(e) => write!(f, "grind numeric: {e:?}"),
            Self::SegmentCount => f.write_str("grind segment count exceeds native range"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum GrindDataSourceError {
    WrongOperation,
    MissingDictionary,
    MissingBytes,
    WrongType,
    MissingToc,
    WrongTocTarget,
    Parse(PegasusError),
    EmptySegments,
}
impl fmt::Display for GrindDataSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "grind source: {error}"),
            other => write!(f, "grind source: {other:?}"),
        }
    }
}

/// Pointer-equivalent immutable kind6 source. No diagnostic primitive is read.
#[derive(Debug, Clone)]
pub(crate) struct GrindSplineSource {
    owner: GrindSplineOwner,
    operation_index: Option<usize>,
    dictionary_index: u32,
    asset_id: u64,
    data: Arc<SplineData>,
}
#[derive(Debug, Clone)]
enum GrindSplineOwner {
    Normal(Arc<SimulationNormalArenaPlan>),
    /// The actual collision arena retained by the CharacterAssetManager's
    /// ambient recipe load; no synthetic normal-arena operation is invented.
    Character(Arc<crate::world::CollisionArena>),
}
impl GrindSplineSource {
    pub(crate) fn try_from_operation(
        owner: Arc<SimulationNormalArenaPlan>,
        operation_index: usize,
    ) -> Result<Self, GrindDataSourceError> {
        use GrindDataSourceError as E;
        let Some(SimulationNormalOperation::Grind {
            static_operation_index,
        }) = owner.operations().get(operation_index)
        else {
            return Err(E::WrongOperation);
        };
        let Some(SimulationStaticOperation::Grind {
            source_toc_index,
            source_dictionary_index,
            ..
        }) = owner
            .static_plan()
            .operations()
            .get(*static_operation_index)
        else {
            return Err(E::WrongOperation);
        };
        let dictionary_index = *source_dictionary_index;
        let decoded = &owner.static_plan().asset().decoded;
        let arena = &decoded.arena;
        let entry = arena
            .dictionary
            .get(dictionary_index as usize)
            .ok_or(E::MissingDictionary)?;
        if entry.type_id != SPLINE_DATA_TYPE_ID {
            return Err(E::WrongType);
        }
        let bytes = arena
            .object_bytes(&decoded.resources.resources, entry)
            .ok_or(E::MissingBytes)?;
        let toc_entry = arena
            .dictionary
            .iter()
            .find(|entry| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
            .ok_or(E::MissingToc)?;
        let toc = TableOfContents::parse(
            arena
                .object_bytes(&decoded.resources.resources, toc_entry)
                .ok_or(E::MissingBytes)?,
        )
        .map_err(E::Parse)?;
        let toc = toc
            .entries
            .get(*source_toc_index as usize)
            .ok_or(E::MissingToc)?;
        if toc.object_type != SPLINE_DATA_TYPE_ID
            || toc.object.section_index() != 0
            || toc.object.entry_index() != dictionary_index
        {
            return Err(E::WrongTocTarget);
        }
        let asset_id = toc.guid;
        let data = Arc::new(SplineData::parse(bytes).map_err(E::Parse)?);
        Ok(Self {
            owner: GrindSplineOwner::Normal(owner),
            operation_index: Some(operation_index),
            dictionary_index,
            asset_id,
            data,
        })
    }
    /// A DMO model's embedded grind spline (`DynamicObject::InitSplines`
    /// 0x82d2bf18): the bank arena's `tSplineData` object whose inline header
    /// the `tEmbeddedData` reference names. `operation_index` is the DMO
    /// instance operation that owns the reference.
    pub(crate) fn try_from_dmo_spline_data(
        owner: Arc<SimulationNormalArenaPlan>,
        operation_index: usize,
        dictionary_index: u32,
        asset_id: u64,
        bytes: &[u8],
    ) -> Result<Self, GrindDataSourceError> {
        let data = Arc::new(SplineData::parse(bytes).map_err(GrindDataSourceError::Parse)?);
        Ok(Self {
            owner: GrindSplineOwner::Normal(owner),
            operation_index: Some(operation_index),
            dictionary_index,
            asset_id,
            data,
        })
    }
    pub(crate) fn try_from_character_spline_data(
        owner: Arc<crate::world::CollisionArena>,
        dictionary_index: u32,
        asset_id: u64,
        bytes: &[u8],
    ) -> Result<Self, GrindDataSourceError> {
        Ok(Self {
            owner: GrindSplineOwner::Character(owner),
            operation_index: None,
            dictionary_index,
            asset_id,
            data: Arc::new(SplineData::parse(bytes).map_err(GrindDataSourceError::Parse)?),
        })
    }
    pub(crate) fn data(&self) -> &SplineData {
        &self.data
    }
    pub(crate) fn asset_id(&self) -> u64 {
        self.asset_id
    }
    pub(crate) fn operation_index(&self) -> Option<usize> {
        self.operation_index
    }
    pub(crate) fn dictionary_index(&self) -> u32 {
        self.dictionary_index
    }
    pub(crate) fn is_same_source(&self, other: &Self) -> bool {
        let same_owner = match (&self.owner, &other.owner) {
            (GrindSplineOwner::Normal(a), GrindSplineOwner::Normal(b)) => Arc::ptr_eq(a, b),
            (GrindSplineOwner::Character(a), GrindSplineOwner::Character(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        same_owner && self.dictionary_index == other.dictionary_index
    }
    /// The inline `SplineHeader` a segment's `mpSplineHeader` points at.
    pub(crate) fn spline_header(
        &self,
        offset: u32,
    ) -> Option<&skate2_formats::pegasus::SplineHeader> {
        self.data.spline_at_object_offset(offset)
    }
}

/// The native constructor writes only bit31 of its flags word. Lower bits are
/// deliberately absent; copying into a query destination preserves those bits.
#[derive(Debug, Clone)]
pub(crate) struct GrindDataPrimitive {
    pub(crate) start: [f32; 4],
    pub(crate) end: [f32; 4],
    pub(crate) source: GrindSplineSource,
    pub(crate) segment_index: u32,
    pub(crate) spline_header_offset: Option<u32>,
    pub(crate) vertical: bool,
}
impl GrindDataPrimitive {
    /// 82cfb538, `SplineEval_One`82e49908 corrected raw VMX operands.
    fn from_segment(
        source: &GrindSplineSource,
        segment_index: u32,
        segment: &SplineSegment,
    ) -> Self {
        let start = std::array::from_fn(|lane| segment.basis_matrix[12 + lane]);
        let mut end = std::array::from_fn(|lane| {
            let ab = segment.basis_matrix[lane] + segment.basis_matrix[4 + lane];
            let cd = segment.basis_matrix[8 + lane] + segment.basis_matrix[12 + lane];
            ab + cd
        });
        // Native permute<0,5,0,0> followed by Z insertion publishes W=X.
        end[3] = end[0];
        let epsilon = f32::from_bits(0x3780_0000);
        let vertical = !([0, 2]
            .into_iter()
            .any(|lane| (end[lane] - start[lane]).abs() > epsilon));
        Self {
            start,
            end,
            source: source.clone(),
            segment_index,
            spline_header_offset: segment.spline_offset,
            vertical,
        }
    }
}

struct GrindAssetRecord {
    source: GrindSplineSource,
    bounds: OctreeBounds,
    octree: GrindDataOctree,
    primitives: Vec<GrindDataPrimitive>,
}
impl GrindAssetRecord {
    fn try_new<N: GrindDataNumeric>(
        source: GrindSplineSource,
        numeric: &mut N,
    ) -> Result<Self, GrindDataError<N::Error>> {
        use GrindDataError as E;
        let first = source
            .data
            .segments
            .first()
            .ok_or(E::Source(GrindDataSourceError::EmptySegments))?;
        let mut bounds = OctreeBounds {
            minimum: first.bounding_box_min,
            maximum: first.bounding_box_max,
        };
        for segment in source.data.segments.iter().skip(1) {
            let minimum = numeric
                .minimum(bounds.minimum, segment.bounding_box_min)
                .map_err(E::Numeric)?;
            let maximum = numeric
                .maximum(bounds.maximum, segment.bounding_box_max)
                .map_err(E::Numeric)?;
            bounds.minimum = minimum;
            bounds.maximum = maximum;
        }
        let expansion = [1.0, 1.0, 1.0, 0.0];
        bounds.maximum = std::array::from_fn(|lane| bounds.maximum[lane] + expansion[lane]);
        bounds.minimum = std::array::from_fn(|lane| bounds.minimum[lane] - expansion[lane]);
        let count = u32::try_from(source.data.segments.len()).map_err(|_| E::SegmentCount)?;
        let mut octree = GrindDataOctree::try_new(count, bounds).map_err(E::Octree)?;
        let mut primitives = Vec::with_capacity(source.data.segments.len());
        for (index, segment) in source.data.segments.iter().enumerate() {
            octree
                .try_insert(
                    index,
                    OctreeBounds {
                        minimum: segment.bounding_box_min,
                        maximum: segment.bounding_box_max,
                    },
                )
                .map_err(E::Octree)?;
            primitives.push(GrindDataPrimitive::from_segment(
                &source,
                u32::try_from(index).map_err(|_| E::SegmentCount)?,
                segment,
            ));
        }
        Ok(Self {
            source,
            bounds,
            octree,
            primitives,
        })
    }
}

/// Caller-owned fixed result storage; clearing count does not erase old slots.
/// Source copies endpoints/header and ONLY flag bit31; no lower flag bits are
/// fabricated by this representation.
pub(crate) struct GrindDataQueryResult {
    slots: [Option<GrindDataPrimitive>; 40],
    flags: [(u32, u32); 40],
    count: usize,
}
impl GrindDataQueryResult {
    pub(crate) fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            flags: [(0, 0); 40],
            count: 0,
        }
    }
    /// Native count-only clear preserves prior slot contents and known flag bits.
    pub(crate) fn clear_count(&mut self) {
        self.count = 0;
    }
    pub(crate) fn count(&self) -> usize {
        self.count
    }
    pub(crate) fn flag_bits(&self, index: usize) -> Option<(u32, u32)> {
        self.flags.get(index).copied()
    }
    pub(crate) fn results(&self) -> impl Iterator<Item = &GrindDataPrimitive> {
        self.slots[..self.count]
            .iter()
            .map(|entry| entry.as_ref().expect("published grind result"))
    }
    pub(crate) fn try_append(&mut self, primitive: GrindDataPrimitive) -> bool {
        if self.count == 40 {
            return false;
        }
        let (value, known) = &mut self.flags[self.count];
        *value = (*value & 0x7fff_ffff) | (u32::from(primitive.vertical) << 31);
        *known |= 0x8000_0000;
        self.slots[self.count] = Some(primitive);
        self.count += 1;
        true
    }
}

/// The actual installed dynamic provider must be passed as Some. None expresses
/// the actual null native provider; it must not substitute for an absent adapter.
pub(crate) trait DynamicGrindSplineProvider {
    type Error;
    fn query(
        &mut self,
        bounds: OctreeBounds,
        result: &mut GrindDataQueryResult,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum GrindDataQueryError<N, D> {
    Static(GrindDataError<N>),
    Dynamic(D),
}

/// Process-global `GrindData` implementation ownership, explicitly initialized
/// by the caller corresponding to World.Initialize827b12a4.
pub(crate) struct GrindDataRuntime {
    records: Option<Vec<GrindAssetRecord>>,
}
impl GrindDataRuntime {
    pub(crate) fn new() -> Self {
        Self { records: None }
    }
    /// Selected successful first Init82cfcfb8 allocation. Native repeated Init
    /// overwrites mpImpl without destroying its previous allocation; this
    /// explicit host admission guard does not model that abandoned lifetime.
    /// Allocation-failure publication of null is outside this successful path.
    pub(crate) fn try_initialize(
        &mut self,
    ) -> Result<(), GrindDataError<std::convert::Infallible>> {
        if self.records.is_some() {
            return Err(GrindDataError::AlreadyInitialized);
        }
        self.records = Some(Vec::new());
        Ok(())
    }
    pub(crate) fn is_initialized(&self) -> bool {
        self.records.is_some()
    }
    pub(crate) fn try_add<N: GrindDataNumeric>(
        &mut self,
        source: GrindSplineSource,
        numeric: &mut N,
    ) -> Result<(), GrindDataError<N::Error>> {
        let records = self.records.as_mut().ok_or(GrindDataError::Uninitialized)?;
        let record = GrindAssetRecord::try_new(source, numeric)?;
        records.push(record); //82cfd1fc..204, after complete construction; duplicates remain.
        Ok(())
    }
    /// Raw82cfb8e8 LD + b8ec CMPLD proves full64 GUID equality.
    pub(crate) fn try_remove(
        &mut self,
        asset_id: u64,
    ) -> Result<(), GrindDataError<std::convert::Infallible>> {
        let records = self.records.as_mut().ok_or(GrindDataError::Uninitialized)?;
        let mut index = 0;
        while index < records.len() {
            if records[index].source.asset_id == asset_id {
                records.swap_remove(index);
            } else {
                index += 1;
            }
        }
        Ok(())
    }
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn skate3_sources(&self) -> Result<Vec<GrindSplineSource>,String> {
        Ok(self.records.as_ref().ok_or("Host GrindData is not initialized")?.iter()
            .map(|record|record.source.clone()).collect())
    }
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn matches_skate3_sources(&self, sources: &[GrindSplineSource]) -> Result<bool,String> {
        let records=self.records.as_ref().ok_or("Host GrindData is not initialized")?;
        Ok(records.len()==sources.len() && records.iter().zip(sources)
            .all(|(record,source)|record.source.is_same_source(source)))
    }
    pub(crate) fn try_box_test<N: GrindDataNumeric, D: DynamicGrindSplineProvider>(
        &self,
        bounds: OctreeBounds,
        result: &mut GrindDataQueryResult,
        ignore_vertical: bool,
        numeric: &mut N,
        dynamic: Option<&mut D>,
    ) -> Result<bool, GrindDataQueryError<N::Error, D::Error>> {
        use GrindDataQueryError as E;
        let records = self
            .records
            .as_ref()
            .ok_or(E::Static(GrindDataError::Uninitialized))?;
        result.count = 0; //9848, before extent gate/numeric service.
        let difference = std::array::from_fn(|lane| bounds.minimum[lane] - bounds.maximum[lane]);
        let squared = numeric
            .dot3(difference, difference)
            .map_err(|e| E::Static(GrindDataError::Numeric(e)))?;
        if squared.partial_cmp(&f32::from_bits(0x3780_0000)) != Some(std::cmp::Ordering::Greater) {
            return Ok(false);
        }
        let mut accepted = 0;
        for record in records {
            if !overlap(bounds, record.bounds) {
                continue;
            }
            let mut query = record
                .octree
                .try_query(bounds)
                .map_err(|e| E::Static(GrindDataError::Octree(e)))?;
            while let Some(index) = query
                .try_next()
                .map_err(|e| E::Static(GrindDataError::Octree(e)))?
            {
                let primitive = &record.primitives[index];
                if ignore_vertical && primitive.vertical {
                    continue;
                }
                result.try_append(primitive.clone());
                accepted += 1;
                if accepted == 40 {
                    break;
                }
            }
            if accepted == 40 {
                break;
            }
        }
        // Source999c..99cc calls the installed provider even when static hit40.
        if let Some(provider) = dynamic {
            provider.query(bounds, result).map_err(E::Dynamic)?;
        }
        Ok(result.count != 0)
    }
}

fn overlap(left: OctreeBounds, right: OctreeBounds) -> bool {
    !(0..3).any(|lane| {
        left.minimum[lane] > right.maximum[lane] || right.minimum[lane] > left.maximum[lane]
    })
}

#[cfg(test)]
#[path = "grind_data_runtime_tests.rs"]
pub(crate) mod tests;
