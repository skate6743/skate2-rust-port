//! Ground drag calculation82D39100/104 and inertia publication82D392A8..B4.
//! Body::SetLinearDrag82D9CD60 writes inertia fields, not current velocities.
use crate::{
    physics::rigid_body::RetailInertiaDynamics,
    riding::braking::{LinearDragInput, LinearDragSettings, calculate_linear_drag},
};

/// Literal at822F860C, verified in the original TU3 image. It is not replaced
/// by60 or recomputed from the current simulation timestep.
pub const DRAG_FREQUENCY: f32 = f32::from_bits(0x426f_ffff);

#[derive(Clone, Copy, Debug)]
pub struct GroundDragInput {
    pub flags_2468: u32,
    pub absolute_body_speed_2616: f32,
    pub balance_2720: f32,
    pub scalar_2724: f32,
    /// Ground context+1216 vector Y, passed in f1 at82D39100. This is not
    /// Processed speed, a manual output, or a timestep.
    pub ground_normal_y: f32,
}

impl GroundDragInput {
    /// Calculate before the manual-output80 branch at82D39110. The returned
    /// drag must be retained across force submission, then applied afterward.
    pub fn calculate(self, settings: LinearDragSettings) -> f32 {
        calculate_linear_drag(
            LinearDragInput {
                flags_2468: self.flags_2468,
                absolute_body_speed: self.absolute_body_speed_2616,
                balance_2720: self.balance_2720,
                scalar_2724: self.scalar_2724,
                comparison_scalar: self.ground_normal_y,
            },
            settings,
        )
    }
}

/// Native body+24 assembly contains part count+28 and definitions+8; each
///96-byte part resolves body+76 -> inertia+92. Shared inertias remain shared:
/// this binding references the live dynamics objects, not copies per part.
pub struct BodyInertias<'a> {
    pub part_inertia_indices: &'a [usize],
    pub inertias: &'a mut [RetailInertiaDynamics],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragSelection {
    /// Native r5=-1, used by the inspected Ground update and exit callsites.
    AllParts,
    Part(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragBindingError {
    PartOutsideAssembly,
    InertiaOutsideStorage,
}

impl BodyInertias<'_> {
    /// Exact valid-binding setter82D9CD60. Invalid host bindings are rejected
    /// before writes; native would address invalid storage rather than recover.
    /// All-parts order is0..count, with four resolved handles before each group
    /// of four writes and a remaining single-part tail. Only inertia+32 changes.
    pub fn set_linear_drag(
        &mut self,
        drag: f32,
        selection: DragSelection,
    ) -> Result<(), DragBindingError> {
        let selected = match selection {
            DragSelection::AllParts => self.part_inertia_indices,
            DragSelection::Part(part) => self
                .part_inertia_indices
                .get(part..part.saturating_add(1))
                .filter(|parts| parts.len() == 1)
                .ok_or(DragBindingError::PartOutsideAssembly)?,
        };
        if selected.iter().any(|&index| index >= self.inertias.len()) {
            return Err(DragBindingError::InertiaOutsideStorage);
        }
        let mut groups = selected.chunks_exact(4);
        for group in &mut groups {
            let [first, second, third, fourth] = [group[0], group[1], group[2], group[3]];
            let coefficient = drag * DRAG_FREQUENCY;
            self.inertias[first].linear_drag = coefficient;
            self.inertias[second].linear_drag = coefficient;
            self.inertias[third].linear_drag = coefficient;
            self.inertias[fourth].linear_drag = coefficient;
        }
        for &index in groups.remainder() {
            self.inertias[index].linear_drag = drag * DRAG_FREQUENCY;
        }
        Ok(())
    }

    /// Ground82D392B4: after either ordinary tag16 or alternate tag7 submission,
    /// apply the drag retained from82D39104 to all parts. The caller owns the
    /// remaining force-strength flag and ground-correction helpers afterward.
    pub fn apply_ground_drag(&mut self, calculated_drag: f32) -> Result<(), DragBindingError> {
        self.set_linear_drag(calculated_drag, DragSelection::AllParts)
    }
}
