//! The online protocol's messages. The lobby half follows the original `Sk8::Net::Message`
//! set (`Message::Create` 0x82a6d350: `GameReset`, `GameRequest`, `GameAttributes`,
//! `GameResults`, `GameFinalResults`, `GameTimer`, `GameSyncPoint`, ...) with the server in
//! the role of EA's "RB" game server; matchmaking stands in for FESL PlayNow
//! (`StateOnlinePlayNow::DoPlayNow` 0x827736b8), and the recipes, replay frames and challenge
//! facts replace the lockstep's `GameSync`.

use crate::codec::{DecodeError, DecodeResult, Reader, Writer};
use crate::frames::{EncodedRun, MAX_RUN};

/// The most players a game holds: an unranked game's `MaxPlayerFilter` (`mMaxPlayers` 8).
pub const MAX_PLAYERS: usize = 8;
/// The most games one lobby browser listing carries.
pub const MAX_LOBBIES: usize = 64;
/// The longest nickname in bytes.
pub const MAX_NAME: usize = 64;
/// The longest recipe a player uploads.
pub const MAX_RECIPE: usize = 128 * 1024;
/// The longest challenge facts record of one tick.
pub const MAX_FACTS: usize = 16 * 1024;
/// The most online challenges one `Content` message names.
pub const MAX_CONTENT: usize = 4096;

/// `Sk8::Net::ChallengeType`.
pub mod challenge {
    pub const OWN_THE_SPOT: u8 = 0;
    pub const SLAP: u8 = 1;
    pub const BEST_TRICK: u8 = 2;
    pub const JAM: u8 = 3;
    pub const ONE_UP: u8 = 4;
    pub const SKATE: u8 = 5;
    pub const DEATH_RACE: u8 = 6;
    pub const HALL_OF_MEAT: u8 = 7;
    pub const ONLINE_FREE_SKATE: u8 = 8;
    pub const COUNT: u8 = 9;

    /// The fewest participants a game of `challenge_type` starts its challenge with: two
    /// (`GoToStartChallenge` 0x826617a0), or one for Spot Battle and Hall of Meat, which a player
    /// can play alone for a record (PC port addition).
    #[must_use]
    pub const fn min_participants(challenge_type: u8) -> usize {
        match challenge_type {
            OWN_THE_SPOT | HALL_OF_MEAT => 1,
            _ => 2,
        }
    }
}

/// `Sk8::Net::RequestType` of `GameRequest`.
pub mod request {
    pub const START_GAME: u8 = 1;
    pub const CHANGE_PRIVATE_STATUS: u8 = 3;
    pub const KICK_PLAYER: u8 = 4;
    pub const PAUSE: u8 = 5;
    pub const HEARTBEAT: u8 = 7;
}

/// `GameReset`'s reset type.
pub mod reset {
    pub const LOBBY_SKATE: u8 = 0;
    pub const LOAD_CHALLENGE: u8 = 1;
    pub const CHALLENGE: u8 = 2;
}

/// `Message_GameProposal::ProposalType`: a freeskate activity proposed in an online free skate.
pub mod proposal {
    /// A member proposes the activity (`GoToStartFreeskateActivity` 0x8266c728).
    pub const PROPOSE: u8 = 0;
    /// A member accepts it (`FrontEndState_FreeskateActivitySignUp::ReceiveInput` 0x826a5648),
    /// and the server tells every member who did.
    pub const ACCEPT: u8 = 1;
    /// The server proposes it to every member, with its proposer and the time left.
    pub const BROADCAST: u8 = 2;
    /// The time ran out before every member it was proposed to accepted it.
    pub const TIMEOUT: u8 = 3;
    /// The activity started without the member, who was still on its way into the game's world.
    pub const MISSED: u8 = 4;
}

/// `Sk8::Net::TimerType` of `GameTimer`.
pub mod timer {
    pub const LOCK_TIME: u8 = 1;
    pub const START_TIME: u8 = 2;
    pub const HURRY_TIME: u8 = 3;
    pub const DISPLAY_RESULTS: u8 = 4;
}

/// What became of a matchmaking request (`OE_PlayNowResult`, `SER_PLAYNOW_*`).
pub mod play_now {
    pub const JOINED: u8 = 0;
    pub const NO_MATCH: u8 = 1;
    pub const NO_SERVER: u8 = 2;
    pub const CANCELED: u8 = 3;
    pub const FULL: u8 = 4;
}

/// Why a player left a game (`GameRemovePlayer`, `KickReason`).
pub mod leave {
    pub const QUIT: u8 = 0;
    pub const LOST_CONNECTION: u8 = 1;
    pub const KICKED_BY_HOST: u8 = 4;
    pub const KICKED_FOR_INACTIVITY: u8 = 5;
    /// Asked what only the game's host may while another member was the host: the server
    /// dropped the connection.
    pub const KICKED_FOR_HOST_REQUEST: u8 = 6;
    /// Signed in with a nickname the server does not permit: one the game's prompt cannot make,
    /// or one holding a word of the server's list (`badwords.txt`).
    pub const KICKED_FOR_NICKNAME: u8 = 7;
    /// Asked for something before signing in.
    pub const KICKED_BEFORE_SIGN_IN: u8 = 8;
    /// Sent a chat line the game's chat prompt cannot make.
    pub const KICKED_FOR_CHAT: u8 = 9;
    /// Sent reliable messages far faster than the game does.
    pub const KICKED_FOR_FLOOD: u8 = 10;
}

/// Why the server refused a client.
pub mod refuse {
    /// Another protocol version.
    pub const VERSION: u8 = 1;
    pub const FULL: u8 = 2;
    /// Another build of the game: the player has to update.
    pub const BUILD: u8 = 4;
}

/// The longest nickname, in characters: where the game's nickname prompt stops.
pub const NICKNAME_CHARACTERS: usize = 16;

/// Whether a nickname may hold the character: a letter, a digit, a space, `-`, `_` or `'`.
#[must_use]
pub const fn nickname_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_' | '\'')
}

