//! `PhonelistHUD`826b5b20..826c0940. The marker gameplay already lives in
//! `session_marker`; this component only owns the original frontend commands.

/// Immutable facts from the same completed gameplay tick as the visual pose.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkerHudSnapshot {
    pub visible: bool,
    pub bumper_held: bool,
    pub permissions: Option<MarkerPermissions>,
    pub successful_sets: u64,
    /// PhysOutModule_Hud::UpdateHUDsAndObjectGrabbing (0x827c6ee0).
    pub prop: Option<PropHudSnapshot>,
    pub phone_hidden: bool,
    pub prop_tick: u32,
    /// LB's quick menu offers B Spectate in its options row (PC port addition): an online game's
    /// pre-game lobby where another player can be watched, or one is.
    pub spectate: bool,
}

/// SignUpData::mExtraData and DMO_GetParams (0x826b88c8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropHudSnapshot {
    pub object: u32,
    /// Upright disabled, reset disabled, marker valid, object in marker, maximum reached.
    pub params: [bool; 5],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhoneListState {
    #[default]
    Uninitialized,
    Hidden,
    Minimized,
    Maximized,
}

/// Arguments passed to the original authored `hudphonelist.SetState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ScreenState {
    Minimized = 0,
    MaximizedWithOptions = 1,
    MaximizedMarkerOnly = 2,
    Hidden = 3,
    RefreshMarkerPermissions = 50,
    TrySetMarker = 51,
    TryUseMarker = 52,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkerPermissions {
    pub use_allowed: bool,
    pub set_allowed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerNotification {
    #[default]
    Idle,
    TrySet,
    TryUse,
    Close,
}

/// `PhonelistHUDData`. Notifications replace the pending state; they are not
/// a FIFO. Reset826b3728 clears three service icons and the marker state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneListData {
    pub service_icons: [bool; 3],
    pub notification: MarkerNotification,
    pub dirty: bool,
}
impl Default for PhoneListData {
    fn default() -> Self {
        // PhonelistHUDData constructor826b3ee8 sets IHUDData::mDirty=true.
        Self {
            service_icons: [false; 3],
            notification: MarkerNotification::Idle,
            dirty: true,
        }
    }
}
impl PhoneListData {
    pub fn reset(&mut self) {
        self.service_icons = [false; 3];
        self.notification = MarkerNotification::Idle;
    }
    /// StateManagerFrontEnd8266fbf0: failures only play a sound, no HUD pulse.
    pub fn marker_set_succeeded(&mut self) {
        self.notification = MarkerNotification::TrySet;
        self.dirty = true;
    }
    /// StateManagerFrontEnd8266fcc0 likewise marks data only on success.
    pub fn marker_use_succeeded(&mut self) {
        self.notification = MarkerNotification::TryUse;
        self.dirty = true;
    }
    /// `hudphonelist.UpdateMinimizedMenu`. The service icons the HUD answers fill the two slots
    /// in security, pool drain, uncapping order (`GetServiceIconStatus` 0x826b5f90: the data's
    /// icon, then `CanShowPhoneListAlerts`). Without one, the anywhere film's alert shows, else the
    /// sponsorship's (`GetAnywhereFilmPP` 0x826b6428, `GetSponsorPP` 0x826b6328); without either,
    /// the menu hides.
    #[must_use]
    pub fn minimized_menu(&self, facts: AlertFacts) -> MinimizedMenu {
        const LABELS: [&str; 3] = ["bb", "sammis", "mikes"];
        let mut services = self.service_icons.map(|shown| shown && facts.can_show_alerts);
        if services.contains(&true) {
            return MinimizedMenu::Services(std::array::from_fn(|_| {
                let Some(icon) = services.iter().position(|&shown| shown) else { return "off" };
                services[icon] = false;
                LABELS[icon]
            }));
        }
        if facts.anywhere_film_pinch_point && facts.can_show_alerts {
            MinimizedMenu::Alert("ID_PHONELIST_ANYWHERE_ALERT")
        } else if facts.sponsor_pinch_point && facts.can_show_alerts {
            MinimizedMenu::Alert("ID_PHONELIST_SPONSORSHIP_ALERT")
        } else {
            MinimizedMenu::Hidden
        }
    }
}

