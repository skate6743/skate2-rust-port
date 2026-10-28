//! Native SKATER.P container and CAC record serialization.
//! Unknown backend chunks and the closet/customization suffix are retained.
use std::fmt;

pub const PROFILE_SIZE: usize = 0x400004;
pub const PART_SLOTS: usize = 26;

/// Intentional PC options, not fields in ProfileBackEnd::Settings v3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcDisplaySettings {
    pub postfx: bool,
    pub fisheye: bool,
    pub frame_rate_cap: FrameRateCap,
    /// The frame rate counter in the window's corner.
    pub fps_counter: bool,
    /// A borderless window over the whole monitor (Alt+Enter toggles it as well).
    pub fullscreen: bool,
    /// The 3D scene's size in percent of the window, from `RESOLUTION_SCALES` in steps of
    /// `RESOLUTION_SCALE_STEP`; the HUD and menus keep the window's.
    pub resolution_scale: u8,
    /// Each frame waits for the display's refresh, so none tears; off, frames show at once.
    pub vsync: bool,
}
impl Default for PcDisplaySettings {
    fn default() -> Self {
        Self { postfx: true, fisheye: true, frame_rate_cap: FrameRateCap::RefreshRate, fps_counter: false, fullscreen: false,
            resolution_scale: 100, vsync: true }
    }
}
/// The resolution scales Display Settings offers, in percent.
pub const RESOLUTION_SCALES: std::ops::RangeInclusive<u8> = 50..=200;
pub const RESOLUTION_SCALE_STEP: u8 = 10;
/// The most frames presented a second: no cap; 60, the original's rate (`RenderMan::Render`
/// 0x82858360 presents every refresh of the 60 Hz display outside its 30 Hz mode); 120; or one
/// each refresh. Before VSync had its own setting (version 7), Refresh Rate was the only cap
/// that waited for the refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameRateCap {
    Unlimited = 0,
    Fps60 = 1,
    Fps120 = 2,
    RefreshRate = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError(pub &'static str);
impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ProfileError {}
type Result<T> = std::result::Result<T, ProfileError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacPartId {
    pub model: u64,
    pub material: u64,
}
impl CacPartId {
    /// Dynamic initializer82f7e278..82f7e2e0.
    pub fn invalid() -> Self {
        let id = crate::hash::rw_hash64_string("Null Asset ID");
        Self {
            model: id,
            material: id,
        }
    }
}

/// RecipeData::Pack 82b657b0. Float bits preserve all source values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeData {
    pub recipe: Vec<u8>,
    pub tuning_bits: [u32; 2],
    pub hat_type: u32,
    pub stance: u32,
    pub style: u32,
    pub gestures: [u32; 4],
    pub is_male: bool,
}

/// CACRecord::Serialize 82b69768 / Deserialize 82b6bca0, version 3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacRecord {
    pub data: RecipeData,
    pub thumbnail: Vec<u8>,
    pub thumbnail_size: u32,
    pub installed_parts: [CacPartId; PART_SLOTS],
    pub owned_parts: [Vec<CacPartId>; PART_SLOTS],
    pub colour_ids: [u64; 12],
    pub skin_normal: [u8; 64],
    /// Closet::Serialize followed by Customizations::Serialize. Preserved
    /// verbatim until the caller explicitly replaces either native payload.
    pub closet_and_customizations: Vec<u8>,
    unused_owned_slots: Vec<u8>,
}

