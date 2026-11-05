use super::*;

#[test]
#[ignore = "requires a converted stock animation bank and collections (not in the repository)"]
fn stock_push_metrics_load_through_the_runtime_data_path() {
    let root = std::path::PathBuf::from(
        std::env::var_os("SKATE3_ASSET_ROOT").expect("Set SKATE3_ASSET_ROOT"),
    );
    let collections = Collections::load(&root).unwrap();
    let settings = PushingSettings::load_stock(&collections, &root).unwrap();
    for group in [&settings.regular, &settings.mongo] {
        for clip in group.clips {
            assert!(clip.length.is_finite() && clip.length > 0.0);
            assert!(clip.end_velocity > clip.begin_velocity);
        }
    }
    // Distinct authored timings must survive data conversion and dispatch.
    assert_ne!(
        settings.regular.clips[0].length,
        settings.mongo.clips[0].length
    );
    println!("regular={:?}\nmongo={:?}", settings.regular, settings.mongo);
}
