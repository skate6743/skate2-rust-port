//! Complete normal-camera ShakeEffect update82E05800.
use super::manager_state::clamp;
use super::{ShakeEffect, ShakeSamples, ShakeSettings};
use crate::math::Basis3;

impl ShakeEffect {
    pub fn update(
        &mut self,
        dt: f32,
        speed: f32,
        distance: f32,
        basis: Basis3,
        enabled: bool,
        samples: &ShakeSamples,
        settings: ShakeSettings,
    ) -> Basis3 {
        self.weight = clamp(
            if enabled {
                dt.mul_add(5.0, self.weight)
            } else {
                -dt.mul_add(5.0, -self.weight)
            },
            0.0,
            1.0,
        );
        let amplitude = if settings.amplitude_top_speed != 0.0 {
            super::shake_curve::sample(
                settings.amplitude_curve,
                clamp(speed / settings.amplitude_top_speed, 0.0, 1.0),
            )
            .mul_add(
                settings.amplitude_maximum - settings.amplitude_minimum,
                settings.amplitude_minimum,
            )
        } else {
            settings.amplitude_minimum
        };
        let amplitude = (self.impulse + amplitude) * self.weight;
        let mut frequency = if settings.frequency_top_speed != 0.0 {
            super::shake_curve::sample(
                settings.frequency_curve,
                clamp(speed / settings.frequency_top_speed, 0.0, 1.0),
            )
            .mul_add(
                settings.frequency_maximum - settings.frequency_minimum,
                settings.frequency_minimum,
            )
        } else {
            settings.frequency_minimum
        };
        if self.impulse > 0.0 {
            frequency = settings.impulse_frequency;
        }
        self.time = (settings.data_frames_per_second * frequency).mul_add(dt, self.time);
        let inverse = super::vector_tracker::refined_reciprocal(1.0);
        let whole = (inverse * self.time).trunc();
        let fraction = (-whole).mul_add(1.0, self.time);
        let left = (self.time - fraction) as u32 as usize % samples.rotations.len();
        let right = (left + 1) % samples.rotations.len();
        let distance_weight = clamp(
            (f32::from_bits(0x4059999a) - distance) * f32::from_bits(0x3fd55552),
            0.0,
            1.0,
        );
        let inverse_fraction = 1.0 - fraction;
        let mut angles: [f32; 3] = core::array::from_fn(|i| {
            let interpolated = samples.rotations[left][i]
                .mul_add(inverse_fraction, samples.rotations[right][i] * fraction);
            ((interpolated * amplitude) * distance_weight) * settings.rotation_multiplier
        });
        // vrlimi mask2 selects Z (Dutch/roll), not Y.
        angles[2] *= settings.dutch_multiplier;
        let rotation = super::orientation_math::basis_from_angles(angles[0], angles[1], angles[2]);
        let local_translation: [f32; 3] = core::array::from_fn(|i| {
            let interpolated = samples.translations[left][i]
                .mul_add(inverse_fraction, samples.translations[right][i] * fraction);
            ((interpolated * settings.translation_multiplier) * amplitude) * distance_weight
        });
        self.translation = core::array::from_fn(|i| {
            if i == 3 {
                0.0
            } else {
                basis.columns[2][i].mul_add(
                    local_translation[2],
                    basis.columns[1][i].mul_add(
                        local_translation[1],
                        basis.columns[0][i].mul_add(local_translation[0], 0.0),
                    ),
                )
            }
        });
        let result = Basis3 {
            columns: rotation.columns.map(|column| {
                core::array::from_fn(|i| {
                    column[2].mul_add(
                        basis.columns[2][i],
                        column[1].mul_add(basis.columns[1][i], column[0] * basis.columns[0][i]),
                    )
                })
            }),
        };
        self.impulse = (1.0 - settings.impulse_decay) * self.impulse;
        if self.impulse < f32::from_bits(0x3ba3d70a) {
            self.impulse = 0.0;
        }
        result
    }
}
