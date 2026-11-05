//! UpdateRidingFakie82BB2330. The two persistent clocks belong to the graph
//! instance, so re-entering another leaf does not restart them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub high_speed: f32,
    pub low_speed: f32,
    pub slowly_backwards_seconds: f32,
    pub after_teleport_seconds: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    pub category: u32,
    pub grind_state: u32,
    pub doing_trick: bool,
    /// PhysOutSkeleton+0, as produced by Skeleton::GetData.
    pub board_axis: [f32; 4],
    pub deck_velocity: [f32; 4],
    /// PhysOut bundle+36, used outside filtered Ground category.
    pub external_velocity: [f32; 4],
    pub ground_projected_speed: f32,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    slowly_backwards: f32,
    after_teleport: f32,
}
impl State {
    /// None preserves the existing fakie flag during grounded/air trick execution.
    pub fn update(&mut self, physical: Physical, dt: f32, settings: Settings) -> Option<bool> {
        let eligible = if physical.category == 5 {
            self.after_teleport = 0.0;
            false
        } else {
            let eligible = self.after_teleport > settings.after_teleport_seconds;
            self.after_teleport += dt;
            eligible
        };
        if !eligible {
            self.slowly_backwards = 0.0;
            return Some(false);
        }
        let allowed = (matches!(physical.category, 1 | 2)
            || physical.category == 6 && physical.grind_state == 503)
            && !physical.doing_trick;
        if !allowed {
            self.slowly_backwards = 0.0;
            return (!matches!(physical.category, 1 | 2)).then_some(false);
        }
        let velocity = if physical.category == 1 {
            physical.deck_velocity
        } else {
            physical.external_velocity
        };
        let projection = crate::physics::native_arithmetic::dot3(velocity, physical.board_axis);
        if physical.ground_projected_speed > settings.high_speed && projection < -0.5 {
            self.slowly_backwards = 0.0;
            Some(true)
        } else if physical.ground_projected_speed > settings.low_speed && projection < -0.5 {
            self.slowly_backwards = dt + self.slowly_backwards;
            Some(self.slowly_backwards > settings.slowly_backwards_seconds)
        } else {
            self.slowly_backwards = 0.0;
            Some(false)
        }
    }
}
