//! Stateless offboard grab-scene execution over explicitly authored objects.
//! Ground owns pending/completion, validation and selection; this owns no ticks.
mod collision;
pub(crate) use native::{Descriptor, Hit, Line, Object, Query, Record};
use skate3_core::{physics::board_world::BoardWorld, player::offboard::grab_scene as native};

///Explicit collision mesh -> physical assembly relation, not identity guessing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MeshAssembly {
    pub mesh: u32,
    pub assembly: u32,
}

/// Immutable host geometry for the admitted tick. Hit identities are actual
/// physical assembly occurrences supplied by the host, never mesh numbers.
pub(crate) trait HostLines: Send + Sync {
    fn line(&self, line: Line) -> Result<Option<Hit>, String>;
}
pub(crate) struct Registry {
    pub objects: native::Registry,
    mesh_assemblies: Vec<MeshAssembly>,
    pub(crate) host_lines: Option<std::sync::Arc<dyn HostLines>>,
}
impl Registry {
    pub(crate) fn new(
        world: &BoardWorld,
        objects: Vec<Object>,
        mesh_assemblies: Vec<MeshAssembly>,
    ) -> Result<Self, String> {
        let metadata = world.query_metadata().map_err(str::to_owned)?;
        let objects = native::Registry::new(objects).map_err(str::to_owned)?;
        for (index, binding) in mesh_assemblies.iter().enumerate() {
            if binding.assembly == 0 || !metadata.meshes.iter().any(|m| m.geometry == binding.mesh)
            {
                return Err("Grab collision binding must name an existing mesh and nonzero physical assembly".into());
            }
            if mesh_assemblies[..index]
                .iter()
                .any(|prior| prior.mesh == binding.mesh)
            {
                return Err("One canonical collision mesh cannot name two assemblies".into());
            }
        }
        Ok(Self {
            objects,
            mesh_assemblies,
            host_lines: None,
        })
    }
    fn assembly(&self, mesh: u32) -> Option<u32> {
        self.mesh_assemblies
            .iter()
            .find(|b| b.mesh == mesh)
            .map(|b| b.assembly)
    }
}
pub(crate) struct Scene<'a> {
    world: &'a BoardWorld,
    registry: &'a Registry,
}
impl<'a> Scene<'a> {
    pub(crate) fn new(world: &'a BoardWorld, registry: &'a Registry) -> Self {
        Self { world, registry }
    }
    pub(crate) fn query(&self, query: &Query) -> Result<Vec<Record>, String> {
        let records=native::query(&self.registry.objects, query).map_err(str::to_owned)?;
        if !records.is_empty() && crate::diagnostics::test_log::enabled() {
            crate::diagnostics::test_log::event("skate3_grab_candidates",format_args!(
                "records={} descriptors={:?}",records.len(),records.iter().map(|r|(r.descriptor(),r.assembly())).collect::<Vec<_>>()));
        }
        Ok(records)
    }
    ///8275F8D8 dispatches ONLY descriptor kinds1/2. Resolve the actual authored
    ///spline geometry and CURRENT physical transform, not an old query result.
    pub(crate) fn resolve(&self, descriptor: Descriptor) -> Result<Option<Record>, String> {
        if !matches!(descriptor.kind, 1 | 2) {
            return Ok(None);
        }
        for object in &self.registry.objects.objects {
            if let Some(spline) = object.splines.iter().find(|s| s.descriptor == descriptor) {
                return object.record(spline).map(Some).map_err(str::to_owned);
            }
        }
        Ok(None)
    }
    pub(crate) fn line(&self, line: Line) -> Result<Option<Hit>, String> {
        collision::line(self, line)
    }
    pub(crate) fn eligible_object(&self, hit: Hit) -> Option<u32> {
        self.registry.objects.eligible_object(hit)
    }
}
