//! The production Skeleton::ProcessData82BD8918 boundary. All borrows point
//! at the same owners subsequently used by Ground, the solve and pose output.
//! GeneralUpdate belongs to the later movement-state phase, not PlayerInput.
#[path = "skeleton_input_general.rs"]
mod general;
#[path = "skeleton_input_teleport.rs"]
mod teleport;

use super::{
    animated_skeleton::AnimatedSkeleton, animation_input::AnimationInput, foot_ik::FootIk,
};
use skate3_core::{
    animation::output::{NativeMatrix, attributes::AnimationAttribute},
    input::controller::ActionMap,
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        board_toolkit::BoardToolkit,
        skeleton_animation_record::{AnimationPartTransform as Transform, IDENTITY},
        skeleton_body::{SkeletonBody, SkeletonDrives, SkeletonPoseErrors},
        skeleton_landing::LandingInput,
        skeleton_output::correction::CorrectionState,
    },
    player::input_phase::{AnimationInputPacket, PhysicalPlayerInput, ProcessedPhysicsInput},
    riding::ground_correction_math::dot_product as dot3,
};

/// Original SkeletonCollision publications. The collision owner fills these
/// from actual solved contacts; there is deliberately no default observation.
pub(crate) struct CollisionInput {
    pub contact_4070: bool,
    pub has_pose_error_4077: bool,
    pub pose_error_16272: [f32; 4],
    pub partial_ragdoll: bool,
    pub drive_weight_4028: f32,
    ///SkeletonCollision1152, copied into IK DataIn416 by UpdateIKData.
    pub toe_contact_normal_1152: [f32; 4],
}

pub(crate) struct SkeletonOwners<'a> {
    pub animated: &'a mut AnimatedSkeleton,
    pub body: &'a mut SkeletonBody,
    pub drives: &'a mut SkeletonDrives,
    pub ik: &'a mut FootIk,
    pub animation_input: &'a mut AnimationInput,
    pub correction: &'a mut CorrectionState,
    pub pose_errors: &'a mut SkeletonPoseErrors,
}

pub(crate) struct SkeletonPoseInput<'a> {
    pub globals: &'a [NativeMatrix],
    pub attributes: &'a [AnimationAttribute],
    pub actions: &'a mut dyn ActionMap,
}

pub(crate) struct SkeletonInputRuntime {
    ///Skeleton16128, read from the actual deck before ProcessAnimAttributes.
    pub deck_velocity: [f32; 4],
    ///Skeleton16336 and16352, the animation root derivative and its history.
    pub root_velocity: [f32; 4],
    pub previous_root_position: [f32; 4],
    ///Skeleton12240 from the actual target update, not animation-to-world.
    pub animation_board_to_physics: Transform,
    ///Skeleton11632/11648/11664 from the three extra-target positions.
    pub extra_target_positions: [[f32; 4]; 3],
    ///Skeleton16208, authored hips translation cached by physical reset.
    pub reset_local_hips: [f32; 4],
    ///Skeleton12624, rewritten by ProcessData then modified in place by IK.
    ///The animation COM record retains its pre-IK copy independently.
    pub drive_frames: [Transform; 24],
    pub reenable_requested: bool,
    pub(crate) offboard_reset_pose_pending: bool,
    pub invalid_target_reset: bool,
    pub teleporting: bool,
    ///Skeleton16420, changed by graph force-mode requests and reset82BE3508.
    pub force_mode: u32,
    ///HeadTracking32..144 and160. Reset82BE3550..35A4 preserves its settings.
    pub head_tracking_history: [[f32; 4]; 8],
    pub head_tracking_active: bool,
    ///GrindAirAdjust ctor82D7116C initializes activated289=false.
    ///Only KnownAir Update82D35D30/Exit82D3597C subsequently write this flag.
    ///288, ctor82D71160; activated289 is a separate retained flag.
    pub grind_air_started: bool,
    pub grind_air_active: bool,
    pub grind_air_adjusting: bool,
    pub grind_air: skate3_core::physics::grind_air::GrindAir,
    grind_air_settings: Option<skate3_core::physics::grind_air::Settings>,
}

