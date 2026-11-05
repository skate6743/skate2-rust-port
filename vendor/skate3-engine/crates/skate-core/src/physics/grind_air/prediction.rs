use super::*;
pub(super) type Samples = [[V; 5]; 12];
///82D71430: frame zero is the actual processed deck; advance velocity before position.
pub(super) fn project(
    input: Input,
    s: &Settings,
    rail: V,
    normal: V,
    headings: &mut [f32; 12],
) -> Samples {
    let mut samples = [[ZERO; 5]; 12];
    let mut board = input.board;
    let mut velocity = input.velocity;
    let speed = length(input.angular_velocity);
    let rotation = if speed < f32::from_bits(0x3780_0000) {
        super::super::skeleton_animation_record::IDENTITY
    } else {
        pose::axis_rotation(
            scale(input.angular_velocity, 1. / speed),
            speed * input.timestep,
        )
    };
    let acceleration_step = madd(
        input.up,
        s.stomp * input.timestep,
        [0., input.timestep * f32::from_bits(0xc11c_cccd), 0., 0.],
    );
    for frame in 0..s.frames {
        samples[frame] = s.points.map(|p| {
            madd(
                board[2],
                p[2],
                madd(board[1], p[1], madd(board[0], p[0], board[3])),
            )
        });
        //82D71F40, preserve the target's literal post-ACos 90/180 fold.
        let forward = sub(samples[frame][1], samples[frame][4]);
        let projected = unit(sub(forward, scale(normal, dot(normal, forward))));
        let aligned = if dot(projected, rail) < 0. {
            scale(projected, -1.)
        } else {
            projected
        };
        let mut angle = crate::trigonometry::acos(dot(aligned, rail).max(-1.).min(1.)).abs();
        if angle > 90. {
            angle = 180. - angle;
        }
        let sign = if dot(normal, cross(aligned, rail)) < 0. {
            -1.
        } else {
            1.
        };
        headings[frame] = -(sign * angle * RAD);
        for axis in &mut board[..3] {
            *axis = pose::direction(rotation, *axis);
        }
        velocity = add(velocity, acceleration_step);
        board[3] = madd(velocity, input.timestep, board[3]);
    }
    samples
}
