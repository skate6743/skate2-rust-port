//! The menu table extractor against local executables (never committed):
//! the TU3 memory image (documented addresses) and the disc's default.xex
//! (another build: proves the locators do not depend on addresses).
//! Run with `cargo test -p skate-data --test menu_tables -- --ignored`;
//! each test skips when its file is absent. Override the paths with
//! SKATE3_TU3_IMAGE / SKATE3_DISC_XEX.
use skate_data::menu_tables::{MenuTables, extract};
use skate_data::xex::XexImage;
use std::path::PathBuf;

fn local(var: &str, relative: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(var) {
        return Some(PathBuf::from(path));
    }
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.ancestors()
        .map(|a| a.join(relative))
        .find(|p| p.is_file())
}

fn tu3() -> Option<MenuTables> {
    let path = local(
        "SKATE3_TU3_IMAGE",
        ".local/tu3-image/default_82000000_011B0000.bin",
    )?;
    let image = XexImage {
        base_address: 0x8200_0000,
        image: std::fs::read(path).ok()?,
    };
    Some(extract(&image).expect("TU3 tables"))
}

fn disc() -> Option<MenuTables> {
    let path = local("SKATE3_DISC_XEX", ".local/skate3-disc/default.xex")?;
    let image = XexImage::parse(&std::fs::read(path).ok()?).expect("disc xex unpacks");
    Some(extract(&image).expect("disc tables"))
}

fn names(list: &[String]) -> Vec<&str> {
    list.iter().map(String::as_str).collect()
}

