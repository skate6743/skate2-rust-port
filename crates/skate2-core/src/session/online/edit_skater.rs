//! Edit Skater from an online game's pre-game lobby (PC port addition). The pause menu's row, as
//! the single-player hub's, opens the editor while the local skater is in the game's world with no
//! round, travel, change of location or freeskate activity of its own coming. The game simulates
//! under the editor as under the pause menu beneath it (`Net::ResumeGame`): the skater, without
//! PlayerUI's focus, brakes to a stop and idles in the lobby, and its replay frames keep going
//! out. The shop sells against the profile's career cash and unlockables, which the editor writes
//! back as it closes. Once the local skater has the look the editor left, it goes to every
//! player (`Recipe`), whose consoles build the skater again. The game no longer offering it (a
//! round loading with the player, a change of location, a travel, the game ending) closes the
//! editor at once: the receipt's lines leave the skater, which keeps only what the player owns or
//! bought, and the rest saves as Finish's would.

use super::{SelectedLiveError, SelectedLiveSession, error, trace};
use crate::{
    challenge::{signpost_progress, system::ChallengeSystem, unlockables},
    edit_skater_profile::EditSkaterProfileStore,
    edit_skater_shop::{CLOTHING_SPONSOR, CLOTHING_UNLOCK},
};
use skate2_formats::skater_profile::ProfileData;
use std::sync::Arc;

impl<T, C> SelectedLiveSession<T, C> {
    /// Whether the online game offers Edit Skater: the local skater is in its world, plays no
    /// round, and no travel, change of location or freeskate activity of its own is coming.
    pub(in super::super) fn online_can_edit_skater(&self) -> bool {
        self.online.game.as_ref().is_some_and(|game| game.entered)
            && self.online.round.is_none()
            && self.online.changing_location.is_none()
            && self.online.start.is_none()
            && self.map_travel.is_none()
            && !self.online_activity_awaits_local()
    }

    /// The pause menu's Edit Skater in an online game: watching another player ends, and the
    /// editor opens behind the fade as the single-player hub's does.
    pub(in super::super) fn online_open_edit_skater(&mut self) -> Result<(), SelectedLiveError> {
        self.online_end_spectate()?;
        self.online.edit_skater = true;
        trace(format_args!("Edit Skater opens"));
        Ok(())
    }

    /// Each update: the editor, opening or open, closes once the game no longer offers it.
    pub(super) fn update_online_edit_skater(&mut self) -> Result<(), SelectedLiveError> {
        if self.online.edit_skater && !self.online_can_edit_skater() {
            self.online_close_edit_skater()?;
        }
        Ok(())
    }

    /// The editor closes for the player: still behind the entry's fade it does not open; open,
    /// the receipt's lines leave the skater and the rest saves as Finish's would.
    pub(super) fn online_close_edit_skater(&mut self) -> Result<(), SelectedLiveError> {
        if !self.online.edit_skater {
            return Ok(());
        }
        trace(format_args!("Edit Skater closes: the game no longer offers it"));
        if self.editor.is_none() {
            self.online.edit_skater = false;
            self.editor_enter_pending = false;
            self.frontend_reveal_pending = self.map_travel.is_none();
            return Ok(());
        }
        self.close_edit_skater_for_player()
    }

    /// The local skater's look changed: it goes to the other players again once loaded.
    pub(in super::super) fn online_character_changed(&mut self) {
        if let Some(game) = self.online.game.as_mut() {
            game.character_sent = false;
        }
    }

    /// The shop's cash and unlockables online: the profile's career, its saved SignPost and
    /// UNLOCKABLES savables as a career starting would load them (`cSignPostManager::GetCash`,
    /// `LockedFilter`'s and `NewFilter`'s unlockables).
    pub(in super::super) fn online_shop_funds(&mut self) -> Result<(i32, unlockables::Unlockables), SelectedLiveError> {
        let system = self.online_profile_system()?;
        let world = system.world();
        Ok((signpost_progress::cash(world), world.unlockables.clone()))
    }

    /// The editor's close online, as the career's (`FrontEndState_CaC::Shutdown` 0x82659d28 and
    /// the receipt's `SubtractCash`): the clothing unlockables (categories 15 and 16) no longer
    /// new and each line bought off the profile's cash, saved at once. The profile's statistics
    /// are not loaded online, so a purchase's statistic 9 goes uncounted.
    pub(in super::super) fn online_close_editor_shop(&mut self, costs: &[i32]) -> Result<(), SelectedLiveError> {
        let mut system = self.online_profile_system()?;
        let world = system.world_mut();
        let before = world.unlockables.save();
        for category in [CLOTHING_UNLOCK, CLOTHING_SPONSOR] {
            world.unlockables.reset_all_new_unlocks_from_category(category);
        }
        for &cost in costs {
            signpost_progress::subtract_cash(world, cost, 0);
        }
        let unlocks = world.unlockables.save();
        if costs.is_empty() && unlocks == before {
            return Ok(());
        }
        let leaves = signpost_progress::save(world);
        let cash = signpost_progress::cash(world);
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        let mut store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let data = match store.load_profile_data().map_err(error)? {
            Some(data) => data,
            // ProfileBackEnd::ResetProfile 0x8256d8f0.
            None => ProfileData::reset(
                &super::super::career::localized(&mounts, "ID_DEFAULT_PLAYER_NAME")?,
                &super::super::career::localized(&mounts, "ID_DEFAULT_PLAYER_NICKNAME")?,
            ),
        };
        store
            .save_memcard(
                &data,
                None,
                &[
                    (unlockables::SAVABLE, &[], &unlocks),
                    (signpost_progress::SAVABLE, &signpost_progress::SAVABLES_BEFORE, &leaves),
                ],
            )
            .map_err(error)?;
        trace(format_args!("Edit Skater bought {costs:?}: cash {cash}"));
        Ok(())
    }

    /// The profile's challenge system with no career started, over the game's career database.
    fn online_profile_system(&mut self) -> Result<ChallengeSystem, SelectedLiveError> {
        self.online_data()?;
        let db = Arc::clone(&self.freeskate.data.as_ref().ok_or_else(|| error("data absent"))?.0);
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        super::super::career::profile_challenge_system(db, &mounts).map_err(error)
    }
}
