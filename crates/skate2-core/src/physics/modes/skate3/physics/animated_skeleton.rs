//! Stock skeleton-to-physics ownership. Animation globals and real physical
//! observations enter here; native calculations and history stay in core.
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId,
        board_runtime::BoardRuntime,
        skeleton_animation_record::{
            AnimationPartTransform, IDENTITY, SkeletonAnimationMasses, SkeletonAnimationRecord,
            compose_affine, map_animation_parts, physics_bone_frame,
        },
        skeleton_board_frames::SkeletonBoardFrames,
        skeleton_board_offset::SkateboardOffset,
        skeleton_landing::{LandingAdjustment, LandingInput, LandingSettings},
        skeleton_motion::SkeletonMotion,
        skeleton_root::SkeletonRootFrames,
    },
    point_graph::PointGraph,
};
use skate_data::{
    animation_frames::AnimationFrames, collections::Collections, physics_skeleton::PhysicsSkeleton,
};
use std::path::Path;

pub(crate) struct AnimatedSkeleton {
    pub record: SkeletonAnimationRecord,
    pub roots: SkeletonRootFrames,
    pub board_frames: SkeletonBoardFrames,
    pub board_offset: SkateboardOffset,
    pub landing: LandingAdjustment,
    pub targets: [AnimationPartTransform; 4],
    pub masses: SkeletonAnimationMasses,
    pub bone_indices: [usize; 24],
    pub physics_frames: [AnimationPartTransform; 24],
    pub animation_board: AnimationPartTransform,
    ///Skeleton12560, copied BEFORE board offsets and IK by82BD8CB0..8D10.
    pub unadjusted_board: AnimationPartTransform,
    landing_on_board_blend: PointGraph<8>,
    pub animation_hips: AnimationPartTransform,
    pub motion: SkeletonMotion,
    pub board_at_y_delta: f32,
    target_bones: [usize; 4],
    landing_settings: LandingSettings,
}
impl AnimatedSkeleton {
    ///Existing stock82BED688 target mappings; left and right reparented hands.
    pub(crate) fn reparented_hand_indices(&self) -> [usize; 2] {
        [self.target_bones[2], self.target_bones[3]]
    }

