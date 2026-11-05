//! Foot contact history and correction82BEFC80, helpers82BF04C8/82BF0558.
use super::{
    status::{LimbStatus, Mode},
    transforms::LimbFrames,
};
use crate::physics::skeleton_animation_record::{
    AnimationPartTransform as Transform, transform_point,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct FootContact {
    /// SkeletonIK3104:0 pending,1 previous query hit,2 previous query missed.
    pub query_state: u32,
    pub position: [f32; 4],
    pub desired_offset: f32,
    pub offset: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ContactState {
    pub feet: [FootContact; 2],
    ///3184 persists until Reset;3185 is cleared at each UpdateContact entry.
    pub support_failed: bool,
    pub support_failed_this_update: bool,
}

pub struct ContactInput<'a> {
    pub flags_2468: u32,
    pub contact_bone: usize,
    pub foot_bones: [usize; 2],
    /// SkeletonIK DataIn416, SkeletonCollision1152: the most vertical toe
    /// (part15/19) contact normal of the latest collision update.
    pub toe_contact_normal: [f32; 4],
    pub inverse_board: &'a Transform,
    pub board: &'a Transform,
    /// Actual cached line tests from Processed960/1008. Current observations
    /// replace the retained query only AFTER the previous query is consumed.
    pub current_contacts: [Option<[f32; 4]>; 2],
}

impl ContactState {
    pub fn update(
        &mut self,
        statuses: &[LimbStatus; 4],
        frames: &mut [LimbFrames; 4],
        input: ContactInput<'_>,
    ) {
        self.support_failed_this_update = false;
        let requires_support = input.flags_2468 & 0x4200_0000 != 0;
        for foot in 0..2 {
            let contact = &mut self.feet[foot];
            let frame = &mut frames[foot];
            let mut failed = false;
            if statuses[foot].mode == Mode::OnDeck && input.contact_bone == input.foot_bones[foot] {
                match contact.query_state {
                    1 => {
                        let local = transform_point(input.inverse_board, contact.position);
                        let height = local[1] + f32::from_bits(0x3DA8_F5C3);
                        if height <= f32::from_bits(0x3E99_999A) {
                            contact.desired_offset = if input.flags_2468 & 0x0180_0000 == 0 {
                                limit_offset(height)
                            } else {
                                0.0
                            };
                            approach(contact);
                            let distance = distance_to_contact(
                                contact,
                                frame,
                                input.board,
                                input.inverse_board,
                            );
                            //82BEFFB4..FFCC: fix modest penetration immediately;
                            //larger separations retain the bounded transition.
                            if distance < 0.0 && distance > f32::from_bits(0xBE99_999A) {
                                contact.offset -= distance;
                            }
                            add_offset(frame, input.board, contact.offset);
                            failed = requires_support
                                && distance > f32::from_bits(0x3CF5_C28F)
                                && contact.offset < f32::from_bits(0xBC23_D70A);
                        } else {
                            contact.desired_offset = limit_offset(contact.desired_offset);
                            approach(contact);
                            add_offset(frame, input.board, contact.offset);
                            failed = requires_support
                                || input.toe_contact_normal[1] > f32::from_bits(0x3F33_3333);
                        }
                    }
                    0 | 2 => {
                        contact.desired_offset = limit_offset(contact.desired_offset);
                        approach(contact);
                        add_offset(frame, input.board, contact.offset);
                        failed = contact.query_state == 2 && requires_support;
                    }
                    _ => {}
                }
                if let Some(position) = input.current_contacts[foot] {
                    contact.position = position;
                    contact.query_state = 1;
                } else {
                    contact.query_state = 2;
                }
            } else {
                contact.query_state = 0;
                contact.desired_offset = 0.0;
                approach(contact);
                add_offset(frame, input.board, contact.offset);
            }
            if failed {
                self.support_failed = true;
                self.support_failed_this_update = true;
            }
        }
    }
}

///82BF0558: compare both positions in inverse-board space, retaining the
///separate point transforms and the .025 foot-surface distance adjustment.
fn distance_to_contact(
    contact: &FootContact,
    frame: &LimbFrames,
    board: &Transform,
    inverse: &Transform,
) -> f32 {
    let local_contact = transform_point(inverse, contact.position);
    let delta = contact_offset(board, contact.offset);
    let foot = core::array::from_fn(|i| frame.world[3][i] + delta[i]);
    let local_foot = transform_point(inverse, foot);
    (local_foot[1] - f32::from_bits(0x3CCC_CCCD)) - local_contact[1]
}

fn add_offset(frame: &mut LimbFrames, board: &Transform, offset: f32) {
    let delta = contact_offset(board, offset);
    for (i, value) in delta.into_iter().enumerate() {
        frame.world[3][i] += value;
        frame.parent_world[3][i] += value;
    }
}
fn contact_offset(board: &Transform, offset: f32) -> [f32; 4] {
    core::array::from_fn(|i| {
        let x = board[0][i] * 0.0;
        let y = board[1][i].mul_add(offset, x);
        board[2][i].mul_add(0.0, y)
    })
}
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
fn limit_offset(value: f32) -> f32 {
    let min = f32::from_bits(0xBDCC_CCCD);
    let max = f32::from_bits(0x3DCC_CCCD);
    let lower = select(min - value, min, value);
    select(max - lower, lower, max)
}
fn approach(contact: &mut FootContact) {
    let min = f32::from_bits(0xBC75_C28F);
    let max = f32::from_bits(0x3C75_C28F);
    let delta = contact.desired_offset - contact.offset;
    let lower = select(min - delta, min, delta);
    contact.offset = select(max - lower, lower, max) + contact.offset;
}
