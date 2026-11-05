use super::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn icon(name: &str) -> Icon {
    Icon::Retail(name.into())
}

/// A small hand-made structure in the retail shape (ids only).
fn fixture() -> MenuData {
    let category = |id: &str| Category {
        id: id.into(),
        label: format!("L_{id}"),
        icon: icon("x"),
    };
    let item = |id: &str| Item {
        id: id.into(),
        label: format!("L_{id}"),
        icon: icon("x"),
        help: String::new(),
        sub_option_kind: -1,
    };
    let setting = |id: &str, widget| Setting {
        id: id.into(),
        label: id.into(),
        widget,
        link: retail_link(id),
    };
    let tab = |c: &str, items: &[&str]| Tab {
        category: c.into(),
        items: items.iter().map(|s| s.to_string()).collect(),
    };
    let mut screens = vec![Vec::new(); 13];
    screens[1] = vec![
        "ID_GAMESETTINGS_DIFFICULTY_SETTINGS".into(),
        "ID_GAMESETTINGS_AUDIO_SETTINGS".into(),
    ];
    screens[2] = vec!["ID_GAMESETTINGS_DIFFICULTY_OPTIONS".into()];
    screens[3] = vec![
        "ID_GAMESETTINGS_SFXVOLUME".into(),
        "ID_GAMESETTINGS_SFXPACK".into(),
    ];
    MenuData {
        categories: ["Options", "SinglePlayer", "Create"].map(category).to_vec(),
        items: [
            "ChallengeMap",
            "GameSettings",
            "SkateWith",
            "Project10",
            "SavePark",
            "ReplayEditor",
        ]
        .map(item)
        .to_vec(),
        settings: vec![
            setting("ID_GAMESETTINGS_DIFFICULTY_SETTINGS", Widget::Option),
            setting("ID_GAMESETTINGS_AUDIO_SETTINGS", Widget::Option),
            setting("ID_GAMESETTINGS_DIFFICULTY_OPTIONS", Widget::Selector),
            setting("ID_GAMESETTINGS_SFXVOLUME", Widget::Slider),
            setting("ID_GAMESETTINGS_SFXPACK", Widget::Selector),
        ],
        screens,
        modes: vec![
            Mode {
                key: "Career".into(),
                title: "T".into(),
                slots: 8,
                tabs: vec![
                    tab("SinglePlayer", &["ChallengeMap", "SkateWith"]),
                    tab("Create", &["ReplayEditor", "Project10"]),
                    tab("Options", &["GameSettings"]),
                ],
            },
            Mode {
                key: "CareerPark".into(),
                title: "T".into(),
                slots: 9,
                tabs: vec![
                    tab("SinglePlayer", &["ChallengeMap", "SkateWith"]),
                    tab("Create", &["SavePark", "ReplayEditor"]),
                ],
            },
        ],
    }
}

fn engine_handler(owner: &str, count: Arc<AtomicUsize>) -> Handler {
    Handler {
        owner: Owner::Engine(owner.into()),
        select: true,
        highlight: true,
        callback: Some(Arc::new(move |_, _| {
            count.fetch_add(1, Ordering::SeqCst);
        })),
    }
}

fn mod_handler(id: &str) -> Handler {
    Handler {
        owner: Owner::Mod(id.into()),
        select: true,
        highlight: false,
        callback: None,
    }
}

