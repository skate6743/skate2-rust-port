//! RigidBody::DynamicUpdate82AE6590 reconstruction candidate.
//! Guest pointers are opaque metadata; the caller supplies their referenced data.
use super::RetailSimulationStep;
type V = [f32; 4];

#[derive(Clone, Copy, Debug)]
pub struct DynamicUpdateResult {
    pub orientation_displacement: [f32; 3],
    /// Native cap-selected squared speeds used by the sleep metric.
    pub linear_speed_squared: f32,
    pub angular_speed_squared: f32,
}

/// Update every native body field and clear all 64 reaction bytes. `body` is the
/// 176-byte RigidBody prefix; `inertia` is its 40-byte Inertia record. Metadata
/// lanes, including pointers and IDs, are preserved. No activation is performed.
pub fn dynamic_update_packed(
    body: &mut [u32; 44],
    inertia: &[u32; 10],
    simulation: RetailSimulationStep,
    reactions: &mut [u32; 16],
) -> DynamicUpdateResult {
    let dt = [simulation.time_step; 4];
    let linear = madd(
        madd(load(body, 36), dt, load(body, 8)),
        dt,
        load(reactions, 0),
    );
    let angular = madd(
        madd(load(body, 40), dt, load(body, 12)),
        dt,
        load(reactions, 8),
    );
    let position = add(add(load(body, 4), load(reactions, 4)), linear);
    store_xyz(body, 4, position);
    let rotation = add(angular, load(reactions, 12));
    let q = orientation(load(body, 0), rotation);
    body[..4].copy_from_slice(&q.map(f32::to_bits));
    let basis = quaternion_basis(q);
    for (offset, column) in [16, 20, 24].into_iter().zip(basis) {
        store_xyz(body, offset, column);
    }
    let tensor = inertia[..3].try_into().unwrap();
    let (full, split) = inverse_inertia(basis, tensor);
    store_xyz(body, 28, full);
    store_xyz(body, 32, split);
    let rate = |drag: u32| {
        let difference = simulation.frequency - f32::from_bits(drag);
        if difference > 0.0 { difference } else { 0.0 }
    };
    let (omega, angular_squared) = cap(
        mul(angular, [rate(inertia[9]); 4]),
        f32::from_bits(inertia[7]),
    );
    let (velocity, linear_squared) = cap(
        mul(linear, [rate(inertia[8]); 4]),
        f32::from_bits(inertia[6]),
    );
    // Scalar fmuls followed by fmadds; cap branches publish the squared cap,
    // rather than recalculating squared speed from the rounded scaled vector.
    let factor = f32::from_bits(inertia[5]) * f32::from_bits(body[31]);
    let energy = factor.mul_add(angular_squared, linear_squared);
    // S3 82AE6974 / S2 82AE487C: bge tests LT clear, including unordered.
    // Do not replace this with >=; the host predicate below is intentional.
    body[43] = if !(energy < simulation.minimum_energy) {
        0
    } else {
        let cooldown = if !(energy > f32::from_bits(body[39])) {
            body[43].wrapping_add(1)
        } else {
            body[43]
        };
        cooldown.min(simulation.cool_down)
    };
    body[39] = energy.to_bits();
    store_xyz(body, 8, velocity);
    store_xyz(body, 12, omega);
    reactions.fill(0);
    let gravity = simulation.gravity_acceleration;
    store_xyz(body, 36, [gravity.x, gravity.y, gravity.z, 0.0]);
    store_xyz(body, 40, [0.0; 4]);
    DynamicUpdateResult {
        orientation_displacement: rotation[..3].try_into().unwrap(),
        linear_speed_squared: linear_squared,
        angular_speed_squared: angular_squared,
    }
}

