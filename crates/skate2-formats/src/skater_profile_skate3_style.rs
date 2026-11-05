//! PC Skate 3 animation and board choices, independent of the native Skate 2 CAC record.
//! GetCACSettings82590D20 maps style; posture profile mapping is82590B50. Skate 3's CAC record
//! keeps its own truck tightness and wheel hardness (+0x2004/+0x2008), which the Skate 3
//! physics reads through PlayersBackEnd (82593998/825939B0).
use super::{ProfileError, Result, SkaterProfile};

const SAVABLE: &str = "SK82_PC";
const LEAF: &str = "SK82_PC::Skate3Style";
/// A leaf of its own, so builds that read only the style leaf keep reading this profile.
const EQUIPMENT_LEAF: &str = "SK82_PC::Skate3Equipment";
/// CASPlayerInitializer::InitializePlayer 8253D4BC/8253D4C8: a new Skate 3 skater's truck
/// tightness and wheel hardness (flt_821161AC). Skate 2's are 0.3 and 0.7.
const DEFAULT_TUNING: f32 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcSkate3Style {
    pub style: u32,
    pub posture: u32,
    /// Truck tightness and wheel hardness, as `f32` bits in `0..=1`.
    pub tuning_bits: [u32; 2],
}

impl Default for PcSkate3Style {
    fn default() -> Self {
        Self { style: 0, posture: 0, tuning_bits: [DEFAULT_TUNING.to_bits(); 2] }
    }
}

