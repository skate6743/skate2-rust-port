//! Actor animation phase2/3 on the actual retained `SkaterAnim` and tree owners.
//! Action and Motion run between phase1 and phase2 in the outer Actor caller.
use super::{
    ActorAnimationConstruction, AnimationPointerState,
    channel_runtime::{ChannelBaseReference, GestureChannelError},
    gesture_writes::GestureAnimationWriteError,
};
use crate::actor_animation_tree::blend_space::{
    jobs::{BlendSpaceJobError, BlendSpacePool},
    update::BlendSpaceAttributeContext,
};
use crate::input_cvar_runtime::InputCvarReferences;
use crate::{
    actor_animation_construction::initialization::body_dispatch::{
        BodyAnimationDispatchSource, BodyDispatchError,
    },
    actor_animation_tree::attributes::{
        SettableTreeAttribute, TreeAttributeError, append_attribute, packed_name, push_scratch,
    },
    play_animation::FastString30,
    simulation_animation_startup::AnimationAttributePool,
};
use skate2_formats::andale::AndaleRuntimeAttribute;
use std::sync::Arc;

/// GetTreeAttr82d79170: first matching cached row, five packed name words.
pub(crate) fn find_cached_tree_attribute<'a>(
    rows: impl IntoIterator<Item = &'a Option<AndaleRuntimeAttribute>>,
    name: FastString30,
) -> Result<Option<AndaleRuntimeAttribute>, AnimationPhaseError> {
    for row in rows {
        let row = row.as_ref().ok_or(AnimationPhaseError::Missing("occupied tree attribute"))?;
        let packed = FastString30::encode(&row.name)
            .map_err(|_| AnimationPhaseError::Missing("tree attribute name encoding"))?;
        if packed == name { return Ok(Some(row.clone())); }
    }
    Ok(None)
}

impl<K, W> ActorAnimationConstruction<K, W> {
    pub(crate) fn try_tree_attribute(&self, name: FastString30)
        -> Result<Option<AndaleRuntimeAttribute>, AnimationPhaseError> {
        let base = self.base().ok_or(AnimationPhaseError::Missing("Animatable base"))?;
        find_cached_tree_attribute(base.tree_attributes.occupied_slots(), name)
    }
}

