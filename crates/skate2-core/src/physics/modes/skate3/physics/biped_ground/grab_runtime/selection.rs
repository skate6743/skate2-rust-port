//! Ground mode0 consumer of the original core82D4D150 implementation.
use skate3_core::player::offboard::grab_scene::{Record, best_spline};
pub(super) fn best(records: &[Record], position: [f32; 4]) -> Option<Record> {
    best_spline(records, position)
}
