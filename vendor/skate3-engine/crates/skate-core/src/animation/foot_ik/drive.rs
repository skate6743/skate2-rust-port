//! SkeletonIK::UpdateDriveFrames82BF0198 and its solve/mapping chain.
use super::{
    math::{Vector, cross, length, normalize},
    status::{LimbStatus, Mode},
    transforms::{LimbBinding, LimbFrames, inverse_rigid},
    two_bone::{self, AngleLimits, SolveResult},
};
use crate::physics::{
    native_arithmetic::dot3,
    skeleton_animation_record::{
        AnimationPartTransform as Transform, compose_affine, transform_point,
    },
    skeleton_root::orthonormalize,
};

#[derive(Clone, Debug)]
pub struct Geometry {
    /// SkeletonData4704: nearest ancestor represented by a physical part.
    pub parents: [Option<usize>; 24],
    /// SkeletonData1536: native cofactor inverse of the authored part frames.
    pub inverse_part_frames: [Transform; 24],
}
impl Geometry {
    /// SkeletonData Init82BD6C40 builds these inverses once from the real
    /// PhysicsParamBoneData frames. It does not assume an orthogonal basis.
    pub fn new(
        parents: [Option<usize>; 24],
        part_frames: &[Transform; 24],
    ) -> Result<Self, String> {
        for part in 0..24 {
            let mut parent = parents[part];
            let mut visited = [false; 24];
            visited[part] = true;
            while let Some(index) = parent {
                if index >= 24 || visited[index] {
                    return Err(format!(
                        "Invalid physical ancestor chain for IK part {part}"
                    ));
                }
                visited[index] = true;
                parent = parents[index];
            }
        }
        Ok(Self {
            parents,
            inverse_part_frames: part_frames.map(|frame| super::math::inverse_affine(&frame)),
        })
    }

    pub fn validate_limbs(&self, bindings: &[LimbBinding; 4]) -> Result<(), String> {
        for binding in bindings {
            let end = binding.parent_part.unwrap_or(binding.part);
            if binding.part >= 24
                || end >= 24
                || self.parents[end].and_then(|p| self.parents[p]).is_none()
            {
                return Err(format!(
                    "IK part {end} has no two-bone physical ancestor chain"
                ));
            }
        }
        Ok(())
    }
}

pub fn update(
    statuses: &[LimbStatus; 4],
    frames: &mut [LimbFrames; 4],
    bindings: &[LimbBinding; 4],
    geometry: &Geometry,
    original_animation: &[Transform; 24],
    drives: &mut [Transform; 24],
    world_to_animation: &Transform,
    limits: AngleLimits,
) -> [bool; 4] {
    let mut updated = [false; 4];
    for limb in 0..4 {
        if statuses[limb].mode == Mode::Disabled {
            continue;
        }
        let binding = bindings[limb];
        let target = compose_affine(world_to_animation, &frames[limb].world);
        let parent_target = binding
            .parent_part
            .map(|_| compose_affine(world_to_animation, &frames[limb].parent_world));
        let driven = binding.parent_part.unwrap_or(binding.part);
        let driven_target = parent_target.as_ref().unwrap_or(&target);
        let endpoint = transform_point(driven_target, geometry.inverse_part_frames[driven][3]);
        let offset = subtract(endpoint, original_animation[driven][3]);
        //82BF01DC..1F4: these are maximum root-to-target distances passed
        //through f1 into82BD41B0, not animation blend weights.
        let maximum_distance = if limb < 2 {
            f32::from_bits(0x3F4C_CCCD)
        } else {
            1.0
        };
        if let Some(residual) = solve_drives(
            driven,
            offset,
            maximum_distance,
            geometry,
            original_animation,
            drives,
            limits,
        ) {
            drives[binding.part] = target;
            add_translation(&mut drives[binding.part], residual);
            // The source also adds the same returned delta to retained world
            //targets. Do not insert an extra rotation absent from82BF0458.
            add_translation(&mut frames[limb].world, residual);
            if let Some(parent) = binding.parent_part {
                drives[parent] = parent_target.unwrap();
                add_translation(&mut drives[parent], residual);
                add_translation(&mut frames[limb].parent_world, residual);
            }
            updated[limb] = true;
        }
    }
    updated
}

