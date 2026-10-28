//! Original frontend archive, font Vault references and Localizer strings.
use crate::{gpu_renderer::RenderResult, render_environment::with_vault};
use skate2_core::{mounted_character_scene::own_texture_arena, texture::NativeTexture};
use skate2_formats::{
    apt::{ActionStream, Animation, BitmapFont, Geometry},
    localization::{Histogram, LanguageCategory, string_hash},
    mount::{MountError, MountSet, MountedBig},
    rw4::Arena,
};
use std::collections::BTreeMap;

pub(super) struct MovieAsset {
    /// The authored movie basename also owns its `<name>_textures` geometry.
    pub texture_owner: String,
    pub movie: Animation,
    pub geometry: Geometry,
    pub bytes: Vec<u8>,
    /// For a retail movie, each script action whose offset moved, as (retail, verified) offsets.
    pub moved_actions: &'static [(usize, usize)],
}
pub(super) struct Font {
    pub metrics: BitmapFont,
    pub texture: String,
    /// The `data\fe\fonts` file name `cFontManager::GetFontByName` matches.
    pub file: String,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
    pub line_height_scale: f32,
}
pub(super) struct MapInfo {
    pub texture: String,
    // Original MapInfo: xmax,xmin,ymax,ymin,width,height.
    pub extents: [i32; 4],
    pub pixels: [i32; 2],
}
pub(super) struct Assets {
    cac_archive: MountedBig,
    /// `FrontEndManager::SetupBigFiles`826062e8's data/fe/locations.big.
    locations_archive: MountedBig,
    /// The linked downloadable content's archives (`dlcbig:`), which hold its locations' images.
    dlc_archives: Vec<std::sync::Arc<MountedBig>>,
    /// `screens.big`, for the DPO HUD's `dpoicons`.
    screens_archive: MountedBig,
    pub map_info: MapInfo,
    pub local_player_icon: String,
    /// Current FEBackEnd custom-argument values, refreshed with each career view.
    pub profile_name: String,
    pub pro_film_pro_name:String,
    pub sponsor_labels: [String; 4],
    pub frontend_arguments: skate2_core::FrontendArguments,
    pub movies: [MovieAsset; 120],
    pub fonts: BTreeMap<String, Font>,
    pub credits_source: String,
    /// The Flash name of the first font group entry whose file is not
    /// `debug`: `GetFontByAptName`'s answer to an unknown name.
    pub font_fallback: String,
    pub textures: BTreeMap<String, NativeTexture>,
    /// [`Assets::texture`]'s answers by the suffix as asked; adding a texture clears them.
    texture_suffixes: std::sync::Mutex<std::collections::HashMap<String, SuffixMatch>>,
    pub strings: BTreeMap<String, Vec<u16>>,
    pub localized: BTreeMap<u32, Vec<u16>>,
}

/// What a case-insensitive texture-name suffix selects.
enum SuffixMatch {
    Key(String),
    Absent,
    Ambiguous,
}

fn textures(bytes: &[u8]) -> RenderResult<Vec<NativeTexture>> {
    let resources = Arena::split_file_resources(bytes)?;
    let arena = Arena::parse(&resources)?;
    Ok(own_texture_arena(&arena, &resources)?.textures)
}
/// The retail game's versions of verified movies, each with the fingerprint the translation was
/// made from (the debug build's data) and the script actions whose offset moved, as (retail,
/// verified) offsets; the moved stops run from their bytes. Compared in full, none changes what
/// the translations draw: `challenge_info` sets an unread `temp`, no longer destroys
/// `mMenuPicker` and tests `_visible` without `== true`; `challenge_signup` adds traces, an
/// `inputLocked` flag no script reads, and `SetButtonType` calls on `mButtonItem1` to `3`, clips
/// it never places; `hudobjective2` keeps its labels' `"right"` alignment in `mAlignment` and
/// drops `InitTitleSection`'s repeat of the constructor's alignment loop; `onlineHubMenu` no
/// longer destroys `mButtonHelp`; `skateProfile`'s functions push `this` where the debug build's
/// keep it in a register, their other registers one lower.
const RETAIL_EQUIVALENTS: [(u32, u32, &[(usize, usize)]); 5] = [
    (0x56ba_b8fa, 0xfee3_1040, &[]),
    (0x5273_d80c, 0x0ecc_a09a, &[(0x55fc, 0x5604)]),
    (
        0x711e_9b3c,
        0x2600_ff8b,
        &[
            (0x63ac, 0x6348),
            (0x63d8, 0x6374),
            (0x63f8, 0x6398),
            (0x6414, 0x63b4),
        ],
    ),
    (
        0xc773_1a08,
        0xd6d4_a1a4,
        &[
            (0x6f80, 0x6f74),
            (0x6ff4, 0x6fe8),
            (0x7d8c, 0x7dbc),
            (0x7da8, 0x7dd8),
            (0x7dd4, 0x7e04),
            (0x7f64, 0x7f94),
            (0x7fc8, 0x7ff8),
            (0x7ff8, 0x8028),
            (0x8038, 0x8068),
            (0x8068, 0x8098),
        ],
    ),
    (
        0x66f8_fc9d,
        0x111a_2d82,
        &[(0x35e8, 0x35f4), (0x363c, 0x3648)],
    ),
];

