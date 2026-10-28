//! Retained selected same-frame presentation composition, using existing owners.
//! Model membership and whole-manager scheduling remain caller admission.
use super::super::{
    ActorFaceError,
    corrections::CorrectionError,
    final_pose::PresentationFinalPoseError,
    move_root::matrix_inverse::PresentationMatrixNumeric,
    read::{PresentationReadScratch, PresentationReadSettings, decode::PresentationDecodeNumeric},
    startup::SelectedPresentationStartup,
};
use super::{
    ActorPresentationFrameSource, PresentationFrameError, RootPositionWTransport,
    SelectedPresentationFrameProducer, SubmittedPresentationFrame, buffer::PresentationBufferError,
    read_pass::PresentationReadPassError,
};
use crate::{
    actor_animation_construction::initialization::{
        ActorAnimationInitializeSource,
        bind_pose::{BindPoseError, sqt_output::OwnedAnimationSqtBuffer},
    },
    animation::AnimationMatrix,
    character::{
        CharacterHierarchy, CharacterScene,
        palette::{CharacterPalette, CharacterPaletteError},
    },
    game_input_command::GameInputCommandNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
use skate2_formats::render::AxisAlignedBox;
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};
#[path = "skater_presentation_render_opacity.rs"]
pub(crate) mod render_opacity;

const MATERIAL_LOCK: crate::character::material_admission::CharacterMaterialError =
    crate::character::material_admission::CharacterMaterialError::Missing("unpoisoned material lock");