///82BF0FA8, using original animation joint centers and mutable volume frames.
fn solve_drives(
    end_part: usize,
    offset: Vector,
    maximum_distance: f32,
    geometry: &Geometry,
    original: &[Transform; 24],
    drives: &mut [Transform; 24],
    limits: AngleLimits,
) -> Option<Vector> {
    let middle_part = geometry.parents[end_part]?;
    let root_part = geometry.parents[middle_part]?;
    let [root, middle, end] = [
        original[root_part][3],
        original[middle_part][3],
        original[end_part][3],
    ];
    let requested: Vector = core::array::from_fn(|i| end[i] + offset[i]);
    let mut target = clamp_point(requested, root, maximum_distance);
    let mut solved_middle = [0.0; 4];
    if two_bone::solve(
        root,
        middle,
        end,
        &mut solved_middle,
        &mut target,
        limits,
        true,
        2,
    ) == SolveResult::Invalid
    {
        return None;
    }
    let residual = subtract(target, requested);
    let original_normal = cross(subtract(root, middle), subtract(end, middle));
    let solved_normal = cross(
        subtract(root, solved_middle),
        subtract(target, solved_middle),
    );
    let normal = normalize_safe(core::array::from_fn(|i| {
        original_normal[i] + solved_normal[i]
    }));
    let middle_mapping = line_mapping(middle, end, normal, solved_middle, target, normal)?;
    let root_mapping = line_mapping(root, middle, normal, root, solved_middle, normal)?;
    drives[root_part] = orthonormalize(compose_affine(&root_mapping, &drives[root_part]));
    drives[middle_part] = orthonormalize(compose_affine(&middle_mapping, &drives[middle_part]));
    Some(residual)
}

///82BD41B0 clamps a point relative to the original chain root using scalar
///division after the two-refinement length; it is not limit_length's reciprocal.
fn clamp_point(point: Vector, origin: Vector, maximum: f32) -> Vector {
    let delta = subtract(point, origin);
    let distance = length(delta);
    if maximum >= distance {
        point
    } else {
        let scale = maximum / distance;
        core::array::from_fn(|i| delta[i].mul_add(scale, origin[i]))
    }
}

///82BF1F20, two82BF1D28 frames and target * rigidInverse(original).
pub(super) fn line_mapping(
    start: Vector,
    end: Vector,
    normal: Vector,
    new_start: Vector,
    new_end: Vector,
    new_normal: Vector,
) -> Option<Transform> {
    let original = line_frame(start, end, normal)?;
    let target = line_frame(new_start, new_end, new_normal)?;
    Some(compose_affine(&target, &inverse_rigid(&original)))
}

fn line_frame(start: Vector, end: Vector, normal: Vector) -> Option<Transform> {
    let line = subtract(end, start);
    let across = cross(line, normal);
    let third = cross(across, line);
    let x = normalize_safe(across);
    let z = normalize_safe(third);
    let y = normalize_safe(line);
    let product = (dot3(z, z) * dot3(x, x)) * dot3(y, y);
    //830BD300 <-821647E0, initializer82F82690.
    if product > f32::from_bits(0x3780_0000) {
        Some([x, y, z, start])
    } else {
        None
    }
}

pub(super) fn normalize_safe(value: Vector) -> Vector {
    let magnitude = length(value);
    let normalized = normalize(value);
    //830BD350 <-82181A88; no arbitrary fallback axis for a degenerate line.
    if magnitude > f32::from_bits(0x3586_37BD) {
        normalized
    } else {
        [0.0; 4]
    }
}
fn subtract(a: Vector, b: Vector) -> Vector {
    core::array::from_fn(|i| a[i] - b[i])
}
fn add_translation(frame: &mut Transform, offset: Vector) {
    for lane in 0..4 {
        frame[3][lane] += offset[lane];
    }
}
