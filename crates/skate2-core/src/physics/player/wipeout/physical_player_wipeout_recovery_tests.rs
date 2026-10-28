use super::*;
use crate::{
    input_cvar_runtime::wipeout_control::WipeoutRecoveryFloat,
    simulation_assembly_backend::constraint_session::physical_grounded::wipeout::tail::{
        AUTO_FADE_TIME, MAX_AUTO_TIME, MIN_AUTO_TIME, MIN_SETTLED_TIME, RecoveryFlag,
        WipeoutRecoverySource, try_impulse, try_manage_recovery,
    },
};
struct Source {
    reads: Vec<&'static str>,
    fail: Option<&'static str>,
    flags: [bool; 5],
    limit: i32,
    dt: f32,
    maximum: f32,
    minimum: f32,
    settled: f32,
    fade: f32,
    bail_limit: Option<f32>,
}
impl Source {
    fn new() -> Self {
        Self {
            reads: vec![],
            fail: None,
            flags: [false; 5],
            limit: 21,
            dt: 0.1,
            maximum: 5.0,
            minimum: 2.0,
            settled: 1.0,
            fade: 3.0,
            bail_limit: None,
        }
    }
    fn read(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.reads.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl WipeoutRecoverySource for Source {
    type Error = &'static str;
    fn unknown(&mut self, name: &'static str) -> Self::Error {
        name
    }
    fn impulse_limit(&mut self) -> Result<i32, Self::Error> {
        self.read("limit")?;
        Ok(self.limit)
    }
    fn apply_impulse(&mut self, force: Option<SkeletonRootVector>) -> Result<(), Self::Error> {
        self.read("apply")?;
        assert!(force.is_some());
        Ok(())
    }
    fn flag(&mut self, flag: RecoveryFlag) -> Result<bool, Self::Error> {
        let (i, name) = match flag {
            RecoveryFlag::PreventManual => (0, "prevent"),
            RecoveryFlag::IgnoreReset => (1, "ignore"),
            RecoveryFlag::ResetPressed => (2, "reset"),
            RecoveryFlag::Impaled => (3, "impaled"),
            RecoveryFlag::Ai => (4, "ai"),
        };
        self.read(name)?;
        Ok(self.flags[i])
    }
    fn threshold(&mut self, field: WipeoutRecoveryFloat) -> Result<f32, Self::Error> {
        let (name, value) = match field {
            WipeoutRecoveryFloat::MaximumAi => ("ai duration", 8.0),
            WipeoutRecoveryFloat::MinimumUser => ("user time", 1.6),
            WipeoutRecoveryFloat::MinimumImpaled => ("impaled time", 0.5),
        };
        self.read(name)?;
        Ok(value)
    }
    fn global(&mut self, key: u64) -> Result<f32, Self::Error> {
        let (name, value) = match key {
            MAX_AUTO_TIME => ("max auto", self.maximum),
            MIN_AUTO_TIME => ("min auto", self.minimum),
            MIN_SETTLED_TIME => ("settled", self.settled),
            AUTO_FADE_TIME => ("fade", self.fade),
            _ => panic!("unexpected global key"),
        };
        self.read(name)?;
        Ok(value)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.read("dt")?;
        Ok(self.dt)
    }
    fn turn_bail_limit(&mut self) -> Result<Option<f32>, Self::Error> {
        Ok(self.bail_limit)
    }
}
#[test]
fn an_online_turns_limit_ends_a_wipeout_that_never_settles() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.time_in_state = 3.0;
    let mut source = Source::new();
    try_manage_recovery(&mut fields, &mut source).unwrap();
    assert!(!fields.auto_teleport_pending);
    source.bail_limit = Some(2.5);
    try_manage_recovery(&mut fields, &mut source).unwrap();
    assert!(fields.auto_teleport_pending);
    assert_eq!(fields.time_until_teleport.to_bits(), 3.0_f32.to_bits());
}
#[test]
fn impulse_false_and_signed_limit_are_lazy_and_failed_force_preserves_counter() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.is_impulse = Some(false);
    let mut source = Source::new();
    source.fail = Some("limit");
    try_impulse(&mut fields, &mut source).unwrap();
    assert!(source.reads.is_empty());
    fields.is_impulse = Some(true);
    fields.impulse_frames = Some(21);
    source.fail = None;
    try_impulse(&mut fields, &mut source).unwrap();
    assert_eq!(source.reads, ["limit"]);
    fields.impulse_frames = Some(-1);
    fields.impulse = Some(SkeletonRootVector::known([2.0; 4]));
    source.limit = 0;
    source.fail = Some("apply");
    assert_eq!(try_impulse(&mut fields, &mut source), Err("apply"));
    assert_eq!(fields.impulse_frames, Some(-1));
    source.fail = None;
    try_impulse(&mut fields, &mut source).unwrap();
    assert_eq!(fields.impulse_frames, Some(0));
}
#[test]
fn fresh_ignore_read_is_not_short_circuited_and_failure_preserves_native_prefix() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.ignore_reset_button = true;
    fields.over = true;
    let mut source = Source::new();
    source.flags[0] = true;
    source.fail = Some("ignore");
    assert_eq!(try_manage_recovery(&mut fields, &mut source), Err("ignore"));
    assert!(fields.prevent_respawn);
    assert!(fields.ignore_reset_button);
    assert!(!fields.ever_at_rest);
    assert_eq!(source.reads, ["prevent", "ignore"]);
}
#[test]
fn existing_enough_time_latch_does_not_skip_required_minimum_read() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.enough_time_for_reset = true;
    fields.time_settled = 9.0;
    let mut source = Source::new();
    source.fail = Some("user time");
    assert_eq!(
        try_manage_recovery(&mut fields, &mut source),
        Err("user time")
    );
    assert_eq!(fields.time_settled.to_bits(), 9.0_f32.to_bits());
    assert_eq!(source.reads, ["prevent", "ignore", "reset", "user time"]);
}
#[test]
fn unordered_pending_countdown_requests_teleport_without_global_reads() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.auto_teleport_pending = true;
    fields.time_until_teleport = 1.0;
    let mut source = Source::new();
    source.dt = f32::NAN;
    try_manage_recovery(&mut fields, &mut source).unwrap();
    assert!(fields.request_teleport);
    assert!(fields.time_until_teleport.is_nan());
    assert_eq!(
        source.reads,
        ["prevent", "ignore", "reset", "user time", "impaled", "dt"]
    );
}
#[test]
fn actual_auto_decision_sets_fade_then_later_countdown_requests_reset() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.time_in_state = 10.0;
    let mut source = Source::new();
    try_manage_recovery(&mut fields, &mut source).unwrap();
    assert!(fields.auto_teleport_pending);
    assert!(!fields.request_teleport);
    assert_eq!(fields.time_until_teleport.to_bits(), 3.0_f32.to_bits());
    assert_eq!(
        source.reads,
        [
            "prevent",
            "ignore",
            "reset",
            "user time",
            "impaled",
            "ai",
            "max auto",
            "fade"
        ]
    );
    source.reads.clear();
    source.dt = 4.0;
    try_manage_recovery(&mut fields, &mut source).unwrap();
    assert!(fields.request_teleport);
    assert!(!source.reads.contains(&"max auto"));
}
#[test]
fn impaled_escape_precedes_manual_prevention_and_retains_fixed_time_update() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.ever_at_rest = true;
    fields.prevent_respawn = true;
    fields.has_been_impaled = true;
    fields.time_impaled = 0.5;
    let mut source = Source::new();
    source.flags[3] = true;
    try_manage_recovery(&mut fields, &mut source).unwrap();
    assert!(fields.request_teleport);
    assert_eq!(
        fields.time_impaled.to_bits(),
        (0.5 + f32::from_bits(0x3c88_8889)).to_bits()
    );
    assert_eq!(
        source.reads,
        ["prevent", "ignore", "reset", "impaled", "impaled time"]
    );
}
#[test]
fn late_global_fade_failure_keeps_recovery_stores_without_arming_auto_reset() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.over = true;
    fields.time_in_state = 10.0;
    fields.time_settled = 2.0;
    let mut source = Source::new();
    source.fail = Some("fade");
    assert_eq!(try_manage_recovery(&mut fields, &mut source), Err("fade"));
    assert!(fields.ever_at_rest);
    assert!(fields.enough_time_for_reset);
    assert_eq!(
        fields.time_settled.to_bits(),
        (2.0 + f32::from_bits(0x3c88_8889)).to_bits()
    );
    assert!(!fields.auto_teleport_pending);
    assert!(!fields.request_teleport);
}
