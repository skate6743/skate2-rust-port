//! Stock tree ownership and persistent playback used directly by MotionHost.
mod selection_space_host;
mod tree_builder;
mod stock_clip_query;
use skate3_core::animation::posture::{PendingPosture, PosturePose};
use skate3_core::animation::{
    clip_clock::AdvanceResult,
    output::attributes::{AnimationAttribute, AttributeName, MotionGraphAttribute},
    phase_blend::PhaseBlend,
    playback::{PlaybackRequest, PlaybackService},
    playback_clip::{ClipAttribute, PlaybackClip},
    playback_parameters::{AttributeSink, ParameterInputs, SettableAttribute, SettableAttributes},
    playback_transition::PlaybackTransition,
    playback_tree::{Evaluation, PlaybackTree, PoseCommand},
    skeleton_input::name::encode,
};
use skate3_core::graph::intents::IntentMap;
use skate_data::animation_metadata::{AnimationMetadata, TreeMetadata};
use tree_builder::build;
use super::outputs::ActionGraphOutput;

pub struct MotionAnimation {
    metadata: AnimationMetadata,
    current: Option<PlaybackTree>,
    pub channels: super::motion_channels::MotionChannels,
    pub current_name: Option<String>,
    pub motion_intents: IntentMap,
    pub filtered_intents: IntentMap,
    pub motion_attributes: Vec<MotionGraphAttribute>,
    pub grab_type: Option<super::motion_stock_gameplay::GrabType>,
    pub construction_values: Vec<(AttributeName, AttributeName)>,
    pub posture: PendingPosture,
    pub posture_bank_valid: bool,
    ///Actual fullSkaterAnim15180 flags. AddBindPose consumes bits22/21 on
    ///construction; the owner publishes the updated flags back to the actor.
    pub skater_animation_flags: Option<u32>,
    /// Pending SkaterAnim15184 write from AirDismounting; consumed by the actor.
    pub air_dismount_revert_frames: Option<i32>,
    /// SkaterAnim15188/15196/15200 at the graph publication boundary.
    pub natural_stance: u32,
    pub relative_stance: u32,
    pub requested_stance: u32,
    pub reset_action_intents: bool,
    /// Actor-owned, scoped to an explicit offboard reset dispatch only.
    pub(crate) host_offboard_reset: bool,
    attribute_mirror: std::sync::Arc<skate3_core::animation::playback_attributes::AttributeMirror>,
    settable: SettableAttributes,
    tree_attributes: Vec<AnimationAttribute>,
    property: AdvanceResult,
    /// Creation order of the transitions `play` makes (Andale::TransitionStore's time order).
    transition_serial: u64,
}
impl MotionAnimation {
    #[cfg(any())]
    pub fn seek_current_fraction(&mut self, fraction: f32) {
        if let Some(tree) = &mut self.current { tree.set_time(tree.length() * fraction); }
    }

    pub(crate) fn metadata(&self) -> &AnimationMetadata {
        &self.metadata
    }

    #[cfg(any())]
    pub(super) fn pending_parameter(&self, name: AttributeName) -> Option<f32> {
        self.settable.entries().iter().find(|a| a.name == name).map(|a| a.value)
    }
    pub fn accept_action_graph(&mut self, output: ActionGraphOutput) {
        output.motion_effects.apply_to(&mut self.motion_intents);
    }

