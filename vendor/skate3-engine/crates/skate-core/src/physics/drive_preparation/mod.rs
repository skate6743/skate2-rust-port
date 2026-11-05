//! Drive-frame preparation in TU3 Island::Step_Solver2 82763B08.
//! The active-drive list is visited before thaw/partition and island-table
//! construction. Every island mode runs this loop, regardless of body flags.

/// Normalize each nonnull frame record in native active-list order. Entries
/// are frame-record indices, not drive/body indices; duplicate references are
/// processed repeatedly just as live pointers are in 82763BB4..82763C50.
/// Null frames are skipped. Translations and all their packed lanes survive.
pub fn normalize_active_drive_frames(
    frames: &mut [[u32; 16]],
    active_frame_indices: &[Option<usize>],
) {
    for index in active_frame_indices.iter().flatten() {
        normalize_drive_frames(&mut frames[*index]);
    }
}

/// The two quaternion operations in the same stage. No zero-length or finite
/// fallback is supplied here.
pub fn normalize_drive_frames(frames: &mut [u32; 16]) {
    for offset in [0, 8] {
        let q: [f32; 4] = core::array::from_fn(|i| f32::from_bits(frames[offset + i]));
        let d = quaternion_squared_length(q);
        let mut e = super::reciprocal_sqrt::estimate(d);
        for _ in 0..2 {
            let squared = e * e;
            let half = e * 0.5;
            e = half.mul_add((-d).mul_add(squared, 1.0), e);
        }
        for i in 0..4 {
            frames[offset + i] = (q[i] * e).to_bits();
        }
    }
}

fn quaternion_squared_length(q: [f32; 4]) -> f32 {
    super::native_arithmetic::dot4(q, q)
}
