# Online — the port's implementation

What the port ships for online play as of 2026-10-17: a dedicated server, the client's
sign-in and matchmaking through the original Online screens, lobbies of up to eight skaters
who see and hear each other, the in-game online hub, and challenges (S.K.A.T.E. verified end
to end) with the original HUD, results board and next-challenge loop, and the Lobby Browser (a
port addition). The rules layer and every screen follow the original ([README.md](README.md));
the netcode and the Lobby Browser are the port's own.

## Running it

- **Players** keep `online-config.json` in the save root: `nickname` (asked the first time
  Online opens, or when the file's is not one the prompt could make; 1 to 24 letters, digits,
  spaces, `-`, `_` or `'`, at least one a letter or digit) and `server` (`host:port`, or a host
  name at the server's default port; empty or
  absent for the build's default, which the game writes back as empty). Main
  menu > Online signs in; the Online options then offer Edit Nickname (the port's addition: the
  nickname prompt again, and a new nickname signs in again under it), Freeskate, Unranked (Quick
  Match or Custom Match), the offline Skate.Reel/Create-a-Spot rows and the Lobby Browser (the
  port's addition, [below](#the-lobby-browser)). The native Ranked Match, Ranked Leaderboards
  and skate.profile Online rows are left out, and the board behind the rows is drawn two rows
  shorter.
- **The default server** is compiled in: build the release with
  `SKATE2_DEFAULT_SERVER=<host:port>` (e.g. the VPS's address) and every player whose file names
  no server connects there; without it the build's default is `127.0.0.1:32082`
  (`DEFAULT_SERVER` in `crates/skate2-core/src/ui/online_config.rs`). An `online-config.json`
  shipped beside the game with a `server` works too, but pins that address in players' files.
- **Dedicated server**: `skate2-pc --server <ip:port>` (Windows or Linux) and `skate2-server`
  (Linux, the server alone, for a host with no display) run no game, only the server: sign-in,
  matchmaking and every lobby's relays, for any number of lobbies. It logs its build id, joins
  and leaves, and a traffic line every 60 seconds. Self-hosters point `server` in their
  `online-config.json` at it. Only players on the same executable connect
  ([Builds and encryption](#builds-and-encryption)).
- **Diagnostics**: `SKATE2_TRACE_ONLINE=1` traces the client; `SKATE2_DIAGNOSTIC_ONLINE=join:<type>`
  or `create:<type>` (each with an optional `:<challenge name>`) signs in from the main menu and
  matchmakes a custom match of a network challenge type, and `list` prints every type's online
  challenges with their names, locations and titles; `SKATE2_DIAGNOSTIC_ONLINE_START=<ticks>` has the host start the challenge after
  that many lobby ticks; `SKATE2_DIAGNOSTIC_TURN_TRICK=<push>:<flick>[:<step>]` plays each of the
  local skater's turns; `SKATE2_DIAGNOSTIC_ONLINE_FRAME_DUMP=<path>` writes every tick's frame,
  its pose included, for `crates/skate2-net/examples/frame_stats.rs` (`<dump> [ack delay]
  [run length]`); `SKATE2_DIAGNOSTIC_NET=<delay ms>:<jitter ms>:<loss %>` delays, reorders and
  drops the client's datagrams both ways, as a long link does. With the trace on, each remote
  skater reports every ten seconds how many ticks showed it at or past its newest sample, its
  playout delay, its sender's pace and how many of its frames came with the teleport bit, and the
  round how many ticks it stalled waiting for facts.

## Where the code is

| Piece | Files |
| --- | --- |
| Wire protocol, transport, client, server | `crates/skate2-net/src/{protocol,transport,client,server}.rs` |
| Encryption, the build's secret | `crates/skate2-net/src/crypto.rs`, `apps/skate2-pc/build.rs` |
| Replay-frame codec | `crates/skate2-net/src/frames.rs` (+ `examples/frame_stats.rs`, `frame_lab.rs`) |
| Challenge facts codec | `crates/skate2-core/src/online/facts.rs` |
| Challenge catalog, event log, audio record | `crates/skate2-core/src/online/{catalog,event_log,audio_record}.rs` |
| Session: sign-in, games, frames, remote skaters | `crates/skate2-core/src/session/selected_session_online.rs`, `online/remote_skater.rs` |
| Session: challenge rounds, results | `crates/skate2-core/src/session/online/online_challenge.rs` |
| Session: Online options screens, Lobby Browser, in-game hub, lobby board | `crates/skate2-core/src/session/online/{online_menu,lobby_browser,online_hub,lobby_hud}.rs` |
| Front-end controllers | `crates/skate2-core/src/ui/{skate_reel,online_match,lobby_browser,online_hub}.rs` |
| Movies | `apps/skate2-pc/src/marker_ui/{online_eventlog,online_match,lobby_browser,online_hub,online_results}_movie.rs`, `results_list.rs` (`OnlineList`) |

## The server

The server plays EA's "RB" role ([sessions-and-transport.md](sessions-and-transport.md)): no
console hosts. It signs players in by nickname and game version, matchmakes, and owns every
game's state:

- **Matchmaking** (`PlayNow`): a quick match joins any public unranked game; a custom or free
  skate Join searches public games of the type at the chosen location, or at any location when
  none is chosen (the original's `ChallengeKeyPreference` fits only the chosen key), the fuller
  and then the older first, and finding none creates a public one there; Create makes a private
  game. A search must be of the version its player signed in with, a free skate is searched only
  as one and a challenge only as one, a free skate or private game is never ranked, a game with
  no one in it is never offered, and no game is made without a location. Eight players at most
  (`protocol::MAX_PLAYERS`, the original's unranked `MaxPlayerFilter`; its ranked games took 6,
  or 4 for 1-Up and S.K.A.T.E.). A private game takes no one new, one made private after players
  joined included: matchmaking never offers it, and joining refuses a private game that has
  members.
- **Lobby list** (`ListLobbies`, answered with `Lobbies`): the public unranked games with
  members whose game version is the asker's, at the challenge a location change is taking them
  to, those with room first, then the fuller, then the older, at most 64
  (`protocol::MAX_LOBBIES`): each game's id, challenge type and key, players, most players and
  phase. `JoinLobby` joins a game the list would still show and that has room, as matchmaking
  does (`GameJoined`); otherwise `PlayNowResult` says `FULL` or `NO_MATCH` (ended, private, or
  another version's).
- **Sign-in**: a connection's first message must be its `Hello`, on the reliable channel, with a
  nickname the game's prompt makes (`protocol::valid_nickname`: 1 to 24 letters, digits, spaces,
  `-`, `_` or `'`, at least one a letter or digit, no space at either end) and that holds no word
  of the server's list (below); the game sends nothing else until it is welcomed (it pings and
  acknowledges frames only once signed in). Any other message before it, another nickname, or no
  hello within 10 seconds of the handshake drops the connection (`leave::KICKED_BEFORE_SIGN_IN`,
  `KICKED_FOR_NICKNAME`), and so does any message, reliable or not, that does not decode. The game
  shows "Nickname not permitted." for `KICKED_FOR_NICKNAME`, and its nickname prompt takes only
  those characters.
- **Word list** (`crates/skate2-net/src/words.rs`): `badwords.txt`, private to the server's host
  (git ignores it, no release zip carries it and an update never replaces it), beside the
  server's executable or, when only that one exists, in its working directory. One word or
  phrase a line, `#` lines skipped; the server reads it again whenever it changes and logs at
  start how many words it holds. A text and the entries are compared as `words::normalize` reads
  them: lowercase; `1`, `!`, `|` and `l` read as `i`, `4` and `@` as `a`, `3` as `e`, `0` as `o`,
  `5` and `$` as `s`, `7` as `t`, `8` as `b`, `6` and `9` as `g`; everything else but letters
  dropped, spaces too; each run of one letter cut to one. A text holding an entry anywhere is
  caught; an entry that reads as fewer than two letters is ignored. Without the file nothing is
  filtered.
- **Bans** (PC port addition, `crates/skate2-net/src/moderation.rs`): two lists of IP addresses,
  each a JSON object of `"address": "name"` pairs (the nickname the address was banned under).
  `chat-shadowbans.json`: a member's chat line from one of its addresses comes back to that member
  as any line does and reaches no one else. `ip-bans.json`: the server reads no datagram from one
  of its addresses and answers none; a ban closes the address's connections without a word to
  them, and their games' other members see them lose their connection. The server looks at both
  files every second and reads one again when it changed, so a hand edit applies without a
  restart; a file that no longer reads as such a list leaves the bans in force, and a deleted one
  lifts its bans. A ban holds an address, not a nickname, so it covers every player behind that
  address (a household's router, a mobile network's shared address).
- **Moderation panel** (PC port addition, `crates/skate2-net/src/panel.rs`): with
  `"moderation_panel_port": <port>` in `server-config.json` and a key in `moderation-key.txt`
  (16 to 256 letters, digits, `-`, `_`, `.` or `~`, on one line), the server serves
  `http://<server>:<port>/moderationpanel?key=<key>` at its own address. The page lists the
  signed-in players (name, game, time online, version, chat shadowban), each with a box to tick,
  with Chat Shadowban and IP Ban beside them, and under them the bans in force, with Lift Ban; an
  action adds to or removes from the lists above and shows its outcome once. The page shows no
  IP address, not even in its source: a box sends a reference drawn from the address with a
  secret the panel makes at each start, which the panel maps back for an hour after the page last
  listed it (a form from an older page is asked to be refreshed). The ban lists' files and the
  server's console still name the addresses.
  Without the key, every request is answered as one for a page that does not exist; an address
  that sends 10 wrong keys within 15 minutes is answered so for the rest of those 15 minutes,
  whatever key it sends. The key is compared by its SHA-256 hash, in time that does not depend on
  where two keys differ, and is never logged. The page runs no script, loads nothing else, and
  is never cached or framed or followed by a referrer. Requests are served on threads of their
  own (8 at once, each with 10 seconds to arrive, 8 KiB of headers and a 16 KiB form at most),
  which ask the server's loop for what they show and do. The panel speaks plain HTTP, so the key
  crosses the network unencrypted. Without the setting or a valid key the panel is off, and the
  server's console says why.
- **The server's files**: `server-config.json`, `moderation-key.txt` and the two ban lists sit
  together in the folder of `server-config.json` (beside the server's executable or, when only it
  holds one, the working directory). Like `badwords.txt`, git ignores them, no release zip
  carries them and an update never replaces them.
- **Members**: the first member is the game host peer (the one with the host menus); when the
  host leaves the next member takes over. A member silent for 10 seconds is dropped; one that
  sends neither a replay frame nor `Busy` for 2 seconds (a frozen or paused game) is kicked for
  inactivity ([facts.md](facts.md)).
- **Lobby state machine**: lobby skate → `GameReset(LoadChallenge)` with the seed and the
  participants (the members in the game's world: the server holds their recipe and a replay
  frame; one still joining stays in lobby skate) → every participant's `ChallengeLoaded` → `GameReset(Challenge)` → the round's
  sync points → `Complete` → `AllPlayersComplete` → `Results` → `FinalResults` and
  `GameTimer(DisplayResults)` (the board shows four seconds) → `ExitPostChallenge` → the host's next challenge
  (`ResetAttributes`) → lobby skate. Barriers time out so a dropped player cannot hang the rest.
  A challenge every participant left goes straight back to lobby skate, with no board. The board
  holds the participants still in the challenge, each line at its place among them in the
  round's order; one who left has no line.
  A console loads its round every frame, as `ChallengeLoadState` does, with or without a pause menu
  or Options open; only a travel to the lobby's location comes first. The load pauses the
  simulation (`kPAUSE_STATE_ONLINE`) and puts the loading screen up (`WaitForStreamer`: the
  front-end background snapped black, no pause menu or online hub opening) at the player's spawn
  point; `ChallengeLoaded` goes once the challenge's assets are in and the area there has stayed
  loaded for 16 updates. `GameReset(Challenge)` then closes every front-end screen as
  `PopAllStates` does (the pause menu, Options and its music player, Skate Profile, the online
  hub), each with its `OnLeave` (Options saves nothing), and unpauses the simulation; the loading
  screen comes down once the area has stayed loaded for 16 more updates.
- **Leave Challenge** (PC port addition): `LeaveChallenge` takes a participant out of the
  challenge as a leave of the game would (its barriers, completion and result no longer wait
  for it, and the board leaves it out), but it stays a member: `ChallengeLeave` tells every
  member, and it waits in lobby skate apart from the challenge until the lobby reset. One from a
  member not playing the challenge is ignored, as are its later barriers and facts.
- **Kicks** (PC port addition): a kick closes the game to the kicked player's IP address for as
  long as the game lasts. The lobby browser no longer lists it to that address, matchmaking
  passes over it, and joining it by its id gets `NO_MATCH`, whatever nickname the address comes
  back under.
- **Host requests**: `GameRequest(StartGame)` (two or more players in the world, not in an
  online free skate, not during a location change), privacy, kick; `RequestChange` broadcasts
  `GameChange` and moves the game two seconds later. A player who joins in those two seconds
  joins at the new location (`GameJoined` carries the change's challenge), and matchmaking
  matches the game on where it is going. These and `ResetAttributes` come only from the game host peer: a member that sends one
  while another member is the host is not running the game (the game sends them only while the
  server's messages so far make it the first member, and the host changes only when the host
  leaves), so its connection is dropped (`leave::KICKED_FOR_HOST_REQUEST`). One that arrives
  after its sender left the game is ignored.
- **Relays**: each player's replay frames (re-encoded for each receiver against the frames that
  receiver acknowledged), challenge facts, recipes (appearance). From a challenge's load until
  the game is back in lobby skate its participants see nothing of the members in lobby skate
  (who joined during it, were not in the world at its start, did not load it, or left it), as
  in the original: their frames reach no participant, and facts go only to participants. The
  members in lobby skate do get the participants' frames and watch them as ghosts (PC port
  addition; `GameJoined` names the participants to one who joins during the challenge). A
  participant's frames reach those members from the lobby reset on, coded afresh (the receiver
  holds none of the sender's recent ones). An acknowledgement counts only for a frame the server still
  holds of the slot's member: one a receiver sent for the slot's member before (a player who
  left, acknowledged before the receiver heard so) would otherwise stand, its ticks far past the
  new member's, and every run to that receiver would go coded whole.
- **One thread**: the server (`server::run`) handles every connection and game on one thread,
  with no pool to run short. Each pass reads at most 64 datagrams (`MAX_BATCH`) before sending
  what they made, so a stream of arrivals cannot hold every reply back, and it sleeps a
  millisecond only when nothing arrived. Its console prints from a thread of its own, so a
  console that stops taking lines (a selection held in a Windows console, a closed terminal)
  neither stalls nor ends it; a datagram the socket cannot hand over whole is dropped and counted
  in the status line ("unreadable"). Connection and game ids pass over those still in use when
  their counters wrap. `crates/skate2-net/examples/server_load.rs` runs simulated players through
  it (`[players] [seconds] [game size] [one-way delay ms] [ticks a run]`): 20 players in games of
  7 over a 30 ms link take 9 to 12% of one core, its longest pass under 2 ms; 80 take 41 to 57%.
  Every frame reached every other member of its own game and no one else.

## The Lobby Browser

The port's addition at the bottom of the Online options, enabled while signed in. It is drawn
on `unranked_custom_match`'s timeline (its intro, outro, fades, panel, glare, title and button
help), the panel without its header band, location preview and help band, its content
stretched under the title strip for the list
([evidence](../evidence/lobby-browser-2026-10-17.md#what-the-screen-reuses)):

- **The list**: the browser asks for it on opening, on X and after a failed join's notice, at
  most once a second, and again after 5 seconds without an answer. "Searching for Game
  Session..." (`mMatch`, the screen's matchmaking card) shows until it comes. Games at a
  challenge this build's data lacks are left out; an empty list reads "No online games right
  now."
- **Cards**: three in view, filling the list evenly and scrolled with the selection (up and
  down wrap), with `basic_slider` beside them when the list is longer. Each is a `mcShading`
  panel, its fades at half their height so that its text and image sit on solid black: the
  challenge's title over its location's small image (the image the custom match screen shows)
  on the left; the challenge type over the game's phase and players on the right: "Pre-Game"
  in a challenge game's lobby skate, "In Game" past it, then "(players/8)"; a free skate, which
  is no challenge, shows only its players. The selected card has the menu picker's indicator
  and selected-item colours.
- **A** joins the selected game under "Matching up with skaters...". `FULL` opens "The Server
  is Full" / "Couldn't connect: the server is full!", `NO_MATCH` "The game session you are
  attempting to join no longer exists.", and another failure or no answer within 10 seconds
  "Unable to join game session. Please try again."; OK closes the notice and asks for the list
  again.
- **B** returns to the Online options on the Lobby Browser row. A lost connection closes the
  browser and the Online options show its notice.

## Builds and encryption

In place of ProtoTunnel's RC4, which in Skate 2 covered only each packet's preamble window
([sessions-and-transport.md](sessions-and-transport.md) §8), the port seals every packet after
the handshake (protocol 10; protocol 9 sent the replay frames in runs, 10 added the lobby list
and eight-player games):

- **A build talks only to builds of its release.** The build scripts (`apps/build_secret.rs`, for
  `skate2-pc` and `skate2-server`) give the executable a 32-byte secret. A release build reads
  the release key from `SKATE2_RELEASE_SECRET`: `build-release.bat` (the Windows game) and
  `build-release.sh` (the Linux game and the Linux dedicated server, `skate2-server`) load it
  from `release-secret.txt`, which `tools/new_release_secret.py` writes once and which stays
  private and out of git. The secret is SHA-256 of the key and the workspace version
  (`[workspace.package] version`), so the Windows game, the Linux game and the Linux server of
  one version match, and raising the version for a release turns every older build away. A
  Windows and a Linux player run the same challenge rules: these use arithmetic, which both
  systems compute alike, and of the C library's functions only `pow` (Lua's `^` and
  `math.pow`, rounded to single precision). Without a key a
  build makes a random secret each time it is compiled (any change to the app, the crates or
  the lockfile builds it again; a rebuild with nothing changed keeps it), and only its own
  `skate2-pc --server` takes it. Any other build is refused before it signs in
  (`refuse::BUILD`; an older protocol gets `refuse::VERSION`) and its player sees "Your game
  doesn't match the server's version. Update the game to play online." on the Online screen,
  where the original showed its sign-in failures; the original has no such string (Xbox LIVE
  updated the title). The server prints the build's id (a hash that says nothing of the secret)
  when it starts: the game and server of one release show the same id. A release build updates
  itself from the project's GitHub releases, and this refusal offers the update again on the main
  menu ([../release/auto-update.md](../release/auto-update.md)).
- **Handshake.** `CONNECT` carries a nonce, the client's new X25519 key and an HMAC-SHA256 proof
  keyed by the build's secret; `ACCEPT` answers with the connection id, the server's new key and
  its own proof over both keys. The session keys come from HKDF-SHA256 over the exchange, salted
  with the build's secret: one key each way, new for every connection. A connect without a valid
  proof never makes a connection.
- **Every `DATA` and `DISCONNECT` datagram** (lobby skate, challenges, facts, recipes, every
  message) is ChaCha20-Poly1305: 8 bytes stay readable (magic, version, kind, connection id),
  then a 4-byte counter (the nonce), the encrypted body and a 16-byte tag over all of it. A
  packet capture shows no nickname, message or frame; an altered, forged or replayed datagram (a
  window of 128 counters) is dropped without hurting the connection. Sealing adds 20 bytes a
  datagram, about 1 KB/s each way for a player at a race's 53 datagrams/s.
- **Limits.** The secret is inside the executable. Someone who digs it out of the binary can
  make a client the server accepts, or sit in the middle of a connection as it starts; they
  still cannot read traffic they only captured, since each connection's keys come from its own
  exchange.

## What another player can send

- **Nothing is downloaded.** The original's recipes could name user-generated stamps (custom
  graphics) by a path its consoles fetched from EA's servers, a path anyone could point
  elsewhere. The port has no HTTP code at all: a recipe's stamps are resolved by asset id from
  the game's own files, and every look leaving or entering a console (recipe messages, replay
  files) has its stamp paths emptied and user-generated stamps dropped
  (`replay_editor_archive/characters.rs`). Colours and morphs that are no number read as neutral
  ones; a recipe may name only the game's own asset namespaces and is at most 128 KB.
- **No addresses.** Players only ever talk to the server; no message carries an IP or port, a
  frame's `peerId` words go out zeroed, and player ids, sessions and seeds come from the
  system's random source, so no one can work out another player's id from their own. Only the
  server's console sees addresses (its connection lines).
- **Nothing ends the session.** Another console's facts read any float that is no number as
  zero; replay frames are quantized, so their values are always numbers. A game at a challenge
  this build's data lacks (another build, or a made-up key from a host or a game's creator) is
  never loaded: joining it, or its location changing to it, leaves it with the disconnect notice
  (`online_challenge_known`), and any other message the game cannot follow does the same rather
  than ending the session (`online_unplayable`).
- **Server limits.** Eight connections an IP; replay frames past 90 a second (bursts of 60) are
  dropped unrelayed; reliable messages past 240 a second (bursts of 600) drop the connection
  (`leave::KICKED_FOR_FLOOD`); matchmaking requests and lobby joins past 1 a second (bursts of
  4) find nothing; lobby lists past 1 a second (bursts of 5) go unanswered;
  a member's recipe is relayed once a game, and its replay frames only once the server holds it
  (the game sends the recipe with its first runs; the earlier runs only keep the member in the
  game); a privacy request that changes nothing is not sent on; a participant waits at one
  round sync point at a time. The chat's own limits are under Lobby chat below.
- **Nothing piles up on the server.** A connection whose reliable data waits 10 seconds
  unacknowledged is lost like a silent one, so a peer that keeps the connection alive without
  taking it cannot grow its queue; a reliable message is at most a recipe and its header; a game
  closes with its last member and every member's state goes with it; a run of frames that does
  not decode is logged once a member.

## Online areas

As the original's `GameModule::GameLoad` network branch (0x8273be50), every online game's
challenge (a custom match's challenge, or a free-skate lobby's location challenge) has its area,
found by its `challenges.Name`:

- **Preset chunks.** `data\stream\<Name>.xml`'s first tile lists the simulation collections (its
  center and up to 16 neighbors; `tStreamFocus::Update` 0x824e1770). Before every travel into
  the game (joining, a location change, the return to lobby skate) the simulation focus is held
  at the challenge location's position (`Net::GetStreamerPosition` for player -1) with exactly
  those collections, so the travel's wait loads them all; without an XML it is the spatial
  selection there, uncapped. Nothing moves the simulation focus in an online game
  (`CreateSimStreamerController` makes none); presentation keeps following the skater, as
  `Pres_OnlineStreamerController` does (0x8277d310: the root of the skater at `GetLocalIndex`,
  the spectated one while a challenge follows another player).
