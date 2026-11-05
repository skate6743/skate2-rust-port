//! Imported grab/tweak and moving-object registry behavior dispatch.
use super::*;
pub(super) fn execute(host: &mut MotionHost, operation: super::super::motion_stock_gameplay::Operation, phase: u8) -> Result<(), String> {
    match operation {
                    crate::skate3_native::graph_host::motion_stock_gameplay::Operation::InitMovingObjects { path } => {
                        if phase == 0 {
                            host.moving_objects.register(path);
                        }
                    }
                    crate::skate3_native::graph_host::motion_stock_gameplay::Operation::MovingObject { path } => {
                        match phase {
                            0 => host.moving_objects.begin(&path)?,
                            1 => {
                                let active = host.moving_objects.update(&path)?;
                                if !active {
                                    return Err(format!("MovingObject is not active: {path}"));
                                }
                            }
                            _ => host.moving_objects.end(),
                        }
                    }
                    crate::skate3_native::graph_host::motion_stock_gameplay::Operation::SetGrabType { grab } => {
                        let grab = crate::skate3_native::graph_host::motion_stock_gameplay::GrabType::parse(&grab)?;
                        if phase == 0 {
                            host.animation.set_grab_type(grab);
                        } else if phase == 2 {
                            host.animation.clear_grab_type();
                        }
                    }
                    crate::skate3_native::graph_host::motion_stock_gameplay::Operation::ScoringGrabs {
                        grab_name, up, left, down, right, intent_x, intent_y, invert_y, polar,
                    } => {
                        if phase == 1 {
                            // TU3 82BBEF60 publishes a score name and vector.
                            // It never starts animations or changes channel influence.
                            let first = host.animation.filtered_intent(&intent_x).unwrap_or(0.0);
                            let second = host.animation.filtered_intent(&intent_y).unwrap_or(0.0);
                            let (x, y) = if polar {
                                let x = second * first.sin();
                                let y = second * first.cos();
                                (x, if invert_y { -y } else { y })
                            } else {
                                (first, second)
                            };
                            let selected = crate::skate3_native::graph_host::motion_stock_gameplay::select_grab_score(
                                &grab_name, [up.as_deref(), left.as_deref(), down.as_deref(), right.as_deref()], x, y,
                            );
                            host.score_packet.grab = Some((encode(selected.as_bytes()), [x, y]));
                        }
                    }
                    crate::skate3_native::graph_host::motion_stock_gameplay::Operation::TweakProject {
                        intent_x, intent_y, attribute_x, attribute_y,
                    } => {
                        if phase == 1 {
                            // TU3 82BABFD0/82BABFFC read the same slot+8 map
                            // written by FilterMotionGraphIntent at 82BB19D0.
                            // This is filtered output, NOT raw AG-produced MG
                            // input. Bypassing it removes authored tweak timing.
                            let source_x = host.animation.filtered_intent(&intent_x);
                            let source_y = host.animation.filtered_intent(&intent_y);
                            if source_x.is_none() && source_y.is_none() {
                                return Ok(());
                            }
                            let mut x = source_x.unwrap_or(0.0);
                            let mut y = source_y.unwrap_or(0.0);
                            let total = x.abs() + y.abs();
                            if total > 1.0 {
                                let scale = 1.0 / total;
                                x *= scale;
                                y *= scale;
                            }
                            host.animation.set_attribute(SettableAttribute {
                                name: attribute_x,
                                value: x,
                                normalized: false,
                                sequence_id: -1,
                            });
                            host.animation.set_attribute(SettableAttribute {
                                name: attribute_y,
                                value: y,
                                normalized: false,
                                sequence_id: -1,
                            });
                        }
                    }
    }
    Ok(())
}
