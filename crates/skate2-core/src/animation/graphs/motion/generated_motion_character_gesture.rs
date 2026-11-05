//! `CharacterGesture::Update82cc3198` on the actual retained Actor owners.
//! Channel creation uses retained mutable trees, without substituting an
//! evaluated pose or claiming a missing runtime body base animation.
use super::{ActorConstruction, ActorConstructionError, ActorIdentity};
use crate::{
    actor_animation_construction::{
        channel_runtime::GestureChannelError, gesture_writes::GestureAnimationWriteError,
    },
    actor_graph_construction::ActorGraphIntents,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    motion_graph_entity_authority::BoundMotionGraphContext,
    physical_player_phys_out::construct::ConcretePhysOutError,
    play_animation::{FastString30, FastString36},
};
use std::{convert::Infallible, sync::Arc};

#[path = "generated_motion_character_gesture_names.rs"]
mod names;

#[cfg(test)]
pub(crate) fn check_unpublished(actor: &ActorConstruction, cvars: &InputCvarReferences) {
    use crate::dynamic_state_graph_controller::DynamicStateGraphContext;
    let (context, before) = {
        let prefix = actor.prefix().unwrap();
        assert!(prefix.physical_cached.is_none());
        let motion = prefix.graphs.as_ref().unwrap().motion().unwrap();
        let state = motion.state.as_ref().unwrap();
        (
            motion
                .controller
                .as_ref()
                .unwrap()
                .context()
                .copy_for_behaviour(),
            (
                state.gesture,
                state.is_gesturing,
                motion.intents.as_ref().unwrap().len(),
            ),
        )
    };
    assert!(matches!(
        actor.try_update_character_gesture(cvars, &context),
        Err(CharacterGestureError::MissingOwner(
            "cached Actor IPhysical"
        ))
    ));
    let prefix = actor.prefix().unwrap();
    assert!(prefix.physical_cached.is_none());
    let motion = prefix.graphs.as_ref().unwrap().motion().unwrap();
    let state = motion.state.as_ref().unwrap();
    assert_eq!(
        before,
        (
            state.gesture,
            state.is_gesturing,
            motion.intents.as_ref().unwrap().len()
        )
    );
}

#[derive(Debug)]
pub(crate) enum CharacterGestureError {
    Actor(ActorConstructionError),
    Cvar(InputCvarError),
    PhysOut(ConcretePhysOutError<Infallible>),
    MissingOwner(&'static str),
    ForeignOwner(&'static str),
    Lock,
    InvalidGestureIndex(i64),
    AnimationWrite(GestureAnimationWriteError),
    Channel(GestureChannelError),
}
impl std::fmt::Display for CharacterGestureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "gesture Actor: {error}"),
            Self::Cvar(error) => write!(f, "gesture cvar: {error:?}"),
            Self::PhysOut(error) => write!(f, "gesture PhysOut: {error:?}"),
            Self::MissingOwner(name) => write!(f, "gesture requires {name}"),
            Self::ForeignOwner(name) => write!(f, "gesture owner mismatch: {name}"),
            Self::Lock => f.write_str("gesture owner lock failed"),
            Self::InvalidGestureIndex(index) => {
                write!(f, "gesture table index outside source bounds: {index}")
            }
            Self::AnimationWrite(error) => write!(f, "gesture animation write: {error}"),
            Self::Channel(error) => write!(f, "gesture channel: {error}"),
        }
    }
}

#[derive(Clone, Copy)]
struct GestureNames {
    both: FastString36,
    left: FastString36,
    right: FastString36,
}
impl GestureNames {
    fn from_index(index: i64) -> Result<Self, CharacterGestureError> {
        let index = usize::try_from(index)
            .ok()
            .filter(|&index| index < names::BOTH.len())
            .ok_or(CharacterGestureError::InvalidGestureIndex(index))?;
        Ok(Self {
            both: ActorGraphIntents::native_name(names::BOTH[index].as_bytes()),
            left: ActorGraphIntents::native_name(names::LEFT[index].as_bytes()),
            right: ActorGraphIntents::native_name(names::RIGHT[index].as_bytes()),
        })
    }
}

