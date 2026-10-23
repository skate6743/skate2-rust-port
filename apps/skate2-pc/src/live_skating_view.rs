//! App-only visual consumer of one completed selected frame.
//! The map consumes completed moving visual inventories. Full native
//! processed Presentation registration/PROXY activation remains pending.
use crate::{
    dmo_render::DmoRender,
    gpu_renderer::{
        CameraRelativeClip, GpuObject, GpuRenderer, GpuScene, RenderResult, character::CharacterGhostState,
        character_palette::SelectedCharacterGpuBinding,
    },
    render_inspection::Inspection,
    render_shadows::ShadowCasterInputs,
    render_window::RenderWindowClient,
    selected_character_material,
};
use skate2_core::{
    SelectedVisualSnapshot,
    character::{material_admission::CharacterMaterialAdmission, palette::CharacterPalette},
    presentation_restore::{PresentationRestorer, camera::CameraView},
};
use skate2_formats::mount::MountSet;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
#[path="trick_guide_room.rs"]
mod trick_guide_room;

/// Keeps the inspection map in the scene's retained map draw list, the
/// selected character objects in its object list and spawned DMOs/vehicles in
/// the shared dynamic draw list. Vehicles use the existing character palette
/// implementation and shared dynamic material/shadow pipelines.
pub(crate) struct LiveSkatingView {
    map: Inspection,
    lighting: crate::render_irradiance::SkaterLighting,
    /// The Vault the character materials' draw orders come from.
    mounts: Arc<MountSet>,
    bindings: Vec<SelectedCharacterGpuBinding>,
    character_palette: Option<skate2_core::character::palette::CharacterPaletteIdentity>,
    /// The later skaters' objects, after the local skater's in `scene.objects`.
    party: Vec<PartySkaterObjects>,
    /// The NIS scene characters' objects, after the skaters' in `scene.objects`.
    scene_characters: Vec<SceneCharacterObjects>,
    marker_ui: Option<crate::marker_ui::MarkerUi>,
    audio: Option<crate::game_audio::GameAudio>,
    replay_watermarks:Option<[crate::gpu_renderer::GpuTexture;2]>,
    /// `m_pTex_DeckAO`, which `SetTexturePointers` 0x82828728 leaves null when absent.
    deck_occlusion: Option<crate::gpu_renderer::GpuTexture>,
    dynamic_objects: DmoRender,
    vehicles: crate::vehicle_render::VehicleRender,
    pedestrians: crate::pedestrian_render::PedestrianRender,
    editor: Option<crate::edit_skater_room::EditSkaterRoomView>,
    trick_guide_room:Option<trick_guide_room::Room>,
    retained_trick_guide_scene:Option<GpuScene>,
    retained_world_scene: Option<GpuScene>,
    main_menu: Option<Inspection>,
    retained_main_scene: Option<GpuScene>,
    /// The FreeSkate viewer's skater in the front end scene's objects, and the last front end
    /// frame's time for its lighting.
    viewer: Option<SceneCharacterObjects>,
    /// The front end's AI skaters' objects, after the viewer's in the front end scene's objects.
    frontend_ai: Vec<PartySkaterObjects>,
    main_menu_previous: Option<Instant>,
    /// The last tick's world inputs, which frames between ticks draw again.
    retained: Option<RetainedFrame>,
    /// The last tick's frame waits for the presentation's restore.
    pending_draw: bool,
    /// `VIPresentation`: the challenge indicators drawn into the scene.
    visual_indicators: crate::visual_indicator_view::VisualIndicatorView,
    /// `ParticlePresentation`: the world's smoke, steam and leaves.
    particles: crate::particle_view::ParticleView,
    /// `CrowdPresentation`'s imposter targets, one pedestrian draw per target slot.
    crowd: Vec<crate::pedestrian_render::PedestrianRender>,
    /// `AptMovieIntegration`'s render side: the front end's movies.
    movie: crate::movie_playback::MovieRenderer,
}

