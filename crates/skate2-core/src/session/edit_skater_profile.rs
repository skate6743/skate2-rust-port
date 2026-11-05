//! Host filesystem boundary for the native SKATER.P codec.
//! The caller selects the save location; original asset directories are never
//! selected implicitly. Existing unknown career chunks survive edits.
use skate2_formats::skater_profile::{
    CacRecord, MemcardLeaf, PcDisplaySettings, PcGameplayModes, ProfileData, ProfileError, ProfileMusicPlayer, ProfileSettings, SkaterProfile,
};
use std::{
    cell::Cell,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Condvar, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// SKATER.P is read and replaced one access at a time. The count is the saves handed to
/// another thread and not yet written (the career's autosave): every access made after a
/// hand-over waits for that save, so it reads it and is not overwritten by it.
static FILE: Mutex<usize> = Mutex::new(0);
static WRITTEN: Condvar = Condvar::new();

thread_local! {
    /// This thread writes a handed-over save and holds the file.
    static WRITING: Cell<bool> = const { Cell::new(false) };
}

/// The file, once the handed-over saves are written.
fn file() -> MutexGuard<'static, usize> {
    let mut handed = FILE.lock().unwrap_or_else(PoisonError::into_inner);
    while *handed > 0 {
        handed = WRITTEN.wait(handed).unwrap_or_else(PoisonError::into_inner);
    }
    handed
}

/// The file for one access; a thread writing a handed-over save already holds it.
fn access() -> Option<MutexGuard<'static, usize>> {
    (!WRITING.with(Cell::get)).then(file)
}

/// A save made on one thread and written on another: from the hand-over, every other
/// access waits until it is written.
pub struct HandedSave(());

impl HandedSave {
    #[must_use]
    pub fn hand_over() -> Self {
        *FILE.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        Self(())
    }

    /// Writes the save: `save` gets the store as the file is now, and the file to itself.
    ///
    /// # Errors
    /// The file exists but cannot be read.
    pub fn write<T>(
        self,
        path: impl Into<PathBuf>,
        save: impl FnOnce(&mut EditSkaterProfileStore) -> T,
    ) -> Result<T, EditSkaterProfileError> {
        let mut handed = FILE.lock().unwrap_or_else(PoisonError::into_inner);
        WRITING.with(|writing| writing.set(true));
        let result = EditSkaterProfileStore::read(path.into()).map(|mut store| save(&mut store));
        WRITING.with(|writing| writing.set(false));
        *handed -= 1;
        WRITTEN.notify_all();
        drop(handed);
        std::mem::forget(self);
        result
    }
}

impl Drop for HandedSave {
    /// A save given up unwritten no longer holds the other accesses back.
    fn drop(&mut self) {
        *FILE.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        WRITTEN.notify_all();
    }
}

#[derive(Debug)]
pub enum EditSkaterProfileError {
    Io(io::Error),
    Format(skate2_formats::skater_profile::ProfileError),
}
impl std::fmt::Display for EditSkaterProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "SKATER.P storage: {e}"),
            Self::Format(e) => write!(f, "SKATER.P format: {e}"),
        }
    }
}
impl std::error::Error for EditSkaterProfileError {}
impl From<io::Error> for EditSkaterProfileError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<skate2_formats::skater_profile::ProfileError> for EditSkaterProfileError {
    fn from(e: skate2_formats::skater_profile::ProfileError) -> Self {
        Self::Format(e)
    }
}

