//! Native TU3 pose-output kernels. These do not evaluate clips or choose graphs.
pub mod actor_packet;
pub mod attributes;
mod hierarchy;
pub mod intents;
pub mod motion_graph_packet;
pub mod packet_reset;
pub mod physics_packet;
pub mod setup_physics;
mod sqt;

pub use hierarchy::{compose_hierarchy, compose_hierarchy_in_place};
pub use sqt::{Sqt, sqt_to_local, sqt_to_matrix};

/// Four consecutive native vectors, including all packed fourth lanes.
/// Row/column conversion for a renderer belongs outside this module.
pub type NativeMatrix = [[f32; 4]; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseBufferError {
    NonFiniteWeight,
    InvalidQuaternion,
    CountOverflow,
    ShortInput,
    ShortOutput,
    ShortParents,
    InvalidCacheOffset,
    InvalidParent { bone: usize, parent: i32 },
}
