//! Controller-derived state is evaluated on the same fixed gameplay tick.
use super::{Skate3PhysicsState, skater::Skate3ControllerState};
use crate::skate3_native::input::PublishedTickInput;
use crate::skate3_native::math_host::*;
use skate3_core::graph::intents::IntentMap;
use skate3_core::input::{
    controller::{ActionMap, DerivedControllerInput, MagnitudeHeldSettings},
    riding_intentions::{self, PushPreferences, RidingIntent},
    wipeout_intentions,
};
use std::collections::BTreeMap;
mod object_settings;


pub(crate) struct PlayerControls {
    pub controller: DerivedControllerInput,
    pub offboard_direction: Option<[f32; 4]>,
    pub intents: Vec<RidingIntent>,
    /// Named ActionGraph/gesture intents for the current tick (SDK / observation).
    pub named_intents: BTreeMap<String, f32>,
    pub action_intents: IntentMap,
    pub ticks: u64,
    pub actor_flags: u32,
    #[cfg(feature = "skate3-runtime")]
    pub host_actor_input: super::player::actor_input::HostActorInput,
    pub bumper_state_502: bool,
    pub bumper_state_104: bool,
    pub preferences: PushPreferences,
    //One tick's native PlayerUI82898D20 result, shared by animation and PhysIn.
    //None means the native offboard remap gate did not run, not missing camera.
    offboard_axes: Option<[f32; 2]>,
    gestures: Option<crate::skate3_native::input::gesture_input::GestureInput>,
    object_curves: Option<skate3_core::input::object_intentions::Curves>,
}
impl Default for PlayerControls {
    fn default() -> Self {
        let mut controller = DerivedControllerInput::from_words([0; 26]);
        controller.initialize();
        Self {
            controller,
            offboard_direction: None,
            intents: Vec::new(),
            named_intents: BTreeMap::new(),
            action_intents: IntentMap::new(),
            ticks: 0,
            actor_flags: 0,
            #[cfg(feature = "skate3-runtime")]
            host_actor_input: Default::default(),
            bumper_state_502: false,
            bumper_state_104: false,
            preferences: PushPreferences::default(),
            offboard_axes: None,
            gestures: None,
            object_curves: None,
        }
    }
}



impl PlayerControls {
    /// PlayerUI82898920 transforms the gameplay packet before Raw/Derived input.
    /// State83 enables the same native camera remap for bail and footplant control.
    pub fn update_for_physics(
        &mut self,
        map: &mut impl ActionMap,
        physics: &Skate3PhysicsState,
        skater: &Skate3ControllerState,
        camera: &crate::skate3_native::camera::CameraRuntime,
    ) -> Result<(), String> {
        self.update_for_physics_with_entry_camera(map, physics, skater, camera, None)
    }

    /// A newly acquired offboard actor uses the host's last presented basis
    /// until this source camera has completed its first ordinary world tick.
    pub(crate) fn update_for_physics_with_entry_camera(
        &mut self, map: &mut impl ActionMap, physics: &Skate3PhysicsState,
        skater: &Skate3ControllerState,
        camera: &crate::skate3_native::camera::CameraRuntime,
        entry_camera: Option<[[f32; 3]; 3]>,
    ) -> Result<(), String> {
        let physical = &skater.player_input.physical;
        //82DB7678/7694 writes category and State75 from the same category==500.
        //82898B80..BC4 excludes grabbing an object (304) and state503.
        let graph_offboard = skater.teleport_state.manual_on_board()
            .map_or(physical.state.category_12 == 500, |on_board| !on_board);
        //82898BC8..BE0 also remaps State83 while on board. Wipeout Fill
        //82D3EF84 and Footplant Fill82D7112C publish that actual byte.
        let remap = remap_enabled(graph_offboard,physical.off_board.flag_304!=0,
            physical.state.state_16,skater.player_state.state_flags[83-52]);
        let axes = if remap {
            let basis = camera.frame.as_ref().map(|frame| frame.basis.columns)
                .or(entry_camera)
                .ok_or("Native camera-relative input requires a completed presentation camera frame")?;
            Some(camera_relative_axes([map.value(64), map.value(65)], basis))
        } else {
            None
        };
        self.update(
            &mut SimulationActions {
                source: map,
                offboard_axes: axes,
            },
            physics.settings.step.simulation.time_step,
            physics.settings.input_magnitude_threshold,
            physical.scoring.capabilities_204,
        );
        self.offboard_axes = axes;
        self.offboard_direction = if graph_offboard {axes.map(|v| [v[0], 0., v[1], 0.])}else{None};
        Ok(())
    }

