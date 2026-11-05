//! TU3 skitch control from 0x82D477C0, 0x82D47FC0 and 0x82D49580.
//!
//! This is the state-control portion of physical state104. Geometry, following,
//! hand IK, forces and source publication must also be connected before the
//! production registry can admit104. No Skate2 state is used as its substitute.

pub const ENTERING: u32 = 0;
pub const MIDDLE: u32 = 1;
pub const EDGE: u32 = 2;
pub const EXITING: u32 = 3;
pub const END: u32 = 4;

pub const ENTER_DELAY: u64 = 0xcc25_7f85_6577_59a1;
pub const EDGE_DELAY: u64 = 0x81a1_fc1e_9c2b_44d7;
pub const MAX_HANG_TIME: u64 = 0x08fe_1471_1b1c_2bfa;
pub const EDGE_WIDTH: u64 = 0x6027_2903_ad60_ff3b;
pub const EXIT_REVERSAL_TIME: u64 = 0xd60a_7ae0_8b00_e714;
pub const REVERSED_HAND_DELAY: u64 = 0x44ee_9c49_8ee7_c3c2;
pub const EXIT_DURATION: u64 = 0xba19_8476_1d05_7860;
pub const EXIT_RESKITCH_DELAY: u64 = 0xf5f9_d6cf_4d6a_c6d1;

/// Runtime attribute access remains owned by the source Vault/Collections layer.
/// Reads occur at their native branches; no tuning is stored in this kernel.
pub trait Parameters {
    type Error;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
}

/// Scalar control fields of the native Skitching owner. The geometry/hand
/// services fill these before CheckStateChange; this type owns no rigid body.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    /// Native state+1324 (0x52c), kept unsigned for the native >3 early return.
    pub phase: u32,
    pub left_hand: u32,
    pub right_hand: u32,
    pub deck_offset: f32,
    pub half_length: f32,
    pub turn: f32,
    pub time_in_state: f32,
    pub hang_time: f32,
    pub centrifugal_state: i32,
    pub hands_distance: f32,
    pub follow_offset: f32,
    pub reskitch_delay: f32,
    pub hands_off_delay: f32,
    pub target_locked: bool,
    pub target_active: bool,
    pub hands_moving: bool,
    pub push_back: bool,
    pub push_forward: bool,
}

impl Control {
    /// Only this scalar subset of Reset82D47318, not the whole physical reset.
    /// The caller obtains the hand delay from the live ENTER_DELAY attribute.
    pub fn reset_scalars(hands_off_delay: f32) -> Self {
        Self {
            phase: ENTERING,
            left_hand: 1,
            right_hand: 1,
            deck_offset: 0.,
            half_length: 0.,
            turn: 0.,
            time_in_state: 0.,
            hang_time: 0.,
            centrifugal_state: 0,
            hands_distance: 0.,
            follow_offset: 0.,
            reskitch_delay: 0.,
            hands_off_delay,
            target_locked: false,
            target_active: false,
            hands_moving: false,
            push_back: false,
            push_forward: false,
        }
    }

    fn change_phase(&mut self, phase: u32) {
        self.phase = phase;
        self.hands_moving = true;
        self.time_in_state = 0.;
    }

    fn clear_target(&mut self) {
        self.target_locked = false;
        self.target_active = false;
    }

    /// TurningAwayFromEdge82D4C180. Literal3c23d70a at0x820D71E8.
    pub fn turning_away(&self) -> bool {
        self.turn.abs() > f32::from_bits(0x3c23_d70a) && (self.turn > 0.) == (self.deck_offset < 0.)
    }

    /// TurningTowardsEdge82D4C1F0. Literal3f000000 at0x8209975C.
    pub fn turning_towards(&self) -> bool {
        self.turn.abs() > f32::from_bits(0x3f00_0000) && (self.turn > 0.) == (self.deck_offset > 0.)
    }

