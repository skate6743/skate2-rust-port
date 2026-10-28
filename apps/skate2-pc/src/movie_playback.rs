//! The presentation's half of the front end's movies: `AptMovieIntegration::Render`
//! (0x82630198) over the game's `MoviePlayer` (`Update` 0x82695d98, `Render` 0x82690c58) and
//! `rw::movie::MoviePlayer`. The EA chunked file streams on a reader thread, which decodes the
//! sound as it goes (`AudioDecoder_EAXA`) into the audio system's movie renderer; a decoder
//! thread turns the video chunks into frames (`VideoDecoder_Vp6`, four renderables); each render
//! shows the frame `rw::movie::MoviePlayer::Decode` (0x82b85230) picks for the sound's rendering
//! position.
use crate::game_audio::GameAudio;
use crate::gpu_renderer::{GpuRenderer, movie::MovieDraw};
use skate2_audio::movie::MovieSound;
use skate2_core::ui::movie_player::{MoviePresentation, MovieRequest, MovieSettings};
use skate2_formats::ea_movie::{AudioHeader, ChunkKind, EaMovie, OwnedChunk, read_chunk};
use skate2_formats::ea_xa::EaXaDecoder;
use skate2_formats::mount::MountSet;
use skate2_formats::vp6::{Vp6Decoder, YuvFrame};
use std::collections::VecDeque;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::time::Instant;

/// `rw::movie::MoviePlayer::SetNumVideoRenderables(4)` from `MoviePlayer::Init` (0x826a3b18):
/// the frame shown and those decoded ahead of it.
const VIDEO_RENDERABLES: usize = 4;
/// The video chunks the reader runs ahead of the video decoder (three seconds), so that the
/// sound, which the files store just behind each frame, stays ahead of its renderer the way the
/// native streams' own buffers keep it (`MoviePlayer::Play` 0x82695bd8 opens them with 4 MB).
const VIDEO_CHUNKS_AHEAD: usize = 90;
/// `AptMovieIntegration::Render` draws the background for three more renders after a stop
/// request before `DoStop`.
const STOP_RENDERS: u32 = 3;

/// `MoviePlayer::eMOVIEPLAYER_STATUS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Completed,
    Interrupted,
    Playing,
}

/// `rw::movie::MoviePlayer::mState` from `PreparePlayer`: prepared, rendering, finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RwState {
    Prepared,
    Rendering,
    Finished,
}

struct DecodedFrame {
    number: u32,
    frame: YuvFrame,
}

/// One open movie: `MoviePlayer` with its `rw::movie` player.
struct Playback {
    status: Status,
    state: RwState,
    /// `mPlaybackFps`.
    fps: f32,
    /// `mTotalFrames`.
    total_frames: u32,
    frames: Receiver<DecodedFrame>,
    /// The filled renderables, oldest first; the shown one is at the front.
    filled: VecDeque<DecodedFrame>,
    /// The frame in the renderer's planes.
    uploaded: Option<u32>,
    /// `mLastRenderedFrame`.
    last_rendered: Option<u32>,
    /// The reader thread delivered every chunk of the file.
    read_all: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    /// An audio renderer took the sound.
    audio: bool,
    /// `MovieTimer`'s stopwatch, started by the first render.
    stopwatch: Option<Instant>,
    /// The quad's centre and scale (`mPositionX`, `mPositionY`, `mScaleX`, `mScaleY`).
    position: [f32; 2],
    scale: f32,
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// `AptMovieIntegration`'s render-side state.
#[derive(Default)]
pub(crate) struct MovieRenderer {
    /// `mRequests` taken from the front end, not yet run.
    requests: VecDeque<MovieRequest>,
    /// `mMoviePlayer`.
    playback: Option<Playback>,
    /// `mIsPlaying`.
    playing: bool,
    /// `mbStopRequested` and `mStopCount`.
    stop_requested: bool,
    stop_count: u32,
    /// `mSetVolume` and `mVolume`.
    set_volume: bool,
    volume: f32,
}

/// A movie's bytes as the reader takes them.
type MovieSource = BufReader<Box<dyn Read + Send>>;

/// The game root, from the Boot archive's path (`data\big\miscboot.big`).
fn game_root(mounts: &MountSet) -> Option<PathBuf> {
    let boot = mounts.archives().iter().find(|a| {
        a.data_path()
            .file_name()
            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("miscboot.big"))
    })?;
    boot.data_path()
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

impl MovieRenderer {
    /// One render: the audio system's movie calls at once, then `AptMovieIntegration::Render`.
    /// Returns what the front end learns and what the renderer draws.
    pub(crate) fn render(
        &mut self,
        requests: Vec<MovieRequest>,
        foreground: bool,
        mounts: &MountSet,
        audio: Option<&GameAudio>,
        renderer: &GpuRenderer,
    ) -> (MoviePresentation, Option<MovieDraw>) {
        let trace = std::env::var_os("SKATE2_TRACE_MOVIE").is_some();
        for request in requests {
            if trace {
                eprintln!("movie trace: request {request:?}");
            }
            match request {
                MovieRequest::PauseAudio => {
                    if let Some(audio) = audio {
                        audio.pause_for_movie();
                    }
                }
                MovieRequest::ResumeAudio => {
                    if let Some(audio) = audio {
                        audio.resume_after_movie();
                    }
                }
                request => self.requests.push_back(request),
            }
        }
        let mut draw = MovieDraw {
            foreground,
            background: false,
            video: None,
        };
        let mut stop_requested = false;
        self.render_movie(mounts, audio, renderer, &mut draw, &mut stop_requested);
        let presentation = MoviePresentation {
            has_player: self.playback.is_some(),
            playing: self.playing,
            stop_requested,
        };
        if trace {
            static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
            let now = format!(
                "has_player={} playing={} stop_requested={} stop_pending={} status={:?} video={}",
                presentation.has_player,
                presentation.playing,
                stop_requested,
                self.stop_requested,
                self.playback.as_ref().map(|p| p.status),
                draw.video.is_some()
            );
            if let Ok(mut last) = LAST.lock()
                && *last != now
            {
                eprintln!("movie trace: {now}");
                *last = now;
            }
        }
        let shown = draw.background || draw.video.is_some();
        (presentation, shown.then_some(draw))
    }

