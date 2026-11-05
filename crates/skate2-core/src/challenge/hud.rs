//! `Sk8::HUD::HUDManager2` as challenge scripts drive it (`HUD2.*`): named
//! groups of HUD component instances, the active-group stack, the central
//! message and the objective HUD binding. The presentation layer renders this
//! state; the group-loaded notification is `FrontEndManager::OnHUDEvent`
//! (`0x82628818`) → `cMsgHUDLoaded`.

use super::{
    keys::hash_value32,
    dpo_hud::{self, DpoHud},
    leaderboard_hud::{AbdHud, CountdownHud, LeaderboardHud, ThrowDownMessageHud, TimerHud},
    voting::OnlineVotingHud,
};

/// `Sk8::HUD::gHUDComponentNames` (`eHUDComponent` order, `kHUDName`s).
pub const COMPONENT_NAMES: [&str; 25] = [
    "timer",
    "bigblacktimer",
    "central",
    "distance",
    "countdown",
    "objective",
    "minimap",
    "signup",
    "trickdisplay",
    "onlineeventlog",
    "highlightreel",
    "leaderboard",
    "onlinevoting",
    "onlineobservingplayer",
    "onlineotsstat",
    "homscoring",
    "abd",
    "dpo",
    "phototreatment",
    "tutorialinstruction",
    "phonelist",
    "PreFilmSetupHUD",
    "throwdownmessage",
    "online_chyron",
    "freeskateactivity_overlay",
];

/// `Sk8::HUD::GetHUDComponentName`.
#[must_use]
pub fn component_name(component: i32) -> Option<&'static str> {
    usize::try_from(component).ok().and_then(|i| COMPONENT_NAMES.get(i)).copied()
}

/// `HUDInstance`: one component in a group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudInstance {
    pub component: String,
    pub shown: bool,
}

/// `ObjectiveHUDData` (`0xc8` bytes), created with a group that has the
/// `objective` component. `Reset` (`0x82d828e0`) gives the defaults;
/// `HUD2.SetupObjectiveHUDData` (`0x82899810`) binds the challenge and its
/// objective group.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "native layout")]
pub struct ObjectiveHud {
    /// `mChallengeKey`: the activity's unique id.
    pub challenge: u64,
    /// `mObjectiveGroup`: the objectives manager's group.
    pub group: Option<u32>,
    /// `mObjectives`/`mNumObjectives`: the group's displayed objectives.
    pub listed: Vec<usize>,
    pub score: u32,
    pub score_required: u32,
    pub enable_score: bool,
    pub variant_hal_id: String,
    pub dirty: bool,
    pub scoring_dirty: bool,
    pub is_modified_position: bool,
    pub is_frozen: bool,
    pub show_activity_type: bool,
    pub show_activity_name: bool,
    pub show_curr_and_req: bool,
}

impl Default for ObjectiveHud {
    fn default() -> Self {
        Self {
            challenge: 0x1_0000_0000,
            group: None,
            listed: Vec::new(),
            score: 0,
            score_required: 0,
            enable_score: true,
            variant_hal_id: "#".to_owned(),
            dirty: true,
            scoring_dirty: true,
            is_modified_position: false,
            is_frozen: false,
            show_activity_type: false,
            show_activity_name: true,
            show_curr_and_req: true,
        }
    }
}

/// `PhonelistHUDData`: `mShowIcon` of the security, pool drain and uncapping services.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhonelistHud {
    pub dirty: bool,
    pub show_icon: [bool; 3],
}

/// `DistanceMeterData` (ctor `0x826b3a00`), created with a group that has the `distance`
/// component: the meter's distance out of its maximum.
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceMeterHud {
    pub dirty: bool,
    pub distance: f32,
    pub max_distance: f32,
}

impl Default for DistanceMeterHud {
    fn default() -> Self {
        Self { dirty: true, distance: 0.0, max_distance: 100.0 }
    }
}

/// `TutorialInstructionHUDData` (ctor `0x826b1d08` then `Reset` `0x82d82a50`),
/// created with a group that has the `tutorialinstruction` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TutorialInstructionHud {
    /// `mInstruction`: a `RefreshTutorialHUD` case or a trick guide name.
    pub instruction: String,
    /// `mTutorialInstructionHALID`.
    pub hal_id: String,
    pub dirty: bool,
    /// `mIsTutorialDirty`.
    pub tutorial_dirty: bool,
}

