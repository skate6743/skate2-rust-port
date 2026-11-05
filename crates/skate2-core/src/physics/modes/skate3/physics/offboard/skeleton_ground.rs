//! Original Biped Skeleton update82BDE060 with the existing physical owners.
mod frames;
#[cfg(any())]
mod pose_audit;
mod state;
use crate::skate3_native::physics::{
    skeleton_air::SkeletonAir,
    skeleton_input_runtime::{CollisionInput, SkeletonInputRuntime, SkeletonOwners},
};
#[cfg(any())]
pub(crate) use pose_audit::audit_render_parts;
use skate3_core::{
    animation::output::NativeMatrix,
    physics::{
        board_runtime::BoardRuntime, rigid_body::RetailSimulationStep,
        skeleton_animation_record::{AnimationPartTransform as Transform, compose_affine},
    },
    player::input_phase::ProcessedPhysicsInput,
};
pub(crate) use state::State;
///Actual Sync input: caller's constructed frame and Biped state1056 COM.
///State retains animation-space16016 independently of world-space12496.
pub(crate) struct Input<'a> {
    pub world_frame: &'a Transform,
    pub centre_of_mass_1056: [f32; 4],
}
///Terminal82D8E3E0 arguments, emitted after GeneralUpdate and trajectory clear.
///The caller invokes the canonical ground_reckoning producer in this callback.
pub(crate) struct ReckoningUpdate {
    pub up: [f32; 4],
    pub forward: [f32; 4],
    pub blend: f32,
}
impl SkeletonInputRuntime {
    pub(crate) fn update_biped_ground<F>(
        &mut self,
        air: &mut SkeletonAir,
        board: &mut BoardRuntime,
        input: Input<'_>,
        state: &mut State,
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
        finish_reckoning: F,
    ) -> Result<Transform, String>
    where
        F: FnOnce(ReckoningUpdate) -> Result<(), String>,
    {
        let s = &mut owners.animated;
        let target = frames::prepare_pose(
            &mut s.roots,
            &mut s.board_frames,
            &s.record.pose[0],
            &self.drive_frames[0],
            &mut state.retained_board,
            input,
            p.flags_2476,
            p.flags_2484,
            &mut p.flags_2468,
        );
        if p.flags_2480 & 0x8000 != 0 || p.flags_2484 & 1 != 0 {
            //Reuse the shared Ground/Air history and actual board anchor.
            s.board_frames.physical_board = air.apply_board(board, &target, true);
        }
        if std::mem::take(&mut self.offboard_reset_pose_pending) {
            // PC host reset: the new standing biped pose is a discontinuity
            // from the transient board-reset pose, not a spring-driven jump.
            self.reset_physical_pose(owners, simulation);
            crate::diagnostics::test_log::event("skate3_offboard_reset_pose",
                format_args!("hips={:?} local_hips={:?}",owners.body.part_transforms()[23][3],self.drive_frames[23][3]));
        }
        self.general_update(p, owners, globals, collision, simulation)?;
        owners.animated.finish_ground();
        let root = owners.animated.roots.animation_to_world;
        finish_reckoning(ReckoningUpdate {
            up: root[1],
            forward: root[2],
            blend: 0.5,
        })?;
        owners.animation_input.fields.flags2468 = p.flags_2468;
        Ok(target)
    }

    /// Skeleton::UpdateOffBoardPushing 0x82BDF268, Pushing::Update's skeleton step in place of
    /// the Biped update: the centre of mass follows the grab within 0.3 of where the skeleton
    /// moves, the root moves and turns by the push, and no reckoning update follows.
    pub(crate) fn update_offboard_pushing(
        &mut self,
        air: &mut SkeletonAir,
        board: &mut BoardRuntime,
        push: Push,
        pose_error_16272: [f32; 4],
        p: &mut ProcessedPhysicsInput,
        owners: &mut SkeletonOwners<'_>,
        globals: &[NativeMatrix],
        collision: &CollisionInput,
        simulation: RetailSimulationStep,
    ) -> Result<Transform, String> {
        let s = &mut owners.animated;
        //82BDEE08 consumes animation record6464+16 using the OLD world root.
        s.board_frames.skate_root = compose_affine(&s.roots.animation_to_world, &s.record.pose[0]);
        //82BDF29C..2DC: Skeleton15872 plus the motion, then 82BDE310 with no lift.
        let com = s.board_frames.com_frame[3];
        let moved: [f32; 4] = std::array::from_fn(|i| com[i] + push.motion[i]);
        let placed = clamp_within(push.follow, moved, f32::from_bits(0x3e99_999a));
        s.board_frames.update_com_lift(&s.roots.animation_to_world, placed, 0.0);
        //82BDF2F8..37C: the motion in animation space, a translation for the root update.
        let w = s.roots.world_to_animation;
        let local: [f32; 4] = std::array::from_fn(|i| {
            let x = w[0][i] * push.motion[0];
            let y = w[1][i].mul_add(push.motion[1], x);
            w[2][i].mul_add(push.motion[2], y)
        });
        root_transforms_offboard(&mut s.roots, push.turn, local, pose_error_16272);
        //82BDF3BC..474: processed +0 and Skeleton16208 = the mapped pose under the new root.
        let target = compose_affine(&s.roots.animation_to_world, &self.drive_frames[0]);
        s.board_frames.animation_target = target;
        p.flags_2468 |= 0x80000;
        if p.flags_2480 & 0x8000 != 0 {
            s.board_frames.physical_board = air.apply_board(board, &target, true);
        }
        self.general_update(p, owners, globals, collision, simulation)?;
        owners.animated.finish_ground();
        owners.animation_input.fields.flags2468 = p.flags_2468;
        Ok(target)
    }
}

