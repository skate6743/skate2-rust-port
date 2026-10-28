//! Retained original `SkaterPresentation::InitAnimation827e1c08` owners.
//! Draft child of `actor_face_playback`; shared ACS amendments are gate coordinated.
use super::{
    ActorFaceError, AnimationCommandSetup, AnimationPoseRef, FaceAnimationManagers,
    FaceCommandPhase, FaceCommands, FaceDatabase, FacePlaybackData,
};
use crate::{
    actor_animation_construction::{
        AnimationPointerState,
        hands_face::{
            HandFaceStreamCommands,
            commands::StreamCommands,
            runtime::{self, HandFaceRuntimeError},
        },
        initialization::{
            ActorAnimationInitializeSource,
            bind_pose::{
                BindPoseError, BufferedBodyCommands, OwnedAnimationPoseBuffer,
                sqt_output::OwnedAnimationSqtBuffer,
            },
        },
    },
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_graph_construction::ActorGraphIntents,
    simulation_animation_startup::{AnimationStartupError, AnimationStartupSource},
};
use skate2_formats::andale::{AndaleHierarchy, PackedFastString};
use std::{
    mem::MaybeUninit,
    sync::{Arc, Mutex},
};

#[path = "skater_presentation_final_pose.rs"]
pub(crate) mod final_pose;
#[path = "skater_presentation_offsets.rs"]
pub(crate) mod offsets;
#[path = "skater_presentation_read.rs"]
pub(crate) mod read;
#[path = "skater_presentation_restore.rs"]
pub(crate) mod restore;
#[path = "skater_presentation_startup.rs"]
pub(crate) mod startup;
use offsets::{PresentationOffsets, PresentationOffsetsError};

#[derive(Debug)]
pub(crate) enum PresentationAnimationError<S, J> {
    Source(S),
    Startup(AnimationStartupError<S>),
    Dispatch(HandFaceRuntimeError<J>),
    Animation(ActorFaceError),
    Commands(BindPoseError),
    Offsets(PresentationOffsetsError),
    Missing(&'static str),
    AlreadyAttempted,
    Allocation,
}
impl<S: std::fmt::Debug, J: std::fmt::Debug> std::fmt::Display
    for PresentationAnimationError<S, J>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "presentation source: {e:?}"),
            Self::Startup(e) => write!(f, "presentation ACS construction: {e:?}"),
            Self::Dispatch(e) => write!(f, "presentation dispatch: {e:?}"),
            Self::Animation(e) => write!(f, "presentation animation: {e:?}"),
            Self::Commands(e) => write!(f, "presentation commands: {e:?}"),
            Self::Offsets(e) => write!(f, "presentation offsets: {e}"),
            Self::Missing(name) => write!(f, "presentation owner missing: {name}"),
            Self::AlreadyAttempted => f.write_str("presentation initialization already attempted"),
            Self::Allocation => f.write_str("presentation allocation failed"),
        }
    }
}
impl<S, J> From<ActorFaceError> for PresentationAnimationError<S, J> {
    fn from(error: ActorFaceError) -> Self {
        Self::Animation(error)
    }
}
impl<S, J> From<BindPoseError> for PresentationAnimationError<S, J> {
    fn from(error: BindPoseError) -> Self {
        Self::Commands(error)
    }
}
impl<S, J> From<PresentationOffsetsError> for PresentationAnimationError<S, J> {
    fn from(error: PresentationOffsetsError) -> Self {
        Self::Offsets(error)
    }
}

