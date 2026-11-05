use super::{math::*, *};
use crate::point_graph::PointGraph;
pub struct Settings {
    pub hide_distance: f32,
    pub hide_offset: f32,
    pub return_distance: f32,
    pub mounted_return_distance: f32,
    pub mounting_time: f32,
    pub retrieval_time: PointGraph<8>,
    pub retrieval_weight: PointGraph<8>,
    pub throw_pitch: f32,
    pub throw_velocity: PointGraph<8>,
    pub throw_target_pitch: f32,
    pub throw_pitch_scalar: f32,
    pub throw_roll_scalar: f32,
    pub throw_yaw_scalar: f32,
}
/// Actual completed physical and evaluated skeleton observations.
pub struct Observation {
    pub processed: Processed,
    pub board_collision_flags_872: u32,
    pub board_state_840: u32,
    pub hand_contacts: [bool; 2],
    pub physical_hand_positions: [Vector; 2],
    pub animation_board_frame_12624: Frame,
    pub animation_hand_frames: [Frame; 2],
    pub attachment_frame_0: Frame,
}
#[derive(Clone, Copy, Debug)]
pub struct Alignment {
    pub first_1008: Vector,
    pub second_1024: Vector,
    pub factor_1040: f32,
    pub flag_1044: bool,
}
/// Each call mutates the actual board/solver owner. No observation defaults.
pub trait Effects {
    fn enable_animation_soft(&mut self);
    fn enable_animation_angular_only(&mut self);
    fn disable_animation(&mut self);
    fn disable_linear_drive(&mut self);
    ///82C03AF8 clears8384 bit7, executes091F8/090C0, enables selected volumes.
    fn standard_board(&mut self);
    ///7549C..54D0 sets8384 bit7, deck drag literal416FFFFF,09160, enables volumes.
    fn released_board(&mut self);
    fn collision_volumes(&mut self, enabled: bool);
    fn clear_alignment(&mut self);
    fn alignment(&mut self, value: Alignment);
    fn velocity(&mut self, value: Vector);
    fn position(&mut self, value: Vector);
    fn hook_frame(&mut self, value: Frame);
    fn target_position_velocity(&mut self, value: Vector);
    fn torque(&mut self, value: Vector);
}
impl State {
    ///75EA0 retains the optional throw's word444 write made inside LetGo.
    pub fn stop(
        &mut self,
        f: &mut SkateboardControllerFields,
        o: &Observation,
        s: &Settings,
        e: &mut impl Effects,
    ) {
        if f.system_on_452 {
            f.word_444 = 0;
            if f.state_448 != 0 {
                self.let_go(f, o, s, e);
                f.state_448 = 0;
            }
            f.system_on_452 = false;
        }
    }
    ///75370 writes held state BEFORE selecting/arming the hand and drive frames.
    pub fn hold(
        &mut self,
        fields: &mut SkateboardControllerFields,
        o: &Observation,
        e: &mut impl Effects,
    ) {
        e.enable_animation_soft();
        e.standard_board();
        e.collision_volumes(true);
        fields.state_448 = 1;
        self.selected_hand_424 = (!o.processed.flags_2476 >> 2) & 1;
        self.hands[self.selected_hand_424 as usize].dynamics = [[0x4395ffff, 0, 0x468c9fff, 2]; 2];
        self.update_drive_frames(o);
    }
    pub fn disable_hand(&mut self) {
        if self.selected_hand_424 != 2 {
            self.hands[self.selected_hand_424 as usize].dynamics = [[0, 0, 0, 2]; 2];
            self.selected_hand_424 = 2;
        }
    }
    ///75440 does not itself change448. Caller transitions after all actions.
    pub fn let_go(
        &mut self,
        fields: &mut SkateboardControllerFields,
        o: &Observation,
        s: &Settings,
        e: &mut impl Effects,
    ) {
        e.disable_animation();
        e.released_board();
        e.collision_volumes(true);
        e.clear_alignment();
        self.disable_hand();
        if fields.state_448 == 4 {
            e.velocity([0.; 4]);
        }
        fields.word_444 = 0;
        if o.processed.flags_2476 & 0x1000 != 0 {
            fields.word_444 = 8;
            e.velocity(motion::throw_velocity(&o.processed, s));
        }
    }
    pub fn hide(&mut self, o: &Observation, e: &mut impl Effects) {
        e.collision_volumes(false);
        e.disable_linear_drive();
        e.clear_alignment();
        self.disable_hand();
        self.retrieval = Retrieval::default();
        self.retrieval.initial_0 = o.processed.board_frame_64;
        self.retrieval.elapsed_192 = 0.;
        e.enable_animation_angular_only();
    }
    pub fn retrieve(
        &mut self,
        fields: &SkateboardControllerFields,
        o: &Observation,
        s: &Settings,
        e: &mut impl Effects,
    ) {
        e.collision_volumes(false);
        e.clear_alignment();
        e.enable_animation_angular_only();
        let old = self.retrieval.initial_0[3];
        self.retrieval = Retrieval::default();
        self.retrieval.initial_0 = o.processed.board_frame_64;
        if fields.state_448 == 3 {
            self.retrieval.initial_0[3] = old;
        }
        self.retrieval.target_64 = o.attachment_frame_0;
        let mut delta = sub(self.retrieval.initial_0[3], self.retrieval.target_64[3]);
        let mut distance = length(delta);
        let tiny = f32::from_bits(0x3a83126f);
        if !(distance >= tiny) {
            distance = tiny;
            delta = scale(o.processed.player_frame_192[2], tiny);
        }
        let mounting = o.processed.flags_2480 & 0x80000 != 0;
        let maximum = if mounting {
            s.mounted_return_distance
        } else {
            s.return_distance
        };
        if distance > maximum || fields.state_448 == 3 {
            self.retrieval.initial_0[3] = madd(
                scale(delta, maximum),
                reciprocal(distance, 2),
                self.retrieval.target_64[3],
            );
            e.position(self.retrieval.initial_0[3]);
            distance = maximum;
        }
        self.retrieval.duration_196 = s.retrieval_time.evaluate(distance);
        self.retrieval.elapsed_192 = 0.;
        if mounting {
            let d = self.retrieval.duration_196;
            self.retrieval.duration_196 = if s.mounting_time - d >= 0. {
                d
            } else {
                s.mounting_time
            };
        }
    }
    ///75F00. Fields are borrowed from the single existing controller owner.
    pub fn update_state(
        &mut self,
        f: &mut SkateboardControllerFields,
        o: &Observation,
        s: &Settings,
        e: &mut impl Effects,
    ) {
        let p = &o.processed;
        match f.state_448 {
            1 => {
                let free_allowed = p.flags_2480 & 0x80000 == 0 && p.flags_2488 & 0x8000000 == 0;
                if p.flags_2476 & 0x2000 != 0
                    || (free_allowed && o.board_collision_flags_872 & 0x4000000 != 0)
                {
                    f.word_444 = 0;
                    self.let_go(f, o, s, e);
                    f.state_448 = 2;
                } else {
                    let hand = if self.selected_hand_424 == 0 { 0 } else { 1 };
                    e.alignment(Alignment {
                        first_1008: unit(
                            sub(o.physical_hand_positions[hand], p.board_frame_64[3]),
                            [0.; 4],
                        ),
                        second_1024: unit(sub(p.position_592, p.board_frame_64[3]), [0.; 4]),
                        factor_1040: f32::from_bits(0x3f4ccccd),
                        flag_1044: true,
                    });
                }
            }
            2 => {
                let delta = sub(p.board_frame_64[3], p.position_592);
                let distant = dot(delta, delta) > s.hide_distance * s.hide_distance;
                if p.flags_2480 & 0x80000 != 0 || p.flags_2476 & 0x800 != 0 {
                    f.word_444 = 0;
                    self.retrieve(f, o, s, e);
                    f.state_448 = 4;
                } else if distant || o.board_state_840 == 6 {
                    f.word_444 = 0;
                    self.hide(o, e);
                    f.state_448 = 3;
                }
            }
            3 => {
                if p.flags_2480 & 0x80000 != 0 || p.flags_2476 & 0x800 != 0 {
                    f.word_444 = 0;
                    self.retrieve(f, o, s, e);
                    f.state_448 = 4;
                }
            }
            4 => {
                let hand = ((!p.flags_2476 >> 2) & 1) as usize;
                if p.flags_2480 & 0x80000 == 0 && o.hand_contacts[hand] {
                    f.word_444 = 0;
                    self.let_go(f, o, s, e);
                    f.state_448 = 2;
                } else if !(self.retrieval.progress_200 < 1.) {
                    e.velocity([0.; 4]);
                    f.word_444 = 0;
                    self.hold(f, o, e);
                    f.state_448 = 1;
                }
            }
            5 => {
                if o.board_collision_flags_872 & 0x4000000 != 0 {
                    f.word_444 = 0;
                    self.let_go(f, o, s, e);
                    f.state_448 = 2;
                } else {
                    e.alignment(Alignment {
                        first_1008: p.board_frame_64[2],
                        second_1024: p.board_frame_64[2],
                        factor_1040: f32::from_bits(0x3f733333),
                        flag_1044: false,
                    });
                }
            }
            _ => {}
        }
    }
}
