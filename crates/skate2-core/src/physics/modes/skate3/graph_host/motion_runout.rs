//! TU3 AddRunoutAttribs: factory82BC9BA8, vtable82320BDC,
//! Begin82BBA4A8, Update82BBA770, End82B61BB8.
use skate3_core::{math::Vector3, physics::board_motion_output::length,
    player::wipeout_state::orientation::projected_angle};

#[derive(Clone, Copy, Debug)]
pub struct Physical {
    /// Skeleton0: effective animation root Z (Fill82BE1AE8 / GetEffectiveRoot).
    pub forward: [f32; 4],
    /// SystemReckoning16 and96, respectively.
    pub velocity: [f32; 4],
    pub up: [f32; 4],
    /// OffBoard128, selected only when OffBoard331 is set.
    pub trajectory_velocity: Option<[f32; 4]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    pub angle_degrees: f32,
    pub speed: f32,
}

pub fn capture(physical: Physical, mirrored: bool) -> Parameters {
    let velocity = physical.trajectory_velocity.unwrap_or(physical.velocity);
    let angle = projected_angle(physical.forward, velocity, physical.up);
    let angle = if mirrored { -angle } else { angle };
    // Native wrap has a strict >0.5 boundary, preserving positive 180 degrees.
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    let radians = (fraction - if fraction > 0.5 { 1.0 } else { 0.0 })
        * f32::from_bits(0x40c9_0fdb);
    Parameters {
        angle_degrees: radians * f32::from_bits(0x4265_2ee1),
        speed: length(Vector3::new(velocity[0], velocity[1], velocity[2])),
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn runout_direction_speed_stance_and_trajectory_selection() {
        let mut p = Physical { forward: [0.,0.,1.,0.], velocity: [3.,4.,0.,0.],
            up: [0.,1.,0.,0.], trajectory_velocity: None };
        let regular = capture(p, false);
        let mirrored = capture(p, true);
        assert!((regular.angle_degrees - 90.).abs() < 0.001);
        assert!((mirrored.angle_degrees + 90.).abs() < 0.001);
        assert!((regular.speed - 5.).abs() < 0.00001);
        p.trajectory_velocity = Some([0.,0.,-2.,0.]);
        assert!((capture(p, false).angle_degrees - 180.).abs() < 0.001);
        assert!((capture(p, true).angle_degrees.abs() - 180.).abs() < 0.001);
        assert!((capture(p, false).speed - 2.).abs() < 0.00001);
        p.trajectory_velocity = Some([0.;4]);
        assert_eq!(capture(p, false), Parameters { angle_degrees: 0., speed: 0. });
    }
}
