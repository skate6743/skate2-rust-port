//! PC port addition: the music player's custom songs, MP3 and WAV files the player adds to its
//! song list (`custom_songs.json`). A song decodes on a thread of its own into a queue, and plays
//! as the voice "Custom Song" (as a movie's sound does, `movie::MovieAudio`): `PacketPlayer`,
//! `Rechannel`, `Resample`, `GainFader` and `Send`, at the file's channels and rate, the `Send`
//! into the sub mix the music player's own songs play into and the `GainFader` at the music
//! system's volume for them.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::rwac::core::{AudioCore, PlugInConfig, PlugInKind};
use crate::rwac::ealayer3::mpeg::Mp3Stream;
use crate::rwac::plugins::packet_player::Packet;
use crate::rwac::{PluginRef, VoiceId};

/// Frames a WAV file gives at a time.
const WAV_BLOCK: usize = 1152;
/// Frames in a packet handed to the voice, and the frames kept handed ahead of it.
const PACKET_FRAMES: usize = 512;
const QUEUED_FRAMES: usize = 1536;
/// The seconds a volume change and a pause fade over.
const FADE: f32 = 0.05;

/// A WAV file's `data` chunk as 16-bit samples.
struct WavStream {
    data: Arc<[u8]>,
    /// The next frame's byte and the chunk's end.
    position: usize,
    end: usize,
    /// 1 integer PCM, 3 IEEE float.
    format: u16,
    bytes_per_sample: usize,
    file_channels: usize,
    channels: u8,
    sample_rate: u32,
}

impl WavStream {
    /// A RIFF WAVE file of integer PCM (8, 16, 24 or 32 bits) or 32-bit float samples; its first
    /// two channels are read. `None` for another format.
    fn open(data: Arc<[u8]>) -> Option<Self> {
        if data.get(..4) != Some(&b"RIFF"[..]) || data.get(8..12) != Some(&b"WAVE"[..]) {
            return None;
        }
        let u16_at = |at: usize| data.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let u32_at = |at: usize| data.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let mut format = None;
        let mut sound = None;
        let mut at = 12;
        while let (Some(id), Some(size)) = (data.get(at..at + 4), u32_at(at + 4)) {
            let body = at + 8;
            let size = size as usize;
            match id {
                b"fmt " => {
                    let mut tag = u16_at(body)?;
                    // WAVE_FORMAT_EXTENSIBLE: the sub format's first word is the tag.
                    if tag == 0xFFFE {
                        tag = u16_at(body + 24)?;
                    }
                    format = Some((tag, u16_at(body + 2)?, u32_at(body + 4)?, u16_at(body + 14)?));
                }
                b"data" => sound = Some((body, body.saturating_add(size).min(data.len()))),
                _ => {}
            }
            if format.is_some() && sound.is_some() {
                break;
            }
            at = body.checked_add(size + (size & 1))?;
        }
        let (format, file_channels, sample_rate, bits) = format?;
        let (position, end) = sound?;
        let supported = match format {
            1 => matches!(bits, 8 | 16 | 24 | 32),
            3 => bits == 32,
            _ => false,
        };
        if !supported || file_channels == 0 || !(8000..=192_000).contains(&sample_rate) {
            return None;
        }
        Some(Self {
            data,
            position,
            end,
            format,
            bytes_per_sample: usize::from(bits / 8),
            file_channels: usize::from(file_channels),
            channels: file_channels.min(2) as u8,
            sample_rate,
        })
    }

    /// The sample at `at` as 16 bits.
    fn sample(&self, at: usize) -> i16 {
        let b = |index: usize| self.data.get(at + index).copied().unwrap_or(0);
        match (self.format, self.bytes_per_sample) {
            (1, 1) => (i16::from(b(0)) - 128) << 8,
            (1, 2) => i16::from_le_bytes([b(0), b(1)]),
            (1, 3) => i16::from_le_bytes([b(1), b(2)]),
            (1, _) => i16::from_le_bytes([b(2), b(3)]),
            _ => (f32::from_le_bytes([b(0), b(1), b(2), b(3)]) * 32767.0).round().clamp(-32768.0, 32767.0) as i16,
        }
    }