/// What the retail notes document, true for any build.
fn check_structure(t: &MenuTables) {
    // TU3 has 36 rows; the disc build 35 (no SKATEFEED_STATUS_MESSAGE), so rows
    // after it shift: screens are compared by string id.
    assert!(
        t.settings_rows.len() == 35 || t.settings_rows.len() == 36,
        "{} rows",
        t.settings_rows.len()
    );
    assert_eq!(t.settings_rows[7].id, "ID_GAMESETTINGS_SFXVOLUME");
    assert_eq!(t.settings_rows[7].widget, "slider");
    assert_eq!(
        t.settings_rows[11].id,
        "ID_GAMESETTINGS_CAMERA_ANGLE_TOGGLE"
    );
    assert_eq!(t.settings_rows[11].widget, "selector");
    assert_eq!(
        t.settings_rows.last().unwrap().id,
        "ID_GAMESETTINGS_SKATEFEED_SETTINGS"
    );
    assert_eq!(t.settings_screens.len(), 13);
    let screen = |i: usize| -> Vec<String> {
        t.to_menu_data().screens[i]
            .iter()
            .map(|s| s.trim_start_matches("ID_GAMESETTINGS_").to_owned())
            .collect()
    };
    assert_eq!(
        screen(1),
        [
            "DIFFICULTY_SETTINGS",
            "AUDIO_SETTINGS",
            "VIDEO_SETTINGS",
            "CONTROL_SETTINGS",
            "ONLINE_SETTINGS",
            "SKATEFEED_SETTINGS"
        ]
    );
    assert_eq!(
        screen(3),
        ["SFXVOLUME", "DIALOGVOLUME", "MUSICVOLUME", "SFXPACK"]
    );
    assert_eq!(
        screen(6),
        [
            "CAMERA_ANGLE_TOGGLE",
            "VIDEO_CALIBRATION",
            "SUBTITLE_TOGGLE",
            "MINIMAP_TOGGLE",
            "CAM_MAN_TOGGLE",
            "HUD_TOGGLE"
        ]
    );
    assert_eq!(screen(12), ["SFXVOLUME", "DIALOGVOLUME", "MUSICVOLUME"]);
    assert_eq!(
        screen(11)[..4],
        [
            "SKATEFEED_BOOTFLOW",
            "SKATEFEED_CHYRON",
            "SKATEFEED_INCOMING_MESSAGE",
            "SKATEFEED_OUTGOING_MESSAGE"
        ]
    );

    let categories: Vec<&str> = t.categories.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        categories,
        ["Multiplayer", "Options", "SinglePlayer", "Create", "Learn"]
    );
    // TU3 added Party Play (items 47 PartyPlay, 48 ExitPartyPlay); the disc build has 47 items.
    assert!(
        t.items.len() == 47 || t.items.len() == 49,
        "{} items",
        t.items.len()
    );
    assert_eq!(t.items[46].name, "ReferDemoToFriend");
    assert_eq!(t.items[0].name, "ReplayEditor");
    assert_eq!(t.items[4].name, "GameSettings");
    assert_eq!(t.items[4].icon, "settings");
    assert_eq!(t.items[9].sub_option_kind, 3);
    assert_eq!(t.items[40].name, "Project10");
    let party_play = t.items.len() == 49;
    if party_play {
        assert_eq!(t.items[48].name, "ExitPartyPlay");
    }

    assert_eq!(t.keys.len(), 56);
    assert_eq!(
        (t.keys[0].name.as_str(), t.keys[0].apt_key),
        ("AptStart", 0x12d)
    );
    assert!(
        t.keys
            .iter()
            .any(|k| k.name == "AptNext" && k.apt_key == 0x12e && !k.released)
    );
    assert!(
        t.keys
            .iter()
            .any(|k| k.name == "AptUpReleased" && k.apt_key == 0xe && k.released)
    );

    let career = t.mode("Career").expect("Career");
    assert_eq!(career.title, "ID_CROSSBAR_SINGLE_PLAYER_MODE_TITLE");
    assert_eq!(career.slots, 8);
    let tabs: Vec<&str> = career.tabs.iter().map(|x| x.category.as_str()).collect();
    assert_eq!(
        tabs,
        ["SinglePlayer", "Multiplayer", "Create", "Learn", "Options"]
    );
    let mut main = vec![
        "ChallengeMap",
        "MyCareerTeam",
        "SkateProfileOffline",
        "SkateWith",
        "SkateFeed",
        "SoloFreeskate",
    ];
    if party_play {
        main.push("PartyPlay");
    }
    assert_eq!(names(&career.tabs[0].items), main);
    assert_eq!(
        names(&career.tabs[1].items),
        [
            "OnlineChallengeMap",
            "OnlineTeam",
            "SkateProfileOnline",
            "Contacts",
            "XBLPartyInvite",
            "Leaderboards"
        ]
    );
    assert_eq!(
        names(&career.tabs[4].items),
        ["GameSettings", "MusicPlayer", "Extras", "SignIn"]
    );

    let park = t.mode("CareerPark").expect("CareerPark");
    assert_eq!(park.slots, 9);
    assert_eq!(
        names(&park.tabs[2].items),
        [
            "SavePark",
            "EnterSkatePark",
            "ReplayEditor",
            "SkateReel",
            "RatePark",
            "FlagSkatePark",
            "Project10"
        ]
    );
    assert!(!park.tabs[0].items.iter().any(|i| i == "PartyPlay"));

    let free = t.mode("FreePlay").expect("FreePlay");
    assert_eq!(free.tabs.len(), 4);
    assert_eq!(free.slots, 8);
    assert_eq!(
        names(&free.tabs[0].items),
        [
            "ChallengeMap",
            "ResumeCareer",
            "SoloFreeskateOptions",
            "SkateWith",
            "SkateProfileOffline"
        ]
    );
    let free_park = t.mode("FreePlayPark").expect("FreePlayPark");
    assert_eq!((free_park.slots, free_park.tabs.len()), (9, 4));
    let restricted = t.mode("FreeSkateRestricted").expect("FreeSkateRestricted");
    assert_eq!(names(&restricted.tabs[0].items), ["ChallengeMap"]);

    // The engine structure built from it.
    let data = t.to_menu_data();
    assert_eq!(data.settings[0].id, "GAMESETTINGS_ROW_0");
    assert_eq!(
        data.screens[3],
        [
            "ID_GAMESETTINGS_SFXVOLUME",
            "ID_GAMESETTINGS_DIALOGVOLUME",
            "ID_GAMESETTINGS_MUSICVOLUME",
            "ID_GAMESETTINGS_SFXPACK"
        ]
    );
    let json = t.to_json();
    assert_eq!(MenuTables::from_json(&json).unwrap(), *t);
}

