//! Retained `SkaterAnim` constructor prefix and Vault/cvar continuation.
//!
//! `0x82cc0a90` constructs `Animatable`, `ChannelAnimatable` and `ISkaterAnim` on
//! the SAME Entity. The first stage stops before `Attrib::FindCollection` at
//! `0x82cc0ef4`; the continuation stops before face setup at `0x82cc1190`.
//! It does not publish Actor+0x1fc, initialize playback, perform
//! face setup, or assert that later animation/physics jobs exist.
//! Native allocation size is 0x61d0; the constructor code length is 0x720.
#![expect(
    dead_code,
    reason = "retained animation construction awaits Actor mounting and later Vault/playback stages"
)]

use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{
    actor_face_playback::{ActorFaceError, ActorFacePlayback, FaceAnimationManagers},
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    character_face_manager::CharacterFaceManager,
    motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority},
    play_animation::{EMPTY_FAST_STRING_36, FastString30, FastString36},
};
use skate2_formats::andale::AndaleRuntimeAttribute;
use std::{
    fmt,
    sync::{Arc, Mutex},
};

#[path = "actor_animation_body_spin_channels.rs"]
pub(crate) mod body_spin_channels;
#[path = "actor_animation_initialization.rs"]
pub(crate) mod initialization;

#[path = "actor_animation_blend_anim.rs"]
pub(crate) mod blend_anim;
#[cfg(test)]
#[path = "actor_animation_blend_corpus.rs"]
mod blend_corpus;
#[path = "../channels/actor_animation_channel_remove.rs"]
pub(crate) mod channel_remove;
#[path = "actor_animation_play_anim.rs"]
pub(crate) mod play_anim;
#[path = "actor_animation_reset_skater.rs"]
pub(crate) mod reset_skater;
#[path = "../channels/actor_animation_transition_eval.rs"]
pub(crate) mod transition_eval;
#[path = "../channels/actor_animation_transition_time.rs"]
pub(crate) mod transition_time;

#[path = "../locomotion/actor_animation_locomotion_action.rs"]
pub(crate) mod locomotion_action;
#[path = "../locomotion/actor_animation_locomotion_create.rs"]
pub(crate) mod locomotion_create;
#[path = "../locomotion/actor_animation_locomotion_create_attribute.rs"]
pub(crate) mod locomotion_create_attribute;
#[path = "../locomotion/actor_animation_locomotion_create_correction.rs"]
pub(crate) mod locomotion_create_correction;
#[path = "../locomotion/actor_animation_locomotion_create_finish.rs"]
pub(crate) mod locomotion_create_finish;
#[path = "../locomotion/actor_animation_locomotion_create_mg.rs"]
pub(crate) mod locomotion_create_mg;

#[path = "actor_animation_frame_phase.rs"]
pub(crate) mod frame_phase;
#[path = "../locomotion/actor_animation_locomotion_collision_offset.rs"]
pub(crate) mod locomotion_collision_offset;
#[path = "../locomotion/actor_animation_locomotion_debug.rs"]
pub(crate) mod locomotion_debug;
#[path = "../locomotion/actor_animation_locomotion_lifecycle.rs"]
pub(crate) mod locomotion_lifecycle;
#[path = "../locomotion/actor_animation_locomotion_mid.rs"]
pub(crate) mod locomotion_mid;
#[path = "../locomotion/actor_animation_locomotion_params.rs"]
pub(crate) mod locomotion_params;
#[path = "../locomotion/actor_animation_locomotion_producer.rs"]
pub(crate) mod locomotion_producer;
#[path = "../locomotion/actor_animation_locomotion_update.rs"]
pub(crate) mod locomotion_update;
#[path = "../graphs/motion/generated_motion_runtime_animation.rs"]
pub(crate) mod motion_runtime;
#[path = "actor_animation_moving_objects.rs"]
pub(crate) mod moving_objects;

#[path = "actor_animation_get_phys_update_data.rs"]
pub(crate) mod physics_update_data;

#[cfg(test)]
#[path = "actor_animation_construction_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActorAnimationInterface {
    Animatable,
    ChannelAnimatable,
    SkaterAnim,
}
impl ActorAnimationInterface {
    pub(crate) const fn source_name(self) -> u32 {
        match self {
            Self::Animatable => 0x825e_c080,
            Self::ChannelAnimatable => 0x825e_cbb0,
            Self::SkaterAnim => 0x824b_0488,
        }
    }
}

/// Actual Actor registry and Actor query bridge. Add must validate the same
/// issuance, reject duplicate interfaces, and retain this component occurrence.
/// Query must return the SAME Actor's source player index at its call point.
pub(crate) trait ActorAnimationRegistry<K> {
    type Error;
    fn register_animation_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        kind: ActorAnimationInterface,
        occurrence: &Arc<()>,
    ) -> Result<(), Self::Error>;
    fn actor_player_index(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<u32, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationLifetime {
    Transient,
    Permanent,
}
impl AnimationLifetime {
    pub(crate) const fn source_value(self) -> u32 {
        match self {
            Self::Transient => 0,
            Self::Permanent => 1,
        }
    }
}

/// Retained selected caller arguments, distinct from constructor member writes.
/// W must be the caller's actual world owner; an arbitrary W is not certification
/// that native `IWorld` services have been reconstructed. Face names retain bytes,
/// including any C-string suffix; no early encoding or face-data query occurs.
pub(crate) struct ActorAnimationInputs<W> {
    pub(crate) physics_world: Arc<W>,
    pub(crate) life: AnimationLifetime,
    pub(crate) use_low_blend_cull_threshold: bool,
    pub(crate) face_name: Option<Arc<[u8]>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActorAnimationPhase {
    Unattempted,
    RegisteringAnimatable,
    RegisteringChannelAnimatable,
    RegisteringSkaterAnim,
    QueryingPlayerIndex,
    VaultRequired,
    SamplingSprintCvar,
    FaceSetupRequired,
    FaceSetupRunning,
    ConstructorReturned,
}
#[derive(Debug, PartialEq)]
pub(crate) enum ActorAnimationConstructionError<E> {
    AlreadyAttempted,
    Registry(E),
    VaultPrefixRequired,
    Cvar(E),
    FacePrefixRequired,
    FaceManagerRequired,
    Face(ActorFaceError),
    FaceCommands(E),
}

pub(crate) trait ActorAnimationCvars {
    type Error;
    /// Actual `kUseSprintButton` target byte, read at 82cc0f94/0f98, after
    /// the preceding constructor fields. Any nonzero byte selects 0.8.
    fn use_sprint_button(&mut self) -> Result<u8, Self::Error>;
}

pub(crate) const ACTOR_ANIMATION_OFFBOARD_CLASS: u64 = 0x4920_26e0_f096_f283;
pub(crate) const ACTOR_ANIMATION_OFFBOARD_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
impl<E: fmt::Debug> fmt::Display for ActorAnimationConstructionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Actor animation constructor: {self:?}")
    }
}