    pub fn reset_from_stock(&mut self) {
        self.current = None;
        self.current_name = None;
        self.channels.reset_from_stock();
        self.grab_type = None;
        self.motion_intents.clear();
        self.filtered_intents.clear();
        self.motion_attributes.clear();
        self.tree_attributes.clear();
        self.settable.clear();
        //82B98050 preserves local-player and board-present flags, and saved15200.
        self.skater_animation_flags = self.skater_animation_flags.map(|f| f & 0x0ff7_ffff);
        self.relative_stance = 0;
        self.reset_action_intents = true;
        self.property = AdvanceResult { crossed_end: false, overshoot: -1.0, remaining_before_wrap: -1.0 };
    }
    pub fn reset_to_given_stance(&mut self) -> Result<(), String> {
        let flags = self.skater_animation_flags.as_mut().ok_or("Stance reset requires SkaterAnim flags")?;
        let requested_before=self.requested_stance;
        let mut relative = self.relative_stance != 0;
        skate3_core::player::offboard::reset_animation::given_stance(
            self.natural_stance, &mut self.requested_stance, flags, &mut relative,
        );
        self.relative_stance = u32::from(relative);
        crate::diagnostics::test_log::event("skate3_stance_reset",format_args!(
            "natural={} requested={requested_before} relative={} flags={:08x}",self.natural_stance,self.relative_stance,*flags));
        Ok(())
    }
    pub fn from_metadata(metadata: AnimationMetadata) -> Self {
        Self {
            metadata,
            current: None,
            channels: super::motion_channels::MotionChannels::default(),
            current_name: None,
            motion_intents: IntentMap::new(),
            filtered_intents: IntentMap::new(),
            motion_attributes: Vec::new(),
            grab_type: None,
            construction_values: Vec::new(),
            posture: PendingPosture::default(),
            posture_bank_valid: false,
            skater_animation_flags: None,
            air_dismount_revert_frames: None,
            natural_stance: 1,
            relative_stance: 0,
            requested_stance: 0,
            reset_action_intents: false,
            host_offboard_reset: false,
            attribute_mirror: std::sync::Arc::new(
                skate3_core::animation::playback_attributes::AttributeMirror(Vec::new()),
            ),
            settable: SettableAttributes::default(),
            tree_attributes: Vec::new(),
            property: AdvanceResult {
                crossed_end: false,
                overshoot: -1.0,
                remaining_before_wrap: -1.0,
            },
            transition_serial: 0,
        }
    }
    ///Share the already loaded stock hierarchy with playback; no frame reload.
    pub fn set_hierarchy(
        &mut self,
        names: &[String],
        mirror_indices: &[i32],
    ) -> Result<(), String> {
        if names.len() != mirror_indices.len() {
            return Err("Animation hierarchy mirror count differs from bone count".into());
        }
        let mut pairs = Vec::with_capacity(names.len());
        for (index, name) in names.iter().enumerate() {
            let target = match mirror_indices[index] {
                -1 => name,
                i if i >= 0 => names.get(i as usize).ok_or("Invalid mirrored bone index")?,
                _ => return Err("Invalid negative mirrored bone index".into()),
            };
            pairs.push((encode(name.as_bytes()), encode(target.as_bytes())));
        }
        self.attribute_mirror = std::sync::Arc::new(
            skate3_core::animation::playback_attributes::AttributeMirror(pairs),
        );
        Ok(())
    }
    pub fn tree_attributes(&self) -> &[AnimationAttribute] {
        &self.tree_attributes
    }
    pub fn property(&self) -> AdvanceResult {
        self.property
    }
    pub fn current_time(&self) -> Result<f32, String> {
        self.current
            .as_ref()
            .map(PlaybackTree::time)
            .ok_or_else(|| "No current animation tree".into())
    }
    pub fn current_length(&self) -> Result<f32, String> {
        self.current
            .as_ref()
            .map(PlaybackTree::length)
            .ok_or_else(|| "No current animation tree".into())
    }
    /// MatchAirTime82BBA198..1CC: current tree v20 length, v28 SetTime.
    /// Missing current tree skips the seek, but not the caller's parameters.
    pub(super) fn synchronize_air_time(&mut self, fraction: f32) {
        if let Some(tree) = &mut self.current {
            tree.set_time(tree.length() * fraction);
        }
    }
    pub fn in_transition(&self) -> bool {
        self.current.as_ref().is_some_and(has_transition)
    }
    ///JumpInto Update82BACDC0: apply pending parameters, then query the
    ///current tree with mask31 and seek to the authored attribute's begin.
    ///The caller owns the one-shot instance latch; a missing marker is a miss.
    pub(super) fn jump_into(&mut self, name: AttributeName) -> Result<(), String> {
        self.apply_parameters()?;
        let tree = self.current.as_mut().ok_or("JumpInto requires a current animation tree")?;
        let mut attribute = MotionGraphAttribute { name, value: 0.0 }.to_animation();
        if tree.query_attribute(name, 31, &mut attribute)? {
            tree.set_time(attribute.begin_time);
        }
        Ok(())
    }
    ///825310F0: retire completed transitions before advancing; reset property
    /// once at the root. Graph evaluation, parametrization and pose evaluation
    /// remain separate calls so the actor schedule owns their meaningful order.
    pub fn advance(&mut self, dt: f32, phase: f32) {
        self.channels.retire();
        self.current = self.current.take().map(prune);
        self.property = AdvanceResult {
            crossed_end: false,
            overshoot: -1.0,
            remaining_before_wrap: -1.0,
        };
        if let Some(tree) = &mut self.current {
            tree.advance(dt, phase, &mut self.property);
        }
        self.channels.advance(dt, phase);
    }
    pub fn apply_parameters(&mut self) -> Result<(), String> {
        let attributes = self.settable.entries().to_vec();
        if let Some(mut tree) = self.current.take() {
            let result = self.prepare_selection_spaces(&mut tree, &attributes);
            self.current = Some(tree);
            result?;
        }
        let mut channels = std::mem::take(&mut self.channels);
        let result =
            channels.prepare_trees(|tree| self.prepare_selection_spaces(tree, &attributes));
        self.channels = channels;
        result?;
        if let Some(tree) = &mut self.current {
            tree.set_attributes(self.settable.entries())?;
        }
        self.channels.set_attributes(self.settable.entries())?;
        self.settable.clear();
        Ok(())
    }
    pub fn evaluate_pose(&mut self, parameters: Evaluation) -> Result<Vec<PoseCommand>, String> {
        let mut output = Vec::new();
        let tree = self
            .current
            .as_mut()
            .ok_or("MotionGraph has not selected an animation")?;
        tree.evaluate(parameters, true, &mut output)?;
        self.channels.evaluate(parameters, &mut output)?;
        Ok(output)
    }
    pub fn refresh_tree_attributes(&mut self) -> Result<(), String> {
        self.tree_attributes = self.channels.attributes(
            match &self.current {
                Some(tree) => tree.attributes(15)?,
                None => Vec::new(),
            },
            15,
        )?;
        Ok(())
    }
    pub fn begin_graph_update(&mut self) {
        self.motion_attributes.clear();
    }
    pub fn attach(&mut self, intent: &str, attribute: AttributeName, set: bool) {
        if let Some(value) = self.motion_intent(intent) {
            self.motion_attributes.push(MotionGraphAttribute {
                name: attribute,
                value,
            });
            if set {
                self.set_attribute(SettableAttribute {
                    name: attribute,
                    value,
                    normalized: false,
                    sequence_id: -1,
                });
            }
        }
    }
    pub fn emit_packet(&mut self, name: AttributeName, value: f32) {
        self.motion_attributes
            .push(MotionGraphAttribute { name, value });
    }
    pub fn set_grab_type(&mut self, grab_type: super::motion_stock_gameplay::GrabType) {
        self.grab_type = Some(grab_type);
        self.emit_packet(encode(b"GrabType"), grab_type.as_value());
    }
    pub fn clear_grab_type(&mut self) {
        self.grab_type = None;
    }
    pub fn build_tree(&self, name: &str) -> Result<PlaybackTree, String> {
        build(
            &self.metadata,
            name,
            &self.construction_values,
            &mut Vec::new(),
        )
    }
    pub fn new_channel(
        &mut self,
        name: &str,
        animation: &str,
        settings: skate3_core::animation::channel_playback::ChannelSettings,
    ) -> Result<bool, String> {
        if self.channels.has(name) {
            return Ok(false);
        }
        let mut motion = self.build_tree(animation)?;
        motion.set_speed(settings.speed);
        let mut tree = self.add_bind_pose(motion, None)?;
        if settings.mirrored {
            if let PlaybackTree::BindPose { mirror_modes, .. } = &mut tree {
                mirror_modes.push(2);
            }
        }
        self.channels.insert(name.into(), tree, settings);
        Ok(true)
    }
    pub fn transition_channel(
        &mut self,
        name: &str,
        animation: &str,
        settings: skate3_core::animation::channel_playback::ChannelSettings,
        transition: skate3_core::animation::playback::TransitionSettings,
        resurrect: bool,
        create_missing: bool,
    ) -> Result<bool, String> {
        if !self.channels.has(name) {
            return if create_missing {
                self.new_channel(name, animation, settings)
            } else {
                Ok(false)
            };
        }
        if !self.channels.can_transition(name, resurrect) {
            return Ok(false);
        }
        let mut motion = self.build_tree(animation)?;
        motion.set_speed(settings.speed);
        let mut tree = self.add_bind_pose(motion, None)?;
        if settings.mirrored {
            if let PlaybackTree::BindPose { mirror_modes, .. } = &mut tree {
                mirror_modes.push(2);
            }
        }
        self.channels
            .transition(name, tree, settings, transition, resurrect);
        Ok(true)
    }
    fn add_bind_pose(
        &mut self,
        motion: PlaybackTree,
        posture: Option<PosturePose>,
    ) -> Result<PlaybackTree, String> {
        let flags = self
            .skater_animation_flags
            .ok_or("Animation construction requires actual SkaterAnim flags")?;
        let mut mirror_modes = Vec::new();
        if flags & 0x40000000 != 0 {
            mirror_modes.push(2);
        }
        if flags & 0x00400000 != 0 {
            mirror_modes.push(u32::from(flags & 0x00200000 == 0) + 1);
        }
        self.skater_animation_flags = Some(flags & !0x00600000);
        if !mirror_modes.is_empty() && self.attribute_mirror.0.is_empty() {
            return Err("Mirrored animation requires the stock bone hierarchy".into());
        }
        Ok(PlaybackTree::BindPose {
            motion: Box::new(motion),
            posture,
            board_backwards: flags & 0x80000000 != 0,
            mirror_modes,
            attribute_mirror: self.attribute_mirror.clone(),
        })
    }
}
/// PC host-reset preference; ordinary dismounts retain their authored clip.
fn prepare_offboard_reset(request: &mut PlaybackRequest, host_reset: bool) -> bool {
    if !host_reset || !request.animation.eq_ignore_ascii_case("BR_SPAWN_DISMOUNT_INTO_STAND_0") {
        return false;
    }
    request.transition.kind = 1;
    request.transition.seconds = 0.0;
    request.transition.under = 0;
    true
}

