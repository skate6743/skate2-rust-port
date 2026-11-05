//! Actor::Teleport 0x825ed1c8..0x825ed220, after a separate source-selected
//! respawn transform has been chosen. This only publishes the Actor's reset.
use super::{ActorConstruction, ActorConstructionError};

#[derive(Debug)]
pub(crate) enum ActorTeleportError {
    Actor(ActorConstructionError),
    Missing(&'static str),
    AnimationLock,
}

impl std::fmt::Display for ActorTeleportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => error.fmt(f),
            Self::Missing(owner) => write!(f, "Actor::Teleport missing {owner}"),
            Self::AnimationLock => f.write_str("Actor SkaterAnim lock poisoned"),
        }
    }
}

impl ActorConstruction {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_skate3_teleport_publication(&self)
        -> Result<(u32,crate::skeleton_root_transforms::SkeletonRootMatrix,bool,bool),ActorTeleportError> {
        let state=self.prefix().map_err(ActorTeleportError::Actor)?;
        let fields=state.fields.as_ref().ok_or(ActorTeleportError::Missing("initial fields"))?;
        let reset=state.external_player_reset.as_ref().ok_or(ActorTeleportError::Missing("external-reset allocation"))?;
        let flags=state.input_generation_flags.ok_or(ActorTeleportError::Missing("input flags"))?;
        Ok((fields.teleport_count,reset.transform,reset.on_board,flags & 0x2000_0000 != 0))
    }

