//! TU3 RenderWare rigid-body accumulation and integration primitives.
//!
//! These routines are ports of the arithmetic in the Skate 3 TU3 executable,
//! not a wrapper around Bevy or a generic rigid-body engine. The caller supplies
//! solver corrections; this module does not own collision or solver scheduling.

pub use super::point_force::{RetailForceAccumulator, accumulate_point_force};
use crate::math::{Basis3, Vector3};
mod dynamic_update;
pub use dynamic_update::{DynamicUpdateResult, dynamic_update_packed};

pub mod tu3 {
    /// Per-active-body integration routine called by
    /// `rw::physics::Simulation::BatchIntegrator`.
    pub const BATCH_INTEGRATOR_BODY: u32 = 0x82AE_6590;
    /// `Sk8::Physics::Skateboard::ApplyQueuedSkateboardForces`.
    pub const APPLY_QUEUED_SKATEBOARD_FORCES: u32 = 0x82C0_3718;
}

/// Exact scalar fields read from TU3's `rw::physics::Simulation` by
/// `0x82AE6590`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailSimulationStep {
    pub time_step: f32,
    pub frequency: f32,
    pub cool_down: u32,
    pub minimum_energy: f32,
    pub gravity_acceleration: Vector3,
}

impl RetailSimulationStep {
    pub fn fixed_60_hz(cool_down: u32, minimum_energy: f32, gravity_acceleration: Vector3) -> Self {
        let time_step = f32::from_bits(0x3C88_8889);
        Self {
            time_step,
            frequency: time_step.recip(),
            cool_down,
            minimum_energy,
            gravity_acceleration,
        }
    }
}

/// Exact scalar layout of `rw::physics::Inertia` after its inverse-tensor
/// vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailInertiaDynamics {
    pub inverse_tensor: Vector3,
    pub inverse_mass: f32,
    pub spherical: f32,
    pub maximum_linear_velocity: f32,
    pub maximum_angular_velocity: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
}

/// Inverse local principal-axis frame stored by TU3 `ComputeMassProperties`
/// (`0x82AE7770`) at `PartDefinition + 64` (`inverseBodyLTM`).
///
/// The stock wheel and truck retain identity. Deck construction `0x82C09290`
/// also explicitly resets this frame to identity after computing aggregate
/// principal inertia; that override does not discard the calculated inertia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailLocalMassFrame {
    /// Column vectors in RenderWare `Ri`, `Up`, `At` order.
    pub basis: Basis3,
    pub translation: Vector3,
}

impl RetailLocalMassFrame {
    pub const IDENTITY: Self = Self {
        basis: Basis3 {
            columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        translation: Vector3::ZERO,
    };
}

/// Complete per-part mass data needed by the TU3 rigid-body integrator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailBodyMassProperties {
    pub local_mass_frame: RetailLocalMassFrame,
    pub dynamics: RetailInertiaDynamics,
}

/// The four 16-byte correction vectors indexed by `RigidBody::mId`.
///
/// The first and third vectors participate in the velocity-producing frame
/// displacement. The second and fourth vectors correct position and
/// orientation only. This separation is directly visible in `0x82AE6590`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RetailReactionCorrections {
    pub linear_displacement: Vector3,
    pub position_displacement: Vector3,
    pub angular_displacement: Vector3,
    pub orientation_displacement: Vector3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailBodyRates {
    pub orientation: RetailQuaternion,
    pub basis: Basis3,
    pub world_inverse_inertia: Basis3,
    pub position: Vector3,
    pub linear_velocity: Vector3,
    pub angular_velocity: Vector3,
    /// Acceleration accumulator. `RigidBody::AddForce` has already multiplied
    /// physical force by inverse mass before this field reaches the integrator.
    pub force_acceleration: Vector3,
    /// Angular-acceleration accumulator. Point-force torque has already passed
    /// through the body's world inverse inertia.
    pub torque_acceleration: Vector3,
    pub kinetic_energy: f32,
    pub cool_down: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailBodyRateStep {
    pub state: RetailBodyRates,
    /// Rotation increment consumed by the quaternion branch in the same retail
    /// function. Keeping this explicit prevents a presentation system from
    /// replacing it with a surface-normal snap.
    pub orientation_displacement: Vector3,
    pub linear_speed_squared: f32,
    pub angular_speed_squared: f32,
}

/// Skate-era RenderWare's six-value packed symmetric world inverse-inertia
/// tensor.
///
/// The lane mapping is:
///
/// - `mIfull = (Ixx, Ixy, Ixz)`;
/// - `mIsplt = (Izz, Iyy, Iyz)`.
///
/// Independently traced through original S3 DynamicUpdate82AE6778..6804 and
/// S2 DynamicUpdate82AE467C..46FC. This verifies the six-component layout,
/// not bit-exact Xenon arithmetic; generated-code validation is withdrawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailPackedWorldInverseInertia {
    pub full: Vector3,
    pub split: Vector3,
}

