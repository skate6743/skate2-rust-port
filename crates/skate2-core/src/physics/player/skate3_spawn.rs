//! PC offboard reset placement. S2 AdjustSpawnPosition825F1B8C adds .15m,
//! then the shared S3 board reset82DB8A10 adds .1m. A standing biped must
//! start on its actual nearby support, not fall from that combined clearance.
use skate3_core::{math::Vector3, physics::{board_world::BoardWorld,
    skeleton_animation_record::AnimationPartTransform}};

pub(super) fn target(world: &BoardWorld, mut requested: AnimationPartTransform,
                     on_board: bool) -> Result<AnimationPartTransform, String> {
    if on_board { return Ok(requested); }
    let [x,y,z,_] = requested[3];
    // A short query cannot pull an intentionally airborne spawn onto a lower
    // storey. Missing/steep support retains the ordinary native placement.
    if let Some(hit) = world.query_thin_line(
        Vector3::new(x,y+0.25,z), Vector3::new(x,y-0.6,z)).map_err(str::to_owned)? {
        if hit.geometry.normal.y > 0.5 {
            let floor = hit.geometry.position.y;
            // Compensate the board-only clearance before the ordinary reset,
            // so all physical bodies and the biped enter use the same target.
            requested[3][1] = floor - 0.1;
            crate::diagnostics::test_log::event("skate3_offboard_spawn_support",
                format_args!("requested_y={y} support_y={floor} reset_y={}",requested[3][1]));
        }
    }
    Ok(requested)
}
