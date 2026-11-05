use crate::physics::reciprocal_sqrt::estimate;

type V = [f32; 4];

/// Complete TU3 82ACEA30 candidate producer. GP records are the native 192-byte
/// representation: origin +0, normals +16/+32/+48, edges +64 onward, counts
/// bytes +140/+141. All four output lanes and speculative rejected stores are
/// preserved. Returns zero when the native function returns null.
pub fn separating_axis_candidates(
    a: &[u32; 48],
    b: &[u32; 48],
    output: &mut [[u32; 4]; 16],
) -> usize {
    let mut count = 0;
    for gp in [a, b] {
        let normals = gp[35] >> 24;
        if (1..=3).contains(&normals) {
            for index in (1..=normals as usize).rev() {
                output[count].copy_from_slice(&gp[index * 4..index * 4 + 4]);
                count += 1;
            }
        }
    }
    let ae = ((a[35] >> 16) & 255) as usize;
    let be = ((b[35] >> 16) & 255) as usize;
    assert!(ae <= 3 && be <= 3, "native GP contains at most three edges");
    for i in 0..ae {
        for j in 0..be {
            let candidate = cross(load(a, 16 + 4 * i), load(b, 16 + 4 * j));
            let length_squared = dot(candidate);
            output[count] = candidate.map(f32::to_bits);
            if length_squared > f32::from_bits(0x3a83_126f) {
                output[count] = normalized(candidate, length_squared).map(f32::to_bits);
                count += 1;
            }
        }
    }
    if ae == 1 && be == 1 {
        let edge_a = load(a, 16);
        let edge_b = load(b, 16);
        let perpendicular = cross(edge_a, edge_b);
        if length(perpendicular) > f32::from_bits(0x3400_0000) {
            // Unlike the edge-pair loop, these two secondary directions are
            // unconditionally normalized once the common perpendicular passes.
            for edge in [edge_a, edge_b] {
                let candidate = cross(edge, perpendicular);
                output[count] = normalized(candidate, dot(candidate)).map(f32::to_bits);
                count += 1;
            }
        }
    }
    if count != 0 {
        return count;
    }
    let delta = sub(load(a, 0), load(b, 0));
    if ae == 1 && be == 1 {
        let edge = load(a, 16);
        append(cross(cross(delta, edge), edge), output, &mut count);
    }
    if ae + be == 1 {
        let edge = if ae != 0 { load(a, 16) } else { load(b, 16) };
        append(cross(cross(delta, edge), edge), output, &mut count);
    }
    if count != 0 {
        return count;
    }
    append(delta, output, &mut count);
    count
}

fn append(candidate: V, output: &mut [[u32; 4]; 16], count: &mut usize) {
    output[*count] = candidate.map(f32::to_bits);
    if length(candidate) > f32::from_bits(0x3400_0000) {
        output[*count] = normalized(candidate, dot(candidate)).map(f32::to_bits);
        *count += 1;
    }
}

fn load(gp: &[u32; 48], offset: usize) -> V {
    std::array::from_fn(|i| f32::from_bits(gp[offset + i]))
}

fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}

fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}

fn dot(a: V) -> f32 {
    super::arithmetic::dot([a[0], a[1], a[2]], [a[0], a[1], a[2]])
}

fn inverse_length(squared: f32) -> f32 {
    let mut r = estimate(squared);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-squared).mul_add(r * r, 1.0), r);
    }
    r
}

fn normalized(a: V, squared: f32) -> V {
    let reciprocal = inverse_length(squared);
    a.map(|v| v * reciprocal)
}

fn length(a: V) -> f32 {
    let squared = dot(a);
    if squared == 0.0 {
        0.0
    } else {
        squared * inverse_length(squared)
    }
}
