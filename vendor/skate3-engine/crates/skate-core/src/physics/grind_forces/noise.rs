//! Original orientation noise82D40580 / S2 AddNoiseToOrientation82D8FE78.
//! No replacement RNG: caller supplies the three consecutive board-stream u32
//! draws, even for zero amount/speed. S3 hardcodes S2's full-speed/scalar tunables.
use super::V;

pub fn angles(speed: f32, amount: f32, samples: [u32; 3]) -> [f32; 3] {
    let speed_factor = (speed * 0.5).min(1.0);
    let amplitude = speed_factor * amount * 0.06;
    let width = amplitude * 2.0;
    samples.map(|sample| {
        let unit = (sample % 100_000) as f32 * 0.00001;
        unit.mul_add(width, -amplitude)
    })
}

pub fn apply(frame: [V; 4], speed: f32, amount: f32, samples: [u32; 3]) -> [V; 4] {
    let [x, y, z] = angles(speed, amount, samples);
    let (sx, cx) = crate::trigonometry::sin_cos(x);
    let (sy, cy) = crate::trigonometry::sin_cos(y);
    let (sz, cz) = crate::trigonometry::sin_cos(z);
    // Raw standard-VMX words82D407F0=10CC526E and82D407F4=118C4AAE
    // disambiguate IDA's misleading printed multiply/add operand order.
    let sx_cz = sx * cz;
    let cx_sz = cx * sz;
    let cx_cz = cx * cz;
    let sx_sz = sx * sz;
    let columns = [
        [cy * cz, cy * sz, -sy],
        [sy * sx_cz - cx_sz, sy.mul_add(sx_sz, cx_cz), cy * sx],
        [sy.mul_add(cx_cz, sx_sz), sy * cx_sz - sx_cz, cy * cx],
    ];
    let mut result = frame;
    for column in 0..3 {
        let v = columns[column];
        result[column] = core::array::from_fn(|lane| {
            v[2].mul_add(
                frame[2][lane],
                v[1].mul_add(frame[1][lane], v[0] * frame[0][lane]),
            )
        });
    }
    //82D407EC/40810/40828 multiply the three basis vectors by zero
    //and add the original translation. No positional jitter is introduced.
    result
}
