use super::*;
#[derive(Clone, Copy, Debug)]
pub struct PoseInput {
    ///Skeleton11920, not the processed physical board frame.
    pub animation_to_world: Frame,
    ///Skeleton11984, already produced by UpdateRootTransforms.
    pub world_to_animation: Frame,
    ///Skeleton12528: retained physical-board Z axis.
    pub physical_forward: V,
    ///Skeleton12672: this frame's mapped animation board translation.
    pub animation_board_position: V,
}
///Full-angle axis matrix used by82D71430 and82BDCFB0 (not half-angle quaternion).
pub(super) fn axis_rotation(axis: V, angle: f32) -> Frame {
    let (sin, cos) = crate::trigonometry::sin_cos(angle);
    let [x, y, z, _] = axis;
    let k = 1. - cos;
    [
        [
            (k * x).mul_add(x, cos),
            (k * x).mul_add(y, sin * z),
            (k * x).mul_add(z, -sin * y),
            0.,
        ],
        [
            (k * y).mul_add(x, -sin * z),
            (k * y).mul_add(y, cos),
            (k * y).mul_add(z, sin * x),
            0.,
        ],
        [
            (k * z).mul_add(x, sin * y),
            (k * z).mul_add(y, -sin * x),
            (k * z).mul_add(z, cos),
            0.,
        ],
        ZERO,
    ]
}
pub(super) fn direction(frame: Frame, vector: V) -> V {
    madd(
        frame[2],
        vector[2],
        madd(frame[1], vector[1], scale(frame[0], vector[0])),
    )
}
impl Adjustment {
    ///82BDCFB0: R(up,Y)*R(rail,X)*R(physicalZ,Z), converted to animation space,
    ///then pivot around the current mapped board and add the local displacement.
    pub fn local_transform(self, input: PoseInput) -> Frame {
        //Unlike NormalizeSafe, this native wrapper has no zero-vector fallback.
        let across = cross(UP, self.axis);
        let across = scale(across, inverse_length(dot(across, across)));
        let up = cross(self.axis, across);
        let up = scale(up, inverse_length(dot(up, up)));
        let yaw = axis_rotation(up, self.angles[1]);
        let lean = axis_rotation(self.axis, self.angles[0]);
        let pitch = axis_rotation(input.physical_forward, self.angles[2]);
        let mut local = super::super::skeleton_animation_record::IDENTITY;
        for i in 0..3 {
            local[i] = direction(
                input.world_to_animation,
                direction(
                    yaw,
                    direction(lean, direction(pitch, input.animation_to_world[i])),
                ),
            );
        }
        let pivot = input.animation_board_position;
        local[3] = sub(
            add(direction(input.world_to_animation, self.offset), pivot),
            direction(local, pivot),
        );
        local
    }
}