/// The game's side of `PhonelistHUD`'s alerts: `CanShowPhoneListAlerts` (0x826b5ea0) and the
/// `Sponsor PinchPoint` and `Anywhere Film PinchPoint` settings `GetSponsorPP` and
/// `GetAnywhereFilmPP` read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlertFacts {
    pub can_show_alerts: bool,
    pub sponsor_pinch_point: bool,
    pub anywhere_film_pinch_point: bool,
}

/// What `UpdateMinimizedMenu` makes of the quick menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimizedMenu {
    /// `minimized`, the alert line hidden, each icon slot at its label (`bb`, `sammis`, `mikes`
    /// or `off`).
    Services([&'static str; 2]),
    /// `minimized`, the alert line showing this text and both icon slots `off`.
    Alert(&'static str),
    /// `hidden`.
    Hidden,
}

/// Side effects go through the same screen/HUD owners as the native component.
/// Rendering, sounds and full phone menus are consumers, not simulated here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudCommand {
    SetScreenState(ScreenState),
    SetPhoneListGroupFilter,
    ClearGroupFilter,
    CentralHudForceOutro,
    PlayOpenSound,
    PlayPhoneOptionsSound,
    PushPhoneOptionsState,
}

/// Answers from native game/marker owners, not keyboard/controller heuristics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudContext {
    pub can_show_marker: bool,
    pub can_show_options: bool,
    /// None when the active module is not a GameModule or has no marker.
    pub marker: Option<MarkerPermissions>,
}

/// `StateManagerFrontEnd::TranslateEHudInputBit`82641648. These are frontend
/// keycodes, not XInput button masks or the simulation input-state indices.
pub fn translate_frontend_input(keycode: i32) -> u32 {
    match keycode {
        2 => 0x10,
        3 => 0x40,
        6 => 0x400,
        9 => INPUT_LEFT_BUMPER,
        16 => 1,
        17 => 2,
        18 => 0x2000,
        19 => 0x1000,
        32 => 0x4000,
        33 => 0x8000,
        46 => 0x20,
        47 => 0x80,
        48 => INPUT_OPTIONS_RELEASED,
        49 => 0x200,
        50 => 0x800,
        53 => INPUT_LEFT_BUMPER_RELEASED,
        _ => 0,
    }
}
pub const INPUT_LEFT_BUMPER: u32 = 4;
pub const INPUT_LEFT_BUMPER_RELEASED: u32 = 8;
pub const INPUT_OPTIONS_RELEASED: u32 = 256;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhoneListHud {
    visible: bool,
    state: PhoneListState,
    permissions: MarkerPermissions,
}
impl PhoneListHud {
    /// Native screen template12: hud2/hudphonelist.swf, 3D HUD focus left5.
    pub const SCREEN_TEMPLATE: u32 = 12;
    pub const SCREEN_LAYER: u32 = 5;
    pub const INPUT_SIGNATURE: u32 = 1020;

    pub fn state(&self) -> PhoneListState {
        self.state
    }
    pub fn visible(&self) -> bool {
        self.visible
    }
    pub fn active(&self) -> bool {
        self.state == PhoneListState::Maximized
    }

    /// Show826c0580 polls SIM state29 only on the show path. The central HUD
    /// outro is conditional on the old visibility and HUDManager's own flags.
    pub fn show(
        &mut self,
        show: bool,
        bumper_held: bool,
        hud_manager_visible: bool,
        context: HudContext,
        output: &mut Vec<HudCommand>,
    ) {
        if show {
            if bumper_held && context.can_show_marker {
                self.maximize(context, output);
            } else {
                self.minimize(output);
            }
        } else {
            if self.active() {
                output.push(HudCommand::ClearGroupFilter);
            }
            output.push(HudCommand::SetScreenState(ScreenState::Hidden));
            self.state = PhoneListState::Hidden;
            if self.visible && hud_manager_visible {
                output.push(HudCommand::CentralHudForceOutro);
            }
        }
        self.visible = show;
    }

