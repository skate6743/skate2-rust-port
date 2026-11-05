//! Original82DB18A0 initialization of the represented input owner. Native
//! opaque allocation flags are replaced by zero-initialized host storage.
use skate3_core::player::input_phase::{PlayerInputState, StateVariantFields};
use skate_data::collections::Collections;
pub(super) fn player(data: &Collections) -> Result<PlayerInputState, String> {
    let mut result = PlayerInputState {
        //82DB1A9C inserts E0080000; active human actor adds bit18 at307C.
        flags_1296: 0xe00c_0000,
        ground_history_frames_1304: 100,
        ..PlayerInputState::default()
    };
    //82DB19C0: the external packet's ninth vector is the -1 sentinel.
    result.external_physics_cache_1008.vectors[8] = [(-1.0f32).to_bits(); 4];
    for (index, key) in ["easy", "normal", "hardcore", "motorized", "test"]
        .into_iter()
        .enumerate()
    {
        //Original full64 keys at82DB1B08..1B78; stock XML layout+60.
        result.state_variants_1408[index] = StateVariantFields {
            surface_override_enabled_60: u8::from(data.boolean(
                "physics_mode",
                key,
                "Hash_5548109D7B0CB70C",
            )?),
        };
    }
    Ok(result)
}
