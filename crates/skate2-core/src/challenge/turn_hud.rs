//! The turn-based challenges' HUD components over their group data, bound and shown as
//! `HUDManager2::Update` (`0x82cee4f0`) does: `LeaderboardHUD`, `ABDHUD`, `TimerHUD`,
//! `CountdownHUD`, `OnlineVotingHUD`, `ThrowDownMessageHUD`, `DPOHUD` and `DistanceMeter`. Each
//! records the screen states its movie runs.

use super::{
    dpo_hud::DpoHud,
    hud::{DistanceMeterHud, HudGroup},
    leaderboard_hud::{AbdHud, CountdownHud, LeaderboardHud, ThrowDownMessageHud, TimerHud},
    system::World,
};

/// `LeaderboardHUD::Update` refreshes at most every 170 ms.
const LEADERBOARD_UPDATE_MS: u32 = 170;
/// `TimerHUD::Update` runs `UpdateTime` every fifth update.
const TIMER_UPDATE_TICKS: u32 = 5;

/// What a component's movie reads while it runs a screen state: the bound group's data
/// (`mData`) then, or the timer's `mTimeBuffer` and `mColour`.
#[derive(Clone, Debug, PartialEq)]
pub enum TurnHudView {
    /// No group is bound.
    None,
    /// Each with `PlayerStance_IsRegular` (`0x826b4bd8`): the local skater's stance. The
    /// leaderboard's `online` is `Mission.GetIsOnlineMission` (`0x826058b0`):
    /// `sGameInfo.isNetworkGame`. `skate3_letters`: see [`TurnHudView::ThrowDown`].
    Leaderboard { data: LeaderboardHud, stance_regular: bool, online: bool, skate3_letters: bool },
    Abd { data: AbdHud, stance_regular: bool },
    Timer(TimerText),
    /// `CountdownHUD::mCountdownVal` (`Countdown_GetCountdownTime` `0x826b8bd8`) and
    /// `Countdown_ChallengeHasAudio` (`0x826b8c50`).
    Countdown { value: i32, has_audio: bool },
    /// `OnlineVotingHUD`'s and `ThrowDownMessageHUD`'s screens read no data.
    Voting,
    /// PC port: `skate3_letters`, the main challenge is a linked Skate 3 data folder's game that
    /// runs Skate 3's own state graph, whose letters are Skate 3's own movies'.
    ThrowDown { skate3_letters: bool },
    Dpo(DpoView),
    Distance(DistanceText),
}

/// What the `dpo` movie's `Update` reads while it runs a screen state: `DPO_GetParams`
/// (`0x826b89a0`) of the data, `HUDComponents.Leaderboard_GetNumPlayers` and
/// `Game.GetActiveChallengeIconName`.
#[derive(Clone, Debug, PartialEq)]
pub struct DpoView {
    pub data: DpoHud,
    /// The bound leaderboard data's player count, 0 when it is unbound.
    pub leaderboard_players: i32,
    /// `GameCmdTranslator::sMethod_GetActiveChallengeIconName` (`0x8260b158`).
    pub challenge_icon: String,
    /// PC port: the main challenge is a linked Skate 3 data folder's game that runs Skate 3's
    /// own state graph. Skate 3's `dpo` movie has no icon (its `DPO_GetParams` 0x8265d420 gives
    /// none), and neither game ships a 1-Up icon.
    pub skate3: bool,
}

/// `TimerHUD::mTimeBuffer`'s minutes and seconds (`FormatTime(buffer, 0, minutes, seconds,
/// 0, true, 16)`) and `mColour` (`eTimerColour_White` at or above the distress time,
/// `eTimerColour_Red` below it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimerText {
    pub minutes: i32,
    pub seconds: i32,
    pub white: bool,
}

/// `DistanceMeter::mBuffer` (`ID_COMMON_DISTANCE` of the distance as `%.1f`, empty before the
/// first `Refresh`) and `mColour` (`DistanceMeter_GetData` `0x826b7c48`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DistanceText {
    pub distance: Option<f32>,
    /// `eMeterColour_Red` rather than `eMeterColour_White`.
    pub red: bool,
}

