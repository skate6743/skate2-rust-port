//! Original TU3 gesture membership,82BA10C0 ->82BA07F0.
//! The catalog is constructed by82B98DB8 ->82B98F70, independently checked
//! against all270 raw map-insertion calls. PAT paths do not define these groups.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Square,
    Nose,
    Tail,
    Nose90,
    Tail90,
    NoseN90,
    TailN90,
}

impl Group {
    /// Original82BA0F78 does case-sensitive constructor comparisons.
    pub(crate) fn parse(name: &str) -> Result<Self, String> {
        Ok(match name {
            "Square" => Self::Square,
            "Nose" => Self::Nose,
            "Tail" => Self::Tail,
            "90Nose" => Self::Nose90,
            "90Tail" => Self::Tail90,
            "N90Nose" => Self::NoseN90,
            "N90Tail" => Self::TailN90,
            _ => return Err(format!("HasGestureIntent has undefined group `{name}`")),
        })
    }

    ///82BA07F0 tests map-key presence, never the intent value. All seven maps
    ///contain the same30 unrotated keys; each rotated map adds its15 own keys.
    pub(crate) fn has_intent(self, has: impl Fn(&str) -> bool) -> bool {
        let additional: &[&str] = match self {
            Self::Square | Self::Nose | Self::Tail => &[],
            Self::Nose90 => &NOSE_90,
            Self::Tail90 => &TAIL_90,
            Self::NoseN90 => &NOSE_N90,
            Self::TailN90 => &TAIL_N90,
        };
        COMMON.iter().chain(additional).any(|name| has(name))
    }
}

// Square82B99060..99DAC; Tail82B99E20..9AAD0; Nose82B9AB40..9B7F0.
const COMMON: [&str; 30] = [
    "Ollie",
    "PopShuvit",
    "FSPopShuvit",
    "VarialKickflip",
    "VarialHeelflip",
    "Hardflip",
    "InwardHeelflip",
    "360PopShuvit",
    "FS360PopShuvit",
    "360Flip",
    "Laserflip",
    "360Hardflip",
    "360InwardHeelflip",
    "Kickflip",
    "Heelflip",
    "Nollie",
    "N_PopShuvit",
    "N_FSPopShuvit",
    "N_VarialKickflip",
    "N_VarialHeelflip",
    "N_Hardflip",
    "N_InwardHeelflip",
    "N_360PopShuvit",
    "N_FS360PopShuvit",
    "N_360Flip",
    "N_Laserflip",
    "N_360Hardflip",
    "N_360InwardHeelflip",
    "N_Kickflip",
    "N_Heelflip",
];
// Tail90 map5676,82B9B864..9BE84; commonkeys82B9BEF4..9CBA4.
const TAIL_90: [&str; 15] = [
    "90_Ollie",
    "90_PopShuvit",
    "90_FSPopShuvit",
    "90_VarialKickflip",
    "90_VarialHeelflip",
    "90_Hardflip",
    "90_InwardHeelflip",
    "90_360PopShuvit",
    "90_FS360PopShuvit",
    "90_360Flip",
    "90_Laserflip",
    "90_360Hardflip",
    "90_360InwardHeelflip",
    "90_Kickflip",
    "90_Heelflip",
];
// TailN90 map9460,82B9CC18..9D238; commonkeys82B9D2A8..9DF58.
const TAIL_N90: [&str; 15] = [
    "N90_Ollie",
    "N90_PopShuvit",
    "N90_FSPopShuvit",
    "N90_VarialKickflip",
    "N90_VarialHeelflip",
    "N90_Hardflip",
    "N90_InwardHeelflip",
    "N90_360PopShuvit",
    "N90_FS360PopShuvit",
    "N90_360Flip",
    "N90_Laserflip",
    "N90_360Hardflip",
    "N90_360InwardHeelflip",
    "N90_Kickflip",
    "N90_Heelflip",
];
// Nose90 map3784,82B9DFCC..9E5EC; commonkeys82B9E65C..9F30C.
const NOSE_90: [&str; 15] = [
    "90_Nollie",
    "90_N_PopShuvit",
    "90_N_FSPopShuvit",
    "90_N_VarialKickflip",
    "90_N_VarialHeelflip",
    "90_N_Hardflip",
    "90_N_InwardHeelflip",
    "90_N_360PopShuvit",
    "90_N_FS360PopShuvit",
    "90_N_360Flip",
    "90_N_Laserflip",
    "90_N_360Hardflip",
    "90_N_360InwardHeelflip",
    "90_N_Kickflip",
    "90_N_Heelflip",
];
// NoseN90 map7568,82B9F380..9F9A0; commonkeys82B9FA10..BA06C0.
const NOSE_N90: [&str; 15] = [
    "N90_Nollie",
    "N90_N_PopShuvit",
    "N90_N_FSPopShuvit",
    "N90_N_VarialKickflip",
    "N90_N_VarialHeelflip",
    "N90_N_Hardflip",
    "N90_N_InwardHeelflip",
    "N90_N_360PopShuvit",
    "N90_N_FS360PopShuvit",
    "N90_N_360Flip",
    "N90_N_Laserflip",
    "N90_N_360Hardflip",
    "N90_N_360InwardHeelflip",
    "N90_N_Kickflip",
    "N90_N_Heelflip",
];
