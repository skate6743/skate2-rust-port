//! Skate 3 CharacterGesture82BAA320 on the existing Skate 2 Actor's ACS channels.
//! Only animation is owned here. The selected game's physical player, root,
//! character recipe, hands/face writers and save owner remain authoritative.
use crate::skate3_native::{
    animation_pose::PoseEvaluator,
    graph_host::{motion::MotionAnimation,
        motion_character_gesture::{CharacterGesture, CharacterGestureInputs}},
};
use skate3_core::{animation::{output::Sqt,playback_tree::{Evaluation,PoseCommand}},
    graph::intents::IntentMap,input::{controller::DerivedControllerInput,gameplay_gestures}};
use std::sync::Arc;
#[path="skate3_gesture_streams.rs"]
mod streams;

pub(crate) struct ChannelPose {
    pub pose: Vec<Sqt>,
    pub influence: f32,
    pub weights_from_channel: bool,
}

#[derive(Clone,Copy)]
pub(crate) struct Context {
    pub busy_hands: [u32;2],
    pub filtered: u32,
    pub hippy: bool,
    pub holding_board: bool,
    pub offboard: bool,
    pub distance_to_cog: f32,
    pub blocked: bool,
    pub skater_animation_flags: u32,
}

pub(crate) struct Playback {
    animation: MotionAnimation,
    character: CharacterGesture,
    evaluator: Arc<PoseEvaluator>,
    previous_buttons: u32,
    publication: Option<(u32,bool)>,
    started: bool,
    streams: streams::GestureStreams,
    stream_channels: Vec<(Vec<PoseCommand>,f32)>,
}

impl ChannelPose {
    /// Same packed ACS coefficient and per-bone W channel as body ChannelBlend.
    pub(crate) fn compose_over(self,base:&mut [skate2_formats::andale::AndaleSqt])->Result<(),String> {
        use crate::actor_animation_construction::initialization::body_dispatch::tree_commands::{encode_coefficient,numeric};
        if base.len()!=self.pose.len(){return Err("Gesture preview hierarchy size differs".into());}
        let coefficient=f32::from(encode_coefficient(self.influence).map_err(|e|format!("gesture coefficient: {e:?}"))?)*f32::from_bits(0x39000400);
        for (a,b) in base.iter_mut().zip(self.pose) {
            let b=skate2_formats::andale::AndaleSqt {scale:b.scale,rotation:b.rotation,translation:b.translation};
            let weighted=coefficient*if self.weights_from_channel {a.translation[3]}else{b.translation[3]};
            let upper=if weighted>1. {1.}else{weighted};
            let weight=if 0.>upper {0.}else{upper};
            *a=numeric::blend(*a,b,weight);
        }
        Ok(())
    }
}
impl Playback {
    pub(crate) fn new()->Result<Self,String> {
        let source=crate::skate3_native::physics::player::stock_animation_source()?;
        let evaluator=source.evaluator();
        let mut animation=MotionAnimation::from_metadata(source.metadata()?);
        animation.set_hierarchy(&evaluator.frames.bone_names,&evaluator.frames.mirror_indices)?;
        animation.posture_bank_valid=true;
        let streams=streams::GestureStreams::load(&source)?;
        Ok(Self {animation,character:CharacterGesture::new(),evaluator,previous_buttons:0,publication:None,started:false,streams,stream_channels:Vec::new()})
    }

    #[cfg(test)]
    pub(crate) fn active(&self)->bool {self.publication.is_some()}
    /// S2 GetPhysUpdateData reports a concrete ID once, then Invalid+active.
    pub(crate) fn native_fact(&self)->(u32,bool) {
        match self.publication {
            Some((gesture,_))=>(if self.started {native_gesture_id(gesture)}else{35},true),
            None=>(35,false),
        }
    }
    pub(crate) fn names(&self)->&[String] {&self.evaluator.frames.bone_names}
    /// Skate 3's hand poses of hand 0 (left) or 1 (right): the hand streams' poses 61 on.
    pub(crate) fn skate3_hand_poses(&self,hand:usize)->&[Vec<skate2_formats::andale::AndaleSqt>] {self.streams.hand_palette(hand)}
    pub(crate) fn parents(&self)->&[i32] {&self.evaluator.frames.parents}