/// RenderWare quaternion lane order is `(x, y, z, w)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailQuaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl RetailQuaternion {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
}

/// Typed adapter to the complete TU3 `RigidBody::DynamicUpdate` (`0x82AE6590`).
/// The packed entry point additionally preserves guest metadata and clears its
/// mutable reaction record. This value-taking adapter returns the updated body.
///
/// The order matters:
///
/// 1. acceleration is integrated to a candidate velocity;
/// 2. candidate velocity is converted to a frame displacement;
/// 3. solver correction displacements are added;
/// 4. position is advanced;
/// 5. velocity is reconstructed from displacement with retail drag;
/// 6. linear and angular speed caps are applied;
/// 7. energy/cool-down is updated;
/// 8. force/torque accumulators are reset for the next step.
///
/// `BoardStep` supplies all four correction vectors after the shared contact,
/// joint and drive solve. Player-state scheduling remains a separate owner.
pub fn integrate_body_rates(
    mut body: RetailBodyRates,
    inertia: RetailInertiaDynamics,
    simulation: RetailSimulationStep,
    reactions: RetailReactionCorrections,
) -> RetailBodyRateStep {
    let mut words = [0_u32; 44];
    words[..4].copy_from_slice(&[
        body.orientation.x.to_bits(),
        body.orientation.y.to_bits(),
        body.orientation.z.to_bits(),
        body.orientation.w.to_bits(),
    ]);
    for (offset, value) in [
        (4, body.position),
        (8, body.linear_velocity),
        (12, body.angular_velocity),
        (36, body.force_acceleration),
        (40, body.torque_acceleration),
    ] {
        words[offset..offset + 3].copy_from_slice(&[
            value.x.to_bits(),
            value.y.to_bits(),
            value.z.to_bits(),
        ]);
    }
    words[31] = inertia.inverse_mass.to_bits();
    words[39] = body.kinetic_energy.to_bits();
    words[43] = body.cool_down;
    let native_inertia = [
        inertia.inverse_tensor.x,
        inertia.inverse_tensor.y,
        inertia.inverse_tensor.z,
        0.0,
        inertia.inverse_mass,
        inertia.spherical,
        inertia.maximum_linear_velocity,
        inertia.maximum_angular_velocity,
        inertia.linear_drag,
        inertia.angular_drag,
    ]
    .map(f32::to_bits);
    let mut correction_words = [0; 16];
    for (index, value) in [
        reactions.linear_displacement,
        reactions.position_displacement,
        reactions.angular_displacement,
        reactions.orientation_displacement,
    ]
    .into_iter()
    .enumerate()
    {
        correction_words[index * 4..index * 4 + 3].copy_from_slice(&[
            value.x.to_bits(),
            value.y.to_bits(),
            value.z.to_bits(),
        ]);
    }
    let result = dynamic_update_packed(
        &mut words,
        &native_inertia,
        simulation,
        &mut correction_words,
    );
    let vector = |offset| {
        Vector3::new(
            f32::from_bits(words[offset]),
            f32::from_bits(words[offset + 1]),
            f32::from_bits(words[offset + 2]),
        )
    };
    body.orientation = RetailQuaternion {
        x: f32::from_bits(words[0]),
        y: f32::from_bits(words[1]),
        z: f32::from_bits(words[2]),
        w: f32::from_bits(words[3]),
    };
    body.position = vector(4);
    body.linear_velocity = vector(8);
    body.angular_velocity = vector(12);
    body.force_acceleration = vector(36);
    body.torque_acceleration = vector(40);
    body.kinetic_energy = f32::from_bits(words[39]);
    body.cool_down = words[43];
    body.basis = Basis3 {
        columns: core::array::from_fn(|i| {
            core::array::from_fn(|j| f32::from_bits(words[16 + i * 4 + j]))
        }),
    };
    body.world_inverse_inertia = unpack_world_inverse_inertia(vector(28), vector(32));
    RetailBodyRateStep {
        state: body,
        orientation_displacement: Vector3::new(
            result.orientation_displacement[0],
            result.orientation_displacement[1],
            result.orientation_displacement[2],
        ),
        linear_speed_squared: result.linear_speed_squared,
        angular_speed_squared: result.angular_speed_squared,
    }
}

