//! `Simulation` slow-motion rate: `OnRequestSetSimRate`8289e350, the
//! `SimUpdate2` countdown8289fd40..fd78 and the ExecutionMan timer rate.
//! Per-tick physics keeps its fixed `kUpdateTime`; only the timer that wakes
//! the game loop changes, so this owner paces ticks and nothing else.
use crate::simulation_assembly_backend::constraint_session::world_services::{
    PhysicalGameInfo, PhysicalWorldServicesError,
};
use std::sync::Arc;

/// `kUpdateTime_48`82158ec0.
pub(crate) const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);
/// `SimUpdate2`8289fd5c passes 0x3c to `SetTimerHz` on the hard reset.
pub(crate) const BASE_TIMER_HZ: i32 = 60;

/// `cMsgSetSimRate` (kind 0x3fbf5e43, size 0x18): +0x10 fSimTime, +0x14 ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SetSimRate {
    pub(crate) sim_time: f32,
    pub(crate) ticks_of_slow_motion: i32,
}

#[derive(Debug)]
pub(crate) enum SimRateError {
    Queue(String),
    MissingGameInfo,
    GameInfo(PhysicalWorldServicesError),
    /// 8289e3bc reads the int cvar `kMaxTicksForSlowMotion`; no port sender
    /// posts a non-default rate with a non-positive tick count.
    PendingMaxTicksForSlowMotion,
    /// `fctiwz`8289e408 outside the finite i32 range; the exceptional result
    /// is not modeled.
    TimerHzConversion {
        bits: u32,
    },
    /// `SetTimerHz`82985ec0 stores the rate, then traps on zero; negative and
    /// sub-100ns periods are not modeled.
    TimerHz(i32),
}
impl std::fmt::Display for SimRateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Simulation sim rate: {self:?}")
    }
}