/// One component: whether its screen shows, the group whose data it is bound to, its
/// update clocks and the screen states its movie has not run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TurnHudComponent {
    /// `mVisible`.
    pub visible: bool,
    /// `mData`: the hash of the bound group.
    pub bound: Option<u32>,
    /// `LeaderboardHUD::mTimeSinceUpdate`.
    time_since_update: u32,
    /// `TimerHUD::mTicks`.
    ticks: u32,
    pub screen_states: Vec<(i32, TurnHudView)>,
}

/// The components.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TurnHuds {
    pub leaderboard: TurnHudComponent,
    pub abd: TurnHudComponent,
    pub timer: TurnHudComponent,
    pub big_black_timer:TurnHudComponent,
    pub big_black_timer_time:f32,
    pub big_black_timer_text:TimerText,
    pub countdown: TurnHudComponent,
    pub voting: TurnHudComponent,
    pub throwdown: TurnHudComponent,
    pub dpo: TurnHudComponent,
    pub distance: TurnHudComponent,
    /// `TimerHUD::mTime`: the seconds the timer shows.
    pub timer_time: f32,
    /// `TimerHUD::mTimeBuffer` and `mColour`.
    pub timer_text: TimerText,
    /// `CountdownHUD::mCountdownVisible` and `mCountdownVal`.
    pub countdown_visible: bool,
    pub countdown_value: i32,
    /// `DistanceMeter::mBuffer` and `mColour`.
    pub distance_text: DistanceText,
    /// FE sounds the components played, in order.
    pub sounds: Vec<&'static str>,
}

#[derive(Clone, Copy)]
enum Kind {
    Leaderboard,
    Abd,
    Timer,
    BigBlackTimer,
    Countdown,
    Voting,
    ThrowDown,
    Dpo,
    Distance,
}

impl Kind {
    const ALL: [Self; 9] = [
        Self::Leaderboard,
        Self::Abd,
        Self::Timer,
        Self::BigBlackTimer,
        Self::Countdown,
        Self::Voting,
        Self::ThrowDown,
        Self::Dpo,
        Self::Distance,
    ];

    /// The component's `kHUDName`.
    const fn name(self) -> &'static str {
        match self {
            Self::Leaderboard => "leaderboard",
            Self::Abd => "abd",
            Self::Timer => "timer",
            Self::BigBlackTimer => "bigblacktimer",
            Self::Countdown => "countdown",
            Self::Voting => super::voting::COMPONENT,
            Self::ThrowDown => "throwdownmessage",
            Self::Dpo => super::dpo_hud::COMPONENT,
            Self::Distance => "distance",
        }
    }

    fn has_data(self, group: &HudGroup) -> bool {
        match self {
            Self::Leaderboard => group.leaderboard.is_some(),
            Self::Abd => group.abd.is_some(),
            Self::Timer => group.timer.is_some(),
            Self::BigBlackTimer=>group.big_black_timer.is_some(),
            Self::Countdown => group.countdown.is_some(),
            Self::Voting => group.voting.is_some(),
            Self::ThrowDown => group.throwdown.is_some(),
            Self::Dpo => group.dpo.is_some(),
            Self::Distance => group.distance.is_some(),
        }
    }

    fn component(self, world: &mut World) -> &mut TurnHudComponent {
        let huds = &mut world.turn_huds;
        match self {
            Self::Leaderboard => &mut huds.leaderboard,
            Self::Abd => &mut huds.abd,
            Self::Timer => &mut huds.timer,
            Self::BigBlackTimer=>&mut huds.big_black_timer,
            Self::Countdown => &mut huds.countdown,
            Self::Voting => &mut huds.voting,
            Self::ThrowDown => &mut huds.throwdown,
            Self::Dpo => &mut huds.dpo,
            Self::Distance => &mut huds.distance,
        }
    }
}

fn bound_group(world: &mut World, kind: Kind) -> Option<&mut HudGroup> {
    let hash = kind.component(world).bound?;
    world.hud.groups.iter_mut().find(|g| g.hash == hash)
}

fn leaderboard_data(world: &mut World) -> Option<&mut LeaderboardHud> {
    bound_group(world, Kind::Leaderboard)?.leaderboard.as_mut()
}