impl LiveSkatingView {
    pub(crate) fn set_trick_guide(&mut self,state:Option<&skate2_core::ui::trick_guide::Snapshot>)->RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.set_trick_guide(state)
    }
    pub(crate) fn set_skate_profile(&mut self, state: Option<&skate2_core::ui::skate_profile::View>) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.set_skate_profile(state)
    }
    pub(crate) fn update_trick_guide(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,now:Instant,
        frame:&skate2_core::TrickGuideView<'_>,pause:&skate2_core::ui::pause_menu::PauseMenuSnapshot)->RenderResult<()> {
        if self.trick_guide_room.is_none() {
            let mut room=trick_guide_room::Room::load(&self.mounts,now)?;
            let mut new_scene=room.initialize(renderer)?;
            room.update(renderer,&mut new_scene,&self.mounts,now,frame)?;
            self.retained_trick_guide_scene=Some(std::mem::replace(scene,new_scene));self.trick_guide_room=Some(room);
        } else {self.trick_guide_room.as_mut().expect("room").update(renderer,scene,&self.mounts,now,frame)?;}
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_trick_guide(renderer,scene,now,pause)
    }
    pub(crate) fn close_trick_guide(&mut self,scene:&mut GpuScene) {
        if let Some(prior)=self.retained_trick_guide_scene.take(){*scene=prior;}
        self.trick_guide_room=None;
    }
    pub(crate) fn set_replay_editor(&mut self,state:Option<&skate2_core::ui::replay_editor::Snapshot>,states:&[u32],
        reel:Option<&skate2_core::ui::skate_reel::Snapshot>)->RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.set_replay_editor(state,states,reel)
    }
    pub(crate) fn set_music_player(&mut self, state: &skate2_core::ui::music_player::Snapshot) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.set_music_player(state)
    }
    /// What the StartHub's, Options' and the music player's screens told their states since the
    /// last call.
    pub(crate) fn take_screen_events(&mut self) -> crate::marker_ui::ScreenEvents {
        self.marker_ui.as_mut().map_or_else(Default::default, crate::marker_ui::MarkerUi::take_screen_events)
    }
    pub(crate) fn exit_to_front_end(&mut self) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.exit_to_front_end()
    }
    pub(crate) fn end_career(&mut self, last: &skate2_core::CareerHud) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.end_career(last)
    }
    pub(crate) fn take_online_match_intro(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(crate::marker_ui::MarkerUi::take_online_match_intro)
    }
    #[expect(clippy::too_many_arguments, reason = "the hub, its screen calls, the screen over it, the popup and its messages")]
    pub(crate) fn update_online_hub(&mut self, renderer: &GpuRenderer, hub: Option<&skate2_core::ui::online_hub::Snapshot>, opened: bool,
        calls: &[skate2_core::ui::online_hub::ScreenCall], screen: Option<&skate2_core::ui::online_match::Snapshot>,
        popup: Option<&skate2_core::ui::online_hub::Popup>, nickname: &str, central: &[skate2_core::challenge::hud::CentralMessage]) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_online_hub(renderer, hub, opened, calls, screen, popup, nickname, central)
    }
    pub(crate) fn take_online_hub_intro(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(crate::marker_ui::MarkerUi::take_online_hub_intro)
    }
    pub(crate) fn update_online_lobby_hud(&mut self, states: &[(i32, skate2_core::challenge::turn_hud::TurnHudView)],
        title: &skate2_core::challenge::system::MissionTitle) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_online_lobby_hud(states, title)
    }
    pub(crate) fn update_online_event_log(&mut self, open: bool, log: Option<skate2_core::online::event_log::EventLogSnapshot>) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_online_event_log(open, log.as_ref())
    }
    pub(crate) fn update_online_chat(&mut self, prompt: Option<&skate2_core::online::chat::ChatPrompt>, nickname: Option<&skate2_core::ui::profile_name::Prompt>) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_online_chat(prompt, nickname)
    }
    pub(crate) fn update_chyron(&mut self, state: i32, songs: &[u32]) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_chyron(state, songs)
    }
    pub(crate) fn set_options_menu(&mut self, state: skate2_core::ui::options::Snapshot) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.set_options_menu(state)
    }
    pub(crate) fn set_free_skate_settings(&mut self, settings: Option<(bool, bool)>) {
        if let Some(ui) = self.marker_ui.as_mut() {
            ui.set_free_skate_settings(settings);
        }
    }
    pub(crate) fn navigation(&self) -> Vec<skate2_core::presentation::PresentationNavigationArena> {
        self.map.navigation()
    }
    pub(crate) fn take_jacked_request(&mut self)->bool{
        self.marker_ui.as_mut().is_some_and(|ui|ui.take_jacked_request())
    }
    pub(crate) fn take_poster_intro_complete(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(|ui| ui.take_poster_intro_complete())
    }
    pub(crate) fn take_unlock_intro_complete(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(|ui| ui.take_unlock_intro_complete())
    }
    pub(crate) fn take_cash_anim_done(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(|ui| ui.take_cash_anim_done())
    }
    pub(crate) fn take_hom_annals_intro_complete(&mut self)->bool {
        self.marker_ui.as_mut().is_some_and(|ui|ui.take_hom_annals_intro_complete())
    }
    pub(crate) fn take_hom_results_intro_complete(&mut self)->bool {
        self.marker_ui.as_mut().is_some_and(|ui|ui.take_hom_results_intro_complete())
    }
    pub(crate) fn take_skate101_intro_complete(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(|ui| ui.take_skate101_intro_complete())
    }
    pub(crate) fn take_change_game_intro(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(|ui| ui.take_change_game_intro())
    }
    pub(crate) fn take_end_options_screen(&mut self) -> (bool, bool) {
        self.marker_ui.as_mut().map_or((false, false), |ui| ui.take_end_options_screen())
    }
    pub(crate) fn take_sponsor_screen(&mut self) -> (bool, bool) {
        self.marker_ui.as_mut().map_or((false, false), |ui| ui.take_sponsor_screen())
    }
    pub(crate) fn take_slappys_intro_complete(&mut self) -> bool {
        self.marker_ui.as_mut().is_some_and(|ui| ui.take_slappys_intro_complete())
    }
    /// The pausing career screens' frames: front end only.
    pub(crate) fn update_career_frontend(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        now: Instant, career: &skate2_core::CareerHud) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("marker UI not initialized")?.update_career_frontend(renderer, scene, now, career)
    }
    /// FE state 28 on the stack or not, with the movie player's subtitle line.
    pub(crate) fn set_movie_state(&mut self, open: bool, subtitle: skate2_core::ui::movie_player::MovieSubtitle,
        subtitles: bool, screen: Option<skate2_core::ui::movie_player::MovieScreen>) -> RenderResult<Option<u64>> {
        match self.marker_ui.as_mut() {
            Some(ui) => ui.set_movie_state(open, subtitle, subtitles, screen),
            None => Ok(None),
        }
    }
    /// The movie player's frames: front end only, the HUD hidden.
    pub(crate) fn take_extras_events(&mut self) -> (bool, bool, bool) {
        self.marker_ui.as_mut().map_or((false, false, false), |ui| ui.take_extras_events())
    }
    pub(crate) fn update_movie_frontend(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene, now: Instant) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("marker UI not initialized")?.update_movie_frontend(renderer, scene, now)
    }
    /// The last career update's NIS line and the line the subtitle layer shows.
    pub(crate) fn subtitle_lines(&self) -> (Option<u32>, Option<u32>) {
        self.marker_ui.as_ref().map_or((None, None), |ui| (ui.nis_subtitle_line(), ui.subtitle_shown()))
    }
    /// The mission hulls' presentation collections the session's hull loaders registered.
    pub(crate) fn set_hull_presentations(&mut self, hulls: Vec<skate2_core::presentation::HullPresentation>) {
        self.map.set_hull_presentations(hulls);
    }
    /// The presentation stream for the session's `IsAreaLoaded`.
    pub(crate) fn presentation_stream_area(&self) -> Option<skate2_core::streaming::area_loaded::PresentationStreamArea> {
        self.map.presentation_stream_area()
    }
    pub(crate) fn replay_stream_focuses(&self)->Option<[skate2_core::streaming::replay_focus::VisualFocus;2]> {
        self.map.replay_stream_focuses()
    }
    pub(crate) fn prepare_map_travel(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        position: [f32; 3], disabled: bool, xml_enabled: bool,
    ) -> RenderResult<bool> {
        self.map.prepare_teleport(renderer, scene, position, disabled, xml_enabled)
    }
    pub(crate) fn new(map: Inspection, mounts: Arc<MountSet>) -> Self {
        let dynamic_objects = DmoRender::new(Arc::clone(&mounts), Arc::clone(map.environment()));
        let visual_indicators = crate::visual_indicator_view::VisualIndicatorView::new(Arc::clone(&mounts));
        let vehicles = crate::vehicle_render::VehicleRender::new(
            Arc::clone(&mounts),
            Arc::clone(map.environment()),
        );
        Self {
            lighting: crate::render_irradiance::SkaterLighting::new(map.environment().default_irradiance),
            pedestrians: crate::pedestrian_render::PedestrianRender::new(Arc::clone(&mounts),Arc::clone(map.environment())),
            crowd: (0..CROWD_TARGETS)
                .map(|_| crate::pedestrian_render::PedestrianRender::crowd_target(Arc::clone(&mounts), Arc::clone(map.environment())))
                .collect(),
            map,
            mounts,
            bindings: Vec::new(),
            character_palette: None,
            party: Vec::new(),
            scene_characters: Vec::new(),
            marker_ui: None,
            audio: None,
            replay_watermarks:None,
            deck_occlusion: None,
            dynamic_objects,
            vehicles,
            editor: None,
            trick_guide_room:None,
            retained_trick_guide_scene:None,
            retained_world_scene: None,
            main_menu:None,
            retained_main_scene:None,
            viewer: None,
            frontend_ai: Vec::new(),
            main_menu_previous: None,
            retained: None,
            pending_draw: false,
            visual_indicators,
            particles: crate::particle_view::ParticleView::new(),
            movie: Default::default(),
        }
    }

    /// One game frame of audio: the tick's menu sound requests and the movies' `Audio.PlaySound`
    /// requests, then `AudioSystem::Update`. Requests are drained even without an output device.
    /// One audio frame: the front end's sounds and music player calls, then the update. Returns
    /// the music player's song and playlist index afterwards.
    pub(crate) fn update_audio(&mut self, dt: f32, state: skate2_audio::system::GameAudioState, world: Option<crate::game_audio::AudioWorldInput>,
        menu_sounds: Vec<skate2_core::ui::FeSoundRequest>, playlist: (&[u32], i32),
        music: Vec<skate2_core::ui::music_player::MusicRequest>) -> Option<(u32, i32)> {
        let movie_sounds = self.marker_ui.as_mut().map(crate::marker_ui::MarkerUi::take_sounds).unwrap_or_default();
        let audio = self.audio.as_mut()?;
        if audio.exporting(){return None;}
        for request in menu_sounds.into_iter().chain(movie_sounds.into_iter()) {
            audio.request(request);
        }
        audio.music_player(playlist, music);
        audio.update(dt, state, world);
        Some(audio.music_player_song())
    }

    /// The song the music system's `UpdateChyron` displayed this update, when audio is running.
    pub(crate) fn take_audio_chyron(&self) -> Option<u32> {
        self.audio.as_ref().and_then(crate::game_audio::GameAudio::take_chyron)
    }
    pub(crate) fn begin_export_audio(&mut self) {if let Some(audio)=&mut self.audio {audio.begin_export();}}
    pub(crate) fn end_export_audio(&mut self) {if let Some(audio)=&mut self.audio {audio.end_export();}}
    pub(crate) fn export_audio_frame(&mut self,state:skate2_audio::system::GameAudioState,world:Option<crate::game_audio::AudioWorldInput>)->Vec<f32> {
        self.audio.as_mut().map_or_else(||vec![0.;crate::media_encoder::AUDIO_FRAMES*2],|audio|audio.export_frame(state,world))
    }

    /// The music system's song catalogue, when audio is running.
    pub(crate) fn music_player_song_ids(&self) -> Option<Vec<u32>> {
        self.audio.as_ref().map(crate::game_audio::GameAudio::music_player_song_ids)
    }

    /// The audio system's `IAudioInterface`, when audio is running.
    pub(crate) fn audio_interface(&self) -> Option<std::sync::Arc<dyn skate2_core::nis::audio::AudioInterface>> {
        self.audio.as_ref().map(crate::game_audio::GameAudio::audio_interface)
    }

    /// `IAudioInterface::PlayNISSfx` for each id the NIS's audio effect channel fired.
    /// `AptMovieIntegration::Render` with the front end's requests; the scene draws its result.
    pub(crate) fn update_movie(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene, requests: Vec<skate2_core::ui::movie_player::MovieRequest>,
        foreground: bool, force_background:bool) -> skate2_core::ui::movie_player::MoviePresentation {
        let (presentation, draw) = self.movie.render(requests, foreground, &self.mounts, self.audio.as_ref(), renderer);
        scene.movie = if force_background { Some(crate::gpu_renderer::movie::MovieDraw {
            foreground:false,background:true,video:draw.and_then(|d|d.video),
        }) } else { draw };
        presentation
    }

    pub(crate) fn play_nis_sfx(&self, ids: Vec<i32>) {
        let Some(audio) = self.audio.as_ref() else { return };
        for id in ids {
            audio.play_nis_sfx(id);
        }
    }

    pub(crate) fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuScene> {
        self.marker_ui = Some(crate::marker_ui::MarkerUi::load(&self.mounts, renderer)?);
        self.audio = match crate::game_audio::GameAudio::start(&self.mounts) {
            Ok(audio) => Some(audio),
            Err(error) => {
                eprintln!("audio disabled: {error}");
                None
            }
        };
        let parameters=crate::gpu_renderer::final_pass::FisheyeParameters::load(&self.mounts)?;
        let vignette=self.map.vignette_texture(renderer)?;
        let mut scene=self.map.initialize(renderer)?;
        scene.final_blit=Some((parameters,vignette));
        scene.postfx_overlay=Some(self.map.xray_texture(renderer)?);
        // SetWatermarkTexture82828610 retains a global registry pointer. Keep
        // these boot textures while the city worker owns the mutable map/cache.
        self.replay_watermarks=Some([self.map.postfx_texture(renderer,"logo_watermark.Texture")?,
            self.map.postfx_texture(renderer,"logo_watermark_pro.Texture")?]);
        self.deck_occlusion = self.map.postfx_texture(renderer, "board_occlusion.Texture")
            .inspect_err(|error| eprintln!("deck AO boxes off: {error}")).ok();
        Ok(scene)
    }

    pub(crate) fn update_editor(&mut self, renderer:&GpuRenderer,scene:&mut GpuScene,
        now:Instant,character:&mut skate2_core::edit_skater_character::EditSkaterCharacter,
        male:bool,body_key:u64,camera_key:u64,near_plane:f32)->RenderResult<()> {
        if self.editor.is_none() {
            let mut editor=crate::edit_skater_room::EditSkaterRoomView::load(&self.mounts,male,now)?;
            let mut room_scene=editor.initialize(renderer)?;
            // Prepare a complete room before replacing the visible scene.
            editor.update(renderer,&mut room_scene,&self.mounts,now,character,male,body_key,camera_key,near_plane)?;
            self.retained_world_scene=Some(std::mem::replace(scene,room_scene));
            self.editor=Some(editor);
            return Ok(());
        }
        self.editor.as_mut().ok_or("CAS room owner missing")?.update(
            renderer,scene,&self.mounts,now,character,male,body_key,camera_key,near_plane)
    }

    pub(crate) fn close_editor(&mut self,scene:&mut GpuScene) {
        if let Some(world)=self.retained_world_scene.take(){*scene=world;}
        self.editor=None;
    }
    pub(crate) fn close_main_menu(&mut self,scene:&mut GpuScene)->RenderResult<()>{
        if let Some(world)=self.retained_main_scene.take(){*scene=world;}
        self.main_menu=None;
        self.viewer=None;
        self.frontend_ai.clear();
        self.main_menu_previous=None;
        if let Some(ui)=self.marker_ui.as_mut(){ui.close_main_menu()?;}
        Ok(())
    }
    pub(crate) fn update_main_menu(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,now:Instant,
        focus:[f32;3],frame:&skate2_core::ui::main_menu::Frame,freeskate_states:&[u32],party_play_states:&[u32],
        viewer:&[skate2_core::SelectedSceneCharacterVisual<'_>],
        ai:&[skate2_core::SelectedSkaterVisual<'_>])->RenderResult<(bool,bool,bool)>{
        if self.main_menu.is_none(){
            let presentation=crate::load_runtime_presentation_scene_at(&self.mounts,"frontend",focus)?;
            let textures=crate::load_runtime_texture_scene_at(&self.mounts,"frontend",focus)?;
            let mut menu=Inspection::from_assets(&self.mounts,presentation,textures,
                crate::render_environment::EnvironmentLighting::load(&self.mounts)?)?;
            let mut next=menu.initialize(renderer)?;
            let vignette=menu.vignette_texture(renderer)?;
            next.postfx_overlay=Some(menu.xray_texture(renderer)?);
            next.final_blit=Some((crate::gpu_renderer::final_pass::FisheyeParameters::load(&self.mounts)?,vignette));
            self.retained_main_scene=Some(std::mem::replace(scene,next));
            self.main_menu=Some(menu);
        }
        let camera=frame.camera;
        let clip=window_clip_from_world(frame.clip,renderer.dimensions());
        let dt = self.main_menu_previous.map_or(0., |previous| now.saturating_duration_since(previous).as_secs_f32());
        let menu=self.main_menu.as_mut().expect("frontend scene prepared");
        let position=[camera[12],camera[13],camera[14]];
        if std::env::var_os("SKATE2_TRACE_FRONTEND_CAMERA").is_some() {
            eprintln!("frontend camera {:?} at {position:?} looking {:?}",frame.screen,[camera[8],camera[9],camera[10]]);
        }
        menu.set_frontend_world_view(clip,position,
            std::array::from_fn(|j|std::array::from_fn(|i|camera[j*4+i])),dt);
        let (skater_hips,skater_bounds)=ai.first()
            .map_or((position,[position;2]),|skater|(skater.hips_position,skater.world_bounds));
        menu.set_frontend_shadow_casters(crate::render_shadows::ShadowCasterInputs{
            camera_position:position,camera_forward:[camera[8],camera[9],camera[10]],skater_hips,skater_bounds});
        menu.frame(renderer,scene,now)?;
        let particle_camera = skate2_core::world_particles::ParticleCamera {
            right: [camera[0], camera[1], camera[2]],
            up: [camera[4], camera[5], camera[6]],
            forward: [camera[8], camera[9], camera[10]],
            position,
        };
        let relative = CameraRelativeClip::new(&clip, position);
        let delta = menu.animation_delta();
        scene.particles =
            self.particles.frame(renderer, &self.mounts, particle_camera, &relative, delta, None, menu.emitter_waypoints());
        if let Some((_,postfx)) = scene.postfx.as_mut() {
            // MainMenuOnlineOptions holds the exposure from OnEnter to OnLeave, which the Skate.Reel
            // pages and FE states 68 and 69 run.
            postfx.fe_online_menu = frame.skate_reel.as_ref().is_some_and(|reel| reel.page == skate2_core::ui::skate_reel::Page::Online)
                && frame.online_match.is_none();
            // The frontend scene's take drives the ICE path as a NIS's does.
            ice_postfx(postfx, &frame.ice_postfx, 0., frame.planes);
        }
        // The viewer's objects lead the front end scene's objects; the AI skaters' follow.
        let viewer_count = self.viewer.as_ref().map_or(0, |objects| objects.objects.len());
        let mut ai_prior = std::mem::take(&mut scene.objects);
        scene.objects = ai_prior.drain(..viewer_count.min(ai_prior.len())).collect();
        self.update_viewer(renderer,scene,now,frame,viewer)?;
        self.update_frontend_ai(renderer,scene,frame,dt,ai_prior,ai)?;
        self.marker_ui.as_mut().ok_or("frontend UI absent")?.update_main_menu(renderer,scene,now,frame,freeskate_states,party_play_states)
    }
    /// The front end's AI skaters, the `SkaterPresEntity`s of the skaters the menu's world
    /// simulates: lit at their hips by the front end scene's probes as the viewer's skater is,
    /// faded and ghosted by their own `DoSimRestore`, casting by their hips and `m_BB`. A changed
    /// roster or recipe builds new objects.
    fn update_frontend_ai(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene,
        frame: &skate2_core::ui::main_menu::Frame, dt: f32, prior: Vec<GpuObject>,
        skaters: &[skate2_core::SelectedSkaterVisual<'_>],
    ) -> RenderResult<()> {
        let Self { main_menu, frontend_ai, mounts, .. } = self;
        let menu = main_menu.as_mut().ok_or("frontend scene absent")?;
        let clip = window_clip_from_world(frame.clip, renderer.dimensions());
        let camera = CameraRelativeClip::new(&clip, [frame.camera[12], frame.camera[13], frame.camera[14]]);
        let retained = frontend_ai.len() == skaters.len()
            && frontend_ai.iter().zip(skaters).all(|(objects, skater)| objects.palette == skater.palette.identity());
        if !retained {
            frontend_ai.clear();
        }
        let mut prior = if retained { prior } else { Vec::new() }.into_iter();
        for (index, skater) in skaters.iter().enumerate() {
            let probe = menu.light_probe_at(skater.hips_position);
            let ghost = CharacterGhostState { opacity: skater.opacity, ghosted: skater.ghosted };
            let materials = skater.materials.lock().map_err(|_| "frontend AI skater materials lock poisoned")?;
            if retained {
                let objects = &mut frontend_ai[index];
                objects.lighting.update_frontend(probe, dt);
                for binding in &objects.bindings {
                    let mut object = prior.next().ok_or("frontend AI skater object absent")?;
                    selected_character_material::update_object(
                        renderer, binding, &mut object, skater.palette, &materials, &camera, ghost,
                        menu.environment(), menu.frame_environment(), objects.lighting.coefficients,
                    )?;
                    renderer.set_character_shadow_bounds(&mut object, skater.hips_position, skater.world_bounds)?;
                    scene.objects.push(object);
                }
            } else {
                let mut lighting = crate::render_irradiance::SkaterLighting::new(menu.environment().default_irradiance);
                lighting.update_frontend(probe, dt);
                let created = selected_character_material::create_objects(
                    renderer, mounts, skater.palette, &materials, &camera, ghost,
                    menu.environment(), menu.frame_environment(), lighting.coefficients,
                )?;
                let mut bindings = Vec::new();
                for (binding, mut object) in created {
                    renderer.set_character_shadow_bounds(&mut object, skater.hips_position, skater.world_bounds)?;
                    bindings.push(binding);
                    scene.objects.push(object);
                }
                frontend_ai.push(PartySkaterObjects { palette: skater.palette.identity(), bindings, lighting });
            }
        }
        Ok(())
    }
    /// The FreeSkate viewer's skater, `MVManager`'s `SkaterPresEntity` (a NIS character's entity
    /// type): lit at its hips by the front end scene's probes as the CAS room's skater is, at full
    /// opacity; another marquee builds new objects.
    fn update_viewer(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene, now: Instant,
        frame: &skate2_core::ui::main_menu::Frame, viewer: &[skate2_core::SelectedSceneCharacterVisual<'_>],
    ) -> RenderResult<()> {
        let dt = self.main_menu_previous.map_or(0., |previous| now.saturating_duration_since(previous).as_secs_f32());
        self.main_menu_previous = Some(now);
        let Some(character) = viewer.first() else {
            self.viewer = None;
            scene.objects.clear();
            return Ok(());
        };
        let materials = character.materials.ok_or("free skate viewer without skater materials")?;
        let menu = self.main_menu.as_mut().ok_or("frontend scene absent")?;
        let clip = window_clip_from_world(frame.clip, renderer.dimensions());
        let camera = CameraRelativeClip::new(&clip, [frame.camera[12], frame.camera[13], frame.camera[14]]);
        let ghost = CharacterGhostState { opacity: 1.0, ghosted: false };
        let identity = character.palette.identity();
        let probe = menu.light_probe_at(character.hips_position);
        match self.viewer.as_mut().filter(|objects| objects.palette == identity) {
            Some(objects) => {
                objects.lighting.update_frontend(probe, dt);
                let SceneObjects::Skater(bindings) = &objects.objects else {
                    return Err("free skate viewer objects changed kind".into());
                };
                if bindings.len() != scene.objects.len() {
                    return Err("free skate viewer object count".into());
                }
                for (binding, object) in bindings.iter().zip(&mut scene.objects) {
                    selected_character_material::update_object(
                        renderer, binding, object, character.palette, materials, &camera, ghost,
                        menu.environment(), menu.frame_environment(), objects.lighting.coefficients,
                    )?;
                }
            }
            None => {
                let mut lighting = crate::render_irradiance::SkaterLighting::new(menu.environment().default_irradiance);
                lighting.update_frontend(probe, dt);
                let created = selected_character_material::create_objects(
                    renderer, &self.mounts, character.palette, materials, &camera, ghost,
                    menu.environment(), menu.frame_environment(), lighting.coefficients,
                )?;
                scene.objects.clear();
                let mut bindings = Vec::new();
                for (binding, object) in created {
                    bindings.push(binding);
                    scene.objects.push(object);
                }
                self.viewer = Some(SceneCharacterObjects { palette: identity, objects: SceneObjects::Skater(bindings), lighting });
            }
        }
        Ok(())
    }
    pub(crate) fn update_editor_ui(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,
        now:Instant,view:&skate2_core::ui::edit_skater::EditSkaterView)->RenderResult<(bool,bool)> {
        self.marker_ui.as_mut().ok_or("marker UI not initialized")?
            .update_editor(renderer,scene,now,view,&self.mounts)
    }

    /// Frontend-only presentation; retain completed world, palette and camera.
    pub(crate) fn update_pause(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        pause: &skate2_core::ui::pause_menu::PauseMenuSnapshot,
        map: &skate2_core::ui::map_menu::MapMenuSnapshot,
        locations: &[skate2_core::ui::map_menu::MapLocation],
        player_pose: Option<([f32; 2], [f32; 2])>,
        career: Option<&skate2_core::CareerHud>,
    ) -> RenderResult<()> {
        self.marker_ui.as_mut().ok_or("marker UI not initialized")?
            .update_pause(renderer, scene, now, pause, map, locations, player_pose, career)
    }

    pub(crate) fn prepare_replay_capture(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,watermark:u8)->RenderResult<()> {
        scene.capture_watermark=None;
        if watermark!=0 {
            let position=scene.postfx.as_ref().ok_or("capture PostFX assets absent")?.0.watermark_position;
            let textures=self.replay_watermarks.as_ref().ok_or("replay watermark textures not initialized")?;
            scene.capture_watermark=renderer.replay_watermark(&textures[usize::from(watermark==2)],position,watermark==2)?;
        }
        Ok(())
    }

    pub(crate) fn media_scene_ready(&self)->bool {self.map.media_scene_ready()}

    /// One completed tick: its streaming, scene characters and front end, and its frame.
    /// Its world inputs are kept for the frames between this tick and the next, which
    /// `update_frame` draws from the presentation's restored camera and skaters.
    #[expect(clippy::too_many_arguments, reason = "one completed tick's front-end inputs")]
    pub(crate) fn update(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        visual: &mut SelectedVisualSnapshot<'_>,
        pause: &skate2_core::ui::pause_menu::PauseMenuSnapshot,
        map: &skate2_core::ui::map_menu::MapMenuSnapshot,
        locations: &[skate2_core::ui::map_menu::MapLocation],
        player_pose: Option<([f32; 2], [f32; 2])>,
        career: Option<&skate2_core::CareerHud>,
        restore: Option<&PresentationRestorer>,
        replay: Option<&PresentationRestorer>,
    ) -> RenderResult<()> {
        scene.out_of_boundary_amount=visual.out_of_boundary_amount;
        let identity=visual.palette.identity();
        if self.character_palette.as_ref().is_some_and(|old|old!=&identity) {
            if scene.objects.len()<self.bindings.len(){return Err("replaced skater object count".into());}
            scene.objects.drain(..self.bindings.len());
            self.bindings.clear();
            self.lighting = crate::render_irradiance::SkaterLighting::new(self.map.environment().default_irradiance);
        }
        self.character_palette=Some(identity);
        let local = RetainedSkater {
            palette: visual.palette.clone(),
            materials: Arc::clone(&visual.materials),
            hips_position: visual.skater_hips_position,
            bounds: visual.skater_world_bounds,
            ghost: CharacterGhostState { opacity: visual.skater_opacity, ghosted: visual.skater_ghosted },
            entity_type: visual.skater_entity_type,
            players_recipe: true,
            name_tag: None,
            lobby_ghost: false,
        };
        let party: Vec<RetainedSkater> = visual
            .party_skaters
            .iter()
            .map(|skater| RetainedSkater {
                palette: skater.palette.clone(),
                materials: Arc::clone(&skater.materials),
                hips_position: skater.hips_position,
                bounds: skater.world_bounds,
                ghost: CharacterGhostState { opacity: skater.opacity, ghosted: skater.ghosted },
                entity_type: skater.entity_type,
                players_recipe: skater.players_recipe,
                name_tag: skater.name_tag.clone(),
                lobby_ghost: skater.lobby_ghost,
            })
            .collect();
        let shadow = visual.local_skater_shadow;
        let shadow_skater = if (local.hips_position, local.bounds) == shadow {
            ShadowSkater::Local
        } else {
            party
                .iter()
                .position(|skater| (skater.hips_position, skater.bounds) == shadow)
                .map_or(ShadowSkater::Fixed(shadow), ShadowSkater::Party)
        };
        self.retained = Some(RetainedFrame {
            simulation_tick: visual.simulation_tick,
            hom_visual: visual.hom_visual,
            traffic_light_status: visual.traffic_light_status,
            world_painter: visual.world_painter.clone(),
            nis: career.map(|career| career.nis.clone()),
            challenge_fade: career.and_then(|career| career.fade),
            result_info: career.map(|career| career.result_info).unwrap_or_default(),
            camera: CameraView {
                clip_from_world: visual.provisional_clip_from_world,
                position: visual.camera_world_position,
                right: visual.camera_right,
                up: visual.camera_up,
                forward: visual.camera_forward,
                planes: visual.camera_planes,
                update_counter: visual.camera_update_counter,
            },
            local,
            party,
            shadow_skater,
            dynamic_objects: std::mem::take(&mut visual.dynamic_objects),
            vehicles: std::mem::take(&mut visual.vehicles),
            pedestrians: std::mem::take(&mut visual.pedestrians),
            crowd: std::mem::take(&mut visual.crowd),
            chase_arrows: std::mem::take(&mut visual.chase_arrows),
            visual_indicators: visual.visual_indicators.take(),
            in_replay: replay.is_some(),
        });
        use skate2_core::diagnostics::frame_profile as profile;
        // A tick the presentation restores draws once its clock has stepped to this
        // frame's moment (`update_frame`); a NIS's scene characters draw with their tick.
        if restore.is_some_and(PresentationRestorer::is_running)
            && visual.scene_characters.is_empty()
            && self.scene_characters.is_empty()
        {
            let span = profile::span(profile::VIEW_CITY_STREAMING);
            self.map.update_city_streaming(renderer, scene, visual)?;
            drop(span);
            self.pending_draw = true;
        } else {
            // A replay-controlled NIS draws the presentations it restored from the recorded
            // frames, under the scene's camera.
            self.draw(renderer, scene, now, replay, Some(visual))?;
        }
        let _span = profile::span(profile::VIEW_MARKER_UI);
        self.marker_ui
            .as_mut()
            .ok_or("marker UI not initialized")?
            .update(renderer, scene, now, visual.marker_hud,
                &visual.scoring_hud_frames, visual.scoring_hud_tuning, pause, map, locations, player_pose,
                &visual.minimap, visual.minimap_icons, career)
    }

    pub(crate) fn begin_ui_geometry_batch(&mut self) {
        if let Some(ui) = &mut self.marker_ui { ui.begin_geometry_batch(); }
    }
    pub(crate) fn finish_ui_geometry_batch(&mut self, renderer: &GpuRenderer, scene: &mut GpuScene) -> RenderResult<()> {
        if let Some(ui) = &mut self.marker_ui { ui.finish_geometry_batch(renderer, scene)?; }
        Ok(())
    }

    /// A frame between ticks: the last tick's world drawn again from the presentation's
    /// restored camera and skaters.
    pub(crate) fn update_frame(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        restore: &PresentationRestorer,
    ) -> RenderResult<()> {
        self.draw(renderer, scene, now, Some(restore), None)
    }

    /// Whether frames between ticks can draw the last tick's world: a NIS's scene
    /// characters are drawn only with their tick.
    pub(crate) fn draws_between_ticks(&self) -> bool {
        self.retained.is_some() && self.scene_characters.is_empty()
    }

    /// Whether the last tick left its frame to `update_frame`.
    pub(crate) fn take_pending_draw(&mut self) -> bool {
        std::mem::take(&mut self.pending_draw)
    }

    /// One frame of the world and its moving objects, from the restored camera and
    /// skaters where the presentation has them, else as the last tick left them. `tick`
    /// is the completed tick this frame first shows: its streaming and scene characters.
    fn draw(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        restore: Option<&PresentationRestorer>,
        tick: Option<&SelectedVisualSnapshot<'_>>,
    ) -> RenderResult<()> {
        let retained = self.retained.take().ok_or("frame without a completed tick")?;
        let result = self.draw_retained(renderer, scene, now, restore, tick, &retained);
        self.retained = Some(retained);
        result
    }

    #[expect(clippy::too_many_lines, reason = "one frame's ordered map, skater and object passes")]
    fn draw_retained(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        restore: Option<&PresentationRestorer>,
        tick: Option<&SelectedVisualSnapshot<'_>>,
        retained: &RetainedFrame,
    ) -> RenderResult<()> {
        let view = restore.and_then(PresentationRestorer::camera).unwrap_or(retained.camera);
        scene.stereo_planes=Some(view.planes);
        let local = retained.local.draw(restore);
        let party: Vec<SkaterDraw<'_>> = retained.party.iter().map(|skater| skater.draw(restore)).collect();
        let shadow = match retained.shadow_skater {
            ShadowSkater::Local => (local.hips_position, local.bounds),
            ShadowSkater::Party(index) => party
                .get(index)
                .map_or((local.hips_position, local.bounds), |skater| (skater.hips_position, skater.bounds)),
            ShadowSkater::Fixed(shadow) => shadow,
        };
        let scene_objects: usize = self.scene_characters.iter().map(|c| c.objects.len()).sum();
        let party_objects: usize = self.party.iter().map(|p| p.bindings.len()).sum();
        let skater_objects = self.bindings.len() + party_objects;
        if scene.objects.len()<skater_objects+scene_objects{return Err("live visual character object count".into());}
        let prior_pedestrians=scene.objects.split_off(skater_objects+scene_objects);
        let prior_scene_characters=scene.objects.split_off(skater_objects);
        let prior_party=scene.objects.split_off(self.bindings.len());
        if scene.objects.len() != self.bindings.len() {
            return Err("live visual object count differs from retained bindings".into());
        }
        let clip_from_world = window_clip_from_world(view.clip_from_world, renderer.dimensions());
        // The map rebases the same matrix at the same `g_vViewPos`.
        let camera = CameraRelativeClip::new(&clip_from_world, view.position);
        self.map.set_world_shader_state(retained.traffic_light_status, [view.right, view.up, view.forward]);
        self.map.set_external_view(
            clip_from_world,
            view.position,
            retained.world_painter.clone(),
            ShadowCasterInputs {
                camera_position: view.position,
                camera_forward: view.forward,
                skater_hips: shadow.0,
                skater_bounds: shadow.1,
            },
            restore
                .and_then(PresentationRestorer::shown_tick)
                .unwrap_or((retained.simulation_tick, 0.0)),
        );
        use skate2_core::diagnostics::frame_profile as profile;
        if let Some(visual) = tick {
            let span = profile::span(profile::VIEW_CITY_STREAMING);
            self.map.update_city_streaming(renderer, scene, visual)?;
            drop(span);
        }
        let span = profile::span(profile::VIEW_MAP_FRAME);
        self.map.frame(renderer, scene, now)?;
        drop(span);
        if let Some((assets, postfx)) = scene.postfx.as_mut() {
            postfx.camera_planes = Some(view.planes);
            let result = retained.result_info;
            if retained.hom_visual.is_some_and(|bail| bail.tazer) {
                // VDWriter827fc768: TazeIsOn && HoM_BailInProgress;
                // VisualFX_Animation puts this before the HoM/results/fade branches.
                postfx.tazer = true;
                postfx.hom = None;
                postfx.bloom = [postfx.bloom[0], postfx.bloom[1], 0., 1., 1.];
            } else if let Some(bail) = retained.hom_visual.filter(|bail| bail.effect_enabled) {
                assets.apply_hom(postfx, bail, view.planes);
            } else if result.success.is_some() || result.failed.is_some() {
                challenge_result(postfx);
            } else if let Some(fade) = retained.challenge_fade {
                challenge_fade(postfx, fade);
            }
            if let Some(nis) = retained.nis.as_ref() {
                nis_postfx(postfx, nis, view.planes);
            }
        }
        let span = profile::span(profile::VIEW_CHARACTER);
        self.lighting.update_tick(self.map.light_probe_at(local.hips_position), retained.simulation_tick);
        {
            let materials = local.materials.lock().map_err(|_| "skater materials lock poisoned")?;
            if self.bindings.is_empty() {
                let created = selected_character_material::create_objects(
                    renderer,
                    &self.mounts,
                    local.palette,
                    &materials,
                    &camera,
                    local.ghost,
                    self.map.environment(), self.map.frame_environment(), self.lighting.coefficients,
                )?;
                if created.is_empty() {
                    return Err("selected character material admitted no GPU objects".into());
                }
                for (binding, mut object) in created {
                    renderer.set_character_shadow_bounds(&mut object, local.hips_position, local.bounds)?;
                    self.bindings.push(binding);
                    scene.objects.push(object);
                }
            } else {
                for (binding, object) in self.bindings.iter().zip(&mut scene.objects) {
                    selected_character_material::update_object(
                        renderer, binding, object, local.palette, &materials, &camera, local.ghost,
                        self.map.environment(), self.map.frame_environment(), self.lighting.coefficients,
                    )?;
                    renderer.set_character_shadow_bounds(object, local.hips_position, local.bounds)?;
                }
            }
        }
        self.update_party_skaters(renderer, scene, prior_party, &party, retained.simulation_tick, &camera)?;
        let mut scene_vehicles = Vec::new();
        match tick {
            Some(visual) => self.update_scene_characters(
                renderer, scene, prior_scene_characters, visual, &camera, &mut scene_vehicles,
            )?,
            None => scene.objects.extend(prior_scene_characters),
        }
        // RenderMan::UpdateVisibleTypes (0x82839be0) while a scene exists;
        // RenderView::DetermineVisibilityFromGameState (0x82829698) draws an
        // entity only when the mask covers its whole type, and in a photo
        // outro all but the photographers and maybe the other skaters. The
        // other entities keep updating, as the native only culls them.
        let nis = retained.nis.as_ref();
        let drawn = |kind: u32, model: u64, players_recipe: bool| nis.is_none_or(|nis| nis.draws(kind, model, players_recipe));
        for object in &mut scene.objects[..self.bindings.len()] {
            object.hidden = !drawn(local.entity_type, 0, local.players_recipe);
        }
        let mut at = self.bindings.len();
        for (objects, skater) in self.party.iter().zip(&party) {
            for object in &mut scene.objects[at..at + objects.bindings.len()] {
                object.hidden = !drawn(skater.entity_type, 0, skater.players_recipe);
            }
            at += objects.bindings.len();
        }
        // PostFX::RenderAOBoxes 0x828465b0 after the uber pass: a box under each skater the
        // game state draws (`CalculateDeckShadowMatrix` 0x82845f98), `board_occlusion.Texture`.
        let camera_rows = [view.right, view.up, view.forward, view.position];
        let deck_boxes: Vec<_> = std::iter::once(&local)
            .chain(party.iter())
            .filter(|skater| !skater.lobby_ghost && drawn(skater.entity_type, 0, skater.players_recipe))
            .filter_map(|skater| crate::gpu_renderer::deck_ao::deck_box(&skater.palette.deck_world()?, &clip_from_world, camera_rows))
            .collect();
        scene.deck_ao = self.deck_occlusion.clone().map(|texture| crate::gpu_renderer::deck_ao::DeckAoFrame {
            texture,
            boxes: deck_boxes,
            fov: crate::gpu_renderer::deck_ao::fov(&clip_from_world, view.right, view.up),
        });
        // SceneRenderView::Render 0x828508f8: DrawName for each entity SceneRenderView::Cull
        // 0x828500a8 kept, none in a replay.
        let tags: Vec<crate::marker_ui::online_names::NameTag<'_>> = party
            .iter()
            .filter(|skater| {
                !retained.in_replay
                    && drawn(skater.entity_type, 0, skater.players_recipe)
                    && crate::gpu_renderer::world_box_in_view(&clip_from_world, skater.bounds)
            })
            .filter_map(|skater| {
                skater.name_tag.map(|tag| crate::marker_ui::online_names::NameTag { name: &tag.name, index: tag.index, hips: skater.hips_position })
            })
            .collect();
        if let Some(ui) = self.marker_ui.as_ref() {
            let names = crate::marker_ui::online_names::NameView { clip_from_world: view.clip_from_world, camera: view.position };
            ui.update_online_names(renderer, scene, &tags, &names)?;
            // FrontEndManager::Render2D 0x82633be0: ChaseArrowMan::Render while no HUDManager2
            // visibility flag hides the HUD. Each arrow's pedestrian is where this frame draws it.
            let quads: Vec<_> = if ui.chase_arrows_shown() {
                retained.chase_arrows.iter().filter_map(|arrow| {
                    let visual = retained.pedestrians.iter().find(|visual| visual.entity_id == arrow.pedestrian)?;
                    let position = restore.and_then(|restore| restore.pedestrian(visual.entity_id))
                        .map_or(visual.position, |transform| visual.placement_at(transform).position);
                    let placement = skate2_core::ui::chase_arrows::placement(position, shadow.0, &view.clip_from_world);
                    skate2_core::ui::chase_arrows::quad(arrow.state, &placement, view.forward)
                }).collect()
            } else {
                Vec::new()
            };
            ui.update_chase_arrows(renderer, scene, &quads)?;
        }
        drop(span);
        let span = profile::span(profile::VIEW_DMO);
        self.dynamic_objects.update(
            renderer,
            &mut scene.dynamic_objects,
            &retained.dynamic_objects,
            view.position,
            restore,
        )?;
        if !drawn(ENTITY_DYNAMIC_OBJECT, 0, false) {
            scene.dynamic_objects.clear();
        }
        let dynamic_objects = scene.dynamic_objects.len();
        drop(span);
        let span = profile::span(profile::VIEW_VEHICLES);
        self.vehicles.update(
            renderer,
            &mut scene.dynamic_objects,
            &retained.vehicles,
            view.position,
            restore,
        )?;
        if !drawn(ENTITY_VEHICLE, 0, false) {
            scene.dynamic_objects.truncate(dynamic_objects);
        }
        scene.dynamic_objects.extend(scene_vehicles);
        drop(span);
        let span = profile::span(profile::VIEW_PEDESTRIANS);
        self.pedestrians.update(renderer,&mut scene.objects,prior_pedestrians,&retained.pedestrians,
            &camera,self.map.frame_environment(),restore,|visual|drawn(ENTITY_PEDESTRIAN,visual.model_key,false))?;
        let prior_crowd = scene.crowd.take_targets();
        scene.crowd = crowd_frame(&mut self.crowd, renderer, prior_crowd, &retained.crowd, &camera, view.forward,
            self.map.frame_environment())?;
        drop(span);
        // The ambient effect's anchor: the local skater's audio centre of mass plus its hips' up
        // axis (`Update` 0x827ee4b4..d8), here the hips a metre up.
        let hips = retained.local.hips_position;
        scene.particles = self.particles.frame(
            renderer,
            &self.mounts,
            skate2_core::world_particles::ParticleCamera {
                right: view.right,
                up: view.up,
                forward: view.forward,
                position: view.position,
            },
            &camera,
            self.map.animation_delta(),
            Some([hips[0], hips[1] + 1.0, hips[2]]),
            self.map.emitter_waypoints(),
        );
        scene.visual_indicators = match retained.visual_indicators.as_ref() {
            Some(indicators) => self.visual_indicators.frame(
                renderer,
                &self.mounts,
                indicators,
                &view,
                &clip_from_world,
                &camera,
                restore.and_then(PresentationRestorer::shown_tick).unwrap_or((retained.simulation_tick, 0.0)),
                retained.nis.as_ref(),
            ),
            None => None,
        };
        // Opt-in diagnostics only; leave normal playtests free of frame logging.
        static PED_TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static PED_TRACE_FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
        if tick.is_some() && *PED_TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some()) {
            let frame = u64::from(view.update_counter);
            if frame % 300 == 0 && PED_TRACE_FRAME.swap(frame, std::sync::atomic::Ordering::Relaxed) != frame {
                std::eprintln!("pedestrian presentation frame={frame} submitted={} meshes={} entities={:?}",
                    retained.pedestrians.len(), scene.objects.len().saturating_sub(self.bindings.len()),
                    retained.pedestrians.iter().map(|p| (p.entity_id, p.position)).collect::<Vec<_>>());
            }
        }
        Ok(())
    }
}

/// A skater as the last tick drew it, which the frames after it draw again.
struct RetainedSkater {
    palette: CharacterPalette,
    materials: Arc<Mutex<CharacterMaterialAdmission>>,
    hips_position: [f32; 3],
    bounds: [[f32; 3]; 2],
    ghost: CharacterGhostState,
    entity_type: u32,
    players_recipe: bool,
    /// Its online name while `mDrawPlayerName` is set.
    name_tag: Option<skate2_core::SkaterNameTag>,
    /// A challenge's player watched from its pre-game lobby: its tick's ghost opacity holds,
    /// and it casts no deck shadow.
    lobby_ghost: bool,
}

impl RetainedSkater {
    /// The restored skater drawn with this palette, else the tick's.
    fn draw<'a>(&'a self, restore: Option<&'a PresentationRestorer>) -> SkaterDraw<'a> {
        match restore.and_then(|restore| restore.skater(&self.palette.identity())) {
            Some(restored) => SkaterDraw {
                palette: restored.palette,
                materials: &self.materials,
                hips_position: restored.hips_position,
                bounds: restored.bounds,
                ghost: if self.lobby_ghost { self.ghost } else { CharacterGhostState { opacity: restored.opacity, ghosted: restored.ghosted } },
                entity_type: self.entity_type,
                players_recipe: self.players_recipe,
                name_tag: self.name_tag.as_ref(),
                lobby_ghost: self.lobby_ghost,
            },
            None => SkaterDraw {
                palette: &self.palette,
                materials: &self.materials,
                hips_position: self.hips_position,
                bounds: self.bounds,
                ghost: self.ghost,
                entity_type: self.entity_type,
                players_recipe: self.players_recipe,
                name_tag: self.name_tag.as_ref(),
                lobby_ghost: self.lobby_ghost,
            },
        }
    }
}

/// One skater as a frame draws it.
struct SkaterDraw<'a> {
    palette: &'a CharacterPalette,
    materials: &'a Arc<Mutex<CharacterMaterialAdmission>>,
    hips_position: [f32; 3],
    bounds: [[f32; 3]; 2],
    ghost: CharacterGhostState,
    entity_type: u32,
    players_recipe: bool,
    name_tag: Option<&'a skate2_core::SkaterNameTag>,
    lobby_ghost: bool,
}

/// The skater `PostFX::RenderShadows_CSM` 0x82855888 fits the cascades to.
#[derive(Clone, Copy)]
enum ShadowSkater {
    Local,
    Party(usize),
    /// Neither drawn skater: the tick's hips and bounds.
    Fixed(([f32; 3], [[f32; 3]; 2])),
}

/// The last tick's world inputs, which the frames after it draw again with the
/// restored camera and skaters.
struct RetainedFrame {
    simulation_tick: u32,
    hom_visual: Option<skate2_core::BailVisual>,
    traffic_light_status: [[f32; 4]; 2],
    world_painter: Result<skate2_core::WorldPainterKeys, String>,
    nis: Option<skate2_core::nis::manager::NisView>,
    /// The tick's Visual Director fade info (`VDReaderData::sFadeInfo`), in a career.
    challenge_fade: Option<f32>,
    /// The tick's Visual Director challenge success and failure info, in a career.
    result_info: skate2_core::challenge::vd_results::ResultInfo,
    /// The tick's own camera, drawn until the presentation restores one.
    camera: CameraView,
    local: RetainedSkater,
    party: Vec<RetainedSkater>,
    shadow_skater: ShadowSkater,
    dynamic_objects: Vec<skate2_core::SelectedDynamicObjectVisual>,
    vehicles: Vec<skate2_core::SelectedVehicleVisual>,
    pedestrians: Vec<skate2_core::SelectedPedestrianVisual>,
    /// The tick's `CrowdPresentation` targets and imposters.
    crowd: skate2_core::SelectedCrowdVisual,
    /// The tick's `ChaseArrowMan` arrows.
    chase_arrows: Vec<skate2_core::ui::chase_arrows::Arrow>,
    /// The tick's `VDReaderData` challenge indicator records, in a career.
    visual_indicators: Option<skate2_core::challenge::visual_director::VisualIndicatorFrame>,
    /// A replay draws the frame (`Presentation::IsInReplay`): no names.
    in_replay: bool,
}

/// `CrowdPresentation`'s six imposter targets.
const CROWD_TARGETS: usize = 6;

fn multiply(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    std::array::from_fn(|row| std::array::from_fn(|column| (0..4).map(|k| a[row][k] * b[k][column]).sum()))
}

/// `RenderCrowdTargets`' pedestrians, each in its slot under its cell's camera: its own
/// view-projection over the game camera's position, which `g_vViewPos` keeps through the
/// atlas pass. Then the imposters, `SyncAnim`'s `UpdateTransform` facing this frame's camera,
/// sized by their target (`i_matModelTarget`) and mapped into its cell (`i_matTexture`).
fn crowd_frame(
    slots: &mut [crate::pedestrian_render::PedestrianRender],
    renderer: &GpuRenderer,
    prior: Vec<crate::gpu_renderer::crowd::Target>,
    crowd: &skate2_core::SelectedCrowdVisual,
    camera: &CameraRelativeClip,
    camera_z: [f32; 3],
    environment: crate::gpu_renderer::FrameEnvironment,
) -> crate::gpu_renderer::RenderResult<crate::gpu_renderer::crowd::Frame> {
    let mut prior = prior.into_iter();
    let mut targets = Vec::new();
    for (index, slot) in slots.iter_mut().enumerate() {
        let previous = prior.next().map(|target| target.objects).unwrap_or_default();
        let target = crowd.targets.get(index);
        let visuals = target.and_then(|target| target.pedestrian.as_ref()).map(std::slice::from_ref).unwrap_or_default();
        let clip = target.map_or([[0.0; 4]; 4], |target| multiply(&target.camera.view, &target.camera.projection));
        let clip_from_world: [f32; 16] = std::array::from_fn(|index| clip[index / 4][index % 4]);
        let atlas = CameraRelativeClip::new(&clip_from_world, camera.camera);
        let mut objects = Vec::new();
        slot.update(renderer, &mut objects, previous, visuals, &atlas, environment, None, |_| true)?;
        if let Some(target) = target {
            targets.push(crate::gpu_renderer::crowd::Target { objects, scissor: target.scissor });
        }
    }
    let imposters: Vec<_> = crowd
        .imposters
        .iter()
        .filter_map(|imposter| {
            let target = crowd.targets.get(imposter.target)?;
            let mut rows = multiply(&target.camera.model, &skate2_core::imposter_transform(camera_z, imposter.position));
            for (lane, origin) in rows[3].iter_mut().zip(camera.camera) {
                *lane -= origin;
            }
            let texture = target.camera.texture;
            Some(crate::gpu_renderer::crowd::Imposter {
                rows,
                texture: [
                    [texture[0][0], texture[1][0], texture[3][0], 0.0],
                    [texture[0][1], texture[1][1], texture[3][1], 0.0],
                ],
                colours: imposter.colours,
            })
        })
        .collect();
    Ok(renderer.crowd_frame(targets, &imposters, camera, [environment.fog_k1, environment.fog_colour]))
}

/// `eEntTyp` values of the gameplay entities a NIS can leave out (the scene
/// characters are `eEntTypNISChar`, 0x822; the skater's type comes with
/// the snapshot, as a NIS may borrow it).
const ENTITY_PEDESTRIAN: u32 = 0x222;
const ENTITY_VEHICLE: u32 = 0x422;
const ENTITY_DYNAMIC_OBJECT: u32 = 0x42;

/// A later skater's `SkaterPresEntity` objects and its lighting.
struct PartySkaterObjects {
    palette: skate2_core::character::palette::CharacterPaletteIdentity,
    bindings: Vec<SelectedCharacterGpuBinding>,
    lighting: crate::render_irradiance::SkaterLighting,
}

/// A scene character's GPU objects and its lighting.
struct SceneCharacterObjects {
    palette: skate2_core::character::palette::CharacterPaletteIdentity,
    objects: SceneObjects,
    lighting: crate::render_irradiance::SkaterLighting,
}

/// A `SkaterPresEntity`'s skater material bindings, or a
/// `ColorizedSkaterPresEntity`'s living world or vehicle draws.
enum SceneObjects {
    Skater(Vec<SelectedCharacterGpuBinding>),
    Colorized(crate::pedestrian_render::ColorizedObjects),
    /// A Vehicle recipe's, drawn as dynamic objects.
    Vehicle(crate::vehicle_render::SceneVehicle),
}

impl SceneObjects {
    fn len(&self) -> usize {
        match self {
            Self::Skater(bindings) => bindings.len(),
            Self::Colorized(objects) => objects.len(),
            Self::Vehicle(_) => 0,
        }
    }
}

impl LiveSkatingView {
    /// The later skaters, drawn as the local skater is: each lit at its hips, ghosted and faded
    /// by its own `DoSimRestore`, casting by its own hips and `m_BB`. A skater keeps its objects
    /// and lighting while its palette stays; a new skater or recipe builds new objects.
    fn update_party_skaters(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        prior: Vec<GpuObject>,
        skaters: &[SkaterDraw<'_>],
        simulation_tick: u32,
        camera: &CameraRelativeClip,
    ) -> RenderResult<()> {
        let mut prior = prior.into_iter();
        let mut previous = Vec::with_capacity(self.party.len());
        for objects in self.party.drain(..) {
            let owned: Vec<GpuObject> = prior.by_ref().take(objects.bindings.len()).collect();
            if owned.len() != objects.bindings.len() {
                return Err("party skater object absent".into());
            }
            previous.push((objects, owned));
        }
        for skater in skaters {
            let probe = self.map.light_probe_at(skater.hips_position);
            let identity = skater.palette.identity();
            let materials = skater.materials.lock().map_err(|_| "party skater materials lock poisoned")?;
            if let Some(at) = previous.iter().position(|(objects, _)| objects.palette == identity) {
                let (mut objects, owned) = previous.remove(at);
                objects.lighting.update_tick(probe, simulation_tick);
                for (binding, mut object) in objects.bindings.iter().zip(owned) {
                    selected_character_material::update_object(
                        renderer, binding, &mut object, skater.palette, &materials, camera, skater.ghost,
                        self.map.environment(), self.map.frame_environment(), objects.lighting.coefficients,
                    )?;
                    renderer.set_character_shadow_bounds(&mut object, skater.hips_position, skater.bounds)?;
                    scene.objects.push(object);
                }
                self.party.push(objects);
            } else {
                let mut lighting = crate::render_irradiance::SkaterLighting::new(self.map.environment().default_irradiance);
                lighting.update_tick(probe, simulation_tick);
                let created = selected_character_material::create_objects(
                    renderer, &self.mounts, skater.palette, &materials, camera, skater.ghost,
                    self.map.environment(), self.map.frame_environment(), lighting.coefficients,
                )?;
                let mut bindings = Vec::new();
                for (binding, mut object) in created {
                    renderer.set_character_shadow_bounds(&mut object, skater.hips_position, skater.bounds)?;
                    bindings.push(binding);
                    scene.objects.push(object);
                }
                self.party.push(PartySkaterObjects { palette: skater.palette.identity(), bindings, lighting });
            }
        }
        Ok(())
    }

    /// The NIS scene characters (`SceneCharacterManager`'s renderables),
    /// each lit at its hips (`GetHipsPos`) and casting by its own hips and
    /// `m_BB` (`ShadowView::Cull`); a new set of characters builds new
    /// objects. They take the full opacity and are not ghosted.
    fn update_scene_characters(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        prior: Vec<GpuObject>,
        visual: &SelectedVisualSnapshot<'_>,
        camera: &CameraRelativeClip,
        vehicles: &mut Vec<crate::gpu_renderer::DynamicObjectDraw>,
    ) -> RenderResult<()> {
        let ghost = CharacterGhostState { opacity: 1.0, ghosted: false };
        let identities: Vec<_> = visual.scene_characters.iter().map(|c| c.palette.identity()).collect();
        let retained = identities.len() == self.scene_characters.len()
            && identities.iter().zip(&self.scene_characters).all(|(identity, objects)| *identity == objects.palette);
        if !retained {
            self.scene_characters.clear();
        }
        let mut prior = if retained { prior } else { Vec::new() }.into_iter();
        for (index, (character, identity)) in visual.scene_characters.iter().zip(identities).enumerate() {
            let materials = character.materials;
            let probe = self.map.light_probe_at(character.hips_position);
            let colorized = |irradiance| {
                character.colours.map(|colours| crate::pedestrian_render::ColorizedInputs {
                    colours,
                    irradiance,
                    position: character.hips_position,
                    bounds: character.world_bounds,
                })
            };
            if retained {
                let objects = &mut self.scene_characters[index];
                objects.lighting.update_tick(probe, visual.simulation_tick);
                let coefficients = objects.lighting.coefficients;
                match (&objects.objects, materials, colorized(coefficients)) {
                    (SceneObjects::Skater(bindings), Some(materials), _) => {
                        for binding in bindings {
                            let mut object = prior.next().ok_or("scene character object absent")?;
                            selected_character_material::update_object(
                                renderer, binding, &mut object, character.palette, materials, camera, ghost,
                                self.map.environment(), self.map.frame_environment(), coefficients,
                            )?;
                            renderer.set_character_shadow_bounds(&mut object, character.hips_position, character.world_bounds)?;
                            scene.objects.push(object);
                        }
                    }
                    (SceneObjects::Colorized(objects), None, Some(inputs)) => objects.update(
                        renderer, self.map.environment(), self.map.frame_environment(), character.palette, inputs, camera,
                        &mut prior, &mut scene.objects,
                    )?,
                    (SceneObjects::Vehicle(vehicle), None, Some(inputs)) => vehicle.draws(
                        character.palette, character.root, character.hips_position, inputs.colours, inputs.irradiance,
                        vehicles,
                    )?,
                    _ => return Err("scene character renderable changed kind".into()),
                }
            } else {
                let mut lighting = crate::render_irradiance::SkaterLighting::new(self.map.environment().default_irradiance);
                lighting.update_tick(probe, visual.simulation_tick);
                let objects = match (materials, colorized(lighting.coefficients)) {
                    (Some(materials), _) => {
                        let created = selected_character_material::create_objects(
                            renderer, &self.mounts, character.palette, materials, camera, ghost,
                            self.map.environment(), self.map.frame_environment(), lighting.coefficients,
                        )?;
                        let mut bindings = Vec::new();
                        for (binding, mut object) in created {
                            renderer.set_character_shadow_bounds(&mut object, character.hips_position, character.world_bounds)?;
                            bindings.push(binding);
                            scene.objects.push(object);
                        }
                        SceneObjects::Skater(bindings)
                    }
                    (None, Some(inputs)) if crate::vehicle_render::takes_vehicle_effects(&self.mounts, character.palette)? => {
                        let vehicle = self.vehicles.scene_vehicle(renderer, character.palette, character.local_bounds)?;
                        vehicle.draws(
                            character.palette, character.root, character.hips_position, inputs.colours, inputs.irradiance,
                            vehicles,
                        )?;
                        SceneObjects::Vehicle(vehicle)
                    }
                    (None, Some(inputs)) => SceneObjects::Colorized(crate::pedestrian_render::ColorizedObjects::create(
                        renderer, &self.mounts, self.map.environment(), self.map.frame_environment(), character.palette,
                        inputs, camera, &mut scene.objects,
                    )?),
                    (None, None) => return Err("scene character without materials or colours".into()),
                };
                self.scene_characters.push(SceneCharacterObjects { palette: identity, objects, lighting });
            }
        }
        Ok(())
    }
}

/// `PostFX::VisualFX_Animation` (`0x82847610`) with the Visual Director's fade info, which it
/// takes only without a tazer or Hall of Meat effect and without challenge
/// success or failure info ([`challenge_result`]): the bloom threshold drops by the fade
/// (`0x828485e8`); `m_bloomMultiply` keeps the function's (1, 1, 0, 0)
/// (`0x82847654..0x8284767c`), its x raised by half the threshold's overshoot below zero, at most
/// 0.5 (`0x82848600..0x8284864c`), where the branch without effects writes the vault's fine,
/// medium and coarse intensities; the grades are not written. `SetParameters`' ICE path then
/// replaces the bloom, `m_bloomMultiply` and the grades, so a NIS shows none of it.
fn challenge_fade(postfx: &mut crate::render_postfx::Frame, fade: f32) {
    let threshold = postfx.bloom[0] - fade;
    let fine = if threshold < 0. {
        let half = -threshold * 0.5;
        1. + if half > 0.5 { 0.5 } else { half }
    } else {
        1.
    };
    postfx.bloom = [threshold, postfx.bloom[1], 0., 1., fine];
    postfx.keep_matrices = true;
}

/// `VisualFX_Animation`'s challenge success branch (`0x82847e7c`, `bChallengeSuccessInfo`) or,
/// without it, its failure branch (`0x82848378`, `bChallengeFailedInfo`), ahead of the fade's:
/// the grades (`Blend_FXMatrices`), `m_NISNearDist`, `m_NISFarDist` and the bloom threshold and
/// saturation blend from the normal ones toward the success (or failure) post-fx by
/// `functions_success` (or `function_failure_effect_vs_time`: x 0, 1, 5, 10; y 0, 0, 1, 1) at
/// the info's value, and `m_bloomMultiply` keeps the function's (1, 1, 0, 0). The shipped
/// treatment's values are all 0 (`vd_results`), where both functions are 0, so everything
/// blended stays normal and only the bloom's intensities change.
fn challenge_result(postfx: &mut crate::render_postfx::Frame) {
    postfx.bloom = [postfx.bloom[0], postfx.bloom[1], 0., 1., 1.];
}

/// The NIS's part of `PostFX::SetParameters` (`0x8284bbf0`). The general
/// blocks' `bloomToWhiteValue` lowers the bloom threshold by twice its
/// value (`VisualFX_Animation` `0x82848718`), on the ICE path too.
fn nis_postfx(postfx: &mut crate::render_postfx::Frame, nis: &skate2_core::nis::manager::NisView, planes: [f32; 2]) {
    let white = nis.bloom_to_white;
    match nis.ice_postfx {
        Some(ice) => ice_postfx(postfx, &ice, white, planes),
        None => postfx.bloom[0] = (-2f32).mul_add(white, postfx.bloom[0]),
    }
}

/// `SetParameters`' ICE path while `mICEPostfxActive`, from `mICEPostfxData`: it replaces the
/// game bloom with half the ICE threshold, saturation 1, `m_bloomMultiply` four times the ICE
/// intensity and four times `(1 - m_RadialBlur)` times it (`0x8284c104..0x8284c380`), and the
/// ICE fade colour as `g_FinalMult` (`0x8284c1f8..0x8284c260`). `ApplyFX_NEW` builds the near
/// and far grades, both with the near saturation (the far call reads
/// `nisNear.saturateIntensity` at `0x8284c1a0`).
fn ice_postfx(postfx: &mut crate::render_postfx::Frame, ice: &skate2_core::nis::manager::IcePostfx, white: f32, planes: [f32; 2]) {
    postfx.cinematic = true;
    let threshold = (-2f32).mul_add(white, ice.bloom[0] * 0.5);
    postfx.bloom = [threshold, 1., 0., (1. - ice.radial_blur) * ice.bloom[1] * 4., ice.bloom[1] * 4.];
    let [r, g, b, a] = ice.fade;
    let keep = 1. - a;
    postfx.final_mult = [r.mul_add(a, keep), g.mul_add(a, keep), b.mul_add(a, keep), keep];
    let [near, far] = ice.grade;
    let grade = |side: skate2_core::nis::manager::Grade| {
        crate::render_postfx_math::colour_matrix(side.contrast, side.midpoint, near.saturate + 1., [side.brightness; 3], side.colour)
    };
    postfx.ice = Some(crate::render_postfx::IceInputs {
        radial_blur: ice.radial_blur,
        scanline: ice.scanline,
        fisheye: ice.fisheye,
        matrices: [grade(near), grade(far)],
        lerp: [ice.far[1], ice.far[0]],
        tonemap: ice.tonemap,
        planes,
    });
}

/// The original renders one fixed 16:9 target:
/// `ViewCamera::SetPerspectiveMatrixRadians` 0x82840770 hard-codes
/// 1.777..., which the core projection keeps. Any other window shape keeps
/// that whole 16:9 view visible without stretching it: a wider window widens
/// the horizontal field (the vertical field stays the original's), a
/// narrower one widens the vertical field. Only the clip x or y row scales,
/// so depth is untouched and a 16:9 window stays bit-exact.
#[expect(
    clippy::cast_possible_truncation,
    reason = "f32 clip lanes scaled by an f64 aspect ratio"
)]
pub(crate) fn window_clip_from_world(clip_from_world: [f32; 16], (width, height): (u32, u32)) -> [f32; 16] {
    const NATIVE_ASPECT: f64 = 16.0 / 9.0;
    let aspect = f64::from(width.max(1)) / f64::from(height.max(1));
    let (x, y) = if aspect > NATIVE_ASPECT {
        (NATIVE_ASPECT / aspect, 1.0)
    } else {
        (1.0, aspect / NATIVE_ASPECT)
    };
    let mut clip = clip_from_world;
    for column in clip.chunks_exact_mut(4) {
        column[0] = (f64::from(column[0]) * x) as f32;
        column[1] = (f64::from(column[1]) * y) as f32;
    }
    clip
}