    /// Up to [`WAV_BLOCK`] frames onto `out`. False at the end of the chunk.
    fn read(&mut self, out: &mut Vec<i16>) -> bool {
        let frame = self.bytes_per_sample * self.file_channels;
        let frames = ((self.end.saturating_sub(self.position)) / frame).min(WAV_BLOCK);
        if frames == 0 {
            return false;
        }
        for _ in 0..frames {
            for channel in 0..usize::from(self.channels) {
                out.push(self.sample(self.position + channel * self.bytes_per_sample));
            }
            self.position += frame;
        }
        true
    }
}

enum Source {
    Wav(WavStream),
    Mp3(Box<Mp3Stream>),
}

/// A song file's sound: interleaved 16-bit samples of one or two channels.
pub struct SongStream {
    source: Source,
    pub sample_rate: u32,
    pub channels: u8,
}

impl SongStream {
    /// The sound of a WAV or MP3 file's bytes; `None` when they are neither.
    #[must_use]
    pub fn open(data: Arc<[u8]>) -> Option<Self> {
        if let Some(wav) = WavStream::open(Arc::clone(&data)) {
            return Some(Self { sample_rate: wav.sample_rate, channels: wav.channels, source: Source::Wav(wav) });
        }
        if data.starts_with(b"RIFF") {
            return None;
        }
        let mp3 = Mp3Stream::open(data)?;
        Some(Self { sample_rate: mp3.sample_rate, channels: mp3.channels, source: Source::Mp3(Box::new(mp3)) })
    }

    /// The next samples onto `out`. False at the end of the sound.
    pub fn read(&mut self, out: &mut Vec<i16>) -> bool {
        match &mut self.source {
            Source::Wav(wav) => wav.read(out),
            Source::Mp3(mp3) => mp3.decode_frame(out),
        }
    }
}

/// What a song file says of itself, for the prompts that name it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SongTags {
    pub artist: String,
    pub title: String,
}

/// Text of an ID3v2 text frame: its encoding byte, then the text up to a terminator.
fn id3_text(body: &[u8]) -> String {
    let Some((&encoding, text)) = body.split_first() else { return String::new() };
    let wide = |text: &[u8], big: bool| {
        let units: Vec<u16> =
            text.chunks_exact(2).map(|pair| if big { u16::from_be_bytes([pair[0], pair[1]]) } else { u16::from_le_bytes([pair[0], pair[1]]) }).collect();
        String::from_utf16_lossy(&units)
    };
    let text = match encoding {
        0 => text.iter().map(|&byte| char::from(byte)).collect(),
        1 => match text.get(..2) {
            Some([0xFE, 0xFF]) => wide(&text[2..], true),
            Some([0xFF, 0xFE]) => wide(&text[2..], false),
            _ => wide(text, false),
        },
        2 => wide(text, true),
        _ => String::from_utf8_lossy(text).into_owned(),
    };
    text.split('\0').next().unwrap_or("").trim().to_owned()
}

/// The artist and title of an ID3v2 tag (versions 2 to 4) at the start of `data`.
fn id3_tags(data: &[u8]) -> SongTags {
    let mut tags = SongTags::default();
    let Some(head) = data.get(..10).filter(|head| head.starts_with(b"ID3")) else { return tags };
    let version = head[3];
    let size = head[6..10].iter().fold(0usize, |size, &byte| (size << 7) | usize::from(byte & 0x7F));
    let Some(body) = data.get(10..(10 + size).min(data.len())) else { return tags };
    // Version 2 has three-letter frames of six header bytes; later ones four letters and ten.
    let (id_bytes, header) = if version == 2 { (3, 6) } else { (4, 10) };
    let mut at = 0;
    while let Some(frame) = body.get(at..at + header) {
        if frame[0] == 0 {
            break;
        }
        let length = match version {
            2 => frame[3..6].iter().fold(0usize, |size, &byte| (size << 8) | usize::from(byte)),
            4 => frame[4..8].iter().fold(0usize, |size, &byte| (size << 7) | usize::from(byte & 0x7F)),
            _ => frame[4..8].iter().fold(0usize, |size, &byte| (size << 8) | usize::from(byte)),
        };
        let Some(text) = body.get(at + header..at + header + length) else { break };
        match &frame[..id_bytes] {
            b"TPE1" | b"TP1" => tags.artist = id3_text(text),
            b"TIT2" | b"TT2" => tags.title = id3_text(text),
            _ => {}
        }
        at += header + length;
    }
    tags
}