impl Default for SkeletonInputRuntime {
    fn default() -> Self {
        //Original Skeleton ctor82BD78xx/79xx zeroes these vector histories;
        //82BD7A78/84/8C clear16504/16506/16508. Frames are native identity.
        Self {
            deck_velocity: [0.; 4],
            root_velocity: [0.; 4],
            previous_root_position: [0.; 4],
            animation_board_to_physics: IDENTITY,
            extra_target_positions: [[0.; 4]; 3],
            reset_local_hips: [0.; 4],
            drive_frames: [IDENTITY; 24],
            reenable_requested: false,
            offboard_reset_pose_pending: false,
            invalid_target_reset: false,
            teleporting: false,
            force_mode: 0,
            head_tracking_history: [[0.0; 4]; 8],
            head_tracking_active: false,
            grind_air_started: false,
            grind_air_active: false,
            grind_air_adjusting: false,
            grind_air: Default::default(),
            grind_air_settings: None,
        }
    }
}

impl SkeletonInputRuntime {
    pub fn load(data: &skate_data::collections::Collections) -> Result<Self, String> {
        Ok(Self {
            grind_air_settings: Some(super::grind_air_settings::load(data)?),
            ..Self::default()
        })
    }
    ///Called by PlayerInputCallbacks::process_skeleton, after the actual
    ///BoardToolkit and line tests have been published. It does not advance
    ///Ground, rebuild a toolkit or submit a second physical simulation.
    pub fn process_data(
        &mut self,
        board: &mut BoardRuntime,
        toolkit: &BoardToolkit,
        packet: &AnimationInputPacket<'_>,
        _physical: &mut PhysicalPlayerInput,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        pose: SkeletonPoseInput<'_>,
        collision: &CollisionInput,
    ) -> Result<(), String> {
        let velocity = board.bodies()[BodyId::Deck.index()].rates.linear_velocity;
        self.deck_velocity = [velocity.x, velocity.y, velocity.z, 0.];
        //82ADF7B8: wake sleeping state2, preserving bit3, and clear cooldown
        //even for already active parts. Pool/list ownership is our engine's.
        for body in owners.body.bodies_mut() {
            if body.state_flags & 7 == 2 {
                body.state_flags = (body.state_flags & 8) | 4;
            }
            body.rates.cool_down = 0;
        }
        let input = &mut owners.animation_input;
        input.reset_processed();
        input.fields.flags2468 = p.flags_2468;
        input.fields.flags2472 = p.flags_2472;
        input.fields.flags2476 = p.flags_2476;
        input.fields.flags2484 = p.flags_2484;
        input.fields.flags2488 = p.flags_2488;
        input.extra.flags2480 = p.flags_2480;
        input.process(
            pose.attributes,
            pose.globals,
            packet.publication.timestep,
            packet.flags_10932,
            packet.publication.flags_10375_10496_10784[1] != 0,
            pose.actions,
        )?;
        publish_attribute_flags(input, p);
        p.spin_input_2672 = input.fields.spin;

        //These authored offset producers are separate from Ground riding.
        //Do not silently reuse a riding pose if a future state enables them.
        let reparented_hands = owners.animated.reparented_hand_indices();
        super::offboard::pose_adjust::update(owners.animated, pose.globals, reparented_hands, p)?;
        if p.flags_2476 & 0x100 == 0 {
            //82D712E0 clears result48/49 first and always writes adjusting290.
            //Inactive Ground leaves retained total offset/angle untouched.
            self.grind_air_adjusting = false;
            if self.grind_air_active {
                let settings = self
                    .grind_air_settings
                    .as_ref()
                    .ok_or("GrindAirAdjust stock settings were not loaded")?;
                self.grind_air_adjusting = super::skeleton_grind_air::update(
                    &mut self.grind_air,
                    settings,
                    toolkit.deck,
                    p,
                    owners.animated,
                    pose.globals,
                )?;
            }
        }
        let up = p.vectors_544_560_592_608[0].map(f32::from_bits);
        let com_velocity = p.vectors_544_560_592_608[3].map(f32::from_bits);
        let com_delta = std::array::from_fn(|i| {
            owners.body.record.centre_of_mass[i] - owners.body.record.positions[0][i]
        });
        let landing = LandingInput {
            filtered_state: p.filtered_state_2524,
            flags_2468: p.flags_2468,
            flags_2472: p.flags_2472,
            flags_2476: p.flags_2476,
            balance: input.fields.balance,
            physical_com_velocity_along_up: dot3(com_velocity, up),
            physical_com_height: dot3(com_delta, up),
            //process_pose replaces this with prior animation COM-currentboard.
            animation_com_height: owners.animated.record.centre_of_mass[1],
        };
        owners.animated.process_pose(
            pose.globals,
            landing,
            packet.publication.timestep,
            &mut p.flags_2468,
            &mut p.flags_2472,
            (p.flags_2476 & 0x10100 == 0x100).then_some((
                toolkit.deck,
                p.vectors_720_784_800_816_832_864[5].map(f32::from_bits),
                p.flags_2480,
                p.off_board_scalar_2832,
            )),
        )?;
        self.drive_frames = owners.animated.record.pose;
        p.board_at_y_delta_2768 = owners.animated.board_at_y_delta;
        p.flags_2472 = (p.flags_2472 & !(1 << 17)) | (u32::from(collision.contact_4070) << 17);
        if collision.has_pose_error_4077 {
            p.collision_pose_error_736 = collision.pose_error_16272.map(f32::to_bits);
        }
        p.animation_com_to_deck_752 = owners.animated.record.com_to_deck_world.map(f32::to_bits);
        p.animation_com_to_deck_delta_768 = owners
            .animated
            .record
            .com_to_deck_world_delta
            .map(f32::to_bits);
        //The scalar fields and ProcessedPhysicsInput share these native words.
        input.fields.flags2468 = p.flags_2468;
        input.fields.flags2472 = p.flags_2472;
        if self.reenable_requested {
            self.reenable_requested = false;
            owners.ik.enable_feet(true);
            general::restore_board_target_drives(owners.drives);
            //Native16416 is the prediction-supplied flag;16417 heading is
            //distinct and must not be cleared by the request's completion.
            owners.animated.roots.supplied_prediction = None;
        }
        Ok(())
    }
}

