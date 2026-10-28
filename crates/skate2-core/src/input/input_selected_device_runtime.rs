//! Explicit serialized input events over the existing manager and group caches, and each pad's
//! `Vibration`. This does not implement the platform timer, front-end hotplug notifications or
//! the complete application pre-update scheduler.
use super::{DeviceManagerError, InputDeviceManager};
use crate::{
    input_cvar_runtime::InputCvarReferences,
    input_map::SelectedInputMap,
    input_pad::{
        device::{PadActuatorError, PadPlatform, PadRemap},
        vibration::Vibration,
    },
    input_pad_cache::{InputPadCache, InputPadCacheError, PublishedInputPadSample},
    reckoning_numeric::ReckoningNumeric,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};
use std::sync::Arc;

/// Native input-group selection; the two caches never share pad history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedInputGroup {
    Simulation,
    Presentation,
}
/// Narrow platform event boundary implemented by the retained native owners.
/// Hosts supply explicit events; this API chooses no polling frequency.
pub trait SelectedDeviceInputEvents<P: PadPlatform> {
    type Error;
    /// # Errors
    /// Preserves actual platform/numeric/remap failure and incomplete publication.
    fn poll_hardware(&mut self, platform: &mut P) -> Result<(), Self::Error>;
    /// # Errors
    /// Refuses an uninitialized device manager or selected group cache.
    fn cache_input(&mut self, group: SelectedInputGroup) -> Result<(), Self::Error>;
    /// # Errors
    /// Preserves cache drain and earlier pad updates on a later pad failure.
    fn last_input(&mut self, group: SelectedInputGroup) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SelectedDeviceInputError<P, N, R> {
    Device(DeviceManagerError<P, N, R>),
    Cache(InputPadCacheError),
}
impl<P: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for SelectedDeviceInputError<P, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(e) => write!(f, "selected device polling: {e:?}"),
            Self::Cache(e) => e.fmt(f),
        }
    }
}
/// Moves the actual existing owners; no independent controller, pad history,
/// map parser or mapping arithmetic is introduced. Startup completion is not
/// inferred: an uninitialized owner or unknown guitar capability still fails
/// at the existing consuming operation.
pub(crate) struct SelectedDeviceInputRuntime<R, N, M> {
    devices: InputDeviceManager<R>,
    sim: InputPadCache,
    pres: InputPadCache,
    map: Arc<SelectedInputMap>,
    numeric: N,
    remap: M,
    /// `InputManager::mVibration`, one per pad.
    vibration: [Vibration; 4],
}
impl<R: Clone, N: SkeletonCollisionNumeric, M: PadRemap<R>> SelectedDeviceInputRuntime<R, N, M> {
    pub(crate) fn from_owners(
        devices: InputDeviceManager<R>,
        sim: InputPadCache,
        pres: InputPadCache,
        map: Arc<SelectedInputMap>,
        numeric: N,
        remap: M,
    ) -> Self {
        Self {
            devices,
            sim,
            pres,
            map,
            numeric,
            remap,
            vibration: std::array::from_fn(|_| Vibration::new()),
        }
    }
    /// `InputManager::GetVibration` 0x82728eb0.
    pub(crate) fn vibration_mut(&mut self, controller: usize) -> Option<&mut Vibration> {
        self.vibration.get_mut(controller)
    }
    /// `InputManager::UpdateVibration` 0x826e22b0: each pad's `Vibration::Update`, whose low and
    /// high frequencies go to its actuators 0 and 1 (`cXenonPadDevice::SetActuator`).
    pub(crate) fn try_update_vibration<P: PadPlatform, Num: ReckoningNumeric>(
        &mut self,
        platform: &mut P,
        refs: &InputCvarReferences,
        numeric: &mut Num,
    ) -> Result<(), PadActuatorError<P::Error, Num::Error>> {
        let Some(devices) = self.devices.devices_mut() else { return Ok(()) };
        for (device, vibration) in devices.iter_mut().zip(&mut self.vibration) {
            let [low, high] = vibration.update();
            device.set_actuator(0, low, platform, refs, numeric)?;
            device.set_actuator(1, high, platform, refs, numeric)?;
        }
        Ok(())
    }
    /// `Vibration::Reset` 0x826e1590 of one pad: its channels, then `ResetActuators`.
    pub(crate) fn try_reset_vibration<P: PadPlatform, Num: ReckoningNumeric>(
        &mut self,
        controller: usize,
        platform: &mut P,
        refs: &InputCvarReferences,
        numeric: &mut Num,
    ) -> Result<(), PadActuatorError<P::Error, Num::Error>> {
        if let Some(vibration) = self.vibration.get_mut(controller) {
            vibration.clear();
        }
        match self.devices.devices_mut().and_then(|devices| devices.get_mut(controller)) {
            Some(device) => device.reset_actuators(platform, refs, numeric),
            None => Ok(()),
        }
    }
    pub(crate) fn map(&self) -> &Arc<SelectedInputMap> {
        &self.map
    }
    /// `GetRawInputCacheLine826dfc90`: gameplay payload takes the current device
    /// half, not the last drained SIM pad or a pre-evaluated gameplay mapping.
    pub(crate) fn raw_current_cache_line(
        &self,
        controller: usize,
    ) -> Result<PublishedInputPadSample, InputPadCacheError> {
        self.devices.raw_current_cache_line(controller)
    }
    pub(crate) fn cache(&self, group: SelectedInputGroup) -> &InputPadCache {
        match group {
            SelectedInputGroup::Simulation => &self.sim,
            SelectedInputGroup::Presentation => &self.pres,
        }
    }
}
impl<R: Clone, N: SkeletonCollisionNumeric, M: PadRemap<R>, P: PadPlatform>
    SelectedDeviceInputEvents<P> for SelectedDeviceInputRuntime<R, N, M>
{
    type Error = SelectedDeviceInputError<P::Error, N::Error, M::Error>;
    fn poll_hardware(&mut self, platform: &mut P) -> Result<(), Self::Error> {
        self.devices
            .poll(platform, &mut self.numeric, &mut self.remap)
            .map_err(SelectedDeviceInputError::Device)
    }
    fn cache_input(&mut self, group: SelectedInputGroup) -> Result<(), Self::Error> {
        let cache = match group {
            SelectedInputGroup::Simulation => &mut self.sim,
            SelectedInputGroup::Presentation => &mut self.pres,
        };
        self.devices
            .cache_into(cache)
            .map_err(SelectedDeviceInputError::Cache)
    }
    fn last_input(&mut self, group: SelectedInputGroup) -> Result<bool, Self::Error> {
        let cache = match group {
            SelectedInputGroup::Simulation => &mut self.sim,
            SelectedInputGroup::Presentation => &mut self.pres,
        };
        cache
            .try_last_input()
            .map_err(SelectedDeviceInputError::Cache)
    }
}
#[path = "input_selected_player_sample.rs"]
pub(crate) mod player_sample;
#[cfg(test)]
#[path = "input_selected_device_runtime_tests.rs"]
mod tests;