#[derive(Debug)]
pub(crate) enum AnimationPhaseError {
    Missing(&'static str),
    UnknownFlags(u32),
    Tree(TreeAttributeError),
    BlendSpace(BlendSpaceJobError),
    Time(GestureChannelError),
    #[cfg(feature = "skate3-runtime")]
    PcGesture(String),
    Write(GestureAnimationWriteError),
}
impl From<TreeAttributeError> for AnimationPhaseError {
    fn from(error: TreeAttributeError) -> Self {
        Self::Tree(error)
    }
}
impl std::fmt::Display for AnimationPhaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(owner) => write!(f, "animation phase requires {owner}"),
            Self::UnknownFlags(mask) => write!(f, "animation phase flags {mask:08x} unknown"),
            Self::Tree(error) => write!(f, "animation phase: {error}"),
            Self::BlendSpace(error) => error.fmt(f),
            Self::Time(error) => write!(f, "animation phase: {error}"),
            #[cfg(feature = "skate3-runtime")]
            Self::PcGesture(error)=>write!(f,"gesture phase: {error}"),
            Self::Write(error) => write!(f, "animation phase: {error}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum AnimationPhaseDispatchError<E> {
    Phase(AnimationPhaseError),
    Dispatch(BodyDispatchError<E>),
}
impl<E: std::fmt::Debug> std::fmt::Display for AnimationPhaseDispatchError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Phase(error) => write!(f, "{error}"),
            Self::Dispatch(error) => write!(f, "animation phase dispatch: {error:?}"),
        }
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Interface setter `SkaterAnim::SetOpacity82cb75d0`: +52e0 interface
    /// adjustment maps +e68/+e4c to the actual +6148/+612c player members.
    pub(crate) fn try_set_animation_opacity(
        &mut self,
        opacity: f32,
        main_camera_only: bool,
    ) -> Result<(), AnimationPhaseError> {
        let player = self
            .player
            .as_mut()
            .ok_or(AnimationPhaseError::Missing("SkaterAnim player"))?;
        player.opacity = opacity;
        player.flags_bits =
            (player.flags_bits & !0x0400_0000) | (u32::from(main_camera_only) << 26);
        player.flags_known_mask |= 0x0400_0000;
        Ok(())
    }

    /// Actor825f4264..42c0. Concrete vtable+D0 is `IsMounting82cb7668`;
    /// +D4 is `SetIsHoldingBoard82cb7728`, not the decompiler's grab setter.
    /// Returns the subsequent actual `IsInLocomotion` branch condition.
    pub(crate) fn try_update_phase1_holding(
        &mut self,
        is_offboard: bool,
        physical_holding: bool,
    ) -> Result<bool, AnimationPhaseError> {
        let holding = physical_holding || !is_offboard || self.phase_flag(0x0001_0000)?;
        let fields = self
            .before_sprint
            .as_mut()
            .ok_or(AnimationPhaseError::Missing("SkaterAnim second flags"))?;
        fields.second_flags_bits =
            (fields.second_flags_bits & !0x8000_0000) | (u32::from(holding) << 31);
        fields.second_flags_known_mask |= 0x8000_0000;
        self.phase_flag(0x0010_0000)
    }

    pub(super) fn phase_flag(&self, mask: u32) -> Result<bool, AnimationPhaseError> {
        // S3's controller owns its trajectory queries. Do not schedule native
        // S2 walking futures against an inactive native S2 player solver.
        if self.skate3_movement && matches!(mask, 0x0010_0000 | 0x0008_0000) {
            return Ok(false);
        }
        let player = self
            .player
            .as_ref()
            .ok_or(AnimationPhaseError::Missing("SkaterAnim player"))?;
        if player.flags_known_mask & mask != mask {
            return Err(AnimationPhaseError::UnknownFlags(mask));
        }
        Ok(player.flags_bits & mask != 0)
    }

    fn current_attribute_reference(
        &self,
    ) -> Result<Option<ChannelBaseReference>, AnimationPhaseError> {
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(AnimationPhaseError::Missing("Initialize"))?;
        let channels = initialized
            .channels
            .as_ref()
            .ok_or(AnimationPhaseError::Missing("ChannelAnimatable"))?;
        if let Some(root) = channels.root {
            return Ok(Some(ChannelBaseReference::Channel(root)));
        }
        let base = self
            .base
            .as_ref()
            .ok_or(AnimationPhaseError::Missing("Animatable"))?;
        if let Some(root) = &base.body_root {
            return Ok(Some(ChannelBaseReference::Body(root.clone())));
        }
        if base.base_animation == AnimationPointerState::Null {
            return Ok(None);
        }
        Err(AnimationPhaseError::Missing("retained base tree"))
    }

    /// `SkaterAnim::SetAttributes`82cb7c38: shared by Actor phase2 and
    /// KickTurnSteering's first Update through IAnimatable vtable +0x4c.
    pub(crate) fn try_skater_set_attributes(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhaseError> {
        if self.phase_flag(0x0010_0000)? && !self.phase_flag(0x0008_0000)? {
            // SkaterAnim82cb7c38..7d1c: exact literals and distinct sequence IDs.
            for (name, bits, sequence) in [
                ("ZPrevStride", 0x3fc0_0000, -1),
                ("ZNextStride", 0x3fc0_0000, -1),
                ("YSpeed", 0xbf85_1eb8, 0),
                ("ZSpeed", 0x407e_b852, 0),
            ] {
                self.try_add_gesture_attribute(
                    FastString30::encode(name).expect("source literal"),
                    f32::from_bits(bits),
                    false,
                    sequence,
                )
                .map_err(AnimationPhaseError::Write)?;
            }
        }
        self.try_animatable_set_attributes(cvars)
    }

    /// Shared `Animatable::SetAttributes`; caller-specific attribute writes
    /// precede this method (SkaterAnim) or are absent (cPedestrianAnim).
    pub(crate) fn try_animatable_set_attributes(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhaseError> {
        let base = self
            .base
            .as_mut()
            .ok_or(AnimationPhaseError::Missing("Animatable"))?;
        if base.settable_attributes.len != 0 {
            base.settable_animation = AnimationPointerState::Constructed;
            base.request_blend_space = AnimationPointerState::Constructed;
            let input = base
                .settable_attributes
                .occupied_slots()
                .iter()
                .map(|slot| {
                    let attribute = slot
                        .as_ref()
                        .ok_or(AnimationPhaseError::Missing("settable attribute cell"))?;
                    Ok(SettableTreeAttribute {
                        sequence_id: attribute.sequence_id,
                        name: attribute.name,
                        value: attribute
                            .attribute
                            .scalar_value()
                            .ok_or(AnimationPhaseError::Missing("scalar settable attribute"))?,
                        normalized: attribute.normalized,
                    })
                })
                .collect::<Result<Vec<_>, AnimationPhaseError>>()?;
            let reference = self
                .current_attribute_reference()?
                .ok_or(AnimationPhaseError::Missing("SetAttributes current tree"))?;
            self.set_reference_attributes(&reference, &input, cvars)?;
            self.base
                .as_mut()
                .expect("retained base")
                .settable_attributes
                .len = 0;
        }
        Ok(())
    }

    /// `Actor::UpdatePhase2`825ed70c..738: Skater `SetAttributes`, BS dispatch, optional wait.
    /// The supported Clip/PhaseBlend/Pose/AddSQT/Mirror nodes do not register BS work.
    pub(crate) fn try_animation_phase2(
        &mut self,
        wait_for_bs_job: bool,
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhaseError> {
        self.try_skater_set_attributes(cvars)?;
        self.dispatch_blend_space(cvars)
            .map_err(AnimationPhaseError::BlendSpace)?;
        if wait_for_bs_job {
            self.wait_blend_space(cvars)
                .map_err(AnimationPhaseError::BlendSpace)?;
        }
        Ok(())
    }

    // TransitionBlend's attribute virtuals tail-call mToAnim, without blending.
    pub(super) fn attribute_body_tree(
        &self,
        reference: &super::blend_anim::BodyAnimationReference,
    ) -> Result<super::play_anim::BodyAnimationRoot, AnimationPhaseError> {
        use super::blend_anim::BodyAnimationReference as R;
        match reference {
            R::Tree(root) => Ok(root.clone()),
            R::TransitionBlend(index) => {
                let runtime = self
                    .base
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("Animatable"))?
                    .transition_store
                    .retained_blend(*index)
                    .map_err(|_| AnimationPhaseError::Missing("retained transition"))?;
                self.attribute_body_tree(&runtime.to)
            }
            R::TransitionSequence(index) => {
                let store = &self
                    .base
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("Animatable"))?
                    .transition_store;
                let runtime = store
                    .retained_sequence(*index)
                    .map_err(|_| AnimationPhaseError::Missing("retained Sequence"))?;
                let sequence = store
                    .sequences
                    .get(*index)
                    .ok_or(AnimationPhaseError::Missing("Sequence slot"))?;
                self.attribute_body_tree(if sequence.playing_new_animation {
                    &runtime.to
                } else {
                    &runtime.from
                })
            }
        }
    }

    fn set_body_reference_attributes(
        &mut self,
        reference: &super::blend_anim::BodyAnimationReference,
        input: &[SettableTreeAttribute],
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhaseError> {
        use super::blend_anim::BodyAnimationReference as R;
        match reference {
            R::Tree(root) => {
                let queue = self
                    .blend_space_buffer()
                    .map_err(AnimationPhaseError::BlendSpace)?;
                self.initialization
                    .as_mut()
                    .ok_or(AnimationPhaseError::Missing("Initialize"))?
                    .tree_pools
                    .get_mut(root.pool_index)
                    .ok_or(AnimationPhaseError::Missing("body tree pool"))?
                    .runtime
                    .set_attributes_with_blend_space(
                        &root.node,
                        input,
                        &mut Some(BlendSpaceAttributeContext {
                            cvars,
                            queue,
                            pool: BlendSpacePool::Body(root.pool_index),
                        }),
                    )?;
                Ok(())
            }
            R::TransitionBlend(index) => {
                let to = self
                    .base
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("Animatable"))?
                    .transition_store
                    .retained_blend(*index)
                    .map_err(|_| AnimationPhaseError::Missing("retained Blend"))?
                    .to
                    .clone();
                self.set_body_reference_attributes(&to, input, cvars)
            }
            R::TransitionSequence(index) => {
                let store = &self
                    .base
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("Animatable"))?
                    .transition_store;
                let runtime = store
                    .retained_sequence(*index)
                    .map_err(|_| AnimationPhaseError::Missing("retained Sequence"))?;
                let to = runtime.to.clone();
                let from = (!store
                    .sequences
                    .get(*index)
                    .ok_or(AnimationPhaseError::Missing("Sequence slot"))?
                    .playing_new_animation)
                    .then(|| runtime.from.clone());
                self.set_body_reference_attributes(&to, input, cvars)?;
                if let Some(from) = from {
                    self.set_body_reference_attributes(&from, input, cvars)?;
                }
                Ok(())
            }
        }
    }

    fn set_reference_attributes(
        &mut self,
        reference: &ChannelBaseReference,
        input: &[SettableTreeAttribute],
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhaseError> {
        if let ChannelBaseReference::Body(reference) = reference {
            return self.set_body_reference_attributes(reference, input, cvars);
        }
        let queue = self
            .blend_space_buffer()
            .map_err(AnimationPhaseError::BlendSpace)?;
        let initialized = self
            .initialization
            .as_mut()
            .ok_or(AnimationPhaseError::Missing("Initialize"))?;
        match reference {
            ChannelBaseReference::Empty => {
                Err(AnimationPhaseError::Missing("ChannelBlend base tree"))
            }
            ChannelBaseReference::Body(_) => unreachable!("handled before channel borrow"),
            ChannelBaseReference::Channel(index) => {
                let channels = initialized
                    .channels
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("channels"))?;
                let channel = channels
                    .channels
                    .get(*index)
                    .ok_or(AnimationPhaseError::Missing("channel"))?;
                let runtime = channel
                    .runtime
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("ChannelBlend"))?;
                let base = runtime.base.clone();
                let (tree, animation) = super::channel_runtime::transition::target(channel)
                    .map_err(|_| AnimationPhaseError::Missing("channel animation owner"))?;
                let from = super::channel_runtime::transition::settable_from(channel)
                    .map_err(|_| AnimationPhaseError::Missing("channel Sequence child"))?;
                // ChannelBlend82d6f334 then34c: base first; channel always, ignoring return flags.
                self.set_reference_attributes(&base, input, cvars)?;
                let trees = &mut self
                    .initialization
                    .as_mut()
                    .expect("retained initialization")
                    .channels
                    .as_mut()
                    .expect("retained channels")
                    .trees;
                trees
                    .get_mut(tree)
                    .ok_or(AnimationPhaseError::Missing("channel tree pool"))?
                    .runtime
                    .set_attributes_with_blend_space(
                        &animation,
                        input,
                        &mut Some(BlendSpaceAttributeContext {
                            cvars,
                            queue: Arc::clone(&queue),
                            pool: BlendSpacePool::Channel(tree),
                        }),
                    )?;
                // Sequence.SetAttributes82d6ded0: TO above, then FROM until PlayingNewAnim.
                if let Some((tree, animation)) = from {
                    trees
                        .get_mut(tree)
                        .ok_or(AnimationPhaseError::Missing("channel tree pool"))?
                        .runtime
                        .set_attributes_with_blend_space(
                            &animation,
                            input,
                            &mut Some(BlendSpaceAttributeContext {
                                cvars,
                                queue,
                                pool: BlendSpacePool::Channel(tree),
                            }),
                        )?;
                }
                Ok(())
            }
        }
    }

    /// `Animatable::ProcessTreeAttrs82d72238`: clear first, `GetCurrAnim`, actual pool.
    pub(crate) fn try_process_tree_attributes(&mut self) -> Result<(), AnimationPhaseError> {
        self.base
            .as_mut()
            .ok_or(AnimationPhaseError::Missing("Animatable"))?
            .tree_attributes
            .len = 0;
        let Some(reference) = self.current_attribute_reference()? else {
            return Ok(());
        };
        let pool = Arc::clone(
            self.initialization
                .as_ref()
                .expect("retained initialization")
                .attributes
                .as_ref()
                .ok_or(AnimationPhaseError::Missing("attributes pool"))?,
        );
        let mut pool = pool
            .lock()
            .map_err(|_| AnimationPhaseError::Missing("unpoisoned attributes pool"))?;
        let mut attributes = Vec::new();
        let result = self.get_reference_attributes(&reference, 15, &mut pool, &mut attributes);
        let destination = &mut self.base.as_mut().expect("retained base").tree_attributes;
        // Preserve every accepted output preceding a native assertion boundary.
        for attribute in attributes {
            destination.slots[destination.len] = Some(attribute);
            destination.len += 1;
        }
        if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            let present = |name: &str| {
                destination
                    .occupied_slots()
                    .iter()
                    .filter_map(Option::as_ref)
                    .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
                    .map(|attribute| {
                        (
                            attribute.status,
                            attribute.begin_seconds,
                            attribute.end_seconds,
                        )
                    })
            };
            let jump = present("jump");
            let danger = present("dangerzone");
            if jump.is_some() || danger.is_some() {
                let height = destination
                    .occupied_slots()
                    .iter()
                    .filter_map(Option::as_ref)
                    .find(|attribute| attribute.name.eq_ignore_ascii_case("trickheight"))
                    .and_then(AndaleRuntimeAttribute::scalar_value);
                eprintln!(
                    "known air: tree attributes jump={jump:?} dangerzone={danger:?} trickheight={height:?}"
                );
            }
        }
        result
    }

    fn get_reference_attributes(
        &self,
        reference: &ChannelBaseReference,
        flags: u8,
        scratch: &mut AnimationAttributePool,
        output: &mut Vec<AndaleRuntimeAttribute>,
    ) -> Result<(), AnimationPhaseError> {
        let destination = if let ChannelBaseReference::Body(reference) = reference {
            Some(self.attribute_body_tree(reference)?)
        } else {
            None
        };
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(AnimationPhaseError::Missing("Initialize"))?;
        match reference {
            ChannelBaseReference::Empty => {
                Err(AnimationPhaseError::Missing("ChannelBlend base tree"))
            }
            ChannelBaseReference::Body(_) => {
                let root = destination.as_ref().expect("resolved body destination");
                initialized
                    .tree_pools
                    .get(root.pool_index)
                    .ok_or(AnimationPhaseError::Missing("body tree pool"))?
                    .runtime
                    .get_attributes(&root.node, flags, scratch, output)?;
                Ok(())
            }
            ChannelBaseReference::Channel(index) => {
                let channels = initialized
                    .channels
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("channels"))?;
                let channel = channels
                    .channels
                    .get(*index)
                    .ok_or(AnimationPhaseError::Missing("channel"))?;
                let runtime = channel
                    .runtime
                    .as_ref()
                    .ok_or(AnimationPhaseError::Missing("ChannelBlend"))?;
                if !runtime.use_attributes {
                    return self.get_reference_attributes(&runtime.base, flags, scratch, output);
                }
                // ChannelBlend82d809f0: channel first, regardless of pose blend coefficient.
                // Sequence82d6de00 reads FROM until PlayingNewAnim; Transition reads TO.
                push_scratch(scratch)?;
                let (tree, animation) =
                    super::channel_runtime::transition::attribute_target(channel)
                        .map_err(|_| AnimationPhaseError::Missing("channel animation owner"))?;
                let mut channel_attributes = Vec::new();
                channels
                    .trees
                    .get(tree)
                    .ok_or(AnimationPhaseError::Missing("channel tree pool"))?
                    .runtime
                    .get_attributes(&animation, flags, scratch, &mut channel_attributes)?;
                if channel_attributes.is_empty() {
                    self.get_reference_attributes(&runtime.base, flags, scratch, output)?;
                    scratch.element_count -= 1;
                    return Ok(());
                }
                push_scratch(scratch)?;
                let mut base_attributes = Vec::new();
                self.get_reference_attributes(&runtime.base, flags, scratch, &mut base_attributes)?;
                merge_channel_attributes(base_attributes, channel_attributes, output)?;
                scratch.element_count -= 2;
                Ok(())
            }
        }
    }

    /// `SkaterAnim::ToggleSkaterStates82ca9f70`; lookup presence, not scalar truth.
    fn toggle_skater_states(&mut self) -> Result<(), AnimationPhaseError> {
        // Dynamic initializers82f82138/68/80 provide these exact source names.
        for (name, mask) in [
            ("animboardbackward", 0x8000_0000),
            ("mirrored", 0x4000_0000),
            ("switch", 0),
        ] {
            let name = FastString30::encode(name).expect("source literal");
            let attributes = &self
                .base
                .as_ref()
                .ok_or(AnimationPhaseError::Missing("Animatable"))?
                .tree_attributes;
            let mut found = false;
            for slot in attributes.occupied_slots() {
                if packed_name(
                    slot.as_ref()
                        .ok_or(AnimationPhaseError::Missing("tree attribute cell"))?,
                )? == name
                {
                    found = true;
                    break;
                }
            }
            if !found {
                continue;
            }
            if mask != 0 {
                self.phase_flag(mask)?;
            }
            let player = self
                .player
                .as_mut()
                .ok_or(AnimationPhaseError::Missing("SkaterAnim player"))?;
            if mask == 0 {
                player.relative_stance = u32::from(player.relative_stance == 0);
            } else {
                player.flags_bits ^= mask;
            }
        }
        Ok(())
    }

    /// Selected `Actor::UpdatePhase3`825f443c..4680. The outer owner must execute
    /// actual Action/Motion and phase2 first; the manager performs the final Sync.
    pub(crate) fn try_animation_phase3_body<S: BodyAnimationDispatchSource>(
        &mut self,
        dt: f32,
        source: &mut S,
    ) -> Result<(), AnimationPhaseDispatchError<S::Error>> {
        use AnimationPhaseDispatchError as E;
        if self.try_animation_phase3_prefix(dt).map_err(E::Phase)? {
            return Err(E::Phase(AnimationPhaseError::Missing(
                "CreateLocomotionTrajectory phase3 physical collaborators",
            )));
        }
        self.try_animation_phase3_suffix(source)
    }

    /// The world adapter must complete Create and StepSelector Launch between these halves.
    pub(crate) fn try_animation_phase3_prefix(
        &mut self,
        dt: f32,
    ) -> Result<bool, AnimationPhaseError> {
        #[cfg(feature = "skate3-runtime")]
        if let Some(gesture)=self.pc_gesture_playback.as_mut(){gesture.advance(dt).map_err(AnimationPhaseError::PcGesture)?;}
        let was = self.phase_flag(0x0008_0000)?;
        if !was || !self.phase_flag(0x0010_0000)? {
            self.try_add_animation_time(dt)
                .map_err(AnimationPhaseError::Time)?;
            self.try_process_tree_attributes()?;
        }
        self.toggle_skater_states()?;
        self.phase_flag(0x0010_0000)
    }
    pub(crate) fn try_animation_phase3_suffix<S: BodyAnimationDispatchSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), AnimationPhaseDispatchError<S::Error>> {
        use AnimationPhaseDispatchError as E;
        // 825f45b0: both locomotion bits set suppress this phase's Dispatch.
        if self.phase_flag(0x0008_0000).map_err(E::Phase)?
            && self.phase_flag(0x0010_0000).map_err(E::Phase)?
        {
            return Ok(());
        }
        if self.phase_flag(0x0008_0000).map_err(E::Phase)? {
            self.try_sync_body_animation(source).map_err(E::Dispatch)?;
        }
        self.try_dispatch_body_animation(source)
            .map_err(E::Dispatch)
    }
}