pub struct EditSkaterProfileStore {
    path: PathBuf,
    profile: Option<SkaterProfile>,
}
/// `ProfileBackEnd::mSettings` and, once the music system named its catalogue, `mMusicPlayer`.
pub struct AudioProfile<'a> {
    pub settings: &'a ProfileSettings,
    pub music_player: Option<(&'a ProfileMusicPlayer, &'a [u32])>,
}
impl AudioProfile<'_> {
    fn apply(&self, profile: &mut SkaterProfile) -> Result<(), ProfileError> {
        profile.set_settings(self.settings)?;
        if let Some((player, _)) = self.music_player {
            profile.set_music_player(player)?;
        }
        Ok(())
    }
    fn validate(&self, profile: &SkaterProfile) -> Result<(), ProfileError> {
        if profile.settings()?.as_ref() != Some(self.settings) {
            return Err(ProfileError("settings read-back mismatch"));
        }
        if let Some((player, catalogue)) = self.music_player {
            if profile.music_player(catalogue)?.as_ref() != Some(player) {
                return Err(ProfileError("music player read-back mismatch"));
            }
        }
        Ok(())
    }
}
impl EditSkaterProfileStore {
    /// PC user-data location. Original mounted game assets are always read-only.
    pub fn default_path() -> Result<PathBuf, io::Error> {
        Ok(std::env::var_os("SKATE2_SAVE_ROOT").map(PathBuf::from)
            .unwrap_or(std::env::current_dir()?.join("saves")).join("SKATER.P"))
    }
    /// Missing files are a first-use state; malformed existing files are errors.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, EditSkaterProfileError> {
        let _file = access();
        Self::read(path.into())
    }
    fn read(path: PathBuf) -> Result<Self, EditSkaterProfileError> {
        let profile = match fs::read(&path) {
            Ok(bytes) => Some(SkaterProfile::parse(&bytes)?),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self { path, profile })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load_record(&self) -> Result<Option<CacRecord>, EditSkaterProfileError> {
        self.profile
            .as_ref()
            .map(SkaterProfile::cac_record)
            .transpose()
            .map(Option::flatten)
            .map_err(Into::into)
    }
    pub fn load_gesture_settings(&self) -> Result<skate2_formats::skater_profile::PcGestureSettings, EditSkaterProfileError> {
        self.profile.as_ref().map_or(Ok(Default::default()), SkaterProfile::gesture_settings).map_err(Into::into)
    }
    pub fn load_skate3_style(&self) -> Result<skate2_formats::skater_profile::PcSkate3Style, EditSkaterProfileError> {
        self.profile.as_ref().map_or(Ok(Default::default()), SkaterProfile::skate3_style).map_err(Into::into)
    }
    /// `ProfileBackEnd::mNewProfile`. A missing file or a save without a
    /// `SKATER_PROFILE` savable is a profile `ResetProfile` just created.
    pub fn is_new_profile(&self) -> Result<bool, EditSkaterProfileError> {
        Ok(self.profile.as_ref().map(SkaterProfile::profile_data).transpose()?.flatten().is_none_or(|p| p.new_profile))
    }
    pub fn load_profile_data(&self) -> Result<Option<ProfileData>, EditSkaterProfileError> {
        Ok(self.profile.as_ref().map(SkaterProfile::profile_data).transpose()?.flatten())
    }
    pub fn save_profile_data(&mut self, data: &ProfileData) -> Result<(), EditSkaterProfileError> {
        let _file = access();
        let mut next = Self::read(self.path.clone())?.profile.unwrap_or_else(SkaterProfile::new_empty);
        next.set_profile_data(data)?;
        let bytes = next.encode()?;
        let validated = SkaterProfile::parse(&bytes)?;
        if validated.profile_data()?.as_ref() != Some(data) {
            return Err(skate2_formats::skater_profile::ProfileError("profile data read-back mismatch").into());
        }
        self.commit(&bytes, validated)
    }
    pub fn load_display_settings(&self) -> Result<PcDisplaySettings, EditSkaterProfileError> {
        self.profile.as_ref().map_or(Ok(PcDisplaySettings::default()), |p| p.display_settings()).map_err(Into::into)
    }
    pub fn save_display_settings(&mut self, settings: PcDisplaySettings) -> Result<(), EditSkaterProfileError> {
        // Reopen before merging: an editor save may have replaced our snapshot.
        let _file = access();
        let mut next = Self::read(self.path.clone())?.profile.unwrap_or_else(SkaterProfile::new_empty);
        next.set_display_settings(settings)?;
        let bytes = next.encode()?;
        let validated = SkaterProfile::parse(&bytes)?;
        if validated.display_settings()? != settings {
            return Err(skate2_formats::skater_profile::ProfileError("display settings read-back mismatch").into());
        }
        self.commit(&bytes, validated)
    }
    /// PC port addition: Physics Settings and Weather Settings.
    pub fn load_gameplay_modes(&self) -> Result<PcGameplayModes, EditSkaterProfileError> {
        self.profile.as_ref().map_or(Ok(PcGameplayModes::default()), |p| p.gameplay_modes()).map_err(Into::into)
    }
    pub fn save_gameplay_modes(&mut self, modes: PcGameplayModes) -> Result<(), EditSkaterProfileError> {
        let _file = access();
        let mut next = Self::read(self.path.clone())?.profile.unwrap_or_else(SkaterProfile::new_empty);
        next.set_gameplay_modes(modes)?;
        let bytes = next.encode()?;
        let validated = SkaterProfile::parse(&bytes)?;
        if validated.gameplay_modes()? != modes {
            return Err(skate2_formats::skater_profile::ProfileError("gameplay modes read-back mismatch").into());
        }
        self.commit(&bytes, validated)
    }
    /// `ProfileBackEnd::tSettings`: the saved leaf, else `tSettings::Default`.
    pub fn load_settings(&self) -> Result<ProfileSettings, EditSkaterProfileError> {
        Ok(self.profile.as_ref().map(SkaterProfile::settings).transpose()?.flatten().unwrap_or_default())
    }
    /// `ProfileBackEnd::tMusicPlayer`: the saved leaf, else `ResetProfile`'s over `catalogue`.
    pub fn load_music_player(&self, catalogue: &[u32]) -> Result<ProfileMusicPlayer, EditSkaterProfileError> {
        Ok(self
            .profile
            .as_ref()
            .map(|p| p.music_player(catalogue))
            .transpose()?
            .flatten()
            .unwrap_or_else(|| ProfileMusicPlayer::reset(catalogue)))
    }
    /// `ProfileBackEnd::SaveData`'s Settings and MusicPlayer leaves, committed together.
    pub fn save_audio_profile(&mut self, audio: &AudioProfile<'_>) -> Result<(), EditSkaterProfileError> {
        let _file = access();
        let mut next = Self::read(self.path.clone())?.profile.unwrap_or_else(SkaterProfile::new_empty);
        audio.apply(&mut next)?;
        let bytes = next.encode()?;
        let validated = SkaterProfile::parse(&bytes)?;
        audio.validate(&validated)?;
        self.commit(&bytes, validated)
    }
    /// One savable's leaf (`ReadHashSkipUnknownHashValues`), when saved.
    pub fn load_leaf(&self, savable: &str, leaf: &str) -> Result<Option<MemcardLeaf>, EditSkaterProfileError> {
        Ok(self.profile.as_ref().map(|p| p.leaf(savable, leaf)).transpose()?.flatten())
    }
    /// A career memcard save: `SKATER_PROFILE`'s profile data and one
    /// savable's leaves (registered after `after`), committed together.
    pub fn save_career(
        &mut self,
        data: &ProfileData,
        savable: &str,
        after: &[&str],
        leaves: &[(&str, u32, Vec<Vec<u8>>)],
    ) -> Result<(), EditSkaterProfileError> {
        self.save_career_savables(data, &[(savable, after, leaves)])
    }

    /// A career memcard save of several savables, each (name, the savables
    /// registered before it, leaves), with `SKATER_PROFILE`'s profile data,
    /// committed together after read-back validation.
    pub fn save_career_savables(
        &mut self,
        data: &ProfileData,
        savables: &[(&str, &[&str], &[(&str, u32, Vec<Vec<u8>>)])],
    ) -> Result<(), EditSkaterProfileError> {
        self.save_memcard(data, None, savables)
    }
    /// `ProfileBackEnd::SaveData`: Settings, ProfileData and MusicPlayer, with the savables.
    pub fn save_memcard(
        &mut self,
        data: &ProfileData,
        audio: Option<&AudioProfile<'_>>,
        savables: &[(&str, &[&str], &[(&str, u32, Vec<Vec<u8>>)])],
    ) -> Result<(), EditSkaterProfileError> {
        let _file = access();
        let mut next = Self::read(self.path.clone())?.profile.unwrap_or_else(SkaterProfile::new_empty);
        next.set_profile_data(data)?;
        if let Some(audio) = audio {
            audio.apply(&mut next)?;
        }
        for (savable, after, leaves) in savables {
            for (leaf, version, tuples) in *leaves {
                next.set_leaf(savable, after, leaf, *version, tuples)?;
            }
        }
        let bytes = next.encode()?;
        let validated = SkaterProfile::parse(&bytes)?;
        if validated.profile_data()?.as_ref() != Some(data) {
            return Err(ProfileError("profile data read-back mismatch").into());
        }
        if let Some(audio) = audio {
            audio.validate(&validated)?;
        }
        for (savable, _, leaves) in savables {
            for (leaf, version, tuples) in *leaves {
                if validated.leaf(savable, leaf)?.is_none_or(|l| l.version != *version || l.tuples != *tuples) {
                    return Err(ProfileError("savable read-back mismatch").into());
                }
            }
        }
        self.commit(&bytes, validated)
    }
    /// Commit only after complete encoding and read-back validation. On Windows,
    /// replacement uses a same-directory backup because rename cannot replace
    /// an existing file. The previous save remains in that backup on success.
    pub fn save_record(&mut self, record: &CacRecord) -> Result<(), EditSkaterProfileError> {
        self.save_record_and_gestures(record, None)
    }
    /// Commit the CAC record and the optional PC gesture catalog in the same
    /// verified SKATER.P replacement. Other save writers retain this leaf.
    pub fn save_record_and_gestures(
        &mut self,
        record: &CacRecord,
        gestures: Option<skate2_formats::skater_profile::PcGestureSettings>,
    ) -> Result<(), EditSkaterProfileError> {
        self.save_record_and_mod_preferences(record, gestures, None)
    }
    /// Merge native CAC, gestures and animation choices in one verified save,
    /// reading the latest career/autosave before applying the edit.
    pub fn save_record_and_mod_preferences(
        &mut self,
        record: &CacRecord,
        gestures: Option<skate2_formats::skater_profile::PcGestureSettings>,
        style: Option<skate2_formats::skater_profile::PcSkate3Style>,
    ) -> Result<(), EditSkaterProfileError> {
        let _file = access();
        let mut next = Self::read(self.path.clone())?.profile.unwrap_or_else(SkaterProfile::new_empty);
        next.set_cac_record(record)?;
        if let Some(gestures) = gestures { next.set_gesture_settings(gestures)?; }
        if let Some(style) = style { next.set_skate3_style(style)?; }
        let bytes = next.encode()?;
        let validated = SkaterProfile::parse(&bytes)?;
        if let Some(gestures) = gestures {
            if validated.gesture_settings()? != gestures {
                return Err(skate2_formats::skater_profile::ProfileError("gesture settings read-back mismatch").into());
            }
        }
        if let Some(style) = style {
            if validated.skate3_style()? != style {
                return Err(ProfileError("Skate 3 style read-back mismatch").into());
            }
        }
        if validated.cac_record()?.as_ref() != Some(record) {
            return Err(skate2_formats::skater_profile::ProfileError(
                "CAC save read-back mismatch",
            )
            .into());
        }
        self.commit(&bytes, validated)
    }
    fn commit(&mut self, bytes: &[u8], validated: SkaterProfile) -> Result<(), EditSkaterProfileError> {
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let suffix = format!(".{}.{}", std::process::id(), sequence);
        let temp = self.path.with_extension(format!("P{suffix}.tmp"));
        let backup = self.path.with_extension(format!("P{suffix}.bak"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        if let Err(e) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temp);
            return Err(e.into());
        }
        drop(file);
        let had_original = self.path.try_exists()?;
        if had_original {
            if backup.try_exists()? {
                let _ = fs::remove_file(&temp);
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "SKATER.P backup path already exists",
                )
                .into());
            }
            if let Err(e) = fs::rename(&self.path, &backup) {
                let _ = fs::remove_file(&temp);
                return Err(e.into());
            }
        }
        if let Err(e) = fs::rename(&temp, &self.path) {
            if had_original {
                let _ = fs::rename(&backup, &self.path);
            }
            let _ = fs::remove_file(&temp);
            return Err(e.into());
        }
        self.profile = Some(validated);
        Ok(())
    }
}

