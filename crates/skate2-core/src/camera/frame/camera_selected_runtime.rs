//! Same retained camera observer, selected capture and scheduled native prefix.
//! Activity remains unavailable; no full CameraManager.Init/Update token is issued.
use super::{CameraStartupFields, construction::CameraConstructionError};
use crate::{
    actor_construction::{
        ActorConstruction, ActorConstructionError,
        physical_mount::ActorPhysicalMountError,
    },
    attribute_notifications::AttributeNotifications,
    attribute_runtime::AttributeRuntimeVault,
    camera_attribute_read::CameraAttributeReadError,
    camera_compass::CameraAnchorError,
    camera_graph::CameraGraphCache,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::LivingWorldGrindServices,
    living_world_rain::RainServiceError,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::ConcretePhysOutError,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, cross_typed,
        try_normalize_unsafe,
    },
    selected_simulation_asset_services::SelectedSimulationAssetServices,
    simulation_assembly_backend::constraint_session::prediction::PhysicalPredictionOwner,
    skeleton_root_transforms::SkeletonRootMatrix,
    trigger_camera_group::CameraTriggerGroup,
    world_painter_layer::WorldPainterLayerMan,
};
use skate2_formats::mount::MountSet;
use std::{
    convert::Infallible,
    sync::{Arc, Mutex, Weak},
};

/// Actual session capabilities retained before roster notification. Cloning this
/// descriptor clones only owner handles, never the world, graph or asset payload.
#[derive(Clone)]
pub(crate) struct CameraSelectedInputs {
    pub(crate) vault: Arc<AttributeRuntimeVault>,
    pub(crate) cvars: InputCvarReferences,
    pub(crate) mounts: Arc<MountSet>,
    pub(crate) monitor: Arc<AttributeNotifications>,
    pub(crate) cache: CameraGraphCache,
    pub(crate) world: Weak<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) predictor: Arc<PhysicalPredictionOwner>,
    pub(crate) living_world: Arc<Mutex<LivingWorldGrindServices>>,
    pub(crate) skaters: Arc<crate::skater_manager_local_identity::SkaterManagerLocalIdentity>,
    pub(crate) assets: Arc<SelectedSimulationAssetServices>,
    pub(crate) layers: Arc<Mutex<WorldPainterLayerMan>>,
    pub(crate) triggers: Arc<Mutex<CameraTriggerGroup>>,
    /// The Simulation's slow-motion rate and its eEQGameSim message queue.
    pub(crate) sim_rate: Arc<Mutex<crate::simulation_sim_rate::SimulationSimRate>>,
    /// Copied from the actual `CommonBackEnd` constructor field, not a fallback.
    pub(crate) camera_set: String,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent native camera condition flags are not mutually exclusive states"
)]
pub(crate) struct CameraGlobalCondition {
    pub(crate) phys_out: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    pub(crate) current_camera_type: i32,
    pub(crate) invert_x: bool,
    pub(crate) invert_y: bool,
    pub(crate) grind_detector_active: bool,
    pub(crate) painted_region_blend: f32,
    pub(crate) is_grinding: bool,
    pub(crate) on_road: bool,
    pub(crate) ledge_left: bool,
    pub(crate) ledge_right: bool,
    pub(crate) subject_transform: SkeletonRootMatrix,
}
impl CameraGlobalCondition {
    /// `GameCameraManager` ctor82e37cf0..7d28 and subject transform7ea0..7ed0.
    pub(crate) fn construct() -> Self {
        Self {
            phys_out: None,
            current_camera_type: 0,
            invert_x: false,
            invert_y: false,
            grind_detector_active: false,
            painted_region_blend: 0.0,
            is_grinding: false,
            on_road: false,
            ledge_left: false,
            ledge_right: false,
            subject_transform: crate::camera_subject::identity(),
        }
    }
}
#[derive(Debug)]
pub(crate) enum CameraSelectedError {
    AlreadyCaptured,
    CameraSetNameAssertion,
    MissingCapture,
    MissingConstruction,
    PhysicalOutput(ActorPhysicalMountError),
    Actor(ActorConstructionError),
    MissingAnimation,
    Anchors(CameraAnchorError<ReckoningHostNumericError>),
    OwnerLock,
    Construction(CameraConstructionError<ReckoningHostNumericError>),
    Teleport(ReckoningHostNumericError),
    Attributes(CameraAttributeReadError),
    Component(ConcretePhysOutError<Infallible>),
    Road(RainServiceError),
    GrindLedgeCvar(crate::input_cvar_runtime::InputCvarError),
    GrindLedge(ReckoningHostNumericError),
    LookAt(ReckoningHostNumericError),
    Subject(
        super::subject_update::CameraSubjectUpdateError<
            super::subject_update::CameraAnimationReadError,
        >,
    ),
    Reference(super::reference_points::CameraReferenceError<CameraReferenceOwnerError>),
    Compass(crate::camera_compass::runtime::CameraCompassError<ReckoningHostNumericError>),
    WrongPhysicsIsland,
    CameraInstanceCvar(crate::input_cvar_runtime::InputCvarError),
    CameraPreUpdate(
        crate::camera_man::preupdate::CameraPreUpdateContinuationError<ReckoningHostNumericError>,
    ),
    MissingCameraGraphController,
    CameraGraph(crate::camera_graph::update::CameraGraphUpdateError<ReckoningHostNumericError>),
    CameraManUpdate(crate::camera_man::update::CameraManUpdateError),
    PendingCameraInstancesTail,
    PendingActivity,
}
#[derive(Debug)]
pub(crate) enum CameraReferenceOwnerError {
    OwnerLock,
}

