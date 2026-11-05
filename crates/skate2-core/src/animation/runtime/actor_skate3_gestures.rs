//! PC gesture catalog and movement-base admission on the retained Actor.
use super::ActorConstruction;
use crate::skate3_gesture_playback::{Context,Playback};
use skate2_formats::{andale::AndaleSqt,skater_profile::{GestureSet,PcGestureSettings}};
use skate3_core::animation::output::Sqt;
use crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;

fn buffer(values:&[Sqt])->Result<OwnedAnimationSqtBuffer,String> {
    let buffer=OwnedAnimationSqtBuffer::allocate_unknown(values.len());
    buffer.extract(&values.iter().map(|sqt|AndaleSqt {
        scale:sqt.scale,rotation:sqt.rotation,translation:sqt.translation,
    }).collect::<Vec<_>>()).map_err(|e|format!("gesture SQT extraction: {e:?}"))?;
    buffer.complete_sync().map_err(|e|format!("gesture SQT synchronization: {e:?}"))?;
    Ok(buffer)
}

impl ActorConstruction {
    #[cfg(test)]
    pub(crate) fn test_gesture_active(&self)->bool {
        let owner=self.checked_body_animation().unwrap();let animation=owner.lock().unwrap();
        if animation.pc_gesture_preferences.selected==GestureSet::Skate3 {
            animation.pc_gesture_playback.as_ref().is_some_and(Playback::active)
        }else{
            let init=animation.initialization().unwrap();
            let channels=init.channels.as_ref().unwrap();
            [b"GestureBoth".as_slice(),b"GestureLeft",b"GestureRight"].iter().filter_map(|name|
                channels.dictionary.get(&crate::actor_graph_construction::ActorGraphIntents::native_name(name)))
                .any(|index|channels.channels[*index].blend_coefficient.is_some_and(|weight|weight>0.))
        }
    }

    pub(crate) fn try_set_pc_gesture_preferences(&self,prefs:PcGestureSettings)->Result<(),String> {
        prefs.validate().map_err(|e|e.to_string())?;
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let mut animation=owner.lock().map_err(|_|"gesture animation lock")?;
        if animation.pc_gesture_preferences!=prefs {
            animation.pc_gesture_playback=None;
            animation.pc_gesture_preferences=prefs;
            crate::diagnostics::test_log::event("gesture_catalog_apply",format_args!(
                "set={:?} skate3_up_down_left_right={:?}",prefs.selected,prefs.skate3));
        }
        Ok(())
    }

    /// The Edit Skater Skate 3 gesture selections (Up, Down, Left, Right) while the Skate 3
    /// gesture set is chosen.
    pub(crate) fn try_skate3_gesture_selections(&self)->Result<Option<[u32;4]>,String> {
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let prefs=owner.lock().map_err(|_|"gesture animation lock")?.pc_gesture_preferences;
        Ok((prefs.selected==GestureSet::Skate3).then_some(prefs.skate3))
    }

    /// Skate 3 physics plays Skate 3 gestures in its own channel stack: a gesture the overlay
    /// still holds from Skate 2 physics ends at once.
    pub(crate) fn try_end_pc_gesture(&self)->Result<(),String> {
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let mut animation=owner.lock().map_err(|_|"gesture animation lock")?;
        if let Some(playback)=animation.pc_gesture_playback.as_mut() && playback.native_fact().1 {
            playback.reset();
        }
        Ok(())
    }

    /// The gesture fact Skate 2's MotionGraph publishes (`8258FB30`).
    pub(crate) fn try_publish_gesture_fact(&self,gesture:u32,gesturing:bool)->Result<(),String> {
        let mut prefix=self.prefix().map_err(|e|e.to_string())?;
        let state=prefix.graphs.as_mut().and_then(|graphs|graphs.motion_mut())
            .and_then(|motion|motion.state.as_mut()).ok_or("gesture MotionGraph publication")?;
        state.gesture=gesture;
        state.is_gesturing=gesturing;
        Ok(())
    }

