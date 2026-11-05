//! Physical skeleton IK data binding. This is the production skeleton's IK owner,
//! using authored physics volumes and the real reparented animation targets.
use super::animated_skeleton::AnimatedSkeleton;
use skate3_core::{
    animation::foot_ik::{
        drive::Geometry,
        post_contact::SettingsPost,
        post_physics,
        settings::Settings,
        state::{FootIkState, LIMBS, UpdateInput},
        status::BlendSettings,
        transforms::inverse_rigid,
        two_bone::AngleLimits,
    },
    physics::skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
};
use skate_data::{animation_frames::AnimationFrames, collections::Collections};

pub(crate) struct FootIk {
    pub state: FootIkState,
    geometry: Geometry,
    settings: Settings,
    post_settings: SettingsPost,
    bone_indices: [usize; 24],
    ///DataIn416 as UpdateIKData82BDC9E8 last copied it. UpdateWipeoutAdjust
    ///82BF1B90 reruns the contact pass without refreshing it.
    pub toe_contact_normal: [f32; 4],
}

/// Physical observations from the real processed state and body owner. All
///matrices remain in stock simulation coordinates, before renderer conversion.
pub(crate) struct PhysicalInput<'a> {
    pub state_id: u32,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2480: u32,
    pub contact_bone: usize,
    /// Skeleton12496, produced by the current board-update branch.
    pub physical_board: &'a Transform,
    /// DataIn416, SkeletonCollision1152's most vertical toe contact normal.
    pub toe_contact_normal: [f32; 4],
    /// Actual processed foot queries960/1008, not fabricated deck hits.
    pub current_contacts: [Option<[f32; 4]>; 2],
}

pub(crate) struct DriveOutput {
    pub frames: [Transform; 24],
}

impl FootIk {
    pub fn load(
        data: &Collections,
        animation: &AnimationFrames,
        skeleton: &AnimatedSkeleton,
    ) -> Result<Self, String> {
        //82BD3720 walks parents until it finds a represented physical part;
        //intermediate hierarchy bones do not become phantom physical joints.
        let mut parents = [None; 24];
        for (part, &bone) in skeleton.bone_indices.iter().enumerate() {
            let mut ancestor = *animation
                .parents
                .get(bone)
                .ok_or("IK bone exceeds stock hierarchy")?;
            let mut visited = vec![false; animation.parents.len()];
            while ancestor >= 0 {
                let index = ancestor as usize;
                if index >= visited.len() || visited[index] {
                    return Err(format!(
                        "Invalid stock IK hierarchy ancestor for part {part}"
                    ));
                }
                visited[index] = true;
                if let Some(parent) = skeleton.bone_indices.iter().position(|&bone| bone == index) {
                    parents[part] = Some(parent);
                    break;
                }
                ancestor = animation.parents[index];
            }
        }
        let geometry = Geometry::new(parents, &skeleton.physics_frames)?;
        geometry.validate_limbs(&LIMBS)?;
        Ok(Self {
            toe_contact_normal: [0.0; 4],
            state: FootIkState::default(),
            geometry,
            settings: load_settings(data)?,
            post_settings: SettingsPost {
                minimum_board_up: data.float(
                    "physics_animation",
                    "default",
                    "PostIKMinYAxisVal",
                )?,
                wipeout_height: data.float("animation", "default", "FeetRelativeHeightWipeout")?,
                riding_height: data.float("animation", "default", "FeetRelativeHeightOnDeck")?,
            },
            bone_indices: skeleton.bone_indices,
        })
    }

    pub fn enable_feet(&mut self, enabled: bool) {
        self.state.enable_feet(enabled);
    }

    ///Skeleton Fill82BE1D98..1E00 publishes each physical toe frame applied
    ///to the adjacent authored inverse-part translation, not its volume COM.
    pub fn physical_toe_positions(
        &self,
        record: &skate3_core::physics::skeleton_body::SkeletonPhysicalRecord,
    ) -> [[f32; 4]; 2] {
        use skate3_core::physics::skeleton_animation_record::transform_point;
        [
            transform_point(&record.pose[19], self.geometry.inverse_part_frames[20][3]),
            transform_point(&record.pose[15], self.geometry.inverse_part_frames[16][3]),
        ]
    }

