//! Legacy continuous puddle-field/CDF reference retained for component fixtures.
//! The runtime query-only geometry path skips these curves. Authored GPU wet
//! contours now live in gpu_renderer/rain.wgsl; receivers still gate admission.
use crate::weather_runtime::LOOK_PUDDLE_SIZE;

pub(crate) const BINS: usize = 256;
pub(crate) const NO_CURVE: u32 = u32::MAX;

pub(crate) struct Curve {
    pub(crate) domain: [f32; 2],
    pub(crate) limits: [f32; BINS + 1],
}

impl Curve {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped field bins and upward-rounded area fractions"
    )]
    pub(crate) fn build(patches: impl Iterator<Item = ([[f32; 3]; 3], f64)>) -> Self {
        Self::from_samples(patches.map(|(points,area)|(lower_bound(points),area)).collect())
    }

    /// Large retail-city patches can span several noise cells. Their conservative
    /// minimum collapses the CDF and rejects every visible puddle. Area-weighted
    /// centre sampling estimates coverage there; fine source meshes keep the bound.
    pub(crate) fn sampled(patches:impl Iterator<Item=([[f32;3];3],f64)>)->Self {
        Self::from_samples(patches.map(|(points,area)|(centre_value(points),area)).collect())
    }
    fn from_samples(samples:Vec<(f32,f64)>)->Self {
        let low = samples.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let high = samples
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let domain = [low, 1.0 / (high - low).max(0.000_5)];
        let mut histogram = [0.0_f64; BINS];
        let mut total = 0.0;
        for (minimum, area) in samples {
            let bin = (((minimum - domain[0]) * domain[1] * 256.0).floor() as usize).min(BINS - 1);
            histogram[bin] += area;
            total += area;
        }
        let mut limits = [1.0; BINS + 1];
        let mut cumulative = 0.0;
        for (bin, area) in histogram.into_iter().enumerate() {
            cumulative += area;
            if total > 0.0 {
                let value = (cumulative / total) as f32;
                limits[bin] = if value > 0.0 {
                    f32::from_bits(value.to_bits() + 1)
                } else {
                    0.0
                };
            }
        }
        limits[BINS] = limits[BINS - 1];
        Self { domain, limits }
    }

    /// Upper bound on the area whose continuous field is at or below this value.
    #[cfg(test)]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "bounded field lookup, exactly mirroring the fragment shader"
    )]
    pub(crate) fn rank(&self, value: f32) -> f32 {
        let coordinate = ((value - self.domain[0]) * self.domain[1]).clamp(0.0, 0.999_999) * 256.0;
        let bin = coordinate as usize;
        let fraction = coordinate - bin as f32;
        self.limits[bin] + (self.limits[bin + 1] - self.limits[bin]) * fraction
    }
}

/// Same integer hash and smooth interpolation as the authored WGSL.
#[expect(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    reason = "integer lattice coordinates and bilinear interpolation"
)]
fn value_noise(x: f32, z: f32) -> f32 {
    let a = x.floor() as i32;
    let b = z.floor() as i32;
    let hash = |x: i32, z: i32| {
        let mut v = u32::from_ne_bytes(x.to_ne_bytes())
            ^ u32::from_ne_bytes(z.to_ne_bytes()).wrapping_mul(0x9e37_79b9);
        v = (v ^ (v >> 16)).wrapping_mul(0x7feb_352d);
        v = (v ^ (v >> 15)).wrapping_mul(0x846c_a68b);
        f32::from(((v ^ (v >> 16)) >> 16) as u16) / 65_535.0
    };
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let u = smooth(x - x.floor());
    let v = smooth(z - z.floor());
    mix(
        mix(hash(a, b), hash(a.wrapping_add(1), b), u),
        mix(
            hash(a, b.wrapping_add(1)),
            hash(a.wrapping_add(1), b.wrapping_add(1)),
            u,
        ),
        v,
    )
}

pub(crate) fn field(point: [f32; 3]) -> f32 {
    let x = point[0] / LOOK_PUDDLE_SIZE;
    let z = point[2] / LOOK_PUDDLE_SIZE;
    value_noise(x, z) * 0.65
        + value_noise(x * 2.13, z * 2.13) * 0.25
        + value_noise(x * 4.31, z * 4.31) * 0.10
}

/// Exact rectangle minima of each bilinear smooth-noise octave are attained at
/// rectangle corners or integer grid crossings. Sum of minima is conservative.
pub(crate) fn lower_bound(points: [[f32; 3]; 3]) -> f32 {
    let x0 = points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let x1 = points
        .iter()
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let z0 = points.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
    let z1 = points
        .iter()
        .map(|p| p[2])
        .fold(f32::NEG_INFINITY, f32::max);
    // Guard GPU position interpolation / field arithmetic at distant native coordinates.
    let slack = x0.abs().max(x1.abs()).max(z0.abs()).max(z1.abs()) * 0.000_001 + 0.000_1;
    let mut minimum = 0.0;
    for (frequency, weight) in [(1.0, 0.65), (2.13, 0.25), (4.31, 0.10)] {
        let x0 = (x0 - slack) / LOOK_PUDDLE_SIZE * frequency;
        let x1 = (x1 + slack) / LOOK_PUDDLE_SIZE * frequency;
        let z0 = (z0 - slack) / LOOK_PUDDLE_SIZE * frequency;
        let z1 = (z1 + slack) / LOOK_PUDDLE_SIZE * frequency;
        minimum += rectangle_minimum(x0, x1, z0, z1) * weight;
    }
    (minimum - 0.000_1).max(0.0)
}

fn rectangle_minimum(x0: f32, x1: f32, z0: f32, z1: f32) -> f32 {
    // A huge native triangle may span many cells. Zero is still a rigorous
    // lower bound without an unbounded map-loading loop.
    if x1 - x0 > 8.0 || z1 - z0 > 8.0 {
        return 0.0;
    }
    let axis = |start: f32, end: f32| {
        let mut coordinates = vec![start, end];
        let mut value = start.ceil();
        while value < end {
            coordinates.push(value);
            value += 1.0;
        }
        coordinates
    };
    let xs = axis(x0, x1);
    let zs = axis(z0, z1);
    let mut minimum = 1.0_f32;
    for &x in &xs {
        for &z in &zs {
            minimum = minimum.min(value_noise(x, z));
        }
    }
    minimum
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_bounds_contain_the_continuous_field_at_origin_and_city_coordinates() {
        for offset in [0.0, 10_000.0, -10_000.0] {
            for i in 0..12_u16 {
                let x = offset + f32::from(i) * 0.37;
                let z = f32::from(i) * -0.29;
                let points = [
                    [x, 0.0, z],
                    [x + 0.8, 0.0, z + 0.3],
                    [x + 0.2, 0.0, z + 0.9],
                ];
                let minimum = lower_bound(points);
                for a in 0..11_u16 {
                    for b in 0..11_u16 - a {
                        let first = f32::from(a) / 10.0;
                        let second = f32::from(b) / 10.0;
                        let point = std::array::from_fn(|j| {
                            points[0][j] * (1.0 - first - second)
                                + points[1][j] * first
                                + points[2][j] * second
                        });
                        assert!(
                            field(point) >= minimum,
                            "field escaped conservative calibration"
                        );
                    }
                }
            }
        }
    }
}

pub(crate) fn centre_value(points:[[f32;3];3])->f32 {
    field(std::array::from_fn(|axis|(points[0][axis]+points[1][axis]+points[2][axis])/3.))
}