pub(crate) struct SimulationSimRate {
    game_info: Option<Arc<PhysicalGameInfo>>,
    /// The port's online game, which `Simulation::IsNetworkGame` answers for the rate and which
    /// solves with the network game's passes (`kMaxOnlineIterations`). The port leaves
    /// `sGameInfo.isNetworkGame` clear online (its other network-game branches are the original
    /// lockstep's), so the online session sets this in its place.
    online_game: bool,
    /// Simulation+0x84 `mCurrentSimUpdateRate`.
    current_sim_update_rate: f32,
    /// Simulation+0x88 `mTicksUntilHardSimRateReset`.
    ticks_until_hard_sim_rate_reset: i32,
    /// `XBox2ExecutionMan`+0x20 `TimerHz`.
    timer_hz: i32,
    /// Standalone unbound world messages. Selected sessions bind the sole
    /// GameSim route before senders run; this vector then remains empty.
    queue: Vec<SetSimRate>,
    game_sim_events: Option<Arc<crate::hermes_game_sim_queue::GameSimEventRoute>>,
}
impl SimulationSimRate {
    /// Simulation ctor828a8764..876c: rate `kUpdateTime`, countdown 0.
    pub(crate) fn construct() -> Self {
        Self {
            game_info: None,
            online_game: false,
            current_sim_update_rate: UPDATE_TIME,
            ticks_until_hard_sim_rate_reset: 0,
            timer_hz: BASE_TIMER_HZ,
            queue: Vec::new(),
            game_sim_events: None,
        }
    }
    /// The same `CommonBackEnd::sGameInfo` owner the World services retain.
    pub(crate) fn bind_game_info(&mut self, game_info: Arc<PhysicalGameInfo>) {
        self.game_info = Some(game_info);
    }
    /// `StateOnlineGame::SetupInitalGameInfo` sets `isNetworkGame` (0x8275dba8) and
    /// `GameModule::BackToSP` clears it (0x8273dac8).
    pub(crate) fn set_online_game(&mut self, online: bool) {
        self.online_game = online;
    }
    /// The port's online game in play.
    pub(crate) fn online_game(&self) -> bool {
        self.online_game
    }
    /// `Simulation::IsNetworkGame`8289e448; `SkaterManager::IsInAnOnlineGame`
    /// (ISkaterManager+0x2c, folded 825f1bb8) calls the same Simulation slot. In a network game
    /// `OnRequestSetSimRate` keeps 1/60 and `SlowMotionController` posts nothing: no slow
    /// motion online.
    pub(crate) fn try_is_network_game(&self) -> Result<bool, SimRateError> {
        if self.online_game {
            return Ok(true);
        }
        self.game_info
            .as_ref()
            .ok_or(SimRateError::MissingGameInfo)?
            .try_is_network_game()
            .map_err(SimRateError::GameInfo)
    }
    /// `ThreadSafeSinglePortBroadcaster::SendMsg`82655840 only enqueues.
    pub(crate) fn send(&mut self, message: SetSimRate) -> Result<(),SimRateError> {
        if let Some(events)=&self.game_sim_events {
            events.send(crate::hermes_game_sim_queue::LivingWorldMessage::SetSimRate {
                sim_time:message.sim_time.to_bits(),ticks:message.ticks_of_slow_motion,
            }).map_err(SimRateError::Queue)
        } else {self.queue.push(message);Ok(())}
    }
    /// Selected Simulation construction binds its single queue4 before any
    /// GameSim producer runs. Unbound standalone worlds retain their harness.
    pub(crate) fn bind_game_sim_events(&mut self,events:Arc<crate::hermes_game_sim_queue::GameSimEventRoute>) -> Result<(),SimRateError> {
        if self.game_sim_events.is_some(){return Err(SimRateError::Queue("GameSim route already bound".into()));}
        if !self.queue.is_empty(){return Err(SimRateError::Queue("GameSim binding must precede rate messages".into()));}
        self.game_sim_events=Some(events);Ok(())
    }
    pub(crate) fn game_sim_events(&self)->Option<Arc<crate::hermes_game_sim_queue::GameSimEventRoute>> {
        self.game_sim_events.as_ref().map(Arc::clone)
    }
    /// `SimController::Update1` Dispatch828a739c: before NIS, commands and
    /// `SimUpdate1`, every queued message reaches `OnRequestSetSimRate` in order.
    pub(crate) fn try_dispatch(&mut self) -> Result<(), SimRateError> {
        if self.game_sim_events.is_some() {
            return Err(SimRateError::Queue("bound GameSim dispatch requires full registered subscriber owners".into()));
        }
        for message in std::mem::take(&mut self.queue) {
            self.try_on_request_set_sim_rate(message)?;
        }
        Ok(())
    }
    pub(crate) fn try_on_request_set_sim_rate(&mut self, message: SetSimRate) -> Result<(), SimRateError> {
        let rate = if self.try_is_network_game()? {
            UPDATE_TIME
        } else {
            message.sim_time
        };
        self.current_sim_update_rate = rate;
        // fcmpu/beq8289e3a4: only an ordered equal rate clears the countdown.
        if rate == UPDATE_TIME {
            self.ticks_until_hard_sim_rate_reset = 0;
        } else if message.ticks_of_slow_motion > 0 {
            self.ticks_until_hard_sim_rate_reset = message.ticks_of_slow_motion.wrapping_add(1);
        } else {
            return Err(SimRateError::PendingMaxTicksForSlowMotion);
        }
        // 8289e3ec..e408: fdivs, fadds 0.5, then fctiwz.
        let hz = 1.0 / rate + 0.5;
        if !(hz > -2_147_483_649.0 && hz < 2_147_483_648.0) {
            return Err(SimRateError::TimerHzConversion { bits: hz.to_bits() });
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "fctiwz truncates an in-range finite value toward zero"
        )]
        let hz = hz as i32;
        self.set_timer_hz(hz)
    }
    fn set_timer_hz(&mut self, hz: i32) -> Result<(), SimRateError> {
        self.timer_hz = hz;
        if hz <= 0 || hz > 10_000_000 {
            return Err(SimRateError::TimerHz(hz));
        }
        Ok(())
    }
    /// `SimUpdate2`8289fd40..fd78, after `SkaterManager::PostUpdate`, only when
    /// updateMask bit0 is set: `addic.` decrement, hard reset on reaching zero.
    pub(crate) fn try_count_down(&mut self) -> Result<(), SimRateError> {
        self.ticks_until_hard_sim_rate_reset = self.ticks_until_hard_sim_rate_reset.wrapping_sub(1);
        if self.ticks_until_hard_sim_rate_reset == 0 {
            self.current_sim_update_rate = UPDATE_TIME;
            self.set_timer_hz(BASE_TIMER_HZ)?;
        }
        Ok(())
    }
    /// `TimerHz`; `SetTimerHz` derives TimerPeriod = 10,000,000 / Hz (100 ns).
    pub(crate) fn timer_hz(&self) -> i32 {
        self.timer_hz
    }
    pub(crate) fn current_sim_update_rate(&self) -> f32 {
        self.current_sim_update_rate
    }
    #[cfg(test)]
    pub(crate) fn ticks_until_hard_sim_rate_reset(&self) -> i32 {
        self.ticks_until_hard_sim_rate_reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offline() -> SimulationSimRate {
        let mut rate = SimulationSimRate::construct();
        rate.bind_game_info(PhysicalGameInfo::construct_network_field());
        rate
    }

    #[test]
    fn queued_rate_waits_for_dispatch_then_survives_one_countdown() {
        let mut rate = offline();
        rate.send(SetSimRate {
            sim_time: 1.0 / 30.0,
            ticks_of_slow_motion: 1,
        }).unwrap();
        assert_eq!(rate.timer_hz(), 60);
        rate.try_dispatch().unwrap();
        assert_eq!(rate.timer_hz(), 30);
        assert_eq!(rate.ticks_until_hard_sim_rate_reset(), 2);
        rate.try_count_down().unwrap();
        assert_eq!(rate.timer_hz(), 30);
        rate.try_count_down().unwrap();
        assert_eq!(rate.timer_hz(), 60);
        assert_eq!(
            rate.current_sim_update_rate().to_bits(),
            UPDATE_TIME.to_bits()
        );
    }

    #[test]
    fn default_rate_clears_countdown_and_idle_countdown_never_resets() {
        let mut rate = offline();
        rate.send(SetSimRate {
            sim_time: UPDATE_TIME,
            ticks_of_slow_motion: 0,
        }).unwrap();
        rate.try_dispatch().unwrap();
        assert_eq!(rate.ticks_until_hard_sim_rate_reset(), 0);
        rate.try_count_down().unwrap();
        assert_eq!(rate.ticks_until_hard_sim_rate_reset(), -1);
        assert_eq!(rate.timer_hz(), 60);
    }

    #[test]
    fn non_default_rate_without_ticks_stops_at_the_cvar_read() {
        let mut rate = offline();
        rate.send(SetSimRate {
            sim_time: 1.0 / 30.0,
            ticks_of_slow_motion: 0,
        }).unwrap();
        assert!(matches!(
            rate.try_dispatch(),
            Err(SimRateError::PendingMaxTicksForSlowMotion)
        ));
        assert_eq!(
            rate.current_sim_update_rate().to_bits(),
            (1.0_f32 / 30.0).to_bits()
        );
    }

    #[test]
    fn an_online_game_keeps_the_normal_rate() {
        let mut rate = offline();
        rate.set_online_game(true);
        assert!(rate.try_is_network_game().unwrap());
        rate.send(SetSimRate {
            sim_time: 1.0 / 30.0,
            ticks_of_slow_motion: 1,
        }).unwrap();
        rate.try_dispatch().unwrap();
        assert_eq!(rate.timer_hz(), 60);
        assert_eq!(
            rate.current_sim_update_rate().to_bits(),
            UPDATE_TIME.to_bits()
        );
        rate.set_online_game(false);
        assert!(!rate.try_is_network_game().unwrap());
    }
}
