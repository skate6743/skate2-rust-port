//! Source-bound packet composition for concrete Actor::SetUpPhysics82593640.
//! The caller supplies the runtime services and completed materialized poses.
//! This is not the complete Actor update or Skeleton attribute interpreter.
use super::{
    actor_packet::{self, ActorPacketFields, ActorPublicationEnvironment, ActorPublicationState},
    attributes::{AnimationAttribute, MotionGraphAttribute, PacketAttributes},
    intents::IntentMap,
    motion_graph_packet::{self, MotionGraphPacket},
    packet_reset::{self, AdditionalResetFields},
    physics_packet::{self, ActorPoseBuffers, PhysicsPosePacket, SkaterPublicationState},
};
use crate::animation::commands::{batch::CompletedSetData, buffers::BufferError};

/// Views cover fields touched by this concrete publication chain. Packet fields
/// only used by later Skeleton consumers are not fabricated here.
pub struct PhysicsInputPacket {
    pub pose: PhysicsPosePacket,
    pub reset: AdditionalResetFields,
    pub motion_graph: MotionGraphPacket,
    pub intents: IntentMap,
    pub attributes: PacketAttributes,
    pub actor: ActorPacketFields,
}

pub struct AnimationPublication<'a> {
    pub state: &'a mut SkaterPublicationState,
    pub completed: &'a CompletedSetData,
    pub poses: &'a ActorPoseBuffers,
    pub signal_name: &'a [u8],
    pub tree_attributes: &'a [AnimationAttribute],
}

pub enum IntentPublication<'a> {
    MotionGraph(&'a IntentMap),
    /// Exact same-container branch825936FC; no clear/reinsert occurs.
    RetainPacket,
}

pub struct MotionGraphPublication<'a> {
    pub state: &'a MotionGraphPacket,
    pub intents: IntentPublication<'a>,
    pub attributes: &'a [MotionGraphAttribute],
}

#[derive(Debug, PartialEq, Eq)]
pub enum PublicationError<E> {
    Pose(BufferError),
    RuntimeService(E),
}

///82DB3C48 only stores a borrowed packet pointer at physical+1808. This safe
/// binding keeps the entire owned packet alive and prevents its next mutation
/// while a consumer reads it. It does not execute the downstream physics step.
pub struct PhysicsPacketBinding<'a> {
    packet: &'a PhysicsInputPacket,
}
impl<'a> PhysicsPacketBinding<'a> {
    pub fn packet(&self) -> &'a PhysicsInputPacket {
        self.packet
    }
}

/// Concrete canonical virtual144 is a no-op. Remaining phases execute in the
/// native order: reset, SkaterAnim, MG, intents, MG/tree attrs, actor suffix,
/// then packet binding. Failure preserves writes already made and returns no
/// binding; the caller must not consume the partially published packet.
pub fn compose<'a, E: ActorPublicationEnvironment>(
    packet: &'a mut PhysicsInputPacket,
    animation: AnimationPublication<'_>,
    graph: MotionGraphPublication<'_>,
    actor: &mut ActorPublicationState,
    environment: &mut E,
) -> Result<PhysicsPacketBinding<'a>, PublicationError<E::Error>> {
    packet_reset::reset(&mut packet.pose, &mut packet.reset).map_err(PublicationError::Pose)?;
    physics_packet::publish(
        animation.state,
        animation.completed,
        animation.poses,
        &mut packet.pose,
        f64::from(f32::from_bits(0x3c88_8889)),
        animation.signal_name,
    )
    .map_err(PublicationError::Pose)?;
    motion_graph_packet::publish(graph.state, &mut packet.motion_graph);
    if let IntentPublication::MotionGraph(source) = graph.intents {
        packet.intents.replace_from(source);
    }
    packet
        .attributes
        .replace_from(graph.attributes, animation.tree_attributes);
    actor_packet::publish(
        actor,
        &mut packet.pose,
        &mut packet.reset,
        &mut packet.actor,
        environment,
    )
    .map_err(PublicationError::RuntimeService)?;
    Ok(PhysicsPacketBinding { packet })
}