    /// Fill8259BF50 consumes the pad's original D-pad and bumper history.
    /// Other buttons/sticks belong to the ordinary game input owner.
    pub(crate) fn prepare(&mut self,raw:Option<crate::input_pad::XboxPadSample>,
        selections:[u32;4],context:Context,policy:crate::skate3_native::physics::player::actor_input::HostActorInput)
        ->Result<(),String> {
        let buttons=raw.map_or(0,|pad|pad.buttons);
        let current=pad_gesture_bits(buttons);
        let mut words=[0;26];words[6]=self.previous_buttons;words[13]=current;
        self.previous_buttons=current;
        let mut intents=IntentMap::new();
        for intent in gameplay_gestures::produce(&DerivedControllerInput::from_words(words)) {
            intents.insert(intent.name,intent.value);
        }
        policy.apply_map(&mut intents,false);
        // This is the same blocked-channel/mount gate as native82BAB258.
        if context.blocked {intents.insert("OB_Mount",1.);}
        self.animation.skater_animation_flags=Some(context.skater_animation_flags);
        let previous=self.publication;
        let publication=self.character.update(&mut self.animation,CharacterGestureInputs {
            motion_intents:&intents,busy_hands:context.busy_hands,filtered_state:context.filtered,
            ground321:context.hippy,board_held311:context.holding_board,state_offboard75:context.offboard,
            distance_to_cog:context.distance_to_cog,selections:Some(selections),suppress_up:false,
            force_brake_bypass:policy.pinned_gestures,
        })?;
        self.started=publication.is_some_and(|value|value.started);
        self.publication=publication.map(|value|(value.gesture,value.down));
        if self.publication!=previous {
            crate::diagnostics::test_log::event("skate3_gesture_playback",format_args!(
                "previous={previous:?} current={:?} offboard={} blocked={} busy_hands={:?}",
                self.publication,context.offboard,context.blocked,context.busy_hands));
        }
        Ok(())
    }

    /// Actor ResetSkaterAnim ends owned channels while controller history stays
    /// with the input owner. A still-held D-pad is not a fresh press on reset.
    pub(crate) fn reset(&mut self) {
        self.character.end(&mut self.animation);
        self.publication=None;
        self.started=false;
        self.stream_channels.clear();
    }

    /// Native Actor phase3 advances the retained gesture tree once per tick.
    pub(crate) fn advance(&mut self,dt:f32)->Result<(),String> {
        self.animation.advance(dt,1.);
        self.animation.apply_parameters()
    }

    pub(crate) fn apply_hand_face_streams(&self,buffer:&mut crate::actor_face_playback::blend_pose_stream::BlendPoseBuffer,
        pool:&mut crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool)->Result<(),String> {
        self.streams.apply(&self.stream_channels,buffer,pool)
    }
    /// Skate 3 physics: the S3 body tree's own clips pose the fingers.
    pub(crate) fn apply_body_hand_streams(&self,commands:&[PoseCommand],weight:f32,buffer:&mut crate::actor_face_playback::blend_pose_stream::BlendPoseBuffer,
        pool:&mut crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool)->Result<(),String> {
        self.streams.apply_body(commands,weight,buffer,pool)
    }
    pub(crate) fn register_hand_poses(&self,left:&std::sync::Arc<std::sync::Mutex<crate::actor_face_playback::FaceCommands>>,
        right:&std::sync::Arc<std::sync::Mutex<crate::actor_face_playback::FaceCommands>>)->Result<(),String> {
        self.streams.register(left,right)
    }