impl CacRecord {
    /// Locate the customization serializer after Closet v1 and Outfit v3.
    /// Outfit82b64b68 writes 17060 bytes; Vector3 helper82a6e330 writes12.
    fn customization_offset(&self) -> Result<usize> {
        let bytes = &self.closet_and_customizations;
        if read32(bytes, 0)? != 1 {
            return Err(ProfileError("unsupported closet version"));
        }
        let count = read32(bytes, 12)? as usize;
        let size = count
            .checked_mul(17060)
            .and_then(|n| n.checked_add(16))
            .ok_or(ProfileError("closet size overflow"))?;
        if size > bytes.len() {
            return Err(ProfileError("truncated closet"));
        }
        for index in 0..count {
            if read32(bytes, 16 + index * 17060)? != 3 {
                return Err(ProfileError("unsupported outfit version"));
            }
        }
        Ok(size)
    }
    pub fn customizations_bytes(&self) -> Result<&[u8]> {
        Ok(&self.closet_and_customizations[self.customization_offset()?..])
    }
    pub fn replace_customizations(&mut self, serialized: &[u8]) -> Result<()> {
        let offset = self.customization_offset()?;
        self.closet_and_customizations.truncate(offset);
        self.closet_and_customizations.extend_from_slice(serialized);
        Ok(())
    }
    /// Construct with explicitly supplied native customization/closet bytes.
    /// Native defaults: Init82b6a9e0, InitPlayer82573bb0, GestureSet82561d70.
    pub fn new(
        data: RecipeData,
        invalid_part: CacPartId,
        closet_and_customizations: Vec<u8>,
    ) -> Self {
        let mut unused_owned_slots = Vec::with_capacity(6000 * 16);
        for _ in 0..6000 {
            part_write(&mut unused_owned_slots, invalid_part);
        }
        Self {
            data,
            thumbnail: vec![0; 15000],
            thumbnail_size: 0,
            installed_parts: [invalid_part; 26],
            owned_parts: std::array::from_fn(|_| Vec::new()),
            colour_ids: [u64::MAX; 12],
            skin_normal: [0; 64],
            closet_and_customizations,
            unused_owned_slots,
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader { bytes, pos: 0 };
        if r.u32()? != 3 {
            return Err(ProfileError("unsupported CACRecord version"));
        }
        let outer_recipe_size = r.u32()?;
        if r.u32()? != 7 {
            return Err(ProfileError("unsupported RecipeData version"));
        }
        let recipe_size = r.u32()? as usize;
        if recipe_size > 8192 || outer_recipe_size as usize != recipe_size {
            return Err(ProfileError("invalid CAC recipe size"));
        }
        let recipe = r.take(recipe_size)?.to_vec();
        let tuning_bits = [r.u32()?, r.u32()?];
        let hat_type = r.u32()?;
        let stance = r.u32()?;
        let style = r.u32()?;
        let gestures = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
        let is_male = r.take(1)?[0] != 0;
        let thumbnail = r.take(15000)?.to_vec();
        let thumbnail_size = r.u32()?;
        let mut installed_parts = [CacPartId {
            model: 0,
            material: 0,
        }; 26];
        for part in &mut installed_parts {
            *part = r.part()?;
        }
        let mut owned_parts: [Vec<CacPartId>; 26] = std::array::from_fn(|_| Vec::new());
        let mut total = 0usize;
        for parts in &mut owned_parts {
            let count = r.u32()? as usize;
            total = total
                .checked_add(count)
                .ok_or(ProfileError("owned part count overflow"))?;
            if total > 6000 {
                return Err(ProfileError("CAC owned parts exceed native capacity"));
            }
            for _ in 0..count {
                parts.push(r.part()?);
            }
        }
        let unused_owned_slots = r.take((6000 - total) * 16)?.to_vec();
        let mut colour_ids = [0; 12];
        for id in &mut colour_ids {
            *id = r.u64()?;
        }
        let skin_normal = r.take(64)?.try_into().expect("length checked");
        let closet_and_customizations = r.take(bytes.len() - r.pos)?.to_vec();
        if closet_and_customizations.len() < 16 {
            return Err(ProfileError("missing CAC closet/customizations"));
        }
        Ok(Self {
            data: RecipeData {
                recipe,
                tuning_bits,
                hat_type,
                stance,
                style,
                gestures,
                is_male,
            },
            thumbnail,
            thumbnail_size,
            installed_parts,
            owned_parts,
            colour_ids,
            skin_normal,
            closet_and_customizations,
            unused_owned_slots,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let total: usize = self.owned_parts.iter().map(Vec::len).sum();
        if total >= 6000 || self.data.recipe.len() > 8192 || self.thumbnail.len() != 15000 {
            return Err(ProfileError("CAC record exceeds native capacity"));
        }
        let mut b = Vec::new();
        put32(&mut b, 3);
        put32(&mut b, self.data.recipe.len() as u32);
        put32(&mut b, 7);
        put32(&mut b, self.data.recipe.len() as u32);
        b.extend_from_slice(&self.data.recipe);
        for x in self.data.tuning_bits {
            put32(&mut b, x);
        }
        for x in [self.data.hat_type, self.data.stance, self.data.style] {
            put32(&mut b, x);
        }
        for x in self.data.gestures {
            put32(&mut b, x);
        }
        b.push(u8::from(self.data.is_male));
        b.extend_from_slice(&self.thumbnail);
        put32(&mut b, self.thumbnail_size);
        for x in self.installed_parts {
            part_write(&mut b, x);
        }
        for parts in &self.owned_parts {
            put32(&mut b, parts.len() as u32);
            for x in parts {
                part_write(&mut b, *x);
            }
        }
        let padding = (6000 - total) * 16;
        if self.unused_owned_slots.len() != padding {
            return Err(ProfileError(
                "owned part edits require rebuilding native unused slots",
            ));
        }
        b.extend_from_slice(&self.unused_owned_slots);
        for x in self.colour_ids {
            b.extend_from_slice(&x.to_be_bytes());
        }
        b.extend_from_slice(&self.skin_normal);
        b.extend_from_slice(&self.closet_and_customizations);
        Ok(b)
    }
    pub fn rebuild_unused_owned_slots(&mut self, invalid_part: CacPartId) -> Result<()> {
        let count: usize = self.owned_parts.iter().map(Vec::len).sum();
        if count >= 6000 {
            return Err(ProfileError("CAC owned parts exceed native capacity"));
        }
        self.unused_owned_slots.clear();
        for _ in count..6000 {
            part_write(&mut self.unused_owned_slots, invalid_part);
        }
        Ok(())
    }
}

/// One native leaf chunk (`StartWriteChunk`..`EndWriteChunk`8255f9e0): the
/// version and the `Write*` tuples, read up to the zero-size terminator as
/// `GetNextTuple`82569e30 does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemcardLeaf {
    pub version: u32,
    pub tuples: Vec<Vec<u8>>,
}

/// `ProfileBackEnd::tSettings` as `SaveSettings`8255bc68 writes it (v3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileSettings {
    pub sfx_volume: f32,
    pub dialogue_volume: f32,
    pub music_volume: f32,
    pub screen_brightness: f32,
    pub subtitles: bool,
    pub mini_map: bool,
    pub hud: bool,
    pub auto_save: bool,
    pub rumble: bool,
    pub units: i32,
    pub silent_sign_in: bool,
    pub volume_mode: i32,
    pub camera_angle: i32,
    pub invert_y: bool,
    pub invert_x: bool,
    pub auto_sign_in: bool,
    pub grind_transparency: bool,
}
impl Default for ProfileSettings {
    /// `tSettings::Default`825644d8 for an English, NTSC-region profile:
    /// subtitles only default on for Italian/Spanish/Japanese/Dutch.
    fn default() -> Self {
        Self {
            sfx_volume: 1.0,
            dialogue_volume: 1.0,
            music_volume: 1.0,
            screen_brightness: 0.0,
            subtitles: false,
            mini_map: true,
            hud: true,
            auto_save: true,
            // PC port: a new save's Vibration starts off (the original's default is on).
            rumble: false,
            units: 1,
            silent_sign_in: true,
            volume_mode: 0,
            camera_angle: 0,
            invert_y: false,
            invert_x: false,
            auto_sign_in: true,
            grind_transparency: false,
        }
    }
}
impl ProfileSettings {
    /// `LoadSettings`8255b950: v3 only, each field read in `SaveSettings` order; a field the
    /// leaf lacks keeps its default.
    fn parse(leaf: &MemcardLeaf) -> Result<Option<Self>> {
        if leaf.version != 3 {
            return Ok(None);
        }
        let mut settings = Self::default();
        let mut tuples = leaf.tuples.iter();
        let mut f = |value: &mut f32| -> Result<()> {
            if let Some(t) = tuples.next() {
                *value = f32::from_bits(read32(t, 0)?);
            }
            Ok(())
        };
        f(&mut settings.sfx_volume)?;
        f(&mut settings.dialogue_volume)?;
        f(&mut settings.music_volume)?;
        f(&mut settings.screen_brightness)?;
        let b = |t: Option<&Vec<u8>>, value: &mut bool| {
            if let Some(&byte) = t.and_then(|t| t.first()) {
                *value = byte != 0;
            }
        };
        let i = |t: Option<&Vec<u8>>, value: &mut i32| -> Result<()> {
            if let Some(t) = t {
                *value = read32(t, 0)? as i32;
            }
            Ok(())
        };
        b(tuples.next(), &mut settings.subtitles);
        b(tuples.next(), &mut settings.mini_map);
        b(tuples.next(), &mut settings.hud);
        b(tuples.next(), &mut settings.auto_save);
        b(tuples.next(), &mut settings.rumble);
        i(tuples.next(), &mut settings.units)?;
        b(tuples.next(), &mut settings.silent_sign_in);
        i(tuples.next(), &mut settings.volume_mode)?;
        i(tuples.next(), &mut settings.camera_angle)?;
        b(tuples.next(), &mut settings.invert_y);
        b(tuples.next(), &mut settings.invert_x);
        b(tuples.next(), &mut settings.auto_sign_in);
        b(tuples.next(), &mut settings.grind_transparency);
        Ok(Some(settings))
    }
    fn tuples(&self) -> Vec<Vec<u8>> {
        let f = |v: f32| v.to_be_bytes().to_vec();
        let b = |v: bool| vec![u8::from(v)];
        let i = |v: i32| v.to_be_bytes().to_vec();
        vec![
            f(self.sfx_volume), f(self.dialogue_volume), f(self.music_volume), f(self.screen_brightness),
            b(self.subtitles), b(self.mini_map), b(self.hud), b(self.auto_save), b(self.rumble),
            i(self.units), b(self.silent_sign_in), i(self.volume_mode), i(self.camera_angle),
            b(self.invert_y), b(self.invert_x), b(self.auto_sign_in), b(self.grind_transparency),
        ]
    }
}

/// `ProfileBackEnd::tProfileData` as `SaveProfileData`8255be68 writes it (v3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileData {
    pub name: [u8; 32],
    pub nickname: [u8; 32],
    /// `mNewProfile`: set by `ResetProfile`8256d8f0, cleared when the new
    /// career's Create-A-Skater shuts down (`FrontEndState_CaC::Shutdown`).
    pub new_profile: bool,
    pub local_peer_id: u64,
}
impl ProfileData {
    /// `ResetProfile`: the localized default names, `mNewProfile = 1` and a
    /// `-1` peer id.
    #[must_use]
    pub fn reset(name: &str, nickname: &str) -> Self {
        let mut data = Self { name: [0; 32], nickname: [0; 32], new_profile: true, local_peer_id: u64::MAX };
        // StringnCopy with 0x1F (name) and 0x20 (nickname) characters.
        for (dst, src, max) in [(&mut data.name, name, 31), (&mut data.nickname, nickname, 32)] {
            let bytes = src.as_bytes();
            let n = bytes.len().min(max);
            dst[..n].copy_from_slice(&bytes[..n]);
        }
        data
    }
    fn tuples(&self) -> Vec<Vec<u8>> {
        vec![
            self.name.to_vec(),
            self.nickname.to_vec(),
            i32::from(self.new_profile).to_be_bytes().to_vec(),
            self.local_peer_id.to_be_bytes().to_vec(),
        ]
    }
    /// `LoadProfileData`8256c1a0: v3 only; `ReadString` copies the tuple.
    fn parse(leaf: &MemcardLeaf) -> Result<Option<Self>> {
        if leaf.version != 3 {
            return Ok(None);
        }
        let [name, nickname, new_profile, peer, ..] = leaf.tuples.as_slice() else {
            return Err(ProfileError("truncated ProfileBackEnd::ProfileData"));
        };
        let mut data = Self { name: [0; 32], nickname: [0; 32], new_profile: false, local_peer_id: u64::MAX };
        data.name[..name.len().min(31)].copy_from_slice(&name[..name.len().min(31)]);
        data.nickname[..nickname.len().min(32)].copy_from_slice(&nickname[..nickname.len().min(32)]);
        if new_profile.len() != 4 {
            return Err(ProfileError("invalid mNewProfile tuple"));
        }
        data.new_profile = read32(new_profile, 0)? != 0;
        if peer.len() == 8 {
            data.local_peer_id = u64::from_be_bytes(peer[..8].try_into().expect("length checked"));
        }
        Ok(Some(data))
    }
}

