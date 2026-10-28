//! Pads read with SDL3's gamepad API on a thread of their own (SDL's objects stay on the thread
//! that initialised it): on Windows the ones XInput does not list (PlayStation, Switch and other
//! HID controllers), elsewhere every pad, Xbox pads and a Steam Deck's controls included. A
//! PlayStation pad's sample is the XInput state DS4Windows 1.4 gave its virtual Xbox 360 pad, so
//! the native pad conversion in core reads what it read there; an Xbox pad's is the pad's own.
use sdl3::gamepad::{Axis, Button, Gamepad, GamepadType};
use skate2_core::{PadVibration, XboxPadSample};
use std::{
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// Well inside a DualShock 4's 4 ms USB report period.
const POLL: Duration = Duration::from_millis(1);
/// How often the thread looks for pads that connected or went away.
const SCAN: Duration = Duration::from_millis(250);
/// The longest the game waits for SDL's first look for pads, so the pads connected at launch
/// are in the slots `InputManager::Init` reads; one SDL finds later joins the first slot
/// XInput leaves empty on its next poll.
const STARTUP: Duration = Duration::from_secs(2);
/// SDL ends a rumble after at most `SDL_MAX_RUMBLE_DURATION_MS` (0xFFFF). XInput's motors
/// hold until set again, so a held rumble is sent again before it would lapse.
const RUMBLE_MS: u32 = 0xFFFF;
const RUMBLE_RESEND: Duration = Duration::from_secs(30);
const STILL: PadVibration = PadVibration { left: 0, right: 0 };

/// Set by the panic hook: the game stopped, and every pad's motors stop with it.
static SILENCED: AtomicBool = AtomicBool::new(false);

/// Stops every pad's rumble for the rest of the process.
pub(crate) fn silence() {
    SILENCED.store(true, Ordering::Relaxed);
}

/// SDL's buttons and the XInput bits DS4Windows set for them. XInputGetState never reports
/// Guide, so the PS button has none.
const BUTTONS: [(Button, u16); 14] = [
    (Button::DPadUp, 0x0001),
    (Button::DPadDown, 0x0002),
    (Button::DPadLeft, 0x0004),
    (Button::DPadRight, 0x0008),
    (Button::Start, 0x0010),
    (Button::Back, 0x0020),
    (Button::LeftStick, 0x0040),
    (Button::RightStick, 0x0080),
    (Button::LeftShoulder, 0x0100),
    (Button::RightShoulder, 0x0200),
    (Button::South, 0x1000),
    (Button::East, 0x2000),
    (Button::West, 0x4000),
    (Button::North, 0x8000),
];

#[derive(Clone, Copy)]
struct Pad {
    id: u32,
    sample: XboxPadSample,
    /// Counts changes of `sample`, as XInput's packet number does.
    packet: u32,
}

#[derive(Default)]
struct Shared {
    /// SDL's first look for pads is done, or SDL is unavailable.
    ready: bool,
    /// The connected pads, in the order they connected.
    pads: Vec<Pad>,
    /// The vibration the game last set on each pad.
    vibration: Vec<(u32, PadVibration)>,
}

/// `Shared`, and the signal that it became ready.
type SharedState = (Mutex<Shared>, Condvar);

/// The SDL thread, and the game slots its pads are shown in.
pub(crate) struct SdlPads {
    shared: Arc<SharedState>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    slots: [Option<u32>; 4],
}

impl SdlPads {
    pub(crate) fn start() -> Self {
        let shared = Arc::<SharedState>::default();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (shared, stop) = (Arc::clone(&shared), Arc::clone(&stop));
            std::thread::Builder::new()
                .name("sk8 pads".to_owned())
                .spawn(move || {
                    if let Err(error) = run(&shared, &stop) {
                        eprintln!("SDL gamepads unavailable: {error}");
                    }
                    mark_ready(&shared);
                })
        };
        if let Err(error) = &thread {
            eprintln!("SDL gamepad thread did not start: {error}");
        }
        Self {
            shared,
            stop,
            thread: thread.ok(),
            slots: [None; 4],
        }
    }

    /// The pad shown in a slot XInput found empty: the one already there, else the first pad
    /// not shown in another slot. Its packet number and sample.
    pub(crate) fn claim(&mut self, index: u32) -> Option<(u32, XboxPadSample)> {
        let slot = usize::try_from(index)
            .ok()
            .filter(|slot| *slot < self.slots.len())?;
        let (lock, started) = &*self.shared;
        let shared = lock.lock().unwrap_or_else(PoisonError::into_inner);
        let (shared, _) = started
            .wait_timeout_while(shared, STARTUP, |shared| !shared.ready)
            .unwrap_or_else(PoisonError::into_inner);
        let pad = self.slots[slot]
            .and_then(|id| shared.pads.iter().find(|pad| pad.id == id))
            .or_else(|| {
                shared
                    .pads
                    .iter()
                    .find(|pad| !self.slots.contains(&Some(pad.id)))
            })
            .copied();
        self.slots[slot] = pad.map(|pad| pad.id);
        pad.map(|pad| (pad.packet, pad.sample))
    }

    /// XInput lists a pad in this slot; one of ours shown there moves to the next free slot.
    #[cfg(windows)]
    pub(crate) fn release(&mut self, index: u32) {
        if let Some(slot) = usize::try_from(index)
            .ok()
            .and_then(|slot| self.slots.get_mut(slot))
        {
            *slot = None;
        }
    }

    /// Sets the vibration of the pad shown in a slot; false when none is.
    pub(crate) fn vibrate(&mut self, index: u32, vibration: PadVibration) -> bool {
        let Some(id) = usize::try_from(index)
            .ok()
            .and_then(|slot| self.slots.get(slot))
            .copied()
            .flatten()
        else {
            return false;
        };
        let mut shared = self.shared.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !shared.pads.iter().any(|pad| pad.id == id) {
            return false;
        }
        match shared.vibration.iter_mut().find(|(pad, _)| *pad == id) {
            Some((_, current)) => *current = vibration,
            None => shared.vibration.push((id, vibration)),
        }
        true
    }
}