fn abd_data(world: &mut World) -> Option<&mut AbdHud> {
    bound_group(world, Kind::Abd)?.abd.as_mut()
}

fn timer_data(world: &mut World) -> Option<&mut TimerHud> {
    bound_group(world, Kind::Timer)?.timer.as_mut()
}

fn countdown_data(world: &mut World) -> Option<&mut CountdownHud> {
    bound_group(world, Kind::Countdown)?.countdown.as_mut()
}

/// `Countdown_ChallengeHasAudio`: false for the sign-up poster's challenge
/// (`FrontEndState_SignUpPoster::mKey`, the sign-up manager's selected challenge here) when
/// loaded with a `ChallengeType` of 19, 4, 32, 30 or 0.
fn countdown_has_audio(world: &World) -> bool {
    world.signup.selected.is_none_or(|activity| !matches!(super::signup::challenge_type(world, activity), 19 | 4 | 32 | 30 | 0))
}

fn throwdown_data(world: &mut World) -> Option<&mut ThrowDownMessageHud> {
    bound_group(world, Kind::ThrowDown)?.throwdown.as_mut()
}

fn dpo_data(world: &mut World) -> Option<&mut DpoHud> {
    bound_group(world, Kind::Dpo)?.dpo.as_mut()
}

fn distance_data(world: &mut World) -> Option<&mut DistanceMeterHud> {
    bound_group(world, Kind::Distance)?.distance.as_mut()
}

/// `fe_map_icons`, and the main challenge's reference to its collection.
const FE_MAP_ICONS: u64 = 0xf788_504a_8d92_2ced;
const MAP_ICON: u64 = 0xe616_acdf_5a36_bfac;

/// `GameCmdTranslator::sMethod_GetActiveChallengeIconName` (`0x8260b158`): the main challenge's
/// `fe_map_icons` name (layout word 0), or `nochallenge` without one.
fn active_challenge_icon_name(world: &World) -> String {
    let Some(main) = world.activities.main else { return "nochallenge".to_owned() };
    let collection = world.activities.activities[main]
        .global_data
        .and_then(|(class, key)| world.db.attribute(class, key, MAP_ICON))
        .and_then(|attribute| attribute.ref_spec(0))
        .map_or(0, |(_, collection)| collection);
    world.db.layout_text(FE_MAP_ICONS, collection, 0).unwrap_or_default().to_owned()
}

/// PC port: whether the main challenge is a linked Skate 3 data folder's game that runs Skate 3's
/// own state graph (`crate::skate3`), a 1-Up whose letters the HUD shows as Skate 3 does.
fn skate3_letters(world: &World) -> bool {
    let Some(main) = world.activities.main else { return false };
    let key = world.activities.activities[main].key;
    crate::skate3::linked().is_some_and(|data| data.runs_own_graph(key))
}

/// `PlayersBackEnd::IsPlayerStanceRegular` of `SkaterManager::GetLocalIndex`.
fn local_stance_regular(world: &World) -> bool {
    world.players.as_deref().is_none_or(|players| players.is_stance_regular(world.local_player))
}

fn set_screen_state(world: &mut World, kind: Kind, state: i32) {
    let view = match kind {
        Kind::Leaderboard => {
            let stance_regular = local_stance_regular(world);
            let online = world.online.is_some();
            let skate3_letters = skate3_letters(world);
            leaderboard_data(world)
                .map_or(TurnHudView::None, |d| TurnHudView::Leaderboard { data: d.clone(), stance_regular, online, skate3_letters })
        }
        Kind::Abd => {
            let stance_regular = local_stance_regular(world);
            abd_data(world).map_or(TurnHudView::None, |d| TurnHudView::Abd { data: d.clone(), stance_regular })
        }
        Kind::Timer => TurnHudView::Timer(world.turn_huds.timer_text),
        Kind::BigBlackTimer=>TurnHudView::Timer(world.turn_huds.big_black_timer_text),
        Kind::Countdown => TurnHudView::Countdown { value: world.turn_huds.countdown_value, has_audio: countdown_has_audio(world) },
        Kind::Voting => TurnHudView::Voting,
        Kind::ThrowDown => TurnHudView::ThrowDown { skate3_letters: skate3_letters(world) },
        Kind::Dpo => {
            let leaderboard_players = leaderboard_data(world).map_or(0, |d| d.num_players);
            let challenge_icon = active_challenge_icon_name(world);
            let skate3 = skate3_letters(world);
            dpo_data(world).map_or(TurnHudView::None, |d| {
                TurnHudView::Dpo(DpoView { data: d.clone(), leaderboard_players, challenge_icon, skate3 })
            })
        }
        Kind::Distance => TurnHudView::Distance(world.turn_huds.distance_text),
    };
    kind.component(world).screen_states.push((state, view));
}