impl Default for TutorialInstructionHud {
    fn default() -> Self {
        Self { instruction: "ollie".to_owned(), hal_id: String::new(), dirty: false, tutorial_dirty: false }
    }
}

impl TutorialInstructionHud {
    /// `SetInstruction(attribName, HALString)` (`0x82d82a98`): a change copies
    /// both (127 characters at most) and dirties the data and the tutorial.
    pub fn set_instruction(&mut self, instruction: &str, hal_id: &str) {
        if instruction != self.instruction || hal_id != self.hal_id {
            self.hal_id = hal_id.chars().take(127).collect();
            self.instruction = instruction.chars().take(127).collect();
            self.dirty = true;
            self.tutorial_dirty = true;
        }
    }
}

/// `HUDGroup` (`0x31c` bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct HudGroup {
    pub name: String,
    pub hash: u32,
    pub instances: Vec<HudInstance>,
    pub objective: Option<ObjectiveHud>,
    pub tutorial: Option<TutorialInstructionHud>,
    pub distance: Option<DistanceMeterHud>,
    pub phonelist: Option<PhonelistHud>,
    pub leaderboard: Option<LeaderboardHud>,
    pub abd: Option<AbdHud>,
    pub timer: Option<TimerHud>,
    /// BigBlackTimerHUDData::Reset826b1a40 initializes its timer ID to -1.
    pub big_black_timer:Option<i32>,
    pub countdown: Option<CountdownHud>,
    pub voting: Option<OnlineVotingHud>,
    pub dpo: Option<DpoHud>,
    pub throwdown: Option<ThrowDownMessageHud>,
    /// `mJustFinishedLoading`: the next front-end update reports the group.
    just_finished_loading: bool,
}

/// `CentralHUD::ShowCentralMessage` arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CentralMessage {
    pub hal_id: String,
    pub colour: i32,
    pub args: [String; 4],
    pub audio_hook: i32,
}

/// A change to `CentralHUD`'s modified line, the one under the central message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModifiedCentral {
    /// `CentralHUD::ShowModifiedCentralMessage` (`0x826bc050`): the line, held up until it is
    /// cleared when `stay_up` (Skate 3's seventh argument, TU3 `0x826640f0`'s
    /// `mIsModMessageForcedToStayUp`), else for 180 updates.
    Show { message: CentralMessage, stay_up: bool },
    /// Skate 3's `HUD2.ClearModifiedCentralMessage` (`0x828508e0`): the hold ends and screen
    /// state 88 plays the line's outro.
    Clear,
}

#[derive(Debug, Default)]
pub struct HudManager {
    pub groups: Vec<HudGroup>,
    /// `mActiveGroups`, front first.
    pub active: Vec<u32>,
    /// Central messages in the order scripts showed them.
    pub central_messages: Vec<CentralMessage>,
    /// `HUDManager2::Show(false, flag)` visibility flags currently hiding the HUD.
    pub hidden_flags: u32,
    /// `mActiveGroupsChanged`: the next update rebinds the components.
    pub active_groups_changed: bool,
    /// "main"'s `TrickDisplayHUDData::mContextTrickVisible`.
    pub context_trick_display: ContextTrickDisplay,
}

/// `TrickDisplayHUDData::mContextTrickVisible`, which the constructor (`0x826b1b60`) and
/// `Reset` (`0x826b1bc8`) set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextTrickDisplay(pub bool);

impl Default for ContextTrickDisplay {
    fn default() -> Self {
        Self(true)
    }
}

/// `eHUDVisibilityFlag_ChallengeEffects`.
pub const VISIBILITY_CHALLENGE_EFFECTS: u32 = 8;

/// `FrontEndManager::StartInGame`'s (`0x82640450`) group of the free-skate HUD.
pub const MAIN: &str = "main";

/// The instance `HUDManager2::FixupActiveComponents` (`0x82cedff0`) binds a component to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Binding {
    /// "main"'s, which free skate's own code shows and hides.
    #[default]
    Main,
    /// A challenge group's, with its `mVisible`.
    Group(bool),
    /// None: no active group lists the component, or a visibility flag hides the HUD.
    Unbound,
}