/// `ChannelBlend::GetAttributes82d80b04..d30`: sorted union, channel wins equal names.
fn merge_channel_attributes(
    base: Vec<AndaleRuntimeAttribute>,
    channel: Vec<AndaleRuntimeAttribute>,
    output: &mut Vec<AndaleRuntimeAttribute>,
) -> Result<(), TreeAttributeError> {
    let mut base = base.into_iter().peekable();
    let mut channel = channel.into_iter().peekable();
    while let (Some(left), Some(right)) = (base.peek(), channel.peek()) {
        match packed_name(left)?.words.cmp(&packed_name(right)?.words) {
            std::cmp::Ordering::Less => {
                append_attribute(output, base.next().expect("peeked base"))?;
            }
            std::cmp::Ordering::Equal => {
                base.next();
                append_attribute(output, channel.next().expect("peeked channel"))?;
            }
            std::cmp::Ordering::Greater => {
                append_attribute(output, channel.next().expect("peeked channel"))?;
            }
        }
    }
    for attribute in base.chain(channel) {
        append_attribute(output, attribute)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::andale::AndaleAttributeValue;

    #[test]
    fn phase1_holding_short_circuits_unknown_mounting_and_opacity_replaces_one_flag() {
        use crate::actor_animation_construction::{
            ActorAnimationBeforeSprint, ActorAnimationInputs, ActorAnimationPlayerFields,
            AnimationLifetime,
        };
        use crate::motion_graph_entity_authority::ActorGraphAuthority;
        let authority = ActorGraphAuthority::new(7);
        let mut animation = ActorAnimationConstruction::new(
            &authority,
            ActorAnimationInputs {
                physics_world: Arc::new(()),
                life: AnimationLifetime::Transient,
                use_low_blend_cull_threshold: false,
                face_name: None,
            },
        );
        animation.player = Some(ActorAnimationPlayerFields {
            player_index: 0,
            natural_stance: 0,
            last_stance: 0,
            relative_stance: 0,
            spawn_relative_stance: 0,
            opacity: 1.0,
            flags_known_mask: 0x0010_0000,
            flags_bits: 0x8000_0001,
            gestures: [0; 4],
            pro_skater: FastString30 { words: [0; 5] },
        });
        animation.before_sprint = Some(ActorAnimationBeforeSprint::new());
        assert!(!animation.try_update_phase1_holding(true, true).unwrap());
        assert_eq!(
            animation.before_sprint.as_ref().unwrap().second_flags_bits & 0x8000_0000,
            0x8000_0000
        );
        assert!(matches!(
            animation.try_update_phase1_holding(true, false),
            Err(AnimationPhaseError::UnknownFlags(0x0001_0000))
        ));
        assert!(!animation.try_update_phase1_holding(false, false).unwrap());
        animation.try_set_animation_opacity(-0.0, true).unwrap();
        let player = animation.player.as_ref().unwrap();
        assert_eq!(player.opacity.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(player.flags_bits, 0x8400_0001);
        assert_eq!(player.flags_known_mask, 0x0410_0000);
    }

    #[test]
    fn channel_attributes_replace_equal_name_without_interpolating_values() {
        let attribute = |name: &str, value| AndaleRuntimeAttribute {
            sequence_id: -1,
            name: name.into(),
            begin_seconds: -1.0,
            end_seconds: -1.0,
            status: 6,
            value: AndaleAttributeValue::Float(value),
        };
        let mut output = Vec::new();
        merge_channel_attributes(
            vec![attribute("a", 1.0), attribute("b", 2.0)],
            vec![attribute("b", 7.0), attribute("c", 8.0)],
            &mut output,
        )
        .unwrap();
        assert_eq!(
            output
                .iter()
                .map(|attribute| attribute.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(
            output
                .iter()
                .map(|attribute| attribute.scalar_value().unwrap().to_bits())
                .collect::<Vec<_>>(),
            [1.0_f32.to_bits(), 7.0_f32.to_bits(), 8.0_f32.to_bits()]
        );
    }
}