/// A file name's artist and title: `Artist - Title`, else the whole name as the title.
fn name_tags(path: &Path) -> SongTags {
    let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    match stem.split_once(" - ") {
        Some((artist, title)) => SongTags { artist: artist.trim().to_owned(), title: title.trim().to_owned() },
        None => SongTags { artist: String::new(), title: stem.trim().to_owned() },
    }
}

/// Whether the file at `path` is a song the music player can play, and the names it suggests
/// (its ID3 tag's, else its file name's).
///
/// # Errors
/// The file cannot be read, or is neither a WAV nor an MP3 file the player reads.
pub fn probe(path: &Path) -> Result<SongTags, String> {
    let data: Arc<[u8]> = std::fs::read(path).map_err(|error| format!("The file could not be read: {error}"))?.into();
    let mut stream = SongStream::open(Arc::clone(&data)).ok_or("That file is not an MP3 or WAV file the music player can play.")?;
    let mut samples = Vec::new();
    if !stream.read(&mut samples) {
        return Err("That file holds no sound.".to_owned());
    }
    let from_name = name_tags(path);
    let from_tag = id3_tags(&data);
    Ok(SongTags {
        artist: if from_tag.artist.is_empty() { from_name.artist } else { from_tag.artist },
        title: if from_tag.title.is_empty() { from_name.title } else { from_tag.title },
    })
}

/// What a song's decode thread and its player share.
#[derive(Default)]
struct Feed {
    samples: Mutex<VecDeque<i16>>,
    /// The file's sample rate and channels, once it is open.
    format: Mutex<Option<(u32, u8)>>,
    /// The thread ended: at the end of the sound, or without any when `failed`.
    done: AtomicBool,
    failed: AtomicBool,
    /// The player let the song go.
    stop: AtomicBool,
}

impl Feed {
    fn samples(&self) -> std::sync::MutexGuard<'_, VecDeque<i16>> {
        self.samples.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn format(&self) -> Option<(u32, u8)> {
        *self.format.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The decode thread: the file read whole, then its sound kept a second ahead of the player.
fn decode(path: &Path, feed: &Feed) {
    /// Marks the feed done however the thread ends.
    struct Done<'a>(&'a Feed);
    impl Drop for Done<'_> {
        fn drop(&mut self) {
            self.0.done.store(true, Ordering::Release);
        }
    }
    let _done = Done(feed);
    let Some(mut stream) = std::fs::read(path).ok().and_then(|data| SongStream::open(data.into())) else {
        feed.failed.store(true, Ordering::Release);
        return;
    };
    *feed.format.lock().unwrap_or_else(PoisonError::into_inner) = Some((stream.sample_rate, stream.channels));
    let ahead = stream.sample_rate as usize * usize::from(stream.channels);
    let mut block = Vec::new();
    while !feed.stop.load(Ordering::Acquire) {
        if feed.samples().len() >= ahead {
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }
        block.clear();
        if !stream.read(&mut block) {
            return;
        }
        feed.samples().extend(block.iter().copied());
    }
}

struct SongVoice {
    voice: VoiceId,
    packet_player: PluginRef,
    gain_fader: PluginRef,
    channels: u8,
}

/// One custom song playing.
pub struct CustomSongPlayer {
    feed: Arc<Feed>,
    voice: Option<SongVoice>,
    /// The sub mix the song plays into.
    target: PluginRef,
    /// The packets handed to the voice and not yet played: each one's id and frames.
    handed: VecDeque<(u32, usize)>,
    next_id: u32,
    volume: f32,
    paused: bool,
}

