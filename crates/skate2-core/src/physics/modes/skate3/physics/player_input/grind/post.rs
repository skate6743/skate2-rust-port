use super::*;
use skate3_core::physics::grind_contact::admission::EntryKind;

impl GrindInputState {
    ///82D8AB08's meaningful order. Geometry consumes the pre-submitted hits;
    /// material application has already happened. The complete snapshot is
    /// copied only after balance, engagement, controls and jumper publication.
    pub fn post_update(
        &mut self,
        p: &mut ProcessedPhysicsInput,
        world: &BoardWorld,
        pending: Pending,
        context: PostContext,
        host: &mut impl Host,
    ) -> Result<PostResult, String> {
        let Pending {
            mut fields,
            geometry,
            metadata,
        } = pending;
        let surface = if let Some(work) = geometry {
            let surface = if let Some(plan) = &work.plan {
                grind_surface::resolve(plan, &work.hits, fields.geometry_flags_1476)
            } else {
                //The original inadequate-up gate returns its specific
                //not-submitted result. Reuse the core branch, not a fake miss.
                grind_surface::investigate(work.input, |index, probe| {
                    host.surface_probe(
                        world,
                        [p.actor_query_2948, p.actor_query_2952],
                        index,
                        probe,
                    )
                })?
            };
            publication::surface(&mut fields, &surface);
            let output = manager::tweak_geometry(
                manager::GeometryInput {
                    valid: fields.valid_1488,
                    family: fields.family_1248,
                    current_state: p.state_2508,
                    category: p.category_2512,
                    air_frames: self.air_frames,
                    geometry_kind: surface.kind as u32,
                    geometry_flags: surface.flags,
                    far_points: surface.far_points,
                    upmost: surface.upmost_normal,
                    high_side: surface.high_side,
                    point: float(fields.point_1120),
                    direction: float(fields.direction_1136),
                    board_position: context.board[3],
                    board_forward: context.board[2],
                    velocity: float(p.vectors_400_416[0]),
                    deck_to_truck: self.settings.deck_to_truck,
                    previous_exit_angle: self.balance.exit_angle_degrees,
                    previous_exit_direction: self.balance.exit_direction,
                    flags: fields.flags_1516,
                },
                &mut self.secondary_history,
            );
            fields.valid_1488 = output.valid;
            fields.family_1248 = output.family;
            fields.flags_1516 = output.flags;
            Some(surface)
        } else {
            None
        };

        fields.gravity_relief_1512 = manager::gravity_relief(
            &mut self.gravity_timer,
            fields.valid_1488,
            p.category_2512,
            float(fields.tangent_1104),
            float(p.vectors_400_416[0]),
            p.timestep_2604,
            &self.settings.gravity_vertical,
            &self.settings.gravity_linear,
        );
        let contact = match surface.as_ref().filter(|_| fields.valid_1488) {
            Some(surface) => Some(balance::BalanceContact {
                surface,
                primitive_direction: float(fields.tangent_1104),
                directed_grind_direction: float(fields.direction_1136),
                kind: fields.family_1248,
                entry_kind: entry_kind(fields.entry_kind_1252)?,
            }),
            None => None,
        };
        //Zeroed each pre frame by the original investigation reset. Invalid
        //target-up updates leave these zeros; history lives in BalanceState.
        let mut vectors = balance::BalanceVectors {
            grind_normal: float(fields.normal_1152),
            target_up: float(fields.target_up_1168),
        };
        self.balance.update_target_up(
            balance::TargetUpInput {
                category: p.category_2512,
                previous_state: p.state_2504,
                board_up: float(p.vectors_544_560_592_608[0]),
            },
            contact.as_ref(),
            &mut vectors,
        );
        fields.exit_lean_1500 = self.balance.update_exit_lean(
            balance::ExitLeanInput {
                category: p.category_2512,
                current_state: p.state_2508,
                grind_substate: p.grind_words_2532_2536[1],
                timestep: p.timestep_2604,
            },
            contact.as_ref(),
            &self.settings.exit_lean,
            &mut vectors,
        );
        fields.normal_1152 = raw(vectors.grind_normal);
        fields.target_up_1168 = raw(vectors.target_up);
        self.balance.update_force_exit(
            float(fields.point_1120),
            &mut fields.flags_1516,
            |probe| host.force_exit_line(world, [p.actor_query_2948, p.actor_query_2952], probe),
        )?;

        let engagement = self.engagement.update(entry::Input {
            valid: fields.valid_1488,
            kind: fields.family_1248,
            category: p.category_2512,
            previous_state_2504: p.state_2504,
            speed: p.scalar_2652,
            balance_2720: context.balance_2720,
            direction: float(fields.direction_1136),
            normal: vectors.grind_normal,
            up: float(p.vectors_544_560_592_608[0]),
            board_velocity: float(p.vectors_400_416[0]),
            air_velocity: float(p.vectors_544_560_592_608[3]),
            surface_kind: fields.geometry_kind_1464,
            high_side: float(fields.high_side_1440),
            vertical_help: &self.settings.vertical_help,
            max_delta: self.settings.max_impact,
            flags: fields.flags_1516,
            previous_entry_velocity: float(fields.entry_velocity_1184),
        });
        fields.valid_1488 = engagement.valid;
        fields.flags_1516 = engagement.flags;
        fields.entry_velocity_1184 = raw(engagement.entry_velocity);
        fields.impact_speed_1492 = engagement.impact_speed;
        if !engagement.wipeout_reasons.is_empty() {
            p.flags_2468 |= 0x0004_0000;
        }
        //Control::update's other-family branch is exactly the invalid-candidate
        //zero-target smoothing branch. Do not feed reset family0 as a50-50.
        self.control.update(
            if fields.valid_1488 {
                fields.family_1248
            } else {
                u32::MAX
            },
            context.board[2],
            vectors.grind_normal,
            fields.flags_1516 & 0x2000_0000 != 0,
            p.flags_2468 & 0x0010_0000 != 0,
            fields.flags_1516 & 0x1000_0000 != 0,
            context.translation_2796,
            context.stability_nudge_2800,
            context.up_down_2804,
            context.grab_min_height_2808,
        );
        fields.yaw_1504 = self.control.yaw;
        fields.pitch_1508 = self.control.pitch;
        let jump_geometry = fields.valid_1488.then(|| manager::JumpGeometry {
            geometry_kind: fields.geometry_kind_1464,
            high_side: float(fields.high_side_1440),
            normal: vectors.grind_normal,
            direction: float(fields.direction_1136),
            upmost: float(fields.upmost_normal_1408),
            point: float(fields.point_1120),
        });
        p.flags_2476 |= self
            .jumper
            .update(jump_geometry, p.grind_words_2532_2536[0]);
        self.investigation = fields;
        p.grind = fields;
        let observation = publication::observation(p, context, metadata, &self.jumper)?;
        Ok(PostResult {
            wipeout_reasons: engagement.wipeout_reasons,
            observation,
        })
    }
}

fn entry_kind(value: u32) -> Result<EntryKind, String> {
    //Only reset0 or a typed investigator decision reaches this adapter.
    Ok(match value {
        0 => EntryKind::RideFromAbove,
        1 => EntryKind::RideFromBelow,
        2 => EntryKind::RideIntoCoping,
        3 => EntryKind::StayInGrind,
        4 => EntryKind::ChangeGrind,
        5 => EntryKind::AirToGrind,
        6 => EntryKind::DropIn,
        _ => return Err(format!("Invalid published grind admission kind {value}")),
    })
}
