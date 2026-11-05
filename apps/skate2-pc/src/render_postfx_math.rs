//! Scalar producers for PostFX, checked against the debug executable.
//! No display-brightness preference is folded into these original values.

/// SetBloomCoeefs828233b0. The surprising 10 (not .1) on the narrow
/// threshold branch is the actual lfs at82823418.
pub(crate) fn bloom_coefficients(threshold: f32) -> [f32; 4] {
    let mut range = 1.0 - threshold;
    if range > 1.0 {
        range = 1.0;
    } else if range < 0.1 {
        range = 10.0;
    }
    [
        if threshold < 0.0 { -threshold } else { 0.0 },
        0.0,
        1.0 / range,
        threshold,
    ]
}

/// ApplyFX_NEW82823f08: contrast/offset, saturation, then RGB modulation.
pub(crate) fn colour_matrix(
    contrast: f32,
    midpoint: f32,
    saturation: f32,
    add: [f32; 3],
    multiply: [f32; 3],
) -> [[f32; 4]; 3] {
    let offset = add.map(|v| (midpoint - contrast * midpoint) + v);
    let sat = saturation_matrix(saturation);
    std::array::from_fn(|row| {
        let s = sat[row];
        [
            s[0] * contrast * multiply[row],
            s[1] * contrast * multiply[row],
            s[2] * contrast * multiply[row],
            s[2].mul_add(offset[2], s[1].mul_add(offset[1], s[0] * offset[0])) * multiply[row],
        ]
    })
}

pub(crate) fn saturation_matrix(s: f32) -> [[f32; 4]; 3] {
    let r = (1.0 - s) * 0.3;
    let g = (1.0 - s) * 0.6;
    let b = (1.0 - s) * 0.1;
    [
        [s.mul_add(0.7, 0.3), g, b, 0.],
        [r, s.mul_add(0.4, 0.6), b, 0.],
        [r, g, s.mul_add(0.9, 0.1), 0.],
    ]
}

/// GetSampleOffsets_Bloom82827b00, called with deviation3/multiplier1.
/// These are nine bilinear fetches covering seventeen Gaussian taps. The
/// source deliberately does not renormalize the truncated Gaussian.
pub(crate) fn blur_samples(size: u32) -> [[f32; 4]; 9] {
    let inv = 1.0 / size as f32;
    let gaussian = |x: f32| {
        ((-(x * x / 18.0)) as f64).exp() as f32 * (1.0 / (std::f32::consts::TAU.sqrt() * 3.0))
    };
    let mut result = [[0.; 4]; 9];
    result[0] = [0., 0., gaussian(0.), 1.];
    for i in 1..5 {
        let a = gaussian((i * 2 - 1) as f32);
        let b = gaussian((i * 2) as f32);
        let weight = a + b;
        let offset = (i as f32 * 2.0 - a / weight) * inv;
        result[i] = [offset, offset, weight, 1.];
        result[i + 4] = [-offset, -offset, weight, 0.];
    }
    result
}

/// Tonemap82828f68: alpha from the delayed32x16 bloom surface, with the
/// original spatial weights. This is intentionally not an arithmetic mean.
pub(crate) fn luminance(rgba: &[u8], stride: usize) -> Option<f32> {
    if rgba.len() < stride * 15 + 128 {
        return None;
    }
    let mut sum = 0.0f32;
    let mut y = 0.0f32;
    for row in 0..16 {
        let mut x = 0.0f32;
        for column in 0..32 {
            let weight = x * y;
            let value = rgba[row * stride + column * 4 + 3] as f32 * f32::from_bits(0x3b808081);
            sum += value * (weight * weight);
            x += 1.0 / 16.0;
        }
        y += 1.0 / 8.0;
    }
    Some(sum / 512.0)
}

/// target/min/max/damping are rendering layout104/108/112/116.
pub(crate) fn adapt(exposure: f32, luminance: f32, parameters: [f32; 4]) -> f32 {
    let [target, min, max, damping] = parameters;
    (((target - luminance) * damping + 1.0) * exposure).clamp(min, max)
}

