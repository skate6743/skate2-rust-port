use super::*;
impl MotionHost {
    pub(super) fn execute(&mut self, behavior: BehaviorId, frame: &Frame, phase: u8) -> Result<(), String> {
        let id = *self.remap.behaviors.get(behavior).ok_or("Unbound MotionGraph behavior")?;
        let operation = self.operations[id].clone();
        if let MotionOperation::Grind(operation) = operation {
            if phase == 2 || (phase == 0 && matches!(operation,
                crate::skate3_native::graph_host::motion_grind::Operation::Attributes)) {
                return Ok(());
            }
            let mut physical = self.grind_physical
                .ok_or("Grind graph requires completed physical grind observations")?;
            if (phase == 0 && matches!(operation,
                crate::skate3_native::graph_host::motion_grind::Operation::Fade { .. }))
                || (phase == 1 && physical.grinding && matches!(operation,
                    crate::skate3_native::graph_host::motion_grind::Operation::Attributes)) {
                physical.animation_mirrored = self.animation.skater_animation_flags
                    .ok_or("Grind graph requires live animation stance")?
                    & 0x4000_0000 != 0;
            }
            let Instance::Grind(state) = self.instances.get_mut(behavior)
                .ok_or("Unallocated grind behavior")? else {
                return Err("Grind operation/instance mismatch".into());
            };
            return crate::skate3_native::graph_host::motion_grind::execute(
                state, &operation, phase, frame.dt, &self.grind_settings,
                &physical, &mut self.animation,
            );
        }
        if let MotionOperation::Trick(operation) = operation {
            let instance = std::mem::replace(&mut self.instances[behavior], Instance::Stateless);
            let Instance::Trick(mut updates) = instance else {
                return Err("Trick operation/instance mismatch".into());
            };
            let result = crate::skate3_native::graph_host::motion_tricks::execute(self, operation, &mut updates, phase);
            self.instances[behavior] = Instance::Trick(updates);
            return result;
        }
        if matches!(operation, MotionOperation::ToggleBoard) {
            return self.execute_toggle_board(behavior, phase);
        }
        if let MotionOperation::Runout(_) = operation {
            // The native producer samples a physical runout bundle at Begin.
            // This host does not yet publish that bundle, so retain the
            // source-backed state without inventing angle/speed values.
            let Instance::Runout(state) = self.instances.get_mut(behavior)
                .ok_or("Unallocated runout behavior")? else {
                return Err("Runout operation/instance mismatch".into());
            };
            if phase == 0 {
                state.begin(self.runout_physical);
                return Ok(());
            }
            return state.update(Some(&mut self.animation)).map_err(str::to_owned);
        }
        if self.execute_score_or_bump(&operation, phase)? {
            return Ok(());
        }
        if let MotionOperation::Slide(operation) = operation {
            return self.execute_slide(behavior, operation, frame, phase);
        }
        if matches!(
            operation,
            MotionOperation::ClearTrickAttr
                | MotionOperation::AirLeg(_)
                | MotionOperation::BodySpin
        ) {
            return self.execute_air(behavior, operation, frame, phase);
        }
        if let MotionOperation::TwistLean(operation) = operation {
            return self.execute_twist_lean(behavior, operation, phase);
        }
        if let MotionOperation::Wipeout(operation) = operation {
            return self.execute_wipeout(behavior, operation, phase);
        }
        if let MotionOperation::IntentFilter(operation) = operation {
            return self.execute_intent_filter(behavior, operation, frame, phase);
        }
        if let MotionOperation::ResetAnimation(operation) = operation {
            if phase == 0 {
                match operation {
                    crate::skate3_native::graph_host::motion_reset::Operation::SkaterAnimation => {
                        self.action_intents.clear();
                        self.action_controls = Default::default();
                        self.animation.motion_intents.clear();
                        self.animation.filtered_intents.clear();
                        //825953B0's represented selective MG output reset.
                        self.flags = Default::default();
                        self.is_power_sliding = false;
                        self.riding.last_good_landing_velocity = 0.0;
                        self.riding.manual_out_timer = 0.0;
                        if self.hand_services.busy_hands != [0; 2] && std::env::var_os("SKATE2_TRACE_SKATE3_HANDS").is_some() {
                            eprintln!("skate3 hands: SkaterAnimation reset clears busy {:?}", self.hand_services.busy_hands);
                        }
                        self.hand_services.busy_hands = [0; 2];
                        self.gesture_publication = None;
                        self.animation.reset_from_stock();
                    }
                    crate::skate3_native::graph_host::motion_reset::Operation::GivenStance => {
                        self.animation.reset_to_given_stance()?;
                    }
                }
                //Later PlayAnimation nodes in this same traversal see the reset.
                self.playback_context.is_mirrored = self.animation.skater_animation_flags.map(|f| f & 0x4000_0000 != 0);
                self.playback_context.is_switch = Some(self.animation.relative_stance == 1);
            }
            return Ok(());
        }
        if let MotionOperation::HandService(operation) = operation {
            self.hand_services
                .execute(operation, phase, &mut self.animation);
            return Ok(());
        }
        if let MotionOperation::Landing(operation) = &operation {
            let Instance::Landing(state) = self
                .instances
                .get_mut(behavior)
                .ok_or("Unallocated landing behavior")?
            else {
                return Err("Landing operation/instance mismatch".into());
            };
            //StoreLandingData can update C54 earlier in this same graph traversal.
            let physical = self.landing_physical.map(|mut p| {
                p.last_good_landing_velocity = self.riding.last_good_landing_velocity;
                p
            });
            return super::super::motion_landing_execute::execute(
                operation,
                state,
                &mut self.flags,
                &mut self.animation,
                &self.condition_random,
                physical,
                self.playback_context.is_mirrored,
                frame.dt,
                phase,
            );
        }
        return leaf_execution::execute(self, behavior, operation, frame, phase);
    }
}
