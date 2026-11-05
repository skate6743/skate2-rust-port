//! Concrete SkaterMotionGraph::GetPhysUpdateData82595FA8 and copy82596008.
//! Actor construction82590DC0 ->8258F488 installs base vtable823006D0;
//! slot36 resolves82595FA8. Unlike the reviewed Skate2 counterpart, this
//! Skate3 body does not reset trick, gesture or wipeout source state afterward.

/// Native name storage at the data boundary; no addresses or host pointers.
/// Exact text/hash member interpretation is a separate string-format contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrickIdentifier(pub [u32; 6]);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrickData {
    pub identifiers: [TrickIdentifier; 2],
    /// Native trick record+48. Its producer/enum meaning is not recovered here.
    pub word48: u32,
    /// Native record+52/+56; copied by lfs/stfs.
    pub scalars52_56: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureData {
    pub gesture: u32,
    /// The native body copies the complete second word, including unused bytes.
    pub flag_bytes: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionGraphPacket {
    /// Source MG+5924 -> physics packet+10392,60 bytes.
    pub trick: TrickData,
    /// MG+5984/+5988 -> packet+10452/+10456.
    pub gesture: GestureData,
    /// MG+5992/+5996 -> packet+10460/+10464; complete words are copied.
    pub wipeout_gesture: [f32; 2],
}

/// Complete concrete publication. Timestep is unused by82595FA8; no source
/// request is consumed, and repeated publication copies the same state.
pub fn publish(source: &MotionGraphPacket, destination: &mut MotionGraphPacket) {
    destination.trick = source.trick;
    destination.gesture.gesture = source.gesture.gesture;
    destination.gesture.flag_bytes = source.gesture.flag_bytes;
    destination.wipeout_gesture[0] = source.wipeout_gesture[0];
    destination.wipeout_gesture[1] = source.wipeout_gesture[1];
}
