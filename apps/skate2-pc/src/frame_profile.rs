//! Opt-in, bounded frame capture. No console/file writes during play.
//! CPU durations include host work/waits; separate GPU window means aggregate
//! asynchronous timestamps. Acquisition/submission may include backpressure.
//! `elapsed_s` counts from the host's start, as `SKATE2_LOG_ELAPSED` log lines do.
use super::ClientFrameTimings;
use crate::gpu_renderer::FrameDiagnostics;
use std::{
    collections::VecDeque,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

const MAX_WINDOWS: usize = 1800;
const METRICS: usize = BASE + skate2_core::diagnostics::frame_profile::COUNT;
const BASE: usize = 19;

#[derive(Clone, Copy)]
struct Sample {
    milliseconds: [f64; METRICS],
    client: ClientFrameTimings,
    render: FrameDiagnostics,
}
impl Default for Sample {
    fn default() -> Self {
        Self {
            milliseconds: [0.; METRICS],
            client: ClientFrameTimings::default(),
            render: FrameDiagnostics::default(),
        }
    }
}
/// Frames that presented a new simulation tick: their spacing against the 60 Hz step.
#[derive(Clone, Copy, Default)]
struct ImagePacing {
    ticks: u64,
    worker_ticks: u64,
    debt_max_ms: f64,
    lock_wait_ms: f64,
    ready_wait_ms: f64,
    completion_ms: f64,
    finish_ms: f64,
    presentation_ms: f64,
    catch_up_budget_max: u32,
    /// The on-screen frame counter's lowest value this window.
    fps_counter_min: u32,
    images: u64,
    max: f64,
    late: u64,
    deviation: f64,
    skips: u64,
}
struct Window {
    image: ImagePacing,
    gpu_samples: u64,
    gpu_means: [f64; 4],
    elapsed: f64,
    seconds: f64,
    frames: u64,
    mean: [f64; METRICS],
    worst: Sample,
    /// Each metric's highest value this window, whichever frame had it.
    peak: [f64; METRICS],
    p95: f64,
}
pub(super) struct FrameProfile {
    gpu_samples: u64,
    gpu_totals: [f64; 4],
    path: PathBuf,
    start: Instant,
    window_start: Instant,
    frames: u64,
    totals: [f64; METRICS],
    worst: Sample,
    peak: [f64; METRICS],
    durations: Vec<f64>,
    windows: VecDeque<Window>,
    image: ImagePacing,
    last_image: Option<Instant>,
}
impl FrameProfile {
    pub(super) fn from_environment() -> Option<Self> {
        let path = PathBuf::from(std::env::var_os("SKATE2_FRAME_PROFILE")?);
        let now = Instant::now();
        Some(Self {
            gpu_samples: 0,
            gpu_totals: [0.; 4],
            path,
            start: skate2_core::diagnostics::start(),
            window_start: now,
            frames: 0,
            totals: [0.; METRICS],
            worst: Sample::default(),
            peak: [0.; METRICS],
            durations: Vec::with_capacity(4096),
            windows: VecDeque::new(),
            image: ImagePacing::default(),
            last_image: None,
        })
    }

    pub(super) fn record(
        &mut self,
        redraw: Duration,
        client_time: Duration,
        client: ClientFrameTimings,
        render: FrameDiagnostics,
    ) {
        let mut durations = [Duration::ZERO; METRICS];
        durations[..BASE].copy_from_slice(&[
            redraw,
            client_time,
            client.tick,
            client.snapshot,
            client.view,
            render.acquire,
            render.record,
            render.dynamic_upload,
            render.dynamic_shadows,
            render.world_shadows,
            render.scene,
            render.submit_present,
            render.map_prepare,
            render.objects,
            render.pass_end,
            render.post,
            render.finish,
            render.submit,
            render.present,
        ]);
        durations[BASE..].copy_from_slice(&client.core);
        let milliseconds = durations.map(|value| value.as_secs_f64() * 1000.);
        let sample = Sample {
            milliseconds,
            client,
            render,
        };
        if render.fps_counter > 0
            && (self.image.fps_counter_min == 0 || render.fps_counter < self.image.fps_counter_min)
        {
            self.image.fps_counter_min = render.fps_counter;
        }
        self.image.ticks += u64::from(client.ticks);
        self.image.worker_ticks += u64::from(client.worker_ticks);
        self.image.debt_max_ms = self.image.debt_max_ms.max(client.simulation_debt.as_secs_f64() * 1000.);
        self.image.lock_wait_ms += client.sim_lock_wait.as_secs_f64() * 1000.;
        self.image.ready_wait_ms += client.sim_ready_wait.as_secs_f64() * 1000.;
        self.image.completion_ms += client.tick_completion.as_secs_f64() * 1000.;
        self.image.finish_ms += client.tick_finish.as_secs_f64() * 1000.;
        self.image.presentation_ms += client.post_tick_presentation.as_secs_f64() * 1000.;
        self.image.catch_up_budget_max = self.image.catch_up_budget_max.max(client.catch_up_budget);
        if client.ticks > 0 {
            let now = Instant::now();
            if let Some(last) = self.last_image.replace(now) {
                let interval = now.duration_since(last).as_secs_f64() * 1000.;
                self.image.images += 1;
                self.image.max = self.image.max.max(interval);
                self.image.late += u64::from(interval > 20.);
                self.image.deviation += (interval - 1000. / 60.).abs();
            }
            self.image.skips += u64::from(client.ticks > 1);
        }
        self.frames += 1;
        self.gpu_samples += render.gpu.samples;
        for (total, value) in self.gpu_totals.iter_mut().zip(render.gpu.totals_ms) {
            *total += value;
        }
        for (total, value) in self.totals.iter_mut().zip(milliseconds) {
            *total += value;
        }
        for (peak, value) in self.peak.iter_mut().zip(milliseconds) {
            *peak = peak.max(value);
        }
        if milliseconds[0] >= self.worst.milliseconds[0] {
            self.worst = sample;
        }
        // Bound temporary storage even with extraordinary redraw rates.
        if self.durations.len() < 16_384 {
            self.durations.push(milliseconds[0]);
        }
        if self.window_start.elapsed() >= Duration::from_secs(1) {
            self.finish_window();
        }
    }

    fn finish_window(&mut self) {
        if self.frames == 0 {
            return;
        }
        let now = Instant::now();
        self.durations.sort_unstable_by(f64::total_cmp);
        let p95 = self.durations[(self.durations.len() * 95).div_ceil(100) - 1];
        if self.windows.len() == MAX_WINDOWS {
            self.windows.pop_front();
        }
        self.windows.push_back(Window {
            image: self.image,
            gpu_samples: self.gpu_samples,
            gpu_means: self
                .gpu_totals
                .map(|total| total / self.gpu_samples.max(1) as f64),
            elapsed: now.duration_since(self.start).as_secs_f64(),
            seconds: now.duration_since(self.window_start).as_secs_f64(),
            frames: self.frames,
            mean: self.totals.map(|total| total / self.frames as f64),
            worst: self.worst,
            peak: self.peak,
            p95,
        });
        self.frames = 0;
        self.image = ImagePacing::default();
        self.gpu_samples = 0;
        self.gpu_totals = [0.; 4];
        self.totals = [0.; METRICS];
        self.worst = Sample::default();
        self.peak = [0.; METRICS];
        self.durations.clear();
        self.window_start = now;
    }

    fn save(&self) -> std::io::Result<()> {
        let mut file = std::io::BufWriter::new(std::fs::File::create(&self.path)?);
        write!(
            file,
            "elapsed_s,kind,window_fps,p95_redraw_ms,redraw_ms,client_ms,tick_ms,snapshot_ms,view_ms,acquire_ms,record_ms,dynamic_upload_ms,dynamic_shadow_cpu_ms,world_shadow_cpu_ms,scene_cpu_ms,submit_present_ms,map_prepare_ms,objects_ms,pass_end_ms,post_ms,finish_ms,submit_ms,present_ms"
        )?;
        for name in skate2_core::diagnostics::frame_profile::NAMES {
            write!(file, ",core_{name}_ms")?;
        }
        writeln!(
            file,
            ",worst_ticks,worst_x,worst_y,worst_z,worst_map_objects,worst_map_drawn,worst_map_islands,worst_world_shadow_islands,worst_dynamic_objects,worst_vehicle_visuals,worst_vehicle_draws,worst_car_opacity,worst_car_x,worst_car_y,worst_car_z,worst_car_palette_x,worst_car_palette_y,worst_car_palette_z,worst_car_palette_rows,worst_car_axis_x,worst_car_axis_y,worst_car_axis_z,window_gpu_samples,window_gpu_dynamic_ms,window_gpu_world_ms,window_gpu_scene_ms,window_gpu_overlay_ms,worst_map_calls,image_intervals,image_max_ms,image_late,image_mean_deviation_ms,image_skips,fps_counter_min,ticks_per_frame,worker_ticks_per_frame,simulation_ticks_per_second,sim_debt_max_ms,sim_lock_wait_mean_ms,sim_ready_wait_mean_ms,tick_completion_mean_ms,tick_finish_mean_ms,post_tick_presentation_mean_ms,catch_up_budget_max"
        )?;
        for window in &self.windows {
            for (kind, values) in [("mean", window.mean), ("worst", window.worst.milliseconds), ("peak", window.peak)] {
                write!(
                    file,
                    "{:.3},{kind},{:.2},{:.4}",
                    window.elapsed,
                    window.frames as f64 / window.seconds,
                    window.p95
                )?;
                for value in values {
                    write!(file, ",{value:.4}")?;
                }
                let sample = window.worst;
                write!(
                    file,
                    ",{},{:.3},{:.3},{:.3},{},{},{},{},{}",
                    sample.client.ticks,
                    sample.client.position[0],
                    sample.client.position[1],
                    sample.client.position[2],
                    sample.render.map.objects,
                    sample.render.map.drawn_objects,
                    sample.render.map.drawn_islands,
                    sample.render.world_shadow_islands,
                    sample.render.dynamic_objects
                )?;
                let car = sample.client.vehicles;
                write!(
                    file,
                    ",{},{},{:.4},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{:.5},{:.5},{:.5}",
                    car.visuals,
                    car.draws,
                    car.opacity,
                    car.position[0],
                    car.position[1],
                    car.position[2],
                    car.palette_origin[0],
                    car.palette_origin[1],
                    car.palette_origin[2],
                    car.palette_rows,
                    car.palette_axis_lengths[0],
                    car.palette_axis_lengths[1],
                    car.palette_axis_lengths[2]
                )?;
                write!(file, ",{}", window.gpu_samples)?;
                for mean in window.gpu_means {
                    if window.gpu_samples == 0 {
                        write!(file, ",")?;
                    } else {
                        write!(file, ",{mean:.4}")?;
                    }
                }
                let image = window.image;
                writeln!(
                    file,
                    ",{},{},{:.3},{},{:.3},{},{},{:.4},{:.4},{:.3},{:.3},{:.4},{:.4},{:.4},{:.4},{:.4},{}",
                    sample.render.map.draw_calls,
                    image.images,
                    image.max,
                    image.late,
                    image.deviation / image.images.max(1) as f64,
                    image.skips,
                    image.fps_counter_min,
                    image.ticks as f64 / window.frames as f64,
                    image.worker_ticks as f64 / window.frames as f64,
                    image.ticks as f64 / window.seconds,
                    image.debt_max_ms,
                    image.lock_wait_ms / window.frames as f64,
                    image.ready_wait_ms / window.frames as f64,
                    image.completion_ms / window.frames as f64,
                    image.finish_ms / window.frames as f64,
                    image.presentation_ms / window.frames as f64,
                    image.catch_up_budget_max
                )?;
            }
        }
        file.flush()
    }
}
impl Drop for FrameProfile {
    fn drop(&mut self) {
        self.finish_window();
        if let Err(error) = self.save() {
            std::eprintln!(
                "Could not save optional frame profile {}: {error}",
                self.path.display()
            );
        }
    }
}
