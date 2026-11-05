//! The Skate 3 physics player's settings (PC port): which Skate 3 parts run and at which tuning.
//! `gameplay_modes::skate3_settings` makes them from Physics Settings; Skate 2 physics is none of
//! them at the Skate 2 tuning.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Difficulty {
    #[default]
    Skate2Default, Easy, Normal, Hardcore, Motorized, EasyMotorized,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuningSource { Skate2, Skate3(&'static str) }
impl Difficulty {
    pub const ALL: [Self; 6] = [Self::Skate2Default, Self::Easy, Self::Normal, Self::Hardcore, Self::Motorized, Self::EasyMotorized];
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self { Self::Skate2Default => "skate2", Self::Easy => "easy", Self::Normal => "normal",
            Self::Hardcore => "hardcore", Self::Motorized => "motorized", Self::EasyMotorized => "easy_motorized" }
    }
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self { Self::Skate2Default => "Skate 2", Self::Easy => "Easy", Self::Normal => "Normal",
            Self::Hardcore => "Hardcore", Self::Motorized => "Motorized", Self::EasyMotorized => "Easy Motorized" }
    }
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        // The first playtest used this label. Keep existing preferences readable.
        if key == "skate2_default" { return Some(Self::Skate2Default); }
        Self::ALL.into_iter().find(|value| value.key() == key)
    }
    #[must_use]
    pub fn step(self, forward: bool) -> Self {
        let index = Self::ALL.iter().position(|&value| value == self).unwrap_or(0);
        Self::ALL[(index + if forward { 1 } else { Self::ALL.len() - 1 }) % Self::ALL.len()]
    }
    #[must_use]
    pub const fn tuning(self) -> TuningSource {
        match self { Self::Skate2Default => TuningSource::Skate2,
            Self::Easy | Self::EasyMotorized => TuningSource::Skate3("easy"),
            Self::Normal => TuningSource::Skate3("normal"), Self::Hardcore => TuningSource::Skate3("hardcore"),
            Self::Motorized => TuningSource::Skate3("motorized") }
    }
    #[must_use]
    pub const fn motor_enabled(self) -> bool { matches!(self, Self::Motorized | Self::EasyMotorized) }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// `skate3`: the Skate 3 player owns the local skater (Physics Settings "Skate 3").
pub struct GameplaySettings { pub skate3: bool, pub difficulty: Difficulty, pub airtime_slow_motion: bool }
impl GameplaySettings {
    pub fn step_difficulty(&mut self, forward: bool) { self.difficulty = self.difficulty.step(forward); }
    #[must_use]
    pub const fn stock_skate2(self) -> bool { !self.skate3 && matches!(self.difficulty, Difficulty::Skate2Default) }
}

/// Authored Skate 3 labels used by the additional tricks in the original HUD.
#[cfg(feature = "skate3-runtime")]
pub fn skate3_trick_strings() -> Result<Vec<(u32, Vec<u16>)>, String> {
    crate::physical_player_phys_out::conditioners::skate3_trick_strings()
}
