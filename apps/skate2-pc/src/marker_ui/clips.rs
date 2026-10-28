//! Retained APT instances for audited marker, trick-score and HoM movies.
//! Fingerprinted ActionScript is translated here; this is not a general AVM.
use super::assets::Assets;
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    apt_playback::ClipPlayback,
    apt_timeline::{DisplayEntry, FrameEffect},
};
use skate2_formats::apt::{ActionStream, Character};
use std::{cell::RefCell, collections::{BTreeMap, VecDeque}, rc::Rc};

/// `AptActionQueueC::AddActionBack`829d9b68 retains the instance, not its
/// depth/name. Removed instances must not dispatch into their replacement.
#[derive(Default)]
struct FrameActions {
    next_instance: u64,
    pending: VecDeque<QueuedFrameAction>,
}
struct QueuedFrameAction {
    instance: u64,
    action: ActionStream,
    // tick82a1b70c stores -frame; jumpToFrame82a1b568 stores +frame.
    // RunActions82a1c8c0 guards only a negative frame (zero is unguarded).
    expected_frame: Option<usize>,
}
#[derive(Clone)]
struct DeferredFrameActions {
    instance: u64,
    queue: Rc<RefCell<FrameActions>>,
}

#[derive(Clone)]
pub(super) struct Child {
    pub entry: DisplayEntry,
    pub clip: Clip,
}
#[derive(Clone)]
pub(super) struct Clip {
    pub asset: usize,
    pub character: u32,
    pub playback: ClipPlayback,
    pub children: BTreeMap<i32, Child>,
    pub properties: BTreeMap<String, String>,
    pub text: Option<Vec<u16>>,
    pub visible: bool,
    pub calls: Vec<&'static str>,
    /// `SetProceduralProperty` with the ActionScript flag (`AptCIH+0xc` bit
    /// 31): `placeObject` and `ReplaceDisplyListItem` then skip this
    /// instance's timeline matrix and colour moves.
    pub script_transform: bool,
    deferred_actions: Option<DeferredFrameActions>,
}
impl Clip {
    pub fn new(assets: &Assets, asset: usize, character: u32) -> RenderResult<Self> {
        let (asset, character) = if assets.movies[asset]
            .movie
            .characters
            .get(character as usize)
            .is_some_and(Option::is_none)
        {
            let import = assets.movies[asset]
                .movie
                .imports
                .iter()
                .find(|i| i.character_id == character)
                .ok_or("unresolved APT import")?;
            let imported_asset = match import.file.rsplit(['/', '\\']).next() {
                Some("button_item2") => 1,
                Some("challenge_info") => 5,
                Some("menu_picker") => 6,
                Some("basic_menu_picker")=>super::contacts_movie::PICKER,
                Some("button_menu") => 7,
                Some("image_renderer") => 8,
                Some("basic_slider") => 10,
                Some("highlight_morph") => 16,
                Some("highlight_color") => 17,
                Some("highlight_selector") => 18,
                Some("highlight_category") => 19,
                Some("highlight_option") => 20,
                Some("highlight_slider") => 21,
                Some("highlight_arrow_left") => 22,
                Some("highlight_arrow_right") => 23,
                Some("swatch_renderer") => 24,
                Some("gesture_item") => 31,
                Some("results_lb") => super::end_challenge_movie::RESULTS_LB,
                Some("dynamic_selectable_list") => super::end_challenge_movie::SELECTABLE_LIST,
                Some("list_item_textbox") => super::end_challenge_movie::LIST_TEXTBOX,
                Some("list_item_numberbox") => super::end_challenge_movie::LIST_NUMBERBOX,
                Some("list_item_awardbox") => super::end_challenge_movie::LIST_AWARDBOX,
                Some("rank_badges") => super::end_challenge_movie::RANK_BADGES,
                Some("list_item_bonusbox") => super::end_challenge_movie::LIST_BONUSBOX,
                Some("sponsor_browser") => super::sponsor_overlay_movie::BROWSER,
                Some("skeleton") => super::hom_skeleton_movie::ASSET,
                Some("media_browser")=>super::replay_footage_movie::BROWSER,
                Some("mediathumb")=>super::replay_footage_movie::THUMB,
                Some("online_gamer_tag")=>super::online_match_movie::GAMER_TAG,
                Some("online_gamer_rank")=>super::skate_profile_movie::RANK,
                Some("hub_tree")=>super::online_hub_movie::TREE,
                Some("hub_menu")=>super::online_hub_movie::MENU,
                Some("background_scaler")=>super::online_hub_movie::SCALER,
                Some("challengesignup_icon")=>super::online_results_movie::ICON,
                _ => return Err(format!("untranslated APT import {}", import.file).into()),
            };
            let export = assets.movies[imported_asset]
                .movie.exports.iter().find(|e| e.name == import.name)
                .ok_or("APT imported export absent")?;
            (imported_asset, export.character_id)
        } else {
            (asset, character)
        };
        let mut clip = Self {
            asset,
            character,
            playback: Default::default(),
            children: BTreeMap::new(),
            properties: BTreeMap::new(),
            text: None,
            visible: true,
            calls: Vec::new(),
            script_transform: false,
            deferred_actions: None,
        };
        if let Character::Text(text) = clip.definition(assets)? {
            clip.text = Some(assets.initial_text(&text.initial_text));
        }
        if let Some(movie) = clip.definition(assets)?.movie() {
            let effects = clip.playback.tick(movie)?;
            clip.apply(assets, effects)?;
        }
        Ok(clip)
    }
    pub fn definition<'a>(&self, assets: &'a Assets) -> RenderResult<&'a Character> {
        assets.movies[self.asset]
            .movie
            .characters
            .get(self.character as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| "APT character absent".into())
    }
    fn apply(&mut self, assets: &Assets, effects: Vec<FrameEffect>) -> RenderResult<()> {
        self.apply_frame_effects(assets, effects, None, true)
    }
    fn apply_frame_effects(&mut self, assets: &Assets, effects: Vec<FrameEffect>,
        expected_frame: Option<usize>, queue_actions: bool) -> RenderResult<()> {
        for effect in effects {
            match effect {
                FrameEffect::Create { depth, entry } => {
                    let mut clip = Self::new(assets, self.asset, entry.identity.character)?;
                    if let Some(deferred) = &self.deferred_actions {
                        clip.share_frame_actions(Rc::clone(&deferred.queue));
                    }
                    for event in &entry.events {
                        // photoselect names its buttons in load events (0x200) instead.
                        if event.triggers != 0x40000 && !(self.asset == super::photo_select_movie::ASSET && event.triggers == 0x200) {
                            return Err("untranslated marker clip event".into());
                        }
                        let props = property_initializers(
                            &assets.movies[self.asset].bytes,
                            event.actions.offset,
                        )?;
                        clip.properties.extend(props);
                    }
                    // Button Item constructor invokes SetButtonType with the
                    // placement's mButtonImage, after initialize-event setup.
                    if clip.asset == 1 {
                        if let Some(button) = clip.properties.get("mButtonImage").cloned() {
                            clip.named(&["mButtonRender"])?
                                .properties
                                .insert("_test".into(), button);
                        }
                    }
                    // The Gesture Item constructor runs with the
                    // placement's mGestureImage, after initialize events.
                    if clip.asset == 31 && clip.character == super::tutorial_movie::GESTURE_ITEM {
                        super::tutorial_movie::construct_gesture_item(&mut clip, assets)?;
                    }
                    self.children.insert(depth, Child { entry, clip });
                }
                FrameEffect::Update { depth, entry } => {
                    let child = self.children
                        .get_mut(&depth)
                        .ok_or("APT retained child absent")?;
                    if !child.clip.script_transform {
                        child.entry = entry;
                    }
                }
                FrameEffect::Remove { depth, .. } => {
                    self.children.remove(&depth);
                }
                FrameEffect::Action(action) => {
                    if queue_actions {
                        if let Some(deferred) = &self.deferred_actions {
                            deferred.queue.borrow_mut().pending.push_back(QueuedFrameAction {
                                instance: deferred.instance, action, expected_frame,
                            });
                            continue;
                        }
                    }
                    let bytes = &assets.movies[self.asset].bytes;
                    let stop = bytes.get(action.offset..action.offset + 2) == Some(&[7, 0]);
                    let empty = bytes.get(action.offset) == Some(&0);
                    let action = assets.movies[self.asset].translated(action);
                    if stop {
                        self.playback.stop();
                    } else if empty {
                        // An empty action: its End alone.
                    } else if self.asset == super::replay_editor_movie::ASSET {
                        super::replay_editor_movie::action(self, action.offset)?;
                    } else if self.asset == super::spectator_movie::ASSET {
                        super::spectator_movie::action(self, action.offset)?;
                    } else if self.asset == super::activity_overlay_movie::ASSET {
                        super::activity_overlay_movie::action(self, action.offset)?;
                    } else if self.asset == super::activity_signup_movie::ASSET {
                        super::activity_signup_movie::action(self, action.offset)?;
                    } else if self.asset == super::contacts_movie::ASSET {
                        super::contacts_movie::action(self, action.offset)?;
                    } else if self.asset == super::trick_guide_movie::ASSET {
                        super::trick_guide_movie::action(self, action.offset)?;
                    } else if (super::replay_footage_movie::ASSET..=super::replay_footage_movie::THUMB).contains(&self.asset) {
                        super::replay_footage_movie::action(self,action.offset)?;
                    } else if (super::skate_reel_movie::ONLINE..=super::skate_reel_movie::PHOTO).contains(&self.asset) {
                        super::skate_reel_movie::action(self,action.offset)?;
                    } else if self.asset == super::extras_movie::ASSET {
                        super::extras_movie::action(self, action.offset)?;
                    } else if self.asset == super::dlc_movie::ASSET {
                        super::dlc_movie::action(self, action.offset)?;
                    } else if self.asset == super::credits_movie::ASSET {
                        super::credits_movie::action(self, action.offset)?;
                    } else if (super::fmv_tutorial_movie::PLAYER..=super::fmv_tutorial_movie::HOM).contains(&self.asset) {
                        super::fmv_tutorial_movie::action(self, action.offset)?;
                    } else if self.asset == super::freeskate_movie::ASSET {
                        super::freeskate_movie::action(self, action.offset)?;
                    } else if self.asset == super::music_player_movie::ASSET {
                        super::music_player_movie::action(self, action.offset)?;
                    } else if self.asset == super::chyron_movie::ASSET {
                        super::chyron_movie::action(self, action.offset)?;
                    } else if self.asset == super::party_play_movie::ASSET {
                        super::party_play_movie::action(self, action.offset)?;
                    } else if self.asset == super::party_play_press_movie::ASSET {
                        super::party_play_press_movie::action(self, action.offset)?;
                    } else if self.asset == super::throwdown_results_movie::ASSET {
                        super::throwdown_results_movie::action(self, action.offset)?;
                    } else if self.asset == super::big_black_timer_movie::ASSET {
                        super::big_black_timer_movie::action(self,action.offset)?;
                    } else if self.asset == super::hud_timer_movie::ASSET {
                        super::hud_timer_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_throwmes_movie::ASSET {
                        super::hud_throwmes_movie::action(self, action.offset)?;
                    } else if self.asset == super::online_eventlog_movie::ASSET {
                        super::online_eventlog_movie::action(self, action.offset)?;
                    } else if (super::online_match_movie::CUSTOM..=super::online_match_movie::GAMER_TAG).contains(&self.asset) {
                        super::online_match_movie::action(self, action.offset)?;
                    } else if self.asset == super::online_hub_movie::HUB {
                        super::online_hub_movie::action(self, action.offset)?;
                    } else if self.asset == super::online_results_movie::ASSET {
                        super::online_results_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_abd_movie::ASSET {
                        super::hud_abd_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_leaderboard_movie::ASSET {
                        super::hud_leaderboard_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_countdown_movie::ASSET {
                        super::hud_countdown_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_dpo_movie::ASSET {
                        super::hud_dpo_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_distance_movie::ASSET {
                        super::hud_distance_movie::action(self, action.offset)?;
                    } else if self.asset == super::hud_voting_movie::ASSET {
                        super::hud_voting_movie::action(self, action.offset)?;
                    } else if matches!(self.asset, super::end_challenge_movie::ASSET | super::end_challenge_movie::GVR_ASSET) {
                        super::end_challenge_movie::action(self, action.offset)?;
                    } else if self.asset == super::challenge_failed_movie::ASSET {
                        super::challenge_failed_movie::action(self, action.offset)?;
                    } else if self.asset == super::challenge_failed_small_movie::ASSET {
                        super::challenge_failed_small_movie::action(self, action.offset)?;
                    } else if self.asset == super::end_challenge_small_movie::ASSET {
                        super::end_challenge_small_movie::action(self, action.offset)?;
                    } else if self.asset == super::party_play_change_movie::ASSET {
                        super::party_play_change_movie::action(self, action.offset)?;
                    } else if (super::end_challenge_movie::RESULTS_LB..=super::end_challenge_movie::LIST_BONUSBOX).contains(&self.asset) {
                        return Err(format!(
                            "untranslated results list APT action {}:{} {:#x}",
                            self.asset, self.character, action.offset
                        )
                        .into());
                    } else if self.asset == super::signup_poster_movie::ASSET {
                        super::signup_poster_movie::action(self, action.offset)?;
                    } else if self.asset == super::unlock_overlay_movie::ASSET {
                        super::unlock_overlay_movie::action(self, action.offset)?;
                    } else if self.asset == super::skate101_movie::ASSET {
                        super::skate101_movie::action(self, action.offset)?;
                    } else if self.asset == super::subtitle_movie::ASSET {
                        super::subtitle_movie::action(self, action.offset)?;
                    } else if self.asset == super::gestollie_movie::ASSET {
                        super::gestollie_movie::action(self, action.offset)?;
                    } else if self.asset == super::ownthespot_movie::ASSET {
                        super::ownthespot_movie::action(self, action.offset)?;
                    } else if self.asset == super::spotfound_movie::ASSET {
                        super::spotfound_movie::action(self, action.offset)?;
                    } else if self.asset == super::killedit_movie::ASSET {
                        super::killedit_movie::action(self, action.offset)?;
                    } else if self.asset == super::hom_annals_movie::ASSET {
                        super::hom_annals_movie::action(self, action.offset)?;
                    } else if self.asset == super::hom_results_movie::ASSET {
                        super::hom_results_movie::action(self, action.offset)?;
                    } else if self.asset == super::hom_skeleton_movie::ASSET {
                        super::hom_skeleton_movie::action(self, action.offset)?;
                    } else if self.asset == super::photo_select_movie::ASSET {
                        super::photo_select_movie::action(self, action.offset)?;
                    } else if self.asset == super::receipt_movie::ASSET {
                        super::receipt_movie::action(self, action.offset)?;
                    } else if matches!(self.asset, super::sponsor_overlay_movie::ASSET | super::sponsor_overlay_movie::BROWSER) {
                        super::sponsor_overlay_movie::action(self, action.offset)?;
                    } else if matches!(self.asset, super::slappys_movie::BLOG | super::slappys_movie::ARTICLES | super::slappys_movie::COVERS) {
                        super::slappys_movie::action(self, action.offset)?;
                    } else if self.asset == super::skate_profile_movie::ASSET {
                        super::skate_profile_movie::action(self, action.offset)?;
                    } else if matches!(
                        self.asset,
                        super::phototreatment_movie::ASSET | super::phototreatment_movie::FIXED | super::phototreatment_movie::NON_FIXED
                    ) {
                        super::phototreatment_movie::action(self, action.offset)?;
                    } else if self.asset == super::pro_bio_movie::ASSET {
                        super::pro_bio_movie::action(self, action.offset)?;
                    } else if self.asset == super::gvr_movie::REGULAR || self.asset == super::gvr_movie::GOOFY {
                        super::gvr_movie::action(self, action.offset)?;
                    } else if self.asset == super::sanvantv_movie::ASSET {
                        super::sanvantv_movie::action(self, action.offset)?;
                    } else if self.asset == 32 {
                        super::action_overlay_movie::action(self, action.offset)?;
                    } else if matches!(self.asset, 30 | 31) {
                        super::tutorial_movie::action(self, action.offset)?;
                    } else if self.asset == 29 {
                        super::objective_movie::action(self, action.offset)?;
                    } else if self.asset == 28 {
                        super::settings_movie::action(self, action.offset)?;
                    } else if self.asset == 27 {
                        super::main_movie::action(self,action.offset)?;
                    } else if self.asset == 26 {
                        super::minimap_movie::action(self,action.offset)?;
                    } else if self.asset >= 15 {
                        super::edit_skater_movie::action(self,action.offset)?;
                    } else if self.asset==13 {
                        super::jacked_movie::action(self,action.offset)?;
                    } else if self.asset==14 {
                        super::central_movie::action(self,action.offset)?;
                    } else if (9..=12).contains(&self.asset) {
                        super::map_movie::action(self, action.offset)?;
                    } else if (4..=8).contains(&self.asset) {
                        super::pause_movie::action(self, action.offset)?;
                    } else if self.asset == 3 {
                        match action.offset {
                            0x5ce0 | 0x5de4 => {}, // HomMovie owns the verified class/constructor.
                            0x6cb8 => self.calls.push("HomAnimStarted"),
                            0x6cfc => { self.calls.push("HomAnimFinished"); self.playback.stop(); }
                            offset => return Err(format!("untranslated HoM APT action {offset:#x}").into()),
                        }
                    } else if self.asset == 2 {
                        self.score_action(action.offset)?;
                    } else if self.asset == 0
                        && self.character == 0
                        && matches!(action.offset, 0x5114 | 0x5134)
                    {
                        // Original class initialization/constructor. Assets
                        // are fingerprint-checked before this translation.
                        self.playback.stop();
                    } else {
                        return Err(format!(
                            "untranslated APT action {}:{} {:#x}",
                            self.asset, self.character, action.offset
                        )
                        .into());
                    }
                }
                FrameEffect::InitAction { character, actions } => {
                    let actions = assets.movies[self.asset].translated(actions);
                    // __Packages.hudphonelist defines the class before its
                    // root constructor. Its methods are the checked Rust
                    // translation; initialization itself creates no display.
                    if !((self.asset == 0 && character == 78 && actions.offset == 0x5134)
                        || (self.asset == 2 && character == 109 && actions.offset == 0x21078)
                        || (self.asset == 3 && character == 59 && actions.offset == 0x5de4)
                        || (self.asset == 13 && character == 16 && actions.offset == 0x1304)
                        || (self.asset == 26 && character == 10 && actions.offset == 0x2e18)
                        || (self.asset == 27 && character == 2 && actions.offset == 0x554)
                        || (self.asset == 85 && character == 2 && actions.offset == 0x308)
                        || (self.asset == 86 && character == 4 && actions.offset == 0x314)
                        || super::fmv_tutorial_movie::init_action(self.asset, character, actions.offset)
                        || (self.asset == super::replay_editor_movie::ASSET && character == 200 && actions.offset == 0x82d8)
                        || (self.asset == super::trick_guide_movie::ASSET && character == 32 && actions.offset == 0x45fc)
                        || super::replay_footage_movie::init_action(self.asset, character, actions.offset)
                        || super::skate_reel_movie::init_action(self.asset, character, actions.offset)
                        || super::contacts_movie::init_action(self.asset, character, actions.offset)
                        || (self.asset == 28 && character == 4 && actions.offset == 0x3a4)
                        || super::objective_movie::init_action(self.asset, character, actions.offset)
                        || super::tutorial_movie::init_action(self.asset, character, actions.offset)
                        || super::action_overlay_movie::init_action(self.asset, character, actions.offset)
                        || super::signup_poster_movie::init_action(self.asset, character, actions.offset)
                        || super::unlock_overlay_movie::init_action(self.asset, character, actions.offset)
                        || super::skate101_movie::init_action(self.asset, character, actions.offset)
                        || super::subtitle_movie::init_action(self.asset, character, actions.offset)
                        || super::gestollie_movie::init_action(self.asset, character, actions.offset)
                        || super::ownthespot_movie::init_action(self.asset, character, actions.offset)
                        || super::spotfound_movie::init_action(self.asset, character, actions.offset)
                        || super::killedit_movie::init_action(self.asset, character, actions.offset)
                        || super::photo_select_movie::init_action(self.asset, character, actions.offset)
                        || super::receipt_movie::init_action(self.asset, character, actions.offset)
                        || super::sponsor_overlay_movie::init_action(self.asset, character, actions.offset)
                        || super::slappys_movie::init_action(self.asset, character, actions.offset)
                        || super::skate_profile_movie::init_action(self.asset, character, actions.offset)
                        || super::freeskate_movie::init_action(self.asset, character, actions.offset)
                        || super::music_player_movie::init_action(self.asset, character, actions.offset)
                        || super::chyron_movie::init_action(self.asset, character, actions.offset)
                        || super::party_play_movie::init_action(self.asset, character, actions.offset)
                        || super::party_play_press_movie::init_action(self.asset, character, actions.offset)
                        || super::throwdown_results_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_timer_movie::init_action(self.asset, character, actions.offset)
                        || super::big_black_timer_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_throwmes_movie::init_action(self.asset, character, actions.offset)
                        || super::online_eventlog_movie::init_action(self.asset, character, actions.offset)
                        || super::online_match_movie::init_action(self.asset, character, actions.offset)
                        || super::online_hub_movie::init_action(self.asset, character, actions.offset)
                        || super::online_results_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_abd_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_leaderboard_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_countdown_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_dpo_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_distance_movie::init_action(self.asset, character, actions.offset)
                        || super::hud_voting_movie::init_action(self.asset, character, actions.offset)
                        || super::spectator_movie::init_action(self.asset, character, actions.offset)
                        || super::activity_overlay_movie::init_action(self.asset, character, actions.offset)
                        || super::activity_signup_movie::init_action(self.asset, character, actions.offset)
                        || super::end_challenge_movie::init_action(self.asset, character, actions.offset)
                        || super::challenge_failed_movie::init_action(self.asset, character, actions.offset)
                        || super::challenge_failed_small_movie::init_action(self.asset, character, actions.offset)
                        || super::end_challenge_small_movie::init_action(self.asset, character, actions.offset)
                        || super::party_play_change_movie::init_action(self.asset, character, actions.offset)
                        || super::phototreatment_movie::init_action(self.asset, character, actions.offset)
                        || super::pro_bio_movie::init_action(self.asset, character, actions.offset)
                        || super::gvr_movie::init_action(self.asset, character, actions.offset)
                        || super::sanvantv_movie::init_action(self.asset, character, actions.offset)
                        || super::edit_skater_movie::init_action(self.asset, character, actions.offset)
                        || super::central_movie::init_action(self.asset, character, actions.offset)
                        || super::hom_results_movie::init_action(self.asset, character, actions.offset)
                        || super::hom_annals_movie::init_action(self.asset, character, actions.offset)
                        || super::hom_skeleton_movie::init_action(self.asset, character, actions.offset)
                        || super::pause_movie::init_action(self.asset, character, actions.offset)
                        || super::map_movie::init_action(self.asset, character, actions.offset)
                        || super::dlc_movie::init_action(self.asset, character, actions.offset)) {
                        return Err(format!(
                            "untranslated APT InitAction {}:{character} {:#x}",
                            self.asset, actions.offset
                        )
                        .into());
                    }
                }
                FrameEffect::Background(_) => {} // mbEnableBackground=false, ctor826091c0.
            }
        }
        Ok(())
    }
    /// Opt in after constructing the movie. The score movie owns the native
    /// tick -> RunActions boundary; other translated screens keep their
    /// existing dispatch until their own action lifetimes are audited.
    pub fn defer_frame_actions(&mut self) {
        self.share_frame_actions(Rc::new(RefCell::new(FrameActions::default())));
    }
    fn share_frame_actions(&mut self, queue: Rc<RefCell<FrameActions>>) {
        let instance = {
            let mut actions = queue.borrow_mut();
            let instance = actions.next_instance;
            actions.next_instance += 1;
            instance
        };
        self.deferred_actions = Some(DeferredFrameActions {instance, queue: Rc::clone(&queue)});
        for child in self.children.values_mut() {
            child.clip.share_frame_actions(Rc::clone(&queue));
        }
    }
    fn frame_action_instance(&mut self, instance: u64) -> Option<&mut Self> {
        if self.deferred_actions.as_ref().is_some_and(|actions| actions.instance == instance) {
            return Some(self);
        }
        self.children.values_mut().find_map(|child| child.clip.frame_action_instance(instance))
    }
    /// Run one queued body in FIFO order. Let the movie handle its callbacks
    /// before the next body, including any actions those callbacks append.
    /// RunActions82a1c8bc..e0 skips removed owners and obsolete tick actions;
    /// explicit goto actions survive subsequent seeks on the same instance.
    pub fn run_next_frame_action(&mut self, assets: &Assets) -> RenderResult<bool> {
        let pending = self.deferred_actions.as_ref()
            .and_then(|actions| actions.queue.borrow_mut().pending.pop_front());
        let Some(pending) = pending else {return Ok(false);};
        if let Some(clip) = self.frame_action_instance(pending.instance) {
            if pending.expected_frame.is_none_or(|frame| clip.playback.timeline().frame() == Some(frame)) {
                clip.apply_frame_effects(assets, vec![FrameEffect::Action(pending.action)], None, false)?;
            }
        }
        Ok(true)
    }
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(movie) = self.definition(assets)?.movie() {
            let effects = self.playback.tick(movie)?;
            let expected_frame = self.playback.timeline().frame().filter(|&frame| frame != 0);
            self.apply_frame_effects(assets, effects, expected_frame, true)?;
        }
        // Stopped parents still advance all nested clips.
        for child in self.children.values_mut() {
            child.clip.tick(assets)?;
        }
        Ok(())
    }
    pub fn goto(&mut self, assets: &Assets, label: &str) -> RenderResult<()> {
        let movie = self.definition(assets)?.movie().ok_or("goto on nonmovie")?;
        let effects = match self.playback.goto_label(movie, label, true) {
            Ok(effects) => effects,
            // labelToFrame returns -1; _gotoAndX82a1d00c ignores that target
            // without changing playback. Shipped mcSwitch's obsolete intro
            // call reaches this native path (the clip has no labels).
            Err(skate2_core::ui::apt_timeline::TimelineError::UnknownLabel(_)) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        self.apply(assets, effects)
    }
    /// `gotoAndStop` on a label; the shared `_gotoAndX82a1d00c` ignores a label the clip lacks,
    /// as for `gotoAndPlay`.
    pub fn goto_and_stop(&mut self, assets: &Assets, label: &str) -> RenderResult<()> {
        let movie = self.definition(assets)?.movie().ok_or("goto on nonmovie")?;
        let effects = match self.playback.goto_label(movie, label, false) {
            Ok(effects) => effects,
            Err(skate2_core::ui::apt_timeline::TimelineError::UnknownLabel(_)) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        self.apply(assets, effects)
    }
    pub fn goto_number(&mut self, assets: &Assets, number: usize) -> RenderResult<()> {
        let movie = self.definition(assets)?.movie().ok_or("goto on nonmovie")?;
        let effects = self.playback.goto_number(movie, number, true)?;
        self.apply(assets, effects)
    }
    /// `gotoAndStop` on a frame number (1 is the first frame).
    pub fn goto_number_and_stop(&mut self, assets: &Assets, number: usize) -> RenderResult<()> {
        let movie = self.definition(assets)?.movie().ok_or("goto on nonmovie")?;
        let effects = self.playback.goto_number(movie, number, false)?;
        self.apply(assets, effects)
    }
    pub fn take_calls(&mut self, calls: &mut Vec<&'static str>) {
        calls.append(&mut self.calls);
        for child in self.children.values_mut() { child.clip.take_calls(calls); }
    }
    fn score_action(&mut self, offset: usize) -> RenderResult<()> {
        // Exact original trickdisplay2 timeline scripts, after fingerprint
        // admission. All other bodies remain errors, never generic no-ops.
        match offset {
            0x20f88 | 0x21078 => {}, // Class/constructor owned by ScoreMovie.
            0x23024 | 0x230c4 | 0x23254 | 0x2326c => self.visible = true,
            0x2304c | 0x2306c | 0x230ac | 0x23104 | 0x23120 | 0x23208 | 0x23288 => {
                self.visible = false; self.playback.stop();
            }
            0x230e0 => {
                self.playback.stop(); self.visible = false;
                self.calls.push("TrickDisplayTrickNameOutroDone");
            }
            0x23320 => {
                self.calls.push("TrickDisplayLineTimerOutroDone"); self.playback.stop();
            }
            _ => return Err(format!("untranslated scoring APT action {offset:#x}").into()),
        }
        Ok(())
    }
    /// Apt finds named children case-insensitively
    /// (`AptNativeHash::HashFindKey` `0x829da970`, `AptStricmp`).
    pub fn named(&mut self, path: &[&str]) -> RenderResult<&mut Self> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(self);
        };
        self.children
            .values_mut()
            .find(|c| c.entry.name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case(first)))
            .ok_or_else(|| format!("missing authored marker child {first}"))?
            .clip
            .named(rest)
    }
}

/// APT's specialized PushString/PushStringSetVar pair in initialize events.
/// Real byte offsets and alignment, not hardcoded controller icons by depth.
fn property_initializers(bytes: &[u8], mut p: usize) -> RenderResult<BTreeMap<String, String>> {
    fn string_operand(bytes: &[u8], p: &mut usize, opcode: u8) -> RenderResult<String> {
        if bytes.get(*p) != Some(&opcode) {
            return Err("unexpected APT initializer opcode".into());
        }
        *p = (*p + 4) & !3;
        let address = u32::from_be_bytes(
            bytes
                .get(*p..*p + 4)
                .ok_or("APT string operand")?
                .try_into()?,
        ) as usize;
        *p += 4;
        let tail = bytes.get(address..).ok_or("APT string address")?;
        let length = tail
            .iter()
            .position(|b| *b == 0)
            .ok_or("APT string terminator")?;
        Ok(std::str::from_utf8(&tail[..length])?.into())
    }
    let mut result = BTreeMap::new();
    while bytes.get(p) != Some(&0) {
        let name = string_operand(bytes, &mut p, 161)?;
        let value = string_operand(bytes, &mut p, 166)?;
        result.insert(name, value);
    }
    Ok(result)
}