/// A pointer member that this prefix either initializes null or leaves unknown.
/// This deliberately cannot masquerade as a live playback/tree/job reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationPointerState {
    Unwritten,
    Null,
    /// A concrete retained owner is stored on `ActorFacePlayback`.
    Constructed,
}

/// Concrete fixed-capacity storage. Empty slots are host ownership bookkeeping,
/// not a claim that native unused inline elements were zero initialized.
pub(crate) struct AnimationFixedStorage<T, const N: usize> {
    slots: [Option<T>; N],
    len: usize,
}
impl<T, const N: usize> AnimationFixedStorage<T, N> {
    fn empty() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            len: 0,
        }
    }
    pub(crate) const fn len(&self) -> usize {
        self.len
    }
    pub(crate) const fn maximum(&self) -> usize {
        self.slots.len()
    }
    /// Occupied source cells in native vector order. An absent occupied cell
    /// remains visible as an error boundary instead of being filtered out.
    pub(crate) fn occupied_slots(&self) -> &[Option<T>] {
        &self.slots[..self.len]
    }
}
pub(crate) struct AnimationSettableAttribute {
    name: crate::play_animation::FastString30,
    sequence_id: i32,
    attribute: AndaleRuntimeAttribute,
    normalized: bool,
}
#[path = "../channels/actor_animation_channel_runtime.rs"]
pub(crate) mod channel_runtime;
#[path = "actor_animation_gesture_writes.rs"]
pub(crate) mod gesture_writes;
#[path = "../hands_face/actor_animation_hands_face_dispatch.rs"]
pub(crate) mod hands_face;

/// Empty `PoolStack` constructor at 82d7b70c..b778 / 82cc0b44..b74.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnimationPoolStorage {
    pub(crate) max_size: u32,
    pub(crate) max_elements: u32,
    pub(crate) count: u32,
    pub(crate) location: u32,
    pub(crate) elements: AnimationPointerState,
    pub(crate) memory: AnimationPointerState,
    pub(crate) memory_cursor: AnimationPointerState,
}
impl AnimationPoolStorage {
    fn empty() -> Self {
        Self {
            max_size: 0,
            max_elements: 0,
            count: 0,
            location: 0,
            elements: AnimationPointerState::Null,
            memory: AnimationPointerState::Null,
            memory_cursor: AnimationPointerState::Null,
        }
    }
}