fn has_transition(tree: &PlaybackTree) -> bool {
    match tree {
        PlaybackTree::Transition(_) => true,
        PlaybackTree::PhaseBlend(tree) => tree.children.iter().any(has_transition),
        PlaybackTree::BlendSpace(tree) => tree.children.iter().any(has_transition),
        PlaybackTree::SelectionSpace(tree) => tree.current().is_some_and(has_transition),
        PlaybackTree::BindPose { motion, .. } => has_transition(motion),
        _ => false,
    }
}
/// Andale::TransitionStore (ctor 0x82D17370): mMaxNumTransitionBlends 3, mMaxNumTrees 4.
const MAX_TRANSITION_BLENDS: usize = 3;
const MAX_TREES: usize = 4;

/// The store's live transitions (those `play` made), each with whether it is a blend (kind 2 or
/// 3; kind 4 is a Sequence).
fn store_entries(tree: &PlaybackTree) -> Vec<(u64, bool)> {
    fn collect(tree: &PlaybackTree, out: &mut Vec<(u64, bool)>) {
        if let PlaybackTree::Transition(transition) = tree {
            if transition.serial != 0 {
                out.push((transition.serial, transition.settings.kind != 4));
            }
            collect(&transition.from, out);
            collect(&transition.to, out);
        }
    }
    let mut out = Vec::new();
    collect(tree, &mut out);
    out
}

