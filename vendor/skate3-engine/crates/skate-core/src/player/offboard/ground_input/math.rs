//! Independently derived from original82D310F8 and82473B98.
//! Ordinary PC reciprocal/sqrt seeds retain original refinements; they are not
//! claimed to reproduce Xenon estimate instructions bit for bit.

type Vector = [f32; 3];
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const PI: f32 = f32::from_bits(0x4049_0fdb);

pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    // PC dot3, without a hardware VMX sum-rounding parity claim.
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}

pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
    ]
}

fn inverse_sqrt(q: f32) -> f32 {
    let mut r = 1.0 / q.sqrt();
    for _ in 0..2 {
        let square = r * r;
        let half = r * 0.5;
        let error = (-q).mul_add(square, 1.0);
        r = half.mul_add(error, r);
    }
    r
}

pub(super) fn normalize(v: Vector) -> Vector {
    let r = inverse_sqrt(dot(v, v));
    v.map(|value| value * r)
}

pub(super) fn length(v: Vector) -> f32 {
    let q = dot(v, v);
    let result = q * inverse_sqrt(q);
    if q == 0.0 { 0.0 } else { result }
}

pub(super) fn inverse_frame(right: Vector, up: Vector, forward: Vector, v: Vector) -> Vector {
    let c0 = cross(up, forward);
    let c1 = cross(forward, right);
    let c2 = cross(right, up);
    let determinant = dot(right, c0);
    let mut reciprocal = 1.0 / determinant;
    for _ in 0..2 {
        reciprocal = reciprocal.mul_add((-determinant).mul_add(reciprocal, 1.0), reciprocal);
    }
    // Scale inverse columns before multiplying and accumulating input components.
    let c = [c0, c1, c2];
    std::array::from_fn(|i| {
        let x = c[i][0] * reciprocal;
        let y = c[i][1] * reciprocal;
        let z = c[i][2] * reciprocal;
        z.mul_add(v[2], y.mul_add(v[1], x * v[0]))
    })
}

pub(super) fn signed_angle(x: f32, y: f32) -> f32 {
    let seed = 1.0 / y;
    let reciprocal = seed.mul_add((-y).mul_add(seed, 1.0), seed);
    let mut angle = rational_atan(x.mul_add(reciprocal, 0.0));
    if y < 0.0 {
        angle += PI.copysign(x);
    }
    if y == 0.0 {
        angle = HALF_PI.copysign(x);
    }
    angle
}

fn reciprocal_two(d: f32) -> f32 {
    let seed = 1.0 / d;
    let first = seed.mul_add((-d).mul_add(seed, 1.0), seed);
    let second = first.mul_add((-d).mul_add(first, 1.0), first);
    // Original compares the FIRST refinement before selecting the second.
    if first.is_nan() { seed } else { second }
}

///82473B98: range reduction, rational polynomial, strict thresholds and signs.
pub(super) fn rational_atan(original: f32) -> f32 {
    let absolute = original.abs();
    let invert = absolute > 1.0;
    let reciprocal = reciprocal_two(absolute);
    let x = if invert { reciprocal } else { absolute };
    let base_offset = if invert { HALF_PI } else { 0.0 };
    let reduced_offset = if invert {
        f32::from_bits(0x3f86_0a92)
    } else {
        f32::from_bits(0x3f06_0a92)
    };
    let denominator = x + f32::from_bits(0x3fdd_b3d7);
    let numerator = x.mul_add(f32::from_bits(0x3f3b_67af), x) - 1.0;
    let transformed = numerator * reciprocal_two(denominator);
    let reduce = x > f32::from_bits(0x3e89_30a3);
    let t = if reduce { transformed } else { x };
    let offset = if reduce { reduced_offset } else { base_offset };
    let q = t * t;
    let mut p = q.mul_add(f32::from_bits(0xbf56_6bd7), f32::from_bits(0xc107_e9fb));
    p = q.mul_add(p, f32::from_bits(0xc1a4_0bfe));
    p = q.mul_add(p, f32::from_bits(0xc15b_0533));
    p *= q;
    let mut d = q + f32::from_bits(0x4170_624f);
    d = q.mul_add(d, f32::from_bits(0x426e_5052));
    d = q.mul_add(d, f32::from_bits(0x42ac_5090));
    d = q.mul_add(d, f32::from_bits(0x4224_43e6));
    let correction = p * reciprocal_two(d);
    let polynomial = t.mul_add(correction, t);
    let mut result = if t.abs() < f32::from_bits(0x3980_0000) {
        t
    } else {
        polynomial
    };
    if invert {
        result = -result;
    }
    result += offset;
    if original < 0.0 {
        result = -result;
    }
    let huge = f32::from_bits(0x7e80_0000);
    if original > huge {
        result = HALF_PI;
    }
    if original < -huge {
        result = -HALF_PI;
    }
    result
}
