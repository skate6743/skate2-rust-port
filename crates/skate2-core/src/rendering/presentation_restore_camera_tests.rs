use super::*;

fn frame(yaw: f32, position: [f32; 3], fov: f32) -> CameraFrame {
    let (sine, cosine) = yaw.sin_cos();
    let matrix = [
        cosine,
        0.0,
        -sine,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        sine,
        0.0,
        cosine,
        0.0,
        position[0],
        position[1],
        position[2],
        1.0,
    ];
    CameraFrame {
        matrix: SkeletonRootMatrix::known(matrix),
        previous_matrix: SkeletonRootMatrix::known(matrix),
        field_of_view: fov,
        ..CameraFrame::constructed()
    }
}

fn lens() -> CameraLens {
    CameraLens {
        near_plane: 0.2,
        far_plane: 7700.0,
        override_fov: None,
        update_counter: 1,
    }
}

fn game(frame: CameraFrame) -> CameraSubmission {
    CameraSubmission {
        frames: vec![frame],
        lens: lens(),
    }
}

#[test]
fn halfway_blend_turns_and_moves_halfway() {
    let a = frame(0.0, [0.0, 2.0, 0.0], 60.0);
    let b = frame(0.04, [1.0, 2.0, 0.5], 70.0);
    let middle = CameraFrame::interpolated(&a, &b, 0.5);
    let columns = middle.matrix.columns();
    assert!((columns[3].xyz[0] - 0.5).abs() < 1e-6);
    assert!((columns[3].xyz[2] - 0.25).abs() < 1e-6);
    assert_eq!(columns[3].w, Some(1.0));
    assert!((columns[2].xyz[0] - 0.02_f32.sin()).abs() < 1e-5);
    assert!((middle.field_of_view - 65.0).abs() < 1e-5);
}

#[test]
fn restore_blends_until_a_hard_cut() {
    let mut restore = CameraRestore::new();
    let a = game(frame(0.0, [0.0; 3], 60.0));
    let b = game(frame(0.0, [2.0, 0.0, 0.0], 60.0));
    restore.restore(&b, Some(&a), 0.25, true);
    let view = restore.view().expect("restored view");
    assert!((view.position[0] - 0.5).abs() < 1e-6);
    let mut cut = frame(0.0, [8.0, 0.0, 0.0], 60.0);
    cut.hard_cut = true;
    restore.restore(&game(cut), Some(&b), 0.25, true);
    assert!((restore.view().expect("cut view").position[0] - 8.0).abs() < 1e-5);
}

#[test]
fn a_collision_nudge_flagged_as_a_cut_blends_with_the_frame() {
    let mut restore = CameraRestore::new();
    let a = game(frame(0.0, [0.0; 3], 60.0));
    let mut nudged = frame(0.0, [0.3, 0.0, 0.0], 60.0);
    nudged.hard_cut = true;
    restore.restore(&game(nudged), Some(&a), 0.5, true);
    assert!((restore.view().expect("blended view").position[0] - 0.15).abs() < 1e-5);
}

#[test]
fn a_changed_camera_count_keeps_the_last_blend() {
    let mut restore = CameraRestore::new();
    let a = game(frame(0.0, [0.0; 3], 60.0));
    let b = game(frame(0.0, [2.0, 0.0, 0.0], 60.0));
    restore.restore(&b, Some(&a), 1.0, true);
    let two = CameraSubmission {
        frames: vec![frame(0.0, [9.0, 0.0, 0.0], 60.0); 2],
        lens: lens(),
    };
    restore.restore(&two, Some(&b), 0.5, true);
    assert!((restore.view().expect("kept view").position[0] - 2.0).abs() < 1e-5);
}
