//! TU3 Reckoning::UpdateBodyFlip 82D8EC70, with explicit attribute-read inputs.
//! Full scalar gates, spin matrix and conditional combined-matrix publication.
use crate::trigonometry::sin_cos;

pub type Matrix = [[f32; 4]; 4];

#[derive(Clone, Debug, PartialEq)]
pub struct BodyFlipState {
    /// Reckoning +1584, +1588, +1592.
    pub angle: f32,
    pub speed: f32,
    pub requested_speed: f32,
    /// Reckoning +1072 and +1008. These are distinct native output buffers.
    pub spin_transform: Matrix,
    pub combined_transform: Matrix,
}

/// Actual Attrib::GetAttributePointer results (82B72420), not guessed settings.
/// Missing collection/attribute resolves to the supplied live 830D0850 value.
pub struct BodyFlipSettings {
    /// Hash 26E7322CCDC3FE23.
    pub smoothing: Option<f32>,
    /// Hash 3D95C06DC6E3A002.
    pub maximum_speed: Option<f32>,
    /// Hash 89763E47712871C3.
    pub spin_scale: Option<f32>,
    pub missing_attribute_value: f32,
}

pub struct BodyFlipInput {
    pub requested_speed: f32,
    /// Reckoning +1568, +1152 and +1232; caller-provided axes, without repair.
    pub spin_angle: f32,
    pub normal: [f32; 4],
    pub flip_axis: [f32; 4],
    /// Processed +2604; only the non-perfect branch uses this dt.
    pub timestep: f32,
    /// Processed +2548 -> layout +4 -> byte +28.
    pub perfect_body_flips: bool,
}

pub fn update(state: &mut BodyFlipState, settings: &BodyFlipSettings, input: &BodyFlipInput) {
    let fallback = settings.missing_attribute_value;
    let smoothing = settings.smoothing.unwrap_or(fallback);
    let maximum = settings.maximum_speed.unwrap_or(fallback);
    let spin_scale = settings.spin_scale.unwrap_or(fallback);
    state.requested_speed = input.requested_speed;
    let mut speed = (input.requested_speed - state.speed).mul_add(smoothing, state.speed);
    if input.perfect_body_flips {
        let full_turn = f32::from_bits(0x40C9_0FDB);
        let remaining_rate = (full_turn - state.angle.abs()) * 12.0;
        let sign = if speed > 0.0 { 1.0 } else { -1.0 };
        if !(speed.abs() < remaining_rate) {
            speed = sign * remaining_rate;
        }
        state.angle = clamp(
            speed.mul_add(f32::from_bits(0x3C88_8889), state.angle),
            -full_turn,
            full_turn,
        );
    } else {
        speed = clamp(speed, -maximum, maximum);
        state.angle = input.timestep.mul_add(speed, state.angle);
    }
    state.speed = speed;
    state.spin_transform = rotation(input.normal, input.spin_angle * spin_scale);
    // Native zero-angle branch leaves the +1008 matrix untouched.
    if state.angle != 0.0 {
        let flip = rotation(input.flip_axis, state.angle);
        let spin = state.spin_transform;
        for col in 0..4 {
            for lane in 0..4 {
                let first = if col == 3 {
                    spin[col][0].mul_add(flip[0][lane], flip[3][lane])
                } else {
                    spin[col][0] * flip[0][lane]
                };
                let second = spin[col][1].mul_add(flip[1][lane], first);
                state.combined_transform[col][lane] = spin[col][2].mul_add(flip[2][lane], second);
            }
        }
    }
}

fn clamp(value: f32, lower: f32, upper: f32) -> f32 {
    let value = if lower - value >= 0.0 { lower } else { value };
    if upper - value >= 0.0 { value } else { upper }
}

fn rotation(axis: [f32; 4], angle: f32) -> Matrix {
    let [x, y, z, _] = axis;
    let (s, c) = sin_cos(angle);
    let t = 1.0 - c;
    let (tx, ty, tz) = (t * x, t * y, t * z);
    let (sx, sy, sz) = (s * x, s * y, s * z);
    let xx = tx.mul_add(x, c);
    let yx = ty * x - sz;
    let zx = tz.mul_add(x, sy);
    // The native 822FB890 permute repeats column.x into lane w. It does not
    // create a renderer-style affine matrix with zero w basis lanes.
    [
        [xx, tx.mul_add(y, sz), tx * z - sy, xx],
        [yx, ty.mul_add(y, c), ty.mul_add(z, sx), yx],
        [zx, tz * y - sx, tz.mul_add(z, c), zx],
        [0.0; 4],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::skeleton_animation_record::IDENTITY;

    #[test]
    fn perfect_front_and_back_flips_finish_at_one_revolution_with_spin() {
        for direction in [-1., 1.] {
            let mut state = BodyFlipState {
                angle: 0.,
                speed: 0.,
                requested_speed: 0.,
                spin_transform: IDENTITY,
                combined_transform: IDENTITY,
            };
            let settings = BodyFlipSettings {
                smoothing: Some(1.),
                maximum_speed: Some(20.),
                spin_scale: Some(1.),
                missing_attribute_value: 0.,
            };
            for tick in 1..=360 {
                update(
                    &mut state,
                    &settings,
                    &BodyFlipInput {
                        requested_speed: direction * 4.,
                        spin_angle: std::f32::consts::TAU * (tick as f32 / 180.).min(1.),
                        normal: [0., 1., 0., 0.],
                        flip_axis: [1., 0., 0., 0.],
                        timestep: 1. / 60.,
                        perfect_body_flips: true,
                    },
                );
                assert!(state.angle.abs() <= std::f32::consts::TAU);
                assert!(state.angle * direction > 0.);
                assert!(
                    state
                        .combined_transform
                        .iter()
                        .flatten()
                        .all(|x| x.is_finite())
                );
            }
            assert!((state.angle - direction * std::f32::consts::TAU).abs() < 0.001);
            // After both revolutions, all three spatial basis axes are upright again.
            for i in 0..3 {
                for j in 0..3 {
                    assert!((state.combined_transform[i][j] - IDENTITY[i][j]).abs() < 0.001);
                }
            }
        }
    }
}
