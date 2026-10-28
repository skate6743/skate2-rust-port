//! The pads' rumble: the local skater's `RumbleManager::UpdateRumbleEvents` after each simulation
//! tick of a game, `StateManagerFrontEnd::ResetRumble` 0x82656998 for every front-end state a game
//! enters or leaves, the profile's Vibration setting on the controller's `Vibration`, Options' test
//! rumble, and `InputManager::UpdateVibration` at the end of every frame (`Sk8AppMan::PreUpdate`
//! 0x8272c478).
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::input_pad::{
    device::PadPlatform,
    rumble::{RumbleManager, STEP},
};
use std::fmt::Debug;

fn error(stage: SelectedLiveStage, e: impl Debug) -> SelectedLiveError {
    SelectedLiveError::failed(stage, e)
}

#[derive(Default)]
pub(super) struct VibrationSession {
    /// `SkaterManager::mRumbleManager`, read from the vault the first time a game rumbles.
    rumble: Option<RumbleManager>,
    /// The vault's rumble data did not read: the skater makes no rumble (Options' test rumble
    /// still plays).
    rumble_unreadable: bool,
    /// The front-end state on top at the last frame of a game; `None` outside one.
    frontend: Option<Option<i32>>,
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `IsStateInStack(7)`: the StartHub, or a screen it pushed, is up.
    fn start_hub_in_stack(&self) -> bool {
        let pause = self.pause_menu.snapshot();
        (pause.visible && !pause.jacked)
            || self.options_menu.visible()
            || self.audio.menu.visible()
            || self.map_menu.snapshot().visible
            || self.skate_profile.visible()
            || self.trick_guide.is_some()
            || self.replay_editor.is_some()
    }

    /// `SkaterManager::PostUpdate` 0x825f8b88: after a simulation tick of a game
    /// (`mFEState != eFE_STATE_IN_FE`) with no StartHub on the front end's stack, the local skater's
    /// rumble events go to the player's pad (`PlayerUI::ReceiveRumbleChannelMsg`). A paused
    /// simulation or a replay runs no `PostUpdate`; a skater whose `PhysOut` is not whole yet
    /// rumbles nothing, as the audio roster reads nothing of it.
    pub(super) fn try_update_rumble_events(&mut self) -> Result<(), SelectedLiveError> {
        use SelectedLiveStage as Stage;
        if self.main_menu.is_some()
            || self.main_menu_pending
            || self.vibration.rumble_unreadable
            || self.audio_paused()
            || self.audio_playback().0
            || self.start_hub_in_stack()
        {
            return Ok(());
        }
        let Some(actor) = self.audio_local_actor() else { return Ok(()) };
        let phys = {
            let Ok(physical) = actor.cached_physical_allocation() else { return Ok(()) };
            let output = physical
                .owner()
                .phys_out()
                .lock()
                .map_err(|_| error(Stage::QueuedFrame, "PhysOut lock poisoned"))?;
            let Ok(phys) = output.try_rumble_phys_out() else { return Ok(()) };
            phys
        };
        if self.vibration.rumble.is_none() {
            let Some(assets) = self.inner.assets.as_ref() else { return Ok(()) };
            match RumbleManager::try_new(&assets.vault) {
                Ok(rumble) => self.vibration.rumble = Some(rumble),
                Err(e) => {
                    eprintln!("rumble data unreadable, the skater will not rumble: {e}");
                    self.vibration.rumble_unreadable = true;
                    return Ok(());
                }
            }
        }
        let rumbles = self.vibration.rumble.as_mut().expect("rumble manager made").update(&phys, STEP);
        if let Some(vibration) = self.inner.bound_input.as_mut().and_then(|input| input.vibration_mut()) {
            for rumble in rumbles {
                vibration.add_rumble_event(rumble.low, rumble.high, rumble.decay, rumble.channel);
            }
        }
        Ok(())
    }

    /// The end of `Sk8AppMan::PreUpdate`. In a game, a front-end state entered or left resets the
    /// controller's rumble (`ResetRumble`'s `cMsgRumbleReset`); the controller's `Vibration`
    /// follows the profile's Vibration setting (the `cMsgRumbleState` the profile's load and
    /// reset, a game's start and every toggle send); turning Vibration on plays
    /// `ToggleVibration`'s test rumble (0.5 on both motors, decaying at 5 per second); then every
    /// pad's rumble goes to its motors (`InputManager::UpdateVibration`).
    pub(super) fn try_update_vibration<P: PadPlatform>(&mut self, pad: &mut P) -> Result<(), SelectedLiveError>
    where
        P::Error: Debug,
    {
        use SelectedLiveStage as Stage;
        let in_game = self.main_menu.is_none() && !self.main_menu_pending;
        let frontend = in_game.then(|| self.frontend_audio_state().map(|state| state.state));
        let reset = frontend.is_some() && frontend != self.vibration.frontend;
        self.vibration.frontend = frontend;
        let test = self.options_menu.take_vibration_test();
        let inner = &mut self.inner;
        let (Some(assets), Some(input)) = (inner.assets.as_ref(), inner.bound_input.as_mut()) else {
            return Ok(());
        };
        if reset {
            input.try_reset_vibration(pad, &assets.cvars).map_err(|e| error(Stage::PreupdateInput, e))?;
        }
        if let Some(vibration) = input.vibration_mut() {
            vibration.enabled = self.audio.settings.rumble;
            if test {
                vibration.enabled = true;
                vibration.add_rumble_event(0.5, 0.5, 5.0, 0);
            }
        }
        input.try_update_vibration(pad, &assets.cvars).map_err(|e| error(Stage::PreupdateInput, e))
    }
}
