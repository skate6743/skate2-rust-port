# Automatic disconnects online — November 22, 2026

## What happened

Some players reported being put out of online games now and then while playing; others never
were.

## Every automatic disconnect

What ends a connection or takes a player out of a game without the player or the host asking.
Server: `crates/skate2-net/src/server.rs`; transport: `transport.rs`; game:
`crates/skate2-net/src/client.rs`, `crates/skate2-core/src/session/selected_session_online.rs`.

The last column is the line the notice of a game lost ends with since this date (see What
changed); until then the notices were `ID_ONLINE_GAME_LEFT` alone, and the host's
`ID_ONLINE_KICKED_BY_GAME_HOST` for every kick.

| Condition | Limit | Who acts | The notice's reason |
| --- | --- | --- | --- |
| The game sends neither a replay frame nor `Busy`, as a participant of a challenge in play | 5 s (2 s until this date) | server: `Kicked`, out of the game, the connection stays | `5 second challenge timeout` |
| The same, as any other member | 60 s (2 s until this date) | server: `Kicked`, out of the game, the connection stays | `60 second inactivity timeout` |
| The host's kick (the one disconnect a player asks for) | at once | server: `Kicked` | `Kicked by host` |
| No datagram from the other end | 10 s (`transport::TIMEOUT`) | either end | `Connection timed out` |
| Reliable data the other end has not acknowledged | 10 s | either end | `Connection timed out` |
| A datagram or message that does not decode, an acknowledgement of data never sent | at once | server (the game too, for a reliable message) | `Connection timed out` |
| The same address connecting again | at once | server drops the older connection | `Connection timed out` on the older one |
| No sign-in after the handshake, or anything sent before it | 10 s | server | none: sign-in failed, no game to lose |
| A nickname the prompt cannot make or one holding a listed word | at sign-in | server | none: "Nickname not permitted." |
| A chat line the prompt cannot make | at once | server | `Invalid chat message` |
| Reliable messages past 240 a second (bursts of 600) | at once | server | `Too many messages` |
| A request only the host may make, from another member | at once | server | `Host-only request` |
| An address added to the IP bans | when the list is read (every second) | server: the connection is closed without a word and the address never answered again | `Connection timed out`, after the game's own 10 seconds of hearing nothing |
| An address added to the matchmaking shadowbans while in a shared game | when the list is read | server: closed as a lost connection | `Connection timed out` |
| The socket fails | at once | game | `Network error` |
| A server message the game cannot follow (a location its data lacks, a round it cannot build) | at once | game: `LeaveGame` | `Game error` |
| A challenge the build lacks | at once | game: `LeaveGame` | `Missing content`, under `ID_DLC_REQUIRED` |

Out of a challenge only, the player staying in the game: a S.K.A.T.E. round every other player's
console waited on for four seconds (`Stalled`), a challenge not loaded within 90 seconds, a sync
point not reached within 45, results not sent within 60.

## Which of these a healthy game met

- The inactivity rule. `Busy` and the replay frames come from the game's own update, so two
  seconds without one were any two seconds the update did not run or its datagrams did not
  arrive: a hitch (a stall while streaming, a device reset, a window the system stops drawing), a
  load that outlasted the 30 seconds the connection's thread says `Busy` for it, or a link down
  for a moment (the transport itself allows 10 seconds). Which players meet those depends on
  their disk, their system and their link, as the reports did. In lobby skate no one waits on
  the member, so the kick protected nothing there.
- The kick's notice was the host's (`ID_ONLINE_KICKED_BY_GAME_HOST`), whatever the reason.

None of the other conditions is met by the game as it runs: its messages decode, its nickname and
chat prompts make only what the server takes, it sends reliable messages at the simulation's
rate (60 a second and a few), and it asks the host's requests only as the host.

## What changed

- Server: the short limit holds only for the participants of a challenge in play, whose facts the
  other players' rounds wait for, and is five seconds (`INACTIVITY`). Every other member has a
  minute (`IDLE_INACTIVITY`, `Game::inactivity`). A connection that goes silent is still dropped
  after 10 seconds, and a challenge's load still has 90.
- Server: a kick at the minute's limit carries its own reason (`leave::KICKED_FOR_IDLE`, 11); a
  challenge's keeps `KICKED_FOR_INACTIVITY` (5). The message's format is the same, and a game
  from before this date reads either as it read every kick.
- Game: a kick for inactivity shows `ID_ONLINE_KICKED_INACTIVITY`, the dialog
  `OnlineBackEnd::ShowErrorDialog` (0x8257d068) has for `eONLINE_ERROR_KICKED_FOR_INACTIVITY`.
- Game: every notice of a game lost ends with a line `Reason: <name>` under the original's text
  (`LossReason`, `notice_text`; the popup reads a label, a line break and a second label as
  both). The names are the table's last column; the two inactivity timeouts take their seconds
  from the limits the server is built with (`protocol::INACTIVITY`, `IDLE_INACTIVITY`).
- A ban shows nothing of its own. An IP-banned address is told nothing and never answered again
  (as before), so its game ends as any game whose link went silent, with
  `Reason: Connection timed out`; a matchmaking shadowban's lost connection reads the same, as
  does every close the server gives no reason of the player's own doing for.

The server's limits need no new game: every game connected to a server with them is held to
them. The reason lines need the new game.

## Left as it is

- A participant of a challenge in play is still kicked from the game, after five silent seconds,
  whatever stopped it; the other players' rounds wait that long for it.
- A player whose address changes (a new IP or port mid-game) is still dropped after the
  transport's 10 seconds: a connection is tied to the address it was made from.

## Verified

- The server's tests (`crates/skate2-net/src/tests.rs`): a challenge's participant whose game
  stops is kicked after five seconds and told why; in lobby skate a game that stops for 2.5, 9 and
  30 seconds stays in and one stopped for a minute is kicked with the minute's reason; a link
  down for five seconds loses no one and one that stays down is lost after ten; a member waiting
  out a challenge is kept while its game stops; a banned player's connection hears nothing and
  ends as a timeout after ten seconds.
- The notices' tests (`online_menu::notice_tests`): every game lost names its reason; the two
  timeouts read the server's limits; a ban's close and every close the game knows no reason for
  read `Reason: Connection timed out`.
- On a local server: a game killed while it played a Spot Battle was kicked five seconds later
  (`sent nothing for 5 s: kicked for inactivity (reason 5)`), its connection closed as lost at
  ten.
- Not seen on screen: the notice popups with their reason lines.
