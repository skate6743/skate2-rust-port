//! Normal CameraMan82DFEAA8/82DFEC90/82DFEE80. The host publishes the real
//! physics/animation subject, evaluates the stock graph between prepare and
//! update, and supplies the three native kinds of collision query.
use super::manager_blends::BlendEnvironment;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagerSettings {
    pub rig: RigSettings,
    pub orientation: OrientationSettings,
    pub framing: FrameSettings,
    pub drop: DropSettings,
    pub look: LookSettings,
    pub shake: ShakeSettings,
    pub steering_threshold: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraMan {
    pub state: ManagerState,
    pub rig: Rig,
    pub shots: ShotManager,
    pub orientation: RigOrientation,
    pub composer: FrameComposer,
    pub drop: DropPredictor,
    pub look: LookInput,
    pub shake: [ShakeEffect; 2],
    pub frame: CameraFrame,
    // Rig transform0..32 is the previous subject orientation; the camera's
    // tracked orientation lives separately in RigOrientation (native64..96).
    subject_forward: [f32; 4],
    heading_multiplier: f32,
}

impl CameraMan {
    pub fn new() -> Self {
        Self {
            state: ManagerState::new(),
            rig: Rig::new(1),
            shots: ShotManager::new(),
            orientation: RigOrientation::new(),
            composer: FrameComposer::new(),
            drop: DropPredictor::new(),
            look: LookInput::new(),
            shake: [ShakeEffect::new(); 2],
            frame: CameraFrame::new(),
            subject_forward: [0.0, 0.0, 1.0, 0.0],
            heading_multiplier: 1.0,
        }
    }

    /// Once per published subject, before the camera graph reads its conditions.
    /// Landing response consumes the previous velocity/normal before incline
    /// publication replaces them; swapping these operations changes impacts.
    pub fn prepare(&mut self, subject: &ManagerSubject, settings: ManagerSettings) {
        self.state
            .update_timing(subject, settings.steering_threshold);
        if let Some((normal, velocity)) = self.state.update_air(subject, &mut self.rig) {
            self.shake[0].landing_impulse(normal, velocity, settings.shake);
        }
        self.state.update_incline(subject);
    }

    pub fn set_shot(
        &mut self,
        name: &str,
        force: bool,
        subject: &ManagerSubject,
        database: &impl ShotDatabase,
    ) -> Result<bool, String> {
        let mut environment = BlendEnvironment {
            state: &self.state,
            subject,
            rig: &self.rig,
            rig_forward: self.subject_forward,
            angular_velocity: [0.0; 4],
            heading_multiplier: self.heading_multiplier,
            previous_distance: self.shots.previous.distance,
        };
        self.shots.set_shot(
            name,
            force,
            database,
            &mut environment,
            ShotPlacement {
                anchor: self.rig.fields.anchor,
                camera_position: self.rig.positioner.position,
                clamp_height: self.rig.positioner.config.clamp_height,
                floor_height: self.rig.positioner.config.floor_height,
            },
        )
    }

    pub fn update(
        &mut self,
        dt: f32,
        subject: &mut ManagerSubject,
        settings: ManagerSettings,
        samples: [&ShakeSamples; 2],
        requests: &mut [impl TrajectoryCollisionRequest; 3],
        moving: &mut impl MovingObstacleProvider,
        collision: &mut (impl PositionerCollisionProvider + DropCollisionProvider),
    ) -> Result<CameraFrame, String> {
        if self.shots.current().name.is_empty() {
            return Err("The stock camera graph has not selected a shot".into());
        }
        self.state.frames = self.state.frames.wrapping_add(1);
        if self.state.frames > 30 {
            self.state.flags |= 1;
        }
        self.state
            .update_mirrors(dt, subject, &self.rig, &self.shots);
        let reset = self.state.reset_requested();
        if self.state.options & 0x40 != 0 {
            if reset {
                self.drop.reset();
            }
            let velocity = DropPredictor::prediction_velocity(
                reset,
                subject.rig.transform,
                self.rig.fields.velocity,
                settings.drop,
            );
            self.drop.update(
                subject.rig.transform[3],
                velocity,
                self.shots.current().shot.use_drop_predictor != 0 && !reset,
                subject.rig.off_board != 0,
                subject.rig.context,
                settings.drop,
                collision,
            )?;
        }
        let look_enabled = if reset {
            self.shots.first_leaf().use_free_camera_stick
        } else {
            self.shots.current().shot.use_free_camera_stick
        } != 0;
        self.rig.fields.flags_517 =
            (self.rig.fields.flags_517 & !0x10) | (u8::from(!look_enabled) << 4);
        self.look.update(
            if look_enabled { subject.look } else { [0.0; 2] },
            settings.look,
        );

        if reset {
            self.shots.make_instant();
            self.rig.fields.velocity = [0.0; 4];
            self.rig.fields.acceleration = [0.0; 4];
            self.state.slowmo_frames = 0;
            self.shake = [ShakeEffect::new(); 2];
            self.heading_multiplier = self.state.heading_mirror.position;
            self.orientation.framing_mirror = self.state.framing_mirror.position;
        } else {
            self.state.slowmo_frames = if subject.rig.broken_bone_slowmo != 0 {
                self.state.slowmo_frames.wrapping_add(1)
            } else {
                0
            };
            if self.state.slowmo_frames == 1 {
                let impulse = settings.shake.impulse_magnitude * 20.0;
                if impulse > self.shake[0].impulse {
                    self.shake[0].impulse = impulse;
                }
            }
        }
        self.shots.update(
            dt,
            &mut BlendEnvironment {
                state: &self.state,
                subject,
                rig: &self.rig,
                rig_forward: self.subject_forward,
                angular_velocity: [0.0; 4],
                heading_multiplier: self.heading_multiplier,
                previous_distance: self.shots.previous.distance,
            },
        );
        self.bind_anchor(subject, reset);
        self.heading_multiplier = self.state.bind_shot(
            self.shots.interpolated,
            subject,
            &mut self.rig,
            &mut self.orientation,
            &self.drop,
            self.look,
            self.composer.roll,
            self.shots.is_transitioning(),
        );
        if reset {
            self.rig.teleport();
            self.frame.shake_translation = [0.0; 4];
        }
        self.rig.update(
            dt,
            settings.rig,
            &mut subject.rig,
            requests,
            moving,
            collision,
        );
        self.state.opacity = self.shots.interpolated.subject_opacity;
        self.state.field_of_view = super::manager_frame::lens_field_of_view(
            self.shots.interpolated.lens_length,
            self.state.aspect_ratio,
        );
        self.orientation.target = self.composer.compose(
            dt,
            self.shots.framing_fraction(),
            self.orientation.framing_mirror,
            self.shots.previous,
            self.shots.current().shot,
            self.rig.anchor.position,
            self.rig.positioner.position,
            FrameSubject {
                positions: subject.rig.reference_positions,
                board_offset_direction: subject.board_offset_direction,
            },
            settings.framing,
        );
        self.orientation.update(
            dt,
            settings.orientation,
            self.rig.fields.reset_time,
            &mut self.rig.fields.flags_516,
            self.rig.fields.flags_517,
        );
        self.finish_frame(dt, subject, settings.shake, samples, reset);
        self.shots.instant_changes = reset || self.state.instant_frames > 0;
        if reset {
            self.state.options &= !0x80;
            self.state.flags |= 2;
        } else if subject.is_ground_camera(self.rig.fields.height_mode) {
            self.state.flags &= !2;
        }
        if self.state.instant_frames > 0 {
            self.state.instant_frames -= 1;
        }
        Ok(self.frame)
    }

    fn bind_anchor(&mut self, subject: &ManagerSubject, reset: bool) {
        let anchor = if reset {
            self.shots.interpolated.anchor
        } else {
            self.shots.current().shot.anchor
        };
        if !reset && anchor != self.state.selected_anchor {
            self.rig.fields.flags_517 |= 0x40;
        }
        self.state.selected_anchor = anchor;
        let selected = subject.anchors[anchor as usize];
        self.state.anchor = selected.position;
        self.state.anchor_velocity = if reset { [0.0; 4] } else { selected.velocity };
        // Native setter subtracts velocities without division by the timestep.
        self.rig.fields.acceleration =
            core::array::from_fn(|i| self.state.anchor_velocity[i] - self.rig.fields.velocity[i]);
        self.rig.fields.velocity = self.state.anchor_velocity;
        self.rig.fields.anchor = self.state.anchor;
        self.subject_forward = subject.rig.transform[2];
        self.rig.fields.flags_516 |= 0x10;
        self.composer.reference = if reset {
            self.state.anchor
        } else {
            self.rig.anchor.position
        };
    }
}
