//! The game's single skater: stock graph animation and its physical skeleton.
//! Loading creates the real bodies from the animator's initial evaluated pose.
use super::{
    Skate3PhysicsState,
    animated_skeleton::AnimatedSkeleton,
    animation_feedback::AnimationFeedback,
    animation_input::AnimationInput,
    foot_ik::FootIk,
    foot_physical_output::FootPhysicalOutputs,
    ground_runtime::{GroundRuntime, GroundSettings, GroundState},
    player_input::PlayerInputRuntime,
    skeleton_body,
    skeleton_input_runtime::SkeletonInputRuntime,
    skeleton_output::SkeletonOutput,
};
use crate::skate3_native::{graph_runtime::StockGraphs, skater_animation::SkaterAnimation};

use skate3_core::physics::{
    board::BodyId,
    skeleton_animation_record::{IDENTITY, map_animation_parts},
    skeleton_body::{
        SkeletonBody, SkeletonCollisionMode, SkeletonDriveBatch, SkeletonDrives, SkeletonJoints,
    },
};
use skate_data::collections::Collections;
use std::path::Path;


pub(crate) struct Skate3ControllerState {
    /// The actual input-phase reset callback completed in this source tick.
    pub teleported_this_tick: bool,
    pub climbing: super::climbing::Runtime,
    /// Completed physical pose in native animation space, read by rendering.
    pub render_pose: Vec<skate3_core::animation::output::NativeMatrix>,
    pub pose_generation: u64,
    pub centre_of_mass_filter: skate3_core::physics::centre_of_mass_filter::CentreOfMassFilter,
    pub centre_of_mass_output: skate3_core::physics::centre_of_mass_filter::CentreOfMassOutput,
    /// TU3's shove probe (0x82d7a440): its inputs from the previous output, the host's pedestrians
    /// and a strike for the game.
    pub shove: super::shove_probe::ShoveProbe,
    /// This frame's solver contacts with host world bodies, for their own collision responses.
    pub(crate) host_contacts: Vec<super::skeleton_feedback::HostContact>,
    pub animation: SkaterAnimation,
    pub animated_skeleton: AnimatedSkeleton,
    pub skeleton_air: super::skeleton_air::SkeletonAir,
    pub air_reckoning: super::air_reckoning::AirReckoning,
    pub air_state: skate3_core::air::state::PhysicsAirState,
    pub air_settings: super::air_phase::AirSettings,
    pub known_air: super::known_air::KnownAir,
    pub biped_air: super::biped_air::BipedAir,
    pub landing_on_deck: super::landing_on_deck::Runtime,
    pub landing_deck: super::offboard::landing_deck::Owner,
    pub grind_trick: super::grind_trick::GrindTrick,
    pub ground_animation: super::ground_animation::GroundAnimationRuntime,
    pub ground_animation_settings: super::ground_animation::GroundAnimationSettings,
    pub revert_state: super::revert_state::RevertState,
    pub slide_state: super::slide_state::SlideState,
    pub trajectory: super::air_trajectory::AirTrajectoryRuntime,
    pub grind_camera: super::grind_camera::GrindCamera,
    pub grind: super::grind::Runtime,
    pub footplant: super::footplant::Footplant,
    pub boneless: super::boneless::Boneless,
    pub handplant: super::handplant::Handplant,
    pub wipeout: super::wipeout::Wipeout,
    pub wipeout_state: super::wipeout_states::WipeoutState,
    pub(super) respawn: super::respawn::Runtime,
    pub teleport_state: super::teleport_state::Runtime,
    pub skeleton: SkeletonBody,
    pub skeleton_joints: SkeletonJoints,
    pub(crate) mod_part_overrides: std::collections::BTreeMap<usize,(String,crate::skate3_native::modding::extensions::PartOverride)>,
    pub(crate) mod_contact_frame: crate::skate3_native::modding::player_physics::ContactFrame,
    pub(crate) mod_joint_overrides: std::collections::BTreeMap<usize,(String,crate::skate3_native::modding::extensions::JointOverride)>,
    pub skeleton_drives: SkeletonDrives,
    pub skeleton_collision: SkeletonCollisionMode,
    pub collision_feedback: skate3_core::physics::skeleton_body::SkeletonCollisionFeedback,
    pub pose_errors: skate3_core::physics::skeleton_body::SkeletonPoseErrors,
    pub collision_pose_error: [f32; 4],
    /// Skeleton16288/16304, retained by the post-solver response producer.
    pub collision_extra_errors: [[f32; 4]; 2],
    ///Skeleton16384 is published by the completed pose-error response. Wipeout
    ///runs after that publication; no fabricated pre-solve measurement exists.
    pub collision_maximum_error: Option<f32>,
    pub solved_drives: Option<SkeletonDriveBatch>,
    pub foot_ik: FootIk,
    pub foot_physical: FootPhysicalOutputs,
    /// Origin paired with the completed native hips trajectory, not the later solved pose.
    pub hips_ground_start: [f32; 4],
    pub player_input: PlayerInputRuntime,
    pub player_state: super::player_state::PlayerState,
    pub skeleton_input: SkeletonInputRuntime,
    pub ground: GroundState,
    pub ground_runtime: GroundRuntime,
    pub ground_profiles: super::ground_runtime::GroundProfiles,
    pub ground_settings: std::sync::Arc<GroundSettings>,
    pub ground_lifecycle: super::ground_phase::GroundLifecycle,
    pub biped_ground: super::biped_ground::Owner,
    pub offboard_contact: super::offboard::contact_toolkit::Owner,
    pub offboard_air_selector: super::offboard::air_selector::AirSelector,
    pub offboard_feet: skate3_core::player::offboard::board_possession::manager::State,
    pub offboard_grab: super::biped_ground::grab_runtime::Owner,
    /// PhysState_Skitching (104) and PhysicsGround's grabbable-object check settings (absent
    /// without the game's skitching collection).
    pub skitching: super::skitching::Skitching,
    pub ground_grab: Option<super::ground_grab::Settings>,
    /// This tick's Skitching FillPhysOut values that the PhysOut record does not hold.
    pub skitch_output: Option<super::skitching::Output>,
    /// Skitching's grab manager messages, sent to the host world after the tick.
    pub skitch_messages: Vec<super::skitching::Message>,
    /// PC port: the pushing state's follow (TU3 Skeleton::UpdateOffBoardPushing's skeleton COM
    /// target and turn), set by the host's pushing driver for the next BipedGround update.
    pub pushing_follow: Option<super::biped_ground::PushingFollow>,
    pub animation_input: AnimationInput,
    pub animation_feedback: AnimationFeedback,
    pub physical_feedback: skate3_core::animation::physical_feedback::PhysicalFeedback,
    pub landing_quality: skate3_core::animation::landing_quality::Output,
    pub landing_quality_settings: skate3_core::animation::landing_quality::Settings,
    pub skeleton_output: SkeletonOutput,
    pub skateboard_controller: super::skateboard_controller::SkateboardController,
    pub board_possession: super::offboard::board_manager::Owner,
    pub board_possession_live: super::offboard::board_manager::runtime::LiveState,
}

