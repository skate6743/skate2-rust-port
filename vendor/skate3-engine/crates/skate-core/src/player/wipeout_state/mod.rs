//! Original physical WipeoutGround300 and its internal air control.
mod data;
pub mod drives;
pub mod skeleton;
pub use data::State;
pub mod body;
pub mod contact_response;
pub mod control_air;
pub mod control_ground;
pub mod control_profile_air;
pub mod math;
pub mod orientation;
pub mod output;
pub mod profiles;
pub mod recovery;
pub mod response;
pub mod torque;
pub mod weights;
