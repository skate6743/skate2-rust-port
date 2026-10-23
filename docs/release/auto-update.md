# Releases and updates

A release build of the game, on Windows or Linux, looks for a newer release on the project's
GitHub releases when it starts, offers it on the main menu, and puts it in place itself. A
server says when a newer release is out and updates from a command line: `skate2-server
--update`, or `skate2-pc --update` when the game hosts. Online play needs it: a game connects
only to a server of the same release.

## What a player sees

- Once the main menu is up with nothing over it, a newer release opens "Update Available":
  "Version 0.2.0 is out (you have 0.1.0). Online play needs the newest version. Update now?"
  with Yes and No (B is No).
- **Yes:** "Downloading version 0.2.0: 45%", then "Version 0.2.0 is installed. The game
  starts again now." The screen fades, the game closes, and the new version starts with the
  same arguments, in the same folder. On Linux the new version takes the old one's process, so
  whatever started the game (a terminal, Steam) still has it.
- **After the update,** a player's first launch of the new release, once the main menu is up
  with nothing over it, shows "Version 0.2.0": "Update Changelogs:" and each change in
  `changelog.txt` under a dash, a blank line between them (as many as fit on the screen; past
  that the last line says how many more), with OK, before any newer release is offered.
  `config.json` then names the release as seen (`changelog_seen`), so it shows once, whatever
  version the update came from and however the new release was put in place. A first install,
  which has no `config.json` before the game folder is picked, records it without showing it,
  and a `changelog.txt` without changes shows nothing. `skate2-pc --changelog` shows it at any
  launch.
- **No:** the game carries on. That version is offered again at the next start, or when the
  server refuses this version ("Your game doesn't match the server's version...").
- **A failure** (no connection, a release that does not check out, a folder that cannot be
  written to) shows "The update could not be installed, and the game is unchanged." with what
  to do, and OK.
- With no connection, or when this is the newest release, nothing shows.

The settings and saves (`config.json`, `online-config.json`, `saves/`) are never replaced. The
game must be in a folder it can write to: not under Program Files.

## How it works (`crates/skate2-update`)

**Each release on GitHub holds four files:**

| File | What it is |
| --- | --- |
| `skate2-windows-x64.zip` | The game for Windows: `skate2-pc.exe` (and any other files to install beside it). |
| `skate2-linux-x64.zip` | The game and the dedicated server for Linux: `skate2-pc` and `skate2-server`. |
| `latest-release.txt` | The manifest: the version and each zip's name, size and SHA-256. |
| `latest-release.txt.sig` | The manifest's Ed25519 signature. |

```text
skate2 release 1
version 0.2.0
asset windows skate2-windows-x64.zip 48123456 <SHA-256>
asset linux skate2-linux-x64.zip 23123456 <SHA-256>
```

**The check:**

1. The game reads `https://github.com/<owner>/<name>/releases/latest/download/latest-release.txt`.
   GitHub serves that file from the newest release that is not a draft or a pre-release.
2. It reads the signature from the release the manifest names (`.../releases/download/v<version>/`).
3. It checks the signature against the public key built into it. A manifest that is not signed
   with this project's release key is refused, so a changed release file or a compromised
   GitHub account cannot push an update.
4. A version newer than its own, with a zip for its platform, is offered.

**The update:**

1. The zip downloads beside the executable as `skate2-update.zip.part`.
2. Its size and SHA-256 must match the signed manifest.
3. Each file of the zip is written beside its target as `<name>.new`. Then each target becomes
   `<name>.old` (Windows lets a running executable be renamed, not deleted) and the `.new` takes
   its name. If any step fails, every file is put back.
4. The `.old` files are listed in `skate2-update.cleanup` and deleted at the next start, once the
   old version has exited. Only the files on that list are touched.

A zip's files may sit in one top folder, which is dropped. The zip must hold the executable of
the program updating (`skate2-pc.exe` on Windows; `skate2-pc` or `skate2-server` on Linux);
links and names that leave the folder are refused. Files keep the zip's Unix permissions, so
the Linux programs stay executable.

