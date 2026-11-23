//! PC port addition: the music player's custom songs. `custom_songs.json` beside `config.json`
//! (the working directory the game runs from, or `SKATE2_SAVE_ROOT`) lists each MP3 or WAV file
//! the player added with Add Song: its path, the artist and title typed for it, and the playlists
//! that hold it. A song whose file is missing when the game starts is left out of the music
//! player until it is back.
//!
//! The profile's music player (`ProfileBackEnd::tMusicPlayer` in SKATER.P) never holds a custom
//! song: while the game runs its playlists do (`merge`), and what is saved is the game's own
//! songs alone (`game_songs_only`), a playlist of custom songs alone as every song of the game's.
//! The file keeps which playlists those are, so removing a song or the file leaves a profile the
//! game reads as it always did.
use serde::{Deserialize, Serialize};
use skate2_formats::skater_profile::{ProfileMusicPlayer, MUSIC_PLAYER_PLAYLISTS, MUSIC_PLAYER_SONGS};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
};

/// A custom song's id has this top byte, which none of the game's songs has.
const CUSTOM_ID: u32 = 0xCC00_0000;
/// The longest artist or title Add Song's prompts take.
pub const NAME_LIMIT: usize = 40;

/// Whether `song` is a custom song's id.
#[must_use]
pub const fn is_custom(song: u32) -> bool {
    song >> 24 == CUSTOM_ID >> 24
}

/// The file Add Song's picker gave, with the names it suggests for the song's prompts (its tag's
/// or its file name's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickedSong {
    pub path: PathBuf,
    pub artist: String,
    pub title: String,
}

/// What Add Song shows over the music player: a prompt's question and the line typed (with the
/// cursor's place), or why a file was not added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongPrompt {
    pub message: String,
    pub field: Option<(String, usize)>,
    pub error: Option<String>,
}

/// One custom song of `custom_songs.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
    pub id: u32,
    pub path: String,
    pub artist: String,
    pub title: String,
    /// The playlists that hold it, Playlist 01 first.
    #[serde(default)]
    pub playlists: [bool; MUSIC_PLAYER_PLAYLISTS],
}

/// `custom_songs.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SongFile {
    #[serde(default)]
    songs: Vec<Song>,
    /// The playlists that hold custom songs alone, which the profile saves with every song of
    /// the game's.
    #[serde(default)]
    custom_only_playlists: [bool; MUSIC_PLAYER_PLAYLISTS],
}

struct State {
    file: SongFile,
    /// The songs whose files were there when the list was read, and those added since.
    available: Vec<u32>,
    /// Counts each change of the songs the music player holds.
    generation: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// `SKATE2_SAVE_ROOT` relocates it with the saves; otherwise it sits in the working directory the
/// game runs from.
///
/// # Errors
/// The working directory is unknown.
pub fn path() -> io::Result<PathBuf> {
    Ok(match std::env::var_os("SKATE2_SAVE_ROOT") {
        Some(root) => PathBuf::from(root),
        None => std::env::current_dir()?,
    }
    .join("custom_songs.json"))
}

/// The file's songs; none without a file, or from one that does not read.
fn read() -> SongFile {
    let mut file: SongFile = path().and_then(fs::read_to_string).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    // An entry the file's reader would not tell from the game's songs, or from another entry.
    let mut seen = Vec::new();
    file.songs.retain(|song| {
        let kept = is_custom(song.id) && !seen.contains(&song.id);
        seen.push(song.id);
        kept
    });
    file
}

fn write(file: &SongFile) -> io::Result<()> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(file).map_err(io::Error::other)?;
    fs::write(path, text + "\n")
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut state = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let state = state.get_or_insert_with(|| {
        let file = read();
        let available = file.songs.iter().filter(|song| Path::new(&song.path).is_file()).map(|song| song.id).collect();
        State { file, available, generation: 1 }
    });
    f(state)
}

/// The songs the music player holds: the file's whose sound files exist, in the file's order.
#[must_use]
pub fn songs() -> Vec<Song> {
    with(|state| state.file.songs.iter().filter(|song| state.available.contains(&song.id)).cloned().collect())
}

/// Their ids, the music player's catalogue before the game's songs.
#[must_use]
pub fn ids() -> Vec<u32> {
    songs().into_iter().map(|song| song.id).collect()
}

/// Each one's id and sound file, for the music system.
#[must_use]
pub fn files() -> Vec<(u32, PathBuf)> {
    songs().into_iter().map(|song| (song.id, PathBuf::from(song.path))).collect()
}

