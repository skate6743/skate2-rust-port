//! `Sk8::Challenge::cSequenceConnectorManager`: whether landing in a manual keeps a line going
//! (`PlayersBackEnd::SetManualsAreConnectors` for the nine players). A main challenge whose
//! `GlobalData` has `SetManualAndWalkAsConnector`, unless it is a Best Trick, gives every player
//! that setting as it starts. While manuals do not connect, the challenge's own spot volumes make
//! them connect for a player inside one, and a player who leaves the last one in a manual has
//! the line ended. When that challenge shuts down every player's manuals connect again. The
//! contest rounds' spot volumes (`SetContestRound`, whose `SequenceConnector.SetContestRound` no
//! shipped script calls) and Create-a-Spot's (`AddSpotVolume`, `RemoveSpotVolume`) are not
//! ported.

use super::system::{Event, Output, World};

/// `challenge_global_data.ChallengeType`.
const CHALLENGE_TYPE: u64 = 0x02b8_28cc_db47_85e4;
/// `eChallengeTypes::BestTrick`, whose setting `RunMainActivity` leaves alone.
const BEST_TRICK: i32 = 8;
/// `challenge_global_data.SetManualAndWalkAsConnector`.
const SET_MANUAL_AND_WALK_AS_CONNECTOR: u64 = 0x0661_a17a_a0e5_c774;
/// `challenge_local_data`'s spot volumes: `tTriggerVolumeInstanceID`s, the volume's id at +8.
const SPOT_VOLUMES: u64 = 0x0a67_5615_20c8_5c04;
/// `SpotVolumeList`'s capacity (`fixed_vector<u64, 128>`), which every add checks.
const SPOT_VOLUME_CAPACITY: usize = 128;
/// `eTrickTypeManual`.
const TRICK_TYPE_MANUAL: i32 = 8;

/// `Sk8ChallengeSystem::SequenceConnectorManager`.
#[derive(Debug)]
pub struct SequenceConnectorManager {
    /// `mCurrentConnectorSetting`, which a challenge's spawned AI players take.
    pub(crate) setting: bool,
    /// `mAllowSpotVolume`: off once a challenge makes manuals connect everywhere.
    allow_spot_volume: bool,
    /// `mCurrentChallenge`: the main challenge that set the players.
    current: Option<usize>,
    /// `SpotVolumeList`: the spot volumes' ids.
    spot_volumes: Vec<u64>,
    /// `mCurrentIsInSpotVolume`: per player, the spot volumes it is inside.
    inside: [i32; 9],
}

impl Default for SequenceConnectorManager {
    /// The constructor (`0x825bd0f8`).
    fn default() -> Self {
        Self { setting: true, allow_spot_volume: true, current: None, spot_volumes: Vec::new(), inside: [0; 9] }
    }
}

impl SequenceConnectorManager {
    /// The spot volumes whose id has the volume's low word, as the handlers compare them.
    fn matching(&self, volume: u64) -> usize {
        self.spot_volumes.iter().filter(|&&id| (id & 0xffff_ffff) == (volume & 0xffff_ffff)).count()
    }

    /// `AdjustManualConnector` (`0x825af0a0`)'s count of the spot volumes a player is inside: the
    /// first one entered makes its manuals connect, leaving the last one stops that.
    fn adjust(&mut self, in_spot_volume: bool, player: i32) -> Option<bool> {
        let inside = self.inside.get_mut(usize::try_from(player).ok()?)?;
        match (*inside > 0, in_spot_volume) {
            (false, false) => None,
            (false, true) => {
                *inside += 1;
                Some(true)
            }
            (true, true) => {
                *inside += 1;
                None
            }
            (true, false) => {
                *inside -= 1;
                (*inside == 0).then_some(false)
            }
        }
    }

    fn is_inside(&self, player: i32) -> bool {
        usize::try_from(player).ok().and_then(|player| self.inside.get(player)).is_some_and(|&inside| inside != 0)
    }
}

fn global_attribute(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    let (class, collection) = world.activities.activities[activity].global_data?;
    world.db.attribute(class, collection, key)
}

fn local_attribute(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    let (class, collection) = world.activities.activities[activity].local_data?;
    world.db.attribute(class, collection, key)
}

/// `PlayersBackEnd::SetManualsAreConnectors` of each of the nine players.
fn set_every_player(world: &World, connects: bool) {
    if let Some(players) = world.players.as_deref() {
        for player in 0..9 {
            players.set_manuals_are_connectors(player, connects);
        }
    }
}

/// `GameLoad` (`0x825b9c28`): no challenge, no spot volume, no player inside one.
pub(crate) fn game_load(world: &mut World) {
    let manager = &mut world.sequence_connector;
    manager.spot_volumes.clear();
    manager.allow_spot_volume = true;
    manager.inside = [0; 9];
    manager.current = None;
}

/// `RunMainActivity` (`0x825be5a0`): a challenge with `SetManualAndWalkAsConnector` that is no Best
/// Trick becomes the manager's, gives every player its setting and, while spot volumes are
/// allowed, has its own spot volumes take effect.
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    world.sequence_connector.inside = [0; 9];
    let challenge_type = global_attribute(world, activity, CHALLENGE_TYPE).and_then(|a| a.i32(0)).unwrap_or(0);
    let Some(connects) = global_attribute(world, activity, SET_MANUAL_AND_WALK_AS_CONNECTOR).map(|a| a.bool(0).unwrap_or(false)) else { return };
    if challenge_type == BEST_TRICK {
        return;
    }
    world.sequence_connector.current = Some(activity);
    set_global_manual_connector(world, connects);
    if world.sequence_connector.allow_spot_volume {
        remove_spot_volumes(world, activity);
        if add_spot_volumes(world, activity) {
            reset_trigger_spot_volumes(world);
        }
    }
}

