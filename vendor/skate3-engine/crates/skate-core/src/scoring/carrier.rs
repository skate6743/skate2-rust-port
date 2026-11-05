//! Scorable carrier lifecycle, TU3 82DA45C8/46D0/5DE0/5F98.
//! Collectors supply the native clock and already-conditioned point factor.
use super::Scorable;

/// 82DA45C8 multiplies the sum of authored and collector delay by the
/// binary32 constant at 8303745C (0x42700000), then truncates to integer.
pub fn delay_ticks(authored_seconds: f32, extra_seconds: f32) -> u32 {
    (((authored_seconds + extra_seconds) * f32::from_bits(0x42700000)) as i64) as u32
}

#[derive(Clone, Debug)]
pub struct Carrier {
    pub scorable: Scorable,
    pub points: i32,
    pub factor: f32,
    pub reward: f32,
    pub announcement_threshold: f32,
    pub start_tick: u32,
    pub delay_ticks: u32,
    pub announced: bool,
    pub completed: bool,
    pub unannounced: bool,
    pub switch: bool,
    pub fakie: bool,
}

impl Carrier {
    /// Delay is supplied in native ticks: simulation frame count must not be
    /// substituted for the source clock's frequency.
    pub fn new(
        scorable: Scorable,
        points: i32,
        factor: f32,
        announcement_threshold: f32,
        start_tick: u32,
        delay_ticks: u32,
        switch: bool,
        fakie: bool,
    ) -> Self {
        Self {
            scorable,
            points,
            factor,
            reward: 0.0,
            announcement_threshold: announcement_threshold * factor,
            start_tick,
            delay_ticks,
            announced: false,
            completed: false,
            unannounced: false,
            switch,
            fakie,
        }
    }

    fn credit(&mut self, unannounced_factor: f32) {
        let mut reward = self.points as f32 * self.factor;
        if self.unannounced {
            reward *= unannounced_factor;
        }
        self.reward += reward;
    }

    /// Returns true only on the announcement edge. Unsigned subtraction is
    /// intentional and preserves the executable's behavior at clock rollover.
    pub fn announce(&mut self, tick: u32, unannounced_factor: f32) -> bool {
        if self.announced
            || !self.scorable.valid()
            || tick.wrapping_sub(self.start_tick) < self.delay_ticks
        {
            return false;
        }
        self.announced = true;
        self.credit(unannounced_factor);
        true
    }

    /// Called once by the collector when removing its current carrier. The
    /// executable does not mark an early completion as an announcement.
    pub fn complete(&mut self, unannounced_factor: f32) -> bool {
        if !self.scorable.valid() {
            return false;
        }
        if !self.announced {
            self.unannounced = true;
            self.credit(unannounced_factor);
        }
        self.completed = true;
        true
    }

    /// Replace a carrier through its conversion link (82DA6320). Conversion
    /// reports the previous carrier as completed and credits the replacement
    /// immediately, even before its normal announcement delay.
    pub fn convert_to(&mut self, replacement: &mut Self, unannounced_factor: f32) {
        self.completed = true;
        replacement.announced = true;
        replacement.credit(unannounced_factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn carrier(start: u32) -> Carrier {
        Carrier::new(
            Scorable {
                id: 96,
                class: 3,
                score_type: 2,
            },
            100,
            0.2,
            2.25,
            start,
            4,
            false,
            false,
        )
    }
    #[test]
    fn announcement_clock_wraps_and_does_not_credit_twice() {
        let mut c = carrier(u32::MAX - 2);
        assert!(!c.announce(0, 0.3));
        assert!(c.announce(1, 0.3));
        assert!(!c.announce(2, 0.3));
        assert!(c.complete(0.3));
        assert_eq!(c.reward, 20.0);
        assert!(!c.unannounced);
    }
    #[test]
    fn early_completion_uses_authored_discount_without_announcement() {
        let mut c = carrier(0);
        assert!(c.complete(0.3));
        assert_eq!(c.reward, 6.0);
        assert!(c.completed && c.unannounced && !c.announced);
    }
    #[test]
    fn conversion_announces_replacement_without_completing_it() {
        let mut previous = carrier(0);
        let mut next = carrier(1);
        previous.convert_to(&mut next, 0.3);
        assert!(previous.completed);
        assert_eq!(previous.reward, 0.0);
        assert!(next.announced && !next.completed);
        assert_eq!(next.reward, 20.0);
        assert!(!next.announce(5, 0.3));
    }
}