#[derive(Clone, Copy)]
pub(crate) struct SelectedPresentationPolicy {
    pub(crate) lod: u8,
    pub(crate) submission_capacity: usize,
    pub(crate) gather_capacity: usize,
    pub(crate) provider: u32,
    pub(crate) root_w: RootPositionWTransport,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectedPresentationStage {
    Produced,
    Decoded,
    Corrected,
    Finalized,
    Published,
}
#[derive(Debug)]
pub(crate) enum SelectedPresentationConstructionError {
    Missing(&'static str),
    Owner(ActorFaceError),
    Palette(CharacterPaletteError),
    Buffer(PresentationBufferError),
}
impl std::fmt::Display for SelectedPresentationConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "selected presentation missing {s}"),
            Self::Owner(e) => write!(f, "selected presentation owner: {e:?}"),
            Self::Palette(e) => e.fmt(f),
            Self::Buffer(e) => e.fmt(f),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SelectedPresentationError<N, C, R, J, O> {
    ForeignPresentation,
    Material(crate::character::material_admission::CharacterMaterialError),
    ClothNumeric(N),
    Missing(&'static str),
    Commands(BindPoseError),
    Produce(PresentationFrameError<N, PresentationBufferError>),
    Frame(PresentationFrameError<Infallible, PresentationBufferError>),
    ReplayOverride(ActorFaceError),
    HipsTransform(ActorFaceError),
    Decode(PresentationReadPassError<N, C, J>),
    Correct(CorrectionError<N, R>),
    Finalize(PresentationFinalPoseError<J>),
    Palette(CharacterPaletteError),
    Observe {
        stage: SelectedPresentationStage,
        error: O,
    },
}
impl<
    N: std::fmt::Display + std::fmt::Debug,
    C: std::fmt::Debug,
    R: std::fmt::Debug,
    J: std::fmt::Debug,
    O: std::fmt::Debug,
> std::fmt::Display for SelectedPresentationError<N, C, R, J, O>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignPresentation => {
                f.write_str("selected runtime belongs to another presentation allocation")
            }
            Self::Material(e) => e.fmt(f),
            Self::ClothNumeric(e) => write!(f,"cloth transform inverse: {e}"),
            Self::Missing(s) => write!(f, "selected runtime missing {s}"),
            Self::Commands(e) => write!(f, "selected runtime commands: {e:?}"),
            Self::Produce(e) => e.fmt(f),
            Self::Frame(e) => e.fmt(f),
            Self::ReplayOverride(e) => write!(f, "selected replay override: {e:?}"),
            Self::HipsTransform(e) => write!(f, "selected saved Hips transform: {e:?}"),
            Self::Decode(e) => e.fmt(f),
            Self::Correct(e) => e.fmt(f),
            Self::Finalize(e) => e.fmt(f),
            Self::Palette(e) => e.fmt(f),
            Self::Observe { stage, error } => {
                write!(f, "selected presentation observer {stage:?}: {error:?}")
            }
        }
    }
}
/// The original gather allocations, scratch, model palette and replay handles.
/// No copied pose, independent animation manager or inferred model membership.
pub(crate) struct SelectedPresentationRuntime {
    policy: SelectedPresentationPolicy,
    producer: SelectedPresentationFrameProducer,
    quaternions: Vec<[f32; 4]>,
    translations: Vec<[f32; 4]>,
    palette: CharacterPalette,
    origin: Arc<Mutex<[Option<AnimationMatrix>; 9]>>,
    base: OwnedAnimationSqtBuffer,
    replay: OwnedAnimationSqtBuffer,
    published: bool,
    published_peer: Option<i64>,
    cloth_tick:Option<u32>,
    /// The presentation's restore steps the cloth each frame.
    cloth_by_presentation: bool,
    published_root_position: Option<[f32; 3]>,
    /// `SkaterPresentation::GetSkaterRoot` 0x826c71f8 for the published frame.
    published_root: Option<[f32; 16]>,
    /// `SkaterPresentation::GetHipsPos` 0x827e3ef0 for the published frame.
    published_hips_position: Option<[f32; 3]>,
    published_hips_forward: Option<[f32; 3]>,
    /// The saved Hips the published frame packed with. `DoSimRestore` 0x827e7158 reads them
    /// from this entity's own slot (`mIndex` +0x1E0, times 0xB0); every port entity shares one
    /// `hips_transforms` row, which holds the last skater's once a tick has published them all.
    published_hips: Option<AnimationMatrix>,
    /// The scene character's `mHipsPos` while a NIS borrows the entity.
    borrowed_hips_position: Option<[f32; 3]>,
    /// SkaterPresEntity `m_localBB` (+0xE0): the LOD 0 union from
    /// `CalculateBoundsInfo`, both y lanes lowered by 1 in the constructor.
    local_bounds: AxisAlignedBox,
    /// `m_BB` (+0xC0) for the published frame, min then max.
    published_bounds: Option<[[f32; 3]; 2]>,
    /// Shared with the presentation, which draws with them between ticks.
    materials: Option<Arc<Mutex<crate::character::material_admission::CharacterMaterialAdmission>>>,
    /// This frame's restored header opacity/flags and hips/up matrices.
    render_inputs: Option<render_opacity::RenderOpacityInputs>,
    /// The retained SkaterPresEntity +0x870 lane x and +0x61a.
    render_opacity: render_opacity::SkaterRenderOpacity,
    /// GlobalAttributeCollection+0x198 `rendering_skaters`, bound on first use.
    rendering_skaters: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    /// The unposed palette a presentation restore poses its own copy of.
    palette_template: Arc<CharacterPalette>,
    /// The hierarchy the final pose composes through.
    skeleton: crate::animation::AnimationSkeleton,
    /// What the per-frame restore reads that the submissions do not change.
    restore_rig: Option<Arc<crate::presentation_restore::skater::SkaterRig>>,
    /// This tick's `Data` header and `ReadSkater` pose, before the root move.
    submitted: Option<([u8; crate::presentation_restore::skater::HEADER_BYTES], Vec<skate2_formats::andale::AndaleSqt>)>,
}
impl SelectedPresentationRuntime {
    pub(crate) fn try_new(
        presentation: &SelectedPresentationStartup,
        scene: Arc<CharacterScene>,
        policy: SelectedPresentationPolicy,
    ) -> Result<Self, SelectedPresentationConstructionError> {
        use SelectedPresentationConstructionError as E;
        let animation = &presentation.animation;
        let full = animation
            .full
            .as_ref()
            .ok_or(E::Missing("full animation"))?
            .lock()
            .map_err(|_| E::Owner(ActorFaceError::Lock))?;
        let hierarchy = super::super::hierarchy(&full.database).map_err(E::Owner)?;
        drop(full);
        let mut skeleton = crate::animation::AnimationSkeleton::from_andale(&hierarchy)
            .map_err(|_| E::Missing("presentation hierarchy"))?;
        // The presentation ACS descriptor's trajectory bone (MatrixProgram::execute).
        skeleton.trajectory_bone = Some(0);
        // SkaterPresEntity::SkaterPresEntity 0x827e5740: InitFromRecipe runs
        // CalculateBoundsInfo, then 0x827e5a30..5b2c subtracts 1.0 from the
        // y lane of m_BB.m_Min and m_BB.m_Max and copies both to m_localBB.
        let mut local_bounds = scene
            .level_bounds(0)
            .map_err(|error| E::Palette(CharacterPaletteError::Skinning(error)))?
            .ok_or(E::Missing("LOD 0 model bounds"))?;
        local_bounds.minimum[1] -= 1.0;
        local_bounds.maximum[1] -= 1.0;
        let palette = CharacterPalette::new(
            scene,
            &CharacterHierarchy {
                bone_names: hierarchy.bone_names,
            },
            policy.lod,
        )
        .map_err(E::Palette)?;
        let offsets = animation
            .offsets
            .as_ref()
            .ok_or(E::Missing("InitOffsets"))?;
        let restored = presentation
            .restored
            .as_ref()
            .ok_or(E::Missing("restored base"))?;
        Ok(Self {
            policy,
            palette_template: Arc::new(palette.clone()),
            skeleton,
            restore_rig: None,
            submitted: None,
            palette,
            producer: SelectedPresentationFrameProducer::new_uncompressed_host_policy(
                animation,
                policy.submission_capacity,
                policy.gather_capacity,
                policy.provider,
                policy.root_w,
            )
            .map_err(E::Buffer)?,
            quaternions: scratch(offsets.matrix_rotations.len() + offsets.quaternion3_count)
                .map_err(E::Buffer)?,
            translations: scratch(offsets.translation_count).map_err(E::Buffer)?,
            origin: Arc::clone(&animation.hips_transforms),
            base: restored.next.clone(),
            replay: restored.previous.clone(),
            published: false,
            published_peer: None,
            cloth_tick:None,
            cloth_by_presentation: false,
            published_root_position: None,
            published_root: None,
            published_hips_position: None,
            published_hips_forward: None,
            published_hips: None,
            borrowed_hips_position: None,
            local_bounds,
            published_bounds: None,
            materials: None,
            render_inputs: None,
            render_opacity: render_opacity::SkaterRenderOpacity::constructed(),
            rendering_skaters: None,
        })
    }
    pub(crate) fn palette(&self) -> Result<&CharacterPalette, CharacterPaletteError> {
        if self.published {
            Ok(&self.palette)
        } else {
            Err(CharacterPaletteError::NotPublished)
        }
    }
    pub(crate) fn published_peer(&self)->Option<i64>{self.published_peer}
    pub(crate) fn set_cloth_tick(&mut self,tick:u32){self.cloth_tick=Some(tick);}
    /// `ClothUpdate` runs on the presentation side while the restore draws this skater.
    pub(crate) fn set_cloth_by_presentation(&mut self, presentation: bool) {
        self.cloth_by_presentation = presentation;
    }