    /// CheckStateChange82D49580, including branch-local reads and the distinct
    /// clock/target behavior of reversal versus the final direct END write.
    pub fn check_state_change<P: Parameters>(
        &mut self,
        parameters: &mut P,
    ) -> Result<(), P::Error> {
        match self.phase {
            ENTERING => {
                self.push_back = false;
                self.push_forward = false;
                let delay = parameters.scalar(ENTER_DELAY)?;
                if self.time_in_state > delay || self.left_hand == 0 || self.right_hand == 0 {
                    self.hands_distance = self.deck_offset;
                    self.follow_offset = self.deck_offset;
                    self.change_phase(if self.deck_offset.abs() <= self.half_length {
                        MIDDLE
                    } else {
                        EDGE
                    });
                }
            }
            MIDDLE => {
                self.push_back = true;
                self.push_forward = false;
                if !(self.deck_offset.abs() <= self.half_length) && !self.turning_away() {
                    self.change_phase(EDGE);
                }
            }
            EDGE => {
                self.push_back = false;
                self.push_forward = true;
                if self.time_in_state <= parameters.scalar(EDGE_DELAY)? {
                    return Ok(());
                }
                let maximum_hang = parameters.scalar(MAX_HANG_TIME)?;
                let within_edge = if self.deck_offset.abs() <= self.half_length {
                    false
                } else {
                    self.deck_offset.abs() <= self.half_length + parameters.scalar(EDGE_WIDTH)?
                };
                if within_edge
                    && self.turning_towards()
                    && !opposing(self.centrifugal_state as f32, self.deck_offset)
                {
                    self.change_phase(EXITING);
                } else if self.deck_offset.abs() <= self.half_length || self.turning_away() {
                    self.change_phase(MIDDLE);
                } else if self.hang_time > maximum_hang {
                    self.push_back = false;
                    self.push_forward = false;
                    self.change_phase(END);
                }
                if self.phase != EDGE {
                    self.hang_time = 0.;
                    self.clear_target();
                }
            }
            EXITING => {
                self.push_back = false;
                self.push_forward = true;
                if self.time_in_state < parameters.scalar(EXIT_REVERSAL_TIME)?
                    && !self.turning_towards()
                {
                    self.change_phase(EDGE);
                    self.hands_off_delay = parameters.scalar(REVERSED_HAND_DELAY)?;
                }
                if self.phase != EXITING {
                    self.clear_target();
                }
                //82D49A54 rereads the current time, which a reversal just reset.
                if self.time_in_state > parameters.scalar(EXIT_DURATION)? {
                    self.reskitch_delay = parameters.scalar(EXIT_RESKITCH_DELAY)?;
                    self.phase = END;
                }
            }
            //Native unsigned comparison at82D49598 returns for every value>3.
            _ => {}
        }
        Ok(())
    }
}

fn opposing(a: f32, b: f32) -> bool {
    a != 0. && b != 0. && (a > 0.) != (b > 0.)
}

/// Inline UpdateLocalControl82D47824..82D478B0. Keep the scalar multiply,
/// subtract, fsel, multiply ordering instead of reducing it to abs/max.
pub fn combined_turn(turn: f32, spin: f32) -> f32 {
    if opposing(turn, spin) {
        return turn;
    }
    let sign = if turn > 0. { 1. } else { -1. };
    let signed_turn = sign * turn;
    let signed_spin = sign * spin;
    let difference = signed_turn - signed_spin;
    let selected = if difference >= 0. {
        signed_turn
    } else {
        signed_spin
    };
    selected * sign
}

