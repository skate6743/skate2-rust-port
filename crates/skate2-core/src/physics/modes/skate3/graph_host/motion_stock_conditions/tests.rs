use super::*;
use crate::skate3_native::graph_host::motion_animation::MotionAnimation;

#[test]
fn obstacle_distance_uses_named_clip_displacement_and_native_boundary() {
    assert!(!enough_distance(0.49, 0.2));
    assert!(enough_distance(0.5, 0.2));
    assert!(enough_distance(0.51, 0.2));
    assert!(!enough_distance(0.29, 0.0));
    assert!(enough_distance(0.3, 0.0));
    //Native fcmpu/blt; this leaf does not substitute a finite-value guard.
    assert!(enough_distance(f32::NAN, 0.2));
}

#[test]
fn stop_animation_query_uses_database_and_ignores_event_time_gate() {
    let metadata = skate_data::animation_metadata::AnimationMetadata::parse(r#"{
        "version":1,"source_bank":"OffBoard.abin",
        "source_sha256":"0000000000000000000000000000000000000000000000000000000000000000",
        "source_bytes":256,"clips":[{
            "name":"TEST_STOP","source_offset":48,"fps_bits":1114636288,
            "frames_bits":1110704128,"base_speed_bits":1065353216,"flags_word":0,
            "attributes":[{"name":"ANIMTRANSZ","type_id":0,
                "begin_bits":1056964608,"end_bits":1065353216,
                "payload_words":[1056964608],"source_offset":128}]
        }],"unsupported_trees":[]
    }"#).unwrap();
    let animation = MotionAnimation::from_metadata(metadata);
    assert_eq!(animation.stock_clip_translation_z("offboard", "test_stop").unwrap(), 0.5);
    assert_eq!(animation.stock_clip_translation_z("OnBoard", "TEST_STOP").unwrap(), 0.0);
    assert_eq!(animation.stock_clip_translation_z("OffBoard", "MISSING_STOP").unwrap(), 0.0);
}

#[test]
#[ignore = "requires private stock banks; named stop-clip predicate regression, not gameplay acceptance"]
fn stock_stop_clips_drive_obstacle_distance_predicate() {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT")
        .expect("Set SKATE3_ASSET_ROOT"));
    let banks = skate_data::animation_banks::AnimationBanks::load(&root).unwrap();
    let metadata = banks.metadata().unwrap();
    let animation = MotionAnimation::from_metadata(banks.metadata().unwrap());
    //Every authored EnoughDistToObstacle dependency in OffBoard/intostand.xml.
    for prefix in ["WALK", "RUN"] {
        for phase in [0, 25, 50, 75] {
            let name = format!("BR_{prefix}_FWD_{phase}_INTO_STAND_0");
            let clip = metadata.clip(&name).unwrap();
            let attribute = clip.attributes.iter().find(|a| a.name.eq_ignore_ascii_case("AnimTransZ"))
                .expect("Authored stopping displacement");
            assert_eq!(attribute.type_id, 0, "{name}");
            let displacement = f32::from_bits(attribute.payload_words[0]);
            assert_eq!(animation.stock_clip_translation_z("OffBoard", &name).unwrap(), displacement);
            let boundary = displacement + f32::from_bits(0x3e99_999a);
            assert!(enough_distance(boundary, displacement), "{name}");
            assert!(!enough_distance(boundary - 0.001, displacement), "{name}");
        }
    }
}