impl Skate3ControllerState {
    /// Prepare all settings before committing; never replace active physics state.
    pub(crate) fn reload_difficulty(&mut self, data: &Collections) -> Result<(), String> {
        let profiles = super::ground_runtime::GroundProfiles::load(data)?;
        let ground_settings = profiles.select(self.player_input.processed.state_variant_index_2528,
            self.player_input.processed.surface_mode_2540.clamp(1, 5))?;
        let ground = GroundState::load(data, "test", true)?;
        let air = super::air_phase::AirSettings::load(data)?;
        let reckoning = super::air_reckoning::AirReckoning::load(data)?;
        let known = super::known_air::KnownAir::load(data)?;
        let wipeout = super::wipeout::Wipeout::load(data)?;
        let grind = super::grind::Runtime::load(data)?;
        let jumps = super::ground_animation::GroundAnimationSettings::load(data)?;
        let animation = AnimationInput::load(data, &self.animation.evaluator.frames, "test")?;
        let player = PlayerInputRuntime::load(data)?;
        self.ground_profiles = profiles; self.ground_settings = ground_settings;
        self.ground.adopt_mode_settings(ground);
        self.air_settings = air;
        self.air_reckoning.adopt_mode_settings(reckoning);
        self.known_air.adopt_mode_settings(known);
        self.wipeout.adopt_mode_settings(wipeout);
        self.grind.adopt_mode_settings(grind);
        self.ground_animation_settings = jumps;
        self.animation_input.adopt_mode_settings(animation);
        self.player_input.player.state_variants_1408 = player.player.state_variants_1408;
        Ok(())
    }

