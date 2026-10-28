<div align="center">

# Skate 2 | Rust Port 🦀

**Skate 2, rebuilt as a native PC game in Rust.**

Windows · Linux · macOS · Online play

[Download](https://github.com/skate6743/skate2-rust-port/releases/latest) ·
[How to play](#how-to-play) ·
[Build from source](#building-from-source)

</div>

This isn't an emulator or a recompilation. The game's own systems (the skating physics,
animation, the streaming city, challenges and audio) are reconstructed as readable Rust from
Skate 2's Xbox 360 debug build of January 17, 2009 (`sk82_na_m.xex`), and run natively on your
PC. Requires your own legally obtained Xbox 360 copy of Skate 2 to run, you will be prompted to choose ROM location during first startup.

> [!NOTE]
> **Work in progress.** Free skate, Party Play and online play are playable; the career is
> mostly there. Expect some rough edges.

## Screenshot
<div align="center">
<img src="docs/images/online-lobby.png" alt="Skaters in an online lobby" width="720">
</div>

## Features

- **The real Skate 2 skating.** Flick-It, tricks, grabs, grinds, manuals, bails and off-board
  play, driven by the game's own physics and animation graphs.
- **San Vanelona, alive.** The whole city streams in as you skate, with pedestrians, traffic,
  AI skaters, the original audio and the Music Player.
- **Free skate** anywhere in the city, straight from the main menu.
- **Career** (in progress). New Career, missions, challenges, cutscenes and saves. It can't be
  played start to finish yet.
- **Party Play.** S.K.A.T.E., Spot Battle and Hall of Meat for two to four players.
- **Online.** Quick Match, Custom Match and online Freeskate on dedicated servers, with every
  unranked challenge: Spot Battle, Best Trick, Jam, S.K.A.T.E., Death Race and Hall of Meat.
- **Create-a-Skater** and the clothing shop.
- **Skate.Reel.** Edit replays and take photos, and save them as MP4 video or
  JPEG.

## What's new compared to the original

- **New netcode.** The original sends only controller input and simulates every skater on every
  console, so any difference between two machines desyncs the whole lobby. Here each game
  simulates its own skater and streams compact replay frames: other skaters play back smoothly
  through packet loss and jitter, and challenge rules still run the same on every machine.
- **Bigger lobbies.** Up to 8 skaters, a **Lobby Browser** that lists the games on the server,
  and **lobby chat** (press T).
- **Watch a challenge in progress.** Join a game mid-challenge, or pick **Leave Challenge** in
  the pause menu, and you wait in the pre-game lobby watching the players still in the challenge
  as ghosts. They never see you. Online Settings' **Show Ghost Players in pregame lobby** turns
  the ghosts off.
- **Display Settings.** A frame-rate cap (60, 120, your monitor's refresh rate or uncapped), VSync
  (on: no tearing; off: frames show at once, with the cap still applied), a resolution scale
  (50% to 200%: above 100% the world renders larger and is scaled down for a sharper picture,
  below it renders smaller for speed; the HUD always stays sharp), a frame-rate counter,
  borderless fullscreen (Alt+Enter works too), and switches for PostFX and the fisheye lens.
- **Better Party Play.** No pedestrians or traffic in the way and your own career skater is in the
  list to choose from.
- **Stick Settings.** Under Control Settings, each stick takes a multiplier (0.40 to 1.10), a
  deadzone and an anti-deadzone that work as RPCS3's do, with both sticks drawn live beside them.
  Out of the box, and after Reset To Default, the sticks feel exactly as the original's.

## How to play

### What you need

- Windows 10 or 11 (64-bit), 64-bit Linux, or macOS on Apple silicon (build it yourself:
  [docs/release/macos.md](docs/release/macos.md)).
- A processor with AVX2 (Intel since 2013, AMD since 2015) and a graphics card with DirectX 12
  or Vulkan on Windows and Linux; any Apple silicon Mac on macOS, through Metal.
- **Your own copy of retail Skate 2 for Xbox 360**, extracted to a folder: the one that holds
  `default.xex` and the game's `data` folder. A disc image has to be extracted first.
- A controller. Xbox and PlayStation controllers work on Windows; on Linux and the Steam Deck,
  every controller SDL3 knows works. Skating isn't playable on keyboard and mouse yet, but may be implemented later.

### Windows

1. Download `skate2-windows-x64.zip` from the
   [latest release](https://github.com/skate6743/skate2-rust-port/releases/latest).
2. Unzip it into a folder of your own, for example `C:\Games\Skate 2`.
3. Run `skate2-pc.exe`. If Windows says it protected your PC, choose **More info** and then
   **Run anyway**.
4. The first time, select your copy's `default.xex`. The game remembers the folder.

### Linux and Steam Deck

1. Download `skate2-linux-x64.zip` and unzip it into a folder of your own.
2. Run `./skate2-pc` from that folder and select your copy's `default.xex`.
3. On the Steam Deck, do this once in Desktop Mode, then add `skate2-pc` to Steam as a non-Steam
   game.

What Linux needs (drivers, libraries, the Steam Deck) is in [docs/release/linux.md](docs/release/linux.md).

### macOS

There is no download for macOS yet: build it from source, which leaves a **Skate 2.app** in
`dist/macos`. Open it and select your copy's `default.xex` the first time. The app starts itself
in `~/Library/Application Support/Skate 2`, where `config.json` and `saves/` then stay, and it
opts into macOS Game Mode (`LSSupportsGameMode`, macOS 26 and later), which quiets background
work while you play. An Xbox, PlayStation or Switch controller works through SDL3, as on Linux.
What the build needs, and the parts of the game macOS does not have yet, are in
[docs/release/macos.md](docs/release/macos.md).

### If the game closes or shows nothing

- A message names the error and your graphics card, and a report is saved to
  `saves\crash_reports` beside the game. Send both with your report. On macOS the report is in
  `~/Library/Application Support/Skate 2/saves/crash_reports`, and a fatal error the app had no
  message box for is written there too; run `Skate 2.app/Contents/MacOS/Skate 2` from a terminal
  to see what it prints.
- To try the other graphics API, make a `.bat` file beside the game with `set WGPU_BACKEND=dx12`
  (or `vulkan`) on one line and `skate2-pc.exe` on the next, and run it. `set SKATE2_NO_MSAA=1`
  turns antialiasing off.

### Online

Choose **Online** on the main menu and pick a nickname. Releases connect to the project's
server. To play on another one, set `"server": "host:port"` in `online-config.json` beside the
game.

**Hosting:** `skate2-pc --server <ip:port>` runs the game in dedicated server mode, on Windows,
Linux or macOS. The Linux zip also holds `skate2-server`, the server alone, for a VPS or any machine
without a display. Players join only a server of the same release; `--update` on either program
puts the newest release in place. A server can also run a barebones moderation panel, with chat shadowbans
and IP bans: see [the server's documentation](docs/online/implementation.md#the-server).

### Updates

Each time it starts, the game looks for a newer release and offers it on the main menu. **Yes**
downloads it, checks its signature and starts the new version, which then shows what changed.
Settings and saves are never replaced. Online play on our community servers needs the newest
version.

### Saves and settings

Everything stays in the folder you run the game from: `config.json`, `online-config.json`,
`controller_config.json` and `saves/`. On macOS the app starts itself in `~/Library/Application Support/Skate 2`, so they stay
there. Replays and photos go to `saves/skate_reel/`. MP4 export needs FFmpeg with H.264 and
AAC: put `ffmpeg.exe` (`ffmpeg` on Linux and macOS) beside the game, or on your `PATH`.

## Not done yet

- [ ] Create-a-Spot
- [ ] DLC support (perhaps even being able to link Skate 3 data)
- [ ] Ranked online play
- [ ] Online Freeskate activities
- [ ] Controller vibration

## Building from source

You need a current stable Rust toolchain (the MSVC one on Windows).

**Windows:**

```powershell
.\build-release.bat                 # target\release\skate2-pc.exe
cargo run --release -p skate2-pc    # build it and play from the repository root
```

Played from the repository root, `config.json` and `saves/` stay there, and git ignores them.

**Linux** (or WSL, through `build-release-linux.bat`):

```sh
./build-release.sh                  # dist/linux/skate2-pc and dist/linux/skate2-server
```

The packages it needs are in [docs/release/linux.md](docs/release/linux.md).

**macOS** (built and played on Apple silicon; Intel Macs are untested):

```sh
./build-release-macos.sh            # dist/macos/skate2-pc, skate2-server and 'Skate 2.app'
cargo run --release -p skate2-pc    # build the game and start it from the repository root
```

SDL3 builds from source here too, so CMake, Ninja and the Xcode command line tools are needed;
the bundle's icon is made with the `sips` and `iconutil` that macOS ships. The rest is in
[docs/release/macos.md](docs/release/macos.md).

A build without `release-secret.txt` makes its own online key, so it plays only on a server run
from the same executable (`skate2-pc --server`). Releases (the release key, the update files and
the steps) are in [docs/release/auto-update.md](docs/release/auto-update.md). Run the tests with
`cargo test --workspace`; tests that read the original files use `SKATE2_GAME_ROOT`. Diagnostic
runs, traces and research tools are in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## How it's made

- Every system is typed, named Rust that reproduces the original's behaviour. The source of
  truth is the January 17, 2009 debug build, `sk82_na_m.xex`: its code and debug symbols are
  studied as evidence, and no PowerPC code is translated or run.
- Nothing is emulated. Rendering, audio and input use native PC backends: wgpu, cpal, XInput
  and SDL3.
- The game's assets stay on your machine and load through reconstructed file formats.
- Matching the original is backed by evidence and tests, and every deliberate difference is
  written down.

| Topic | Read |
| --- | --- |
| Reconstruction rules | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| Source layout | [crates/skate2-core/src/README.md](crates/skate2-core/src/README.md), [docs/CORE_LAYOUT.md](docs/CORE_LAYOUT.md) |
| Career, missions and challenges | [docs/career/](docs/career/README.md) |
| Online play and the netcode | [docs/online/](docs/online/README.md), [implementation](docs/online/implementation.md) |
| Party Play | [docs/partyplay/](docs/partyplay/README.md) |
| Audio | [docs/audio/](docs/audio/README.md) |
| AI skaters | [docs/ai/](docs/ai/README.md) |
| Graphics | [docs/GRAPHICS.md](docs/GRAPHICS.md) |
| Replay and photo export | [docs/MEDIA_EXPORT.md](docs/MEDIA_EXPORT.md) |
| Releases and updates | [docs/release/auto-update.md](docs/release/auto-update.md), [Linux](docs/release/linux.md), [macOS](docs/release/macos.md) |

## AI usage

This project was written with the help of large language models. They turned research into
code and documentation; the behaviour is checked against the debug build's own code and, where
it matters, against the original game running. Some things are still partly unfinished, so some features might be missing.

## Credits

- **EA Black Box**, for making Skate 2.
- **FFmpeg**, whose WMA Pro decoder `skate2-xma` ports to play the game's XMA audio.
- **wgpu, winit, cpal, SDL3** and the rest of the Rust ecosystem this is built on.

## License

The source code is MIT-licensed ([LICENSE](LICENSE)), except the `skate2-xma` audio decoder,
which is LGPL-2.1-or-later like the FFmpeg code it ports
([crates/skate2-xma/LICENSE](crates/skate2-xma/LICENSE)).

This is an unofficial fan project, not affiliated with or endorsed by Electronic Arts. Skate and
Skate 2 are trademarks of Electronic Arts Inc. The repository holds no game executables, symbols
or assets: you need your own legally obtained copy of the game.
