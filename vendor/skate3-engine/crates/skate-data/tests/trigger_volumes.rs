use skate_data::skate_map::{Extension, SkateMap};
use skate_data::trigger_volumes::{Origin, TriggerGroup, load_for_map, parse};
use std::path::{Path, PathBuf};

const RESET: &str = r#"{
 "format": "skate3rust-trigger-volumes", "version": 1, "map": "SkateSchool",
 "volumes": [
  {"id": "2c7017060025128f", "name": "tut_sksc_reset_vol01", "instance_id": "2c7017060025128f",
   "link_guid": "a2456e5cb2b3f6ae", "group": "challenge",
   "shape": {"kind": "box", "center": [183.023, 2.994, 4.522],
             "axes": [[0, 0, 1], [0, 1, 0], [1, 0, 0]], "half_extents": [487.281, 76.021, 394.996], "fatness": 0},
   "aabb": {"min": [-211.973, -73.027, -482.759], "max": [578.019, 79.014, 491.803]},
   "stream": "cSim_Global.xsf", "arena": "a255bfd8fda03a28"}
 ]}"#;

fn custom(id: &str, group: &str) -> String {
    format!(
        r#"{{"format":"skate3rust-trigger-volumes","version":1,"volumes":[
        {{"id":"{id}","name":"start_gate","group":"{group}","shape":{{"kind":"box","center":[0,1,0],"half_extents":[2,1,2]}}}}]}}"#
    )
}

#[test]
fn retail_record_round_trips() {
    let file = parse(RESET.as_bytes()).unwrap();
    let v = &file.volumes[0];
    assert_eq!(v.instance(), Some(0x2C70_1706_0025_128F));
    assert_eq!(v.link(), Some(0xA245_6E5C_B2B3_F6AE));
    assert_eq!(v.group, TriggerGroup::Challenge);
    let b = v.shape.oriented_box();
    assert!(b.contains_point([0., 0., 0.]));
    assert!(!b.contains_point([0., 100., 0.]));
    let (lo, hi) = b.aabb();
    let bounds = v.aabb.as_ref().unwrap();
    for i in 0..3 {
        assert!((lo[i] - bounds.min[i]).abs() < 0.01 && (hi[i] - bounds.max[i]).abs() < 0.01);
    }
}

#[test]
fn custom_maps_can_write_minimal_boxes() {
    let file = parse(custom("gate", "stairs").as_bytes()).unwrap();
    assert_eq!(file.volumes[0].group, TriggerGroup::Stairs);
    assert_eq!(
        file.volumes[0].shape.oriented_box().axes,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
    );
}

