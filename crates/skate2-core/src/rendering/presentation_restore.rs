//! `DataSysMan`'s presentation side: each tick's submissions (`SubmitFrame` 0x8277bca0)
//! and, each rendered frame, the clock and the restore of every presentation between
//! two submitted frames (`StepTime` 0x8277c1d0, `RestoreFrame` 0x8277f040,
//! `DataMan::RestoreData` 0x82996580).
//!
//! The PC clock replaces `RealTimeController`, which the original paces for a 60 Hz
//! display: each frame shows the moment one tick period plus the ticks' delivery
//! latency behind it, on the timeline of the ticks' scheduled due times, so every
//! frame shows a new, evenly spaced moment at any refresh rate.
//! `SKATE2_NATIVE_PRESENT_CLOCK` restores with `RealTimeController` instead.

#[path = "presentation_restore_camera.rs"]
pub mod camera;
#[path = "presentation_restore_clock.rs"]
pub mod clock;
#[path = "presentation_restore_living_world.rs"]
pub mod living_world;
#[path = "presentation_restore_math.rs"]
pub(crate) mod math;
#[path = "presentation_restore_skater.rs"]
pub(crate) mod skater;

use crate::{
    actor_face_playback::presentation::frame::selected_runtime::render_opacity::SkaterRenderOpacity,
    animation::AnimationMatrix,
    character::{
        material_admission::CharacterMaterialAdmission,
        palette::{CharacterPalette, CharacterPaletteIdentity},
    },
    skater_ropa_runtime::ClothClock,
};
use camera::{CameraRestore, CameraSubmission, CameraView};
use clock::{NORMAL_UPDATE_RATE, PresentationFrames, RealTimeController};
use living_world::{LivingWorldRestore, LivingWorldSubmission, VehicleWheels};
use skater::{SkaterFrame, SkaterRestore};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// One tick's `SubmitFrame`: what each presentation's `DoSimSubmit` recorded.
pub struct PresentationSubmission {
    pub(crate) camera: CameraSubmission,
    /// PC port addition, Skate While Waiting: the local game camera while `camera` shows another
    /// player's, for the corner the local skater's own view draws in.
    pub(crate) pip_camera: Option<CameraSubmission>,
    pub(crate) skaters: Vec<SkaterFrame>,
    pub(crate) living_world: LivingWorldSubmission,
    /// The completed simulation ticks this submission follows.
    pub(crate) tick: u32,
    /// `SimPresentation`'s `mLastKnownSimRate`, which the clock's query answers.
    pub(crate) sim_rate: f32,
}

/// A submission with the time the fixed-step schedule made its tick due.
struct Submitted {
    due: Instant,
    submission: Arc<PresentationSubmission>,
}

/// The frames a restore may blend between: the newest, the one before it, and the
/// one before that, which the moment shown keeps blending toward until it passes it.
const TRACKED_FRAMES: usize = 3;

/// The ticks whose delivery latency sets the PC clock's delay.
const DELIVERY_SAMPLES: usize = 120;

/// Added to the latency the PC clock waits for, so a tick that arrives as late as the
/// slow end of the recent ones still arrives before it is shown.
const DELIVERY_MARGIN: Duration = Duration::from_micros(500);

/// A skater as the last restore left it.
pub struct RestoredSkater<'a> {
    pub palette: &'a CharacterPalette,
    /// The restored root: rotation rows, then the translation row.
    pub root: [f32; 16],
    /// `GetHipsPos`.
    pub hips_position: [f32; 3],
    /// `GetHipsMatrix`'s Z row.
    pub hips_forward: [f32; 3],
    /// `m_BB`, min then max.
    pub bounds: [[f32; 3]; 2],
    /// `i_params[0].x`.
    pub opacity: f32,
    /// `mIsGhosted`.
    pub ghosted: bool,
    /// The entity's materials.
    pub materials: Option<&'a Arc<Mutex<CharacterMaterialAdmission>>>,
}

/// How each frame picks the moment it restores.
enum Clock {
    /// `RealTimeController`, stepped with each frame's elapsed time.
    Native(RealTimeController),
    /// The PC clock over the ticks' due times.
    Scheduled,
}

