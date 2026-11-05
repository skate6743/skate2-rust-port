use super::*;
use crate::reckoning_body_spin::BodySpinCvar;
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError};

#[derive(Debug, PartialEq)]
enum AttributeEvent {
    Capture(Scope),
    Layout(usize),
    Graph(usize, u32),
    Lookup(Scope, u64),
    Float(u64),
    Signed(u64),
}
struct Attributes {
    words: [f32; 308],
    events: Vec<AttributeEvent>,
    fail_offset: Option<usize>,
    reversal_invalid: bool,
}
impl Attributes {
    fn new() -> Self {
        let mut words = [0.0; 308];
        for offset in [0x4a0, 0x4b0, 0x4c0] {
            words[offset / 4] = 0.2;
            words[offset / 4 + 1] = 0.2;
            words[offset / 4 + 2] = 0.05;
            words[offset / 4 + 3] = 0.1;
        }
        for offset in [0x110, 0x1a0, 0x1e0, 0x2a0, 0x320, 0x3a0, 0x3e0, 0x420] {
            for index in 0..8 {
                words[offset / 4 + index] = f32::from(u8::try_from(index).unwrap());
                words[offset / 4 + index + 8] = match offset {
                    0x3e0 => 0.25,
                    0x3a0 => 0.9,
                    _ => 0.5,
                };
            }
        }
        Self {
            words,
            events: vec![],
            fail_offset: None,
            reversal_invalid: false,
        }
    }
}
impl ReckoningAttributeRead for Attributes {
    type Layout = ();
    type Pointer = u64;
    type Error = &'static str;
    fn capture_layout(&mut self, scope: Scope) -> Result<(), Self::Error> {
        self.events.push(AttributeEvent::Capture(scope));
        Ok(())
    }
    fn layout_float(&mut self, (): &(), offset: usize) -> Result<f32, Self::Error> {
        self.events.push(AttributeEvent::Layout(offset));
        if self.fail_offset == Some(offset) {
            Err("layout")
        } else {
            Ok(self.words[offset / 4])
        }
    }
    fn evaluate_graph8(&mut self, (): &(), offset: usize, input: f32) -> Result<f32, Self::Error> {
        self.events
            .push(AttributeEvent::Graph(offset, input.to_bits()));
        Ok(crate::physics::PointGraph {
            x: std::array::from_fn::<_, 8, _>(|i| self.words[offset / 4 + i]),
            y: std::array::from_fn::<_, 8, _>(|i| self.words[offset / 4 + i + 8]),
        }
        .evaluate(input))
    }
    fn lookup(&mut self, scope: Scope, key: u64) -> Result<u64, Self::Error> {
        self.events.push(AttributeEvent::Lookup(scope, key));
        Ok(key)
    }
    fn float(&mut self, p: &u64) -> Result<f32, Self::Error> {
        self.events.push(AttributeEvent::Float(*p));
        match *p {
            0x6ab1_ba81_9741_4fb3 => Ok(0.1),
            0x8b86_e017_4aee_5d6e => Ok(2.0),
            0x5366_a548_f61b_bbea => Ok(0.6),
            0xaf8a_5d55_853a_d035 if !self.reversal_invalid => Ok(0.5),
            _ => Err("unexpected scalar"),
        }
    }
    fn signed_word(&mut self, p: &u64) -> Result<i32, Self::Error> {
        self.events.push(AttributeEvent::Signed(*p));
        assert_eq!(*p, 0xfae8_0499_e7d5_f3e4);
        Ok(0)
    }
}
struct Cvars {
    debug: Vec<ReckoningDebug>,
    stop: Option<ReckoningDebug>,
    body_values: usize,
}
impl Cvars {
    fn new() -> Self {
        Self {
            debug: vec![],
            stop: None,
            body_values: 0,
        }
    }
}
impl BodySpinCvars for Cvars {
    type Pointer = BodySpinCvar;
    type Error = &'static str;
    fn resolve(&mut self, kind: BodySpinCvar) -> Result<BodySpinCvar, Self::Error> {
        Ok(kind)
    }
    fn float(&mut self, p: &BodySpinCvar) -> Result<f32, Self::Error> {
        self.body_values += 1;
        Ok(match p {
            BodySpinCvar::DerivativeScalar => 1.0,
            BodySpinCvar::DerivativeThreshold => 0.0,
            _ => 0.5,
        })
    }
}
impl ReckoningCvars for Cvars {
    fn debug_enabled(&mut self, kind: ReckoningDebug) -> Result<bool, Self::Error> {
        self.debug.push(kind);
        Ok(self.stop == Some(kind))
    }
}
fn input() -> ReckoningGroundInput {
    ReckoningGroundInput {
        com_to_deck: J,
        deck_y: J,
        deck_z: [0.0, 0.0, 1.0, 0.0],
        effective_z: [0.0, 0.0, 1.0, 0.0],
        body_spin: 0.0,
        deck_speed: 0.0,
        absolute_body_speed: 0.0,
        balance: 0.0,
        time_delta: 1.0 / 60.0,
        wheels_in_contact: 4,
        flags_8c4: 0,
        flags_8c8: 0,
    }
}
fn bits(a: impl Into<Vector>) -> [u32; 4] {
    a.into()
        .known_lanes()
        .expect("controlled known lanes")
        .map(f32::to_bits)
}

