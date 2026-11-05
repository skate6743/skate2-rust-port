//! Complete player-owned portion of TU3 `PhysicalPlayerHiLOD::PostInput`
//! (`0x82DB5588`) and its flag-latch helper (`0x82DB5CF8`).
//!
//! Grind, trajectory, the scalar calculation at `0x82DB5E10`, and candidate
//! registration remain required services because they own separate systems.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostInputPlayerFields {
    /// Player+1264, captured from PhysOut when its +442 byte is set.
    pub jump_reference_1264: [u32; 4],
    /// Player+1296.
    pub flags_1296: u32,
    /// Player+1304.
    pub state_frames_1304: u32,
    /// Player+1308.
    pub jump_fix_frames_1308: u32,
    /// Player+1320, owned by `0x82DB5CF8`.
    pub latch_frames_1320: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PostInputProcessedFields {
    /// ProcessedPhysIn+848.
    pub jump_reference_848: [u32; 4],
    pub word_2464: u32,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub current_state_2508: u32,
    pub state_frames_2572: u32,
    pub jump_fix_frames_2576: u32,
    pub scalar_2740: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostInputPhysOutFields {
    /// PhysOut+32 -> +316.
    pub reset_state_frames_316: bool,
    /// PhysOut+8 -> +442.
    pub capture_jump_reference_442: bool,
    /// PhysOut+8 -> +128.
    pub jump_reference_128: [u32; 4],
    /// PhysOut+76, set after every successful PostInput pass.
    pub complete_76: bool,
}

/// Player+1840 data consumed by `0x82D740F8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidatePublicationFields {
    pub first_object_present_196: bool,
    pub first_pending_288: bool,
    pub second_object_present_500: bool,
    pub second_pending_592: bool,
    pub staged_word_12768: u32,
    pub staged_valid_12772: u32,
    pub staged_latched_12776: u32,
    pub staged_pending_12780: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateRegistration {
    /// `sub_82762AB0(ProcessedPhysIn+1888, component)`.
    First1888,
    /// `sub_82762AB0(ProcessedPhysIn+2176, component+304)`.
    Second2176,
}

/// Calls to independent systems, in their exact `0x82DB5588` order.
pub trait PostInputServices {
    fn update_grind_manager_82d8ab08(&mut self);
    fn update_trajectory_selector_82d68800(&mut self) -> u8;
    fn calculate_scalar_2740_82db5e10(&mut self) -> f32;
    fn register_candidate_82762ab0(&mut self, registration: CandidateRegistration);
}

pub struct PostInputContext<'a> {
    pub player: &'a mut PostInputPlayerFields,
    pub processed: &'a mut PostInputProcessedFields,
    pub phys_out: &'a mut PostInputPhysOutFields,
    pub candidates: &'a mut CandidatePublicationFields,
}

/// Runs the full TU3 PostInput phase for the fields owned by PhysicalPlayer.
pub fn run_post_input(context: PostInputContext<'_>, services: &mut impl PostInputServices) {
    let PostInputContext {
        player,
        processed,
        phys_out,
        candidates,
    } = context;

    services.update_grind_manager_82d8ab08();

    if phys_out.reset_state_frames_316 {
        player.state_frames_1304 = 0;
    }
    player.state_frames_1304 = player.state_frames_1304.wrapping_add(1);

    let reset_state_frames = processed.flags_2480 & 0x0000_2000 != 0
        || (processed.flags_2468 & 0x0040_0000 != 0 && player.flags_1296 & 0x4000_0000 != 0);
    if reset_state_frames {
        player.state_frames_1304 = 0;
        player.flags_1296 &= !0x4000_0000;
    } else {
        replace_bit(
            &mut player.flags_1296,
            30,
            u32::from(processed.flags_2468 & 0x0040_0000 == 0),
        );
        processed.flags_2468 &= !0x0040_0000;
    }
    processed.state_frames_2572 = player.state_frames_1304;

    if phys_out.capture_jump_reference_442 {
        player.jump_fix_frames_1308 = 1;
        player.jump_reference_1264 = phys_out.jump_reference_128;
    } else {
        player.jump_fix_frames_1308 = player.jump_fix_frames_1308.wrapping_add(1);
    }
    processed.jump_fix_frames_2576 = player.jump_fix_frames_1308;
    processed.jump_reference_848 = player.jump_reference_1264;

    replace_bit(
        &mut processed.flags_2468,
        10,
        u32::from(services.update_trajectory_selector_82d68800() & 1),
    );
    processed.scalar_2740 = services.calculate_scalar_2740_82db5e10();
    replace_bit(&mut processed.flags_2484, 25, (player.flags_1296 >> 24) & 1);

    if processed.flags_2484 & 0x0000_0800 != 0 {
        player.flags_1296 |= 0x0020_0000;
    } else if processed.current_state_2508 == 103 {
        replace_bit(&mut processed.flags_2484, 11, (player.flags_1296 >> 21) & 1);
    } else {
        player.flags_1296 &= !0x0020_0000;
    }

    update_flag_latches_82db5cf8(player, processed);
    publish_candidates_82d740f8(candidates, processed, services);
    phys_out.complete_76 = true;
}

