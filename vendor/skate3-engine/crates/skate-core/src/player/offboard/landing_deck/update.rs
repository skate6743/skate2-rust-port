//! Ordered Air assistance82D78D30, manager Update82D794A0 and IK82D792D0.
use super::*;
use crate::player::wipeout_state::math::{add, dot, length, madd, normalize_or, scale, sub};
const UP: Vector = [0., 1., 0., 0.];

impl Manager {
    ///82D78D30. No pending guard is added: the host's real batch must preserve
    ///the original Begin/submit ordering when replacing an outstanding request.
    pub fn assist(&mut self, input: &AssistInput, settings: &Settings) -> Option<QueryRequest> {
        self.completed_queries_252 = 0;
        self.tested_259 = false;
        let p = &input.processed;
        if (self.force_257 || p.flags_2480 & 0x8000 == 0)
            && p.board_up_80[1] > settings.deck_min_uprightness
        {
            self.consider_board(p, settings, input.position, input.velocity);
            if self.time_to_land_244 > 0.4 {
                let error = length(sub(input.velocity, self.proposed_96.velocity));
                self.trajectory_32 = self.proposed_96;
                self.trajectory_valid_164 = true;
                self.time_to_land_244 = self.proposed_time_248;
                self.can_land_256 = error < input.maximum_velocity_change;
            }
            if self.can_land_256 && !self.tested_259 {
                self.tested_259 = true;
                return Some(self.prepare_query(p, input.position, self.proposed_time_248));
            }
        }
        None
    }

    ///82D794A0. This returns ALL numeric fields consumed by503; no target-frame
    ///input is consumed by the original (r4 is its output structure).
    pub fn update(&mut self, p: &Input, settings: &Settings) -> UpdateOutput {
        self.publish_moving_contact_261 = false;
        self.elapsed_160 += STEP;
        if self.hippy_hurdling_260 {
            let past_apex = self.elapsed_160 - self.trajectory_32.highest_position().1;
            if past_apex > 0.3 || (past_apex > 0. && self.time_to_land_244 < 0.2) {
                self.hippy_hurdling_260 = false;
            }
        }
        let position = self.trajectory_32.position_at(self.elapsed_160);
        let velocity = self.trajectory_32.velocity_at(self.elapsed_160);
        self.can_land_256 = !matches!(p.mode_2540, 6 | 9 | 12);
        self.consider_board(p, settings, position, velocity);
        //Native query decision precedes the final error/possession/upright gate.
        let query = if !self.pending_262 && !self.tested_259 && self.can_land_256 {
            Some(self.prepare_query(p, position, self.time_to_land_244))
        } else {
            None
        };
        let maximum_error = if p.support_1776 < 0 { 0.55 } else { 0.5 };
        self.can_land_256 = length(self.ik_offset_176) < maximum_error
            && (self.force_257 || p.flags_2480 & 0x8000 == 0)
            && p.board_up_80[1] > settings.deck_min_uprightness;
        let landing_time = self.time_to_land_244 + self.elapsed_160;
        let landing_velocity = self.trajectory_32.velocity_at(landing_time);
        let (apex_position, apex_time) = self.trajectory_32.highest_position();
        UpdateOutput {
            can_land: self.can_land_256 && !self.blocked_258,
            trajectory_valid: self.trajectory_valid_164,
            time_to_land: self.time_to_land_244,
            elapsed: self.elapsed_160,
            landing_time,
            apex_time,
            position,
            landing_velocity,
            launch_position: self.trajectory_32.position,
            landing_position: self.trajectory_32.position_at(landing_time),
            normal: UP,
            apex_position,
            direction: normalize_or(landing_velocity, UP),
            up: UP,
            query,
        }
    }

    ///82D792D0. Disassembly792DC..79308 restores the decompiler's lost elapsed
    ///sum. Root transforms the LOCAL mapped-position-minus-animation-COM vector.
    pub fn calculate_accurate_ik_offset(&mut self, input: &IkInput) -> Vector {
        let p = &input.processed;
        let trajectory_time = self.elapsed_160 + input.time_to_land;
        let deck_time = input.time_to_land + STEP;
        let local = sub(input.mapped_position_12608, input.animation_com_10960);
        let root = input.animation_root;
        let offset = madd(
            root[2],
            local[2],
            madd(root[1], local[1], scale(root[0], local[0])),
        );
        let tangent_velocity = sub(
            p.board_velocity_400,
            scale(p.up_544, dot(p.up_544, p.board_velocity_400)),
        );
        self.ik_offset_176 = sub(
            madd(tangent_velocity, deck_time, p.board_position_112),
            add(self.trajectory_32.position_at(trajectory_time), offset),
        );
        self.ik_offset_176[1] = 0.;
        self.ik_offset_176
    }
}
