//! Source stages 0x82b36ad4..0x82b36eb4 on the existing contact owner.
//! The enclosing completed-frame traversal calls these once, in source order.

use super::{PhysicalContactInfo, SkateboardContactSurface, SkateboardSurfaceWinner, physics_surface_type};
use crate::simulation_assembly_backend::constraint_session::physical_island::{
    SimulationIslandBodyHandle, contact_owners::SimulationContactEntityOwner,
};
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::{convert::Infallible, fmt};

#[cfg(test)]
#[path = "skateboard_contact_prefix_tests.rs"]
mod tests;

const RESET_COLOR: [f32; 4] = [
    f32::from_bits(0x3dcc_cccd),
    f32::from_bits(0x3dcc_cccd),
    f32::from_bits(0x3e4c_cccd),
    1.0,
];
const CONTACT_COLOR: [f32; 4] = [
    f32::from_bits(0x3f19_999a),
    f32::from_bits(0x3f19_999a),
    f32::from_bits(0x3f19_999a),
    1.0,
];

pub(crate) trait SkateboardContactDebug {
    type Error;
    fn start(&mut self, part: usize) -> Result<i32, Self::Error>;
    fn end(&mut self, part: usize) -> Result<i32, Self::Error>;
    fn store_color(&mut self, index: i32, color: [f32; 4]) -> Result<(), Self::Error>;
}

pub(crate) trait SkateboardPostContactServices: ReckoningNumeric {
    type DeckEndTarget;
    fn owner(
        &mut self,
        body: &SimulationIslandBodyHandle,
    ) -> Result<SimulationContactEntityOwner, Self::Error>;
    fn other_group(&mut self, body: &SimulationIslandBodyHandle) -> Result<u32, Self::Error>;
    /// Actual same-board Part6 Body getter, not `Part::GetTransform`.
    fn deck_body_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    fn capture_deck_end_target(&mut self) -> Result<Self::DeckEndTarget, Self::Error>;
    fn deck_end_value(&mut self, target: &Self::DeckEndTarget) -> Result<f32, Self::Error>;
}

pub(crate) struct SkateboardContactPrefixState {
    pub(crate) max_speed: Option<f32>,
    pub(crate) max_velocity: Option<SkeletonRootVector>,
    pub(crate) opposing_min: Option<f32>,
    pub(crate) opposing_max: Option<f32>,
    pub(crate) best_part: Option<usize>,
    pub(crate) best_y: Option<f32>,
    begun: bool,
}
impl SkateboardContactPrefixState {
    pub(crate) const fn uninitialized() -> Self {
        Self {
            max_speed: None,
            max_velocity: None,
            opposing_min: None,
            opposing_max: None,
            best_part: None,
            best_y: None,
            begun: false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardContactPrefixError<S, D> {
    ResetRequired,
    BeginRequired,
    InvalidPart(u32),
    Unready(&'static str),
    Service(S),
    Debug(D),
}
impl<S: fmt::Debug, D: fmt::Debug> fmt::Display for SkateboardContactPrefixError<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResetRequired => f.write_str("contact state source Reset has not run"),
            Self::BeginRequired => f.write_str("contact frame initialization did not complete"),
            Self::InvalidPart(part) => {
                write!(f, "contact Part {part} is outside the seven-part board")
            }
            Self::Unready(field) => write!(f, "contact field {field} is unready"),
            Self::Service(error) => write!(f, "contact source service: {error:?}"),
            Self::Debug(error) => write!(f, "contact debug store: {error:?}"),
        }
    }
}

impl SkateboardContactSurface {
    pub(crate) fn begin_post_selection<D: SkateboardContactDebug>(
        &mut self,
        debug: &mut D,
    ) -> Result<(), SkateboardContactPrefixError<Infallible, D::Error>> {
        use SkateboardContactPrefixError as E;
        if !self.collision_state.reset_completed() {
            return Err(E::ResetRequired);
        }
        self.post_prefix.begun = false;
        for part in 0..7 {
            self.collision_state.in_contact[part] = Some(false);
            self.parts[part].in_contact = false;
            self.collision_state.surfaces.set_validity(part, 1);
            self.mirror_first_physics_word();
            self.post_prefix.max_speed = Some(0.0);
            self.post_prefix.max_velocity = Some(SkeletonRootVector::known([0.0; 4]));
            recolor(debug, part, false).map_err(E::Debug)?;
        }
        self.collision_state.flags.and_mask(0x87ff_ffff);
        self.post_prefix.opposing_min = Some(1.0);
        self.post_prefix.opposing_max = Some(-1.0);
        self.post_prefix.best_part = None;
        self.post_prefix.best_y = Some(-2.0);
        self.post_prefix.begun = true;
        Ok(())
    }