/// UpdateLocalPhysical82D47FC0. The caller supplies live attribute values at
/// 40AAD3FD99B464F4, F178F963558D24A0 and03A59CDFA0B0B967, in that order.
pub fn is_in_air(
    height: f32,
    time_without_wheels: f32,
    high: f32,
    low: f32,
    threshold_time: f32,
) -> bool {
    height > high || (height > low && time_without_wheels > threshold_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Reads {
        values: BTreeMap<u64, f32>,
        trace: Vec<u64>,
    }
    impl Parameters for Reads {
        type Error = u64;
        fn scalar(&mut self, key: u64) -> Result<f32, u64> {
            self.trace.push(key);
            self.values.get(&key).copied().ok_or(key)
        }
    }
    fn authored() -> Reads {
        // Actual embedded skatercollections.vlt default record, SHA256
        //3b7dbd062bb1c906a085514355afff35cfa22f486ae70820c5ad1a42a7aab25b.
        Reads {
            values: [
                (ENTER_DELAY, 0x3f000000),
                (EDGE_DELAY, 0x3f000000),
                (MAX_HANG_TIME, 0x3e4ccccd),
                (EDGE_WIDTH, 0x3ecccccd),
                (EXIT_REVERSAL_TIME, 0x3f000000),
                (REVERSED_HAND_DELAY, 0x3f000000),
                (EXIT_DURATION, 0x3f000000),
                (EXIT_RESKITCH_DELAY, 0x3f000000),
            ]
            .map(|(key, bits)| (key, f32::from_bits(bits)))
            .into_iter()
            .collect(),
            trace: Vec::new(),
        }
    }
    #[test]
    fn entering_waits_at_the_exact_boundary_and_either_hand_can_finish_it() {
        let mut p = authored();
        let mut s = Control::reset_scalars(0.5);
        s.half_length = 1.;
        s.deck_offset = 0.75;
        s.time_in_state = 0.5;
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, ENTERING);
        assert_eq!(p.trace, [ENTER_DELAY]);
        s.right_hand = 0;
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, MIDDLE);
        assert_eq!(s.time_in_state.to_bits(), 0);
        assert_eq!(s.hands_distance, 0.75);
        assert_eq!(s.follow_offset, 0.75);
        assert!(s.hands_moving);
        assert!(!s.push_back && !s.push_forward);
    }
    #[test]
    fn edge_change_retains_the_old_phases_exit_force_for_this_call() {
        let mut p = authored();
        let mut s = Control::reset_scalars(0.5);
        s.phase = MIDDLE;
        s.half_length = 1.;
        s.deck_offset = 1.25;
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, EDGE);
        assert!(s.push_back && !s.push_forward);
        assert!(p.trace.is_empty());
        s.check_state_change(&mut p).unwrap();
        assert!(!s.push_back && s.push_forward);
        assert_eq!(p.trace, [EDGE_DELAY]);
    }
    #[test]
    fn edge_exit_requires_strict_turn_and_resets_its_target_after_transition() {
        let mut p = authored();
        let mut s = Control::reset_scalars(0.5);
        s.phase = EDGE;
        s.half_length = 1.;
        s.deck_offset = 1.25;
        s.time_in_state = 0.6;
        s.turn = 0.5;
        s.target_active = true;
        s.target_locked = true;
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, EDGE);
        s.turn = f32::from_bits(0x3f000001);
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, EXITING);
        assert_eq!(s.time_in_state.to_bits(), 0);
        assert!(!s.target_active && !s.target_locked);
        assert!(s.push_forward);
        assert_eq!(&p.trace[..3], &[EDGE_DELAY, MAX_HANG_TIME, EDGE_WIDTH]);
    }
    #[test]
    fn reversal_rereads_the_reset_clock_but_direct_expiry_preserves_clock_and_target() {
        let mut p = authored();
        let mut s = Control::reset_scalars(0.5);
        s.phase = EXITING;
        s.half_length = 1.;
        s.deck_offset = 1.25;
        s.time_in_state = 0.25;
        s.turn = 0.;
        s.target_active = true;
        s.target_locked = true;
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, EDGE);
        assert_eq!(s.time_in_state.to_bits(), 0);
        assert!(!s.target_active && !s.target_locked);
        assert_eq!(
            p.trace,
            [EXIT_REVERSAL_TIME, REVERSED_HAND_DELAY, EXIT_DURATION]
        );
        p.trace.clear();
        s.phase = EXITING;
        s.turn = 1.;
        s.time_in_state = 0.75;
        s.target_active = true;
        s.target_locked = true;
        s.hands_moving = false;
        s.check_state_change(&mut p).unwrap();
        assert_eq!(s.phase, END);
        assert_eq!(s.time_in_state, 0.75);
        assert!(s.target_active && s.target_locked && !s.hands_moving);
        assert_eq!(
            p.trace,
            [EXIT_REVERSAL_TIME, EXIT_DURATION, EXIT_RESKITCH_DELAY]
        );
    }
    #[test]
    fn completed_and_unknown_unsigned_phases_do_not_touch_the_owner_or_attributes() {
        for phase in [END, 5, u32::MAX] {
            let mut s = Control::reset_scalars(0.5);
            s.phase = phase;
            s.push_back = true;
            let before = s.clone();
            let mut p = authored();
            s.check_state_change(&mut p).unwrap();
            assert_eq!(s, before);
            assert!(p.trace.is_empty());
        }
    }
    #[test]
    fn control_sign_zero_and_physical_thresholds_keep_native_branch_rules() {
        let cases: [(f32, f32, f32); 7] = [
            (0.2, -0.9, 0.2),
            (-0.2, 0.9, -0.2),
            (0.2, 0.9, 0.9),
            (-0.2, -0.9, -0.9),
            //With zero turn,82D47894 takes sign=-1; positive spin is ignored.
            (0., 0.9, 0.),
            (0., -0.9, -0.9),
            //Scalar fsel selects signed_turn on a +0 control, retaining -0.
            (-0., 0., -0.),
        ];
        for (turn, spin, expected) in cases {
            assert_eq!(combined_turn(turn, spin).to_bits(), expected.to_bits());
        }
        assert!(!is_in_air(0.1, 0.1, 0.1, 0.05, 0.1));
        assert!(is_in_air(0.1, f32::from_bits(0x3dccccce), 0.1, 0.05, 0.1));
        assert!(is_in_air(f32::from_bits(0x3dccccce), 0., 0.1, 0.05, 0.1));
        assert!(!is_in_air(0.05, 1., 0.1, 0.05, 0.1));
    }
}
