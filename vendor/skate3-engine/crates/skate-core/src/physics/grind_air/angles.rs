use super::*;
pub(super) fn update(
    state: &mut GrindAir,
    input: Input,
    s: &Settings,
    target: Target,
    samples: &prediction::Samples,
    rail: V,
    normal: V,
    kind: usize,
    time: f32,
) {
    //82D723D0 stores angleArray[type], NOT the range validator's frame index.
    let heading = state.headings[kind];
    let desired = if heading > 0. {
        targets::DESIRED[kind]
    } else {
        -targets::DESIRED[kind]
    };
    let speed = ((desired - heading) / time * s.yaw_assist[kind])
        .max(-s.max_angle)
        .min(s.max_angle);
    state.angle_delta[1] = (state.angle_delta[1] * RAD + speed)
        .max(-s.max_angle)
        .min(s.max_angle)
        * DEG;
    let points = samples[(time + 0.5) as usize];
    let forward = sub(points[1], points[4]);
    let projected = unit(sub(forward, scale(rail, dot(rail, forward))));
    let o = target.orientation;
    let error = match kind {
        0 | 3 => {
            let angle = signed_angle(projected, o.boardslide_dir, rail);
            if angle > PI * 0.5 {
                angle - PI
            } else if angle < -PI * 0.5 {
                angle + PI
            } else {
                angle
            }
        }
        1 | 2 => {
            let from = if kind == 1 {
                scale(projected, -1.)
            } else {
                projected
            };
            let a = signed_angle(from, o.tipslide_dir, rail);
            let b = signed_angle(from, o.backslash_dir, rail);
            if a.abs() < b.abs() { a } else { b }
        }
        _ => 0.,
    };
    let maximum = s.max_angle * DEG;
    state.angle_delta[0] = (state.angle_delta[0] + error / time)
        .max(-maximum)
        .min(maximum);
    //82D71BB0 native broadcast subtraction is intentional, not vector rejection.
    let projected_right = unit(input.board[0].map(|v| v - dot(input.board[0], normal)));
    let correction = if dot(projected_right, projected_right) > 0.9 {
        signed_angle(input.board[0], projected_right, input.board[2]) * 0.9
    } else {
        0.
    };
    state.angle_delta[2] = if kind < 4 { correction / time } else { 0. };
    state.angles = add(state.angles, state.angle_delta);
}