    pub(crate) fn process_post_contact<
        S: SkateboardPostContactServices,
        D: SkateboardContactDebug,
    >(
        &mut self,
        contact: &PhysicalContactInfo,
        passed_up: SkeletonRootVector,
        services: &mut S,
        debug: &mut D,
    ) -> Result<(), SkateboardContactPrefixError<S::Error, D::Error>> {
        use SkateboardContactPrefixError as E;
        if !self.post_prefix.begun {
            return Err(E::BeginRequired);
        }
        let part = usize::try_from(contact.part())
            .ok()
            .filter(|part| *part < 7)
            .ok_or(E::InvalidPart(contact.part()))?;
        let normal = from_xyz(*contact.normal());
        let position = from_xyz(*contact.position());
        let relative_velocity = from_xyz(*contact.relative_velocity());
        self.pre_contact(part, normal, passed_up, services)?;
        if let Some(other) = contact.other_body() {
            let other_owner = services.owner(other).map_err(E::Service)?;
            let this_owner = services.owner(contact.this_body()).map_err(E::Service)?;
            let same = other_owner.same_owner(&this_owner);
            let group = services.other_group(other).map_err(E::Service)?;
            if same {
                return Ok(());
            }
            self.contact_group(part, group, position, services)?;
        }
        self.commit_contact(
            part,
            normal,
            position,
            relative_velocity,
            contact.surface(),
            debug,
        )
    }

    fn pre_contact<N: ReckoningNumeric, D>(
        &mut self,
        part: usize,
        normal: SkeletonRootVector,
        up: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), SkateboardContactPrefixError<N::Error, D>> {
        use SkateboardContactPrefixError as E;
        if part == 6 {
            let dot = numeric
                .dot3(xyz_scratch(normal), xyz_scratch(up))
                .map_err(E::Service)?;
            let minimum = self
                .post_prefix
                .opposing_min
                .ok_or(E::Unready("opposing minimum"))?;
            let maximum = self
                .post_prefix
                .opposing_max
                .ok_or(E::Unready("opposing maximum"))?;
            self.post_prefix.opposing_min = Some(fsel(minimum - dot, dot, minimum));
            self.post_prefix.opposing_max = Some(fsel(maximum - dot, maximum, dot));
        }
        let old_speed = self
            .post_prefix
            .max_speed
            .ok_or(E::Unready("max contact speed"))?;
        let velocity =
            self.collision_state.velocity[part].ok_or(E::Unready("old Part velocity"))?;
        let dot = numeric
            .dot3(xyz_scratch(normal), xyz_scratch(velocity))
            .map_err(E::Service)?;
        let speed = f32::from_bits(dot.to_bits() ^ 0x8000_0000);
        if speed > old_speed {
            self.post_prefix.max_speed = Some(speed);
            self.post_prefix.max_velocity =
                Some(scale(normal, f32::from_bits(speed.to_bits() ^ 0x8000_0000)));
        }
        Ok(())
    }

