#!/bin/sh
# Builds the macOS release programs into dist/macos: the game (skate2-pc, which also hosts with
# --server), the dedicated server (skate2-server) and the Skate 2.app bundle that runs the game
# from its own data folder. With release-secret.txt (the same file the other scripts read) their
# secret is the release's, so they meet the Windows and Linux builds of the same version in
# Cargo.toml. `./build-release-macos.sh server` builds only the server, all a server's host
# needs; `./build-release-macos.sh game` only the game. docs/release/macos.md lists what the
# build needs.
set -e
cd "$(dirname "$0")"
# rustup's PATH, which a non-interactive shell does not read.
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi
case "${1:-all}" in
    all | game | server) programs=${1:-all} ;;
    *) echo "usage: build-release-macos.sh [game | server]"; exit 1 ;;
esac
if [ -f release-secret.txt ]; then
    SKATE2_RELEASE_SECRET=$(tr -d ' \r\n' < release-secret.txt)
    export SKATE2_RELEASE_SECRET
fi
if [ -n "${SKATE2_RELEASE_SECRET:-}" ]; then
    echo "Release build: its secret comes from the release key and the version in Cargo.toml."
else
    echo "Development build: no release-secret.txt, so its secret is random."
fi
# release-repository.txt names the GitHub repository (owner/name) whose releases a release build
# updates to (docs/release/auto-update.md). A build for macOS has no zip to update from yet, so
# its self-update stays inert whatever this file names.
if [ -f release-repository.txt ]; then
    SKATE2_UPDATE_REPOSITORY=$(tr -d ' \r\n' < release-repository.txt)
    export SKATE2_UPDATE_REPOSITORY
fi
# Release binaries embed their dependencies' source paths, which name the home folder and with it
# the user name; the build writes ~ in its place.
remap="--remap-path-prefix=$HOME=~"
# The bundle's version is the workspace's, so it names the release the build belongs to.
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist/macos
# The game's crates need several GB each to compile: two compiler jobs keep a 16 GB machine off
# the swap file it would need for more.
if [ "$programs" != server ]; then
    RUSTFLAGS="$remap" cargo build --release -p skate2-pc -j 2
    app="dist/macos/Skate 2.app"
    resources="$app/Contents/Resources"
    # Assembled afresh: a rebuild keeps nothing an earlier layout left in the bundle, which would
    # be sealed into this signature and counted in its size.
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$resources"
    # The bundle's executable is the game itself, where a bundle keeps it: the Dock identifies a
    # running app by that path and draws the bundle's icon for it, which a launcher script that
    # execs a program elsewhere in the bundle would lose. The game starts itself in its own folder
    # instead of being started there (apps/skate2-pc/src/main.rs), because a bundle's app is
    # launched in "/".
    cp target/release/skate2-pc "$app/Contents/MacOS/Skate 2"
    cp target/release/skate2-pc dist/macos/skate2-pc
    # The icon: the Windows icon's art as a Big Sur squircle, in the sizes an .icns holds. sips and
    # iconutil come with macOS, so the repository keeps the 1024 master and not the .icns.
    iconset=$(mktemp -d)/skate2.iconset
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" apps/skate2-pc/skate2-macos.png \
            --out "$iconset/icon_${size}x${size}.png" >/dev/null
        sips -z "$((size * 2))" "$((size * 2))" apps/skate2-pc/skate2-macos.png \
            --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil --convert icns "$iconset" --output "$resources/skate2.icns"
    rm -rf "$(dirname "$iconset")"
    # A bundle's executable is in Contents/MacOS; this one runs the game from Contents/Resources in
    # a folder of its own: Finder starts an app in "/", and the game keeps config.json,
    # online-config.json and saves/ in the folder it is started from.
    cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Skate 2</string>
  <key>CFBundleDisplayName</key><string>Skate 2</string>
  <key>CFBundleIdentifier</key><string>org.skate6743.skate2-pc</string>
  <key>CFBundleExecutable</key><string>Skate 2</string>
  <key>CFBundleIconFile</key><string>skate2.icns</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>LSSupportsGameMode</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT-licensed fan project; Skate and Skate 2 are trademarks of Electronic Arts Inc.</string>
</dict></plist>
PLIST
    # Ad-hoc signatures, which is all an unsigned build can have: they keep macOS from treating
    # the bundle's own files as foreign when it verifies them.
    codesign --force --sign - "$app" 2>/dev/null || true
    echo "Built dist/macos/skate2-pc and 'dist/macos/Skate 2.app': open the app to play"
fi
if [ "$programs" != game ]; then
    RUSTFLAGS="$remap" cargo build --release -p skate2-server -j 2
    cp target/release/skate2-server dist/macos/skate2-server
    echo "Built dist/macos/skate2-server: run it as ./skate2-server [ip:port]"
fi