pub(super) fn orientation(q: V, rotation: V) -> V {
    // S3 82AE6668..66C8 / S2 82AE455C..45C8: cross product then normalization.
    let cross = perm(
        nmsub(
            perm(rotation, [1, 2, 0, 3]),
            q,
            mul(rotation, perm(q, [1, 2, 0, 3])),
        ),
        [1, 2, 0, 3],
    );
    let mut increment = mul(madd(rotation, [q[3]; 4], cross), [0.5; 4]);
    increment[3] = dot(rotation, q, false) * -0.5;
    let candidate = add(q, increment);
    mul(
        candidate,
        [refined_rsqrt(dot(candidate, candidate, true)); 4],
    )
}

pub(super) fn quaternion_basis(q: V) -> [V; 3] {
    // 82139A00 contains sqrt(2) rounded to binary32. Replacing these products
    // with 2*q_i*q_j changes the native basis words.
    let s = mul(q, [f32::from_bits(0x3FB5_04F3); 4]);
    let d = nmsub(s, s, [0.5; 4]);
    let diagonal = add(d, perm(d, [1, 2, 0, 0]));
    let products = mul(s, perm(s, [1, 2, 0, 0]));
    let w_products = mul([s[3]; 4], perm(s, [2, 0, 1, 0]));
    let plus = add(products, w_products);
    let minus = sub(products, w_products);
    [
        [diagonal[1], plus[0], minus[2], 0.0],
        [minus[0], diagonal[2], plus[1], 0.0],
        [plus[2], minus[1], diagonal[0], 0.0],
    ]
}

pub(super) fn inverse_inertia(basis: [V; 3], tensor: [u32; 3]) -> (V, V) {
    let [ri, up, at] = basis;
    let r = mul(ri, [f32::from_bits(tensor[0]); 4]);
    let u = mul(up, [f32::from_bits(tensor[1]); 4]);
    let a = mul(at, [f32::from_bits(tensor[2]); 4]);
    let full = madd(a, [at[0]; 4], madd(r, [ri[0]; 4], mul(u, [up[0]; 4])));
    let p = |v| perm(v, [2, 1, 1, 3]);
    let s = |v| perm(v, [2, 1, 2, 3]);
    let split = madd(p(at), s(a), madd(p(ri), s(r), mul(p(up), s(u))));
    (full, split)
}

fn cap(value: V, maximum: f32) -> (V, f32) {
    let squared = dot(value, value, false);
    let maximum_squared = maximum * maximum;
    // S3 82AE6888/68FC and S2 82AE478C/4804 skip on GT clear (ble),
    // including unordered. Thus ordered >, not !(<=), enters the cap path.
    if squared > maximum_squared {
        let ratio = maximum_squared / squared;
        let root = ratio * refined_rsqrt(ratio);
        let scale = if ratio == 0.0 { 0.0 } else { root };
        (mul(value, [scale; 4]), maximum_squared)
    } else {
        (value, squared)
    }
}

fn refined_rsqrt(value: f32) -> f32 {
    let mut estimate = super::super::reciprocal_sqrt::estimate(value);
    for _ in 0..2 {
        let square = estimate * estimate;
        let half_estimate = estimate * 0.5;
        let residual = (-value).mul_add(square, 1.0);
        estimate = half_estimate.mul_add(residual, estimate);
    }
    estimate
}

fn dot(a: V, b: V, four: bool) -> f32 {
    if four {
        crate::physics::native_arithmetic::dot4(a, b)
    } else {
        crate::physics::native_arithmetic::dot3(a, b)
    }
}

fn load(words: &[u32], offset: usize) -> V {
    core::array::from_fn(|i| f32::from_bits(words[offset + i]))
}
fn store_xyz(words: &mut [u32], offset: usize, value: V) {
    for i in 0..3 {
        words[offset + i] = value[i].to_bits();
    }
}
fn perm(value: V, indices: [usize; 4]) -> V {
    indices.map(|i| value[i])
}
fn mul(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] * b[i])
}
fn add(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
fn madd(a: V, b: V, c: V) -> V {
    core::array::from_fn(|i| a[i].mul_add(b[i], c[i]))
}
fn nmsub(a: V, b: V, c: V) -> V {
    core::array::from_fn(|i| (-a[i]).mul_add(b[i], c[i]))
}