    /// PC mod counterpart of Actor.Update825f1138's reset acknowledgement.
    /// Clear only the request the alternate solver actually consumed; a newer
    /// Actor reset must remain pending. No native physics or animation runs.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_acknowledge_skate3_teleport(&self, generation:u32)->Result<bool,ActorTeleportError> {
        let mut state=self.prefix().map_err(ActorTeleportError::Actor)?;
        if state.fields.as_ref().ok_or(ActorTeleportError::Missing("initial fields"))?.teleport_count != generation {
            return Ok(false);
        }
        let flags=state.input_generation_flags.as_mut().ok_or(ActorTeleportError::Missing("input flags"))?;
        *flags &= !0x2000_0000;
        Ok(true)
    }

    /// Read Actor::Teleport's generation and retained reset without mutating it.
    pub(crate) fn try_teleport_publication(
        &self,
    ) -> Result<(u32, crate::skeleton_root_transforms::SkeletonRootMatrix), ActorTeleportError> {
        let state = self.prefix().map_err(ActorTeleportError::Actor)?;
        let fields = state.fields.as_ref().ok_or(ActorTeleportError::Missing("initial fields"))?;
        let reset = state.external_player_reset.as_ref().ok_or(ActorTeleportError::Missing("external-reset allocation"))?;
        Ok((fields.teleport_count, reset.transform))
    }

    /// `mLastSetFootForward` (+0x240) as its `FootForward` enum, RIGHT being 0.
    pub(crate) fn try_last_set_foot_forward(&self) -> Result<u32, ActorTeleportError> {
        let state = self.prefix().map_err(ActorTeleportError::Actor)?;
        let fields = state.fields.as_ref().ok_or(ActorTeleportError::Missing("initial fields"))?;
        Ok(u32::from(!fields.last_foot_forward_right))
    }

    /// `Actor::SpawnFootForwardStance`825ed638: `SetSpawnFootForwardStance` on
    /// the published SkaterAnim (Actor+0x1fc minus 0x52e0, vtable +0xc0), then
    /// `mLastSetFootForward` (+0x240).
    pub(crate) fn try_spawn_foot_forward_stance(
        &self,
        foot_forward: u32,
    ) -> Result<(), ActorTeleportError> {
        use ActorTeleportError as E;
        self.published_camera_animation()
            .map_err(E::Actor)?
            .ok_or(E::Missing("SkaterAnim"))?
            .lock()
            .map_err(|_| E::AnimationLock)?
            .try_set_spawn_foot_forward_stance(foot_forward)
            .map_err(E::Missing)?;
        self.prefix()
            .map_err(E::Actor)?
            .fields
            .as_mut()
            .ok_or(E::Missing("initial fields"))?
            .last_foot_forward_right = foot_forward == 0;
        Ok(())
    }

    /// `Actor::SetTransform` 0x825ed698: `mSetTransformCount` counts it, then
    /// `Teleport(transform, true)`.
    pub(crate) fn try_set_transform(
        &self,
        transform: &crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), ActorTeleportError> {
        {
            let mut state = self.prefix().map_err(ActorTeleportError::Actor)?;
            let fields = state.fields.as_mut().ok_or(ActorTeleportError::Missing("initial fields"))?;
            fields.set_transform_count = fields.set_transform_count.wrapping_add(1);
        }
        self.try_publish_teleport(transform, true)
    }

    /// `Actor::Teleport` publishes the caller's chosen matrix to the existing
    /// external-reset allocation. The breadcrumb/AI respawn choice is upstream.
    pub(crate) fn try_publish_teleport(
        &self,
        transform: &crate::skeleton_root_transforms::SkeletonRootMatrix,
        on_board: bool,
    ) -> Result<(), ActorTeleportError> {
        use ActorTeleportError as E;
        let mut state = self.prefix().map_err(E::Actor)?;
        if state.fields.is_none() {
            return Err(E::Missing("initial fields"));
        }
        if state.external_player_reset.is_none() {
            return Err(E::Missing("external-reset allocation"));
        }
        if state.input_generation_flags.is_none() {
            return Err(E::Missing("flags +600"));
        }

        let next_count = state
            .fields
            .as_ref()
            .expect("checked")
            .teleport_count
            .wrapping_add(1);
        state.fields.as_mut().expect("checked").teleport_count = next_count;
        let reset = state.external_player_reset.as_mut().expect("checked");
        for column in 0..4 {
            reset
                .transform
                .set_column(column, transform.columns()[column]);
        }
        reset.on_board = on_board;
        let flags = state.input_generation_flags.expect("checked");
        state.fields.as_mut().expect("checked").time_since_fade_in = 0.0;
        state.input_generation_flags = Some(flags | 0x2000_0000);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_construction::{
        animation_mount::ExternalPlayerResetStorage, tests::actor_with_listener,
    };

    #[test]
    #[cfg(feature = "skate3-runtime")]
    fn skate3_acknowledgement_cannot_clear_a_newer_actor_reset() {
        let (actor, _route) = actor_with_listener();
        let matrix = crate::skeleton_root_transforms::SkeletonRootMatrix::known([0.;16]);
        {
            let mut state = actor.prefix().unwrap();
            state.external_player_reset=Some(ExternalPlayerResetStorage { transform:matrix,on_board:true });
            state.input_generation_flags=Some(0x0800_0000);
        }
        actor.try_publish_teleport(&matrix,true).unwrap();
        let generation=actor.try_teleport_publication().unwrap().0;
        actor.try_publish_teleport(&matrix,false).unwrap();
        assert!(!actor.try_acknowledge_skate3_teleport(generation).unwrap());
        assert_eq!(actor.try_input_generation_flags().unwrap(),0x2800_0000);
        assert!(actor.try_acknowledge_skate3_teleport(generation+1).unwrap());
        assert_eq!(actor.try_input_generation_flags().unwrap(),0x0800_0000);
        assert_eq!(actor.try_teleport_publication().unwrap().0,generation+1);
    }

    #[test]
    fn publishes_all_matrix_lanes_and_actor_reset_flag_from_existing_owner() {
        let (actor, _route) = actor_with_listener();
        let matrix = crate::skeleton_root_transforms::SkeletonRootMatrix::known(
            std::array::from_fn(|index| f32::from_bits(0x3f00_0000 + index as u32 * 0x10000)),
        );
        assert!(matches!(
            actor.try_publish_teleport(&matrix, true),
            Err(ActorTeleportError::Missing("external-reset allocation"))
        ));
        {
            let mut state = actor.prefix().unwrap();
            state.external_player_reset = Some(ExternalPlayerResetStorage {
                transform: crate::skeleton_root_transforms::SkeletonRootMatrix::known([0.0; 16]),
                on_board: false,
            });
            state.input_generation_flags = Some(0x0800_0000);
            state.fields.as_mut().unwrap().time_since_fade_in = 3.25;
        }
        actor.try_publish_teleport(&matrix, true).unwrap();
        let state = actor.prefix().unwrap();
        let fields = state.fields.as_ref().unwrap();
        let reset = state.external_player_reset.as_ref().unwrap();
        assert_eq!(fields.teleport_count, 1);
        assert_eq!(fields.time_since_fade_in.to_bits(), 0);
        assert_eq!(state.input_generation_flags, Some(0x2800_0000));
        assert_eq!(
            reset.transform.known_lanes().unwrap().map(f32::to_bits),
            matrix.known_lanes().unwrap().map(f32::to_bits)
        );
        assert!(reset.on_board);
        drop(state);
        {
            let mut state = actor.prefix().unwrap();
            state.fields.as_mut().unwrap().teleport_count = u32::MAX;
        }
        let mut unknown_w = matrix;
        unknown_w.set_column(
            3,
            crate::skeleton_root_transforms::SkeletonRootVector {
                xyz: matrix.columns()[3].xyz,
                w: None,
            },
        );
        actor.try_publish_teleport(&unknown_w, false).unwrap();
        let state = actor.prefix().unwrap();
        assert_eq!(state.fields.as_ref().unwrap().teleport_count, 0);
        assert!(!state.external_player_reset.as_ref().unwrap().on_board);
        assert_eq!(
            state
                .external_player_reset
                .as_ref()
                .unwrap()
                .transform
                .columns()[3]
                .w,
            None
        );
    }
}
