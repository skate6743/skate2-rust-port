//! `SkaterAnim::ResetSkaterAnim` on the existing initialized Actor animation.
//! Its tailcall runs `ChannelAnimatable::ResetChannelAnimatable` exactly once.
use super::{ActorAnimationConstruction, AnimationPointerState};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResetSkaterAnimError {
    Missing(&'static str),
    PoisonedBlendSpaceBuffer,
    PendingBlendSpaceJobWait,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResetToSpawnStanceError {
    MissingPlayer,
}

impl std::fmt::Display for ResetToSpawnStanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResetToSpawnStance requires the retained SkaterAnim player")
    }
}

impl std::fmt::Display for ResetSkaterAnimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "SkaterAnim reset requires {name}"),
            Self::PoisonedBlendSpaceBuffer => f.write_str("SkaterAnim blend-space buffer poisoned"),
            Self::PendingBlendSpaceJobWait => {
                f.write_str("Animatable::Clear requires the active blend-space job wait")
            }
        }
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// `ResetToGivenStance::Begin` 0x82ca2ea0 calls the ISkaterAnim vtable
    /// slot +0x2c, `SkaterAnim::ResetToSpawnStance` 0x82cb77e0.
    pub(crate) fn try_reset_to_spawn_stance(&mut self) -> Result<(), ResetToSpawnStanceError> {
        let player = self
            .player
            .as_mut()
            .ok_or(ResetToSpawnStanceError::MissingPlayer)?;
        // Native reads both values before the first branch. Neither enum is
        // silently narrowed to a boolean; values outside 0/1 take the else.
        let natural = player.natural_stance;
        let spawn_relative = player.spawn_relative_stance;
        if (natural == 1 && spawn_relative == 0) || (natural == 0 && spawn_relative == 1) {
            player.flags_bits &= 0x3fff_ffff; // 82cb7824, SkaterAnim+0x612c
            player.relative_stance = u32::from(natural == 0); // 782c, +0x6140
            player.spawn_relative_stance = 0; // 7830, +0x6144
        } else {
            player.flags_bits |= 0xc000_0000; // 82cb7850
            player.relative_stance = u32::from(natural != 0); // 7854
            player.spawn_relative_stance = 0; // 7858
        }
        player.flags_known_mask |= 0xc000_0000;
        Ok(())
    }

    /// `ResetSkaterAnim` 0x82c8b6a8 followed by its single tailcall to
    /// `ResetChannelAnimatable` 0x82d7d220 and `Animatable::Clear` 0x82d7a6b8.
    /// All pools and vectors remain owned by this initialized occurrence.
    pub(crate) fn try_reset_skater_anim(&mut self) -> Result<(), ResetSkaterAnimError> {
        use ResetSkaterAnimError as E;
        if self.player.is_none()
            || self.before_sprint.is_none()
            || self.base.is_none()
            || self.channel.is_none()
            || self
                .initialization
                .as_ref()
                .and_then(|init| init.channels.as_ref())
                .is_none()
        {
            return Err(E::Missing("initialized SkaterAnim/ChannelAnimatable"));
        }
        #[cfg(feature="skate3-runtime")]
        if let Some(playback)=&mut self.pc_gesture_playback {playback.reset();}
        let player = self.player.as_mut().expect("checked player");
        player.relative_stance = 0; // 82c8b6c0, SkaterAnim+0xe60
        self.before_sprint
            .as_mut()
            .expect("checked suffix")
            .grab_type = 0; // c8, +0xee8
        const KEEP: u32 = 0x0ffd_95ff;
        player.flags_bits &= KEEP; // cc, +0xe4c
        player.flags_known_mask |= !KEEP;

        let init = self
            .initialization
            .as_mut()
            .expect("checked initialization");
        let channels = init.channels.as_mut().expect("checked channels");
        channels.dictionary.clear_preserving_buckets(); // 82d7d230..24c
        channels.free_channels.clear(); // d260
        for (index, channel) in channels.channels.iter_mut().enumerate() {
            // d27c..290 writes only these six fields in each 148-byte Channel.
            channel.parent = None;
            channel.channel_tree = None;
            channel.transitioning = false;
            channel.transition_tree = None;
            channel.base = None;
            channel.priority = 0;
            channels.free_channels.push(index); // d2a0..2d0
        }
        channels.free_trees.clear(); // d324
        for (index, tree) in channels.trees.iter_mut().enumerate() {
            tree.runtime.clear(); // d344, existing `PoolStack::Clear`
            channels.free_trees.push(index); // d348..378
        }
        channels.root = None; // d3c8, ChannelAnimatable::mRootChannel

        self.try_clear_animatable()
    }

    /// Andale.Animatable.Clear82d7a6b8, also called by its destructor82d7ba44.
    /// Shared with ResetSkaterAnim; the retained pools stay with this owner.
    pub(crate) fn try_clear_animatable(&mut self)->Result<(),ResetSkaterAnimError>{
        use ResetSkaterAnimError as E;
        let base=self.base.as_mut().ok_or(E::Missing("Animatable"))?;
        base.settable_attributes.len = 0; // 82d7a700
        base.tree_attributes.len = 0; // a70c
        base.trajectory = Some([[1.0; 4], [0.0, 0.0, 0.0, 1.0], [0.0; 4]]); // a710..71c
        base.is_playing = false; // a720
        base.expired_or_looped_by_time = -1.0; // a724
        base.expired_or_looped = false; // a728
        base.time_to_expire_or_loop = -1.0; // a72c
        base.tree_store.clear_play_list(); // a730
        base.transition_store.clear_play_lists(); // a738
        base.construction_map.len = 0; // a74c, ClearConstructionAttrs
        if base.blend_space_job_started {
            // a754 tests the job pointer first; a75c tests the started byte.
            // Unknown job identity cannot select either branch. The direct
            // native wait (without PostBlendSpace) is not mounted yet.
            match base.blend_space_job {
                AnimationPointerState::Constructed => return Err(E::PendingBlendSpaceJobWait),
                AnimationPointerState::Unwritten => {
                    return Err(E::Missing("known blend-space job pointer"));
                }
                AnimationPointerState::Null => {}
            }
        }
        base.blend_space_data
            .as_ref()
            .ok_or(E::Missing("initialized blend-space data"))?
            .lock()
            .map_err(|_| E::PoisonedBlendSpaceBuffer)?
            .records
            .clear(); // a784, count only; retain the actual buffer
        Ok(())
    }
    /// ChannelAnimatable::~ChannelAnimatable82cb7298 precedes base Clear and
    /// job destruction82d7ba20. Pedestrians own no external channel registry.
    pub(crate) fn try_prepare_pedestrian_destruction(&mut self)->Result<(),ResetSkaterAnimError>{
        use ResetSkaterAnimError as E;
        if self.flavor!=super::AnimationConstructionFlavor::Pedestrian {
            return Err(E::Missing("pedestrian animation occurrence"));
        }
        self.initialization.as_mut().ok_or(E::Missing("animation initialization"))?.channels=None;
        self.try_clear_animatable()?;
        // Pedestrian initialization has no asynchronous blend scheduler. If
        // a later owner introduces a started job, Clear rejects the missing
        // native WaitForJob before any job storage is released.
        self.initialization.as_mut().expect("checked initialization").job=None;
        Ok(())
    }

}
