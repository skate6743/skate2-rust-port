//! Two-bone IK over the authored pose. Preserve limb lengths and
//! align both palms to fixed surface contacts with independent wrist rotation.
use super::clip::Clip;
use crate::skate3_native::math_host::*;

pub(super) fn clearance(clip: &Clip, globals: &[Mat4]) -> Vec3 {
    let hands = clip.hands(globals);
    let hips = globals[clip.index("HIPS")].w_axis.truncate();
    // The Mixamo freehang arches its hips past its hands. Bring the torso
    // outside a vertical wall and raise it enough to keep the edge in reach.
    let back = (hips.z - hands.z + 0.24).max(0.);
    Vec3::new(0., back * 0.53, -back)
}

pub(super) const PALM: Vec3 = Vec3::new(0.075, 0.052, 0.);
pub(super) fn wrist(ledge: super::ledge::Ledge, i: usize) -> (Vec3, Quat) {
    let normal = ledge.normals[i];
    let forward = (ledge.forward - normal * ledge.forward.dot(normal)).normalize();
    let rotation = Quat::from_mat3(&Mat3::from_cols(forward, -normal, forward.cross(-normal)));
    (ledge.palms[i] - rotation * PALM, rotation)
}
pub(super) fn hands(
    clip: &Clip,
    locals: &mut [Transform],
    root: Mat4,
    ledge: super::ledge::Ledge,
    weight: f32,
) {
    let globals = clip.globals(locals);
    let inverse = root.inverse();
    let root_rotation = Transform::from_matrix(root).rotation;
    for (i, side) in ["LEFT", "RIGHT"].into_iter().enumerate() {
        let upper = clip.index(&format!("{side}ARM"));
        let lower = clip.index(&format!("{side}FOREARM"));
        let hand = clip.index(&format!("{side}HAND"));
        let (position, rotation) = wrist(ledge, i);
        let target = globals[hand]
            .w_axis
            .truncate()
            .lerp(inverse.transform_point3(position), weight);
        let rotation = Transform::from_matrix(globals[hand])
            .rotation
            .slerp(root_rotation.inverse() * rotation, weight);
        solve(clip, locals, upper, lower, hand, target, rotation);
    }
}
fn solve(
    clip: &Clip,
    locals: &mut [Transform],
    upper: usize,
    lower: usize,
    hand: usize,
    target: Vec3,
    wrist: Quat,
) {
    let g = clip.globals(locals);
    let a = g[upper].w_axis.truncate();
    let b = g[lower].w_axis.truncate();
    let c = g[hand].w_axis.truncate();
    let l1 = a.distance(b);
    let l2 = b.distance(c);
    let Some(dir) = (target - a).try_normalize() else {
        return;
    };
    let distance = a
        .distance(target)
        .clamp((l1 - l2).abs() + 0.0001, l1 + l2 - 0.0001);
    let along = (l1 * l1 - l2 * l2 + distance * distance) / (2. * distance);
    let bend = (b - a) - dir * (b - a).dot(dir);
    let pole = bend
        .try_normalize()
        .unwrap_or_else(|| dir.any_orthonormal_vector());
    let elbow = a + dir * along + pole * (l1 * l1 - along * along).max(0.).sqrt();
    rotate(clip, locals, &g, upper, b - a, elbow - a);
    let g = clip.globals(locals);
    let b = g[lower].w_axis.truncate();
    rotate(
        clip,
        locals,
        &g,
        lower,
        g[hand].w_axis.truncate() - b,
        a + dir * distance - b,
    );
    let g = clip.globals(locals);
    let relative = Transform::from_matrix(g[lower]).rotation.inverse() * wrist;
    // Pronation belongs in the forearm, not a 100+ degree wrist twist.
    // Roll about the actual elbow-to-wrist axis so the contact cannot move.
    let axis = locals[hand].translation.normalize();
    let projected = axis * Vec3::new(relative.x, relative.y, relative.z).dot(axis);
    let raw_twist = Quat::from_xyzw(projected.x, projected.y, projected.z, relative.w);
    let twist = if raw_twist.length_squared() > 1e-8 {
        raw_twist.normalize()
    } else {
        Quat::IDENTITY
    };
    locals[lower].rotation = (locals[lower].rotation * twist).normalize();
    locals[hand].rotation = (twist.inverse() * relative).normalize();
}
fn rotate(
    clip: &Clip,
    locals: &mut [Transform],
    globals: &[Mat4],
    bone: usize,
    from: Vec3,
    to: Vec3,
) {
    let delta = Quat::from_rotation_arc(from.normalize(), to.normalize());
    let parent = clip.parents[bone] as usize;
    locals[bone].rotation = (Transform::from_matrix(globals[parent]).rotation.inverse()
        * delta
        * Transform::from_matrix(globals[bone]).rotation)
        .normalize();
}
