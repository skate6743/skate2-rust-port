//! `FrontEndState_OnlineHubMenu` (FE state 44, screen 92 `onlinehub/onlineHubMenu.swf`): the hub
//! Back opens (`StateManagerFrontEnd::CheckForOnlineHubMenu` 0x82656428). Its `HubTree` stacks
//! menus. In an online game: the host's (9, `SetupOnlineHostMenu` 0x82683e50) or a client's (10,
//! `SetupOnlineClientMenu` 0x82683fc8), the player list (14), a player's menu (4,
//! `SetupHostPlayerTemplate` 0x82684200 and `SetupClientPlayerTemplate` 0x82684350), Host
//! Options (11, `SetupHostOptionsMenu` 0x82688f90) and session access (17,
//! `GoToSetupToggleSlotAccessMenu` 0x826889e0). In a single player game: the single player menu
//! (0, `SetupSinglePlayerMenu` 0x82688b38) with Freeskate (5, `SetupFreeskateMenu` 0x82688d00)
//! and Unranked (7, `SetupUnrankedOnlineMenu` 0x82688e48), or before signing in the offline menu
//! (15, `SetupOfflineMainMenu` 0x82689068). An online free skate's Freeskate Activities lead to
//! the activity types (12, `SetupFreeskateActivitiesMenu` 0x8268a500) and a type's activities
//! (13, `GoToFreeskateType` 0x826886f0). The screen's `menu_picker`s keep the selection as
//! `NextValueCalc` and `PrevValueCalc` step it: over disabled items, wrapping.

use super::FeSoundRequest;
use super::pause_menu::PauseMenuInput;
use std::collections::BTreeMap;

pub const SINGLE_PLAYER: u32 = 0;
pub const FRIENDS: u32 = 1;
pub const PLAYER: u32 = 4;
pub const FREESKATE: u32 = 5;
pub const UNRANKED: u32 = 7;
pub const HOST: u32 = 9;
pub const CLIENT: u32 = 10;
pub const HOST_OPTIONS: u32 = 11;
pub const FREESKATE_ACTIVITIES: u32 = 12;
pub const FREESKATE_ACTIVITY_LIST: u32 = 13;
pub const PLAYER_LIST: u32 = 14;
pub const OFFLINE: u32 = 15;
pub const SESSION_ACCESS: u32 = 17;
/// The sound `CheckForOnlineHubMenu` plays as it opens the hub and `ReceiveInput` 0x8266c578 as
/// B leaves a submenu.
pub const HUB_SOUND: u64 = 0x1A53_83D3_0000_0000;
/// `"#%s (%d/%d)"`'s total of the player list (`HubStateOnlineHost::OnEvent` 0x8268b940): the
/// other players a full game holds. The original prints a literal 5; a game here holds
/// `MAX_PLAYERS`.
const PLAYER_LIST_TOTAL: usize = skate2_net::protocol::MAX_PLAYERS - 1;

/// A menu or item title as the hub's strings hold it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Title {
    /// A HAL id.
    Hal(&'static str),
    /// `"#%s"` of a name.
    Literal(String),
    /// `"#%s (%d/%d)"` of a HAL id's packed string.
    Count { label: &'static str, count: usize, total: usize },
    /// A HAL id the game data names: an activity's `Title`.
    Text(String),
    /// `"#\xBB %s"` of a HAL id's packed string: a completed activity's title behind the
    /// fonts' flash character.
    Completed(String),
}

/// One freeskate activity of the game's location as `GoToFreeskateType` reads it: its key,
/// `FreeskateActivityType` and `Title`, and `OnlineBackEnd::IsFSACompleted`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HubActivity {
    pub key: u64,
    pub kind: i32,
    pub title: String,
    pub completed: bool,
}

/// The hub `PickCurrentState` 0x82688470 sets up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HubKind {
    /// In an online game: the host's or a client's menu.
    #[default]
    Online,
    /// Signed in, in a single player or free skate game (`IsSinglePlayerHubMode` 0x82645170):
    /// the single player menu, titled with the player's name (`SetSinglePlayerHubMenu`
    /// 0x82683cc8).
    SinglePlayer,
    /// Not signed in (`Fesl::HubSingle` not `CONNECTION_READY`): the offline menu, titled
    /// `ID_ONLINE_HUB_OFFLINE_TITLE`.
    Offline,
}

/// What the game knows of the session as the hub sets its menus up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lobby {
    pub kind: HubKind,
    /// `GetLocalUser()->GetUsername()`.
    pub nickname: String,
    /// `GetGameHostPeerID() == GetLocalUser()->GetUserId()`.
    pub host: bool,
    /// Attribute 6, `Attribute_IsRanked`.
    pub ranked: bool,
    /// Attribute 4: the session is private.
    pub private: bool,
    /// The game's challenge type is `Challenge_OnlineFreeSkate`.
    pub free_skate_type: bool,
    /// `sGameInfo.gameMode == GM_OnlineFreeSkate`: lobby skate.
    pub lobby_mode: bool,
    /// `Net::IsChallengeInProgress`: the game's challenge is on, whether the local player plays
    /// it or waits in the pre-game lobby.
    pub challenge_in_progress: bool,
    /// `mIsChangingLocation`: the host's change of challenge is counting down.
    pub changing_location: bool,
    /// `OE_PlayersSkatingWith`: the other players, by user id and name.
    pub players: Vec<(u64, String)>,
    /// `GetPlayerTypeCount(GAME_PLAYER_TYPE_PARTICIPANT)`.
    pub participants: usize,
    /// The fewest participants the game's challenge starts with (`challenge::min_participants`).
    pub min_participants: usize,
    /// The other players the player list's Spectate can watch: they wait in the pre-game lobby as
    /// the local player does, and their skaters show (PC port addition).
    pub spectatable: Vec<u64>,
    /// `GetOnlineFreeSkateActivities` of the game's location, in its order.
    pub activities: Vec<HubActivity>,
    /// `sProposal`: an activity is proposed.
    pub proposal: bool,
}

/// The player menu's item in place of View Gamercard (PC port addition).
pub const SPECTATE: &str = "Spectate";

