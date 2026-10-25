//! Display-facing TricksBackEndCore and PhysOutModule_Hud827c79b8 bridge.
//! Scoring owns points, repeat penalties and meters. This module only retains
//! the native frontend values and orders display notifications. Career stats,
//! achievements are not projected here. Hall of Meat carries its own completed
//! physics facts for the separate authored display controller.
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrickName {
    pub index: i32,
    /// The trick's Vault collection key (`TrickSettings::trick_attrib_key`), 0 for none: what an
    /// online skater's HUD record names the trick by, since an index depends on the game data.
    pub attrib_key: u64,
    pub valid: bool,
    pub label: Option<Arc<str>>,
    pub accepts_spin: bool,
    pub nollie: bool,
    pub fakie_variant: Option<(Arc<str>, bool)>,
}

/// Completed physics facts, sampled once per selected simulation update.
#[derive(Clone, Debug, Default)]
pub struct PhysicsFrame {
    pub backend_commands: Vec<BackendCommand>,
    pub hom: super::hom_scoring::HomPhysicsFrame,
    pub simulation_rate: f32,
    pub context_entries: Vec<super::trick_context::Entry>,
    pub begun: Option<TrickName>,
    pub converted: Option<TrickName>,
    pub ended_count: u32,
    pub sequence_begun: bool,
    pub sequence_completed: bool,
    pub finished_sequence_score: u32,
    pub finished_tricks: u32,
    pub finished_taps: u32,
    pub interrupted: bool,
    pub just_wiped_out: bool,
    pub reactivated: bool,
    pub line_seconds: f32,
    pub line_points: i32,
    pub line_max_points: i32,
    pub line_score: u32,
    pub multiplier: f32,
    pub hud_score: u32,
    pub momentum_points: i32,
    pub aerial: bool,
    pub spins: i32,
    pub flips: i32,
    pub switched: bool,
    pub fakie: bool,
    pub manual_length: f32,
    pub grind_distance: f32,
    pub powerslide_length: f32,
    pub air_distance: f32,
    pub air_height: f32,
    pub air_step: f32,
    pub air_finished: bool,
    pub just_landed: bool,
    pub sketchy: bool,
    pub landed_height: f32,
    pub landed_drop: f32,
}