    pub(crate) fn try_prepare_pc_gestures(&self,raw:Option<crate::input_pad::XboxPadSample>,
        source:Option<Context>)->Result<(),String> {
        let policy=self.try_skate3_actor_input().map_err(|e|e.to_string())?;
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let prefs=owner.lock().map_err(|_|"gesture animation lock")?.pc_gesture_preferences;
        if prefs.selected!=GestureSet::Skate3 {return Ok(());}
        let context=if let Some(context)=source {context}else {
            let physical=self.cached_physical_allocation().map_err(|e|e.to_string())?;
            let output=physical.owner().phys_out().lock().map_err(|_|"gesture physical output lock")?;
            let filtered=output.try_filtered_output().map_err(|e|format!("gesture filtered state: {e:?}"))?;
            let ground=output.try_ground_output().map_err(|e|format!("gesture ground: {e:?}"))?;
            let offboard=output.try_offboard_output().map_err(|e|format!("gesture offboard: {e:?}"))?;
            let state=output.try_state_output().map_err(|e|format!("gesture state: {e:?}"))?;
            let pose=output.try_animation_output().map_err(|e|format!("gesture animation output: {e:?}"))?;
            let observed=(filtered.filtered_physics_state as u32,ground.is_hippy_jumping,
                offboard.is_holding_skateboard,state.is_offboard,pose.centre_of_mass_to_deck);
            drop(output);
            let prefix=self.prefix().map_err(|e|e.to_string())?;
            let motion=prefix.graphs().and_then(|graphs|graphs.motion()).ok_or("gesture MotionGraph")?;
            let busy=motion.state.as_ref().ok_or("gesture MotionGraph state")?.grab_busy;
            let animation=owner.lock().map_err(|_|"gesture animation lock")?;
            let player=animation.player().ok_or("gesture SkaterAnim player")?;
            let channels=&animation.initialization().ok_or("gesture initialized animation")?.channels;
            let blocked=channels.as_ref().is_some_and(|channels|[b"RetrieveBoard".as_slice(),b"Shove",b"WipeoutPushOff"]
                .iter().any(|name|channels.dictionary.contains_key(&crate::actor_graph_construction::ActorGraphIntents::native_name(name))));
            Context {busy_hands:[u32::from(busy[1]!=0),u32::from(busy[0]!=0)],filtered:observed.0,
                hippy:observed.1,holding_board:observed.2,
                offboard:observed.3,distance_to_cog:observed.4,blocked:blocked || observed.0==crate::physics::FilteredPhysicsState::Wipeout as u32,
                skater_animation_flags:player.flags_bits}
        };
        let mut animation=owner.lock().map_err(|_|"gesture animation lock")?;
        if animation.pc_gesture_playback.is_none() {animation.pc_gesture_playback=Some(Playback::new()?);}
        let init=animation.initialization().ok_or("gesture initialized hierarchy")?;
        let hierarchy=std::sync::Arc::clone(init.hierarchy.as_ref().ok_or("gesture hierarchy")?);
        let guard=hierarchy.lock().map_err(|_|"gesture hierarchy lock")?;
        let hierarchy=&guard.parsed.as_ref().ok_or("gesture hierarchy content")?.hierarchy;
        let runtime=animation.pc_gesture_playback.as_ref().unwrap();
        validate_hierarchy(&hierarchy.bone_names,&hierarchy.parents,runtime.names(),runtime.parents())?;
        drop(guard);
        let playback=animation.pc_gesture_playback.as_mut().unwrap();
        playback.prepare(raw,prefs.skate3,context,policy)?;
        let (gesture,gesturing)=playback.native_fact();
        // Match native Prefix->Animation lock ordering; never reacquire Prefix
        // while the animation owner is locked.
        drop(animation);
        self.try_publish_gesture_fact(gesture,gesturing)
    }

    pub(crate) fn try_set_mode_body_pose(&self,names:&[String],parents:&[i32],pose:&[Sqt])->Result<(),String> {
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let mut animation=owner.lock().map_err(|_|"mode body pose lock")?;
        let init=animation.initialization().ok_or("mode initialized hierarchy")?;
        let hierarchy=std::sync::Arc::clone(init.hierarchy.as_ref().ok_or("mode hierarchy")?);
        let guard=hierarchy.lock().map_err(|_|"mode hierarchy lock")?;
        let hierarchy=&guard.parsed.as_ref().ok_or("mode hierarchy content")?.hierarchy;
        validate_hierarchy(&hierarchy.bone_names,&hierarchy.parents,names,parents)?;
        if names.len()!=pose.len(){return Err("Mode body pose size differs from its hierarchy".into());}
        drop(guard);
        animation.mode_body_pose=Some(buffer(pose)?);
        Ok(())
    }

    pub(crate) fn try_set_mode_body_commands(&self,commands:Vec<skate3_core::animation::playback_tree::PoseCommand>,foot_forward_right:bool)->Result<(),String> {
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let mut animation=owner.lock().map_err(|_|"mode body commands lock")?;
        // Skate 3 physics poses the fingers from the S3 clips' hand streams whatever the
        // gesture set: the playback owns those streams and the appended S3 hand palettes.
        if animation.pc_gesture_playback.is_none() {animation.pc_gesture_playback=Some(Playback::new()?);}
        animation.mode_body_commands=Some(commands);
        animation.mode_foot_forward_right=Some(foot_forward_right);
        Ok(())
    }

    pub(crate) fn try_composed_body_pose(&self)->Result<(Vec<[[f32;4];4]>,Vec<[[f32;4];4]>),String> {
        let owner=self.checked_body_animation().map_err(|e|e.to_string())?;
        let animation=owner.lock().map_err(|_|"composed body pose lock")?;
        let init=animation.initialization().ok_or("composed initialized animation")?;
        let convert=|matrices:Vec<[f32;16]>|matrices.into_iter().map(|m|std::array::from_fn(|column|
            std::array::from_fn(|lane|m[column*4+lane]))).collect();
        let local=init.local_matrices.matrices().map_err(|e|format!("composed locals: {e:?}"))?
            .ok_or("composed body local matrices")?;
        let global=init.global_matrices.matrices().map_err(|e|format!("composed globals: {e:?}"))?
            .ok_or("composed body global matrices")?;
        Ok((convert(local),convert(global)))
    }
}

fn validate_hierarchy(a:&[String],ap:&[i32],b:&[String],bp:&[i32])->Result<(),String> {
    if a.len()!=b.len()||ap!=bp||!a.iter().zip(b).all(|(a,b)|a.eq_ignore_ascii_case(b)) {
        return Err("Gesture/movement bank differs from the actual Skate 2 Actor hierarchy".into());
    }
    Ok(())
}
