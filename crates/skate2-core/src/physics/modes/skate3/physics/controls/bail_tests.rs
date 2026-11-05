//! PC bail input regression; the core native producer remains unmodified.
use super::{ActionMap, PlayerControls};

struct Sticks([f32; 4]);
impl ActionMap for Sticks {
    fn value(&mut self, action: u32) -> f32 {
        match action {
            64 => self.0[0],
            65 => self.0[1],
            67 => self.0[2],
            68 => self.0[3],
            _ => 0.0,
        }
    }
    fn state(&mut self, _: u32) -> u8 {
        0
    }
}

#[test]
fn native_bail_intents_preserve_the_already_remapped_axes() {
    let mut controls = PlayerControls::default();
    for x in [-1.0, -0.25, 0.0, 0.25, 1.0] {
        for y in [-1.0, 1.0] {
            let mut packet = Sticks([x, y, 0.75, -0.5]);
            controls.update(&mut packet, 1.0 / 60.0, 0.1, 0x180);
            let value = |name: &str| controls.action_intents.get(name).copied().unwrap_or(0.0);
            assert_eq!(value("WipeoutControlX"), x);
            assert_eq!(value("WipeoutControlY"), y);
            assert_eq!(value("WipeoutGestureX"), 0.75);
            assert_eq!(value("WipeoutGestureY"), -0.5);
            assert_eq!(f32::from_bits(controls.controller.words()[7]), x);
            assert_eq!(
                controls
                    .named_intents
                    .get("WipeoutControlX")
                    .copied()
                    .unwrap_or(0.0),
                x
            );
        }
    }
    // Centering and leaving bail must remove the previous horizontal intent.
    for capabilities in [0x180, 0] {
        controls.update(&mut Sticks([0.0; 4]), 1.0 / 60.0, 0.1, capabilities);
        assert!(!controls.action_intents.contains_key("WipeoutControlX"));
    }
    controls.update(&mut Sticks([1.0, 1.0, 1.0, 1.0]), 1.0 / 60.0, 0.1, 0);
    assert!(!controls.action_intents.contains_key("WipeoutControlX"));
    assert!(!controls.action_intents.contains_key("WipeoutControlY"));
}

#[test]
fn native_ui_remaps_bails_and_plants_and_retains_offboard_exclusions() {
    use super::{remap_enabled,camera_relative_axes};
    for state in [300,601] {
        assert!(remap_enabled(false,false,state,true));
        assert!(!remap_enabled(false,false,state,false));
    }
    assert!(remap_enabled(true,false,500,false));
    assert!(!remap_enabled(true,true,500,true));
    assert!(!remap_enabled(true,false,503,true));
    //A camera facing -Z is the reported vertical-inversion case. Native
    //camera-space Up must become world -Z; raw +Y cannot pass unchanged.
    let camera=[[-1.,0.,0.],[0.,1.,0.],[0.,0.,-1.]];
    assert_eq!(camera_relative_axes([0.,1.],camera),[0.,-1.]);
    assert_eq!(camera_relative_axes([0.,-1.],camera),[0.,1.]);
}
