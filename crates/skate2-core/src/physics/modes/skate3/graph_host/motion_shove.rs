//! Original TU3 Shove constructor82BBCCF0, Begin82BBCDD0,
//! Update82BBCDF0 and End82BBD540. The host owns channels and graph values.
use super::motion_animation::MotionAnimation;
use skate3_core::animation::{
    channel_playback::ChannelSettings,
    playback::TransitionSettings,
    playback_parameters::{AttributeSink, ParameterInputs, SettableAttribute},
    skeleton_input::name::encode,
};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub struct ShoveOperation {
    selection: String,
    selection_board: String,
    anticipation: String,
    anticipation_board: String,
}
impl ShoveOperation {
    pub fn parse(a: &Attributes<'_>) -> Result<Option<Self>, String> {
        if a.text("name") != Some("Shove") {
            return Ok(None);
        }
        //The required lookups are virtual8; virtual4 supplies empty defaults
        //only for the two board-specific alternatives.
        Ok(Some(Self {
            selection: a
                .text("Selection")
                .ok_or("Shove requires Selection")?
                .into(),
            selection_board: a.text("SelectionBrd").unwrap_or("").into(),
            anticipation: a.text("Antic").ok_or("Shove requires Antic")?.into(),
            anticipation_board: a.text("AnticBrd").unwrap_or("").into(),
        }))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ShovePhysical {
    ///PhysOutInteraction (bundle52)+56; ProcessOutput82DB7E38 owns this.
    pub interaction_trigger: bool,
    ///PhysOutGround240, the interaction direction published by that producer.
    pub direction: [f32; 4],
    ///PhysOutState75 and PhysOutOffboard311, respectively.
    pub in_biped_category: bool,
    pub board_on_ground: bool,
    ///PhysOutAnimation72, used only while the shove channel is active.
    pub animation_height: f32,
}

#[derive(Clone, Debug)]
pub struct ShoveState {
    pub angle: f32,
    ///Native instance12 records the anticipation selection. Update does not
    ///use this cache to replace the newly observed board state.
    pub board_anticipation: bool,
}
impl ShoveState {
    pub fn new() -> Self {
        Self {
            angle: 0.0,
            board_anticipation: false,
        }
    }
    pub fn begin(&mut self) {
        //82BBCDD0 clears both fields; this does not terminate any channel.
        self.angle = 0.0;
        self.board_anticipation = false;
    }
    pub fn update(
        &mut self,
        operation: &ShoveOperation,
        animation: &mut MotionAnimation,
        physical: ShovePhysical,
        busy_hands: [u32; 2],
        mirrored: bool,
    ) -> Result<(), String> {
        let retrieving = animation.channels.has("RetrieveBoard");
        trace_channels(animation, busy_hands);
        let board_variant = physical.in_biped_category && physical.board_on_ground;
        if animation.motion_intent("GrabWorld").is_some() && !retrieving && busy_hands == [0, 0] {
            if !animation.channels.has("SkitchAntic") && !animation.channels.has("Shove") {
                self.board_anticipation = board_variant;
                start_channel(
                    animation,
                    "SkitchAntic",
                    if board_variant {
                        &operation.anticipation_board
                    } else {
                        &operation.anticipation
                    },
                    true,
                )?;
            }
        } else if animation.channels.has("SkitchAntic") {
            animation.channels.end("SkitchAntic");
        }

        let mut shove = animation.channels.has("Shove");
        if !physical.interaction_trigger || retrieving {
            if shove {
                if retrieving {
                    animation.channels.end("Shove");
                } else {
                    set(animation, b"ShoveDirection", self.angle);
                }
            }
            return Ok(());
        }
        self.angle = direction_angle(physical.direction, mirrored);
        if animation.channels.has("SkitchAntic") {
            animation.channels.end("SkitchAntic");
        }
        if !shove {
            start_channel(
                animation,
                "Shove",
                if board_variant {
                    &operation.selection_board
                } else {
                    &operation.selection
                },
                false,
            )?;
            //Original marks this true after the call, rather than querying
            //the channel manager a second time during the same update.
            shove = true;
        }
        if shove {
            set(animation, b"GstrDistToCog", physical.animation_height);
            set(animation, b"ShoveDirection", self.angle);
        }
        Ok(())
    }
    pub fn end(&mut self, animation: &mut MotionAnimation, keep_channels: bool) {
        //SpecificMotionGraph virtual212=8258FC30, bit18. Native reset clears it.
        //Virtual44 is EndChannelPrematurely82D1D6A8, preserving fade timing.
        if !keep_channels {
            for name in ["SkitchAntic", "Shove"] {
                if animation.channels.has(name) {
                    animation.channels.end(name);
                }
            }
        }
    }
}
fn set(animation: &mut MotionAnimation, name: &[u8], value: f32) {
    animation.set_attribute(SettableAttribute {
        name: encode(name),
        value,
        normalized: false,
        sequence_id: -1,
    });
}
fn start_channel(
    animation: &mut MotionAnimation,
    channel: &str,
    tree: &str,
    anticipation: bool,
) -> Result<(), String> {
    let seconds = f32::from_bits(if anticipation { 0x3e4ccccd } else { 0x3dcccccd });
    let settings = ChannelSettings {
        priority: 0,
        keep_alive: anticipation,
        mirrored: false,
        speed: 1.0,
        blend_in: seconds,
        hold_during_blend_in: false,
        blend_out: seconds,
        hold_during_blend_out: false,
        // Update82BBCDF0 never writes the settings' use-attributes byte (var_D8+1), so it is
        // stack contents there; the shove clip's "Shoving" attribute (Processed2484 bit 19, the
        // strike window ProcessOutput82DB7E38 requires) reaches physics only through it, and
        // shoves land in TU3, so the shove itself uses its attributes. The anticipation does not.
        use_attributes: !anticipation,
    };
    //Vtable16 is TransitionTo82D1D090. Stack booleans at87/95/103 are1:
    //allow resurrection, create missing and use channel weights.
    let transition = TransitionSettings {
        kind: 2,
        seconds: f32::from_bits(0x3dcccccd),
        under: 0,
        matching: 0,
        use_channels_from_weights: true,
    };
    animation.transition_channel(channel, tree, settings, transition, true, true)?;
    Ok(())
}
///82BBD250..D390: original one-refinement reciprocal followed by82473B98.
///Unlike left-stick angle, zero direction has no extra zero-vector guard.
fn direction_angle(direction: [f32; 4], mirrored: bool) -> f32 {
    let x = direction[0];
    let z = if mirrored {
        -direction[2]
    } else {
        direction[2]
    };
    let reciprocal = z.recip();
    let reciprocal = reciprocal.mul_add((-reciprocal).mul_add(z, 1.0), reciprocal);
    let basic = skate3_core::input::angle::atan(x.mul_add(reciprocal, 0.0));
    let sign = x.to_bits() & 0x80000000;
    let angle = if z < 0.0 {
        basic + f32::from_bits(0x40490fdb | sign)
    } else {
        basic
    };
    let angle = if z == 0.0 {
        f32::from_bits(0x3fc90fdb | sign)
    } else {
        angle
    };
    let degrees = angle * f32::from_bits(0x42652ee1);
    if degrees < 0.0 {
        degrees + f32::from_bits(0x43b40000)
    } else {
        degrees
    }
}

/// `SKATE2_TRACE_SHOVE`: the running channels, GrabWorld and the busy hands, once per change.
fn trace_channels(animation: &MotionAnimation, busy_hands: [u32; 2]) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
    if !*TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SHOVE").is_some()) {
        return;
    }
    let names: Vec<&str> = animation.channels.names().collect();
    let text = format!("channels {names:?} grab_world {} busy {busy_hands:?}", animation.motion_intent("GrabWorld").is_some());
    if let Ok(mut last) = LAST.lock() && *last != text {
        eprintln!("shove: {text}");
        *last = text;
    }
}
