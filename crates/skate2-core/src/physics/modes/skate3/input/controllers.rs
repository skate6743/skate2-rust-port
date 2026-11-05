//! Persistent platform-input owner. Device collection runs on host frames;
//! this does not choose the skater simulation clock or advance Derived input.
use super::platform::{DeviceError, DevicePacket};

use skate3_core::input::{
    controller::ActionMap,
    gameplay_map::GameplayActions,
    history::{DEVICE_SLOTS, HistoryRecord, PadHistory},
    pad::Pad,
    player_input_data,
    xbox,
    tick::TickInput,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControllerStatus {
    Unpolled,
    Ready,
    Unavailable(DeviceError),
}


pub(crate) struct ControllerInput {
    raw: [RawInput; DEVICE_SLOTS],
    cache: [[HistoryRecord; DEVICE_SLOTS]; 2],
    active: usize,
    history: PadHistory,
    pads: [Pad; DEVICE_SLOTS],
    pub status: [ControllerStatus; DEVICE_SLOTS],
    pub packet_numbers: [Option<u32>; DEVICE_SLOTS],
    pub mapped_actions: [[f32; 18]; DEVICE_SLOTS],
    pub publications: u64,
    pub consumed_batches: u64,
    tick: u64,
}

impl Default for ControllerInput {
    fn default() -> Self {
        Self {
            raw: [RawInput::default(); DEVICE_SLOTS],
            cache: [[HistoryRecord::new(&[]); DEVICE_SLOTS]; 2],
            active: 0,
            history: PadHistory::new(),
            pads: std::array::from_fn(|_| Pad::new()),
            status: [ControllerStatus::Unpolled; DEVICE_SLOTS],
            packet_numbers: [None; DEVICE_SLOTS],
            mapped_actions: [[0.0; 18]; DEVICE_SLOTS],
            publications: 0,
            consumed_batches: 0,
            tick: 0,
        }
    }
}

impl ControllerInput {
    /// Original input.cfg: LB.held && DPadD.pressed / LB.held && DPadU.held.
    /// Use the same debounced native Pad publication as ordinary gameplay.
    pub(crate) fn session_marker_actions(&self) -> (bool, bool, bool) {
        let Some(device) = self.status.iter().position(|s| *s == ControllerStatus::Ready) else {
            return (false, false, false);
        };
        let pad = &self.pads[device];
        if pad.count() == 0 { return (false, false, false); }
        let flags = |i: usize| pad.records().get(i).map_or(0, |r| r[1]);
        let modifier = flags(8) & 0xff00 != 0;
        (modifier, modifier && flags(1) & 0xff00_0000 != 0,
            modifier && flags(0) & 0xff00 != 0)
    }
    pub(crate) fn raw_input(&self) -> RawInput {
        self.status.iter().position(|s| *s == ControllerStatus::Ready)
            .map_or(RawInput::default(), |i| self.raw[i])
    }

    // Replay controls must never queue a flick or a button press for gameplay.
    pub(crate) fn discard_gameplay(&mut self) {
        self.history = PadHistory::new();
        self.pads = std::array::from_fn(|_| Pad::new());
        self.mapped_actions = [[0.0; 18]; DEVICE_SLOTS];
    }
    #[cfg(any())]
    pub(crate) fn player_actions(&self) -> GameplayActions {
        let device = self.status.iter().position(|status| *status == ControllerStatus::Ready).unwrap_or(0);
        GameplayActions::from_pad(&self.pads[device])
    }

    pub(super) fn tick_input(&self) -> TickInput {
        let device = self
            .status
            .iter()
            .position(|status| *status == ControllerStatus::Ready);
        let controller_available = device.is_some();
        let actions = device
            .map(|device| GameplayActions::from_pad(&self.pads[device]))
            .unwrap_or_else(|| GameplayActions::from_values([0.0; 18]));
        TickInput::new(
            self.tick,
            actions,
            controller_available,
        )
    }
    /// TU3 8296D288/8296D0D0: write the inactive cache, publish one four-device
    /// batch. Even an unchanged platform packet is sampled; packet-number
    /// deduplication would alter native Pad edge/repeat behavior.
    pub(super) fn collect(&mut self, samples: [Result<DevicePacket, DeviceError>; DEVICE_SLOTS]) {
        let next = self.active ^ 1;
        for (device, sample) in samples.into_iter().enumerate() {
            match sample {
                Ok(packet) => {
                    self.raw[device] = RawInput {
                        buttons: packet.state.buttons,
                        triggers: packet.state.triggers.map(|v| f32::from(v) / 255.0),
                        left: packet.state.left.map(|v| f32::from(v) / 32768.0),
                        right: packet.state.right.map(|v| f32::from(v) / 32768.0),
                    };
                    // 8296D480 sets byte13 only when capability SubType == 7. Gameplay reads the
                    // line after PlayerInputData's Pack/Unpack (82898920 -> 82695438).
                    let values = player_input_data::round_trip(&xbox::convert(&packet.state, u8::from(packet.subtype == 7)));
                    self.cache[next][device] = HistoryRecord::new(&values);
                    self.packet_numbers[device] = Some(packet.number);
                    self.status[device] = ControllerStatus::Ready;
                }
                Err(error) => {
                    self.raw[device] = RawInput::default();
                    self.cache[next][device].clear_count();
                    self.packet_numbers[device] = None;
                    self.status[device] = ControllerStatus::Unavailable(error);
                }
            }
        }
        self.active = next;
        self.history.publish(&self.cache[self.active]);
        self.publications += 1;
    }

    /// TU3 82699230 drains first, then updates each Pad once. The game owns
    /// this snapshot for subsequent consumers; Derived timers require the
    /// actual actor timestep and state flags and are not driven by render dt.
    pub(super) fn publish_actions(&mut self) -> bool {
        self.tick = self.tick.wrapping_add(1);
        if !self.history.drain_to_latest(&mut self.pads) {
            return false;
        }
        for (device, pad) in self.pads.iter().enumerate() {
            let mut actions = GameplayActions::from_pad(pad);
            self.mapped_actions[device] = std::array::from_fn(|i| actions.value(64 + i as u32));
        }
        self.consumed_batches += 1;
        true
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct RawInput {
    pub buttons: u16,
    pub triggers: [f32; 2],
    pub left: [f32; 2],
    pub right: [f32; 2],
}

#[cfg(any())]
#[path = "tests/controllers.rs"]
mod tests;
