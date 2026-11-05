use super::*;

impl GrindInputState {
    ///82D8A828/82D8ABD8; preserved current permission/counter arithmetic.
    pub(super) fn permission(&mut self, p: &ProcessedPhysicsInput, air_counter: i32) {
        let state = p.state_2508;
        if state != self.previous_state {
            self.engagement_counter = self.engagement_counter.wrapping_add(match state {
                400 | 402 | 404 => 50,
                403 => 20,
                _ => 0,
            });
            self.previous_state = state;
        }
        self.engagement_counter = decrement(self.engagement_counter);
        self.suppressed = self.engagement_counter > 151;
        self.cooldown = if self.suppressed {
            90
        } else {
            decrement(self.cooldown)
        };
        self.disabled = p.flags_2468 & 0x1800_0000 != 0
            || p.flags_2472 & 0x8008 != 0
            || p.flags_2476 & 0x0040_0000 != 0
            || self.cooldown > 0
            || (p.category_2512 == 200
                && (p.state_timer_2664 <= f32::from_bits(0x3da3_d70a) || air_counter <= 10))
            || p.category_2512 == 500
            || (p.flags_2472 & 4 != 0 && p.category_2512 != 400);
        self.elapsed = if p.category_2512 == 400 || state == 701 {
            self.elapsed + p.timestep_2604
        } else {
            0.
        };
    }

    ///82D875A8; accepted families RETAIN previous proximity. Only fallback
    ///writes the history byte, which is not the valid-candidate flag.
    pub(super) fn advance_history(&mut self, p: &ProcessedPhysicsInput) {
        self.low_wheel_frames = if self.previous_proximity
            && p.state_2508 == 100
            && (p.wheel_count_2556 as i32) < 2
            && p.scalar_2652 < 0.8
        {
            self.low_wheel_frames.wrapping_add(1)
        } else {
            0
        };
        self.grind_history = if p.grind_words_2532_2536[1] == 2 {
            70
        } else {
            decrement(self.grind_history)
        };
        self.secondary_history = decrement(self.secondary_history);
        self.grounded_frames = if p.category_2512 == 100 {
            self.grounded_frames.wrapping_add(1)
        } else {
            0
        };
        self.air_frames = if p.category_2512 == 100 {
            0
        } else {
            self.air_frames.wrapping_add(1)
        };
    }

    ///82D89DC0: use the PRECEDING KnownAir target on ground, then update it.
    pub(super) fn material_mode(
        &mut self,
        p: &ProcessedPhysicsInput,
        nearby: bool,
        targeting: bool,
    ) -> MaterialMode {
        if p.state_2508 == 300 || p.category_2512 == 500 {
            return MaterialMode::Unchanged;
        }
        let grind = (nearby && matches!(p.state_2508, 200 | 201))
            || p.category_2512 == 400
            || p.state_2508 == 701
            || (p.category_2512 == 100 && self.previous_air_target);
        self.previous_air_target = p.state_2508 == 201 && targeting;
        if grind {
            MaterialMode::Grind
        } else {
            MaterialMode::Standard
        }
    }
}

pub(super) fn decrement(value: u32) -> u32 {
    let next = value.wrapping_sub(1);
    if next & 0x8000_0000 != 0 { 0 } else { next }
}