impl CameraStartupFields {
    /// `CameraManager::Update` (`0x82584448`): the profile's `mCameraAngle`, invert axes and
    /// `mGrindTransparency` (`mIsGrindDetectorActive`) into the global condition data every
    /// update; nothing before the camera is constructed.
    pub(crate) fn set_profile_settings(&mut self, camera_type: i32, invert_x: bool, invert_y: bool, grind_detector: bool) {
        if let Some(global) = self.global.as_mut() {
            global.current_camera_type = camera_type;
            global.invert_x = invert_x;
            global.invert_y = invert_y;
            global.grind_detector_active = grind_detector;
        }
    }
    /// Explicit selected capture; native full Init still requires Activity.
    /// Inputs survive any later constructor failure. No callback is invoked here.
    pub(crate) fn try_capture_selected(
        &mut self,
        inputs: CameraSelectedInputs,
    ) -> Result<(), CameraSelectedError> {
        if self.selected_inputs.is_some() {
            return Err(CameraSelectedError::AlreadyCaptured);
        }
        if inputs.camera_set.len() >= 64 {
            return Err(CameraSelectedError::CameraSetNameAssertion);
        }
        let vault = Arc::clone(&inputs.vault);
        let monitor = Arc::clone(&inputs.monitor);
        self.selected_inputs = Some(inputs);
        // CameraManager825849c8 constructs its BasicCamera list before the
        // GameCameraManager. BasicCamera829d2fa0 starts at (-7,1,0), up +Y,
        // looking at zero. SetTarget829d2ab8 normalizes heading with two
        // reciprocal-sqrt refinements and crosses up/heading WITHOUT further
        // normalization. Reuse the existing numeric graph, including W lanes.
        let position =
            crate::skeleton_root_transforms::SkeletonRootVector::known([-7.0, 1.0, 0.0, 0.0]);
        let up = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        let delta = crate::camera_subject::ZERO.zip(position, |target, eye| target - eye);
        let heading = try_normalize_unsafe(delta, &mut HostReckoningNumeric)
            .map_err(CameraSelectedError::LookAt)?;
        self.simulation_camera_matrix = Some(SkeletonRootMatrix::from_columns([
            cross_typed(up, heading),
            up,
            heading,
            position,
        ]));
        self.try_construct_selected_components::<ReckoningHostNumericError>(&vault, &monitor)
            .map_err(CameraSelectedError::Construction)
    }

