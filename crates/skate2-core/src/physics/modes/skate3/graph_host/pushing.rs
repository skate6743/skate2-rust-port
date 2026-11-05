//! MotionGraph pushing operations. The stock graph owns activation and order;
//! these handlers only mutate native push properties and animation attributes.
use super::pushing_settings::PushingSettings;
use skate3_core::graph::intents::IntentMap;
use skate3_core::riding::push_behaviors::{
    FirstPushStrength, PushCycle, PushFootFrame, PushIntents, PushState,
};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PushOperation {
    InitPush,
    ComputeFirstPushStrength,
    ComputeTargetCoefsFromSpeedAndStrength { regular_attributes: bool },
    ComputeRepushDeadline { regular_attributes: bool },
    SetPushCoefs { on_first_update_only: bool },
    PushCycle { new_push_name: String },
    PushOut { is_right_foot: bool },
}
impl PushOperation {
    /// Defaults verified in TU3 factories82BC7ED0/7F98/80B8/8180/8248.
    /// None means this is another operation, for the enclosing host to resolve.
    pub fn from_attributes(attributes: &Attributes<'_>) -> Option<Self> {
        Some(match attributes.text("name")? {
            "InitPush" => Self::InitPush,
            "ComputeFirstPushStrength" => Self::ComputeFirstPushStrength,
            "ComputeTargetCoefsFromSpeedAndStrength" => {
                Self::ComputeTargetCoefsFromSpeedAndStrength {
                    regular_attributes: attributes.boolean_byte("regularAttributes", 1) != 0,
                }
            }
            "ComputeRepushDeadline" => Self::ComputeRepushDeadline {
                regular_attributes: attributes.boolean_byte("regularAttributes", 1) != 0,
            },
            "SetPushCoefs" => Self::SetPushCoefs {
                on_first_update_only: attributes.boolean_byte("onFirstUpdateOnly", 0) != 0,
            },
            "PushCycle" => Self::PushCycle {
                new_push_name: attributes
                    .text("newPushName")
                    .unwrap_or("LeftPush")
                    .to_owned(),
            },
            "PushOut" => Self::PushOut {
                is_right_foot: attributes.boolean_byte("isRightFoot", 1) != 0,
            },
            _ => return None,
        })
    }
}

/// The scalar SettableAtt values used here are untimed (begin/end=-1),
/// kind0/status6, as published by82D19100. `normalized` is the control flag
/// consumed by the animation tree, not a request to clamp the stored scalar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettableAttribute {
    pub name: &'static str,
    pub value: f32,
    pub normalized: bool,
    pub sequence_id: i32,
}

pub trait PushAnimationSink {
    /// Apply in call order to the active animation controller. Upsert the first
    /// matching (name,sequence_id), or append, following TU382D19100.
    fn set_attribute(&mut self, attribute: SettableAttribute);
}
impl PushAnimationSink for Vec<SettableAttribute> {
    fn set_attribute(&mut self, attribute: SettableAttribute) {
        if let Some(existing) = self.iter_mut().find(|existing| {
            existing.name == attribute.name && existing.sequence_id == attribute.sequence_id
        }) {
            *existing = attribute;
        } else {
            self.push(attribute);
        }
    }
}

pub struct PushContext<'a> {
    pub settings: &'a PushingSettings,
    pub shared: &'a mut PushState,
    /// The current MG list: Pushing carries accumulated AG-held seconds.
    pub motion_intents: &'a IntentMap,
    pub forward_speed: f32,
    pub delta_seconds: f32,
    pub time_since_teleport: f32,
    /// ISkaterAnim::GetIsSwitch (TU382B97128), distinct from mirror/natural stance.
    pub is_switch: bool,
    /// None only for an actor without a physical interface, as in82595B28.
    /// A physical actor must supply its actual skeleton/reckoning outputs.
    pub foot_frame: Option<PushFootFrame>,
}
impl PushContext<'_> {
    fn intents(&self, configured_name: &str) -> PushIntents {
        PushIntents {
            pushing: self.motion_intents.get("Pushing").copied(),
            new_push: self.motion_intents.contains_key("NewPush"),
            configured_foot: self.motion_intents.contains_key(configured_name),
        }
    }
}

