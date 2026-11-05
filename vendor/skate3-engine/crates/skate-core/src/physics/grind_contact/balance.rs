//! S3 TU3 GrindBalance: 82D86318, 82D86808, 82D86C88; constructor82D8A318.
//! S2 counterparts82DD2168/82DD64D8/82DD26C0 corroborate field identities.
//! Keep these three stages in that order, after geometry correction and before
//! engagement. Independent PC arithmetic is not bit-exact Xenon emulation.
use super::{V, admission::EntryKind, arithmetic, cross, dot3, scale, sub};
use crate::air::trajectory::grind_surface::{GeometryType, GrindSurface};
use crate::{point_graph::PointGraph, trigonometry};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceState {
    pub elapsed: f32,
    pub exit_angle_degrees: f32,
    pub entry_delay: f32,
    pub frames_away: i32,
    /// Unrotated normal retained by82D86318, not the exit-lean output normal.
    pub previous_normal: V,
    pub exit_direction: V,
}

impl Default for BalanceState {
    fn default() -> Self {
        //82D8A4BC..82D8A4EC; original vectors82139A20 and82139A10.
        Self {
            elapsed: 0.0,
            exit_angle_degrees: 0.0,
            entry_delay: 2.0,
            frames_away: 21,
            previous_normal: [0.0, 1.0, 0.0, 0.0],
            exit_direction: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// A valid, geometry-corrected investigation, never a synthesized surface.
#[derive(Clone, Copy, Debug)]
pub struct BalanceContact<'a> {
    pub surface: &'a GrindSurface,
    /// Investigation+0; distinct from travel-directed investigation+32.
    pub primitive_direction: V,
    pub directed_grind_direction: V,
    pub kind: u32,
    pub entry_kind: EntryKind,
}

#[derive(Clone, Copy, Debug)]
pub struct TargetUpInput {
    pub category: u32,
    /// Processed+2504, NOT current state+2508.
    pub previous_state: u32,
    pub board_up: V,
}

#[derive(Clone, Copy, Debug)]
pub struct ExitLeanInput {
    pub category: u32,
    /// Processed+2508, unlike TargetUpInput::previous_state.
    pub current_state: u32,
    /// Processed+2536; value2 freezes the elapsed-time increment.
    pub grind_substate: u32,
    pub timestep: f32,
}

/// Investigation+48/+64, deliberately without a fabricated default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceVectors {
    pub grind_normal: V,
    pub target_up: V,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceExitProbe {
    pub start: V,
    pub end: V,
}

#[derive(Clone, Copy, Debug)]
pub struct ForceExitHit {
    pub normal: V,
}

impl BalanceState {
    ///82D86318. None is the native invalid-candidate no-write branch.
    pub fn update_target_up(
        &mut self,
        input: TargetUpInput,
        contact: Option<&BalanceContact<'_>>,
        vectors: &mut BalanceVectors,
    ) {
        let Some(contact) = contact else { return };
        let surface = contact.surface;
        let normal = if input.category != 400 {
            input.board_up
        } else if matches!(input.previous_state, 402 | 404) {
            blend(
                self.previous_normal,
                0.85,
                surface.upmost_normal,
                f32::from_bits(0x3e19_9998),
            )
        } else {
            blend(
                self.previous_normal,
                0.925,
                surface.tilted_upmost_normal,
                f32::from_bits(0x3d99_9998),
            )
        };
        let direction = contact.directed_grind_direction;
        let projected = cross(cross(direction, normal), direction);
        let length = magnitude(projected);
        vectors.grind_normal = if length <= 0.001 {
            surface.upmost_normal
        } else {
            scale(projected, arithmetic::reciprocal(length))
        };
        self.previous_normal = vectors.grind_normal;

        let target = blend(
            input.board_up,
            0.8,
            surface.upmost_normal,
            f32::from_bits(0x3e4c_cccc),
        );
        // Unlike the projected-normal and rotation-axis branches, the original
        // has NO zero-length fallback here. Do not silently invent one.
        vectors.target_up = scale(target, arithmetic::reciprocal(magnitude(target)));
        let alignment = dot3(vectors.grind_normal, vectors.target_up)
            .max(-1.0)
            .min(1.0);
        let max_angle = f32::from_bits(0x3e7a_35dd);
        if trigonometry::acos(alignment) > max_angle {
            let axis = cross(vectors.grind_normal, vectors.target_up);
            let length = magnitude(axis);
            vectors.target_up = if length <= 0.001 {
                vectors.grind_normal
            } else {
                rotate(
                    scale(axis, arithmetic::reciprocal(length)),
                    vectors.grind_normal,
                    max_angle,
                )
            };
        }
    }

    ///82D86808. Returns investigation+396 even without a valid candidate.
    /// Graph is physics_grinds.ExitLeanAngleVsTime, eight X at+16/Y at+48.
    pub fn update_exit_lean(
        &mut self,
        input: ExitLeanInput,
        contact: Option<&BalanceContact<'_>>,
        exit_lean_angle_vs_time: &PointGraph<8>,
        vectors: &mut BalanceVectors,
    ) -> f32 {
        let active = input.category == 400 || input.current_state == 701 || contact.is_some();
        self.frames_away = if active {
            0
        } else {
            self.frames_away.wrapping_add(1)
        };
        let droppable = contact.is_some_and(|contact| {
            if contact.kind == 2 {
                active && contact.surface.kind != GeometryType::ThinRail
            } else {
                contact.surface.kind == GeometryType::Ledge
            }
        });
        let mut started = false;
        //82D86880..94's carry/sign sequence implements a signed >20 test.
        if self.frames_away > 20 {
            self.exit_angle_degrees = 0.0;
            self.elapsed = 0.0;
            self.entry_delay = 2.0;
        } else if self.elapsed > 0.0 || (active && droppable && contact.is_some()) {
            if self.elapsed == 0.0 {
                // Only a real contact can start the timer in this branch.
                if let Some(contact) = contact {
                    self.entry_delay = match contact.entry_kind {
                        EntryKind::RideFromBelow => 0.8,
                        EntryKind::RideIntoCoping => 0.1,
                        _ => 2.0,
                    };
                }
                started = true;
            }
            if input.grind_substate != 2 {
                self.elapsed = input.timestep + self.elapsed;
            }
        }
        if self.elapsed <= 0.0 {
            return self.exit_angle_degrees;
        }
        let time = self.elapsed - self.entry_delay;
        self.exit_angle_degrees = if time <= 0.0 {
            0.0
        } else {
            exit_lean_angle_vs_time.evaluate(time)
        };
        if let Some(contact) = contact {
            let surface = contact.surface;
            self.exit_direction = if surface.kind == GeometryType::ThinRail {
                retain_side(
                    self.exit_direction,
                    cross(surface.upmost_normal, contact.primitive_direction),
                )
            } else if started {
                surface.high_side
            } else {
                retain_side(self.exit_direction, surface.high_side)
            };
            if self.exit_angle_degrees > 0.0 {
                let direction = contact.directed_grind_direction;
                let axis = if cross(direction, self.exit_direction)[1] > 0.0 {
                    direction
                } else {
                    negate(direction)
                };
                let radians = self.exit_angle_degrees * f32::from_bits(0x3c8e_fa35);
                // Both outputs rotate; previous_normal deliberately does not.
                vectors.target_up = rotate(axis, vectors.target_up, radians);
                vectors.grind_normal = rotate(axis, vectors.grind_normal, radians);
            }
        }
        self.exit_angle_degrees
    }

    ///82D86C88. The callback must execute the real thin nearest world-line
    /// query82E0AAD0, retaining its filtering and endpoint/edge tolerances.
    /// No query is issued outside (15,28]. Errors are not collision misses.
    /// Updates only investigation+412 bit31, and leaves flags intact on error.
    pub fn update_force_exit<E>(
        &self,
        primitive_location: V,
        investigation_flags: &mut u32,
        mut world_line: impl FnMut(ForceExitProbe) -> Result<Option<ForceExitHit>, E>,
    ) -> Result<(), E> {
        let force_exit = if self.exit_angle_degrees > 28.0 {
            true
        } else if self.exit_angle_degrees > 15.0 {
            let start = sub(primitive_location, scale(self.exit_direction, 0.1));
            let end = super::add(start, [0.0, -5.0, 0.0, 0.0]);
            match world_line(ForceExitProbe { start, end })? {
                Some(hit) => dot3(self.exit_direction, hit.normal) > -0.1,
                None => true,
            }
        } else {
            false
        };
        *investigation_flags = (*investigation_flags & 0x7fff_ffff) | ((force_exit as u32) << 31);
        Ok(())
    }
}

fn magnitude(value: V) -> f32 {
    arithmetic::square_root(dot3(value, value))
}

fn blend(a: V, a_weight: f32, b: V, b_weight: f32) -> V {
    core::array::from_fn(|i| a[i].mul_add(a_weight, b[i] * b_weight))
}

fn negate(value: V) -> V {
    value.map(|lane| -lane)
}

fn retain_side(previous: V, side: V) -> V {
    if dot3(previous, side) > 0.0 {
        side
    } else {
        negate(side)
    }
}

///82C1E220 and82D86BC0..C70: quaternion half-angle rotation, not a
/// Rodrigues replacement with a different operation order.
fn rotate(axis: V, value: V, angle: f32) -> V {
    let (sin, cos) = trigonometry::sin_cos(angle * 0.5);
    let q = scale(axis, sin);
    let first_cross = cross(q, value);
    let intermediate = core::array::from_fn(|i| value[i].mul_add(cos, first_cross[i]));
    let second_cross = cross(q, intermediate);
    core::array::from_fn(|i| second_cross[i].mul_add(2.0, value[i]))
}

#[cfg(test)]
#[path = "balance_tests.rs"]
mod tests;