/// The submitted frames, the clock and every presentation's restored state.
pub struct PresentationRestorer {
    frames: PresentationFrames<Submitted>,
    clock: Option<Clock>,
    native: bool,
    /// How long after its due time each recent tick reached the presentation.
    latencies: VecDeque<Duration>,
    /// The latency the PC clock waits for: the recent ticks' 95th percentile.
    delivery: Duration,
    camera: CameraRestore,
    /// The recorded game camera a replay restores beside the scene's camera, which the scene's
    /// camera reads (`CameraPresentation::GetReplayMatrix`) but does not show.
    recorded_camera: CameraRestore,
    /// The local game camera beside a shown camera of another player (PC port addition, Skate
    /// While Waiting), and whether the last restore had one.
    pip_camera: CameraRestore,
    pip_shown: bool,
    skaters: Vec<SkaterRestore>,
    living_world: LivingWorldRestore,
    /// The frame and fraction bits the last restore showed.
    restored: Option<(u32, u32)>,
}

impl Default for PresentationRestorer {
    fn default() -> Self {
        Self::new()
    }
}

impl PresentationRestorer {
    pub fn new() -> Self {
        Self {
            frames: PresentationFrames::new(TRACKED_FRAMES),
            clock: None,
            native: std::env::var_os("SKATE2_NATIVE_PRESENT_CLOCK").is_some(),
            latencies: VecDeque::with_capacity(DELIVERY_SAMPLES),
            delivery: Duration::ZERO,
            camera: CameraRestore::new(),
            recorded_camera: CameraRestore::new(),
            pip_camera: CameraRestore::new(),
            pip_shown: false,
            skaters: Vec::new(),
            living_world: LivingWorldRestore::default(),
            restored: None,
        }
    }