impl PcSkate3Style {
    pub const STYLES: [&'static str; 4] = ["Default", "Loose", "Gonzo", "Aggressive"];
    pub const POSTURES: [&'static str; 4] = ["Default", "Stiff", "Slouch", "Buff"];

    pub fn validate(self) -> Result<()> {
        if self.style >= Self::STYLES.len() as u32 || self.posture >= Self::POSTURES.len() as u32 {
            return Err(ProfileError("invalid Skate 3 style or posture selection"));
        }
        if !self.tuning_bits.map(f32::from_bits).iter().all(|value| (0.0..=1.0).contains(value)) {
            return Err(ProfileError("invalid Skate 3 truck or wheel setting"));
        }
        Ok(())
    }

    #[must_use]
    pub fn truck_tightness(self) -> f32 {
        f32::from_bits(self.tuning_bits[0])
    }

    #[must_use]
    pub fn wheel_hardness(self) -> f32 {
        f32::from_bits(self.tuning_bits[1])
    }
}

impl SkaterProfile {
    pub fn skate3_style(&self) -> Result<PcSkate3Style> {
        let Some(leaf) = self.leaf(SAVABLE, LEAF)? else {
            return Ok(PcSkate3Style::default());
        };
        if leaf.version != 1 {
            return Err(ProfileError("unsupported PC Skate 3 style version"));
        }
        let [style, posture] = leaf.tuples.as_slice() else {
            return Err(ProfileError("invalid PC Skate 3 style tuples"));
        };
        let settings = PcSkate3Style {
            style: u32::from_be_bytes(
                style
                    .as_slice()
                    .try_into()
                    .map_err(|_| ProfileError("invalid Skate 3 style storage"))?,
            ),
            posture: u32::from_be_bytes(
                posture
                    .as_slice()
                    .try_into()
                    .map_err(|_| ProfileError("invalid Skate 3 posture storage"))?,
            ),
            tuning_bits: self.skate3_tuning_bits()?,
        };
        settings.validate()?;
        Ok(settings)
    }

    /// The equipment leaf's truck and wheel bits, or a new Skate 3 skater's without it.
    fn skate3_tuning_bits(&self) -> Result<[u32; 2]> {
        let Some(leaf) = self.leaf(SAVABLE, EQUIPMENT_LEAF)? else {
            return Ok(PcSkate3Style::default().tuning_bits);
        };
        if leaf.version != 1 {
            return Err(ProfileError("unsupported PC Skate 3 equipment version"));
        }
        let [truck, wheel] = leaf.tuples.as_slice() else {
            return Err(ProfileError("invalid PC Skate 3 equipment tuples"));
        };
        let bits = |tuple: &Vec<u8>| {
            tuple
                .as_slice()
                .try_into()
                .map(u32::from_be_bytes)
                .map_err(|_| ProfileError("invalid Skate 3 equipment storage"))
        };
        Ok([bits(truck)?, bits(wheel)?])
    }

    pub fn set_skate3_style(&mut self, settings: PcSkate3Style) -> Result<()> {
        settings.validate()?;
        // A malformed or future leaf must not be overwritten with defaults.
        self.skate3_style()?;
        self.set_leaf(
            SAVABLE,
            &["CAC"],
            LEAF,
            1,
            &[
                settings.style.to_be_bytes().to_vec(),
                settings.posture.to_be_bytes().to_vec(),
            ],
        )?;
        self.set_leaf(SAVABLE, &["CAC"], EQUIPMENT_LEAF, 1, &settings.tuning_bits.map(|bits| bits.to_be_bytes().to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_and_postures_survive_other_profile_writers_without_changing_native_choices() {
        let mut profile = SkaterProfile::new_empty();
        profile
            .set_leaf("FUTURE", &["CAC"], "opaque", 23, &[vec![9, 8, 7, 6]])
            .unwrap();
        let unknown = profile.leaf("FUTURE", "opaque").unwrap();
        let cac = profile.cac_record().unwrap();
        assert_eq!(profile.skate3_style().unwrap(), PcSkate3Style::default());
        assert_eq!(PcSkate3Style::default().truck_tightness(), 0.7);
        let gestures = super::super::PcGestureSettings {
            selected: super::super::GestureSet::Skate3,
            skate3: [36, 17, 5, 29],
        };
        for style in 0..4 {
            for posture in 0..4 {
                let expected = PcSkate3Style { style, posture, tuning_bits: [(style as f32 / 4.0).to_bits(), (posture as f32 / 3.0).to_bits()] };
                profile.set_skate3_style(expected).unwrap();
                profile.set_gesture_settings(gestures).unwrap();
                profile.set_display_settings(Default::default()).unwrap();
                profile = SkaterProfile::parse(&profile.encode().unwrap()).unwrap();
                assert_eq!(profile.skate3_style().unwrap(), expected);
                assert_eq!(profile.gesture_settings().unwrap(), gestures);
                assert_eq!(profile.cac_record().unwrap(), cac);
                assert_eq!(profile.leaf("FUTURE", "opaque").unwrap(), unknown);
            }
        }
    }

    #[test]
    fn malformed_and_future_style_extensions_are_not_replaced() {
        let mut profile = SkaterProfile::new_empty();
        let before = profile.encode().unwrap();
        for settings in [
            PcSkate3Style {
                style: 4,
                ..PcSkate3Style::default()
            },
            PcSkate3Style {
                posture: 4,
                ..PcSkate3Style::default()
            },
            PcSkate3Style {
                tuning_bits: [1.1f32.to_bits(), 0.7f32.to_bits()],
                ..PcSkate3Style::default()
            },
        ] {
            assert!(profile.set_skate3_style(settings).is_err());
            assert_eq!(profile.encode().unwrap(), before);
        }
        for (version, tuples) in [
            (2, vec![vec![0; 4], vec![0; 4]]),
            (1, vec![vec![0; 3], vec![0; 4]]),
            (1, vec![vec![0; 4]]),
            (1, vec![4_u32.to_be_bytes().to_vec(), vec![0; 4]]),
        ] {
            let mut profile = SkaterProfile::new_empty();
            profile
                .set_leaf(SAVABLE, &["CAC"], LEAF, version, &tuples)
                .unwrap();
            let before = profile.encode().unwrap();
            assert!(profile.skate3_style().is_err());
            assert!(profile.set_skate3_style(Default::default()).is_err());
            assert_eq!(profile.encode().unwrap(), before);
        }
    }

    /// A profile saved before the equipment leaf reads a new Skate 3 skater's trucks and wheels.
    #[test]
    fn a_style_leaf_alone_keeps_skate3_default_equipment() {
        let mut profile = SkaterProfile::new_empty();
        profile
            .set_leaf(SAVABLE, &["CAC"], LEAF, 1, &[2_u32.to_be_bytes().to_vec(), 1_u32.to_be_bytes().to_vec()])
            .unwrap();
        let settings = profile.skate3_style().unwrap();
        assert_eq!((settings.style, settings.posture), (2, 1));
        assert_eq!(settings.tuning_bits, [0.7f32.to_bits(); 2]);
    }
}
