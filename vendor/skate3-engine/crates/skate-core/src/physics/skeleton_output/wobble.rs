//! SkateboardWobble constructor82BF2E48, trigger82BF2F18, update82BF2FB8.
use crate::point_graph::PointGraph;

pub struct Settings {
    pub takeoff_tilt: PointGraph<8>,
    pub landing_tilt: PointGraph<8>,
    pub takeoff_squish: PointGraph<8>,
    pub landing_squish: PointGraph<8>,
    pub maximum_time: f32,
}
pub struct Wobble {
    pub active: bool,
    pub landing: bool,
    pub time: f32,
    pub amplitude: f32,
    pub direction: f32,
    selected_landing_curves: bool,
}
impl Default for Wobble {
    fn default() -> Self {
        //Constructor copies landing curves while initializing type17=false.
        Self {
            active: false,
            landing: false,
            time: 0.0,
            amplitude: 0.0,
            direction: 1.0,
            selected_landing_curves: true,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct Output {
    ///Native out0 reports whether this update evaluated the effect, including
    ///the final sample whose clock expires it. Return value is active AFTER.
    pub sampled: bool,
    pub tilt: f32,
    pub squish: f32,
    pub remains_active: bool,
}
impl Wobble {
    pub fn trigger(&mut self, landing: bool, reverse: bool) {
        self.selected_landing_curves = landing;
        self.landing = landing;
        self.active = true;
        self.time = 0.0;
        self.amplitude = 1.0;
        self.direction = if reverse { -1.0 } else { 1.0 };
    }
    pub fn update(&mut self, settings: &Settings) -> Output {
        if !self.active {
            return Output::default();
        }
        let (tilt, squish) = if self.selected_landing_curves {
            (&settings.landing_tilt, &settings.landing_squish)
        } else {
            (&settings.takeoff_tilt, &settings.takeoff_squish)
        };
        let squish = squish.evaluate(self.time) * self.amplitude;
        let tilt = (tilt.evaluate(self.time) * self.direction) * self.amplitude;
        self.time += f32::from_bits(0x3C88_8889);
        if self.time > settings.maximum_time {
            self.active = false;
            self.time = 0.0;
        }
        Output {
            sampled: true,
            tilt,
            squish,
            remains_active: self.active,
        }
    }
}

/// PostWipeoutCheck82BD857C..86D4 modifies the observed board record only.
///This wobble is not a force or a SetPartTransform on the simulated deck.
pub fn apply(
    output: Output,
    board: &mut crate::physics::skeleton_animation_record::AnimationPartTransform,
) {
    if !output.sampled {
        return;
    }
    use crate::{physics::skeleton_animation_record::compose_affine, trigonometry::sin_cos};
    let (sine, cosine) = sin_cos(output.tilt);
    //822FB890 permute and vrlimi mask2 retain the native packed W lanes.
    let rotation = [
        [cosine, sine, 0.0, cosine],
        [-sine, cosine, 0.0, -sine],
        [0.0, 0.0, 1.0, 0.0],
        [0.0; 4],
    ];
    *board = compose_affine(board, &rotation);
    for lane in 0..4 {
        board[3][lane] = board[1][lane].mul_add(output.squish, board[3][lane]);
    }
}
