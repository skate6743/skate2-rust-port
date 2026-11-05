//! Authored ActionGraph intent behavior configuration.
use skate_data::state_graph::{
    attributes::Attributes,
    binding::{Node, OperationFactory, OperationKind},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub name: Option<String>,
    pub mg_intent: Option<String>,
    pub mg_intent_mag: Option<String>,
    pub mg_intent_angle: Option<String>,
    pub ag_intent: Option<String>,
    pub text: Option<String>,
    pub float_bits: Option<u32>,
    pub boolean_byte: Option<u8>,
    pub default_value: Option<f32>,
    pub scale: Option<f32>,
    pub on_update: bool,
    pub filters: [u32; 4],
    pub angle_filter: u32,
    pub negate_on_mirror: bool,
}

impl Parameter {
    fn from_attributes(attributes: &Attributes<'_>) -> Self {
        Self {
            name: attributes.text("name").map(str::to_owned),
            mg_intent: attributes.text("MGIntent").map(str::to_owned),
            mg_intent_mag: attributes.text("MGIntentMag").map(str::to_owned),
            mg_intent_angle: attributes.text("MGIntentAngle").map(str::to_owned),
            ag_intent: attributes.text("AGIntent").map(str::to_owned),
            text: attributes.text("value").map(str::to_owned),
            float_bits: attributes.get("value").map(|value| value.float_bits),
            boolean_byte: attributes.get("value").map(|value| value.boolean_byte),
            default_value: attributes
                .get("defaultValue")
                .map(|value| f32::from_bits(value.float_bits)),
            scale: attributes
                .get("scale")
                .map(|value| f32::from_bits(value.float_bits)),
            // Both TU3 constructors82BA23C0/82BA1768 default this to true.
            on_update: attributes.boolean_byte("onUpdate", 1) != 0,
            filters: [
                filter(
                    attributes
                        .text("filter1")
                        .or_else(|| attributes.text("filter")),
                ),
                filter(attributes.text("filter2")),
                filter(attributes.text("fakieFilter")),
                filter(attributes.text("mirrorFilter")),
            ],
            angle_filter: filter(attributes.text("angleFilter")),
            //BoardAdjust82BA2A90 passes true as the constructor default.
            negate_on_mirror: attributes.boolean_byte("negateOnMirror", 1) != 0,
        }
    }
}

fn filter(value: Option<&str>) -> u32 {
    // TU382F847C0..82F84850 initialize the names consumed by82BA0A48.
    match value {
        Some("negate") => 1,
        Some("abs") => 2,
        Some("oneMinus") => 3,
        Some("clamp") => 4,
        Some("angleFlip") => 5,
        Some("angleRot90") => 6,
        Some("angleRotN90") => 7,
        _ => 0,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionOperation {
    Condition(skate3_core::graph::conditions::ActionCondition),
    ExtraCondition(super::action_conditions::ActionCondition),
    PhysicsRequestsDismount,
    IsLandingOnBoard,
    CreateMgIntent,
    CreateMgTimeIntent,
    CreateConstMgIntent,
    BoardAdjust,
    CreateTrickIntentFromGesture { group: crate::skate3_native::input::gesture_catalog::Group, override_name: Option<String> },
    JuiceHook,
    BodyFlippingSignal,
    /// Diagnostic presentation is owned by this host and has no intent effect.
    PrintText(String),
    Unsupported {
        kind: OperationKind,
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionInstance {
    pub operation: ActionOperation,
    pub config: Parameter,
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionFactoryError(pub String);

impl std::fmt::Display for ActionFactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Default)]
pub struct ActionFactory;

impl ActionFactory {
    fn operation(kind: OperationKind, name: &str) -> ActionOperation {
        match (kind, name) {
            (OperationKind::Condition, "PhysicsRequestsDismount") => {
                ActionOperation::PhysicsRequestsDismount
            }
            (OperationKind::Condition, "IsLandingOnBoard") => ActionOperation::IsLandingOnBoard,
            (OperationKind::Behavior, "CreateMGIntentFromAGIntent") => {
                ActionOperation::CreateMgIntent
            }
            (OperationKind::Behavior, "CreateMGTimeIntentFromAGIntent") => {
                ActionOperation::CreateMgTimeIntent
            }
            (OperationKind::Behavior, "CreateConstMGIntent") => {
                ActionOperation::CreateConstMgIntent
            }
            (OperationKind::Behavior, "BoardAdjust") => ActionOperation::BoardAdjust,
            (OperationKind::Behavior, "JuiceHook") => ActionOperation::JuiceHook,
            (OperationKind::Behavior, "BodyFlippingSignal") => ActionOperation::BodyFlippingSignal,
            _ => ActionOperation::Unsupported {
                kind,
                name: name.to_owned(),
            },
        }
    }
}

impl OperationFactory for ActionFactory {
    type Instance = ActionInstance;
    type Error = ActionFactoryError;

    fn create(
        &mut self,
        kind: OperationKind,
        _parent: Node,
        attributes: &Attributes<'_>,
    ) -> Result<Option<Self::Instance>, Self::Error> {
        let Some(name) = attributes.text("name") else {
            return Err(ActionFactoryError("operation has no name attribute".into()));
        };
        Ok(Some(ActionInstance {
            operation: if kind == OperationKind::Condition {
                if let Some(condition) =
                    super::action_conditions::ActionCondition::parse(attributes)
                        .map_err(ActionFactoryError)?
                {
                    ActionOperation::ExtraCondition(condition)
                } else {
                    super::condition_nodes::parse(attributes)
                        .map_err(ActionFactoryError)?
                        .map(ActionOperation::Condition)
                        .unwrap_or_else(|| Self::operation(kind, name))
                }
            } else {
                if kind == OperationKind::Behavior && name == "CreateTrickIntentFromGesture" {
                    ActionOperation::CreateTrickIntentFromGesture {
                        group: crate::skate3_native::input::gesture_catalog::Group::parse(attributes.text("group").unwrap_or("Square")).map_err(ActionFactoryError)?,
                        override_name: attributes.text("override").map(str::to_owned),
                    }
                } else if kind == OperationKind::Behavior && name == "PrintText2D" {
                    ActionOperation::PrintText(attributes.text("text").unwrap_or("").to_owned())
                } else {
                    Self::operation(kind, name)
                }
            },
            config: Parameter::from_attributes(attributes),
            parameters: Vec::new(),
        }))
    }

    fn add_parameter(
        &mut self,
        instance: &mut Self::Instance,
        attributes: &Attributes<'_>,
    ) -> Result<(), Self::Error> {
        instance
            .parameters
            .push(Parameter::from_attributes(attributes));
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedOperation {
    pub kind: OperationKind,
    pub name: String,
}

impl ActionInstance {
    pub fn unsupported(&self) -> Option<UnsupportedOperation> {
        match &self.operation {
            ActionOperation::Unsupported { kind, name } => Some(UnsupportedOperation {
                kind: *kind,
                name: name.clone(),
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActionInstances {
    pub operations: Vec<ActionInstance>,
}

impl ActionInstances {
    pub fn new(operations: Vec<ActionInstance>) -> Self {
        Self { operations }
    }

    pub fn get(&self, operation: usize) -> Option<&ActionInstance> {
        self.operations.get(operation)
    }
}
