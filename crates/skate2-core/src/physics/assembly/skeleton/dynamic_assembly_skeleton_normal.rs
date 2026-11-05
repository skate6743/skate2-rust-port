//! Source normal setup over the actual retained/admitted Skeleton Assembly.
//!
//! The wrapper owns its session and live material/Volume/collision-data state;
//! no body handles or mutable pools escape. It does not implement missing
//! island/body-owner registration or fabricate native Body/State references.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "normal skeleton awaits physical-player construction"
    )
)]
#[path = "dynamic_assembly_skeleton_force_physics.rs"]
mod force_physics;
#[path = "dynamic_assembly_skeleton_velocity.rs"]
pub(crate) mod velocity;

#[path = "../../player/wipeout/physical_player_post_wipeout_foot_ik_runtime.rs"]
pub(crate) mod post_wipeout_foot_live;
#[path = "../../player/wipeout/physical_player_post_wipeout_head_orientation.rs"]
pub(crate) mod post_wipeout_head_orientation;
#[path = "../../player/wipeout/physical_player_post_wipeout_pose.rs"]
pub(crate) mod post_wipeout_pose_live;
#[path = "../../player/wipeout/physical_player_post_wipeout_prefix.rs"]
pub(crate) mod post_wipeout_prefix;
use crate::{
    dynamic_assembly_session::{DynamicAssemblyAttachError, DynamicAssemblyDetachError},
    ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex},
    simulation_joint_pool::SimulationJointPoolError,
    simulation_rigid_body_pool::{SimulationRigidBodyHandle, SimulationRigidBodyPoolError},
    skeleton_assembly::{
        SkeletonAssemblyBindingError, SkeletonPreNormalAssemblySession,
        SkeletonPreNormalSimulationBackend, try_create_pre_normal_skeleton_assembly,
    },
    skeleton_body_attributes::{SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute},
    skeleton_body_definition::SkeletonBodyDefinition,
    skeleton_body_joints::SkeletonBodyJoints,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_culling::SkeletonCullingResource,
    skeleton_normal_joint_limits::{SkeletonJointLimitBinding, SkeletonNormalJointLimitsError},
    skeleton_runtime_state::{SkeletonRuntimeState, SkeletonRuntimeStateError},
};
use std::sync::Arc;

use super::part_transform::DynamicAssemblyPartTransformError;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonPlacementError {
    Detached,
    ForeignBackend,
    Part {
        part: usize,
        source: DynamicAssemblyPartTransformError,
    },
}

impl std::fmt::Display for SkeletonPlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detached => f.write_str("Skeleton placement requires an attached Assembly"),
            Self::ForeignBackend => f.write_str("Skeleton placement backend mismatch"),
            Self::Part { part, source } => write!(f, "Skeleton placement Part {part}: {source}"),
        }
    }
}
impl std::error::Error for SkeletonPlacementError {}

#[cfg(test)]
#[path = "dynamic_assembly_skeleton_normal_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "dynamic_assembly_skeleton_placement_tests.rs"]
pub(crate) mod placement_tests;

#[path = "dynamic_assembly_skeleton_drives.rs"]
pub(crate) mod drive_owner;

#[path = "dynamic_assembly_skeleton_history.rs"]
pub(crate) mod history_owner;

#[path = "dynamic_assembly_skeleton_collision.rs"]
pub(crate) mod collision_owner;

#[path = "dynamic_assembly_skeleton_reset.rs"]
pub(crate) mod reset_owner;

#[path = "dynamic_assembly_skeleton_general_update.rs"]
pub(crate) mod general_owner;

#[path = "dynamic_assembly_skeleton_animation_query.rs"]
pub(crate) mod animation_query;

#[path = "../../skeleton/movement/skeleton_effective_animation_transform.rs"]
pub(crate) mod effective_animation_transform;
#[path = "../../skeleton/movement/skeleton_part_transform_query.rs"]
pub(crate) mod part_transform_query;
#[path = "dynamic_assembly_skeleton_wipeout_torque.rs"]
pub(crate) mod wipeout_torque;

#[path = "../../skeleton/movement/skeleton_process_data_output_runtime.rs"]
pub(crate) mod process_data_output;
#[path = "../../skeleton/movement/skeleton_process_data_prefix.rs"]
pub(crate) mod process_data_prefix;
#[path = "dynamic_assembly_skeleton_ragdoll.rs"]
pub(crate) mod ragdoll_owner;