    fn contact_group<S: SkateboardPostContactServices, D>(
        &mut self,
        part: usize,
        group: u32,
        position: SkeletonRootVector,
        services: &mut S,
    ) -> Result<(), SkateboardContactPrefixError<S::Error, D>> {
        use SkateboardContactPrefixError as E;
        match group {
            4 => {
                let body = services.deck_body_transform().map_err(E::Service)?;
                let columns = body.columns();
                let displacement = subtract(position, columns[3]);
                let dot = services
                    .dot3(xyz_scratch(columns[1]), xyz_scratch(displacement))
                    .map_err(E::Service)?;
                let horizontal = subtract(displacement, scale(columns[1], dot));
                // Only normalized XYZ feeds the following dot. The scratch W
                // never becomes a retained lane or a body-transform producer.
                let normalized = try_normalize_safe(xyz_scratch(horizontal), [0.0; 4], services)
                    .map_err(E::Service)?;
                let target = services.capture_deck_end_target().map_err(E::Service)?;
                let dot = services
                    .dot3(normalized, xyz_scratch(columns[2]))
                    .map_err(E::Service)?;
                let threshold = services.deck_end_value(&target).map_err(E::Service)?;
                let absolute = f32::from_bits(dot.to_bits() & 0x7fff_ffff);
                if absolute > threshold {
                    self.collision_state.flags.or_mask(0x4000_0000);
                }
            }
            15 | 16 => {
                self.collision_state.flags.or_mask(0x2000_0000);
                self.collision_state.surfaces.set_validity(part, 0);
                self.mirror_first_physics_word();
            }
            14 => self.collision_state.flags.or_mask(0x1000_0000),
            _ => {}
        }
        Ok(())
    }

    fn commit_contact<S, D: SkateboardContactDebug>(
        &mut self,
        part: usize,
        normal: SkeletonRootVector,
        position: SkeletonRootVector,
        relative_velocity: SkeletonRootVector,
        surface: u32,
        debug: &mut D,
    ) -> Result<(), SkateboardContactPrefixError<S, D::Error>> {
        use SkateboardContactPrefixError as E;
        let in_contact = self.collision_state.in_contact[part].ok_or(E::Unready("in contact"))?;
        let wins = if in_contact {
            normal.xyz[1]
                > self.collision_state.normal[part]
                    .ok_or(E::Unready("retained normal"))?
                    .xyz[1]
        } else {
            true
        };
        if wins {
            self.collision_state.normal[part] = Some(normal);
            self.collision_state.position[part] = Some(position);
            self.collision_state.relative_velocity[part] = Some(relative_velocity);
            self.collision_state.pattern_type[part] = Some((surface >> 12) & 15);
            self.collision_state
                .surfaces
                .set_physics_type(part, physics_surface_type(surface));
            self.parts[part].winner = Some(SkateboardSurfaceWinner {
                normal_y: normal.xyz[1],
                physics_type: physics_surface_type(surface),
            });
            self.collision_state.audio_type[part] = Some(surface & 127);
        }
        let retained_y = self.collision_state.normal[part]
            .ok_or(E::Unready("retained normal"))?
            .xyz[1];
        if retained_y > self.post_prefix.best_y.ok_or(E::Unready("best normal Y"))? {
            self.post_prefix.best_part = Some(part);
            self.post_prefix.best_y = Some(retained_y);
        }
        self.collision_state.in_contact[part] = Some(true);
        self.parts[part].in_contact = true;
        recolor(debug, part, true).map_err(E::Debug)
    }

    fn mirror_first_physics_word(&mut self) {
        if let (Some(winner), Some(physics)) = (
            &mut self.parts[0].winner,
            self.collision_state.surfaces.physics_type(0),
        ) {
            winner.physics_type = physics;
        }
    }
}

fn recolor<D: SkateboardContactDebug>(
    debug: &mut D,
    part: usize,
    contact: bool,
) -> Result<(), D::Error> {
    let (mut index, mut end) = if contact {
        let end = debug.end(part)?;
        (debug.start(part)?, end)
    } else {
        let start = debug.start(part)?;
        (start, debug.end(part)?)
    };
    while index < end {
        let current = index;
        index = index.wrapping_add(1);
        debug.store_color(current, if contact { CONTACT_COLOR } else { RESET_COLOR })?;
        end = debug.end(part)?;
    }
    Ok(())
}

fn from_xyz(xyz: [f32; 3]) -> SkeletonRootVector {
    SkeletonRootVector { xyz, w: None }
}
fn xyz_scratch(vector: SkeletonRootVector) -> [f32; 4] {
    [vector.xyz[0], vector.xyz[1], vector.xyz[2], 0.0]
}
fn scale(vector: SkeletonRootVector, factor: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: vector.xyz.map(|lane| lane * factor),
        w: vector.w.map(|lane| lane * factor),
    }
}
fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}
fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