    /// Return the CPU clothing stream to the retained game pose after a replay.
    /// DataSysMan::RestoreFrame (0x8277f040) and SkaterPresEntity::Update
    /// (0x827e8568) still run skin jobs while a results/menu screen pauses SIM.
    /// Replay restores use another palette but share these materials, so merely
    /// drawing our game palette would leave the replay's vertices/world rows.
    pub(crate) fn restore_game_cloth(&mut self) -> Result<(), String> {
        if !self.published {
            return Ok(());
        }
        let Some(materials) = self.materials.as_ref() else { return Ok(()) };
        let mut materials = materials.lock().map_err(|_| "return cloth material lock poisoned")?;
        if materials.cloth_disabled() != Some(false) {
            return Ok(());
        }
        let frame = self.cloth_tick.ok_or("return cloth game clock absent")?;
        // A completed game tick may already have republished the clothing on
        // this return. Do not step it twice. Replay's step_restored clears the
        // completed-game-step marker, including when it freezes on one frame.
        if !materials.cloth_needs_step(frame) {
            return Ok(());
        }
        let root = self.published_root.ok_or("return cloth game root absent")?;
        let hips = self.published_hips.ok_or("return cloth game hips absent")?;
        let world = super::super::move_root::multiply(root, hips);
        let mut cloth_hips = hips;
        cloth_hips[3] = 0.;
        cloth_hips[7] = 0.;
        cloth_hips[11] = 0.;
        let inverse = super::super::move_root::matrix_inverse::try_inverse(
            cloth_hips,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        ).map_err(|_| "return cloth hips inverse")?;
        let (header, _) = self.submitted.as_ref().ok_or("return cloth game header absent")?;
        let wipeout = u32::from_be_bytes([header[192], header[193], header[194], header[195]]) >> 16;
        materials.step_restored_cloth(
            &self.palette, cloth_hips, inverse,
            crate::skater_ropa_runtime::ClothClock { frame, fraction: 0. }, wipeout,
        ).map_err(|e| e.to_string())?;
        materials.publish_cloth(&self.palette, world, 1.).map_err(|e| e.to_string())?;
        // EndJobs (0x827dbd50) finishes CPU skinning and publishes the matching
        // palette together. Advance the publication so the GPU vertex cache
        // also uploads the returned stream before another physics tick runs.
        self.palette.repack_for_upload(world).map_err(|e| e.to_string())
    }