/// `ProfileBackEnd::tMusicPlayer`'s playlists: the song ids and how many are in use.
pub const MUSIC_PLAYER_PLAYLISTS: usize = 5;
pub const MUSIC_PLAYER_SONGS: usize = 100;

/// One `tMusicPlayer` playlist (436 bytes in the backend): its name, songs and count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicPlayerPlaylist {
    pub name: [u8; 32],
    pub songs: [u32; MUSIC_PLAYER_SONGS],
    pub num_songs: i32,
}

/// `ProfileBackEnd::tMusicPlayer` as `SaveMusicPlayer`825646d8 writes it (v2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileMusicPlayer {
    pub num_playlists: i32,
    pub current_playlist: i32,
    pub current_song_index: i32,
    /// `eMusicMode`: 0 off, 1 the world's music, 2 the playlists.
    pub music_mode: i32,
    pub shuffle: bool,
    pub playlists: [MusicPlayerPlaylist; MUSIC_PLAYER_PLAYLISTS],
}
impl ProfileMusicPlayer {
    /// `ResetProfile`8256d8f0: five playlists, each the whole music player catalogue in order,
    /// the world's music, unshuffled, at the first playlist's first song.
    #[must_use]
    pub fn reset(catalogue: &[u32]) -> Self {
        let mut songs = [0; MUSIC_PLAYER_SONGS];
        for (slot, &song) in songs.iter_mut().zip(catalogue) {
            if song != 0 {
                *slot = song;
            }
        }
        let playlist = MusicPlayerPlaylist { name: [0; 32], songs, num_songs: catalogue.len() as i32 };
        Self {
            num_playlists: MUSIC_PLAYER_PLAYLISTS as i32,
            current_playlist: 0,
            current_song_index: 0,
            music_mode: 1,
            shuffle: false,
            playlists: std::array::from_fn(|_| playlist.clone()),
        }
    }
    fn tuples(&self) -> Vec<Vec<u8>> {
        let i = |v: i32| v.to_be_bytes().to_vec();
        let mut tuples = vec![
            i(self.num_playlists),
            i(self.current_playlist),
            i(self.current_song_index),
            i(self.music_mode),
            vec![u8::from(self.shuffle)],
        ];
        // SavePlaylist8255bf40: WriteString(name, 32), the 100 songs, then the count.
        for playlist in &self.playlists {
            tuples.push(playlist.name.to_vec());
            tuples.extend(playlist.songs.iter().map(|&song| song.to_be_bytes().to_vec()));
            tuples.push(i(playlist.num_songs));
        }
        tuples
    }
    /// `LoadMusicPlayer`825645a8: v2 only, over the `tMusicPlayer` constructor's values.
    fn parse(leaf: &MemcardLeaf, catalogue: &[u32]) -> Result<Option<Self>> {
        if leaf.version != 2 {
            return Ok(None);
        }
        let mut player = Self::reset(catalogue);
        let mut tuples = leaf.tuples.iter();
        let mut int = |value: &mut i32| -> Result<()> {
            if let Some(t) = tuples.next() {
                *value = read32(t, 0)? as i32;
            }
            Ok(())
        };
        int(&mut player.num_playlists)?;
        int(&mut player.current_playlist)?;
        int(&mut player.current_song_index)?;
        int(&mut player.music_mode)?;
        if let Some(&byte) = tuples.next().and_then(|t| t.first()) {
            player.shuffle = byte != 0;
        }
        // LoadPlaylist8255bbd0: ReadString copies the name tuple.
        for playlist in &mut player.playlists {
            if let Some(name) = tuples.next() {
                playlist.name = [0; 32];
                let n = name.len().min(32);
                playlist.name[..n].copy_from_slice(&name[..n]);
            }
            for song in &mut playlist.songs {
                if let Some(t) = tuples.next() {
                    *song = read32(t, 0)?;
                }
            }
            if let Some(t) = tuples.next() {
                playlist.num_songs = read32(t, 0)? as i32;
            }
        }
        Ok(Some(player))
    }
}