impl CustomSongPlayer {
    /// Starts the file at `path` decoding; its voice follows once the file is open.
    #[must_use]
    pub fn start(path: PathBuf, target: PluginRef, volume: f32) -> Self {
        let feed = Arc::new(Feed::default());
        let thread_feed = Arc::clone(&feed);
        let spawned = std::thread::Builder::new().name("custom song".into()).spawn(move || decode(&path, &thread_feed));
        if spawned.is_err() {
            feed.failed.store(true, Ordering::Release);
            feed.done.store(true, Ordering::Release);
        }
        Self { feed, voice: None, target, handed: VecDeque::new(), next_id: 1, volume, paused: false }
    }

    /// The song has sound left to play, or is still opening its file.
    #[must_use]
    pub fn playing(&self) -> bool {
        !(self.feed.done.load(Ordering::Acquire) && self.handed.is_empty() && self.feed.samples().is_empty())
    }

    /// The file could not be played.
    #[must_use]
    pub fn failed(&self) -> bool {
        self.feed.failed.load(Ordering::Acquire)
    }

    /// The music system's volume for the song.
    pub fn set_volume(&mut self, core: &mut AudioCore, volume: f32) {
        if (volume - self.volume).abs() < 0.0005 {
            return;
        }
        self.volume = volume;
        if let Some(voice) = self.voice.as_ref().filter(|_| !self.paused) {
            core.start_fade(voice.gain_fader, 0.0, FADE, volume, 0.0);
        }
    }

    /// A paused song fades out and keeps its place: no more of it is handed to the voice.
    pub fn set_paused(&mut self, core: &mut AudioCore, paused: bool) {
        if self.paused == paused {
            return;
        }
        self.paused = paused;
        if let Some(voice) = &self.voice {
            core.start_fade(voice.gain_fader, 0.0, FADE, if paused { 0.0 } else { self.volume }, 0.0);
        }
    }

    /// The voice, once the decode thread knows the file's rate and channels.
    fn create_voice(&mut self, core: &mut AudioCore) {
        let Some((sample_rate, channels)) = self.feed.format() else { return };
        let config = |kind| PlugInConfig { kind, output_channels: channels };
        let Some(voice) = core.create_voice(
            0,
            "Custom Song",
            &[
                config(PlugInKind::PacketPlayer),
                config(PlugInKind::Rechannel),
                config(PlugInKind::Resample),
                config(PlugInKind::GainFader),
                config(PlugInKind::Send),
            ],
        ) else {
            return;
        };
        let song = SongVoice { voice, packet_player: PluginRef { voice, index: 0 }, gain_fader: PluginRef { voice, index: 3 }, channels };
        core.start_fade(song.gain_fader, 0.0, 0.0, if self.paused { 0.0 } else { self.volume }, 0.0);
        core.connect_send(PluginRef { voice, index: 4 }, Some(self.target));
        core.packet_play(song.packet_player, sample_rate as f32, f32::from(channels));
        self.voice = Some(song);
    }

    /// From the core's timer before its commands: the packets the voice finished are dropped, and
    /// decoded sound is handed to it up to [`QUEUED_FRAMES`] ahead.
    pub fn timer(&mut self, core: &mut AudioCore) {
        if self.voice.is_none() {
            self.create_voice(core);
        }
        let Some(voice) = &self.voice else { return };
        while let Some(&(id, _)) = self.handed.front() {
            if core.packet_in_use(voice.packet_player, id) {
                break;
            }
            self.handed.pop_front();
        }
        if self.paused {
            return;
        }
        let channels = usize::from(voice.channels).max(1);
        let mut queued: usize = self.handed.iter().map(|&(_, frames)| frames).sum();
        while queued < QUEUED_FRAMES {
            let data: Vec<i16> = {
                let mut samples = self.feed.samples();
                let count = (samples.len() / channels).min(PACKET_FRAMES) * channels;
                samples.drain(..count).collect()
            };
            if data.is_empty() {
                // Less than a frame left at the end of the sound is not played.
                if self.feed.done.load(Ordering::Acquire) {
                    self.feed.samples().clear();
                }
                break;
            }
            let frames = data.len() / channels;
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            core.packet_submit(voice.packet_player, Packet { id, num_samples: frames as i32, data: data.into() });
            self.handed.push_back((id, frames));
            queued += frames;
        }
    }