/// Whether a nickname is one the game's prompt makes: 1 to 16 [`nickname_character`]s, at least
/// one a letter or a digit, no space at either end (the game trims what was typed). The server
/// drops a connection that signs in with any other.
#[must_use]
pub fn valid_nickname(nickname: &str) -> bool {
    (1..=NICKNAME_CHARACTERS).contains(&nickname.len())
        && nickname.trim() == nickname
        && nickname.chars().all(nickname_character)
        && nickname.chars().any(|character| character.is_ascii_alphanumeric())
}

/// The longest chat line, in characters: where the game's chat prompt stops (PC port addition).
pub const CHAT_CHARACTERS: usize = 200;

/// Whether a chat line is one the game's chat prompt sends: 1 to 200 printable ASCII
/// characters or spaces, none at either end (the game trims what was typed). The server drops a
/// connection that sends any other.
#[must_use]
pub fn valid_chat(text: &str) -> bool {
    (1..=CHAT_CHARACTERS).contains(&text.len())
        && text.trim() == text
        && text.bytes().all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

/// The game's attributes (`Sk8::Net::AttributeType`, `GameAttributes`), typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameAttributes {
    pub game_version: String,
    pub challenge_type: u8,
    pub challenge_key: u64,
    pub private: bool,
    pub max_players: u8,
    pub ranked: bool,
}

impl GameAttributes {
    fn write(&self, w: &mut Writer) {
        w.string(&self.game_version);
        w.u8(self.challenge_type);
        w.u64(self.challenge_key);
        w.bool(self.private);
        w.u8(self.max_players);
        w.bool(self.ranked);
    }

    fn read(r: &mut Reader<'_>) -> DecodeResult<Self> {
        let attributes = Self {
            game_version: r.string(MAX_NAME)?,
            challenge_type: r.u8()?,
            challenge_key: r.u64()?,
            private: r.bool()?,
            max_players: r.u8()?,
            ranked: r.bool()?,
        };
        if attributes.challenge_type >= challenge::COUNT || attributes.max_players == 0 || usize::from(attributes.max_players) > MAX_PLAYERS {
            return Err(DecodeError("game attributes out of range"));
        }
        Ok(attributes)
    }
}

/// How a player looks for a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayNowMode {
    /// `DoQuickMatch` 0x827737d0: any public game of the ranking, never a new one.
    Quick,
    /// A custom match: `ChallengeTypeFilter` with a location preference, or a new game.
    Custom,
    /// `FrontEndState_UnrankedFreeskate`: an online free skate at a location.
    Freeskate,
}

/// `StateMessage_EnterOnlineGame` as `SendUnrankedMatchMessage` 0x826a85a0 builds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayNow {
    pub mode: PlayNowMode,
    pub ranked: bool,
    pub challenge_type: u8,
    /// The preferred challenge, or any.
    pub challenge_key: Option<u64>,
    pub private: bool,
    /// The custom match's Create rather than Join.
    pub create: bool,
    /// The challenge a new game starts with when none matches.
    pub fallback_key: u64,
    pub game_version: String,
}

/// One player of a game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerInfo {
    pub user_id: u64,
    pub slot: u8,
    pub name: String,
}

impl PlayerInfo {
    fn write(&self, w: &mut Writer) {
        w.u64(self.user_id);
        w.u8(self.slot);
        w.string(&self.name);
    }

    fn read(r: &mut Reader<'_>) -> DecodeResult<Self> {
        let info = Self { user_id: r.u64()?, slot: r.u8()?, name: r.string(MAX_NAME)? };
        if usize::from(info.slot) >= MAX_PLAYERS {
            return Err(DecodeError("slot out of range"));
        }
        Ok(info)
    }
}

/// `Sk8::Net::GameResult` (`GameResults`, `ReportLocalPlayerResults` 0x8274ce30).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GameResult {
    pub event_time: f32,
    pub score: i32,
    /// `eFinishReason`.
    pub finish_reason: u32,
    pub ranking: u16,
    pub players_choice: bool,
}

impl GameResult {
    fn write(&self, w: &mut Writer) {
        w.f32(self.event_time);
        w.signed(i64::from(self.score));
        w.varint(u64::from(self.finish_reason));
        w.u16(self.ranking);
        w.bool(self.players_choice);
    }

    fn read(r: &mut Reader<'_>) -> DecodeResult<Self> {
        Ok(Self {
            event_time: r.f32()?,
            score: i32::try_from(r.signed()?).map_err(|_| DecodeError("score"))?,
            finish_reason: r.varint_u32()?,
            ranking: r.u16()?,
            players_choice: r.bool()?,
        })
    }
}

/// A row of `GameFinalResults` (`Sk8::Net::PlayerStatsData`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerStats {
    pub user_id: u64,
    pub result: GameResult,
    pub cash: i32,
    pub exp: f32,
    pub wager: i32,
    pub winnings: i32,
}

/// The phase a game is in, for a player who joins it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    LobbySkate,
    LoadingChallenge,
    Challenge,
    Results,
}

impl Phase {
    const fn code(self) -> u8 {
        match self {
            Self::LobbySkate => 0,
            Self::LoadingChallenge => 1,
            Self::Challenge => 2,
            Self::Results => 3,
        }
    }

    fn from_code(code: u8) -> DecodeResult<Self> {
        Ok(match code {
            0 => Self::LobbySkate,
            1 => Self::LoadingChallenge,
            2 => Self::Challenge,
            3 => Self::Results,
            _ => return Err(DecodeError("phase")),
        })
    }
}

/// One game as the lobby browser lists it (PC port addition): its id, challenge type and
/// challenge (where a location change is taking it), its players, the most it holds and its
/// phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LobbyInfo {
    pub game: u32,
    pub challenge_type: u8,
    pub challenge_key: u64,
    pub players: u8,
    pub max_players: u8,
    pub phase: Phase,
}

impl LobbyInfo {
    fn write(&self, w: &mut Writer) {
        w.u32(self.game);
        w.u8(self.challenge_type);
        w.u64(self.challenge_key);
        w.u8(self.players);
        w.u8(self.max_players);
        w.u8(self.phase.code());
    }

