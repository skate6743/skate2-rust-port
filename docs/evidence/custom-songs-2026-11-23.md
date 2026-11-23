# Music player custom songs (PC port addition)

Status on 2026-11-23: compiles (`cargo check` of the audio, core and app crates). The audio
crate's unit tests pass and the MP3 reader is measured against a reference decoder (below). The
menu, the prompts and playback in the game have not been run yet.

The music player takes MP3 and WAV files as songs. Nothing here comes from the retail game: its
music player plays the 52 songs of its own stream only.

## What the player does

| Screen | Button | Result |
| --- | --- | --- |
| Music Player | X, "Add Song" (left of Edit Playlist and Back) | The system's file dialog for an MP3 or WAV file, then two prompts: the artist or band, and the song's title. Each is filled in from the file's ID3v2 tag, else from an `Artist - Title` file name. Enter or A confirms, Escape or B leaves without adding. |
| Edit Playlist, a custom song highlighted | Y, "Remove Song" (fourth item) | The song leaves the music player and `custom_songs.json`. The sound file is not touched. |
| Edit Playlist | X, as for the game's songs | The song joins or leaves the current playlist, which is written to `custom_songs.json` at once. |

Custom songs come first in the song list, in the order they were added, before the game's
songs; a new one joins the playlist that is current, in that order too (anywhere in a shuffled
one). A playlist holds 100 songs, as the profile's does: it keeps every song of the game's it
has, and custom songs take the room left.

## `custom_songs.json`

Beside `config.json` (the working directory, or `SKATE2_SAVE_ROOT`).

```json
{
  "songs": [
    { "id": 3424267068, "path": "D:\\Music\\song.mp3", "artist": "Band", "title": "Song",
      "playlists": [true, false, false, false, false] }
  ],
  "custom_only_playlists": [false, false, false, false, false]
}
```

- `id`: `0xCC000000` with the low 24 bits of the path's FNV-1a hash (the next free one on a
  clash). None of the game's song ids has the top byte `0xCC`.
- A song whose file is missing when the game starts is not shown; its entry and playlists are
  kept for when the file is back.
- `custom_only_playlists`: the playlists that hold custom songs and none of the game's.

## SKATER.P

`ProfileBackEnd::tMusicPlayer` never holds a custom song id. Every save of it goes through
`custom_songs::game_songs_only`: custom songs are taken out of each playlist, a playlist they
leave empty is saved as every song of the game's, and the song index follows the same song (the
first when a custom song is playing). A profile without custom songs is saved byte for byte as
before. Loading merges the file's songs back into the playlists in memory (`custom_songs::merge`).
Deleting `custom_songs.json`, or any sound file, leaves a profile the game reads as it always did.

## Playback

- `skate2_audio::custom_song::CustomSongPlayer`: a decode thread fills a queue one second ahead;
  the voice "Custom Song" (`PacketPlayer`, `Rechannel`, `Resample`, `GainFader`, `Send`, as a
  movie's sound) plays it at the file's rate and channels.
- The `Send` goes into the music system's destination sub mix, and the `GainFader` takes
  `MusicSystem::UpdateVolume`'s value each update, so the Music Volume setting, the mixer's
  ducking and `game_music_disabled` act on it as on the game's songs.
- `UpdateIPodState` starts it in place of the stream when the iPod state's song is a custom id,
  and picks the next song when it ends, as for a stream song. It stops when the music mode
  leaves the iPod state and pauses where the stream pauses.
- WAV: integer PCM of 8, 16, 24 or 32 bits and 32-bit float, the first two channels.
- MP3: `rwac::ealayer3::mpeg::Mp3Stream` reads standard MPEG-1, MPEG-2 and MPEG-2.5 layer III
  frames, keeps the bit reservoir, and hands each granule to the game's own layer III decoder
  (`Layer3Core`) repacked as EA's layer III streams hold them. ID3v2 tags and the encoder's
  Xing, Info or VBRI frame are passed over. Free-format streams are not read.
- `Layer3Core::standard_stream` (false for the game's streams) changes three things for a
  standard stream: a granule's stuffing bits are passed in 16-bit steps, the bit register is
  cleared after a rewind to a byte boundary, and a quantized value of 32 or more takes
  `x^(4/3)` itself. The native `SToPowerOf4over3` 0x82c09bf0 is linear between powers of two,
  which EA's streams are made for; on a standard stream it left about 1% error (41 dB) wherever
  large values are common (joint stereo, 320 kbit/s).

## Measured

Thirteen files of a six-second test signal (sine, beeps and pink noise), decoded by `Mp3Stream`
and by FFmpeg, compared sample by sample after aligning the encoder delay:

| Encoder | Streams | Result |
| --- | --- | --- |
| LAME | MPEG-1 44.1 and 48 kHz: 128 and 320 kbit/s, VBR, joint and plain stereo, mono, with and without the bit reservoir | 93.0 to 94.5 dB, largest difference 1 |
| LAME | MPEG-2 22.05 kHz: mono, joint and plain stereo | 91.8 to 93.7 dB, largest difference 1 |
| LAME | MPEG-2.5 11.025 kHz: mono and stereo | 88.9 to 90.7 dB, largest difference 1 |
| shine | MPEG-1 44.1 kHz stereo | 94.8 dB, largest difference 1 |

The decoded sound keeps the encoder's delay (1105 samples of silence at the start for LAME),
which a gapless player would drop. The ignored test `custom_song::tests::a_named_file_decodes_to_pcm`
writes a named file's samples for such a comparison.

## To verify

1. The core crate's new unit tests (`ui::custom_songs`, `ui::music_player`).
2. In game: add, play through to the next song, toggle, remove while playing, Music Volume,
   pause, and a save and reload of SKATER.P with and without `custom_songs.json`.
3. Linux: the file dialog through the XDG portal.
