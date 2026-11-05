//! PC mod gesture preferences in SKATER.P's existing SK82_PC extension.
//! The four stock Skate 2 choices stay in CACRecord. This versioned leaf
//! stores the selected catalog and its independent Skate 3 assignments.
use super::{ProfileError, Result, SkaterProfile};

const SAVABLE: &str = "SK82_PC";
const LEAF: &str = "SK82_PC::GestureSets";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GestureSet {
    #[default]
    Skate2,
    Skate3,
}

impl GestureSet {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Skate2 => "Skate 2",
            Self::Skate3 => "Skate 3",
        }
    }

    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Skate2 => Self::Skate3,
            Self::Skate3 => Self::Skate2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcGestureSettings {
    pub selected: GestureSet,
    /// Up, Down, Left, Right; native Skate 3 CharacterGesture table ordinals.
    pub skate3: [u32; 4],
}

impl Default for PcGestureSettings {
    fn default() -> Self {
        // Skate 3 ResetGestureSet824FA730 selects the first four entries.
        Self {
            selected: GestureSet::Skate2,
            skate3: [0, 1, 2, 3],
        }
    }
}

impl PcGestureSettings {
    pub fn validate(self) -> Result<()> {
        if self.skate3.iter().any(|&id| id >= 37) {
            return Err(ProfileError("invalid Skate 3 character gesture selection"));
        }
        Ok(())
    }
}

impl SkaterProfile {
    pub fn gesture_settings(&self) -> Result<PcGestureSettings> {
        let Some(leaf) = self.leaf(SAVABLE, LEAF)? else {
            return Ok(PcGestureSettings::default());
        };
        if leaf.version != 1 {
            return Err(ProfileError("unsupported PC gesture settings version"));
        }
        let [set, slots] = leaf.tuples.as_slice() else {
            return Err(ProfileError("invalid PC gesture settings tuples"));
        };
        let selected = match set.as_slice() {
            [0] => GestureSet::Skate2,
            [1] => GestureSet::Skate3,
            _ => return Err(ProfileError("invalid gesture set selection")),
        };
        if slots.len() != 16 {
            return Err(ProfileError("invalid Skate 3 gesture slot storage"));
        }
        let result = PcGestureSettings {
            selected,
            skate3: std::array::from_fn(|slot| {
                u32::from_be_bytes(
                    slots[slot * 4..slot * 4 + 4]
                        .try_into()
                        .expect("four checked bytes"),
                )
            }),
        };
        result.validate()?;
        Ok(result)
    }

    pub fn set_gesture_settings(&mut self, settings: PcGestureSettings) -> Result<()> {
        settings.validate()?;
        // Do not overwrite a newer or malformed extension with a default.
        self.gesture_settings()?;
        self.set_leaf(
            SAVABLE,
            &["CAC"],
            LEAF,
            1,
            &[
                vec![u8::from(settings.selected == GestureSet::Skate3)],
                settings
                    .skate3
                    .iter()
                    .flat_map(|id| id.to_be_bytes())
                    .collect(),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a copied SKATE2_SAVE_ROOT/SKATER.P"]
    fn original_profile_keeps_cac_and_unknown_chunks_when_gesture_sets_are_saved() {
        let path = std::path::PathBuf::from(
            std::env::var_os("SKATE2_SAVE_ROOT").expect("copied save root"),
        )
        .join("SKATER.P");
        let bytes = std::fs::read(&path).unwrap();
        let mut profile = SkaterProfile::parse(&bytes).unwrap();
        let original = profile.clone();
        let expected = PcGestureSettings {
            selected: GestureSet::Skate3,
            skate3: [36, 17, 5, 29],
        };
        profile.set_gesture_settings(expected).unwrap();
        let loaded = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
        assert_eq!(loaded.gesture_settings().unwrap(), expected);
        assert_eq!(loaded.cac_record().unwrap(), original.cac_record().unwrap());
        for chunk in &original.chunks {
            if super::super::read32(chunk, 0).unwrap() != super::super::name_hash(SAVABLE) {
                assert!(
                    loaded.chunks.contains(chunk),
                    "an existing native save chunk changed"
                );
            }
        }
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn gesture_sets_round_trip_without_changing_other_save_data() {
        let mut profile = SkaterProfile::new_empty();
        profile.set_display_settings(Default::default()).unwrap();
        profile
            .set_leaf("FUTURE", &["CAC"], "opaque", 23, &[vec![9, 8, 7, 6]])
            .unwrap();
        let unknown = profile.leaf("FUTURE", "opaque").unwrap();
        let cac = profile.cac_record().unwrap();
        assert_eq!(
            profile.gesture_settings().unwrap(),
            PcGestureSettings::default()
        );
        let mut expected = PcGestureSettings {
            selected: GestureSet::Skate3,
            skate3: [36, 17, 5, 29],
        };
        for selected in [GestureSet::Skate3, GestureSet::Skate2, GestureSet::Skate3] {
            expected.selected = selected;
            profile.set_gesture_settings(expected).unwrap();
            profile.set_display_settings(Default::default()).unwrap();
            profile = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
            assert_eq!(profile.gesture_settings().unwrap(), expected);
            assert_eq!(profile.cac_record().unwrap(), cac);
            assert_eq!(profile.leaf("FUTURE", "opaque").unwrap(), unknown);
        }
    }

    #[test]
    fn bad_gesture_data_is_rejected_before_replacement() {
        let mut profile = SkaterProfile::new_empty();
        let before = profile.encode().unwrap();
        assert!(
            profile
                .set_gesture_settings(PcGestureSettings {
                    skate3: [37, 1, 2, 3],
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(profile.encode().unwrap(), before);
        profile
            .set_leaf(SAVABLE, &["CAC"], LEAF, 2, &[vec![1], vec![0; 16]])
            .unwrap();
        let future = profile.encode().unwrap();
        assert!(profile.gesture_settings().is_err());
        assert!(profile.set_gesture_settings(Default::default()).is_err());
        assert_eq!(profile.encode().unwrap(), future);
    }
}