/// Allocate one per graph activation. Its first-push/cycle data must not be
/// shared between separate active nodes, nor retained across a fresh activation.
#[derive(Clone, Debug)]
pub struct PushInstance {
    pub operation: PushOperation,
    first_strength: Option<FirstPushStrength>,
    cycle: Option<PushCycle>,
    first_update: bool,
}
impl PushInstance {
    pub fn new(operation: PushOperation) -> Self {
        Self {
            operation,
            first_strength: None,
            cycle: None,
            first_update: false,
        }
    }
    pub fn begin(&mut self, context: &mut PushContext<'_>, sink: &mut impl PushAnimationSink) {
        match &self.operation {
            PushOperation::InitPush => context.shared.initialize_push(), //82BAD5B8
            PushOperation::ComputeFirstPushStrength => {
                self.first_strength = Some(FirstPushStrength::begin(
                    context.time_since_teleport,
                    context.settings.teleport_window,
                )); //82BAD6E8
            }
            PushOperation::SetPushCoefs { .. } => self.first_update = true, //82BACEF0
            PushOperation::PushCycle { new_push_name } => {
                let intents = context.intents(new_push_name);
                self.cycle = Some(PushCycle::begin(
                    context.shared,
                    &context.settings.curves,
                    intents,
                ));
            }
            PushOperation::PushOut { is_right_foot } => {
                //82BAE210 ->82595B28; output writes are begin-only and ordered X,Y.
                let [x, y] = context
                    .foot_frame
                    .map_or([0.0; 2], |frame| frame.out_distance(*is_right_foot));
                publish(sink, "OutDistanceX", x, false);
                publish(sink, "OutDistanceY", y, false);
            }
            _ => {}
        }
    }
    pub fn update(
        &mut self,
        context: &mut PushContext<'_>,
        sink: &mut impl PushAnimationSink,
    ) -> Result<(), String> {
        match &self.operation {
            PushOperation::ComputeFirstPushStrength => {
                self.first_strength
                    .as_mut()
                    .ok_or("ComputeFirstPushStrength updated before Begin")?
                    .update(
                        context.shared,
                        &context.settings.curves,
                        context.motion_intents.get("Pushing").copied(),
                        context.forward_speed,
                        context.delta_seconds,
                    );
            }
            PushOperation::ComputeTargetCoefsFromSpeedAndStrength { regular_attributes } => {
                context.shared.target = context
                    .settings
                    .attributes(*regular_attributes, context.is_switch)
                    .target(context.forward_speed, context.shared.current_push_dv); //82BAD258/82BAD260
                if trace_push_coefs() {
                    eprintln!(
                        "skate3 push coefs: regular_attributes={} switch={} set={} speed={:.3} dv={:.3} target={:?}",
                        regular_attributes, context.is_switch,
                        if *regular_attributes != context.is_switch { "regular" } else { "mongo" },
                        context.forward_speed, context.shared.current_push_dv, context.shared.target
                    );
                }
            }
            PushOperation::SetPushCoefs {
                on_first_update_only,
            } => {
                if !on_first_update_only || self.first_update {
                    context.shared.current = context.settings.curves.update_blend(
                        context.shared.current,
                        context.shared.target,
                        context.forward_speed,
                        context.delta_seconds,
                    ); //82BACF50
                    let values = context.shared.current;
                    publish(sink, "HStr_Vel_B", values.hstr_vel_b, true);
                    publish(sink, "LStr_Vel_B", values.lstr_vel_b, true);
                    publish(sink, "Vel_E", values.vel_e, true);
                }
                self.first_update = false; //82BACF00
            }
            PushOperation::PushCycle { new_push_name } => {
                let intents = context.intents(new_push_name);
                self.cycle
                    .as_mut()
                    .ok_or("PushCycle updated before Begin")?
                    .update(
                        context.shared,
                        &context.settings.curves,
                        intents,
                        context.forward_speed,
                        context.settings.maximum_holding_acceleration,
                    );
            }
            _ => {}
        }
        Ok(())
    }
    pub fn end(&mut self, context: &mut PushContext<'_>) {
        if let PushOperation::ComputeRepushDeadline { regular_attributes } = &self.operation {
            context.shared.out_factor = context
                .settings
                .attributes(*regular_attributes, context.is_switch)
                .out_factor(
                    context.forward_speed,
                    context.shared.current_push_dv,
                    context.settings.out_speed_weight,
                    context.settings.maximum_out_factor,
                ); //82BADA90
        }
    }
}
fn publish(sink: &mut impl PushAnimationSink, name: &'static str, value: f32, normalized: bool) {
    sink.set_attribute(SettableAttribute {
        name,
        value,
        normalized,
        sequence_id: -1,
    });
}

/// SKATE2_TRACE_SKATE3_PUSH_COEFS: each ComputeTargetCoefsFromSpeedAndStrength update.
fn trace_push_coefs() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_PUSH_COEFS").is_some())
}

#[cfg(any())]
#[path = "tests/pushing.rs"]
mod tests;
