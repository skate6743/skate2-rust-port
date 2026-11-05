use super::{lifecycle::*, math::*, *};
const DT: f32 = f32::from_bits(0x3c888889);
const RADIANS: f32 = f32::from_bits(0x3c8efa35);
impl State {
    ///751D8: inverse board rotation and inverse translation are evaluated
    ///separately before composing the selected hand. Preserve z/y/x order.
    pub fn update_drive_frames(&mut self, o: &Observation) {
        let hand = if self.selected_hand_424 == 0 { 0 } else { 1 };
        let a = o.animation_board_frame_12624;
        let b = o.animation_hand_frames[hand];
        let inverse: Frame = std::array::from_fn(|column| {
            if column < 3 {
                [a[0][column], a[1][column], a[2][column], 0.]
            } else {
                [0.; 4]
            }
        });
        let negative = sub([0.; 4], a[3]);
        let translation = madd(
            inverse[0],
            negative[0],
            madd(inverse[1], negative[1], scale(inverse[2], negative[2])),
        );
        let mut parent = IDENTITY;
        for i in 0..3 {
            parent[i] = transform_direction(inverse, b[i]);
        }
        parent[3] = madd(
            inverse[2],
            b[3][2],
            madd(inverse[1], b[3][1], madd(inverse[0], b[3][0], translation)),
        );
        self.hands[self.selected_hand_424 as usize].child = IDENTITY;
        self.hands[self.selected_hand_424 as usize].parent = parent;
    }
    /// Controller block82DB6150..61B0, after physical state's update callback.
    pub fn update(
        &mut self,
        f: &mut SkateboardControllerFields,
        o: &Observation,
        s: &Settings,
        e: &mut impl Effects,
    ) {
        if !f.system_on_452 {
            return;
        }
        self.update_state(f, o, s, e);
        match f.state_448 {
            1 => self.update_drive_frames(o),
            2 => {
                if (f.word_444 as i32) > 0 {
                    f.word_444 = f.word_444.wrapping_sub(1);
                    for torque in throw_torques(&o.processed, s) {
                        e.torque(torque);
                    }
                }
            }
            3 => {
                self.retrieval.elapsed_192 += DT;
                self.retrieval.progress_200 = 0.5;
                self.retrieval.weight_204 = 0.5;
                self.retrieval.target_64 = self.retrieval.initial_0;
                self.retrieval.target_64[3] = madd(
                    o.processed.hide_direction_464,
                    s.hide_offset,
                    o.processed.player_frame_192[3],
                );
                self.retrieval.current_128 = self.retrieval.target_64;
                e.hook_frame(self.retrieval.current_128);
                e.position(self.retrieval.current_128[3]);
                e.velocity([0.; 4]);
            }
            4 => {
                self.retrieval.elapsed_192 += DT;
                let fraction = self.retrieval.elapsed_192 / self.retrieval.duration_196;
                self.retrieval.progress_200 = if 1. - fraction >= 0. { fraction } else { 1. };
                self.retrieval.weight_204 =
                    s.retrieval_weight.evaluate(self.retrieval.progress_200);
                self.retrieval.target_64 = o.attachment_frame_0;
                self.retrieval.current_128 = crate::animation::foot_ik::interpolate_affine(
                    &self.retrieval.initial_0,
                    &self.retrieval.target_64,
                    self.retrieval.weight_204,
                );
                e.hook_frame(self.retrieval.current_128);
                e.target_position_velocity(self.retrieval.current_128[3]);
            }
            _ => {}
        }
    }
}
///764F0 retains even the initial zero-yaw construction and all fused products.
pub fn throw_velocity(p: &Processed, s: &Settings) -> Vector {
    let (sin, cos) = crate::trigonometry::sin_cos(0. * RADIANS);
    let yaw = [
        [cos, 0., -sin, cos],
        [0., 1., 0., 0.],
        [sin, 0., cos, sin],
        [0.; 4],
    ];
    let (sin, cos) = crate::trigonometry::sin_cos(-s.throw_pitch * RADIANS);
    let local = transform_direction(yaw, [0., -sin, cos, 0.]);
    let direction = transform_direction(p.player_frame_192, local);
    let speed = length(p.velocity_912);
    scale(direction, speed + s.throw_velocity.evaluate(speed))
}
///767D0: yaw, pitch, roll torque outputs in source append order.
pub fn throw_torques(p: &Processed, s: &Settings) -> [Vector; 3] {
    let mut flat_board = p.board_frame_64[2];
    flat_board[1] = 0.;
    let mut flat_direction = p.direction_400;
    flat_direction[1] = 0.;
    let heading = unit(flat_direction, unit(flat_board, [0., 0., 1., 0.]));
    let up = [0., 1., 0., 0.];
    let right = cross(up, heading);
    let basis = [right, up, heading, [0.; 4]];
    let (sin, cos) = crate::trigonometry::sin_cos(-s.throw_target_pitch * RADIANS);
    let target_right = transform_direction(basis, [1., 0., 0., 1.]);
    let target_up = transform_direction(basis, [0., cos, sin, 0.]);
    let target_forward = transform_direction(basis, [0., -sin, cos, 0.]);
    let mut forward = p.board_frame_64[2];
    let mut right = p.board_frame_64[0];
    if 0. > dot(forward, target_forward) {
        forward = forward.map(|v| -v);
        right = right.map(|v| -v);
    }
    [
        scale(
            scale(
                target_up,
                wrap(projected_angle(forward, target_forward, target_up)),
            ),
            s.throw_yaw_scalar,
        ),
        scale(
            scale(
                target_right,
                wrap(projected_angle(forward, target_forward, target_right)),
            ),
            s.throw_pitch_scalar,
        ),
        scale(
            scale(
                target_forward,
                wrap(projected_angle(right, target_right, target_forward)),
            ),
            s.throw_roll_scalar,
        ),
    ]
}
