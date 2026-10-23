//! `FrontEndState_SkateProfile` (FE state 45) as the `StartHub`'s skate.Profile row opens it
//! (`IStartHubState::OnMenuSelect` `0x82674f28`: `mEntryScreenMode = eScreenMode_Offline`, the
//! local peer, `PushState(45)`). `OnEnter` (`0x8266cb70`) keeps the offline mode
//! (`SetIsOnlineMode(false)` `0x826624d8`) and opens screen 93, `onlinehub/skateProfile`, whose
//! constructor sets the offline state; `OnLoaded` (`0x82662000`) is `SetScreenState(10101)`.
//! `ReceiveInput` (`0x826621d0`) pops the state for B with `FE_SOUND_CORE_B_BUTTON`; keys 6 and 9
//! open the Xbox LIVE gamercard and achievements, which the PC has no service for. Every other key
//! reaches the screen (`gFEStateInfo` row 45's `mbInputToActionscript`): `skateProfile.UpdateInput`
//! moves between the Stats and Personal Bests tabs as the triggers are released, and the list
//! (`dynamic_selectable_list.UpdateInput`) steps its row on up and down.

use super::pause_menu::PauseMenuInput;

/// How `StatsBackEnd::GetStatsValue` (`0x82568270`) and `GetPersonalBestsValue` (`0x8255f1d8`)
/// write a row's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StatValue {
    /// `secondsToFormattedTime` (`0x82a730a8`), `FormatTime(.., true, 0)`, a space and the packed
    /// `ID_COMMON_METRIC_TIME`.
    Time { hours: i32, minutes: i32, seconds: i32, hundredths: i32 },
    /// `FormatNumber(value, true, 0)`.
    Number(f32),
    /// `FormatCurrency(value, true)`.
    Currency(f32),
    /// `FormatString("ID_LOGBOOK_STATS_PERCENT_INT", value)`.
    Percent(f32),
    /// `GetMetricText("ID_COMMON_DISTANCE", value)` (`0x82a73050`): `%d` of the value.
    DistanceInt(i32),
    /// The float `GetMetricText` (`0x82a72ff0`): `%.1f` of the value.
    DistanceFloat(f32),
    /// `FormatString(label)`.
    Label(&'static str),
    /// The label as it is, which the text field localizes.
    Hal(&'static str),
}

impl StatValue {
    /// `secondsToFormattedTime`: the hours and minutes through `1/3600` and `1/60` (the minutes
    /// with one `fmsubs`), the seconds left of both, and the hundredths of the fraction.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "the native fcfid")]
    pub fn time(time: f32) -> Self {
        let whole = fctiwz(time);
        let hours = fctiwz(time * f32::from_bits(0x3991_a2b4));
        let minutes = fctiwz(time.mul_add(f32::from_bits(0x3c88_8889), -(hours.wrapping_mul(60) as f32)));
        let seconds = fctiwz((time - hours.wrapping_mul(3600) as f32) - minutes.wrapping_mul(60) as f32);
        let hundredths = fctiwz((time - whole as f32) * 100.0);
        Self::Time { hours, minutes, seconds, hundredths }
    }
}

/// `fctiwz`: toward zero and saturating, NaN as `i32::MIN`.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "the cast saturates as fctiwz does")]
pub(crate) fn fctiwz(value: f32) -> i32 {
    if value.is_nan() { i32::MIN } else { value as i32 }
}

/// A list row: `GetStatTitle`/`GetPersonalBestTitle` and the value text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub title: &'static str,
    pub value: StatValue,
}

/// The rows the screen's tabs list, and the total its untabbed panel writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Lists {
    pub stats: Vec<Row>,
    pub personal_bests: Vec<Row>,
    /// `Stats.GetTotalCashEarned` (`GetStatsValue(10)`), which `PopulateUntabbedData` writes.
    pub total_cash: StatValue,
}

/// `skateProfile.m_iStats` and `m_iPersonal`.
pub const STATS_TAB: i32 = 0;
pub const PERSONAL_TAB: i32 = 1;
/// `skateProfile.m_iMaxTabsOffline`.
const TABS: i32 = 2;

/// What the screen shows.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// Each opening's own screen.
    pub instance: u32,
    /// `skateProfile.m_iCurrentTab`.
    pub tab: i32,
    /// The list's `CurrentValue`.
    pub selected: usize,
    pub lists: Lists,
}

impl View {
    /// The tab's rows.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        if self.tab == STATS_TAB { &self.lists.stats } else { &self.lists.personal_bests }
    }
}

const B_BUTTON: &str = "FE_SOUND_CORE_B_BUTTON";
const TAB: &str = "FE_SOUND_CORE_TAB";
const NAV_UP: &str = "FE_SOUND_CORE_NAV_UP";
const NAV_DOWN: &str = "FE_SOUND_CORE_NAV_DOWN";

