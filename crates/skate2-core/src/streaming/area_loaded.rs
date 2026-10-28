//! `cAssetStreamSystem::IsAreaLoaded` (`0x824e0340`): whether the high collections a stream focus
//! would want around a point are in, as `cTeleportManager::TeleportRequiresLoadScreen`
//! (`0x828a9420`) asks it for the simulation and presentation foci.

use crate::game_stream::StreamKind;
use skate2_formats::{
    hash::rw_hash64_string,
    stream::{CollectionSpace, DEFAULT_PRESENTATION_FOCUS_COLLECTIONS, MAX_FOCUS_ENTRIES, high_detail_collection_filename, merge_focus_collections},
    stream_xml::StreamXml,
};
use std::sync::Arc;

/// The presentation stream as the renderer streams it, which `IsAreaLoaded` reads for the
/// presentation focus.
#[derive(Clone, Debug)]
pub struct PresentationStreamArea {
    /// The presentation stream's XSS.
    pub space: Arc<CollectionSpace>,
    /// Its stream XML, used with `kEnableStreamXml`.
    pub xml: Option<Arc<StreamXml>>,
    /// A loaded additional world's presentation XSS, whose collections `FindCollections`
    /// (`0x824dfea8`) merges with the first file's.
    pub additional: Option<Arc<CollectionSpace>>,
    /// The focus radius.
    pub radius: f32,
    /// The high collections registered in the presentation.
    pub resident: Vec<u64>,
}

impl PresentationStreamArea {
    /// `cAssetStreamSystem::IsAreaLoaded` (`0x824e0340`) for the presentation focus: a level-0
    /// collection of the XSS counts as loaded once the presentation has registered it, any other
    /// id as loaded; `FindCollections` is the renderer's focus selection at the point
    /// (`cStreamFile::FindCollections` `0x824df068`, at most 16).
    #[must_use]
    pub fn is_area_loaded(&self, position: [f32; 3], radius: f32, xml_enabled: bool) -> bool {
        let loaded = |id: u64| {
            let known = std::iter::once(&*self.space)
                .chain(self.additional.as_deref())
                .any(|space| space.collections.iter().any(|entry| entry.lod == 0 && entry.id == id));
            Ok::<_, std::convert::Infallible>(!known || self.resident.contains(&id))
        };
        let wanted = |at: [f32; 3]| {
            let mut picked = match self.xml.as_deref().filter(|_| xml_enabled) {
                Some(xml) => xml.focus_collections(&self.space, at, self.radius, 16),
                None => self.space.focus_collections(at, self.radius, 16),
            };
            if let Some(more) = self.additional.as_deref() {
                picked = merge_focus_collections([picked, more.focus_collections(at, self.radius, MAX_FOCUS_ENTRIES)]);
                picked.truncate(DEFAULT_PRESENTATION_FOCUS_COLLECTIONS);
            }
            Ok(picked.iter().map(|collection| collection.id).collect())
        };
        match is_area_loaded(position, radius, StreamKind::Presentation, loaded, wanted) {
            Ok(loaded) => loaded,
            Err(never) => match never {},
        }
    }
}

/// `cAssetStreamSystem::GetAreaTestPoints` (`0x824d7460`): the corners and edge midpoints of the
/// square `radius` out from `position` on x and z, in its order (the centre is not one of them).
#[must_use]
pub fn area_test_points(position: [f32; 3], radius: f32) -> [[f32; 3]; 8] {
    let [x, y, z] = position;
    [
        (radius, radius),
        (radius, 0.0),
        (radius, -radius),
        (0.0, radius),
        (0.0, -radius),
        (-radius, radius),
        (-radius, 0.0),
        (-radius, -radius),
    ]
    .map(|(dx, dz)| [x + dx, y, z + dz])
}

/// `cAssetStreamSystem::GetCollectionId` (`0x824d73b0`): `RwHash64String` of the high collection
/// name of the 200 m cell holding `point`, for the focus's stream.
#[must_use]
pub fn high_collection_id(kind: StreamKind, point: [f32; 3]) -> u64 {
    rw_hash64_string(&high_detail_collection_filename(kind.suffix(), point))
}

/// `cAssetStreamSystem::IsAreaLoaded` (`0x824e0340`) for a focus of `kind`: every test point's
/// cell, each once, must be loaded (`collection_loaded` is `IsCollectionLoaded(id, true)`
/// `0x824daa58`, which takes a collection its stream does not have as loaded) or not among the
/// collections `FindCollections` (`0x824dfea8`) picks for the focus moved to `position`;
/// `wanted_at` gives those, and is asked at most once.
///
/// # Errors
///
/// Whatever `collection_loaded` or `wanted_at` return.
pub fn is_area_loaded<E>(
    position: [f32; 3],
    radius: f32,
    kind: StreamKind,
    mut collection_loaded: impl FnMut(u64) -> Result<bool, E>,
    wanted_at: impl FnOnce([f32; 3]) -> Result<Vec<u64>, E>,
) -> Result<bool, E> {
    let mut checked = Vec::with_capacity(8);
    let mut find = Some(wanted_at);
    let mut wanted: Option<Vec<u64>> = None;
    for point in area_test_points(position, radius) {
        let id = high_collection_id(kind, point);
        if checked.contains(&id) {
            continue;
        }
        if !collection_loaded(id)? {
            if wanted.is_none()
                && let Some(find) = find.take()
            {
                wanted = Some(find(position)?);
            }
            if wanted.as_ref().is_some_and(|wanted| wanted.contains(&id)) {
                return Ok(false);
            }
        }
        checked.push(id);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_points_ring_the_square_without_its_centre() {
        let points = area_test_points([10.0, 2.0, -5.0], 20.0);
        assert_eq!(points[0], [30.0, 2.0, 15.0]);
        assert_eq!(points[3], [10.0, 2.0, 15.0]);
        assert_eq!(points[7], [-10.0, 2.0, -25.0]);
        assert!(!points.contains(&[10.0, 2.0, -5.0]));
    }

    #[test]
    fn an_unloaded_cell_fails_only_when_the_focus_there_wants_it() {
        let wanted = high_collection_id(StreamKind::Simulation, [150.0, 0.0, 150.0]);
        let loaded = |id: u64| Ok::<_, ()>(id != wanted);
        assert_eq!(is_area_loaded([150.0, 0.0, 150.0], 20.0, StreamKind::Simulation, loaded, |_| Ok(vec![wanted])), Ok(false));
        assert_eq!(is_area_loaded([150.0, 0.0, 150.0], 20.0, StreamKind::Simulation, loaded, |_| Ok(Vec::new())), Ok(true));
        assert_eq!(is_area_loaded([150.0, 0.0, 150.0], 20.0, StreamKind::Simulation, |_| Ok(true), |_| Err(())), Ok(true));
    }
}