    ///Call after native post-physics records/COM/error publication and board
    ///wobble, before SkeletonOutput::publish replaces the final pose buffers.
    pub fn post_physics(
        &mut self,
        body: &mut skate3_core::physics::skeleton_body::SkeletonBody,
        input: post_physics::Input<'_>,
    ) -> [bool; 4] {
        post_physics::update(
            &mut self.state,
            body,
            &self.geometry,
            &self.settings,
            &self.post_settings,
            input,
        )
    }

    pub fn update(
        &mut self,
        skeleton: &AnimatedSkeleton,
        globals: &[Transform],
        input: PhysicalInput<'_>,
    ) -> Result<DriveOutput, String> {
        let mut drives = skeleton.record.pose;
        //GeneralUpdate skips all IK stages for702 but still submits mapped
        //animation drives. Preserve histories across that skipped update.
        if input.state_id != 702 {
            let mut originals = skeleton.record.pose;
            for (part, &bone) in self.bone_indices.iter().enumerate() {
                originals[part] = *globals
                    .get(bone)
                    .ok_or("IK original joint is absent from the current pose")?;
            }
            //UpdateIKData's503 branch uses the animated board in world space
            //for both board inputs; every other state consumes distinct caches.
            let animated_world =
                compose_affine(&skeleton.roots.animation_to_world, &skeleton.record.pose[0]);
            let special_inverse = inverse_rigid(&animated_world);
            let (physical_board, contact_board, inverse_contact_board) = if input.state_id == 503 {
                (&animated_world, &animated_world, &special_inverse)
            } else {
                (
                    input.physical_board,
                    &skeleton.roots.board,
                    &skeleton.roots.inverse_board,
                )
            };
            let _ = self.state.update(
                UpdateInput {
                    flags_2468: input.flags_2468,
                    flags_2472: input.flags_2472,
                    flags_2480: input.flags_2480,
                    animation_to_world: &skeleton.roots.animation_to_world,
                    world_to_animation: &skeleton.roots.world_to_animation,
                    physical_board,
                    contact_board,
                    inverse_contact_board,
                    animation: &skeleton.record.pose,
                    original_animation: &originals,
                    targets: &skeleton.targets,
                    toe_contact_normal: input.toe_contact_normal,
                    contact_bone: input.contact_bone,
                    foot_bones: [self.bone_indices[15], self.bone_indices[19]],
                    current_contacts: input.current_contacts,
                },
                &self.geometry,
                &self.settings,
                &mut drives,
            );
        }
        Ok(DriveOutput { frames: drives })
    }
}

///82BED418 classF2BD493E resolves to physics_skeletonik/default. XML schema
/// gives the cached offsets; each vector retains its authored fourth word.
pub(crate) fn load_settings(data: &Collections) -> Result<Settings, String> {
    let vector = |name| {
        data.words::<4>("physics_skeletonik", "default", name)
            .map(|words| words.map(f32::from_bits))
    };
    let scalar = |name| data.float("physics_skeletonik", "default", name);
    let half_width = data.float("physicsdeck", "default", "DeckWidth")? * 0.5;
    let half_length = data.float("physicsdeck", "default", "DeckMidLength")? * 0.5;
    Ok(Settings {
        angle_limits: AngleLimits {
            minimum_degrees: data.float("physics_animation", "default", "IKMinAngle")?,
            maximum_degrees: data.float("physics_animation", "default", "IKMaxAngle")?,
        },
        blend: BlendSettings {
            hand_inner_padding: vector("IKOnPadding")?,
            hand_outer_padding: vector("IKOffPadding")?,
            external_blend_step: scalar("FeetIKMaxBlendDeltaOut")?,
            board_blend_step: scalar("FeetIKMaxBlendDelta")?,
        },
        post_ik_padding: vector("PostIKPadding")?,
        contact_bounds: vector("IKBoneOnDeckBBox")?,
        foot_on_deck_padding: vector("FootOnDeckPadding")?,
        wipeout_feet_offset: scalar("WipeoutFeetOffset")?,
        deck_half_width: half_width,
        deck_half_length: half_length,
        // Source uses FrontEndSize here, while the deck collider's fan loop
        // uses BackEndSize. They are distinct authored parameters.
        deck_total_half_length: data.float("physicsdeck", "default", "DeckFrontEndSize")?
            + half_length,
        deck_front_angle_degrees: data.float("physicsdeck", "default", "DeckFrontEndAngle")?,
    })
}