/// `HUDManager2::Update` (`0x82cee4f0`) for these components, `dt_ms` the update's
/// milliseconds, after `FixupActiveComponents` when the active groups changed.
pub fn update(world: &mut World, active_groups_changed: bool, dt_ms: u32) {
    for kind in Kind::ALL {
        if active_groups_changed {
            fixup(world, kind);
        }
        let shown = bound_group(world, kind)
            .and_then(|g| g.instances.iter().find(|i| i.component == kind.name()))
            .is_some_and(|i| i.shown);
        if kind.component(world).visible != shown {
            show(world, kind, shown);
        }
        component_update(world, kind, dt_ms);
    }
}

/// `FixupActiveComponents` (`0x82cedff0`): while no visibility flag hides the HUD, bound to
/// the front active group that has the data; otherwise unbound and hidden.
fn fixup(world: &mut World, kind: Kind) {
    let front = if world.hud.hidden_flags == 0 {
        world.hud.active.iter().copied().find(|&hash| world.hud.groups.iter().any(|g| g.hash == hash && kind.has_data(g)))
    } else {
        None
    };
    if let Some(hash) = front {
        kind.component(world).bound = Some(hash);
        if let Kind::Leaderboard = kind
            && let Some(data) = leaderboard_data(world)
        {
            data.dirty = true;
        }
    } else if kind.component(world).bound.take().is_some() {
        show(world, kind, false);
    }
}

fn show(world: &mut World, kind: Kind, show_it: bool) {
    match kind {
        // LeaderboardHUD::Show (0x826bdcc0).
        Kind::Leaderboard => {
            if show_it {
                leaderboard_refresh(world);
            }
            if world.turn_huds.leaderboard.visible != show_it {
                if show_it {
                    set_screen_state(world, kind, 1);
                    set_screen_state(world, kind, 3);
                } else {
                    set_screen_state(world, kind, 2);
                }
            }
            world.turn_huds.leaderboard.visible = show_it;
        }
        // ABDHUD::Show (0x826be4b0, shared with PhotoTreatmentHUD).
        Kind::Abd => {
            set_screen_state(world, kind, if show_it { 1 } else { 2 });
            world.turn_huds.abd.visible = show_it;
        }
        // BigBlackTimerHUD uses the shared Show826bc780 with its own Refresh.
        Kind::BigBlackTimer=>{
            if show_it{big_black_timer_refresh(world);}
            if world.turn_huds.big_black_timer.visible!=show_it{
                set_screen_state(world,kind,if show_it{1}else{2});
                world.turn_huds.big_black_timer.visible=show_it;
            }
        }
        // TimerHUD::Show (0x826bc780, shared with ObjectiveHUD; its Refresh is empty).
        Kind::Timer => {
            if world.turn_huds.timer.visible != show_it {
                set_screen_state(world, kind, if show_it { 1 } else { 2 });
                world.turn_huds.timer.visible = show_it;
            }
        }
        // OnlineVotingHUD shares PhotoTreatmentHUD::Show (0x826be4b0).
        Kind::Voting => {
            set_screen_state(world, kind, if show_it { 1 } else { 2 });
            world.turn_huds.voting.visible = show_it;
        }
        // CountdownHUD::Show (0x826c0198).
        Kind::Countdown => {
            if show_it && countdown_data(world).is_some() {
                countdown_start(world);
            } else {
                world.turn_huds.countdown_visible = false;
                world.turn_huds.countdown_value = 0;
                set_screen_state(world, kind, 1);
            }
            world.turn_huds.countdown.visible = show_it;
        }
        // ThrowDownMessageHUD::Show (0x826bec20).
        Kind::ThrowDown => {
            set_screen_state(world, kind, if show_it { -1 } else { -2 });
            world.turn_huds.throwdown.visible = show_it;
            if let Some(data) = throwdown_data(world) {
                data.message_state = -3;
                data.message_dirty = true;
            }
        }
        // DPOHUD::Show (0x826be3b0).
        Kind::Dpo => {
            if show_it {
                set_screen_state(world, kind, 1);
                set_screen_state(world, kind, 3);
            } else {
                set_screen_state(world, kind, 2);
            }
            world.turn_huds.dpo.visible = show_it;
        }
        // DistanceMeter shares OnlineOTSStatHUD::Show (0x826bc780).
        Kind::Distance => {
            if show_it {
                distance_refresh(world);
            }
            if world.turn_huds.distance.visible != show_it {
                set_screen_state(world, kind, if show_it { 1 } else { 2 });
                world.turn_huds.distance.visible = show_it;
            }
        }
    }
}