#[test]
#[ignore = "needs the local TU3 image"]
fn tu3_image_matches_the_documented_tables() {
    let Some(t) = tu3() else {
        eprintln!("skipped: no TU3 image");
        return;
    };
    check_structure(&t);
    assert_eq!(t.addresses["settings_rows"], "0x83026EC8");
    assert_eq!(t.addresses["settings_screens"], "0x83026FE8");
    assert_eq!(t.addresses["crossbar_categories"], "0x8302751C");
    assert_eq!(t.addresses["crossbar_items"], "0x83027558");
    assert_eq!(t.addresses["apt_keys"], "0x83027BF0");
    assert_eq!(t.mode("Career").unwrap().vtable, "0x82303D54");
    assert_eq!(t.mode("CareerPark").unwrap().vtable, "0x82303D04");
    assert_eq!(
        (t.settings_rows.len(), t.items.len(), t.modes.len()),
        (36, 49, 9)
    );
}

#[test]
#[ignore = "needs the local disc executable"]
fn disc_executable_gives_the_same_tables() {
    let Some(t) = disc() else {
        eprintln!("skipped: no disc default.xex");
        return;
    };
    for (name, address) in &t.addresses {
        eprintln!("disc {name} at {address}");
    }
    for mode in &t.modes {
        eprintln!(
            "disc mode {} {} slots {} at {}",
            mode.key, mode.title, mode.slots, mode.vtable
        );
    }
    check_structure(&t);
    if let Some(tu3) = tu3() {
        // Same content apart from TU3's Party Play additions, at other addresses.
        assert_eq!(t.items[..], tu3.items[..t.items.len()]);
        let ids: Vec<&str> = t.settings_rows.iter().map(|r| r.id.as_str()).collect();
        let tu3_rows: Vec<_> = tu3
            .settings_rows
            .iter()
            .filter(|r| ids.contains(&r.id.as_str()))
            .cloned()
            .collect();
        assert_eq!(t.settings_rows, tu3_rows);
        let (mine, theirs) = (t.to_menu_data(), tu3.to_menu_data());
        for (a, b) in mine.screens.iter().zip(&theirs.screens) {
            let b: Vec<&String> = b.iter().filter(|id| mine.setting(id).is_some()).collect();
            assert_eq!(a.iter().collect::<Vec<_>>(), b);
        }
        assert_eq!(t.categories, tu3.categories);
        assert_eq!(t.keys, tu3.keys);
        let known: Vec<&str> = t.items.iter().map(|i| i.name.as_str()).collect();
        for mode in &t.modes {
            let Some(other) = tu3.mode(&mode.key) else {
                continue;
            };
            let filtered: Vec<(String, Vec<String>)> = other
                .tabs
                .iter()
                .map(|tab| {
                    (
                        tab.category.clone(),
                        tab.items
                            .iter()
                            .filter(|i| known.contains(&i.as_str()))
                            .cloned()
                            .collect(),
                    )
                })
                .collect();
            let mine: Vec<(String, Vec<String>)> = mode
                .tabs
                .iter()
                .map(|tab| (tab.category.clone(), tab.items.clone()))
                .collect();
            assert_eq!(
                (mode.slots, &mode.title, mine),
                (other.slots, &other.title, filtered),
                "{}",
                mode.key
            );
        }
    }
}