pub(crate) const NORMAL_COLLISION_ENABLED: u64 = 0x043f_9227_035d_e245;

#[derive(Debug)]
pub(crate) enum OffboardFootTargetNormalError {
    MissingIk,
    Normal(crate::skeleton_ik_state::set_target_normal::SetTargetNormalError),
}

pub(crate) struct SkeletonNormalAssembly {
    /// SAME Skeleton+4 plain animation Instance, moved from construction.
    animation_query_instance: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    session: SkeletonPreNormalAssemblySession,
    state: SkeletonRuntimeState,
    drives: Option<drive_owner::SkeletonDriveRuntime>,
    physics_history: crate::skeleton_physics_history::SkeletonPhysicsHistory,
    history_initialization: history_owner::SkeletonHistoryInitialization,
    history_frame: history_owner::SkeletonHistoryFrameState,
    collision_frame: collision_owner::SkeletonCollisionFrameState,
    animation_targets: crate::skeleton_animation_targets::SkeletonAnimationTargets,
    root_transforms: crate::skeleton_root_transforms::SkeletonRootTransforms,
    current_animation: crate::skeleton_current_animation::SkeletonCurrentAnimation,
    current_animation_attributes:
        Option<crate::skeleton_current_animation::SkeletonCurrentAnimationAttributes>,
    /// Actual constructor+14 Instance, consumed by explicit animation activation.
    post_physics_records: crate::skeleton_post_physics_records::SkeletonPostPhysicsRecords,
    general_update: crate::skeleton_general_update::SkeletonGeneralUpdate,
    /// Skeleton+3d70 ctor57394. Distinct from post-record HookToHips+3d80;
    /// no later writer has been recovered. Reset preserves this field.
    hips_offset: crate::skeleton_root_transforms::SkeletonRootVector,
    jump_adjustments: crate::skeleton_jump_adjustments::SkeletonJumpAdjustments,
    /// Skeleton+3e41; unknown until the selected post-IK constructor store.
    use_physics_truck_tilt: Option<bool>,
    physics_animation_state: Option<i32>,
    // Host sequencing evidence, not an invented native Skeleton field.
    retained_pose_set: bool,
}

#[derive(Debug)]
pub(crate) enum SkeletonNormalSetupError<A, C> {
    Detached,
    ForeignBackend,
    BindingAllocation,
    Joint(SimulationJointPoolError),
    Limits(SkeletonNormalJointLimitsError<C>),
    Body {
        part: usize,
        source: SimulationRigidBodyPoolError,
    },
    Attribute(A),
    RuntimeState(SkeletonRuntimeStateError),
}

impl<A: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display for SkeletonNormalSetupError<A, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detached => f.write_str("Skeleton normal setup requires an attached Assembly"),
            Self::ForeignBackend => f.write_str("Skeleton normal setup backend mismatch"),
            Self::BindingAllocation => {
                f.write_str("could not reserve actual Skeleton Joint bindings")
            }
            Self::Joint(source) => write!(f, "Skeleton Joint property: {source}"),
            Self::Limits(source) => write!(f, "Skeleton normal limits: {source:?}"),
            Self::Body { part, source } => write!(f, "Skeleton Part {part} inertia: {source}"),
            Self::Attribute(source) => write!(f, "Skeleton normal collision attribute: {source:?}"),
            Self::RuntimeState(source) => write!(f, "Skeleton collision state: {source}"),
        }
    }
}
impl<A: std::fmt::Debug, C: std::fmt::Debug> std::error::Error for SkeletonNormalSetupError<A, C> {}

impl SkeletonNormalAssembly {
    /// OffBoard's skate-root placement writes the existing general-update root.
    /// The caller supplies the source-ordered transform and owns its gates.
    pub(crate) fn set_offboard_skate_root_transform(
        &mut self,
        transform: crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) {
        self.general_update.set_animated_skate_root(transform);
    }

    pub(crate) fn try_set_offboard_foot_ik_target_normal(
        &mut self,
        left: bool,
        normal: crate::skeleton_root_transforms::SkeletonRootVector,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), OffboardFootTargetNormalError> {
        self.current_animation
            .ik_runtime_mut()
            .ok_or(OffboardFootTargetNormalError::MissingIk)?
            .try_set_foot_target_normal(left, normal, cvars)
            .map_err(OffboardFootTargetNormalError::Normal)
    }

