//! Native Skate2 authored grab providers and immutable admitted-tick mesh lines.
//! Spline IDs remain the actual LivingWorld IDs. Assembly IDs retain the
//! allocation handle occurrence, independent of entity, mesh and body slots.
use super::super::offboard::grab_scene::{HostLines, Registry};
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::source_parts::SourcePartGrabSnapshot,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::dmo_models::DmoGrabSpline,
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_dmo_assembly_part_session::dynamic_pool::{
        DmoMeshSlot, SimulationDmoAssemblyHandle,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use skate3_core::{
    physics::board_world::BoardWorld,
    player::offboard::grab_scene::{
        AssemblyData, Descriptor, Geometry, Hit, Line, Object, Provider, Spline,
        record::{PartData, RatesData},
    },
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct HostGrabs {
    next_assembly: u32,
    next_geometry: u32,
    assemblies: Vec<(SimulationDmoAssemblyHandle, u32)>,
    geometries: Vec<(DmoGrabSpline, Arc<Geometry>)>,
    last_snapshot_stats: Option<[usize; 4]>,
}
struct MeshLines {
    dmos: Vec<DmoMeshSlot>,
    vehicles: Vec<DmoMeshSlot>,
    assemblies: Vec<(SimulationDmoAssemblyHandle, u32)>,
    island_flags: u32,
}
impl HostLines for MeshLines {
    fn line(&self, line: Line) -> Result<Option<Hit>, String> {
        let mut nearest = None;
        let input = CollisionLineInput {
            start: SkeletonRootVector::known(line.start),
            end: SkeletonRootVector::known(line.end),
            fatness: line.radius,
        };
        for (bit, slots) in [(2, &self.dmos), (4, &self.vehicles)] {
            if line.source_pool_mask & bit == 0 || (bit == 4 && self.island_flags != 3) {
                continue;
            }
            // This is the same published mesh set and native ordered kernel
            // used by the host World, with no Rapier approximation or body ID cast.
            let hit = DmoMeshSlot::try_snapshot_line(slots, input, &mut HostCollisionLineNumeric)
                .map_err(|e| format!("source grab native mesh line: {e:?}"))?;
            if hit.hit_count == 0 {
                continue;
            }
            let assembly = hit.owner.as_ref().and_then(|h| {
                self.assemblies
                    .iter()
                    .find(|(a, _)| a == h)
                    .map(|(_, id)| *id)
            });
            let hit = Hit {
                fraction: hit.line_parameter,
                assembly,
            };
            if nearest.is_none_or(|old: Hit| hit.fraction < old.fraction) {
                nearest = Some(hit);
            }
        }
        Ok(nearest)
    }
}
impl HostGrabs {
    fn assembly_id(&mut self, handle: &SimulationDmoAssemblyHandle) -> Result<u32, String> {
        if let Some((_, id)) = self.assemblies.iter().find(|(h, _)| h == handle) {
            return Ok(*id);
        }
        self.next_assembly = self
            .next_assembly
            .checked_add(1)
            .ok_or("source grab assembly IDs exhausted")?;
        self.assemblies.push((handle.clone(), self.next_assembly));
        Ok(self.next_assembly)
    }
    fn spline(&mut self, id: u32, kind: u32, source: &DmoGrabSpline) -> Result<Spline, String> {
        let geometry = if let Some((_, g)) = self.geometries.iter().find(|(s, _)| {
            s.header_offset == source.header_offset
                && s.entry_index == source.entry_index
                && Arc::ptr_eq(&s.data, &source.data)
        }) {
            Arc::clone(g)
        } else {
            self.next_geometry = self
                .next_geometry
                .checked_add(1)
                .ok_or("source grab geometry IDs exhausted")?;
            let data = source.grab();
            let g = Arc::new(Geometry {
                id: self.next_geometry,
                points: data.spline_points.clone(),
                approach_vectors: data.approach_vectors.clone(),
                word_60: data.flags,
            });
            self.geometries.push((source.clone(), Arc::clone(&g)));
            g
        };
        // Word272 is not consumed by the source runtime. Preserve the record
        // constructor's zero; no equivalence to an unknown S2 field is claimed.
        Ok(Spline {
            descriptor: Descriptor { kind, id },
            geometry,
            word_272: 0,
        })
    }
    fn assembly(
        &mut self,
        handle: &SimulationDmoAssemblyHandle,
        part: Option<SourcePartGrabSnapshot>,
    ) -> Result<AssemblyData, String> {
        let identity = self.assembly_id(handle)?;
        let first_part = part
            .map(|p| {
                let i = p.inertia;
                let rates = match p.body {
                    Some(_) => {
                        let t = p
                            .body_transform
                            .ok_or("source grab attached body has no transform")?;
                        Some(RatesData {
                            identity,
                            vector_48: p
                                .velocity
                                .ok_or("source grab attached body has no velocity")?,
                            transform_position_48: [t[12], t[13], t[14], t[15]],
                        })
                    }
                    None => None,
                };
                // Actual Part inertia words0..36: packed inverse tensor including W,
                // inverse mass, spherical, velocity/omega limits and two drag values.
                Ok::<_, String>(PartData {
                    identity,
                    rates,
                    coefficients_0_to_36: [
                        i.inverse_tensor[0],
                        i.inverse_tensor[1],
                        i.inverse_tensor[2],
                        p.inverse_tensor_w,
                        i.inverse_mass,
                        i.spherical,
                        i.max_velocity,
                        i.max_omega,
                        i.linear_drag,
                        i.angular_drag,
                    ],
                })
            })
            .transpose()?;
        Ok(AssemblyData {
            identity,
            first_part,
        })
    }
    pub(super) fn sync(
        &mut self,
        world: &PhysicalAssemblyWorld,
        board_world: &BoardWorld,
        cvars: Option<&InputCvarReferences>,
    ) -> Result<Registry, String> {
        let (dmos, vehicles, island_flags) = world.skate3_grab_meshes();
        for mesh in dmos.iter().chain(&vehicles) {
            self.assembly_id(mesh.owner())?;
        }
        let mut objects = Vec::new();
        let feed_dmos = cvars
            .map(|c| c.try_feed_dmo_grab_splines())
            .transpose()
            .map_err(|e| format!("source grab DMO cvar: {e:?}"))?
            .unwrap_or(false);
        let feed_vehicles = cvars
            .map(|c| {
                c.try_use_authored_vehicle_grab_splines()
                    .and_then(|authored| {
                        if authored {
                            c.try_feed_vehicle_grab_splines()
                        } else {
                            Ok(false)
                        }
                    })
            })
            .transpose()
            .map_err(|e| format!("source grab vehicle cvar: {e:?}"))?
            .unwrap_or(false);
        let living = world
            .grind_services
            .living_world()
            .lock()
            .map_err(|_| "source grab LivingWorld lock")?;
        if let Some(manager) = living.dmo_objects.as_deref() {
            for id in manager.ids() {
                let object = manager
                    .object(id)
                    .ok_or("source grab DMO map changed during locked traversal")?;
                let part = world
                    .try_living_world_grab_part(&object.body)
                    .map_err(|e| format!("source grab DMO Part: {e:?}"))?;
                let velocity = part.and_then(|p| p.velocity).unwrap_or(object.velocity);
                let assembly = self.assembly(&object.body, part)?;
                let mut splines = Vec::new();
                if feed_dmos && !object.is_static() {
                    for (&id, s) in &object.grab_splines {
                        if s.grab().spline_points.len() > 1 && !s.grab().approach_vectors.is_empty()
                        {
                            splines.push(self.spline(id, 2, s)?);
                        }
                    }
                }
                objects.push(Object {
                    id,
                    provider: Provider::LivingWorld,
                    disabled: object.is_static(),
                    assembly_ready: true,
                    assembly: Some(assembly),
                    frame: std::array::from_fn(|c| {
                        std::array::from_fn(|r| object.target_transform[c * 4 + r])
                    }),
                    object_vector_128: velocity,
                    splines,
                });
            }
        }
        if let Some(manager) = living.grind_vehicle_entities.as_deref() {
            for id in manager.ids() {
                let object = manager
                    .get(id)
                    .ok_or("source grab vehicle map changed during locked traversal")?;
                let Some(handle) = object
                    .collision
                    .as_ref()
                    .and_then(|c| c.attached_assembly_handle())
                else {
                    continue;
                };
                let part = world
                    .try_vehicle_grab_part(handle)
                    .map_err(|e| format!("source grab vehicle Part: {e:?}"))?;
                let assembly = self.assembly(handle, part)?;
                let mut splines = Vec::new();
                if feed_vehicles {
                    for (&id, s) in &object.grab_splines {
                        if !s.grab().spline_points.is_empty()
                            && !s.grab().approach_vectors.is_empty()
                        {
                            splines.push(self.spline(id, 1, s)?);
                        }
                    }
                }
                objects.push(Object {
                    id,
                    provider: Provider::LivingWorld,
                    disabled: false,
                    assembly_ready: true,
                    assembly: Some(assembly),
                    frame: object.core.target_transform,
                    object_vector_128: object.core.velocity,
                    splines,
                });
            }
        }
        let stats = [
            objects.len(),
            objects.iter().map(|o| o.splines.len()).sum(),
            dmos.len(),
            vehicles.len(),
        ];
        if self.last_snapshot_stats != Some(stats) {
            crate::diagnostics::test_log::event(
                "skate3_grab_scene",
                format_args!(
                    "objects={} authored_splines={} native_dmo_meshes={} native_vehicle_meshes={}",
                    stats[0], stats[1], stats[2], stats[3]
                ),
            );
            self.last_snapshot_stats = Some(stats);
            // SKATE2_TRACE_SKATE3_GRABS: each enabled object's grab handles, their world midpoints
            // and approach directions.
            if std::env::var_os("SKATE2_TRACE_SKATE3_GRABS").is_some() {
                for object in objects.iter().filter(|object| !object.disabled) {
                    let f = object.frame;
                    let world = |p: [f32; 4], w: f32| -> [f32; 3] {
                        std::array::from_fn(|i| p[0] * f[0][i] + p[1] * f[1][i] + p[2] * f[2][i] + w * f[3][i])
                    };
                    for spline in &object.splines {
                        let points = &spline.geometry.points;
                        let (Some(first), Some(last)) = (points.first(), points.last()) else { continue };
                        let (a, b) = (world(*first, 1.0), world(*last, 1.0));
                        let middle: [f32; 3] = std::array::from_fn(|i| (a[i] + b[i]) * 0.5);
                        let approach = spline.geometry.approach_vectors.first().map(|v| world(*v, 0.0));
                        eprintln!("skate3 grab handle: object {} spline {} kind {} middle {middle:?} approach {approach:?}",
                            object.id, spline.descriptor.id, spline.descriptor.kind);
                    }
                }
            }
        }
        let mut registry = Registry::new(board_world, objects, Vec::new())?;
        registry.host_lines = Some(Arc::new(MeshLines {
            dmos,
            vehicles,
            assemblies: self.assemblies.clone(),
            island_flags,
        }));
        Ok(registry)
    }
}

#[cfg(test)]
impl HostGrabs {
    pub(super) fn test_audit(
        &self,
        world: &PhysicalAssemblyWorld,
        board_world: &BoardWorld,
        registry: &Registry,
    ) -> Result<(usize, usize, usize), String> {
        let living = world
            .grind_services
            .living_world()
            .lock()
            .map_err(|_| "grab audit LivingWorld lock")?;
        let manager = living
            .dmo_objects
            .as_deref()
            .ok_or("grab audit DMO manager missing")?;
        let (meshes, _, _) = world.skate3_grab_meshes();
        let scene = super::super::offboard::grab_scene::Scene::new(board_world, registry);
        let mut records = 0;
        let mut hits = 0;
        for object in &registry.objects.objects {
            let Some(host) = manager.object(object.id) else {
                continue;
            };
            let assembly = self
                .assemblies
                .iter()
                .find(|(h, _)| h == &host.body)
                .ok_or("grab audit assembly missing")?
                .1;
            assert_eq!(object.assembly.as_ref().unwrap().identity, assembly);
            // The query uses its admitted pre-solve snapshot. At this audit,
            // native world integration/reactions have already updated targets.
            for spline in &object.splines {
                let original = host
                    .grab_splines
                    .get(&spline.descriptor.id)
                    .ok_or("grab audit lost native spline ID")?
                    .grab();
                assert_eq!(spline.geometry.points, original.spline_points);
                assert_eq!(spline.geometry.approach_vectors, original.approach_vectors);
                let record = scene
                    .resolve(spline.descriptor)?
                    .ok_or("source grab resolver lost real native spline")?;
                assert_eq!(record.assembly(), Some(assembly));
                assert_eq!(record.descriptor().kind, 2);
                let point = crate::physical_player_grab_spline::geometry::transform_point;
                assert_eq!(
                    record.endpoints(),
                    [
                        point(&object.frame, original.spline_points[0]),
                        point(&object.frame, *original.spline_points.last().unwrap())
                    ]
                );
                records += 1;
            }
            if object.disabled {
                continue;
            }
            'mesh: for mesh in meshes.iter().filter(|m| m.owner() == &host.body) {
                for (start, end) in mesh.skate3_probe_lines()? {
                    let line = Line {
                        start,
                        end,
                        radius: 0.,
                        group: -1,
                        reject_flags: 0,
                        source_pool_mask: 2,
                        selection_flags: 0,
                    };
                    if let Some(hit) = scene.line(line)? {
                        if hit.assembly == Some(assembly) {
                            assert_eq!(scene.eligible_object(hit), Some(object.id));
                            assert_eq!(manager.owner_of(mesh.owner()), object.id);
                            hits += 1;
                            break 'mesh;
                        }
                    }
                }
            }
        }
        Ok((registry.objects.objects.len(), records, hits))
    }
}
