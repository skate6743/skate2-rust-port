use crate::physics::board_pose::{PartPose, PoseMatrix};

type V = [f32; 4];
type M = [V; 4];

fn matrix(words: PoseMatrix) -> M {
    std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(words[i * 4 + j])))
}
fn madd(a: V, s: f32, b: V) -> V {
    std::array::from_fn(|i| a[i].mul_add(s, b[i]))
}
fn scale(a: V, s: f32) -> V {
    a.map(|v| v * s)
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn dot(a: V, b: V) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}

/// Complete 82585B58. Live bodies return their mass-frame basis/COM directly.
/// A body-less part with a local mass frame removes that frame from its cache.
/// This is distinct from 82585CB0's geometric-part transform getter.
pub fn part_mass_transform(part: &PartPose) -> PoseMatrix {
    if let Some(body) = &part.body {
        return std::array::from_fn(|i| body[[16, 20, 24, 4][i / 4] + i % 4]);
    }
    let Some(local) = part.local_mass_frame else {
        return part.transform;
    };
    let local = matrix(local);
    let basis: [V; 3] = std::array::from_fn(|i| [local[0][i], local[1][i], local[2][i], 0.0]);
    let neg = local[3].map(|v| 0.0 - v);
    let position = madd(
        basis[0],
        neg[0],
        madd(basis[1], neg[1], scale(basis[2], neg[2])),
    );
    let cached = matrix(part.transform);
    let result: M = std::array::from_fn(|i| {
        let v = if i == 3 { position } else { basis[i] };
        let first = if i == 3 {
            madd(cached[0], v[0], cached[3])
        } else {
            scale(cached[0], v[0])
        };
        madd(cached[2], v[2], madd(cached[1], v[1], first))
    });
    std::array::from_fn(|i| result[i / 4][i % 4].to_bits())
}

/// Complete 82C08968: observe wheel COM positions in inverse deck mass space,
/// average native pairs0/1 and2/3, subtract board+8380, store+8372/+8376 only.
pub fn calculate_wheel_compressions(board: &mut [u8; 8400], parts: &[PartPose; 7]) {
    let deck = matrix(part_mass_transform(&parts[6]));
    let wheels = std::array::from_fn(|i| matrix(part_mass_transform(&parts[i]))[3]);
    let distance = f32::from_bits(u32::from_be_bytes(board[8380..8384].try_into().unwrap()));
    let heights = average_wheel_compressions(deck, wheels, distance);
    for (offset, height) in [(8372, heights[0]), (8376, heights[1])] {
        let word = if height.is_nan() {
            height.to_bits() | 0x0040_0000
        } else {
            height.to_bits()
        };
        board[offset..offset + 4].copy_from_slice(&word.to_be_bytes());
    }
}

///82C08968, with live mass-frame observations rather than a byte-buffer owner.
///Rest height is physicstrucks/default.TruckYPos, copied to8380 by82C08D18.
pub fn average_wheel_compressions(deck: M, wheel_positions: [V; 4], rest_height: f32) -> [f32; 2] {
    let c0 = cross(deck[1], deck[2]);
    let c1 = cross(deck[2], deck[0]);
    let c2 = cross(deck[0], deck[1]);
    let determinant = dot(deck[0], c0);
    // Native reciprocal estimate and two refinements.
    let mut inverse = reciprocal_estimate(determinant);
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(determinant, 1.0), inverse);
    }
    let basis: [V; 3] = std::array::from_fn(|i| scale([c0[i], c1[i], c2[i], c1[i]], inverse));
    let neg = deck[3].map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    let position = madd(
        basis[2],
        neg[2],
        madd(basis[1], neg[1], scale(basis[0], neg[0])),
    );
    let heights: [f32; 4] = std::array::from_fn(|i| {
        let p = wheel_positions[i];
        let local = madd(
            basis[2],
            p[2],
            madd(basis[1], p[1], madd(basis[0], p[0], position)),
        );
        local[1]
    });
    [
        (heights[0] + heights[1]) * 0.5 - rest_height,
        (heights[2] + heights[3]) * 0.5 - rest_height,
    ]
}

fn reciprocal_estimate(value: f32) -> f32 {
    crate::physics::native_arithmetic::reciprocal_estimate(value)
}
