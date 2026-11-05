//! cLWPedestrianCollision82e720c0 and InitCollisionBodies82e99df0.
//! Candidates move into the existing Island pedestrian pool on admission.
//! Native pose/plugin transform producers remain explicit caller inputs.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    living_world_grind_services::character_models::{CharacterModel, CharacterModelKind,
        collision_assembly::CharacterCollisionAssembly},
    physical_player_assembly::PhysicalAssemblyWorld,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::entity_core::CommonEntityCore,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_assembly_backend::constraint_session::physical_island::vehicle::{
        LivingWorldBodyKind, VehicleAdmissionFailure,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::{
        SimulationDmoAssemblyHandle, character::CharacterAssemblyCandidate,
    },
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;
#[path = "pedestrian_collision_contacts.rs"]
pub(crate) mod contacts;
#[path = "pedestrian_collision_shove.rs"]
pub(crate) mod shove;
#[path = "pedestrian_collision_transforms.rs"]
mod transforms;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PedestrianBodyKind { World, Skater }
pub(crate) enum PedestrianCollisionBody {
    Constructed(CharacterAssemblyCandidate),
    InWorld(SimulationDmoAssemblyHandle),
}

pub(crate) struct PedestrianCollision {
    pub(crate) ignore_gravity: bool,
    pub(crate) required_to_begin_colliding: bool,
    pub(crate) forced_to_recover: bool,
    pub(crate) colliding: bool,
    pub(crate) colliding_with_skater: bool,
    pub(crate) num_recovery_attempts: u32,
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) world_part_colliding: bool,
    pub(crate) skater_parts_colliding: [bool; 3],
    pub(crate) model: Option<Arc<CharacterModel>>,
    pub(crate) world_body: Option<PedestrianCollisionBody>,
    pub(crate) skater_body: Option<PedestrianCollisionBody>,
}
impl PedestrianCollision {
    pub(crate) fn set_world_collision_group<C>(&mut self, world: &mut crate::simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession<AssemblyPartMaterial,C>, group: u32)
        -> Result<(), String> {
        match self.world_body.as_mut().ok_or("pedestrian world body absent")? {
            PedestrianCollisionBody::Constructed(candidate) => {
                candidate.assembly.group = group;
                candidate.assembly.part_user_data.fill(group);
                Ok(())
            }
            PedestrianCollisionBody::InWorld(handle) => world.try_set_living_world_collision_group(
                LivingWorldBodyKind::Pedestrian, handle, group).map_err(|e| format!("pedestrian collision group: {e:?}")),
        }
    }
    pub(crate) fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self { ignore_gravity: true,
            // Shared cLivingWorldCollision ctor82e571e8, explicit stores.
            required_to_begin_colliding: false, forced_to_recover: false,
            colliding: false, colliding_with_skater: false, num_recovery_attempts: 0,
            attributes: vault.instance(crate::living_world_rain::LIVING_WORLD_CLASS,
                0x5b6a_0051_dd6a_295c),
            world_part_colliding: false, skater_parts_colliding: [false; 3],
            model: None, world_body: None, skater_body: None }
    }
    /// Original callback order: world masses/body first, then skater masses/body.
    /// The two material callbacks are the folded noop825187c0, leaving zeroes.
    /// Retain the completed world candidate if skater construction fails.
    pub(crate) fn initialize_bodies(&mut self, model: Option<Arc<CharacterModel>>,
        entity_id: u32, world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), String> {
        if self.model.is_some() || self.world_body.is_some() || self.skater_body.is_some() {
            return Err("pedestrian collision already initialized".into());
        }
        self.model = model;
        let Some(model) = &self.model else { return Ok(()); };
        let CharacterModelKind::Pedestrian { main, arms, world: world_params } = &model.kind else {
            return Err("pedestrian collision received non-pedestrian model".into());
        };
        let definition = model.world_collision.as_ref().ok_or("pedestrian world definition missing")?;
        let material = || AssemblyPartMaterial { static_friction: 0., dynamic_friction: 0., restitution: 0. };
        let assembly = CharacterCollisionAssembly::try_create(definition, world.backend_mut(),
            11, &[world_params.mass], material(), &mut HostSimulationVolumeIntegralNumeric,
            &mut HostSkeletonBodyMassNumeric)
            .map_err(|e| format!("pedestrian world assembly: {e:?}"))?;
        self.world_body = Some(PedestrianCollisionBody::Constructed(CharacterAssemblyCandidate::new(assembly, entity_id)));
        let assembly = CharacterCollisionAssembly::try_create(&model.skater_collision,
            world.backend_mut(), 13, &[main.mass, arms.mass, arms.mass], material(),
            &mut HostSimulationVolumeIntegralNumeric, &mut HostSkeletonBodyMassNumeric)
            .map_err(|e| format!("pedestrian skater assembly: {e:?}"))?;
        self.skater_body = Some(PedestrianCollisionBody::Constructed(CharacterAssemblyCandidate::new(assembly, entity_id)));
        Ok(())
    }

    fn body(&self, kind: PedestrianBodyKind) -> &Option<PedestrianCollisionBody> {
        match kind { PedestrianBodyKind::World => &self.world_body, PedestrianBodyKind::Skater => &self.skater_body }
    }
    fn body_mut(&mut self, kind: PedestrianBodyKind) -> &mut Option<PedestrianCollisionBody> {
        match kind { PedestrianBodyKind::World => &mut self.world_body, PedestrianBodyKind::Skater => &mut self.skater_body }
    }
    pub(crate) fn attached_handle(&self, kind: PedestrianBodyKind) -> Option<&SimulationDmoAssemblyHandle> {
        match self.body(kind) { Some(PedestrianCollisionBody::InWorld(handle)) => Some(handle), _ => None }
    }
    /// Body.GetAssembly -> Part.GetTransform, preserving detached-cache errors.
    pub(crate) fn part_transform(&self, kind: PedestrianBodyKind, part: usize,
        world: &PhysicalAssemblyWorld,
    ) -> Result<[f32; 16], String> {
        match self.body(kind).as_ref().ok_or("pedestrian collision body missing")? {
            PedestrianCollisionBody::Constructed(candidate) => candidate.assembly.parts
                .world_part_transform(world.backend(), part)
                .ok_or_else(|| format!("pedestrian detached part transform index out of range: {part}")),
            PedestrianCollisionBody::InWorld(handle) => world.try_pedestrian_collision_part_transform(handle, part),
        }
    }
    pub(crate) fn skater_part_transform(&self, part: usize, world: &PhysicalAssemblyWorld)
        -> Result<[f32; 16], String> {
        self.part_transform(PedestrianBodyKind::Skater, part, world)
    }
    pub(crate) fn skater_part_velocities(&self, part: usize, world: &PhysicalAssemblyWorld)
        -> Result<([f32; 4], [f32; 4]), String> {
        match self.skater_body.as_ref().ok_or("pedestrian skater collision body missing")? {
            PedestrianCollisionBody::Constructed(candidate) => candidate.assembly.parts
                .try_live_part_velocities_host(world.backend(), part)
                .map_err(|e| format!("pedestrian detached Part velocities: {e:?}")),
            PedestrianCollisionBody::InWorld(handle) => world.try_pedestrian_collision_part_velocities(handle, part),
        }
    }
    fn attribute_flag(&self, key: u64) -> Result<bool, String> {
        let Some(data) = self.attributes.try_get_data(key, 0).map_err(|e| e.to_string())?
            else { return Ok(false); }; // Attrib::DefaultDataArea(1)
        Ok(*data.bytes().first().ok_or("pedestrian collision flag is empty")? != 0)
    }
    /// Pedestrian collision vtable+92/+100 shares82d1a870/82d1a8d0.
    pub(crate) fn do_world_collisions(&self) -> Result<bool, String> {
        self.attribute_flag(0xaf6d_ee43_dcf7_3377)
    }
    pub(crate) fn do_skater_collisions(&self) -> Result<bool, String> {
        self.attribute_flag(0x3bca_c7a1_4c4e_bf5f)
    }
    /// Vtable+96 ->82d1a8c0 reads the first byte of the native cached layout.
    pub(crate) fn do_world_collisions_only_in_hifi(&self) -> Result<bool, String> {
        let data = self.attributes.try_cached_layout_field(0, 1).map_err(|e| e.to_string())?
            .ok_or("pedestrian collision cached layout missing")?;
        Ok(*data.bytes().first().ok_or("pedestrian collision layout flag empty")? != 0)
    }

    /// AddModels82e5cd40: set actual transform first; WORLD before SKATER.
    /// FillWorld82e90e30/FillSkater82e91530 include pose and plugin branches;
    /// callers supply those real producers, never vehicle box transforms.
    /// Producers run even for an already admitted body, before its early exit.
    pub(crate) fn add_models_to_world(
        &mut self, core: &mut CommonEntityCore, world: &mut PhysicalAssemblyWorld,
        allow_world: bool, world_state: u32, skater_state: u32,
        mut fill: impl FnMut(PedestrianBodyKind, &CommonEntityCore, &Self, &PhysicalAssemblyWorld)
            -> Result<Vec<[f32; 16]>, String>,
    ) -> Result<(), String> {
        core.set_transform::<std::convert::Infallible>(core.target_transform)
            .map_err(|e| format!("pedestrian add transform: {e:?}"))?;
        if self.do_world_collisions()? && allow_world {
            let transforms = fill(PedestrianBodyKind::World, core, self, world)?;
            self.add_body_to_world(PedestrianBodyKind::World, &transforms, world_state, world)?;
        }
        if self.do_skater_collisions()? {
            let transforms = fill(PedestrianBodyKind::Skater, core, self, world)?;
            self.add_body_to_world(PedestrianBodyKind::Skater, &transforms, skater_state, world)?;
        }
        Ok(())
    }
    /// Base AddWorld/Skater82e505d8/82e50658 assert nonempty transforms when
    /// a body exists, before Body.AddToWorld's existing-Island early return.
    pub(crate) fn add_body_to_world(&mut self, kind: PedestrianBodyKind,
        transforms: &[[f32; 16]], state: u32, world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), String> {
        let body = self.body_mut(kind);
        if body.is_none() { return Ok(()); }
        if transforms.is_empty() { return Err("pedestrian AddBody native assertion: no transforms".into()); }
        if matches!(body, Some(PedestrianCollisionBody::InWorld(_))) { return Ok(()); }
        let Some(PedestrianCollisionBody::Constructed(candidate)) = body.take() else { unreachable!() };
        match world.try_admit_pedestrian_collision(candidate, transforms, state) {
            Ok(handle) => *body = Some(PedestrianCollisionBody::InWorld(handle)),
            Err(VehicleAdmissionFailure::Placement { error, candidate }) => {
                *body = Some(PedestrianCollisionBody::Constructed(candidate));
                return Err(format!("pedestrian placement: {error:?}"));
            }
            Err(VehicleAdmissionFailure::Insert(failure)) => {
                let failure = *failure;
                *body = Some(PedestrianCollisionBody::Constructed(failure.candidate));
                return Err(format!("pedestrian pool insert: {:?}", failure.error));
            }
            Err(VehicleAdmissionFailure::Island { error, handle }) => {
                *body = Some(PedestrianCollisionBody::InWorld(handle));
                return Err(format!("pedestrian island registration: {error:?}"));
            }
        }
        Ok(())
    }
    /// UpdateCollisionTransforms82e5cf28 always fills and updates WORLD first.
    /// Actual attached Parts use the shared drive loop82e506d8.
    pub(crate) fn update_collision_transforms(&self, core: &CommonEntityCore,
        world: &mut PhysicalAssemblyWorld, dt: f32, cvars: &InputCvarReferences,
        mut fill: impl FnMut(PedestrianBodyKind, &CommonEntityCore, &Self, &PhysicalAssemblyWorld)
            -> Result<Vec<[f32; 16]>, String>,
    ) -> Result<(), String> {
        for kind in [PedestrianBodyKind::World, PedestrianBodyKind::Skater] {
            let targets = fill(kind, core, self, world)?;
            if targets.is_empty() { return Err("pedestrian UpdateBody native assertion: no transforms".into()); }
            if let Some(handle) = self.attached_handle(kind) {
                world.try_drive_living_world_collision_body(LivingWorldBodyKind::Pedestrian,
                    handle, &targets, dt, self.ignore_gravity, cvars,
                    &mut HostReckoningNumeric, &mut HostReckoningSinCosNumeric)
                    .map_err(|e| format!("pedestrian body drive: {e:?}"))?;
            }
        }
        Ok(())
    }
    pub(crate) fn remove_body_from_world(&mut self, kind: PedestrianBodyKind,
        world: &mut PhysicalAssemblyWorld,
    ) -> Result<(), String> {
        let body = self.body_mut(kind);
        if let Some(PedestrianCollisionBody::InWorld(handle)) = body {
            let candidate = world.try_remove_pedestrian_collision(handle)
                .map_err(|e| format!("pedestrian body remove: {e:?}"))?;
            *body = Some(PedestrianCollisionBody::Constructed(candidate));
        }
        Ok(())
    }
    pub(crate) fn remove_models_from_world(&mut self, world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        self.remove_body_from_world(PedestrianBodyKind::World, world)?;
        self.remove_body_from_world(PedestrianBodyKind::Skater, world)
    }
    /// Base destructor82e57220 removes WORLD then SKATER, destroys both,
    /// then releases the model. Retain partial cleanup state on errors.
    pub(crate) fn destroy_bodies(&mut self, world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        self.remove_models_from_world(world)?;
        for kind in [PedestrianBodyKind::World, PedestrianBodyKind::Skater] {
            let body = self.body_mut(kind);
            if let Some(PedestrianCollisionBody::Constructed(candidate)) = body {
                if candidate.assembly.parts.is_attached() {
                    candidate.try_detach(world.backend_mut()).map_err(|e| format!("pedestrian failed candidate detach: {e:?}"))?;
                }
            }
            *body = None;
        }
        self.model = None;
        Ok(())
    }
    /// LWCollideableObject::ResetCollisionFlags82e4c6c8 does not clear the
    /// separate persistent Colliding/recovery/required flags.
    pub(crate) fn reset_collision_flags(&mut self) {
        self.world_part_colliding = false;
        self.skater_parts_colliding.fill(false);
        self.colliding_with_skater = false;
    }
    /// Is/SetWorldPart82e72138/188 and Is/SetSkaterPart82e721e0/2248.
    pub(crate) fn set_part_colliding(&mut self, kind: PedestrianBodyKind,
        part: usize, colliding: bool,
    ) -> Result<(), String> {
        match kind {
            PedestrianBodyKind::World => {
                if part >= 1 { return Err("pedestrian world part outside0..1".into()); }
                self.world_part_colliding = colliding;
            }
            PedestrianBodyKind::Skater => {
                *self.skater_parts_colliding.get_mut(part).ok_or("pedestrian skater part outside0..3")? = colliding;
            }
        }
        Ok(())
    }
}
