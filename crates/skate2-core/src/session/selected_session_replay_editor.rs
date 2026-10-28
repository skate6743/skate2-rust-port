//! FEReplay2/ReplayEditorBackEnd lifecycle, beside the simulation recorder.
//! Start82567450, Stop825675d0, SceneLiteTimeController8287d590.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage, SelectedVisualSnapshot,
    SelectedSkaterVisual, career, replay_session::ReplayClock};
use crate::{
    challenge::unlockables::{self, Unlockables},
    edit_skater_profile::EditSkaterProfileStore,
    nis::manager::{NisCamera, FrozenFocus},
    presentation_restore::{PresentationRestorer, camera::CameraRestore},
    ui::replay_editor::{self, Action, Controller, Mode},
};
use skate2_formats::ice_camera::{self, CameraSet, Mover, Spaces, editor::{self, element}};
#[path="replay_editor_collision.rs"]
mod collision;
#[path="replay_editor_archive.rs"]
pub(super) mod archive;

fn error(e:impl std::fmt::Display)->SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual,format!("Replay Editor: {e}"))
}

pub(super) struct ReplayEditorSession {
    export:Option<VideoExport>,
    pub loaded_index:Option<crate::challenge::replay::ReplaySystem>,
    pub loaded_frames:Option<std::collections::BTreeMap<u32,std::sync::Arc<super::replay_session::ReplayFrame>>>,
    pub controller: Controller,
    pub camera: NisCamera,
    pub rate: f32,
    loaded_hulls:Option<crate::challenge::ChallengeSystem>,
    intro_done:bool,
    animation_ready:bool,
    game_camera: CameraRestore,
    shakes: CameraSet,
    mover: Mover,
    spaces: Spaces,
    last_rate_frame: f32,
    exit: u8,
    save: Option<Action>,
    location: String,
    last_stream_focus:[f32;3],
    collision:collision::Collision,
    /// Pushed by the Hall of Meat results, which the exit pops back to.
    hom_results:bool,
    /// `sGameInfo.additionalWorldLocation`, which a save writes: the game's in play, or the
    /// saved replay's.
    world_location:Option<u64>,
    /// The world a saved replay plays in, put in place before its first frame shows
    /// (`StateReplay::LoadAdditionalWorld` 0x827756e0).
    world:Option<super::world_travel::TravelWorld>,
}

/// Temporary presentation ownership during a PC media export. Simulation and
/// the retained recording stay paused; edits are restored even after failure.
struct VideoExport {
    author:editor::Author,
    mover:Mover,
    game_camera:CameraRestore,
    collision:collision::Collision,
    clock:ReplayClock,
    seconds:f64,
    end_seconds:f64,
    frame:Option<u64>,
    cancelled:bool,
    finished:bool,
}

