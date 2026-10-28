#!/bin/sh
# Builds the Linux release programs into dist/linux: the game (skate2-pc, which also hosts with
# --server) and the dedicated server (skate2-server). With release-secret.txt (the same file
# build-release.bat reads on Windows) their secret is the release's, so they meet the Windows game
# of the same version in Cargo.toml. `./build-release.sh server` builds only the server, all a
# server's host needs; `./build-release.sh game` only the game. docs/release/linux.md lists what
# the build needs.
set -e
cd "$(dirname "$0")"
# rustup's PATH, which a non-interactive shell (wsl.exe running this script) does not read.
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi
case "${1:-all}" in
    all | game | server) programs=${1:-all} ;;
    *) echo "usage: build-release.sh [game | server]"; exit 1 ;;
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
# updates to (docs/release/auto-update.md).
if [ -f release-repository.txt ]; then
    SKATE2_UPDATE_REPOSITORY=$(tr -d ' \r\n' < release-repository.txt)
    export SKATE2_UPDATE_REPOSITORY
fi
# release-server.txt (host:port, kept out of the repository) is the server a player's game
# connects to when their file names none; SKATE2_DEFAULT_SERVER set by the caller wins.
if [ -z "${SKATE2_DEFAULT_SERVER:-}" ] && [ -f release-server.txt ]; then
    SKATE2_DEFAULT_SERVER=$(tr -d ' \r\n' < release-server.txt)
    export SKATE2_DEFAULT_SERVER
fi
if [ -n "${SKATE2_DEFAULT_SERVER:-}" ]; then
    echo "Default server: $SKATE2_DEFAULT_SERVER"
else
    echo "No release-server.txt: the default server is this machine's."
fi
# Release binaries embed their dependencies' source paths, which name the home folder and with it
# the user name; the build writes ~ in its place.
host=$(rustc -vV | sed -n 's/^host: //p')
remap="'--remap-path-prefix=$HOME=~'"
# The builds stay on Linux's own file system, away from a Windows drive WSL may have mounted and
# from the Windows build's target folder. Release builds are not incremental here.
target=${CARGO_TARGET_DIR:-$HOME/.cache/skate2-recomp}
export CARGO_INCREMENTAL=0
mkdir -p dist/linux
# The copies in dist/linux keep their symbols (a crash report names the functions) but not the
# release profile's line tables, three quarters of the game's file; the builds keep them.
release() {
    objcopy --strip-debug "$1" "dist/linux/$(basename "$1")"
}
if [ "$programs" != server ]; then
    # The game for the processors the Windows build takes (.cargo/config.toml): x86-64-v3, AVX2.
    # One compiler job, as on Windows: the game's crates need several GB each.
    CARGO_TARGET_DIR="$target/game" cargo build --release -p skate2-pc -j 1 \
        --config "target.$host.rustflags=['-C', 'target-cpu=x86-64-v3', $remap]"
    release "$target/game/release/skate2-pc"
    echo "Built dist/linux/skate2-pc: run ./skate2-pc to play, ./skate2-pc --server [ip:port] to host"
fi
if [ "$programs" != game ]; then
    # The server for any x86-64 processor: a VPS may not offer AVX2.
    CARGO_TARGET_DIR="$target/server" cargo build --release -p skate2-server \
        --config "target.$host.rustflags=[$remap]"
    release "$target/server/release/skate2-server"
    echo "Built dist/linux/skate2-server: run it as ./skate2-server [ip:port]"
fi