    /// Track a tick's submission, due at `due` and reaching the presentation at
    /// `delivered`. The clock starts (`EnterRealTime`) once a frame precedes the newest,
    /// so every restore has a frame to blend from.
    pub fn submit(&mut self, submission: Arc<PresentationSubmission>, due: Instant, delivered: Instant) {
        let latency = delivered.saturating_duration_since(due);
        let period = Duration::try_from_secs_f32(submission.sim_rate).unwrap_or(Duration::ZERO);
        self.frames.submit(Submitted { due, submission });
        // A tick later than its own period is the simulation catching up after a stall (a
        // save, a load), not the spread of its delivery. Counted, it would set the delay
        // for the next ticks too, and a delay past a period leaves the moment shown behind
        // every tracked frame: each frame between ticks would repeat the last one.
        if latency <= period {
            if self.latencies.len() == DELIVERY_SAMPLES {
                self.latencies.pop_front();
            }
            self.latencies.push_back(latency);
            let mut sorted: Vec<Duration> = self.latencies.iter().copied().collect();
            sorted.sort_unstable();
            self.delivery = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)] + DELIVERY_MARGIN;
        }
        if self.clock.is_none() && self.frames.len() >= 2 {
            self.clock = Some(if self.native {
                Clock::Native(RealTimeController::new(&self.frames, NORMAL_UPDATE_RATE))
            } else {
                Clock::Scheduled
            });
        }
    }

    /// Forget the submitted frames and the clock: the presentation's time was cut (a
    /// pause, a menu or a jump of the world), so the next ticks start a new run.
    pub fn reset(&mut self) {
        self.frames.clear();
        self.clock = None;
        self.restored = None;
        self.latencies.clear();
        self.living_world.reset();
    }

    /// Whether a restore can show anything yet.
    pub fn is_running(&self) -> bool {
        self.clock.is_some()
    }

    /// The PC clock's moment for a frame drawn at `now`: one tick period plus the
    /// delivery latency earlier, between the two frames whose due times bracket it.
    fn scheduled_moment(&self, now: Instant) -> Option<(u32, f32)> {
        let frames: Vec<(u32, &Submitted)> = self.frames.iter().collect();
        let [.., (_, before), (newest, latest)] = frames.as_slice() else {
            return None;
        };
        let period = latest.due.checked_duration_since(before.due)?;
        let oldest_pair = frames[1].0;
        let Some(shown) = now.checked_sub(period + self.delivery) else {
            return Some((oldest_pair, 0.0));
        };
        if shown >= latest.due {
            return Some((*newest, 1.0));
        }
        for pair in frames.windows(2).rev() {
            let [(_, previous), (frame, next)] = pair else {
                continue;
            };
            if shown >= previous.due {
                let span = next
                    .due
                    .saturating_duration_since(previous.due)
                    .as_secs_f64();
                let into = shown.saturating_duration_since(previous.due).as_secs_f64();
                #[expect(clippy::cast_possible_truncation, reason = "a fraction of one tick")]
                let fraction = if span > 0.0 {
                    (into / span).min(1.0) as f32
                } else {
                    1.0
                };
                return Some((*frame, fraction));
            }
        }
        Some((oldest_pair, 0.0))
    }

    /// `StepTime`, then `RestoreFrame`: blend every presentation toward the frame the
    /// clock picks for a frame drawn at `now`, `dt` after the previous one. Whether
    /// the restored image changed.
    ///
    /// # Errors
    /// A skater whose pose or palette cannot be restored.
    pub fn step(&mut self, dt: f32, now: Instant) -> Result<bool, String> {
        let moment = if matches!(self.clock, Some(Clock::Scheduled)) {
            self.scheduled_moment(now)
        } else if let Some(Clock::Native(controller)) = &mut self.clock {
            let rate = self
                .frames
                .newest()
                .map_or(NORMAL_UPDATE_RATE, |submitted| {
                    submitted.submission.sim_rate
                });
            controller.update(dt, &self.frames, rate);
            let info = controller.time_info();
            Some((info.frame, info.fraction))
        } else {
            None
        };
        match moment {
            Some((frame, fraction)) => self.restore(frame, fraction, dt),
            None => Ok(false),
        }
    }

    /// `DataMan::RestoreData`: every presentation between `frame`'s predecessor and
    /// `frame`, `fraction` of the way, for a frame drawn `elapsed` after the previous one.
    fn restore(&mut self, frame: u32, fraction: f32, elapsed: f32) -> Result<bool, String> {
        let Some(next) = self
            .frames
            .get(frame)
            .map(|submitted| Arc::clone(&submitted.submission))
        else {
            return Ok(false);
        };
        let previous = self
            .frames
            .previous(frame)
            .map(|submitted| Arc::clone(&submitted.submission));
        self.restore_between(previous.as_deref(), &next, frame, fraction, None, elapsed)
    }

    /// `DataMan::RestoreData` over two frames the replay recorder kept: every presentation
    /// `fraction` of the way from `previous` to `next`, which a replay's
    /// `SceneReplayTimeController` (`0x828727e8`) picked, restoring `forward` or back. The
    /// scene's camera stands at `camera` and shows; the recorded game camera restores beside it
    /// ([`Self::replay_matrix`]). The skaters' cloth steps by the replay's frame and fraction, and
    /// their fades by `elapsed`, the presentation time this restore stands for.
    /// Whether the restored image changed.
    ///
    /// # Errors
    /// A skater whose pose or palette cannot be restored.
    #[expect(clippy::too_many_arguments, reason = "one recorded frame's restore inputs")]
    pub fn restore_recorded(
        &mut self,
        previous: Option<&PresentationSubmission>,
        next: &PresentationSubmission,
        frame: u32,
        fraction: f32,
        camera: [f32; 3],
        forward: bool,
        elapsed: f32,
    ) -> Result<bool, String> {
        self.recorded_camera.restore(&next.camera, previous.map(|frame| &frame.camera), fraction, forward);
        self.restore_between(previous, next, frame, fraction, Some(camera), elapsed)
    }

    fn restore_between(
        &mut self,
        previous: Option<&PresentationSubmission>,
        next: &PresentationSubmission,
        frame: u32,
        fraction: f32,
        scene_camera: Option<[f32; 3]>,
        elapsed: f32,
    ) -> Result<bool, String> {
        let moment = (frame, fraction.to_bits());
        let mut changed = self.restored != Some(moment);
        self.restored = Some(moment);
        let camera = if let Some(camera) = scene_camera {
            camera
        } else {
            // Both clocks restore forward: the controller's delta is positive.
            self.camera.restore(
                &next.camera,
                previous.map(|frame| &frame.camera),
                fraction,
                true,
            );
            self.camera.view().map_or([0.0; 3], |view| view.position)
        };
        // The local game camera beside another player's, blended as the shown one is; a first
        // frame of it (the frame before had none) stands where it is.
        self.pip_shown = false;
        if scene_camera.is_none()
            && let Some(pip) = next.pip_camera.as_ref()
        {
            let before = previous.and_then(|frame| frame.pip_camera.as_ref()).unwrap_or(pip);
            self.pip_camera.restore(pip, Some(before), fraction, true);
            self.pip_shown = true;
        }
        let clock = ClothClock { frame, fraction };
        let mut retained = std::mem::take(&mut self.skaters);
        for skater in &next.skaters {
            let identity = skater.palette_identity();
            let mut restore = match retained
                .iter()
                .position(|restore| restore.palette.identity() == identity)
            {
                Some(index) => retained.swap_remove(index),
                None => SkaterRestore::new(
                    CharacterPalette::clone(&skater.palette),
                    SkaterRenderOpacity::constructed(),
                ),
            };
            let before = restore.opacity;
            let paired = previous.and_then(|previous| {
                previous
                    .skaters
                    .iter()
                    .find(|other| other.pairs_with(skater))
            });
            restore
                .restore(paired, skater, clock, camera, scene_camera.is_some(), elapsed)
                .map_err(|error| error.to_string())?;
            changed |= restore.opacity != before;
            self.skaters.push(restore);
        }
        self.living_world
            .restore(
                previous.map(|frame| &frame.living_world),
                &next.living_world,
                fraction,
            )
            .map_err(|error| format!("living world restore: {error}"))?;
        Ok(changed)
    }

    /// The camera the last restore left (`RenderMan::UpdateCamera`).
    pub fn camera(&self) -> Option<CameraView> {
        self.camera.view()
    }

    /// The local game camera the last restore left beside another player's shown camera (PC port
    /// addition, Skate While Waiting); none when the restored frame showed the game camera.
    #[must_use]
    pub fn pip_camera(&self) -> Option<CameraView> {
        if self.pip_shown { self.pip_camera.view() } else { None }
    }

    /// `CameraPresentation::GetReplayMatrix` (`0x826cf400`) in a replay: the recorded game
    /// camera the last replay restore left, Skate rows.
    pub fn replay_matrix(&self) -> Option<[f32; 16]> {
        self.recorded_camera.replay_matrix()
    }

    /// The moment the last restore showed, in ticks since the run's first frame: the
    /// frame before the restored one plus the fraction toward it.
    pub fn shown_moment(&self) -> Option<f64> {
        let (frame, fraction) = self.restored?;
        Some(f64::from(frame) - 1.0 + f64::from(f32::from_bits(fraction)))
    }

    /// The restore data's frame and fraction as the presentations step their own
    /// clocks by them (`WorldPresentation::Update` 0x827d3af8): the restored frame's
    /// completed ticks, and the fraction of the way to it.
    pub fn shown_tick(&self) -> Option<(u32, f32)> {
        let (frame, fraction) = self.restored?;
        let submitted = self.frames.get(frame)?;
        Some((submitted.submission.tick, f32::from_bits(fraction)))
    }

    /// The restored skater drawn with this palette.
    pub fn skater(&self, palette: &CharacterPaletteIdentity) -> Option<RestoredSkater<'_>> {
        let restore = self
            .skaters
            .iter()
            .find(|restore| restore.palette.identity() == *palette)?;
        Some(RestoredSkater {
            palette: &restore.palette,
            root: restore.root,
            hips_position: restore.hips_position,
            hips_forward: restore.hips_forward,
            bounds: restore.bounds,
            opacity: restore.opacity.opacity,
            ghosted: restore.opacity.ghosted,
            materials: restore.materials.as_ref(),
        })
    }

    /// The restored transform of the pedestrian `id`: x, y and z axis rows, then the
    /// translation row.
    pub fn pedestrian(&self, id: u32) -> Option<AnimationMatrix> {
        self.living_world.pedestrian(id)
    }

    /// The restored transform and wheels of the vehicle `id`.
    pub fn vehicle(&self, id: u32) -> Option<(AnimationMatrix, VehicleWheels)> {
        self.living_world.vehicle(id)
    }

    /// The restored part transform of the dynamic object `id`.
    pub fn dynamic_object(&self, id: u32) -> Option<AnimationMatrix> {
        self.living_world.dynamic_object(id)
    }
}

#[cfg(test)]
#[path = "presentation_restore_tests.rs"]
mod tests;