#[cfg(test)]
mod tests {
    use super::window_clip_from_world;

    #[test]
    fn a_nis_lowers_the_bloom_threshold_and_takes_the_ice_bloom_and_fade() {
        let frame = crate::render_postfx::Frame {
            bloom: [0.75, 0.3, 1.0, 2.0, 3.0],
            exposure: [0.0; 4],
            cube: 0,
            dt: 0.0,
            updated_at: std::time::Instant::now(),
            cinematic: false, camera_planes: None,
            final_mult: [1.0; 4],
            keep_matrices: false, tazer: false,
            hom: None,
            ice: None,
            fe_online_menu: false,
        };
        let mut nis = skate2_core::nis::manager::NisView { bloom_to_white: 0.25, ..Default::default() };
        let mut game = frame;
        super::nis_postfx(&mut game, &nis, [0.2, 1000.0]);
        assert_eq!(game.bloom, [0.25, 0.3, 1.0, 2.0, 3.0]);
        assert!(!game.cinematic);
        nis.ice_postfx = Some(skate2_core::nis::manager::IcePostfx { bloom: [1.5, 0.5], fade: [1.0, 0.0, 0.0, 0.25], ..Default::default() });
        let mut ice = frame;
        super::nis_postfx(&mut ice, &nis, [0.2, 1000.0]);
        assert!(ice.cinematic);
        assert_eq!(ice.bloom, [0.25, 1.0, 0.0, 2.0, 2.0]);
        assert_eq!(ice.final_mult, [1.0, 0.75, 0.75, 0.75]);
        let inputs = ice.ice.expect("ICE inputs");
        let identity = crate::render_postfx_math::colour_matrix(1.0, 0.5, 1.0, [0.0; 3], [1.0; 3]);
        assert_eq!((inputs.matrices, inputs.lerp, inputs.planes), ([identity; 2], [40.0, 150.0], [0.2, 1000.0]));
        assert_eq!((inputs.radial_blur, inputs.scanline, inputs.fisheye, inputs.tonemap), (0.0, 0.0, 1.0, 0.0));
    }

