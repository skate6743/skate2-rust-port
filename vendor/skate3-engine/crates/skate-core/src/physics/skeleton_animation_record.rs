//! Animation-record COM producer82BEBF88 and mass normalization82BEBAA8.
//! Skeleton embeds this source state at6464, so com_to_deck_world4544 is the
//! ProcessData82BD8918 input published from Skeleton11008 to Processed752.
use super::rigid_body::{RetailQuaternion, basis_from_quaternion};
use crate::math::Vector3;

/// Native affine columns X,Y,Z,translation. Keep all four stored lanes;
/// UpdateAnimRecord copies24 complete transforms, not just the positions.
pub type AnimationPartTransform = [[f32; 4]; 4];
pub const IDENTITY: AnimationPartTransform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
];

/// Forward PhysicsParamBoneData frame from SkeletonData::Init82BD6E24..6E98.
/// The same sqrt(2), fused negative product and lane permutations are used by
/// RigidBody::DynamicUpdate. Keep the otherwise unused fourth rotation lanes:
/// native permute masks822FB8A0/B0/C0 duplicate Z.z, Y.x and X.y respectively.
/// This is the forward frame, not the inverse stored by the later init slice.
pub fn physics_bone_frame(quaternion: [f32; 4], translation: [f32; 4]) -> AnimationPartTransform {
    let [x, y, z, w] = quaternion;
    let [ri, up, at] = basis_from_quaternion(RetailQuaternion { x, y, z, w }).columns;
    [
        [ri[0], ri[1], ri[2], at[2]],
        [up[0], up[1], up[2], up[0]],
        [at[0], at[1], at[2], ri[1]],
        translation,
    ]
}

/// ProcessData82BD8BB4..8CAC: gather each named global animation bone, then
/// right-multiply its forward physics frame. Native maps names to hierarchy
/// indices in82BD6CB4..6CE4; the host loader provides those verified indices.
/// The result is the input to the following skeleton IK and adjustment stages,
/// which must run before UpdateAnimRecord for the final physical pose.
pub fn map_animation_parts(
    global_bones: &[AnimationPartTransform],
    bone_indices: &[usize; 24],
    physics_frames: &[AnimationPartTransform; 24],
) -> Result<[AnimationPartTransform; 24], &'static str> {
    if bone_indices.iter().any(|i| *i >= global_bones.len()) {
        return Err("physics skeleton bone index is outside the animation hierarchy");
    }
    Ok(std::array::from_fn(|part| {
        let animation = &global_bones[bone_indices[part]];
        let physics = &physics_frames[part];
        std::array::from_fn(|column| {
            if column == 3 {
                transform_point(animation, physics[3])
            } else {
                std::array::from_fn(|lane| {
                    let x = physics[column][0] * animation[0][lane];
                    let y = physics[column][1].mul_add(animation[1][lane], x);
                    physics[column][2].mul_add(animation[2][lane], y)
                })
            }
        })
    }))
}

#[derive(Clone, Debug)]
pub struct SkeletonAnimationMasses {
    /// Native misleadingly calls these part masses. They are bone-box volumes
    /// before density and animated/ragdoll physical mass scaling.
    pub part_weights: [f32; 24],
    pub total: f32,
    pub fractional: [f32; 24],
}
impl SkeletonAnimationMasses {
    /// Full weight assignment slice82BE4900/82BE5428. Board part0 is weight0;
    /// other parts use original PhysicsParamBoneData.size32, not collider size.
    /// Shapes0/1/2 all use the same box product. Native unrecognized shapes>=3
    /// retain the iteration's initialized0. A hat on head1 takes its own branch
    /// and still uses the original head box product, excluding the hat volume.
    pub fn from_bone_data(
        sizes: [Vector3; 24],
        collision_shapes: [u32; 24],
        head_has_hat: bool,
    ) -> Self {
        let weights = std::array::from_fn(|i| {
            if i == 0 || collision_shapes[i] >= 3 && !(i == 1 && head_has_hat) {
                0.0
            } else {
                (sizes[i].x * sizes[i].y) * sizes[i].z
            }
        });
        Self::normalize(weights)
    }