/// `DistanceMeter::Refresh` (`0x826bc568`): `mColour` red while the maximum is not negative and
/// the distance is more than 0.6 of it, `mBuffer` the distance, then state 3.
fn distance_refresh(world: &mut World) {
    let Some((distance, max)) = distance_data(world).map(|d| (d.distance, d.max_distance)) else { return };
    let white = max < 0.0 || distance / max <= 0.6;
    world.turn_huds.distance_text = DistanceText { distance: Some(distance), red: !white };
    set_screen_state(world, Kind::Distance, 3);
}

/// `LeaderboardHUD::Refresh` (`0x826bdd58`): the players' names, then states 3 and 4.
fn leaderboard_refresh(world: &mut World) {
    let players = world.players.clone();
    let Some(data) = leaderboard_data(world) else { return };
    data.refresh_player_names(|player| players.as_deref().map(|b| b.player_name(player)).unwrap_or_default());
    set_screen_state(world, Kind::Leaderboard, 3);
    set_screen_state(world, Kind::Leaderboard, 4);
}

fn component_update(world: &mut World, kind: Kind, dt_ms: u32) {
    match kind {
        // LeaderboardHUD::Update (0x826bddc0), outside a network game.
        Kind::Leaderboard => {
            if !world.turn_huds.leaderboard.visible || leaderboard_data(world).is_none() {
                return;
            }
            let c = &mut world.turn_huds.leaderboard;
            if c.time_since_update < LEADERBOARD_UPDATE_MS {
                c.time_since_update += dt_ms;
                return;
            }
            c.time_since_update = 0;
            let (dirty, non_order) = leaderboard_data(world).map_or((false, false), |d| (d.dirty, d.non_order_data_dirty));
            if dirty {
                leaderboard_refresh(world);
                if let Some(d) = leaderboard_data(world) {
                    d.dirty = false;
                    d.non_order_data_dirty = false;
                }
            } else if non_order {
                set_screen_state(world, kind, 5);
                if let Some(d) = leaderboard_data(world) {
                    d.non_order_data_dirty = false;
                }
            }
        }
        // ABDHUD::Update (0x826be258).
        Kind::Abd => {
            let Some(d) = abd_data(world) else { return };
            if !d.dirty {
                return;
            }
            let d = std::mem::replace(
                d,
                AbdHud {
                    dirty: false,
                    update_list: false,
                    update_set_trick: false,
                    update_copy_trick: false,
                    update_list_visibility: false,
                    update_gesture_visibility: false,
                    ..d.clone()
                },
            );
            let mut states = Vec::new();
            if d.update_list {
                states.push(4);
            }
            if d.update_set_trick {
                states.push(5);
            }
            if d.update_copy_trick {
                states.push(6);
            }
            if d.update_list_visibility {
                states.push(if d.list_items_visible { 7 } else { 8 });
            }
            if d.update_gesture_visibility {
                states.push(if d.gesture_area_visible { 100 } else { 101 });
            }
            let stance_regular = local_stance_regular(world);
            world
                .turn_huds
                .abd
                .screen_states
                .extend(states.into_iter().map(|state| (state, TurnHudView::Abd { data: d.clone(), stance_regular })));
        }
        // BigBlackTimerHUD.Update826b2a28: every fifth visible HUD update.
        Kind::BigBlackTimer=>{
            if !world.turn_huds.big_black_timer.visible{return;}
            world.turn_huds.big_black_timer.ticks+=1;
            if world.turn_huds.big_black_timer.ticks>=TIMER_UPDATE_TICKS{
                big_black_timer_refresh(world);world.turn_huds.big_black_timer.ticks=0;
            }
        }
        // TimerHUD::Update (0x826bbbe0): UpdateTime every fifth update.
        Kind::Timer => {
            if !world.turn_huds.timer.visible {
                return;
            }
            let c = &mut world.turn_huds.timer;
            c.ticks += 1;
            if c.ticks >= TIMER_UPDATE_TICKS {
                timer_update_time(world);
                world.turn_huds.timer.ticks = 0;
            }
        }
        // OnlineVotingHUD::Update (0x826b2ce0) → Refresh (0x826bdf48): a dirty vote sets 3 plus
        // its score.
        Kind::Voting => {
            if !world.turn_huds.voting.visible {
                return;
            }
            let Some(vote) = bound_group(world, kind).and_then(|g| g.voting.as_mut()).filter(|d| d.dirty).map(|d| {
                d.dirty = false;
                d.vote
            }) else {
                return;
            };
            set_screen_state(world, kind, 3 + vote);
        }
        // CountdownHUD::Update (0x826c0200): a shown count follows the timer's whole seconds
        // until zero; a hidden one starts again when its data is dirty.
        Kind::Countdown => {
            if !world.turn_huds.countdown.visible {
                return;
            }
            if world.turn_huds.countdown_visible {
                let value = countdown_seconds(world);
                if value != world.turn_huds.countdown_value {
                    world.turn_huds.countdown_value = value;
                    set_screen_state(world, kind, 2);
                }
                if world.turn_huds.countdown_value == 0 {
                    world.turn_huds.countdown_visible = false;
                }
            } else if countdown_data(world).is_some_and(|d| d.dirty) {
                countdown_start(world);
            }
        }
        // ThrowDownMessageHUD::Update (0x826b2e80) → Refresh (0x826bec88).
        Kind::ThrowDown => {
            let Some(d) = throwdown_data(world) else { return };
            if !d.message_dirty {
                return;
            }
            let state = if d.skate_mode { d.message_state } else { d.message_state + 5 };
            d.message_dirty = false;
            set_screen_state(world, kind, state);
        }
        // DPOHUD::Update (0x826b2e40) → Refresh (0x826be420): dirty data sets 3, a dirty
        // status 10 plus the status.
        Kind::Dpo => {
            if !world.turn_huds.dpo.visible {
                return;
            }
            let Some(d) = dpo_data(world) else { return };
            let (dirty, status_dirty, status) = (d.dirty, d.status_dirty, d.status);
            if dirty {
                set_screen_state(world, kind, 3);
                if let Some(d) = dpo_data(world) {
                    d.dirty = false;
                }
            }
            if status_dirty {
                set_screen_state(world, kind, status + 10);
                if let Some(d) = dpo_data(world) {
                    d.status_dirty = false;
                }
            }
        }
        // DistanceMeter shares BigBlackTimerHUD::Update (0x826b2a28): a refresh every fifth
        // update while shown.
        Kind::Distance => {
            if !world.turn_huds.distance.visible {
                return;
            }
            let c = &mut world.turn_huds.distance;
            c.ticks += 1;
            if c.ticks >= TIMER_UPDATE_TICKS {
                distance_refresh(world);
                world.turn_huds.distance.ticks = 0;
            }
        }
    }
}

