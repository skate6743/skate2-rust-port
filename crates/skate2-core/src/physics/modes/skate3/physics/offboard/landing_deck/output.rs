//! Complete82D79420 writes into caller-owned physical output fields.
use super::Owner;
use skate3_core::player::input_phase::OffBoardOutputFields;

///Borrow actual shared output storage; this type retains no parallel state.
///The parent must supply native skeleton+48/+3482 destinations, not temporaries.
pub(crate) struct PublishTargets<'a> {
    pub off_board: &'a mut OffBoardOutputFields,
    pub skeleton_position_48: &'a mut [u32; 4],
    pub skeleton_flag_3482: &'a mut u8,
}
impl Owner {
    pub(crate) fn publish(&self, output: PublishTargets<'_>) {
        let values = self.manager.fill();
        output.off_board.flag_316 = u8::from(values.can_land_316);
        output.off_board.hippy_hurdling_317 = u8::from(values.hippy_hurdling_317);
        if let Some(position) = values.moving_contact {
            *output.skeleton_flag_3482 = 1;
            *output.skeleton_position_48 = position.map(f32::to_bits);
        }
        //Native261 false leaves48 AND3482 untouched.
    }
}
