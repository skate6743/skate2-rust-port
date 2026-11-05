//! Authored seven-part matrices passed to Part::SetTransform by TU3 82C0ADF0.
//! This function is called by construction/reset before world-pose publication.
use super::RetailAffineTransform;
use crate::math::{Basis3, Vector3};

/// Resolved stock attributes read by InitializeTransforms. Attribute lookup
/// failure selects the engine's runtime fallback scalar at830D0850; callers
/// resolving absent attributes must supply that actual initialized value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthoredTransformInputs {
    /// physicsdeck.DeckMidLength; full attribute key C94D65091DCB828B.
    pub deck_mid_length: f32,
    /// physicswheels.WheelXDist; full attribute key 5A84C435DC340AF8.
    pub wheel_x_distance: f32,
    /// physicstrucks layout+4, +8, +16.
    pub truck_z_position_front: f32,
    pub truck_z_position_back: f32,
    pub truck_y_position: f32,
}

impl AuthoredTransformInputs {
    /// Actual stock XML scalar words, not constructor output snapshots.
    pub const STOCK: Self = Self {
        deck_mid_length: f32::from_bits(0x3f17_0a3d),
        wheel_x_distance: f32::from_bits(0x3dc2_8f5c),
        truck_z_position_front: f32::from_bits(0xbd54_fdf4),
        truck_z_position_back: f32::from_bits(0xbd54_fdf4),
        truck_y_position: f32::from_bits(0xbd67_6c8b),
    };
}

/// Complete authored matrix calculation, in body order wheels0..3, trucks4..5,
/// deck6. Native publication order is deck6 first, then bodies0..5. Front truck
/// is identity; back truck uses the actual SinCos(pi/2) result about Y. The
/// tiny polynomial cosine residual is retained, never replaced by exact zero.
pub fn authored_body_transforms(input: AuthoredTransformInputs) -> [RetailAffineTransform; 7] {
    // fmadds82C0AEE4 and fnmsubs82C0AF60: one rounded multiply/add each.
    let front_z = input
        .deck_mid_length
        .mul_add(0.5, input.truck_z_position_front);
    let back_z = -input
        .deck_mid_length
        .mul_add(0.5, input.truck_z_position_back);
    let y = input.truck_y_position;
    let x = input.wheel_x_distance;
    let mut transforms = [RetailAffineTransform::IDENTITY; 7];
    for (part, translation) in [
        Vector3::new(-x, y, front_z),
        Vector3::new(x, y, front_z),
        Vector3::new(-x, y, back_z),
        Vector3::new(x, y, back_z),
        Vector3::new(0.0, y, front_z),
        Vector3::new(0.0, y, back_z),
    ]
    .into_iter()
    .enumerate()
    {
        transforms[part].translation = translation;
    }
    // Literal pi/2 word at820C69C8. Shared polynomial82473A08 was re-audited
    // against fresh disassembly and constants822F97C0..822F985F for this path.
    let (sin, cos) = crate::trigonometry::sin_cos(f32::from_bits(0x3fc9_0fdb));
    transforms[5].basis = Basis3 {
        columns: [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]],
    };
    transforms
}

/// Native affine rows, ready for reset_skateboard's `authored` argument. All
/// fourth lanes are zero, including the affine translation:82C0ADF0 explicitly
/// clears them. This does not perform the downstream COM/quaternion conversion.
pub fn authored_body_pose_records(input: AuthoredTransformInputs) -> [[u32; 16]; 7] {
    authored_body_transforms(input).map(|transform| {
        let mut words = [0; 16];
        for (axis, column) in transform.basis.columns.into_iter().enumerate() {
            for (component, value) in column.into_iter().enumerate() {
                words[axis * 4 + component] = value.to_bits();
            }
        }
        words[12] = transform.translation.x.to_bits();
        words[13] = transform.translation.y.to_bits();
        words[14] = transform.translation.z.to_bits();
        words
    })
}