    /// UpdatePresEntity827e61e8 replaces the presentation entity when its recipe
    /// identity changes. Retain the application's existing allocation policy.
    pub(crate) fn replace_recipe(&mut self,presentation:&SelectedPresentationStartup,
        scene:Arc<CharacterScene>)->Result<(),SelectedPresentationConstructionError> {
        *self=Self::try_new(presentation,scene,self.policy)?;
        Ok(())
    }
    pub(crate) fn try_bind_materials(
        &mut self,
        finalized: &crate::player::mounted_recipe::finalize::FinalizedRecipeScene,
        vault: std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars: crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), crate::character::material_admission::CharacterMaterialError> {
        if self.materials.is_some() {
            return Err(crate::character::material_admission::CharacterMaterialError::AlreadyBound);
        }
        self.materials = Some(Arc::new(Mutex::new(
            crate::character::material_admission::CharacterMaterialAdmission::try_new(
                finalized,
                &self.palette,
                vault,
                cvars,
            )?,
        )));
        Ok(())
    }
    pub(crate) fn materials(
        &self,
    ) -> Result<
        &Arc<Mutex<crate::character::material_admission::CharacterMaterialAdmission>>,
        crate::character::material_admission::CharacterMaterialError,
    > {
        self.materials.as_ref().ok_or(
            crate::character::material_admission::CharacterMaterialError::Missing(
                "installed materials",
            ),
        )
    }
    pub(crate) fn producer(&self) -> &SelectedPresentationFrameProducer {
        &self.producer
    }
    /// GetSkaterRoot826c71f8 reads DataPostBlend.root (+16), not
    /// root * mHipsTransform. Pres_LocalPlayerStreamerController8277d3d8
    /// follows this matrix's translation after publication.
    pub(crate) fn published_root_position(&self) -> Option<[f32; 3]> {
        self.published
            .then_some(self.published_root_position)
            .flatten()
    }
    /// The whole root, axes and translation, which the ICE camera anchor reads.
    pub(crate) fn published_root(&self) -> Option<[f32; 16]> {
        self.published.then_some(self.published_root).flatten()
    }
    /// The hips position `RenderMan::UpdateRenderParams` 0x82852780 samples
    /// the sun-direction map at.
    pub(crate) fn published_hips_position(&self) -> Option<[f32; 3]> {
        self.published
            .then_some(self.published_hips_position)
            .flatten()
    }
    /// `GetHipsMatrix` (`0x826c70d8`)'s Z row, which the ICE avatars' orientations read.
    pub(crate) fn published_hips_forward(&self) -> Option<[f32; 3]> {
        self.published
            .then_some(self.published_hips_forward)
            .flatten()
    }
    /// The world `m_BB` `PostFX::RenderShadows_CSM` 0x82855888 reads for the
    /// local skater, min then max.
    pub(crate) fn published_bounds(&self) -> Option<[[f32; 3]; 2]> {
        self.published.then_some(self.published_bounds).flatten()
    }
    /// The retained SkaterPresEntity opacity after the last restore.
    pub(crate) fn render_opacity(&self) -> Option<render_opacity::SkaterRenderOpacity> {
        self.published.then_some(self.render_opacity)
    }
    /// DoSimRestore827e7298..73e0 once per completed frame (one presented frame's
    /// `kUpdateTime`). The camera is the
    /// position the renderer draws from: the provisional CameraMan producer
    /// matrix, since the native CameraPresentation restore is still absent.
    pub(crate) fn try_restore_render_opacity(
        &mut self,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        camera: [f32; 3],
    ) -> Result<(), render_opacity::RenderOpacityError> {
        if !self.published { return Ok(()); }
        let inputs = self
            .render_inputs
            .ok_or(render_opacity::RenderOpacityError::MissingInputs)?;
        let rendering = self
            .rendering_skaters
            .get_or_insert_with(|| render_opacity::rendering_skaters(vault));
        render_opacity::try_restore(
            &mut self.render_opacity,
            inputs,
            camera,
            rendering,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            crate::simulation_sim_rate::UPDATE_TIME,
        )
    }
    fn is_origin(&self, presentation: &SelectedPresentationStartup) -> bool {
        Arc::ptr_eq(&self.origin, &presentation.animation.hips_transforms)
            && presentation.restored.as_ref().is_some_and(|r| {
                self.base.is_same_view(&r.next) && self.replay.is_same_view(&r.previous)
            })
    }
    /// Ordered selected single-model path. Failure preserves completed native
    /// stores; callers must not replay a failed phase as a new successful frame.
    /// The optional observer performs no diagnostic reads on the `None` path.
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "explicit numeric/job/settings services and optional read-only boundary observer"
    )]
    pub(crate) fn try_update<N, C, R, J, O>(
        &mut self,
        presentation: &SelectedPresentationStartup,
        source: ActorPresentationFrameSource<'_>,
        settings: &mut C,
        numeric: &mut N,
        rounding: &mut R,
        jobs: &mut J,
        mut observer: Option<
            &mut dyn FnMut(
                SelectedPresentationStage,
                &Self,
                &SubmittedPresentationFrame,
            ) -> Result<(), O>,
        >,
    ) -> Result<(), SelectedPresentationError<N::Error, C::Error, R::Error, J::Error, O>>
    where
        N: GameInputCommandNumeric + PresentationDecodeNumeric + PresentationMatrixNumeric,
        C: PresentationReadSettings,
        R: ReckoningSinCosNumeric,
        J: ActorAnimationInitializeSource,
    {
        use SelectedPresentationError as E;
        if !self.is_origin(presentation) {
            return Err(E::ForeignPresentation);
        }
        self.published = false;
        self.published_peer = None;
        self.published_root_position = None;
        self.published_root = None;
        self.published_hips_position = None;
        self.published_hips_forward = None;
        self.published_hips = None;
        self.published_bounds = None;
        self.render_inputs = None;
        self.submitted = None;
        // DoSimSubmit 0x827dd3c0..3e8 filters the live Actor list before
        // WriteData. An unspawned AI's backend row is already gone while
        // its queued Actor destruction still waits for dispatch.
        if !source.submit_in_actor_list().map_err(|error| E::Produce(PresentationFrameError::Source(error)))? {
            return Ok(());
        }
        let animation = &presentation.animation;
        let frame = self
            .producer
            .try_produce(animation, &[source], numeric)
            .map_err(E::Produce)?;
        macro_rules! observe {
            ($stage:ident) => {
                if let Some(callback) = observer.as_mut() {
                    callback(SelectedPresentationStage::$stage, self, &frame).map_err(|error| {
                        E::Observe {
                            stage: SelectedPresentationStage::$stage,
                            error,
                        }
                    })?;
                }
            };
        }
        observe!(Produced);
        let view = self.producer.read_frame(&frame, 0).map_err(E::Frame)?;
        // UpdateSkater827da1d0..1f0 copies header word 0xbc bits 0xff00 and
        // word 0xc0 into the post-blend flags DoSimRestore reads.
        let opacity_byte = *view
            .read_base
            .get(0xbe)
            .ok_or(E::Missing("frame opacity byte"))?;
        let header_flags = u32::from_be_bytes(
            view.read_base
                .get(192..196)
                .ok_or(E::Missing("frame locality flags"))?
                .try_into()
                .expect("four bytes"),
        );
        if let Some(materials) = self.materials.as_ref() {
            materials
                .lock()
                .map_err(|_| E::Material(MATERIAL_LOCK))?
                .try_construct_from_header(view.read_base)
                .map_err(E::Material)?;
        }
        // DoSimRestore827e6fe0 copies odd/base into even/replay on EVERY
        // restore, immediately before UpdateBodies827e7014. ReadSkater adds
        // each face delta to the even destination, so retaining last frame's
        // even pose here would accumulate that delta across frames.
        self.replay
            .try_override_from(&self.base)
            .map_err(E::ReplayOverride)?;
        let mut scratch = PresentationReadScratch {
            quaternions: &mut self.quaternions,
            translations: &mut self.translations,
        };
        let decoded = animation
            .try_decode_and_extract(
                &view,
                &self.base,
                &self.replay,
                &mut scratch,
                settings,
                numeric,
                jobs,
            )
            .map_err(E::Decode)?;
        observe!(Decoded);
        let mut header = [0; crate::presentation_restore::skater::HEADER_BYTES];
        header.copy_from_slice(
            view.read_base
                .get(..crate::presentation_restore::skater::HEADER_BYTES)
                .ok_or(E::Missing("frame header"))?,
        );
        let pose = self.replay.read().map_err(E::ReplayOverride)?.1;
        animation
            .try_correct_replay(1, presentation.twist_attributes.as_ref(), numeric, rounding)
            .map_err(E::Correct)?;
        observe!(Corrected);
        decoded
            .try_finalize(
                animation,
                0,
                Arc::clone(&self.palette.skinning().hierarchy_inverse_bind_pose),
                jobs,
            )
            .map_err(E::Finalize)?;
        observe!(Finalized);
        let matrices = animation.interpolated[0]
            .matrices()
            .map_err(E::Commands)?
            .ok_or(E::Missing("synchronized final matrices"))?;
        self.palette
            .update_bone_transforms(&matrices)
            .map_err(E::Palette)?;
        // DoSimRestore827e7160..7248 gives the skater entity root * saved
        // Hips as m_MatLtoW. EndJobs827dbe24 uses that same matrix to pack
        // each gathered skin matrix; the submitted root alone omits the
        // physical Hips placement from the GPU palette.
        let root = self.producer.same_frame_root(&frame, 0).map_err(E::Frame)?;
        let hips = animation
            .hips_transforms
            .lock()
            .map_err(|_| E::HipsTransform(ActorFaceError::Lock))?[0]
            .ok_or(E::Missing("saved Hips transform"))?;
        let local_to_world = super::super::move_root::multiply(root, hips);
        if let Some(materials)=self.materials.as_ref() {
            let mut materials = materials.lock().map_err(|_| E::Material(MATERIAL_LOCK))?;
            if materials.cloth_disabled()==Some(false) && !self.cloth_by_presentation {
                let mut cloth_hips=hips;
                cloth_hips[3]=0.;cloth_hips[7]=0.;cloth_hips[11]=0.;
                let inverse=super::super::move_root::matrix_inverse::try_inverse(cloth_hips,numeric)
                    .map_err(E::ClothNumeric)?;
                // Selected publication occurs on completed simulation ticks;
                // its GAME clock fraction is zero, never render wall time.
                let tick=self.cloth_tick.unwrap_or(0);
                let clock = crate::skater_ropa_runtime::ClothClock { frame: tick, fraction: 0. };
                materials.update_cloth(&self.palette,cloth_hips,inverse,local_to_world,clock,header_flags>>16)
                    .map_err(E::Material)?;
            }
        }
        // DoSimRestore827e72fc..7320: m_MatLtoW translation is the hips position
        // and the DataPostBlend root (+16) Y axis is the up vector.
        self.render_inputs = Some(render_opacity::RenderOpacityInputs {
            opacity_byte,
            collidable: header_flags & 0x4000 != 0,
            main_camera_only: header_flags & 0x2000 != 0,
            hips: [local_to_world[12], local_to_world[13], local_to_world[14]],
            up: [root[4], root[5], root[6]],
            // A completed tick's frame is real time, never a replay's.
            in_replay: false,
        });
        self.palette
            .pack_for_upload(local_to_world)
            .map_err(E::Palette)?;
        self.submitted = Some((header, pose));
        self.published = true;
        self.published_peer=Some(i64::from_be_bytes(view.read_base[160..168].try_into().expect("validated frame header")));
        self.published_root_position = Some([root[12], root[13], root[14]]);
        self.published_root = Some(root);
        self.published_hips_position = Some(hips_position(root, hips));
        self.published_hips_forward = Some(hips_forward(root, hips));
        self.published_hips = Some(hips);
        // SceneRenderView::Cull 0x828501c0..e0: m_BB is m_localBB plus the
        // m_MatLtoW translation row (+0x190), lane by lane.
        let translation = [local_to_world[12], local_to_world[13], local_to_world[14]];
        self.published_bounds = Some([
            std::array::from_fn(|lane| self.local_bounds.minimum[lane] + translation[lane]),
            std::array::from_fn(|lane| self.local_bounds.maximum[lane] + translation[lane]),
        ]);
        observe!(Published);
        Ok(())
    }
}
impl SelectedPresentationRuntime {
    /// `DoSimSubmit`'s record of this skater for the tick it last published: the
    /// header, `ReadSkater`'s pose and the twist ratios the expressions sample.
    pub(crate) fn submitted_frame(
        &mut self,
        presentation: &SelectedPresentationStartup,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    ) -> Result<Option<crate::presentation_restore::skater::SkaterFrame>, String> {
        use super::super::{
            corrections::native_bone_index,
            expressions::{TwistAttributeSource, TwistRatio},
        };
        let Some((header, pose)) = self.submitted.as_ref().filter(|_| self.published) else {
            return Ok(None);
        };
        let rig = match &self.restore_rig {
            Some(rig) => Arc::clone(rig),
            None => {
                let animation = &presentation.animation;
                let bind = animation
                    .bind
                    .as_ref()
                    .ok_or("presentation bind pose absent")?
                    .read()
                    .map_err(|e| format!("presentation bind pose: {e:?}"))?
                    .1;
                let rig = Arc::new(crate::presentation_restore::skater::SkaterRig {
                    skeleton: self.skeleton.clone(),
                    bind,
                    inverse_bind: Arc::clone(&self.palette.skinning().hierarchy_inverse_bind_pose),
                    hips: native_bone_index(animation.hips),
                    board: native_bone_index(animation.skateboard),
                    twist_bones: animation.twist_bones.map(native_bone_index),
                    local_bounds: [
                        std::array::from_fn(|lane| self.local_bounds.minimum[lane]),
                        std::array::from_fn(|lane| self.local_bounds.maximum[lane]),
                    ],
                    rendering_skaters: render_opacity::rendering_skaters(vault),
                });
                self.restore_rig = Some(Arc::clone(&rig));
                rig
            }
        };
        let mut attributes = presentation.twist_attributes.as_ref();
        let mut ratio = |ratio| {
            attributes
                .read_ratio(ratio)
                .map_err(|e| format!("presentation twist ratio: {e}"))
        };
        let twist_ratios = [
            ratio(TwistRatio::Arm)?,
            ratio(TwistRatio::Forearm)?,
            ratio(TwistRatio::ForearmSecond)?,
            ratio(TwistRatio::UpperLeg)?,
        ];
        Ok(Some(crate::presentation_restore::skater::SkaterFrame {
            rig,
            palette: Arc::clone(&self.palette_template),
            header: *header,
            pose: pose.clone(),
            twist_ratios,
            materials: self.materials.clone(),
        }))
    }