/// TU3 Andale::Animatable::TransitionAnim 0x82D18780..7CC, before a new tree is made: a blend
/// or channel blend (eTransType 1 or 2) first completes the oldest blend when the store holds
/// mMaxNumTransitionBlends; then any transition completes the oldest transition when the store
/// holds mMaxNumTrees - 1. Fast grabs stack Into/Out/Cycle blends faster than they finish, so
/// without this the oldest blends keep weighting the pose and every reach stays shallow.
fn release_for_capacity(mut tree: PlaybackTree, kind: u32) -> PlaybackTree {
    if matches!(kind, 2 | 3) {
        let blends: Vec<u64> = store_entries(&tree).into_iter().filter(|e| e.1).map(|e| e.0).collect();
        if blends.len() >= MAX_TRANSITION_BLENDS && let Some(&oldest) = blends.iter().min() {
            tree = force_release(tree, oldest);
        }
    }
    let entries = store_entries(&tree);
    if entries.len() >= MAX_TREES - 1 && let Some(oldest) = entries.iter().map(|e| e.0).min() {
        tree = force_release(tree, oldest);
    }
    tree
}

/// ForceTransitionRelease 0x82D18658: the transition's parent (or the root) takes its
/// destination at once; its source subtree, with any transitions in it, is released.
fn force_release(tree: PlaybackTree, serial: u64) -> PlaybackTree {
    match tree {
        PlaybackTree::Transition(transition) if transition.serial == serial => *transition.to,
        PlaybackTree::Transition(mut transition) => {
            transition.from = Box::new(force_release(*transition.from, serial));
            transition.to = Box::new(force_release(*transition.to, serial));
            PlaybackTree::Transition(transition)
        }
        other => other,
    }
}