pub(crate) struct PresentationAnimation {
    /// Presentation's actual command/playback managers and shared Andale DB manager.
    pub(crate) managers: FaceAnimationManagers,
    databases: Vec<Arc<Mutex<FaceDatabase>>>,
    pending_commands: Vec<Arc<Mutex<FaceCommands>>>,
    pub(crate) full: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) face: Option<Arc<Mutex<FacePlaybackData>>>,
    pub(crate) bind: Option<OwnedAnimationSqtBuffer>,
    pub(crate) replay: Vec<OwnedAnimationSqtBuffer>,
    pub(crate) interpolated: Vec<OwnedAnimationPoseBuffer>,
    pub(crate) save_pose: Option<OwnedAnimationPoseBuffer>,
    pub(crate) hips_transforms: Arc<Mutex<[Option<crate::animation::AnimationMatrix>; 9]>>,
    pub(crate) hips: Option<i32>,
    pub(crate) skateboard: Option<i32>,
    /// The hierarchy's `Head`, `LeftFoot` and `RightFoot`, every skater's alike: the body points
    /// another online player's scoring takes of a skater shown from its replay frames (PC port
    /// addition; the original read them of the simulated skeleton).
    pub(crate) body: Option<[usize; 3]>,
    pub(crate) twist_bones: [Option<i32>; 14],
    pub(crate) offsets: Option<PresentationOffsets>,
    pub(crate) ghost_next: Vec<MaybeUninit<u8>>,
    pub(crate) ghost_previous: Vec<MaybeUninit<u8>>,
    pub(crate) initialized: bool,
    attempted: bool,
    owns_shared_resources: bool,
}
impl PresentationAnimation {
    pub(crate) fn new(managers: FaceAnimationManagers) -> Self {
        Self {
            managers,
            databases: vec![],
            pending_commands: vec![],
            full: None,
            face: None,
            bind: None,
            replay: vec![],
            interpolated: vec![],
            save_pose: None,
            hips_transforms: Arc::new(Mutex::new([None; 9])),
            hips: None,
            skateboard: None,
            body: None,
            twist_bones: [None; 14],
            offsets: None,
            ghost_next: vec![],
            ghost_previous: vec![],
            initialized: false,
            attempted: false,
            owns_shared_resources: true,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "native startup order preserves partial owner construction"
    )]
    pub(crate) fn try_initialize<S: AnimationStartupSource, J: ActorAnimationInitializeSource>(
        &mut self,
        source: &mut S,
        jobs: &mut J,
    ) -> Result<(), PresentationAnimationError<S::Error, J::Error>> {
        if self.attempted {
            return Err(PresentationAnimationError::AlreadyAttempted);
        }
        self.attempted = true;
        let full = self.load_database(source, "data\\anim/SkaterPresentationDB.abin")?;
        let full_hierarchy = Arc::new(hierarchy(&full)?);
        self.hips = Some(native_bone(&full_hierarchy, "hips")?);
        self.skateboard = Some(native_bone(&full_hierarchy, "skateboard_root")?);
        let body = [native_bone(&full_hierarchy, "Head")?, native_bone(&full_hierarchy, "LeftFoot")?, native_bone(&full_hierarchy, "RightFoot")?];
        self.body = body.iter().all(|&bone| bone >= 0).then(|| body.map(|bone| usize::try_from(bone).unwrap_or(usize::MAX)));
        let playback =
            self.register_playback(source, &full, b"SkaterACS", b"SkaterFull", &full_setup())?;
        self.full = Some(Arc::clone(&playback));
        let count = full_hierarchy.bone_names.len();
        self.bind = Some(OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::clone(
            &full_hierarchy,
        )));
        for _ in 0..18 {
            self.replay
                .push(OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::clone(
                    &full_hierarchy,
                )));
        }
        for _ in 0..9 {
            self.interpolated
                .push(OwnedAnimationPoseBuffer::allocate_unknown(
                    Arc::clone(&full),
                    count,
                )?);
        }
        self.save_pose = Some(OwnedAnimationPoseBuffer::allocate_unknown(
            Arc::clone(&full),
            count,
        )?);
        let commands = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.commands);
        // Native GetPose2070 precedes BeginCommandSet20a4.
        let pose = playback
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .get_pose(ActorGraphIntents::native_name(b"rig_Tpose"))?
            .ok_or(BindPoseError::MissingBindPose)?;
        runtime::begin_command_set(&commands)?;
        emit_pose(
            &playback,
            &pose,
            self.bind.as_ref().expect("allocated bind").clone(),
        )?;
        runtime::end_command_set(&commands, jobs).map_err(PresentationAnimationError::Dispatch)?;
        let names = [
            "LeftArm",
            "LeftArmTwist",
            "LeftForeArm",
            "LeftForeArmTwist",
            "LeftForeArmTwist1",
            "LeftUpLeg",
            "LeftUpLegTwist",
            "RightArm",
            "RightArmTwist",
            "RightForeArm",
            "RightForeArmTwist",
            "RightForeArmTwist1",
            "RightUpLeg",
            "RightUpLegTwist",
        ];
        // Native publishes each lookup immediately (first store827e21b0).
        for (destination, name) in self.twist_bones.iter_mut().zip(names) {
            *destination = Some(native_bone(&full_hierarchy, name)?);
        }
        runtime::sync_command_set(&commands, jobs).map_err(PresentationAnimationError::Dispatch)?;

        let face = self.load_database(source, "data\\anim/SkaterFacePres.abin")?;
        self.face = Some(self.register_playback(
            source,
            &face,
            b"FacePres",
            b"SkaterFace",
            &face_setup(),
        )?);
        let temporary_start = self.databases.len();
        let face_sim = self.load_database(source, "data\\anim/SkaterFaceSimPres.abin")?;
        let left = self.load_database(source, "data\\anim/SkaterLeftHandPres.abin")?;
        let right = self.load_database(source, "data\\anim/SkaterRightHandPres.abin")?;
        let body = self.load_database(source, "data\\anim/SkaterBodyPres.abin")?;
        let offsets = PresentationOffsets::try_initialize(
            &hierarchy(&body)?,
            &hierarchy(&left)?,
            &hierarchy(&right)?,
            &hierarchy(&face_sim)?,
        )?;
        if std::env::var_os("SKATE2_TRACE_PRESENTATION_SIZE").is_some() {
            eprintln!(
                "skater presentation pose: {} matrix quaternions, {} quantized quaternions, {} translations, {} bytes",
                offsets.matrix_rotations.len(),
                offsets.quaternion3_count,
                offsets.translation_count,
                offsets.pose_bytes,
            );
        }
        self.ghost_next
            .try_reserve_exact(offsets.pose_bytes)
            .map_err(|_| PresentationAnimationError::Allocation)?;
        self.ghost_next
            .resize(offsets.pose_bytes, MaybeUninit::uninit());
        self.ghost_previous
            .try_reserve_exact(offsets.pose_bytes)
            .map_err(|_| PresentationAnimationError::Allocation)?;
        self.ghost_previous
            .resize(offsets.pose_bytes, MaybeUninit::uninit());
        self.offsets = Some(offsets);
        // Native destructors2d3c/4c/5c/6c remove these slots, retaining global part names.
        for database in [&face_sim, &left, &right, &body] {
            self.release_temporary(database)?;
        }
        self.databases.truncate(temporary_start);
        self.initialized = true;
        Ok(())
    }

    /// Presentation::Init827f10c0 owns one SkaterPresentation::Init827e4098.
    /// InitAnimation827e1c08 creates SkaterFull/SkaterFace once, with nine ACS
    /// streams. Per-entity port wrappers retain separate pose buffers while
    /// borrowing those global databases and command/playback managers.
    pub(crate) fn try_for_entity<S, J>(&self) -> Result<Self, PresentationAnimationError<S, J>> {
        if !self.initialized {
            return Err(PresentationAnimationError::Missing("shared InitAnimation"));
        }
        let full = self.full.as_ref().ok_or(PresentationAnimationError::Missing("SkaterFull"))?;
        let database = Arc::clone(&full.lock().map_err(|_| ActorFaceError::Lock)?.database);
        let hierarchy = Arc::new(hierarchy(&database)?);
        let mut entity = Self::new(self.managers.clone());
        entity.owns_shared_resources = false;
        entity.full = self.full.clone();
        entity.face = self.face.clone();
        entity.bind = self.bind.clone();
        entity.hips_transforms = Arc::clone(&self.hips_transforms);
        entity.hips = self.hips;
        entity.skateboard = self.skateboard;
        entity.twist_bones = self.twist_bones;
        entity.offsets = self.offsets.clone();
        for _ in 0..18 {
            entity.replay.push(OwnedAnimationSqtBuffer::allocate_for_hierarchy(Arc::clone(&hierarchy)));
        }
        for _ in 0..9 {
            entity.interpolated.push(OwnedAnimationPoseBuffer::allocate_unknown(
                Arc::clone(&database), hierarchy.bone_names.len())?);
        }
        entity.save_pose = Some(OwnedAnimationPoseBuffer::allocate_unknown(
            database, hierarchy.bone_names.len())?);
        let bytes = self.offsets.as_ref().ok_or(PresentationAnimationError::Missing("shared offsets"))?.pose_bytes;
        for buffer in [&mut entity.ghost_next, &mut entity.ghost_previous] {
            buffer.try_reserve_exact(bytes).map_err(|_| PresentationAnimationError::Allocation)?;
            buffer.resize(bytes, MaybeUninit::uninit());
        }
        entity.attempted = true;
        entity.initialized = true;
        Ok(entity)
    }

    fn load_database<S: AnimationStartupSource, J>(
        &mut self,
        source: &mut S,
        path: &str,
    ) -> Result<Arc<Mutex<FaceDatabase>>, PresentationAnimationError<S::Error, J>> {
        let database = self
            .managers
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .allocate()?;
        self.databases.push(Arc::clone(&database));
        let bytes = source
            .read_animation(path)
            .map_err(PresentationAnimationError::Source)?;
        self.managers
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .set_content(
                &mut *database.lock().map_err(|_| ActorFaceError::Lock)?,
                bytes,
            )?;
        Ok(database)
    }

    pub(crate) fn register_playback<S: AnimationStartupSource, J>(
        &mut self,
        source: &mut S,
        database: &Arc<Mutex<FaceDatabase>>,
        command_name: &[u8],
        playback_name: &[u8],
        setup: &AnimationCommandSetup,
    ) -> Result<Arc<Mutex<FacePlaybackData>>, PresentationAnimationError<S::Error, J>> {
        let scheduler = Arc::clone(
            &self
                .managers
                .command
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .scheduler,
        );
        let buffered = Arc::new(Mutex::new(BufferedBodyCommands::new(
            Arc::clone(database),
            Arc::clone(&self.managers.database),
            scheduler,
        )));
        let commands = Arc::new(Mutex::new(FaceCommands {
            database: Arc::clone(database),
fixed_frames: vec![],
            fixed_matrices: Vec::new(),
            max_frames: 0,
            phase: FaceCommandPhase::Ready,
            scratch: None,
            selected_frames: vec![],
            setup: Some(setup.clone()),
            buffered: Some(Arc::clone(&buffered)),
        }));
        self.pending_commands.push(Arc::clone(&commands));
        buffered
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .try_construct(setup, source)
            .map_err(PresentationAnimationError::Startup)?;
        commands
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .setup
            .as_mut()
            .expect("stored setup")
            .fetch_time_error = f32::from_bits(0x3ada_1019);
        self.managers
            .command
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .insert(
                (ActorGraphIntents::native_name(command_name), 0),
                Arc::clone(&commands),
            );
        let playback = Arc::new(Mutex::new(FacePlaybackData {
            database: Arc::clone(database),
            databases: vec![Arc::clone(database)],
            commands,
        }));
        let mut manager = self
            .managers
            .playback
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        let key = (ActorGraphIntents::native_name(playback_name), 0);
        if manager.entries.contains_key(&key) {
            return Err(ActorFaceError::DuplicatePlayback.into());
        }
        manager.entries.insert(key, Arc::clone(&playback));
        Ok(playback)
    }

    /// `SkaterPresentation::UnInitAnimation` 0x827e2d90: the `SkaterACS` and `FacePres`
    /// command systems and the `SkaterFull` and `SkaterFace` playbacks go, then the
    /// `SkaterPresentationDB` and `FacePres` databases.
    pub(crate) fn try_uninit(&mut self) -> Result<(), ActorFaceError> {
        if !self.initialized {
            return Ok(());
        }
        if !self.owns_shared_resources {
            self.full = None;
            self.face = None;
            self.initialized = false;
            return Ok(());
        }
        {
            let mut commands = self.managers.command.lock().map_err(|_| ActorFaceError::Lock)?;
            for name in [b"SkaterACS".as_slice(), b"FacePres"] {
                commands.entries.remove(&(ActorGraphIntents::native_name(name), 0));
            }
        }
        {
            let mut playbacks = self.managers.playback.lock().map_err(|_| ActorFaceError::Lock)?;
            for name in [b"SkaterFull".as_slice(), b"SkaterFace"] {
                playbacks.entries.remove(&(ActorGraphIntents::native_name(name), 0));
            }
        }
        self.full = None;
        self.face = None;
        let mut manager = self.managers.database.lock().map_err(|_| ActorFaceError::Lock)?;
        for database in self.databases.drain(..) {
            manager.remove(&database)?;
        }
        self.initialized = false;
        Ok(())
    }

    fn release_temporary(&self, database: &Arc<Mutex<FaceDatabase>>) -> Result<(), ActorFaceError> {
        self.managers
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .remove(database)
    }
}

