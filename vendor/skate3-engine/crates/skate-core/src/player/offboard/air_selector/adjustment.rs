//!82D6DE08/E018/E178: animation displacement adjusts the sampling arc only.
use super::{DT, Frame, Selector, Vector, math::*};
impl Selector {
    pub fn adjust_animation(&mut self, frame: i32, animation: Vector, axes: Frame, radius: f32) {
        self.sampling.adjustment_8336 = scale(axes[2], animation[2]);
        self.correction_8304 = scale(
            //82D6DF40 is one vmaddfp after the separate forward multiply.
            madd(
                axes[1],
                animation[1] - radius,
                self.sampling.adjustment_8336,
            ),
            -1.,
        );
        self.correction_8304[1] -= 0.08;
        let s = &mut self.sampling.selection;
        let delta = sub(
            sub(s.position_6176, self.correction_8304),
            s.trajectory_8144
                .position_at(s.landing_frame_8480 as f32 * DT),
        );
        shift(&mut s.trajectory_8144, frame as f32 * DT);
        adjust(
            &mut s.trajectory_8144,
            s.landing_frame_8480.wrapping_sub(frame),
            delta,
            0.5,
        );
        shift(&mut s.trajectory_8144, frame.wrapping_neg() as f32 * DT);
        let t = s.trajectory_8144;
        let end = t.position_at(s.landing_frame_8480 as f32 * DT);
        if self.launch.up_48[1] < 0.8
            && dot(self.launch.up_48, flatten(t.velocity)) <= 0.
            && s.normal_6144[1] > 0.9
            && end[1] - t.position[1] > -0.2
        {
            //Runtime constants830BD400/330 are initialized by82F82588/826B0:
            //splat(-9.8), [0,-9.8,0,0]; not the launch gravity attribute.
            let g = f32::from_bits(0x411ccccd);
            let distance = length(sub(t.position, end));
            let square = 2. * distance * reciprocal(g);
            let root = if square == 0. {
                0.
            } else {
                square * crate::physics::board_motion_output::inverse_length_squared(square, 2)
            };
            let duration = root * 0.66 + s.landing_frame_8480 as f32 * 0.0056666667;
            if duration > 0.2 {
                let acceleration = [0., -g, 0., 0.];
                s.trajectory_8144.velocity = sub(
                    scale(sub(end, t.position), reciprocal(duration)),
                    scale(scale(acceleration, 0.5), duration),
                );
                s.trajectory_8144.acceleration = acceleration;
                s.trajectory_8144.duration = -1.;
                s.landing_frame_8480 = (duration * 59.999996) as i32;
                s.scalar_8392 = s.landing_frame_8480 as f32 * DT;
            }
        }
    }
}
