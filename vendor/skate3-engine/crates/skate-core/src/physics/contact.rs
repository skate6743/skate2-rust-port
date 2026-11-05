//! Skate 3 TU3's RenderWare contact-record construction.
//!
//! This is the scalar form of the VMX128 block in TU3's two recovered
//! add-contact paths. It builds the 256-byte logical `rw::physics::Contact`
//! payload consumed in place by `ContactBatchBuild`; it does not solve the
//! contact or move a body.

use crate::math::Vector3;

pub mod tu3 {
    /// Game-side physical-material combine helper used before both recovered
    /// collision-to-contact record writers.
    pub const COMBINE_PHYSICAL_MATERIALS: u32 = 0x8276_3078;
    /// First recovered collision-to-contact record block, end exclusive.
    pub const GENERATE_CONTACT_A: u32 = 0x8277_A828;
    pub const GENERATE_CONTACT_A_END: u32 = 0x8277_AA88;
    /// Second field-for-field collision-to-contact record block, end exclusive.
    pub const GENERATE_CONTACT_B: u32 = 0x8277_BFAC;
    pub const GENERATE_CONTACT_B_END: u32 = 0x8277_C21C;
    /// In-place contact Jacobian builder.
    pub const CONTACT_BATCH_BUILD: u32 = 0x82AE_10C8;
}

/// Exact TU3 constant used to reject a degenerate velocity-derived tangent.
///
/// The Xbox data table at `0x830382B0` contains `0x00800000` in every lane.
/// This is the smallest positive normal binary32 value, not a tuned gameplay
/// threshold.
pub const CONTACT_TANGENT_MINIMUM_SQUARED: f32 = f32::from_bits(0x0080_0000);
pub const CONTACT_FALLBACK_HALF: f32 = f32::from_bits(0x3F00_0000);

/// The five 16-byte body vectors copied into a contact's solver workspace.
///
/// Original member names and offsets are independently present in preserved
/// Skate-era EA SDK DWARF and are confirmed by TU3's loads:
///
/// - `mCom/mId` at body `+0x10`;
/// - `mIfull/mInvm` at `+0x70`;
/// - `mIsplt/mState` at `+0x80`;
/// - `mForce/mKine` at `+0x90`;
/// - `mTorque/mCool` at `+0xa0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailContactBodyState {
    /// ID written to the contact header from the collision queue's body-ref
    /// entry. It is kept separate from `reaction_id` because TU3 loads them
    /// from different records.
    pub contact_body_id: u32,
    pub center_of_mass: Vector3,
    pub reaction_id: u32,
    /// Retail packed world inverse-inertia vector
    /// `mIfull = (Ixx, Ixy, Ixz)`.
    pub inverse_inertia_full: Vector3,
    pub inverse_mass: f32,
    /// Retail packed world inverse-inertia vector
    /// `mIsplt = (Izz, Iyy, Iyz)`.
    pub inverse_inertia_split: Vector3,
    pub state: u32,
    pub force_acceleration: Vector3,
    pub kinetic_energy: f32,
    pub torque_acceleration: Vector3,
    pub cool_down: u32,
    pub linear_velocity: Vector3,
    pub angular_velocity: Vector3,
}

/// Collision result and already-combined material values consumed by TU3's
/// contact generator.
///
/// The material-combine routine that produces the three scalars is outside the
/// recovered block and remains deliberately explicit at this boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailContactInput {
    pub position_on_a: Vector3,
    pub position_on_b: Vector3,
    /// Positive normal impulse acts along this vector on A and opposite it on
    /// B. For wheel A over static ground B, this points upward. Collision query
    /// adapters must preserve this solver convention when ordering the pair.
    pub normal: Vector3,
    pub restitution: f32,
    pub static_friction: f32,
    pub dynamic_friction: f32,
    pub tag: u32,
}

/// Three scalar fields in the game-side physical material record consumed by
/// TU3 `0x82763078`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailContactMaterial {
    pub static_friction: f32,
    pub dynamic_friction: f32,
    pub restitution: f32,
}