#[test]
fn every_entry_is_greyed_until_a_handler_registers() {
    let mut registry = MenuRegistry::new(fixture());
    let ctx = MenuContext::new("Career");
    let rows = registry.tab_rows("SinglePlayer", &ctx);
    assert_eq!(
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["ChallengeMap", "SkateWith"]
    );
    assert!(rows.iter().all(|r| !r.enabled));
    let map = EntryRef::item("ChallengeMap");
    assert_eq!(
        registry.dispatch(MenuAction::Select { entry: map.clone() }, &ctx),
        Outcome::Ignored
    );

    let count = Arc::new(AtomicUsize::new(0));
    registry.register_handler(map.clone(), engine_handler("maps", count.clone()));
    assert!(registry.enabled(&map, &ctx));
    assert_eq!(
        registry.dispatch(MenuAction::Select { entry: map.clone() }, &ctx),
        Outcome::Handled(Owner::Engine("maps".into()))
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    registry.remove_owner(&Owner::Engine("maps".into()));
    assert!(!registry.enabled(&map, &ctx));
}

#[test]
fn park_rules_and_dirty_park_confirmation() {
    let mut registry = MenuRegistry::new(fixture());
    let count = Arc::new(AtomicUsize::new(0));
    for item in ["ChallengeMap", "SkateWith", "SavePark"] {
        registry.register_handler(EntryRef::item(item), engine_handler("f", count.clone()));
    }
    let career = MenuContext::new("Career");
    let park = MenuContext::new("CareerPark");
    assert!(registry.enabled(&EntryRef::item("SkateWith"), &career));
    assert!(
        !registry.enabled(&EntryRef::item("SkateWith"), &park),
        "SkateWith is off in a park"
    );
    assert!(!registry.enabled(&EntryRef::item("SavePark"), &park));
    assert!(registry.enabled(
        &EntryRef::item("SavePark"),
        &park.clone().with(facts::PARK_SAVEABLE, 1.0)
    ));

    let map = EntryRef::item("ChallengeMap");
    let dirty = park.clone().with(facts::PARK_DIRTY, 1.0);
    let Outcome::Confirm(popup) =
        registry.dispatch(MenuAction::Select { entry: map.clone() }, &dirty)
    else {
        panic!("expected a popup")
    };
    assert_eq!(popup.title, "ID_SKATEPARK_DIRTY_TITLE");
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "the handler waits for the answer"
    );
    assert_eq!(
        registry.dispatch(
            MenuAction::Confirm {
                entry: map.clone(),
                accepted: false
            },
            &dirty
        ),
        Outcome::Declined
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    registry.dispatch(
        MenuAction::Confirm {
            entry: map.clone(),
            accepted: true,
        },
        &dirty,
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    // A clean park or Career selects at once.
    registry.dispatch(MenuAction::Select { entry: map.clone() }, &park);
    registry.dispatch(
        MenuAction::Select { entry: map },
        &career.clone().with(facts::PARK_DIRTY, 1.0),
    );
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn hidden_rules_project10_and_sfx_pack() {
    let mut registry = MenuRegistry::new(fixture());
    let ctx = MenuContext::new("Career");
    let ids = |r: &MenuRegistry, c: &MenuContext| {
        r.tab_rows("Create", c)
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&registry, &ctx), ["ReplayEditor"]);
    assert_eq!(
        ids(&registry, &ctx.clone().with(facts::PROJECT10_UNLOCKED, 1.0)),
        ["ReplayEditor", "Project10"]
    );
    let audio = |r: &MenuRegistry, c: &MenuContext| {
        r.screen_rows(3, c)
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(audio(&registry, &ctx), ["ID_GAMESETTINGS_SFXVOLUME"]);
    assert_eq!(
        audio(&registry, &ctx.clone().with(facts::SFX_PACKS, 2.0)).len(),
        2
    );
    // A mod can override a retail rule; disabling the mod restores retail's.
    let owner = Owner::Mod("m".into());
    registry.set_visible(
        EntryRef::item("Project10"),
        owner.clone(),
        Condition::Rule(Rule::Always),
    );
    assert_eq!(ids(&registry, &ctx), ["ReplayEditor", "Project10"]);
    registry.remove_owner(&owner);
    assert_eq!(ids(&registry, &ctx), ["ReplayEditor"]);
}

#[test]
fn settings_links_follow_their_screens_and_values_bind() {
    let mut registry = MenuRegistry::new(fixture());
    let ctx = MenuContext::new("Career");
    let difficulty = EntryRef::setting("ID_GAMESETTINGS_DIFFICULTY_SETTINGS");
    assert!(
        !registry.enabled(&difficulty, &ctx),
        "a link to a screen of greyed rows is greyed"
    );
    let value = Arc::new(Mutex::new(SettingValue::Int(1)));
    let (get_v, set_v) = (value.clone(), value.clone());
    registry.bind_value(
        EntryRef::setting("ID_GAMESETTINGS_DIFFICULTY_OPTIONS"),
        Owner::Engine("difficulty".into()),
        ValueHook::Native {
            get: Arc::new(move || get_v.lock().unwrap().clone()),
            set: Arc::new(move |v| *set_v.lock().unwrap() = v.clone()),
        },
    );
    assert!(registry.enabled(&difficulty, &ctx));
    assert_eq!(
        registry.dispatch(
            MenuAction::Select {
                entry: difficulty.clone()
            },
            &ctx
        ),
        Outcome::OpenScreen(2)
    );
    assert!(
        !registry.enabled(&difficulty, &ctx.clone().with(facts::GAME_MODE, 4.0)),
        "retail greys Difficulty online"
    );
    let options = EntryRef::setting("ID_GAMESETTINGS_DIFFICULTY_OPTIONS");
    registry.dispatch(
        MenuAction::SetValue {
            entry: options.clone(),
            value: SettingValue::Int(2),
        },
        &ctx,
    );
    assert_eq!(registry.value(&options), Some(SettingValue::Int(2)));
    assert_eq!(*value.lock().unwrap(), SettingValue::Int(2));
}

#[test]
fn video_link_chooses_the_retail_screen() {
    let registry = MenuRegistry::new(fixture());
    let target = |ctx: &MenuContext| registry.link_target("ID_GAMESETTINGS_VIDEO_SETTINGS", ctx);
    assert_eq!(target(&MenuContext::new("x")), None, "not in the fixture");
    let mut data = fixture();
    data.settings.push(Setting {
        id: "ID_GAMESETTINGS_VIDEO_SETTINGS".into(),
        label: String::new(),
        widget: Widget::Option,
        link: retail_link("ID_GAMESETTINGS_VIDEO_SETTINGS"),
    });
    let registry = MenuRegistry::new(data);
    let target = |ctx: MenuContext| registry.link_target("ID_GAMESETTINGS_VIDEO_SETTINGS", &ctx);
    assert_eq!(
        target(MenuContext::new("x").with(facts::FE_STATE, 2.0)),
        Some(8)
    );
    assert_eq!(
        target(MenuContext::new("x").with(facts::GAME_MODE, 3.0)),
        Some(6)
    );
    assert_eq!(
        target(
            MenuContext::new("x")
                .with(facts::GAME_MODE, 3.0)
                .with(facts::ONLINE_SESSION, 1.0)
        ),
        Some(7)
    );
    assert_eq!(
        target(
            MenuContext::new("x")
                .with(facts::GAME_MODE, 1.0)
                .with(facts::ONLINE_SESSION, 1.0)
        ),
        Some(5)
    );
    assert_eq!(
        target(MenuContext::new("x").with(facts::GAME_MODE, 1.0)),
        Some(4)
    );
}

#[test]
fn mod_edits_add_hide_reorder_relabel_and_revert() {
    let mut registry = MenuRegistry::new(fixture());
    let retail = registry.data().clone();
    let owner = Owner::Mod("m".into());
    let ctx = MenuContext::new("Career");
    let place = |category: &str, position| {
        Some(Placement {
            modes: vec![],
            category: Some(category.into()),
            position,
        })
    };
    registry
        .edit(
            owner.clone(),
            Edit::Item {
                id: "m.Races".into(),
                label: Some("Races".into()),
                icon: Some(Icon::Image("icons/races.png".into())),
                help: None,
                place: place("SinglePlayer", Some(0)),
            },
        )
        .unwrap();
    registry
        .edit(
            owner.clone(),
            Edit::Item {
                id: "GameSettings".into(),
                label: Some("Settings!".into()),
                icon: None,
                help: None,
                place: None,
            },
        )
        .unwrap();
    registry
        .edit(
            owner.clone(),
            Edit::Item {
                id: "SkateWith".into(),
                label: None,
                icon: None,
                help: None,
                place: Some(Placement {
                    position: Some(0),
                    ..Default::default()
                }),
            },
        )
        .unwrap();
    registry
        .edit(
            owner.clone(),
            Edit::Hide {
                entry: EntryRef::item("ReplayEditor"),
                modes: vec!["Career".into()],
            },
        )
        .unwrap();
    registry
        .edit(
            owner.clone(),
            Edit::Category {
                id: "m.Tab".into(),
                label: Some("Mods".into()),
                icon: None,
                place: Some(Placement {
                    modes: vec!["Career".into()],
                    category: None,
                    position: Some(1),
                }),
            },
        )
        .unwrap();
    registry
        .edit(
            owner.clone(),
            Edit::Setting {
                id: "m.Volume".into(),
                label: Some("Bass".into()),
                widget: Some(Widget::Slider),
                link: None,
                place: Some((3, None)),
            },
        )
        .unwrap();
    assert!(
        registry
            .edit(
                owner.clone(),
                Edit::Item {
                    id: "m.Bad".into(),
                    label: None,
                    icon: None,
                    help: None,
                    place: None
                }
            )
            .is_err(),
        "a new item needs a tab"
    );

    let ids = |c: &str| {
        registry
            .tab_rows(c, &ctx)
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids("SinglePlayer"),
        ["SkateWith", "m.Races", "ChallengeMap"]
    );
    assert!(ids("Create").iter().all(|i| i != "ReplayEditor"));
    assert_eq!(
        registry
            .tab_rows("Create", &MenuContext::new("CareerPark"))
            .len(),
        2,
        "hidden only in Career"
    );
    assert_eq!(
        registry.tabs(&ctx),
        ["SinglePlayer", "m.Tab", "Create", "Options"]
    );
    assert_eq!(
        registry.data().item("GameSettings").unwrap().label,
        "Settings!"
    );
    assert!(
        registry
            .screen_rows(3, &ctx)
            .iter()
            .any(|r| r.id == "m.Volume")
    );

    // The mod's handler enables its item; events go to the mod.
    let races = EntryRef::item("m.Races");
    assert!(!registry.enabled(&races, &ctx));
    registry.register_handler(races.clone(), mod_handler("m"));
    let Outcome::Forward(to, event) = registry.dispatch(
        MenuAction::Select {
            entry: races.clone(),
        },
        &ctx,
    ) else {
        panic!()
    };
    assert_eq!((to, event.mode.as_str()), (owner.clone(), "Career"));
    // A mod-bound value is stored and the change forwarded.
    let volume = EntryRef::setting("m.Volume");
    registry.bind_value(
        volume.clone(),
        owner.clone(),
        ValueHook::Stored(SettingValue::Float(0.5)),
    );
    assert!(matches!(
        registry.dispatch(
            MenuAction::SetValue {
                entry: volume.clone(),
                value: SettingValue::Float(0.7)
            },
            &ctx
        ),
        Outcome::Forward(..)
    ));
    assert_eq!(registry.value(&volume), Some(SettingValue::Float(0.7)));

    registry.remove_owner(&owner);
    assert_eq!(
        *registry.data(),
        retail,
        "disable reverts to the retail structure"
    );
    assert!(!registry.enabled(&races, &ctx));
    assert_eq!(registry.value(&volume), None);
}

#[test]
fn later_owners_win_and_earlier_layers_survive_removal() {
    let mut registry = MenuRegistry::new(fixture());
    let (a, b) = (Owner::Mod("a".into()), Owner::Mod("b".into()));
    let relabel = |label: &str| Edit::Item {
        id: "GameSettings".into(),
        label: Some(label.into()),
        icon: None,
        help: None,
        place: None,
    };
    registry.edit(a.clone(), relabel("A")).unwrap();
    registry.edit(b.clone(), relabel("B")).unwrap();
    assert_eq!(registry.data().item("GameSettings").unwrap().label, "B");
    registry.remove_owner(&b);
    assert_eq!(registry.data().item("GameSettings").unwrap().label, "A");
    let map = EntryRef::item("ChallengeMap");
    registry.register_handler(map.clone(), mod_handler("a"));
    registry.register_handler(map.clone(), mod_handler("b"));
    let ctx = MenuContext::new("Career");
    assert!(
        matches!(registry.dispatch(MenuAction::Select { entry: map.clone() }, &ctx), Outcome::Forward(Owner::Mod(ref m), _) if m == "b")
    );
    registry.remove_owner(&b);
    assert!(
        matches!(registry.dispatch(MenuAction::Select { entry: map }, &ctx), Outcome::Forward(Owner::Mod(ref m), _) if m == "a")
    );
}

#[test]
fn rules_evaluate() {
    let ctx = MenuContext::new("FreePlay").with("x", 2.0);
    assert!(Rule::fact("x", Cmp::Ge, 2.0).eval(&ctx));
    assert!(!Rule::fact("missing", Cmp::Ne, 0.0).eval(&ctx));
    assert!(Rule::Mode(vec!["FreePlay".into()]).eval(&ctx));
    assert!(Rule::Not(Box::new(Rule::Never)).eval(&ctx));
    assert!(!Rule::All(vec![Rule::Always, Rule::Never]).eval(&ctx));
    assert_eq!(setting_key(0, "0"), "GAMESETTINGS_ROW_0");
    assert_eq!(
        setting_key(7, "ID_GAMESETTINGS_SFXVOLUME"),
        "ID_GAMESETTINGS_SFXVOLUME"
    );
}

#[test]
fn step_uses_the_retail_rule_and_engine_binding_round_trips() {
    let mut r = MenuRegistry::new(fixture());
    let ctx = MenuContext::new("Career");
    let sfx = EntryRef::setting("ID_GAMESETTINGS_SFXVOLUME");
    let play = EntryRef::setting("ID_GAMESETTINGS_DIFFICULTY_OPTIONS");
    let engine = Owner::Engine("audio".into());
    // Unbound rows are greyed and Left / Right does nothing.
    assert_eq!(
        r.dispatch(
            MenuAction::Step {
                entry: sfx.clone(),
                direction: Direction::Right
            },
            &ctx
        ),
        Outcome::Ignored
    );
    r.bind_value(
        sfx.clone(),
        engine.clone(),
        ValueHook::Stored(SettingValue::Float(0.75)),
    );
    r.bind_value(
        play.clone(),
        Owner::Engine("difficulty".into()),
        ValueHook::Stored(SettingValue::Int(2)),
    );
    assert!(r.enabled(&sfx, &ctx));
    let out = r.dispatch(
        MenuAction::Step {
            entry: sfx.clone(),
            direction: Direction::Right,
        },
        &ctx,
    );
    assert!(matches!(
        out,
        Outcome::Forward(
            Owner::Engine(_),
            MenuEvent {
                action: MenuAction::SetValue { .. },
                ..
            }
        )
    ));
    assert!((crate::menu_values::number(&r.value(&sfx).unwrap()) - 0.85).abs() < 1e-6);
    assert_eq!(r.display(&sfx), Some(Display::Bars(9)));
    r.dispatch(
        MenuAction::Step {
            entry: play.clone(),
            direction: Direction::Right,
        },
        &ctx,
    );
    assert_eq!(
        r.value(&play),
        Some(SettingValue::Int(0)),
        "Play Mode wraps Hardcore -> Easy"
    );
    // The engine's own source changed (settings file): the binding follows without an event.
    r.set_owner_value(&sfx, &engine, SettingValue::Float(0.3));
    assert_eq!(r.value(&sfx), Some(SettingValue::Float(0.3)));
    // Online, the Difficulty link is greyed but the rule itself is unaffected.
    assert!(!r.enabled(
        &EntryRef::setting("ID_GAMESETTINGS_DIFFICULTY_SETTINGS"),
        &MenuContext::new("Career").with(facts::GAME_MODE, 4.0)
    ));
}

#[test]
fn mod_rule_and_value_override_revert_on_disable() {
    let mut r = MenuRegistry::new(fixture());
    let ctx = MenuContext::new("Career");
    let sfx = EntryRef::setting("ID_GAMESETTINGS_SFXVOLUME");
    let engine = Owner::Engine("audio".into());
    let m = Owner::Mod("m".into());
    r.bind_value(
        sfx.clone(),
        engine.clone(),
        ValueHook::Stored(SettingValue::Float(0.5)),
    );
    r.bind_value(
        sfx.clone(),
        m.clone(),
        ValueHook::Stored(SettingValue::Float(0.5)),
    );
    r.set_rule(
        sfx.clone(),
        m.clone(),
        ValueRule::Slider {
            min: 0.0,
            max: 2.0,
            step: 0.25,
            snap: 0.0,
            bars: 8.0,
        },
    );
    let out = r.dispatch(
        MenuAction::Step {
            entry: sfx.clone(),
            direction: Direction::Right,
        },
        &ctx,
    );
    assert!(matches!(out, Outcome::Forward(Owner::Mod(_), _)));
    assert_eq!(r.value(&sfx), Some(SettingValue::Float(0.75)));
    assert_eq!(
        r.owner_value(&sfx, &engine),
        Some(SettingValue::Float(0.5)),
        "the engine value is untouched under the override"
    );
    r.remove_owner(&m);
    assert_eq!(r.rule(&sfx), Some(&ValueRule::retail_slider()));
    assert_eq!(r.value(&sfx), Some(SettingValue::Float(0.5)));
    // The SFX pack row has no decoded rule: a mod can give it one.
    assert!(
        r.rule(&EntryRef::setting("ID_GAMESETTINGS_SFXPACK"))
            .is_none()
    );
}