/// A custom song's artist and title.
#[must_use]
pub fn info(song: u32) -> Option<(String, String)> {
    if !is_custom(song) {
        return None;
    }
    with(|state| state.file.songs.iter().find(|entry| entry.id == song).map(|entry| (entry.artist.clone(), entry.title.clone())))
}

/// A number that changes whenever the songs the music player holds do.
#[must_use]
pub fn generation() -> u64 {
    with(|state| state.generation)
}

/// FNV-1a of the path, as the low 24 bits of a custom song's id.
fn path_hash(path: &str) -> u32 {
    path.bytes().fold(0x811C_9DC5u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)) & 0x00FF_FFFF
}

/// Add Song: the file at `path` joins the list under `artist` and `title`, in no playlist yet.
///
/// # Errors
/// The file is in the list already, or `custom_songs.json` cannot be written.
pub fn add(path: &Path, artist: &str, title: &str) -> Result<Song, String> {
    let path = path.to_string_lossy().into_owned();
    with(|state| {
        if state.file.songs.iter().any(|song| song.path == path) {
            return Err("That song is already in the music player.".to_owned());
        }
        let mut id = CUSTOM_ID | path_hash(&path);
        while state.file.songs.iter().any(|song| song.id == id) {
            id = CUSTOM_ID | (id.wrapping_add(1) & 0x00FF_FFFF);
        }
        let song = Song { id, path, artist: artist.trim().to_owned(), title: title.trim().to_owned(), playlists: [false; MUSIC_PLAYER_PLAYLISTS] };
        state.file.songs.push(song.clone());
        if let Err(error) = write(&state.file) {
            state.file.songs.pop();
            return Err(format!("Could not save custom_songs.json: {error}"));
        }
        state.available.push(id);
        state.generation += 1;
        Ok(song)
    })
}

/// Remove Song: the entry leaves the list; its sound file stays where it is.
///
/// # Errors
/// `custom_songs.json` cannot be written.
pub fn remove(song: u32) -> io::Result<()> {
    with(|state| {
        state.file.songs.retain(|entry| entry.id != song);
        state.available.retain(|&id| id != song);
        state.generation += 1;
        write(&state.file)
    })
}

/// The songs of `playlist` the profile counts.
fn held(player: &ProfileMusicPlayer, playlist: usize) -> &[u32] {
    let list = &player.playlists[playlist];
    &list.songs[..usize::try_from(list.num_songs).unwrap_or(0).min(list.songs.len())]
}

fn set_songs(player: &mut ProfileMusicPlayer, playlist: usize, songs: &[u32]) {
    let list = &mut player.playlists[playlist];
    list.songs = [0; MUSIC_PLAYER_SONGS];
    let count = songs.len().min(MUSIC_PLAYER_SONGS);
    list.songs[..count].copy_from_slice(&songs[..count]);
    list.num_songs = count as i32;
}

/// The song `current_song_index` names in the current playlist.
fn current_song(player: &ProfileMusicPlayer) -> Option<u32> {
    let playlist = usize::try_from(player.current_playlist).ok().filter(|&playlist| playlist < MUSIC_PLAYER_PLAYLISTS)?;
    held(player, playlist).get(usize::try_from(player.current_song_index).ok()?).copied()
}

/// `current_song_index` back on `song` after the current playlist changed, else its first song.
fn follow(player: &mut ProfileMusicPlayer, song: Option<u32>) {
    let Some(song) = song else { return };
    let Some(playlist) = usize::try_from(player.current_playlist).ok().filter(|&playlist| playlist < MUSIC_PLAYER_PLAYLISTS) else { return };
    player.current_song_index = held(player, playlist).iter().position(|&held| held == song).map_or(0, |index| index as i32);
}

/// The profile's music player as the game holds it: each playlist with the custom songs the
/// file puts in it (before the game's songs, as the catalogue lists them, or among them when the
/// playlists are shuffled), a playlist of custom songs alone without the game's. A playlist
/// keeps every song of the game's it has: the custom songs take the room left.
pub fn merge(player: &mut ProfileMusicPlayer) {
    let (songs, custom_only) = with(|state| {
        let songs: Vec<Song> = state.file.songs.iter().filter(|song| state.available.contains(&song.id)).cloned().collect();
        (songs, state.file.custom_only_playlists)
    });
    let playing = current_song(player);
    for playlist in 0..MUSIC_PLAYER_PLAYLISTS {
        let mine: Vec<u32> = songs.iter().filter(|song| song.playlists[playlist]).map(|song| song.id).collect();
        if mine.is_empty() {
            continue;
        }
        let mut list: Vec<u32> = if custom_only[playlist] { Vec::new() } else { held(player, playlist).iter().copied().filter(|&song| !is_custom(song)).collect() };
        let room = MUSIC_PLAYER_SONGS.saturating_sub(list.len());
        for (index, song) in mine.into_iter().take(room).enumerate() {
            let at = if player.shuffle { (song.wrapping_mul(0x9E37_79B1) >> 8) as usize % (list.len() + 1) } else { index };
            list.insert(at, song);
        }
        set_songs(player, playlist, &list);
    }
    follow(player, playing);
}

