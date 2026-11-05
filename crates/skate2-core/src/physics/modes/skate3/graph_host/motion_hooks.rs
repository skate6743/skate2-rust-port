//! Native MotionGraph transition hooks, dispatched by the stock controller.
use skate3_core::animation::playback::TransitionSettings;
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub enum MotionHook {
    ///82BBBB68 copies the authored transition into the next-play override.
    Override(TransitionSettings),
    MongoPushToAntic {
        animation: String,
    },
    ///82BBB848, factory82BCA6A8 default right=true.
    GrabSlide {
        right: bool,
    },
}
impl MotionHook {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        match a.text("name")? {
            "GrabSlide" => Some(Self::GrabSlide {
                right: a.boolean_byte("right", 1) != 0,
            }),
            "OverideNextAnimTransitionHook" => {
                Some(Self::Override(super::motion_nodes::transition(a)))
            }
            "MongoPushToAntic" => Some(Self::MongoPushToAntic {
                animation: a.text("anim").unwrap_or("").into(),
            }),
            _ => None,
        }
    }
}
