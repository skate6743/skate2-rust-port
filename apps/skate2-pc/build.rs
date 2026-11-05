//! The build's secret: see `../build_secret.rs`. A development build makes it anew whenever
//! anything the executable is built from changes. On Windows the executable also carries its
//! icon, `skate2.ico`, and the window thread's stack size.

#[path = "../build_secret.rs"]
mod build_secret;

fn main() {
    build_secret::emit(&["build.rs", "../build_secret.rs", "Cargo.toml", "src", "../../crates", "../../Cargo.toml", "../../Cargo.lock"]);
    let target = |key: &str, value: &str| std::env::var(key).is_ok_and(|actual| actual == value);
    if target("CARGO_CFG_TARGET_OS", "windows") && target("CARGO_CFG_TARGET_ENV", "msvc") {
        embed_icon("skate2.ico");
        // The window thread loads the menus and builds the client by value, and with
        // SKATE2_SERIAL_SIM it runs the simulation too: it gets the SIM thread's 16 MB, where
        // Windows reserves 1 MB (Linux and macOS 8).
        println!("cargo:rustc-link-arg-bins=/STACK:{}", 16 << 20);
    }
    embed_pool_ground();
}

/// PC weather: the seven outdoor pools' floor heights the rain's water field fills
/// (`weather_hydrology::solve_named_pools`), a game-derived grid kept outside the repository and
/// given as `SKATE2_POOL_GROUND`. Without it the build carries none and those pools collect
/// water from the live ground alone.
fn embed_pool_ground() {
    println!("cargo:rerun-if-env-changed=SKATE2_POOL_GROUND");
    let bytes = std::env::var_os("SKATE2_POOL_GROUND").map_or_else(Vec::new, |path| {
        println!("cargo:rerun-if-changed={}", std::path::Path::new(&path).display());
        let bytes = std::fs::read(path).expect("the pool-ground grid");
        assert_eq!(bytes.len(), 7 * (8 + 73 * 73 * 4), "a pool-ground grid of seven 73x73 pools");
        bytes
    });
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo's output folder"));
    std::fs::write(out.join("pool-ground.bin"), bytes).expect("pool-ground.bin");
}

/// Links `icon` into the executable as icon group 1, the one Explorer shows and the window
/// loads: a resource file holding each of its images as an `RT_ICON` and the `RT_GROUP_ICON`
/// that lists them, which the MSVC linker takes as an input.
fn embed_icon(icon: &str) {
    println!("cargo:rerun-if-changed={icon}");
    let file = std::fs::read(icon).expect("the executable's icon");
    let word = |at: usize| u16::from_le_bytes([file[at], file[at + 1]]);
    let dword = |at: usize| {
        let value = u32::from_le_bytes([file[at], file[at + 1], file[at + 2], file[at + 3]]);
        usize::try_from(value).expect("an icon image inside the file")
    };
    let count = word(4);
    let mut resources = resource(0, 0, 0, &[]);
    let mut group = [0_u16, 1, count].iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<_>>();
    for id in 1..=count {
        let entry = 6 + 16 * usize::from(id - 1);
        let (size, offset) = (dword(entry + 8), dword(entry + 12));
        resources.extend(resource(3, id, 0x1010, &file[offset..offset + size]));
        group.extend_from_slice(&file[entry..entry + 12]);
        group.extend(id.to_le_bytes());
    }
    resources.extend(resource(14, 1, 0x1030, &group));
    let path = std::path::Path::new(&std::env::var("OUT_DIR").expect("cargo's output directory")).join("skate2.res");
    std::fs::write(&path, resources).expect("the icon's resource file");
    println!("cargo:rustc-link-arg-bins={}", path.display());
}

/// One entry of a 32-bit resource file: its header, with the type and name as ordinals and the
/// neutral language, then its data padded to four bytes. Type 0 is the empty entry such a file
/// starts with.
fn resource(kind: u16, name: u16, flags: u16, data: &[u8]) -> Vec<u8> {
    let size = u32::try_from(data.len()).expect("a resource under 4 GB");
    let mut entry = Vec::with_capacity(32 + data.len() + 3);
    entry.extend(size.to_le_bytes());
    entry.extend(32_u32.to_le_bytes());
    entry.extend([0xffff, kind, 0xffff, name].iter().flat_map(|value| value.to_le_bytes()));
    entry.extend(0_u32.to_le_bytes());
    entry.extend(flags.to_le_bytes());
    entry.extend(0_u16.to_le_bytes());
    entry.extend([0_u8; 8]);
    entry.extend_from_slice(data);
    entry.resize(entry.len().next_multiple_of(4), 0);
    entry
}
