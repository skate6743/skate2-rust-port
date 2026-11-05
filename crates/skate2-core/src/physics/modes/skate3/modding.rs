pub(crate) mod player_physics;
pub(crate) mod moving_queries;

/// The source engine's per-Part and per-joint overrides (its mods' extension records). Nothing
/// sets them in this game, so every Part and joint keeps its stock behaviour.
pub(crate) mod extensions {
    #[derive(Clone, Debug, Default)]
    pub(crate) struct JointOverride {
        pub(crate) swing_limit: Option<f32>,
        pub(crate) twist_limit: Option<f32>,
        pub(crate) free_swing: Option<bool>,
        pub(crate) free_twist: Option<bool>,
        pub(crate) drive_enabled: Option<bool>,
        pub(crate) enabled: Option<bool>,
        pub(crate) descendants: bool,
        pub(crate) possession_enabled: Option<bool>,
    }
    #[derive(Clone, Debug)]
    pub(crate) struct NativeBodyRef {
        pub(crate) kind: String,
        pub(crate) index: usize,
    }
    #[derive(Clone, Debug, Default)]
    pub(crate) struct PartOverride {
        pub(crate) motion: Option<String>,
        pub(crate) collision: Option<bool>,
        pub(crate) friction: Option<f32>,
        pub(crate) animation_drives: Option<bool>,
        pub(crate) possession_drives: Option<bool>,
    }
}
