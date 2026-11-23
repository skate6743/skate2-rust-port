//! PC port: the music player's Add Song (`skate2_core::ui::custom_songs`): the system's file
//! dialog for an MP3 or WAV file, on a thread of its own so that the game keeps drawing under it
//! (macOS keeps its dialogs on the main thread, which waits for the picker there). The same thread
//! then reads the file as the music system will (`skate2_audio::custom_song::probe`), which tells
//! a song from another file and gives the names the song's prompts suggest.
use skate2_core::ui::custom_songs::PickedSong;
use std::sync::mpsc;

/// What a closed picker gave: no file, a song, or why the file is no song.
pub(crate) type Picked = Option<Result<PickedSong, String>>;

#[derive(Default)]
pub(crate) struct SongPicker {
    picker: Option<mpsc::Receiver<Picked>>,
}

impl SongPicker {
    /// The picker opens, unless one is open.
    pub(crate) fn open(&mut self) {
        if self.picker.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let work = move || {
            let file = rfd::FileDialog::new().set_title("Add a song to the music player").add_filter("Songs (MP3, WAV)", &["mp3", "wav", "MP3", "WAV"]).pick_file();
            let picked = file.map(|path| skate2_audio::custom_song::probe(&path).map(|tags| PickedSong { path, artist: tags.artist, title: tags.title }));
            let _ = sender.send(picked);
        };
        if cfg!(target_os = "macos") {
            work();
        } else if let Err(error) = std::thread::Builder::new().name("song picker".into()).spawn(work) {
            eprintln!("song picker could not start: {error}");
            let (sender, failed) = mpsc::channel();
            let _ = sender.send(None);
            self.picker = Some(failed);
            return;
        }
        self.picker = Some(receiver);
    }

    /// What the picker gave, once it closed and its file was read.
    pub(crate) fn poll(&mut self) -> Option<Picked> {
        let picked = match self.picker.as_ref()?.try_recv() {
            Ok(picked) => picked,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => None,
        };
        self.picker = None;
        Some(picked)
    }
}