**Platforms:** `windows` is the Windows game (which also hosts with `--server`); `linux` holds
the Linux game and the dedicated server, and each of the two updates from it, so a server's
folder gets the game's file beside it as well. The Windows program also runs on Linux (Proton,
Wine), and then updates from the Windows zip. A macOS build updates from nothing: no release
holds a zip for it, so `Platform::current()` is `None` there, the check is passed over and
`--update` says the build does not update ([macos.md](macos.md)). A build passes over the
manifest's lines it does not know, so a later release can add a zip under a name of its own
without older builds refusing the release; only a new first line, a new format, stops them.

**Which builds update:** only a release build that names its repository. A release build has
`release-secret.txt`, from which the build script derives the public key it checks signatures
with (`apps/build_secret.rs`). The repository comes from `release-repository.txt`. A development
build has no key and never looks for updates.

## The release key and the build secret

`release-secret.txt` (64 hexadecimal digits, made once by `tools/new_release_secret.py`, kept
private, never committed) is the one secret a release needs. Two things are derived from it,
each under its own label:

- **The build secret** = SHA-256 of the release key and the workspace version in `Cargo.toml`.
  Every connection starts with a handshake keyed by this secret, and the server refuses any
  build whose secret differs ("Your game doesn't match the server's version"). So:
  - the Windows game, the Linux game and the Linux server built from the same
    `release-secret.txt` and the same version have the same secret and play together, though
    they are separate compiles on separate systems: nothing about the system goes into it;
  - raising the version gives the next release a new secret, so old games cannot join the new
    server until they update;
  - a build without `release-secret.txt` makes a random secret each compile, so it meets only a
    server run from the same executable (`skate2-pc --server`).
- **The update signing key** = an Ed25519 key whose seed is SHA-256 of a label and the release
  key. It does not depend on the version, so every release checks the next one. The release
  tool signs with it; builds carry only its public key.

Never change `release-secret.txt` after the first release: builds made with the old key would
neither reach the new server nor accept the new releases, and their players would have to
download a new release by hand.

## Setting up (once)

1. Make the release key: `python tools/new_release_secret.py` (keep `release-secret.txt` safe;
   anyone with it can sign updates).
2. Name the repository: create `release-repository.txt` holding `owner/name` (for example
   `yourname/skate2-recomp`). It is not secret. The repository must be public: GitHub serves a
   private repository's release files only to people signed in to it.

## Each release

1. Raise `version` in the workspace `Cargo.toml` (for example `0.1.0` to `0.2.0`).
2. Write the release's changes in `changelog.txt` at the repository's root, one per line: the
   game adds the dash, and lines starting with `#` are left out. It is built into the game.
   About ten short changes fit in the popup, fewer long ones.
3. Build the game on Windows: `build-release.bat` (`target\release\skate2-pc.exe`). Its icon is
   `apps/skate2-pc/skate2.ico`, which the game's `build.rs` links into the executable.
4. Build the Linux game and server from the same files, `release-secret.txt` and
   `release-repository.txt` included: `build-release-linux.bat` runs `build-release.sh` in WSL,
   or run `./build-release.sh` on Linux. Both land in `dist/linux` (`skate2-pc`,
   `skate2-server`). Setting the Linux build up: [linux.md](linux.md).
   `./build-release-macos.sh` builds both for macOS the same way, into `dist/macos`
   ([macos.md](macos.md)); no release holds that build yet.
5. Package and sign on Windows: `package-release.bat`. It writes
   `dist\skate2-windows-x64.zip`, `dist\skate2-linux-x64.zip`, `dist\latest-release.txt` and
   `dist\latest-release.txt.sig`, and refuses a program older than `Cargo.toml`: one built
   before the version changed would report the old version, and its players would be offered
   the same release at every start. The release tool can also be run by hand:
   `cargo run --release -p skate2-update --bin skate2-release -- pack|sign|public-key ...`.
6. On GitHub, create a release tagged `v<version>` (for example `v0.2.0`), not a pre-release,
   and upload the four files.
7. Update the server: stop it, run `./skate2-server --update` (`./skate2-pc --update` when the
   game hosts), start it again. (Unzipping the new Linux zip over the old files works too.)

Players get the update the next time they start the game. Until then the new server refuses
their old build, and they are offered the update again when that happens.
