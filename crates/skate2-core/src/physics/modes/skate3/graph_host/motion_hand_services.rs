//! TU3 HandBusy and MaintainShove lifecycle, independently recovered from IDA.
//! HandBusy: factory82BC8B08, Begin82BAB4D0, End82BAB588.
//! MaintainShove: factory82BCB1C8, Update82BBD690, End82BBD6E8.
use super::motion_animation::MotionAnimation;
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    HandBusy(Hand),
    MaintainShove,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hand {
    Backside,
    Frontside,
}

impl Operation {
    pub fn parse(attributes: &Attributes<'_>) -> Option<Self> {
        match attributes.text("name")? {
            "HandBusy" => Some(Self::HandBusy(Hand::from_name(
                attributes.text("hand").unwrap_or("bs"),
            ))),
            "MaintainShove" => Some(Self::MaintainShove),
            _ => None,
        }
    }
}

impl Hand {
    fn from_name(name: &str) -> Self {
        // Factory compares against bs with strcmp82AE89B0. Every unequal
        // spelling selects index1; there is no validation or stance swap.
        if name == "bs" { Self::Backside } else { Self::Frontside }
    }

    fn index(self) -> usize {
        match self {
            Self::Backside => 0,
            Self::Frontside => 1,
        }
    }
}

/// One persistent owner shared by all graph instances and hand consumers.
#[derive(Clone, Debug, Default)]
pub struct HandServices {
    pub busy_hands: [u32; 2],
    pub keep_shove_channels: bool,
}

impl HandServices {
    pub fn execute(&mut self, operation: Operation, phase: u8, animation: &mut MotionAnimation) {
        let before = self.busy_hands;
        let ended = self.apply(operation, phase);
        if before != self.busy_hands && std::env::var_os("SKATE2_TRACE_SKATE3_HANDS").is_some() {
            eprintln!("skate3 hands: {operation:?} phase {phase} busy {before:?} -> {:?}", self.busy_hands);
        }
        if ended && animation.channels.has("SkitchAntic") {
            // End82BBD6E8 first clears the retention flag, then requests
            // premature channel end (virtual44), preserving its fade timing.
            animation.channels.end("SkitchAntic");
        }
    }

    fn apply(&mut self, operation: Operation, phase: u8) -> bool {
        match (operation, phase) {
            (Operation::HandBusy(hand), 0) => {
                let count = &mut self.busy_hands[hand.index()];
                *count = count.wrapping_add(1);
            }
            (Operation::HandBusy(_), 1) => {
                // HandBusy Update vtable8231F97C+52 is blr82B61BB8.
            }
            (Operation::HandBusy(hand), _) => {
                let count = &mut self.busy_hands[hand.index()];
                *count = count.wrapping_sub(1);
            }
            (Operation::MaintainShove, 0) => {
                // Begin vtable82321000+48 is blr82B61BB8.
            }
            (Operation::MaintainShove, 1) => self.keep_shove_channels = true,
            (Operation::MaintainShove, _) => {
                self.keep_shove_channels = false;
                return true;
            }
        }
        false
    }
}

#[cfg(any())]
#[path = "motion_hand_services/tests.rs"]
mod tests;