/// What an item does when selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Menu(u32),
    /// A `PeerHubMenuItem` of a player, to their menu.
    Peer(u64),
    StartChallenge,
    /// A type of the activity types menu (`GoToFreeskateGap` and the others), to its
    /// activities.
    ActivityType(i32),
    /// An activity of a type's menu (`GoToFreeskateActivityCIC`).
    Activity(u64),
    MatchSetup,
    ChangeLocation,
    SessionAccess,
    ToggleSlotAccess,
    SendFriendInvite,
    /// The player menu's Spectate (PC port addition).
    Spectate,
    KickPlayer,
    PublicFreeskate,
    CustomFreeskate,
    QuickMatch,
    CustomMatch,
    SignIn,
    /// Skate.Reel, Create-a-Spot, Ranked and Skate Profile in a game: screens the port does not
    /// open there, so these items stay off.
    NotPorted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Item {
    title: Title,
    enabled: bool,
    target: Target,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Menu {
    title: Option<Title>,
    items: Vec<Item>,
    selected: usize,
}

/// A menu as the screen's `HubTree` calls read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuView {
    pub id: u32,
    /// `GetHubMenuTitle`.
    pub title: Title,
    /// `GetHubMenuDataForID` and `OnMenuGetItemEnabled`.
    pub items: Vec<(Title, bool)>,
    /// `GetSelectedItemForHubMenuID`.
    pub selected: usize,
}

/// `SetScreenState` of the hub's screen: `SetState(0)` (`HubTree.Initialize`), 1
/// (`OnMenuPush`, "0-1") or 2 (`OnMenuPop`, "1-0").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenCall {
    Initialize,
    Push,
    Pop,
}

/// `GoToKickPlayer` 0x826722e8's confirmation: Yes or No, Yes first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KickPopup {
    pub user_id: u64,
    pub name: String,
    pub selected: usize,
}

/// A popup of the online front end (sk8popup, FE state 34).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Popup {
    Kick(KickPopup),
    /// `OnlineBackEnd::OpenLoadingPopup` 0x825665e8 while matchmaking searches: popup 38,
    /// `ID_ONLINE_MATCHING_SKATERS` with no options, not cancellable.
    Matching,
    /// `StateOnlineSignIn::ConnectToPlasma` 0x8275df08 while the sign-in connects: popup 33,
    /// `ID_ONLINE_SIGNING_IN` with no options; B cancels.
    SigningIn,
    /// What ended a search or the sign-in (`OnlineBackEnd::ShowErrorDialog` 0x825735d0), and OK.
    Notice(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The hub tree's stack, root first.
    pub stack: Vec<MenuView>,
}

/// What the hub asks of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// `OnlineFeManager::Exit` 0x8266c418.
    Exit,
    /// `GoToStartChallenge` 0x826617a0's `GameRequest(1)`.
    StartChallenge,
    /// `GoToNewChallenge` 0x82661970: FE state 69 in host mode.
    MatchSetup,
    /// `GoToChangeLocation` 0x826618f0: FE state 68 in host mode.
    ChangeLocation,
    /// `GoToToggleSlotAccess` 0x8266c998's `GameRequest(3)` of the new privacy, then Exit.
    SetPrivate(bool),
    /// `KickPlayerCallback` 0x8266c908's `Net::KickPlayer`, central message and Exit.
    Kick(u64),
    /// The player menu's Spectate (PC port addition): the console watches the player's skater,
    /// then Exit.
    Spectate(u64),
    /// `GoToFreeskateActivityCIC` 0x82661a98: FE state 67 over the hub, to propose the activity.
    ActivitySignUp(u64),
    /// `GoToFreeSkateHere` 0x8267bee8: a public free skate where the skater is
    /// (`StateOnlineFreeskateHere`).
    PublicFreeskate,
    /// `GoToCustomFreeSkate` 0x82661540: FE state 68.
    CustomFreeskate,
    /// `GoToUnrankedQuickMatch` 0x826616e0: `StartUnrankedCustomMatch(Challenge_None, quick)`.
    QuickMatch,
    /// `GoToUnrankedCustomMatch` 0x82661720: FE state 69.
    CustomMatch,
    /// `GoToSignIn` 0x8267c158: `OnlineFeManager::Exit`, then the sign-in.
    SignIn,
}

pub struct Controller {
    lobby: Lobby,
    menus: BTreeMap<u32, Menu>,
    stack: Vec<u32>,
    /// `FrontEndState_SkateProfile::m_rPeerID`: the player whose menu is set up.
    peer: Option<u64>,
    /// The type whose activities `GoToFreeskateType` last listed in menu 13.
    activity_type: Option<i32>,
    popup: Option<Popup>,
    calls: Vec<ScreenCall>,
    sounds: Vec<FeSoundRequest>,
    /// The screen's intro completed: `StateManagerFrontEnd::ReceiveInput` passes input on.
    pub ready: bool,
}

impl Controller {
    /// `OnEnter` 0x8268a3c0: `PickCurrentState` pushes the kind's root menu; `HubState*::OnLoaded`
    /// asks for the friend list (and online the players), whose events (`OnEvent`) set the menus
    /// up and `SetScreenState(0)`.
    #[must_use]
    pub fn open(lobby: Lobby) -> Self {
        let root = Self::root(&lobby);
        let mut hub =
            Self { lobby, menus: BTreeMap::new(), stack: vec![root], peer: None, activity_type: None, popup: None, calls: Vec::new(), sounds: Vec::new(), ready: false };
        hub.setup_menus();
        hub.calls.push(ScreenCall::Initialize);
        hub
    }

    const fn root(lobby: &Lobby) -> u32 {
        match lobby.kind {
            HubKind::Online if lobby.host => HOST,
            HubKind::Online => CLIENT,
            HubKind::SinglePlayer => SINGLE_PLAYER,
            HubKind::Offline => OFFLINE,
        }
    }

    /// Matchmaking searches under popup 38.
    pub fn matching(&mut self) {
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
        self.popup = Some(Popup::Matching);
    }

    /// `OnlineBackEnd::CloseLoadingPopup` 0x82566650: the search ended, and the searching popup
    /// closes as its notice opens.
    pub fn search_ended(&mut self) {
        if self.popup == Some(Popup::Matching) {
            self.popup = None;
        }
    }