/// What the free-skate HUD components are bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudBinding {
    /// `trickdisplay`: its bound instance's visibility, none while unbound, when
    /// `HUDManager2::Update` (0x82cee4f0) neither shows nor updates it.
    pub trick_display: Option<bool>,
    pub hom_scoring: Binding,
    pub minimap: Binding,
    /// `HUDManager2::mVisibleFlags` is not zero: a `Show(false, flag)` hides the whole HUD.
    pub hidden: bool,
}

impl Default for HudBinding {
    /// "main" alone, with the components `StartInGame` shows.
    fn default() -> Self {
        Self { trick_display: Some(true), hom_scoring: Binding::Main, minimap: Binding::Main, hidden: false }
    }
}

impl HudManager {
    fn index(&self, name: &str) -> Option<usize> {
        let hash = hash_value32(name);
        self.groups.iter().position(|g| g.hash == hash)
    }

    #[must_use]
    pub fn group(&self, name: &str) -> Option<&HudGroup> {
        self.index(name).map(|i| &self.groups[i])
    }

    pub fn group_mut(&mut self, name: &str) -> Option<&mut HudGroup> {
        self.index(name).map(|i| &mut self.groups[i])
    }

    /// `HUDManager2::CreateGroup(name, components)` (`0x82cedf98`). The
    /// component movies are resident in the port, so each component counts as
    /// loaded when added (`AddComponentToGroup` → `OnComponentLoaded`) and the
    /// group reports on the next front-end update.
    pub fn create_group(&mut self, name: &str, components: &[&str]) {
        let hash = hash_value32(name);
        if self.index(name).is_some() {
            return;
        }
        let instances = components.iter().map(|c| HudInstance { component: (*c).to_owned(), shown: false }).collect();
        self.groups.push(HudGroup {
            name: name.to_owned(),
            hash,
            instances,
            objective: components.contains(&"objective").then(ObjectiveHud::default),
            tutorial: components.contains(&super::tutorial_hud::COMPONENT).then(TutorialInstructionHud::default),
            distance: components.contains(&"distance").then(DistanceMeterHud::default),
            phonelist: components.contains(&"phonelist").then(PhonelistHud::default),
            leaderboard: components.contains(&"leaderboard").then(LeaderboardHud::default),
            abd: components.contains(&"abd").then(AbdHud::default),
            timer: components.contains(&"timer").then(TimerHud::default),
            big_black_timer:components.contains(&"bigblacktimer").then_some(-1),
            countdown: components.contains(&"countdown").then(CountdownHud::default),
            voting: components.contains(&super::voting::COMPONENT).then(OnlineVotingHud::default),
            dpo: components.contains(&dpo_hud::COMPONENT).then(DpoHud::default),
            throwdown: components.contains(&"throwdownmessage").then(ThrowDownMessageHud::default),
            just_finished_loading: !components.is_empty(),
        });
    }

    /// `HUDManager2::DoesGroupExist` (`0x82cec8f0`).
    #[must_use]
    pub fn group_exists(&self, name: &str) -> bool {
        self.index(name).is_some()
    }

    /// `HUDManager2::CheckIfComponentAddedToGroup` (`0x82ced128`).
    #[must_use]
    pub fn has_component(&self, group: &str, component: &str) -> bool {
        self.group(group).is_some_and(|g| g.instances.iter().any(|i| i.component == component))
    }

    /// `HUDManager2::AddComponentToGroup` (`0x82cedc48`): the instance and its
    /// data; the group reports loaded again on the next front-end update.
    pub fn add_component(&mut self, group: &str, component: &str) {
        let Some(g) = self.group_mut(group) else { return };
        g.instances.push(HudInstance { component: component.to_owned(), shown: false });
        if component == "objective" && g.objective.is_none() {
            g.objective = Some(ObjectiveHud::default());
        }
        if component == super::tutorial_hud::COMPONENT && g.tutorial.is_none() {
            g.tutorial = Some(TutorialInstructionHud::default());
        }
        match component {
            "distance" if g.distance.is_none() => g.distance = Some(DistanceMeterHud::default()),
            "phonelist" if g.phonelist.is_none() => g.phonelist = Some(PhonelistHud::default()),
            "leaderboard" if g.leaderboard.is_none() => g.leaderboard = Some(LeaderboardHud::default()),
            "abd" if g.abd.is_none() => g.abd = Some(AbdHud::default()),
            "timer" if g.timer.is_none() => g.timer = Some(TimerHud::default()),
            "bigblacktimer" if g.big_black_timer.is_none()=>g.big_black_timer=Some(-1),
            "countdown" if g.countdown.is_none() => g.countdown = Some(CountdownHud::default()),
            super::voting::COMPONENT if g.voting.is_none() => g.voting = Some(OnlineVotingHud::default()),
            dpo_hud::COMPONENT if g.dpo.is_none() => g.dpo = Some(DpoHud::default()),
            "throwdownmessage" if g.throwdown.is_none() => g.throwdown = Some(ThrowDownMessageHud::default()),
            _ => {}
        }
        g.just_finished_loading = true;
    }

