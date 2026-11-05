//! TU3 AirDismounting Begin82BB93E0, Update82BB94C0, empty End82B61BB8.
use skate3_core::animation::{output::attributes::AnimationAttribute, skeleton_input::name::encode};

#[derive(Default)]
pub(super) struct State {
    frames: i32,
}
impl State {
    pub(super) fn begin(&mut self, length: f32, time: f32) -> Result<(), String> {
        let frames = (length - time) * 59.999996_f32;
        // Keep malformed animation timing diagnosable instead of silently
        // converting NaN/infinity to an integer. This does not repair physics.
        if !frames.is_finite() || frames >= 2_147_483_648.0 {
            return Err(format!("AirDismounting invalid animation timing: length={length} time={time}"));
        }
        // Native fsel clamps to one, then fctiwz truncates. It captures once
        // on Begin; this is neither a countdown nor based on the render dt.
        self.frames = frames.max(1.0) as i32;
        Ok(())
    }
    pub(super) fn request(&self, attributes: &[AnimationAttribute]) -> Option<i32> {
        // Animator v96 is the collected-tree membership query. Payload/status
        // do not gate this event, and an absent marker leaves the count alone.
        attributes.iter().any(|a| a.name == encode(b"airdismountrevert"))
            .then_some(self.frames)
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    use skate3_core::animation::output::attributes::AttributePayload;
    fn marker(name: &[u8]) -> AnimationAttribute {
        AnimationAttribute { name: encode(name), payload: AttributePayload([Some(0); 6]),
            begin_time: 0.0, end_time: 0.0, status: 0, kind: 0, sequence_id: -1 }
    }
    #[test]
    fn air_dismounting_marker_uses_captured_duration_and_reentry_recaptures() {
        let mut state = State::default();
        state.begin(1.0, 0.25).unwrap();
        assert_eq!(state.request(&[]), None);
        assert_eq!(state.request(&[marker(b"manualInto")]), None);
        let event = [marker(b"airdismountrevert")];
        assert_eq!(state.request(&event), Some(44));
        assert_eq!(state.request(&event), Some(44));
        state.begin(1.0, 0.75).unwrap();
        assert_eq!(state.request(&event), Some(14));
        let other = State::default();
        assert_eq!(state.request(&event), Some(14));
        assert_eq!(other.request(&[]), None);
    }
    #[test]
    fn air_dismounting_clamps_short_or_finished_clips_and_rejects_invalid_timing() {
        let mut state = State::default();
        for time in [0.999, 1.0, 2.0] {
            state.begin(1.0, time).unwrap();
            assert_eq!(state.request(&[marker(b"airdismountrevert")]), Some(1));
        }
        for (length, time) in [(f32::NAN, 0.0), (1.0, f32::INFINITY), (f32::MAX, 0.0)] {
            assert!(state.begin(length, time).is_err());
        }
    }
}