    fn render_movie(
        &mut self,
        mounts: &MountSet,
        audio: Option<&GameAudio>,
        renderer: &GpuRenderer,
        draw: &mut MovieDraw,
        stop_requested: &mut bool,
    ) {
        if self.stop_requested {
            draw.background = true;
            self.stop_count += 1;
            if self.stop_count <= STOP_RENDERS {
                return;
            }
            self.stop_count = 0;
            self.stop_requested = false;
            self.do_stop(audio);
            return;
        }
        while let Some(request) = self.requests.pop_front() {
            match request {
                MovieRequest::Play(settings) => self.do_open(&settings, mounts, audio),
                MovieRequest::Stop => {
                    self.request_stop();
                    *stop_requested = true;
                }
                MovieRequest::PauseAudio | MovieRequest::ResumeAudio => {}
            }
        }
        let Some(playback) = self.playback.as_mut() else {
            self.playing = false;
            return;
        };
        self.playing = playback.status == Status::Playing;
        if self.playing {
            draw.background = true;
            playback.update(audio);
            playback.render(audio, renderer, draw);
            if self.set_volume {
                if let Some(audio) = audio {
                    audio.movie_set_volume(self.volume);
                }
                self.set_volume = false;
            }
            if playback.status != Status::Playing {
                self.request_stop();
                *stop_requested = true;
            }
        } else if playback.status == Status::Interrupted {
            self.request_stop();
            *stop_requested = true;
        }
    }

    /// `RequestStop` (0x8260a458); the front end applies `mbForceBlackOnExit`.
    fn request_stop(&mut self) {
        self.stop_count = 0;
        self.stop_requested = true;
    }

    /// `DoOpen` (0x8261af18) and, once the file is open, `DoPlay` (0x8260a248): any movie stops,
    /// then the new one starts. A file that does not open leaves no player.
    fn do_open(&mut self, settings: &MovieSettings, mounts: &MountSet, audio: Option<&GameAudio>) {
        self.do_stop(audio);
        let (name, source) = match open_movie(settings, mounts) {
            Ok(opened) => opened,
            Err(error) => {
                eprintln!("movie: {} did not open: {error}", settings.file);
                return;
            }
        };
        match Playback::start(&name, source, settings, audio) {
            Ok(playback) => {
                self.playback = Some(playback);
                self.playing = true;
                self.set_volume = true;
                self.volume = audio.map_or(0.0, |audio| audio.movie_volume() * settings.volume);
            }
            Err(error) => eprintln!("movie: {name} did not open: {error}"),
        }
    }