fn publish_attribute_flags(input: &AnimationInput, p: &mut ProcessedPhysicsInput) {
    // The scalar and extended attribute consumers write into the same
    // ProcessedPhysicsInput flag lanes that the later physics stages read.
    // Publish each lane explicitly at the native boundary:
    //
    //   AnimationInput.fields.flags2468 -> ProcessedPhysicsInput.flags_2468
    //   AnimationInput.fields.flags2472 -> ProcessedPhysicsInput.flags_2472
    //   AnimationInput.fields.flags2476 -> ProcessedPhysicsInput.flags_2476
    //   AnimationInput.extra.flags2480  -> ProcessedPhysicsInput.flags_2480
    //   AnimationInput.fields.flags2484 -> ProcessedPhysicsInput.flags_2484
    //   AnimationInput.fields.flags2488 -> ProcessedPhysicsInput.flags_2488
    //
    // This is publication only. No flags are synthesized, retained, or
    // filtered here; reset and attribute dispatch own those decisions.
    let scalar = &input.fields;
    p.flags_2468 = scalar.flags2468;
    p.flags_2472 = scalar.flags2472;
    p.flags_2476 = scalar.flags2476;
    p.flags_2484 = scalar.flags2484;
    p.flags_2488 = scalar.flags2488;
    p.flags_2480 = input.extra.flags2480;
}
