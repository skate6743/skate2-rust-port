//! Stock normal-camera rig settings, bound by TU3 8252E288/8252EDD0.
//! Runtime offsets in camera kernels are relative to settings-object +16.
use skate3_core::{
    camera::{
        AnchorTrackingSettings, AngleTrackingSettings, AvoidanceSettings, DistanceTrackingSettings,
        DropSettings, FrameSettings, LookSettings, ManualCamSettings, OrientationSettings,
        OrientationTrackerSettings, RigPositioningSettings, RigSettings, ShakeSettings,
        ManagerSettings, CompassSettings,
    },
    point_graph::PointGraph,
};
use skate_data::collections::Collections;

#[derive(Clone, Copy, Debug)]
pub(crate) struct RigSettingsSource {
    heading: AngleTrackingSettings,
    elevation: AngleTrackingSettings,
    anchor: AnchorTrackingSettings,
    avoidance: AvoidanceSettings,
    positioning: RigPositioningSettings,
    collision_hold_duration: f32,
}

pub(crate) fn manager_settings(data: &Collections) -> Result<ManagerSettings, String> {
    Ok(ManagerSettings {
        rig: RigSettingsSource::load(data)?.bind(), orientation: orientation_settings(data)?,
        framing: frame_settings(data)?, drop: drop_settings(data)?, look: look_settings(data)?,
        shake: shake_settings(data)?,
        steering_threshold: data.float("camera", "dynamics", "TurningCentredDeadzone")?,
    })
}

pub(crate) fn compass_settings(data: &Collections) -> Result<CompassSettings, String> {
    let f = |field| data.float("camera_compass", "default", field);
    Ok(CompassSettings {
        heading_response: data.float("camera", "dynamics", "HeadingChangeResponseSpeed")?,
        time_before_lineup: f("TimeBeforeAutoLineup")?, lineup_speed: f("AutoLineupSpeed")?,
        minimum_deadzone_speed: f("MinDeadzoneSpeed")?, maximum_deadzone_speed: f("MaxDeadzoneSpeed")?,
        maximum_deadzone_size: f("MaxDeadzoneSize")?, deadzone_smoothing: f("DeadzoneSizeSmoothing")?,
    })
}

pub(crate) fn drop_settings(data: &Collections) -> Result<DropSettings, String> {
    // Runtime parameter652/656/660/668 correspond to settings object
    //668/672/676/684, bound from droppredictor layout208/204/220/228.
    Ok(DropSettings {
        minimum_test_distance: data.float("camera_droppredictor", "default", "MinTestDistance")?,
        total_test_time: data.float("camera_droppredictor", "default", "TotalTestTime")?,
        maximum_test_distance: data.float(
            "camera_droppredictor",
            "default",
            "Hash_FB5A1024D47298A1",
        )?,
        maximum_drop_distance: data.float("camera_droppredictor", "default", "MaxDropDistance")?,
    })
}

pub(crate) fn manual_cam_settings(data: &Collections) -> Result<ManualCamSettings, String> {
    let mut settings = ManualCamSettings::default();
    if let Ok(speed) = data.float("camera", "freecam", "FreeCamHeadingSpeed") {
        settings.move_speed = speed.max(1.0) * 4.0;
    }
    if let Ok(speed) = data.float("camera", "freecam", "FreeCamElevationSpeed") {
        settings.look_speed = speed.max(0.1);
    }
    Ok(settings)
}

pub(crate) fn look_settings(data: &Collections) -> Result<LookSettings, String> {
    // Settings object952/960/964/972 -> parameter936/944/948/956.
    Ok(LookSettings {
        heading_offset_degrees: data.float("camera", "freecam", "FreeCamHeadingOffset")?,
        heading_speed: data.float("camera", "freecam", "FreeCamHeadingSpeed")?,
        elevation_offset_degrees: data.float("camera", "freecam", "FreeCamElevationOffset")?,
        elevation_speed: data.float("camera", "freecam", "FreeCamElevationSpeed")?,
    })
}

pub(crate) fn shake_settings(data:&Collections)->Result<ShakeSettings,String> {
    let f=|field|data.float("camera_shake","default",field);
    let matrix=|field|->Result<[[f32;4];4],String> {
        let words=data.words::<16>("camera_shake","default",field)?;
        Ok(core::array::from_fn(|row|core::array::from_fn(|i|f32::from_bits(words[row*4+i]))))
    };
    Ok(ShakeSettings {
        amplitude_curve:matrix("AmplitudeCurve")?,frequency_curve:matrix("FrequencyCurve")?,
        impulse_magnitude:f("OneShotMagnitude")?,impulse_minimum_velocity:f("OneShotMinVelocity")?,
        impulse_maximum_velocity:f("OneShotMaxVelocity")?,impulse_frequency:f("OneShotFrequency")?,
        impulse_decay:f("OneShotDecay")?,amplitude_minimum:f("AmplitudeMin")?,
        amplitude_maximum:f("AmplitudeMax")?,amplitude_top_speed:f("AmplitudeTopSkaterSpeed")?,
        frequency_minimum:f("FrequencyMin")?,frequency_maximum:f("FrequencyMax")?,
        frequency_top_speed:f("FrequencyTopSkaterSpeed")?,data_frames_per_second:f("DataFPS")?,
        translation_multiplier:f("TranslationMultiplier")?,rotation_multiplier:f("RotationMultiplier")?,
        dutch_multiplier:f("DutchMultiplier")?,
    })
}