    /// Each native ChannelBlend ends a complete standalone channel subtree.
    /// The actual Actor's preceding ACS body result is its first operand.
    pub(crate) fn evaluate(&mut self,cull:f32)->Result<Vec<ChannelPose>,String> {
        let mut commands=Vec::new();
        self.animation.channels.evaluate(Evaluation {cull_threshold:cull,update_history:true},&mut commands)?;
        self.stream_channels.clear();
        let mut begin=0;let mut result=Vec::new();
        for (index,command) in commands.iter().enumerate() {
            if let PoseCommand::ChannelBlend {weight,use_channels_from_weights}=command {
                self.stream_channels.push((commands[begin..index].to_vec(),*weight));
                let pose=self.evaluator.evaluate(&commands[begin..index])?;
                result.push(ChannelPose {pose,influence:*weight,weights_from_channel:*use_channels_from_weights});
                begin=index+1;
            }
        }
        if begin!=commands.len(){return Err("Gesture channel command stream has no final ChannelBlend".into());}
        Ok(result)
    }
}

fn pad_gesture_bits(buttons:u16)->u32 {
    [(0,27),(1,26),(2,25),(3,24),(8,28),(9,29)].into_iter()
        .fold(0,|out,(pad,derived)|out|((u32::from(buttons)>>pad&1)<<derived))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xbox_gesture_history_keeps_bumper_suppression_and_direction_order() {
        assert_eq!(pad_gesture_bits(1),1<<27);
        assert_eq!(pad_gesture_bits(2),1<<26);
        assert_eq!(pad_gesture_bits(4),1<<25);
        assert_eq!(pad_gesture_bits(8),1<<24);
        for bumper in [256,512] {
            let mut words=[0;26];words[13]=pad_gesture_bits(1|bumper);
            assert!(gameplay_gestures::produce(&DerivedControllerInput::from_words(words)).is_empty());
        }
    }
}

#[cfg(test)]
mod body_stream_tests {
    use super::*;
    use skate3_core::animation::playback_tree::PoseCommand as C;
    /// The hand/face dispatch's token rules: a blend needs two entries for one coefficient,
    /// or exactly its key count of coefficients.
    fn balanced(tokens:&[u32])->bool {
        let (mut depth,mut coefficients)=(0usize,0usize);
        for &token in tokens {
            match token>>30 {
                0=>depth+=1,
                1=>coefficients+=1,
                _=>{
                    let keys=((token>>20)&7) as usize;
                    if depth>1&&coefficients==1 {if keys!=2 {return false;} depth-=1;}
                    else if coefficients>1 {if coefficients!=keys||depth<keys {return false;} depth=depth+1-keys;}
                    else {return false;}
                    coefficients=0;
                }
            }
        }
        depth>=1
    }
    fn clip(name:&str,time:f32)->C {C::Clip{name:name.into(),previous_time:time,time,loops:0}}
    /// `SKATE3_DATA` names a Skate 3 folder: the switch-riding body tree that once left an
    /// unbalanced hand stream (phantom slot tokens) now evaluates cleanly.
    #[test]
    fn switch_riding_body_tree_keeps_hand_streams_balanced() {
        let Some(root)=std::env::var_os("SKATE3_DATA") else {return};
        let data=crate::skate3::Skate3Data::open(std::path::Path::new(&root)).unwrap();
        crate::skate3::set_linked(Some(std::sync::Arc::new(data)));
        let player=Playback::new().unwrap();
        let (a,b)=(0.733_333_35,0.025_614_379);
        let commands=vec![
            clip("R_SWITCH_RIDE_LEAN_0_FS",a),clip("R_SWITCH_RIDE_N_2_N",a),clip("R_SWITCH_RIDE_N_0_N",a),
            C::Blend{weight:0.877_177},C::Blend{weight:0.838_372},C::Pose{name:"RIG_TPOSE".into()},C::Add{motion_is_a:true},
            clip("R_IDLE_LCOM_N100",b),clip("R_IDLE_LCOM_000",b),C::Blend{weight:0.854_610_5},
            clip("LNG_CRV_POSE_5",0.001_024_575_2),clip("LNG_CRV_POSE_0",0.001_024_575_2),C::Blend{weight:0.416_449_52},
            clip("LNG_CRV_POSE_7",0.001_024_575_2),clip("LNG_CRV_POSE_8",0.001_024_575_2),C::Blend{weight:0.279_712_92},
            C::Blend{weight:0.231_690_05},clip("R_IDLE_HCOM_N100",b),clip("R_IDLE_HCOM_000",b),C::Blend{weight:0.854_610_5},
            C::Blend{weight:0.549_624_1},C::Blend{weight:0.878_290_4},C::Pose{name:"RIG_TPOSE".into()},C::Add{motion_is_a:true},
            C::Mirror{trajectory_mode:2},C::Blend{weight:0.019_675_927},
        ];
        let mut pool=crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool::new();
        pool.reset();
        let mut streams=pool.buffer(0).unwrap();
        player.apply_body_hand_streams(&commands,1.,&mut streams,&mut pool).unwrap();
        for name in [b"LeftHand".as_slice(),b"RightHand"] {
            if let Some(index)=streams.stream_index(&crate::actor_graph_construction::ActorGraphIntents::native_name(name)) {
                let tokens=pool.tokens(&streams,index).unwrap().to_vec();
                assert!(balanced(&tokens),"{} tokens {:08x?}",String::from_utf8_lossy(name),tokens);
            }
        }
    }
}

