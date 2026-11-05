//! Skeleton82BDDC68,82BE0858,82BE0CC0 and82BDCC28, original S3 TU3.
//! These calculate animation targets; the existing skeleton drives own physics.
use super::{
    board_motion_output::inverse_length_squared,
    skeleton_animation_record::{AnimationPartTransform as Frame, compose_affine, transform_point},
    skeleton_root::{SkeletonRootFrames, inverse_rigid, orthonormalize},
};
use crate::{
    animation::foot_ik::interpolate_affine,
    player::wipeout_state::math::{cross, dot, scale, sub},
    point_graph::PointGraph,
};
pub type Vector = [f32; 4];
const UP: Vector = [0., 1., 0., 0.];
const EPSILON: f32 = f32::from_bits(0x37800000);
#[derive(Clone, Copy)]
pub struct Settings {
    pub root_y_offset: f32,
    pub capsule_radius: f32,
    pub capsule_length: f32,
}
///82BDDC68 changes only skate-root and invokes COM-lift on the OLD animation root.
pub fn skate_root(
    old: Frame,
    physical_com: Vector,
    flags2480: u32,
    ground_y: f32,
    settings: Settings,
) -> Frame {
    let mut position = physical_com;
    position[1] = old[3][1] + settings.root_y_offset;
    if flags2480 & 1 != 0 {
        let requested = (ground_y + settings.capsule_length) + settings.capsule_radius;
        position[1] = requested.clamp(position[1], position[1] + 0.3);
    }
    [old[2], old[0], old[1], position]
}
///82BE0858. Packet trajectory reset is performed by the owning host before this.
pub fn update_root(
    roots: &mut SkeletonRootFrames,
    com: Vector,
    local_com: Vector,
    spin: f32,
    revert_frames: Option<u32>,
    reversed: bool,
) {
    roots.initialize_heading = true;
    let angle = revert_frames.map_or(spin, |frames| {
        let magnitude = std::f32::consts::PI / frames as f32;
        if reversed { magnitude } else { -magnitude }
    });
    let (sin, cos) = crate::trigonometry::sin_cos(angle);
    let yaw = [
        [cos, 0., -sin, 0.],
        [0., 1., 0., 0.],
        [sin, 0., cos, 0.],
        [0.; 4],
    ];
    let mut frame = compose_affine(&roots.animation_to_world, &yaw);
    if revert_frames.is_some() {
        frame = orthonormalize(frame);
    }
    //82BE0CC0: world-up correction with two independent degeneracy tests.
    let deviation = cross(frame[1], UP);
    if dot(deviation, deviation) > EPSILON {
        let right = cross(UP, frame[2]);
        let sq = dot(right, right);
        if sq > EPSILON {
            let right = scale(right, inverse_length_squared(sq, 2));
            let target = [right, UP, cross(right, UP), frame[3]];
            frame = interpolate_affine(&frame, &target, 0.1);
        }
    }
    let mut rotation = frame;
    rotation[3] = [0.; 4];
    frame[3] = sub(com, transform_point(&rotation, local_com));
    roots.reset_initial_alignment(orthonormalize(frame));
}
///82BDCC28 returns no write outside its descending/unsuppressed branch.
pub fn pose_adjustment(
    world_to_animation: &Frame,
    actual_board: &Frame,
    mapped_board: &Frame,
    ik_offset: Vector,
    com_velocity_y: f32,
    flags2480: u32,
    time: f32,
    blend: &PointGraph<8>,
) -> Option<Frame> {
    if !(com_velocity_y < 0.) || flags2480 & 2 != 0 {
        return None;
    }
    let mut target = compose_affine(world_to_animation, actual_board);
    let mut rotation = *world_to_animation;
    rotation[3] = [0.; 4];
    let mut correction = transform_point(&rotation, ik_offset);
    correction[1] = 0.;
    if target[3][1] > mapped_board[3][1] {
        correction[1] = target[3][1] - mapped_board[3][1];
    }
    target[3] = std::array::from_fn(|lane| mapped_board[3][lane] + correction[lane]);
    let blended = interpolate_affine(mapped_board, &target, blend.evaluate(time));
    let mut offset = compose_affine(&blended, &inverse_rigid(mapped_board));
    offset[3][1] = -offset[3][1];
    Some(offset)
}