/// Exact scalar bit transfers and latch lifetime from TU3 `0x82DB5CF8`.
fn update_flag_latches_82db5cf8(
    player: &mut PostInputPlayerFields,
    processed: &mut PostInputProcessedFields,
) {
    if processed.flags_2468 & 0x0020_0000 != 0 {
        player.latch_frames_1320 = 0;
        player.flags_1296 |= 0x0200_0000;
    }
    if processed.flags_2472 & 0x0000_0020 != 0 || processed.flags_2468 & 0x0040_0000 != 0 {
        player.latch_frames_1320 = 0;
        player.flags_1296 |= 0x0600_0000;
    }

    let flags_before_expiry = player.flags_1296;
    if flags_before_expiry & 0x0200_0000 != 0 {
        if processed.flags_2472 & 0x0000_0010 != 0
            || (flags_before_expiry & 0x0800_0000 != 0 && processed.flags_2472 & 0x0000_8000 == 0)
        {
            player.flags_1296 &= 0xF9FF_FFFF;
        }
        let frames = player.latch_frames_1320;
        if frames > 20 && player.flags_1296 & 0x0800_0000 == 0 {
            player.flags_1296 &= 0xF9FF_FFFF;
        }
        player.latch_frames_1320 = frames.wrapping_add(1);
    }

    replace_bit(&mut player.flags_1296, 28, (processed.flags_2472 >> 4) & 1);
    replace_bit(&mut player.flags_1296, 27, (processed.flags_2472 >> 15) & 1);
    replace_bit(&mut player.flags_1296, 17, (processed.flags_2480 >> 17) & 1);
    replace_bit(&mut processed.flags_2472, 3, (player.flags_1296 >> 26) & 1);
    replace_bit(&mut processed.flags_2472, 2, (player.flags_1296 >> 25) & 1);
}

/// Complete TU3 `0x82D740F8` publication/reset behavior.
fn publish_candidates_82d740f8(
    fields: &mut CandidatePublicationFields,
    processed: &mut PostInputProcessedFields,
    services: &mut impl PostInputServices,
) {
    let first = fields.first_pending_288 && fields.first_object_present_196;
    if first {
        services.register_candidate_82762ab0(CandidateRegistration::First1888);
    }
    replace_bit(&mut processed.flags_2480, 22, u32::from(first));
    fields.first_pending_288 = false;

    let second = fields.second_pending_592 && fields.second_object_present_500;
    if second {
        services.register_candidate_82762ab0(CandidateRegistration::Second2176);
    }
    replace_bit(&mut processed.flags_2480, 21, u32::from(second));
    fields.second_pending_592 = false;

    if fields.staged_pending_12780 {
        fields.staged_latched_12776 = 0;
        if fields.staged_valid_12772 != 0 {
            fields.staged_latched_12776 = 1;
            processed.flags_2480 |= 0x0010_0000;
            processed.word_2464 = fields.staged_word_12768;
        } else {
            processed.flags_2480 &= !0x0010_0000;
        }
    }
    fields.staged_pending_12780 = false;
}

fn replace_bit(word: &mut u32, bit: u32, value: u32) {
    *word = (*word & !(1 << bit)) | ((value & 1) << bit);
}

///82762AB0 copies the defined fields, preserving destination padding and the
///low five bits of byte200. Words use native big-endian bit positions.
pub fn copy_grab_record_82762ab0(destination: &mut [u32; 72], source: &[u32; 72]) {
    destination[..50].copy_from_slice(&source[..50]);
    destination[50] = (destination[50] & 0x1fff_ffff) | (source[50] & 0xe000_0000);
    destination[51..55].copy_from_slice(&source[51..55]);
    destination[56..66].copy_from_slice(&source[56..66]);
    destination[68] = source[68];
}

#[cfg(test)]
#[path = "tests/post_input.rs"]
mod tests;
