//! `LivingWorld::RecipeMan::Init` state and call ordering.
//!
//! Mapping construction is shared by the retained recipe lifecycle in `runtime`.

#[path = "living_world_recipe_attributes.rs"]
pub(crate) mod attributes;
#[path = "living_world_recipe_runtime.rs"]
pub(crate) mod runtime;

use std::{error::Error, fmt};

/// Bucket count selected by the native prime rehash policy for hint 65.
pub const RECIPE_MAPPING_BUCKET_COUNT: usize = 61;
/// Validated inline group domain of the fixed hash map.
pub const RECIPE_MAPPING_GROUP_CAPACITY: usize = 64;
/// Inline model entries available in each mapping group.
pub const RECIPE_MAPPING_GROUP_ENTRY_CAPACITY: usize = 4;
/// Inline capacity of each of the three model-loading queues.
pub const RECIPE_LOADING_QUEUE_CAPACITY: usize = 64;
pub const LIVING_WORLD_MODELS_CLASS_ID: u64 = 0x4e74_0a66_70f2_41a7;
pub const LIVING_WORLD_CONFIG_CLASS_ID: u64 = 0xf97b_9086_caa5_0c53;
pub const LIVING_WORLD_FORCED_CONFIG_COLLECTION_ID: u64 = 0xe587_4fc0_4c11_7535;
pub const LOAD_MODEL_ATTRIBUTE_ID: u64 = 0xa75a_f7f3_cfab_e6e1;
pub const FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID: u64 = 0x8be7_0b1a_756d_9035;
pub const FORCED_VEHICLE_MODEL_ATTRIBUTE_ID: u64 = 0x7b5f_7a6c_78f3_251d;