    /// Native world setters 0x82b41250/70 retain the position unchanged.
    pub(crate) fn try_set_world_foot_ik_target(
        &mut self,
        left: bool,
        world_position: crate::skeleton_root_transforms::SkeletonRootVector,
        blend: f32,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation
            .ik_runtime_mut()
            .ok_or(crate::skeleton_current_animation::SkeletonIkUnconstructed)?
            .set_world_foot_target(left, world_position, blend);
        Ok(())
    }

    /// Native Skeleton left/right local foot setters 0x82b412f0/0x82b41348.
    /// Their world-to-animation matrix is the retained root-transform owner.
    pub(crate) fn try_set_local_foot_ik_target(
        &mut self,
        left: bool,
        world_position: crate::skeleton_root_transforms::SkeletonRootVector,
        blend: f32,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        let local = crate::skeleton_ik_state::external::transform_point(
            self.root_transforms.world_to_animation(),
            world_position,
        );
        self.current_animation
            .ik_runtime_mut()
            .ok_or(crate::skeleton_current_animation::SkeletonIkUnconstructed)?
            .set_local_foot_target(left, local, blend);
        Ok(())
    }

    /// `Skeleton::Init82b62a18..58`, after Drives and before Set.
    pub(crate) fn initialize_animation_buffers(&mut self) {
        self.animation_targets.initialize_skeleton_init_buffers();
    }
    /// `Skeleton::Init82b62ad0..62b28`, after both initial physics records.
    pub(crate) fn try_finish_init(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation.try_finish_skeleton_init()?;
        self.post_physics_records.disable_proxy_for_collision();
        // Folded 825187c0 at62b14 is BLR; the source then disables root volume.
        self.state.disable_root_collision();
        Ok(())
    }
    /// Ground.Update82da0e7c sets this byte only after the strict time gate.
    pub(crate) fn enable_physics_truck_tilt(&mut self) {
        self.use_physics_truck_tilt = Some(true);
    }
    pub(crate) fn use_physics_truck_tilt(&self) -> Option<bool> {
        self.use_physics_truck_tilt
    }
    /// Source `PhysicsGround::Enter` 0x82d99c64..9c74. Enable SAME IK before
    /// clearing truck tilt; do not reset IK contact or blend histories.
    pub(crate) fn try_ground_entry_ik_and_tilt(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation.try_enable_ik()?;
        self.use_physics_truck_tilt = Some(false);
        Ok(())
    }

    /// `Skeleton::EnableIK`, independently of Ground's truck-tilt store.
    pub(crate) fn try_enable_air_entry_ik(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation.try_enable_ik()
    }
    /// KnownAir.Enter writes truck tilt before invoking EnableIK.
    pub(crate) fn try_known_air_entry_ik_and_tilt(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.use_physics_truck_tilt = Some(false);
        self.current_animation.try_enable_ik()
    }
    /// DisableProxyVolume82b42068: byte3e43, folded empty OnEnd, then
    /// Body Part0 Volume flag bit0 and disable-count0. Reuse those same fields.
    pub(crate) fn disable_proxy_volume(&mut self) {
        self.post_physics_records.disable_proxy_for_collision();
        self.state.disable_root_collision();
    }

    /// `SetGroundCollision`82b543d0: normal collision before proxy/repeated root
    /// writes. Folded54400 is verified BLR and has no collaborator effects.
    pub(crate) fn try_set_ground_collision(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.state.try_set_up_normal_collision()?;
        self.post_physics_records.disable_proxy_for_collision();
        self.state.disable_root_collision();
        Ok(())
    }

    /// `SetAirCollision`82b4eea0: actual Body full-collision before proxy/root suffix.
    pub(crate) fn try_set_air_collision(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.state.try_set_up_full_collision()?;
        self.post_physics_records.disable_proxy_for_collision();
        self.state.disable_root_collision();
        Ok(())
    }

