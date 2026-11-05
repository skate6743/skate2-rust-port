//! Original TU3 SkateboardController. Lifecycle state448 remains in the existing
//! SkateboardControllerFields; this module owns only its additional data.
pub mod angular;
pub mod lifecycle;
pub mod manager;
pub mod math;
pub mod motion;
#[cfg(test)]
mod tests;
pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];
pub const IDENTITY: Frame = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0.; 4],
];
use crate::player::lifecycle::SkateboardControllerFields;

#[derive(Clone, Copy, Debug)]
pub struct Retrieval {
    pub initial_0: Frame,
    pub target_64: Frame,
    pub current_128: Frame,
    pub elapsed_192: f32,
    pub duration_196: f32,
    pub progress_200: f32,
    pub weight_204: f32,
}
impl Default for Retrieval {
    fn default() -> Self {
        Self {
            initial_0: IDENTITY,
            target_64: IDENTITY,
            current_128: IDENTITY,
            elapsed_192: 0.,
            duration_196: 0.,
            progress_200: 0.,
            weight_204: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct HandDrive {
    /// Native child frame224/288, parent frame256/320.
    pub child: Frame,
    pub parent: Frame,
    /// Linear then angular spring/maxvelocity,damping,strength,type.
    pub dynamics: [[u32; 4]; 2],
}
impl Default for HandDrive {
    fn default() -> Self {
        Self {
            child: IDENTITY,
            parent: IDENTITY,
            dynamics: [[0, 0, 0, 2]; 2],
        }
    }
}
#[derive(Clone, Debug)]
pub struct State {
    pub retrieval: Retrieval,
    pub hands: [HandDrive; 2],
    pub selected_hand_424: u32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            retrieval: Retrieval::default(),
            hands: [HandDrive::default(); 2],
            selected_hand_424: 2,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Processed {
    pub board_frame_64: Frame,
    pub player_frame_192: Frame,
    pub position_592: Vector,
    pub velocity_912: Vector,
    pub direction_400: Vector,
    pub hide_direction_464: Vector,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2488: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub angle_36: f32,
    pub angle_40: f32,
    pub held_311: bool,
    pub free_312: bool,
    pub returning_313: bool,
    pub hiding_321: bool,
    pub flag_322: bool,
    pub flag_323: bool,
    pub flag_324: bool,
}
/// Complete82D76D20 numerical/byte publication, given actual82BE3170 bone11.
pub fn fill(
    fields: &SkateboardControllerFields,
    state: &State,
    p: &Processed,
    bone11: Frame,
) -> Fill {
    let frame = p.player_frame_192;
    let on_axis = math::madd(
        frame[1],
        math::dot(math::sub(bone11[3], frame[3]), frame[1]),
        frame[3],
    );
    let position = if fields.state_448 == 3 {
        state.retrieval.initial_0[3]
    } else {
        p.board_frame_64[3]
    };
    let delta = math::sub(position, on_axis);
    let angle = math::quadrant_angle(math::dot(delta, frame[0]), math::dot(delta, frame[2]));
    let s = fields.state_448;
    let bits = p.flags_2476;
    Fill {
        angle_36: angle,
        angle_40: 0.,
        held_311: s == 1,
        free_312: matches!(s, 2 | 3 | 4),
        returning_313: s == 4,
        hiding_321: s == 3,
        flag_322: bits & 0x200 != 0,
        flag_323: bits & 0x400 != 0 || s == 4,
        flag_324: bits & 0x400 != 0 || (bits & 0x800 != 0 && s == 4),
    }
}