    /// The popup the hub opened over its menus.
    #[must_use]
    pub const fn popup(&self) -> Option<&Popup> {
        self.popup.as_ref()
    }

    /// `OnEvent(OE_PlayersSkatingWith)` with the session as it is now.
    pub fn refresh(&mut self, lobby: Lobby) {
        if self.lobby == lobby {
            return;
        }
        self.lobby = lobby;
        self.setup_menus();
        self.calls.push(ScreenCall::Initialize);
    }

    pub fn take_calls(&mut self) -> Vec<ScreenCall> {
        std::mem::take(&mut self.calls)
    }

    pub fn take_sounds(&mut self) -> Vec<FeSoundRequest> {
        std::mem::take(&mut self.sounds)
    }

    fn players_count(&self) -> usize {
        self.lobby.players.len()
    }

    fn peer_name(&self, user_id: u64) -> Option<&str> {
        self.lobby.players.iter().find(|(id, _)| *id == user_id).map(|(_, name)| name.as_str())
    }

    /// `SetupMenus` 0x8268a808's items and the menus the events rebuild.
    fn setup_menus(&mut self) {
        match self.lobby.kind {
            HubKind::Online => self.setup_online_menus(),
            HubKind::SinglePlayer => self.setup_single_player_menus(),
            HubKind::Offline => self.setup_offline_menu(),
        }
        let root_id = Self::root(&self.lobby);
        // A menu the stack no longer leads to closes back to its parent.
        while self.stack.len() > 1 && self.stack.last().is_some_and(|id| !self.menus.contains_key(id)) {
            self.stack.pop();
        }
        if self.stack.first() != Some(&root_id) {
            self.stack = vec![root_id];
        }
    }

    /// `OnEvent(OE_FriendList)`: the port's server keeps no friends, so the item counts none.
    fn friends() -> Item {
        Item { title: Title::Count { label: "ID_ONLINE_HUB_FRIENDS", count: 0, total: 0 }, enabled: false, target: Target::Menu(FRIENDS) }
    }