#[test]
fn constructor_uses_source_cached_controls_in_order_and_exact_identity_lanes() {
    let r = Reckoning::try_new(Attributes::new()).unwrap();
    assert_eq!(bits(r.heading), bits(I));
    assert_eq!(bits(r.up), bits(J));
    for matrix in [
        r.ground_transform,
        r.transform,
        r.transform_no_flip,
        r.inverse_transform,
        r.flip_transform,
        r.body_spin_transform,
    ] {
        assert_eq!(
            matrix
                .known_lanes()
                .expect("controlled known matrix")
                .map(f32::to_bits),
            IDENTITY.map(f32::to_bits)
        );
    }
    for vector in [r.up_speed, r.animation_tilt, r.animation_tilt_speed] {
        assert_eq!(bits(vector), [0; 4]);
    }
    assert_eq!(bits(r.flip_axis), bits(I));
    assert_eq!(
        [
            r.spin_angle,
            r.spin_speed,
            r.dynamic_lean,
            r.ground_blend,
            r.flip_angle,
            r.flip_speed,
            r.target_flip_speed
        ]
        .map(f32::to_bits),
        [0; 7]
    );
    assert!(!r.is_flipping && !r.is_front_flipping);
    let offsets: Vec<_> = r
        .attributes
        .events
        .iter()
        .filter_map(|event| match event {
            AttributeEvent::Layout(o) => Some(*o),
            _ => None,
        })
        .collect();
    assert_eq!(
        offsets,
        [
            0x4c0, 0x4c4, 0x4c8, 0x4cc, 0x4a0, 0x4a4, 0x4a8, 0x4ac, 0x4b0, 0x4b4, 0x4b8, 0x4bc
        ]
    );
}

#[test]
fn complete_flat_grounded_updates_keep_identity_and_lazy_reversal_unread() {
    let mut attrs = Attributes::new();
    attrs.reversal_invalid = true;
    let mut r = Reckoning::try_new(attrs).unwrap();
    r.attributes.events.clear();
    let input = input();
    let mut c = Cvars::new();
    let mut n = HostReckoningNumeric;
    for _ in 0..37 {
        r.try_update_grounded(&input, J, J, input.deck_z, &mut c, &mut n)
            .unwrap();
    }
    assert_eq!(bits(r.up()), bits(J));
    assert_eq!(
        r.transform()
            .known_lanes()
            .expect("controlled known matrix")
            .map(f32::to_bits),
        IDENTITY.map(f32::to_bits)
    );
    // Native ACos polynomial retains a small residual at cosine1; no repair
    // to libc acos(1)==0 is permitted. Independently checked raw polynomial.
    assert_eq!(r.dynamic_lean().to_bits(), 0x3500_0000);
    assert_eq!(bits(r.animation_tilt()), [0; 4]);
    assert_eq!(c.body_values, 37 * 5);
    assert_eq!(c.debug, vec![ReckoningDebug::Reckoning; 37 * 2]); // Deck adjustment thenfinal; no tilt gate.
    for e in &r.attributes.events {
        assert!(!matches!(
            e,
            AttributeEvent::Lookup(
                _,
                0xda9e_8318_96b8_6ffc | 0xfae8_0499_e7d5_f3e4 | 0x6ab1_ba81_9741_4fb3
            )
        ));
        assert!(!matches!(e, AttributeEvent::Float(0xaf8a_5d55_853a_d035)));
    }
    assert_eq!(bits(r.slow_filter.value()), bits(r.up));
    assert_eq!(bits(r.fast_filter.value()), bits(r.up));
}