const DEFAULT_RECIPE_MAPPING_SEEDS: [(u64, bool); 4] = [
    (0x473c_3703_9267_1785, true),
    (0xc79a_1101_adab_19db, false),
    (0x8be1_d151_f9da_2d29, false),
    (0x21d3_cdd9_3eae_05e5, false),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecipeMappingEntry {
    pub model_id: u64,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecipeMappingGroup {
    group_id: u64,
    entries: Vec<RecipeMappingEntry>,
}

/// Insertion-owned representation of `RecipeMan`'s fixed hash map.
///
/// Nodes remain in unique-group insertion order. Iteration derives the native
/// 61-bucket, bucket-head-chain order without relying on a host hash map.
/// Insertion is evidenced by the specialized EASTL helper at
/// `0x82727010..0x8272713c`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecipeMappingTable {
    groups: Vec<RecipeMappingGroup>,
}

impl RecipeMappingTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an entry to an existing group or inserts a new bucket-head node.
    ///
    /// # Errors
    ///
    /// Returns a typed fixed-domain or host allocation failure before changing
    /// the requested group.
    pub fn append(
        &mut self,
        group_id: u64,
        entry: RecipeMappingEntry,
    ) -> Result<(), RecipeMappingError> {
        if let Some(group_index) = self
            .groups
            .iter()
            .position(|group| group.group_id == group_id)
        {
            let group = &mut self.groups[group_index];
            if group.entries.len() == RECIPE_MAPPING_GROUP_ENTRY_CAPACITY {
                return Err(RecipeMappingError::GroupEntryCapacity { group_id });
            }
            group
                .entries
                .try_reserve(1)
                .map_err(|_| RecipeMappingError::Allocation)?;
            group.entries.push(entry);
            return Ok(());
        }
        if self.groups.len() == RECIPE_MAPPING_GROUP_CAPACITY {
            return Err(RecipeMappingError::GroupCapacity);
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| RecipeMappingError::Allocation)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(RECIPE_MAPPING_GROUP_ENTRY_CAPACITY)
            .map_err(|_| RecipeMappingError::Allocation)?;
        entries.push(entry);
        self.groups.push(RecipeMappingGroup { group_id, entries });
        Ok(())
    }

    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.groups.iter().map(|group| group.entries.len()).sum()
    }

    /// Visits every model entry together with its group in native construction
    /// order. This is intentionally distinct from hash-table iteration order.
    pub fn insertion_order_entries(&self) -> impl Iterator<Item = (u64, RecipeMappingEntry)> + '_ {
        self.groups.iter().flat_map(|group| {
            group
                .entries
                .iter()
                .copied()
                .map(move |entry| (group.group_id, entry))
        })
    }

    fn native_iteration(&self) -> impl Iterator<Item = &RecipeMappingGroup> {
        (0..RECIPE_MAPPING_BUCKET_COUNT).flat_map(move |bucket| {
            self.groups.iter().rev().filter(move |group| {
                let low_word = group.group_id & u64::from(u32::MAX);
                usize::try_from(low_word % RECIPE_MAPPING_BUCKET_COUNT as u64)
                    .expect("bucket fits usize")
                    == bucket
            })
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeLoadingQueueKind {
    Required,
    Initial,
    Secondary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecipeMappingError {
    GroupCapacity,
    GroupEntryCapacity { group_id: u64 },
    EmptyGroup { group_id: u64 },
    QueueCapacity { queue: RecipeLoadingQueueKind },
    Allocation,
}

impl fmt::Display for RecipeMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GroupCapacity => formatter.write_str("recipe mapping group capacity exhausted"),
            Self::GroupEntryCapacity { group_id } => write!(
                formatter,
                "recipe mapping group {group_id:#018x} entry capacity exhausted"
            ),
            Self::EmptyGroup { group_id } => {
                write!(formatter, "recipe mapping group {group_id:#018x} is empty")
            }
            Self::QueueCapacity { queue } => {
                write!(
                    formatter,
                    "recipe {queue:?} loading queue capacity exhausted"
                )
            }
            Self::Allocation => formatter.write_str("recipe mapping host allocation failed"),
        }
    }
}

impl Error for RecipeMappingError {}

/// Attribute-database effects used by `RecipeMan::InitRecipeMappings`.
///
/// `use_forced_recipes` is intentionally sampled once for seed construction
/// and once again for every seed. The executable does not snapshot the cVar.
pub trait RecipeMappingProvider {
    type Error;

    fn use_forced_recipes(&mut self) -> bool;

    /// Returns the referenced collection key at `RefSpec + 8`, or `None` for
    /// the native zero-filled missing/default value.
    ///
    /// # Errors
    ///
    /// Returns a checked malformed-database or native-precondition failure.
    fn forced_model_collection(&mut self, attribute_id: u64) -> Result<Option<u64>, Self::Error>;

    /// Returns filtered leaf descendants in ascending unsigned-`u64` order.
    ///
    /// # Errors
    ///
    /// Returns a checked malformed hierarchy or missing class/seed failure.
    fn leaf_descendants_sorted(
        &mut self,
        class_id: u64,
        seed_id: u64,
    ) -> Result<Vec<u64>, Self::Error>;

    /// Resolves inherited `Load_Model`; a missing value is native false.
    ///
    /// # Errors
    ///
    /// Returns a checked malformed-database or missing collection failure.
    fn load_model_nonzero(
        &mut self,
        class_id: u64,
        collection_id: u64,
    ) -> Result<bool, Self::Error>;

    /// # Errors
    ///
    /// Returns a checked malformed hierarchy or missing collection failure.
    fn collection_parent(&mut self, class_id: u64, collection_id: u64) -> Result<u64, Self::Error>;
}

#[derive(Debug)]
pub enum RecipeMappingBuildError<E> {
    Provider(E),
    Mapping(RecipeMappingError),
}

impl<E: fmt::Display> fmt::Display for RecipeMappingBuildError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(source) => write!(formatter, "recipe mapping provider failed: {source}"),
            Self::Mapping(source) => source.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for RecipeMappingBuildError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(source) => Some(source),
            Self::Mapping(source) => Some(source),
        }
    }
}