impl Drop for SdlPads {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// How SDL reports a pad's sticks.
#[derive(Clone, Copy)]
enum Sticks {
    /// PlayStation: each 8-bit stick byte as `byte * 257 - 32768`
    /// (`HIDAPI_DriverPS4_HandleStatePacket`, and the PS3 and PS5 drivers).
    Bytes,
    /// Xbox: the pad's own values, Y as `~y` (SDL's Xbox drivers, and Linux's xpad).
    Xbox,
    /// Any other pad: SDL's values, Y pointing down.
    Other,
}

impl Sticks {
    fn of(kind: GamepadType) -> Self {
        match kind {
            GamepadType::PS3 | GamepadType::PS4 | GamepadType::PS5 => Self::Bytes,
            GamepadType::Xbox360 | GamepadType::XboxOne => Self::Xbox,
            _ => Self::Other,
        }
    }
}

/// A pad this thread opened.
struct Open {
    id: u32,
    gamepad: Gamepad,
    sticks: Sticks,
    sample: Option<XboxPadSample>,
    packet: u32,
    /// The vibration last sent, and when.
    rumble: Option<(PadVibration, Instant)>,
}

fn mark_ready((lock, started): &SharedState) {
    lock.lock().unwrap_or_else(PoisonError::into_inner).ready = true;
    started.notify_all();
}

fn run(shared: &SharedState, stop: &AtomicBool) -> Result<(), sdl3::Error> {
    // Pads read whatever window has focus, as XInput does; the game applies its own rule.
    sdl3::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
    // SDL would turn SIGINT and SIGTERM into quit events, which nothing reads: the game would
    // outlive Ctrl+C, `kill` and a launcher's stop.
    sdl3::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
    let sdl = sdl3::init()?;
    let joysticks = sdl.joystick()?;
    let gamepads = sdl.gamepad()?;
    // This thread updates the pads itself, and nothing drains SDL's event queue.
    joysticks.set_joystick_events_enabled(false);
    gamepads.set_events_processing_state(false);
    let mut open: Vec<Open> = Vec::new();
    // Xbox pads on Windows, which XInput reads.
    let mut skipped: Vec<u32> = Vec::new();
    let mut scanned: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) {
        gamepads.update();
        if scanned.is_none_or(|at| at.elapsed() >= SCAN) {
            scanned = Some(Instant::now());
            let listed = gamepads.gamepads().unwrap_or_default();
            open.retain(|pad| pad.gamepad.connected());
            skipped.retain(|id| listed.iter().any(|listed| listed.raw() == *id));
            for id in listed {
                let raw = id.raw();
                if skipped.contains(&raw) || open.iter().any(|pad| pad.id == raw) {
                    continue;
                }
                let Ok(gamepad) = gamepads.open(id) else {
                    continue;
                };
                match gamepad.r#type() {
                    GamepadType::Xbox360 | GamepadType::XboxOne if cfg!(windows) => skipped.push(raw),
                    kind => open.push(Open {
                        id: raw,
                        gamepad,
                        sticks: Sticks::of(kind),
                        sample: None,
                        packet: 0,
                        rumble: None,
                    }),
                }
            }
        }
        let mut state = shared.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !state.ready {
            let names: Vec<String> = open
                .iter()
                .map(|pad| format!("{:?} {:?}", pad.gamepad.name(), pad.gamepad.r#type()))
                .collect();
            eprintln!("diagnostic SDL gamepads: {names:?}");
        }
        state.pads.clear();
        for pad in &mut open {
            let sample = sample(&pad.gamepad, pad.sticks);
            if pad.sample != Some(sample) {
                pad.sample = Some(sample);
                pad.packet = pad.packet.wrapping_add(1);
            }
            state.pads.push(Pad {
                id: pad.id,
                sample,
                packet: pad.packet,
            });
        }
        state
            .vibration
            .retain(|(id, _)| open.iter().any(|pad| pad.id == *id));
        let wanted = state.vibration.clone();
        let first = !state.ready;
        drop(state);
        if first {
            mark_ready(shared);
        }
        let silenced = SILENCED.load(Ordering::Relaxed);
        for (id, vibration) in wanted {
            if let Some(pad) = open.iter_mut().find(|pad| pad.id == id) {
                rumble(pad, if silenced { STILL } else { vibration });
            }
        }
        std::thread::sleep(POLL);
    }
    // A pad keeps its last rumble command: leave none running.
    for pad in &mut open {
        rumble(pad, STILL);
    }
    Ok(())
}

/// XInput's low- and high-frequency motors are SDL's; the PS4 driver sends each one's top byte
/// to the matching motor, as DS4Windows did.
fn rumble(pad: &mut Open, vibration: PadVibration) {
    let due = match pad.rumble {
        Some((sent, at)) => {
            sent != vibration || (vibration != STILL && at.elapsed() >= RUMBLE_RESEND)
        }
        None => vibration != STILL,
    };
    if due {
        let duration = if vibration == STILL { 0 } else { RUMBLE_MS };
        // A pad that cannot rumble keeps playing, as an Xbox pad without motors would.
        let _ = pad
            .gamepad
            .set_rumble(vibration.left, vibration.right, duration);
        pad.rumble = Some((vibration, Instant::now()));
    }
}

fn sample(gamepad: &Gamepad, sticks: Sticks) -> XboxPadSample {
    let stick = |axis: Axis, up: bool| {
        let value = gamepad.axis(axis);
        match sticks {
            Sticks::Bytes => ds4windows_axis(stick_byte(value), up),
            Sticks::Xbox if up => !value,
            Sticks::Other if up => value.saturating_neg(),
            Sticks::Xbox | Sticks::Other => value,
        }
    };
    XboxPadSample {
        buttons: BUTTONS
            .iter()
            .filter(|(button, _)| gamepad.button(*button))
            .fold(0, |bits, (_, bit)| bits | bit),
        left_trigger: trigger_byte(gamepad.axis(Axis::TriggerLeft)),
        right_trigger: trigger_byte(gamepad.axis(Axis::TriggerRight)),
        left_x: stick(Axis::LeftX, false),
        left_y: stick(Axis::LeftY, true),
        right_x: stick(Axis::RightX, false),
        right_y: stick(Axis::RightY, true),
    }
}

/// The stick byte behind SDL's `byte * 257 - 32768`.
fn stick_byte(value: i16) -> u8 {
    u8::try_from(((i32::from(value) + 32768 + 128) / 257).clamp(0, 255)).expect("clamped to a byte")
}

/// The trigger byte behind SDL's 0..32767 trigger, which `SDL_GetGamepadAxis` normalises from
/// `byte * 257 - 32768` in single precision. XInput and DS4Windows pass the byte through.
fn trigger_byte(value: i16) -> u8 {
    u8::try_from(((i32::from(value).max(0) * 255 + 16383) / 32767).min(255))
        .expect("clamped to a byte")
}

/// DS4Windows 1.4 `X360Device.Scale` (ScpVBus): the byte less 0x80, -128 raised to -127,
/// negated for an up-positive Y, times 32767/127 in single precision and truncated.
fn ds4windows_axis(byte: u8, up: bool) -> i16 {
    let mut value = i32::from(byte) - 0x80;
    if value == -128 {
        value = -127;
    }
    if up {
        value = -value;
    }
    i16::try_from((value as f32 * 258.007_874_015_748_031_496_062_992_125_99_f32) as i32)
        .expect("127 * 32767/127 fits an i16")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ds4windows_axis_is_scp_vbus_scale() {
        assert_eq!(ds4windows_axis(0x80, false), 0);
        assert_eq!(ds4windows_axis(0xff, false), 32767);
        assert_eq!(ds4windows_axis(0x00, false), -32767);
        assert_eq!(ds4windows_axis(0x01, false), -32767);
        assert_eq!(ds4windows_axis(0x7f, false), -258);
        assert_eq!(ds4windows_axis(0x81, false), 258);
        assert_eq!(ds4windows_axis(0x00, true), 32767);
        assert_eq!(ds4windows_axis(0xff, true), -32767);
        for offset in 1..=127_u8 {
            assert_eq!(
                ds4windows_axis(0x80 + offset, false),
                -ds4windows_axis(0x80 - offset, false)
            );
            assert_eq!(
                ds4windows_axis(0x80 + offset, true),
                ds4windows_axis(0x80 - offset, false)
            );
        }
    }

    #[test]
    fn sdl_playstation_bytes_come_back_exactly() {
        for byte in 0..=255_u8 {
            let axis = i16::try_from(i32::from(byte) * 257 - 32768).unwrap();
            assert_eq!(stick_byte(axis), byte);
            // SDL_GetGamepadAxis: a full-range input bound to the 0..32767 trigger output.
            let normalized = (i32::from(axis) + 32768) as f32 / 65535_f32;
            let trigger = i16::try_from((normalized * 32767_f32) as i32).unwrap();
            assert_eq!(trigger_byte(trigger), byte);
        }
    }
}
