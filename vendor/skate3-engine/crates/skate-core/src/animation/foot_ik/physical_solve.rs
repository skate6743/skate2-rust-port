//! SkeletonIK::SolvePhysical82BF1480. Physical joint centers, not drive targets.
use super::{
    drive::{Geometry, line_mapping, normalize_safe},
    math::cross,
    two_bone::{self, AngleLimits, SolveResult},
};
use crate::physics::{
    skeleton_animation_record::{compose_affine, transform_point},
    skeleton_body::SkeletonBody,
};

pub(super) fn solve(
    body: &mut SkeletonBody,
    geometry: &Geometry,
    end_part: usize,
    offset: [f32; 4],
) -> bool {
    let Some(middle_part) = geometry.parents[end_part] else {
        return false;
    };
    let Some(root_part) = geometry.parents[middle_part] else {
        return false;
    };
    let point = |part: usize| {
        transform_point(
            &body.record.pose[part],
            geometry.inverse_part_frames[part][3],
        )
    };
    let [root, middle, end] = [point(root_part), point(middle_part), point(end_part)];
    let mut target = core::array::from_fn(|i| end[i] + offset[i]);
    let mut solved_middle = [0.0; 4];
    //Literal0 and180, no original-angle override and recursion budget1.
    //The source requires return0; its extended/invalid paths BOTH reject.
    if two_bone::solve(
        root,
        middle,
        end,
        &mut solved_middle,
        &mut target,
        AngleLimits {
            minimum_degrees: 0.0,
            maximum_degrees: 180.0,
        },
        false,
        1,
    ) != SolveResult::Solved
    {
        return false;
    }
    let original_normal = cross(sub(root, middle), sub(end, middle));
    let solved_normal = cross(sub(root, solved_middle), sub(target, solved_middle));
    let normal = normalize_safe(core::array::from_fn(|i| {
        original_normal[i] + solved_normal[i]
    }));
    let Some(mid_map) = line_mapping(middle, end, normal, solved_middle, target, normal) else {
        return false;
    };
    let Some(root_map) = line_mapping(root, middle, normal, root, solved_middle, normal) else {
        return false;
    };
    let middle_frame = compose_affine(&mid_map, &body.record.pose[middle_part]);
    let root_frame = compose_affine(&root_map, &body.record.pose[root_part]);
    //Unlike pre-physics drive solving, no extra orthonormalization precedes
    //the setter. The live body normalizes; the record keeps these frames.
    body.set_part_transform(middle_part, middle_frame);
    body.set_part_transform(root_part, root_frame);
    true
}
fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|i| a[i] - b[i])
}