- **Boundary.** `data\boundary\<Name>.xml`'s polygon (`InvisibleBoundaryManager`,
  `crates/skate2-core/src/challenge/invisible_boundary.rs`). Within 13 m of its edge
  (`kBoundaryMinWarningDistance`) the existing out-of-bounds snow, distortion and sound rise from
  0.1 to 0.5 at 5 m (`kBoundaryMaxWarningDistance`). A skater whose respawn transform is outside
  goes to its suggested breadcrumb (`PlayerUI::Update` 0x828af9f8), and breadcrumbs are kept only
  13 m or more inside (`_TestForCollisionConstraints`' network tail). The original shows no text
  for it; "Return to Challenge Area!" and the like are career and contest objectives.
- **Props.** As in the original's network game (`isNetworkGame` and `sGhostSystemEnabled`,
  `StateOnlineGame::SetupInitalGameInfo` 0x8275db08) the living world spawns nothing: the game's
  first tick resets it with spawning off (`Simulation::IsLWSpawnEnabled` 0x8289e780), so none of
  the world's own movable props (free skate's and Party Play's) are there. The props are the
  challenge hull's: the game loads its challenge's assets with asset flags 7
  (`GameModule::GameLoad` 0x8273cc44), whose DMO data registers with `Dmo::Registration`
  (`RegDMOArena` 0x828a0420, userdata 7), and `Dmo::SimManager::SpawnStatic` (0x825eb0a0) places
  each record inside the boundary as a static body (at most 50, models of 1 to 26 Parts): solid
  and grindable, but nothing knocks, pushes or grabs it. Online "Can You Spell Girl?"
  (`m_4a_01_skate_carroll_koston_ol`) so has only its hull's props, frozen (its required hull is
  the career challenge's own, `m_4a_01_skate_carroll_koston`); Party Play adds the world's props
  and the challenge's DMO setup volume (`..._DMO_ops_vol01`). The port keeps them as static
  living world objects, group 0, drawn unfaded and left out of the DMO audio, scoring, spawn
  tests and resets, which reach no `Dmo::Body`
  (`crates/skate2-core/src/session/online/online_dmos.rs`). The hull is held through every
  round (a round's challenge system loads it too, flags 7) and goes when the game is left.
- **Leaving** the game clears both (`BackToSP` 0x8273da50): career and free skate stream around
  the skater again.
- Not ported: the rest of the original's network-game rules key on `sGameInfo.isNetworkGame`,
  which the port keeps clear (its online games simulate as free skate): `RecipeMan::UnloadEverything`
  and no navmeshes, and the front end's paused frames while the snow shows
  (`FrontEndManager::ReceiveOutOfBoundaryAmountMsg`). The Simulation's rate alone answers it, as
  `StateOnlineGame::SetupInitalGameInfo` (0x8275dba8) and `GameModule::BackToSP` (0x8273dac8)
  set and clear it: no slow motion online. `Simulation::OnRequestSetSimRate` (0x8289e350) keeps
  1/60 s, and the camera's big-air `SlowMotionController` (0x82ca8e20, 0x82ca8f48) posts nothing
  ([evidence](../evidence/online-smoothness-2026-10-19.md)). The rate's online game also picks the
  solver's passes. The original's network-game physics islands (`Simulation` and `Visual`, as
  `sGhostSystemEnabled` makes them at `World::Initialize` 0x827b0f3c) run exactly
  `kMaxOnlineIterations` (25) passes a tick, where single player's `Both` island runs
  `kMaxSinglePlayerIterations` (50) (`Island::Step_UpdatePrePhysics1` 0x827b36e8;
  `HorusPipelineIslandUsingIndirectJacobians` 0x82ae2858 has no early exit). The port's online
  games keep the `Both` island and take the online count; the online islands' collision culling
  and fixed per-player islands are not ported.

## The netcode

The original sends only pad input and simulates every skater everywhere
([lockstep.md](lockstep.md)). The port does not: each console simulates only its own skater.

**Skaters (lobby and challenges).** Every tick the client records its skater's replay-recorder
frame (root, board speed, pose, cloth twists, audio record, game camera): the same data the
replay editor plays back, so a remote skater is a puppet that shows exactly what its owner's game
showed, including the skater's sounds. The frames go out in runs (`frames.rs`), each run one
datagram ending in a sample, the tick that carries the pose, twists, audio floats and camera: two
ticks a run (30 a second) while the player plays a challenge round (its turn, or a challenge
everyone plays at once), three (20 a second) otherwise, in lobby skate and in the crowd; a
teleport and the tick after it are samples too. Every tick of a run keeps its root, board speed,
flag words and audio words, so nothing a tick carries waits for a sample. A run is coded against
the frames the receiver acknowledged and its own ticks before: the root, board speed, pose and
camera predicted on from the two frames (or samples) before, a rotation's largest part from the
length its other three leave, the audio record's world positions (centre of mass, deck, wheels)
as offsets from the root; the 32 local header words the puppet never reads stay home. What
remains goes through an adaptive binary range coder (`range.rs`) whose contexts sort each value
by how far its prediction moved it, started afresh for every run so that a lost one never parts
the two sides. Bone rotations are 1/1024 (a tenth of a degree at worst). Measured with
`frame_stats` on a fuzzed skater: with acks 6 ticks late, 9.4 KB/s at 30 runs a second where the
per-tick frames before took 15.2 KB/s on the same recording; on a recording of every tick's pose
with acks 8 ticks late, 6.6 KB/s at 20 runs a second and 9.5 KB/s at 30; a S.K.A.T.E. game the
turn-trick diagnostic played (ordinary skating, not fuzz), acks 8 late, 3.9 KB/s at 20 and 5.6
at 30. Live in a two-player lobby skate: 5.9 KB/s and 21 datagrams a second a player, against
15.6 KB/s and 65 before; over a whole S.K.A.T.E. game on a simulated 50 ms ±20 ms link with 1%
loss, 8.4 KB/s and 27 datagrams a second a player, facts and the transport included.

Frames are unreliable; nothing waits for them. A receiver shows each skater a playout delay
behind its frames' arrivals: the recent spread of arrival times (at most 45 ticks), plus three
ticks, plus the sender's sample interval, so that while a run is lost, even with the next one late,
the moment shown still lies between the samples around it. The moment shown moves on at the
sender's pace, which the delay's own movement over the last three seconds gives: a game that
cannot keep 60 ticks a second sends fewer, and its skater moves at that pace. While the delay
changes, the moment shown runs at most a quarter faster or slower than that pace. It never passes
the newest frame, so a stream that stops resumes where it stopped. A map travel stops a game's
ticks under its fade. When the newest dozen frames all arrive later, or all earlier, than the
older arrivals allow, the two games' ticks have moved apart and the older arrivals are dropped.
Frames that come while the receiver's own ticks are held (`Net::PauseGame`: a travel, a load,
a sync point) are kept but not counted, and the first counted afterwards starts the arrivals
afresh. How far the late arrivals lay behind the median is remembered and decays by eight ticks a
minute, so a link that spikes every few seconds meets each spike with the delay the last one
set rather than with a hold. A moment whose frames were lost blends the frames around it (root slerped and lerped, pose and
camera from their samples), so packet loss and late frames neither stop a skater nor make it
jump. Over a simulated link (70 ms plus up to 30 ms each way between each console and the server,
2% of datagrams lost each way), remote skaters showed past their newest sample on 0.1% of ticks
(2.2% without the interval), at a delay of about ten ticks, before the margin grew to three. A
simulation of the playout over slower senders, stopped streams and bufferbloat is in
[the evidence](../evidence/online-smoothness-2026-10-19.md#2-the-playout).

**Under the front end.** An online game keeps simulating under the pause menu, Options and the
online hub (`Net::ResumeGame`), and those frames draw as gameplay's do, with the presentation
restored between ticks.

**Spectating and scenes.** An online game keeps simulating under a scene (`Net::ResumeGame`),
so its frames are drawn through the camera each tick submits (`CameraPresentation::DoSimSubmit`)
rather than the paused NIS path. While a scene plays (a challenge presentation's NIS) the
submitted camera is the scene's; while a challenge script follows another player
(`Online.SwitchCamera`, `SkaterManager::SetLocalIndex`) it is that player's own game camera: the
camera samples around the tick the puppet shows, blended as a replay's recorded camera is
(`CameraFrame::Lerp`), never across a teleport. A change of view is a hard cut. For the audio the
spectated skater is then the local one, as `SetLocalIndex` makes it for
`AudioPresentation::DoSimSubmit`: its frame's audio record plays as the player's own does in a
replay (stereo spread, its own wheels, the speed layers), the skater microphone follows it and the
audio layers (reverb, ambience, emitters) are read at its hips; the console's own skater sounds
as any other then. The dynamic shadow follows the spectated skater too, and so does the
presentation's stream focus (the root of the frame the puppet shows), so the city loads around
the player watched. Remote skaters' audio ids
are their slots past the SkaterManager's eight Actors, so none shares the local skater's. Scenes
cannot be skipped online, so every console runs them over the same ticks. A scene that snaps its
characters to the ground (Death Race's heat starts) streams its location, and later its restore
position, from the running game's stream focus rather than the paused one.

**Challenges: the facts lockstep.** A challenge script must reach the same letters, turns,
scores and results on every console. Each console runs the whole challenge script, for every
player, from the per-tick "facts" each player's console reports about its own skater: the
`MissionFrame` the script reads (tricks landed and their scores, bails, the volumes the skater is
in, its transform), only the groups the challenge type reads of another player, coded losslessly
against the player's previous tick ([facts.md](facts.md)). A challenge tick runs once the facts
of that tick have arrived from every player pacing it: in a turn-based challenge the player whose
turn it is (and anyone it is teleporting), otherwise every player; the others stand still at their
last facts and send none. The facts travel with the runs of frames, two or three ticks at a
time, so a console that runs out of another player's facts waits until six ticks of them are in
before it runs on, and then runs one challenge tick a simulation tick while up to twelve are
ready (two past that, four past thirty): the round, and the scenes it plays, keep the
simulation's pace through the link's jitter instead of stopping at every arrival. With
the seed the server's `GameReset` gave, every console runs the same ticks over the same inputs,
at the pace of the players playing them. Turn changes, set tricks, "Trick set!",
letters, "Repeated trick" and the final ranking are therefore consequences of the shared script
run, not packets: nothing else needs acknowledging. What the script does to a skater (teleports,
pins) only that skater's console carries out. A player who leaves is reported to the script as
`cMsgPlayerDisconnected` at the tick the server's leave arrives, and the script ends the round
the original way, with one correction: S.K.A.T.E.'s `prepare_follower` skips disconnected
followers and, past the end of `FollowerOrder`, started again at the first one, so a last
follower who left before their turn sent the others back to the same set trick without end; the
search now stops at the last follower and the round goes on to `process_round`
(`challenge/script_fixes.rs`). A Spot Battle's high score works the same way: every console's script scores
each player's lines from that player's facts and keeps the spot's top score, so the "score to
beat" each player sees comes out of the shared run rather than a packet of its own.

**Contest rounds.** Best Trick and Jam start each round at a sync point
(`Online.OnlineChallengeRoundSyncPoint`): every console stops its challenge there, at a tick of
its own (each plays its own player's highlight first), pauses (`kPAUSE_STATE_ONLINE`,
`Net::PauseGame`), sends a standing facts frame with the `syncing` note and puts the loading
screen up at its spawn point (`WaitForStreamer`), then sends `SyncPoint{round}` once the area
there has stayed loaded for 16 updates and no challenge hull is loading; the consoles still on
their way run it standing still ([facts.md](facts.md#sync-points)). Once the server has the
sync point from every player, it releases them all, and every console goes on from the same tick
(the round's next run of ticks). Each console then teleports its skater to its own spawn point
of the round's start location (off the board, as the original's `cMsgTeleport` says), and its
next challenge tick delivers `cMsgOnlineSyncComplete`, the same tick everywhere. The loading
screen comes down once the area has stayed loaded for 16 more updates.
Between rounds FE state 8's online display shows the script's round board (Round Results, "Next
Round Starts in" and the script's six-second timer, nothing after the last round) until the
script closes it. As under every front-end state in an online game (`Net::ResumeGame`), the game
keeps simulating under the board, so the facts keep flowing and the script's timer runs; a Death
Race player who has finished waits on it while the others race.

**Voting.** In Spot Battle and Hall of Meat with three or more players, the other players vote on
each turn: X bad, A OK, B good, Y sick (`ChallengeVotingManager::Update`). A press goes to every
console, and the next facts of a player pacing the round carry it (`FactsNotes`,
[facts.md](facts.md)), so every console counts the same votes at the same tick, `StopVoting` averages the connected voters and the scripts agree on the players' choice.

**Lobby chat** (PC port addition; `crates/skate2-core/src/online/chat.rs`). In the game's world,
T opens the cheat code prompt's popup as "Lobby Chat": up to 200 printable ASCII characters,
Enter sends, Esc cancels. Every member's top-left event log shows the line as `[name]: line` in
its own font, broken at spaces into lines of at most 50 characters. The server is what keeps it
to the lobby:

- A line goes to the game the server itself put the sender in (`game_of` of the connection),
  never one the message names, so no player can reach another game's chat.
- The server enables a member's chat (`ChatEnabled`) only once it holds the member's recipe and
  a replay frame, the moment the others can see its skater; the game opens the prompt only after
  that. Earlier lines are dropped.
- A line the prompt cannot make (empty, over 200 characters, an edge space, anything but
  printable ASCII) drops the connection (`leave::KICKED_FOR_CHAT`); past 5 lines in 10 seconds a
  member's lines are dropped, and so is a line holding a word of the server's list (the sender
  stays connected and sees nothing). The server relays a line to every member, the sender too,
  and logs it; the name shown is the server's, never the sender's.
- Settings' Online Settings page (PC port addition, in the front end and in game) has Show Lobby
  Chat, saved as `show_lobby_chat` in `online-config.json` and on by default. Off, T opens no
  prompt and members' lines stay out of the event log; the change applies as the row turns.
- `SKATE2_DIAGNOSTIC_ONLINE_CHAT=<line>` sends a line once the chat is enabled.

## What follows the original

- **Online options** (`MainMenuOnlineOptions`): the rows and their enabling; Unranked's
  Quick/Custom popup; `unranked_custom_match` (Type Join/Create, Event, Location with image and
  description; Create is a private session, Join a public search that creates a public game when
  nothing matches); `unranked_custom_freeskate`; pushing either leaves the Online options state,
  whose screen plays its outro (and comes back with its intro), under the dimmer; the quick
  match's "Matching up with skaters..." loading popup (popup 38, no options, not cancellable;
  the `MatchingUpSkaters` screen is debug-menu only); the sign-in, no-match, no-server,
  disconnected and kicked notices.
- **Lobby and challenges**: no AI skaters (`AISkaterManager::LoadGame` notes the network game,
  whether the game is joined from the main menu or from a game in play), and, a port change as
  in Party Play, no pedestrians or traffic.
- **Session markers**: `SessionMarker::Update` 0x828acc90 and `VerifyPosition` 0x828ab328 ask
  `Simulation::IsNetworkGame`, which holds in an online game, so security and no-skate zones
  never forbid a marker there. In its place `VerifyLocalPlayerOkToSet` 0x828ab4a0 checks the
  local skater's respawn transform (`CalcRespawnTransform` 0x825f09a8) with
  `BreadCrumbManager::IsPostionOk`.
- **Lobby**: each player starts at the location's spawn point of its player index
  (`GetStreamerPosition`). The original's online challenges are for six players and their
  locations hold six spawn points, so a seventh or eighth player (PC port addition) starts at the
  sixth's, in the lobby and in a round's teleports and sync points alike; `GetSpawnLocation`'s
  own fallback for a missing index (the first point moved 5 m along -z per index) is often in the
  ground. The top-left event log (`hud_online_eventlog2`) tells of joins,
  leaves, host changes, privacy changes and the host's location changes; the bottom-right
  lobby board ("lobbyhud", `SetupLobbyHUD`): the leaderboard titled by `Mission.GetTitle()`
  ("Challenge Lobby", or "Freeskate" in an online free skate game), each player by index with
  the local one highlighted, and "Event in Progress" while a challenge runs without the player.
  As on every leaderboard HUD in a network game (`Mission.GetIsOnlineMission`), a player's
  number sits in a box of the player's colour (`tintBox` played to the player's index) and
  each row keeps 30 for a voice icon (`kTURNIND_VOIPICON_SIZE`), which stays hidden: the port
  has no voice chat (`GetVoipStatus` answers disabled).
- **Player names** (`SkaterPresEntity::DrawName` 0x827d7fe0, app `marker_ui/online_names.rs`):
  every remote player's name over their skater, in the lobby and while a challenge shows
  them, never the local player's. The label is the skater's hips 1.3 m up
  (`kfSkaterLabelYAdjust`), projected by the scene camera; `helvetica_bold` at size 20
  (`kfSkaterLabelFontSize`) times `1 - 0.018 d` and alpha `1 - 0.03 d` for the camera's
  distance d (`kfSkaterLabelFontScale` 0.82, `kfSkaterLabelFontFade` 0.7), so a name fades out
  33 m away; centred on the 1280x720 front buffer (fitted to the window as the 3D view is) and
  coloured by `PlayersBackEnd::GetPlayerColour`'s `kOnlineColours` for the player's index (red,
  green, blue, yellow, purple, orange, teal, grey; the index of the player's leaderboard row).
  `SceneRenderView::Render` draws the names of the entities its `Cull` kept, after the meshes,
  tested against the scene's depth and writing it; none in a replay.
- **Single player hub** (Back in a career or free skate, outside a career challenge,
  `CheckForOnlineHubMenu`, which also refuses while a save is written, a quit is under way, a
  NIS plays, the sign-up poster or the press-A dialog is up, the pause menu is disabled or the
  skater is being jacked): signed in, the single player menu titled with the nickname (Friends
  (0/0), Freeskate, Skate.Reel, Create-a-Spot, Ranked, Unranked, Skate Profile; Skate.Reel,
  Create-a-Spot, Ranked and Skate Profile lead to screens the port does not open in a game and
  stay off); before signing in,
  the offline menu (Sign In, Skate.Reel, Create-a-Spot). Freeskate offers Public Freeskate (the
  online free skate location whose boundary holds the skater, else the nearest boundary's, as
  `StateOnlineFreeskateHere` picks it, then a public free skate there) and Custom Freeskate (FE
  state 68); Unranked offers Quick Match and Custom Match (FE state 69). The hub and its screens
  pause the game as the pause menu does (`gFEStateInfo` rows 44, 68 and 69), and the front end
  keeps updating and drawing over the retained world, as under the pause menu; the hub takes
  input once its screen's intro completes. The searches run
  under the hub's "matching skaters" popup (Quick Match, Public Freeskate) or the screen's own
  state and end in a game or a notice over the hub or the screen; the notice's OK closes them
  all and the game goes on (`StateOnlineFE::HandleErrorState`: `CloseAllPopups`,
  `PopToState(0)`). Sign In closes the hub and signs in under the "signing in" popup, which B
  cancels (a nickname first when `online-config.json` has none: the Online options' prompt, as
  a popup); a failed sign-in's notice shows over the game. A game joined unloads the game in
  play under the fade (`StateGameUnload`) and loads the online game as from the main menu.
- **Online hub** (Back, `FrontEndState_OnlineHubMenu`): the host's and a client's menus,
  Player List (n/7) (the original prints a literal 5; the total here is the other players of a
  full eight-player game), each player's menu with Kick Player and its confirmation, Host Options
  (Match Setup and Change Location over the hub in their host states, Session Access);
  Start Challenge only in a challenge lobby with another player, outside a challenge and a
  change. The original closes Host Options while a game is on and offers Kick Player only with
  no challenge on; the port keeps both open to the host while a challenge runs (PC port
  addition), so a kick or a change of access applies at once, while Match Setup and Change
  Location stay greyed until the game is back in lobby skate. A host waiting in the pre-game
  lobby is offered no Start Challenge while the game's challenge runs. The hub hides the HUD; the game
  keeps running under it (`Net::ResumeGame`).
- **Challenges**: every unranked custom match type: Spot Battle, Best Trick, Jam, S.K.A.T.E.,
  Death Race and Hall of Meat. Loading barrier, the challenge HUD, turns, letters, rounds and
  timers as in the original; contest round boards between rounds; at the end FE state 8's
  online display (`ranked_results_overall`, an `eFEScreenType_2D` screen drawn flat with its
  1280x720 backdrop filling the window, the challenge's map icon at its top left): the
  unranked board, S.K.A.T.E.'s letters, distance
  scores through `NumberToDistance` (a board of seven or eight players takes the screen's `full`
  layout, the rank-up's, without its rank-up panel: `normal` has room for six rows, and the
  letters column gains two rows like its authored six), "Returning to Lobby in" with the server's timer (a ranked
  game would say "Next Challenge Loads in"), B to leave online (with its confirmation); then the
  next challenge in the same lobby. Going back to lobby skate, the skater travels to the game's
  location behind the front end's fade, as the original's `GameModule::GameLoad` does: the
  challenge's dynamic hulls and the lobby's own (kept through the round) all unload before the
  new free-skate game loads its own (`IsAllUnloaded` 0x82da1680). A network game's
  `GenerateGameModeSignature` is 8, under which the dynamic hull manager loads no game, so none
  of the career's gates (Fun Track's, Slappy's alley) shows or blocks online, whatever the
  player's career has opened. Every reset, like `UpdateGameInfo`'s `ResetPlayers` (0x82753b44),
  and the game's unload, like `StateGameUnload`'s (0x82757dcc), put the players' scoring
  settings back, so what a challenge set ends with it: S.K.A.T.E.'s
  `ChallengeManager.SetManualsAreConnectors(false)`, under which landing in a manual ends the
  line, is undone by its script only in single player.

## Not implemented, or different

- **No ranked play or profile services**: the Online options leave out Ranked Match, Ranked
  Leaderboards and skate.profile Online, and the single player hub's Ranked and Skate Profile
  stay disabled; ranks, badges, EXP and cash rewards are not awarded.
- **No Xbox LIVE**: Friends shows (0/0) and is disabled; Send Friend Request and View Gamercard
  are disabled; the results board omits the gamercard and player-review bumpers; no invites.
- **Freeskate Activities** (proposals, voting, their stats panel on the freeskate screen),
  spot leaderboards, Create-a-Spot online and Skate.Reel uploads are not ported.
- **HUD pieces**: the online timer (lock/start/hurry countdowns, the location-change countdown)
  and the "waiting for skaters" chyron.
- **Joining a game mid-challenge**: the joiner skates in the lobby until the next challenge,
  apart from the challenge's players (see Relays above), whom it watches as ghosts (PC port
  addition): drawn with the ghost technique at no more than 40% opacity, with their names and
  sounds and no deck shadow, while they never see it (a frame of a member not in their round
  that still arrives is dropped). A remote skater whose frames stop for three seconds shows no
  more (its name and sounds go with it) until its frames come again; a shorter hitch leaves it
  where it was. The original takes no join while a challenge runs, so the challenge's players
  hear nothing of a member waiting in the pre-game lobby: no line when one leaves. As a port
  addition, their event log reads "<name> has joined pre-game lobby." when one joins during
  their round.
- **Leave Challenge** (PC port addition): the online challenge's pause menu has Leave Challenge
  above Quit Game while the local player's round runs and it has not finished, with a No/Yes
  confirmation. Yes ends the local round as the lobby reset does (the skater travels back to the
  game's location behind the fade) and leaves the player in the pre-game lobby, watching the
  others as ghosts. To the round's other players the leaver disconnects from the challenge
  after its last facts, as a player who left the game does, its skater goes at once, and the log
  reads "<name> has left the challenge."; the members already waiting see the leaver as itself
  again and read the same line. The leave also ends the challenge's mission music as an
  abandoned challenge does (`QuitChallenge`) and resets what the challenge gave the players'
  scoring, as the next lobby reset's `ResetPlayers` would.
- **A round's end** (a lobby reset or Leave Challenge): the challenge system's game unload shuts
  its activities down, then the HUD manager's update runs over the deleted groups, so every
  component they bound hides (S.K.A.T.E.'s trick list, the timers, the challenge's leaderboard)
  before the system goes; overlay screens and the subtitle of a cutscene go with it. Every
  skater then scores as with no challenge: the multiplier a Spot Battle turns off is back and
  every trick kind is enabled. Leaving the game mid-challenge does the same through the game's
  unload.
- **Sync points**: the original's flow also preloaded the round's NIS before reporting the sync
  point; here the scene loads when the script starts it. The loading screen leaves the stream
  focus where the online game's area holds it, and shows no loading tag.
- **No replays** (a port choice, unlike the original): an online game plays no highlight reel
  (`HighlightReel.PlayHighlightReel*` answer `cMsgHighlightReelComplete` at once) and no
  presentation replay (the `REPLAY` event goes on at once), so a challenge's end goes straight
  to its results board and a contest round's to its round board.
