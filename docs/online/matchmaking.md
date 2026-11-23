# Matchmaking and the online hub

How a player gets from a menu to a game, and the menus around it. This is part of the "rules"
layer the port keeps 1:1. Addresses are `sk82_na_m.xex`.

## 1. Ways into a game

| Entry | Native | Result |
| --- | --- | --- |
| Ranked quick match | `OnlineHubMenu::GoToRankedQuickMatch` 0x826615c0 → `StateOnlinePlayNow::DoQuickMatch` 0x827737d0 | PlayNow, any ranked mode |
| Ranked custom match | `GoToRankedCustomMatch`, `FrontEndState_RankedCustomMatch::SendRankedMatchMessage` 0x826a8440 | PlayNow for one mode |
| Unranked quick / custom match | `GoToUnrankedQuickMatch`, `GoToUnrankedCustomMatch`, `FrontEndState_UnrankedCustomMatch::SendUnrankedMatchMessage` 0x826a85a0 | PlayNow; custom picks mode, location (or "any"), private |
| Free skate here / now | `GoToFreeSkateHere`, `StateOnlinePlayNow::DoFreeskateNow` 0x82778d60 | the online free skate nearest the player (`StateOnlineFreeskateHere::FindClosestFreeskateByDistance`), then PlayNow. PC port: among the free skates of the world the player is in (the main world's, a downloadable content world's, or a linked Skate 3 map's own), as another world's locations lie over the same ground |
| Join a friend | `IE_JoinGame_UserId` (`EventSystem::HandleJoinGame_UserId`) | Theater join by user id |
| Accept an invite | `PlasmaSystemPlatform::OnXb360ConnectionGameInviteAccepted` 0x82786970, `EventSystemPlatform::HandleJoinGame_GameInvite` 0x82781d28, `Net::CheckPendingGameInvite` | Theater join by invite; a ranked game refuses invited players (`SER_GAME_INVITE_TO_RANKED_GAME`) |

All of them end in `MESSAGE_JoinGame*` → `StateOnlineGame::DoJoinOnlineGame`
([game-flow.md](game-flow.md)). The unranked custom match also doubles as the in-game *change*
request: when it is not a new match it sends `GameRequestChange` to the RB instead
(`SendUnrankedMatchMessage`).

## 2. PlayNow (`Sk8::Flow::StateOnlinePlayNow`)

`DoPlayNow(type, challenge key, private, ranked, world, dev version)` 0x827736b8 and
`DoQuickMatch(world, dev, ranked)` 0x827737d0 build filters and preferences
(`BuildFilters` 0x8276d2f8), check the skill (`CheckMySkillLevel` 0x827732c8), then search:

```
private ───────────────────────────────────────────► CreateNewServer
FindExistingServers (findServer)
   ├─ match ─► join
   └─ none ─► quick match: error "no match"
              ranked: FindNewServer (resetServer, pool of max players) ─► none ─► CreateNewServer
              unranked: CreateNewServer (resetServer, pool of 1) ─► none ─► error "No RB found"
```

| `PlayNowType` | `PlayNowManager::SetupOptions` 0x8278d518 | FESL `sessionType` |
| --- | --- | --- |
| `PNT_FindExistingGame` | `SESSION_TYPE_FIND_SERVER`, count threshold (0 s, 1) | `findServer` |
| `PNT_FindServer` (ranked) | `SESSION_TYPE_RESET_SERVER`, pool max = max players, pool timeout and the count/fit threshold tables from the settings | `resetServer` |
| `PNT_CreateGame` | `SESSION_TYPE_RESET_SERVER`, pool max 1, count threshold (0 s, 1) | `resetServer` |

"Reset server" asks EA's PlayNow service for an idle dedicated server (RB) and resets it for this
group; nothing in Skate 2 hosts a game on a console. The SDK's other session types,
`listServers` and `assignHost`, are not used by Skate.

**Filters** (must match), in `BuildFilters`:

- always `VersionFilter` (game version; dev builds only match dev);
- quick match: not private, ranked = the choice, and unranked quick match excludes online free
  skate (`NotFreeSkateFilter`);
- custom match / free skate: `ChallengeTypeFilter(type)`; ranked = false when private or free
  skate, else the choice; private = the choice;
- `MaxPlayerFilter(max)`: 8, or in ranked 6 for Own the Spot and Hall of Meat and 4 for 1-Up and
  S.K.A.T.E. (`mMaxPlayers`).

**Preferences** (scored): `LocationPreference` (ping site, when the game browser has ping sites),
`ChallengeKeyPreference(type, key, DLC allowed)` (ranked without DLC permission filters DLC
challenges; its fit table, constructor 0x8276c550, is 1 between a key and itself and -1 between
any two others, so a chosen location takes only games at it; with no key chosen, a key whose
high word is -1, every pair fits 0.5 and its value is a random key of the type's), and for quick
or ranked matches `SkillPreference` on `overall_skill` (and
`challenge_skill` for ranked custom). An expired skill is fetched first (`GetUserSkillLevel`,
`StateOnlineGetProfile`).

**Attribute names** (`Net::GetAttributeName` 0x82a6a6b0, `sOnlineAttributes`):
`game_version`, `challenge_type`, `challenge_key`, `ping_site`, `is_private`, `max_players`,
`is_ranked`, `overall_skill`, `challenge_skill` — the same nine as the game attributes
([messages.md](messages.md) §4).

**Settings** (`Sk8::Xml::PlayNowSettings` from the server's config; defaults
`Net::GetDefaultPlayNowSettings` 0x82798698): pool timeout 20 s; player-count thresholds
(0 s → 8 players, 12 s → 2); fit thresholds (0 s → 100, 8 s → 0).

**Wire** (`Fesl::PlayNowServiceImpl::Start` 0x8243bf58, `EncodeFilters` 0x8243d520,
`EncodePreferences` 0x8243cbb8): FESL `pnow` transaction `Start` with `partition`,
`debugLevel`, `version`, `players.0.ownerId/ownerType` (plus one entry per play-group member),
and under `players.0.props.`: `{sessionType}`, `{name}`, `{firewallType}`
(none/simple/strict/unknown), `{poolMaxPlayers}`, `{poolTimeout}`, `{poolTargetPlayers}`
("0:n" pairs), `{fitThreshold}`, `{hostSetupTimeout}`, `{availableServerCount}`,
`{maxListServersResult}`, `{ping}` ("site:ms|..."); filters as `{filter-<attr>}` with
`{filterToGame-<attr>}` = `U-<attr>` (the Theater game attribute); preferences as
`{pref-<attr>}`, `{prefVotingMethod-<attr>}` (plurality/lottery), `{fitTable-<attr>}`,
`{fitValues-<attr>}`, `{fitWeight-<attr>}`, `{fitScale-<attr>}`, `{fitThresholds-<attr>}`,
`{aggrPref-<attr>}`, `{prefToGame-<attr>}`, `{prefToGameOps-<attr>}` ("matchunusedserver").
The protocol string is `Net::GetProtocolVersion` ("Skate2-220" here).

**Results** (`Fesl::GameBrowserImpl::OnPlayNowStatus` 0x82474238, `resultType`): `JOIN` and
`LIST` → look up the game and join (`PlayNowManager::OnPlayNowJoinGame` → `OE_PlayNowJoinGame`
with lobby and game id); `NOMATCH` / `NOSERVER` → the fallbacks above; `CREATE` /
`CREATE_PROXY` → the SDK would create a console-hosted game (`PlayNowCreateGame` 0x82474fd0),
which the Skate flow never asks for.

## 3. The online hub (`FrontEndState_OnlineHubMenu`)

The hub is the in-world menu, built from `HubMenu` objects by `SetupMenus` 0x8268a808 in one of
three layouts: single player (`SetSinglePlayerHubMenu` 0x82683cc8), online as the game leader
(`SetOnlineHostHubMenu` 0x82683b98) and online as another player (`SetOnlineClientHubMenu`
0x82683c30). Verified sub-menus:

- the single-player online menu (`SetupSinglePlayerMenu` 0x82688b38): Freeskate, Skate.Reel,
  Ranked, Unranked, Skate Profile;
- Ranked (`SetupRankedOnlineMenu` 0x82688d98): quick match, custom match, leaderboards
  (`FrontEndState_RankedLeaderboard`: top 50, my score, friends, by challenge type);
- Unranked (`SetupUnrankedOnlineMenu` 0x82688e48): quick match, custom match;
- Host options (`SetupHostOptionsMenu` 0x82688f90): change location, session access (open or
  closed, `GoToToggleSlotAccess` → `GameRequest` 3).

The other actions, by their handlers: start challenge (`GoToStartChallenge` → `GameRequest` 1),
new challenge (`GoToNewChallenge`), kick (`GoToKickPlayer` → `Net::KickPlayer`), invites and
friend invites, player list, gamercards and skate profiles, Create-a-Spot browser, editor and
tutorial, sign in. Which layout shows which of these was not traced further.

**Free-skate activities** ("FSA", `SetupFreeskateActivitiesMenu`): accumulation, gap, group
trick list, simultaneous trick, survival, tag (`GoToFreeskate*`). Proposing one sends a
`GameProposal` and needs at least two participants ([messages.md](messages.md) §7).

The hub listens to server messages too (`OnMessage` 0x82675cb8): proposals, and resetting the
proposal state on a challenge reset, a kick or a `GameChange`.

## 4. For the port

Everything in this page is front-end and matchmaking logic. It can be ported 1:1 against any
server that answers the same questions (find a game with these filters, else give me a fresh
one), whether that is Arcadia's Plasma emulation or a new protocol carrying the same filter,
preference and threshold model.