    pub fn load(
        asset_root: &Path,
        data: &Collections,
        animation: &AnimationFrames,
        head_has_hat: bool,
    ) -> Result<Self, String> {
        let skeleton = PhysicsSkeleton::load(
            &asset_root.join("private/stock/physics-skeletons.json"),
            &animation.source_sha256,
            "PHYS_TPOSE",
        )?;
        if skeleton.bones.len() != 24 {
            return Err("Stock skater physics skeleton must contain24 parts".into());
        }
        let lookup = |name: &str| {
            animation
                .bone_names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    format!("Stock physics bone {name} is absent from the animation hierarchy")
                })
        };
        let mut bone_indices = [0; 24];
        let mut physics_frames = [IDENTITY; 24];
        let mut sizes = [Vector3::ZERO; 24];
        let mut shapes = [0; 24];
        for (part, bone) in skeleton.bones.iter().enumerate() {
            bone_indices[part] = lookup(&bone.name)?;
            let translation = std::array::from_fn(|i| f32::from_bits(bone.words[16 + i]));
            physics_frames[part] = physics_bone_frame(bone.rotation(), translation);
            let [x, y, z] = bone.size();
            sizes[part] = Vector3::new(x, y, z);
            shapes[part] = data.words::<6>(
                "physics_skeleton_bones",
                "default",
                &format!("PART_{}", bone.name),
            )?[3];
        }
        let mut target_bones = [0; 4];
        // Native82BED688 exact order and names. These are board-parented
        // targets; do not replace them with the physical foot/hand indices.
        for (i, name) in [
            "LeftToeBase_Reparented",
            "RightToeBase_Reparented",
            "LeftHand_Reparented",
            "RightHand_Reparented",
        ]
        .iter()
        .enumerate()
        {
            target_bones[i] = lookup(name)?;
        }
        Ok(Self {
            record: SkeletonAnimationRecord::default(),
            roots: SkeletonRootFrames::default(),
            board_frames: SkeletonBoardFrames::default(),
            board_offset: SkateboardOffset::default(),
            landing: LandingAdjustment::default(),
            targets: [IDENTITY; 4],
            masses: SkeletonAnimationMasses::from_bone_data(sizes, shapes, head_has_hat),
            bone_indices,
            physics_frames,
            animation_board: IDENTITY,
            unadjusted_board: IDENTITY,
            landing_on_board_blend: {
                let words = data
                    .words::<20>("physics_animation", "default", "LandingOnDeckBLendVsTime")?
                    .map(f32::from_bits);
                PointGraph {
                    x: words[4..12].try_into().unwrap(),
                    y: words[12..20].try_into().unwrap(),
                }
            },
            animation_hips: IDENTITY,
            motion: SkeletonMotion::default(),
            board_at_y_delta: 0.0,
            target_bones,
            landing_settings: load_landing_settings(data)?,
        })
    }

    /// ProcessData's mapped pose, target frames, landing adjustment, board offset,
    /// COM and final board cache. The caller supplies actual processed physical
    /// observations; this owner never substitutes the board body's COM for them.
    /// Grind/landing-on-board/offboard offset producers refresh board_offset
    /// before this call's landing stage when their native conditions require it.
    pub fn process_pose(
        &mut self,
        globals: &[AnimationPartTransform],
        mut landing: LandingInput,
        dt: f32,
        flags_2468: &mut u32,
        flags_2472: &mut u32,
        landing_on_board: Option<(AnimationPartTransform, [f32; 4], u32, f32)>,
    ) -> Result<(), String> {
        let trajectory = globals.first().ok_or("Animation has no trajectory bone")?;
        self.motion
            .process_trajectory(trajectory, &self.roots.animation_to_world, dt);
        let mut parts = map_animation_parts(globals, &self.bone_indices, &self.physics_frames)?;
        self.unadjusted_board = parts[0];
        SkeletonMotion::publish_unadjusted_board(&parts[0], flags_2472);
        for (target, part) in [15, 19, 3, 7].into_iter().enumerate() {
            let bone = globals
                .get(self.target_bones[target])
                .ok_or("Missing animation IK target bone")?;
            self.targets[target] = compose_affine(bone, &self.physics_frames[part]);
        }
        landing.animation_com_height = self.record.centre_of_mass[1] - parts[0][3][1];
        if let Some((deck, offset, flags2480, time)) = landing_on_board {
            if let Some(adjustment) =
                skate3_core::physics::skeleton_landing_on_board::pose_adjustment(
                    &self.roots.world_to_animation,
                    &deck,
                    &parts[0],
                    offset,
                    self.board_frames.com_velocity[1],
                    flags2480,
                    time,
                    &self.landing_on_board_blend,
                )
            {
                self.board_offset.refresh_transform(adjustment);
            }
        }
        self.landing
            .update(landing, &self.landing_settings, &mut self.board_offset);
        self.board_offset.update(&mut parts[0], &mut self.targets);
        self.record
            .update(&parts, &self.roots.animation_to_board, &self.masses);
        // ProcessData82BD8E04..8E10 writes local board11728 AFTER COM.
        // World animation target15952 belongs to the later board-update phase.
        self.animation_board = parts[0];
        self.animation_hips = parts[23];
        self.board_at_y_delta = self.motion.publish_adjusted_board(&parts[0], flags_2468);
        Ok(())
    }

    /// UpdateRootTransforms is called by the native ground/animated board update
    /// after ProcessData. Its result becomes the next ProcessData COM frame.
    pub fn update_roots(
        &mut self,
        board: &BoardRuntime,
        reckoning: &AnimationPartTransform,
        dt: f32,
    ) {
        let deck = board.part_transforms()[BodyId::Deck.index()];
        let velocity = board.bodies()[BodyId::Deck.index()].rates.linear_velocity;
        let mut transform = IDENTITY;
        for (axis, column) in deck.basis.columns.iter().enumerate() {
            transform[axis][..3].copy_from_slice(column);
        }
        transform[3] = [
            deck.translation.x,
            deck.translation.y,
            deck.translation.z,
            0.0,
        ];
        #[cfg(debug_assertions)]
        super::dev_trace::event("root_inputs", format!("dt={dt} deck={transform:?} velocity={velocity:?} animation_board={:?} reckoning={reckoning:?} initialize_heading={}", self.animation_board,self.roots.initialize_heading));
        self.roots.update(
            transform,
            [velocity.x, velocity.y, velocity.z, 0.0],
            dt,
            &self.animation_board,
            reckoning,
        );
    }

    ///Ground board phase before GeneralUpdate's IK and physical drives.
    pub fn prepare_ground(
        &mut self,
        board: &BoardRuntime,
        reckoning: &AnimationPartTransform,
        dt: f32,
        flags_2468: &mut u32,
    ) -> AnimationPartTransform {
        self.roots.initialize_heading = true; //82BDF550, on every Ground update.
        self.update_roots(board, reckoning, dt);
        self.board_frames.prepare_ground(
            &self.roots,
            &self.record.pose[0],
            self.roots.board,
            flags_2468,
        )
    }

    ///82BDF778..F7AC runs AFTER the current GeneralUpdate consumes trajectory.
    pub fn finish_ground(&mut self) {
        self.motion.next_trajectory = IDENTITY;
    }
}

fn load_landing_settings(data: &Collections) -> Result<LandingSettings, String> {
    let value = |name| data.float("physics_animation", "default", name);
    let graph = |name| -> Result<PointGraph<4>, String> {
        // PointNegGraphData4 stores four bounds, followed by native X/Y arrays.
        let words = data
            .words::<12>("physics_animation", "default", name)?
            .map(f32::from_bits);
        Ok(PointGraph {
            x: words[4..8].try_into().unwrap(),
            y: words[8..12].try_into().unwrap(),
        })
    };
    Ok(LandingSettings {
        manual_blend: graph("LandingAdjustManualBlend")?,
        grind_blend: graph("LandingAdjustGrindBlend")?,
        coffin_height: graph("LandingAdjustCoffinBounceHeight")?,
        minimum_height: value("LandingAdjustMinYPos")?,
        maximum_velocity: value("LandingAdjustMaxVel")?,
        manual_damping: value("LandingAdjustManualK2")?,
        manual_spring: value("LandingAdjustManualK1")?,
        ground_minimum_compression_time: value("LandingAdjustGroundMinCompressionTime")?,
        ground_damping: value("LandingAdjustGroundK2")?,
        ground_spring: value("LandingAdjustGroundK1")?,
        grind_animation_target_time: value("LandingAdjustGrindStartUsingAnimTargetAfterTime")?,
        grind_damping: value("LandingAdjustGrindK2")?,
        grind_spring: value("LandingAdjustGrindK1")?,
        grind_target_delta: value("LandingAdjustGrindBlendTowardsAnimTargetSpeed")?,
        desired_com_height: value("LandingAdjustDesiredCom")?,
        coffin_time: value("LandingAdjustCoffinBounceTime")?,
        coffin_maximum_velocity: value("LandingAdjustCoffinBounceMaxVel")?,
        coffin_blend_frames: value("LandingAdjustCoffinBounceFramesToBlendAway")?,
        coffin_base_height: value("LandingAdjustCoffinBounceBaseHeight")?,
    })
}