impl ActorConstruction {
    /// The generated Motion caller must deliver its real bound context after
    /// Action's intent translator callbacks. No recognizer or Action map is
    /// substituted for the separate retained Motion map here.
    #[expect(
        clippy::too_many_lines,
        reason = "source callback ordering spans the seven retained interfaces and conditional writes"
    )]
    pub(crate) fn try_update_character_gesture(
        &self,
        cvars: &InputCvarReferences,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), CharacterGestureError> {
        use CharacterGestureError as E;
        let mut actor = self.prefix().map_err(E::Actor)?;
        if !context
            .entity()
            .is_some_and(|entity| entity.is_same_issuance(&self.authority.entity_handle()))
        {
            return Err(E::ForeignOwner("IActor"));
        }
        let physical = actor
            .physical_allocation
            .as_ref()
            .ok_or(E::MissingOwner("IPhysical"))?;
        if !actor
            .physical_interface
            .as_ref()
            .is_some_and(|registered| Arc::ptr_eq(registered, physical))
        {
            return Err(E::ForeignOwner("IPhysical"));
        }
        if !actor
            .physical_cached
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, physical))
        {
            return Err(E::MissingOwner("cached Actor IPhysical"));
        }
        let physical = Arc::clone(physical);
        if !actor.animation_published || !actor.motion_graph_published {
            return Err(E::MissingOwner("published animation and Motion graph"));
        }
        let animation_owner = Arc::clone(
            actor
                .animation
                .as_ref()
                .ok_or(E::MissingOwner("ISkaterAnim"))?,
        );
        let mut animation = animation_owner.lock().map_err(|_| E::Lock)?;
        for (index, name) in ["IAnimatable", "IChannelAnimatable", "ISkaterAnim"]
            .into_iter()
            .enumerate()
        {
            if !actor.animation_interfaces[index]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, animation.occurrence()))
            {
                return Err(E::ForeignOwner(name));
            }
        }
        if !animation
            .entity()
            .is_same_issuance(&self.authority.entity_handle())
        {
            return Err(E::ForeignOwner("ISkaterAnim entity"));
        }
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .ok_or(E::MissingOwner("IMotionGraph"))?;
        for (index, name) in [(1, "IMotionGraph"), (2, "ISkaterMotionGraph")] {
            if !actor.graph_interfaces[index]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, motion.occurrence()))
            {
                return Err(E::ForeignOwner(name));
            }
        }
        let controller = motion
            .controller
            .as_ref()
            .ok_or(E::MissingOwner("Motion controller"))?;
        if !context
            .controller()
            .is_some_and(|token| token.is_same_issuance(controller.controller_identity()))
        {
            return Err(E::ForeignOwner("Motion controller"));
        }

        #[cfg(feature = "skate3-runtime")]
        if animation.pc_gesture_preferences.selected==skate2_formats::skater_profile::GestureSet::Skate3 {
            return Ok(());
        }
        // Up, Down, Left, Right is the real SkaterGestureSet member order.
        // Debug reads Down before Up, then Left and Right, as in 82cc38bc.
        let indices = if cvars.try_enable_debug_gestures().map_err(E::Cvar)? {
            let down = cvars.try_debug_gesture_down().map_err(E::Cvar)?;
            let up = cvars.try_debug_gesture_up().map_err(E::Cvar)?;
            let left = cvars.try_debug_gesture_left().map_err(E::Cvar)?;
            let right = cvars.try_debug_gesture_right().map_err(E::Cvar)?;
            [
                i64::from(up),
                i64::from(down),
                i64::from(left),
                i64::from(right),
            ]
        } else {
            animation
                .player()
                .ok_or(E::MissingOwner("GetGestureSet player"))?
                .gestures
                .map(i64::from)
        };
        let sets = indices.map(GestureNames::from_index);
        let [up, down, left, right] = sets;
        let sets = [up?, down?, left?, right?];
        let intents = motion
            .intents
            .as_ref()
            .ok_or(E::MissingOwner("Motion intents"))?;
        let mut requested = None;
        // Presence, including a zero value, is significant. Later matches win.
        for (name, index, getter) in [
            (
                b"GestureDown".as_slice(),
                1,
                InputCvarReferences::try_debug_gesture_down
                    as fn(&InputCvarReferences) -> Result<i32, InputCvarError>,
            ),
            (
                b"GestureLeft".as_slice(),
                2,
                InputCvarReferences::try_debug_gesture_left,
            ),
            (
                b"GestureRight".as_slice(),
                3,
                InputCvarReferences::try_debug_gesture_right,
            ),
            (
                b"GestureUp".as_slice(),
                0,
                InputCvarReferences::try_debug_gesture_up,
            ),
        ] {
            if intents.get(&ActorGraphIntents::native_name(name)).is_some() {
                // This enum read also occurs with debug gestures disabled.
                requested = Some((sets[index], getter(cvars).map_err(E::Cvar)?));
            }
        }
        let channels = animation
            .initialization()
            .and_then(|initialization| initialization.channels.as_ref())
            .ok_or(E::MissingOwner("initialized channel dictionary"))?;
        let left_name = ActorGraphIntents::native_name(b"GestureLeft");
        let right_name = ActorGraphIntents::native_name(b"GestureRight");
        let both_name = ActorGraphIntents::native_name(b"GestureBoth");
        let mut had_left = channels.dictionary.contains_key(&left_name);
        let mut had_right = channels.dictionary.contains_key(&right_name);
        if channels.dictionary.contains_key(&both_name) {
            had_left = true;
            had_right = true;
        }
        let mut allow_both = !had_left && !had_right;
        let mut allow_left = !had_left;
        let mut allow_right = !had_right;
        let output = physical.owner().phys_out();
        let current = output
            .lock()
            .map_err(|_| E::Lock)?
            .try_filtered_output()
            .map_err(E::PhysOut)?
            .filtered_physics_state;
        let hippy = output
            .lock()
            .map_err(|_| E::Lock)?
            .try_ground_output()
            .map_err(E::PhysOut)?
            .is_hippy_jumping;
        if matches!(current as u32, 6 | 7) && !hippy {
            if output
                .lock()
                .map_err(|_| E::Lock)?
                .try_offboard_output()
                .map_err(E::PhysOut)?
                .is_holding_skateboard
            {
                allow_both = false;
                allow_right = false;
                if had_right {
                    animation
                        .try_end_gesture_channel(&right_name)
                        .map_err(E::AnimationWrite)?;
                    animation
                        .try_end_gesture_channel(&both_name)
                        .map_err(E::AnimationWrite)?;
                    had_right = false;
                }
            }
        } else {
            let grab = motion
                .state
                .as_ref()
                .ok_or(E::MissingOwner("GrabState"))?
                .grab_busy;
            // Raw 82cc4428/58 reads Fs(+4) for left; 82cc447c reads Bs(+0) for right.
            allow_both = allow_both && grab[1] == 0 && grab[0] == 0;
            allow_left = allow_left && grab[1] == 0;
            allow_right = allow_right && grab[0] == 0;
        }
        let mut started_gesture = None;
        if let Some((names, reported_enum)) = requested {
            let selected = if allow_both {
                Some((both_name, names.both))
            } else if allow_left {
                Some((left_name, names.left))
            } else if allow_right {
                Some((right_name, names.right))
            } else {
                None
            };
            if let Some((channel, animation_name)) = selected {
                // Native ignores the returned channel status, including real
                // AlreadyPresent/NoFree/AnimationMissing results.
                animation
                    .try_new_gesture_channel(channel, animation_name)
                    .map_err(E::Channel)?;
                started_gesture = Some(u32::from_ne_bytes(reported_enum.to_ne_bytes()));
            }
        }
        // Release immutable animation/graph borrows before the actual graph write.
        drop(animation);
        if had_left || had_right || started_gesture.is_some() {
            let state = actor
                .graphs
                .as_mut()
                .and_then(|graphs| graphs.motion_mut())
                .and_then(|motion| motion.state.as_mut())
                .ok_or(E::MissingOwner("SetIsGesturing"))?;
            // No new channel: source reports eAnimGestureNames::Invalid (35).
            state.gesture = started_gesture.unwrap_or(35);
            state.is_gesturing = true;
        }
        if had_left || had_right || requested.is_some() {
            let offboard = output
                .lock()
                .map_err(|_| E::Lock)?
                .try_state_output()
                .map_err(E::PhysOut)?
                .is_offboard;
            let value = if offboard {
                1.0
            } else {
                output
                    .lock()
                    .map_err(|_| E::Lock)?
                    .try_animation_output()
                    .map_err(E::PhysOut)?
                    .centre_of_mass_to_deck
            };
            let name = FastString30::encode("GstrDistToCog").expect("source static name");
            animation_owner
                .lock()
                .map_err(|_| E::Lock)?
                .try_add_gesture_attribute(name, value, false, -1)
                .map_err(E::AnimationWrite)?;
        }
        Ok(())
    }
}