    #[test]
    fn a_challenge_fade_blooms_the_game_post_fx_and_a_nis_takes_none_of_it() {
        let frame = crate::render_postfx::Frame {
            bloom: [0.75, 0.3, 1.0, 2.0, 3.0],
            exposure: [0.0; 4],
            cube: 0,
            dt: 0.0,
            updated_at: std::time::Instant::now(),
            cinematic: false, camera_planes: None,
            final_mult: [1.0; 4],
            keep_matrices: false, tazer: false,
            hom: None,
            ice: None,
            fe_online_menu: false,
        };
        let mut half = frame;
        super::challenge_fade(&mut half, 0.5);
        assert_eq!(half.bloom, [0.25, 0.3, 0.0, 1.0, 1.0]);
        assert!(half.keep_matrices);
        let mut full = frame;
        super::challenge_fade(&mut full, 1.0);
        assert_eq!(full.bloom, [-0.25, 0.3, 0.0, 1.0, 1.125]);
        let mut deep = frame;
        super::challenge_fade(&mut deep, 3.0);
        assert_eq!(deep.bloom, [-2.25, 0.3, 0.0, 1.0, 1.5]);
        let nis = skate2_core::nis::manager::NisView {
            ice_postfx: Some(skate2_core::nis::manager::IcePostfx::default()),
            ..Default::default()
        };
        let mut scene = full;
        super::nis_postfx(&mut scene, &nis, [0.2, 1000.0]);
        assert_eq!(scene.bloom, [0.7, 1.0, 0.0, 2.0, 2.0]);
        assert!(scene.cinematic && scene.ice.is_some());
    }