#[test]
fn sloped_ground_uses_angle_graph_not_dead_debug_max_and_keeps_histories() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let mut input = input();
    input.deck_speed = 2.0;
    input.absolute_body_speed = 2.0;
    let slope = [0.6, 0.8, 0.0, 0.125];
    let mut c = Cvars::new();
    let mut n = HostReckoningNumeric;
    for _ in 0..9 {
        r.try_update_grounded(&input, slope, J, input.deck_z, &mut c, &mut n)
            .unwrap();
    }
    assert_eq!(r.ground_blend.to_bits(), 0.25_f32.to_bits()); // speedGraph0.9 is NOT target.
    assert!(r.up.xyz[0] > 0.0);
    assert!(r.up.w.expect("controlled known W") != 0.0);
    assert_ne!(bits(r.slow_filter.integral()), [0; 4]);
    assert_ne!(bits(r.ground_filter.last_target()), bits(J));
    assert_eq!(bits(r.slow_filter.value()), bits(r.up));
    assert_eq!(bits(r.fast_filter.value()), bits(r.up));
    assert!(
        r.attributes
            .events
            .iter()
            .any(|e| matches!(e, AttributeEvent::Lookup(Scope::Own, 0xda9e_8318_96b8_6ffc)))
    );
    assert!(
        !r.attributes
            .events
            .iter()
            .any(|e| matches!(e, AttributeEvent::Float(0xda9e_8318_96b8_6ffc)))
    );
    assert!(
        r.attributes
            .events
            .iter()
            .any(|e| matches!(e, AttributeEvent::Graph(0x110, _)))
    );
}

#[test]
fn selective_reset_preserves_matrices_filters_flip_state_and_original_body_spin() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    {
        let mut v = r.transform.columns()[3];
        v.xyz[0] = 37.0;
        r.transform.set_column(3, v);
    }
    {
        let mut v = r.ground_transform.columns()[3];
        v.w = Some(-0.0);
        r.ground_transform.set_column(3, v);
    }
    {
        let mut v = r.flip_transform.columns()[1];
        v.xyz[1] = 0.75;
        r.flip_transform.set_column(1, v);
    }
    r.flip_axis = Vector::known([2.0; 4]);
    r.flip_angle = 3.0;
    r.flip_speed = 4.0;
    r.target_flip_speed = 5.0;
    r.is_flipping = true;
    r.is_front_flipping = true;
    r.slow_filter.update([2.0; 4]);
    let history = r.slow_filter.integral();
    r.ground_blend = 0.75;
    r.animation_tilt = Vector::known([7.0; 4]);
    r.heading = Vector::known([8.0; 4]);
    r.body_spin
        .try_update_grounded(0.5, 0.75, &mut Cvars::new())
        .unwrap();
    let spin_state = r.body_spin.state_words();
    assert_ne!(spin_state, ([0; 30], [0; 12], 0));
    r.reset();
    assert_eq!(bits(r.heading), bits(I));
    assert_eq!(bits(r.up), bits(J));
    assert_eq!(bits(r.animation_tilt), [0; 4]);
    assert_eq!(r.ground_blend.to_bits(), 0);
    assert_eq!(
        r.transform.known_lanes().expect("controlled known matrix")[12].to_bits(),
        37.0_f32.to_bits()
    );
    assert_eq!(
        r.ground_transform
            .known_lanes()
            .expect("controlled known matrix")[15]
            .to_bits(),
        0x8000_0000
    );
    assert_eq!(
        r.flip_transform
            .known_lanes()
            .expect("controlled known matrix")[5]
            .to_bits(),
        0.75_f32.to_bits()
    );
    assert_eq!(
        [r.flip_angle, r.flip_speed, r.target_flip_speed].map(f32::to_bits),
        [3.0_f32, 4.0, 5.0].map(f32::to_bits)
    );
    assert!(r.is_flipping && r.is_front_flipping);
    assert_eq!(bits(r.flip_axis), bits([2.0; 4]));
    assert_eq!(bits(r.slow_filter.integral()), bits(history));
    assert_eq!(spin_state, r.body_spin.state_words());
}