    pub(crate) fn travel_to(&mut self, transform: [[f32; 4]; 4]) -> Result<(), String> {
        self.travel(transform, None)
    }

    pub(crate) fn travel(
        &mut self,
        transform: [[f32; 4]; 4],
        velocity: Option<[f32; 3]>,
    ) -> Result<(), String> {
        self.player_input.request_teleport_ex(transform, velocity)?;
        self.teleport_state.request_manual(transform, true);
        Ok(())
    }
    pub fn load(
        asset_root: &Path,
        graphs: &StockGraphs,
        physics: &Skate3PhysicsState,
        mode: &str,
    ) -> Result<Self, String> {
        Self::load_for_world(asset_root, graphs, physics, mode, None)
    }

    pub(crate) fn load_for_world(
        asset_root: &Path, graphs: &StockGraphs, physics: &Skate3PhysicsState, mode: &str,
        source: Option<std::sync::Arc<crate::skate3_native::skater_animation::AnimationSource>>,
    ) -> Result<Self, String> {
        let data = crate::skate3_native::custom_difficulty::load_collections(asset_root)?;
        let animation_metadata = animation_metadata(asset_root)?;
        // The host's current character is a custom skater with no pro selector
        // or equipped physical hat. These are profile choices, not force values.
        let mut animation = match source {
            Some(source) => SkaterAnimation::from_source(&data, graphs, b"", source)?,
            None => SkaterAnimation::load(asset_root, &data, graphs, b"")?,
        };
        let initial_hierarchy = animation.evaluate_initial_pose()?;
        let mut animated_skeleton =
            AnimatedSkeleton::load(asset_root, &data, &animation.evaluator.frames, false)?;
        let initial_parts = map_animation_parts(
            &initial_hierarchy,
            &animated_skeleton.bone_indices,
            &animated_skeleton.physics_frames,
        )?;
        let deck = physics.board.part_transforms()[BodyId::Deck.index()];
        let mut spawn = IDENTITY;
        for (column, axis) in deck.basis.columns.iter().enumerate() {
            spawn[column][..3].copy_from_slice(axis);
        }
        spawn[3] = [
            deck.translation.x,
            deck.translation.y,
            deck.translation.z,
            0.0,
        ];
        let mut skeleton = skeleton_body::load(
            asset_root,
            &data,
            &animation.evaluator.frames.source_sha256,
            &initial_parts,
            spawn,
            physics.settings.step.simulation,
            None,
        )?;
        animated_skeleton
            .roots
            .reset_initial_alignment(skeleton.animation_to_world);
        animated_skeleton.board_frames.reset(spawn);
        for _ in 0..2 {
            animated_skeleton.board_frames.publish_centre_of_mass(
                skeleton.record.centre_of_mass,
                physics.settings.step.simulation.time_step,
                0,
            );
        }
        animated_skeleton
            .board_frames
            .publish_local_observations(&animated_skeleton.roots, &spawn);
        let animation_input = AnimationInput::load(&data, &animation.evaluator.frames, mode)?;
        let skeleton_joints = skeleton_body::load_joints(
            asset_root,
            &data,
            &animation.evaluator.frames.source_sha256,
            &initial_hierarchy,
            &animation.evaluator.frames.parents,
            &animated_skeleton.bone_indices,
        )?;
        let mut skeleton_drives = skeleton_body::load_drives(
            asset_root,
            &data,
            &animation.evaluator.frames.source_sha256,
            &initial_hierarchy,
            &animated_skeleton.bone_indices,
            &initial_parts,
            &skeleton_joints,
            skeleton.animation_to_world,
            spawn,
            physics.settings.step.simulation,
        )?;
        // Reset82BD9990 finishes by publishing both COM targets to the
        // actual extra bodies. These bodies subsequently share the solver.
        let initial_targets = skeleton_drives.targets.update_extra_targets(
            &mut skeleton,
            &animated_skeleton.board_frames.com_frame,
            &animated_skeleton.board_frames.lifted_com_frame,
        );
        let foot_ik = FootIk::load(&data, &animation.evaluator.frames, &animated_skeleton)?;
        let skeleton_output =
            SkeletonOutput::load(&data, &animation.evaluator.frames, &animated_skeleton)?;
        // Actor82591448 passes network || ghost for self-pair suppression. An online
        // game's value is applied by Skate3Player before the first step (set_cull_all_self_pairs).
        let skeleton_collision = skeleton_body::load_collision(
            asset_root,
            &data,
            &animation.evaluator.frames.source_sha256,
            false,
        )?;
        let wipeout_state = super::wipeout_states::WipeoutState::load(
            &data,
            asset_root,
            &animation.evaluator.frames.source_sha256,
        )?;
        let respawn = super::respawn::Runtime::load(&data, spawn, animation.checkpoint_stance())?;
        let mut trajectory = super::air_trajectory::AirTrajectoryRuntime::load(&data)?;
        trajectory.bind_grind_world(std::sync::Arc::clone(&physics.grind_world));
        let mut skeleton_input = SkeletonInputRuntime::load(&data)?;
        skeleton_input.drive_frames = initial_parts;
        skeleton_input.extra_target_positions = [
            initial_targets.com,
            initial_targets.lifted_com,
            initial_targets.following_com,
        ];
        Ok(Self {
            teleported_this_tick: false,
            respawn,
            climbing: super::climbing::Runtime::load(asset_root, &animation.evaluator.frames.bone_names)?,
            render_pose: initial_hierarchy,
            pose_generation: 0,
            centre_of_mass_filter: Default::default(),
            shove: Default::default(),
            host_contacts: Vec::new(),
            // PhysOut reset82DE53F0 clears these observations before first output.
            centre_of_mass_output: skate3_core::physics::centre_of_mass_filter::CentreOfMassOutput {
                velocity: [0.0; 4],
                acceleration: [0.0; 4],
                position: [0.0; 4],
            },
            animation,
            landing_quality: Default::default(),
            landing_quality_settings: super::landing_quality::load(&data)?,
            animated_skeleton,
            skeleton_air: super::skeleton_air::SkeletonAir::load(&data)?,
            air_reckoning: super::air_reckoning::AirReckoning::load(&data)?,
            air_state: Default::default(),
            air_settings: super::air_phase::AirSettings::load(&data)?,
            known_air: super::known_air::KnownAir::load(&data)?,
            biped_air: super::biped_air::BipedAir::load(&data)?,
            landing_on_deck: super::landing_on_deck::Runtime::load(&data)?,
            landing_deck: super::offboard::landing_deck::Owner::load(&data)?,
            grind_trick: Default::default(),
            ground_animation: Default::default(),
            ground_animation_settings: super::ground_animation::GroundAnimationSettings::load(&data)?,
            revert_state: super::revert_state::RevertState::load(&data)?,
            slide_state: super::slide_state::SlideState::load(&data)?,
            trajectory,
            grind_camera: super::grind_camera::GrindCamera::default(),
            grind: super::grind::Runtime::load(&data)?,
            footplant: super::footplant::Footplant::load(&data)?,
            boneless: super::boneless::Boneless::load(&data)?,
            handplant: super::handplant::Handplant::load(&data)?,
            wipeout: super::wipeout::Wipeout::load(&data)?,
            wipeout_state,
            teleport_state: super::teleport_state::Runtime::new(
                #[cfg(any())]
                super::teleport_state::Checkpoint {
                    transform: spawn,
                    on_board: true,
                },
            ),
            skeleton,
            skeleton_joints,
            mod_joint_overrides: Default::default(),
            mod_contact_frame: Default::default(),
            mod_part_overrides: Default::default(),
            skeleton_drives,
            collision_feedback: skeleton_body::load_feedback(&data, skeleton_collision.settings)?,
            skeleton_collision,
            pose_errors: skate3_core::physics::skeleton_body::SkeletonPoseErrors {
                targets: [
                    initial_targets.com,
                    initial_targets.lifted_com,
                    initial_targets.following_com,
                ],
                ..Default::default()
            },
            collision_pose_error: [0.0; 4],
            collision_extra_errors: [[0.0; 4]; 2],
            collision_maximum_error: None,
            solved_drives: None,
            foot_ik,
            foot_physical: FootPhysicalOutputs::load(&data)?,
            hips_ground_start: [0.0; 4],
            player_input: PlayerInputRuntime::load(&data)?,
            player_state: super::player_state::PlayerState::load(&data, mode)?,
            skeleton_input,
            ground: GroundState::load(&data, mode, true)?,
            ground_runtime: GroundRuntime::load(&data)?,
            ground_profiles: super::ground_runtime::GroundProfiles::load(&data)?,
            ground_settings: std::sync::Arc::new(GroundSettings::load(&data, mode, "smooth")?),
            ground_lifecycle: super::ground_phase::GroundLifecycle::new(),
            biped_ground: super::biped_ground::Owner::load(&data, &animation_metadata)?,
            offboard_contact: Default::default(),
            offboard_air_selector: super::offboard::air_selector::AirSelector::new(
                super::offboard::air_selector::Settings::load(&data)?,
            ),
            offboard_feet: Default::default(),
            offboard_grab: Default::default(),
            skitching: super::skitching::Skitching::load(&data),
            ground_grab: super::ground_grab::Settings::load(&data).ok(),
            skitch_output: None,
            skitch_messages: Vec::new(),
            pushing_follow: None,
            animation_input,
            animation_feedback: AnimationFeedback::load(&data)?,
            physical_feedback: super::animation_phase::initial_feedback(),
            skeleton_output,
            skateboard_controller: super::skateboard_controller::SkateboardController::new(),
            board_possession: super::offboard::board_manager::Owner::load(&data)?,
            board_possession_live: super::offboard::board_manager::runtime::LiveState::load(
                &data, physics,
            )?,
        })
    }
}

/// The animation banks' metadata: read once for the linked game (every new player on a location
/// change reused to read both banks again), from `root` otherwise.
fn animation_metadata(root: &Path) -> Result<std::sync::Arc<skate_data::animation_metadata::AnimationMetadata>, String> {
    let load = || -> Result<_, String> {
        Ok(std::sync::Arc::new(skate_data::animation_banks::AnimationBanks::load(root)?.metadata()?))
    };
    if skate_data::asset_io::is_linked_root(root) {
        static METADATA: std::sync::OnceLock<std::sync::Arc<skate_data::animation_metadata::AnimationMetadata>> =
            std::sync::OnceLock::new();
        return crate::skate3_native::physics::player::linked_once(&METADATA, load).map(std::sync::Arc::clone);
    }
    load()
}
