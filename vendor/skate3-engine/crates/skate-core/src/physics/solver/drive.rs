//! TU3 drive pass 82AE2E78..82AE30D8, reconstructed from direct disassembly.
//! Finite compiled coefficients are required; exceptional SIMD arithmetic is
//! not claimed to match hardware. Linear/angular softness lanes are retained.

use super::joint::data::{
    ConstraintGeometry, ReactionState, project_correction, publish_reactions, read_column,
    write_column, xyz,
};

pub(super) fn solve(record: &mut [u32], a: &mut [u32], b: &mut [u32]) {
    let geometry = ConstraintGeometry::decode(record);
    let mut body_a = ReactionState::decode(a);
    let mut body_b = ReactionState::decode(b);
    let (relative_point, relative_angular) = geometry.relative_corrections(&body_a, &body_b);
    let old_linear = read_column(record, 32);
    let old_angular = read_column(record, 48);
    let linear_response =
        project_correction(&geometry.linear_projection, relative_point, old_linear);
    let angular_response =
        project_correction(&geometry.angular_projection, relative_angular, old_angular);
    let linear_target = read_column(record, 160);
    let angular_target = read_column(record, 176);
    let linear_limits = [
        read_column(record, 96)[3],
        read_column(record, 112)[3],
        linear_target[3],
    ];
    let angular_limits = [
        read_column(record, 128)[3],
        read_column(record, 144)[3],
        angular_target[3],
    ];

    let mut linear_impulse = old_linear;
    let mut angular_impulse = old_angular;
    for axis in 0..3 {
        let linear_candidate = linear_response[axis].mul_add(old_linear[3], linear_target[axis]);
        let angular_candidate =
            angular_response[axis].mul_add(old_angular[3], angular_target[axis]);
        linear_impulse[axis] = symmetric_impulse_limit(linear_candidate, linear_limits[axis]);
        angular_impulse[axis] = symmetric_impulse_limit(angular_candidate, angular_limits[axis]);
    }
    let linear_change = xyz(core::array::from_fn(|axis| {
        linear_impulse[axis] - old_linear[axis]
    }));
    let angular_change = xyz(core::array::from_fn(|axis| {
        angular_impulse[axis] - old_angular[axis]
    }));
    // 82AE2FB8/2FC4 select XYZ from the new impulses while retaining each
    // accumulator's fourth (softness) lane from its old value.
    write_column(record, 32, linear_impulse);
    write_column(record, 48, angular_impulse);
    geometry.apply_impulse_changes(&mut body_a, &mut body_b, linear_change, angular_change);
    publish_reactions(&body_a, &body_b, a, b);
}

fn symmetric_impulse_limit(candidate: f32, limit: f32) -> f32 {
    let upper_limited = if candidate < limit { candidate } else { limit };
    let negative_limit = 0.0 - limit;
    if upper_limited > negative_limit {
        upper_limited
    } else {
        negative_limit
    }
}