/// `SetGlobalManualConnectorForChallenge` (`0x825b3ed0`): every player gets the challenge's
/// setting; manuals that connect everywhere need no spot volume.
fn set_global_manual_connector(world: &mut World, connects: bool) {
    world.sequence_connector.setting = connects;
    set_every_player(world, connects);
    if connects {
        world.sequence_connector.allow_spot_volume = false;
    }
}

/// `RemoveSpotVolumes` (`0x825b68a8`): a challenge with spot volumes of its own empties the list;
/// no player is inside one.
fn remove_spot_volumes(world: &mut World, activity: usize) {
    if local_attribute(world, activity, SPOT_VOLUMES).is_some_and(|volumes| !volumes.is_empty()) {
        world.sequence_connector.spot_volumes.clear();
    }
    world.sequence_connector.inside = [0; 9];
}

/// `AddSpotVolumes` (`0x825be458`): the challenge's own spot volumes, as far as the list holds
/// them (`DefaultDataArea` for one the data lacks); whether it has any.
fn add_spot_volumes(world: &mut World, activity: usize) -> bool {
    let ids: Vec<u64> = local_attribute(world, activity, SPOT_VOLUMES).map_or_else(Vec::new, |volumes| {
        (0..volumes.len())
            .map(|index| volumes.value_bytes(index).filter(|bytes| bytes.len() >= 16).map_or(0, |bytes| u64::from_be_bytes(bytes[8..16].try_into().expect("16 bytes"))))
            .collect()
    });
    let list = &mut world.sequence_connector.spot_volumes;
    for &id in &ids {
        if list.len() < SPOT_VOLUME_CAPACITY {
            list.push(id);
        }
    }
    !ids.is_empty()
}

/// `ResetTriggerSpotVolumes` (`0x825b3f90`): each spot volume's
/// `ResetTriggerVolumeCollisionState(tTriggerVolumeInstanceID)`, so a skater already inside one
/// enters it again.
fn reset_trigger_spot_volumes(world: &mut World) {
    for id in world.sequence_connector.spot_volumes.clone() {
        // ResetTriggerVolumeCollisionState 0x82e22688: (float)_u64tod.
        let volume = crate::skate3::id_float(id);
        world.output(Output::ResetTriggerVolumeState { volume });
    }
}

/// `ShutdownActivity` (`0x825b6930`): the challenge that set the players leaves no spot volume,
/// and every player's manuals connect again.
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    if world.sequence_connector.current != Some(activity) {
        return;
    }
    remove_spot_volumes(world, activity);
    let manager = &mut world.sequence_connector;
    manager.spot_volumes.clear();
    manager.allow_spot_volume = true;
    manager.inside = [0; 9];
    manager.current = None;
    set_every_player(world, true);
}

/// The manager's Hermes handlers, `ReceiveTriggerEnterCollisionMessage` (`0x825b4030`) and
/// `ReceiveTriggerExitCollisionMessage` (`0x825b41a0`): for each matching spot volume the player
/// enters or leaves it, and a player outside every one has its line ended while in a manual
/// (`ResetPlayerSequence` `0x825b4090`).
pub(crate) fn receive(world: &mut World, event: &Event) {
    let Event::TriggerCollision { enter, volume, player } = *event else { return };
    for _ in 0..world.sequence_connector.matching(volume) {
        if let (Some(connects), Some(players)) = (world.sequence_connector.adjust(enter, player), world.players.as_deref()) {
            players.set_manuals_are_connectors(player, connects);
        }
        if !enter && !world.sequence_connector.is_inside(player) {
            let manual = super::skate::skater(world, player).is_some_and(|frame| frame.active_trick_types.contains(&TRICK_TYPE_MANUAL));
            if manual && let Some(players) = world.players.as_deref() {
                players.set_reset_sequence(player, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manuals_connect_from_the_first_spot_volume_entered_to_the_last_one_left() {
        let mut manager = SequenceConnectorManager::default();
        assert_eq!(manager.adjust(false, 2), None);
        assert_eq!(manager.adjust(true, 2), Some(true));
        assert_eq!(manager.adjust(true, 2), None);
        assert_eq!(manager.adjust(false, 2), None);
        assert!(manager.is_inside(2));
        assert_eq!(manager.adjust(false, 2), Some(false));
        assert!(!manager.is_inside(2));
        assert_eq!(manager.adjust(false, 2), None);
        assert_eq!(manager.adjust(true, 9), None);
    }

    #[test]
    fn a_spot_volume_matches_by_its_ids_low_word() {
        let manager = SequenceConnectorManager { spot_volumes: vec![0x0001_0002_0000_0005, 7, 0x0009_0000_0000_0005], ..SequenceConnectorManager::default() };
        assert_eq!(manager.matching(5), 2);
        assert_eq!(manager.matching(0x0001_0002_0000_0007), 1);
        assert_eq!(manager.matching(6), 0);
    }
}