    /// The song let go: its thread ends and its voice is released.
    pub fn stop(self, core: &mut AudioCore) {
        self.feed.stop.store(true, Ordering::Release);
        if let Some(voice) = self.voice {
            core.packet_stop(voice.packet_player);
            core.release_voice(voice.voice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(format: u16, bits: u16, channels: u16, samples: &[u8]) -> Arc<[u8]> {
        let mut data = b"RIFF\0\0\0\0WAVEjunk\x03\0\0\0abc\0fmt \x10\0\0\0".to_vec();
        data.extend(format.to_le_bytes());
        data.extend(channels.to_le_bytes());
        data.extend(44100u32.to_le_bytes());
        data.extend([0; 6]);
        data.extend(bits.to_le_bytes());
        data.extend(b"data");
        data.extend((samples.len() as u32).to_le_bytes());
        data.extend(samples);
        data.into()
    }

    #[test]
    fn wav_samples_become_sixteen_bits() {
        let read = |data: Arc<[u8]>| {
            let mut stream = SongStream::open(data).unwrap();
            let mut out = Vec::new();
            assert!(stream.read(&mut out));
            assert!(!stream.read(&mut out));
            (stream.sample_rate, stream.channels, out)
        };
        assert_eq!(read(wav(1, 16, 2, &[1, 0, 2, 0, 0xFF, 0xFF, 0, 0x80])), (44100, 2, vec![1, 2, -1, -32768]));
        assert_eq!(read(wav(1, 8, 1, &[128, 255, 0])), (44100, 1, vec![0, 127 << 8, -128 << 8]));
        assert_eq!(read(wav(1, 24, 1, &[9, 0x34, 0x12])), (44100, 1, vec![0x1234]));
        let float: Vec<u8> = [0.5f32, -1.0].iter().flat_map(|value| value.to_le_bytes()).collect();
        assert_eq!(read(wav(3, 32, 1, &float)), (44100, 1, vec![16384, -32767]));
        // Six channels give their first two; a compressed format is not read.
        assert_eq!(read(wav(1, 8, 6, &[128, 129, 0, 0, 0, 0])), (44100, 2, vec![0, 256]));
        assert!(SongStream::open(wav(0x55, 16, 2, &[0; 8])).is_none());
    }

    #[test]
    fn names_come_from_the_tag_then_the_file_name() {
        let mut tag = b"ID3\x03\0\0\0\0\0\x27".to_vec();
        tag.extend(b"TIT2\0\0\0\x06\0\0\0Title");
        tag.extend(b"TPE1\0\0\0\x0b\0\0\x01\xFF\xFEB\0a\0n\0d\0\0\0");
        assert_eq!(id3_tags(&tag), SongTags { artist: "Band".into(), title: "Title".into() });
        assert_eq!(id3_tags(b"not a tag"), SongTags::default());
        assert_eq!(name_tags(Path::new("music/The Band - A Song.mp3")), SongTags { artist: "The Band".into(), title: "A Song".into() });
        assert_eq!(name_tags(Path::new("track01.wav")), SongTags { artist: String::new(), title: "track01".into() });
    }

    /// A developer's check of a real file against another decoder: `SKATE2_CUSTOM_SONG` names the
    /// file and `SKATE2_CUSTOM_SONG_PCM` the file its 16-bit little-endian samples are written to.
    #[test]
    #[ignore = "needs SKATE2_CUSTOM_SONG and SKATE2_CUSTOM_SONG_PCM"]
    fn a_named_file_decodes_to_pcm() {
        let (Some(song), Some(pcm)) = (std::env::var_os("SKATE2_CUSTOM_SONG"), std::env::var_os("SKATE2_CUSTOM_SONG_PCM")) else { return };
        let tags = probe(Path::new(&song)).unwrap();
        let mut stream = SongStream::open(std::fs::read(&song).unwrap().into()).unwrap();
        let mut samples = Vec::new();
        while stream.read(&mut samples) {}
        eprintln!("{tags:?}: {} Hz, {} channels, {} samples", stream.sample_rate, stream.channels, samples.len());
        let bytes: Vec<u8> = samples.iter().flat_map(|sample| sample.to_le_bytes()).collect();
        std::fs::write(pcm, bytes).unwrap();
    }
}