/// Ordered simulation writes delivered by the existing frontend frame route.
#[derive(Clone,Copy,Debug)]
pub enum BackendCommand { SetIsJacked(bool), SetCanShowJackedOverlay(bool), ShowCentralMessage(&'static str) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion { None, Success, Wipeout }

#[derive(Clone, Debug, PartialEq)]
pub enum DisplayEvent {
    /// SetCurrentTrick -> UpdateTrickDisplayState(Start).
    Start,
    /// UpdateTrickLandedStats only sends Landed for a valid failed trick.
    Landed,
    EndSequence,
    AirFinished(f32),
    ReadyToScore,
}

#[derive(Clone, Debug)]
pub struct TricksBackend {
    pub is_jacked: bool,
    pub can_show_jacked_overlay: bool,
    pub current: Option<TrickName>,
    pub completion: Completion,
    pub sequence_id: u32,
    pub total_tricks: u32,
    pub tricks_in_sequence: u32,
    pub sequence_on: bool,
    pub sequence_score: u32,
    pub sequence_sketchy: bool,
    pub converted: bool,
    pub multiplier: f32,
    pub line_seconds: f32,
    pub line_points: i32,
    pub line_max_points: i32,
    pub line_score: u32,
    pub line_on: bool,
    pub momentum_points: i32,
    pub spins: i32,
    pub flips: i32,
    pub switched: bool,
    pub fakie: bool,
    pub aerial: bool,
    pub trick_started: bool,
    pub wiped_out: bool,
    pub manual_length: f32,
    pub grind_distance: f32,
    pub powerslide_length: f32,
    pub air_distance: f32,
    pub air_height: f32,
    pub air_step: f32,
    pub drop_height: f32,
    pub air_on: bool,
    pub air_time: f32,
    pub sequence_time: f32,
    pub line_time: f32,
    current_tricks: u32,
    current_line_tricks: u32,
    active_spin: bool,
}
impl Default for TricksBackend {
    fn default() -> Self {
        Self {
            is_jacked:false,can_show_jacked_overlay:false,
            current: None, completion: Completion::None, sequence_id: 0,
            total_tricks: 0, tricks_in_sequence: 0, sequence_on: false,
            sequence_score: 0, sequence_sketchy: false, converted: false,
            multiplier: 0., line_seconds: 0., line_points: 0, line_max_points: 0,
            line_score: 0, line_on: false, momentum_points: 0, spins: 0, flips: 0,
            switched: false, fakie: false, aerial: false, trick_started: false,
            wiped_out: false, manual_length: 0., grind_distance: 0.,
            powerslide_length: 0., air_distance: 0., air_height: 0., air_step: 0.,
            drop_height: 0., air_on: false, air_time: 0., sequence_time: 0.,
            line_time: 0., current_tricks: 0, current_line_tricks: 0, active_spin: false,
        }
    }
}
impl TricksBackend {
    /// ClearData82a735f8 deliberately preserves sequence_on, wiped_out, meter
    /// points/maximum and the module's active-spin flag. Reset is distinct.
    pub fn clear_data(&mut self) {
        self.current = None;
        self.completion = Completion::None;
        self.line_seconds = 0.;
        self.air_distance = 0.; self.air_height = 0.; self.air_step = 0.;
        self.grind_distance = 0.; self.tricks_in_sequence = 0;
        self.switched = false; self.fakie = false;
        self.spins = 0; self.flips = 0; self.sequence_id = 0;
        self.total_tricks = 0; self.aerial = false; self.trick_started = false;
        self.multiplier = 0.; self.powerslide_length = 0.; self.sequence_score = 0;
        self.drop_height = 0.; self.sequence_sketchy = false; self.air_time = 0.;
        self.line_score = 0; self.manual_length = 0.; self.line_on = false;
        self.current_tricks = 0; self.current_line_tricks = 0; self.converted = false;
    }

    fn landed(&mut self, completion: Completion, events: &mut Vec<DisplayEvent>) {
        self.completion = completion;
        if !self.current.as_ref().is_some_and(|t| t.valid) { return; }
        if completion != Completion::Success {
            events.push(DisplayEvent::Landed);
        } else {
            self.total_tricks = self.total_tricks.saturating_add(1);
            if self.line_on { self.current_line_tricks = self.current_line_tricks.wrapping_add(1); }
            self.current_tricks = self.current_tricks.wrapping_add(1);
        }
    }

    /// Original PhysOutModule_Hud.Update ordering; one success notification
    /// per ended-list update, NOT one notification per element of that list.
    pub fn consume(&mut self, f: &PhysicsFrame, events: &mut Vec<DisplayEvent>) -> Result<(), &'static str> {
        for command in &f.backend_commands { match *command {
            BackendCommand::SetIsJacked(value)=>self.is_jacked=value,
            BackendCommand::SetCanShowJackedOverlay(value)=>self.can_show_jacked_overlay=value,
            // The single CentralHUD movie consumes this from the same frame;
            // TricksBackEnd has no central-message state.
            BackendCommand::ShowCentralMessage(_)=>{},
        }}
        let new_trick = if let Some(trick) = &f.begun {
            self.sequence_sketchy = false;
            self.converted = false;
            self.active_spin = true;
            Some(trick)
        } else if let Some(trick) = &f.converted {
            self.converted = true;
            Some(trick)
        } else { None };
        if f.sequence_begun {
            self.active_spin = false;
            self.current_tricks = 0;
            self.sequence_on = true;
            self.sequence_time = 0.;
            // UpdateTrickDisplaySequenceStarted82564d50 calls empty825187c0.
        }
        if f.sequence_completed { self.sequence_on = false; }
        self.line_seconds = f.line_seconds;
        self.line_points = f.line_points;
        self.line_max_points = f.line_max_points;
        if f.line_points > 0 {
            if !self.line_on {
                self.current_line_tricks = 0; self.current_tricks = 0; self.line_time = 0.;
            }
            self.line_on = true;
        }
        self.line_score = f.line_score;
        self.multiplier = f.multiplier;
        self.sequence_score = f.hud_score;
        self.momentum_points = f.momentum_points;
        self.air_distance = f.air_distance; self.air_height = f.air_height; self.air_step = f.air_step;
        self.aerial = f.aerial; self.trick_started = new_trick.is_some();
        self.manual_length = f.manual_length; self.grind_distance = f.grind_distance;
        self.powerslide_length = f.powerslide_length;
        self.switched = f.switched; self.fakie = f.fakie;
        if f.aerial { self.air_on = true; }
        if f.air_finished {
            if f.just_landed { self.sequence_sketchy = f.sketchy; }
            self.air_on = false;
            if f.landed_drop > 0. { self.drop_height = f.landed_drop; }
            events.push(DisplayEvent::AirFinished(f.landed_height + f.landed_drop));
        }
        if f.interrupted {
            self.landed(Completion::Wipeout, events);
        } else {
            if let Some(trick) = new_trick {
                self.current = trick.valid.then(|| trick.clone());
                events.push(DisplayEvent::Start);
                if f.sequence_begun { self.sequence_id = self.sequence_id.wrapping_add(1); }
            }
            if f.aerial {
                if !self.active_spin && f.spins != 0 {
                    self.active_spin = true;
                    self.current = None;
                    events.push(DisplayEvent::Start);
                }
                self.spins = f.spins; self.flips = f.flips;
            }
            if f.ended_count > 0 { self.landed(Completion::Success, events); }
            if f.sequence_completed && !f.just_wiped_out {
                self.tricks_in_sequence = f.finished_tricks.checked_sub(f.finished_taps)
                    .ok_or("finished sequence taps exceed trick count")?;
                self.sequence_score = f.finished_sequence_score;
                self.current_tricks = 0; self.converted = false; self.drop_height = 0.;
                self.active_spin = false;
                events.push(DisplayEvent::EndSequence);
            }
        }
        if f.just_wiped_out { self.wiped_out = true; }
        if f.reactivated { events.push(DisplayEvent::ReadyToScore); }
        Ok(())
    }

    /// Update82564a88 uses current simulation rate, not rendered frame delta.
    /// Context-label timers have their own millisecond clock in the frontend.
    pub fn update_timers(&mut self, simulation_rate: f32, paused: bool) {
        if paused { return; }
        if self.air_on { self.air_time += simulation_rate; }
        if self.line_on {
            if self.line_points > 0 { self.line_time += simulation_rate; }
            else { self.line_on = false; }
        }
        if self.sequence_on { self.sequence_time += simulation_rate; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn trick() -> TrickName { TrickName { index: 12, valid: true, ..Default::default() } }
    #[test]
    fn landing_uses_finished_score_and_one_success_for_the_entire_ended_list() {
        let mut b = TricksBackend::default(); let mut e = Vec::new();
        b.consume(&PhysicsFrame { begun: Some(trick()), sequence_begun: true,
            hud_score: 75, ..Default::default() }, &mut e).unwrap();
        e.clear();
        b.consume(&PhysicsFrame { ended_count: 3, sequence_completed: true,
            finished_sequence_score: 90, finished_tricks: 3, finished_taps: 1,
            ..Default::default() }, &mut e).unwrap();
        assert_eq!((b.sequence_score, b.total_tricks, b.tricks_in_sequence), (90, 1, 2));
        assert_eq!(e, [DisplayEvent::EndSequence]);
    }
    #[test]
    fn wipeout_does_not_activate_successful_sequence_animation() {
        let mut b = TricksBackend::default(); b.current = Some(trick()); let mut e = Vec::new();
        b.consume(&PhysicsFrame { interrupted: true, just_wiped_out: true,
            sequence_completed: true, finished_sequence_score: 500,
            ..Default::default() }, &mut e).unwrap();
        assert!(b.wiped_out); assert_eq!(b.total_tricks, 0);
        assert_eq!(e, [DisplayEvent::Landed]);
    }
    #[test]
    fn clear_data_preserves_distinct_reset_and_meter_fields() {
        let mut b = TricksBackend::default();
        b.wiped_out = true; b.sequence_on = true; b.line_points = 312;
        b.line_max_points = 400; b.multiplier = 3.; b.sequence_score = 50;
        b.clear_data();
        assert!(b.wiped_out && b.sequence_on);
        assert_eq!((b.line_points, b.line_max_points), (312,400));
        assert_eq!((b.sequence_score,b.multiplier), (0,0.));
    }
}