    /// UpdateInput826c0848 takes *translated frontend HUD bits*. It compares
    /// complete values, not individual bits of a combined button mask.
    pub fn input(&mut self, input: u32, context: HudContext, output: &mut Vec<HudCommand>) -> bool {
        if !self.visible {
            return false;
        }
        match self.state {
            PhoneListState::Minimized if input == INPUT_LEFT_BUMPER && context.can_show_marker => {
                output.push(HudCommand::PlayOpenSound);
                self.maximize(context, output);
                true
            }
            PhoneListState::Maximized if input == INPUT_LEFT_BUMPER_RELEASED => {
                self.minimize(output);
                true
            }
            PhoneListState::Maximized
                if context.can_show_options && input == INPUT_OPTIONS_RELEASED =>
            {
                output.push(HudCommand::PlayPhoneOptionsSound);
                output.push(HudCommand::PushPhoneOptionsState);
                true
            }
            _ => false,
        }
    }

    /// Update826c06f0 consumes dirty data before refreshing permission changes.
    /// A notification received while minimized is not consumed as a pulse.
    pub fn update(
        &mut self,
        data: Option<&mut PhoneListData>,
        context: HudContext,
        output: &mut Vec<HudCommand>,
    ) {
        if !self.visible {
            return;
        }
        let Some(data) = data else {
            return;
        };
        if data.dirty {
            match self.state {
                PhoneListState::Minimized => self.minimize(output),
                PhoneListState::Maximized => {
                    let screen = match data.notification {
                        MarkerNotification::Idle => None,
                        MarkerNotification::TrySet => Some(ScreenState::TrySetMarker),
                        MarkerNotification::TryUse => Some(ScreenState::TryUseMarker),
                        MarkerNotification::Close => Some(ScreenState::RefreshMarkerPermissions),
                    };
                    if let Some(screen) = screen {
                        data.notification = MarkerNotification::Idle;
                        output.push(HudCommand::SetScreenState(screen));
                    }
                }
                _ => {}
            }
            data.dirty = false;
        }
        if self.active() {
            if let Some(permissions) = context.marker {
                if self.permissions != permissions {
                    self.permissions = permissions;
                    output.push(HudCommand::SetScreenState(
                        ScreenState::RefreshMarkerPermissions,
                    ));
                }
            }
        }
    }

