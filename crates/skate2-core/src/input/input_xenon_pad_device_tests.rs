use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
pub(crate) struct Platform {
    pub(crate) status: u32,
    pub(crate) subtype: Option<u8>,
    pub(crate) calls: Vec<(u32, u32)>,
    pub(crate) fail_index: Option<u32>,
    pub(crate) wireless: bool,
    pub(crate) scripted_controller_zero: Option<crate::input_pad::XboxPadSample>,
    pub(crate) polled_samples: Vec<(u32, crate::input_pad::XboxPadSample)>,
}
impl Platform {
    pub(crate) fn new() -> Self {
        Self {
            status: 0,
            subtype: Some(0),
            calls: Vec::new(),
            fail_index: None,
            wireless: false,
            scripted_controller_zero: None,
            polled_samples: Vec::new(),
        }
    }
}

/// Inverse of the source right-stick radial conversion for authored pattern
/// fixture points. The mounted input map must still admit the raw sample.
pub(crate) fn scripted_right_point(point: [f32; 2]) -> crate::input_pad::XboxPadSample {
    let magnitude = point[1].mul_add(point[1], point[0] * point[0]).sqrt();
    let axis = |value: f32| -> i16 {
        if magnitude == 0.0 {
            return 0;
        }
        let raw_radius = 0.25 + magnitude / f32::from_bits(0x3fb6_db6e);
        ((value / magnitude * raw_radius * 32768.0).round()).clamp(-32768.0, 32767.0) as i16
    };
    crate::input_pad::XboxPadSample {
        buttons: 1,
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: axis(point[0]),
        right_y: axis(-point[1]),
    }
}
impl PadPlatform for Platform {
    type Error = &'static str;
    fn get_state(&mut self, index: u32) -> Result<PadStateResult, Self::Error> {
        self.calls.push((0, index));
        if self.fail_index == Some(index) {
            return Err("platform boundary");
        }
        let default = crate::input_pad::XboxPadSample {
            buttons: 1,
            left_trigger: 0,
            right_trigger: 0,
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
        };
        let sample = if index == 0 {
            self.scripted_controller_zero.unwrap_or(default)
        } else {
            default
        };
        self.polled_samples.push((index, sample));
        Ok(PadStateResult {
            status: self.status,
            state: Some((7, sample)),
        })
    }
    fn get_capabilities(
        &mut self,
        index: u32,
        flags: u32,
    ) -> Result<PadCapabilitiesResult, Self::Error> {
        assert_eq!(flags, 1);
        self.calls.push((1, index));
        Ok(PadCapabilitiesResult {
            status: self.status,
            device_type: Some(1),
            subtype: self.subtype,
            flags: Some(if self.wireless { 2 } else { 0 }),
        })
    }
    fn set_state(&mut self, index: u32, _v: PadVibration) -> Result<u32, Self::Error> {
        self.calls.push((2, index));
        Ok(u32::MAX)
    }
}
pub(crate) struct Remap;
impl PadRemap<u32> for Remap {
    type Error = &'static str;
    fn update(
        &mut self,
        handle: &u32,
        player: u32,
        values: &mut [f32; 24],
    ) -> Result<(), Self::Error> {
        assert_eq!(player, 0);
        assert_eq!(*handle, 42);
        values[0] = 0.75;
        Ok(())
    }
}
#[test]
fn constructor_unknown_guitar_and_failed_caps_map_nothing() {
    let mut device = XenonPadDevice::<u32>::new(0);
    let mut p = Platform::new();
    p.status = u32::MAX;
    p.subtype = None;
    let refs = InputCvarReferences::new_static_initialized();
    device
        .reset_actuators(&mut p, &refs, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(device.guitar, None);
    assert_eq!(device.actuators[0].set, 1);
    let mut values = [9.0; 24];
    assert_eq!(
        device
            .sample(
                &mut values,
                &mut p,
                &mut HostSkeletonCollisionNumeric,
                &mut Remap
            )
            .unwrap(),
        0
    );
    assert!(values.iter().all(|v| v.to_bits() == 9.0f32.to_bits()));
    assert_eq!(p.calls, [(1, 0), (1, 0), (0, 0)]);
    p.status = 0;
    assert_eq!(
        device
            .sample(
                &mut values,
                &mut p,
                &mut HostSkeletonCollisionNumeric,
                &mut Remap
            )
            .unwrap(),
        0
    );
    assert_eq!(device.guitar, None);
    assert!(values.iter().all(|v| v.to_bits() == 9.0f32.to_bits()));
    // Failed native SetState status is ignored; Set frequencies stay committed.
    assert_eq!(device.actuators[0].set, 0);
    assert_eq!(device.actuators[1].set, 0);
}
#[test]
fn a_pad_connected_after_the_reset_is_classified_by_its_capabilities_then_maps() {
    let mut device = XenonPadDevice::<u32>::new(0);
    let mut p = Platform::new();
    p.status = u32::MAX;
    p.subtype = None;
    let refs = InputCvarReferences::new_static_initialized();
    device
        .reset_actuators(&mut p, &refs, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(device.guitar, None);
    p.status = 0;
    p.subtype = Some(1);
    p.calls.clear();
    let mut values = [9.0; 24];
    let sample = |device: &mut XenonPadDevice<u32>, p: &mut Platform, values: &mut [f32; 24]| {
        device
            .sample(values, p, &mut HostSkeletonCollisionNumeric, &mut Remap)
            .unwrap()
    };
    assert_eq!(sample(&mut device, &mut p, &mut values), 24);
    assert_eq!(device.guitar, Some(false));
    assert_eq!(p.calls, [(0, 0), (2, 0), (1, 0)]);
    p.calls.clear();
    assert_eq!(sample(&mut device, &mut p, &mut values), 24);
    assert_eq!(p.calls, [(0, 0)]);
}
#[test]
fn repeated_packet_maps_again_and_live_remap_is_retained() {
    let mut device = XenonPadDevice::<u32>::new(2);
    let mut p = Platform::new();
    let refs = InputCvarReferences::new_static_initialized();
    device
        .reset_actuators(&mut p, &refs, &mut HostReckoningNumeric)
        .unwrap();
    let mut values = [0.0; 24];
    device
        .sample(
            &mut values,
            &mut p,
            &mut HostSkeletonCollisionNumeric,
            &mut Remap,
        )
        .unwrap();
    assert_eq!(values[0].to_bits(), 1.0f32.to_bits());
    device.set_remap(Some(42));
    device
        .sample(
            &mut values,
            &mut p,
            &mut HostSkeletonCollisionNumeric,
            &mut Remap,
        )
        .unwrap();
    assert_eq!(values[0].to_bits(), 0.75f32.to_bits());
    assert_eq!(p.calls, [(1, 2), (1, 2), (0, 2), (2, 2), (0, 2)]);
    assert!(!device.active);
}
#[test]
fn wireless_reset_stays_still_and_unknown_guitar_survives_numeric_failure() {
    let mut device = XenonPadDevice::<u32>::new(0);
    let mut p = Platform::new();
    p.wireless = true;
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    device
        .reset_actuators(&mut p, &refs, &mut HostReckoningNumeric)
        .unwrap();
    // A still request stays still (PC port); a moving one takes the wireless tuning.
    assert_eq!(device.actuators[0].desired, 0);
    assert_eq!(device.actuators[1].desired, 0);
    device
        .set_actuator(1, 0.5, &mut p, &refs, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(device.actuators[1].desired, 37682);
    // Supported tuning preserves unordered through both raw branch clamps.
    assert!(matches!(
        device.set_actuator(0, f32::NAN, &mut p, &refs, &mut HostReckoningNumeric),
        Err(PadActuatorError::Numeric(_))
    ));
    assert_eq!(device.guitar, Some(false));
    p.status = 1;
    p.subtype = None;
    assert!(matches!(
        device.set_actuator(0, f32::NAN, &mut p, &refs, &mut HostReckoningNumeric),
        Err(PadActuatorError::Numeric(_))
    ));
    assert_eq!(device.guitar, None);
    assert_eq!(device.actuators[0].desired, 0);
}