/// Reconstructs `RecipeMan::InitRecipeMappings`
/// (`0x82728488..0x827289f4`) into an existing constructor-owned table.
///
/// The table is deliberately not cleared at entry. On a checked provider or
/// fixed-capacity failure, all earlier native-order appends remain visible and
/// the outer initialization boundary must not mark the manager initialized.
///
/// # Errors
///
/// Returns the first provider failure or checked fixed-container failure at
/// the same candidate prefix.
pub fn build_recipe_mappings<P: RecipeMappingProvider>(
    mappings: &mut RecipeMappingTable,
    provider: &mut P,
) -> Result<(), RecipeMappingBuildError<P::Error>> {
    let seeds = if provider.use_forced_recipes() {
        [
            (
                provider
                    .forced_model_collection(FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID)
                    .map_err(RecipeMappingBuildError::Provider)?
                    .unwrap_or(0),
                false,
            ),
            (
                provider
                    .forced_model_collection(FORCED_VEHICLE_MODEL_ATTRIBUTE_ID)
                    .map_err(RecipeMappingBuildError::Provider)?
                    .unwrap_or(0),
                false,
            ),
        ]
        .to_vec()
    } else {
        DEFAULT_RECIPE_MAPPING_SEEDS.to_vec()
    };

    for (seed_id, required) in seeds {
        let candidates = if provider.use_forced_recipes() {
            vec![seed_id]
        } else {
            provider
                .leaf_descendants_sorted(LIVING_WORLD_MODELS_CLASS_ID, seed_id)
                .map_err(RecipeMappingBuildError::Provider)?
        };
        for model_id in candidates {
            if !provider
                .load_model_nonzero(LIVING_WORLD_MODELS_CLASS_ID, model_id)
                .map_err(RecipeMappingBuildError::Provider)?
            {
                continue;
            }
            let mut group_id = model_id;
            if model_id != seed_id {
                let parent_id = provider
                    .collection_parent(LIVING_WORLD_MODELS_CLASS_ID, model_id)
                    .map_err(RecipeMappingBuildError::Provider)?;
                if parent_id != seed_id {
                    group_id = parent_id;
                }
            }
            mappings
                .append(group_id, RecipeMappingEntry { model_id, required })
                .map_err(RecipeMappingBuildError::Mapping)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecipeLoadingQueues {
    required: Vec<u64>,
    initial: Vec<u64>,
    secondary: Vec<u64>,
}

impl RecipeLoadingQueues {
    #[must_use]
    pub fn required(&self) -> &[u64] {
        &self.required
    }

    #[must_use]
    pub fn initial(&self) -> &[u64] {
        &self.initial
    }

    #[must_use]
    pub fn secondary(&self) -> &[u64] {
        &self.secondary
    }
}

/// Reconstructs `RecipeMan::InitLoadingQueues`
/// (`0x8271ffb0..0x827202d4`) for a completed mapping table.
///
/// Both passes use native bucket/chain order. The first collects every required
/// entry. The second may take only group entry zero for the Initial queue, then
/// appends remaining nonrequired entries to Secondary. Secondary is truncated
/// by repeatedly removing its back element.
///
/// # Errors
///
/// Returns a typed fixed-queue or host allocation failure. Mappings built via
/// [`RecipeMappingTable::append`] cannot contain the native-invalid empty group.
pub fn build_recipe_loading_queues(
    mappings: &RecipeMappingTable,
    maximum_initial_loads: usize,
    maximum_secondary_loads: usize,
) -> Result<RecipeLoadingQueues, RecipeMappingError> {
    let mut queues = RecipeLoadingQueues::default();
    reserve_queue(&mut queues.required)?;
    reserve_queue(&mut queues.initial)?;
    reserve_queue(&mut queues.secondary)?;

    for group in mappings.native_iteration() {
        for entry in &group.entries {
            if entry.required {
                push_queue(
                    &mut queues.required,
                    entry.model_id,
                    RecipeLoadingQueueKind::Required,
                )?;
            }
        }
    }

    let initial_budget = maximum_initial_loads.saturating_sub(queues.required.len());
    for group in mappings.native_iteration() {
        let mut first_remaining = 0;
        if queues.initial.len() < initial_budget {
            let first = group
                .entries
                .first()
                .ok_or(RecipeMappingError::EmptyGroup {
                    group_id: group.group_id,
                })?;
            if !first.required {
                push_queue(
                    &mut queues.initial,
                    first.model_id,
                    RecipeLoadingQueueKind::Initial,
                )?;
            }
            first_remaining = 1;
        }
        for entry in &group.entries[first_remaining..] {
            if !entry.required {
                push_queue(
                    &mut queues.secondary,
                    entry.model_id,
                    RecipeLoadingQueueKind::Secondary,
                )?;
            }
        }
    }
    queues.secondary.truncate(maximum_secondary_loads);
    Ok(queues)
}

fn reserve_queue(queue: &mut Vec<u64>) -> Result<(), RecipeMappingError> {
    queue
        .try_reserve_exact(RECIPE_LOADING_QUEUE_CAPACITY)
        .map_err(|_| RecipeMappingError::Allocation)
}

fn push_queue(
    queue: &mut Vec<u64>,
    model_id: u64,
    queue_kind: RecipeLoadingQueueKind,
) -> Result<(), RecipeMappingError> {
    if queue.len() == RECIPE_LOADING_QUEUE_CAPACITY {
        return Err(RecipeMappingError::QueueCapacity { queue: queue_kind });
    }
    queue.push(model_id);
    Ok(())
}

/// The two fields directly sampled or written by `RecipeMan::Init`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecipeManagerInitializationState {
    initialized_raw: u8,
    frame_delay_raw: u32,
}

impl RecipeManagerInitializationState {
    #[must_use]
    pub const fn from_raw(initialized_raw: u8, frame_delay_raw: u32) -> Self {
        Self {
            initialized_raw,
            frame_delay_raw,
        }
    }

    #[must_use]
    pub const fn initialized_raw(self) -> u8 {
        self.initialized_raw
    }

    #[must_use]
    pub const fn frame_delay_raw(self) -> u32 {
        self.frame_delay_raw
    }
}

/// Observable result of one native `void` initialization call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeManagerInitialization {
    AlreadyInitialized,
    InitializedNow,
}

/// Effects called by `LivingWorld::RecipeMan::Init` (`0x827289f8`).
pub trait RecipeManagerInitializationRuntime {
    type Error;

    /// Calls `RecipeMan::InitRecipeMappings` (`0x82728488`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while building mappings.
    fn initialize_recipe_mappings(&mut self) -> Result<(), Self::Error>;

    /// Calls `RecipeMan::InitLoadingQueues` (`0x8271ffb0`).
    ///
    /// # Errors
    ///
    /// Returns a checked host-runtime failure while building loading queues.
    fn initialize_loading_queues(&mut self) -> Result<(), Self::Error>;
}

/// Reconstructs `LivingWorld::RecipeMan::Init`.
///
/// Any nonzero initialized byte returns immediately and preserves the raw byte
/// and frame-delay word. Raw zero performs mapping initialization, then queue
/// initialization, then stores initialized byte one followed by frame delay
/// zero. Checked host failures do not fabricate the two terminal stores or
/// roll back earlier collaborator effects.
///
/// # Errors
///
/// Returns the first checked mapping or queue initialization failure.
pub fn initialize_recipe_manager<R>(
    state: &mut RecipeManagerInitializationState,
    runtime: &mut R,
) -> Result<RecipeManagerInitialization, R::Error>
where
    R: RecipeManagerInitializationRuntime,
{
    if state.initialized_raw != 0 {
        return Ok(RecipeManagerInitialization::AlreadyInitialized);
    }

    runtime.initialize_recipe_mappings()?;
    runtime.initialize_loading_queues()?;
    state.initialized_raw = 1;
    state.frame_delay_raw = 0;
    Ok(RecipeManagerInitialization::InitializedNow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, convert::Infallible};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum MappingEvent {
        UseForced(bool),
        Forced(u64),
        Leaves(u64),
        Enabled(u64),
        Parent(u64),
    }

    #[derive(Default)]
    struct MappingRuntime {
        forced_samples: Vec<bool>,
        next_forced_sample: usize,
        forced_collections: BTreeMap<u64, Option<u64>>,
        leaves: BTreeMap<u64, Vec<u64>>,
        enabled: BTreeMap<u64, bool>,
        parents: BTreeMap<u64, u64>,
        events: Vec<MappingEvent>,
    }

    impl RecipeMappingProvider for MappingRuntime {
        type Error = Infallible;

        fn use_forced_recipes(&mut self) -> bool {
            let value = self.forced_samples[self.next_forced_sample];
            self.next_forced_sample += 1;
            self.events.push(MappingEvent::UseForced(value));
            value
        }

        fn forced_model_collection(
            &mut self,
            attribute_id: u64,
        ) -> Result<Option<u64>, Self::Error> {
            self.events.push(MappingEvent::Forced(attribute_id));
            Ok(self
                .forced_collections
                .get(&attribute_id)
                .copied()
                .flatten())
        }

        fn leaf_descendants_sorted(
            &mut self,
            class_id: u64,
            seed_id: u64,
        ) -> Result<Vec<u64>, Self::Error> {
            assert_eq!(class_id, LIVING_WORLD_MODELS_CLASS_ID);
            self.events.push(MappingEvent::Leaves(seed_id));
            Ok(self.leaves.get(&seed_id).cloned().unwrap_or_default())
        }

        fn load_model_nonzero(
            &mut self,
            class_id: u64,
            collection_id: u64,
        ) -> Result<bool, Self::Error> {
            assert_eq!(class_id, LIVING_WORLD_MODELS_CLASS_ID);
            self.events.push(MappingEvent::Enabled(collection_id));
            Ok(self.enabled.get(&collection_id).copied().unwrap_or(false))
        }

        fn collection_parent(
            &mut self,
            class_id: u64,
            collection_id: u64,
        ) -> Result<u64, Self::Error> {
            assert_eq!(class_id, LIVING_WORLD_MODELS_CLASS_ID);
            self.events.push(MappingEvent::Parent(collection_id));
            Ok(self.parents[&collection_id])
        }
    }

    fn append_group(mappings: &mut RecipeMappingTable, group_id: u64, entries: &[(u64, bool)]) {
        for &(model_id, required) in entries {
            mappings
                .append(group_id, RecipeMappingEntry { model_id, required })
                .unwrap();
        }
    }

    #[allow(clippy::too_many_lines)] // Exact supplied-vault golden fixture.
    fn shipped_default_mapping() -> RecipeMappingTable {
        let mut mappings = RecipeMappingTable::new();
        append_group(
            &mut mappings,
            0xa8ae_0c85_0ebb_bbfd,
            &[
                (0x030a_b922_4e08_cbd1, true),
                (0x8960_aeb0_8f12_71cd, true),
                (0xf977_b957_a0e8_68cb, true),
            ],
        );
        append_group(
            &mut mappings,
            0x1562_d5e0_4bc3_9614,
            &[
                (0x041a_0b27_8eae_b09c, true),
                (0x1e58_8e6d_426d_1a2f, true),
                (0x5ef2_eddd_c75f_25de, true),
            ],
        );
        append_group(
            &mut mappings,
            0x916c_84f7_3149_9248,
            &[
                (0x29ea_db80_7f46_edf6, true),
                (0x7db7_14d7_e927_5d53, true),
                (0x8151_8f0e_dafe_70a6, true),
            ],
        );
        append_group(
            &mut mappings,
            0x7fb4_08b2_7e0c_2521,
            &[(0x56f3_cd83_30b9_78ee, true), (0x5bbf_cd8b_c95c_b2e7, true)],
        );
        append_group(
            &mut mappings,
            0xc281_eaea_ad42_219b,
            &[(0x58b6_0bae_7322_56bb, true), (0xcb2c_143b_9929_a70a, true)],
        );
        append_group(
            &mut mappings,
            0xbb5c_8edb_f201_ac9d,
            &[(0xb7cd_d46e_e0b8_e9b0, true)],
        );
        append_group(
            &mut mappings,
            0xbd79_86b3_d673_b134,
            &[(0xef73_f403_30bd_5d77, true)],
        );
        append_group(
            &mut mappings,
            0xdca9_faa9_2bda_1c8a,
            &[
                (0x05f6_62f7_109b_dede, false),
                (0x798c_b224_ed7f_bfee, false),
                (0xfe1f_4594_b6f9_18e2, false),
            ],
        );
        append_group(
            &mut mappings,
            0x62e6_9f57_ed03_12fc,
            &[
                (0x1594_5e1b_6db9_452d, false),
                (0x3457_c13f_e6d1_28f1, false),
                (0x9552_f722_9512_9362, false),
            ],
        );
        append_group(
            &mut mappings,
            0x153d_8034_460d_6051,
            &[
                (0x325e_baa6_9b25_8848, false),
                (0x9a4d_5e87_ade3_af28, false),
                (0xedeb_a27c_4163_bfae, false),
            ],
        );
        append_group(
            &mut mappings,
            0xaceb_008f_9d80_5483,
            &[
                (0x34a2_a830_f2ab_aee2, false),
                (0x5ac9_5e9c_35f6_5bed, false),
                (0xe602_f3b8_5b00_f793, false),
            ],
        );
        append_group(
            &mut mappings,
            0x13c6_2aae_f214_349d,
            &[
                (0x3ba9_21fc_ef1b_45fd, false),
                (0x445f_8033_83f2_6837, false),
                (0x6daf_1766_3d47_fd14, false),
            ],
        );
        append_group(
            &mut mappings,
            0xcf10_791d_c5c3_3c13,
            &[
                (0x47ad_17fd_06f5_3aba, false),
                (0xa2b2_41f6_ac35_b367, false),
                (0xc81c_3aac_7f34_f22d, false),
            ],
        );
        append_group(
            &mut mappings,
            0x1bf4_c3c1_72a6_f81a,
            &[
                (0x538d_c4b8_8de9_4552, false),
                (0x785d_7012_1bfa_192d, false),
                (0xf455_ac5a_7af6_9c18, false),
            ],
        );
        append_group(
            &mut mappings,
            0x8d1a_ab96_6e2b_650f,
            &[
                (0x5dbe_f74c_53c5_5e6e, false),
                (0xde2d_9c8a_d95e_dcb0, false),
            ],
        );
        append_group(
            &mut mappings,
            0x0abe_4139_de1a_e16a,
            &[
                (0x036e_aaba_3222_ec24, false),
                (0xc801_dd94_c359_8622, false),
            ],
        );
        append_group(
            &mut mappings,
            0x5d3b_409b_4750_a362,
            &[
                (0x195a_abb6_6c9e_4935, false),
                (0x19ee_bdd7_dc36_58a3, false),
                (0xb21b_a6b1_85b0_1ec9, false),
            ],
        );
        append_group(
            &mut mappings,
            0x01a6_c596_8e02_163d,
            &[
                (0x2743_8ef4_e22d_3d39, false),
                (0x6032_5114_3925_78e4, false),
            ],
        );
        append_group(
            &mut mappings,
            0x42e5_f4d3_cb27_83e2,
            &[
                (0x3dcd_5154_e552_6fa7, false),
                (0x4ce4_5ca0_fc8f_0308, false),
                (0x9bc1_f7cb_5a63_b806, false),
            ],
        );
        append_group(
            &mut mappings,
            0x23e9_5e38_df87_3a6a,
            &[(0x52be_4f59_7e67_7629, false)],
        );
        append_group(
            &mut mappings,
            0x0e7e_bc63_a64b_786a,
            &[
                (0x79b9_da91_8371_86b1, false),
                (0x8d7a_a208_b7ef_f707, false),
            ],
        );
        append_group(
            &mut mappings,
            0x8e3c_ee2e_8cee_3440,
            &[(0xbba7_197f_af01_7c2f, false)],
        );
        append_group(
            &mut mappings,
            0x5249_a4c7_13df_82ff,
            &[(0xc8ea_be24_881e_8004, false)],
        );
        append_group(
            &mut mappings,
            0x2e0e_f977_4e79_1492,
            &[(0xe4cf_d73f_9ee2_fe3e, false)],
        );
        mappings
    }

    #[test]
    fn bucket_collisions_iterate_head_first_and_keep_full_width_keys_distinct() {
        let mut mappings = RecipeMappingTable::new();
        append_group(&mut mappings, 1, &[(10, false)]);
        append_group(&mut mappings, 62, &[(20, false)]);
        append_group(&mut mappings, 0xffff_ffff_0000_0001, &[(30, false)]);

        let queues = build_recipe_loading_queues(&mappings, 3, 64).unwrap();

        assert!(queues.required().is_empty());
        assert_eq!(queues.initial(), [30, 20, 10]);
        assert!(queues.secondary().is_empty());
    }

    #[test]
    fn mapping_builder_preserves_default_seed_candidate_and_grouping_order() {
        let first_seed = DEFAULT_RECIPE_MAPPING_SEEDS[0].0;
        let mut runtime = MappingRuntime {
            forced_samples: vec![false; 5],
            leaves: BTreeMap::from([(first_seed, vec![10, 11, 12])]),
            enabled: BTreeMap::from([(10, true), (11, true), (12, false)]),
            parents: BTreeMap::from([(10, first_seed), (11, 99)]),
            ..MappingRuntime::default()
        };
        let mut mappings = RecipeMappingTable::new();

        build_recipe_mappings(&mut mappings, &mut runtime).unwrap();

        assert_eq!(mappings.groups.len(), 2);
        assert_eq!(mappings.groups[0].group_id, 10);
        assert_eq!(
            mappings.groups[0].entries,
            [RecipeMappingEntry {
                model_id: 10,
                required: true,
            }]
        );
        assert_eq!(mappings.groups[1].group_id, 99);
        assert_eq!(
            mappings.groups[1].entries,
            [RecipeMappingEntry {
                model_id: 11,
                required: true,
            }]
        );
        assert_eq!(
            runtime
                .events
                .iter()
                .filter(|event| matches!(event, MappingEvent::UseForced(false)))
                .count(),
            5
        );
        assert_eq!(
            runtime
                .events
                .iter()
                .filter_map(|event| match event {
                    MappingEvent::Leaves(seed) => Some(*seed),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            DEFAULT_RECIPE_MAPPING_SEEDS.map(|seed| seed.0)
        );
    }

    #[test]
    fn mapping_builder_forced_mode_uses_ref_collection_keys_as_optional_self_groups() {
        let mut runtime = MappingRuntime {
            forced_samples: vec![true; 3],
            forced_collections: BTreeMap::from([
                (FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID, Some(100)),
                (FORCED_VEHICLE_MODEL_ATTRIBUTE_ID, Some(200)),
            ]),
            enabled: BTreeMap::from([(100, true), (200, true)]),
            ..MappingRuntime::default()
        };
        let mut mappings = RecipeMappingTable::new();

        build_recipe_mappings(&mut mappings, &mut runtime).unwrap();

        assert_eq!(
            mappings.groups,
            [
                RecipeMappingGroup {
                    group_id: 100,
                    entries: vec![RecipeMappingEntry {
                        model_id: 100,
                        required: false,
                    }],
                },
                RecipeMappingGroup {
                    group_id: 200,
                    entries: vec![RecipeMappingEntry {
                        model_id: 200,
                        required: false,
                    }],
                },
            ]
        );
        assert_eq!(
            runtime.events,
            [
                MappingEvent::UseForced(true),
                MappingEvent::Forced(FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID),
                MappingEvent::Forced(FORCED_VEHICLE_MODEL_ATTRIBUTE_ID),
                MappingEvent::UseForced(true),
                MappingEvent::Enabled(100),
                MappingEvent::UseForced(true),
                MappingEvent::Enabled(200),
            ]
        );
    }

    #[test]
    fn mapping_builder_resamples_forced_cvar_for_each_default_seed() {
        let first_seed = DEFAULT_RECIPE_MAPPING_SEEDS[0].0;
        let mut runtime = MappingRuntime {
            forced_samples: vec![false, true, false, false, false],
            enabled: BTreeMap::from([(first_seed, true)]),
            ..MappingRuntime::default()
        };
        let mut mappings = RecipeMappingTable::new();

        build_recipe_mappings(&mut mappings, &mut runtime).unwrap();

        assert_eq!(mappings.groups.len(), 1);
        assert_eq!(mappings.groups[0].group_id, first_seed);
        assert_eq!(mappings.groups[0].entries[0].model_id, first_seed);
        assert_eq!(
            runtime
                .events
                .iter()
                .filter(|event| matches!(event, MappingEvent::Leaves(_)))
                .count(),
            3
        );
    }

    #[test]
    fn fixed_mapping_and_queue_capacities_fail_without_silent_overflow() {
        let mut one_group = RecipeMappingTable::new();
        for model_id in 0..4 {
            one_group
                .append(
                    7,
                    RecipeMappingEntry {
                        model_id,
                        required: false,
                    },
                )
                .unwrap();
        }
        let full_group = one_group.clone();
        assert_eq!(
            one_group.append(
                7,
                RecipeMappingEntry {
                    model_id: 4,
                    required: false,
                }
            ),
            Err(RecipeMappingError::GroupEntryCapacity { group_id: 7 })
        );
        assert_eq!(one_group, full_group);

        let mut full_table = RecipeMappingTable::new();
        for group_id in 0..64 {
            full_table
                .append(
                    group_id,
                    RecipeMappingEntry {
                        model_id: group_id,
                        required: false,
                    },
                )
                .unwrap();
        }
        let before_extra_group = full_table.clone();
        assert_eq!(
            full_table.append(
                64,
                RecipeMappingEntry {
                    model_id: 64,
                    required: false,
                }
            ),
            Err(RecipeMappingError::GroupCapacity)
        );
        assert_eq!(full_table, before_extra_group);

        let mut overfull_secondary = RecipeMappingTable::new();
        for group_id in 0..17 {
            for entry_index in 0..4 {
                overfull_secondary
                    .append(
                        group_id,
                        RecipeMappingEntry {
                            model_id: group_id * 4 + entry_index,
                            required: false,
                        },
                    )
                    .unwrap();
            }
        }
        assert_eq!(
            build_recipe_loading_queues(&overfull_secondary, 0, 64),
            Err(RecipeMappingError::QueueCapacity {
                queue: RecipeLoadingQueueKind::Secondary,
            })
        );
    }

    #[test]
    fn shipped_default_mapping_reproduces_exact_native_queue_order() {
        let mappings = shipped_default_mapping();
        assert_eq!(mappings.group_count(), 24);
        assert_eq!(mappings.entry_count(), 54);

        let untruncated = build_recipe_loading_queues(&mappings, 15, 64).unwrap();
        assert_eq!(
            untruncated.required(),
            [
                0xb7cd_d46e_e0b8_e9b0,
                0x29ea_db80_7f46_edf6,
                0x7db7_14d7_e927_5d53,
                0x8151_8f0e_dafe_70a6,
                0xef73_f403_30bd_5d77,
                0x56f3_cd83_30b9_78ee,
                0x5bbf_cd8b_c95c_b2e7,
                0x58b6_0bae_7322_56bb,
                0xcb2c_143b_9929_a70a,
                0x041a_0b27_8eae_b09c,
                0x1e58_8e6d_426d_1a2f,
                0x5ef2_eddd_c75f_25de,
                0x030a_b922_4e08_cbd1,
                0x8960_aeb0_8f12_71cd,
                0xf977_b957_a0e8_68cb,
            ]
        );
        assert!(untruncated.initial().is_empty());
        assert_eq!(
            untruncated.secondary(),
            [
                0xbba7_197f_af01_7c2f,
                0x47ad_17fd_06f5_3aba,
                0xa2b2_41f6_ac35_b367,
                0xc81c_3aac_7f34_f22d,
                0x3dcd_5154_e552_6fa7,
                0x4ce4_5ca0_fc8f_0308,
                0x9bc1_f7cb_5a63_b806,
                0x036e_aaba_3222_ec24,
                0xc801_dd94_c359_8622,
                0x34a2_a830_f2ab_aee2,
                0x5ac9_5e9c_35f6_5bed,
                0xe602_f3b8_5b00_f793,
                0x3ba9_21fc_ef1b_45fd,
                0x445f_8033_83f2_6837,
                0x6daf_1766_3d47_fd14,
                0xc8ea_be24_881e_8004,
                0x538d_c4b8_8de9_4552,
                0x785d_7012_1bfa_192d,
                0xf455_ac5a_7af6_9c18,
                0x1594_5e1b_6db9_452d,
                0x3457_c13f_e6d1_28f1,
                0x9552_f722_9512_9362,
                0x05f6_62f7_109b_dede,
                0x798c_b224_ed7f_bfee,
                0xfe1f_4594_b6f9_18e2,
                0xe4cf_d73f_9ee2_fe3e,
                0x2743_8ef4_e22d_3d39,
                0x6032_5114_3925_78e4,
                0x5dbe_f74c_53c5_5e6e,
                0xde2d_9c8a_d95e_dcb0,
                0x195a_abb6_6c9e_4935,
                0x19ee_bdd7_dc36_58a3,
                0xb21b_a6b1_85b0_1ec9,
                0x79b9_da91_8371_86b1,
                0x8d7a_a208_b7ef_f707,
                0x325e_baa6_9b25_8848,
                0x9a4d_5e87_ade3_af28,
                0xedeb_a27c_4163_bfae,
                0x52be_4f59_7e67_7629,
            ]
        );

        let queues = build_recipe_loading_queues(&mappings, 15, 10).unwrap();
        assert_eq!(queues.required(), untruncated.required());
        assert!(queues.initial().is_empty());
        assert_eq!(
            queues.secondary(),
            [
                0xbba7_197f_af01_7c2f,
                0x47ad_17fd_06f5_3aba,
                0xa2b2_41f6_ac35_b367,
                0xc81c_3aac_7f34_f22d,
                0x3dcd_5154_e552_6fa7,
                0x4ce4_5ca0_fc8f_0308,
                0x9bc1_f7cb_5a63_b806,
                0x036e_aaba_3222_ec24,
                0xc801_dd94_c359_8622,
                0x34a2_a830_f2ab_aee2,
            ]
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Mappings,
        Queues,
    }

    #[derive(Default)]
    struct Runtime {
        events: Vec<Event>,
    }

    impl RecipeManagerInitializationRuntime for Runtime {
        type Error = Infallible;

        fn initialize_recipe_mappings(&mut self) -> Result<(), Self::Error> {
            self.events.push(Event::Mappings);
            Ok(())
        }

        fn initialize_loading_queues(&mut self) -> Result<(), Self::Error> {
            self.events.push(Event::Queues);
            Ok(())
        }
    }

    #[test]
    fn zero_state_initializes_mappings_then_queues_then_terminal_fields() {
        let mut state = RecipeManagerInitializationState::from_raw(0, 0xdead_beef);
        let mut runtime = Runtime::default();

        assert_eq!(
            initialize_recipe_manager(&mut state, &mut runtime).unwrap(),
            RecipeManagerInitialization::InitializedNow
        );
        assert_eq!(runtime.events, [Event::Mappings, Event::Queues]);
        assert_eq!(state.initialized_raw(), 1);
        assert_eq!(state.frame_delay_raw(), 0);
    }

    #[test]
    fn any_nonzero_initialized_byte_preserves_both_raw_fields_and_calls_nothing() {
        let mut state = RecipeManagerInitializationState::from_raw(7, 0xdead_beef);
        let mut runtime = Runtime::default();

        assert_eq!(
            initialize_recipe_manager(&mut state, &mut runtime).unwrap(),
            RecipeManagerInitialization::AlreadyInitialized
        );
        assert!(runtime.events.is_empty());
        assert_eq!(state.initialized_raw(), 7);
        assert_eq!(state.frame_delay_raw(), 0xdead_beef);
    }
}