impl RigSettingsSource {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let f = |class, key, field| data.float(class, key, field);
        // 8252E6CC binds heading; 8252E680 binds elevation. These are
        // settings for the normal camera's angular trackers, in degrees.
        let angle = |key| -> Result<AngleTrackingSettings, String> {
            Ok(AngleTrackingSettings {
                speed_clamp_degrees: f("camera_tracker", key, "SpeedClamp")?,
                acceleration_min_degrees: f("camera_tracker", key, "AccelerationClampMin")?,
                acceleration_max_degrees: f("camera_tracker", key, "AccelerationClampMax")?,
                delta_umbra_degrees: f("camera_tracker", key, "DeltaUmbra")?,
                delta_penumbra_degrees: f("camera_tracker", key, "DeltaPenumbra")?,
            })
        };
        Ok(Self {
            heading: angle("heading")?,
            elevation: angle("elevation")?,
            anchor: AnchorTrackingSettings {
                latch_curve: curve(data, "camera_lag_profile", "push", "Curve")?,
                input_curve: curve(data, "camera_lag_profile", "pump", "Curve")?,
                speed_clamp: f("camera", "subject_tracker", "SubjectTrackerMaxSpeed")?,
                acceleration_clamp: f(
                    "camera",
                    "subject_tracker",
                    "SubjectTrackerMaxAcceleration",
                )?,
                latch_scale: f("camera_lag_profile", "push", "MaxSmoothing")?,
                input_threshold: f("camera", "dynamics", "PumpingMinAcceleration")?,
                input_scale: f("camera_lag_profile", "pump", "MaxSmoothing")?,
            },
            avoidance: AvoidanceSettings {
                horizon: f("camera", "dynamics", "AvoidanceLookahead")?,
                heading_smoothing: f("camera", "dynamics", "AvoidanceHeadingSmoothing")?,
                elevation_smoothing: f("camera", "dynamics", "AvoidanceElevationSmoothing")?,
                ease_out_time: f("camera", "dynamics", "AvoidanceEaseOut")?,
                radius_padding: f("camera_positioner", "default", "AvoidanceCollisionOffset")?,
            },
            positioning: RigPositioningSettings {
                normal: DistanceTrackingSettings {
                    speed_clamp: f("camera_tracker", "distance", "SpeedClamp")?,
                    smoothing: f("camera_tracker", "distance", "SmoothingConstant")?,
                    acceleration_clamp: f("camera_tracker", "distance", "AccelerationClampMax")?,
                },
                alternate: DistanceTrackingSettings {
                    speed_clamp: f("camera_tracker", "distance", "FastSpeedClamp")?,
                    // Native layout +12 has no recovered spelling. Its complete
                    // key is preserved by the stock collection conversion.
                    smoothing: f("camera_tracker", "distance", "Hash_E2D9D0FAAB8CB7F3")?,
                    acceleration_clamp: f(
                        "camera_tracker",
                        "distance",
                        "FastAccelerationClampMax",
                    )?,
                },
                minimum_distance: f("camera_positioner", "default", "MinDistance")?,
                alternate_minimum_distance: f(
                    "camera_positioner",
                    "default",
                    "MinDistanceDuringWipeout",
                )?,
            },
            collision_hold_duration: f("camera", "dynamics", "CollisionInterpolationDuration")?,
        })
    }

    pub fn bind(self) -> RigSettings {
        RigSettings {
            heading: self.heading,
            elevation: self.elevation,
            anchor: self.anchor,
            avoidance: self.avoidance,
            positioning: self.positioning,
            collision_hold_duration: self.collision_hold_duration,
            // 82F826F8 loads82181A88 with lvlx128, splats lane0, and stores
            // all four lanes at830BD350. This is not mapped BSS zero data.
            normalization_threshold: [f32::from_bits(0x358637bd); 4],
        }
    }
}

pub(crate) fn orientation_settings(data: &Collections) -> Result<OrientationSettings, String> {
    let f = |class, key, field| data.float(class, key, field);
    let tracker = |key| -> Result<OrientationTrackerSettings, String> {
        Ok(OrientationTrackerSettings {
            acceleration_min_degrees: f("camera_tracker", key, "AccelerationClampMin")?,
            acceleration_max_degrees: f("camera_tracker", key, "AccelerationClampMax")?,
            // 8252F448/8252F508 attribute lookup low wordD1CA9582.
            smoothing_min: f("camera_tracker", key, "SmoothingMin")?,
            delta_umbra_degrees: f("camera_tracker", key, "DeltaUmbra")?,
            delta_penumbra_degrees: f("camera_tracker", key, "DeltaPenumbra")?,
        })
    };
    Ok(OrientationSettings {
        pan_umbra_degrees: f("camera", "rotation", "PanUmbra")?,
        pan_penumbra_degrees: f("camera", "rotation", "PanPenumbra")?,
        pan_smoothing_curve: curve(data, "camera", "rotation", "PanSmoothingCurve")?,
        pan_minimum_smoothing: f("camera", "rotation", "PanMinSmoothing")?,
        pan: tracker("pan")?,
        tilt: tracker("tilt")?,
    })
}

pub(crate) fn frame_settings(data: &Collections) -> Result<FrameSettings, String> {
    // 8252EE80/8252EE90 read camera/dynamics layout624/576 into
    // settings-object660/628, hence parameter644/612.
    Ok(FrameSettings {
        maximum_pitch_degrees: data.float("camera", "dynamics", "MaxPitch")?,
        roll_response: data.float("camera", "dynamics", "DutchInterpolationSpeed")?,
    })
}

fn curve(data: &Collections, class: &str, key: &str, name: &str) -> Result<PointGraph<8>, String> {
    let words = data.words::<16>(class, key, name)?;
    let values = words.map(f32::from_bits);
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("Non-finite camera curve {class}/{key}/{name}"));
    }
    Ok(PointGraph {
        x: core::array::from_fn(|i| values[i]),
        y: core::array::from_fn(|i| values[i + 8]),
    })
}