/// The youngest store transition's destination becomes the source of the new transition.
fn insert_under(
    tree: &mut PlaybackTree,
    youngest: u64,
    new: &mut Option<(PlaybackTree, skate3_core::animation::playback::TransitionSettings, u64)>,
) {
    let PlaybackTree::Transition(transition) = tree else { return };
    if transition.serial == youngest {
        if let Some((to, settings, serial)) = new.take() {
            let from = std::mem::replace(&mut *transition.to, to.clone());
            let mut inserted = PlaybackTransition::new(from, to, settings);
            inserted.serial = serial;
            *transition.to = PlaybackTree::Transition(inserted);
        }
        return;
    }
    insert_under(&mut transition.from, youngest, new);
    if new.is_some() {
        insert_under(&mut transition.to, youngest, new);
    }
}

fn prune(tree: PlaybackTree) -> PlaybackTree {
    match tree {
        PlaybackTree::Transition(mut transition) => {
            if transition.complete() {
                prune(*transition.to)
            } else {
                transition.from = Box::new(prune(*transition.from));
                transition.to = Box::new(prune(*transition.to));
                PlaybackTree::Transition(transition)
            }
        }
        PlaybackTree::BlendSpace(mut tree) => {tree.children=tree.children.into_iter().map(prune).collect();PlaybackTree::BlendSpace(tree)}
        PlaybackTree::PhaseBlend(mut tree) => {
            tree.children = tree.children.into_iter().map(prune).collect();
            PlaybackTree::PhaseBlend(tree)
        }
        PlaybackTree::SelectionSpace(mut tree) => {
            tree.candidates = tree
                .candidates
                .into_iter()
                .map(|mut candidate| {
                    candidate.tree = prune(candidate.tree);
                    candidate
                })
                .collect();
            PlaybackTree::SelectionSpace(tree)
        }
        PlaybackTree::BindPose {
            motion,
            posture,
            board_backwards,
            mirror_modes,
            attribute_mirror,
        } => PlaybackTree::BindPose {
            motion: Box::new(prune(*motion)),
            posture,
            board_backwards,
            mirror_modes,
            attribute_mirror,
        },
        other => other,
    }
}
impl ParameterInputs for MotionAnimation {
    fn motion_intent(&self, name: &str) -> Option<f32> {
        self.motion_intents.get(name).copied()
    }
    fn filtered_intent(&self, name: &str) -> Option<f32> {
        self.filtered_intents.get(name).copied()
    }
    fn last_attribute(
        &mut self,
        name: AttributeName,
    ) -> Result<Option<AnimationAttribute>, String> {
        Ok(self
            .tree_attributes
            .iter()
            .find(|a| a.name == name)
            .copied())
    }
}
impl AttributeSink for MotionAnimation {
    fn set_attribute(&mut self, attribute: SettableAttribute) {
        self.settable.set_attribute(attribute);
    }
}
impl super::pushing::PushAnimationSink for MotionAnimation {
    fn set_attribute(&mut self, a: super::pushing::SettableAttribute) {
        AttributeSink::set_attribute(
            self,
            SettableAttribute {
                name: encode(a.name.as_bytes()),
                value: a.value,
                normalized: a.normalized,
                sequence_id: a.sequence_id,
            },
        );
    }
}
impl PlaybackService for MotionAnimation {
    fn set_construction_value(&mut self, name: AttributeName, value: AttributeName) {
        if let Some(entry) = self
            .construction_values
            .iter_mut()
            .find(|entry| entry.0 == name)
        {
            entry.1 = value;
        } else {
            self.construction_values.push((name, value));
        }
    }
    fn set_posture_enabled(&mut self, enabled: bool) {
        self.posture.set_requested(enabled);
    }
    fn play(&mut self, mut request: PlaybackRequest) -> Result<bool, String> {
        if !(1..=4).contains(&request.transition.kind) {
            return Ok(false);
        }
        let settled_spawn = prepare_offboard_reset(&mut request, self.host_offboard_reset);
        if request.transition.kind == 1 {
            self.current = None;
        }
        let motion = self.build_tree(&request.animation)?;
        //SkaterAnim::GetAnimTree82B980A0 calls AddBindPose82B98118 before
        //Play/Blend set time/speed and before transition initialization.
        let (motion, posture) = if self.posture_bank_valid {
            self.posture
                .apply::<_, String>((motion, None), |(motion, _), pose| Ok((motion, Some(pose))))?
        } else {
            (motion, None)
        };
        let mut tree = self.add_bind_pose(motion, posture)?;
        tree.set_speed(request.speed);
        let prior = self.current.take().map(|prior| release_for_capacity(prior, request.transition.kind));
        //82D186F8's no-current branch calls Play with start0.
        if settled_spawn {
            // Original SpawnDismount ends standing. A marker/map arrival uses
            // that pose immediately, rather than replaying a crouched dismount
            // at the graph's half playback speed after the physical reset.
            tree.set_time(tree.length());
            crate::diagnostics::test_log::event("skate3_offboard_spawn_animation",
                format_args!("tree={} settled_time={}",request.animation,tree.time()));
        } else if request.start_time > 0.0 && (request.transition.kind == 1 || prior.is_some()) {
            tree.set_time(request.start_time);
        }
        self.current = Some(match prior {
            Some(mut from) => {
                self.transition_serial += 1;
                let serial = self.transition_serial;
                // TU3 TransitionAnim 0x82D18950/0x82D188A8: with a live transition, a transition
                // placed under goes below the youngest one, from its destination.
                let youngest = store_entries(&from).into_iter().map(|(serial, _)| serial).max();
                let mut new = Some((tree, request.transition, serial));
                if let Some(youngest) = youngest.filter(|_| request.transition.under != 0) {
                    insert_under(&mut from, youngest, &mut new);
                }
                match new {
                    None => from,
                    Some((tree, settings, serial)) => {
                        let mut transition = PlaybackTransition::new(from, tree, settings);
                        transition.serial = serial;
                        PlaybackTree::Transition(transition)
                    }
                }
            }
            None => tree,
        });
        self.current_name = Some(request.animation);
        Ok(true)
    }
}

#[cfg(any())]
#[path = "tests/animation_jump_into.rs"]
mod jump_into_tests;
#[path = "motion_grind/animation_owner.rs"]
mod grind_owner;

#[cfg(any())]
#[path = "tests/motion_air_dismounting.rs"]
mod air_dismounting_tests;