    /// `DoStop` (0x8260a4d0): a playing movie is interrupted, then the player goes with its sound.
    fn do_stop(&mut self, audio: Option<&GameAudio>) {
        if let Some(mut playback) = self.playback.take() {
            if playback.status == Status::Playing {
                playback.status = Status::Interrupted;
            }
            if playback.audio
                && let Some(audio) = audio
            {
                audio.movie_shutdown();
            }
        }
        self.playing = false;
    }
}

/// The movie's file: the game's own on disk (`data\movies\`), else the one an archive holds, as
/// downloadable content's are (`data\movies_dlc\`, read through the `dlcbig:` device).
fn open_movie(settings: &MovieSettings, mounts: &MountSet) -> Result<(String, MovieSource), String> {
    if let Some(root) = game_root(mounts) {
        let path = skate2_formats::host_path::on_disk(&root.join(settings.file.replace('\\', "/")));
        if path.is_file() {
            let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            return Ok((path.display().to_string(), BufReader::new(Box::new(file))));
        }
    }
    let stream = mounts.open_stream(&settings.file).map_err(|e| e.to_string())?;
    Ok((settings.file.clone(), BufReader::new(Box::new(stream))))
}

impl Playback {
    /// `MoviePlayer::Init` and `Play` with `PreparePlayer`: the header and the sound's header
    /// read, the sound's renderer made, then the reader and decoder threads.
    fn start(
        name: &str,
        mut reader: MovieSource,
        settings: &MovieSettings,
        audio: Option<&GameAudio>,
    ) -> Result<Self, String> {
        let mut header = None;
        let mut sound_header = None;
        let mut early = Vec::new();
        while header.is_none() || sound_header.is_none() {
            let Some(chunk) = read_chunk(&mut reader).map_err(|e| e.to_string())? else {
                break;
            };
            match chunk.kind(settings.audio_filter) {
                ChunkKind::Header => {
                    let movie = EaMovie::from_chunk(&skate2_formats::ea_movie::Chunk {
                        tag: chunk.tag,
                        payload: &chunk.payload,
                        offset: 0,
                    })
                    .map_err(|e| e.to_string())?;
                    header = Some(movie);
                }
                ChunkKind::AudioHeader => sound_header = Some(AudioHeader::parse(&chunk.payload)),
                ChunkKind::Video => {
                    early.push(chunk);
                    if early.len() > VIDEO_RENDERABLES {
                        break;
                    }
                }
                ChunkKind::Audio | ChunkKind::Skipped => early.push(chunk),
            }
        }
        let header = header.ok_or("no MVhd chunk")?;
        let sound = MovieSound::default();
        let audio_started = sound_header.is_some_and(|h| {
            let channels = u8::try_from(h.channels).unwrap_or(0);
            audio.is_some_and(|audio| audio.movie_init(h.sample_rate, channels, sound.clone()))
        });
        if std::env::var_os("SKATE2_TRACE_MOVIE").is_some() {
            eprintln!("movie trace: start {name} sound_header={} audio_started={audio_started}", sound_header.is_some());
        }
        let (video_tx, video_rx) = sync_channel::<OwnedChunk>(VIDEO_CHUNKS_AHEAD);
        let (frame_tx, frame_rx) = sync_channel::<DecodedFrame>(1);
        let read_all = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        {
            let read_all = Arc::clone(&read_all);
            let stop = Arc::clone(&stop);
            let filter = settings.audio_filter;
            let channels = sound_header.map_or(0, |h| h.channels as usize);
            let sound = audio_started.then(|| sound.clone());
            std::thread::Builder::new()
                .name("movie reader".into())
                .spawn(move || {
                    read_movie(
                        reader, early, filter, channels, sound, &video_tx, &read_all, &stop,
                    )
                })
                .map_err(|e| e.to_string())?;
        }
        {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("movie video".into())
                .spawn(move || decode_video(&video_rx, &frame_tx, &stop))
                .map_err(|e| e.to_string())?;
        }
        // DoPlay: the game's movies are not network files, so the quad sits at
        // (640, 360 - (1 - scale) * 360) at the movie scale.
        let scale = settings.scale;
        let y = -((1.0 - scale) * 360.0 - 360.0);
        Ok(Self {
            status: Status::Playing,
            state: RwState::Prepared,
            fps: header.fps(),
            total_frames: header.frame_count,
            frames: frame_rx,
            filled: VecDeque::new(),
            uploaded: None,
            last_rendered: None,
            read_all,
            stop,
            audio: audio_started,
            stopwatch: None,
            position: [640.0, y],
            scale,
        })
    }