    /// Parent must pass this SAME sampled mapping into input_phase::advance.
    /// Do not rotate again: the camera/state may have advanced since sampling.
    pub fn simulation_actions<'a>(&self, source: &'a mut dyn ActionMap) -> SimulationActions<'a> {
        SimulationActions {
            source,
            offboard_axes: self.offboard_axes,
        }
    }

    pub fn load(root: &std::path::Path) -> Result<Self, String> {
        Ok(Self {
            gestures: Some(crate::skate3_native::input::gesture_input::GestureInput::load(root)?),
            object_curves: Some(object_settings::load(&*crate::skate3_native::physics::player::collections_at(root)?)?),
            ..Self::default()
        })
    }

    pub fn publish_gestures(&mut self, difficulty: u32, physical_state: u32) {
        if let Some(gestures) = &mut self.gestures {
            let words = self.controller.words();
            let axes = [[words[7], words[8]], [words[9], words[10]]].map(|p| p.map(f32::from_bits));
            for (name, value) in gestures.publish(
                axes,
                difficulty,
                self.actor_flags,
                physical_state,
                &mut self.action_intents,
            ) {
                if value.abs() > 0.01 {
                    self.named_intents.insert(name, value);
                }
            }
        }
    }

    pub fn update(
        &mut self,
        map: &mut impl ActionMap,
        dt: f32,
        magnitude_threshold: f32,
        physical_capabilities: u32,
    ) {
        self.offboard_axes = None;
        // S2 GenerateActionGraphIntents skips Listener.Fill while pinned,
        // unless its explicit gesture exception is enabled. Retain the same
        // Derived history so releasing a pin preserves the real next edge.
        #[cfg(feature = "skate3-runtime")]
        let sample_derived=!self.host_actor_input.pinned || self.host_actor_input.pinned_gestures;
        #[cfg(not(feature = "skate3-runtime"))]
        let sample_derived=true;
        if sample_derived {
        self.controller.update(
            map,
            dt,
            self.bumper_state_502,
            self.bumper_state_104,
            &MagnitudeHeldSettings {
                attribute: Some(magnitude_threshold),
                // The validated field is present, so no engine missing-field path runs.
                missing_attribute_value: 0.0,
            },
        );
        }
        self.intents =
            riding_intentions::produce(&self.controller, self.actor_flags, self.preferences);
        self.intents
            .extend(skate3_core::input::manual_intentions::produce(
                &self.controller,
                self.actor_flags,
            ));
        self.intents.extend(skate3_core::input::gameplay_gestures::produce(&self.controller));
        self.intents.extend(wipeout_intentions::produce(
            &self.controller,
            self.actor_flags,
            physical_capabilities,
        ));
        self.intents
            .extend(skate3_core::input::anticipation_intentions::produce(
                &self.controller,
            ));
        self.intents.extend(
            skate3_core::input::trick_intentions::produce(&self.controller)
                .into_iter()
                .map(|intent| RidingIntent {
                    name: intent.name,
                    value: intent.value,
                }),
        );
        if let Some(curves)=&self.object_curves {
            self.intents.extend(curves.produce(&self.controller));
        }
        //GenerateActionGraphIntents82594310 clears the AG map through82BC1B68
        //before Listener::Fill. MG lifecycle intents use a different persistent map.
        self.action_intents.clear();
        self.named_intents.clear();
        for intent in &self.intents {
            self.action_intents.insert(intent.name, intent.value);
            if intent.value.abs() > 0.01 {
                self.named_intents
                    .insert(intent.name.to_string(), intent.value);
            }
        }
        self.ticks += 1;
    }
}

