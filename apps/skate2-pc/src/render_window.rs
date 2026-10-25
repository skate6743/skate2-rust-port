//! Native diagnostic window. Display cadence is not the simulation timestep.
use crate::gpu_renderer::{GpuRenderer, GpuScene, RenderResult};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[path = "frame_profile.rs"]
mod profile;

#[derive(Debug)]
struct CapturePending;
impl std::fmt::Display for CapturePending {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("media map is still loading")}
}
impl std::error::Error for CapturePending {}
fn retry_capture(error:&(dyn std::error::Error+'static))->bool {
    error.is::<CapturePending>()||error.is::<crate::gpu_renderer::SurfaceUnavailable>()
}

#[derive(Clone, Copy, Debug)]
pub struct ClientFrameTimings {
    pub(crate) ticks: u32,
    pub(crate) worker_ticks: u32,
    pub(crate) simulation_debt: Duration,
    pub(crate) sim_lock_wait: Duration,
    pub(crate) sim_ready_wait: Duration,
    pub(crate) tick_completion: Duration,
    pub(crate) tick_finish: Duration,
    pub(crate) post_tick_presentation: Duration,
    pub(crate) catch_up_budget: u32,
    pub(crate) tick: Duration,
    pub(crate) snapshot: Duration,
    pub(crate) view: Duration,
    pub(crate) position: [f32; 3],
    pub(crate) core: [Duration; skate2_core::diagnostics::frame_profile::COUNT],
    pub(crate) vehicles: VehicleFrameProbe,
}

impl Default for ClientFrameTimings {
    fn default() -> Self {
        Self {
            ticks: 0,
            worker_ticks: 0,
            simulation_debt: Duration::ZERO,
            sim_lock_wait: Duration::ZERO,
            sim_ready_wait: Duration::ZERO,
            tick_completion: Duration::ZERO,
            tick_finish: Duration::ZERO,
            post_tick_presentation: Duration::ZERO,
            catch_up_budget: 0,
            tick: Duration::ZERO,
            snapshot: Duration::ZERO,
            view: Duration::ZERO,
            position: [0.0; 3],
            core: [Duration::ZERO; skate2_core::diagnostics::frame_profile::COUNT],
            vehicles: VehicleFrameProbe::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct VehicleFrameProbe {
    pub(crate) visuals: usize,
    pub(crate) draws: usize,
    pub(crate) opacity: f32,
    pub(crate) position: [f32; 3],
    pub(crate) palette_origin: [f32; 3],
    pub(crate) palette_axis_lengths: [f32; 3],
    pub(crate) palette_rows: usize,
}
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

/// Host owns input mapping, fixed-step scheduling and actual retained scene/pose.
pub trait RenderWindowClient {
    fn frame_timings(&self) -> ClientFrameTimings {
        ClientFrameTimings::default()
    }
    fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuScene>;
    fn window_event(&mut self, event: &WindowEvent) -> RenderResult<()>;
    fn text_entry_active(&self) -> bool { false }
    /// Alt+Enter: the client flips its Fullscreen setting.
    fn toggle_fullscreen(&mut self) {}
    /// The client closes the window, as the window's close button does.
    fn exit_requested(&self) -> bool {
        false
    }
    fn frame(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
    ) -> RenderResult<()>;

    /// A client-defined completed result ready for presentation. Inspection
    /// and other window clients retain the ordinary presented-frame bound.
    fn completion_reached(&self) -> bool {
        false
    }

    /// Whether capture and exit are bounded by client completions instead of
    /// the window's ordinary presented-frame count.
    fn completion_bound_active(&self) -> bool {
        false
    }

    /// Called only after a frame containing the requested client completion
    /// was successfully presented (and captured, when capture was requested).
    fn record_presented_completion(&mut self) {}
    /// Also called for ordinary frames, only after a successful presentation.
    fn frame_presented(&mut self, _scene: &GpuScene) {}
    fn career_photo_saved(&mut self, _slot: u32) {}
    fn replay_media_saved(&mut self, _replay:bool, _result:Result<String,String>) {}
    fn refresh_media_capture(&mut self,_renderer:&GpuRenderer,_scene:&mut GpuScene)->RenderResult<()> {Ok(())}
    fn media_capture_ready(&self)->bool {true}
    fn begin_replay_export(&mut self)->RenderResult<()> {Err("this client has no replay editor".into())}
    fn prepare_replay_export_frame(&mut self,_renderer:&GpuRenderer,_scene:&mut GpuScene,_frame:u64)->RenderResult<skate2_core::ui::replay_editor::ExportStatus> {Err("this client has no replay editor".into())}
    fn replay_export_audio(&mut self)->RenderResult<Vec<f32>> {Err("this client has no replay audio".into())}
    fn end_replay_export(&mut self)->RenderResult<()> {Ok(())}
    fn replay_media_cancelled(&mut self) {}
}

#[derive(Default)]
pub struct RenderWindowOptions {
    /// Number of successfully presented frames; None runs until close.
    pub frame_limit: Option<u64>,
    /// Capture the final finite frame, or first frame for an interactive run.
    pub capture: Option<std::path::PathBuf>,
    /// Explicit diagnostic title for this window client.
    pub title: Option<&'static str>,
    /// Keep the completed captured frame visible until Escape or window close.
    pub hold_at_completion: bool,
}

/// How often a minimized window runs its client's frame.
const MINIMIZED_FRAME: Duration = Duration::from_micros(8_333);

/// An unfocused window's frame that waits this long for its swapchain image or present, or
/// gets none, moves the client's frames to the minimized window's timer: a window another one
/// covers can stop taking frames, and each wait holds the simulation (wgpu's acquire gives up
/// only after a second).
const BACKGROUND_STALL: Duration = Duration::from_millis(250);

/// The window system's event loop. On Linux, winit takes Wayland whenever `WAYLAND_DISPLAY` is
/// set and gets no second attempt after a failed one, so a session whose Wayland socket is
/// missing (a stale variable) and that has an X server gets X11.
#[cfg(target_os = "linux")]
fn event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    use winit::platform::x11::EventLoopBuilderExtX11 as _;
    let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let socket = set("WAYLAND_DISPLAY").map(|display| {
        let display = std::path::PathBuf::from(display);
        if display.is_absolute() {
            display
        } else {
            set("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_default().join(display)
        }
    });
    let stale = set("WAYLAND_SOCKET").is_none() && socket.is_some_and(|socket| !socket.exists());
    if stale && set("DISPLAY").is_some() {
        eprintln!("the Wayland compositor's socket is missing: the window opens on X11");
        return winit::event_loop::EventLoopBuilder::new().with_x11().build();
    }
    EventLoop::new()
}

/// macOS: winit needs the principal class (`NSApplication`) to be its own, and another part of the
/// application (rfd's first-launch folder dialog) would take it first, which winit refuses. The
/// event loop is therefore made before that dialog and kept for [`run_with_options`].
#[cfg(target_os = "macos")]
pub fn prepare_event_loop() {
    EARLY_EVENT_LOOP.with(|slot| {
        if slot.borrow().is_none() {
            *slot.borrow_mut() = Some(EventLoop::new());
        }
    });
}

#[cfg(target_os = "macos")]
std::thread_local! {
    static EARLY_EVENT_LOOP: std::cell::RefCell<
        Option<Result<EventLoop<()>, winit::error::EventLoopError>>,
    > = const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "macos")]
fn event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    match EARLY_EVENT_LOOP.with(|slot| slot.borrow_mut().take()) {
        Some(prepared) => prepared,
        None => EventLoop::new(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    EventLoop::new()
}

pub fn run_with_options(
    mut client: impl RenderWindowClient + 'static,
    mut options: RenderWindowOptions,
) -> RenderResult<()> {
    if options.frame_limit == Some(0) {
        return Err("frame limit must be positive".into());
    }
    let event_loop = event_loop()?;
    let mut builder = WindowBuilder::new()
        .with_title(
            options
                .title
                .unwrap_or("Skate 2 — original geometry / diagnostic camera and shading"),
        )
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
    // The executable's icon (build.rs), in the title bar and on the taskbar.
    #[cfg(windows)]
    {
        use winit::platform::windows::{IconExtWindows, WindowBuilderExtWindows};
        let icon = || winit::window::Icon::from_resource(1, None).ok();
        builder = builder.with_window_icon(icon()).with_taskbar_icon(icon());
    }
    // SKATE2_DIAGNOSTIC_WINDOW_SIZE=WxH: a scripted run's logical window size.
    if let Some((width, height)) = std::env::var("SKATE2_DIAGNOSTIC_WINDOW_SIZE")
        .ok()
        .and_then(|size| size.split_once('x').and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))))
    {
        builder = builder.with_inner_size(winit::dpi::LogicalSize::new(width, height));
    }
    // SKATE2_DIAGNOSTIC_BACKGROUND_WINDOW: a scripted run's window opens inactive, left of
    // every monitor, so it draws and captures without covering the desktop.
    if std::env::var_os("SKATE2_DIAGNOSTIC_BACKGROUND_WINDOW").is_some() {
        let left = event_loop.available_monitors().map(|monitor| monitor.position().x).min().unwrap_or(0);
        builder = builder.with_position(winit::dpi::PhysicalPosition::new(left - 4000, 0)).with_active(false);
    }
    let window = Arc::new(builder.build(&event_loop)?);
    let mut renderer = pollster::block_on(GpuRenderer::new(Arc::clone(&window)))?;
    let mut scene = client.initialize(&renderer)?;
    let mut profile = profile::FrameProfile::from_environment();
    let profile_redraw = std::env::var_os("SKATE2_PROFILE_REDRAW").is_some();
    let failure = Arc::new(std::sync::Mutex::new(None::<String>));
    let event_failure = Arc::clone(&failure);
    let mut presented = 0_u64;
    // SKATE2_DIAGNOSTIC_CAPTURE_EVERY: besides the completion capture, one every given number
    // of seconds, written beside it as <capture>_<n>.png.
    let periodic = std::env::var("SKATE2_DIAGNOSTIC_CAPTURE_EVERY")
        .ok()
        .and_then(|seconds| seconds.parse::<f32>().ok())
        .filter(|seconds| *seconds > 0.)
        .map(std::time::Duration::from_secs_f32);
    let periodic_base = options.capture.clone();
    // SKATE2_DIAGNOSTIC_CAPTURE_FROM: the periodic captures start that many seconds in.
    let periodic_from = std::env::var("SKATE2_DIAGNOSTIC_CAPTURE_FROM")
        .ok()
        .and_then(|seconds| seconds.parse::<f32>().ok())
        .map_or(Duration::ZERO, Duration::from_secs_f32);
    let mut periodic_next = periodic.map(|every| Instant::now() + periodic_from + every);
    let mut periodic_index = 0_u32;
    let mut video_save:Option<crate::replay_media::VideoSave>=None;
    // SKATE2_DIAGNOSTIC_CAPTURE_COUNT: the periodic captures stop after that many.
    let periodic_count = std::env::var("SKATE2_DIAGNOSTIC_CAPTURE_COUNT")
        .ok()
        .and_then(|count| count.parse::<u32>().ok())
        .unwrap_or(u32::MAX);
    let mut alt = false;
    let mut focused = true;
    // Unfocused and its swapchain stalled: the client's frames run on the minimized timer and
    // draw nothing until the window is focused again or the cursor moves over it.
    let mut background = false;
    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Wait);
        if let Event::WindowEvent { window_id, event } = event {
            if window_id != window.id() {
                return;
            }
            let text_entry = matches!(event, WindowEvent::KeyboardInput { .. }) && client.text_entry_active();
            let result = client.window_event(&event).and_then(|()| {
                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Focused(now_focused) => {
                        renderer.set_focused(now_focused);
                        focused = now_focused;
                        if focused && background {
                            background = false;
                            window.request_redraw();
                        }
                    }
                    WindowEvent::CursorMoved { .. } if background => {
                        background = false;
                        window.request_redraw();
                    }
                    WindowEvent::ModifiersChanged(modifiers) => alt = modifiers.state().alt_key(),
                    WindowEvent::KeyboardInput { event, .. }
                        if alt && !text_entry && !event.repeat && event.state == winit::event::ElementState::Pressed
                            && matches!(event.physical_key, winit::keyboard::PhysicalKey::Code(
                                winit::keyboard::KeyCode::Enter | winit::keyboard::KeyCode::NumpadEnter)) =>
                    {
                        client.toggle_fullscreen();
                        window.request_redraw();
                    }
                    WindowEvent::KeyboardInput { event, .. }
                        if !text_entry && !event.repeat && event.state == winit::event::ElementState::Pressed
                            && event.physical_key
                                == winit::keyboard::PhysicalKey::Code(
                                    winit::keyboard::KeyCode::Escape,
                                ) =>
                    {
                        if video_save.take().is_some() {
                            renderer.end_media_capture();
                            client.end_replay_export()?;client.replay_media_cancelled();
                            window.request_redraw();
                        }
                    }
                    WindowEvent::Resized(size) => {
                        if std::env::var_os("SKATE2_DIAGNOSTIC_BACKGROUND_WINDOW").is_some() {
                            eprintln!("diagnostic: window resized to {}x{}", size.width, size.height);
                        }
                        renderer.resize(size.width, size.height);
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested if background => {}
                    WindowEvent::RedrawRequested => {
                        renderer.wait_for_frame_cap();
                        let redraw_start = Instant::now();
                        client.frame(&renderer, &mut scene, Instant::now())?;
                        if client.exit_requested() {
                            target.exit();
                            return Ok(());
                        }
                        let client_time = redraw_start.elapsed();
                        window.pre_present_notify();
                        if window.inner_size().width == 0 || window.inner_size().height == 0 {
                            renderer.submit_uploads();
                            return Ok(());
                        }
                        if let Some(slot) = scene.career_photo.take() {
                            let result=(||->RenderResult<()> {
                                renderer.begin_media_capture(1080)?;
                                client.refresh_media_capture(&renderer,&mut scene)?;
                                if !client.media_capture_ready(){return Err(Box::new(CapturePending));}
                                renderer.save_career_photo(&scene,slot)
                            })();
                            renderer.end_media_capture();
                            scene.career_photo=None;
                            match result {
                                Ok(()) => eprintln!("career: saved photo slot {slot}"),
                                Err(error) if retry_capture(error.as_ref()) => {
                                    scene.career_photo = Some(slot);
                                    window.request_redraw();
                                    return Ok(());
                                }
                                Err(error) => eprintln!("career: photo slot {slot} save failed: {error}"),
                            }
                            client.career_photo_saved(slot);
                            client.frame_presented(&scene);
                            presented += 1;
                            window.request_redraw();
                            return Ok(());
                        }
                        if let Some(request)=scene.replay_media.take() {
                            let replay=request.replay.is_some();
                            let result=(||->RenderResult<Option<String>> {
                                renderer.begin_media_capture(request.video.height)?;
                                client.refresh_media_capture(&renderer,&mut scene)?;
                                if !client.media_capture_ready(){return Err(Box::new(CapturePending));}
                                if replay {
                                    let (width,height,rgba)=renderer.capture_pixels(&scene,true)?;
                                    let save=crate::replay_media::VideoSave::begin(&request,width,height,&rgba)?;
                                    client.begin_replay_export()?;video_save=Some(save);Ok(None)
                                } else {renderer.save_replay_media(&scene,&request).map(Some)}
                            })();
                            match result {
                                Ok(None)=>(),
                                Ok(Some(path))=>{renderer.end_media_capture();client.replay_media_saved(replay,Ok(path));}
                                Err(error)=>{
                                    renderer.end_media_capture();
                                    if retry_capture(error.as_ref()) {
                                        scene.replay_media=Some(request);
                                    } else {client.replay_media_saved(replay,Err(error.to_string()));}
                                }
                            }
                            window.request_redraw();return Ok(());
                        }
                        if let Some(save)=video_save.as_mut() {
                            use skate2_core::ui::replay_editor::ExportStatus;
                            let result=(||->RenderResult<Option<Option<String>>> {
                                let status=client.prepare_replay_export_frame(&renderer,&mut scene,save.step())?;
                                let encoded=status==ExportStatus::Frame&&save.encodes_step();
                                match status {
                                    ExportStatus::Cancelled=>return Ok(Some(None)),
                                    ExportStatus::Finished=>save.finish_frames()?,
                                    ExportStatus::Waiting=>(),
                                    ExportStatus::Frame=>{
                                        // A 30 FPS video keeps every second 60 Hz step. The steps between
                                        // are still drawn, so per-frame post-fx adaptation runs as at 60.
                                        let rgba=if encoded {Some(renderer.capture_pixels(&scene,true)?.2)} else {None};
                                        let audio=client.replay_export_audio()?;
                                        save.push(rgba.as_deref(),&audio)?;
                                    }
                                }
                                if !encoded {renderer.render(&scene)?;}
                                Ok(save.poll()?.map(Some))
                            })();
                            match result {
                                Ok(None)=>(),
                                Err(error) if error.downcast_ref::<crate::gpu_renderer::SurfaceUnavailable>().is_some()=>(),
                                outcome=>{
                                    video_save.take();renderer.end_media_capture();
                                    let restored=client.end_replay_export().map_err(|e|e.to_string());
                                    match (outcome,restored) {
                                        (Ok(Some(None)),Ok(()))=>client.replay_media_cancelled(),
                                        (Ok(Some(Some(path))),Ok(()))=>client.replay_media_saved(true,Ok(path)),
                                        (_,Err(message))=>client.replay_media_saved(true,Err(message)),
                                        (Err(error),_)=>client.replay_media_saved(true,Err(error.to_string())),
                                        _=>unreachable!(),
                                    }
                                }
                            }
                            client.frame_presented(&scene);presented+=1;
                            window.request_redraw();return Ok(());
                        }
                        let completion_reached = client.completion_reached();
                        let capture_now = options.capture.is_some()
                            && if client.completion_bound_active() {
                                completion_reached
                            } else {
                                options
                                    .frame_limit
                                    .is_none_or(|limit| presented + 1 == limit)
                            };
                        if !capture_now
                            && periodic_index < periodic_count
                            && let (Some(every), Some(next), Some(base)) = (periodic, periodic_next, periodic_base.as_ref())
                            && Instant::now() >= next
                        {
                            periodic_index += 1;
                            periodic_next = Some(next + every);
                            let path = base.with_file_name(format!(
                                "{}_{periodic_index:03}.png",
                                base.file_stem().and_then(|stem| stem.to_str()).unwrap_or("capture")
                            ));
                            if let Err(error) = renderer.render_capture(&scene, &path) {
                                eprintln!("diagnostic capture {}: {error}", path.display());
                            }
                            client.frame_presented(&scene);
                            presented += 1;
                            window.request_redraw();
                            return Ok(());
                        }
                        // A frame with nothing new is not presented.
                        if scene.unchanged && !capture_now {
                            renderer.submit_uploads();
                            window.request_redraw();
                            return Ok(());
                        }
                        let did_present = if capture_now {
                            renderer.render_capture(
                                &scene,
                                options.capture.as_deref().ok_or("missing capture path")?,
                            )?;
                            options.capture = None;
                            true
                        } else {
                            match renderer.render(&scene) {
                                Ok(()) => {
                                    let last = renderer.last_frame();
                                    background = !focused && last.acquire + last.present >= BACKGROUND_STALL;
                                    true
                                }
                                Err(error) => match error.downcast_ref::<crate::gpu_renderer::SurfaceUnavailable>() {
                                    Some(crate::gpu_renderer::SurfaceUnavailable::Timeout
                                        | crate::gpu_renderer::SurfaceUnavailable::Occluded) => {
                                        background = !focused;
                                        false
                                    }
                                    Some(crate::gpu_renderer::SurfaceUnavailable::Lost
                                        | crate::gpu_renderer::SurfaceUnavailable::Outdated) => {
                                        let size = window.inner_size();
                                        renderer.resize(size.width, size.height);
                                        false
                                    }
                                    // Preserve shadow-selection and other rendering errors
                                    // for the caller, alongside fatal surface failures.
                                    _ => return Err(error),
                                },
                            }
                        };
                        if did_present {
                            client.frame_presented(&scene);
                            if let Some(profile) = &mut profile {
                                profile.record(redraw_start.elapsed(), client_time,
                                    client.frame_timings(), renderer.last_frame());
                            }
                            presented += 1;
                            if completion_reached {
                                client.record_presented_completion();
                            }
                        }
                        if profile_redraw {
                            let last = renderer.last_frame();
                            eprintln!(
                                "diagnostic redraw {presented}: client={}ms present={}ms acquire={}ms record={}ms submit={}ms map_objects={}/{} map_islands={}",
                                client_time.as_secs_f64() * 1000.0,
                                redraw_start
                                    .elapsed()
                                    .saturating_sub(client_time)
                                    .as_secs_f64()
                                    * 1000.0,
                                last.acquire.as_secs_f64() * 1000.0,
                                last.record.as_secs_f64() * 1000.0,
                                last.submit_present.as_secs_f64() * 1000.0,
                                last.map.drawn_objects,
                                last.map.objects,
                                last.map.drawn_islands,
                            );
                        }
                        if options.frame_limit == Some(presented)
                            || (completion_reached && did_present && !options.hold_at_completion)
                        {
                            target.exit();
                            return Ok(());
                        }
                        if !background
                            && window.inner_size().width != 0
                            && window.inner_size().height != 0
                            && !(completion_reached && options.hold_at_completion)
                        {
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
                Ok(())
            });
            if let Err(error) = result {
                if let Ok(mut failure) = event_failure.lock() {
                    *failure = Some(error.to_string());
                }
                target.exit();
            }
        } else if matches!(event, Event::Resumed) {
            window.request_redraw();
        } else if matches!(event, Event::AboutToWait) {
            let minimized = window.inner_size().width == 0 || window.inner_size().height == 0;
            // The Fullscreen display setting: a borderless window over its monitor, switched
            // between frames. The swapchain takes the new size before the next frame presents;
            // the Resized events of the switch wait in winit's queue until the next input. The
            // game takes no mouse input, so fullscreen hides the cursor over the window (winit
            // shows it again outside it).
            let fullscreen = renderer.fullscreen();
            if !minimized && fullscreen != window.fullscreen().is_some() {
                window.set_fullscreen(fullscreen.then_some(winit::window::Fullscreen::Borderless(None)));
                window.set_cursor_visible(!fullscreen);
                let size = window.inner_size();
                renderer.resize(size.width, size.height);
                window.request_redraw();
            }
            if minimized || background {
                // A minimized window gets no redraws and presents nothing, nor does
                // a background one; their client frames keep the simulation at
                // wall-clock pace, so the game neither stalls nor races to catch up
                // once restored, and online it keeps reporting to the server. With
                // nothing on screen each frame counts as shown, so the frontend
                // fade's scene switches, which wait for a presented frame, go on.
                if let Err(error) = client.frame(&renderer, &mut scene, Instant::now()) {
                    if let Ok(mut failure) = event_failure.lock() {
                        *failure = Some(error.to_string());
                    }
                    target.exit();
                    return;
                }
                renderer.submit_uploads();
                client.frame_presented(&scene);
                if client.exit_requested() {
                    target.exit();
                    return;
                }
                target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + MINIMIZED_FRAME));
            }
        }
    })?;
    let failure = failure
        .lock()
        .map_err(|_| "window failure holder poisoned")?
        .take();
    if let Some(message) = failure {
        return Err(message.into());
    }
    Ok(())
}
