//! Retained surface-only projection of board `PostPhysics` 0x82b36a58 and
//! `ChoosePhysicsSurfaceType` 0x82b24878. Contact production is shared, not repeated.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "board surface publisher integration pending")
)]

use crate::simulation_assembly_backend::constraint_session::physical_island::{
    SimulationIslandBodyHandle, contact_owners::SimulationContactEntityOwner,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    simulation_assembly_backend::constraint_session::physical_contact_list::{
        PhysicalContactInfo, SimulationPhysicalContactListError, SimulationPhysicalContactLists,
    },
    simulation_contact_jacobian::{
        contact_spy::{SimulationContactSpyError, SimulationContactSpyFrame},
        network_preparation::SimulationManagerContactDomain,
    },
    simulation_rigid_body_pool::SimulationRigidBodyPool,
};
use std::{fmt, marker::PhantomData, sync::Arc};

#[path = "skateboard_contact_surface_choice.rs"]
pub(crate) mod choice;
#[path = "skateboard_contact_motion.rs"]
pub(crate) mod motion;
#[path = "../runtime/skateboard_ground_normal_attributes.rs"]
pub(crate) mod normal_attributes;
#[path = "skateboard_contact_normal_tail.rs"]
pub(crate) mod normal_tail;
#[path = "skateboard_contact_prefix.rs"]
pub(crate) mod prefix;
#[path = "skateboard_contact_state.rs"]
pub(crate) mod state;

#[derive(Debug)]
pub(crate) enum SkateboardContactFrameError {
    Spy(SimulationContactSpyError),
    List(SimulationPhysicalContactListError),
    WrongBody,
    WrongFrame,
}
impl fmt::Display for SkateboardContactFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spy(error) => write!(f, "contact spy frame: {error:?}"),
            Self::List(error) => write!(f, "completed contact list: {error:?}"),
            Self::WrongBody => f.write_str("contact belongs to another body"),
            Self::WrongFrame => f.write_str("contact belongs to another frame"),
        }
    }
}

/// Certifies an actual completed list, including empty frames. The borrowed
/// manager/pool lifetime prevents mutation between current validation and use.
pub(crate) struct CompletedSkateboardContacts<'a> {
    frame: &'a Arc<SimulationContactSpyFrame>,
    contacts: &'a [PhysicalContactInfo],
    current: PhantomData<(
        &'a SimulationManagerContactDomain,
        &'a SimulationRigidBodyPool,
    )>,
}
impl<'a> CompletedSkateboardContacts<'a> {
    pub(crate) fn contacts(&self) -> &[PhysicalContactInfo] {
        self.contacts
    }
    pub(crate) fn try_new(
        frame: &'a Arc<SimulationContactSpyFrame>,
        lists: &'a SimulationPhysicalContactLists,
        manager: &'a SimulationManagerContactDomain,
        pool: &'a SimulationRigidBodyPool,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<Self, SkateboardContactFrameError> {
        frame
            .try_validate_current(manager, pool)
            .map_err(SkateboardContactFrameError::Spy)?;
        lists
            .try_validate_completed_frame(frame)
            .map_err(SkateboardContactFrameError::List)?;
        let contacts = lists.retained_contacts(board);
        for contact in contacts {
            if !contact.this_body().same_as(board) {
                return Err(SkateboardContactFrameError::WrongBody);
            }
            if !Arc::ptr_eq(contact.spy_frame(), frame) {
                return Err(SkateboardContactFrameError::WrongFrame);
            }
        }
        Ok(Self {
            frame,
            contacts,
            current: PhantomData,
        })
    }
}

pub(crate) trait SkateboardContactSurfaceOwners {
    type Error;
    fn owner(
        &mut self,
        body: &SimulationIslandBodyHandle,
    ) -> Result<SimulationContactEntityOwner, Self::Error>;
}
pub(crate) trait SkateboardContactSurfaceCvars {
    type Error;
    fn forced_world_surface(&mut self) -> Result<u32, Self::Error>;
}

/// Only the surface winner dependency is ready here. Native position/velocity
/// and sibling collision responses are intentionally not represented as ready.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkateboardPartSurface {
    pub(crate) in_contact: bool,
    pub(crate) winner: Option<SkateboardSurfaceWinner>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkateboardSurfaceWinner {
    pub(crate) normal_y: f32,
    pub(crate) physics_type: u32,
}
#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardContactSurfaceError<O, C> {
    AlreadyAttempted,
    InvalidPart(u32),
    InvalidVoteType(u32),
    Owner(O),
    Cvar(C),
}