/// Borrow the existing action packet; override only the native left-stick pair.
pub(crate) struct SimulationActions<'a> {
    source: &'a mut dyn ActionMap,
    offboard_axes: Option<[f32; 2]>,
}

impl ActionMap for SimulationActions<'_> {
    fn value(&mut self, action: u32) -> f32 {
        if let Some(axes) = self.offboard_axes {
            match action {
                64 => return axes[0],
                65 => return axes[1],
                _ => {}
            }
        }
        self.source.value(action)
    }

    fn state(&mut self, action: u32) -> u8 {
        if self.offboard_axes.is_some() && matches!(action, 64 | 65) {
            return u8::from(self.value(action) != 0.0);
        }
        self.source.state(action)
    }
}

/// Complete numerical remap82898D20; camera is the native XYZ presentation
/// basis, before Bevy's presentation-only two-axis sign conversion.
fn camera_relative_axes(stick: [f32; 2], camera: [[f32; 3]; 3]) -> [f32; 2] {
    let [mut right, mut up, mut forward] = camera;
    //82898D98..DB4: ABS(dot(world_up, camera_Z)) < literal820ED5E8.
    //Near either vertical pole, native retains ALL original camera axes.
    if forward[1].abs() < f32::from_bits(0x3f7d_70a4) {
        right = normalize_camera_axis(cross_camera_axis([0.0, 1.0, 0.0], forward));
        forward = normalize_camera_axis(cross_camera_axis(right, [0.0, 1.0, 0.0]));
        up = normalize_camera_axis(cross_camera_axis(forward, right));
    }
    //CacheLine includes its count at0.82898EE0..F0C reads Buttons17-16
    //and Buttons18-19: [-LeftAnalogStickLR, 0, LeftAnalogStickUD].
    let local = [-stick[0], 0.0, stick[1]];
    let world: [f32; 3] = std::array::from_fn(|i| {
        forward[i].mul_add(local[2], up[i].mul_add(local[1], right[i] * local[0]))
    });
    //82898F44..FAC splits world X/Z into positive/negative pad channels;
    //stock actions64/65 subtract those channels again. Keep that ordering.
    let negative_x = if world[0] >= 0.0 { 0.0 } else { -world[0] };
    let positive_x = if world[0] > 0.0 { world[0] } else { 0.0 };
    let negative_z = if world[2] >= 0.0 { 0.0 } else { -world[2] };
    let positive_z = if world[2] <= 0.0 { 0.0 } else { world[2] };
    [positive_x - negative_x, positive_z - negative_z]
}

fn cross_camera_axis(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
    ]
}

fn normalize_camera_axis(v: [f32; 3]) -> [f32; 3] {
    let squared = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    //Independent PC rsqrt seed, retaining both native refinement iterations.
    //No fitted epsilon/fallback and no claim of bit-exact Xenon emulation.
    let mut inverse = squared.sqrt().recip();
    for _ in 0..2 {
        let correction = (-squared).mul_add(inverse * inverse, 1.0);
        inverse = (inverse * 0.5).mul_add(correction, inverse);
    }
    v.map(|component| component * inverse)
}

#[cfg(test)]
#[path = "controls/offboard_tests.rs"]
mod offboard_tests;

#[cfg(test)]
#[path = "controls/bail_tests.rs"]
mod bail_tests;

/// PlayerUI82898B80..BE0: offboard exclusions and the separate State83 path.
fn remap_enabled(offboard:bool,grabbing:bool,state:u32,world_control:bool)->bool {
    if offboard {!grabbing && state!=503}else{world_control}
}
impl PlayerControls {
    pub(crate) fn sampled_camera_axes(&self)->Option<[f32;2]> {self.offboard_axes}
}