fn movie(screens: &MountedBig, base: &str) -> RenderResult<MovieAsset> {
    let bytes = screens.read_path(&format!("{base}.apt"))?;
    Ok(MovieAsset {
        texture_owner: base.rsplit(['\\', '/']).next().ok_or("UI movie without basename")?.to_owned(),
        movie: Animation::parse(&bytes, &screens.read_path(&format!("{base}.const"))?)?,
        geometry: Geometry::parse(&screens.read_path(&format!("{base}.geo"))?)?,
        bytes,
        moved_actions: &[],
    })
}
impl MovieAsset {
    /// The action as the translations know it: at the verified movie's offset.
    pub(super) fn translated(&self, actions: ActionStream) -> ActionStream {
        self.moved_actions
            .iter()
            .find(|&&(retail, _)| retail == actions.offset)
            .map_or(actions, |&(_, offset)| ActionStream { offset })
    }
}
impl Assets {
    /// FEDynamicTexture resolves CAC thumbnails in the separately mounted
    /// data/fe/cas.big (original header has data\\fe\\cas\\...rx2 entries).
    pub(super) fn ensure_cac_textures(
        &mut self,
        _mounts: &MountSet,
        paths: &[String],
    ) -> RenderResult<bool> {
        let mut changed = false;
        for path in paths {
            let key = path.to_ascii_lowercase();
            if self.textures.contains_key(&key) {
                continue;
            }
            let mut loaded = textures(&self.cac_archive.read_path(&format!("data\\{path}.rx2"))?)?;
            if loaded.len() != 1 {
                return Err(format!("CAC thumbnail {path} has {} textures", loaded.len()).into());
            }
            let mut texture = loaded.remove(0);
            texture.name = key.clone();
            self.textures.insert(key, texture);
            self.forget_texture_suffixes();
            changed = true;
        }
        Ok(changed)
    }
    /// `FrontEndState_FreeSkateOption::OnUpdate`8266a298's `LoadSyncArena` of
    /// data\fe\source\images\locations\<image>.rx2 and its `<image>.Texture`: the texture key,
    /// and whether it was loaded now.
    pub(super) fn ensure_location_texture(&mut self, image: &str) -> RenderResult<(String, bool)> {
        let path = format!("fe\\source\\images\\locations\\{image}");
        let key = path.to_ascii_lowercase();
        if self.textures.contains_key(&key) {
            return Ok((key, false));
        }
        let entry = format!("data\\{path}.rx2");
        let bytes = match self.locations_archive.read_path(&entry) {
            Ok(bytes) => bytes,
            // A downloadable content location's image is in its archives, past the game's.
            Err(MountError::EntryNotFound(_)) => self
                .dlc_archives
                .iter()
                .find_map(|archive| archive.read_path(&entry).ok())
                .ok_or_else(|| format!("location image {image}.rx2 absent"))?,
            Err(error) => return Err(error.into()),
        };
        let loaded = textures(&bytes)?;
        let name = format!("{image}.Texture");
        let mut texture = loaded
            .into_iter()
            .find(|texture| texture.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| format!("location image {image}.rx2 lacks {name}"))?;
        texture.name = key.clone();
        self.textures.insert(key.clone(), texture);
        self.forget_texture_suffixes();
        Ok((key, true))
    }
    /// The DPO HUD's `LoadRX2Texture("fe\source\images\dpoicons\" + name)` from
    /// `screens.big`: the texture's key, and whether it was loaded now.
    pub(super) fn ensure_dpo_icon(&mut self, name: &str) -> RenderResult<(String, bool)> {
        let path = format!("fe\\source\\images\\dpoicons\\{name}");
        let key = path.to_ascii_lowercase();
        if self.textures.contains_key(&key) {
            return Ok((key, false));
        }
        let loaded = textures(&self.screens_archive.read_path(&format!("data\\{path}.rx2"))?)?;
        let wanted = format!("{name}.Texture");
        let mut texture = loaded
            .into_iter()
            .find(|texture| texture.name.eq_ignore_ascii_case(&wanted))
            .ok_or_else(|| format!("DPO icon {name}.rx2 lacks {wanted}"))?;
        texture.name = key.clone();
        self.textures.insert(key.clone(), texture);
        self.forget_texture_suffixes();
        Ok((key, true))
    }
    /// An image renderer's `LoadRX2Texture(path)` of a `screens.big` image (`<path>.rx2` and its
    /// `<name>.Texture`): the texture's key, and whether it was loaded now.
    pub(super) fn ensure_rx2_image(&mut self, path: &str) -> RenderResult<(String, bool)> {
        let key = path.to_ascii_lowercase();
        if self.textures.contains_key(&key) {
            return Ok((key, false));
        }
        let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
        let loaded = textures(&self.screens_archive.read_path(&format!("data\\{path}.rx2"))?)?;
        let wanted = format!("{name}.Texture");
        let mut texture = loaded
            .into_iter()
            .find(|texture| texture.name.eq_ignore_ascii_case(&wanted))
            .ok_or_else(|| format!("image {path}.rx2 lacks {wanted}"))?;
        texture.name = key.clone();
        self.textures.insert(key.clone(), texture);
        self.forget_texture_suffixes();
        Ok((key, true))
    }
    /// MemcardAssetManager career-photo load types 4 (thumbnail) and 5 (full).
    pub(super) fn ensure_career_photo(&mut self, slot: u32, thumbnail: bool) -> RenderResult<Option<(String, bool)>> {
        let key = crate::career_photos::texture_key(slot, thumbnail);
        if self.textures.contains_key(&key) { return Ok(Some((key, false))); }
        let Some(texture) = crate::career_photos::load(slot, thumbnail)? else { return Ok(None) };
        self.textures.insert(key.clone(), texture);
        self.forget_texture_suffixes();
        Ok(Some((key, true)))
    }

    pub(super) fn ensure_replay_thumbnail(&mut self,replay:bool,slot:u32)->RenderResult<(String,bool)> {
        let key=crate::replay_media::texture_key(replay,slot);
        if self.textures.contains_key(&key) {return Ok((key,false));}
        self.textures.insert(key.clone(),crate::replay_media::thumbnail_texture(replay,slot)?);
        self.forget_texture_suffixes();Ok((key,true))
    }
    pub(super) fn ensure_replay_photo(&mut self,slot:u32)->RenderResult<(String,bool)> {
        let key=crate::replay_media::photo_key(slot);
        if self.textures.contains_key(&key) {return Ok((key,false));}
        self.textures.insert(key.clone(),crate::replay_media::photo_texture(slot)?);
        self.forget_texture_suffixes();Ok((key,true))
    }