    /// `SetOffBoardCollision`82b4f038: Body group/userdata7 and
    /// SetUpNoCollision(false), three cvar-gated Part routes in source order,
    /// then the proxy flag, the conditional root enable and root userdata6.
    pub(crate) fn try_set_offboard_collision(
        &mut self,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), crate::skeleton_controller::SkeletonControllerError<SkeletonRuntimeStateError>>
    {
        use crate::skeleton_controller::SkeletonControllerError as E;
        const LIGHT_DMO: u32 = 0x11;
        const NORMAL: u32 = 6;
        const LW: u32 = 0x12;
        self.state.set_group_and_userdata(7);
        self.state.set_up_no_collision(false);
        if cvars
            .try_skeleton_ob_use_light_dmo_group()
            .map_err(E::Cvar)?
        {
            for part in [17, 21, 15, 19, 16, 20] {
                self.state
                    .try_part_collide_with(part, LIGHT_DMO)
                    .map_err(E::Collision)?;
            }
        }
        if cvars
            .try_skeleton_ob_have_body_collision()
            .map_err(E::Cvar)?
        {
            for part in [5, 4, 3, 9, 8, 7, 2, 1] {
                self.state
                    .try_part_collide_with(part, NORMAL)
                    .map_err(E::Collision)?;
            }
        }
        let core = if cvars
            .try_skeleton_ob_core_collide_with_lw()
            .map_err(E::Cvar)?
        {
            LW
        } else {
            LIGHT_DMO
        };
        for part in [6, 10, 11, 12, 13, 14, 23] {
            self.state
                .try_part_collide_with(part, core)
                .map_err(E::Collision)?;
        }
        self.post_physics_records.enable_proxy_for_collision();
        self.state.enable_root_collision_if_ready();
        self.state.set_root_user_data(6);
        Ok(())
    }

    /// `SetOffBoardAirCollision`82b4efd8: the `SetAirCollision` suffix after
    /// Body `SetUpRagdollCollision` instead of `SetUpFullCollision`.
    pub(crate) fn try_set_offboard_air_collision(
        &mut self,
    ) -> Result<(), SkeletonRuntimeStateError> {
        self.state.try_set_up_ragdoll_collision()?;
        self.post_physics_records.disable_proxy_for_collision();
        self.state.disable_root_collision();
        Ok(())
    }

    /// `CollideWithWorldOnly`82b54370 preserves its own Body failure prefix.
    pub(crate) fn try_collide_with_world_only(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.state.try_collide_with_world_only()?;
        self.post_physics_records.disable_proxy_for_collision();
        self.state.disable_root_collision();
        Ok(())
    }
    /// `Skeleton::CollideWithWorldOnlyOffboard` 0x82b542c0. The group and
    /// all-Part userdata stores precede Body NoCollision(true); the proxy and
    /// conditional root re-enable are its outer suffix on the SAME Skeleton.
    pub(crate) fn try_collide_with_world_only_offboard(&mut self) {
        self.state.set_group_and_userdata(3);
        self.state.set_up_no_collision(true);
        self.post_physics_records.enable_proxy_for_collision();
        self.state.enable_root_collision_if_ready();
        self.state.set_root_user_data(3);
    }
    pub(crate) fn retain_constructor_runtime(
        &mut self,
        constructor: &mut crate::skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    ) {
        // Move the existing native outer+14 Instance directly into its reader.
        // This host adapter construction does not reset the constructor-owned IK.
        if let Some(instance) = constructor.take_physics_animation_instance() {
            self.current_animation_attributes = Some(
                crate::skeleton_current_animation::SkeletonCurrentAnimationAttributes::from_constructor(
                    vault, instance,
                ),
            );
        }
        if let Some(instance) = constructor.take_animation_instance() {
            self.animation_query_instance = Some(instance);
        }
        if let Some(outer) = constructor.outer.as_mut() {
            if let Some(hips) = outer.hips_offset.take() {
                self.hips_offset = crate::skeleton_root_transforms::SkeletonRootVector::known(hips);
            }
            self.physics_animation_state = outer.physics_animation_state.take();
            self.use_physics_truck_tilt = outer.use_physics_truck_tilt.take();
        }
        if let Some(root) = constructor.root_transforms.take() {
            self.root_transforms = root;
        }
        if let Some(general) = constructor.general_update.take() {
            self.general_update = general;
        }
        if let Some(records) = constructor.post_physics_records.take() {
            self.post_physics_records = records;
        }
        if let Some(current) = constructor.current_animation.take() {
            self.current_animation = current;
        }
        if let Some(state) = constructor.state.as_mut() {
            self.post_physics_records.retain_constructor_errors(
                state
                    .linear_error
                    .take()
                    .expect("same early SkeletonState errors"),
            );
            let mut history = state
                .history
                .take()
                .expect("same early SkeletonState history");
            for part in 0..24 {
                history.store_body_part_mass(
                    part,
                    self.state.binding.body().parts[part].unscaled_part_mass,
                );
            }
            self.physics_history = history;
            self.state.proxy_volume_target = Some(state.proxy_target);
        }
    }

