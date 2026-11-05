use super::{Settings, curves, metrics};
use skate_data::animation_metadata::{ClipAttribute, ClipMetadata};
fn clip() -> ClipMetadata {
    ClipMetadata {
        name: "NB_WALK_FWD_CYC".into(),
        source_offset: 48,
        fps_bits: 60.0f32.to_bits(),
        frames_bits: 57.0f32.to_bits(),
        base_speed_bits: 3.0f32.to_bits(),
        flags_word: 0,
        attributes: vec![ClipAttribute {
            name: "ANIMTRANSZ".into(),
            type_id: 0,
            begin_bits: (-1.0f32).to_bits(),
            end_bits: (-1.0f32).to_bits(),
            payload_words: vec![1.5f32.to_bits()],
            source_offset: 128,
        }],
    }
}
#[test]
fn query_uses_full_frame_duration_and_negative_untimed_end() {
    let metric = metrics::metric(&clip()).unwrap().unwrap();
    assert_eq!(metric.end_time, -0.95);
    assert_eq!(metric.translation_z, 1.5);
    assert!((-metric.translation_z / metric.end_time - 1.5789474).abs() < 1e-6);
}
#[test]
fn first_matching_record_wins_without_a_playback_status_filter() {
    let mut clip = clip();
    clip.attributes[0].begin_bits = 0.8f32.to_bits();
    clip.attributes[0].end_bits = 0.5f32.to_bits();
    let mut later = ClipAttribute {
        name: "ANIMTRANSZ".into(),
        type_id: 0,
        begin_bits: 0,
        end_bits: 1.0f32.to_bits(),
        payload_words: vec![99.0f32.to_bits()],
        source_offset: 192,
    };
    later.begin_bits = 0;
    clip.attributes.push(later);
    let metric = metrics::metric(&clip).unwrap().unwrap();
    assert_eq!(metric.translation_z, 1.5);
    assert_eq!(metric.end_time, 0.475);
}
#[test]
fn actual_attribute_absence_is_distinct_from_malformed_data() {
    let mut clip = clip();
    clip.attributes[0].type_id = 2;
    assert!(metrics::metric(&clip).is_err());
    clip.attributes.clear();
    assert!(metrics::metric(&clip).unwrap().is_none());
}
#[test]
fn graph_bounds_are_separate_and_duplicate_knots_preserve_endpoint_jump() {
    let values = [-9.0f32, -8.0, 7.0, 6.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0];
    let hex = values
        .iter()
        .map(|v| format!("{:08X}", v.to_bits()))
        .collect::<String>();
    let (g, bounds) = curves::parse::<3>(&hex).unwrap();
    assert_eq!(bounds, [-9.0, -8.0, 7.0, 6.0]);
    assert_eq!(g.evaluate(-0.01), 2.0);
    assert_eq!(g.evaluate(0.0), 3.0);
    assert_eq!(g.evaluate(0.5), 3.5);
    assert_eq!(g.evaluate(1.0), 4.0);
    assert!(curves::parse::<4>(&hex).is_err());
}
#[test]
#[ignore = "requires extracted private stock assets"]
fn actual_stock_settings_and_metrics_load_together() {
    let root =
        std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").expect("SKATE3_ASSET_ROOT"));
    let collections = skate_data::collections::Collections::load(&root).unwrap();
    let metadata = skate_data::animation_banks::AnimationBanks::load(&root)
        .unwrap()
        .metadata()
        .unwrap();
    let settings = Settings::load(&collections, &metadata).unwrap();
    assert!(settings.metrics.iter().all(Option::is_some));
    let speeds = settings.metrics.map(|m| {
        let m = m.unwrap();
        (-1.0 / m.end_time) * m.translation_z
    });
    assert!(speeds[0] > 0.0 && speeds[0] < speeds[1] && speeds[1] < speeds[2]);
    assert_eq!(settings.controller.movement_intent.sprint_time_cap, 4.0);
}