#[cfg(test)]
mod corpus {
    use super::*;
    #[test]
    #[ignore="requires the executable embedded original Skate 3 movement bank"]
    fn all_original_gesture_choices_play_entry_hold_exit_for_each_available_hand() {
        for id in 0..37 {for busy in [[0,0],[0,1],[1,0]] {
            let mut player=Playback::new().unwrap();
            let context=Context {busy_hands:busy,filtered:0,hippy:false,holding_board:false,offboard:false,
                distance_to_cog:1.,blocked:false,skater_animation_flags:0};
            let mut observed=false;let mut finished=false;
            let mut stream_pool=crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool::new();
            let mut finger_cue=false;
            for step in 0..360 {
                let raw=crate::input_pad::XboxPadSample {buttons:if step<120 {1}else{0},left_trigger:0,right_trigger:0,left_x:0,left_y:0,right_x:0,right_y:0};
                player.prepare(Some(raw),[id;4],context,Default::default()).unwrap();
                player.advance(1./60.).unwrap();
                let poses=player.evaluate(0.).unwrap();
                stream_pool.reset();let mut streams=stream_pool.buffer(0).unwrap();
                player.apply_hand_face_streams(&mut streams,&mut stream_pool).unwrap();
                for name in [b"LeftHand".as_slice(),b"RightHand",b"Face"] {
                    if let Some(index)=streams.stream_index(&crate::actor_graph_construction::ActorGraphIntents::native_name(name)) {
                        for token in stream_pool.tokens(&streams,index).unwrap() {
                            if token>>30==0 {
                                let pose=(token>>23)&127;
                                assert!(pose<if name==b"Face" {61}else{123});
                                finger_cue|=name!=b"Face" && pose>=61;
                            }
                        }
                    }
                }
                observed|=!poses.is_empty();
                finished|=step>300&&poses.is_empty();
                for channel in poses {
                    assert_eq!(channel.pose.len(),36,"gesture={id} busy={busy:?}");
                    assert!(channel.influence.is_finite());
                    assert!(channel.pose.iter().all(|p|p.scale.iter().chain(&p.rotation).chain(&p.translation).all(|v|v.is_finite())),"gesture={id} busy={busy:?} step={step}");
                }
            }
            assert!(finger_cue,"gesture={id} busy={busy:?} no original finger cue");
            assert!(observed,"gesture={id} busy={busy:?} never emitted its actual animation");
            assert!(finished,"gesture={id} busy={busy:?} did not leave after release");
        }}
    }
}

