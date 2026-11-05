//! Native host assets through the existing source loose-octree/contact path.
use super::super::{
    octree::{Bounds, Octree},
    spline::PrimitiveMetadata,
};
use super::{Asset, StaticProvider};
use crate::skate3_native::world_types::HostGrindAsset;
use skate3_core::physics::grind_contact::Primitive;

impl StaticProvider {
    pub(crate) fn from_host(host: &[HostGrindAsset]) -> Result<Self, String> {
        let mut result = Self {
            primitives: Vec::new(),
            metadata: Vec::new(),
            assets: Vec::new(),
            #[cfg(any())]
            rail_guids: Vec::new(),
            #[cfg(any())]
            source_for_primitive: Vec::new(),
            #[cfg(any())]
            source_rail_indices: Vec::new(),
        };
        for (_asset_index, asset) in host.iter().enumerate() {
            if asset.segments.is_empty() {
                return Err("Empty native host grind asset".into());
            }
            let mut bounds = Vec::new();
            let mut indices = Vec::new();
            for segment in &asset.segments {
                if segment.owner == 0
                    || segment
                        .coefficients
                        .iter()
                        .flatten()
                        .chain(&segment.minimum)
                        .chain(&segment.maximum)
                        .any(|v| !v.is_finite())
                    || (0..3).any(|i| segment.minimum[i] > segment.maximum[i])
                {
                    return Err("Invalid native host grind segment".into());
                }
                let [a, b, c, d] = segment.coefficients;
                let end: [f32; 4] = std::array::from_fn(|i| (a[i] + b[i]) + (c[i] + d[i]));
                if end.iter().any(|v| !v.is_finite()) {
                    return Err("Native host grind chord overflow".into());
                }
                let epsilon = f32::from_bits(0x3780_0000);
                let vertical = (end[0] - d[0]).abs() <= epsilon && (end[2] - d[2]).abs() <= epsilon;
                indices.push(result.primitives.len());
                result.primitives.push(Primitive {
                    start: d,
                    end,
                    owner: segment.owner,
                });
                result.metadata.push(PrimitiveMetadata {
                    spline_guids: segment.spline_guids,
                    segment_index: segment.segment_index,
                    flags: if vertical { 0x8000_0000 } else { 0 },
                });
                bounds.push(
                    Bounds {
                        min: segment.minimum,
                        max: segment.maximum,
                    }
                    .identity_transformed(),
                );
                #[cfg(any())]
                {
                    result.source_for_primitive.push(_asset_index);
                    result.source_rail_indices.push(segment.owner);
                }
            }
            let asset_bounds = bounds
                .iter()
                .copied()
                .reduce(Bounds::union)
                .expect("nonempty")
                .padded();
            let tree = Octree::new(asset_bounds, bounds)?;
            result.assets.push(Asset {
                bounds: asset_bounds,
                indices,
                tree,
                #[cfg(any())]
                source: None,
            });
        }
        Ok(result)
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    use crate::skate3_native::world_types::HostGrindSegment;
    fn asset() -> HostGrindAsset {
        HostGrindAsset {
            segments: vec![HostGrindSegment {
                coefficients: [
                    [-2., 0., 0., 0.],
                    [3., 0., 0., 0.],
                    [0.; 4],
                    [10., 2., 3., 1.],
                ],
                minimum: [10., 2., 3.],
                maximum: [11., 2., 3.],
                owner: 41,
                spline_guids: [5, 6],
                segment_index: 0,
            }],
        }
    }
    #[test]
    fn host_splines_keep_the_native_chord_identity_and_duplicate_source_order() {
        let provider = StaticProvider::from_host(&[asset(), asset()]).unwrap();
        assert_eq!(provider.query([9., 1., 2.], [12., 3., 4.]).unwrap(), [0, 1]);
        assert_eq!(provider.primitives()[0].start, [10., 2., 3., 1.]);
        assert_eq!(provider.primitives()[0].end, [11., 2., 3., 1.]);
        assert_eq!(provider.primitives()[0].owner, 41);
        assert_eq!(provider.metadata(0).unwrap().spline_guids, [5, 6]);
        assert!(
            provider.source(0).is_none(),
            "host data must not claim Skate 3 WMET provenance"
        );
        let mut bad = asset();
        bad.segments[0].coefficients[0][0] = f32::NAN;
        assert!(StaticProvider::from_host(&[asset(), bad]).is_err());
    }
}
