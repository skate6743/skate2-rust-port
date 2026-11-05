//! ZIP grind runtime split into source-closed components against CURRENT APIs.
//! Physical acquisition remains with the input manager. The retained runtime
//! owns family state, body operations and publication, never candidate selection.
mod advance;
mod arithmetic;
pub(crate) mod board;
mod condition;
mod contact;
pub(crate) mod forces;
mod involuntary;
pub(crate) mod launch;
pub(crate) mod lifecycle;
pub(crate) mod observation;
pub(crate) mod output;
mod phase;
pub(crate) mod post;
mod rng;
mod runtime;
mod settings;
mod state;
mod substate;
mod tip_nudge;
mod wipeout_output;
pub(crate) use advance::advance;
pub(crate) use condition::condition;
pub(crate) use phase::{enter, exit, fill, post};
pub(crate) use runtime::Runtime;

pub(crate) use observation::ManagerObservation;

/// Physical/output family, NOT the seven-valued admission kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum Family {
    FiftyFifty = 0,
    Boardslide = 1,
    Tipslide = 2,
    FiveO = 3,
    Backslash = 4,
    Darkslide = 5,
}

impl Family {
    pub fn from_physical_state(state: u32) -> Option<Self> {
        Some(match state {
            400 => Self::Boardslide,
            401 => Self::FiftyFifty,
            402 => Self::Tipslide,
            403 => Self::FiveO,
            404 => Self::Backslash,
            405 => Self::Darkslide,
            _ => return None,
        })
    }
}