pub const SKATER_PROFILE_SAVABLE: &str = "SKATER_PROFILE";
/// `ProfileBackEnd::SaveData`8256c2c0's leaves, in its order.
const PROFILE_LEAVES: [&str; 3] = ["ProfileBackEnd::Settings", "ProfileBackEnd::ProfileData", "ProfileBackEnd::MusicPlayer"];

#[derive(Clone, Debug)]
pub struct SkaterProfile {
    prefix: [u8; 20],
    chunks: Vec<Vec<u8>>,
}
impl SkaterProfile {
    /// MemcardProfile::GenerateSaveData82569f48 and LoadData8256ece8.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PROFILE_SIZE {
            return Err(ProfileError("SKATER.P must contain 0x400004 bytes"));
        }
        if read32(bytes, 0)? != native_crc(&bytes[4..]) {
            return Err(ProfileError("SKATER.P CRC mismatch"));
        }
        if read32(bytes, 20)? != name_hash("MEMCARD_ROOT") || bytes[32] != 1 {
            return Err(ProfileError("SKATER.P root chunk missing"));
        }
        let end = 36usize
            .checked_add(read32(bytes, 28)? as usize)
            .ok_or(ProfileError("root size overflow"))?;
        if end > bytes.len() {
            return Err(ProfileError("SKATER.P root exceeds buffer"));
        }
        let chunks = split_chunks(&bytes[36..end])?;
        let version = chunks.first().ok_or(ProfileError("save version absent"))?;
        let children = split_chunks(&version[16..])?;
        let leaf = children
            .first()
            .ok_or(ProfileError("save version tuple absent"))?;
        if read32(leaf, 0)? != name_hash("IMemcardSavable::SaveMemcardSaveVersion")
            || read32(leaf, 16)? != 4
            || read32(leaf, 20)? != 1025
        {
            return Err(ProfileError("unsupported memcard save version"));
        }
        Ok(Self {
            prefix: bytes[..20].try_into().expect("length checked"),
            chunks,
        })
    }
    /// Host profile containing only CAC. This does not synthesize career data.
    pub fn new_cac_only(record: &CacRecord) -> Result<Self> {
        let mut p = Self::new_empty();
        p.set_cac_record(record)?;
        Ok(p)
    }
    /// Native version envelope; no fabricated career or CAC record.
    pub fn new_empty() -> Self {
        let version = leaf_chunk(
            name_hash("IMemcardSavable::SaveMemcardSaveVersion"),
            1,
            &[1025u32.to_be_bytes().to_vec()],
        );
        Self {
            prefix: [0; 20],
            chunks: vec![nested_chunk(name_hash("CAC"), &version)],
        }
    }
    pub fn display_settings(&self) -> Result<PcDisplaySettings> {
        for chunk in self.chunks.iter().skip(1) {
            if read32(chunk, 0)? != name_hash("SK82_PC") { continue; }
            for leaf in split_chunks(&chunk[16..])? {
                if read32(&leaf, 0)? != name_hash("SK82_PC::DisplaySettings") { continue; }
                // Version 1 predates VSync and reads as Refresh Rate. Version 2's VSync Off,
                // Half and On read as Unlimited, 60 and Refresh Rate. Versions before 4 have no
                // frame rate counter, which reads as off, before 5 no fullscreen: windowed, before
                // 6 no resolution scale: 100%, and before 7 no VSync, which the Refresh Rate cap
                // alone meant.
                let version = read32(&leaf, 4)?;
                if !matches!(version, 1..=7) { return Err(ProfileError("unsupported PC display settings version")); }
                let mut r = Reader { bytes: &leaf[16..], pos: 0 };
                let mut boolean = || match r.tuple()? {
                    [0] => Ok(false), [1] => Ok(true),
                    _ => Err(ProfileError("invalid PC display setting boolean")),
                };
                let (postfx, fisheye) = (boolean()?, boolean()?);
                let frame_rate_cap = if version == 1 { FrameRateCap::RefreshRate } else {
                    match (version, r.tuple()?) {
                        (_, [0]) => FrameRateCap::Unlimited,
                        (_, [1]) => FrameRateCap::Fps60,
                        (3..=7, [2]) => FrameRateCap::Fps120,
                        (2, [2]) | (3..=7, [3]) => FrameRateCap::RefreshRate,
                        _ => return Err(ProfileError("invalid PC frame rate cap")),
                    }
                };
                let fps_counter = version >= 4 && match r.tuple()? {
                    [0] => false, [1] => true,
                    _ => return Err(ProfileError("invalid PC display setting boolean")),
                };
                let fullscreen = version >= 5 && match r.tuple()? {
                    [0] => false, [1] => true,
                    _ => return Err(ProfileError("invalid PC display setting boolean")),
                };
                let resolution_scale = if version >= 6 {
                    match r.tuple()? {
                        [scale] if RESOLUTION_SCALES.contains(scale) && scale % RESOLUTION_SCALE_STEP == 0 => *scale,
                        _ => return Err(ProfileError("invalid PC resolution scale")),
                    }
                } else { 100 };
                let vsync = if version >= 7 {
                    match r.tuple()? {
                        [0] => false, [1] => true,
                        _ => return Err(ProfileError("invalid PC display setting boolean")),
                    }
                } else { frame_rate_cap == FrameRateCap::RefreshRate };
                return Ok(PcDisplaySettings { postfx, fisheye, frame_rate_cap, fps_counter, fullscreen, resolution_scale, vsync });
            }
        }
        Ok(PcDisplaySettings::default())
    }
    pub fn set_display_settings(&mut self, settings: PcDisplaySettings) -> Result<()> {
        let id = name_hash("SK82_PC::DisplaySettings");
        let leaf = leaf_chunk(id, 7, &[vec![u8::from(settings.postfx)], vec![u8::from(settings.fisheye)],
            vec![settings.frame_rate_cap as u8], vec![u8::from(settings.fps_counter)], vec![u8::from(settings.fullscreen)],
            vec![settings.resolution_scale], vec![u8::from(settings.vsync)]]);
        for chunk in self.chunks.iter_mut().skip(1) {
            if read32(chunk, 0)? != name_hash("SK82_PC") { continue; }
            let mut children = split_chunks(&chunk[16..])?;
            if let Some(old) = children.iter_mut().find(|c| read32(c, 0).ok() == Some(id)) {
                // Validate the version before modifying an existing extension.
                if !matches!(read32(old, 4)?, 1..=7) { return Err(ProfileError("unsupported PC display settings version")); }
                *old = leaf;
            } else { children.push(leaf); }
            *chunk = nested_chunk(name_hash("SK82_PC"), &children.concat());
            return Ok(());
        }
        self.chunks.push(nested_chunk(name_hash("SK82_PC"), &leaf));
        Ok(())
    }
    pub fn cac_record(&self) -> Result<Option<CacRecord>> {
        for chunk in self.chunks.iter().skip(1) {
            if read32(chunk, 0)? != name_hash("CAC") {
                continue;
            }
            for leaf in split_chunks(&chunk[16..])? {
                if read32(&leaf, 0)? != name_hash("CACBackEnd::Data") {
                    continue;
                }
                if read32(&leaf, 4)? != 7 {
                    return Err(ProfileError("unsupported CAC backend version"));
                }
                let mut r = Reader {
                    bytes: &leaf[16..],
                    pos: 0,
                };
                let valid = r.tuple()?;
                if valid.len() != 1 {
                    return Err(ProfileError("invalid CAC valid tuple"));
                }
                if valid[0] == 0 {
                    return Ok(None);
                }
                return CacRecord::parse(r.tuple()?).map(Some);
            }
        }
        Ok(None)
    }
    pub fn set_cac_record(&mut self, record: &CacRecord) -> Result<()> {
        let encoded = record.encode()?;
        let mut found = false;
        for chunk in self.chunks.iter_mut().skip(1) {
            if read32(chunk, 0)? != name_hash("CAC") {
                continue;
            }
            let mut children = split_chunks(&chunk[16..])?;
            for leaf in &mut children {
                if read32(leaf, 0)? != name_hash("CACBackEnd::Data") {
                    continue;
                }
                if read32(leaf, 4)? != 7 {
                    return Err(ProfileError("unsupported CAC backend version"));
                }
                let mut r = Reader {
                    bytes: &leaf[16..],
                    pos: 0,
                };
                let valid = r.tuple()?;
                if valid.len() != 1 {
                    return Err(ProfileError("invalid CAC valid tuple"));
                }
                if valid[0] != 0 {
                    r.tuple()?;
                }
                let tail_start = r.pos;
                // Keep user-generated textures and all trailing tuple bytes.
                // Existing tail includes the extra native 16-byte gap. Size
                // changes require rebuilding the leaf terminator/alignment.
                let mut tail = Reader {
                    bytes: &leaf[16 + tail_start..],
                    pos: 0,
                };
                let mut tuples = vec![vec![1], encoded.clone()];
                while tail.pos + 4 <= tail.bytes.len() {
                    let value = tail.tuple()?;
                    if value.is_empty() {
                        break;
                    }
                    tuples.push(value.to_vec());
                }
                *leaf = leaf_chunk(name_hash("CACBackEnd::Data"), 7, &tuples);
                found = true;
            }
            *chunk = nested_chunk(name_hash("CAC"), &children.concat());
        }
        if !found {
            let mut tuples = vec![vec![1], encoded, 2u32.to_be_bytes().to_vec()];
            // CACBackEnd constructor82580b60 initializes all four UGC slots.
            for _ in 0..4 {
                tuples.push(vec![0; 86016]);
                tuples.push(vec![0; 4]);
                tuples.push(vec![0; 256]);
            }
            self.chunks.push(nested_chunk(
                name_hash("CAC"),
                &leaf_chunk(name_hash("CACBackEnd::Data"), 7, &tuples),
            ));
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let root = nested_chunk(name_hash("MEMCARD_ROOT"), &self.chunks.concat());
        if 20 + root.len() > PROFILE_SIZE {
            return Err(ProfileError("SKATER.P exceeds fixed native buffer"));
        }
        let mut bytes = vec![0; PROFILE_SIZE];
        bytes[..20].copy_from_slice(&self.prefix);
        bytes[20..20 + root.len()].copy_from_slice(&root);
        let crc = native_crc(&bytes[4..]);
        bytes[..4].copy_from_slice(&crc.to_be_bytes());
        Ok(bytes)
    }
}