/// Quaternion branch from TU3 `0x82AE6668..0x82AE66C8`, paired with
/// Skate 2 `0x82AE455C..0x82AE45C8`.
///
/// Mapping the VMX128 word permutations back to RenderWare's public
/// `(x,y,z,w)` lane order gives:
///
/// `q += 0.5 * Quaternion(angular_displacement, 0) * q`
///
/// followed by normalization. This pre-multiplication order is important:
/// swapping it changes the board's world-space angular response.
pub fn integrate_orientation(
    orientation: RetailQuaternion,
    angular_displacement: Vector3,
) -> RetailQuaternion {
    let q = dynamic_update::orientation(
        [orientation.x, orientation.y, orientation.z, orientation.w],
        [
            angular_displacement.x,
            angular_displacement.y,
            angular_displacement.z,
            0.0,
        ],
    );
    RetailQuaternion {
        x: q[0],
        y: q[1],
        z: q[2],
        w: q[3],
    }
}

/// `Ri`, `Up`, and `At` columns rebuilt immediately after TU3 normalizes the
/// quaternion in `0x82AE6590`.
pub fn basis_from_quaternion(q: RetailQuaternion) -> Basis3 {
    let columns = dynamic_update::quaternion_basis([q.x, q.y, q.z, q.w]);
    Basis3 {
        columns: columns.map(|column| [column[0], column[1], column[2]]),
    }
}

/// Native packed inverse-inertia rebuild in 82AE6590, with its fused order.
pub fn world_inverse_inertia(basis: Basis3, inverse_tensor: Vector3) -> Basis3 {
    let native_basis = basis.columns.map(|v| [v[0], v[1], v[2], 0.0]);
    let tensor = [inverse_tensor.x, inverse_tensor.y, inverse_tensor.z].map(f32::to_bits);
    let (full, split) = dynamic_update::inverse_inertia(native_basis, tensor);
    unpack_world_inverse_inertia(
        Vector3::new(full[0], full[1], full[2]),
        Vector3::new(split[0], split[1], split[2]),
    )
}

fn unpack_world_inverse_inertia(full: Vector3, split: Vector3) -> Basis3 {
    Basis3 {
        columns: [
            [full.x, full.y, full.z],
            [full.y, split.y, split.z],
            [full.z, split.z, split.x],
        ],
    }
}

/// Packs a symmetric world inverse-inertia tensor in RenderWare's `mIfull` /
/// `mIsplt` lane order.
pub fn pack_world_inverse_inertia(tensor: Basis3) -> RetailPackedWorldInverseInertia {
    RetailPackedWorldInverseInertia {
        full: Vector3::new(
            tensor.columns[0][0],
            tensor.columns[1][0],
            tensor.columns[2][0],
        ),
        split: Vector3::new(
            tensor.columns[2][2],
            tensor.columns[1][1],
            tensor.columns[2][1],
        ),
    }
}

/// Multiplies a vector by the exact symmetric tensor represented by retail
/// `mIfull` and `mIsplt`.
pub fn multiply_packed_world_inverse_inertia(
    tensor: RetailPackedWorldInverseInertia,
    vector: Vector3,
) -> Vector3 {
    Vector3::new(
        tensor.full.x * vector.x + tensor.full.y * vector.y + tensor.full.z * vector.z,
        tensor.full.y * vector.x + tensor.split.y * vector.y + tensor.split.z * vector.z,
        tensor.full.z * vector.x + tensor.split.z * vector.y + tensor.split.x * vector.z,
    )
}

#[cfg(test)]
fn multiply_basis(basis: Basis3, vector: Vector3) -> Vector3 {
    Vector3::new(
        basis.columns[0][0] * vector.x
            + basis.columns[1][0] * vector.y
            + basis.columns[2][0] * vector.z,
        basis.columns[0][1] * vector.x
            + basis.columns[1][1] * vector.y
            + basis.columns[2][1] * vector.z,
        basis.columns[0][2] * vector.x
            + basis.columns[1][2] * vector.y
            + basis.columns[2][2] * vector.z,
    )
}

#[cfg(test)]
#[path = "tests/rigid_body.rs"]
mod tests;

// Existing equation-level unit checks retain their independent scalar helpers.
#[cfg(test)]
fn column(basis: Basis3, index: usize) -> Vector3 {
    let c = basis.columns[index];
    Vector3::new(c[0], c[1], c[2])
}
#[cfg(test)]
fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[cfg(test)]
fn scale(v: Vector3, s: f32) -> Vector3 {
    Vector3::new(v.x * s, v.y * s, v.z * s)
}
#[cfg(test)]
fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
#[cfg(test)]
fn dot(a: Vector3, b: Vector3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[cfg(test)]
fn length_squared(v: Vector3) -> f32 {
    dot(v, v)
}
#[cfg(test)]
fn normalize_quaternion(q: RetailQuaternion) -> RetailQuaternion {
    let inverse = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w)
        .sqrt()
        .recip();
    RetailQuaternion {
        x: q.x * inverse,
        y: q.y * inverse,
        z: q.z * inverse,
        w: q.w * inverse,
    }
}
