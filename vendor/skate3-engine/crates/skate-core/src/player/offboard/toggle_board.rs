//! ToggleBoard TU3 82BA8FE8, retrieval 82BA9310, drop 82BA9990.
//! Caller supplies completed physical observations and actual channel timing.
//! Commands retain the native TransitionTo (+16) versus SequenceTo (+20) distinction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    RetrieveStart,
    RetrieveInto,
    RetrieveCycle,
    RetrieveOut,
    DropStart,
    DropPlaying,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// OffBoard304, OffBoard311, bundle28 byte87, OffBoard313.
    pub grabbing_object: bool,
    pub holding_board: bool,
    pub retrieval_blocked: bool,
    pub retrieval_active: bool,
    pub drop_requested: bool,
    pub throw_requested: bool,
    pub retrieve_requested: bool,
    /// OffBoard36/40 and animation component virtual28.
    pub yaw_radians: f32,
    pub pitch_radians: f32,
    pub mirrored: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Channel {
    pub exists: bool,
    pub remaining: f32,
    pub elapsed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Throw,
    Drop,
    FrontInto,
    FrontCycle,
    FrontOut,
    BackInto,
    BackCycle,
    BackOut,
}
impl Clip {
    /// Constructor82BA8EC8; drop deliberately selects THROW_90.
    pub fn name(self) -> &'static str {
        match self {
            Self::Throw => "OFB_THROW_0",
            Self::Drop => "OFB_THROW_90",
            Self::FrontInto => "OFB_RETRIEVE_HIGH_FRONT_INTO",
            Self::FrontCycle => "OFB_RETRIEVE_HIGH_FRONT_CYC",
            Self::FrontOut => "OFB_RETRIEVE_HIGH_FRONT_OUT",
            Self::BackInto => "OFB_RETRIEVE_HIGH_BACK_INTO",
            Self::BackCycle => "OFB_RETRIEVE_HIGH_BACK_CYC",
            Self::BackOut => "OFB_RETRIEVE_HIGH_BACK_OUT",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    BlendTo(Clip),
    SequenceTo(Clip),
    Stop { blend_seconds: f32 },
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Output {
    /// Append each true flag to MG physical attributes and playback parameters.
    pub retrieving: bool,
    pub dropping: bool,
    /// One-tick OB_RetrieveBoard publication on each cycle transition.
    pub retrieve: bool,
    /// Retrieval branches publish yaw then pitch, even on the terminating tick.
    pub yaw_pitch: Option<[f32; 2]>,
    pub channel: Option<Command>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub phase: Phase,
    previous_yaw: f32,
    yaw: f32,
    pitch: f32,
    initialized_yaw: bool,
    back: bool,
    throwing: bool,
}
impl State {
    /// Begin82BA8FD8 resets phase then tail-calls ResetYawPitch82BA9D08.
    pub fn begin(&mut self) {
        self.phase = Phase::Idle;
        self.reset_orientation();
    }
    fn reset_orientation(&mut self) {
        self.previous_yaw = 0.0;
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.initialized_yaw = false;
        self.back = false;
    }
    /// 82BA9E20; five degrees per update, independent of delta time.
    fn orientation(&mut self, input: Input) {
        let mut yaw = input.yaw_radians * -57.295776_f32;
        if input.mirrored {
            yaw = -yaw;
        }
        if yaw < -90.0 {
            yaw = if self.initialized_yaw {
                if self.yaw >= 0.0 { 180.0 } else { -90.0 }
            } else if yaw >= -135.0 {
                -90.0
            } else {
                180.0
            };
        }
        if input.mirrored {
            yaw = -yaw;
        }
        if self.initialized_yaw {
            let difference = yaw - self.previous_yaw;
            // Preserve the original ordered fsel arithmetic, including NaNs.
            let lower = if -5.0 - difference >= 0.0 {
                -5.0
            } else {
                difference
            };
            let step = if 5.0 - lower >= 0.0 { lower } else { 5.0 };
            yaw = step + self.previous_yaw;
        }
        self.previous_yaw = yaw;
        self.yaw = if input.mirrored { -yaw } else { yaw };
        self.pitch = input.pitch_radians * 57.295776_f32;
        self.initialized_yaw = true;
        self.back = self.yaw.abs() > 90.0;
    }
    pub fn update(&mut self, input: Input, channel: Channel) -> Output {
        if !input.grabbing_object && self.phase == Phase::Idle {
            if input.drop_requested && input.holding_board {
                self.phase = Phase::DropStart;
                self.throwing = false;
            } else if input.throw_requested && input.holding_board {
                self.phase = Phase::DropStart;
                self.throwing = true;
            } else if input.retrieve_requested && !input.holding_board && !input.retrieval_blocked {
                self.phase = Phase::RetrieveStart;
            }
        }
        let mut out = Output::default();
        match self.phase {
            Phase::Idle => self.reset_orientation(),
            Phase::DropStart => {
                out.dropping = true;
                self.phase = Phase::DropPlaying;
                out.channel = Some(Command::BlendTo(if self.throwing {
                    Clip::Throw
                } else {
                    Clip::Drop
                }));
            }
            Phase::DropPlaying => {
                out.dropping = true;
                if !channel.exists || channel.remaining <= 0.0 || channel.elapsed > 0.5 {
                    self.stop(&mut out);
                }
            }
            phase => {
                out.retrieving = true;
                match phase {
                    Phase::RetrieveStart => {
                        self.orientation(input);
                        self.phase = Phase::RetrieveInto;
                        out.channel = Some(Command::BlendTo(if self.back {
                            Clip::BackInto
                        } else {
                            Clip::FrontInto
                        }));
                    }
                    Phase::RetrieveInto => {
                        self.orientation(input);
                        if !channel.exists || channel.remaining <= 0.0 {
                            self.phase = Phase::Idle;
                        } else if channel.remaining <= 0.25 {
                            self.cycle(&mut out);
                        }
                    }
                    Phase::RetrieveCycle => {
                        self.orientation(input);
                        if !input.retrieval_active {
                            self.stop(&mut out);
                        } else if !channel.exists || channel.remaining <= 0.0 {
                            self.phase = Phase::Idle;
                        } else if input.holding_board {
                            self.phase = Phase::RetrieveOut;
                            out.channel = Some(Command::BlendTo(if self.back {
                                Clip::BackOut
                            } else {
                                Clip::FrontOut
                            }));
                        } else if channel.remaining <= 0.25 {
                            self.cycle(&mut out);
                        }
                    }
                    Phase::RetrieveOut => {
                        if !channel.exists
                            || channel.remaining <= 0.0
                            || channel.elapsed > 0.16666667
                        {
                            self.stop(&mut out);
                        }
                    }
                    _ => unreachable!(),
                }
                out.yaw_pitch = Some([self.yaw, self.pitch]);
            }
        }
        out
    }
    fn cycle(&mut self, out: &mut Output) {
        self.phase = Phase::RetrieveCycle;
        out.channel = Some(Command::SequenceTo(if self.back {
            Clip::BackCycle
        } else {
            Clip::FrontCycle
        }));
        out.retrieve = true;
    }
    fn stop(&mut self, out: &mut Output) {
        self.phase = Phase::Idle;
        out.channel = Some(Command::Stop {
            blend_seconds: 0.16666667,
        });
    }
}

#[cfg(test)]
mod tests;