    fn read(r: &mut Reader<'_>) -> DecodeResult<Self> {
        let lobby = Self {
            game: r.u32()?,
            challenge_type: r.u8()?,
            challenge_key: r.u64()?,
            players: r.u8()?,
            max_players: r.u8()?,
            phase: Phase::from_code(r.u8()?)?,
        };
        if lobby.challenge_type >= challenge::COUNT || lobby.max_players == 0 || usize::from(lobby.max_players) > MAX_PLAYERS || lobby.players > lobby.max_players {
            return Err(DecodeError("lobby out of range"));
        }
        Ok(lobby)
    }
}

/// A presentation event the console of a round's pacing leader saw, numbered in its kind's order
/// (every console sees the same ones in the same order, each at a time of its own).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presented {
    /// The round's `seq`-th NIS event: `cMsgNISChallengeNotify` 1 (the scene's end, with
    /// `cMsgNISComplete`), 2 or 3.
    Nis { seq: u32, notification: u8 },
    /// The `seq`-th end of the speech line (`cMsgSpeechEnded`).
    Speech { character: i32, id: i32, seq: u32 },
}

/// At most this many presentation events go with one tick's facts.
pub const MAX_PRESENTED: usize = 15;

/// What the player whose facts pace a tick tells every console with them: the votes the
/// players watching cast (player index, score) and the players who left while not sending
/// facts (player index), each counted at that tick; whether the player waits at a round's sync
/// point from this tick, standing still at these facts until every player has reached it; and
/// the presentation events the pacing leader's console saw, for every console to take at this
/// tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactsNotes {
    pub votes: Vec<(u8, i32)>,
    pub left: Vec<u8>,
    pub syncing: bool,
    pub presented: Vec<Presented>,
}