#[test]
fn invalid_documents_are_rejected() {
    assert!(
        parse(RESET.replace("\"version\": 1", "\"version\": 2").as_bytes())
            .unwrap_err()
            .contains("version 2")
    );
    assert!(
        parse(
            RESET
                .replace("skate3rust-trigger-volumes", "other")
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        parse(RESET.replace("a2456e5cb2b3f6ae", "nothex").as_bytes())
            .unwrap_err()
            .contains("link_guid")
    );
    assert!(
        parse(RESET.replace("[[0, 0, 1]", "[[0, 0, 2]").as_bytes())
            .unwrap_err()
            .contains("invalid box")
    );
    assert!(parse(custom("gate", "lobby").as_bytes()).is_err());
    assert!(parse(custom("", "camera").as_bytes()).is_err());
    let mut twice = parse(RESET.as_bytes()).unwrap();
    twice.volumes.push(twice.volumes[0].clone());
    assert!(
        parse(&serde_json::to_vec(&twice).unwrap())
            .unwrap_err()
            .contains("duplicate")
    );
}

fn demo() -> SkateMap {
    SkateMap::parse(include_bytes!("../../../maps/format-demo.skate")).unwrap()
}

#[test]
fn sidecar_wins_over_embedded_extension_and_absence_is_empty() {
    let dir = std::env::temp_dir().join(format!("skate-triggers-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let map_path = dir.join("Custom.skate");
    let mut map = demo();
    assert_eq!(
        load_for_map(Some(&map_path), Some(&map)).unwrap(),
        (vec![], Origin::None)
    );
    map.extensions.push(Extension {
        tag: *b"TVOL",
        schema: 1,
        payload: custom("embedded", "camera").into_bytes(),
    });
    let (volumes, origin) = load_for_map(Some(&map_path), Some(&map)).unwrap();
    assert_eq!(
        (volumes[0].id.as_str(), origin),
        ("embedded", Origin::Embedded)
    );
    std::fs::write(dir.join("Custom.triggers"), custom("sidecar", "challenge")).unwrap();
    let (volumes, origin) = load_for_map(Some(&map_path), Some(&map)).unwrap();
    assert_eq!(volumes[0].id, "sidecar");
    assert_eq!(origin, Origin::Sidecar(dir.join("Custom.triggers")));
    map.extensions.last_mut().unwrap().schema = 2;
    std::fs::remove_file(dir.join("Custom.triggers")).unwrap();
    assert!(load_for_map(Some(&map_path), Some(&map)).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

/// Research (notes triggers-volumes-re.md §1, = PR #25's surfaceless boxes): 11 world-stream volumes.
const RETAIL: &[(&str, &[&str])] = &[
    (
        "SkateSchool",
        &[
            "coach_frank_sksc",
            "ws_sksc_coachfrank_instance_01",
            "tut_sksc_inthehub_vol_01",
            "tut_sksc_inthehub_vol_02",
            "tut_sksc_reset_vol01",
            "tut_sksc_wrongway_vol_01",
        ],
    ),
    ("MegaPark", &["tele_stadium_to_world_volume_a"]),
    (
        "MaloofMoneyCup",
        &["tele_mega_ramp_up_volume_01", "tele_mmcp_to_dwtn_volume_01"],
    ),
    (
        "DownTown",
        &[
            "dwtn_sessionspot_01_kubetower_volume_01",
            "dwtn_sessionspot_02_spillway_volume_02",
        ],
    ),
    ("University", &[]),
    ("Industrial", &[]),
    ("StartPark", &[]),
    ("BlackBoxPark", &[]),
    ("DownTownSkatePark", &[]),
    ("IndustrialSkatePark", &[]),
];

fn converted_maps() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("SKATE3_MAPS_DIR")?);
    dir.join("SkateSchool.triggers").is_file().then_some(dir)
}

/// Data-gated: `SKATE3_MAPS_DIR` = a converted install's `maps` directory (or a
/// staging directory from `python -m tools.asset_pipeline.map_volumes`).
#[test]
fn converted_retail_maps_carry_the_research_volumes() {
    let Some(dir) = converted_maps() else {
        eprintln!("skipped: set SKATE3_MAPS_DIR to a maps directory with .triggers sidecars");
        return;
    };
    let mut total = 0;
    for (map, names) in RETAIL {
        let skate = dir.join(format!("{map}.skate"));
        let (volumes, origin) = load_for_map(Some(Path::new(&skate)), None).unwrap();
        assert_eq!(
            origin,
            Origin::Sidecar(dir.join(format!("{map}.triggers"))),
            "{map}"
        );
        assert_eq!(
            volumes.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
            *names,
            "{map}"
        );
        for v in &volumes {
            assert_eq!(v.group, TriggerGroup::Challenge);
            assert_eq!(
                v.instance().map(|i| i >> 32),
                Some(0x2C70_1706),
                "{map} {}",
                v.name
            );
            let (lo, hi) = v.shape.oriented_box().aabb();
            let b = v.aabb.as_ref().unwrap();
            for i in 0..3 {
                assert!(
                    (lo[i] - b.min[i]).abs() < 0.25 && (hi[i] - b.max[i]).abs() < 0.25,
                    "{map} {}",
                    v.name
                );
            }
        }
        total += volumes.len();
    }
    assert_eq!(total, 11);
}
