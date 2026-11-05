use serde::Deserialize;
use std::{
    fmt,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameAssets {
    pub version: u32,
    pub action_graph: String,
    pub motion_graph: String,
}

#[derive(Debug)]
pub struct AssetError(pub String);
impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AssetError {}

impl GameAssets {
    /// The stock graphs the physics player runs, from the linked data.
    pub fn load(root: &Path) -> Result<Self, AssetError> {
        let path = root.join("private/game.json");
        let json = crate::asset_io::read_to_string(&path)
            .map_err(|e| AssetError(format!("{}: {e}", path.display())))?;
        let manifest = Self::parse(&json)?;
        for relative in [&manifest.action_graph, &manifest.motion_graph] {
            if !crate::asset_io::is_file(root.join(relative)) {
                return Err(AssetError(format!("Stock graph missing: {relative}")));
            }
        }
        Ok(manifest)
    }

    pub fn load_physics(root: &Path) -> Result<Self, AssetError> {
        Self::load(root)
    }

    pub fn parse(json: &str) -> Result<Self, AssetError> {
        let manifest: Self = serde_json::from_str(json)
            .map_err(|e| AssetError(format!("Invalid game asset manifest: {e}")))?;
        if manifest.version != 1 {
            return Err(AssetError(format!(
                "Unsupported asset manifest version {}",
                manifest.version
            )));
        }
        validate_path(&manifest.action_graph, "action_graph", "stategraph")?;
        validate_path(&manifest.motion_graph, "motion_graph", "stategraph")?;
        Ok(manifest)
    }
}

fn validate_path(value: &str, field: &str, extension: &str) -> Result<(), AssetError> {
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty()
        || !path.components().all(|c| matches!(c, Component::Normal(_)))
        || value.contains([':', '\\', '#'])
    {
        return Err(AssetError(format!(
            "{field} must be a relative asset path using forward slashes"
        )));
    }
    if path.extension().and_then(|x| x.to_str()) != Some(extension) {
        return Err(AssetError(format!("{field} must name a .{extension} file")));
    }
    Ok(())
}
