//! TU3 one-truck control82D89F58 and orientation82D40290.
use super::*;

#[derive(Default)]
pub struct Control {
    pub yaw: f32,
    pub pitch: f32,
}
impl Control {
    ///Animation2796/2800/2804/2808; candidate412 bit29 selects the front end.
    pub fn update(
        &mut self,
        kind: u32,
        board_forward: V,
        normal: V,
        front: bool,
        switched: bool,
        hanging_back: bool,
        translation: f32,
        nudge: f32,
        up_down: f32,
        grab_min_height: f32,
    ) {
        let (yaw, pitch, modified) = if kind == 0 {
            (0.0, up_down * up_down * up_down, false)
        } else if kind == 3 {
            let selected = front != switched;
            let tilt = dot3(normal, board_forward);
            let height = (if selected { -tilt } else { tilt }) * if switched { -1.0 } else { 1.0 };
            let mut input = up_down;
            if height > 0.0 && up_down.abs() <= 0.01 && height > -0.25 {
                input = if selected { -0.71 } else { 0.71 };
            }
            let mut target = input;
            if tilt.abs() > 0.26 {
                target = if selected {
                    input.min(0.0)
                } else {
                    input.max(0.0)
                };
            }
            if height < -0.25 {
                target = if selected {
                    input.max(0.0)
                } else {
                    input.min(0.0)
                };
            }
            let sign = (if front { 1.0 } else { -1.0 }) * if switched { -1.0 } else { 1.0 };
            if grab_min_height > 0.0 && height <= grab_min_height {
                let value = (grab_min_height - height) * sign * 10.0;
                target = if sign > 0.0 {
                    value.min(sign)
                } else {
                    value.max(sign)
                };
            }
            if hanging_back {
                target = (target + sign * 0.5).clamp(-1.0, 1.0);
            }
            //82D8A238/240: round the nudge product, then fused translation.
            let yaw = deadzone(translation)
                .mul_add(0.85, deadzone(nudge) * 0.79)
                .clamp(-0.85, 0.85);
            (yaw * yaw * yaw, target * target * target, target != input)
        } else {
            (0.0, 0.0, false)
        };
        let response: f32 = if modified { 0.76 } else { 0.91 };
        self.yaw = self.yaw.mul_add(0.91, yaw * f32::from_bits(0x3db851e8));
        self.pitch = (1.0 - response).mul_add(pitch, self.pitch * response);
        if self.yaw.abs() < 0.0001 {
            self.yaw = 0.0;
        }
        if self.pitch.abs() < 0.0001 {
            self.pitch = 0.0;
        }
    }
}
///82D862B8: signed .25 dead zone, remaining range scaled by4/3.
fn deadzone(v: f32) -> f32 {
    if v > 0.25 {
        (v - 0.25) * (4.0 / 3.0)
    } else if v < -0.25 {
        (v + 0.25) * (4.0 / 3.0)
    } else {
        0.0
    }
}

pub fn rotate(axis: V, value: V, angle: f32) -> V {
    let (s, c) = crate::trigonometry::sin_cos(angle * 0.5);
    let q = scale(axis, s);
    add(
        value,
        scale(cross(q, add(scale(value, c), cross(q, value))), 2.0),
    )
}

///82D40290: yaw around support, orthogonalize, then pitch around deck right.
pub fn truck_frame(board: [V; 4], normal: V, control: &Control, switched: bool) -> [V; 4] {
    let mut forward = rotate(normal, board[2], control.yaw * -0.68);
    let right = cross(normal, forward);
    let length = dot3(right, right).sqrt();
    if length <= 0.001 {
        return board;
    }
    let right = scale(right, length.recip());
    forward = rotate(
        right,
        forward,
        control.pitch * if switched { -0.68 } else { 0.68 },
    );
    [right, cross(forward, right), forward, board[3]]
}

///82D42528/82D42AF8: rotate support by20/42 degrees about the directed rail,
///then align board-right with the rail. Side is determined by contact offset.
pub fn tip_frame(board: [V; 4], direction: V, normal: V, point: V, backslash: bool) -> [V; 4] {
    let across = cross(direction, normal);
    let axis = scale(
        direction,
        if dot3(across, sub(board[3], point)) > 0.0 {
            -1.0
        } else {
            1.0
        },
    );
    let angle = if backslash {
        f32::from_bits(0x3f3ba866)
    } else {
        f32::from_bits(0x3eb2b8c3)
    };
    let up = rotate(axis, normal, angle);
    let right = scale(
        direction,
        if dot3(direction, board[0]) > 0.0 {
            1.0
        } else {
            -1.0
        },
    );
    [right, up, cross(right, up), board[3]]
}