    #[test]
    fn a_photo_take_rolls_the_screen_zooms_the_bloom_and_grades_far_with_the_near_saturation() {
        use skate2_core::nis::manager::{Grade, IcePostfx, NisView};
        let frame = crate::render_postfx::Frame {
            bloom: [0.75, 0.3, 1.0, 2.0, 3.0],
            exposure: [0.0; 4],
            cube: 0,
            dt: 0.0,
            updated_at: std::time::Instant::now(),
            cinematic: false, camera_planes: None,
            final_mult: [1.0; 4],
            keep_matrices: false, tazer: false,
            hom: None,
            ice: None,
            fe_online_menu: false,
        };
        let near = Grade { brightness: 0.0, contrast: 1.0, midpoint: 0.5, saturate: -0.6957, colour: [1.0, 0.57, 0.49] };
        let far = Grade { brightness: 0.1, contrast: 2.5679, midpoint: 0.6602, saturate: 10.0, colour: [1.0, 0.63, 0.2] };
        let ice = IcePostfx { grade: [near, far], radial_blur: 1.0, scanline: 0.25, fisheye: 4.761_904_7, tonemap: 1.5, ..Default::default() };
        let nis = NisView { ice_postfx: Some(ice), ..Default::default() };
        let mut photo = frame;
        super::nis_postfx(&mut photo, &nis, [0.2, 1000.0]);
        // The full radial blur leaves the medium bloom level nothing.
        assert_eq!(photo.bloom[3..], [0.0, 2.0]);
        let inputs = photo.ice.expect("ICE inputs");
        let math = crate::render_postfx_math::colour_matrix;
        assert_eq!(inputs.matrices[0], math(1.0, 0.5, 1.0 - 0.6957, [0.0; 3], near.colour));
        assert_eq!(inputs.matrices[1], math(2.5679, 0.6602, 1.0 - 0.6957, [0.1; 3], far.colour));
        assert_eq!((inputs.radial_blur, inputs.scanline, inputs.fisheye, inputs.tonemap), (1.0, 0.25, 4.761_904_7, 1.5));
    }

    #[test]
    fn widescreen_keeps_pixels_square_and_the_native_frame_visible() {
        let clip = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        assert_eq!(window_clip_from_world(clip, (1920, 1080)), clip);
        assert_eq!(window_clip_from_world(clip, (1280, 720)), clip);
        // 21:9 keeps the vertical field and narrows x by (16/9) / (21/9).
        let wide = window_clip_from_world(clip, (2520, 1080));
        assert_eq!(wide[1], clip[1]);
        assert!((wide[0] - clip[0] * 16.0 / 21.0).abs() < 1e-6);
        // 4:3 keeps the horizontal field and narrows y by (4/3) / (16/9).
        let tall = window_clip_from_world(clip, (1024, 768));
        assert_eq!(tall[0], clip[0]);
        assert!((tall[1] - clip[1] * 0.75).abs() < 1e-6);
        assert_eq!(tall[2..4], clip[2..4]);
    }
}