impl<T,C> SelectedLiveSession<T,C> {
    pub fn skate_reel_snapshot(&self)->Option<crate::ui::skate_reel::Snapshot> {
        self.main_menu.as_ref()?.skate_reel.as_ref().map(|r|r.snapshot())
    }
    pub fn take_skate_reel_action(&mut self)->crate::ui::skate_reel::Action {
        self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()).map_or(crate::ui::skate_reel::Action::None,|r|r.take_action())
    }
    pub fn skate_reel_failed(&mut self,message:&str) {
        if let Some(r)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {r.failed(message);}
    }
    pub fn skate_reel_footage_loaded(&mut self,replay:bool,items:Vec<replay_editor::FootageItem>) {
        if let Some(r)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {r.loaded(replay,items);}
    }
    pub fn skate_reel_photo_opened(&mut self,slot:u32) {
        if let Some(r)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {r.photo_opened(slot);}
    }
    pub fn skate_reel_screen_ready(&mut self,intros:[bool;3],footage:bool) {
        use crate::ui::skate_reel::Page;
        if let Some(r)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {
            let ready=match r.page {Page::Online=>intros[0],Page::General=>intros[1],Page::Photo(_)=>intros[2],Page::Footage=>footage,Page::Replay=>false};
            if ready {r.ready=true;}
        }
    }
    /// MyFootage's load confirmation fades before MemcardReplay::RequestLoadFull.
    pub fn queue_replay_file(&mut self,data:Vec<u8>) {
        self.replay_file_pending=Some(data);self.frontend_fade.cover();
    }
    pub(super) fn complete_replay_file_load(&mut self)->Result<(),SelectedLiveError> {
        let data=self.replay_file_pending.take().ok_or_else(||error("saved replay request absent"))?;
        let assets=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?;
        let archive=match archive::decode(&data,&assets.mounts,&assets.vault,&assets.cvars) {
            Ok(archive)=>archive,
            Err(message)=>{self.skate_reel_failed(&message);self.frontend_fade.reveal();return Ok(());}
        };
        self.open_replay_recording(Some(archive),false)?;
        if self.replay_editor.is_some() {
            if let Some(r)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {r.replay_opened();}
        }
        Ok(())
    }
    /// StartHub ProcessSelection8265ea98 pushes state12 after FadeComplete.
    pub(super) fn open_replay_editor(&mut self)->Result<(),SelectedLiveError> {
        self.open_replay_recording(None,false)
    }
    /// FrontEndState_HOMResults::OnMenuSelect826591e8 pushes state12 at once, unfaded.
    pub(super) fn open_hom_results_replay_editor(&mut self)->Result<(),SelectedLiveError> {
        self.open_replay_recording(None,true)
    }
    fn open_replay_recording(&mut self,mut recording:Option<archive::Archive>,hom_results:bool)->Result<(),SelectedLiveError> {
        if self.replay_editor.is_some() {return Ok(());}
        let index=recording.as_ref().map(|a|&a.index).unwrap_or_else(||self.replay_index());
        let (start,end)=(index.oldest_frame_number_written(),index.last_frame_number_written());
        let assets=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?;
        let shakes=editor::parse_shakes(&assets.mounts.read_path("data/scene/ICELite.bin").map_err(error)?).map_err(error)?;
        let db=self.movie_attrib_db()?;
        let store=EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
        let mut unlocks=Unlockables::new(&db);
        unlocks.load(career::leaf_tuples(&store.load_leaf(unlockables::SAVABLE,unlockables::DATA).map_err(error)?));
        let film=unlocks.is_unlocked(unlockables::category::REPLAY,unlockables::string_hash("filmpack"),true);
        if !film&&recording.as_ref().is_some_and(|a|a.film_pack) {
            if let Some(r)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {
                r.localized_notice("ID_DLC_FILMERPACK_REQUIRED");
            }
            self.frontend_fade.reveal();return Ok(());
        }
        // A saved replay's world location (`loadAdditionalWorldLocation`; none: the main world);
        // over the game in play, the game's own.
        let (world_location,world)=match recording.as_ref() {
            Some(a)=>{
                let file=a.world_location.and_then(|key|crate::ui::freeskate_option::location_world(&db,key));
                let world=match (a.world_location,file) {
                    (Some(key),Some(file))=>super::world_travel::TravelWorld::of_location(&file,key),
                    _=>super::world_travel::TravelWorld::Main,
                };
                (a.world_location,Some(world))
            }
            None=>(self.additional_location,None),
        };
        let tuning=assets.cvars.try_replay_editor().map_err(error)?;
        // The PC profile is the offline signed-in user. Online authentication is separate.
        let signed_in=store.load_profile_data().map_err(error)?.is_some();
        let mut controller=Controller::new((end-start) as f32*(1.0/60.0),film,signed_in,tuning).map_err(error)?;
        let (loaded_index,loaded_frames,loaded_hulls)=if let Some(a)=recording.take() {
            controller.restore_take(a.take);
            // StateReplay restores the saved hull lookup and game-mode mask,
            // with independent ownership while the front-end game is paused.
            let global=assets.mounts.read_path("data/script/challenge/global.lua").map_err(error)?;
            let mut hulls=crate::challenge::ChallengeSystem::new_career(std::sync::Arc::clone(&db),&global).map_err(error)?;
            let locations=skate2_formats::location::LocationTable::parse(&assets.mounts.read_path("data/content/world/stream/BAM/BAM_Sim.loc").map_err(error)?).map_err(error)?;
            for location in locations.locations {hulls.world_mut().locations.insert(crate::challenge::keys::hash_value32(&location.name),location.transform);}
            hulls.world_mut().dynamic_hulls.unpack(&a.hull_states);
            hulls.game_load_free_skate_dynamic_hulls();
            (Some(a.index),Some(a.frames),Some(hulls))
        } else {(None,None,None)};
        self.inner.bound_input.as_ref().ok_or_else(||error("input absent"))?
            .try_set_replay_pause(true).map_err(error)?;
        if let Some(career)=self.career.as_mut() {career.nis.unload_any_preloaded_niss();}
        self.replay_clock=Some(ReplayClock::new(start,end));
        self.replay_restorer=PresentationRestorer::new();
        self.replay_editor=Some(ReplayEditorSession {export:None,loaded_index,loaded_frames,loaded_hulls,intro_done:false,animation_ready:false,controller,camera:NisCamera::default(),rate:0.,
            game_camera:CameraRestore::new(),shakes,mover:Mover::default(),spaces:Spaces::default(),
            last_rate_frame:0.,exit:0,save:None,location:String::new(),last_stream_focus:[0.;3],collision:Default::default(),hom_results,
            world_location,world});
        if self.main_menu.is_none()&&!hom_results {self.pause_menu.hide_for_replay();}
        self.restore_editor_frame(0.)?;
        let location=self.initial_replay_location().map_err(error)?;
        self.replay_editor.as_mut().expect("opened editor").location=location;
        let e=self.replay_editor.as_mut().expect("opened editor");
        e.last_stream_focus=e.spaces.anchor.map(|p|[p[1],p[2],p[0]]).ok_or_else(||error("recorded focus absent"))?;
        Ok(())
    }

    pub fn replay_editor_open(&self)->bool {self.replay_editor.is_some()}
    pub fn set_replay_stream_focuses(&mut self,focuses:[crate::streaming::replay_focus::VisualFocus;2]) {
        self.replay_visual_focuses=Some(focuses);
    }
    pub fn replay_editor_snapshot(&self)->Option<replay_editor::Snapshot> {
        self.replay_editor.as_ref().map(|e|e.controller.snapshot())
    }
    pub fn take_replay_editor_states(&mut self)->Vec<u32> {
        self.replay_editor.as_mut().map(|e|e.controller.take_states()).unwrap_or_default()
    }
    /// Only the original APT intro's ScreenManager.IntroComplete makes input live.
    pub fn replay_editor_intro_complete(&mut self)->Result<(),SelectedLiveError> {
        if let Some(editor)=self.replay_editor.as_mut() {
            editor.intro_done=true;
            if editor.loaded_index.is_none() {
                editor.controller.intro_complete().map_err(error)?;
                self.frontend_fade.reveal();
            } else {editor.controller.enable_timeline_render();}
        }
        Ok(())
    }
    pub fn take_replay_editor_save(&mut self)->Option<Action> {
        self.replay_editor.as_mut().and_then(|e|e.save.take())
    }
    pub fn replay_editor_animation_ready(&mut self) {
        if let Some(editor)=self.replay_editor.as_mut() {editor.animation_ready=true;}
    }
    pub fn replay_editor_media_full(&mut self,replay:bool) {
        if let Some(e)=&mut self.replay_editor {e.controller.media_full(replay);}
    }
    pub fn replay_editor_footage_loaded(&mut self,replay:bool,items:Vec<replay_editor::FootageItem>) {
        if let Some(e)=&mut self.replay_editor {e.controller.footage_loaded(replay,items);}
    }
    pub fn replay_editor_footage_ready(&mut self) {
        if let Some(e)=&mut self.replay_editor {e.controller.footage_ready();}
    }
    /// Original media metadata uses the FE location trigger at the replay's
    /// initial player position (FELocationManager8261bbb0/82615b68).
    pub fn replay_editor_location(&self)->Result<String,String> {
        Ok(self.replay_editor.as_ref().ok_or("editor absent")?.location.clone())
    }
    fn initial_replay_location(&self)->Result<String,String> {
        let editor=self.replay_editor.as_ref().ok_or("editor absent")?;
        let p=editor.spaces.anchor.ok_or("replay player position absent")?;
        let db=self.movie_attrib_db().map_err(|e|e.to_string())?;
        let world=self.inner.world.as_ref().ok_or("world absent")?;
        let terrain=self.inner.terrain.as_ref().ok_or("terrain absent")?.terrain_for(&world.world).map_err(|e|e.to_string())?;
        let layers=terrain.layers.lock().map_err(|_|"FE location layers lock")?;
        let triggers=crate::challenge::keys::hash64("worldpainter_fe_location_triggers");
        let mut key=layers.try_attrib_key(5,[p[1],p[0]]).map_err(|e|e.to_string())?;
        if key==0||!db.has_collection(triggers,key) {key=layers.try_attrib_key(4,[p[1],p[0]]).map_err(|e|e.to_string())?;}
        if key==0 {key=u64::MAX;}
        Ok(crate::challenge::hom_save::location_name(&db,key))
    }
    /// SerializeReplay82575728: save the entire retained recording and its
    /// edit track, not only the trimmed portion used for playback/encoding.
    pub fn serialize_replay_editor(&self)->Result<Vec<u8>,String> {
        let editor=self.replay_editor.as_ref().ok_or("Replay Editor is not open")?;
        let snapshot=editor.controller.snapshot();
        let hulls=editor.loaded_hulls.as_ref().or_else(||self.career.as_ref().map(|c|&c.system)).or(self.freeskate.hulls.as_ref());
        let default=crate::challenge::dynamic_hull::DynamicHulls::default();
        let hulls=hulls.map(|s|&s.world().dynamic_hulls).unwrap_or(&default);
        let mut packed=hulls.pack();packed[4..8].copy_from_slice(&hulls.game_type_flags.to_be_bytes());
        archive::encode(&editor.controller.author.take,snapshot.film_pack,editor.world_location,packed,self.replay_index(),self.recorded_frames())
    }
    pub fn replay_editor_save_complete(&mut self,replay:bool,first:bool,result:Result<String,String>) {
        if let Some(editor)=self.replay_editor.as_mut() {
            match result {
                Ok(path)=>editor.controller.save_complete(replay,first,&path),
                Err(message)=>editor.controller.save_failed("ID_REPLAY_SAVEWARNING_TITLE",&message),
            }
        }
    }
    /// SimController::Update1 dispatches GameSim at828a739c before checking
    /// whether simulation is paused. Replay still streams waypoint additions
    /// and culls, so it must keep the same registered subscriber route alive.
    fn dispatch_replay_game_sim(&self)->Result<(),SelectedLiveError> {
        let world_owner=self.inner.world.as_ref().ok_or_else(||error("world absent"))?;
        let assets=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?;
        let terrain_owner=self.inner.terrain.as_ref().ok_or_else(||error("terrain absent"))?;
        let recipes=world_owner.living_world_recipes.lock()
            .map_err(|_|error("GameSim recipe runtime lock poisoned"))?;
        let terrain=terrain_owner.terrain_for(&world_owner.world).map_err(error)?;
        let ground=terrain_owner.ground_pool_for(&world_owner.world).map_err(error)?;
        let mut world=world_owner.world.lock()
            .map_err(|_|error("GameSim world lock poisoned"))?;
        crate::selected_living_world_recipes::mood::SelectedMoodFrame {
            triggers:&terrain.triggers,runtime:&recipes,
            vault:&assets.vault,painter:&terrain.layers,
        }.dispatch_game_sim(&mut world,&assets.cvars,ground).map_err(error)
    }
    pub(super) fn update_replay_editor(&mut self,input:&replay_editor::Input,dt:f32,focused:bool)->Result<(),SelectedLiveError> {
        // This also runs during export, while its separate frame preparation
        // moves the stream focus. Do not run a simulation tick to drain events.
        self.dispatch_replay_game_sim()?;
        let editor=self.replay_editor.as_mut().ok_or_else(||error("editor absent"))?;
        if let Some(export)=&mut editor.export {
            if focused&&input.keys.iter().any(|key|matches!(key,2|3)) {export.cancelled=true;}
            return Ok(());
        }
        let tuning=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?.cvars.try_replay_editor().map_err(error)?;
        if editor.exit==0 {
            let was_ready=editor.controller.ready;
            match editor.controller.update(input,dt,focused,tuning,&editor.spaces).map_err(error)? {
                Action::None=>(),
                Action::Exit=>{editor.exit=1;self.frontend_fade.cover();}
                action@ (Action::SaveReplay|Action::SavePhoto|Action::Browse(_)|Action::DeleteMedia(_,_))=>editor.save=Some(action),
                Action::SignInRequired=>editor.controller.sign_in_notice("ID_EANATION","ID_ONLINE_PLEASE_SIGN_IN"),
            }
            if was_ready&&!editor.controller.ready {editor.intro_done=false;editor.animation_ready=false;}
        } else if editor.exit==1 && self.frontend_fade.covered() {
            // FadeComplete8265f6d8 restores the *untrimmed* newest live frame.
            editor.controller.author.take.set_parameter(1.);editor.exit=2;
        }
        self.restore_editor_frame(dt)?;
        let (focus,world)={
            let editor=self.replay_editor.as_ref().expect("open editor");
            // Exit stage 5 is StateReplay::PurgeAdditionalWorld 0x827662f0: the main world again.
            let world=if editor.exit==5 {Some(super::world_travel::TravelWorld::Main)} else {editor.world.clone()};
            (editor.spaces.anchor.map(|p|[p[1],p[2],p[0]]).ok_or_else(||error("recorded local skater absent"))?,world)
        };
        // StateReplay::LoadAdditionalWorld: a saved replay's world is in place before it shows; one
        // the port cannot load leaves the main world.
        let world_ready=match &world {
            Some(world)=>match self.try_prepare_world(world,focus)? {
                super::world_travel::WorldReadiness::Pending=>false,
                super::world_travel::WorldReadiness::Ready=>true,
                super::world_travel::WorldReadiness::MainInstead=>{
                    self.replay_editor.as_mut().expect("open editor").world=Some(super::world_travel::TravelWorld::Main);
                    true
                }
            },
            None=>true,
        };
        let editor=self.replay_editor.as_mut().expect("open editor");
        let terrain=self.inner.terrain.as_mut().ok_or_else(||error("terrain absent"))?;
        terrain.try_advance_asset_release().map_err(error)?;
        terrain.try_update_focus(focus).map_err(error)?;
        let hulls_loaded=if editor.exit>=4 {
            terrain.try_update_dynamic_hulls().map_err(error)?;
            terrain.dynamic_hulls_unloaded()
        } else if let Some(hulls)=&mut editor.loaded_hulls {
            career::try_dynamic_hull_state(None,Some(hulls),terrain.as_mut(),focus)?
        } else {true};
        let mut status=terrain.replay_focus_status().map_err(error)?;
        let mut exit_loaded=status.loaded;
        let xml=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?.cvars.try_enable_stream_xml().map_err(error)?;
        if let Some(focuses)=&self.replay_visual_focuses {
            for (i,visual) in focuses.iter().enumerate() {
                let visual=visual.status(focus,xml).map_err(error)?;
                if i==0 {exit_loaded&=visual.loaded;}
                status=status.combine(visual);
            }
        }
        let loaded=status.loaded;
        if editor.intro_done&&editor.animation_ready&&!editor.controller.ready&&loaded&&hulls_loaded&&world_ready {
            editor.controller.intro_complete().map_err(error)?;self.frontend_fade.reveal();
        }
        // SimUpdate82570e60 never stalls Play. UpdateStall82567158 latches a
        // costly scrub until GameLoad completes; small moves update the origin
        // too, and do not replace load cost with wall-clock waiting.
        if editor.controller.mode==Mode::Play||loaded {editor.controller.set_stalled(false);}
        else if !editor.controller.stalled {
            let distance=focus.iter().zip(editor.last_stream_focus).map(|(a,b)|(a-b)*(a-b)).sum::<f32>().sqrt();
            editor.last_stream_focus=focus;
            editor.controller.set_stalled(distance>=1.&&status.ticks>=120);
        }
        if editor.exit==3&&exit_loaded&&editor.loaded_hulls.is_some() {
            let hulls=editor.loaded_hulls.as_mut().expect("loaded replay hulls");hulls.game_unload_dynamic_hulls();
            let requests=std::mem::take(&mut hulls.world_mut().outputs);
            career::queue_dynamic_hull_requests(terrain.as_mut(),requests)?;editor.exit=4;return Ok(());
        }
        let finished=(editor.exit==3 && exit_loaded&&editor.loaded_hulls.is_none())||(editor.exit==4&&hulls_loaded);
        // StateReplay::ReplayState: a replay that loaded an additional world (mbAdditionalWorld)
        // purges it before the front end returns.
        if finished&&matches!(editor.world,Some(super::world_travel::TravelWorld::Additional{..})) {editor.exit=5;return Ok(());}
        if finished||(editor.exit==5&&world_ready) {
            self.inner.bound_input.as_ref().ok_or_else(||error("input absent"))?
                .try_set_replay_pause(false).map_err(error)?;
            let hom_results=self.replay_editor.take().is_some_and(|e|e.hom_results);
            self.replay_clock=None;
            self.replay_restorer=PresentationRestorer::new();
            super::replay_session::restore_game_cloth(&mut self.inner, self.career.as_ref())?;
            if let Some(reel)=self.main_menu.as_mut().and_then(|m|m.skate_reel.as_mut()) {reel.replay_closed();}
            else if hom_results {if let Some(career)=self.career.as_mut() {career.reenter_hom_results();}}
            else {self.pause_menu.return_from_replay();}
            self.frontend_reveal_pending=true;
        } else if editor.exit==2 && exit_loaded {editor.exit=3;}
        Ok(())
    }

    fn restore_editor_frame(&mut self,dt:f32)->Result<(),SelectedLiveError> {
        let editor=self.replay_editor.as_mut().ok_or_else(||error("editor absent"))?;
        if editor.controller.take_collision_reset() {editor.collision=Default::default();}
        let index=editor.loaded_index.as_ref().unwrap_or_else(||self.career.as_ref().map_or(&self.freeroam_replay,|c|c.replay()));
        let frames=editor.loaded_frames.as_ref().unwrap_or(&self.replay_frames);
        let clock=self.replay_clock.as_mut().ok_or_else(||error("clock absent"))?;
        let seconds=editor.controller.author.take.parameter()*editor.controller.author.take.length();
        clock.update(seconds,index);
        let scrub=(seconds*60.+clock.start as f32).min(clock.end as f32);
        let rate=if editor.export.is_some()||editor.controller.mode==Mode::Play {editor.controller.author.take.value(element::SPEED)*0.01}
            else {scrub-editor.last_rate_frame};
        editor.last_rate_frame=scrub;
        editor.rate=if rate.abs()<=0.01 {0.} else {rate.min(2.)};
        let next=frames.get(&clock.current).ok_or_else(||error("tracked frame absent"))?;
        let previous=index.previous_frame(clock.current).and_then(|i|frames.get(&i));
        editor.game_camera.restore(&next.visual.camera,previous.map(|f|&f.visual.camera),clock.fraction,!clock.rewinding);
        let camera=editor.game_camera.view().ok_or_else(||error("recorded game camera absent"))?;
        self.replay_restorer.restore_recorded(previous.map(|f|&f.visual),&next.visual,clock.current,clock.fraction,camera.position,!clock.rewinding,dt).map_err(error)?;
        // The recording can cross Party Play turns; use this frame's local
        // player, not whichever Actor happens to own the controller at entry.
        let identity=next.visual.skaters.get(next.local_skater).ok_or_else(||error("recorded local player index"))?.palette_identity();
        let skater=self.replay_restorer.skater(&identity).ok_or_else(||error("recorded local skater absent"))?;
        let axes:[[f32;3];4]=std::array::from_fn(|r|std::array::from_fn(|c|skater.root[r*4+c]));
        let ice=|p:[f32;3]|[p[2],p[0],p[1]];
        let camera_axes=[camera.right,camera.up,camera.forward,camera.position];
        let mut spaces=Spaces {anchor:Some(ice(axes[3])),anchor_orientation:Some(ice_camera::make_ice_from_skate([axes[0],axes[1],axes[2]])),..Spaces::default()};
        spaces.avatars[0]=spaces.anchor;
        spaces.avatars[1]=Some(ice(camera.position));
        spaces.avatars[2]=Some(ice(std::array::from_fn(|i|camera.forward[i].mul_add(3.,camera.position[i]))));
        spaces.avatars[3]=Some(ice([skater.hips_position[0],camera.position[1],skater.hips_position[2]]));
        spaces.avatar_orientations[0]=Some(ice_camera::avatar_orientation(axes));
        spaces.avatar_orientations[1]=Some(ice_camera::avatar_orientation(camera_axes));
        spaces.avatar_orientations[2]=spaces.avatar_orientations[1];
        spaces.avatar_orientations[3]=spaces.avatar_orientations[0];
        editor.spaces=spaces;
        if let Some(orientation)=spaces.anchor_orientation {editor.mover.update_forward(orientation[0],dt);}
        // UpdateShake82a58e68 uses fParameter * take length, not this tick's dT.
        let (mut matrix,fov)=editor.controller.author.take.camera(&editor.shakes,&spaces,&mut editor.mover,seconds).map_err(error)?;
        let near=editor.controller.author.take.value(element::NEAR_CLIP);
        let (collision_enabled,max_distance)=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?
            .cvars.try_replay_camera_collision().map_err(error)?;
        let world=&self.inner.world.as_ref().ok_or_else(||error("world absent"))?.world;
        let ground=self.inner.terrain.as_ref().ok_or_else(||error("terrain absent"))?.ground_pool_for(world).map_err(error)?;
        let mut physical=world.lock().map_err(|_|error("world lock poisoned"))?;
        let (queries,dmos)=physical.world_line_owners();
        let queries=queries.ok_or_else(||error("world line queries absent"))?;
        let dmos=dmos.ok_or_else(||error("world line DMO owner absent"))?;
        editor.collision.resolve(&mut editor.controller.author,&spaces,&mut matrix,skater.hips_position,
            collision_enabled,max_distance,near,|lines,fatness| {
                use crate::skeleton_root_transforms::SkeletonRootVector;
                let batch=queries.try_acquire_line_batch(dmos,"ThePresentation",7).map_err(|e|format!("{e:?}"))?;
                queries.try_begin(&batch).map_err(|e|format!("{e:?}"))?;
                for &(start,end) in lines {
                    let vector=|v:[f32;3]|SkeletonRootVector::known([v[0],v[1],v[2],1.]);
                    queries.try_add_line(&batch,crate::collision_line_collection::CollisionLineInput {start:vector(start),end:vector(end),fatness}).map_err(|e|format!("{e:?}"))?;
                }
                queries.try_end(&batch,ground,dmos,&mut crate::collision_line_numeric::HostCollisionLineNumeric).map_err(|e|format!("{e:?}"))?;
                queries.try_sync(&batch).map_err(|e|format!("{e:?}"))?;
                let mut hits=Vec::with_capacity(lines.len());
                for i in 0..lines.len() {hits.push(queries.try_get_result(&batch,i as u32).map_err(|e|format!("{e:?}"))?
                    .map(|hit|collision::Hit {normal:hit.normal.xyz,time:hit.line_parameter}));}
                queries.try_release(&batch).map_err(|e|format!("{e:?}"))?;
                Ok(hits)
            }).map_err(error)?;
        editor.camera=NisCamera {matrix:Some(matrix),fov,near_plane:editor.controller.author.take.value(element::NEAR_CLIP)};
        Ok(())
    }

    pub fn replay_export_active(&self)->bool {self.replay_editor.as_ref().is_some_and(|e|e.export.is_some())}
    pub fn replay_export_cancelled(&self)->bool {self.replay_editor.as_ref().and_then(|e|e.export.as_ref()).is_some_and(|e|e.cancelled)}
    pub fn begin_replay_export(&mut self)->Result<(),SelectedLiveError> {
        let clock=self.replay_clock.ok_or_else(||error("replay clock absent"))?;
        let e=self.replay_editor.as_mut().ok_or_else(||error("editor absent"))?;
        if e.export.is_some()||!e.controller.saving {return Err(error("invalid video export state"));}
        let author=e.controller.author.clone();
        if !author.take.length().is_finite()||author.take.length()<=0.||author.end_frame()<=author.start_frame() {
            return Err(error("the selected replay has no video frames"));
        }
        let seconds=f64::from(author.start_frame())/30.;
        let end_seconds=f64::from(author.end_frame())/30.;
        e.export=Some(VideoExport {author,clock,seconds,end_seconds,mover:std::mem::take(&mut e.mover),game_camera:std::mem::replace(&mut e.game_camera,CameraRestore::new()),
            collision:std::mem::take(&mut e.collision),frame:None,cancelled:false,finished:false});
        let p=(seconds/f64::from(e.controller.author.take.length())) as f32;
        e.controller.author.take.set_parameter_force(p);
        e.controller.export_progress(0.,false);
        self.replay_clock=Some(ReplayClock::new(clock.start,clock.end));
        Ok(())
    }
    /// Each output sample uses the normal recorded-frame, ICELITE, shake and
    /// collision paths. Streaming waits never advance the video clock.
    pub fn prepare_replay_export_frame(&mut self,frame:u64)->Result<replay_editor::ExportStatus,SelectedLiveError> {
        use replay_editor::ExportStatus;
        let e=self.replay_editor.as_mut().ok_or_else(||error("editor absent"))?;
        let export=e.export.as_mut().ok_or_else(||error("video export absent"))?;
        if export.cancelled {return Ok(ExportStatus::Cancelled);}
        if export.finished {return Ok(ExportStatus::Finished);}
        let next=export.frame!=Some(frame);
        if next {
            if let Some(previous)=export.frame {
                if frame!=previous+1 {return Err(error("video frame order"));}
                let speed=e.controller.author.take.value(element::SPEED);
                if !speed.is_finite()||speed<=0. {return Err(error("invalid replay speed"));}
                // The export clock uses f64 so long, slow clips do not gain
                // extra frames through repeated f32 parameter accumulation.
                export.seconds+=f64::from(speed)*0.01/60.;
            } else if frame!=0 {return Err(error("video must begin at frame zero"));}
            export.frame=Some(frame);
            if frame>0&&export.seconds+1e-9>=export.end_seconds {
                export.finished=true;e.controller.export_progress(1.,true);return Ok(ExportStatus::Finished);
            }
            e.controller.author.take.set_parameter((export.seconds/f64::from(e.controller.author.take.length())) as f32);
        }
        self.restore_editor_frame(if next {1./60.} else {0.})?;
        let e=self.replay_editor.as_mut().expect("export editor");
        let focus=e.spaces.anchor.map(|p|[p[1],p[2],p[0]]).ok_or_else(||error("replay focus absent"))?;
        let terrain=self.inner.terrain.as_mut().ok_or_else(||error("terrain absent"))?;
        terrain.try_advance_asset_release().map_err(error)?;
        terrain.try_update_focus(focus).map_err(error)?;
        let hulls_loaded=if let Some(hulls)=&mut e.loaded_hulls {career::try_dynamic_hull_state(None,Some(hulls),terrain.as_mut(),focus)?} else {true};
        let mut loaded=terrain.replay_focus_status().map_err(error)?.loaded&&hulls_loaded;
        let xml=self.inner.assets.as_ref().ok_or_else(||error("assets absent"))?.cvars.try_enable_stream_xml().map_err(error)?;
        if let Some(focuses)=&self.replay_visual_focuses {
            for visual in focuses {loaded&=visual.status(focus,xml).map_err(error)?.loaded;}
        } else {loaded=false;}
        let a=&e.controller.author;
        let progress=(a.take.parameter()-a.start)/(a.end-a.start);
        e.controller.export_progress(progress,false);
        Ok(if loaded {ExportStatus::Frame} else {ExportStatus::Waiting})
    }
    pub fn end_replay_export(&mut self)->Result<(),SelectedLiveError> {
        if let Some(e)=self.replay_editor.as_mut() {
            if let Some(export)=e.export.take() {
                e.controller.author=export.author;e.mover=export.mover;
                e.game_camera=export.game_camera;e.collision=export.collision;
                self.replay_clock=Some(export.clock);
                self.restore_editor_frame(0.)?;
                if let Some(e)=self.replay_editor.as_mut() {e.rate=0.;}
            }
        }
        Ok(())
    }
    pub fn replay_editor_save_cancelled(&mut self) {
        if let Some(e)=self.replay_editor.as_mut(){e.controller.save_cancelled();}
    }

    pub fn try_replay_editor_visual_snapshot(&self)->Result<SelectedVisualSnapshot<'_>,SelectedLiveError> {
        let editor=self.replay_editor.as_ref().ok_or_else(||error("editor absent"))?;
        let focus=editor.spaces.anchor.map(|p|FrozenFocus::At([p[1],p[2],p[0]]));
        let mut visual=self.visual_snapshot(Some(editor.camera),focus)?;
        if let Some(frame)=self.replay_clock.and_then(|clock|self.recorded_frames().get(&clock.current)) {
            visual.dynamic_objects=frame.world.dynamic_objects.clone();
            visual.vehicles=frame.world.vehicles.clone();
            visual.pedestrians=frame.world.pedestrians.clone();
            for pedestrian in &visual.pedestrians {pedestrian.restore_replay_pose().map_err(error)?;}
            visual.traffic_light_status=frame.world.traffic_lights;
            visual.scene_characters.clear();
            let first=frame.visual.skaters.first().and_then(|s|self.replay_restorer.skater(&s.palette_identity()))
                .ok_or_else(||error("recorded first skater absent"))?;
            visual.palette=first.palette;
            visual.materials=first.materials.ok_or_else(||error("recorded skater materials absent"))?.clone();
            visual.skater_hips_position=first.hips_position;visual.skater_world_bounds=first.bounds;
            visual.skater_opacity=first.opacity;visual.skater_ghosted=first.ghosted;visual.skater_entity_type=0x122;
            let local=frame.visual.skaters.get(frame.local_skater).and_then(|s|self.replay_restorer.skater(&s.palette_identity()))
                .ok_or_else(||error("recorded local skater absent"))?;
            visual.local_skater_shadow=(local.hips_position,local.bounds);
            visual.party_skaters.clear();
            for (i,skater) in frame.visual.skaters.iter().enumerate().skip(1) {
                if let Some(restored)=self.replay_restorer.skater(&skater.palette_identity())
                    && let Some(materials)=restored.materials {
                    visual.party_skaters.push(SelectedSkaterVisual {name_tag:None,palette:restored.palette,materials:materials.clone(),
                        hips_position:restored.hips_position,world_bounds:restored.bounds,opacity:restored.opacity,
                        ghosted:restored.ghosted,entity_type:0x122,lobby_ghost:false,players_recipe:frame.players_recipe[i]});
                }
            }
        }
        visual.visual_indicators=None;
        visual.hom_visual=None;
        Ok(visual)
    }
}