/// What SKATER.P saves of `player`: its playlists without their custom songs, a playlist they
/// leave empty as the game's songs of `catalogue`, and the song index on the same song of the
/// game's (the first when a custom song played). A player without custom songs is saved as it is.
#[must_use]
pub fn game_songs_only(player: &ProfileMusicPlayer, catalogue: &[u32]) -> ProfileMusicPlayer {
    let mut saved = player.clone();
    let playing = current_song(player);
    let mut changed = false;
    for playlist in 0..MUSIC_PLAYER_PLAYLISTS {
        if !held(player, playlist).iter().any(|&song| is_custom(song)) {
            continue;
        }
        let mut list: Vec<u32> = held(player, playlist).iter().copied().filter(|&song| !is_custom(song)).collect();
        if list.is_empty() {
            list = catalogue.iter().copied().filter(|&song| song != 0 && !is_custom(song)).collect();
        }
        set_songs(&mut saved, playlist, &list);
        changed |= usize::try_from(player.current_playlist) == Ok(playlist);
    }
    if changed && player.current_song_index >= 0 {
        follow(&mut saved, playing.or(Some(0)));
    }
    saved
}

/// The file's playlists as `player` holds them now: which playlists hold each of the music
/// player's custom songs, and which hold custom songs alone. Songs left out for a missing file
/// keep their playlists.
///
/// # Errors
/// `custom_songs.json` cannot be written.
pub fn store_playlists(player: &ProfileMusicPlayer) -> io::Result<()> {
    with(|state| {
        let mut file = state.file.clone();
        for playlist in 0..MUSIC_PLAYER_PLAYLISTS {
            let songs = held(player, playlist);
            for song in file.songs.iter_mut().filter(|song| state.available.contains(&song.id)) {
                song.playlists[playlist] = songs.contains(&song.id);
            }
            file.custom_only_playlists[playlist] = !songs.is_empty() && songs.iter().all(|&song| is_custom(song));
        }
        if file == state.file {
            return Ok(());
        }
        write(&file)?;
        state.file = file;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: [u32; 4] = [10, 20, 30, 40];
    const MINE: u32 = CUSTOM_ID | 7;
    const OTHER: u32 = CUSTOM_ID | 9;

    fn player() -> ProfileMusicPlayer {
        ProfileMusicPlayer::reset(&GAME)
    }

    #[test]
    fn a_custom_id_is_none_of_the_games() {
        assert!(is_custom(MINE) && is_custom(0xCCFF_FFFF));
        assert!(!is_custom(0) && !is_custom(0xC742_9CCB) && !is_custom(0xCD00_0000));
        assert_eq!(path_hash("a") & 0xFF00_0000, 0);
        assert_ne!(path_hash("C:\\a.mp3"), path_hash("C:\\b.mp3"));
    }

    #[test]
    fn a_profile_without_custom_songs_saves_as_it_is() {
        let mut held = player();
        held.current_playlist = 2;
        held.current_song_index = 3;
        held.playlists[1].songs[60] = 99;
        assert_eq!(game_songs_only(&held, &GAME), held);
    }

    #[test]
    fn custom_songs_stay_out_of_the_profile() {
        let mut held = player();
        // Playlist 01: a custom song between the game's, playing; playlist 02: custom songs alone.
        set_songs(&mut held, 0, &[10, MINE, 20]);
        set_songs(&mut held, 1, &[MINE, OTHER]);
        held.current_song_index = 2;
        let saved = game_songs_only(&held, &GAME);
        assert_eq!(super::held(&saved, 0), [10, 20]);
        assert_eq!(saved.current_song_index, 1, "the index follows the same song of the game's");
        assert_eq!(super::held(&saved, 1), GAME, "a playlist of custom songs alone saves the game's songs");
        assert_eq!(super::held(&saved, 2), GAME);
        assert!(saved.playlists.iter().all(|list| list.songs.iter().all(|&song| !is_custom(song))));
        // A custom song playing: the saved index is the first song's.
        held.current_song_index = 1;
        assert_eq!(game_songs_only(&held, &GAME).current_song_index, 0);
        // The song index of a song outside the playlist (-1) is kept.
        held.current_song_index = -1;
        assert_eq!(game_songs_only(&held, &GAME).current_song_index, -1);
    }
}