    /// `OnLocalSkaterChanged82e3a758`: Create instances, then `SetLocalSkater`.
    /// Preserve Actor, physical-output and collision stores if Teleport fails.
    pub(crate) fn try_local_changed_selected(
        &mut self,
        actor: Option<&Arc<ActorConstruction>>,
    ) -> Result<(), CameraSelectedError> {
        self.producer_snapshot_ready = false;
        let inputs = self
            .selected_inputs
            .as_ref()
            .ok_or(CameraSelectedError::MissingCapture)?
            .clone();
        let mut numeric = HostReckoningNumeric;
        self.try_create_selected_camera_instances(&inputs, &mut numeric)
            .map_err(CameraSelectedError::Construction)?;
        self.actor = actor.cloned();
        let output = if let Some(actor) = actor {
            Some(Arc::clone(
                actor
                    .cached_physical_allocation()
                    .map_err(CameraSelectedError::PhysicalOutput)?
                    .owner(),
            ))
        } else {
            None
        };
        self.global
            .as_mut()
            .ok_or(CameraSelectedError::MissingConstruction)?
            .phys_out = output;
        let retained = self
            .construction_state
            .as_ref()
            .ok_or(CameraSelectedError::MissingConstruction)?;
        if let Some(man) = &retained.camera_man {
            let mut man = man.lock().map_err(|_| CameraSelectedError::OwnerLock)?;
            man.rig
                .collision
                .lock()
                .map_err(|_| CameraSelectedError::OwnerLock)?
                .actor = actor.cloned();
            man.teleport(&mut numeric)
                .map_err(CameraSelectedError::Teleport)?;
        }
        Ok(())
    }