#[cfg(test)]
mod display_store_tests {
    use super::*;
    use skate2_formats::skater_profile::{CacPartId, RecipeData};
    #[test]
    fn stale_editor_and_settings_owners_merge_latest_save() {
        let root = std::env::temp_dir().join(format!("sk82-profile-{}-{}", std::process::id(),
            SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();
        let path = root.join("SKATER.P");
        let mut editor = EditSkaterProfileStore::open(&path).unwrap();
        let mut settings = EditSkaterProfileStore::open(&path).unwrap();
        let values = PcDisplaySettings { postfx: false, fisheye: true, ..PcDisplaySettings::default() };
        settings.save_display_settings(values).unwrap();
        let record = CacRecord::new(RecipeData {
            recipe: vec![2,4,6], tuning_bits: [0.5f32.to_bits();2], hat_type: 0,
            stance: 0, style: 1, gestures: [0;4], is_male: true,
        }, CacPartId { model: 42, material: 43 }, vec![0;16]);
        editor.save_record(&record).unwrap();
        assert_eq!(EditSkaterProfileStore::open(&path).unwrap().load_display_settings().unwrap(), values);
        settings.save_display_settings(PcDisplaySettings { fisheye: false, ..values }).unwrap();
        let reopened = EditSkaterProfileStore::open(&path).unwrap();
        assert_eq!(reopened.load_record().unwrap(), Some(record));
        assert_eq!(reopened.load_display_settings().unwrap(), PcDisplaySettings { postfx: false, fisheye: false, ..values });
        // Only the uniquely-created test directory is removed.
        fs::remove_dir_all(root).unwrap();
    }
}