impl FactsNotes {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.votes.is_empty() && self.left.is_empty() && !self.syncing && self.presented.is_empty()
    }

    fn write(&self, w: &mut Writer) {
        let count = |n: usize, max: usize| u64::try_from(n.min(max)).unwrap_or(0);
        w.varint(
            count(self.votes.len(), MAX_PLAYERS)
                | (count(self.left.len(), MAX_PLAYERS) << 4)
                | (u64::from(self.syncing) << 8)
                | (count(self.presented.len(), MAX_PRESENTED) << 9),
        );
        for &(player, score) in self.votes.iter().take(MAX_PLAYERS) {
            w.u8(player);
            w.signed(i64::from(score));
        }
        for &player in self.left.iter().take(MAX_PLAYERS) {
            w.u8(player);
        }
        for &presented in self.presented.iter().take(MAX_PRESENTED) {
            match presented {
                Presented::Nis { seq, notification } => {
                    w.u8(notification);
                    w.varint(u64::from(seq));
                }
                Presented::Speech { character, id, seq } => {
                    w.u8(0);
                    w.varint(u64::from(seq));
                    w.signed(i64::from(character));
                    w.signed(i64::from(id));
                }
            }
        }
    }

    fn read(r: &mut Reader<'_>) -> DecodeResult<Self> {
        let counts = r.varint_u32()?;
        let field = |shift: u32, mask: u32| usize::try_from((counts >> shift) & mask).unwrap_or(usize::MAX);
        let (votes, left, syncing, presented) = (field(0, 0xf), field(4, 0xf), field(8, 1), field(9, 0xf));
        if votes > MAX_PLAYERS || left > MAX_PLAYERS || counts >> 13 != 0 {
            return Err(DecodeError("facts notes"));
        }
        let player = |r: &mut Reader<'_>| -> DecodeResult<u8> {
            let player = r.u8()?;
            if usize::from(player) < MAX_PLAYERS { Ok(player) } else { Err(DecodeError("player out of range")) }
        };
        let votes = (0..votes)
            .map(|_| Ok((player(r)?, i32::try_from(r.signed()?).map_err(|_| DecodeError("vote"))?)))
            .collect::<DecodeResult<_>>()?;
        let left = (0..left).map(|_| player(r)).collect::<DecodeResult<_>>()?;
        let presented = (0..presented)
            .map(|_| {
                let notification = r.u8()?;
                let seq = r.varint_u32()?;
                if notification != 0 {
                    return Ok(Presented::Nis { seq, notification });
                }
                let value = |r: &mut Reader<'_>| i32::try_from(r.signed()?).map_err(|_| DecodeError("speech line"));
                Ok(Presented::Speech { character: value(r)?, id: value(r)?, seq })
            })
            .collect::<DecodeResult<_>>()?;
        Ok(Self { votes, left, syncing: syncing == 1, presented })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClientMessage {
    /// Sign-in: the player's nickname and build.
    Hello { nickname: String, game_version: String },
    PlayNow(PlayNow),
    /// `StateOnlinePlayNow::CancelPlayNow` 0x82748f40.
    CancelPlayNow,
    /// `IE_LeaveGame`.
    LeaveGame,
    /// The player's appearance, uploaded as the game is entered (`StateOnlineRecipe::
    /// DoUploadRecipe` 0x8275ee18), and again after the pre-game Edit Skater (PC port addition).
    Recipe(Vec<u8>),
    /// `Message_GameRequest`.
    GameRequest { request: u8, value: u64 },
    /// The game host peer's next challenge (`GameResetAttributes`).
    ResetAttributes(GameAttributes),
    /// The game host peer's change of challenge or location (`GameRequestChange`).
    RequestChange { challenge_type: u8, challenge_key: u64 },
    /// `GameChallengeLoaded`.
    ChallengeLoaded,
    /// `GameSyncPoint` (`Online.OnlineChallengeRoundSyncPoint` 0x8289adb8).
    SyncPoint { round: u32 },
    /// `GameComplete`.
    Complete,
    /// `GameResults`.
    Results(GameResult),
    /// One tick of the facts the player's skater gave the challenge, in the run of ticks it sends
    /// facts for (`stint`), with what it tells every console at the tick.
    Facts { tick: u32, stint: u32, notes: FactsNotes, data: Vec<u8> },
    /// A run of the player's replay frames.
    Frames(EncodedRun),
    /// The newest frames received of each other player: slot, latest tick, mask.
    FrameAcks(Vec<(u8, u32, u32)>),
    Ping { time: u64 },
    /// A vote pressed while watching another player's turn (`ChallengeVotingManager::Update`).
    Vote { score: i32 },
    /// The player is still in the game but has no replay frame to send (travelling, loading,
    /// paused at a sync point).
    Busy,
    /// A chat line typed in the game (T, PC port addition). The server relays it to the game's
    /// members once it holds the player's recipe and a replay frame (`ChatEnabled`).
    Chat(String),
    /// The lobby browser's listing of the public games (PC port addition).
    ListLobbies,
    /// The lobby browser's A on a listed game (PC port addition).
    JoinLobby { game: u32 },
    /// The pause menu's Leave Challenge (PC port addition): the participant leaves the challenge
    /// in progress and waits in its pre-game lobby until the game is back in lobby skate.
    LeaveChallenge,
    /// `Message_GameProposal` of a freeskate activity: [`proposal::PROPOSE`] or
    /// [`proposal::ACCEPT`].
    GameProposal { kind: u8, activity: u64 },
    /// The online challenges the build has, by key, sent once signed in (PC port addition, as
    /// `ChallengeKeyPreference` 0x8276c678 fits the search to the console's own challenges):
    /// matchmaking and the lobby browser then offer only games at them.
    Content(Vec<u64>),
    /// The console's S.K.A.T.E. round has waited for one player's facts at a tick for seconds
    /// (PC port addition), sent again every second while the wait goes on: once every other
    /// participant, at least two, has a fresh report of the same player at the same tick, the
    /// server takes that player out of the challenge.
    Stalled { user_id: u64, tick: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerMessage {
    Welcome { user_id: u64 },
    Refused { reason: u8 },
    PlayNowResult { result: u8 },
    /// The game a player joined: its id, the player's slot, attributes, players in join order
    /// (the first is the game host peer, `Net::GetGameHostPeerID` 0x82641098), phase, and the
    /// members playing the challenge in progress (none in lobby skate).
    GameJoined { game: u32, slot: u8, attributes: GameAttributes, players: Vec<PlayerInfo>, phase: Phase, participants: Vec<u64> },
    GameAttributes(GameAttributes),
    /// `GameAttributeUpdate` of `IsPrivate`.
    PrivacyChanged { private: bool },
    PlayerJoin(PlayerInfo),
    /// `GameRemovePlayer`.
    PlayerLeave { user_id: u64, reason: u8 },
    Recipe { user_id: u64, data: Vec<u8> },
    /// `GameReset`: the reset type, the shared random seed, the session, the challenge's
    /// participants by player index (0 past the last) and the freeskate activity the challenge is
    /// (`mActivity`; 0 for the game's own challenge).
    GameReset { kind: u8, seed: u32, session: u32, participants: [u64; MAX_PLAYERS], activity: u64 },
    SyncPoint { round: u32 },
    AllPlayersComplete,
    FinalResults(Vec<PlayerStats>),
    GameTimer { kind: u8, value: u64 },
    ExitPostChallenge,
    /// `GameChange`: the change and the milliseconds until it happens.
    GameChange { challenge_type: u8, challenge_key: u64, delay: u32 },
    /// `GameRequest(KickPlayer)` sent to the target.
    Kicked { reason: u8 },
    /// A player's facts and replay frames, the player named by its slot in the game.
    Facts { slot: u8, tick: u32, stint: u32, notes: FactsNotes, data: Vec<u8> },
    RemoteFrames { slot: u8, run: EncodedRun },
    /// The newest frames the server received of this player's own.
    FrameAck { latest: u32, mask: u32 },
    Pong { time: u64 },
    /// Another player's vote press.
    Vote { user_id: u64, score: i32 },
    /// The server holds this player's recipe and a replay frame: its chat lines go out now.
    ChatEnabled,
    /// A member's chat line, to every member of its game, the sender too.
    Chat { user_id: u64, text: String },
    /// The public games, for the lobby browser (PC port addition).
    Lobbies(Vec<LobbyInfo>),
    /// A participant left the challenge for its pre-game lobby (`LeaveChallenge`, PC port
    /// addition), or every other participant's S.K.A.T.E. round waited on it (`Stalled`), to every
    /// member of the game.
    ChallengeLeave { user_id: u64 },
    /// `Message_GameProposal` of a freeskate activity, [`proposal::ACCEPT`] to
    /// [`proposal::MISSED`]: the member it is about (the proposer of a broadcast, the member who
    /// accepted) and the milliseconds a broadcast leaves to accept it.
    GameProposal { kind: u8, activity: u64, user_id: u64, timeout: u32 },
}

/// A run: its first tick and length, the frames it is coded against as their distance back from
/// that tick (0 for none), then its payload to the end of the message.
fn write_run(w: &mut Writer, run: &EncodedRun) {
    w.varint(u64::from(run.tick));
    w.u8(run.count);
    for reference in [run.frame, run.frame_before, run.sample, run.sample_before] {
        w.varint(reference.map_or(0, |reference| u64::from(run.tick.wrapping_sub(reference))));
    }
    w.raw(&run.payload);
}

fn read_run(r: &mut Reader<'_>) -> DecodeResult<EncodedRun> {
    let tick = r.varint_u32()?;
    let count = r.u8()?;
    if count == 0 || usize::from(count) > MAX_RUN {
        return Err(DecodeError("run length"));
    }
    let mut reference = || -> DecodeResult<Option<u32>> {
        let distance = r.varint_u32()?;
        Ok((distance != 0).then(|| tick.wrapping_sub(distance)))
    };
    let frame = reference()?;
    let frame_before = reference()?;
    let sample = reference()?;
    let sample_before = reference()?;
    let payload = r.raw(r.remaining())?.to_vec();
    Ok(EncodedRun { tick, count, frame, frame_before, sample, sample_before, payload })
}

fn read_slot(r: &mut Reader<'_>) -> DecodeResult<u8> {
    let slot = r.u8()?;
    if usize::from(slot) < MAX_PLAYERS { Ok(slot) } else { Err(DecodeError("slot out of range")) }
}

impl ClientMessage {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        match self {
            Self::Hello { nickname, game_version } => {
                w.u8(0);
                w.string(nickname);
                w.string(game_version);
            }
            Self::PlayNow(play) => {
                w.u8(1);
                w.u8(match play.mode {
                    PlayNowMode::Quick => 0,
                    PlayNowMode::Custom => 1,
                    PlayNowMode::Freeskate => 2,
                });
                w.bool(play.ranked);
                w.u8(play.challenge_type);
                w.bool(play.challenge_key.is_some());
                w.u64(play.challenge_key.unwrap_or(0));
                w.bool(play.private);
                w.bool(play.create);
                w.u64(play.fallback_key);
                w.string(&play.game_version);
            }
            Self::CancelPlayNow => w.u8(2),
            Self::LeaveGame => w.u8(3),
            Self::Recipe(data) => {
                w.u8(4);
                w.bytes(data);
            }
            Self::GameRequest { request, value } => {
                w.u8(5);
                w.u8(*request);
                w.u64(*value);
            }
            Self::ResetAttributes(attributes) => {
                w.u8(6);
                attributes.write(&mut w);
            }
            Self::RequestChange { challenge_type, challenge_key } => {
                w.u8(7);
                w.u8(*challenge_type);
                w.u64(*challenge_key);
            }
            Self::ChallengeLoaded => w.u8(8),
            Self::SyncPoint { round } => {
                w.u8(9);
                w.varint(u64::from(*round));
            }
            Self::Complete => w.u8(10),
            Self::Results(result) => {
                w.u8(11);
                result.write(&mut w);
            }
            Self::Facts { tick, stint, notes, data } => {
                w.u8(12);
                w.varint(u64::from(*tick));
                w.varint(u64::from(*stint));
                notes.write(&mut w);
                w.bytes(data);
            }
            Self::Frames(run) => {
                w.u8(13);
                write_run(&mut w, run);
            }
            Self::FrameAcks(acks) => {
                w.u8(14);
                w.varint(acks.len() as u64);
                for &(slot, latest, mask) in acks {
                    w.u8(slot);
                    w.varint(u64::from(latest));
                    w.u32(mask);
                }
            }
            Self::Ping { time } => {
                w.u8(15);
                w.u64(*time);
            }
            Self::Vote { score } => {
                w.u8(16);
                w.signed(i64::from(*score));
            }
            Self::Busy => w.u8(17),
            Self::Chat(text) => {
                w.u8(18);
                w.string(text);
            }
            Self::ListLobbies => w.u8(19),
            Self::JoinLobby { game } => {
                w.u8(20);
                w.u32(*game);
            }
            Self::LeaveChallenge => w.u8(21),
            Self::GameProposal { kind, activity } => {
                w.u8(22);
                w.u8(*kind);
                w.u64(*activity);
            }
            Self::Content(challenges) => {
                w.u8(23);
                w.varint(challenges.len() as u64);
                for &challenge in challenges {
                    w.u64(challenge);
                }
            }
            Self::Stalled { user_id, tick } => {
                w.u8(24);
                w.u64(*user_id);
                w.u32(*tick);
            }
        }
        w.into_vec()
    }

    /// # Errors
    /// A malformed message.
    pub fn decode(bytes: &[u8]) -> DecodeResult<Self> {
        let mut r = Reader::new(bytes);
        let message = match r.u8()? {
            0 => Self::Hello { nickname: r.string(MAX_NAME)?, game_version: r.string(MAX_NAME)? },
            1 => {
                let mode = match r.u8()? {
                    0 => PlayNowMode::Quick,
                    1 => PlayNowMode::Custom,
                    2 => PlayNowMode::Freeskate,
                    _ => return Err(DecodeError("play now mode")),
                };
                let ranked = r.bool()?;
                let challenge_type = r.u8()?;
                if challenge_type >= challenge::COUNT {
                    return Err(DecodeError("challenge type"));
                }
                let has_key = r.bool()?;
                let key = r.u64()?;
                Self::PlayNow(PlayNow {
                    mode,
                    ranked,
                    challenge_type,
                    challenge_key: has_key.then_some(key),
                    private: r.bool()?,
                    create: r.bool()?,
                    fallback_key: r.u64()?,
                    game_version: r.string(MAX_NAME)?,
                })
            }
            2 => Self::CancelPlayNow,
            3 => Self::LeaveGame,
            4 => Self::Recipe(r.bytes(MAX_RECIPE)?.to_vec()),
            5 => Self::GameRequest { request: r.u8()?, value: r.u64()? },
            6 => Self::ResetAttributes(GameAttributes::read(&mut r)?),
            7 => {
                let challenge_type = r.u8()?;
                if challenge_type >= challenge::COUNT {
                    return Err(DecodeError("challenge type"));
                }
                Self::RequestChange { challenge_type, challenge_key: r.u64()? }
            }
            8 => Self::ChallengeLoaded,
            9 => Self::SyncPoint { round: r.varint_u32()? },
            10 => Self::Complete,
            11 => Self::Results(GameResult::read(&mut r)?),
            12 => Self::Facts { tick: r.varint_u32()?, stint: r.varint_u32()?, notes: FactsNotes::read(&mut r)?, data: r.bytes(MAX_FACTS)?.to_vec() },
            13 => return Ok(Self::Frames(read_run(&mut r)?)),
            14 => {
                let count = r.len(MAX_PLAYERS)?;
                let mut acks = Vec::with_capacity(count);
                for _ in 0..count {
                    let slot = r.u8()?;
                    if usize::from(slot) >= MAX_PLAYERS {
                        return Err(DecodeError("slot out of range"));
                    }
                    acks.push((slot, r.varint_u32()?, r.u32()?));
                }
                Self::FrameAcks(acks)
            }
            15 => Self::Ping { time: r.u64()? },
            16 => Self::Vote { score: i32::try_from(r.signed()?).map_err(|_| DecodeError("vote"))? },
            17 => Self::Busy,
            18 => Self::Chat(r.string(CHAT_CHARACTERS)?),
            19 => Self::ListLobbies,
            20 => Self::JoinLobby { game: r.u32()? },
            21 => Self::LeaveChallenge,
            22 => {
                let kind = r.u8()?;
                if !matches!(kind, proposal::PROPOSE | proposal::ACCEPT) {
                    return Err(DecodeError("proposal type"));
                }
                Self::GameProposal { kind, activity: r.u64()? }
            }
            23 => {
                let count = r.len(MAX_CONTENT)?;
                let mut challenges = Vec::with_capacity(count);
                for _ in 0..count {
                    challenges.push(r.u64()?);
                }
                Self::Content(challenges)
            }
            24 => Self::Stalled { user_id: r.u64()?, tick: r.u32()? },
            _ => return Err(DecodeError("unknown client message")),
        };
        r.finish()?;
        Ok(message)
    }
}

impl ServerMessage {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        match self {
            Self::Welcome { user_id } => {
                w.u8(0);
                w.u64(*user_id);
            }
            Self::Refused { reason } => {
                w.u8(1);
                w.u8(*reason);
            }
            Self::PlayNowResult { result } => {
                w.u8(2);
                w.u8(*result);
            }
            Self::GameJoined { game, slot, attributes, players, phase, participants } => {
                w.u8(3);
                w.u32(*game);
                w.u8(*slot);
                attributes.write(&mut w);
                w.varint(players.len() as u64);
                for player in players {
                    player.write(&mut w);
                }
                w.u8(phase.code());
                w.varint(participants.len() as u64);
                for participant in participants {
                    w.u64(*participant);
                }
            }
            Self::GameAttributes(attributes) => {
                w.u8(4);
                attributes.write(&mut w);
            }
            Self::PrivacyChanged { private } => {
                w.u8(5);
                w.bool(*private);
            }
            Self::PlayerJoin(player) => {
                w.u8(6);
                player.write(&mut w);
            }
            Self::PlayerLeave { user_id, reason } => {
                w.u8(7);
                w.u64(*user_id);
                w.u8(*reason);
            }
            Self::Recipe { user_id, data } => {
                w.u8(8);
                w.u64(*user_id);
                w.bytes(data);
            }
            Self::GameReset { kind, seed, session, participants, activity } => {
                w.u8(9);
                w.u8(*kind);
                w.u32(*seed);
                w.u32(*session);
                for participant in participants {
                    w.u64(*participant);
                }
                w.u64(*activity);
            }
            Self::SyncPoint { round } => {
                w.u8(10);
                w.varint(u64::from(*round));
            }
            Self::AllPlayersComplete => w.u8(11),
            Self::FinalResults(rows) => {
                w.u8(12);
                w.varint(rows.len() as u64);
                for row in rows {
                    w.u64(row.user_id);
                    row.result.write(&mut w);
                    w.signed(i64::from(row.cash));
                    w.f32(row.exp);
                    w.signed(i64::from(row.wager));
                    w.signed(i64::from(row.winnings));
                }
            }
            Self::GameTimer { kind, value } => {
                w.u8(13);
                w.u8(*kind);
                w.u64(*value);
            }
            Self::ExitPostChallenge => w.u8(14),
            Self::GameChange { challenge_type, challenge_key, delay } => {
                w.u8(15);
                w.u8(*challenge_type);
                w.u64(*challenge_key);
                w.u32(*delay);
            }
            Self::Kicked { reason } => {
                w.u8(16);
                w.u8(*reason);
            }
            Self::Facts { slot, tick, stint, notes, data } => {
                w.u8(17);
                w.u8(*slot);
                w.varint(u64::from(*tick));
                w.varint(u64::from(*stint));
                notes.write(&mut w);
                w.bytes(data);
            }
            Self::RemoteFrames { slot, run } => {
                w.u8(18);
                w.u8(*slot);
                write_run(&mut w, run);
            }
            Self::FrameAck { latest, mask } => {
                w.u8(19);
                w.varint(u64::from(*latest));
                w.u32(*mask);
            }
            Self::Pong { time } => {
                w.u8(20);
                w.u64(*time);
            }
            Self::Vote { user_id, score } => {
                w.u8(21);
                w.u64(*user_id);
                w.signed(i64::from(*score));
            }
            Self::ChatEnabled => w.u8(22),
            Self::Chat { user_id, text } => {
                w.u8(23);
                w.u64(*user_id);
                w.string(text);
            }
            Self::Lobbies(lobbies) => {
                w.u8(24);
                let listed = &lobbies[..lobbies.len().min(MAX_LOBBIES)];
                w.varint(listed.len() as u64);
                for lobby in listed {
                    lobby.write(&mut w);
                }
            }
            Self::ChallengeLeave { user_id } => {
                w.u8(25);
                w.u64(*user_id);
            }
            Self::GameProposal { kind, activity, user_id, timeout } => {
                w.u8(26);
                w.u8(*kind);
                w.u64(*activity);
                w.u64(*user_id);
                w.u32(*timeout);
            }
        }
        w.into_vec()
    }

    /// # Errors
    /// A malformed message.
    pub fn decode(bytes: &[u8]) -> DecodeResult<Self> {
        let mut r = Reader::new(bytes);
        let score = |value: i64| i32::try_from(value).map_err(|_| DecodeError("value out of range"));
        let message = match r.u8()? {
            0 => Self::Welcome { user_id: r.u64()? },
            1 => Self::Refused { reason: r.u8()? },
            2 => Self::PlayNowResult { result: r.u8()? },
            3 => {
                let game = r.u32()?;
                let slot = r.u8()?;
                if usize::from(slot) >= MAX_PLAYERS {
                    return Err(DecodeError("slot out of range"));
                }
                let attributes = GameAttributes::read(&mut r)?;
                let count = r.len(MAX_PLAYERS)?;
                let players = (0..count).map(|_| PlayerInfo::read(&mut r)).collect::<DecodeResult<Vec<_>>>()?;
                let phase = Phase::from_code(r.u8()?)?;
                let count = r.len(MAX_PLAYERS)?;
                let participants = (0..count).map(|_| r.u64()).collect::<DecodeResult<Vec<_>>>()?;
                Self::GameJoined { game, slot, attributes, players, phase, participants }
            }
            4 => Self::GameAttributes(GameAttributes::read(&mut r)?),
            5 => Self::PrivacyChanged { private: r.bool()? },
            6 => Self::PlayerJoin(PlayerInfo::read(&mut r)?),
            7 => Self::PlayerLeave { user_id: r.u64()?, reason: r.u8()? },
            8 => Self::Recipe { user_id: r.u64()?, data: r.bytes(MAX_RECIPE)?.to_vec() },
            9 => {
                let kind = r.u8()?;
                if kind > reset::CHALLENGE {
                    return Err(DecodeError("reset type"));
                }
                let seed = r.u32()?;
                let session = r.u32()?;
                let mut participants = [0; MAX_PLAYERS];
                for participant in &mut participants {
                    *participant = r.u64()?;
                }
                Self::GameReset { kind, seed, session, participants, activity: r.u64()? }
            }
            10 => Self::SyncPoint { round: r.varint_u32()? },
            11 => Self::AllPlayersComplete,
            12 => {
                let count = r.len(MAX_PLAYERS)?;
                let mut rows = Vec::with_capacity(count);
                for _ in 0..count {
                    rows.push(PlayerStats {
                        user_id: r.u64()?,
                        result: GameResult::read(&mut r)?,
                        cash: score(r.signed()?)?,
                        exp: r.f32()?,
                        wager: score(r.signed()?)?,
                        winnings: score(r.signed()?)?,
                    });
                }
                Self::FinalResults(rows)
            }
            13 => Self::GameTimer { kind: r.u8()?, value: r.u64()? },
            14 => Self::ExitPostChallenge,
            15 => Self::GameChange { challenge_type: r.u8()?, challenge_key: r.u64()?, delay: r.u32()? },
            16 => Self::Kicked { reason: r.u8()? },
            17 => Self::Facts { slot: read_slot(&mut r)?, tick: r.varint_u32()?, stint: r.varint_u32()?, notes: FactsNotes::read(&mut r)?, data: r.bytes(MAX_FACTS)?.to_vec() },
            18 => {
                let slot = read_slot(&mut r)?;
                return Ok(Self::RemoteFrames { slot, run: read_run(&mut r)? });
            }
            19 => Self::FrameAck { latest: r.varint_u32()?, mask: r.u32()? },
            20 => Self::Pong { time: r.u64()? },
            21 => Self::Vote { user_id: r.u64()?, score: i32::try_from(r.signed()?).map_err(|_| DecodeError("vote"))? },
            22 => Self::ChatEnabled,
            23 => Self::Chat { user_id: r.u64()?, text: r.string(CHAT_CHARACTERS)? },
            24 => {
                let count = r.len(MAX_LOBBIES)?;
                Self::Lobbies((0..count).map(|_| LobbyInfo::read(&mut r)).collect::<DecodeResult<_>>()?)
            }
            25 => Self::ChallengeLeave { user_id: r.u64()? },
            26 => {
                let kind = r.u8()?;
                if kind > proposal::MISSED {
                    return Err(DecodeError("proposal type"));
                }
                Self::GameProposal { kind, activity: r.u64()?, user_id: r.u64()?, timeout: r.u32()? }
            }
            _ => return Err(DecodeError("unknown server message")),
        };
        r.finish()?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes() -> GameAttributes {
        GameAttributes {
            game_version: "skate2-pc".into(),
            challenge_type: challenge::SKATE,
            challenge_key: 0x0123_4567_89ab_cdef,
            private: false,
            max_players: 6,
            ranked: false,
        }
    }

    #[test]
    fn client_messages_round_trip() {
        let messages = vec![
            ClientMessage::Hello { nickname: "Skater99".into(), game_version: "v1".into() },
            ClientMessage::PlayNow(PlayNow {
                mode: PlayNowMode::Custom,
                ranked: false,
                challenge_type: challenge::SKATE,
                challenge_key: Some(42),
                private: true,
                create: true,
                fallback_key: 42,
                game_version: "v1".into(),
            }),
            ClientMessage::CancelPlayNow,
            ClientMessage::LeaveGame,
            ClientMessage::Recipe(vec![1, 2, 3]),
            ClientMessage::GameRequest { request: request::KICK_PLAYER, value: 77 },
            ClientMessage::ResetAttributes(attributes()),
            ClientMessage::RequestChange { challenge_type: challenge::JAM, challenge_key: 9 },
            ClientMessage::ChallengeLoaded,
            ClientMessage::SyncPoint { round: 3 },
            ClientMessage::Complete,
            ClientMessage::Results(GameResult { event_time: 12.5, score: -3, finish_reason: 1, ranking: 2, players_choice: true }),
            ClientMessage::Facts { tick: 600, stint: 2, notes: FactsNotes::default(), data: vec![9; 40] },
            ClientMessage::Facts { tick: 601, stint: 3, notes: FactsNotes { votes: vec![(1, 3), (2, -1)], left: vec![4], syncing: false, presented: vec![Presented::Nis { seq: 7, notification: 1 }, Presented::Speech { character: 40, id: -4004, seq: 2 }] }, data: vec![] },
            ClientMessage::Frames(EncodedRun { tick: 70, count: 3, frame: Some(66), frame_before: Some(65), sample: None, sample_before: Some(60), payload: vec![5, 6, 7] }),
            ClientMessage::FrameAcks(vec![(1, 500, 0xffff), (3, 2, 0)]),
            ClientMessage::Ping { time: 123_456 },
            ClientMessage::Vote { score: 2 },
            ClientMessage::Busy,
            ClientMessage::Chat("gg, one more?".into()),
            ClientMessage::ListLobbies,
            ClientMessage::JoinLobby { game: 41 },
            ClientMessage::LeaveChallenge,
            ClientMessage::GameProposal { kind: proposal::PROPOSE, activity: 0x433d_b94f_fbfa_c4a9 },
            ClientMessage::GameProposal { kind: proposal::ACCEPT, activity: 7 },
            ClientMessage::Content(vec![0x433d_b94f_fbfa_c4a9, 7]),
            ClientMessage::Content(Vec::new()),
            ClientMessage::Stalled { user_id: 0x1234_5678_9abc_def0, tick: (3 << 20) + 77 },
        ];
        for message in messages {
            assert_eq!(ClientMessage::decode(&message.encode()).unwrap(), message);
        }
    }

    #[test]
    fn server_messages_round_trip() {
        let player = PlayerInfo { user_id: 5, slot: 1, name: "Rob".into() };
        let messages = vec![
            ServerMessage::Welcome { user_id: 5 },
            ServerMessage::Refused { reason: refuse::VERSION },
            ServerMessage::PlayNowResult { result: play_now::NO_MATCH },
            ServerMessage::GameJoined { game: 3, slot: 1, attributes: attributes(), players: vec![player.clone()], phase: Phase::Challenge, participants: vec![2, 9] },
            ServerMessage::GameJoined { game: 4, slot: 0, attributes: attributes(), players: vec![], phase: Phase::LobbySkate, participants: vec![] },
            ServerMessage::GameAttributes(attributes()),
            ServerMessage::PrivacyChanged { private: true },
            ServerMessage::PlayerJoin(player),
            ServerMessage::PlayerLeave { user_id: 5, reason: leave::LOST_CONNECTION },
            ServerMessage::Recipe { user_id: 5, data: vec![0; 2000] },
            ServerMessage::GameReset { kind: reset::CHALLENGE, seed: 99, session: 4, participants: [1, 2, 0, 0, 0, 0, 0, 0], activity: 0 },
            ServerMessage::GameReset { kind: reset::LOAD_CHALLENGE, seed: 3, session: 5, participants: [9, 8, 7, 0, 0, 0, 0, 0], activity: 0x71fa_483a_08de_e1b7 },
            ServerMessage::SyncPoint { round: 1 },
            ServerMessage::AllPlayersComplete,
            ServerMessage::FinalResults(vec![PlayerStats { user_id: 1, cash: 300, exp: 1.5, wager: -1, ..Default::default() }]),
            ServerMessage::GameTimer { kind: timer::DISPLAY_RESULTS, value: 10 },
            ServerMessage::ExitPostChallenge,
            ServerMessage::GameChange { challenge_type: challenge::JAM, challenge_key: 5, delay: 5000 },
            ServerMessage::Kicked { reason: leave::KICKED_BY_HOST },
            ServerMessage::Facts { slot: 2, tick: 10, stint: 1, notes: FactsNotes { votes: vec![(0, 1)], left: vec![], syncing: true, presented: vec![] }, data: vec![1] },
            ServerMessage::RemoteFrames { slot: 5, run: EncodedRun { tick: 9, count: 1, frame: None, frame_before: None, sample: Some(8), sample_before: Some(6), payload: vec![] } },
            ServerMessage::FrameAck { latest: 44, mask: 3 },
            ServerMessage::Pong { time: 8 },
            ServerMessage::Vote { user_id: 3, score: 1 },
            ServerMessage::ChatEnabled,
            ServerMessage::Chat { user_id: 3, text: "nice line".into() },
            ServerMessage::Lobbies(vec![]),
            ServerMessage::Lobbies(vec![
                LobbyInfo { game: 7, challenge_type: challenge::OWN_THE_SPOT, challenge_key: 0x55, players: 3, max_players: 8, phase: Phase::LobbySkate },
                LobbyInfo { game: 9, challenge_type: challenge::ONLINE_FREE_SKATE, challenge_key: 0x66, players: 8, max_players: 8, phase: Phase::Challenge },
            ]),
            ServerMessage::ChallengeLeave { user_id: 6 },
            ServerMessage::GameProposal { kind: proposal::BROADCAST, activity: 0x34, user_id: 6, timeout: 15_000 },
            ServerMessage::GameProposal { kind: proposal::MISSED, activity: 0x34, user_id: 0, timeout: 0 },
        ];
        for message in messages {
            assert_eq!(ServerMessage::decode(&message.encode()).unwrap(), message);
        }
    }

    #[test]
    fn a_proposal_carries_only_the_types_its_side_sends() {
        let client = |kind| ClientMessage::GameProposal { kind, activity: 1 }.encode();
        assert!(ClientMessage::decode(&client(proposal::BROADCAST)).is_err(), "only the server broadcasts");
        assert!(ClientMessage::decode(&client(proposal::MISSED)).is_err());
        let server = ServerMessage::GameProposal { kind: proposal::MISSED + 1, activity: 1, user_id: 1, timeout: 0 }.encode();
        assert!(ServerMessage::decode(&server).is_err());
    }

    #[test]
    fn nicknames_are_letters_digits_and_a_few_marks() {
        let longest = "y".repeat(NICKNAME_CHARACTERS);
        for name in ["Rob", "skater_99", "Mc'Gill", "the-kid", "Big Bob", "0", longest.as_str()] {
            assert!(valid_nickname(name), "{name:?}");
        }
        let too_long = "y".repeat(NICKNAME_CHARACTERS + 1);
        for name in ["", " Rob", "Rob ", "---", "_ '", "Rob!", "a.b", "Tab\tName", "Caf\u{e9}", too_long.as_str()] {
            assert!(!valid_nickname(name), "{name:?}");
        }
    }

    #[test]
    fn chat_lines_are_what_the_prompt_makes() {
        assert!(valid_chat("hi"));
        assert!(valid_chat(&"x".repeat(CHAT_CHARACTERS)));
        assert!(!valid_chat(&"x".repeat(CHAT_CHARACTERS + 1)));
        assert!(!valid_chat(""));
        assert!(!valid_chat(" hi"));
        assert!(!valid_chat("hi\nthere"));
        assert!(!valid_chat("caf\u{e9}"));
        assert!(ClientMessage::decode(&ClientMessage::Chat("x".repeat(CHAT_CHARACTERS + 1)).encode()).is_err());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(ClientMessage::decode(&[200]).is_err());
        assert!(ClientMessage::decode(&[13, 5, 0, 0, 0, 0, 0]).is_err(), "a run of no frames");
        assert!(ServerMessage::decode(&[18, 8, 5, 1, 0, 0, 0, 0]).is_err(), "a slot past the last");
        let lobby = |players: u8, max_players: u8| ServerMessage::Lobbies(vec![LobbyInfo { game: 1, challenge_type: challenge::JAM, challenge_key: 2, players, max_players, phase: Phase::LobbySkate }]).encode();
        assert!(ServerMessage::decode(&lobby(9, 9)).is_err(), "a game past the most players");
        assert!(ServerMessage::decode(&lobby(5, 4)).is_err(), "more players than the game holds");
        let mut long = vec![24];
        long.push(u8::try_from(MAX_LOBBIES + 1).unwrap());
        assert!(ServerMessage::decode(&long).is_err(), "a listing past the most lobbies");
        assert!(ServerMessage::decode(&[3, 1]).is_err());
        assert!(ClientMessage::decode(&[1, 0, 0, 99]).is_err());
    }
}
