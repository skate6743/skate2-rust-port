//! Host preference selects the native physics_mode collection, never a multiplier.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) const NATIVE_MODES: [&str; 5] = ["easy", "normal", "hardcore", "motorized", "test"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u32)]
pub(crate) enum Difficulty {
    #[default]
    Easy = 0,
    Normal = 1,
    Hardcore = 2,
    Motorized = 3,
    Custom = 4,
}
impl Difficulty {
    pub const ALL: [Self; 5] = [Self::Easy, Self::Normal, Self::Hardcore, Self::Motorized, Self::Custom];
    pub fn key(self) -> &'static str { if self == Self::Custom { "custom" } else { self.profile_key() } }
    // Custom owns the unused native test slot; stock modes retain their indices.
    pub fn profile_key(self) -> &'static str { NATIVE_MODES[self as usize] }
    pub fn label(self) -> &'static str {
        match self { Self::Easy => "Easy", Self::Normal => "Normal", Self::Hardcore => "Hardcore", Self::Motorized => "Motorized", Self::Custom => "Custom" }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL.into_iter().find(|d| d.key().eq_ignore_ascii_case(value))
            .ok_or_else(|| format!("Unknown difficulty {value:?}; expected easy, normal, hardcore, motorized or custom"))
    }
    pub fn path(root: &Path) -> PathBuf {
        root.parent().unwrap_or(root).join("settings/gameplay.json")
    }
    pub fn load(root: &Path) -> Result<Self, String> {
        match std::fs::read(Self::path(root)) {
            Ok(bytes) => serde_json::from_slice::<Saved>(&bytes).map(|s| s.difficulty)
                .map_err(|e| format!("Invalid saved gameplay settings: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Cannot read gameplay settings: {e}")),
        }
    }
    pub fn save(self, root: &Path) -> Result<(), String> {
        let path = Self::path(root);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, serde_json::to_vec_pretty(&Saved { difficulty: self }).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
}
#[derive(Serialize, Deserialize)]
struct Saved { difficulty: Difficulty }

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn retail_indices_and_saved_names_agree() {
        for (index, d) in Difficulty::ALL.into_iter().enumerate() {
            assert_eq!(d as usize, index);
            assert_eq!(Difficulty::parse(d.label()).unwrap(), d);
            let bytes = serde_json::to_vec(&Saved { difficulty: d }).unwrap();
            assert_eq!(serde_json::from_slice::<Saved>(&bytes).unwrap().difficulty, d);
        }
        assert!(Difficulty::parse("test").is_err());
        assert!(serde_json::from_str::<Saved>(r#"{"difficulty":"made-up"}"#).is_err());
    }
}