    ///82BEBAA8: ordered24 scalar additions, one scalar single-precision divide,
    /// then24 multiplies. No clamping, epsilon, density or re-normalization.
    pub fn normalize(part_weights: [f32; 24]) -> Self {
        let mut total = 0.0f32;
        for (i, weight) in part_weights.iter().enumerate() {
            // Preserve native operand order, including signed zero/NaN cases.
            total = if i % 8 == 0 || i % 8 == 7 {
                *weight + total
            } else {
                total + *weight
            };
        }
        let reciprocal = 1.0f32 / total;
        Self {
            part_weights,
            total,
            fractional: part_weights.map(|v| v * reciprocal),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkeletonAnimationRecord {
    pub pose: [AnimationPartTransform; 24],
    pub centre_of_mass: [f32; 4],
    pub centre_of_mass_delta: [f32; 4],
    pub com_to_deck_world: [f32; 4],
    pub com_to_deck_world_delta: [f32; 4],
    pub reset_scalar: f32,
}
impl Default for SkeletonAnimationRecord {
    fn default() -> Self {
        // Constructor82BEB8C8 initializes24 animation transforms via82BF30C0,
        // then all four vectors and resetScalar5224 to0.
        Self {
            pose: [IDENTITY; 24],
            centre_of_mass: [0.0; 4],
            centre_of_mass_delta: [0.0; 4],
            com_to_deck_world: [0.0; 4],
            com_to_deck_world_delta: [0.0; 4],
            reset_scalar: 0.0,
        }
    }
}
impl SkeletonAnimationRecord {
    /// Animation fields of SkeletonState::Reset82BEBBB0. The function leaves
    /// animation transforms, fractional masses and resetScalar unchanged.
    pub fn reset_history(&mut self) {
        self.centre_of_mass = [0.0; 4];
        self.centre_of_mass_delta = [0.0; 4];
        self.com_to_deck_world = [0.0; 4];
        self.com_to_deck_world_delta = [0.0; 4];
    }

    pub fn update(
        &mut self,
        pose: &[AnimationPartTransform; 24],
        animation_to_board: &AnimationPartTransform,
        masses: &SkeletonAnimationMasses,
    ) {
        self.pose = *pose;
        let mut com = [0.0f32; 4];
        for (part, weight) in self.pose.iter().zip(masses.fractional) {
            for (lane, value) in com.iter_mut().enumerate() {
                *value = part[3][lane].mul_add(weight, *value);
            }
        }
        self.centre_of_mass_delta = std::array::from_fn(|i| com[i] - self.centre_of_mass[i]);
        self.centre_of_mass = com;
        // Native transforms both points separately then subtracts, retaining
        // each affine FMA's rounding; do not rotate their difference instead.
        let transformed_com = transform_point(animation_to_board, com);
        let transformed_deck = transform_point(animation_to_board, self.pose[0][3]);
        let relative = std::array::from_fn(|i| transformed_com[i] - transformed_deck[i]);
        self.com_to_deck_world_delta =
            std::array::from_fn(|i| (relative[i] - self.com_to_deck_world[i]) * self.reset_scalar);
        self.com_to_deck_world = relative;
        self.reset_scalar = 1.0;
    }

    pub fn com_to_deck(&self) -> Vector3 {
        let v = self.com_to_deck_world;
        Vector3::new(v[0], v[1], v[2])
    }
}
pub fn transform_point(transform: &AnimationPartTransform, point: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| {
        let x = transform[0][i].mul_add(point[0], transform[3][i]);
        let y = transform[1][i].mul_add(point[1], x);
        transform[2][i].mul_add(point[2], y)
    })
}

#[cfg(test)]
#[path = "tests/skeleton_animation_record.rs"]
mod tests;

/// Native affine product used by ProcessData and board-offset application.
pub fn compose_affine(
    parent: &AnimationPartTransform,
    child: &AnimationPartTransform,
) -> AnimationPartTransform {
    std::array::from_fn(|column| {
        if column == 3 {
            transform_point(parent, child[3])
        } else {
            std::array::from_fn(|lane| {
                let x = child[column][0] * parent[0][lane];
                let y = child[column][1].mul_add(parent[1][lane], x);
                child[column][2].mul_add(parent[2][lane], y)
            })
        }
    })
}
