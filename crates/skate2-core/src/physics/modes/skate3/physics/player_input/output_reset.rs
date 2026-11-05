//! Represented PhysOut reset values. Original82DE53F0 copies the templates
//! built by82DE4940; scalar zero does not replace any active producer.
use skate3_core::player::input_phase::*;
const UP: RawVector = [0, 0x3f800000, 0, 0];
pub(crate) fn reset_outputs(out: &mut PhysicalPlayerInput) {
    //These are host-owned observations, outside the native PhysOut templates.
    let surface = out.surface_default_mode;
    let world_grab = out.component_1832_word_1876;
    let anim_to_world = out.skeleton.anim_to_world_11920;
    //82DE2EF8 clears these three represented Air324 bits; preserve the rest.
    let handplant_flags = out.air.handplant_flags_324 & !0xb000_0000;
    *out = PhysicalPlayerInput {
        air: AirOutputFields {
            handplant_flags_324: handplant_flags,
            //82DE2F70..78 loads original world-up into144.82DE304C
            //splats-1 into the final four-word trajectory descriptor lane.
            landing_normal_144: UP,
            selected_trajectory_240: [[0; 4], [0; 4], [0; 4], [0xbf80_0000; 4]],
            ..Default::default()
        },
        reckoning: SystemReckoningFields {
            vector_96: UP,
            ..Default::default()
        }, //82DE3CC8
        ground: GroundOutputFields {
            vector_64: UP,
            vector_80: UP,
            vector_96: UP,
            scalar_276: -1.,
            ..Default::default()
        }, //82DE3728
        grinds: GrindOutputFields {
            words_136_140: [u32::MAX, 0],
            ..Default::default()
        }, //82DE3518
        skeleton: SkeletonOutputFields {
            anim_to_world_11920: anim_to_world,
            ..Default::default()
        },
        surface_default_mode: surface,
        component_1832_word_1876: world_grab,
        ..Default::default()
    };
}
