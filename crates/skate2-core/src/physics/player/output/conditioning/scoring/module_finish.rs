//! ScoreModule runout82de9f80, gestures82dea060, banking82df61e8/2888/2b00.
use super::{
    super::scoring::Scoring2,
    score_module::ScoreModule,
    scoring_attributes::{ScoreError, scalar},
    scoring_meters::MeterKind,
};

pub(super) struct SequenceFinishFrame {
    pub activity_worthy: bool,
    pub switched: bool,
    pub fakie: bool,
    pub current_is_grind: bool,
    pub current_is_manual: bool,
    pub multiplier_override: Option<i32>,
}
impl ScoreModule {
    pub(super) fn check_runout(
        &mut self,
        physics_request: bool,
        animation_running_out: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        if physics_request {
            state.num_frames_left_to_remember_requested_runout = 30;
        } else if state.num_frames_left_to_remember_requested_runout != 0 {
            state.num_frames_left_to_remember_requested_runout -= 1;
        }
        if animation_running_out || state.num_frames_left_to_remember_requested_runout != 0 {
            out.is_running_out = true;
        }
        Ok(())
    }
    pub(super) fn check_gestures(
        &mut self,
        gesture: i32,
        is_gesturing: bool,
    ) -> Result<(), ScoreError> {
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        state.just_performed_gesture = false;
        if gesture == 35 && is_gesturing {
            state.num_frames_since_last_gesture =
                state.num_frames_since_last_gesture.wrapping_add(1);
        } else {
            state.current_gesture = gesture;
            state.num_frames_since_last_gesture = 0;
        }
        let data = self.general.try_get_data(0xd588_009e_c03a_a21a, 0)?;
        let frames = match data {
            None => 0,
            Some(data) => u32::from_be_bytes(
                data.bytes()
                    .get(..4)
                    .ok_or(ScoreError::Invalid("gesture threshold width"))?
                    .try_into()
                    .unwrap(),
            ),
        };
        if state.num_frames_since_last_gesture == frames {
            state.just_performed_gesture = true;
        }
        Ok(())
    }
    pub(super) fn reward_air_sequence(
        &mut self,
        f: &SequenceFinishFrame,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let mut holder = self
            .holder
            .published_value()
            .ok_or(ScoreError::Missing("score holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        if !holder.air_sequence_reward_pending {
            return Ok(());
        }
        let mut factor = 1.0;
        let switched = f.switched && !f.fakie;
        let fakie = f.fakie && !f.switched;
        if switched {
            factor = scalar(&self.states, 0xe2af_e1b0_6292_209d)?;
        }
        if fakie {
            factor *= scalar(&self.states, 0x05bb_af2e_866e_083e)?;
        }
        if f.current_is_grind {
            factor = scalar(&self.states, 0x77dd_0728_b473_6957)? * factor;
        }
        if f.current_is_manual {
            factor = scalar(&self.states, 0xb0d7_a387_6d2d_274e)? * factor;
        }
        if holder.pending_air_period.landing_was_sketchy {
            factor *= scalar(&self.states, 0x62b9_c3da_a846_e240)?;
        }
        // TU3 RewardAirSequence82DA32F0..3304 applies the authored clean
        // factor AFTER the sketchy branch. Keep S2's independent policy intact.
        #[cfg(feature = "skate3-runtime")]
        if self.states.skate3_enabled() && holder.skate3_clean_landing {
            factor *= scalar(&self.states, 0x5a2d_1476_f086_90ec)?;
        }
        holder.reward_air(factor, out);
        Ok(())
    }
    pub(super) fn check_sequence(
        &mut self,
        f: &SequenceFinishFrame,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let state = self
            .reset_suffix
            .as_ref()
            .ok_or(ScoreError::Missing("score reset"))?;
        let multiplier_active = state.is_multiplier_active;
        let just_entered_wipeout = state.just_entered_wipeout;
        let scale;
        let flags;
        {
            let mut holder = self
                .holder
                .published_value()
                .ok_or(ScoreError::Missing("score holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            let worthy;
            if state.just_entered_wipeout || out.is_running_out {
                if state.just_entered_wipeout {
                    holder.wipeout(out);
                } else {
                    holder.offboard_runout(out);
                }
                scale = scalar(&self.general, 0xb0b5_6ff0_4650_8506)?;
                worthy = false;
            } else {
                scale = 1.0;
                worthy = f.activity_worthy;
            }
            let sequence = holder.sequence_mut()?;
            sequence.update(worthy, state.reset_sequence, out)?;
            flags = sequence.flags_word;
        }
        if flags & 0x4000_0000 != 0 {
            self.reward_air_sequence(f, out)?;
            let mut holder = self
                .holder
                .published_value()
                .ok_or(ScoreError::Missing("score holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            let raw = (holder.air_sequence_score_pending + holder.air_sequence_score)
                + holder.sequence_score;
            let points = if scale >= 1.0 { raw } else { raw * scale };
            trace_score(format_args!("sequence end: wipeout {} runout {} worthy {} scale {scale} points {points}",
                just_entered_wipeout, out.is_running_out, f.activity_worthy));
            let mut multiplier = 1.0;
            let line = self
                .line_timer
                .published_value_mut()
                .ok_or(ScoreError::Missing("line timer"))?;
            if multiplier_active {
                let momentum = self
                    .momentum
                    .published_value_mut()
                    .ok_or(ScoreError::Missing("momentum meter"))?;
                // The finishing sequence uses the multiplier BEFORE this score
                // raises the meter; subsequent sequences receive the new value.
                multiplier = momentum.get_multiplier(f.multiplier_override)?;
                momentum.end_sequence(points)?;
                if momentum.get_multiplier(f.multiplier_override)? > 1.01 {
                    line.stoke(MeterKind::Line, points)?;
                }
            }
            holder.end_sequence(
                multiplier * points,
                multiplier_active && line.points > 0.0,
                out,
            );
        } else if flags & 0x8000_0000 == 0 {
            let mut holder = self
                .holder
                .published_value()
                .ok_or(ScoreError::Missing("score holder"))?
                .lock()
                .map_err(|_| ScoreError::Poisoned)?;
            holder.tricks_in_sequence.fill(0);
            holder.trick_types_in_sequence.fill(0);
        }
        Ok(())
    }
    pub(super) fn check_line(
        &mut self,
        retrieving_board: bool,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let state = self
            .reset_suffix
            .as_ref()
            .ok_or(ScoreError::Missing("score reset"))?;
        let line = self
            .line_timer
            .published_value_mut()
            .ok_or(ScoreError::Missing("line timer"))?;
        let finished = state.just_entered_wipeout
            || out.is_running_out
            || line.just_reached_zero
            || retrieving_board;
        let mut holder = self
            .holder
            .published_value()
            .ok_or(ScoreError::Missing("score holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?;
        if finished {
            out.line_timer_finished = true;
            line.reset();
            self.momentum
                .published_value_mut()
                .ok_or(ScoreError::Missing("momentum meter"))?
                .multiplier = Some(1.0);
            let score = holder.line_score;
            holder.line_score = 0.0;
            holder.session_score += score;
            holder.line_accumulator.fill(0);
        } else if line.seconds(MeterKind::Line)? <= 0.0 {
            holder.session_score += holder.line_score;
            holder.line_score = 0.0;
            if holder.sequence_mut()?.flags_word & 0x8000_0000 == 0 {
                holder.line_accumulator.fill(0);
            }
        }
        Ok(())
    }
}

/// `SKATE2_TRACE_SCORE_SEQUENCE`: collector changes and sequence endings.
pub(super) fn trace_score(message: std::fmt::Arguments<'_>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
    if !*TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SCORE_SEQUENCE").is_some()) {
        return;
    }
    let text = message.to_string();
    if let Ok(mut last) = LAST.lock() {
        if *last != text {
            eprintln!("score: {text}");
            *last = text;
        }
    }
}
