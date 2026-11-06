//! DetermineOverallCollisionType82e808d8. Facts come from actual contact and
//! entity/interface owners; this module performs no replacement entity lookup.
use super::{PedestrianBodyKind, PedestrianCollision};
use crate::reckoning_numeric::{ReckoningNumeric, cross_full4, try_length};
use crate::simulation_assembly_backend::constraint_session::physical_island::SimulationIslandBodyHandle;

/// What a pedestrian's collision response reads of a contact: the Part, the normal on this Body's
/// side, a self contact and the other Body.
pub(crate) trait CollisionContact {
    fn part(&self) -> u32;
    fn normal(&self) -> &[f32; 3];
    fn self_colliding(&self) -> bool;
    fn other_body(&self) -> Option<&SimulationIslandBodyHandle>;
}
impl CollisionContact for crate::simulation_assembly_backend::constraint_session::physical_contact_list::PhysicalContactInfo {
    fn part(&self) -> u32 { Self::part(self) }
    fn normal(&self) -> &[f32; 3] { Self::normal(self) }
    fn self_colliding(&self) -> bool { Self::self_colliding(self) }
    fn other_body(&self) -> Option<&SimulationIslandBodyHandle> { Self::other_body(self) }
}
/// PC port (Skate 3 physics): the Skate 3 solver's contact between the local skater and a
/// pedestrian collision Part, as TU3's shared island records it on the pedestrian's side
/// (`cLivingWorldCollisionBody::GetContacts` 0x82e0e7b0).
pub(crate) struct Skate3PedestrianContact {
    pub(crate) body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
    pub(crate) normal: [f32; 3],
    /// The contact point, and the velocity of the skater's side (its board or its root).
    pub(crate) point: [f32; 3],
    pub(crate) velocity: [f32; 3],
    pub(crate) other: SimulationIslandBodyHandle,
}
enum AnyContact<'a> {
    Island(&'a crate::simulation_assembly_backend::constraint_session::physical_contact_list::PhysicalContactInfo),
    Skate3 { part: u32, contact: &'a Skate3PedestrianContact },
}
impl CollisionContact for AnyContact<'_> {
    fn part(&self) -> u32 {
        match self { Self::Island(contact) => contact.part(), Self::Skate3 { part, .. } => *part }
    }
    fn normal(&self) -> &[f32; 3] {
        match self { Self::Island(contact) => contact.normal(), Self::Skate3 { contact, .. } => &contact.normal }
    }
    fn self_colliding(&self) -> bool {
        match self { Self::Island(contact) => contact.self_colliding(), Self::Skate3 { .. } => false }
    }
    fn other_body(&self) -> Option<&SimulationIslandBodyHandle> {
        match self { Self::Island(contact) => contact.other_body(), Self::Skate3 { contact, .. } => Some(&contact.other) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum PedestrianCollisionType {
    Ignore = 0, AdjustWorld = 1, AdjustVehicle = 2, AdjustDmoInteraction = 3,
    Light = 4, Full = 5,
}
pub(crate) struct OtherPedestrian {
    pub(crate) position: [f32; 4],
    pub(crate) radius: f32,
}
pub(crate) struct OtherCollisionOwner {
    pub(crate) has_owner: bool,
    pub(crate) vehicle: bool,
    pub(crate) dynamic_object: bool,
    pub(crate) interacting_dynamic_object: bool,
    pub(crate) matches_ignore_target: bool,
    pub(crate) skater_position: Option<[f32; 4]>,
    pub(crate) pedestrian: Option<OtherPedestrian>,
}
pub(crate) struct CollisionClassificationFacts {
    /// Comparison against this collision owner's actual Body identities.
    pub(crate) this_body: Option<PedestrianBodyKind>,
    pub(crate) part: usize,
    pub(crate) normal: [f32; 4],
    pub(crate) target_up: [f32; 4],
    pub(crate) target_forward: [f32; 4],
    pub(crate) position: [f32; 4],
    pub(crate) velocity: [f32; 4],
    pub(crate) adjust_world: bool, // pedestrian+6724 bit40
    pub(crate) ignore_skater_collisions: bool, // state+3241 bit08
    pub(crate) current_losim: u32,
    pub(crate) num_chasers: u32,
    pub(crate) has_chasee: bool,
    pub(crate) own_radius: f32,
    pub(crate) pedestrian_ignorance_factor: f32,
    pub(crate) skater_ignorance_factor: f32,
}
impl PedestrianCollision {
    ///82e808d8: part flags deliberately precede several ignore branches.
    /// The final callback reads active/override animation layout byte16 and
    /// IsAttemptingTakedown only for an in-range arm part (native tags1/2).
    pub(crate) fn determine_collision_type<N: ReckoningNumeric>(
        &mut self, facts: &CollisionClassificationFacts, other: &OtherCollisionOwner,
        mut arm_light_collision: impl FnMut() -> Result<bool, String>, numeric: &mut N,
    ) -> Result<PedestrianCollisionType, String>
    where N::Error: std::fmt::Debug,
    {
        use PedestrianCollisionType as T;
        if facts.this_body == Some(PedestrianBodyKind::World) {
            self.set_part_colliding(PedestrianBodyKind::World, facts.part, true)?;
            if other.has_owner && other.vehicle { return Ok(T::AdjustVehicle); }
            if other.has_owner && other.dynamic_object && other.interacting_dynamic_object {
                return Ok(T::AdjustDmoInteraction);
            }
            if facts.adjust_world {
                let cross = cross_full4(facts.target_up, facts.normal);
                let forward_dot = numeric.dot3(facts.target_forward, facts.normal)
                    .map_err(|e| format!("pedestrian world collision direction: {e:?}"))?;
                let length = try_length(cross, numeric)
                    .map_err(|e| format!("pedestrian world collision slope: {e:?}"))?;
                if length > 0.45_f32 {
                    if forward_dot < 0. { return Ok(T::AdjustWorld); }
                    let velocity_dot = numeric.dot3(facts.velocity, facts.normal)
                        .map_err(|e| format!("pedestrian world collision velocity: {e:?}"))?;
                    if velocity_dot < 0. { return Ok(T::AdjustWorld); }
                }
            }
            return Ok(T::Ignore);
        }
        if facts.this_body != Some(PedestrianBodyKind::Skater) || !other.has_owner
            || other.dynamic_object || facts.ignore_skater_collisions {
            return Ok(T::Ignore);
        }
        if other.skater_position.is_none() && other.pedestrian.is_none() {
            return Err("DetermineOverallCollisionType native assertion: skater body contacted non-actor/non-pedestrian".into());
        }
        if other.matches_ignore_target { return Ok(T::Ignore); }
        self.set_part_colliding(PedestrianBodyKind::Skater, facts.part, true)?;
        let (other_position, radius) = if let Some(pedestrian) = &other.pedestrian {
            if facts.current_losim == 2 && facts.num_chasers == 0 && !facts.has_chasee {
                return Ok(T::Ignore);
            }
            (pedestrian.position, (pedestrian.radius + facts.own_radius) * facts.pedestrian_ignorance_factor)
        } else {
            self.colliding_with_skater = true;
            (other.skater_position.expect("checked actual actor above"), facts.own_radius * facts.skater_ignorance_factor)
        };
        let difference = std::array::from_fn(|i| facts.position[i] - other_position[i]);
        let squared = numeric.dot3(difference, difference)
            .map_err(|e| format!("pedestrian collision distance: {e:?}"))?;
        if squared - radius * radius > 0. { return Ok(T::Ignore); }
        if matches!(facts.part, 1 | 2) && arm_light_collision()? { return Ok(T::Light); }
        Ok(T::Full)
    }
}

/// Actual owner/interface observations from the shared Island contact lookup.
/// Presence is separate from targetability: an owner without ITargetable still
/// sets collisionflag80 and may publish the actual actor's scoring bit.
pub(crate) struct ContactEntity {
    pub(crate) targetable: Option<crate::living_world_chase::TargetableId>,
    pub(crate) actor: Option<(bool, u32)>, // IsLocalPlayer, GetPlayerIndex
    /// Some(actual IsAttemptingTakedown), including an own-body self contact.
    pub(crate) pedestrian: Option<bool>,
}

impl crate::living_world_pedestrian_entity::PedestrianConstruction {
    /// All branches of OnEntityCollision82e95158. A false native callback
    /// return means traversal continues after every contact, even full hits.
    pub(crate) fn resolve_classified_collision<N: ReckoningNumeric>(&mut self,
        contact: &impl CollisionContact,
        kind: PedestrianCollisionType, other: Option<&ContactEntity>,
        selected_animation: &crate::attribute_runtime::AttributeRuntimeInstance,
        world: &crate::physical_player_assembly::PhysicalAssemblyWorld,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        random: &mut crate::native_skater_random::NativeSkaterRandom, numeric: &mut N,
    ) -> Result<(), String> where N::Error: std::fmt::Debug {
        self.record_collision_contact_owner(contact, kind, other)?;
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static CONTACTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some())
            && other.is_some_and(|entity| entity.actor.is_some_and(|(local, _)| local))
        {
            let count = CONTACTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 20 || count % 120 == 0 {
                std::eprintln!("pedestrian physical local-skater contact count={count} kind={kind:?}");
            }
        }
        match kind {
            PedestrianCollisionType::Ignore => Ok(()),
            PedestrianCollisionType::AdjustWorld | PedestrianCollisionType::AdjustVehicle => {
                let mut target = self.core.as_ref().ok_or("world adjustment requires core")?.target_transform;
                let part = self.collision.as_ref().ok_or("world adjustment requires collision")?
                    .part_transform(PedestrianBodyKind::World, 0, world)?;
                let offset = if self.renderable {
                    //A pedestrian spawned this frame meets World.Update3 before its first
                    //UpdateSimLODGlobal; MemStore Init82a6dd98/FreeBlock82a6df20 zero the block.
                    let bone = self.animation.as_ref().ok_or("renderable adjustment requires animation")?
                        .sim_lod_bone_global.as_ref().map_or([0.; 4], |global| global.matrices[8][3]);
                    std::array::from_fn(|i| bone[i] - target[3][i])
                } else { [0.; 4] };
                let mut position = std::array::from_fn::<_, 4, _>(|i| part[12+i] - offset[i]);
                //vrlimi mask4 replaces Y only; W remains the actual Part result.
                position[1] = target[3][1];
                if position[..3].iter().all(|x| x.is_finite() && x.abs() <= 1_000_000.) {
                    target[3] = position;
                }
                self.core.as_mut().unwrap().set_target_transform::<std::convert::Infallible>(target)
                    .map_err(|e| format!("world collision target: {e:?}"))
            }
            PedestrianCollisionType::AdjustDmoInteraction => {
                self.state.as_mut().ok_or("DMO collision requires state")?.flags[2] |= 0x80;
                Ok(())
            }
            PedestrianCollisionType::Light => {
                use crate::pedestrian_motion_delegate::channel::Status;
                if self.animation.as_ref().ok_or("light contact requires animation")?.channels.channel.status != Status::Finished
                    || self.collision.as_ref().ok_or("light contact requires collision")?.colliding { return Ok(()); }
                let forward = self.core.as_ref().ok_or("light contact requires core")?.transform[2];
                let degrees = contact_degrees(forward, contact, numeric)?;
                let back = degrees.abs() < 65.;
                let name = if degrees <= 0. {
                    if back { "LightHitRightBackChannel" } else { "LightHitRightChannel" }
                } else if back { "LightHitLeftBackChannel" } else { "LightHitLeftChannel" };
                //These names never use the four registered-clip routes. Preserve
                //the shared RemapAnim authored-array selection and Manager RNG.
                let mut random_error = None;
                let mapped = crate::pedestrian_animation::remap_animation(name,
                    &self.animation.as_ref().unwrap().motion_intents.registered_clips, selected_animation,
                    || match random.try_next(&crate::native_skater_random::NativeRandomGate::production_shared()) {
                        Ok(value) => value, Err(error) => { random_error = Some(error); 0 }
                    }).map_err(|e| e.to_string())?;
                if let Some(error) = random_error { return Err(format!("light collision RNG: {error:?}")); }
                let animation = crate::play_animation::FastString36::encode(&mapped)
                    .map_err(|e| format!("light collision animation name: {e:?}"))?;
                self.animation.as_mut().unwrap().channels.play(animation, 0.2, 0.2, false)?;
                //Entity vslot184 is SetAudioState82e94ca8, not an event callback.
                self.state_out.state = 8;
                Ok(())
            }
            PedestrianCollisionType::Full => self.resolve_full_collision(contact,
                other.and_then(|entity| entity.pedestrian), selected_animation, world, cvars, numeric),
        }
    }
    /// OnEntityCollision82e951ac..953d4. Called after the sole existing
    /// determine_collision_type owner, before dispatching any response branch.
    /// ContactInfo carries the original sealed Spy/Body identities.
    pub(crate) fn record_collision_contact_owner(&mut self,
        contact: &impl CollisionContact,
        kind: PedestrianCollisionType, other: Option<&ContactEntity>,
    ) -> Result<(), String> {
        if contact.self_colliding() || !matches!(kind, PedestrianCollisionType::Light | PedestrianCollisionType::Full) {
            return Ok(());
        }
        let Some(other) = other else { return Ok(()); };
        let state = self.state.as_mut().ok_or("collision contact requires AI state")?;
        state.collision_flags |= 0x80;
        let local = other.actor.is_some_and(|(local, _)| local);
        if let Some((_, index)) = other.actor {
            if local { state.collision_flags |= 0x40; }
            state.skater_collision_flags |= 1u32.checked_shl(index & 63).unwrap_or(0);
        }
        if let Some(target) = other.targetable {
            let sink = state.take_down_sink;
            let mut replace = false;
            let mut permit = !sink.is_valid();
            if sink.is_valid() {
                if target == sink { replace = true; }
                else if state.collider != sink { permit = true; }
            }
            if permit {
                if local { replace = true; }
                else if state.collision_flags & 0x40 == 0 {
                    replace = (kind as i32) > state.highest_collision_type;
                }
            }
            if replace { state.highest_collision_type = kind as i32; state.collider = target; }
        }
        Ok(())
    }
    /// `Pedestrian::ResetCollisionFlags` 0x82e6e550: the collideable object's
    /// part bits (`LWCollideableObject::ResetCollisionFlags` 0x82e4c6c8), then,
    /// once the last collision is over (0x20), the collider and highest type
    /// clear, 0x80/0x40/0x20 clear and 0x10 lets the next collision broadcast.
    pub(crate) fn reset_collision_flags(&mut self) {
        if let Some(collision) = self.collision.as_mut() { collision.reset_collision_flags(); }
        let Some(state) = self.state.as_mut() else { return; };
        if state.collision_flags & 0x20 != 0 {
            state.collider = crate::living_world_chase::TargetableId::INVALID;
            state.highest_collision_type = 0;
            state.collision_flags = (state.collision_flags & 0x0f) | 0x10;
        }
    }

    /// Full response of OnEntityCollision82e957d8..95d74. The caller has
    /// already run determine_collision_type and record_collision_contact_owner.
    /// Uses actual SKATER Part0 body velocities, never relative contact speed,
    /// entity.velocity, or a substituted target transform. Other pedestrian is
    /// resolved from the original other Body owner, not a proximity search.
    pub(crate) fn resolve_full_collision<N: ReckoningNumeric>(&mut self,
        contact: &impl CollisionContact,
        other_pedestrian: Option<bool>,
        selected_animation: &crate::attribute_runtime::AttributeRuntimeInstance,
        world: &crate::physical_player_assembly::PhysicalAssemblyWorld,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), String> where N::Error: std::fmt::Debug {
        use crate::pedestrian_graph::colliding::{CollisionDirection as D, CollisionForce};
        use crate::reckoning_numeric::length_from_squared_and_estimate;
        use crate::physical_player_trajectory_owners::selector_grind::{vmax, vmin};
        let error = |e| format!("pedestrian full collision numeric: {e:?}");
        let collision = self.collision.as_ref().ok_or("full contact requires collision object")?;
        if collision.forced_to_recover { return Ok(()); }
        let (velocity, omega) = collision.skater_part_velocities(0, world)?;
        //Native dots both vectors, estimates both, then refines both twice.
        let speed_squared = numeric.dot3(velocity, velocity).map_err(&error)?;
        let angular_squared = numeric.dot3(omega, omega).map_err(&error)?;
        let speed_estimate = numeric.reciprocal_sqrt_estimate(speed_squared).map_err(&error)?;
        let angular_estimate = numeric.reciprocal_sqrt_estimate(angular_squared).map_err(&error)?;
        let speed = length_from_squared_and_estimate(speed_squared, speed_estimate);
        let angular = length_from_squared_and_estimate(angular_squared, angular_estimate);
        let attributes = &self.resources.as_ref().ok_or("full contact requires entity attributes")?.entity;
        let max_linear = collision_scalar(attributes, 0xc042_36fb_5486_97d0)?;
        //Raw bgt/ble82e95938/4c, including unordered angular fallthrough.
        if !(speed > max_linear) {
            let max_angular = collision_scalar(attributes, 0x797a_a1d5_f828_819b)?;
            if angular <= max_angular { return Ok(()); }
        }
        //Entity vslot20 GetTransform82e94670 reads core+16 (actual), not+80 target.
        let forward = self.core.as_ref().ok_or("full contact requires core")?.transform[2];
        let degrees = contact_degrees(forward, contact, numeric)?;
        let direction = if !(degrees >= 35.) && !(degrees <= -35.) { D::Back }
            else if !(degrees > -35.) && !(degrees < -145.) { D::Right }
            else if !(degrees < 35.) && degrees <= 145. { D::Left } else { D::Front };
        let allowed = selected_animation.try_cached_layout_field(17, 1).map_err(|e| e.to_string())?
            .ok_or("full contact requires selected animation cached layout")?.bytes()[0] != 0
            && self.state.as_ref().ok_or("full contact requires AI state")?.flags[1] & 0x20 == 0;
        let mut taken_down = false;
        if allowed {
            if let Some(other) = other_pedestrian {
                if other {
                    taken_down = true;
                    let counter = self.state.as_mut().unwrap().takedown_count.as_mut()
                        .ok_or("collision takedown counter lacks constructor initialization")?;
                    *counter = counter.wrapping_add(1);
                }
            }
        }
        //Both ordinary values are loaded even if the following pedestrian
        //branch overwrites them, and even when a takedown forces the result.
        let mut linear_threshold = collision_scalar(selected_animation, 0x541f_fa2e_9d81_c947)?;
        let mut angular_threshold = collision_scalar(selected_animation, 0x7c5e_39ec_e5a5_572e)?;
        if other_pedestrian.is_some() {
            linear_threshold = collision_scalar(selected_animation, 0x081f_978f_8706_d77b)?;
            angular_threshold = collision_scalar(selected_animation, 0x5e56_6360_fdc5_fdd1)?;
        }
        let force = if taken_down || allowed && (speed > linear_threshold || angular > angular_threshold) {
            CollisionForce::Knockdown
        } else {
            if self.state.as_ref().unwrap().flags[1] & 0x10 != 0 { return Ok(()); }
            CollisionForce::Standing
        };
        self.record_accepted_full_collision(force, direction, cvars)
    }
}
fn contact_degrees<N: ReckoningNumeric>(forward: [f32;4],
    contact: &impl CollisionContact, numeric: &mut N,
) -> Result<f32, String> where N::Error: std::fmt::Debug {
    use crate::reckoning_numeric::{normalize_from_squared_and_estimate, try_acos};
    use crate::physical_player_trajectory_owners::selector_grind::{vmax, vmin};
    let error = |e| format!("pedestrian contact direction: {e:?}");
    let squared = numeric.dot3(forward, forward).map_err(&error)?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared).map_err(&error)?;
    let forward = normalize_from_squared_and_estimate(forward, squared, estimate);
    let normal = [contact.normal()[0], 0., contact.normal()[2], 0.];
    let dot = numeric.dot3(normal, forward).map_err(&error)?;
    let mut angle = try_acos(vmin(1., vmax(-1., dot)), numeric).map_err(&error)?;
    if cross_full4(forward, normal)[1] > 0. { angle *= -1.; }
    Ok(angle * f32::from_bits(0x4265_2ee1))
}
fn collision_scalar(instance: &crate::attribute_runtime::AttributeRuntimeInstance, key: u64) -> Result<f32, String> {
    Ok(f32::from_bits(instance.try_scalar_word(key, 0).map_err(|e| e.to_string())?.unwrap_or(0)))
}

impl crate::living_world_grind_services::LivingWorldGrindServices {
    /// `LWPedestrianMan::ResetCollisionFlags` 0x82e83580, the first collision
    /// reset of `Manager::Update1` (0x827281e8), for every pedestrian.
    pub(crate) fn pedestrian_reset_collision_flags(&mut self) {
        let Some(pedestrians) = self.pedestrian_entities.as_deref_mut() else { return; };
        for id in pedestrians.ids() {
            if let Some(entity) = pedestrians.get_mut(id) { entity.reset_collision_flags(); }
        }
    }
    /// Island.ResolveContactSpyList827b3a90 -> collision Body.GetContacts82e50e60.
    /// Every contact of a pedestrian body reaches the one classifier/dispatcher;
    /// OnEntityCollision always returns false, so there is no first-hit break.
    pub(crate) fn pedestrian_resolve_contacts(&mut self,
        world: &mut crate::physical_player_assembly::PhysicalAssemblyWorld,
        local: &crate::skater_manager_local_identity::SkaterManagerLocalIdentity,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
    ) -> Result<(), String> {
        use crate::simulation_assembly_backend::constraint_session::physical_island::{
            SimulationIslandBodyHandle as Body, contact_owners::SimulationContactEntityOwner as Owner};
        use crate::living_world_chase::TargetableId;
        let skate3_contacts = std::mem::take(&mut self.skate3_contacts);
        let random=self.random.as_mut().ok_or("pedestrian contacts require Manager RNG")?;
        let pedestrians = self.pedestrian_entities.as_deref_mut().ok_or("pedestrian contact roster absent")?;
        for id in pedestrians.ids() {
            let (entity, others) = pedestrians.split_entity_mut(id).ok_or("pedestrian contact entity disappeared")?;
            for body_kind in [PedestrianBodyKind::World, PedestrianBodyKind::Skater] {
                let Some(handle) = entity.collision.as_ref().ok_or("pedestrian contact collision absent")?
                    .attached_handle(body_kind).cloned() else { continue; };
                let body = Body::Pedestrian(handle.clone());
                //Clone preserves sealed Spy frame and exact Body handles; only
                //releases the world borrow so callbacks can read actual Parts.
                let contacts = world.try_get_island_collisions(&body)
                    .map_err(|e| format!("pedestrian Island contacts: {e:?}"))?.to_vec();
                let mut all: Vec<AnyContact<'_>> = contacts.iter().map(AnyContact::Island).collect();
                if body_kind == PedestrianBodyKind::Skater {
                    for contact in &skate3_contacts {
                        if let Some(part) = world.pedestrian_body_part(&handle, contact.body) {
                            all.push(AnyContact::Skate3 { part, contact });
                        }
                    }
                }
                for contact in &all {
                    let owner = contact.other_body().map(|other| world.island_body_owner_for_contacts(other))
                        .transpose().map_err(|e| format!("pedestrian other contact owner: {e:?}"))?;
                    let other_id = match owner.as_ref() { Some(Owner::LivingWorld(id)) => Some(*id), _ => None };
                    let other_pedestrian = other_id.and_then(|id| others.iter().find(|(key,_)| *key == id).map(|(_,value)| *value));
                    let other_is_self = other_id == Some(id);
                    let other_attempting = if other_is_self { Some(entity.state.as_ref().ok_or("self contact state absent")?.flags[0]&2!=0) }
                        else { other_pedestrian.map(|other| other.state.as_ref().map(|state|state.flags[0]&2!=0).ok_or("other contact state absent".to_string())).transpose()? };
                    let mut targetable = other_id.filter(|_| other_attempting.is_some())
                        .map(|id| TargetableId { kind:1, id:id as i32 });
                    let mut actor = None;
                    let mut skater_position = None;
                    if let Some(Owner::Physical(owner)) = &owner {
                        let query = owner.actor_query.lock().map_err(|e| format!("contact Actor lock: {e:?}"))?;
                        let query = query.as_ref().ok_or("contact physical has no Actor")?;
                        let is_local = local.try_is_local_player(|| query.try_player_index().map(|v|v as i32))
                            .map_err(|e| format!("contact Actor locality: {e:?}"))?;
                        let index = query.try_player_index().map_err(|e| e.to_string())?;
                        actor = Some((is_local, index));
                        targetable = Some(TargetableId { kind:0, id:index as i32 });
                        //Actor.GetPosition825f9df0 reads PhysOut_Skeleton+1a0,
                        //the root translation, not system centre of mass.
                        let output = owner.phys_out().lock().map_err(|e| format!("contact Actor PhysOut lock: {e:?}"))?;
                        let position = output.try_skeleton_output().map_err(|e| format!("contact Actor position: {e:?}"))?
                            .skeleton_root_matrix.columns()[3];
                        skater_position = Some([position.xyz[0],position.xyz[1],position.xyz[2],position.w.unwrap_or(0.)]);
                    }
                    let vehicle = matches!(contact.other_body(), Some(Body::Vehicle(_)));
                    let dynamic_object = matches!(contact.other_body(), Some(Body::Dmo(_)));
                    let observation = OtherCollisionOwner {
                        has_owner:owner.is_some(),vehicle,dynamic_object,
                        interacting_dynamic_object: dynamic_object && entity.interacting_object.is_some() && entity.interacting_object == other_id,
                        matches_ignore_target: targetable.is_some_and(|value| value == entity.ignore_collision_target),
                        skater_position,
                        pedestrian: (if other_is_self { Some(&*entity) } else { other_pedestrian }).map(|other| -> Result<_,String> {
                            Ok(OtherPedestrian { position:other.core.as_ref().ok_or("other contact pedestrian core absent")?.transform[3],
                                radius:model_radius(other)? })
                        }).transpose()?,
                    };
                    let core = entity.core.as_ref().ok_or("contact pedestrian core absent")?;
                    let state = entity.state.as_ref().ok_or("contact pedestrian state absent")?;
                    let selected = entity.animation.as_ref().ok_or("contact pedestrian animation absent")?
                        .selected_animation_data()?.clone();
                    let num_chasers=entity.chasee.as_ref().ok_or("contact pedestrian Chasee absent")?.chasers.len() as u32;
                    //82e80cac calls GetChasee82e86220, which resolves the live
                    //entity AND its IChasee interface. A valid but culled ID
                    //must not keep distant NPC/NPC collisions enabled.
                    let has_chasee=if body_kind==PedestrianBodyKind::Skater
                        && observation.pedestrian.is_some() && !observation.matches_ignore_target
                        && state.flags[1]&8==0 && core.current_losim==2 && num_chasers==0 {
                        let target=state.chaser.chasee;
                        if !target.is_valid(){false}else{match target.kind {
                            0=>match local.try_find_by_index(target.id).map_err(|e|format!("collision chasee Actor lookup: {e:?}"))? {
                                None=>false,
                                Some(actor)=>{
                                    actor.prefix().map_err(|e|format!("collision chasee Actor: {e:?}"))?
                                        .chasee.as_ref().ok_or("collision target Actor Chasee absent")?;
                                    true
                                },
                            },
                            1=>{
                                let target=if target.id as u32==id{Some(&*entity)}else{
                                    others.iter().find(|(key,_)|*key==target.id as u32).map(|(_,value)|*value)
                                };
                                target.map(|target|target.chasee.as_ref().ok_or("collision target pedestrian Chasee absent"))
                                    .transpose()?.is_some()
                            }
                            //Vehicle and DMO expose ITargetable but no IChasee.
                            _=>false,
                        }}
                    }else{false};
                    let facts = CollisionClassificationFacts {
                        this_body:Some(body_kind),part:contact.part() as usize,
                        normal:[contact.normal()[0],contact.normal()[1],contact.normal()[2],0.],
                        target_up:core.target_transform[1],target_forward:core.target_transform[2],
                        position:core.transform[3],velocity:core.velocity,
                        adjust_world:entity.adjust_world_collisions,ignore_skater_collisions:state.flags[1]&8!=0,
                        current_losim:core.current_losim,
                        num_chasers,has_chasee,own_radius:model_radius(entity)?,
                        pedestrian_ignorance_factor:cvars.try_pedestrian_collision_ignorance_radius_factor().map_err(|e|format!("{e:?}"))?,
                        skater_ignorance_factor:cvars.try_pedestrian_skater_collision_ignorance_radius_factor().map_err(|e|format!("{e:?}"))?,
                    };
                    let attempting_takedown = state.flags[0]&2!=0;
                    let kind = entity.collision.as_mut().unwrap().determine_collision_type(&facts,&observation,|| {
                        Ok(selected.try_cached_layout_field(16,1).map_err(|e|e.to_string())?
                            .ok_or("contact animation cached layout absent")?.bytes()[0]!=0 && !attempting_takedown)
                    }, &mut crate::reckoning_numeric::HostReckoningNumeric)?;
                    let other = owner.as_ref().map(|_|ContactEntity {targetable,actor,pedestrian:other_attempting});
                    entity.resolve_classified_collision(contact,kind,other.as_ref(),&selected,world,cvars,random,
                        &mut crate::reckoning_numeric::HostReckoningNumeric)?;
                }
            }
        }
        Ok(())
    }
}
fn model_radius(entity: &crate::living_world_pedestrian_entity::PedestrianConstruction) -> Result<f32,String> {
    //LWModel.GetRadius82e51708: half dimensions, fsel(X-Z,X,Z).
    let dimensions = entity.model.as_ref().ok_or("collision pedestrian model absent")?.dimensions;
    let x=dimensions[0]*0.5; let z=dimensions[2]*0.5;
    Ok(if x-z>=0. {x} else {z})
}