    pub(crate) fn retain_jump_adjustments(
        &mut self,
        adjustments: crate::skeleton_jump_adjustments::SkeletonJumpAdjustments,
    ) {
        assert!(
            self.jump_adjustments.x().is_err(),
            "constructor transfer must not replace initialized destination storage"
        );
        // The same constructor-owned allocation may already contain the
        // 82b57478/57480 stores, which precede Skeleton::Init and Body creation.
        // Moving it into the admitted assembly must preserve those values.
        self.jump_adjustments = adjustments;
    }
    /// Selected57478/57480, explicitly scheduled after actual IK construction.
    pub(crate) fn initialize_jump_adjustments_constructor(
        &mut self,
    ) -> Result<(), crate::skeleton_jump_adjustments::SkeletonJumpAdjustmentError> {
        self.jump_adjustments.initialize_constructor()
    }
    pub(crate) fn jump_adjustment_x(
        &self,
    ) -> Result<f32, crate::skeleton_jump_adjustments::SkeletonJumpAdjustmentError> {
        self.jump_adjustments.x()
    }
    pub(crate) fn jump_adjustment_z(
        &self,
    ) -> Result<f32, crate::skeleton_jump_adjustments::SkeletonJumpAdjustmentError> {
        self.jump_adjustments.z()
    }
    pub(crate) fn store_jump_adjustment_x(
        &mut self,
        value: f32,
    ) -> Result<(), crate::skeleton_jump_adjustments::SkeletonJumpAdjustmentError> {
        self.jump_adjustments.store_x(value)
    }
    pub(crate) fn store_jump_adjustment_z(
        &mut self,
        value: f32,
    ) -> Result<(), crate::skeleton_jump_adjustments::SkeletonJumpAdjustmentError> {
        self.jump_adjustments.store_z(value)
    }

    /// Fresh source definition/geometry owner, not a reset of an older runtime.
    pub(crate) fn try_create(
        body: Arc<SkeletonBodyDefinition>,
        joints: SkeletonBodyJoints,
        normal: Arc<SkeletonCullingResource>,
        collision: SkeletonCollisionData,
    ) -> Result<Self, SkeletonAssemblyBindingError> {
        let session = try_create_pre_normal_skeleton_assembly(body, joints, normal)?;
        let state =
            SkeletonRuntimeState::new(Arc::clone(session.template().collision()), collision);
        Ok(Self {
            current_animation: crate::skeleton_current_animation::SkeletonCurrentAnimation::new(
                Arc::clone(&state.binding.body().data),
            ),
            current_animation_attributes: None,
            animation_query_instance: None,
            post_physics_records:
                crate::skeleton_post_physics_records::SkeletonPostPhysicsRecords::new(),
            general_update: crate::skeleton_general_update::SkeletonGeneralUpdate::new(),
            hips_offset: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            jump_adjustments:
                crate::skeleton_jump_adjustments::SkeletonJumpAdjustments::uninitialized(),
            use_physics_truck_tilt: None,
            physics_animation_state: None,
            physics_history: crate::skeleton_physics_history::SkeletonPhysicsHistory::new(
                std::array::from_fn(|part| state.binding.body().parts[part].unscaled_part_mass),
            ),
            history_initialization: history_owner::SkeletonHistoryInitialization::NotStarted,
            history_frame: history_owner::SkeletonHistoryFrameState::new(),
            collision_frame: collision_owner::SkeletonCollisionFrameState::new(),
            animation_targets: crate::skeleton_animation_targets::SkeletonAnimationTargets::new(),
            root_transforms: crate::skeleton_root_transforms::SkeletonRootTransforms::new(),
            retained_pose_set: false,
            session,
            state,
            drives: None,
        })
    }

    pub(crate) fn session(&self) -> &SkeletonPreNormalAssemblySession {
        &self.session
    }
    pub(crate) fn state(&self) -> &SkeletonRuntimeState {
        &self.state
    }

    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn accept_mode_return_root(&mut self,
        root: crate::skeleton_root_transforms::SkeletonRootMatrix,
        board: crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) {
        self.root_transforms.accept_mode_return(root, board);
        self.general_update.clear_add_skateboard_error();
    }
    pub(crate) fn root_transforms(
        &self,
    ) -> &crate::skeleton_root_transforms::SkeletonRootTransforms {
        &self.root_transforms
    }