/// FE state 45 on the stack, with its screen.
#[derive(Debug, Default)]
pub struct Controller {
    view: Option<View>,
    /// The screen reported `IntroComplete`.
    ready: bool,
    instance: u32,
    sounds: Vec<&'static str>,
}

impl Controller {
    /// `PushState(45)`: a new screen on the Stats tab.
    pub fn open(&mut self, lists: Lists) {
        self.instance = self.instance.wrapping_add(1);
        self.ready = false;
        self.view = Some(View { instance: self.instance, tab: STATS_TAB, selected: 0, lists });
    }

    #[must_use]
    pub const fn visible(&self) -> bool {
        self.view.is_some()
    }

    /// `OnLeave` (`0x82662088`) as `PopAllStates` pops the state: the screen closes.
    pub fn close(&mut self) {
        self.view = None;
        self.ready = false;
    }

    #[must_use]
    pub const fn snapshot(&self) -> Option<&View> {
        self.view.as_ref()
    }

    /// The screen's `IntroComplete`, from which its state takes input.
    pub fn acknowledge_intro(&mut self) {
        if self.view.is_some() {
            self.ready = true;
        }
    }

    /// One front-end frame's input; true when B popped the state.
    pub fn update(&mut self, input: PauseMenuInput, has_focus: bool) -> bool {
        if !has_focus || !self.ready {
            return false;
        }
        let Some(view) = self.view.as_mut() else { return false };
        if input.cancel {
            self.sounds.push(B_BUTTON);
            self.view = None;
            self.ready = false;
            return true;
        }
        // `NextTab` and `PreviousTab` wrap, and `RefreshTabs` repopulates from the first row.
        if input.right_shift_released || input.left_shift_released {
            self.sounds.push(TAB);
            view.tab = if input.right_shift_released { (view.tab + 1) % TABS } else { (view.tab + TABS - 1) % TABS };
            view.selected = 0;
            return false;
        }
        // `NextValue` and `PrevValue` sound even where the row stops.
        let last = view.rows().len().saturating_sub(1);
        if input.up {
            self.sounds.push(NAV_UP);
            view.selected = view.selected.saturating_sub(1);
        } else if input.down {
            self.sounds.push(NAV_DOWN);
            view.selected = (view.selected + 1).min(last);
        }
        false
    }

    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use super::{Controller, Lists, PERSONAL_TAB, Row, STATS_TAB, StatValue};
    use crate::ui::pause_menu::PauseMenuInput;

    fn rows(count: usize) -> Vec<Row> {
        vec![Row { title: "ID_RECORD_STATS_88", value: StatValue::Number(0.0) }; count]
    }

    #[test]
    fn the_time_splits_as_the_backend_does() {
        assert_eq!(StatValue::time(3725.5), StatValue::Time { hours: 1, minutes: 2, seconds: 5, hundredths: 50 });
        assert_eq!(StatValue::time(59.75), StatValue::Time { hours: 0, minutes: 0, seconds: 59, hundredths: 75 });
        let nan = i32::MIN;
        assert_eq!(StatValue::time(f32::NAN), StatValue::Time { hours: nan, minutes: nan, seconds: nan, hundredths: nan });
    }

    #[test]
    fn input_waits_for_the_intro_then_tabs_and_rows_move() {
        let mut profile = Controller::default();
        profile.open(Lists { stats: rows(3), personal_bests: rows(2), total_cash: StatValue::Currency(0.0) });
        assert!(!profile.update(PauseMenuInput { cancel: true, ..Default::default() }, true));
        assert!(profile.visible());
        profile.acknowledge_intro();
        profile.update(PauseMenuInput { down: true, ..Default::default() }, true);
        profile.update(PauseMenuInput { down: true, ..Default::default() }, true);
        profile.update(PauseMenuInput { down: true, ..Default::default() }, true);
        assert_eq!(profile.snapshot().unwrap().selected, 2);
        profile.update(PauseMenuInput { right_shift_released: true, ..Default::default() }, true);
        let view = profile.snapshot().unwrap();
        assert_eq!((view.tab, view.selected), (PERSONAL_TAB, 0));
        profile.update(PauseMenuInput { right_shift_released: true, ..Default::default() }, true);
        assert_eq!(profile.snapshot().unwrap().tab, STATS_TAB);
        assert_eq!(
            profile.take_sounds(),
            ["FE_SOUND_CORE_NAV_DOWN", "FE_SOUND_CORE_NAV_DOWN", "FE_SOUND_CORE_NAV_DOWN", "FE_SOUND_CORE_TAB", "FE_SOUND_CORE_TAB"]
        );
        assert!(profile.update(PauseMenuInput { cancel: true, ..Default::default() }, true));
        assert!(!profile.visible());
        assert_eq!(profile.take_sounds(), ["FE_SOUND_CORE_B_BUTTON"]);
    }
}
