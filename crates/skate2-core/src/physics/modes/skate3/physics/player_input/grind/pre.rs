use super::*;
use skate3_core::physics::grind_contact::{admission::Admission, investigator};

impl GrindInputState {
    ///82D8A828: permission -> bounded static query -> materials -> investigator
    ///-> directed tangent -> previous velocity -> friction -> geometry probes.
    /// Main may run other input producers between this and post_update.
    pub fn pre_update(
        &mut self,
        p: &ProcessedPhysicsInput,
        provider: &StaticProvider,
        world: &BoardWorld,
        context: PreContext,
        host: &mut impl Host,
    ) -> Result<Pending, String> {
        self.previous_direction = float(self.investigation.direction_1136);
        self.investigation = GrindInvestigationFields::default();
        self.permission(p, context.air_counter);
        let indices = if p.flags_2476 & 0x0100_0000 != 0 {
            Vec::new()
        } else {
            provider.query(
                core::array::from_fn(|i| context.board[3][i] - 1.2),
                core::array::from_fn(|i| context.board[3][i] + 1.2),
            )?
        };
        let mode = self.material_mode(p, !indices.is_empty(), context.air_targeting_grind_9653);
        host.apply_material_mode(mode)?;
        self.advance_history(p);
        let edges = indices
            .iter()
            .map(|&i| provider.primitives()[i])
            .collect::<Vec<_>>();
        let result = investigator::investigate(
            &investigator::Query {
                board: context.board,
                admission: Admission {
                    category: p.category_2512,
                    state: p.state_2508,
                    speed: p.scalar_2652,
                    velocity: float(p.vectors_400_416[0]),
                    threshold_vs_slope: &self.settings.slope_threshold,
                },
                flags_2468: p.flags_2468,
                flags_2472: p.flags_2472,
                flags_2476: p.flags_2476,
                flags_2484: p.flags_2484,
                tip_state: context.tip_state,
                truck_to_wheel: self.settings.truck_to_wheel,
                deck_to_truck: self.settings.deck_to_truck,
                test_above: self.settings.test_above,
                test_below: self.settings.test_below,
                translation: context.translation_2796,
                stability_nudge: context.stability_nudge_2800,
                balance: context.balance_2720,
                reference_right: float(p.vectors_464_480_496_512_528[0]),
                ground_frames: self.grounded_frames,
                low_wheel_frames: self.low_wheel_frames,
                forbidden: self.disabled
                    || (self.grind_history != 0 && p.grind_words_2532_2536[1] != 2),
            },
            &edges,
        );
        let mut fields = GrindInvestigationFields::default();
        let mut metadata = None;
        let mut geometry = None;
        if let Some(contact) = result.candidate {
            let g = contact.geometry;
            let edge = edges[g.primitive];
            let native = provider
                .metadata(indices[g.primitive])
                .ok_or("Selected static grind primitive lacks metadata")?;
            metadata = Some(PrimitiveMetadata {
                spline_guids: Some(native.spline_guids),
            });
            fields.valid_1488 = true;
            fields.family_1248 = contact.kind;
            fields.entry_kind_1252 = contact.entry_kind as u32;
            fields.tangent_1104 = raw(g.direction);
            fields.point_1120 = raw(g.centre);
            fields.direction_1136 = raw(manager::directed_tangent(
                edge.start,
                edge.end,
                float(p.vectors_400_416[0]),
                self.previous_direction,
            ));
            fields.primitive_start_1264 = raw(edge.start);
            fields.primitive_end_1280 = raw(edge.end);
            fields.owner_1296 = Some(edge.owner);
            fields.primitive_flags_1300 = native.flags & 0x8000_0000;
            fields.flags_1516 = if contact.front { 0x2000_0000 } else { 0 };
            //82D8939C/A0 writes both truck hits. FiveO82D89CB8/9D5C
            //writes the selected hit to+96 regardless of front/rear selection.
            if contact.kind == 0 {
                fields.front_contact_1200 = raw(g.front);
                fields.rear_contact_1216 = raw(g.rear);
            } else if contact.kind == 3 {
                fields.front_contact_1200 = raw(g.centre);
            }
            let input = grind_surface::InvestigationInput {
                start: edge.start,
                end: edge.end,
                reference: g.centre,
                optional_probe: (p.state_2508 == 402).then_some(context.board[3]),
                deck_center_to_truck: self.settings.deck_to_truck,
            };
            let plan = grind_surface::prepare(input);
            geometry = Some(GeometryWork {
                input,
                plan,
                hits: [None; 7],
            });
            //Accepted family returns before investigator's history-byte write.
        } else {
            self.previous_proximity = result.proximity.is_some();
            if let Some(proximity) = result.proximity {
                let edge = edges[proximity.contact.primitive];
                let native = provider
                    .metadata(indices[proximity.contact.primitive])
                    .ok_or("Static grind proximity lacks metadata")?;
                //82D87E18/34/3C: contact+128 and SECOND primitive+208/+224.
                fields.vector_1232 = raw(proximity.contact.position);
                fields.second_start_1312 = raw(edge.start);
                fields.second_end_1328 = raw(edge.end);
                fields.second_owner_1344 = Some(edge.owner);
                fields.second_flags_1348 = native.flags & 0x8000_0000;
                fields.flags_1516 = 0x0800_0000
                    | if proximity.deck_contact {
                        0x0400_0000
                    } else {
                        0
                    };
            }
        }
        self.previous_velocity = p.vectors_400_416[0];
        self.friction_vs_time = self.settings.friction.evaluate(self.elapsed);
        fields.friction_1496 = self.friction_vs_time;
        self.investigation = fields;
        //82D8AA54..82D8AAF8 publishes previous velocity and friction before
        //submitting geometry descriptors. Post resolves these exact results.
        if let Some(work) = &mut geometry {
            if let Some(plan) = &work.plan {
                for (index, &probe) in plan.probes.iter().enumerate() {
                    work.hits[index] = host.surface_probe(
                        world,
                        [p.actor_query_2948, p.actor_query_2952],
                        index,
                        probe,
                    )?;
                }
            }
        }
        Ok(Pending {
            fields,
            geometry,
            metadata,
        })
    }
}