fn hierarchy(database: &Arc<Mutex<FaceDatabase>>) -> Result<AndaleHierarchy, ActorFaceError> {
    database
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .parsed
        .as_ref()
        .map(|parsed| parsed.hierarchy.clone())
        .ok_or(ActorFaceError::MissingRecord)
}
/// `GetBone` at82590fc0 returns signed -1 when the name is absent.
fn native_bone(hierarchy: &AndaleHierarchy, name: &str) -> Result<i32, ActorFaceError> {
    let packed = PackedFastString::<5>::encode(name)?;
    hierarchy
        .packed_bone_names
        .iter()
        .position(|bone| *bone == packed)
        .map(i32::try_from)
        .transpose()
        .map_err(|_| ActorFaceError::MissingRecord)
        .map(|index| index.unwrap_or(-1))
}
fn emit_pose(
    playback: &Arc<Mutex<FacePlaybackData>>,
    pose: &AnimationPoseRef,
    output: OwnedAnimationSqtBuffer,
) -> Result<(), BindPoseError> {
    let playback = playback.lock().map_err(|_| ActorFaceError::Lock)?;
    let mut commands = StreamCommands::new(
        Arc::clone(&playback.commands),
        output,
        AnimationPointerState::Null,
    )?;
    commands.begin_stream()?;
    commands.fetch_pose(pose)?;
    commands.extract_sqt()?;
    commands.end_stream()
}
fn full_setup() -> AnimationCommandSetup {
    AnimationCommandSetup {
        capacities: [13, 6, 3, 1, 2],
        averages: [6.0, 3.0, 1.0, 1.0, 2.0],
        max_nblend: 0,
        fixed_sqt: 0,
        fixed_matrix: 0,
        mode: 1,
        fetch_mode: 1,
        category: 2,
        characters: 9,
        truncation_offset: 0.5,
        fetch_time_error: 0.0,
        trajectory: true,
        codec: 0,
    }
}
fn face_setup() -> AnimationCommandSetup {
    AnimationCommandSetup {
        capacities: [2, 1, 1, 0, 0],
        averages: [1.0, 0.5, 0.5, 0.0, 0.0],
        max_nblend: 0,
        fixed_sqt: 0,
        fixed_matrix: 0,
        mode: 1,
        fetch_mode: 0,
        category: 2,
        characters: 9,
        truncation_offset: 0.5,
        fetch_time_error: 0.0,
        trajectory: false,
        codec: 0,
    }
}

#[path = "skater_presentation_corrections.rs"]
pub(crate) mod corrections;
#[path = "skater_presentation_expressions.rs"]
pub(crate) mod expressions;
#[path = "skater_presentation_frame.rs"]
pub(crate) mod frame;
#[path = "skater_presentation_move_root.rs"]
pub(crate) mod move_root;
#[path = "skater_presentation_twist.rs"]
pub(crate) mod twist;
#[path = "skater_presentation_write.rs"]
pub(crate) mod write;
