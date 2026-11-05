//! Teleporting702 from original TU3 VT823272FC.
//! Enter82D431D8, Update82D431F0 and FillPhysOut82D43280.
use super::input_phase::RawMatrix;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub transform: RawMatrix,
    pub on_board: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    /// Skeleton16552 virtual36: ask the actor for a reset target.
    RequestCheckpoint,
    Captured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    pub transform: RawMatrix,
    pub next_state: u32,
    pub state_61: u8,
    pub board_272: u8,
}

#[derive(Clone, Debug)]
pub struct TeleportState {
    target: RawMatrix, //48
    ready: bool,       //116 bit31
    received: bool,    //116 bit30; native serialization retains this bit.
    on_board: bool,    //116 bit29
}
impl Default for TeleportState {
    fn default() -> Self {
        Self {
            target: [[0; 4]; 4],
            ready: false,
            received: false,
            on_board: true,
        }
    }
}
impl TeleportState {
    /// Clears only the three lifecycle flags; the retained matrix is unread
    /// until Update captures a new target. Native Exit is an empty leaf.
    pub fn enter(&mut self) {
        self.ready = false;
        self.received = false;
        self.on_board = true;
    }
    pub fn update(&mut self, flags_2468: u32, matrix_1536: RawMatrix, byte_1600: u8) -> Update {
        if flags_2468 & 2 == 0 {
            return Update::RequestCheckpoint;
        }
        self.received = true;
        self.target = matrix_1536;
        self.ready = true;
        //rlwimi82D43254 inserts only byte1600 bit0, not a nonzero test.
        self.on_board = byte_1600 & 1 != 0;
        Update::Captured
    }
    /// This publisher does not clear the output when no target is ready.
    pub fn output(&self) -> Option<Output> {
        self.ready.then_some(Output {
            transform: self.target,
            next_state: if self.on_board { 100 } else { 500 },
            state_61: 1,
            board_272: 1,
        })
    }
}
