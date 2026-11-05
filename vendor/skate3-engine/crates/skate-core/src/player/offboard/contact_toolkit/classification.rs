//! TU3 82D848D0: slope classification and retained three-sample acceptance.
use super::analyzer_math::tangent;
use super::{ContactPrefix, Vector, length};

/// Toolkit fields +30452..30468, retained across query refreshes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct History {
    accepted: u32,
    samples: [u32; 3],
    cursor: usize,
}

impl History {
    pub fn classify(&mut self, prefix: &mut ContactPrefix, direction: Vector) {
        let horizontal = length([direction[0], 0., direction[2], 0.]);
        // 82D84994 uses scalar division, not the vector reciprocal helper.
        let inverse = 1.
            / if 0.001 - horizontal >= 0. {
                0.001
            } else {
                horizontal
            };
        let slope = inverse * direction[1];
        prefix.scalar_160 = slope;
        // Stock globals 822F8FCC/822F8FD0; arguments are radians.
        let low = tangent(f32::from_bits(0x3dd67750));
        let high = tangent(f32::from_bits(0x3f0efa35));
        let magnitude = slope.abs();
        let proposed = if magnitude <= high {
            if magnitude <= low {
                0
            } else if slope > 0. {
                1
            } else {
                3
            }
        } else if slope > 0. {
            2
        } else {
            4
        };
        let proposed = if proposed != 0 && prefix.flags_176 & 0x40 == 0 {
            proposed + 4
        } else {
            proposed
        };
        prefix.kind_164 = proposed;
        if proposed == 0 {
            *self = Self::default();
        } else if self.samples.iter().all(|&sample| sample == proposed) {
            self.accepted = proposed;
        } else {
            self.cursor = (self.cursor + 1) % 3;
            self.samples[self.cursor] = proposed;
            prefix.kind_164 = self.accepted;
        }
    }
}