/// Exact game-side material combine used immediately before the recovered
/// contact writers.
///
/// TU3 performs three independent ordered comparisons:
///
/// - static friction is the greater operand;
/// - dynamic friction is the greater operand;
/// - restitution is the lesser operand.
///
/// The comparisons are kept explicit instead of using `f32::max`/`min`
/// because the retail branch behavior for unordered inputs is operand-order
/// sensitive: equality and unordered comparisons select B for all three fields.
/// Scalar lfs/stfs also quiet a selected signalling NaN while preserving payload.
pub fn combine_contact_materials(
    material_a: RetailContactMaterial,
    material_b: RetailContactMaterial,
) -> RetailContactMaterial {
    let static_friction = if material_a.static_friction > material_b.static_friction {
        material_a.static_friction
    } else {
        material_b.static_friction
    };
    let dynamic_friction = if material_a.dynamic_friction > material_b.dynamic_friction {
        material_a.dynamic_friction
    } else {
        material_b.dynamic_friction
    };
    let restitution = if material_a.restitution < material_b.restitution {
        material_a.restitution
    } else {
        material_b.restitution
    };
    RetailContactMaterial {
        static_friction: material_scalar_store(static_friction),
        dynamic_friction: material_scalar_store(dynamic_friction),
        restitution: material_scalar_store(restitution),
    }
}

fn material_scalar_store(value: f32) -> f32 {
    if value.is_nan() {
        f32::from_bits(value.to_bits() | 0x0040_0000)
    } else {
        value
    }
}

/// Scalar representation of the complete logical 256-byte retail contact.
///
/// `body_a_workspace` and `body_b_workspace` correspond to the ten alternating
/// 16-byte vectors at offsets `0x60..0xff`. Keeping those copied values in the
/// record is important: `ContactBatchBuild` reads them from the contact and
/// does not dereference the live bodies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailContact {
    pub position_on_a: Vector3,
    pub body_a_id: u32,
    pub position_on_b: Vector3,
    pub body_b_id: u32,
    pub normal: Vector3,
    pub restitution: f32,
    pub tangent_0: Vector3,
    pub static_friction: f32,
    pub tangent_1: Vector3,
    pub dynamic_friction: f32,
    /// Velocity at B's contact point minus velocity at A's contact point.
    pub relative_velocity: Vector3,
    pub tag: u32,
    pub body_a_workspace: RetailContactWorkspace,
    pub body_b_workspace: RetailContactWorkspace,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailContactWorkspace {
    pub center_of_mass: Vector3,
    pub reaction_id: u32,
    pub inverse_inertia_full: Vector3,
    pub inverse_mass: f32,
    pub inverse_inertia_split: Vector3,
    pub state: u32,
    pub force_acceleration: Vector3,
    pub kinetic_energy: f32,
    pub torque_acceleration: Vector3,
    pub cool_down: u32,
}

impl From<RetailContactBodyState> for RetailContactWorkspace {
    fn from(body: RetailContactBodyState) -> Self {
        Self {
            center_of_mass: body.center_of_mass,
            reaction_id: body.reaction_id,
            inverse_inertia_full: body.inverse_inertia_full,
            inverse_mass: body.inverse_mass,
            inverse_inertia_split: body.inverse_inertia_split,
            state: body.state,
            force_acceleration: body.force_acceleration,
            kinetic_energy: body.kinetic_energy,
            torque_acceleration: body.torque_acceleration,
            cool_down: body.cool_down,
        }
    }
}