/// Semantic equivalences in the two original CharacterGesture name tables.
/// New S3 gestures use a PC event namespace; their catalog IDs/poses stay S3.
/// Native S2's Invalid=35 must never alias S3 Wings=35.
pub(crate) fn native_gesture_id(gesture:u32)->u32 {
    const SHARED:[Option<u32>;37]=[
        Some(16),Some(17),Some(19),Some(6),None,Some(9),Some(12),Some(24),
        Some(26),Some(27),Some(18),Some(28),Some(5),Some(29),Some(10),None,
        Some(30),None,Some(4),Some(8),None,Some(13),Some(25),Some(32),Some(14),
        Some(7),Some(0),Some(2),None,Some(15),None,None,None,Some(11),Some(3),Some(33),None,
    ];
    SHARED[gesture as usize].unwrap_or(256+gesture)
}
#[cfg(test)]
mod fact_tests {
    use super::*;
    #[test]
    fn catalog_facts_do_not_alias_native_invalid_or_each_other() {
        let ids=(0..37).map(native_gesture_id).collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(),37);
        assert!(!ids.contains(&35));
        assert_eq!(native_gesture_id(35),33);
        assert_eq!(native_gesture_id(36),292);
    }
}

#[cfg(test)]
mod reset_corpus {
    use super::*;
    #[test]
    #[ignore="requires the executable embedded original Skate 3 movement bank"]
    fn actor_animation_reset_releases_gesture_without_retriggering_held_pad() {
        let mut playback=Playback::new().unwrap();
        let context=Context {busy_hands:[0,0],filtered:0,hippy:false,holding_board:false,
            offboard:false,distance_to_cog:1.,blocked:false,skater_animation_flags:0};
        let raw=crate::input_pad::XboxPadSample {buttons:1,left_trigger:0,right_trigger:0,
            left_x:0,left_y:0,right_x:0,right_y:0};
        playback.prepare(Some(raw),[35;4],context,Default::default()).unwrap();
        playback.advance(1./60.).unwrap();assert!(playback.active());
        playback.reset();assert_eq!(playback.native_fact(),(35,false));
        assert!(playback.evaluate(0.).unwrap().is_empty());
        playback.prepare(Some(raw),[35;4],context,Default::default()).unwrap();
        assert!(!playback.active());
        playback.prepare(None,[35;4],context,Default::default()).unwrap();
        playback.prepare(Some(raw),[35;4],context,Default::default()).unwrap();
        assert_eq!(playback.native_fact(),(33,true));
    }
}

#[cfg(test)]
mod attribute_corpus {
    use super::*;
    #[test]
    #[ignore="requires the executable embedded original Skate 3 movement bank"]
    fn original_gesture_channel_attribute_inventory() {
        let decode=|words:[u32;5]| {
            let mut result=String::new();
            for mut word in words {for power in [79235168,2085136,54872,1444,38,1] {
                let digit=word/power;word%=power;
                if digit==0 {return result;}
                result.push(match digit {1..=10=>(b'0'+(digit-1) as u8) as char,
                    11..=36=>(b'A'+(digit-11) as u8) as char,37=>'_',_=>'?'});
            }}
            result
        };
        for id in 0..37 {
            let mut playback=Playback::new().unwrap();
            let context=Context {busy_hands:[0,0],filtered:0,hippy:false,holding_board:false,
                offboard:false,distance_to_cog:1.,blocked:false,skater_animation_flags:0};
            let raw=crate::input_pad::XboxPadSample {buttons:1,left_trigger:0,right_trigger:0,
                left_x:0,left_y:0,right_x:0,right_y:0};
            let mut names=std::collections::BTreeSet::new();
            for step in 0..180 {
                playback.prepare(if step<120 {Some(raw)}else{None},[id;4],context,Default::default()).unwrap();
                playback.advance(1./60.).unwrap();
                for attribute in playback.animation.channels.attributes(Vec::new(),15).unwrap() {
                    names.insert((decode(attribute.name.0),attribute.kind));
                }
                playback.evaluate(0.).unwrap();
            }
            eprintln!("gesture127 id={id} attributes={names:?}");
        }
    }
}
