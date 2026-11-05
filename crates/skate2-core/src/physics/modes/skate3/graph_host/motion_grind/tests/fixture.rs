//! Independent synthetic data, using the genuine core playback pipeline.
use super::super::animation::Animation;
use skate3_core::{
    animation::{
        output::attributes::{AnimationAttribute, AttributeName, MotionGraphAttribute},
        phase_blend::PhaseBlend,
        playback_clip::{ClipAttribute, PlaybackClip},
        playback_parameters::{AttributeSink, SettableAttribute, SettableAttributes},
        playback_tree::PlaybackTree,
        skeleton_input::name::encode,
    },
    graph::intents::IntentMap,
};

pub struct Owner {
    pub tree: PlaybackTree,
    pub pending: SettableAttributes,
    pub packets: Vec<MotionGraphAttribute>,
    pub intents: IntentMap,
    pub applications: usize,
}
impl Owner {
    pub fn new(tree: PlaybackTree) -> Self {
        Self {
            tree,
            pending: SettableAttributes::default(),
            packets: vec![],
            intents: IntentMap::new(),
            applications: 0,
        }
    }
}
impl AttributeSink for Owner {
    fn set_attribute(&mut self, a: SettableAttribute) {
        self.pending.set_attribute(a);
    }
}
impl Animation for Owner {
    fn apply_parameters(&mut self) -> Result<(), String> {
        self.tree.set_attributes(self.pending.entries())?;
        self.pending.clear();
        self.applications += 1;
        Ok(())
    }
    fn query_current_attribute(
        &self,
        name: AttributeName,
        mask: u32,
        out: &mut AnimationAttribute,
    ) -> Result<bool, String> {
        self.tree.query_attribute(name, mask, out)
    }
    fn emit_packet(&mut self, name: AttributeName, value: f32) {
        self.packets.push(MotionGraphAttribute { name, value });
    }
    fn intent(&self, name: &str) -> Option<f32> {
        self.intents.get(name).copied()
    }
}
pub fn clip(twist: Option<f32>, height: f32) -> PlaybackTree {
    let mut attributes = vec![ClipAttribute {
        name: encode(b"height"),
        kind: 0,
        begin: -1.0,
        end: -1.0,
        payload: vec![height.to_bits()],
    }];
    if let Some(value) = twist {
        attributes.push(ClipAttribute {
            name: encode(b"twist"),
            kind: 0,
            begin: -1.0,
            end: -1.0,
            payload: vec![value.to_bits()],
        });
    }
    PlaybackTree::Clip {
        name: "independent-test".into(),
        clip: PlaybackClip::new(61.0, 60.0, 1.0, 0, attributes),
    }
}
pub fn blend() -> PlaybackTree {
    PlaybackTree::PhaseBlend(
        PhaseBlend::new(
            encode(b"twist"),
            vec![clip(Some(-2.0), 0.0), clip(Some(4.0), 0.0)],
        )
        .unwrap(),
    )
}
pub fn height_dependent_blend() -> PlaybackTree {
    let branch = |a, b| {
        PlaybackTree::PhaseBlend(
            PhaseBlend::new(
                encode(b"height"),
                vec![clip(Some(a), 0.0), clip(Some(b), 1.0)],
            )
            .unwrap(),
        )
    };
    PlaybackTree::PhaseBlend(
        PhaseBlend::new(encode(b"twist"), vec![branch(-1.0, -2.0), branch(2.0, 4.0)]).unwrap(),
    )
}
