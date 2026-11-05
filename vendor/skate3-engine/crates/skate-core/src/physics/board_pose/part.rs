use super::{PoseMatrix, arithmetic::*, orthonormalize_part_basis};

/// Explicit referenced records for a native 96-byte Part. Optional fields model
/// the actual null-pointer branches; callers own loading and activation policy.
#[derive(Clone, Debug)]
pub struct PartPose {
    pub transform: PoseMatrix,
    /// Record addressed by Part+68 (the local mass-frame transform).
    pub local_mass_frame: Option<PoseMatrix>,
    pub body: Option<[u32; 44]>,
    pub inertia: Option<[u32; 10]>,
}

/// 82585CB0: return the Part transform from its body, or its cached part matrix
/// if no body exists. This is not the whole game Body::GetPosition method.
pub fn part_transform(part: &PartPose) -> PoseMatrix {
    let Some(body) = part.body.as_ref() else {
        return part.transform;
    };
    let m = [
        load(body, 16),
        load(body, 20),
        load(body, 24),
        load(body, 4),
    ];
    words(if let Some(local) = part.local_mass_frame {
        compose(m, matrix(local))
    } else {
        m
    })
}

/// 82BD4318: update a live body's pose/caches, then always copy the original
/// requested matrix to the part. Velocity, forces and other packed lanes survive.
pub fn set_part_transform(part: &mut PartPose, requested: PoseMatrix) {
    if let Some(body) = part.body.as_mut() {
        let m = if let Some(local) = part.local_mass_frame {
            words(compose(matrix(requested), inverse_rigid(matrix(local))))
        } else {
            requested
        };
        let m = matrix(orthonormalize_part_basis(m));
        for (off, v) in [(16, m[0]), (20, m[1]), (24, m[2]), (4, m[3])] {
            store_xyz(body, off, v);
        }
        body[..4].copy_from_slice(&quaternion(m).map(f32::to_bits));
        if let Some(inertia) = part.inertia {
            let [ri, up, at, _] = m;
            let r = mul(ri, [f32::from_bits(inertia[0]); 4]);
            let u = mul(up, [f32::from_bits(inertia[1]); 4]);
            let a = mul(at, [f32::from_bits(inertia[2]); 4]);
            let full = madd(a, [at[0]; 4], madd(r, [ri[0]; 4], mul(u, [up[0]; 4])));
            let p = |v| perm(v, [2, 1, 1, 3]);
            let s = |v| perm(v, [2, 1, 2, 3]);
            let split = madd(p(at), s(a), madd(p(ri), s(r), mul(p(up), s(u))));
            store_xyz(body, 28, full);
            store_xyz(body, 32, split);
            body[31] = inertia[4];
        }
    }
    part.transform = requested;
}

fn quaternion(m: M) -> V {
    let [ri, up, at, _] = m;
    // Sign masks are initialized by TU3 82F83360/90/C0, not image defaults.
    let rx = [ri[0], ri[0], -ri[0], -ri[0]];
    let uy = [up[1], -up[1], up[1], -up[1]];
    let az = [at[2], -at[2], -at[2], at[2]];
    let first = add(rx, uy);
    let d = add(first, add(az, [1.0; 4]));
    let inv = d.map(rsqrt);
    let roots = mul(d, inv);
    let halves = mul([0.5; 4], inv);
    let a = [up[2], at[0], ri[1], 0.5];
    let b = [at[1], ri[2], up[0], 0.0];
    let minus = sub(a, b);
    let plus = add(a, b);
    let y = mul(
        [plus[2], minus[3], plus[0], minus[1]],
        [halves[2], roots[2], halves[2], halves[2]],
    );
    let z = mul(
        [plus[1], plus[0], minus[3], minus[2]],
        [halves[3], halves[3], roots[3], halves[3]],
    );
    let x = mul(
        [minus[3], plus[2], plus[1], minus[0]],
        [roots[1], halves[1], halves[1], halves[1]],
    );
    let w = mul(minus, [halves[0], halves[0], halves[0], roots[0]]);
    let yz = if up[1] > at[2] { y } else { z };
    let xyz = if ri[0] > up[1] && ri[0] > at[2] {
        x
    } else {
        yz
    };
    normalize(if first[0] + at[2] > 0.0 { w } else { xyz }, true)
}
fn load(w: &[u32], off: usize) -> V {
    core::array::from_fn(|i| f32::from_bits(w[off + i]))
}
fn store_xyz(w: &mut [u32], off: usize, v: V) {
    for i in 0..3 {
        w[off + i] = v[i].to_bits();
    }
}
