//! TU3 Init82E891D8, IDCT table82E88C58, DecompressFrameBlock82E89770.
//! All coefficient words below were read from the original TU3 mapped image.
use std::sync::OnceLock;

struct Tables {
    quantization: [[f32; 8]; 256],
    inverse_dct: [[f32; 8]; 8],
}
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let base: [f32; 8] = std::array::from_fn(|i| {
            logarithm_for_frequency(i as u32 + 2) * f32::from_bits(0x3800_0000)
        });
        let mut multiplier = 1.0f32;
        let quantization = std::array::from_fn(|_| {
            let row = base.map(|v| v * multiplier);
            multiplier += f32::from_bits(0x3e4c_cccd); // 82099280; accumulated fadds
            row
        });
        // 822F8B48..58 and individual .785398/.1.570796 literal loads.
        let angles = [
            0,
            0x3ec9_0fdb,
            0x3f49_0fdb,
            0x3f96_cbe4,
            0x3fc9_0fdb,
            0x3ffb_53d2,
            0x4016_cbe4,
            0x402f_ede0,
        ];
        let inverse_dct = std::array::from_fn(|frame| {
            let phase = frame as f32 + 0.5;
            let mut row =
                angles.map(|a| crate::trigonometry::cos(phase * f32::from_bits(a)) * 0.25);
            row[0] *= 0.5;
            row
        });
        Tables {
            quantization,
            inverse_dct,
        }
    })
}

/// Original logarithm polynomial82E89598..5DC, evaluated only at integers2..9.
/// For this bounded positive domain, exponent extraction supplies exactly the
/// integer requested by the original floor(log2-estimate) instruction pair.
fn logarithm_for_frequency(integer: u32) -> f32 {
    let input = integer as f32;
    let exponent = ((input.to_bits() >> 23) & 255) as i32 - 127;
    let mantissa = f32::from_bits((input.to_bits() & 0x007f_ffff) | 0x3f80_0000);
    let a = mantissa - 1.0;
    let a2 = a * a;
    let a3 = a * a2;
    let a4 = a2 * a2;
    // Original vectors8230EA70 and8230EA80.
    let c = [
        0x3fb8_aa0e,
        0xbf38_9e52,
        0x3ef5_162d,
        0xbeb1_d204,
        0x3e77_adbd,
        0xbe0c_d4fb,
        0x3d55_41c6,
        0xbc18_8b0b,
    ]
    .map(f32::from_bits);
    let low = a.mul_add(c[1], c[0]);
    let high = a.mul_add(c[5], c[4]);
    let low = a2.mul_add(c[2], low);
    let high = a2.mul_add(c[6], high);
    let low = a3.mul_add(c[3], low);
    let high = a3.mul_add(c[7], high);
    let polynomial = a4.mul_add(high, low);
    a.mul_add(polynomial, exponent as f32) * f32::from_bits(0x3f31_7218) //82139A70
}

pub(super) fn decode(
    frame: usize,
    quantization_index: u8,
    scale: f32,
    coefficients: [f32; 8],
    channel_min: f32,
    channel_range: f32,
) -> f32 {
    let tables = tables();
    let squared = scale * scale;
    // Literal820612E0 is 2^-15. This equation was independently recovered
    // from original vmulfp/vmaddcfp instructions82E89930..4C.
    let bias = scale * f32::from_bits(0x3800_0000);
    let weights: [f32; 8] = std::array::from_fn(|i| {
        tables.quantization[quantization_index as usize][i].mul_add(squared, bias)
            * tables.inverse_dct[frame][i]
    });
    let first = crate::physics::native_arithmetic::dot4(
        coefficients[..4].try_into().unwrap(),
        weights[..4].try_into().unwrap(),
    );
    let last = crate::physics::native_arithmetic::dot4(
        coefficients[4..].try_into().unwrap(),
        weights[4..].try_into().unwrap(),
    );
    // BSS830CEAF0 initializer82F96D58 converts integer1 with scale1 -> .5.
    ((first + last) + 0.5).mul_add(channel_range, channel_min)
}