/// Constructor plus Clear, 82d7b6f0 / 82d79e18. Free-list entries reference
/// these actual four slots by index in native append order, never copies.
pub(crate) struct AnimationTreeStore {
    pub(crate) trees: [AnimationPoolStorage; 4],
    pub(crate) free_order: [usize; 4],
    pub(crate) max_trees: usize,
    pub(crate) free_count: usize,
}
impl AnimationTreeStore {
    fn new() -> Self {
        Self {
            trees: [AnimationPoolStorage::empty(); 4],
            free_order: [0, 1, 2, 3],
            max_trees: 4,
            free_count: 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AnimationTransitionBlend {
    pub(crate) runtime: Option<blend_anim::TransitionBlendRuntime>,
    pub(crate) evaluated: bool,
    pub(crate) to_animation: AnimationPointerState,
    pub(crate) from_animation: AnimationPointerState,
    pub(crate) length: f32,
    pub(crate) time: f32,
    /// Native high three bits clear; low29 remain unknown.
    pub(crate) flags_known_zero_mask: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnimationSequence {
    pub(crate) evaluated: bool,
    pub(crate) to_animation: AnimationPointerState,
    pub(crate) from_animation: AnimationPointerState,
    pub(crate) transition_time: f32,
    pub(crate) playing_new_animation: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnimationTransitionHandle {
    Blend(usize),
    Sequence(usize),
}
/// Constructor plus Clear, 82d7b7a8 / 82d7a080. Clear resets lists only;
/// it does not initialize the unknown transition animation pointers.
pub(crate) struct AnimationTransitionStore {
    pub(crate) max_trees: usize,
    pub(crate) max_transition_blends: usize,
    pub(crate) blends: [AnimationTransitionBlend; 3],
    pub(crate) sequences: [AnimationSequence; 3],
    pub(crate) sequence_runtime: [Option<blend_anim::TransitionSequenceRuntime>; 3],
    pub(crate) transition_order: AnimationFixedStorage<AnimationTransitionHandle, 3>,
    pub(crate) blend_order: AnimationFixedStorage<usize, 3>,
    pub(crate) free_blends: [usize; 3],
    pub(crate) free_sequences: [usize; 3],
    pub(crate) free_blend_count: usize,
    pub(crate) free_sequence_count: usize,
}
impl AnimationTransitionStore {
    fn new() -> Self {
        Self {
            max_trees: 4,
            max_transition_blends: 3,
            blends: std::array::from_fn(|_| AnimationTransitionBlend {
                runtime: None,
                evaluated: false,
                to_animation: AnimationPointerState::Unwritten,
                from_animation: AnimationPointerState::Unwritten,
                length: 0.0,
                time: 0.0,
                flags_known_zero_mask: 0xe000_0000,
            }),
            sequences: [AnimationSequence {
                evaluated: false,
                to_animation: AnimationPointerState::Unwritten,
                from_animation: AnimationPointerState::Unwritten,
                transition_time: 0.0,
                playing_new_animation: false,
            }; 3],
            sequence_runtime: std::array::from_fn(|_| None),
            transition_order: AnimationFixedStorage::empty(),
            blend_order: AnimationFixedStorage::empty(),
            free_blends: [0, 1, 2],
            free_sequences: [0, 1, 2],
            free_blend_count: 3,
            free_sequence_count: 3,
        }
    }
}

/// Actual retained fields established by `Animatable::Animatable` 82d7b8e8.
/// Trajectory, pose-buffer hierarchy, settable animation instance, max clips
/// and blend-space data remain unknown; they are not inferred from zero bones.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent original animation flags have separate source stores"
)]
pub(crate) struct ActorAnimatableBase {
    pub(crate) base_animation_name: FastString36,
    pub(crate) playback: AnimationPointerState,
    pub(crate) bind_pose: AnimationPointerState,
    pub(crate) bone_count: i32,
    pub(crate) settable_attributes: AnimationFixedStorage<AnimationSettableAttribute, 150>,
    pub(crate) request_blend_space: AnimationPointerState,
    pub(crate) settable_animation: AnimationPointerState,
    pub(crate) tree_attributes: AnimationFixedStorage<AndaleRuntimeAttribute, 120>,
    pub(crate) trajectory: Option<[[f32; 4]; 3]>,
    pub(crate) global_pose: AnimationPointerState,
    pub(crate) local_pose: AnimationPointerState,
    pub(crate) global_hierarchy: AnimationPointerState,
    pub(crate) local_hierarchy: AnimationPointerState,
    pub(crate) bind_pose_evaluated: bool,
    pub(crate) bind_pose_header: AnimationPointerState,
    pub(crate) is_playing: bool,
    pub(crate) expired_or_looped: bool,
    pub(crate) expired_or_looped_by_time: f32,
    pub(crate) time_to_expire_or_loop: f32,
    pub(crate) blend_space_job_started: bool,
    pub(crate) scheduler: AnimationPointerState,
    pub(crate) blend_space_job: AnimationPointerState,
    pub(crate) life: AnimationLifetime,
    pub(crate) base_animation: AnimationPointerState,
    pub(crate) body_root: Option<blend_anim::BodyAnimationReference>,
    pub(crate) attributes_pool: AnimationPointerState,
    pub(crate) construction_map: AnimationFixedStorage<(FastString30, FastString30), 6>,
    pub(crate) tree_store: AnimationTreeStore,
    pub(crate) transition_store: AnimationTransitionStore,
    pub(crate) max_clips_used: Option<i32>,
    pub(crate) blend_space_job_code: AnimationPointerState,
    pub(crate) blend_space_data:
        Option<Arc<Mutex<crate::actor_animation_tree::blend_space::jobs::BlendSpaceJobBuffer>>>,
}
impl ActorAnimatableBase {
    fn new(life: AnimationLifetime) -> Self {
        use AnimationPointerState::{Null, Unwritten};
        Self {
            base_animation_name: EMPTY_FAST_STRING_36,
            playback: Null,
            bind_pose: Null,
            bone_count: 0,
            settable_attributes: AnimationFixedStorage::empty(),
            request_blend_space: Null,
            settable_animation: Unwritten,
            tree_attributes: AnimationFixedStorage::empty(),
            trajectory: None,
            global_pose: Null,
            local_pose: Null,
            global_hierarchy: Unwritten,
            local_hierarchy: Unwritten,
            bind_pose_evaluated: false,
            bind_pose_header: Null,
            is_playing: false,
            expired_or_looped: false,
            expired_or_looped_by_time: -1.0,
            time_to_expire_or_loop: -1.0,
            blend_space_job_started: false,
            scheduler: Null,
            blend_space_job: Null,
            life,
            base_animation: Null,
            body_root: None,
            attributes_pool: Null,
            construction_map: AnimationFixedStorage::empty(),
            tree_store: AnimationTreeStore::new(),
            transition_store: AnimationTransitionStore::new(),
            max_clips_used: None,
            blend_space_job_code: Null,
            blend_space_data: None,
        }
    }
}
pub(crate) struct ActorChannelAnimatable {
    pub(crate) max_channels: Option<i32>,
    pub(crate) max_channel_tree_size: Option<i32>,
    pub(crate) channels: AnimationPointerState,
    pub(crate) dictionary: AnimationPointerState,
    pub(crate) free_channels: AnimationPointerState,
    pub(crate) trees: AnimationPointerState,
    pub(crate) free_trees: AnimationPointerState,
    pub(crate) root_channel: AnimationPointerState,
}
impl ActorChannelAnimatable {
    fn new() -> Self {
        use AnimationPointerState::Null;
        Self {
            max_channels: None,
            max_channel_tree_size: None,
            channels: Null,
            dictionary: Null,
            free_channels: Null,
            trees: Null,
            free_trees: Null,
            root_channel: Null,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnimationFootState {
    pub(crate) vectors: [WalkVector; 4],
    pub(crate) time: f32,
    pub(crate) action: i32,
    pub(crate) debug_flags: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnimationFootData {
    pub(crate) position: WalkVector,
    pub(crate) normal: WalkVector,
    pub(crate) surface: u32,
}
impl AnimationFootData {
    fn zero() -> Self {
        Self {
            position: WalkVector::known([0.0; 4]),
            normal: WalkVector::known([0.0; 4]),
            surface: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnimationFootControl {
    pub(crate) states: [AnimationFootState; 3],
    pub(crate) data: AnimationFootData,
    pub(crate) stride: WalkVector,
    pub(crate) initialized: bool,
    pub(crate) stride_initialized: bool,
    /// Word at native tail+134. State -1 action probes alias the preceding
    /// foot's reserved storage. Generic construction cannot invent its value.
    pub(crate) trailing_action_probe_word: Option<u32>,
}
impl AnimationFootControl {
    /// `FootControl` ctor 82c92958 and Reset 82c8a610. Folded vector ctor
    /// 825187c0 is BLR. Reset overwrites all four lanes of every vector.
    fn new() -> Self {
        Self {
            states: [AnimationFootState {
                vectors: [WalkVector::known([0.0; 4]); 4],
                time: 0.0,
                action: -1,
                debug_flags: 0,
            }; 3],
            data: AnimationFootData::zero(),
            stride: WalkVector::known([0.0, 0.0, 1.0, 0.0]),
            initialized: false,
            stride_initialized: false,
            trailing_action_probe_word: None,
        }
    }
    fn reset(&mut self) {
        let trailing_action_probe_word = self.trailing_action_probe_word;
        *self = Self::new();
        self.trailing_action_probe_word = trailing_action_probe_word;
    }
}
pub(crate) struct AnimationLocomotionTrajectory {
    pub(crate) input_feet: [AnimationFootControl; 4],
    pub(crate) physical_feet: [AnimationFootControl; 4],
    pub(crate) physical_foot_index: i32,
    pub(crate) first_update: bool,
    pub(crate) foot_locked: i32,
    pub(crate) time: f32,
    pub(crate) current_state: i32,
    pub(crate) raw_time: f32,
}
impl AnimationLocomotionTrajectory {
    /// 82c92a50 constructs four input then four physical feet. Reset 82c8b880
    /// resets input/physical pairs in index order, then scalar fields.
    fn new() -> Self {
        let mut input_feet = std::array::from_fn(|_| AnimationFootControl::new());
        let mut physical_feet = std::array::from_fn(|_| AnimationFootControl::new());
        for (input, physical) in input_feet.iter_mut().zip(&mut physical_feet) {
            input.reset();
            physical.reset();
        }
        Self {
            input_feet,
            physical_feet,
            physical_foot_index: 0,
            first_update: true,
            foot_locked: 0,
            time: 0.0,
            current_state: -1,
            raw_time: 0.0,
        }
    }
}
pub(crate) type AnimationStepResult = crate::walking_step_results::StepSelectionFuture;

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimationStepInput {
    pub(crate) reference: WalkVector,
    pub(crate) reference_offset: WalkVector,
    pub(crate) parameter_vector: WalkVector,
    pub(crate) speed_at_end: WalkVector,
    pub(crate) parameter: f32,
    pub(crate) spread_angle: f32,
    pub(crate) turn_intent: f32,
    pub(crate) max_y: f32,
    pub(crate) flags: u8,
}

/// `SkaterAnim` fields through 82cc0e24; the Actor player query follows.
pub(crate) struct ActorSkaterAnimationPrefix<W> {
    pub(crate) hand_and_face_poses: [AnimationPointerState; 3],
    pub(crate) hand_and_face_hierarchies: [AnimationPointerState; 3],
    pub(crate) attributes_pool: AnimationPoolStorage,
    pub(crate) face_pose_name: FastString36,
    pub(crate) face_playback_name: FastString36,
    pub(crate) database_manager: AnimationPointerState,
    pub(crate) playback_manager: AnimationPointerState,
    pub(crate) face_playback: AnimationPointerState,
    pub(crate) command_manager: AnimationPointerState,
    pub(crate) trajectory: AnimationLocomotionTrajectory,
    pub(crate) last_steps: [AnimationFootData; 4],
    pub(crate) physics_world: Arc<W>,
    pub(crate) step_results: [AnimationStepResult; 2],
    pub(crate) step_inputs: [AnimationStepInput; 2],
    pub(crate) board_backwards_ik: AnimationPointerState,
    pub(crate) board_backwards: AnimationPointerState,
}
impl<W> ActorSkaterAnimationPrefix<W> {
    fn new(world: &Arc<W>) -> Self {
        use AnimationPointerState::{Null, Unwritten};
        Self {
            hand_and_face_poses: [Null; 3],
            hand_and_face_hierarchies: [Unwritten; 3],
            attributes_pool: AnimationPoolStorage::empty(),
            face_pose_name: EMPTY_FAST_STRING_36,
            face_playback_name: EMPTY_FAST_STRING_36,
            database_manager: Null,
            playback_manager: Null,
            face_playback: Null,
            command_manager: Null,
            trajectory: AnimationLocomotionTrajectory::new(),
            last_steps: [AnimationFootData::zero(); 4],
            physics_world: Arc::clone(world),
            step_results: [AnimationStepResult::constructed(); 2],
            step_inputs: std::array::from_fn(|_| AnimationStepInput {
                reference: WalkVector::known([0.0; 4]),
                reference_offset: WalkVector::known([0.0; 4]),
                parameter_vector: WalkVector::known([0.0; 4]),
                speed_at_end: WalkVector::known([0.0; 4]),
                parameter: 0.0,
                spread_angle: 0.0,
                turn_intent: 0.0,
                max_y: 0.0,
                flags: 0,
            }),
            board_backwards_ik: Null,
            board_backwards: Null,
        }
    }
}
pub(crate) struct ActorAnimationPlayerFields {
    pub(crate) player_index: u32,
    pub(crate) natural_stance: u32,
    pub(crate) last_stance: u32,
    pub(crate) relative_stance: u32,
    pub(crate) spawn_relative_stance: u32,
    pub(crate) opacity: f32,
    pub(crate) flags_known_mask: u32,
    pub(crate) flags_bits: u32,
    /// Up, down, left, right. Raw source values15,12,13,14.
    pub(crate) gestures: [u32; 4],
    pub(crate) pro_skater: FastString30,
}

/// Fields written after the actual attribute instance constructor but BEFORE
/// the cvar sample. These survive a checked failure of the runtime cvar read.
pub(crate) struct ActorAnimationBeforeSprint {
    pub(crate) moving_objects: moving_objects::MovingObjectsCoefficients,
    pub(crate) foot_down: i32,
    pub(crate) last_foot_down: i32,
    pub(crate) speed: f32,
    pub(crate) last_start_state: i32,
    pub(crate) grab_type: u32,
    pub(crate) face_clip: AnimationPointerState,
    pub(crate) left_hand_clip: AnimationPointerState,
    pub(crate) right_hand_clip: AnimationPointerState,
    pub(crate) step_failure_counts: [i32; 2],
    pub(crate) air_dismount_revert_frames: u32,
    pub(crate) second_flags_known_mask: u32,
    pub(crate) second_flags_bits: u32,
}
impl ActorAnimationBeforeSprint {
    fn new() -> Self {
        Self {
            moving_objects: moving_objects::MovingObjectsCoefficients::after_construction(),
            foot_down: -1,
            last_foot_down: -1,
            speed: 0.0,
            last_start_state: -1,
            grab_type: 0,
            face_clip: AnimationPointerState::Null,
            left_hand_clip: AnimationPointerState::Null,
            right_hand_clip: AnimationPointerState::Null,
            step_failure_counts: [0; 2],
            air_dismount_revert_frames: 0,
            second_flags_known_mask: 0xc000_0000,
            second_flags_bits: 0x8000_0000,
        }
    }
}

/// Actual suffix fields at 82cc0fb0..118c. The collision response turn is NOT
/// initialized by this constructor; none must not be interpreted as zero.
pub(crate) struct ActorAnimationBeforeFace {
    pub(crate) speed_clamp: f32,
    pub(crate) desired_speed: f32,
    pub(crate) sprint_level: f32,
    pub(crate) time_to_foot_change: f32,
    pub(crate) time_since_leaving_locomotion: f32,
    pub(crate) turn: f32,
    pub(crate) world_position: crate::skeleton_root_transforms::SkeletonRootMatrix,
    pub(crate) extra_y_rotation: [[f32; 4]; 4],
    pub(crate) applied_extra_y_rotation: [[f32; 4]; 4],
    pub(crate) collision_normal_timer: f32,
    pub(crate) step_turn_adjustment: f32,
    pub(crate) current_stage: i32,
    pub(crate) blend_space_entry_point: AnimationPointerState,
    pub(crate) feet: [WalkVector; 2],
    pub(crate) queried_feet: [WalkVector; 2],
    pub(crate) last_centre_of_mass: WalkVector,
    pub(crate) collision_displacement: WalkVector,
    pub(crate) collision_trajectory_displacement: WalkVector,
    pub(crate) last_collision_normal: WalkVector,
    pub(crate) collision_response_turn: Option<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnimationConstructionFlavor {
    Skater,
    Pedestrian,
}

/// Owns the same source allocation occurrence before any interface Add. A failed
/// Add/query retains earlier components and never retries constructor effects.
/// Host inline storage/Arc allocation is not native allocator parity.
pub(crate) struct ActorAnimationConstruction<K, W> {
    pub(crate) flavor: AnimationConstructionFlavor,
    entity: MotionGraphEntityAuthority<K>,
    occurrence: Arc<()>,
    inputs: ActorAnimationInputs<W>,
    phase: ActorAnimationPhase,
    base: Option<ActorAnimatableBase>,
    channel: Option<ActorChannelAnimatable>,
    skater: Option<ActorSkaterAnimationPrefix<W>>,
    player: Option<ActorAnimationPlayerFields>,
    offboard_attributes: Option<AttributeRuntimeInstance>,
    locomotion_global_attributes: Option<AttributeRuntimeInstance>,
    before_sprint: Option<ActorAnimationBeforeSprint>,
    before_face: Option<ActorAnimationBeforeFace>,
    face_managers: Option<FaceAnimationManagers>,
    face: Option<ActorFacePlayback>,
    initialization: Option<initialization::ActorAnimationInitialization>,
    sim_allocation_cleared: bool,
    /// PC mode: the selected S3 physical owner supplies walking trajectories.
    /// Native S2 character/body/hands/face jobs still use their real owners.
    skate3_movement: bool,
    #[cfg(feature = "skate3-runtime")]
    pub(crate) mode_body_pose: Option<initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer>,
    /// The S3 body tree behind `mode_body_pose`; its clips' hand streams pose the fingers.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) mode_body_commands: Option<Vec<skate3_core::animation::playback_tree::PoseCommand>>,
    /// The S3 SkaterAnim's leading foot with fakie (82592B68), RIGHT as true: markers and
    /// checkpoints read the animation that actually poses the skater.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) mode_foot_forward_right: Option<bool>,
    #[cfg(feature = "skate3-runtime")]
    pub(crate) pc_gesture_preferences: skate2_formats::skater_profile::PcGestureSettings,
    #[cfg(feature = "skate3-runtime")]
    pub(crate) pc_gesture_playback: Option<crate::skate3_gesture_playback::Playback>,
}
impl<K, W> ActorAnimationConstruction<K, W> {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn select_skate3_movement(&mut self, selected: bool) {
        if self.skate3_movement != selected {
            // A handoff never resurrects a pending trajectory from the other
            // solver. These are IsInLocomotion / WasInLocomotion, not tuning.
            if let Some(player) = self.player.as_mut() {
                player.flags_bits &= !(0x0010_0000 | 0x0008_0000);
                player.flags_known_mask |= 0x0010_0000 | 0x0008_0000;
            }
            self.skate3_movement = selected;
            if !selected { self.mode_body_pose=None; self.mode_body_commands=None; self.mode_foot_forward_right=None; }
        }
    }

    /// `cPedestrianAnim82e90390` shares Animatable/ChannelAnimatable storage.
    /// No ISkaterAnim, player flags, face owner or skater suffix is constructed.
    pub(crate) fn new_pedestrian(authority: &ActorGraphAuthority<K>, world: Arc<W>) -> Self {
        let mut owner = Self::new(
            authority,
            ActorAnimationInputs {
                physics_world: world,
                life: AnimationLifetime::Transient,
                use_low_blend_cull_threshold: false,
                face_name: None,
            },
        );
        owner.flavor = AnimationConstructionFlavor::Pedestrian;
        owner.base = Some(ActorAnimatableBase::new(AnimationLifetime::Transient));
        owner.channel = Some(ActorChannelAnimatable::new());
        owner.phase = ActorAnimationPhase::ConstructorReturned;
        owner
    }

    pub(crate) fn new(authority: &ActorGraphAuthority<K>, inputs: ActorAnimationInputs<W>) -> Self {
        Self {
            flavor: AnimationConstructionFlavor::Skater,
            entity: authority.entity_handle(),
            occurrence: Arc::new(()),
            inputs,
            phase: ActorAnimationPhase::Unattempted,
            base: None,
            channel: None,
            skater: None,
            player: None,
            offboard_attributes: None,
            locomotion_global_attributes: None,
            before_sprint: None,
            before_face: None,
            face_managers: None,
            face: None,
            initialization: None,
            sim_allocation_cleared: false,
            skate3_movement: false,
            #[cfg(feature = "skate3-runtime")]
            mode_body_pose: None,
            #[cfg(feature = "skate3-runtime")]
            mode_body_commands: None,
            #[cfg(feature = "skate3-runtime")]
            mode_foot_forward_right: None,
            #[cfg(feature = "skate3-runtime")]
            pc_gesture_preferences: Default::default(),
            #[cfg(feature = "skate3-runtime")]
            pc_gesture_playback: None,
        }
    }
    /// Actual Actor allocation825fedbc uses Sim category55: maskFFFFFFFF,
    /// OR100. `AllocHeapMem82972340/829723bc` clears the full0x61d0 allocation.
    /// This caller provenance is distinct from constructor/Reset writes.
    pub(crate) fn try_mark_sim_allocation(
        &mut self,
    ) -> Result<(), ActorAnimationConstructionError<std::convert::Infallible>> {
        if self.phase != ActorAnimationPhase::Unattempted || self.sim_allocation_cleared {
            return Err(ActorAnimationConstructionError::AlreadyAttempted);
        }
        self.sim_allocation_cleared = true;
        Ok(())
    }
    /// Caller captures actual ACS/playback/database managers before entering
    /// the constructor, in its own source order 825fee04..825feec0.
    pub(crate) fn new_with_face_managers(
        authority: &ActorGraphAuthority<K>,
        inputs: ActorAnimationInputs<W>,
        managers: FaceAnimationManagers,
    ) -> Self {
        let mut constructed = Self::new(authority, inputs);
        constructed.face_managers = Some(managers);
        constructed
    }
    /// Source caller captures managers after allocating storage and before
    /// entering the native constructor. No interface is registered here.
    pub(crate) fn try_bind_face_managers(
        &mut self,
        managers: FaceAnimationManagers,
    ) -> Result<(), ActorAnimationConstructionError<std::convert::Infallible>> {
        if self.phase != ActorAnimationPhase::Unattempted || self.face_managers.is_some() {
            return Err(ActorAnimationConstructionError::AlreadyAttempted);
        }
        self.face_managers = Some(managers);
        Ok(())
    }
    pub(crate) fn try_construct_prefix<R: ActorAnimationRegistry<K>>(
        &mut self,
        registry: &mut R,
    ) -> Result<(), ActorAnimationConstructionError<R::Error>> {
        if self.phase != ActorAnimationPhase::Unattempted {
            return Err(ActorAnimationConstructionError::AlreadyAttempted);
        }
        self.phase = ActorAnimationPhase::RegisteringAnimatable;
        registry
            .register_animation_interface(
                &self.entity,
                ActorAnimationInterface::Animatable,
                &self.occurrence,
            )
            .map_err(ActorAnimationConstructionError::Registry)?;
        self.base = Some(ActorAnimatableBase::new(self.inputs.life));
        self.phase = ActorAnimationPhase::RegisteringChannelAnimatable;
        registry
            .register_animation_interface(
                &self.entity,
                ActorAnimationInterface::ChannelAnimatable,
                &self.occurrence,
            )
            .map_err(ActorAnimationConstructionError::Registry)?;
        self.channel = Some(ActorChannelAnimatable::new());
        self.phase = ActorAnimationPhase::RegisteringSkaterAnim;
        registry
            .register_animation_interface(
                &self.entity,
                ActorAnimationInterface::SkaterAnim,
                &self.occurrence,
            )
            .map_err(ActorAnimationConstructionError::Registry)?;
        self.skater = Some(ActorSkaterAnimationPrefix::new(&self.inputs.physics_world));
        if self.sim_allocation_cleared {
            let trajectory = &mut self.skater.as_mut().expect("constructed prefix").trajectory;
            for foot in trajectory
                .input_feet
                .iter_mut()
                .chain(&mut trajectory.physical_feet)
            {
                foot.trailing_action_probe_word = Some(0);
            }
        }
        self.phase = ActorAnimationPhase::QueryingPlayerIndex;
        let player_index = registry
            .actor_player_index(&self.entity)
            .map_err(ActorAnimationConstructionError::Registry)?;
        // 82cc0e44/54 preserve only 0x039fffff from the unknown old word.
        // 82cc0e4c inserts the bool in bit27. No other unknown bit becomes zero.
        self.player = Some(ActorAnimationPlayerFields {
            player_index,
            natural_stance: 1,
            last_stance: 1,
            relative_stance: 0,
            spawn_relative_stance: 0,
            opacity: 1.0,
            flags_known_mask: !0x039f_ffff,
            flags_bits: u32::from(self.inputs.use_low_blend_cull_threshold) << 27,
            gestures: [15, 12, 13, 14],
            pro_skater: FastString30::encode("Default")
                .expect("source Default literal fits the existing FastString encoder"),
        });
        self.phase = ActorAnimationPhase::VaultRequired;
        Ok(())
    }
    pub(crate) const fn phase(&self) -> ActorAnimationPhase {
        self.phase
    }
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    pub(crate) fn entity(&self) -> &MotionGraphEntityAuthority<K> {
        &self.entity
    }
    pub(crate) fn inputs(&self) -> &ActorAnimationInputs<W> {
        &self.inputs
    }
    pub(crate) fn base(&self) -> Option<&ActorAnimatableBase> {
        self.base.as_ref()
    }
    pub(crate) fn channel(&self) -> Option<&ActorChannelAnimatable> {
        self.channel.as_ref()
    }
    /// StepSelector retains this actual input/future storage through Launch/Update.
    pub(crate) fn step_storage_mut(
        &mut self,
    ) -> Option<(&mut [AnimationStepInput; 2], &mut [AnimationStepResult; 2])> {
        let skater = self.skater.as_mut()?;
        Some((&mut skater.step_inputs, &mut skater.step_results))
    }
    pub(crate) fn skater(&self) -> Option<&ActorSkaterAnimationPrefix<W>> {
        self.skater.as_ref()
    }
    /// DoAirDismountRevert82cb7738: frames store precedes request bit.
    pub(crate) fn try_request_air_dismount_revert(
        &mut self,
        frames: i32,
    ) -> Result<(), crate::actor_animation_construction::gesture_writes::GestureAnimationWriteError>
    {
        use gesture_writes::GestureAnimationWriteError as E;
        let player = self.player.as_mut().ok_or(E::MissingInitialization)?;
        let before_sprint = self
            .before_sprint
            .as_mut()
            .ok_or(E::MissingInitialization)?;
        before_sprint.air_dismount_revert_frames = frames as u32;
        player.flags_bits |= 0x400;
        Ok(())
    }
    pub(crate) fn player(&self) -> Option<&ActorAnimationPlayerFields> {
        self.player.as_ref()
    }

    /// `SkaterAnim::SetLanding82cb7690` writes only bit 0x8000 of +0xe4c.
    pub(crate) fn try_set_obair_landing_bit(&mut self, landing: bool) -> Result<(), &'static str> {
        let player = self.player.as_mut().ok_or("SkaterAnim player fields")?;
        player.flags_bits = (player.flags_bits & !0x8000) | if landing { 0x8000 } else { 0 };
        player.flags_known_mask |= 0x8000;
        Ok(())
    }

    /// `SkaterAnim::EnableAccelDecel` 0x82cb7750 writes only bit 0x10 of +0xe4c.
    pub(crate) fn try_enable_accel_decel(&mut self, enable: bool) -> Result<(), &'static str> {
        let player = self.player.as_mut().ok_or("SkaterAnim player fields")?;
        player.flags_bits = (player.flags_bits & !0x10) | if enable { 0x10 } else { 0 };
        player.flags_known_mask |= 0x10;
        Ok(())
    }

    /// `SkaterAnim::SetSpawnFootForwardStance`82cb7868 (Animatable-base vtable
    /// +0xc0) writes +0x6144 from natural stance +0x6138: RIGHT (zero) spawns
    /// relative stance 0 for natural 1 and 1 otherwise; any other foot spawns
    /// 1 for natural 1 and 0 otherwise.
    pub(crate) fn try_set_spawn_foot_forward_stance(
        &mut self,
        foot_forward: u32,
    ) -> Result<(), &'static str> {
        let player = self.player.as_mut().ok_or("SkaterAnim player fields")?;
        let natural_one = player.natural_stance == 1;
        player.spawn_relative_stance = if foot_forward == 0 {
            u32::from(!natural_one)
        } else {
            u32::from(natural_one)
        };
        Ok(())
    }

    /// GetFootForward82cb78a0: stance before Actor's separate fakie modifier.
    pub(crate) fn try_foot_forward(&self) -> Result<u32, &'static str> {
        let player = self.player.as_ref().ok_or("SkaterAnim player fields")?;
        Ok(u32::from(
            (player.natural_stance == 0 && player.relative_stance == 0)
                || (player.natural_stance == 1 && player.relative_stance == 1),
        ))
    }

    /// `SkaterAnim::IsCurrentGrabType82cb77a0` compares the word at +0xee8.
    pub(crate) fn try_grab_type(&self) -> Result<u32, &'static str> {
        Ok(self
            .before_sprint
            .as_ref()
            .ok_or("SkaterAnim grab type fields")?
            .grab_type)
    }

    /// `SkaterAnim::SetGrabType82cb7798` stores the full word at +0xee8.
    pub(crate) fn try_set_grab_type(&mut self, grab: u32) -> Result<(), &'static str> {
        self.before_sprint
            .as_mut()
            .ok_or("SkaterAnim grab type fields")?
            .grab_type = grab;
        Ok(())
    }

    /// Actual FindCollection/Instance 82cc0ef4/0f04, through fields preceding
    /// `SetupFacePose` 82cc1190. The loaded Vault is the existing real service;
    /// no generic default fallback or early cached-layout data read is added.
    /// The instance owns the SAME Vault but is separate from every physics
    /// instance. Native mutable refcount/source flags stay outside this loaded
    /// immutable Vault projection's previously documented scope.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "caller transfers its Vault handle into the retained constructor instance lifetime"
    )]
    pub(crate) fn try_construct_vault_suffix<C: ActorAnimationCvars>(
        &mut self,
        vault: Arc<AttributeRuntimeVault>,
        cvars: &mut C,
    ) -> Result<(), ActorAnimationConstructionError<C::Error>> {
        if self.phase != ActorAnimationPhase::VaultRequired {
            return Err(ActorAnimationConstructionError::VaultPrefixRequired);
        }
        self.offboard_attributes = Some(vault.instance(
            ACTOR_ANIMATION_OFFBOARD_CLASS,
            ACTOR_ANIMATION_OFFBOARD_COLLECTION,
        ));
        // Immutable projection of actual Global+16c constructor828b31d8.
        self.locomotion_global_attributes =
            Some(vault.instance(0x90c3_decf_0105_631e, 0x6c43_1580_a70f_4281));
        // Native missing collection is legal here; no null assertion follows.
        self.before_sprint = Some(ActorAnimationBeforeSprint::new());
        let player = self
            .player
            .as_mut()
            .expect("VaultRequired retains player fields");
        // 0f80/0f88 insert rotated 0x4001 into low21 and bits25..23.
        // Combined with earlier 0e78, the entire first word is now known.
        let assigned_mask = 0x039f_ffff;
        player.flags_bits = (player.flags_bits & !assigned_mask) | 0x0004_0010;
        player.flags_known_mask |= assigned_mask;
        self.phase = ActorAnimationPhase::SamplingSprintCvar;
        let sprint = cvars
            .use_sprint_button()
            .map_err(ActorAnimationConstructionError::Cvar)?;
        let speed_clamp = if sprint == 0 {
            1.0
        } else {
            f32::from_bits(0x3f4c_cccd)
        };
        // Original gI/gJ/gK at 8227e8f0/e900/e910 have W0; row3 is v127=0.
        let affine = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0; 4],
        ];
        // Reset the same already constructed trajectory, not a replacement.
        let trajectory = &mut self
            .skater
            .as_mut()
            .expect("VaultRequired retains SkaterAnim prefix")
            .trajectory;
        for (input, physical) in trajectory
            .input_feet
            .iter_mut()
            .zip(&mut trajectory.physical_feet)
        {
            input.reset();
            physical.reset();
        }
        trajectory.physical_foot_index = 0;
        trajectory.first_update = true;
        trajectory.foot_locked = 0;
        trajectory.time = 0.0;
        trajectory.current_state = -1;
        trajectory.raw_time = 0.0;
        let before_sprint = self
            .before_sprint
            .as_mut()
            .expect("preceding suffix fields retained");
        before_sprint.foot_down = -1;
        before_sprint.last_foot_down = -1;
        self.before_face = Some(ActorAnimationBeforeFace {
            speed_clamp,
            desired_speed: 0.0,
            sprint_level: 0.0,
            time_to_foot_change: 0.0,
            time_since_leaving_locomotion: 0.0,
            turn: 0.0,
            world_position: crate::skeleton_root_transforms::SkeletonRootMatrix::from_columns(
                affine.map(WalkVector::known),
            ),
            extra_y_rotation: affine,
            applied_extra_y_rotation: affine,
            collision_normal_timer: 0.0,
            step_turn_adjustment: 0.0,
            current_stage: -1,
            blend_space_entry_point: AnimationPointerState::Null,
            feet: [WalkVector::known([0.0; 4]); 2],
            queried_feet: [WalkVector::known([0.0; 4]); 2],
            last_centre_of_mass: WalkVector::known([0.0; 4]),
            collision_displacement: WalkVector::known([0.0; 4]),
            collision_trajectory_displacement: WalkVector::known([0.0; 4]),
            last_collision_normal: WalkVector::known([0.0; 4]),
            collision_response_turn: None,
        });
        self.phase = ActorAnimationPhase::FaceSetupRequired;
        Ok(())
    }
    pub(crate) fn offboard_attributes(&self) -> Option<&AttributeRuntimeInstance> {
        self.offboard_attributes.as_ref()
    }
    pub(crate) fn before_sprint(&self) -> Option<&ActorAnimationBeforeSprint> {
        self.before_sprint.as_ref()
    }
    pub(crate) fn before_face(&self) -> Option<&ActorAnimationBeforeFace> {
        self.before_face.as_ref()
    }

    pub(crate) fn face(&self) -> Option<&ActorFacePlayback> {
        self.face.as_ref()
    }

    /// Selected loaded/non-null `SetupFacePose` 82cbf700. The original recipe
    /// load must publish its actual holder before this continuation is called.
    pub(crate) fn try_finish_loaded_face(
        &mut self,
        faces: &CharacterFaceManager,
    ) -> Result<(), ActorAnimationConstructionError<std::convert::Infallible>> {
        self.try_finish_loaded_face_with_commands(faces, |_, _, _| Ok(()))
    }

    pub(crate) fn try_finish_loaded_face_with_source<
        S: crate::simulation_animation_startup::AnimationStartupSource,
    >(
        &mut self,
        faces: &CharacterFaceManager,
        source: &mut S,
    ) -> Result<
        (),
        ActorAnimationConstructionError<
            crate::simulation_animation_startup::AnimationStartupError<S::Error>,
        >,
    > {
        self.try_finish_loaded_face_with_commands(faces, |managers, commands, setup| {
            let hierarchy =
                Arc::clone(&commands.lock().map_err(|_| ActorFaceError::Lock)?.database);
            let scheduler = Arc::clone(
                &managers
                    .command
                    .lock()
                    .map_err(|_| ActorFaceError::Lock)?
                    .scheduler,
            );
            let buffered = Arc::new(Mutex::new(
                initialization::bind_pose::BufferedBodyCommands::new(
                    hierarchy,
                    Arc::clone(&managers.database),
                    scheduler,
                ),
            ));
            commands.lock().map_err(|_| ActorFaceError::Lock)?.buffered =
                Some(Arc::clone(&buffered));
            buffered
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .try_construct(setup, source)
        })
    }

    fn try_finish_loaded_face_with_commands<E>(
        &mut self,
        faces: &CharacterFaceManager,
        construct: impl FnMut(
            &crate::actor_face_playback::FaceAnimationManagers,
            &Arc<Mutex<crate::actor_face_playback::FaceCommands>>,
            &crate::actor_face_playback::AnimationCommandSetup,
        ) -> Result<(), E>,
    ) -> Result<(), ActorAnimationConstructionError<E>> {
        if self.phase != ActorAnimationPhase::FaceSetupRequired {
            return Err(ActorAnimationConstructionError::FacePrefixRequired);
        }
        let name = self
            .inputs
            .face_name
            .as_ref()
            .ok_or(ActorAnimationConstructionError::FaceManagerRequired)?;
        if !faces.has_face_data(name) {
            return Err(ActorAnimationConstructionError::FaceManagerRequired);
        }
        let managers = self
            .face_managers
            .as_ref()
            .ok_or(ActorAnimationConstructionError::FaceManagerRequired)?
            .clone();
        let player = self
            .player
            .as_ref()
            .expect("player fields precede setup")
            .player_index;
        let face = ActorFacePlayback::new(name, player, managers);
        let skater = self.skater.as_mut().expect("skater prefix precedes setup");
        skater.face_pose_name = face.face_name;
        skater.face_playback_name = face.playback_name;
        skater.database_manager = AnimationPointerState::Constructed;
        skater.playback_manager = AnimationPointerState::Constructed;
        skater.command_manager = AnimationPointerState::Constructed;
        self.face = Some(face);
        self.phase = ActorAnimationPhase::FaceSetupRunning;
        let face = self.face.as_mut().expect("retained face before setup");
        let result = face.try_setup_with_commands(name, faces, construct);
        if face.playback.is_some() {
            skater.face_playback = AnimationPointerState::Constructed;
        }
        if !face
            .local_pose
            .is_empty()
            .map_err(ActorAnimationConstructionError::Face)?
        {
            skater.hand_and_face_poses[2] = AnimationPointerState::Constructed;
            skater.hand_and_face_hierarchies[2] = AnimationPointerState::Constructed;
        }
        result.map_err(|error| match error {
            crate::actor_face_playback::FaceSetupError::Face(error) => {
                ActorAnimationConstructionError::Face(error)
            }
            crate::actor_face_playback::FaceSetupError::Commands(error) => {
                ActorAnimationConstructionError::FaceCommands(error)
            }
        })?;
        // SetupFacePose82cbfb50 writes mInitFace only after all fixed frames.
        let player = self.player.as_mut().expect("player flags precede setup");
        player.flags_bits |= 0x0080_0000;
        player.flags_known_mask |= 0x0080_0000;
        self.phase = ActorAnimationPhase::ConstructorReturned;
        Ok(())
    }

    /// `SkaterAnim::PurgeFacePose` 0x82ca9b38 for a skater whose face pose was set up.
    pub(crate) fn try_purge_face_pose(&mut self) -> Result<(), ActorFaceError> {
        self.face.as_mut().map_or(Ok(()), ActorFacePlayback::try_purge)
    }

    /// `SetupFacePose` 82cbf728/72c branches straight to its epilogue for an
    /// actual null argument, before reading ANY face manager or mutating fields.
    /// A present empty name is nonnull and still requires the actual manager.
    /// This closes only this selected constructor call; Actor publication and
    /// subsequent body/hand Initialize remain the caller's separate stages.
    pub(crate) fn try_finish_null_face(
        &mut self,
    ) -> Result<(), ActorAnimationConstructionError<std::convert::Infallible>> {
        if self.phase != ActorAnimationPhase::FaceSetupRequired {
            return Err(ActorAnimationConstructionError::FacePrefixRequired);
        }
        if self.inputs.face_name.is_some() {
            return Err(ActorAnimationConstructionError::FaceManagerRequired);
        }
        self.phase = ActorAnimationPhase::ConstructorReturned;
        Ok(())
    }
}

#[path = "../trees/actor_animation_blend_space_dispatch.rs"]
mod blend_space_dispatch;

#[path = "../locomotion/actor_animation_locomotion_turn.rs"]
pub(crate) mod locomotion_turn;
#[path = "../../session/edit_skater_player_attributes.rs"]
pub(crate) mod edit_skater_attributes;
