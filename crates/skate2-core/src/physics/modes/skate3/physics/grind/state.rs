//! Separate native state instances: common ctor82D3F210 and Enter82D3F318.
use super::{Family, output};
pub(super) type V = [f32; 4];

#[derive(Clone, Copy)]
pub(super) struct State {
    pub output: output::State,
    pub frame: [V; 4],
    pub already_jumped: bool,
    pub updates: i32,
    pub leaving_updates: i32,
    pub preparing_jump: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            output: output::State {
                direction: [0., 1., 0., 0.],
                normal: [0., 1., 0., 0.],
                across: [0., 1., 0., 0.],
                crouch: 0.,
                leaving: false,
                substate: 0,
                classification_104: 0,
                just_jumped: false,
                jump_velocity: [0.; 4],
                slide_wipeout: false,
                slide_impulse: [0.; 4],
                tipslide_97_98_99: [false; 3],
            },
            frame: [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0.; 4],
            ],
            already_jumped: false,
            updates: 0,
            leaving_updates: 0,
            preparing_jump: false,
        }
    }

    pub fn enter(&mut self, family: Family, board: [V; 4]) {
        // Enter deliberately does not overwrite vectors48/64/80 or word104.
        self.output.substate = 1;
        self.output.leaving = false;
        self.already_jumped = false;
        self.output.just_jumped = false;
        self.output.tipslide_97_98_99 = [false; 3];
        self.frame = board;
        self.updates = 0;
        self.output.crouch = 0.;
        self.leaving_updates = 0;
        self.output.jump_velocity = [0.; 4];
        if matches!(family, Family::Boardslide | Family::Darkslide) {
            // Derived Enter82D41100 clears bytes240/241, not impulse224.
            self.output.slide_wipeout = false;
            self.preparing_jump = false;
        }
    }
}