    pub fn load(mounts: &MountSet) -> RenderResult<Self> {
        // screens.big is mounted separately by AptIntegration, outside the
        // twenty game archive groups. Derive the same game root from Boot.
        let boot = mounts
            .archives()
            .iter()
            .find(|a| {
                a.data_path()
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("miscboot.big"))
            })
            .ok_or("UI requires miscboot mount")?;
        let root = boot
            .data_path()
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .ok_or("invalid Boot archive path")?;
        let screens = MountedBig::open(root.join("data/fe/screens.big"), false)?;
        let cac_archive = MountedBig::open(root.join("data/fe/cas.big"), true)?;
        let locations_archive = MountedBig::open(root.join("data/fe/locations.big"), true)?;
        let hud = "data\\fe\\source\\screens\\hud2\\hudphonelist";
        let button = "data\\fe\\source\\controls\\button_item2";
        let score = "data\\fe\\source\\screens\\hud2\\trickdisplay2";
        let hom = "data\\fe\\source\\screens\\hud2\\homscoring";
        let pause = "data\\fe\\source\\screens\\starthub\\starthubmenu";
        let challenge = "data\\fe\\source\\controls\\challenge_info";
        let picker = "data\\fe\\source\\controls\\menu_picker";
        let button_menu = "data\\fe\\source\\controls\\button_menu";
        let image_renderer = "data\\fe\\source\\controls\\image_renderer";
        let map = "data\\fe\\source\\screens\\map\\challengemap";
        let slider = "data\\fe\\source\\controls\\basic_slider";
        let letterbox = "data\\fe\\source\\screens\\main\\letterbox";
        let dimmer = "data\\fe\\source\\screens\\main\\dimmer";
        let jacked = "data\\fe\\source\\screens\\jacked\\jacked_overlay";
        let central = "data\\fe\\source\\screens\\hud2\\hudcentral";
        let minimap = "data\\fe\\source\\screens\\hud2\\hudminimap";
        let objective = "data\\fe\\source\\screens\\hud2\\hudobjective2";
        let tutorial = "data\\fe\\source\\screens\\hud2\\hudtutorial";
        let gesture = "data\\fe\\source\\controls\\gesture_item";
        let overlay = "data\\fe\\source\\screens\\hud2\\actionoverlay2";
        let poster = "data\\fe\\source\\screens\\challenge\\challenge_signup";
        let unlock = "data\\fe\\source\\screens\\unlock\\unlockoverlay";
        let skate101 = "data\\fe\\source\\screens\\tutorials\\skate101";
        let subtitle = "data\\fe\\source\\screens\\main\\subtitle";
        let gestollie = "data\\fe\\source\\screens\\nis\\gestollie";
        let ownthespot = "data\\fe\\source\\screens\\challenge\\ownthespot";
        let spotfound = "data\\fe\\source\\screens\\challenge\\spotfound";
        let killedit = "data\\fe\\source\\screens\\challenge\\killedit";
        let phototreatment = "data\\fe\\source\\screens\\hudComponents\\phototreatment";
        let pro_bio = "data\\fe\\source\\screens\\nis\\pro_bioinfo_overlay";
        let gvrreg = "data\\fe\\source\\screens\\nis\\gvrreg";
        let gvrgoofy = "data\\fe\\source\\screens\\nis\\gvrgoofy";
        let sanvantv = "data\\fe\\source\\screens\\nis\\sanvantv";
        let fixedphoto = "data\\fe\\source\\screens\\hudComponents\\fixedphoto";
        let nonfixedphoto = "data\\fe\\source\\screens\\hudComponents\\nonfixedphoto";
        let mainmenu = "data\\fe\\source\\screens\\main\\mainmenu";
        let options = "data\\fe\\source\\screens\\options\\options";
        let popup = "data\\fe\\source\\screens\\popup\\sk8popup";
        let edit_skater = "data\\fe\\source\\screens\\cas\\createskater";
        let freeskate = "data\\fe\\source\\screens\\main\\freeskate_options";
        let music_player = "data\\fe\\source\\screens\\musicplayer\\musicplayertest";
        let chyron = "data\\fe\\source\\screens\\main\\chyron";
        let party_play = "data\\fe\\source\\screens\\main\\partyplay";
        let party_play_press = "data\\fe\\source\\screens\\challenge\\partyplaypresscontinue";
        let throwdown_results = "data\\fe\\source\\screens\\challenge\\throwdownresults";
        let timer = "data\\fe\\source\\screens\\hud2\\hudtimer";
        let throwdown_message = "data\\fe\\source\\screens\\hud2\\hudthrowmes";
        let abd = "data\\fe\\source\\screens\\hud2\\hudabd";
        let leaderboard = "data\\fe\\source\\screens\\hud2\\leaderboardhud";
        let countdown = "data\\fe\\source\\screens\\hud2\\hudcountdown";
        let voting = "data\\fe\\source\\screens\\hud2\\hudonlinevoting";
        let end_challenge = "data\\fe\\source\\screens\\hudComponents\\endchallengeoptions";
        let results_list = "data\\fe\\source\\controls\\results_lb";
        let selectable_list = "data\\fe\\source\\controls\\dynamic_selectable_list";
        let list_textbox = "data\\fe\\source\\controls\\list_item_textbox";
        let list_numberbox = "data\\fe\\source\\controls\\list_item_numberbox";
        let list_awardbox = "data\\fe\\source\\controls\\list_item_awardbox";
        let rank_badges = "data\\fe\\source\\controls\\rank_badges";
        let list_bonusbox = "data\\fe\\source\\controls\\list_item_bonusbox";
        let change_game = "data\\fe\\source\\screens\\challenge\\partyplaychangegame";
        let dpo = "data\\fe\\source\\screens\\hud2\\dpo";
        let challenge_failed_small = "data\\fe\\source\\screens\\hudComponents\\challengefailed_small";
        let end_challenge_small = "data\\fe\\source\\screens\\hudComponents\\endchallenge_small";
        let distance = "data\\fe\\source\\screens\\hud2\\huddistance";
        let photo_select = "data\\fe\\source\\screens\\challenge\\photoselect";
        let receipt = "data\\fe\\source\\screens\\cas\\receipt";
        let sponsor_overlay = "data\\fe\\source\\screens\\unlock\\sponsoroverlay";
        let sponsor_browser = "data\\fe\\source\\controls\\sponsor_browser";
        let slappys_blog = "data\\fe\\source\\screens\\slappys\\slappys_blog";
        let slappys_articles = "data\\fe\\source\\screens\\slappys\\articles";
        let slappys_covers = "data\\fe\\source\\screens\\slappys\\covers";
        let end_gvr = "data\\fe\\source\\screens\\hudComponents\\endchallenge_gvr";
        let challenge_failed = "data\\fe\\source\\screens\\hudComponents\\challengefailed";
        let cac_controls = [
            "highlight_morph",
            "highlight_color",
            "highlight_selector",
            "highlight_category",
            "highlight_option",
            "highlight_slider",
            "highlight_arrow_left",
            "highlight_arrow_right",
            "swatch_renderer",
        ];
        let cac_paths = cac_controls.map(|name| format!("data\\fe\\source\\controls\\{name}"));
        let mut movies = [
            movie(&screens, hud)?,
            movie(&screens, button)?,
            movie(&screens, score)?,
            movie(&screens, hom)?,
            movie(&screens, pause)?,
            movie(&screens, challenge)?,
            movie(&screens, picker)?,
            movie(&screens, button_menu)?,
            movie(&screens, image_renderer)?,
            movie(&screens, map)?,
            movie(&screens, slider)?,
            movie(&screens, letterbox)?,
            movie(&screens, dimmer)?,
            movie(&screens, jacked)?,
            movie(&screens, central)?,
            movie(&screens, edit_skater)?,
            movie(&screens, &cac_paths[0])?,
            movie(&screens, &cac_paths[1])?,
            movie(&screens, &cac_paths[2])?,
            movie(&screens, &cac_paths[3])?,
            movie(&screens, &cac_paths[4])?,
            movie(&screens, &cac_paths[5])?,
            movie(&screens, &cac_paths[6])?,
            movie(&screens, &cac_paths[7])?,
            movie(&screens, &cac_paths[8])?,
            movie(&screens, popup)?,
            movie(&screens, minimap)?,
            movie(&screens, mainmenu)?,
            movie(&screens, options)?,
            movie(&screens, objective)?,
            movie(&screens, tutorial)?,
            movie(&screens, gesture)?,
            movie(&screens, overlay)?,
            movie(&screens, poster)?,
            movie(&screens, unlock)?,
            movie(&screens, skate101)?,
            movie(&screens, subtitle)?,
            movie(&screens, gestollie)?,
            movie(&screens, ownthespot)?,
            movie(&screens, spotfound)?,
            movie(&screens, killedit)?,
            movie(&screens, phototreatment)?,
            movie(&screens, pro_bio)?,
            movie(&screens, gvrreg)?,
            movie(&screens, gvrgoofy)?,
            movie(&screens, sanvantv)?,
            movie(&screens, fixedphoto)?,
            movie(&screens, nonfixedphoto)?,
            movie(&screens, freeskate)?,
            movie(&screens, music_player)?,
            movie(&screens, chyron)?,
            movie(&screens, party_play)?,
            movie(&screens, party_play_press)?,
            movie(&screens, throwdown_results)?,
            movie(&screens, timer)?,
            movie(&screens, throwdown_message)?,
            movie(&screens, abd)?,
            movie(&screens, leaderboard)?,
            movie(&screens, countdown)?,
            movie(&screens, voting)?,
            movie(&screens, end_challenge)?,
            movie(&screens, results_list)?,
            movie(&screens, selectable_list)?,
            movie(&screens, list_textbox)?,
            movie(&screens, list_numberbox)?,
            movie(&screens, list_awardbox)?,
            movie(&screens, rank_badges)?,
            movie(&screens, list_bonusbox)?,
            movie(&screens, change_game)?,
            movie(&screens, dpo)?,
            movie(&screens, challenge_failed_small)?,
            movie(&screens, end_challenge_small)?,
            movie(&screens, distance)?,
            movie(&screens, photo_select)?,
            movie(&screens, receipt)?,
            movie(&screens, sponsor_overlay)?,
            movie(&screens, sponsor_browser)?,
            movie(&screens, slappys_blog)?,
            movie(&screens, slappys_articles)?,
            movie(&screens, slappys_covers)?,
            movie(&screens, challenge_failed)?,
            movie(&screens, "data\\fe\\source\\screens\\hudComponents\\hom_results")?,
            movie(&screens, "data\\fe\\source\\controls\\skeleton")?,
            movie(&screens, "data\\fe\\source\\screens\\hom\\annalsofmeat")?,
            movie(&screens, end_gvr)?,
            movie(&screens, "data\\fe\\source\\screens\\main\\extras")?,
            movie(&screens, "data\\fe\\source\\screens\\extras\\credits")?,
            movie(&screens, "data\\fe\\source\\screens\\movieplayer\\movieplayer")?,
            movie(&screens, "data\\fe\\source\\screens\\movieplayer\\tutbasreplay")?,
            movie(&screens, "data\\fe\\source\\screens\\movieplayer\\tutmap")?,
            movie(&screens, "data\\fe\\source\\screens\\movieplayer\\tutcreatespot")?,
            movie(&screens, "data\\fe\\source\\screens\\movieplayer\\tutadvreplay")?,
            movie(&screens, "data\\fe\\source\\screens\\movieplayer\\tuthom")?,
            movie(&screens, "data\\fe\\source\\screens\\replay2\\replay_editor2")?,
            movie(&screens, "data\\fe\\source\\screens\\skatereel\\skatereel_myfootage")?,
            movie(&screens, "data\\fe\\source\\controls\\media_browser")?,
            movie(&screens, "data\\fe\\source\\controls\\mediathumb")?,
            movie(&screens, "data\\fe\\source\\screens\\main\\main_onlineoptions")?,
            movie(&screens, "data\\fe\\source\\screens\\skatereel\\skatereel_general")?,
            movie(&screens, "data\\fe\\source\\screens\\skatereel\\skatereel_photo")?,
            movie(&screens, "data\\fe\\source\\screens\\tricks\\trickguide")?,
            movie(&screens, "data\\fe\\source\\screens\\phonelist\\phonelist")?,
            movie(&screens, "data\\fe\\source\\controls\\basic_menu_picker")?,
            movie(&screens, "data\\fe\\source\\screens\\hud2\\bbtimer")?,
            movie(&screens, "data\\fe\\source\\screens\\hud2\\hud_online_eventlog2")?,
            movie(&screens, "data\\fe\\source\\screens\\unranked\\unranked_custom_match")?,
            movie(&screens, "data\\fe\\source\\screens\\unranked\\unranked_custom_freeskate")?,
            movie(&screens, "data\\fe\\source\\controls\\online_gamer_tag")?,
            movie(&screens, "data\\fe\\source\\screens\\onlinehub\\onlineHubMenu")?,
            movie(&screens, "data\\fe\\source\\controls\\hub_tree")?,
            movie(&screens, "data\\fe\\source\\controls\\hub_menu")?,
            movie(&screens, "data\\fe\\source\\controls\\background_scaler")?,
            movie(&screens, "data\\fe\\source\\screens\\ranked\\ranked_results_overall")?,
            movie(&screens, "data\\fe\\source\\controls\\challengesignup_icon")?,
            movie(&screens, "data\\fe\\source\\screens\\onlinehub\\skateProfile")?,
            movie(&screens, "data\\fe\\source\\controls\\online_gamer_rank")?,
            movie(&screens, "data\\fe\\source\\screens\\hudComponents\\spectator_mode")?,
            movie(&screens, "data\\fe\\source\\screens\\hud2\\hud_freeskateactivity_overlay")?,
            movie(&screens, "data\\fe\\source\\screens\\unranked\\freeskateactivity_signup")?,
            movie(&screens, "data\\fe\\source\\screens\\main\\dlc")?,
        ];
        for (asset, expected) in movies.iter_mut().zip([
            0xf1d585f8u32,
            0xe397484c,
            0x58c4b20b,
            0x65f4830a,
            0x4381187e,
            0x5273d80c,
            0xf52d4e28,
            0x1ee55d34,
            0x94517628,
            0x85446dde,
            0x8b4d0429,
            0x4998ec77,
            0xdd3f773e,
            0xf5a0ff8e,
            0xd85e4167,
            0xbb8d5810,
            0x439f56fd,
            0xde96933e,
            0x2db11cac,
            0x28fd6d36,
            0xcc07f13b,
            0x817fa369,
            0x0c0a026e,
            0xabb8cea2,
            0x703ff60b,
            0xe82ed6ec,
            0x8d92a68f,
            0x4860d06f,
            0x9506be8f,
            0xc7731a08,
            0x5ca788c6,
            0x3577d433,
            0x9bd9bc9c,
            0x711e9b3c,
            0x07458651,
            0xfbc3f712,
            0xd847264a,
            0x4e5752ee,
            0x065eb3bf,
            0x7ced72b1,
            0xc36cd0da,
            0x56133c80,
            0xb9a276d2,
            0xe58a8c6e,
            0x9fa95bcf,
            0x45497183,
            0xba88d09d,
            0x6218005f,
            0xe7de5095,
            0x18056383,
            0xce2ea7f1,
            0xe1735e94,
            0x6b5d9b87,
            0xa1423ba4,
            0x19a5bcd1,
            0x0c72e72c,
            0xf7b8bcef,
            0xc381485c,
            0xb09d0ef1,
            0xf7435871,
            0x3fe11302,
            0x71a2e327,
            0x8a7d1d4a,
            0xbd0adbab,
            0xd1474c40,
            0x6ccb9caf,
            0xbcbad545,
            0xd8eb1885,
            0x1f90c718,
            0xad28a23d,
            0x087b_86f0,
            0x916b_324b,
            0x1aa3_f3be,
            0x80d8_ba48,
            0xfe6c_4d2e,
            0x139c_3d55,
            0x4f6c_ce94,
            0x632b_87a9,
            0xd663_2366,
            0xad9d_0792,
            0xeb5a_8c10,
            0x5b42_76e3,
            0x7b44_af51,
            0xb0e2_d7f4,
            0x3bce_2a31,
            0x7031_d76f,
            0x1402_d4b0,
            0xf5e4_95b6,
            0x0d51_f2a9,
            0x87ff_c45e,
            0x40f6_69fd,
            0x8e92_ecf0,
            0x15bd_a746,
            0x9cb5_95f1,
            0xdbdd_8cad,
            0x37d5_b789,
            0x7264_0e9d,
            0x04e9_e0b3,
            0xafcf_3ebc,
            0xaecf_c0a2,
            0xf306_7239,
            0xfbbb_45c1,
            0x77c9_6ddf,
            0xbcb2eeb7,
            0x33ac_45c2,
            0xf1cd_6f30,
            0xb8ca_cbd1,
            0xc972_6e6b,
            0x66f8_fc9d,
            0xb55d_1026,
            0xa135_e0bf,
            0x4cfe_92e1,
            0x203d_2cc7,
            0xed95_56a4,
            0x56ba_b8fa,
            0x0717_ecf4,
            0xa968_491a,
            0xabb9_425f,
            0x749c_a799,
            0xd5aa_4818,
        ]) {
            let fingerprint = asset.bytes.iter().fold(2166136261u32, |h, b| {
                (h ^ u32::from(*b)).wrapping_mul(16777619)
            });
            if fingerprint != expected {
                let Some(&(_, _, moved)) = RETAIL_EQUIVALENTS
                    .iter()
                    .find(|&&(verified, retail, _)| (verified, retail) == (expected, fingerprint))
                else {
                    return Err(
                        "marker ActionScript asset differs from verified translation".into(),
                    );
                };
                asset.moved_actions = moved;
            }
        }
        let mut texture_map = BTreeMap::new();
        for base in std::iter::once(edit_skater.to_string()).chain(cac_paths[..8].iter().cloned()) {
            for texture in textures(&screens.read_path(&format!("{base}.rx2"))?)? {
                texture_map.insert(texture.name.to_ascii_lowercase(), texture);
            }
        }
        for path in [
            format!("{hud}.rx2"),
            format!("{minimap}.rx2"),
            format!("{objective}.rx2"),
            format!("{tutorial}.rx2"),
            format!("{gesture}.rx2"),
            format!("{overlay}.rx2"),
            format!("{poster}.rx2"),
            format!("{unlock}.rx2"),
            format!("{skate101}.rx2"),
            format!("{subtitle}.rx2"),
            format!("{gestollie}.rx2"),
            format!("{ownthespot}.rx2"),
            format!("{spotfound}.rx2"),
            format!("{killedit}.rx2"),
            format!("{photo_select}.rx2"),
            "data\\fe\\source\\screens\\hudComponents\\hom_results.rx2".into(),
            "data\\fe\\source\\controls\\skeleton.rx2".into(),
            "data\\fe\\source\\screens\\hom\\annalsofmeat.rx2".into(),
            "data\\fe\\source\\screens\\movieplayer\\tutbasreplay.rx2".into(),
            "data\\fe\\source\\screens\\movieplayer\\tutmap.rx2".into(),
            "data\\fe\\source\\screens\\movieplayer\\tutcreatespot.rx2".into(),
            "data\\fe\\source\\screens\\movieplayer\\tutadvreplay.rx2".into(),
            "data\\fe\\source\\screens\\movieplayer\\tuthom.rx2".into(),
            "data\\fe\\source\\screens\\replay2\\replay_editor2.rx2".into(),
            "data\\fe\\source\\screens\\skatereel\\skatereel_myfootage.rx2".into(),
            "data\\fe\\source\\controls\\media_browser.rx2".into(),
            "data\\fe\\source\\controls\\mediathumb.rx2".into(),
            "data\\fe\\source\\screens\\main\\main_onlineoptions.rx2".into(),
            "data\\fe\\source\\screens\\main\\dlc.rx2".into(),
            "data\\fe\\source\\screens\\skatereel\\skatereel_general.rx2".into(),
            "data\\fe\\source\\screens\\skatereel\\skatereel_photo.rx2".into(),
            "data\\fe\\source\\screens\\tricks\\trickguide.rx2".into(),
            "data\\fe\\source\\screens\\phonelist\\phonelist.rx2".into(),
            "data\\fe\\source\\controls\\basic_menu_picker.rx2".into(),
            format!("{receipt}.rx2"),
            format!("{sponsor_overlay}.rx2"),
            format!("{sponsor_browser}.rx2"),
            format!("{slappys_blog}.rx2"),
            format!("{slappys_articles}.rx2"),
            format!("{slappys_covers}.rx2"),
            format!("{phototreatment}.rx2"),
            format!("{pro_bio}.rx2"),
            format!("{gvrreg}.rx2"),
            format!("{gvrgoofy}.rx2"),
            format!("{sanvantv}.rx2"),
            format!("{fixedphoto}.rx2"),
            format!("{nonfixedphoto}.rx2"),
            format!("{mainmenu}.rx2"),
            format!("{music_player}.rx2"),
            format!("{chyron}.rx2"),
            format!("{party_play}.rx2"),
            format!("{party_play_press}.rx2"),
            format!("{throwdown_results}.rx2"),
            format!("{timer}.rx2"),
            "data\\fe\\source\\screens\\hud2\\bbtimer.rx2".into(),
            "data\\fe\\source\\screens\\hud2\\hud_online_eventlog2.rx2".into(),
            "data\\fe\\source\\screens\\hud2\\hud_freeskateactivity_overlay.rx2".into(),
            "data\\fe\\source\\screens\\unranked\\freeskateactivity_signup.rx2".into(),
            "data\\fe\\source\\screens\\unranked\\unranked_custom_match.rx2".into(),
            "data\\fe\\source\\screens\\unranked\\unranked_custom_freeskate.rx2".into(),
            "data\\fe\\source\\screens\\onlinehub\\onlineHubMenu.rx2".into(),
            "data\\fe\\source\\controls\\hub_tree.rx2".into(),
            "data\\fe\\source\\controls\\hub_menu.rx2".into(),
            "data\\fe\\source\\controls\\background_scaler.rx2".into(),
            "data\\fe\\source\\screens\\ranked\\ranked_results_overall.rx2".into(),
            "data\\fe\\source\\controls\\challengesignup_icon.rx2".into(),
            "data\\fe\\source\\screens\\onlinehub\\skateProfile.rx2".into(),
            "data\\fe\\source\\controls\\online_gamer_rank.rx2".into(),
            format!("{throwdown_message}.rx2"),
            format!("{abd}.rx2"),
            format!("{leaderboard}.rx2"),
            format!("{countdown}.rx2"),
            format!("{voting}.rx2"),
            format!("{end_challenge}.rx2"),
            format!("{end_gvr}.rx2"),
            format!("{selectable_list}.rx2"),
            format!("{rank_badges}.rx2"),
            format!("{change_game}.rx2"),
            format!("{dpo}.rx2"),
            format!("{challenge_failed}.rx2"),
            format!("{challenge_failed_small}.rx2"),
            format!("{end_challenge_small}.rx2"),
            format!("{popup}.rx2"),
            format!("{score}.rx2"),
            format!("{hom}.rx2"),
            format!("{pause}.rx2"),
            format!("{challenge}.rx2"),
            format!("{picker}.rx2"),
            format!("{image_renderer}.rx2"),
            format!("{map}.rx2"),
            format!("{slider}.rx2"),
            format!("{letterbox}.rx2"),
            format!("{dimmer}.rx2"),
            format!("{jacked}.rx2"),
            "data\\fe\\source\\images\\buttons\\xbox360\\buttons.rx2".into(),
            "data\\fe\\source\\images\\icons\\minimap.rx2".into(),
        ] {
            for texture in textures(&screens.read_path(&path)?)? {
                texture_map.insert(texture.name.to_ascii_lowercase(), texture);
            }
        }
        // ChallengeMap::LoadMap8264ccf8 selects fe_locations/default.
        // Read original MapInfo including the relocated filename separately.
        let map_info = with_vault(mounts, |v| {
            let class = 0xddcd6bc23255e508;
            let collection = 0xd7edbd362d7d2152;
            let key = 0x20bbc594cd2faef8;
            let name = v.string(class, collection, key)?;
            let data = v.bytes(class, collection, key, 28)?;
            let mut words = [0i32; 6];
            for (word, raw) in words.iter_mut().zip(data[4..].chunks_exact(4)) {
                *word = i32::from_be_bytes(raw.try_into()?);
            }
            Ok((name, words))
        })?;
        let (map_name, words) = map_info;
        for texture in textures(
            &screens.read_path(&format!("data\\fe\\source\\images\\map\\{map_name}.rx2"))?,
        )? {
            texture_map.insert(texture.name.to_ascii_lowercase(), texture);
        }
        let map_info = MapInfo {
            texture: format!("{map_name}.Texture"),
            extents: [words[0], words[1], words[2], words[3]],
            pixels: [words[4], words[5]],
        };
        // MapIconManager::Initialize82635c00 selects the original local icon.
        let local_player_icon = with_vault(mounts, |v| {
            Ok(format!(
                "{}.Texture",
                v.string(0xf788504a8d922ced, 0x8f7f7c8ecf36fd3b, 0xfcfe787ccf2f775e)?
            ))
        })?;
        // InitializeFontDetails8282b930 / default region0 group. Use full
        // native 64-bit keys, inheritance and pointer fixups of the same
        // Vault reader used by world materials; no substituted host fonts.
        let (fonts, font_fallback) = with_vault(mounts, |v| {
            let group = 0x4e432e4cbff3aeca;
            let collection = 0x14df58a3f395690f;
            let refs = 0x5a6eb4d57b7f4ea3;
            let header = v.bytes(group, collection, refs, 8)?;
            let count = u16::from_be_bytes(header[0..2].try_into()?) as usize;
            if u16::from_be_bytes(header[4..6].try_into()?) != 24 {
                return Err("font group stride".into());
            }
            let data = v.bytes(group, collection, refs, 8 + count * 24)?;
            let mut fonts = BTreeMap::new();
            let mut fallback = None;
            for (index, r) in data[8..].chunks_exact(24).enumerate() {
                let class = u64::from_be_bytes(r[0..8].try_into()?);
                let key = u64::from_be_bytes(r[8..16].try_into()?);
                let name = v.string(class, key, 0x340832ccfd9fdeb4)?;
                let file = v.string(class, key, 0x1971fb20f9154669)?;
                if fallback.is_none() && !file.eq_ignore_ascii_case("debug") {
                    fallback = Some(name.clone());
                }
                let base = format!("data\\fe\\fonts\\{file}");
                let metrics = BitmapFont::parse(&mounts.read_path(&format!("{base}.bmpFont"))?)?;
                let atlas = textures(&mounts.read_path(&format!("{base}.rx2"))?)?;
                if atlas.len() != 1 {
                    return Err(format!("font {file} must have one atlas").into());
                }
                let texture = atlas[0].name.to_ascii_lowercase();
                for t in atlas {
                    texture_map.insert(t.name.to_ascii_lowercase(), t);
                }
                fonts.insert(
                    name,
                    Font {
                        metrics,
                        texture,
                        file,
                        scale: if index == 0 {
                            [0.8; 2]
                        } else {
                            [
                                v.floats::<1>(class, key, 0xc1eb472fb491ce25)?[0],
                                v.floats::<1>(class, key, 0x6aafca9d264797dc)?[0],
                            ]
                        },
                        offset: [
                            v.floats::<1>(class, key, 0xa6c19e3e72540a37)?[0],
                            v.floats::<1>(class, key, 0x848992e89fba6ff7)?[0],
                        ],
                        line_height_scale: v.floats::<1>(class, key, 0x672e35a2aa8c4a00)?[0],
                    },
                );
            }
            Ok((fonts, fallback.ok_or("font group has no Apt font")?))
        })?;
        let lang = "data\\fe\\languages\\english\\LANGUAGE_English";
        let histogram =
            Histogram::parse(&mounts.read_path(&format!("{lang}_HISTOGRAM_skate2ng.BIN"))?)?;
        let global_bytes = mounts.read_path(&format!("{lang}_Global_skate2ng.BIN"))?;
        let frontend_bytes = mounts.read_path(&format!("{lang}_FrontEnd_skate2ng.BIN"))?;
        let categories = [
            LanguageCategory::parse(&global_bytes)?,
            LanguageCategory::parse(&frontend_bytes)?,
        ];
        let mut strings = BTreeMap::new();
        let mut localized = BTreeMap::new();
        for category in &categories {
            for (hash, packed) in category.packed_strings() {
                let decoded = histogram.unpack(packed, 16384, false)?;
                if !decoded.valid || !decoded.terminated {
                    return Err("invalid UI localized text".into());
                }
                localized.entry(hash).or_insert(decoded.units);
            }
        }
        // PC: the linked downloadable content's language databases, after the game's.
        for (hash, units) in skate2_core::dlc::strings(mounts) {
            localized.entry(hash).or_insert(units);
        }
        for label in [
            "ID_PHONELIST_TITLE",
            "ID_PHONELIST_GOTO_MARKER",
            "ID_PHONELIST_PLACE_MARKER",
            "ID_PHONELIST_OPTIONS_TITLE",
            "ID_PHONELIST_USE_PHONE",
            "ID_PHONELIST_ANYWHERE_ALERT",
            "ID_PHONELIST_SPONSORSHIP_ALERT",
        ] {
            let packed = categories
                .iter()
                .find_map(|c| c.packed_string(string_hash(label.as_bytes())))
                .ok_or_else(|| format!("missing UI string {label}"))?;
            let decoded = histogram.unpack(packed, 1024, false)?;
            if !decoded.valid || !decoded.terminated {
                return Err("invalid marker text".into());
            }
            strings.insert(label.into(), decoded.units);
        }
        Ok(Self {
            cac_archive,
            locations_archive,
            dlc_archives: mounts.dlc_archives(),
            screens_archive: screens,
            map_info,
            local_player_icon,
            profile_name: String::new(),pro_film_pro_name:String::new(),
            sponsor_labels: std::array::from_fn(|_| "ID_COMMON_UNKNOWN".to_owned()),
            frontend_arguments: Default::default(),
            movies,
            // Original char file uses single-byte Western European names, not UTF-8.
            credits_source: mounts.read_path("data\\credits\\credits.txt")?.into_iter().map(char::from).collect(),
            fonts,
            font_fallback,
            textures: texture_map,
            texture_suffixes: std::sync::Mutex::default(),
            strings,
            localized,
        })
    }
    /// SkateAptString82a70b28 resolves authored text as well as AS text.
    /// LanguageManager82d66a78 sets the missing-string fallback to the single
    /// space at8233afe8 (gEMPTY_STRING), not the untranslated identifier.
    /// `cFontManager::GetFontByAptName` (`0x82851790`): the font whose Flash
    /// name matches ignoring case, else the first that is not the debug
    /// font (0x82851810..0x82851880).
    pub fn apt_font(&self, name: &str) -> RenderResult<&Font> {
        self.fonts
            .iter()
            .find(|(flash, _)| flash.eq_ignore_ascii_case(name))
            .map(|(_, font)| font)
            .or_else(|| self.fonts.get(&self.font_fallback))
            .ok_or_else(|| "Apt font fallback absent".into())
    }
    /// `cFontManager::GetFontByName` (`0x82851738`): the font whose file name matches ignoring
    /// case (`GetFontIndex` 0x828257a0).
    pub fn font_by_file(&self, file: &str) -> RenderResult<&Font> {
        self.fonts
            .values()
            .find(|font| font.file.eq_ignore_ascii_case(file))
            .ok_or_else(|| format!("font {file} absent").into())
    }
    /// The mounted downloadable content's strings, after the game's own (a link refreshing
    /// them as `StateDLCFlow::LoadDLCLanguageDB` does), and its archives' images.
    pub(super) fn add_dlc_strings(&mut self, mounts: &MountSet) {
        self.dlc_archives = mounts.dlc_archives();
        for (hash, units) in skate2_core::dlc::strings(mounts) {
            self.localized.entry(hash).or_insert(units);
        }
    }
    pub fn initial_text(&self, label: &str) -> Vec<u16> {
        if let Some(literal) = label.strip_prefix('#') {
            return literal.encode_utf16().collect();
        }
        self.localized
            .get(&string_hash(label.as_bytes()))
            .cloned()
            .unwrap_or_else(|| vec![u16::from(b' ')])
    }
    pub fn localize(&self, label: &str) -> RenderResult<String> {
        if let Some(literal) = label.strip_prefix('#') {
            return Ok(literal.into());
        }
        let units = self
            .localized
            .get(&string_hash(label.as_bytes()))
            .ok_or_else(|| format!("missing original localized UI string {label}"))?;
        Ok(String::from_utf16(units)?)
    }
    pub fn texture(&self, suffix: &str) -> RenderResult<&NativeTexture> {
        let mut memo = self
            .texture_suffixes
            .lock()
            .map_err(|_| "APT texture lookup lock poisoned")?;
        if !memo.contains_key(suffix) {
            // Whole trailing path components only: nonfixedphoto_textures\1 also ends with
            // fixedphoto_textures\1.
            let lower = suffix.to_ascii_lowercase();
            let mut found = self.textures.keys().filter(|name| {
                name.strip_suffix(lower.as_str()).is_some_and(|rest| rest.is_empty() || rest.ends_with(['\\', '/']))
            });
            let answer = match (found.next(), found.next()) {
                (None, _) => SuffixMatch::Absent,
                (Some(key), None) => SuffixMatch::Key(key.clone()),
                (Some(_), Some(_)) => SuffixMatch::Ambiguous,
            };
            memo.insert(suffix.to_owned(), answer);
        }
        match &memo[suffix] {
            SuffixMatch::Key(key) => self
                .textures
                .get(key)
                .ok_or_else(|| format!("APT texture {} absent", suffix.to_ascii_lowercase()).into()),
            SuffixMatch::Absent => Err(format!("APT texture {} absent", suffix.to_ascii_lowercase()).into()),
            SuffixMatch::Ambiguous => {
                Err(format!("ambiguous APT texture {}", suffix.to_ascii_lowercase()).into())
            }
        }
    }

    fn forget_texture_suffixes(&mut self) {
        if let Ok(memo) = self.texture_suffixes.get_mut() {
            memo.clear();
        }
    }
}
