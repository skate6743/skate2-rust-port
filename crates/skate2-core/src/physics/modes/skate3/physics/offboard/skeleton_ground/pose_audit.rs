//! Read-only fixture diagnostics; never modifies simulation or acceptance limits.
use crate::skate3_native::physics::Skate3ControllerState;
use skate3_core::physics::skeleton_animation_record::compose_affine;

///Each tuple is(part, render-vs-record maximum XYZ matrix error,
///record-vs-live maximum XYZ matrix error). Include basis AND translation:
///a whole-body orientation failure can have small joint-position errors.
///Render matrices are animation-space globals; physical records are world-space
///volume frames. Undo the authored volume frame before comparing bone frames.
pub(crate) fn audit_render_parts(skater: &Skate3ControllerState) -> [(usize, f32, f32); 24] {
    let live = skater.skeleton.part_transforms();
    std::array::from_fn(|part| {
        let expected = compose_affine(
            &skater.skeleton.record.pose[part],
            &skater.skeleton_output.pose.geometry.inverse_part_frames[part],
        );
        let rendered = compose_affine(
            &skater.animated_skeleton.roots.animation_to_world,
            &skater.render_pose[skater.animated_skeleton.bone_indices[part]],
        );
        let error = |a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]| {
            let mut maximum = 0.0_f32;
            for column in 0..4 {
                for lane in 0..3 {
                    let delta = (a[column][lane] - b[column][lane]).abs();
                    if !delta.is_finite() {
                        return f32::INFINITY;
                    }
                    maximum = maximum.max(delta);
                }
            }
            maximum
        };
        //Part0 intentionally records the real deck, not the skeleton proxy.
        //Its second value is diagnostic only and must not be asserted zero.
        (
            part,
            error(&rendered, &expected),
            error(&skater.skeleton.record.pose[part], &live[part]),
        )
    })
}
