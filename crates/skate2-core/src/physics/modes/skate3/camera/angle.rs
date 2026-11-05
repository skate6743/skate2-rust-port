//! Camera graph selection from upstream. Skate 2 owns persistence.
use serde::{Deserialize,Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CameraAngle {
    /// Graph type 0.
    Low,
    /// Graph type 1. The engine's default: retail asks at first boot instead (boot flow
    /// "Choose your camera:"), so there is no shipped default to copy.
    #[default]
    High,
}

impl CameraAngle {
    pub const ALL: [Self; 2] = [Self::High, Self::Low];
    /// The stock camera graph's `IsCameraTypeActive` value.
    pub fn graph_type(self) -> u32 {
        match self {
            Self::Low => 0,
            Self::High => 1,
        }
    }
    /// Retail labels (`ID_GAMESETTINGS_CAMERA_LOW` / `_HIGH`).
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::High => "High",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }
}