pub(crate) struct SkateboardContactSurface {
    pub(crate) post_timestep: Option<f32>,
    pub(crate) body_post_attempted: Option<Arc<SimulationContactSpyFrame>>,
    pub(crate) body_post_completed: Option<Arc<SimulationContactSpyFrame>>,
    pub(crate) up_post_attempted: Option<Arc<SimulationContactSpyFrame>>,
    pub(crate) up_post_completed: Option<Arc<SimulationContactSpyFrame>>,
    pub(crate) normal_frame: Option<Arc<SimulationContactSpyFrame>>,
    pub(crate) post_prefix: prefix::SkateboardContactPrefixState,
    pub(crate) collision_state: state::SkateboardContactState,
    parts: [SkateboardPartSurface; 7],
    overall: Option<u32>,
    attempted: Option<Arc<SimulationContactSpyFrame>>,
}
impl SkateboardContactSurface {
    pub(crate) fn new() -> Self {
        Self {
            post_timestep: None,
            body_post_attempted: None,
            body_post_completed: None,
            up_post_attempted: None,
            up_post_completed: None,
            normal_frame: None,
            post_prefix: prefix::SkateboardContactPrefixState::uninitialized(),
            collision_state: state::SkateboardContactState::uninitialized(),
            parts: [SkateboardPartSurface {
                in_contact: false,
                winner: None,
            }; 7],
            overall: None,
            attempted: None,
        }
    }
    pub(crate) fn parts(&self) -> &[SkateboardPartSurface; 7] {
        &self.parts
    }
    /// ContactState and surface-selection projection represent the same
    /// native per-Part in-contact byte.
    pub(crate) fn reset_collision_part_before_height(&mut self, part: usize) {
        self.collision_state.reset_part_before_height(part);
        self.parts[part].in_contact = false;
    }
    pub(crate) fn overall(&self) -> Option<u32> {
        self.overall
    }
    pub(crate) fn begin_post_frame(&mut self, completed: &CompletedSkateboardContacts<'_>) -> bool {
        if self
            .attempted
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, completed.frame))
        {
            return false;
        }
        self.attempted = Some(Arc::clone(completed.frame));
        self.normal_frame = None;
        true
    }

    /// The enclosing world validates actual physical-board ownership before
    /// constructing the completed token. Reentry refusal is a host lifetime guard.
    pub(crate) fn try_update<
        O: SkateboardContactSurfaceOwners,
        C: SkateboardContactSurfaceCvars,
    >(
        &mut self,
        completed: &CompletedSkateboardContacts<'_>,
        owners: &mut O,
        cvars: &mut C,
    ) -> Result<(), SkateboardContactSurfaceError<O::Error, C::Error>> {
        use SkateboardContactSurfaceError as E;
        if self
            .attempted
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, completed.frame))
        {
            return Err(E::AlreadyAttempted);
        }
        self.attempted = Some(Arc::clone(completed.frame));
        self.begin_selection();
        for contact in completed.contacts {
            let skip = same_entity(
                contact.other_body().is_some(),
                |other| {
                    owners.owner(if other {
                        contact.other_body().expect("nonnull branch")
                    } else {
                        contact.this_body()
                    })
                },
                SimulationContactEntityOwner::same_owner,
            )
            .map_err(E::Owner)?;
            if skip {
                continue;
            }
            self.select(contact.part(), contact.normal()[1], contact.surface())
                .map_err(E::InvalidPart)?;
        }
        self.finish::<O::Error, C>(cvars)
    }
    pub(crate) fn finish<O, C: SkateboardContactSurfaceCvars>(
        &mut self,
        cvars: &mut C,
    ) -> Result<(), SkateboardContactSurfaceError<O, C::Error>> {
        use SkateboardContactSurfaceError as E;
        let voted = self.vote().map_err(E::InvalidVoteType)?;
        let forced = cvars.forced_world_surface().map_err(E::Cvar)?;
        self.overall = Some(if forced != 0 { forced } else { voted });
        Ok(())
    }
    fn begin_selection(&mut self) {
        for part in &mut self.parts {
            part.in_contact = false;
        }
    }
    fn select(&mut self, part: u32, normal_y: f32, surface: u32) -> Result<(), u32> {
        let entry = self
            .parts
            .get_mut(usize::try_from(part).map_err(|_| part)?)
            .ok_or(part)?;
        if !entry.in_contact || entry.winner.is_some_and(|old| normal_y > old.normal_y) {
            entry.winner = Some(SkateboardSurfaceWinner {
                normal_y,
                physics_type: physics_surface_type(surface),
            });
        }
        entry.in_contact = true;
        Ok(())
    }
    fn vote(&self) -> Result<u32, u32> {
        let mut counts = [0_u32; 12];
        for part in &self.parts[..4] {
            if part.in_contact {
                let kind = part
                    .winner
                    .expect("accepted current part has winner")
                    .physics_type;
                let count = counts
                    .get_mut(usize::try_from(kind).map_err(|_| kind)?)
                    .ok_or(kind)?;
                *count += 1;
            }
        }
        Ok(choose_count_winner(&counts))
    }
}

/// A contact surface word's `ePhysicsSurfaceType` (bits 7 to 11). PC port: a linked Skate 3 data
/// folder's surfaces name types past Skate 2's last, Wipeout (9): Skate 3 votes among 1 to 13
/// (0x82c08818 in its title update 3) and its `SurfacePhysics::Dispatch` (0x82d86018) rides
/// every type past 6 as smooth, as these are here; Skate 2's counts have room for 11.
pub(crate) fn physics_surface_type(surface: u32) -> u32 {
    let kind = (surface >> 7) & 31;
    if kind > 9 && crate::skate3::linked().is_some() { 1 } else { kind }
}

fn choose_count_winner(counts: &[u32; 12]) -> u32 {
    let mut winner = 1;
    let mut best = 0;
    for kind in 1_u32..=9 {
        let count = counts[kind as usize];
        if count > best {
            best = count;
            winner = kind;
        }
    }
    winner
}

/// Shared source admission kernel; no owner read occurs for native null other.
fn same_entity<T, E>(
    has_other: bool,
    mut read: impl FnMut(bool) -> Result<T, E>,
    same: impl FnOnce(&T, &T) -> bool,
) -> Result<bool, E> {
    if !has_other {
        return Ok(false);
    }
    let other = read(true)?;
    let this = read(false)?;
    Ok(same(&other, &this))
}

#[cfg(test)]
#[path = "skateboard_contact_surface_tests.rs"]
mod tests;