impl SkaterProfile {
    fn savable_index(&self, savable: &str) -> Result<Option<usize>> {
        let id = name_hash(savable);
        for (index, chunk) in self.chunks.iter().enumerate().skip(1) {
            if read32(chunk, 0)? == id {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }

    /// `ReadHashSkipUnknownHashValues`82569e90 inside one savable's chunk.
    pub fn leaf(&self, savable: &str, leaf: &str) -> Result<Option<MemcardLeaf>> {
        let Some(index) = self.savable_index(savable)? else { return Ok(None) };
        let id = name_hash(leaf);
        for chunk in split_chunks(&self.chunks[index][16..])? {
            if read32(&chunk, 0)? != id {
                continue;
            }
            let size = read32(&chunk, 8)? as usize;
            let end = size.min(chunk.len());
            let mut tuples = Vec::new();
            let mut pos = 16;
            while pos + 4 <= end {
                let n = read32(&chunk, pos)? as usize;
                if n == 0 {
                    break;
                }
                let data = chunk.get(pos + 4..pos + 4 + n).ok_or(ProfileError("truncated memcard tuple"))?;
                tuples.push(data.to_vec());
                pos += 4 + n;
            }
            return Ok(Some(MemcardLeaf { version: read32(&chunk, 4)?, tuples }));
        }
        Ok(None)
    }

    /// Replace a leaf in place, or append it to the savable's chunk. A new
    /// savable is inserted after `after` (native registration order) or last;
    /// with `after` empty it is the first registered savable, which follows
    /// the version chunk (`MemcardProfile::GenerateSaveData` `0x82569f48`).
    pub fn set_leaf(&mut self, savable: &str, after: &[&str], leaf: &str, version: u32, tuples: &[Vec<u8>]) -> Result<()> {
        let id = name_hash(leaf);
        let encoded = leaf_chunk(id, version, tuples);
        let index = match self.savable_index(savable)? {
            Some(index) => index,
            None => {
                let mut at = if after.is_empty() { self.chunks.len().min(1) } else { self.chunks.len() };
                for name in after.iter().rev() {
                    if let Some(found) = self.savable_index(name)? {
                        at = found + 1;
                        break;
                    }
                }
                self.chunks.insert(at, nested_chunk(name_hash(savable), &[]));
                at
            }
        };
        let mut children = split_chunks(&self.chunks[index][16..])?;
        if let Some(existing) = children.iter_mut().find(|c| read32(c, 0).ok() == Some(id)) {
            *existing = encoded;
        } else {
            children.push(encoded);
        }
        self.chunks[index] = nested_chunk(name_hash(savable), &children.concat());
        Ok(())
    }

    /// `SKATER_PROFILE` / `ProfileBackEnd::ProfileData`. `None` when the
    /// profile has never been written, which `ResetProfile` makes new.
    pub fn profile_data(&self) -> Result<Option<ProfileData>> {
        match self.leaf(SKATER_PROFILE_SAVABLE, "ProfileBackEnd::ProfileData")? {
            Some(leaf) => ProfileData::parse(&leaf),
            None => Ok(None),
        }
    }

    /// Writes `ProfileBackEnd::ProfileData`. A profile without the savable
    /// also gets `ProfileBackEnd::Settings` first, with `tSettings::Default`,
    /// as `ProfileBackEnd::SaveData`8256c2c0 always writes both.
    pub fn set_profile_data(&mut self, data: &ProfileData) -> Result<()> {
        if self.leaf(SKATER_PROFILE_SAVABLE, PROFILE_LEAVES[0])?.is_none() {
            self.set_profile_leaf(0, 3, &ProfileSettings::default().tuples())?;
        }
        self.set_profile_leaf(1, 3, &data.tuples())
    }

    /// `SKATER_PROFILE` / `ProfileBackEnd::Settings`.
    pub fn settings(&self) -> Result<Option<ProfileSettings>> {
        match self.leaf(SKATER_PROFILE_SAVABLE, PROFILE_LEAVES[0])? {
            Some(leaf) => ProfileSettings::parse(&leaf),
            None => Ok(None),
        }
    }

    pub fn set_settings(&mut self, settings: &ProfileSettings) -> Result<()> {
        self.set_profile_leaf(0, 3, &settings.tuples())
    }

    /// `SKATER_PROFILE` / `ProfileBackEnd::MusicPlayer` over `ResetProfile`'s playlists of
    /// `catalogue`.
    pub fn music_player(&self, catalogue: &[u32]) -> Result<Option<ProfileMusicPlayer>> {
        match self.leaf(SKATER_PROFILE_SAVABLE, PROFILE_LEAVES[2])? {
            Some(leaf) => ProfileMusicPlayer::parse(&leaf, catalogue),
            None => Ok(None),
        }
    }

    pub fn set_music_player(&mut self, player: &ProfileMusicPlayer) -> Result<()> {
        self.set_profile_leaf(2, 2, &player.tuples())
    }

    /// One of `SaveData`'s leaves, which stay in its order: `LoadData` reads them in turn.
    fn set_profile_leaf(&mut self, leaf: usize, version: u32, tuples: &[Vec<u8>]) -> Result<()> {
        const AFTER: [&str; 2] = ["UNLOCKABLES", "CAC"];
        self.set_leaf(SKATER_PROFILE_SAVABLE, &AFTER, PROFILE_LEAVES[leaf], version, tuples)?;
        let index = self.savable_index(SKATER_PROFILE_SAVABLE)?.ok_or(ProfileError("profile savable absent"))?;
        let mut children = split_chunks(&self.chunks[index][16..])?;
        let order = |chunk: &Vec<u8>| {
            let id = read32(chunk, 0).unwrap_or(0);
            PROFILE_LEAVES.iter().position(|name| name_hash(name) == id).unwrap_or(PROFILE_LEAVES.len())
        };
        children.sort_by_key(order);
        self.chunks[index] = nested_chunk(name_hash(SKATER_PROFILE_SAVABLE), &children.concat());
        Ok(())
    }
}

fn split_chunks(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        let size = read32(bytes, pos + 8)? as usize;
        let len = 16usize
            .checked_add((size + 3) & !3)
            .ok_or(ProfileError("chunk overflow"))?;
        let end = pos.checked_add(len).ok_or(ProfileError("chunk overflow"))?;
        result.push(
            bytes
                .get(pos..end)
                .ok_or(ProfileError("truncated memcard chunk"))?
                .to_vec(),
        );
        pos = end;
    }
    Ok(result)
}
fn nested_chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    put32(&mut b, id);
    put32(&mut b, 0);
    put32(&mut b, payload.len() as u32);
    b.extend_from_slice(&[1, 0, 0, 0]);
    b.extend_from_slice(payload);
    b.resize((b.len() + 3) & !3, 0);
    b
}
fn leaf_chunk(id: u32, version: u32, tuples: &[Vec<u8>]) -> Vec<u8> {
    let mut b = Vec::new();
    put32(&mut b, id);
    put32(&mut b, version);
    put32(&mut b, 0);
    put32(&mut b, 0);
    for tuple in tuples {
        tuple_write(&mut b, tuple);
    }
    b.extend_from_slice(&[0; 5]);
    // EndWriteChunk8255f9e0 counts the header in Size, then advances Size+16.
    let size = b.len();
    b[8..12].copy_from_slice(&(size as u32).to_be_bytes());
    b.resize(16 + ((size + 3) & !3), 0);
    b
}
fn tuple_write(b: &mut Vec<u8>, value: &[u8]) {
    put32(b, value.len() as u32);
    b.extend_from_slice(value);
}
fn part_write(b: &mut Vec<u8>, part: CacPartId) {
    b.extend_from_slice(&part.model.to_be_bytes());
    b.extend_from_slice(&part.material.to_be_bytes());
}
fn put32(b: &mut Vec<u8>, value: u32) {
    b.extend_from_slice(&value.to_be_bytes());
}
fn read32(b: &[u8], pos: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        b.get(pos..pos + 4)
            .ok_or(ProfileError("truncated u32"))?
            .try_into()
            .expect("length checked"),
    ))
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(ProfileError("size overflow"))?;
        let out = self
            .bytes
            .get(self.pos..end)
            .ok_or(ProfileError("truncated CAC data"))?;
        self.pos = end;
        Ok(out)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("length checked"),
        ))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().expect("length checked"),
        ))
    }
    fn part(&mut self) -> Result<CacPartId> {
        Ok(CacPartId {
            model: self.u64()?,
            material: self.u64()?,
        })
    }
    fn tuple(&mut self) -> Result<&'a [u8]> {
        let n = self.u32()? as usize;
        self.take(n)
    }
}
fn name_hash(name: &str) -> u32 {
    name.bytes().fold(u32::MAX, |h, b| {
        h.wrapping_mul(33).wrapping_add(u32::from(b))
    })
}
/// GetCRC829b30e8; kCrcTab821f16c0 uses polynomial04c11db7.
pub fn native_crc(bytes: &[u8]) -> u32 {
    if bytes.len() < 4 {
        return 0;
    }
    let mut crc = !u32::from_be_bytes(bytes[..4].try_into().expect("length checked"));
    for &byte in &bytes[4..] {
        let mut table = crc & 0xff000000;
        for _ in 0..8 {
            table = if table & 0x80000000 != 0 {
                (table << 1) ^ 0x04c11db7
            } else {
                table << 1
            };
        }
        crc = (crc << 8 | u32::from(byte)) ^ table;
    }
    let result = !crc;
    if result == 0 { u32::MAX } else { result }
}