/// BigBlackTimerHUD.Refresh826bbc38. The five-second distress threshold is
/// independent of the ordinary challenge timer and its ten-second cvar.
fn big_black_timer_refresh(world:&mut World){
    if !world.turn_huds.big_black_timer.visible{return;}
    let Some(id)=bound_group(world,Kind::BigBlackTimer).and_then(|g|g.big_black_timer) else{return;};
    let time=world.timers.display_time(id);let whole=time.max(0.).ceil() as i32;
    world.turn_huds.big_black_timer_text.minutes=whole/60-60*(whole/3600);
    world.turn_huds.big_black_timer_text.seconds=whole%60;
    let previous=world.turn_huds.big_black_timer_time;let changed=previous.ceil()!=time.ceil();
    if time<5. && changed{world.turn_huds.sounds.push(TIMER_DISTRESS_SOUND);set_screen_state(world,Kind::BigBlackTimer,4);}
    else if previous<5. && time>5.{set_screen_state(world,Kind::BigBlackTimer,5);}
    world.turn_huds.big_black_timer_time=time;world.turn_huds.big_black_timer_text.white=time>=5.;
    if changed{set_screen_state(world,Kind::BigBlackTimer,3);}
}

/// The countdown timer's time rounded up to whole seconds (`cChallengeTimerManager::GetTime`).
fn countdown_seconds(world: &mut World) -> i32 {
    let id = countdown_data(world).map_or(0, |d| d.timer_id);
    #[expect(clippy::cast_possible_truncation, reason = "the native (int) conversion")]
    let seconds = world.timers.display_time(id).ceil() as i32;
    seconds
}