/// XGRAPHICS::TileVolume82c30c48, specialized to the original32^3 ARGB
/// cube. Authored RGB files retain Xbox volume tiling, with alpha stripped.
pub(crate) fn cube_texel_index(x: usize, y: usize, z: usize) -> usize {
    let micro = ((x & 7) + ((4 * y) & 24)) * 4;
    let macro_tile = (y >> 4) + 2 * (z >> 2);
    let bank = ((y >> 3) + (z >> 2)) & 1;
    let offset = 2 * (2 * ((x >> 5) + macro_tile) * 256 + (micro & !15))
        + (micro & 15)
        + (z & 3) * 256
        + ((16 * y) & 16);
    let swizzle = ((2 * ((x >> 3) + 2 * bank)) & 6) + bank;
    (256 * ((8 * swizzle) & 8)
        + 256 * ((offset >> 6) & 7)
        + 32 * (swizzle & !1)
        + 8 * (offset & !511)
        + (offset & 63))
        / 4
}

/// The identity colour cube in the decoded volume's channel order: what a linked Skate 3
/// district's `colour_cube/default`, which names no file, grades with (no grade).
#[allow(dead_code, reason = "a district keeps the Skate 2 world's PostFX (user request)")]
pub(crate) fn neutral_cube() -> Vec<u8> {
    let mut out = Vec::with_capacity(32 * 32 * 32 * 4);
    for z in 0..32_u32 {
        for y in 0..32_u32 {
            for x in 0..32_u32 {
                let level = |value: u32| u8::try_from(value * 255 / 31).unwrap_or(u8::MAX);
                out.extend_from_slice(&[level(31 - y), level(x), level(z), 255]);
            }
        }
    }
    out
}

pub(crate) fn decode_cube(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() != 32 * 32 * 32 * 3 {
        return None;
    }
    let mut out = Vec::with_capacity(32 * 32 * 32 * 4);
    for z in 0..32 {
        for y in 0..32 {
            for x in 0..32 {
                let i = cube_texel_index(x, y, z) * 3;
                out.extend_from_slice(&[bytes[i], bytes[i + 1], bytes[i + 2], 255]);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires original extracted colour cube; set SKATE2_NEUTRAL_CUBE"]
    fn original_neutral_cube_decodes_every_authored_texel() {
        let path = std::env::var_os("SKATE2_NEUTRAL_CUBE").expect("original neutral cube path");
        let rgba = decode_cube(&std::fs::read(path).unwrap()).unwrap();
        for z in 0..32 {
            for y in 0..32 {
                for x in 0..32 {
                    let i = ((z * 32 + y) * 32 + x) * 4;
                    assert_eq!(
                        &rgba[i..i + 4],
                        &[
                            ((31 - y) * 255 / 31) as u8,
                            (x * 255 / 31) as u8,
                            (z * 255 / 31) as u8,
                            255
                        ]
                    );
                }
            }
        }
    }
    #[test]
    fn volume_swizzle_is_bijective() {
        let mut visited = vec![false; 32 * 32 * 32];
        for z in 0..32 {
            for y in 0..32 {
                for x in 0..32 {
                    let index = cube_texel_index(x, y, z);
                    assert!(!visited[index]);
                    visited[index] = true;
                }
            }
        }
        assert!(visited.into_iter().all(|v| v));
        assert_eq!(cube_texel_index(4, 0, 0), 8);
        assert_eq!(cube_texel_index(0, 1, 0), 4);
        assert_eq!(cube_texel_index(0, 0, 1), 256);
    }
    #[test]
    fn native_threshold_branches_and_matrix() {
        assert_eq!(bloom_coefficients(0.5), [0., 0., 2., 0.5]);
        assert_eq!(bloom_coefficients(1.), [0., 0., 0.1, 1.]);
        assert_eq!(bloom_coefficients(-0.5), [0.5, 0., 1., -0.5]);
        assert_eq!(
            colour_matrix(1., 0.5, 1., [0.; 3], [1.; 3]),
            [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]]
        );
        assert_eq!(saturation_matrix(0.), [[0.3, 0.6, 0.1, 0.]; 3]);
    }
    #[test]
    fn feedback_reads_alpha_and_preserves_native_weighting() {
        let mut bytes = vec![0u8; 256 * 16];
        for y in 0..16 {
            for x in 0..32 {
                bytes[y * 256 + x * 4 + 3] = 255;
            }
        }
        // sum(x^2)*sum(y^2)/512 for x=0..31/16,y=0..15/8.
        let expected = (10416.0 / 256.0) * (1240.0 / 64.0) / 512.0;
        assert!((luminance(&bytes, 256).unwrap() - expected).abs() < 1e-6);
        assert_eq!(adapt(1., 0., [0.25, 0.1, 2., 0.15]), 1.0375);
        assert_eq!(adapt(2., 0., [0.25, 0.1, 2., 0.15]), 2.);
    }
}
