//! Scalar lane of native SIMD atan 82473B98 and intention left-stick angle.
pub(super) fn reciprocal_estimate(value: f32) -> f32 {
    crate::physics::native_arithmetic::reciprocal_estimate(value)
}

fn refined_reciprocal(value: f32) -> f32 {
    let initial = reciprocal_estimate(value);
    let first = initial.mul_add((-value).mul_add(initial, 1.0), initial);
    let second = first.mul_add((-value).mul_add(first, 1.0), first);
    if first.is_nan() { initial } else { second }
}
pub fn atan(value: f32) -> f32 {
    let absolute = value.abs();
    let invert = absolute > 1.0;
    let reduced = if invert {
        refined_reciprocal(absolute)
    } else {
        absolute
    };
    let transform = reduced > f32::from_bits(0x3e8930a3);
    let offset = if transform {
        if invert {
            f32::from_bits(0x3f860a92)
        } else {
            f32::from_bits(0x3f060a92)
        }
    } else if invert {
        f32::from_bits(0x3fc90fdb)
    } else {
        0.0
    };
    let transformed = (reduced.mul_add(f32::from_bits(0x3f3b67af), reduced) - 1.0)
        * refined_reciprocal(reduced + f32::from_bits(0x3fddb3d7));
    let t = if transform { transformed } else { reduced };
    let square = t * t;
    let numerator = square.mul_add(f32::from_bits(0xbf566bd7), f32::from_bits(0xc107e9fb));
    let numerator = square.mul_add(numerator, f32::from_bits(0xc1a40bfe));
    let numerator = square.mul_add(numerator, f32::from_bits(0xc15b0533));
    let denominator = square.mul_add(
        square + f32::from_bits(0x4170624f),
        f32::from_bits(0x426e5052),
    );
    let denominator = square.mul_add(denominator, f32::from_bits(0x42ac5090));
    let denominator = square.mul_add(denominator, f32::from_bits(0x422443e6));
    let polynomial = t.mul_add((numerator * square) * refined_reciprocal(denominator), t);
    let result = if f32::from_bits(0x39800000) > t.abs() {
        t
    } else {
        polynomial
    };
    let result = (if invert { -result } else { result }) + offset;
    let result = if value < 0.0 { -result } else { result };
    let result = if value > f32::from_bits(0x7e800000) {
        f32::from_bits(0x3fc90fdb)
    } else {
        result
    };
    if -f32::from_bits(0x7e800000) > value {
        -f32::from_bits(0x3fc90fdb)
    } else {
        result
    }
}

pub(super) fn left_stick_angle(x: f32, y: f32) -> f32 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let vertical = -y;
    let reciprocal = reciprocal_estimate(vertical);
    let refined = reciprocal.mul_add((-reciprocal).mul_add(vertical, 1.0), reciprocal);
    let basic = atan(x.mul_add(refined, 0.0));
    let sign = x.to_bits() & 0x80000000;
    let pi = f32::from_bits(0x40490fdb | sign);
    let half_pi = f32::from_bits(0x3fc90fdb | sign);
    let angle = if 0.0 > vertical { pi + basic } else { basic };
    if vertical == 0.0 { half_pi } else { angle }
}