    /// `HUDManager2::AddGroupToActive` (`0x82cecdd8`).
    pub fn add_group_to_active(&mut self, name: &str) {
        let Some(index) = self.index(name) else { return };
        let hash = self.groups[index].hash;
        if self.active.first() == Some(&hash) {
            return;
        }
        self.active.retain(|&h| h != hash);
        self.active.insert(0, hash);
        self.active_groups_changed = true;
    }

    /// `HUDManager2::RemoveGroupFromActive` (`0x82cec2d0`).
    pub fn remove_group_from_active(&mut self, name: &str) {
        let hash = hash_value32(name);
        if self.active.contains(&hash) {
            self.active.retain(|&h| h != hash);
            self.active_groups_changed = true;
        }
    }

    /// `HUDManager2::DeleteGroup` (`0x82cee6e0`): `RemoveGroupFromActive`
    /// (`0x82cec2d0`) first.
    pub fn delete_group(&mut self, name: &str) {
        self.delete_group_hash(hash_value32(name));
    }

    /// `DeleteGroup` of the group with this name hash.
    pub fn delete_group_hash(&mut self, hash: u32) {
        if self.active.contains(&hash) {
            self.active.retain(|&h| h != hash);
            self.active_groups_changed = true;
        }
        self.groups.retain(|g| g.hash != hash);
    }

    /// `HUDInstance::Show` on a group's component.
    pub fn show(&mut self, group: &str, component: &str, show: bool) {
        if let Some(g) = self.group_mut(group)
            && let Some(instance) = g.instances.iter_mut().find(|i| i.component == component)
        {
            instance.shown = show;
        }
    }

    /// `HUDManager2::Show(show, flag)` (`0x82cebb30`): the components are
    /// rebound when `show` differs from whether no flag hid the HUD before.
    pub fn set_visible(&mut self, show: bool, flag: u32) {
        let was_visible = self.hidden_flags == 0;
        if show {
            self.hidden_flags &= !flag;
        } else {
            self.hidden_flags |= flag;
        }
        if was_visible != show {
            self.active_groups_changed = true;
        }
    }

    /// `FixupActiveComponents` (`0x82cedff0`) for one component: the front-most active group
    /// listing it binds its instance, unless a `Show(false, flag)` hides the whole HUD.
    #[must_use]
    pub fn binding(&self, component: &str) -> Binding {
        if self.hidden_flags != 0 {
            return Binding::Unbound;
        }
        for hash in &self.active {
            let Some(group) = self.groups.iter().find(|g| g.hash == *hash) else { continue };
            if let Some(instance) = group.instances.iter().find(|i| i.component == component) {
                return if group.name == MAIN { Binding::Main } else { Binding::Group(instance.shown) };
            }
        }
        Binding::Unbound
    }

    /// The free-skate components' bindings; "main"'s trick display is this manager's.
    #[must_use]
    pub fn hud_binding(&self) -> HudBinding {
        let trick_display = match self.binding("trickdisplay") {
            Binding::Main => Some(
                self.group(MAIN)
                    .and_then(|g| g.instances.iter().find(|i| i.component == "trickdisplay"))
                    .is_some_and(|i| i.shown),
            ),
            Binding::Group(shown) => Some(shown),
            Binding::Unbound => None,
        };
        HudBinding {
            trick_display,
            hom_scoring: self.binding("homscoring"),
            minimap: self.binding("minimap"),
            hidden: self.hidden_flags != 0,
        }
    }

    /// The front-end update half of `HUDManager2::Update`: groups that just
    /// finished loading, reported once (`eHUDEvent_GroupLoaded`).
    pub fn take_loaded_groups(&mut self) -> Vec<String> {
        let mut loaded = Vec::new();
        for group in &mut self.groups {
            if group.just_finished_loading {
                group.just_finished_loading = false;
                loaded.push(group.name.clone());
            }
        }
        loaded
    }
}
