use super::*;

#[test]
#[ignore = "requires converted stock camera and graph assets (not in the repository)"]
fn private_stock_camera_loads_every_graph_operation_and_shot_dependency() {
    let root = std::env::var_os("SKATE3_ASSET_ROOT").expect("set SKATE3_ASSET_ROOT");
    let root = std::path::Path::new(&root);
    let camera = CameraRuntime::load(root).expect("complete normal stock camera assets");
    assert!(camera.frame.is_none());
}

/// Minimizing the window makes the caller divide 0.0 by 0.0, and a stored NaN
/// aspect ratio produces a NaN field of view that keeps failing the non-finite
/// frame check after the window is restored.
#[test]
#[ignore = "requires converted stock camera and graph assets (not in the repository)"]
fn a_minimized_window_cannot_poison_the_aspect_ratio() {
    let root = std::env::var_os("SKATE3_ASSET_ROOT").expect("set SKATE3_ASSET_ROOT");
    let mut camera =
        CameraRuntime::load(std::path::Path::new(&root)).expect("stock camera assets");
    camera.set_aspect_ratio(16.0 / 9.0);
    let good = camera.manager.state.aspect_ratio;
    for degenerate in [f32::NAN, 0.0, -1.5, f32::INFINITY] {
        camera.set_aspect_ratio(degenerate);
        assert_eq!(
            camera.manager.state.aspect_ratio, good,
            "{degenerate} should have been rejected"
        );
    }
    camera.set_aspect_ratio(4.0 / 3.0);
    assert_eq!(camera.manager.state.aspect_ratio, 4.0 / 3.0);
}
