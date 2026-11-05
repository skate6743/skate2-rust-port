//! Owned implementation of `Sk8::Behaviours::HippyJumpAntic::Update`.
//!
//! Skate 3 TU3 IDA verifies the complete producer path: factory `0x82F8A340`
//! registers constructor `0x82BCBF60`; vtable `0x82321414` creates a private
//! elapsed-time record in `0x82BBCB50` and dispatches Update `0x82BBE900`.
//! Update advances that record by exactly 1/60 second, evaluates
//! `anim_motion/hippy_flip/antic_length_to_hippy_height`, and writes the
//! constructor-bound `TrickHeight` attribute. Skate 2 `0x82CA80F8` is only
//! corroborating evidence for the same observable algorithm.

use skate3_core::animation::skeleton_input::name::encode;
use skate3_core::point_graph::PointGraph;
use skate_data::collections::Collections;

use super::motion_animation::MotionAnimation;
use skate3_core::animation::playback_parameters::{AttributeSink, SettableAttribute};

pub(super) struct Settings {
    pub antic_length_to_hippy_height: PointGraph<8>,
}

impl Settings {
    pub(super) fn load(data: &Collections) -> Result<Self, String> {
        let words =
            data.words::<20>("anim_motion", "hippy_flip", "antic_length_to_hippy_height")?;
        Ok(Self {
            antic_length_to_hippy_height: PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
                y: std::array::from_fn(|i| f32::from_bits(words[12 + i])),
            },
        })
    }
}

#[derive(Default)]
pub(super) struct State {
    active: bool,
    elapsed: f32,
}

impl State {
    pub(super) fn begin(&mut self) {
        self.active = true;
        self.elapsed = 0.0;
    }

    pub(super) fn update(&mut self, animation: &mut MotionAnimation, settings: &Settings) {
        debug_assert!(self.active);
        self.elapsed += 1.0 / 60.0;
        animation.set_attribute(SettableAttribute {
            name: encode(b"TrickHeight"),
            value: settings.antic_length_to_hippy_height.evaluate(self.elapsed),
            normalized: false,
            sequence_id: -1,
        });
    }

    pub(super) fn end(&mut self) {
        self.active = false;
        self.elapsed = 0.0;
    }
}

#[cfg(any())]
mod tests {
    use super::*;

    fn animation() -> MotionAnimation {
        MotionAnimation::from_metadata(
            skate_data::animation_metadata::AnimationMetadata::parse(
                r#"{"version":1,"source_bank":"test-only","source_sha256":"0000000000000000000000000000000000000000000000000000000000000000","source_bytes":48,"clips":[],"unsupported_trees":[]}"#,
            )
            .unwrap(),
        )
    }

    #[test]
    fn tu3_update_uses_fixed_tick_and_exact_attribute_case() {
        let settings = Settings {
            antic_length_to_hippy_height: PointGraph {
                x: std::array::from_fn(|i| i as f32),
                y: std::array::from_fn(|i| i as f32 * 2.0),
            },
        };
        let mut state = State::default();
        let mut animation = animation();

        state.begin();
        state.update(&mut animation, &settings);

        assert_eq!(state.elapsed.to_bits(), (1.0f32 / 60.0).to_bits());
        let written = animation
            .pending_parameter(encode(b"TrickHeight"))
            .expect("TU3 HippyJumpAntic must publish TrickHeight");
        assert!((written - 2.0 / 60.0).abs() < 1.0e-6);
        // Native encoding823C3C78 folds ASCII case, so this spelling aliases
        // the constructor's literal rather than identifying another output.
        assert_eq!(encode(b"TrickHeight"), encode(b"trickheight"));
    }
}
