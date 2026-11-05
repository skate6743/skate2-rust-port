//! Obstruction request82D79948, moving-contact Sync82D79B40 and79E30.
use super::trajectory::{frames, shift};
use super::*;
use crate::player::wipeout_state::math::{add, dot, madd, reciprocal, scale, sub};

impl Manager {
    ///82D79948 numeric request. Host submits with processed filter2952.
    ///Native radius=.3, duration=time, start/end error=.5; no fake world hit.
    ///Pending is NOT changed here: native sets it after the actual dispatch.
    pub(super) fn prepare_query(&mut self, p: &Input, position: Vector, time: f32) -> QueryRequest {
        let target = add(
            madd(p.board_velocity_400, time, p.board_position_112),
            [0., f32::from_bits(0x3e4c_ccce), 0., 0.],
        );
        self.blocked_258 = false;
        self.obstruction_height_240 = target[1] + -0.1;
        QueryRequest {
            trajectory: Trajectory {
                position,
                velocity: sub(
                    scale(sub(target, position), reciprocal(time)),
                    scale(scale(GRAVITY, 0.5), time),
                ),
                acceleration: GRAVITY,
                duration: time,
            },
            radius: 0.3,
            start_error: 0.5,
            end_error: 0.5,
        }
    }

    ///Call only after the returned request has actually been submitted.
    ///On host submission failure retain/retry that request or propagate failure;
    ///do not acknowledge it, fabricate a miss, or continue as if it succeeded.
    pub fn query_submitted(&mut self) {
        self.pending_262 = true;
    }

    ///Consume one COMPLETED real query. None (or an explicit invalid result)
    ///means no collision; it never means "query not ready". Caller gates pending.
    ///The callback is the real79F00 provider and is invoked only by79E30's gate.
    ///Its error leaves manager state unchanged so the host can propagate/retry.
    pub fn sync<E>(
        &mut self,
        hit: Option<QueryResult>,
        input: &SyncInput,
        hippy_velocity: impl FnOnce(&Manager) -> Result<Vector, E>,
    ) -> Result<(), E> {
        let mut next = *self;
        next.sync_completed(hit.filter(|hit| hit.valid()), input, hippy_velocity)?;
        *self = next;
        Ok(())
    }

    fn sync_completed<E>(
        &mut self,
        hit: Option<QueryResult>,
        input: &SyncInput,
        hippy_velocity: impl FnOnce(&Manager) -> Result<Vector, E>,
    ) -> Result<(), E> {
        if let Some(hit) = hit {
            //82D79B9C uses strict less-than: equality is an obstruction.
            if hit.contact_position[1] < self.obstruction_height_240 {
                self.blocked_258 = false;
                self.tested_259 = true;
            } else if self.tested_259 {
                self.blocked_258 = true;
            } else if self.completed_queries_252 == 0 {
                self.moving_contact_208 = hit.contact_transform[3];
                self.publish_moving_contact_261 = true;
            } else {
                let contact = hit.contact_transform[3]; //native result+112, NOT normal
                let velocity = scale(sub(contact, self.moving_contact_208), 60.);
                if dot(velocity, velocity) > f32::from_bits(0x3efa_e147) {
                    self.hippy_hurdling_260 = true;
                    if self.completed_queries_252 == 1 {
                        if input.flags_2488 & 0x0400_0000 != 0 {
                            //79E30 compares full-precision elapsed velocity BEFORE
                            //quantizing the epoch shift to integer frames.
                            let current = self.trajectory_32.velocity_at(self.elapsed_160);
                            let replacement = hippy_velocity(self)?;
                            if replacement[1] > current[1] {
                                let frame = frames(self.elapsed_160);
                                shift(&mut self.trajectory_32, frame);
                                self.trajectory_32.velocity = replacement;
                                shift(&mut self.trajectory_32, frame.wrapping_neg());
                            }
                        }
                    } else {
                        let landing_time = self.time_to_land_244 + self.elapsed_160;
                        let predicted_contact = madd(velocity, self.time_to_land_244, contact);
                        let mut separation = sub(
                            self.trajectory_32.position_at(landing_time),
                            predicted_contact,
                        );
                        separation[1] = 0.;
                        let square = dot(separation, separation);
                        if self.trajectory_32.velocity_at(self.elapsed_160)[1] < 0. {
                            if input.position_592[1] - hit.contact_position[1] < 0.7 {
                                self.blocked_258 = true;
                                self.tested_259 = true;
                            }
                        } else if square < 1. {
                            self.blocked_258 = true;
                            self.tested_259 = true;
                        } else if square > 6.25 {
                            //Big-endian decompiled sth1 at+258 is bytes00,01.
                            //Saved disassembly79DDC/DE0 proves these two stores.
                            self.blocked_258 = false;
                            self.tested_259 = true;
                        }
                        //Intermediate separation [1,6.25] retains both flags.
                    }
                } else {
                    self.hippy_hurdling_260 = false;
                    self.blocked_258 = true;
                    self.tested_259 = true;
                }
                self.moving_contact_208 = contact;
            }
        } else {
            self.blocked_258 = false;
            self.tested_259 = true;
        }
        self.pending_262 = false;
        self.completed_queries_252 = self.completed_queries_252.wrapping_add(1);
        Ok(())
    }
}