#[test]
fn body_spin_reset_preserves_heading_filters_flip_and_spin_collaborator() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    r.spin_speed = f32::from_bits(0xffc0_0123);
    r.spin_angle = -7.0;
    r.heading = Vector::known([8.0; 4]);
    r.ground_normal = Vector::known([3.0; 4]);
    r.animation_tilt = Vector::known([7.0; 4]);
    r.dynamic_lean = 0.25;
    r.ground_blend = 0.75;
    r.flip_speed = 4.0;
    {
        let mut v = r.body_spin_transform.columns()[3];
        v.xyz[0] = 37.0;
        r.body_spin_transform.set_column(3, v);
    }
    r.slow_filter.update([2.0; 4]);
    let history = r.slow_filter.integral();
    r.body_spin
        .try_update_grounded(0.5, 0.75, &mut Cvars::new())
        .unwrap();
    let spin_state = r.body_spin.state_words();
    r.attributes.events.clear();
    r.reset_body_spin();
    assert_eq!([r.spin_speed, r.spin_angle].map(f32::to_bits), [0; 2]);
    assert_eq!(bits(r.heading), bits([8.0; 4]));
    assert_eq!(bits(r.ground_normal), bits([3.0; 4]));
    assert_eq!(bits(r.animation_tilt), bits([7.0; 4]));
    assert_eq!(
        [r.dynamic_lean, r.ground_blend, r.flip_speed].map(f32::to_bits),
        [0.25, 0.75, 4.0].map(f32::to_bits)
    );
    assert_eq!(
        r.body_spin_transform
            .known_lanes()
            .expect("controlled known matrix")[12]
            .to_bits(),
        37.0_f32.to_bits()
    );
    assert_eq!(bits(r.slow_filter.integral()), bits(history));
    assert_eq!(r.body_spin.state_words(), spin_state);
    assert!(r.attributes.events.is_empty());
}

#[test]
fn ground_wrapper_stores_inputs_before_flipping_assertion_and_any_cvar() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    r.is_flipping = true;
    let mut c = Cvars::new();
    let mut n = HostReckoningNumeric;
    assert!(matches!(
        r.try_update_grounded(&input(), J, [2.0; 4], [3.0; 4], &mut c, &mut n),
        Err(ReckoningError::FlippingAssertion)
    ));
    assert_eq!(bits(r.dynamic_up), bits([2.0; 4]));
    assert_eq!(bits(r.heading), bits([3.0; 4]));
    assert_eq!(c.body_values, 0);
}

#[test]
fn fixed_heading_preserves_early_state_on_assertion_and_numeric_failure() {
    for flipping in [true, false] {
        let mut r = Reckoning::try_new(Attributes::new()).unwrap();
        r.is_flipping = flipping;
        r.heading = Vector::known([0.0, 0.0, 1.0, 7.0]);
        let heading = r.heading;
        let old_dynamic = r.dynamic_up;
        let mut c = Cvars::new();
        let mut n = StopNumeric { at: 1, calls: 0 };
        let result = r.try_update_grounded_fixed_heading(&input(), J, [2.0; 4], &mut c, &mut n);
        if flipping {
            assert!(matches!(result, Err(ReckoningError::FlippingAssertion)));
            assert_eq!(c.body_values, 0);
        } else {
            assert!(matches!(result, Err(ReckoningError::Numeric(1))));
            assert!(c.body_values > 0);
        }
        assert_eq!(bits(r.heading), bits(heading));
        assert_eq!(bits(r.dynamic_up), bits(old_dynamic));
    }
}

#[test]
fn fixed_heading_success_reuses_existing_heading_then_rebuilds_its_basis() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let heading = [0.0, 0.0, 1.0, 0.0];
    r.heading = heading.into();
    r.try_update_grounded_fixed_heading(
        &input(),
        J,
        J,
        &mut Cvars::new(),
        &mut HostReckoningNumeric,
    )
    .unwrap();
    // The fixed entry skips the wrapper's initial heading assignment, but the
    // completed basis rebuild still stores cross-derived heading (including -0).
    assert_eq!(bits(r.heading), bits([-0.0, -0.0, 1.0, -0.0]));
    assert_eq!(bits(r.dynamic_up), bits(J));
}

