//! Authored S3 gesture finger/expression cues on the retained S2 hands/face ACS.
//! Body tree sampling and native clip key/token arithmetic share the same time.
use crate::{actor_face_playback::{FaceCommands,blend_pose_stream::{self,BlendPoseBuffer,BlendPoseTokenPool,ClipStream}},
    actor_graph_construction::ActorGraphIntents};
use skate2_formats::andale::{AndaleDatabase,AndaleSqt};
use skate3_core::animation::playback_tree::PoseCommand;
use std::{collections::BTreeMap,sync::{Arc,Mutex,OnceLock}};

const NATIVE_HAND_FRAMES:usize=61;
struct HandPoses {names:Vec<String>,parents:Vec<i32>,frames:Vec<Vec<AndaleSqt>>}
pub(super) struct GestureStreams {
    clips:BTreeMap<String,Vec<ClipStream>>,
    /// Every other S3 clip's LeftHand/RightHand cues, for the body tree's fingers.
    body:BTreeMap<String,Vec<ClipStream>>,
    lengths:BTreeMap<String,f32>,
    hands:Arc<[HandPoses;2]>,
    hand_names:[crate::play_animation::FastString36;2],
}
impl GestureStreams {
    pub(super) fn load(source:&crate::skate3_native::skater_animation::AnimationSource)->Result<Self,String> {
        let mut clips=BTreeMap::new();
        let mut body=BTreeMap::new();
        let mut lengths=BTreeMap::new();
        let hand_names=[b"LeftHand".as_slice(),b"RightHand"].map(ActorGraphIntents::native_name);
        for bank in &source.banks().banks {
            for record in bank.records() {
                if record.header.type_id!=2 {continue;}
                let gesture=record.header.name.contains("_GSTR_");
                let end=record.header.offset+record.header.size;
                let mut streams=blend_pose_stream::decode_clip_streams(&bank.bytes()[record.header.payload_offset..end])
                    .map_err(|e|format!("{} hand/face streams: {e}",record.header.name))?;
                for stream in &mut streams {
                    if hand_names.contains(&stream.name) {
                        for (_,pose) in &mut stream.keys {
                            if *pose>=62 {return Err(format!("{} hand pose exceeds the authored62-frame palette",record.header.name));}
                            // Existing S2 fixed poses0..60 keep their original identity.
                            // Appended S3 poses61..122 fit the native7-bit token lane.
                            *pose+=NATIVE_HAND_FRAMES as u16;
                        }
                    }
                }
                if let skate_data::abin::RecordData::Clip(clip)=&record.data {
                    // Native clip clock length is (frames-1)/fps.
                    lengths.insert(record.header.name.clone(),(f32::from_bits(clip.frame_count_bits)-1.)/f32::from_bits(clip.fps_bits));
                }
                if gesture {
                    // Skate 3 physics plays gestures in the body channel stack, whose tree supplies
                    // their finger and face cues as the overlay's channels do.
                    body.insert(record.header.name.clone(),streams.clone());
                    clips.insert(record.header.name.clone(),streams);
                } else {
                    // The face keeps its S2 owner: S3 face cues index S3's own face palette.
                    if record.header.name.starts_with("NB_STAND") && std::env::var_os("SKATE2_TRACE_SKATE3_HANDS").is_some() {
                        let names:Vec<String>=streams.iter().map(|stream|stream.name.decode().unwrap_or_default()).collect();
                        eprintln!("skate3 hand clip {}: streams {}", record.header.name, names.join(" "));
                    }
                    streams.retain(|stream|hand_names.contains(&stream.name));
                    body.insert(record.header.name.clone(),streams);
                }
            }
        }
        static HANDS:OnceLock<Arc<[HandPoses;2]>>=OnceLock::new();
        let hands=crate::skate3_native::physics::player::linked_once(&HANDS, || {
            let root=crate::skate3_native::physics::player::assets_root()?;
            let mut result=Vec::new();
            for name in ["SkaterLeftHand","SkaterRightHand"] {
                let bytes=skate_data::asset_io::read(&root.join(format!("private/stock/data/anim/{name}.abin"))).map_err(|e|e.to_string())?;
                let db=AndaleDatabase::parse(&bytes).map_err(|e|e.to_string())?;
                let clip=db.clips.iter().find(|c|c.name.eq_ignore_ascii_case("Poses")).ok_or("S3 hand Poses clip")?;
                if clip.frame_count!=62. {return Err("S3 hand palette must contain its authored62 frames".into());}
                let frames=(0..62).map(|frame|clip.sample_raw_frame_sqt(&db.hierarchy,frame).map_err(|e|e.to_string()))
                    .collect::<Result<Vec<_>,_>>()?;
                result.push(HandPoses {names:db.hierarchy.bone_names.clone(),parents:db.hierarchy.parents.clone(),frames});
            }
            Ok(Arc::new(result.try_into().map_err(|_|"S3 hand pair")?))
        })?.clone();
        if std::env::var_os("SKATE2_TRACE_SKATE3_HANDS").is_some() {
            for (name,streams) in body.iter().filter(|(name,_)|name.starts_with("NB_STAND_0")) {
                for stream in streams {
                    eprintln!("skate3 hand clip {name}: length {:?} {} keys {:?}",lengths.get(name),
                        stream.name.decode().unwrap_or_default(),stream.keys);
                }
            }
        }
        Ok(Self {clips,body,lengths,hands,hand_names})
    }
    /// Skate 3's 62 hand poses of hand 0 (left) or 1 (right), appended after Skate 2's 61.
    pub(super) fn hand_palette(&self,hand:usize)->&[Vec<AndaleSqt>] {&self.hands[hand].frames}
    pub(super) fn register(&self,left:&Arc<Mutex<FaceCommands>>,right:&Arc<Mutex<FaceCommands>>)->Result<(),String> {
        for (commands,poses) in [left,right].into_iter().zip(self.hands.iter()) {
            let mut commands=commands.lock().map_err(|_|"gesture hand command lock")?;
            if commands.fixed_frames.len()==NATIVE_HAND_FRAMES+poses.frames.len() {continue;}
            if commands.fixed_frames.len()!=NATIVE_HAND_FRAMES || commands.max_frames!=NATIVE_HAND_FRAMES {
                return Err("Gesture hand registration requires the original61 fixed S2 poses".into());
            }
            let db=commands.database.lock().map_err(|_|"gesture hand database lock")?;
            let hierarchy=db.parsed.as_ref().ok_or("gesture hand hierarchy")?;
            if hierarchy.hierarchy.parents!=poses.parents || hierarchy.hierarchy.bone_names.len()!=poses.names.len()
                || !hierarchy.hierarchy.bone_names.iter().zip(&poses.names).all(|(a,b)|a.eq_ignore_ascii_case(b)) {
                return Err("S3 gesture fingers differ from the actual S2 character hand hierarchy".into());
            }
            drop(db);
            commands.fixed_frames.extend(poses.frames.iter().cloned());
            commands.max_frames=commands.fixed_frames.len();
            crate::diagnostics::test_log::event("skate3_gesture_hand_palette",format_args!("native_frames=61 appended_frames=62 same_s2_owner=true"));
        }
        Ok(())
    }
    pub(super) fn apply(&self,channels:&[(Vec<PoseCommand>,f32)],destination:&mut BlendPoseBuffer,pool:&mut BlendPoseTokenPool)->Result<(),String> {
        for (commands,weight) in channels {
            if *weight<=0. {continue;}
            let result=self.evaluate(commands,&self.clips,true,pool)?;
            pool.blend(destination,&result,*weight).map_err(|e|e.to_string())?;
        }
        Ok(())
    }
    /// The S3 body tree's finger cues over the hand streams already in `destination`
    /// (Blend82d78038), as the native body tree's EvalBlendPose supplies them: at `weight` one
    /// they replace them.
    pub(super) fn apply_body(&self,commands:&[PoseCommand],weight:f32,destination:&mut BlendPoseBuffer,pool:&mut BlendPoseTokenPool)->Result<(),String> {
        if commands.is_empty() || weight<=0. {return Ok(());}
        static TRACE: OnceLock<bool> = OnceLock::new();
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_HAND_TOKENS").is_some()) {
            eprintln!("hand stream body commands: {commands:?}");
        }
        let result=self.evaluate(commands,&self.body,false,pool)?;
        if weight>=1. {
            // TU3 has no second skeleton: its hands take only the S3 body tree's cues, and a hand
            // stream no playing clip carries leaves the bind pose. The S2 Actor's own body cues
            // under it must not show through.
            for name in [b"LeftHand".as_slice(),b"RightHand"] {
                destination.clear_stream(&ActorGraphIntents::native_name(name));
            }
        }
        pool.blend(destination,&result,weight).map_err(|e|e.to_string())
    }
    /// EvalBlendPose over one postfix command list: an additive node keeps only its motion
    /// child, a mirror marks its result mirrored, a blend weights the second subtree.
    fn evaluate(&self,commands:&[PoseCommand],streams:&BTreeMap<String,Vec<ClipStream>>,required:bool,pool:&mut BlendPoseTokenPool)->Result<BlendPoseBuffer,String> {
        let mut stack:Vec<BlendPoseBuffer>=Vec::new();
        for command in commands {
            match command {
                PoseCommand::Clip {name,time,..}=>{
                    let mut buffer=pool.buffer(1).map_err(|e|e.to_string())?;
                    match streams.get(name) {
                        Some(clip)=>{
                            // Every authored key has normalized8191 timing. The same
                            // stock clock length used by body playback is retained below.
                            let length=self.clip_length(name)?;
                            // The native pose-0 fallback is S3's pose 0, appended after S2's palette.
                            let fallback=|name:&crate::play_animation::FastString36| if self.hand_names.contains(name) {NATIVE_HAND_FRAMES as u32} else {0};
                            blend_pose_stream::eval_named_clip_streams(clip,*time,length,&mut buffer,pool,fallback).map_err(|e|e.to_string())?;
                        }
                        None if required=>return Err(format!("Missing original gesture clip stream {name}")),
                        None=>{}
                    }
                    stack.push(buffer);
                }
                PoseCommand::Pose {..}=>stack.push(pool.buffer(1).map_err(|e|e.to_string())?),
                // AddSQT::EvalBlendPose 82533738 evaluates only its motion child (+0x14). Every
                // AddBindPose 82B98118 layer's motion is the chain under its static pose, whichever
                // operand slot it fills, so the hand cues pass through and the pose adds none.
                PoseCommand::Add {..}=>{
                    stack.pop().ok_or("Hand stream Add underflow")?;
                    stack.last().ok_or("Hand stream Add base")?;
                }
                PoseCommand::Mirror {..}=>stack.last_mut().ok_or("Hand stream Mirror underflow")?.mirror_streams(),
                PoseCommand::Blend {weight} | PoseCommand::ChannelBlend {weight,..}=>{
                    let other=stack.pop().ok_or("Hand stream Blend underflow")?;
                    let base=stack.last_mut().ok_or("Hand stream Blend base")?;
                    pool.blend(base,&other,*weight).map_err(|e|e.to_string())?;
                }
                PoseCommand::WeightedBlend {weights}=>{
                    let start=stack.len().checked_sub(weights.len()).ok_or("Hand stream weighted underflow")?;
                    let children=stack.split_off(start);
                    stack.push(pool.weighted_blend(&children,weights).map_err(|e|e.to_string())?);
                }
            }
            Self::trace_stack(command,&stack,pool);
        }
        if stack.len()!=1 {return Err(format!("Hand stream stack retained{} results",stack.len()));}
        Ok(stack.pop().unwrap())
    }
    #[cfg(test)]
    fn trace_stack(command:&PoseCommand,stack:&[BlendPoseBuffer],pool:&BlendPoseTokenPool) {
        if std::env::var_os("SKATE2_TRACE_HAND_TOKENS").is_none() {return;}
        let summary:Vec<String>=stack.iter().map(|buffer| {
            (0..3).filter_map(|i| {
                let name=["LeftHand","RightHand"].into_iter().find(|n| buffer.stream_index(&ActorGraphIntents::native_name(n.as_bytes()))==Some(i))?;
                let tokens=pool.tokens(buffer,i).ok()?;
                Some(format!("{name}:{:08x?}",tokens))
            }).collect::<Vec<_>>().join(" ")
        }).collect();
        eprintln!("{command:?} -> [{}]",summary.join(" | "));
    }
    #[cfg(not(test))]
    fn trace_stack(_:&PoseCommand,_:&[BlendPoseBuffer],_:&BlendPoseTokenPool) {}
    fn clip_length(&self,name:&str)->Result<f32,String> {
        self.lengths.get(name).copied().ok_or_else(||format!("Missing gesture length {name}"))
    }
}