    /// The frames the decoder finished, up to the renderables.
    fn collect(&mut self) {
        while self.filled.len() < VIDEO_RENDERABLES - 1 {
            match self.frames.try_recv() {
                Ok(frame) => self.filled.push_back(frame),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    /// `MovieTimer::GetElapsedTime` (0x82b80a80): the sound's rendering position, or the
    /// stopwatch without a sound renderer.
    fn elapsed_ms(&self, audio: Option<&GameAudio>) -> f32 {
        let position = audio
            .filter(|_| self.audio)
            .and_then(GameAudio::movie_position)
            .filter(|&p| p >= 0.0);
        position.unwrap_or_else(|| {
            self.stopwatch
                .map_or(0.0, |start| start.elapsed().as_secs_f32() * 1000.0)
        })
    }

    /// `MoviePlayer::Update`: a finished player completes; otherwise `Decode` (0x82b85230) aims
    /// at `(delay + elapsed) * fps / 1000` and `MovieDecoder::Decode` (0x82b84380) keeps the
    /// oldest filled frame at or past it, or the newest one before it.
    fn update(&mut self, audio: Option<&GameAudio>) {
        if self.status != Status::Playing {
            return;
        }
        if self.state == RwState::Finished {
            self.status = Status::Completed;
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the native float-to-int goal frame"
        )]
        let goal = ((0.0 + self.elapsed_ms(audio)) * (self.fps * 0.001)) as i32;
        self.collect();
        while let Some(front) = self.filled.front() {
            if i64::from(front.number) >= i64::from(goal) || self.filled.len() == 1 {
                break;
            }
            self.filled.pop_front();
            self.collect();
        }
    }

    /// `MoviePlayer::Render`: the first render starts the stopwatch and the sound (`Begin`);
    /// each draws the current frame, and the last frame with the file read through finishes.
    fn render(&mut self, audio: Option<&GameAudio>, renderer: &GpuRenderer, draw: &mut MovieDraw) {
        if self.status != Status::Playing || self.state == RwState::Finished {
            return;
        }
        if self.state == RwState::Prepared {
            self.state = RwState::Rendering;
            self.stopwatch = Some(Instant::now());
            if self.audio
                && let Some(audio) = audio
            {
                audio.movie_begin();
            }
        }
        let Some(current) = self.filled.front() else {
            return;
        };
        if self.uploaded != Some(current.number) {
            renderer.upload_movie_frame(&current.frame);
            self.uploaded = Some(current.number);
        }
        self.last_rendered = Some(current.number);
        // VideoRenderer_RwTexture::Render (0x826a3e18) for a widescreen screen.
        #[expect(
            clippy::cast_precision_loss,
            reason = "the native (float) of the frame's size"
        )]
        let (width, height) = (current.frame.width as f32, current.frame.height as f32);
        let half_height = self.scale * height * 0.5;
        let half_width = self.scale * width * 0.5;
        let [x, y] = self.position;
        draw.video = Some([
            x - half_width,
            y - half_height,
            x + half_width,
            y + half_height,
        ]);
        if self.last_rendered == Some(self.total_frames.saturating_sub(1))
            && self.read_all.load(Ordering::Relaxed)
        {
            self.state = RwState::Finished;
        }
    }
}

/// The reader thread: the chunks in file order; the sound of the selected language decoded into
/// the movie's sound queue, the video chunks handed to the decoder thread.
#[allow(clippy::too_many_arguments)]
fn read_movie(
    mut reader: MovieSource,
    early: Vec<OwnedChunk>,
    filter: [u8; 2],
    channels: usize,
    sound: Option<MovieSound>,
    video: &SyncSender<OwnedChunk>,
    read_all: &AtomicBool,
    stop: &AtomicBool,
) {
    let mut decoder = EaXaDecoder::new(channels);
    let mut pcm = Vec::new();
    let mut handle = |chunk: OwnedChunk| -> bool {
        match chunk.kind(filter) {
            ChunkKind::Video => video.send(chunk).is_ok(),
            ChunkKind::Audio => {
                if let Some(sound) = sound.as_ref() {
                    pcm.clear();
                    if decoder.decode_block(&chunk.payload, &mut pcm).is_ok() {
                        sound.push(&pcm);
                    }
                }
                true
            }
            ChunkKind::Header | ChunkKind::AudioHeader | ChunkKind::Skipped => true,
        }
    };
    for chunk in early {
        if !handle(chunk) {
            return;
        }
    }
    while !stop.load(Ordering::Relaxed) {
        match read_chunk(&mut reader) {
            Ok(Some(chunk)) => {
                if !handle(chunk) {
                    return;
                }
            }
            Ok(None) => break,
            Err(error) => {
                eprintln!("movie: read failed: {error}");
                break;
            }
        }
    }
    read_all.store(true, Ordering::Relaxed);
}

/// The decoder thread: each video chunk decoded, its frame numbered in order.
fn decode_video(
    chunks: &Receiver<OwnedChunk>,
    frames: &SyncSender<DecodedFrame>,
    stop: &AtomicBool,
) {
    let mut decoder = Vp6Decoder::new();
    let mut number = 0u32;
    while let Ok(chunk) = chunks.recv() {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if let Err(error) = decoder.decode(&chunk.payload) {
            eprintln!("movie: frame {number}: {error}");
        }
        if let Some(frame) = decoder.copy_display_frame()
            && frames.send(DecodedFrame { number, frame }).is_err()
        {
            return;
        }
        number += 1;
    }
}