/// Port of TU3 `0x8277A828..0x8277AA88` and its duplicate
/// `0x8277BFAC..0x8277C21C`.
///
/// VMX128 operation order is retained at the scalar-expression level:
///
/// 1. compute each contact arm from the copied body center of mass;
/// 2. compute point rates as `linear + angular cross arm`;
/// 3. derive tangent 0 from `relative_velocity cross normal`;
/// 4. if that axis is subnormal, use the exact X/Z projection fallback;
/// 5. derive tangent 1 as `normal cross tangent_0`;
/// 6. copy the ten body-state vectors used by `ContactBatchBuild`.
///
/// The retail input normal is already unit length. This routine intentionally
/// does not renormalize it or repair malformed collision input.
pub fn generate_contact(
    input: RetailContactInput,
    body_a: RetailContactBodyState,
    body_b: RetailContactBodyState,
) -> RetailContact {
    let arm_a = sub(input.position_on_a, body_a.center_of_mass);
    let arm_b = sub(input.position_on_b, body_b.center_of_mass);
    let point_velocity_a = point_velocity(body_a.linear_velocity, body_a.angular_velocity, arm_a);
    let point_velocity_b = point_velocity(body_b.linear_velocity, body_b.angular_velocity, arm_b);
    let relative_velocity = sub(point_velocity_b, point_velocity_a);

    let velocity_tangent = cross(relative_velocity, input.normal);
    let velocity_tangent_squared = length_squared(velocity_tangent);
    let (unnormalized_tangent, tangent_squared) =
        if velocity_tangent_squared >= CONTACT_TANGENT_MINIMUM_SQUARED {
            (velocity_tangent, velocity_tangent_squared)
        } else {
            fallback_tangent(input.normal)
        };

    // TU3 issues one reciprocal-square-root estimate and applies it to both
    // axes. `tangent_1` therefore assumes the collision normal is unit length.
    let inverse_length = super::reciprocal_sqrt::estimate(tangent_squared);
    let tangent_0 = scale(unnormalized_tangent, inverse_length);
    // The second cross product rounds the positive product first, then uses
    // vnmsubfp for the negative term; tangent_0 uses the opposite order.
    let n = input.normal;
    let t = unnormalized_tangent;
    let tangent_1 = scale(
        Vector3::new(
            (-t.y).mul_add(n.z, t.z.mul_add(n.y, 0.0)),
            (-t.z).mul_add(n.x, t.x.mul_add(n.z, 0.0)),
            (-t.x).mul_add(n.y, t.y.mul_add(n.x, 0.0)),
        ),
        inverse_length,
    );

    RetailContact {
        position_on_a: input.position_on_a,
        body_a_id: body_a.contact_body_id,
        position_on_b: input.position_on_b,
        body_b_id: body_b.contact_body_id,
        normal: input.normal,
        restitution: input.restitution,
        tangent_0,
        static_friction: input.static_friction,
        tangent_1,
        dynamic_friction: input.dynamic_friction,
        relative_velocity,
        tag: input.tag,
        body_a_workspace: body_a.into(),
        body_b_workspace: body_b.into(),
    }
}

/// Exact normal-only fallback selected by the TU3 VMX block.
fn fallback_tangent(normal: Vector3) -> (Vector3, f32) {
    let candidate_x = Vector3::new(
        (-normal.x).mul_add(normal.x, 1.0),
        (-normal.y).mul_add(normal.x, 0.0),
        (-normal.z).mul_add(normal.x, 0.0),
    );
    let candidate_z = Vector3::new(
        (-normal.x).mul_add(normal.z, 0.0),
        (-normal.y).mul_add(normal.z, 0.0),
        (-normal.z).mul_add(normal.z, 1.0),
    );
    if (-normal.x).mul_add(normal.x, CONTACT_FALLBACK_HALF) >= -0.0 {
        (candidate_x, candidate_x.x)
    } else {
        (
            Vector3::new(-candidate_z.x, -candidate_z.y, -candidate_z.z),
            candidate_z.z,
        )
    }
}

fn point_velocity(linear: Vector3, omega: Vector3, arm: Vector3) -> Vector3 {
    // Native folds the linear velocity into two fused cross-product terms.
    Vector3::new(
        arm.z.mul_add(omega.y, (-arm.y).mul_add(omega.z, linear.x)),
        arm.x.mul_add(omega.z, (-arm.z).mul_add(omega.x, linear.y)),
        arm.y.mul_add(omega.x, (-arm.x).mul_add(omega.y, linear.z)),
    )
}

fn sub(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn scale(vector: Vector3, scalar: f32) -> Vector3 {
    Vector3::new(
        vector.x.mul_add(scalar, 0.0),
        vector.y.mul_add(scalar, 0.0),
        vector.z.mul_add(scalar, 0.0),
    )
}

fn cross(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(
        left.y.mul_add(right.z, (-left.z).mul_add(right.y, 0.0)),
        left.z.mul_add(right.x, (-left.x).mul_add(right.z, 0.0)),
        left.x.mul_add(right.y, (-left.y).mul_add(right.x, 0.0)),
    )
}

fn dot(left: Vector3, right: Vector3) -> f32 {
    left.z.mul_add(
        right.z,
        left.y.mul_add(right.y, left.x.mul_add(right.x, 0.0)),
    )
}

fn length_squared(vector: Vector3) -> f32 {
    dot(vector, vector)
}

#[cfg(test)]
#[path = "tests/contact.rs"]
mod tests;