    fn maximize(&mut self, context: HudContext, output: &mut Vec<HudCommand>) {
        if let Some(permissions) = context.marker {
            self.permissions = permissions;
        }
        if matches!(
            self.state,
            PhoneListState::Minimized | PhoneListState::Hidden
        ) {
            output.push(HudCommand::SetPhoneListGroupFilter);
        }
        output.push(HudCommand::SetScreenState(if context.can_show_options {
            ScreenState::MaximizedWithOptions
        } else {
            ScreenState::MaximizedMarkerOnly
        }));
        self.state = PhoneListState::Maximized;
    }
    fn minimize(&mut self, output: &mut Vec<HudCommand>) {
        if self.active() {
            output.push(HudCommand::ClearGroupFilter);
        }
        output.push(HudCommand::SetScreenState(ScreenState::Minimized));
        self.state = PhoneListState::Minimized;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> HudContext {
        HudContext {
            can_show_marker: true,
            can_show_options: false,
            marker: Some(MarkerPermissions {
                use_allowed: false,
                set_allowed: true,
            }),
        }
    }
    #[test]
    fn held_on_show_and_press_release_have_distinct_filter_history() {
        let mut hud = PhoneListHud::default();
        let mut commands = vec![];
        hud.show(true, true, true, context(), &mut commands);
        assert_eq!(
            commands,
            [HudCommand::SetScreenState(ScreenState::MaximizedMarkerOnly)]
        );
        commands.clear();
        assert!(hud.input(8, context(), &mut commands));
        assert_eq!(
            commands,
            [
                HudCommand::ClearGroupFilter,
                HudCommand::SetScreenState(ScreenState::Minimized)
            ]
        );
        commands.clear();
        assert!(!hud.input(4 | 256, context(), &mut commands));
        assert!(hud.input(4, context(), &mut commands));
        assert_eq!(
            commands,
            [
                HudCommand::PlayOpenSound,
                HudCommand::SetPhoneListGroupFilter,
                HudCommand::SetScreenState(ScreenState::MaximizedMarkerOnly)
            ]
        );
    }
    #[test]
    fn placement_pulse_precedes_permission_refresh_and_runs_once() {
        let mut hud = PhoneListHud::default();
        let mut commands = vec![];
        let mut ctx = context();
        hud.show(true, true, true, ctx, &mut commands);
        commands.clear();
        ctx.marker.as_mut().unwrap().use_allowed = true;
        let mut data = PhoneListData::default();
        data.marker_set_succeeded();
        hud.update(Some(&mut data), ctx, &mut commands);
        assert_eq!(
            commands,
            [
                HudCommand::SetScreenState(ScreenState::TrySetMarker),
                HudCommand::SetScreenState(ScreenState::RefreshMarkerPermissions)
            ]
        );
        commands.clear();
        hud.update(Some(&mut data), ctx, &mut commands);
        assert!(commands.is_empty());
        assert_eq!(data.notification, MarkerNotification::Idle);
    }
    #[test]
    fn minimized_menu_fills_its_two_slots_in_service_order_then_shows_an_alert() {
        let all = AlertFacts { can_show_alerts: true, sponsor_pinch_point: true, anywhere_film_pinch_point: true };
        let icons = |service_icons| PhoneListData { service_icons, ..PhoneListData::default() };
        assert_eq!(icons([false, false, true]).minimized_menu(all), MinimizedMenu::Services(["mikes", "off"]));
        assert_eq!(icons([true, false, true]).minimized_menu(all), MinimizedMenu::Services(["bb", "mikes"]));
        assert_eq!(icons([true, true, false]).minimized_menu(all), MinimizedMenu::Services(["bb", "sammis"]));
        // No icon: the anywhere film's alert goes before the sponsorship's.
        assert_eq!(icons([false; 3]).minimized_menu(all), MinimizedMenu::Alert("ID_PHONELIST_ANYWHERE_ALERT"));
        let sponsor = AlertFacts { anywhere_film_pinch_point: false, ..all };
        assert_eq!(icons([false; 3]).minimized_menu(sponsor), MinimizedMenu::Alert("ID_PHONELIST_SPONSORSHIP_ALERT"));
        // CanShowPhoneListAlerts gates icons and alerts alike.
        let gated = AlertFacts { can_show_alerts: false, ..all };
        assert_eq!(icons([true, false, true]).minimized_menu(gated), MinimizedMenu::Hidden);
        assert_eq!(icons([false; 3]).minimized_menu(AlertFacts { can_show_alerts: true, ..AlertFacts::default() }), MinimizedMenu::Hidden);
    }
    #[test]
    fn minimized_and_hidden_preserve_native_notification_lifetime() {
        let mut hud = PhoneListHud::default();
        let mut commands = vec![];
        let mut data = PhoneListData::default();
        data.marker_set_succeeded();
        hud.update(Some(&mut data), context(), &mut commands);
        assert!(data.dirty);
        hud.show(true, false, true, context(), &mut commands);
        commands.clear();
        hud.update(Some(&mut data), context(), &mut commands);
        assert!(!data.dirty);
        assert_eq!(data.notification, MarkerNotification::TrySet);
        assert_eq!(
            commands,
            [HudCommand::SetScreenState(ScreenState::Minimized)]
        );
        commands.clear();
        hud.input(4, context(), &mut commands);
        commands.clear();
        hud.update(Some(&mut data), context(), &mut commands);
        assert!(commands.is_empty());
    }
}
