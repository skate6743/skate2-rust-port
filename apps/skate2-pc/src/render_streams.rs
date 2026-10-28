//! The stream files a visual focus loads collections from: BAM's, then a loaded additional
//! world's. `cAssetStreamSystem` holds every stream file of a type, `FindCollections`
//! (`0x824dfea8`) selects from them all, and a collection loads from its own file's folder.
use skate2_core::game_stream::StreamPaths;
use skate2_formats::stream::{CollectionMap, CollectionMapEntry};
use std::sync::Arc;

/// One stream file: its collection map, its folder (`data\content\world\stream\<world>\`) and its
/// manifest stem (`<world>_Pres`), whose `.xst` holds its asset table.
#[derive(Clone, Debug)]
pub(crate) struct RenderStreamFile {
    pub(crate) map: Arc<CollectionMap>,
    directory: String,
    stem: String,
}

impl RenderStreamFile {
    pub(crate) fn from_paths(map: Arc<CollectionMap>, paths: &StreamPaths) -> Self {
        Self { map, directory: paths.collection_directory.clone(), stem: paths.manifest_stem.clone() }
    }

    /// BAM's file of a type (`Pres`, `Tex`).
    pub(crate) fn bam(map: CollectionMap, suffix: &str) -> Self {
        Self {
            map: Arc::new(map),
            directory: "data\\content\\world\\stream\\BAM\\".into(),
            stem: format!("data\\content\\world\\stream\\BAM\\BAM_{suffix}"),
        }
    }

    /// A collection's `.xsf`.
    pub(crate) fn collection_path(&self, entry: &CollectionMapEntry) -> String {
        format!("{}{}.xsf", self.directory, entry.filename)
    }

    /// The asset table (`.xst`).
    pub(crate) fn asset_toc_path(&self) -> String {
        format!("{}.xst", self.stem)
    }
}

/// The files of one stream type, in load order.
#[derive(Clone, Debug)]
pub(crate) struct RenderStreams {
    pub(crate) files: Vec<RenderStreamFile>,
}

impl RenderStreams {
    pub(crate) fn single(file: RenderStreamFile) -> Self {
        Self { files: vec![file] }
    }

    /// The collection's map entry and the file that holds it.
    pub(crate) fn entry(&self, id: u64) -> Option<(&CollectionMapEntry, &RenderStreamFile)> {
        self.files.iter().find_map(|file| file.map.collections.iter().find(|entry| entry.id == id).map(|entry| (entry, file)))
    }

    /// Every file's collections.
    pub(crate) fn entries(&self) -> impl Iterator<Item = &CollectionMapEntry> {
        self.files.iter().flat_map(|file| file.map.collections.iter())
    }
}
