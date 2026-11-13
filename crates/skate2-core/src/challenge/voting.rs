//! `Sk8::Challenge::ChallengeVotingManager`: while a player's turn runs, the other players
//! vote on it; the votee sees nothing, everyone else the `voting` group's `onlinevoting` HUD.
//! `ChallengeOnlineManager::GetLocalPlayerIndex` (`0x825b1818`) is the network game's local
//! player, offline 0. The original read every player's pad over the network game's lockstep; here
//! each player's presses come with that player's skater output ([`MissionFrame::vote`]).
//!
//! [`MissionFrame::vote`]: super::mission::MissionFrame::vote

use super::{
    binding::get_int,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    players::{PlayersBackEnd, network_local_player},
    system::{Event, Field, Message, World},
};

type R = LuaResult<Vec<Value>>;

fn none() -> R {
    Ok(Vec::new())
}

/// `ChallengeVotingManager::kVotingHUDGroupName`.
pub const GROUP: &str = "voting";
/// `OnlineVotingHUDData::kHUDName`.
pub const COMPONENT: &str = "onlinevoting";

/// `eChallengeVotingScore`.
pub mod score {
    pub const BAD: i32 = 0;
    pub const OK: i32 = 1;
    pub const GOOD: i32 = 2;
    pub const SICK: i32 = 3;
}

/// `ChallengeVotingManager` (`0x48` bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VotingManager {
    initialized: bool,
    in_voting: bool,
    votee: i32,
    local_player: i32,
    /// `mScore`: each input's vote, the ninth the shared one the HUD shows.
    scores: [i32; 9],
}

impl Default for VotingManager {
    fn default() -> Self {
        Self { initialized: false, in_voting: false, votee: 0, local_player: 0, scores: [score::OK; 9] }
    }
}

impl VotingManager {
    /// Votes count: between `StartVoting` and `StopVoting`.
    #[must_use]
    pub const fn in_voting(&self) -> bool {
        self.initialized && self.in_voting
    }

    /// The local player votes on another player's turn: the HUD shows.
    #[must_use]
    pub fn shows_hud(&self) -> bool {
        self.initialized && self.in_voting && self.votee != self.local_player
    }

    fn clear(&mut self) {
        self.in_voting = false;
        self.votee = 0;
        self.local_player = 0;
        self.scores = [score::OK; 9];
    }
}

/// `OnlineVotingHUDData` (`0x10` bytes; constructor `0x826b3b90`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnlineVotingHud {
    pub dirty: bool,
    pub vote: i32,
}

impl Default for OnlineVotingHud {
    fn default() -> Self {
        Self { dirty: true, vote: score::OK }
    }
}

/// `ChallengeVotingManager::Update` (`0x825b1a18`), after the activity manager's: while voting,
/// each player's press but the votee's becomes that player's score, the local player's also the
/// ninth; while voting on someone else's turn, the ninth becomes the HUD's vote.
pub fn update(world: &mut World) {
    let v = &mut world.voting;
    if !v.initialized || !v.in_voting {
        return;
    }
    for frame in &world.skaters {
        let Some(vote) = frame.vote else { continue };
        if frame.player == v.votee {
            continue;
        }
        if let Some(slot) = usize::try_from(frame.player).ok().filter(|&player| player < 8).and_then(|player| v.scores.get_mut(player)) {
            *slot = vote;
        }
        if frame.local {
            v.scores[8] = vote;
        }
    }
    if v.votee == v.local_player {
        return;
    }
    let vote = v.scores[8];
    if let Some(data) = world.hud.group_mut(GROUP).and_then(|g| g.voting.as_mut())
        && data.vote != vote
    {
        data.vote = vote;
        data.dirty = true;
    }
}

/// `Online.InitVoting()` (`0x8288a800`): `ChallengeVotingManager::Init` (`0x825b18f0`) creates
/// the `voting` group with the `onlinevoting` component.
fn init_voting(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if !world.voting.initialized {
        world.voting.initialized = true;
        world.hud.create_group(GROUP, &[]);
        world.hud.add_component(GROUP, COMPONENT);
    }
    none()
}

/// `Online.DeInitVoting()` (`0x8288a8b0`): `ChallengeVotingManager::DeInit` (`0x825b1978`).
fn deinit_voting(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    end(world);
    none()
}

/// `ChallengeVotingManager::DeInit` (`0x825b1978`): the voting ends and its group goes, so the HUD's
/// next update unbinds and hides the vote. Only the script's `DeInitVoting` calls it natively. A
/// round the local player leaves before its script ends (PC port: Leave Challenge, or the server
/// taking the player out) calls it too, where the original's only way out of a round, a game
/// unload, takes every HUD screen down.
pub(crate) fn end(world: &mut World) {
    if world.voting.initialized {
        world.voting.initialized = false;
        world.voting.clear();
        world.hud.delete_group(GROUP);
    }
}

/// `Online.StartVoting(votee)` (`0x8288a960`): `ChallengeVotingManager::StartVoting`
/// (`0x825bc4b8`) starts afresh; when the votee is not the local player the group goes to the
/// front of the active groups and the HUD shows.
fn start_voting(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if !world.voting.initialized {
        return none();
    }
    if world.voting.in_voting {
        world.voting.clear();
    }
    world.voting.votee = get_int(call, 0);
    world.voting.local_player = network_local_player(world);
    world.voting.in_voting = true;
    if world.voting.votee != world.voting.local_player {
        world.hud.add_group_to_active(GROUP);
        world.hud.show(GROUP, COMPONENT, true);
    }
    none()
}

/// `Online.StopVoting()` (`0x8288aa08`): `ChallengeVotingManager::StopVoting` (`0x825b8200`)
/// averages the connected voters' scores (`GetPlayerIsDisconnected`) into `cMsgVotingComplete`,
/// hides the HUD and clears.
fn stop_voting(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    if !world.voting.initialized {
        return none();
    }
    let players = world.players.as_deref().map_or(0, PlayersBackEnd::num_players);
    let disconnected = |player: i32| {
        world.online.as_ref().is_some_and(|online| usize::try_from(player).ok().and_then(|player| online.disconnected.get(player)).copied().unwrap_or(false))
    };
    let (mut count, mut total) = (0_i32, 0_i32);
    for player in 0..players.clamp(0, 9) {
        if player != world.voting.votee && !disconnected(player) {
            count += 1;
            total += world.voting.scores[usize::try_from(player).unwrap_or(0)];
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "the native (float) conversions")]
    let average = if count == 0 { 0.0 } else { total as f32 / count as f32 };
    let votee = world.voting.votee;
    #[expect(clippy::cast_precision_loss, reason = "the message's (float) field")]
    let fields = vec![("PlayerID", Field::Number(votee as f32)), ("Score", Field::Number(average))];
    world.send(Event::Challenge(Message::new("cMsgVotingComplete", fields)));
    if votee != world.voting.local_player {
        world.hud.show(GROUP, COMPONENT, false);
    }
    world.voting.clear();
    none()
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("Online", "InitVoting", init_voting),
        ("Online", "DeInitVoting", deinit_voting),
        ("Online", "StartVoting", start_voting),
        ("Online", "StopVoting", stop_voting),
    ]
}