/// Pushing::Update's arguments to UpdateOffBoardPushing: Pushing+0x120, +0x464 and +0x1A0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Push {
    pub motion: [f32; 4],
    pub turn: f32,
    pub follow: [f32; 4],
}

/// PhysicsUtility::ClampVectorAWithinRangeOfVectorB 0x82BD41B0.
fn clamp_within(a: [f32; 4], b: [f32; 4], range: f32) -> [f32; 4] {
    let delta: [f32; 4] = std::array::from_fn(|i| a[i] - b[i]);
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if length <= range {
        return a;
    }
    let scale = range / length;
    std::array::from_fn(|i| delta[i].mul_add(scale, b[i]))
}

fn dot3(a: [f32; 4], b: [f32; 4]) -> f32 {
    a[2].mul_add(b[2], a[1].mul_add(b[1], a[0] * b[0]))
}

/// Skeleton::UpdateRootTransformsOffBoard 0x82BE1058 as Pushing calls it: no ground adjust and no
/// minimum COM velocity. Upright first, then the root turns about its own up and moves by the
/// local translation; a collision error over kMaxSkeletonDisp pushes it out by
/// kCollisionPushFraction of the rest.
fn root_transforms_offboard(
    roots: &mut skate3_core::physics::skeleton_root::SkeletonRootFrames,
    turn: f32,
    local: [f32; 4],
    pose_error: [f32; 4],
) {
    let previous = roots.animation_to_world[3];
    let (sin, cos) = skate3_core::trigonometry::sin_cos(turn);
    reorient_upright(&mut roots.animation_to_world);
    let step = [
        [cos, 0.0, -sin, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sin, 0.0, cos, 0.0],
        local,
    ];
    let mut root = skate3_core::physics::skeleton_root::orthonormalize(compose_affine(
        &roots.animation_to_world,
        &step,
    ));
    let squared = dot3(pose_error, pose_error);
    let length = if squared == 0.0 { 0.0 } else { squared.sqrt() };
    if length > f32::from_bits(0x3c23_d70a) {
        let normal = pose_error.map(|v| v * (1.0 / length));
        let moved: [f32; 4] = std::array::from_fn(|i| root[3][i] - previous[i]);
        let along = dot3(normal, moved);
        if length > along {
            let (inside, rest) = if 0.0 > along { (-along, length) } else { (0.0, length - along) };
            let push = rest.mul_add(f32::from_bits(0x3e4c_cccd), inside);
            root[3] = std::array::from_fn(|i| previous[i] + normal[i].mul_add(push, moved[i]));
        }
    }
    roots.heading_alignment = IDENTITY;
    roots.animation_to_world = root;
    roots.world_to_animation = skate3_core::physics::skeleton_root::inverse_rigid(&root);
}

/// Skeleton::ReorientSkaterUpright 0x82BE0CC0: a tenth of the way toward the upright frame with
/// the same heading.
fn reorient_upright(root: &mut Transform) {
    const UP: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
    let cross = |a: [f32; 4], b: [f32; 4]| -> [f32; 4] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
            0.0,
        ]
    };
    let tilt = cross(root[1], UP);
    if !(dot3(tilt, tilt) > 0.0) {
        return;
    }
    let right = cross(UP, root[2]);
    let squared = dot3(right, right);
    if !(squared > 0.0) {
        return;
    }
    let right = right.map(|v| v / squared.sqrt());
    let forward = cross(right, UP);
    let upright = [right, UP, forward, root[3]];
    *root = skate3_core::animation::foot_ik::interpolate_affine(root, &upright, f32::from_bits(0x3dcc_cccd));
}

const IDENTITY: Transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
];