    /// The palette's inverse bind pose at a hierarchy bone.
    pub(crate) fn inverse_bind(&self, bone: usize) -> Option<crate::animation::AnimationMatrix> {
        self.palette.skinning().hierarchy_inverse_bind_pose.get(bone).copied()
    }

    /// The scene's hips for a borrowed entity, which its `GetPosition`
    /// answers while borrowed.
    pub(crate) fn borrowed_hips_position(&self) -> Option<[f32; 3]> {
        self.borrowed_hips_position
    }

    /// Whether a scene character borrowed this entity at the last publication.
    pub(crate) const fn borrowed(&self) -> bool {
        self.borrowed_hips_position.is_some()
    }

    /// No scene character holds this entity: its callbacks are the game's again.
    pub(crate) fn end_borrow(&mut self) {
        self.borrowed_hips_position = None;
    }

    /// A NIS's "current_player" borrows this entity
    /// (`SceneCharacterManager::ControlCharacterRenderable` `0x82870fb0`):
    /// its bone-buffer and hips callbacks answer the scene character, whose
    /// clip poses this same rig, so the palette takes the scene's pose and
    /// matrix and m_BB follows that matrix as in `Cull`. `GetSkaterRoot`
    /// and `SkaterPresentation::GetHipsPos` still read the restored data.
    /// No restore runs its jobs: `SkaterPresEntity::Update` `0x827e8568`
    /// does, and `ClothUpdate` steps the cloth by `nis_time`, the character
    /// NIS's elapsed time, else by the presentation's clock.
    pub(crate) fn publish_borrowed(
        &mut self,
        character: &crate::nis::characters::SceneCharacter,
        nis_time: Option<f32>,
    ) -> Result<(), String> {
        if !self.published {
            return Err("borrowed skater before its first publication".into());
        }
        let skin = character.pose_palette(&mut self.palette)?;
        if let Some(materials) = self.materials.as_ref() {
            let mut materials = materials
                .lock()
                .map_err(|_| "borrowed skater material lock poisoned")?;
            if materials.cloth_disabled() == Some(false) {
                // StartJobs 0x827dbbf0..0x827dbc88: mHipsTransform is the bone
                // buffer's first matrix with the w of its axes cleared.
                let mut hips = skin.first_bone;
                hips[3] = 0.;
                hips[7] = 0.;
                hips[11] = 0.;
                let inverse = super::super::move_root::matrix_inverse::try_inverse(
                    hips,
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                )
                .map_err(|_| "borrowed cloth hips inverse")?;
                let time = match nis_time {
                    Some(elapsed) => crate::skater_ropa_runtime::ClothTime::Nis(elapsed),
                    None => crate::skater_ropa_runtime::ClothTime::Game(
                        crate::skater_ropa_runtime::ClothClock {
                            frame: self.cloth_tick.unwrap_or(0),
                            fraction: 0.,
                        },
                    ),
                };
                materials
                    .step_borrowed_cloth(&self.palette, hips, inverse, time)
                    .map_err(|e| e.to_string())?;
                // EndJobs 0x827dbd50: the cloth's vertices, then the palette, both
                // with m_MatLtoW, the scene's object matrix.
                materials
                    .publish_cloth(&self.palette, skin.pose.root, 1.)
                    .map_err(|e| e.to_string())?;
            }
        }
        self.palette
            .pack_for_upload(skin.pose.root)
            .map_err(|e| e.to_string())?;
        let pose = skin.pose;
        let translation = [pose.root[12], pose.root[13], pose.root[14]];
        self.borrowed_hips_position = Some(pose.hips);
        self.published_bounds = Some([
            std::array::from_fn(|lane| self.local_bounds.minimum[lane] + translation[lane]),
            std::array::from_fn(|lane| self.local_bounds.maximum[lane] + translation[lane]),
        ]);
        Ok(())
    }
}

