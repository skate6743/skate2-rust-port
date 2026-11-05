//! Native wheel surface vote82C08818 on live wheel lines and contact reports.
use super::super::riding_outputs::RidingOutputs;
use skate3_core::physics::{board_runtime::BoardRuntime, contact_feedback::choose_surface};
/// Wheel surface IDs come from line queries. The report loop82C08198 updates
/// only truck/deck material slots, so wheel contacts contribute vote weights
/// without replacing the query-owned material ID.
pub(crate) fn active_surface(riding: &RidingOutputs, board: &BoardRuntime) -> u32 {
    let contacts = std::array::from_fn(|i| riding.ground.parts[i].in_contact);
    let forced = board
        .contact_reports()
        .iter()
        .any(|r| r.other_surface & 0x0f80 == 0x0600);
    choose_surface(riding.wheel_lines.physics_surfaces, contacts, forced)
}

/// SurfacePhysics constructor82D85D08 binds these exact five collection keys
/// at24/40/56/72/88. Lookup8 hashes independently match the native immediates.
/// The selector's mode normalization lives in the processed-input producer.
pub(crate) fn surface_key(mode: u32) -> Result<&'static str, String> {
    match mode {
        1 => Ok("smooth"),
        2 => Ok("rough"),
        3 => Ok("slow"),
        4 => Ok("slippery"),
        5 => Ok("veryslow"),
        _ => Err(format!(
            "Processed surface mode {mode} was not normalized by SurfacePhysics"
        )),
    }
}