#[cfg(test)]
mod display_tests {
    use super::*;
    #[test]
    fn pc_settings_preserve_cac_and_unknown_chunks_in_both_save_orders() {
        let record = CacRecord::new(RecipeData {
            recipe: vec![1,2,3], tuning_bits: [0.5f32.to_bits(); 2], hat_type: 0,
            stance: 1, style: 0, gestures: [0;4], is_male: true,
        }, CacPartId { model: 17, material: 31 }, vec![0;16]);
        let mut profile = SkaterProfile::new_empty();
        assert_eq!(profile.display_settings().unwrap(), PcDisplaySettings::default());
        let unknown = nested_chunk(name_hash("UnimplementedCareer"), &leaf_chunk(97, 17, &[vec![9,8,7,6]]));
        profile.chunks.push(unknown.clone());
        for (postfx, frame_rate_cap) in [(false, FrameRateCap::Unlimited), (true, FrameRateCap::Fps60), (false, FrameRateCap::Fps120)] { for fisheye in [true, false] {
            let expected = PcDisplaySettings { postfx, fisheye, frame_rate_cap, fps_counter: fisheye, fullscreen: postfx,
                resolution_scale: if fisheye { 200 } else { 50 }, vsync: !postfx };
            profile.set_display_settings(expected).unwrap();
            profile.set_cac_record(&record).unwrap();
            profile = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
            assert_eq!(profile.display_settings().unwrap(), expected);
            assert_eq!(profile.cac_record().unwrap(), Some(record.clone()));
            assert!(profile.chunks.contains(&unknown));
            profile.set_display_settings(expected).unwrap();
            assert_eq!(profile.cac_record().unwrap(), Some(record.clone()));
        }}
    }
    #[test]
    fn earlier_display_settings_read_as_frame_rate_caps() {
        let saved = |version, tuples: &[Vec<u8>]| {
            let mut profile = SkaterProfile::new_empty();
            profile.chunks.push(nested_chunk(name_hash("SK82_PC"),
                &leaf_chunk(name_hash("SK82_PC::DisplaySettings"), version, tuples)));
            profile
        };
        assert_eq!(saved(1, &[vec![0], vec![1]]).display_settings().unwrap(),
            PcDisplaySettings { postfx: false, fisheye: true, frame_rate_cap: FrameRateCap::RefreshRate, fps_counter: false, fullscreen: false,
                resolution_scale: 100, vsync: true });
        for (vsync, frame_rate_cap) in [(0, FrameRateCap::Unlimited), (1, FrameRateCap::Fps60), (2, FrameRateCap::RefreshRate)] {
            assert_eq!(saved(2, &[vec![1], vec![1], vec![vsync]]).display_settings().unwrap().frame_rate_cap, frame_rate_cap);
        }
        assert!(saved(2, &[vec![1], vec![1], vec![3]]).display_settings().is_err());
        assert_eq!(saved(3, &[vec![1], vec![0], vec![2]]).display_settings().unwrap(),
            PcDisplaySettings { postfx: true, fisheye: false, frame_rate_cap: FrameRateCap::Fps120, fps_counter: false, fullscreen: false,
                resolution_scale: 100, vsync: false });
        assert_eq!(saved(4, &[vec![1], vec![0], vec![2], vec![1]]).display_settings().unwrap(),
            PcDisplaySettings { postfx: true, fisheye: false, frame_rate_cap: FrameRateCap::Fps120, fps_counter: true, fullscreen: false,
                resolution_scale: 100, vsync: false });
        assert_eq!(saved(5, &[vec![1], vec![1], vec![3], vec![0], vec![1]]).display_settings().unwrap(),
            PcDisplaySettings { fullscreen: true, ..PcDisplaySettings::default() });
        assert_eq!(saved(6, &[vec![1], vec![1], vec![3], vec![0], vec![1], vec![140]]).display_settings().unwrap(),
            PcDisplaySettings { fullscreen: true, resolution_scale: 140, ..PcDisplaySettings::default() });
        for scale in [0, 40, 145, 210] {
            assert!(saved(6, &[vec![1], vec![1], vec![3], vec![0], vec![0], vec![scale]]).display_settings().is_err());
        }
        // Before version 7 only the Refresh Rate cap waited for the refresh; from 7 VSync is its own.
        assert!(!saved(6, &[vec![1], vec![1], vec![1], vec![0], vec![0], vec![100]]).display_settings().unwrap().vsync);
        assert_eq!(saved(7, &[vec![1], vec![1], vec![3], vec![0], vec![0], vec![100], vec![0]]).display_settings().unwrap(),
            PcDisplaySettings { vsync: false, ..PcDisplaySettings::default() });
        assert_eq!(saved(7, &[vec![1], vec![1], vec![2], vec![0], vec![0], vec![100], vec![1]]).display_settings().unwrap(),
            PcDisplaySettings { frame_rate_cap: FrameRateCap::Fps120, ..PcDisplaySettings::default() });
        assert!(saved(7, &[vec![1], vec![1], vec![3], vec![0], vec![0], vec![100], vec![2]]).display_settings().is_err());
        let mut profile = saved(2, &[vec![1], vec![1], vec![1]]);
        let capped = PcDisplaySettings { frame_rate_cap: FrameRateCap::Fps120, ..PcDisplaySettings::default() };
        profile.set_display_settings(capped).unwrap();
        let profile = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
        assert_eq!(profile.display_settings().unwrap(), capped);
    }
    #[test]
    fn unsupported_extension_version_is_not_silently_replaced() {
        let mut profile = SkaterProfile::new_empty();
        profile.chunks.push(nested_chunk(name_hash("SK82_PC"),
            &leaf_chunk(name_hash("SK82_PC::DisplaySettings"), 99, &[vec![1], vec![1]])));
        assert!(profile.display_settings().is_err());
        assert!(profile.set_display_settings(PcDisplaySettings::default()).is_err());
    }
    #[test]
    fn settings_and_music_player_round_trip_in_save_data_order() {
        let catalogue: Vec<u32> = (1..=52).map(|i| 0x1000 + i).collect();
        let mut profile = SkaterProfile::new_empty();
        assert_eq!(profile.settings().unwrap(), None);
        assert_eq!(profile.music_player(&catalogue).unwrap(), None);
        let mut player = ProfileMusicPlayer::reset(&catalogue);
        player.music_mode = 2;
        player.shuffle = true;
        player.current_playlist = 3;
        player.playlists[3].num_songs = 2;
        player.playlists[3].name[..4].copy_from_slice(b"mine");
        let settings = ProfileSettings { sfx_volume: 0.3, music_volume: 0.7, ..ProfileSettings::default() };
        profile.set_music_player(&player).unwrap();
        profile.set_profile_data(&ProfileData::reset("PLAYER 1", "P1")).unwrap();
        profile.set_settings(&settings).unwrap();
        let profile = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
        assert_eq!(profile.settings().unwrap(), Some(settings));
        assert_eq!(profile.music_player(&catalogue).unwrap(), Some(player));
        // SaveData writes Settings, ProfileData, then MusicPlayer, whatever order they were set in.
        let index = profile.savable_index(SKATER_PROFILE_SAVABLE).unwrap().unwrap();
        let ids: Vec<u32> = split_chunks(&profile.chunks[index][16..]).unwrap().iter().map(|c| read32(c, 0).unwrap()).collect();
        assert_eq!(ids, PROFILE_LEAVES.map(name_hash));
    }
}