/// `GetHipsMatrix` (`0x826c70d8`)'s Z row: the saved Hips Z row through the
/// DataPostBlend root rows.
fn hips_forward(root: [f32; 16], hips: [f32; 16]) -> [f32; 3] {
    let [x, y, z, w] = [hips[8], hips[9], hips[10], hips[11]];
    std::array::from_fn(|lane| {
        root[12 + lane].mul_add(
            w,
            root[8 + lane].mul_add(z, root[4 + lane].mul_add(y, root[lane] * x)),
        )
    })
}

/// `GetHipsPos`: the DataPostBlend root rows scaled by the saved Hips
/// translation row (x, y, z, w) and accumulated with `vmaddfp`, row 0 first.
fn hips_position(root: [f32; 16], hips: [f32; 16]) -> [f32; 3] {
    let [x, y, z, w] = [hips[12], hips[13], hips[14], hips[15]];
    std::array::from_fn(|lane| {
        root[12 + lane].mul_add(
            w,
            root[8 + lane].mul_add(z, root[4 + lane].mul_add(y, root[lane] * x)),
        )
    })
}
fn scratch(count: usize) -> Result<Vec<[f32; 4]>, PresentationBufferError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| PresentationBufferError::Allocation)?;
    values.resize(count, [0.; 4]);
    Ok(values)
}
