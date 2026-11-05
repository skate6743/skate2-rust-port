//! Full ordered Skeleton::ProcessAnimAttributes dispatch82BDA0D0. The game
//! supplies actual stock settings, pose and input map, and retains owned state.
use super::{
    attribute_finalization::{Accumulator, FinalizationInput, JumpAttributeState},
    catalog,
    contact_events::{self, ContactEventPose, ContactEventState},
    extended_attributes::{self, ExtendedAttributes},
    scalar_attributes::{self, AnimationControlOutput, DispatchError, ScalarAttributeInputs},
};
use crate::{animation::output::attributes::AnimationAttribute, input::controller::ActionMap};

pub fn process(
    attributes: &[AnimationAttribute],
    pose: &ContactEventPose<'_>,
    fields: &mut ScalarAttributeInputs,
    extra: &mut ExtendedAttributes,
    contacts: &mut ContactEventState,
    cached_jump: &mut JumpAttributeState,
    output: &mut AnimationControlOutput,
    finalization: FinalizationInput,
    mut input_map: Option<&mut dyn ActionMap>,
) -> Result<(), String> {
    let mut accumulator = Accumulator::new();
    for (index, attribute) in attributes.iter().enumerate() {
        let result = (|| {
            if attribute.kind == 3 {
                return contact_events::dispatch(attribute, pose, fields, contacts);
            }
            match scalar_attributes::dispatch_attribute(attribute, fields, output) {
                Ok(()) => return Ok(()),
                Err(DispatchError::KnownScalarUnavailable { .. }) => {}
                Err(error) => return Err(format!("Invalid scalar payload: {error:?}")),
            }
            let name = catalog::lookup(attribute.name).unwrap().name;
            let scalar = || {
                attribute.payload.0[0]
                    .map(f32::from_bits)
                    .ok_or_else(|| format!("{name}: scalar payload absent"))
            };
            if accumulator.dispatch(name, scalar, fields, cached_jump, &mut input_map)?
                || extended_attributes::dispatch(name, scalar, fields, extra, output)?
            {
                Ok(())
            } else {
                Err(format!("Unimplemented known Skeleton attribute {name}"))
            }
        })();
        result.map_err(|error| format!("Skeleton attribute{index}: {error}"))?;
    }
    accumulator.finish(fields, extra, cached_jump, finalization, input_map);
    Ok(())
}
