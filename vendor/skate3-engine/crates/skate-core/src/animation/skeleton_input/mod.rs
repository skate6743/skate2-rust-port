//! Concrete Skeleton animation-attribute consumers. These do not replace the
//! remaining ProcessInput and ProcessData pose/IK stages.
pub mod attribute_finalization;
pub mod catalog;
pub mod contact_events;
pub mod extended_attributes;
pub mod name;
pub mod process_attributes;
pub mod scalar_attributes;

#[cfg(test)]
mod tests;