    /// `SetupSinglePlayerMenu` 0x82688b38, `SetupFreeskateMenu` 0x82688d00 and
    /// `SetupUnrankedOnlineMenu` 0x82688e48. `HubStateSinglePlayer::OnEvent` 0x82688010 selects
    /// the root's first enabled item.
    fn setup_single_player_menus(&mut self) {
        let off = |title| Item { title: Title::Hal(title), enabled: false, target: Target::NotPorted };
        let items = vec![
            Self::friends(),
            Item { title: Title::Hal("ID_ONLINE_HUB_FREESKATE"), enabled: true, target: Target::Menu(FREESKATE) },
            off("ID_ONLINE_HUB_SKATEREEL"),
            off("ID_ONLINE_HUB_CREATEASPOT"),
            off("ID_ONLINE_HUB_RANKED"),
            Item { title: Title::Hal("ID_ONLINE_HUB_UNRANKED"), enabled: true, target: Target::Menu(UNRANKED) },
            off("ID_ONLINE_HUB_SKATEPROFILE"),
        ];
        let selected = items.iter().position(|item| item.enabled).unwrap_or(0);
        self.menus.insert(SINGLE_PLAYER, Menu { title: Some(Title::Literal(self.lobby.nickname.clone())), items, selected });
        self.menus.insert(FRIENDS, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_FRIENDS")), items: Vec::new(), selected: 0 });
        self.menus.insert(
            FREESKATE,
            Menu {
                title: Some(Title::Hal("ID_ONLINE_HUB_FREESKATE")),
                items: vec![
                    Item { title: Title::Hal("ID_ONLINE_HUB_PUBLIC_FREESKATE"), enabled: true, target: Target::PublicFreeskate },
                    Item { title: Title::Hal("ID_ONLINE_HUB_CUSTOM_FREESKATE"), enabled: true, target: Target::CustomFreeskate },
                ],
                selected: 0,
            },
        );
        self.menus.insert(
            UNRANKED,
            Menu {
                title: Some(Title::Hal("ID_ONLINE_HUB_UNRANKED")),
                items: vec![
                    Item { title: Title::Hal("ID_ONLINE_HUB_QUICK_MATCH"), enabled: true, target: Target::QuickMatch },
                    Item { title: Title::Hal("ID_ONLINE_HUB_CUSTOM_MATCH"), enabled: true, target: Target::CustomMatch },
                ],
                selected: 0,
            },
        );
    }

    /// `SetupOfflineMainMenu` 0x82689068, titled `ID_ONLINE_HUB_OFFLINE_TITLE` by
    /// `PickCurrentState`: Sign In, Skate.Reel and Create-a-Spot.
    fn setup_offline_menu(&mut self) {
        let items = vec![
            Item { title: Title::Hal("ID_ONLINE_HUB_SIGN_IN"), enabled: true, target: Target::SignIn },
            Item { title: Title::Hal("ID_ONLINE_HUB_SKATEREEL"), enabled: false, target: Target::NotPorted },
            Item { title: Title::Hal("ID_ONLINE_HUB_CREATEASPOT"), enabled: false, target: Target::NotPorted },
        ];
        self.menus.insert(OFFLINE, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_OFFLINE_TITLE")), items, selected: 0 });
    }

    /// The host's or a client's menu and the menus they lead to.
    fn setup_online_menus(&mut self) {
        let lobby = &self.lobby;
        let players = self.players_count();
        let friends = Self::friends();
        // OnEvent(OE_PlayersSkatingWith): "#Player List (n/7)", enabled with anyone to list.
        let player_list = Item {
            title: Title::Count { label: "ID_ONLINE_HUB_PLAYER_LIST", count: players, total: PLAYER_LIST_TOTAL },
            enabled: players > 0,
            target: Target::Menu(PLAYER_LIST),
        };
        // SetupStartChallengeMenuItem 0x82661db0, which wants another player; Spot Battle and
        // Hall of Meat start alone (PC port addition).
        let start_challenge = Item {
            title: Title::Hal("ID_ONLINE_HUB_START_CHALLENGE"),
            enabled: lobby.lobby_mode
                && !lobby.ranked
                && !lobby.free_skate_type
                && players + 1 >= lobby.min_participants.max(1)
                && !lobby.challenge_in_progress
                && !lobby.changing_location,
            target: Target::StartChallenge,
        };
        // SetupFreeskateActivitiesMenuItem 0x8268a710: in an online free skate with others and
        // its player list, while no game, proposal or change of location is on.
        let activities_enabled = lobby.free_skate_type && !lobby.challenge_in_progress && players > 0 && !lobby.proposal && !lobby.changing_location;
        let activities = Item {
            title: Title::Hal("ID_ONLINE_HUB_FREESTYLE_ACTIVITES"),
            enabled: activities_enabled,
            target: Target::Menu(FREESKATE_ACTIVITIES),
        };
        // HubStateOnlineHost::OnEvent: Host Options only while no game, proposal or change is on.
        // The port keeps them open while a challenge runs (PC port addition), for Session Access.
        let host_options = Item {
            title: Title::Hal("ID_ONLINE_HUB_HOST_OPTIONS"),
            enabled: lobby.host && !lobby.changing_location,
            target: Target::Menu(HOST_OPTIONS),
        };
        let mut root = Menu { title: Some(Title::Literal(lobby.nickname.clone())), items: vec![friends], selected: 0 };
        if lobby.host {
            if !lobby.ranked {
                root.items.extend([start_challenge, activities]);
            }
            root.items.push(player_list);
            if !lobby.ranked {
                root.items.push(host_options);
            }
        } else {
            if !lobby.ranked {
                root.items.push(activities);
            }
            root.items.push(player_list);
        }
        // OnEvent selects the root's first enabled item.
        root.selected = root.items.iter().position(|item| item.enabled).unwrap_or(0);
        let root_id = if lobby.host { HOST } else { CLIENT };
        self.menus.remove(&if lobby.host { CLIENT } else { HOST });
        self.menus.insert(root_id, root);
        self.menus.insert(FRIENDS, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_FRIENDS")), items: Vec::new(), selected: 0 });
        let list = lobby.players.iter().map(|(user_id, name)| Item { title: Title::Literal(name.clone()), enabled: true, target: Target::Peer(*user_id) }).collect();
        let selected = self.menus.get(&PLAYER_LIST).map_or(0, |menu| menu.selected.min(players.saturating_sub(1)));
        self.menus.insert(PLAYER_LIST, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_PLAYER_LIST")), items: list, selected });
        // Match Setup and Change Location wait for lobby skate with no activity proposed (the
        // original's Host Options waited for both) while Session Access stays open (PC port
        // addition); the selection stays on an item still enabled.
        let setup = !lobby.challenge_in_progress && !lobby.proposal;
        let items = vec![
            Item { title: Title::Hal("ID_ONLINE_HUB_NEW_CHALLENGE"), enabled: setup, target: Target::MatchSetup },
            Item { title: Title::Hal("ID_ONLINE_HUB_CHANGE_LOCATION"), enabled: setup, target: Target::ChangeLocation },
            Item { title: Title::Hal("ID_ONLINEHUB_SESSIONACCESS"), enabled: true, target: Target::SessionAccess },
        ];
        let kept = self.menus.get(&HOST_OPTIONS).map(|menu| menu.selected).filter(|&selected| items.get(selected).is_some_and(|item| item.enabled));
        let selected = kept.or_else(|| items.iter().position(|item| item.enabled)).unwrap_or(0);
        self.menus.insert(HOST_OPTIONS, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_HOST_OPTIONS")), items, selected });
        self.setup_player_menu();
        self.setup_session_access();
        if activities_enabled {
            self.setup_activity_types();
            if let Some(kind) = self.activity_type {
                self.setup_activity_list(kind);
            }
        } else {
            self.menus.remove(&FREESKATE_ACTIVITIES);
            self.menus.remove(&FREESKATE_ACTIVITY_LIST);
        }
    }

    /// `SetupFreeskateActivitiesMenu` 0x8268a500: the six types in its order, each enabled where
    /// the location has an activity of it (`FreeskateActivitiesExistsForType` 0x82654150), the
    /// first enabled selected; the selection stays as the menu is set up again.
    fn setup_activity_types(&mut self) {
        let items: Vec<Item> = crate::online::activities::MENU_KINDS
            .iter()
            .map(|&kind| Item {
                title: Title::Hal(crate::online::activities::type_title(kind)),
                enabled: self.lobby.activities.iter().any(|activity| activity.kind == kind),
                target: Target::ActivityType(kind),
            })
            .collect();
        let kept = self.menus.get(&FREESKATE_ACTIVITIES).map(|menu| menu.selected).filter(|&selected| items.get(selected).is_some_and(|item| item.enabled));
        let selected = kept.or_else(|| items.iter().position(|item| item.enabled)).unwrap_or(0);
        self.menus.insert(FREESKATE_ACTIVITIES, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_FREESTYLE_ACTIVITES")), items, selected });
    }

    /// `GoToFreeskateType` 0x826886f0's menu: the type's activities, each titled by its `Title`,
    /// a completed one's behind the flash character, all enabled. `sCurrentFreeskateActivitiesList`
    /// keeps at most 17 of them to propose; past that an item proposes nothing.
    fn setup_activity_list(&mut self, kind: i32) {
        const LISTED: usize = 17;
        let items: Vec<Item> = self
            .lobby
            .activities
            .iter()
            .filter(|activity| activity.kind == kind)
            .enumerate()
            .map(|(index, activity)| Item {
                title: if activity.completed { Title::Completed(activity.title.clone()) } else { Title::Text(activity.title.clone()) },
                enabled: true,
                target: if index < LISTED { Target::Activity(activity.key) } else { Target::NotPorted },
            })
            .collect();
        if items.is_empty() {
            self.menus.remove(&FREESKATE_ACTIVITY_LIST);
            return;
        }
        let selected = self.menus.get(&FREESKATE_ACTIVITY_LIST).map_or(0, |menu| menu.selected.min(items.len() - 1));
        self.menus.insert(FREESKATE_ACTIVITY_LIST, Menu { title: Some(Title::Hal("ID_ONLINE_HUB_FREESTYLE_ACTIVITES")), items, selected });
    }

    /// `Setup{Host,Client}PlayerTemplate`: the player's name; Send Friend Request (Xbox LIVE's,
    /// which the port has not, so off); Spectate in place of View Gamercard (PC port addition);
    /// the host's Kick Player in an unranked game, which the original offers only with no
    /// challenge on and the port during one too (PC port addition).
    fn setup_player_menu(&mut self) {
        let name = self.peer.and_then(|peer| self.peer_name(peer)).map(str::to_owned);
        let spectate = self.peer.is_some_and(|peer| self.can_spectate(peer));
        let mut items = vec![
            Item { title: Title::Hal("ID_ONLINE_HUB_SEND_FRIEND_INVITE"), enabled: false, target: Target::SendFriendInvite },
            Item { title: Title::Literal(SPECTATE.to_owned()), enabled: spectate, target: Target::Spectate },
        ];
        if self.lobby.host && !self.lobby.ranked && name.is_some() {
            items.push(Item { title: Title::Hal("ID_ONLINE_HUB_KICK_PLAYER"), enabled: true, target: Target::KickPlayer });
        }
        let selected = items.iter().position(|item| item.enabled).unwrap_or(0);
        let title = name.map_or(Title::Hal("ID_ONLINE_HUB_PLAYER_LIST"), Title::Literal);
        self.menus.insert(PLAYER, Menu { title: Some(title), items, selected });
    }

    /// Whether Spectate watches `user_id` (PC port addition): from the pre-game lobby, a free
    /// skate's with no activity under way or loading, with no change of location counting down,
    /// a player waiting there too.
    fn can_spectate(&self, user_id: u64) -> bool {
        let lobby = &self.lobby;
        lobby.lobby_mode
            && !(lobby.free_skate_type && lobby.challenge_in_progress)
            && !lobby.changing_location
            && lobby.spectatable.contains(&user_id)
    }

    /// `GoToSetupToggleSlotAccessMenu`: Make Public when private, Make Private when public, the
    /// header naming the current access and the enabled item selected.
    fn setup_session_access(&mut self) {
        let private = self.lobby.private;
        self.menus.insert(
            SESSION_ACCESS,
            Menu {
                title: Some(Title::Hal(if private { "ID_ONLINEHUB_SESSIONACCESS_HEADER_PRIVATE" } else { "ID_ONLINEHUB_SESSIONACCESS_HEADER_PUBLIC" })),
                items: vec![
                    Item { title: Title::Hal("ID_ONLINE_HUB_MAKE_PUBLIC_SLOT"), enabled: private, target: Target::ToggleSlotAccess },
                    Item { title: Title::Hal("ID_ONLINE_HUB_MAKE_PRIVATE_SLOT"), enabled: !private, target: Target::ToggleSlotAccess },
                ],
                selected: usize::from(!private),
            },
        );
    }

    fn menu_title(&self, id: u32, menu: &Menu) -> Title {
        menu.title.clone().unwrap_or(match id {
            HOST | CLIENT => Title::Literal(self.lobby.nickname.clone()),
            _ => Title::Literal(String::new()),
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let stack = self
            .stack
            .iter()
            .filter_map(|&id| {
                let menu = self.menus.get(&id)?;
                Some(MenuView {
                    id,
                    title: self.menu_title(id, menu),
                    items: menu.items.iter().map(|item| (item.title.clone(), item.enabled)).collect(),
                    selected: menu.selected,
                })
            })
            .collect();
        Snapshot { stack }
    }

    /// `NextValueCalc` and `PrevValueCalc`: the next enabled item past the selection, wrapping;
    /// the selection itself when none is.
    fn step(menu: &Menu, forward: bool) -> usize {
        let count = menu.items.len();
        for offset in 1..=count {
            let index = if forward { (menu.selected + offset) % count } else { (menu.selected + count * offset - offset) % count };
            if menu.items[index].enabled {
                return index;
            }
        }
        menu.selected
    }

    /// `ReceiveInput` 0x8266c578 and the top menu's `menu_picker.UpdateInput`.
    pub fn update(&mut self, input: PauseMenuInput) -> Action {
        if !self.ready {
            return Action::None;
        }
        match self.popup.take() {
            Some(Popup::Kick(mut popup)) => {
                // sk8popup's menu_picker: two options, wrapping, not cancellable.
                if input.up {
                    popup.selected = (popup.selected + 1) % 2;
                    self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_NAV_UP"));
                }
                if input.down {
                    popup.selected = (popup.selected + 1) % 2;
                    self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_NAV_DOWN"));
                }
                if !input.select {
                    self.popup = Some(Popup::Kick(popup));
                    return Action::None;
                }
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
                return if popup.selected == 0 { Action::Kick(popup.user_id) } else { Action::None };
            }
            // The searching popup takes no input: the search ends in a game or a notice.
            Some(popup) => {
                self.popup = Some(popup);
                return Action::None;
            }
            None => {}
        }
        if input.cancel {
            if self.stack.len() == 1 {
                return Action::Exit;
            }
            self.sounds.push(FeSoundRequest::PlayKey(HUB_SOUND));
            self.stack.pop();
            self.calls.push(ScreenCall::Pop);
            return Action::None;
        }
        if input.back {
            return Action::Exit;
        }
        let Some(&id) = self.stack.last() else { return Action::None };
        let Some(menu) = self.menus.get_mut(&id) else { return Action::None };
        if menu.items.is_empty() {
            return Action::None;
        }
        for (pressed, forward) in [(input.up, false), (input.down, true)] {
            if !pressed {
                continue;
            }
            let next = Self::step(menu, forward);
            if next != menu.selected {
                menu.selected = next;
                self.sounds.push(FeSoundRequest::Play(if forward { "FE_SOUND_CORE_NAV_DOWN" } else { "FE_SOUND_CORE_NAV_UP" }));
            }
        }
        if !input.select {
            return Action::None;
        }
        // menu_picker::OnMenuSelect sounds A before Game.OnMenuSelect, which acts on an enabled
        // item only.
        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
        let item = menu.items[menu.selected].clone();
        if !item.enabled {
            return Action::None;
        }
        self.select(item.target)
    }

    /// `OnMenuSelect` 0x82688570: a submenu's push, or the item's function.
    fn select(&mut self, target: Target) -> Action {
        match target {
            Target::Menu(id) => {
                self.stack.push(id);
                self.calls.push(ScreenCall::Push);
                Action::None
            }
            Target::Peer(user_id) => {
                self.peer = Some(user_id);
                self.setup_player_menu();
                self.stack.push(PLAYER);
                self.calls.push(ScreenCall::Push);
                Action::None
            }
            Target::StartChallenge => {
                if self.lobby.participants >= self.lobby.min_participants.max(1) { Action::StartChallenge } else { Action::None }
            }
            Target::MatchSetup => Action::MatchSetup,
            Target::ChangeLocation => Action::ChangeLocation,
            Target::SessionAccess => {
                self.setup_session_access();
                self.stack.push(SESSION_ACCESS);
                self.calls.push(ScreenCall::Push);
                Action::None
            }
            Target::ToggleSlotAccess => Action::SetPrivate(!self.lobby.private),
            Target::KickPlayer => {
                let Some(user_id) = self.peer else { return Action::None };
                let name = self.peer_name(user_id).unwrap_or_default().to_owned();
                // FrontEndState_Popup::OnEnter sounds core_popup.
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
                self.popup = Some(Popup::Kick(KickPopup { user_id, name, selected: 0 }));
                Action::None
            }
            Target::PublicFreeskate => Action::PublicFreeskate,
            Target::CustomFreeskate => Action::CustomFreeskate,
            Target::QuickMatch => Action::QuickMatch,
            Target::CustomMatch => Action::CustomMatch,
            Target::SignIn => Action::SignIn,
            Target::Spectate => self.peer.filter(|&peer| self.can_spectate(peer)).map_or(Action::None, Action::Spectate),
            // GoToFreeskateType: the type's menu starts at its first activity
            // (`HubMenu::ClearMenuItems` 0x82654050) and opens if it has any.
            Target::ActivityType(kind) => {
                self.activity_type = Some(kind);
                self.menus.remove(&FREESKATE_ACTIVITY_LIST);
                self.setup_activity_list(kind);
                if self.menus.contains_key(&FREESKATE_ACTIVITY_LIST) {
                    self.stack.push(FREESKATE_ACTIVITY_LIST);
                    self.calls.push(ScreenCall::Push);
                }
                Action::None
            }
            Target::Activity(key) => Action::ActivitySignUp(key),
            Target::SendFriendInvite | Target::NotPorted => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lobby(host: bool, others: usize) -> Lobby {
        Lobby {
            nickname: "SkaterA".into(),
            host,
            lobby_mode: true,
            players: (0..others).map(|index| (index as u64 + 2, format!("Skater{index}"))).collect(),
            participants: others + 1,
            min_participants: 2,
            ..Lobby::default()
        }
    }

    fn press(f: impl FnOnce(&mut PauseMenuInput)) -> PauseMenuInput {
        let mut input = PauseMenuInput::default();
        f(&mut input);
        input
    }

    fn titles(view: &MenuView) -> Vec<Title> {
        view.items.iter().map(|(title, _)| title.clone()).collect()
    }

    #[test]
    fn host_root_lists_native_items_and_selects_first_enabled() {
        let hub = Controller::open(lobby(true, 1));
        let snapshot = hub.snapshot();
        let root = &snapshot.stack[0];
        assert_eq!(root.id, HOST);
        assert_eq!(root.title, Title::Literal("SkaterA".into()));
        assert_eq!(
            titles(root),
            vec![
                Title::Count { label: "ID_ONLINE_HUB_FRIENDS", count: 0, total: 0 },
                Title::Hal("ID_ONLINE_HUB_START_CHALLENGE"),
                Title::Hal("ID_ONLINE_HUB_FREESTYLE_ACTIVITES"),
                Title::Count { label: "ID_ONLINE_HUB_PLAYER_LIST", count: 1, total: 7 },
                Title::Hal("ID_ONLINE_HUB_HOST_OPTIONS"),
            ]
        );
        assert_eq!(root.items.iter().map(|(_, enabled)| *enabled).collect::<Vec<_>>(), vec![false, true, false, true, true]);
        assert_eq!(root.selected, 1);
    }

    #[test]
    fn start_challenge_needs_another_player_and_a_challenge_type() {
        let alone = Controller::open(lobby(true, 0)).snapshot();
        assert!(!alone.stack[0].items[1].1);
        let mut free_skate = lobby(true, 1);
        free_skate.free_skate_type = true;
        assert!(!Controller::open(free_skate).snapshot().stack[0].items[1].1);
        let mut hub = Controller::open(lobby(true, 1));
        hub.ready = true;
        assert_eq!(hub.update(press(|input| input.select = true)), Action::StartChallenge);
    }

    #[test]
    fn spot_battle_and_hall_of_meat_start_alone() {
        let solo = Lobby { min_participants: 1, ..lobby(true, 0) };
        let mut hub = Controller::open(solo);
        assert!(hub.snapshot().stack[0].items[1].1);
        hub.ready = true;
        assert_eq!(hub.update(press(|input| input.select = true)), Action::StartChallenge);
    }

    #[test]
    fn client_root_has_no_host_items() {
        let snapshot = Controller::open(lobby(false, 1)).snapshot();
        assert_eq!(snapshot.stack[0].id, CLIENT);
        assert_eq!(snapshot.stack[0].items.len(), 3);
        assert_eq!(snapshot.stack[0].selected, 2);
    }

    #[test]
    fn navigation_skips_disabled_items_and_b_pops_or_exits() {
        let mut hub = Controller::open(lobby(true, 1));
        hub.ready = true;
        hub.update(press(|input| input.down = true));
        assert_eq!(hub.snapshot().stack[0].selected, 3);
        hub.update(press(|input| input.select = true));
        assert_eq!(hub.take_calls(), vec![ScreenCall::Initialize, ScreenCall::Push]);
        assert_eq!(hub.snapshot().stack[1].id, PLAYER_LIST);
        hub.update(press(|input| input.select = true));
        let snapshot = hub.snapshot();
        assert_eq!(snapshot.stack[2].id, PLAYER);
        assert_eq!(snapshot.stack[2].title, Title::Literal("Skater0".into()));
        assert_eq!(snapshot.stack[2].selected, 2);
        hub.update(press(|input| input.select = true));
        assert!(matches!(hub.popup(), Some(Popup::Kick(KickPopup { user_id: 2, .. }))));
        assert_eq!(hub.update(press(|input| input.select = true)), Action::Kick(2));
        hub.update(press(|input| input.cancel = true));
        hub.update(press(|input| input.cancel = true));
        assert_eq!(hub.snapshot().stack.len(), 1);
        assert_eq!(hub.update(press(|input| input.cancel = true)), Action::Exit);
    }

    #[test]
    fn session_access_offers_the_other_access() {
        let mut hub = Controller::open(lobby(true, 1));
        hub.ready = true;
        hub.update(press(|input| input.down = true));
        hub.update(press(|input| input.down = true));
        hub.update(press(|input| input.select = true));
        assert_eq!(hub.snapshot().stack[1].id, HOST_OPTIONS);
        hub.update(press(|input| input.up = true));
        hub.update(press(|input| input.select = true));
        let access = &hub.snapshot().stack[2];
        assert_eq!(access.title, Title::Hal("ID_ONLINEHUB_SESSIONACCESS_HEADER_PUBLIC"));
        assert_eq!(access.selected, 1);
        assert_eq!(hub.update(press(|input| input.select = true)), Action::SetPrivate(true));
    }

    #[test]
    fn the_host_keeps_session_access_and_kick_player_while_a_challenge_runs() {
        let mut playing = lobby(true, 1);
        playing.lobby_mode = false;
        playing.challenge_in_progress = true;
        let mut hub = Controller::open(playing.clone());
        let snapshot = hub.snapshot();
        let enabled: Vec<bool> = snapshot.stack[0].items.iter().map(|(_, enabled)| *enabled).collect();
        assert_eq!(enabled, vec![false, false, false, true, true], "no Start Challenge, but the player list and Host Options");
        hub.ready = true;
        hub.update(press(|input| input.select = true));
        hub.update(press(|input| input.select = true));
        let snapshot = hub.snapshot();
        assert_eq!(snapshot.stack[2].items.last(), Some(&(Title::Hal("ID_ONLINE_HUB_KICK_PLAYER"), true)));
        let mut hub = Controller::open(playing);
        hub.ready = true;
        hub.update(press(|input| input.down = true));
        hub.update(press(|input| input.select = true));
        let snapshot = hub.snapshot();
        let options = &snapshot.stack[1];
        assert_eq!(options.id, HOST_OPTIONS);
        assert_eq!(options.items.iter().map(|(_, enabled)| *enabled).collect::<Vec<_>>(), vec![false, false, true], "Match Setup and Change Location wait");
        assert_eq!(options.selected, 2, "on Session Access");
        hub.update(press(|input| input.up = true));
        assert_eq!(hub.snapshot().stack[1].selected, 2, "the greyed items are passed over");
        hub.refresh(lobby(true, 1));
        let snapshot = hub.snapshot();
        assert_eq!(snapshot.stack[1].items.iter().map(|(_, enabled)| *enabled).collect::<Vec<_>>(), vec![true, true, true], "back in lobby skate");
        assert_eq!(snapshot.stack[1].selected, 2);
        let mut waiting = lobby(true, 1);
        waiting.challenge_in_progress = true;
        assert!(!Controller::open(waiting).snapshot().stack[0].items[1].1, "a host waiting in the pre-game lobby starts nothing while the challenge runs");
        let mut changing = lobby(true, 1);
        changing.challenge_in_progress = true;
        changing.changing_location = true;
        assert!(!Controller::open(changing).snapshot().stack[0].items[4].1, "nothing while a change counts down");
    }

    #[test]
    fn spectate_takes_view_gamercards_place_and_watches_from_the_pre_game_lobby() {
        let mut free_skate = lobby(false, 1);
        free_skate.free_skate_type = true;
        free_skate.spectatable = vec![2];
        let challenge = Lobby { free_skate_type: false, ..free_skate.clone() };
        // Waiting in the pre-game lobby while the challenge runs, watching another who waits.
        let waiting = Lobby { challenge_in_progress: true, ..challenge.clone() };
        for open in [free_skate.clone(), challenge, waiting] {
            let mut hub = open_player_menu(open);
            let snapshot = hub.snapshot();
            assert_eq!(snapshot.stack[2].items[1], (Title::Literal(SPECTATE.into()), true));
            assert_eq!(snapshot.stack[2].selected, 1);
            assert_eq!(hub.update(press(|input| input.select = true)), Action::Spectate(2));
        }
        for blocked in [
            Lobby { spectatable: Vec::new(), ..free_skate.clone() },
            Lobby { challenge_in_progress: true, ..free_skate.clone() },
            Lobby { changing_location: true, ..free_skate.clone() },
            Lobby { lobby_mode: false, free_skate_type: false, challenge_in_progress: true, ..free_skate.clone() },
        ] {
            let mut hub = open_player_menu(blocked);
            assert!(!hub.snapshot().stack[2].items[1].1, "greyed out");
            assert_eq!(hub.update(press(|input| input.select = true)), Action::None);
        }
    }

    /// The first player's menu, through the root's Player List (which follows Freeskate
    /// Activities where those are enabled).
    fn open_player_menu(lobby: Lobby) -> Controller {
        let mut hub = Controller::open(lobby);
        hub.ready = true;
        let on_player_list = |hub: &Controller| {
            let snapshot = hub.snapshot();
            let root = &snapshot.stack[0];
            matches!(root.items[root.selected].0, Title::Count { label: "ID_ONLINE_HUB_PLAYER_LIST", .. })
        };
        for _ in 0..4 {
            if on_player_list(&hub) {
                break;
            }
            hub.update(press(|input| input.down = true));
        }
        assert!(on_player_list(&hub), "the root selects its Player List");
        hub.update(press(|input| input.select = true));
        hub.update(press(|input| input.select = true));
        hub
    }

    #[test]
    fn freeskate_activities_list_the_location_s_types_then_a_type_s_activities() {
        use crate::online::activities::kind;
        let mut free_skate = lobby(false, 1);
        free_skate.free_skate_type = true;
        free_skate.activities = vec![
            HubActivity { key: 1, kind: kind::GAP, title: "ID_GAP_ONE".into(), completed: false },
            HubActivity { key: 2, kind: kind::SURVIVAL, title: "ID_SURVIVAL".into(), completed: true },
            HubActivity { key: 3, kind: kind::GAP, title: "ID_GAP_TWO".into(), completed: true },
        ];
        let mut hub = Controller::open(free_skate.clone());
        hub.ready = true;
        let root = &hub.snapshot().stack[0];
        assert_eq!(root.items[1], (Title::Hal("ID_ONLINE_HUB_FREESTYLE_ACTIVITES"), true));
        assert_eq!(root.selected, 1);
        hub.update(press(|input| input.select = true));
        let types = &hub.snapshot().stack[1];
        assert_eq!(types.id, FREESKATE_ACTIVITIES);
        // Gap, Accumulation, Group Trick List, Simultaneous Trick, Survival, Tag.
        assert_eq!(types.items.iter().map(|(_, enabled)| *enabled).collect::<Vec<_>>(), vec![true, false, false, false, true, false]);
        hub.update(press(|input| input.select = true));
        let list = &hub.snapshot().stack[2];
        assert_eq!(list.id, FREESKATE_ACTIVITY_LIST);
        assert_eq!(titles(list), vec![Title::Text("ID_GAP_ONE".into()), Title::Completed("ID_GAP_TWO".into())]);
        hub.update(press(|input| input.down = true));
        assert_eq!(hub.update(press(|input| input.select = true)), Action::ActivitySignUp(3));
        for blocked in [
            Lobby { proposal: true, ..free_skate.clone() },
            Lobby { changing_location: true, ..free_skate.clone() },
            Lobby { challenge_in_progress: true, ..free_skate.clone() },
            Lobby { free_skate_type: false, ..free_skate.clone() },
            Lobby { players: Vec::new(), participants: 1, ..free_skate.clone() },
        ] {
            assert!(!Controller::open(blocked).snapshot().stack[0].items[1].1, "greyed out");
        }
        // Once the item is off, its menus close back to the root.
        hub.refresh(Lobby { proposal: true, ..free_skate });
        assert_eq!(hub.snapshot().stack.len(), 1);
    }

    #[test]
    fn a_proposal_holds_the_host_s_changes_back() {
        let mut proposing = lobby(true, 1);
        proposing.free_skate_type = true;
        proposing.proposal = true;
        let mut hub = Controller::open(proposing);
        hub.ready = true;
        assert_eq!(hub.snapshot().stack[0].selected, 3, "on the player list");
        hub.update(press(|input| input.down = true));
        hub.update(press(|input| input.select = true));
        let options = &hub.snapshot().stack[1];
        assert_eq!(options.id, HOST_OPTIONS);
        assert_eq!(options.items.iter().map(|(_, enabled)| *enabled).collect::<Vec<_>>(), vec![false, false, true]);
    }

    fn single_player(kind: HubKind) -> Controller {
        let mut hub = Controller::open(Lobby { kind, nickname: "SkaterA".into(), ..Lobby::default() });
        hub.ready = true;
        hub
    }

    #[test]
    fn single_player_hub_offers_free_skate_and_unranked_matches() {
        let mut hub = single_player(HubKind::SinglePlayer);
        let root = &hub.snapshot().stack[0];
        assert_eq!(root.id, SINGLE_PLAYER);
        assert_eq!(root.title, Title::Literal("SkaterA".into()));
        assert_eq!(root.items.iter().map(|(_, enabled)| *enabled).collect::<Vec<_>>(), vec![false, true, false, false, false, true, false]);
        assert_eq!(root.selected, 1);
        hub.update(press(|input| input.select = true));
        assert_eq!(hub.snapshot().stack[1].id, FREESKATE);
        assert_eq!(hub.update(press(|input| input.select = true)), Action::PublicFreeskate);
        hub.update(press(|input| input.down = true));
        assert_eq!(hub.update(press(|input| input.select = true)), Action::CustomFreeskate);
        hub.update(press(|input| input.cancel = true));
        hub.update(press(|input| input.down = true));
        assert_eq!(hub.snapshot().stack[0].selected, 5);
        hub.update(press(|input| input.select = true));
        assert_eq!(hub.snapshot().stack[1].id, UNRANKED);
        assert_eq!(hub.update(press(|input| input.select = true)), Action::QuickMatch);
        hub.update(press(|input| input.down = true));
        assert_eq!(hub.update(press(|input| input.select = true)), Action::CustomMatch);
    }

    #[test]
    fn offline_hub_signs_in_and_signing_in_changes_the_menu() {
        let mut hub = single_player(HubKind::Offline);
        let root = &hub.snapshot().stack[0];
        assert_eq!((root.id, root.title.clone()), (OFFLINE, Title::Hal("ID_ONLINE_HUB_OFFLINE_TITLE")));
        assert_eq!(hub.update(press(|input| input.select = true)), Action::SignIn);
        hub.refresh(Lobby { kind: HubKind::SinglePlayer, nickname: "SkaterA".into(), ..Lobby::default() });
        assert_eq!(hub.snapshot().stack[0].id, SINGLE_PLAYER);
    }

    #[test]
    fn the_searching_popup_holds_the_input_until_the_search_ends() {
        let mut hub = single_player(HubKind::SinglePlayer);
        hub.matching();
        for key in [press(|input| input.back = true), press(|input| input.cancel = true), press(|input| input.select = true)] {
            assert_eq!(hub.update(key), Action::None);
        }
        assert_eq!(hub.popup(), Some(&Popup::Matching));
        hub.search_ended();
        assert_eq!(hub.popup(), None);
        assert_eq!(hub.update(press(|input| input.cancel = true)), Action::Exit);
    }
}