    /// Scheduled CameraManager.Update82584448 then GameCameraManager.Update.
    /// This is a partial producer, never a completed camera frame/report.
    #[expect(
        clippy::too_many_lines,
        reason = "Native ordered camera prefix retains each failure publication boundary"
    )]
    pub(crate) fn try_update_selected_prefix(
        &mut self,
        dt: f32,
        authority: crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
        services: &mut crate::simulation_assembly_backend::constraint_session::physical_world_line::camera::CameraLineServices<'_>,
    ) -> Result<(), CameraSelectedError> {
        let camera_profile =
            std::env::var_os("SKATE2_PROFILE_REDRAW").map(|_| std::time::Instant::now());
        self.producer_snapshot_ready = false;
        #[cfg(feature = "skate3-runtime")]
        self.restore_skate2_field_of_view();
        use CameraSelectedError as E;
        let retained = self
            .construction_state
            .as_mut()
            .ok_or(E::MissingConstruction)?;
        retained
            .parameters
            .lock()
            .map_err(|_| E::OwnerLock)?
            .try_read_if_changed()
            .map_err(E::Attributes)?;
        if self.actor.is_none() {
            return Ok(());
        }
        let inputs = self.selected_inputs.as_ref().ok_or(E::MissingCapture)?;
        let global = self.global.as_mut().ok_or(E::MissingConstruction)?;
        if let Some(owner) = &global.phys_out {
            let output = owner.phys_out().lock().map_err(|_| E::OwnerLock)?;
            let hips = output
                .try_skeleton_output()
                .map_err(E::Component)?
                .skeleton_hips_pos;
            global.on_road = inputs
                .living_world
                .lock()
                .map_err(|_| E::OwnerLock)?
                .try_point_on_road_network(hips)
                .map_err(E::Road)?;
            // UpdateGrindLedgeData 0x82e350b8: const PhysOut reads, no latch.
            let was_grinding = global.is_grinding;
            let grinds = output.try_camera_grinds_output().map_err(E::Component)?;
            global.is_grinding = grinds.is_grinding;
            if global.is_grinding && !was_grinding {
                if grinds.is_grinding_ledge {
                    let velocity = output
                        .try_camera_motion_vector(
                            crate::physical_player_phys_out::PhysOutMotionVector::DeckVelocity,
                        )
                        .map_err(E::Component)?;
                    let mut numeric = HostReckoningNumeric;
                    let squared = numeric
                        .dot_vector(velocity, velocity)
                        .map_err(E::GrindLedge)?;
                    let insignificant = inputs
                        .cvars
                        .try_grind_float(
                            crate::input_cvar_runtime::grind_states::GrindCvar::kInsignificantGrindSpeed,
                        )
                        .map_err(E::GrindLedgeCvar)?;
                    // Below the insignificant speed the previous sides are kept.
                    if (insignificant * insignificant).partial_cmp(&squared)
                        != Some(std::cmp::Ordering::Greater)
                    {
                        let mut dir = grinds.grind_primitive_dir;
                        // The shipped comparison constant is -1.0, not zero.
                        if -1.0 > numeric.dot_vector(dir, velocity).map_err(E::GrindLedge)? {
                            dir = dir.map(|lane| lane * -1.0);
                        }
                        let high = grinds.ledge_high_dir;
                        // Y of cross(dir, high): vnmsubfp on the permuted rows.
                        let y = (-dir.xyz[0]).mul_add(high.xyz[2], dir.xyz[2] * high.xyz[0]);
                        global.ledge_left = y > 0.0;
                        global.ledge_right = !global.ledge_left;
                    }
                } else {
                    global.ledge_left = false;
                    global.ledge_right = false;
                }
            }
        }
        let camera_globals_time = camera_profile.map(|start| start.elapsed());
        retained.grind.lock().map_err(|_| E::OwnerLock)?.active = global.grind_detector_active;
        // UpdateSubject gets the Actor output again, not the saved global slot.
        let actor = self.actor.as_ref().unwrap();
        let allocation = actor
            .cached_physical_allocation()
            .map_err(E::PhysicalOutput)?;
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::OwnerLock)?;
        if output
            .try_state_output()
            .map_err(E::Component)?
            .is_wiping_out
        {
            let skeleton = output.try_skeleton_output().map_err(E::Component)?;
            let eye = skeleton.skeleton_hips_pos;
            let target = eye.zip(skeleton.skeleton_hips_look, |a, b| a + b);
            let delta = target.zip(eye, |a, b| a - b);
            let mut numeric = HostReckoningNumeric;
            let z = try_normalize_unsafe(delta, &mut numeric).map_err(E::LookAt)?;
            let up =
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
            let x = try_normalize_unsafe(cross_typed(up, z), &mut numeric).map_err(E::LookAt)?;
            let y = cross_typed(z, x);
            let reloaded_eye = output
                .try_skeleton_output()
                .map_err(E::Component)?
                .skeleton_hips_pos;
            // UpdateSubjectTransform copies the completed local LookAt result.
            global.subject_transform = SkeletonRootMatrix::from_columns([x, y, z, reloaded_eye]);
        } else {
            // Source normal path copies PREVIOUS Subject skateboard transform.
            global.subject_transform = retained
                .subject
                .lock()
                .map_err(|_| E::OwnerLock)?
                .board_effective_transform;
        }
        drop(output);
        let man = Arc::clone(retained.camera_man.as_ref().ok_or(E::MissingConstruction)?);
        let animation = actor
            .published_camera_animation()
            .map_err(E::Actor)?
            .ok_or(E::MissingAnimation)?;
        // Raw8efc uses CameraMan+268 (mSubject), then virtual+fc, proven
        // GetSubjectTransform82e3dd08. This is the previous Subject transform.
        let man_subject = Arc::clone(
            man.lock()
                .map_err(|_| E::OwnerLock)?
                .subject
                .as_ref()
                .ok_or(E::MissingConstruction)?,
        );
        let filtered_position = man_subject
            .lock()
            .map_err(|_| E::OwnerLock)?
            .transform
            .columns()[3];
        retained
            .anchors
            .try_update(
                dt,
                allocation.owner(),
                filtered_position,
                &inputs.cvars,
                &mut HostReckoningNumeric,
            )
            .map_err(E::Anchors)?;
        let mut subject = retained.subject.lock().map_err(|_| E::OwnerLock)?;
        for (index, point) in retained.anchors.points.iter().enumerate() {
            subject.anchor_positions[index] = point.position;
            subject.anchor_velocities[index] = point.velocity;
            subject.anchor_angular_velocities[index] =
                point.angular_velocity.expect("successful anchor update");
        }
        let output = allocation
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::OwnerLock)?;
        subject
            .try_publish_fields(
                &output,
                &mut super::subject_update::CameraAnimationRead(&animation),
                global,
            )
            .map_err(E::Subject)?;
        subject
            .try_update_reference_points(&output, || {
                let camera = man
                    .lock()
                    .map_err(|_| CameraReferenceOwnerError::OwnerLock)?;
                Ok(Some(camera.rig.anchor_tracker.output_position))
            })
            .map_err(E::Reference)?;
        retained
            .compass
            .try_update_subject(
                &mut subject,
                global.subject_transform,
                &mut retained.compass_output,
                dt,
                &mut HostReckoningNumeric,
            )
            .map_err(E::Compass)?;
        drop(subject);
        drop(output);
        let camera_subject_time = camera_profile.map(|start| start.elapsed());
        // UpdateCameraManInstances82e36270 first calls the selected actual
        // Island's ReinstanceCollisionParts (vtable820ffa7c -> folded BLR).
        // Validate the existing same-Island authority before admitting that call.
        // The tick already holds the world; use its actual authority rather
        // than recursively locking the captured weak world.
        if !actor.belongs_to_physics_manager(authority) {
            return Err(E::WrongPhysicsIsland);
        }
        let count = inputs
            .cvars
            .try_camera_instance_count()
            .map_err(E::CameraInstanceCvar)?;
        if count == 0 {
            return Err(E::PendingCameraInstancesTail);
        }
        // Only the first constructed camera is reached until its real graph
        // update and CameraMan.Update complete; no later instance is skipped.
        let graph = retained.graph.as_ref().ok_or(E::MissingConstruction)?;
        man.lock()
            .map_err(|_| E::OwnerLock)?
            .try_preupdate(
                dt,
                0,
                false,
                &inputs.cvars,
                services.overlay,
                services,
                &mut HostReckoningNumeric,
            )
            .map_err(E::CameraPreUpdate)?;
        let camera_preupdate_time = camera_profile.map(|start| start.elapsed());
        let (class, collection, _) = crate::camera_attributes::BINDINGS[0];
        let editor = inputs.vault.instance(class, collection);
        if graph
            .controller
            .fields
            .lock()
            .map_err(|_| E::OwnerLock)?
            .dynamic
            .is_none()
        {
            return Err(E::MissingCameraGraphController);
        }
        // Native36384 invokes the SAME existing dynamic controller at fixed1/60.
        // No man, Subject, output or world guard is reacquired across callbacks.
        graph
            .try_update(global, inputs, &retained.subject, &mut HostReckoningNumeric)
            .map_err(E::CameraGraph)?;
        let camera_graph_time = camera_profile.map(|start| start.elapsed());
        // Native destroys the temporary Editor instance before CameraMan.Update.
        drop(editor);
        man.lock()
            .map_err(|_| E::OwnerLock)?
            .try_update_prefix(
                dt,
                0,
                false,
                &inputs.cvars,
                crate::camera_prediction::CameraObstacleSources {
                    cvars: &inputs.cvars,
                    living_world: &inputs.living_world,
                    skaters: &inputs.skaters,
                },
                services,
            )
            .map_err(E::CameraManUpdate)?;
        // CameraManager.Update8258464c..6464 copies GetFrame's matrix into
        // mCameraList[0] via BasicCamera.SetMatrix829d3158. Retain that snapshot
        // for the next census, including calls before the next camera update.
        // Local-skater notifications do not reset this BasicCamera matrix.
        self.simulation_camera_matrix = Some(man.lock().map_err(|_| E::OwnerLock)?.frame.matrix);
        if let Some(start) = camera_profile {
            let globals = camera_globals_time.unwrap_or_default();
            let subject = camera_subject_time.unwrap_or_default();
            let preupdate = camera_preupdate_time.unwrap_or_default();
            let graph = camera_graph_time.unwrap_or_default();
            eprintln!(
                "{}",
                format!(
                    "diagnostic camera manager: globals={}ms subject={}ms preupdate={}ms graph={}ms man={}ms",
                    globals.as_secs_f64() * 1000.0,
                    subject.saturating_sub(globals).as_secs_f64() * 1000.0,
                    preupdate.saturating_sub(subject).as_secs_f64() * 1000.0,
                    graph.saturating_sub(preupdate).as_secs_f64() * 1000.0,
                    start.elapsed().saturating_sub(graph).as_secs_f64() * 1000.0
                )
            );
        }
        self.producer_snapshot_ready = true;
        self.completed_projection = self.try_provisional_camera_projection().ok();
        self.completed_submission = self.try_camera_submission().ok();
        Ok(())
    }
}