    pub(crate) fn try_reset_push_abort(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation.try_reset_push_abort()
    }

    pub(crate) fn current_animation(
        &self,
    ) -> &crate::skeleton_current_animation::SkeletonCurrentAnimation {
        &self.current_animation
    }

    pub(crate) fn post_physics_records(
        &self,
    ) -> &crate::skeleton_post_physics_records::SkeletonPostPhysicsRecords {
        &self.post_physics_records
    }
    pub(crate) fn general_update(&self) -> &crate::skeleton_general_update::SkeletonGeneralUpdate {
        &self.general_update
    }

    pub(crate) fn try_update_post_physics_records_with_flags<
        S: crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordServices,
        F: FnOnce() -> Result<u32, S::Error>,
    >(
        &mut self,
        time_delta: f32,
        services: &mut S,
        flags: F,
    ) -> Result<
        (),
        crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordError<
            S::Error,
            crate::reckoning_numeric::ReckoningHostNumericError,
            std::convert::Infallible,
        >,
    > {
        self.post_physics_records.try_update_with_flags(
            crate::skeleton_post_physics_records::SkeletonPostPhysicsRecordInput {
                physics: self.physics_history.record(),
                adjusted_volumes: &self
                    .animation_targets
                    .completed_volumes()
                    .expect("checked current animation mapping"),
                anim_to_world: self.root_transforms.animation_to_world(),
                proxy_target: self.state.proxy_volume_target,
                time_delta,
                flags_8c8: flags,
                collision: &self.state.collision,
            },
            services,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
    }

    pub(crate) fn try_update_volume_states(&mut self) -> Result<(), SkeletonRuntimeStateError> {
        self.state.try_update_volume_states()
    }

    pub(crate) fn initialize_current_animation(
        &mut self,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    ) -> bool {
        if self.current_animation_attributes.is_some() {
            return false;
        }
        if !self.current_animation.ik().initialized() {
            self.current_animation.initialize_ik_targets();
        }
        // Standalone leaf fixtures have no enclosing physical constructor.
        self.current_animation_attributes = Some(
            crate::skeleton_current_animation::SkeletonCurrentAnimationAttributes::bind(vault),
        );
        true
    }

    pub(crate) fn current_animation_initialized(&self) -> bool {
        self.current_animation_attributes.is_some()
    }

    pub(crate) fn try_set_skateboard_offset<
        C: crate::skeleton_skateboard_offset::SkeletonSkateboardOffsetCvars,
    >(
        &mut self,
        transform: &crate::animation::AnimationMatrix,
        cvars: &mut C,
    ) -> Result<(), C::Error> {
        self.current_animation
            .try_set_skateboard_offset(transform, cvars)
    }

    #[cfg(test)]
    pub(crate) fn try_process_current_animation<
        C: crate::skeleton_current_animation::SkeletonCurrentAnimationCvars,
    >(
        &mut self,
        pose: &[crate::animation::AnimationMatrix],
        input: &crate::skeleton_current_animation::SkeletonAdjustmentInput,
        passed_deck_zy_delta: &mut f32,
        cvars: &mut C,
    ) -> Result<
        (),
        crate::skeleton_current_animation::SkeletonCurrentAnimationError<
            crate::attribute_runtime::AttributeRuntimeError,
            C::Error,
        >,
    > {
        self.current_animation.try_process(
            &mut self.animation_targets,
            pose,
            input,
            passed_deck_zy_delta,
            &self.physics_history,
            self.root_transforms.animation_to_board(),
            &self.root_transforms.world_to_animation_lanes(),
            self.post_physics_records.centre_of_mass_velocity()[1],
            self.current_animation_attributes
                .as_mut()
                .expect("checked current animation initialization"),
            cvars,
            &mut crate::skeleton_physics_history::HostSkeletonPhysicsHistoryNumeric,
        )
    }

    pub(crate) fn root_transforms_mut(
        &mut self,
    ) -> &mut crate::skeleton_root_transforms::SkeletonRootTransforms {
        &mut self.root_transforms
    }

    /// Group1/all userdata1 precedes native admission (0x82b620f8/0x82b6210c).
    /// This invokes the existing Assembly transaction, not island registration.
    pub(crate) fn attach(
        &mut self,
        backend: &mut SkeletonPreNormalSimulationBackend,
    ) -> Result<(), DynamicAssemblyAttachError> {
        if self.session.is_attached() {
            return Err(DynamicAssemblyAttachError::AlreadyAttached);
        }
        self.state.set_group_and_userdata(1);
        self.session.attach(backend, 4)
    }

    pub(crate) fn detach(
        &mut self,
        backend: &mut SkeletonPreNormalSimulationBackend,
    ) -> Result<(), DynamicAssemblyDetachError> {
        self.try_validate_detach(backend)?;
        self.session.detach(backend)
    }

    /// Preserve the existing drives-first refusal before the session's own
    /// guards, without beginning Island spy removal or Assembly detachment.
    pub(crate) fn try_validate_detach(
        &self,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<(), DynamicAssemblyDetachError> {
        if self.drives.is_some() {
            return Err(DynamicAssemblyDetachError::SkeletonDrivesPresent);
        }
        self.session.try_validate_detach(backend)
    }

    fn validate_attached<A, C>(
        &self,
        backend: &SkeletonPreNormalSimulationBackend,
    ) -> Result<(), SkeletonNormalSetupError<A, C>> {
        let authority = self
            .session
            .attached_authority
            .ok_or(SkeletonNormalSetupError::Detached)?;
        if authority != backend.authority() {
            return Err(SkeletonNormalSetupError::ForeignBackend);
        }
        Ok(())
    }

    /// The property slice 0x82b621c4..0x82b621e4: admitted Joint spies, then
    /// Body-owned zero material selected only for Part0. Actual owner/Part
    /// registration preceding these stores is not implemented by this method.
    pub(crate) fn try_set_pre_normal_properties(
        &mut self,
        backend: &mut SkeletonPreNormalSimulationBackend,
    ) -> Result<(), SkeletonNormalSetupError<std::convert::Infallible, std::convert::Infallible>>
    {
        self.validate_attached(backend)?;
        for joint in &self.session.joints {
            backend
                .try_set_skeleton_joint_spy(
                    joint.joint.expect("attached session retains each Joint"),
                )
                .map_err(SkeletonNormalSetupError::Joint)?;
        }
        self.state.select_root_zero_material();
        Ok(())
    }

    /// Full represented `SetUpNormal` ordering, 0x82b50508..0x82b50720.
    /// Errors preserve the completed source prefix. Mutable owner copies are
    /// atomic host writes; concurrent native scalar-store observations are not
    /// claimed. Fresh local Vault selection occurs only after all mass stores.
    pub(crate) fn try_set_up_normal<R: SkeletonBodyAttributeRead, C>(
        &mut self,
        backend: &mut SkeletonPreNormalSimulationBackend,
        reader: &mut R,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), SkeletonNormalSetupError<R::Error, C>> {
        use SkeletonNormalSetupError as Error;
        self.validate_attached(backend)?;
        let binding = Arc::clone(self.session.template().collision());
        let body = binding.body();
        let mut joints = Vec::new();
        joints
            .try_reserve_exact(self.session.joints.len())
            .map_err(|_| Error::BindingAllocation)?;
        for (joint, limits) in self.session.joints.iter().zip(binding.joint_limits()) {
            joints.push(SkeletonJointLimitBinding {
                joint: joint.joint.expect("attached session retains each Joint"),
                limits: limits.clone(),
            });
        }
        backend
            .try_set_skeleton_normal_limits(&body.data, &body.attributes, &joints, cosine)
            .map_err(Error::Limits)?;
        self.state.select_normal_culling();
        self.state.set_group_and_userdata(6);
        self.state.collision.reset();
        let handles: [SimulationRigidBodyHandle; SKELETON_PART_COUNT] =
            std::array::from_fn(|part| {
                self.session.part_attachment.parts[part]
                    .body
                    .expect("attached session retains each body")
            });
        for angular in [true, false] {
            backend
                .try_zero_skeleton_drag(&handles, binding.inertias(), angular)
                .map_err(|(part, source)| Error::Body { part, source })?;
        }
        for (part, handle) in handles.iter().enumerate().skip(1) {
            self.state.select_normal_material(part);
            backend
                .try_set_skeleton_animated_inertia(
                    *handle,
                    &binding.inertias()[part],
                    body.parts[part].animated,
                )
                .map_err(|source| Error::Body { part, source })?;
            let index = SkeletonPartIndex::ALL[part];
            let drive = body.attributes.drives[part];
            self.state
                .collision
                .set_compliance(index, drive.collision_compliant);
            self.state
                .collision
                .set_priority(index, drive.collision_priority);
        }
        self.state
            .collision
            .set_compliance(SkeletonPartIndex::ALL[0], 1);
        let enabled = reader
            .byte(
                SkeletonBodyAttributeRoute::LocalSkeleton,
                NORMAL_COLLISION_ENABLED,
            )
            .map_err(Error::Attribute)?;
        if enabled != 0 {
            self.state
                .try_set_up_normal_collision()
                .map_err(Error::RuntimeState)?;
        } else {
            self.state.set_up_no_collision(true);
        }
        self.state.is_ragdoll = false;
        // Source folded OnEnd is exactly BLR, not an unreconstructed AI call.
        Ok(())
    }

    /// Source constructor's later `TeleportReset`, kept separate from normal setup.
    pub(crate) fn teleport_reset(&mut self) {
        self.state.teleport_reset();
    }
    pub(crate) fn set_teleporting_now(&mut self, value: bool) {
        self.general_update.set_teleporting_now(value);
    }

    pub(crate) fn clear_teleport_deck_zy(&mut self) {
        self.current_animation.clear_last_deck_zy_for_teleport();
    }

    pub(crate) fn clear_teleport_want_respawn(&mut self) {
        self.general_update.clear_want_respawn_after_fill();
    }

    pub(crate) fn set_teleport_follow_animation_data(&mut self) {
        self.physics_animation_state = Some(0);
    }

    pub(crate) fn try_reset_teleport_ik(
        &mut self,
    ) -> Result<(), crate::skeleton_current_animation::SkeletonIkUnconstructed> {
        self.current_animation.try_reset_ik_for_teleport()
    }

    /// The 24-Part prefix of `Skeleton::Set` (0x82b4c940..0x82b4ca2c), using
    /// the retained constructor physics-pose buffer, not a supplied replacement.
    /// Each `GetVolumeMtxFromPhysicsBuffer` (0x82b413a0) is immediately followed
    /// by the actual in-place Part setter. Both source normalization calls are
    /// retained; the setter also honors inverse-frame metadata and raw cache W.
    ///
    /// This does NOT perform Set's later two skeleton-drive hook placements or
    /// proxy-target update. A live mutable animation pose producer is also still
    /// separate. Errors preserve the completed ascending Part prefix; the pool
    /// owner must invalidate collision before entering this operation.
    pub(crate) fn try_place_parts_from_retained_physics_pose(
        &mut self,
        backend: &mut SkeletonPreNormalSimulationBackend,
    ) -> Result<(), SkeletonPlacementError> {
        match self.session.attached_authority {
            None => return Err(SkeletonPlacementError::Detached),
            Some(authority) if authority != backend.authority() => {
                return Err(SkeletonPlacementError::ForeignBackend);
            }
            Some(_) => {}
        }
        self.retained_pose_set = false;
        // Native captures the pose buffer before the loop. Retain that SAME
        // immutable source owner, without precomputing all transforms or cloning
        // pose records; calculation/store interleaving remains source ordered.
        let data = Arc::clone(&self.state.binding.body().data);
        for part in 0..SKELETON_PART_COUNT {
            let composed = crate::skateboard_spawn_math::skateboard_spawn_compose(
                &data.physics_pose()[part],
                &data.bone_to_volume()[part],
            );
            let transform = crate::world::orthonormalize_collision_affine(composed);
            self.session
                .try_set_part_transform_host(backend, part, transform)
                .map_err(|source| SkeletonPlacementError::Part { part, source })?;
        }
        Ok(())
    }
}

#[path = "../../skeleton/postphysics/skeleton_postphysics_preparation_live.rs"]
pub(crate) mod postphysics_preparation;

#[path = "../../skeleton/movement/skeleton_fill_live_runtime.rs"]
pub(crate) mod fill_live;

impl SkeletonNormalAssembly {
    /// `RequestPushExit82b3bb70` through the actual current IK owner.
    pub(crate) fn try_request_push_exit(&mut self) -> Result<(), &'static str> {
        self.current_animation
            .ik_runtime_mut()
            .ok_or("actual Skeleton IK")?
            .request_push_exit();
        Ok(())
    }
    /// Ground.Update82da1040, preserving all other `GeneralUpdate` flags.
    pub(crate) fn mark_add_skateboard_error(&mut self) {
        self.general_update.mark_add_skateboard_error();
    }
}