#[test]
fn late_control_read_failure_keeps_each_previous_coefficient_write() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let old = r.ground_filter.control();
    r.attributes.words[0x4c0 / 4] = 7.0;
    r.attributes.words[0x4c4 / 4] = 8.0;
    r.attributes.fail_offset = Some(0x4c8);
    let result: RunResult<_, (), _> = r.update_ground_normal(J.into(), &mut HostReckoningNumeric);
    assert!(matches!(result, Err(ReckoningError::Attribute("layout"))));
    let now = r.ground_filter.control();
    assert_eq!(now.proportional.to_bits(), 7.0_f32.to_bits());
    assert_eq!(now.integral.to_bits(), 8.0_f32.to_bits());
    assert_eq!(now.derivative.to_bits(), old.derivative.to_bits());
    assert_eq!(bits(r.ground_filter.last_target()), bits(J));
}

struct StopNumeric {
    at: usize,
    calls: usize,
}
impl ReckoningNumeric for StopNumeric {
    type Error = usize;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, usize> {
        self.calls += 1;
        if self.calls == self.at {
            Err(self.at)
        } else {
            Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, usize> {
        self.calls += 1;
        if self.calls == self.at {
            Err(self.at)
        } else {
            Ok(1.0 / v.sqrt())
        }
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, usize> {
        Ok(1.0 / v)
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, usize> {
        HostReckoningNumeric.truncate_i32(v).map_err(|_| 99)
    }
}
#[test]
fn ground_filter_dot_and_seed_failures_have_distinct_native_store_prefixes() {
    for at in [1, 2] {
        let mut r = Reckoning::try_new(Attributes::new()).unwrap();
        let mut n = StopNumeric { at, calls: 0 };
        let result: RunResult<_, (), _> =
            r.update_ground_normal(Vector::known([2.0, 1.0, 0.0, 4.0]), &mut n);
        assert!(matches!(result,Err(ReckoningError::Numeric(index)) if index==at));
        assert_ne!(bits(r.ground_filter.value()), bits(J));
        assert_eq!(
            bits(r.ground_normal),
            bits(if at == 1 {
                J.into()
            } else {
                r.ground_filter.value()
            })
        );
    }
}

#[test]
fn enabled_debug_stops_at_its_actual_branch_after_completed_target_writes() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let input = input();
    let mut c = Cvars::new();
    c.stop = Some(ReckoningDebug::Reckoning);
    let result = r.try_update_grounded(
        &input,
        J,
        J,
        input.deck_z,
        &mut c,
        &mut HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(ReckoningError::DebugOverlayUnsupported(
            ReckoningDebug::Reckoning
        ))
    ));
    assert_eq!(bits(r.up_target), bits(J));
    assert_eq!(c.body_values, 5);
    assert!(
        !r.attributes
            .events
            .iter()
            .any(|e| matches!(e, AttributeEvent::Lookup(_, 0x8b86_e017_4aee_5d6e)))
    );
}

// Make the host numerical error boundary part of this caller's checked type.
const _: Option<ReckoningHostNumericError> = None;

#[test]
fn coffin_uses_effective_axis_and_preserves_double_cross_projection() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let mut input = input();
    input.flags_8c8 = 1;
    input.effective_z = [0.6, -0.8, 0.0, 0.0];
    let mut c = Cvars::new();
    r.try_update_grounded(
        &input,
        J,
        J,
        input.deck_z,
        &mut c,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    // Controlled geometry oracle: J projected perpendicular to (.6,-.8,0)
    // is normalized (.8,.6,0). The production graph still uses TWO raw crosses.
    assert!((r.up.xyz[0] - 0.8).abs() < 0.000_001);
    assert!((r.up.xyz[1] - 0.6).abs() < 0.000_001);
    assert_eq!(r.up.xyz[2].to_bits(), 0x8000_0000); // Final-negated cross residual.
    assert!(c.debug.contains(&ReckoningDebug::Coffin));
    assert_eq!(bits(r.slow_filter.value()), bits(r.up));
    assert_eq!(bits(r.fast_filter.value()), bits(r.up));
}

#[test]
fn failed_second_basis_keeps_completed_main_transform_and_inverse() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    r.heading = Vector::known([0.0, 0.0, 1.0, 0.0]);
    {
        let mut v = r.transform.columns()[3];
        v.xyz[0] = 3.0;
        r.transform.set_column(3, v);
    }
    {
        let mut v = r.ground_transform.columns()[3];
        v.xyz[0] = 7.0;
        r.ground_transform.set_column(3, v);
    }
    let old_ground = r.ground_transform;
    // Main basis dot/seed/dot/seed succeeds; first ground-basis dot fails.
    let mut numeric = StopNumeric { at: 5, calls: 0 };
    assert_eq!(r.calculate_transform(&mut numeric), Err(5));
    assert_eq!(
        r.transform.known_lanes().expect("controlled known matrix")[12].to_bits(),
        3.0_f32.to_bits()
    );
    assert_eq!(
        r.inverse_transform
            .known_lanes()
            .expect("controlled known matrix")[12]
            .to_bits(),
        (-3.0_f32).to_bits()
    );
    assert_eq!(
        r.ground_transform
            .known_lanes()
            .expect("controlled known matrix")
            .map(f32::to_bits),
        old_ground
            .known_lanes()
            .expect("controlled known matrix")
            .map(f32::to_bits)
    );
    assert_eq!(bits(r.heading), bits([-0.0, -0.0, 1.0, -0.0]));
}

#[test]
fn flat_tilt_keeps_previous_value_and_nan_balance_skips_deck_graph() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    r.animation_tilt = Vector::known([0.25, 1.0, 2.0, -0.0]);
    let mut input = input();
    input.balance = f32::NAN;
    let mut c = Cvars::new();
    r.try_update_grounded(
        &input,
        J,
        J,
        input.deck_z,
        &mut c,
        &mut HostReckoningNumeric,
    )
    .unwrap();
    assert_eq!(bits(r.animation_tilt), bits([0.25, 1.0, 2.0, -0.0]));
    assert_eq!(c.debug, [ReckoningDebug::Reckoning]);
    assert!(
        !r.attributes
            .events
            .iter()
            .any(|e| matches!(e, AttributeEvent::Graph(0x110 | 0x320 | 0x2a0, _)))
    );
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_original_reckoning_advances_retained_grounded_state() {
    use crate::reckoning_attributes::{RECKONING_CLASS, ReckoningVaultAttributes};
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        RECKONING_CLASS,
    ]);
    let mut r = Reckoning::try_new(ReckoningVaultAttributes::bind(&vault)).unwrap();
    // Frozen constructor oracle independently read from original BIN9e110.
    let slow = r.slow_filter.control();
    assert_eq!(
        [
            slow.proportional,
            slow.integral,
            slow.derivative,
            slow.integral_blend
        ]
        .map(f32::to_bits),
        [0x3d23_d70a, 0x3d23_d70a, 0x3e4c_cccd, 0x3e4c_cccd]
    );
    let mut input = input();
    let mut c = Cvars::new();
    let mut n = HostReckoningNumeric;
    r.try_update_grounded(&input, J, J, input.deck_z, &mut c, &mut n)
        .unwrap();
    assert_eq!(bits(r.up), bits(J));
    assert_eq!(
        r.transform
            .known_lanes()
            .expect("controlled known matrix")
            .map(f32::to_bits),
        IDENTITY.map(f32::to_bits)
    );
    assert_eq!(r.dynamic_lean.to_bits(), 0x3500_0000);
    input.deck_speed = 3.0;
    input.absolute_body_speed = 3.0;
    input.body_spin = 0.5;
    let supplied_ground = [0.6, 0.8, 0.0, 0.0];
    let mut first = None;
    for frame in 0..24 {
        r.try_update_grounded(&input, supplied_ground, J, input.deck_z, &mut c, &mut n)
            .unwrap();
        assert!(
            r.transform
                .known_lanes()
                .expect("controlled known matrix")
                .iter()
                .all(|v| v.is_finite())
        );
        assert!(
            r.up.known_lanes()
                .expect("controlled known lanes")
                .iter()
                .all(|v| v.is_finite())
        );
        if frame == 0 {
            first = Some(bits(r.up));
        }
    }
    assert!(r.up.xyz[0] > 0.0);
    assert_ne!(bits(r.up), first.unwrap());
    assert_ne!(bits(r.slow_filter.integral()), [0; 4]);
    assert_eq!(bits(r.ground_filter.last_target()), bits(supplied_ground));
    assert_eq!(bits(r.slow_filter.value()), bits(r.up));
    assert_eq!(bits(r.fast_filter.value()), bits(r.up));
    assert_eq!(c.body_values, 25 * 5);
    println!(
        "original retained Reckoning Vault -> constructor + 25 grounded updates -> current transform/Up/lean/tilt; controlled processed fields and current cvars, host numeric seeds, no native trace or live player claim; Up={:?} lean={} tilt={:?}",
        r.up, r.dynamic_lean, r.animation_tilt
    );
}

#[path = "reckoning_grounded_live_tests.rs"]
mod live_read_tests;