/// `CountdownHUD::StartCountdown` (`0x826bc660`): the count shows from the timer's whole
/// seconds (state 2 while above zero) and the data is clean.
fn countdown_start(world: &mut World) {
    let value = countdown_seconds(world);
    world.turn_huds.countdown_visible = true;
    world.turn_huds.countdown_value = value;
    if value > 0 {
        set_screen_state(world, Kind::Countdown, 2);
    }
    if let Some(data) = countdown_data(world) {
        data.dirty = false;
    }
}

/// `kTimerDistressTime` (FE cvar, default 10): the last seconds the timer is in distress.
const TIMER_DISTRESS_SECONDS: f32 = 10.0;
/// The distress tick `TimerHUD::UpdateTime` plays in a challenge: `GlobalFEPlaySound` of the
/// `fe` collection `0xaabbe111a2855ff0`, `timer_end`.
pub const TIMER_DISTRESS_SOUND: &str = "FE_SOUND_TIMER_END";

/// `TimerHUD::UpdateTime` (`0x826bb910`): the bound timer's time (`cChallengeTimerManager::
/// GetTime`, or the online time) rounded up to whole seconds (`GetTimeFromFloat`
/// `0x826b2880`) into `mTimeBuffer`; a new second inside the distress time plays the tick
/// (in a challenge) and sets state 4, leaving it sets state 5; `mColour` is white from the
/// distress time; a new second then sets state 3, which shows the text.
fn timer_update_time(world: &mut World) {
    let Some(data) = timer_data(world) else { return };
    let (mode, id, online) = (data.timer_mode, data.timer_id, data.current_online_time);
    #[expect(clippy::cast_precision_loss, reason = "the native (float) conversion")]
    let time = if mode == 0 { world.timers.display_time(id) } else { online as f32 };
    #[expect(clippy::cast_possible_truncation, reason = "the native (int) conversion")]
    let whole = time.max(0.0).ceil() as i32;
    let hours = whole / 3600;
    let minutes = whole / 60 - 60 * hours;
    let seconds = whole - 60 * (60 * hours + minutes);
    world.turn_huds.timer_text.minutes = minutes;
    world.turn_huds.timer_text.seconds = seconds;
    let previous = world.turn_huds.timer_time;
    let new_second = previous.ceil() != time.ceil();
    if time < TIMER_DISTRESS_SECONDS && new_second {
        if mode == 0 {
            world.turn_huds.sounds.push(TIMER_DISTRESS_SOUND);
        }
        set_screen_state(world, Kind::Timer, 4);
    } else if previous < TIMER_DISTRESS_SECONDS && time > TIMER_DISTRESS_SECONDS {
        set_screen_state(world, Kind::Timer, 5);
    }
    world.turn_huds.timer_time = time;
    world.turn_huds.timer_text.white = time >= TIMER_DISTRESS_SECONDS;
    if new_second {
        set_screen_state(world, Kind::Timer, 3);
    }
}
