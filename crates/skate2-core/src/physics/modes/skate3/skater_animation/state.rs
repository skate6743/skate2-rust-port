//! State shared by graph construction, stance events and physics publication.
use skate3_core::animation::{
    output::{attributes::AnimationAttribute, physics_packet::SkaterPublicationState},
    skeleton_input::name::encode,
};

pub(super) struct AnimationState {
    pub flags: u32,
    pub publication: SkaterPublicationState,
    /// Base animator14652: ctor82D17CDC sets zero; virtual40/82531200 writes it.
    pub phase: f32,
}

impl AnimationState {
    pub fn new(local_player: bool) -> Self {
        //82B975F4 selects bit27;82B97674 selects bit17 and clears the
        //orientation,mirror,fakie,weight and request bits used by this owner.
        //Unrepresented allocator/padding bits are not gameplay state here.
        Self {
            flags: (u32::from(local_player) << 27) | 0x0002_0000,
            phase: 0.0,
            publication: SkaterPublicationState {
                orientation_bit31: false,
                mirrored: false,
                riding_fakie: false,
                weight_on_nose: false,
                //82B975B8..82B97674: GOOFY natural stance, NATURAL relative.
                natural_stance: 1,
                relative_stance: 0,
                request_bit16: false,
                request_bit15: false,
                air_dismount_revert_requested: false,
                air_dismount_revert_frames: 0,
                signal: None,
            },
        }
    }

    pub fn mirrored(&self) -> bool {
        self.flags & 0x4000_0000 != 0
    }
    pub fn fakie(&self) -> bool {
        self.flags & 0x2000_0000 != 0
    }
    pub fn switch(&self) -> bool {
        self.publication.relative_stance == 1
    }

    ///82B98980 calls82D19010's first-match query. No scalar/status test:
    ///presence in the collected tree attribute list toggles the state once.
    pub fn apply_stance_events(&mut self, attributes: &[AnimationAttribute]) {
        let present = |name: &[u8]| attributes.iter().any(|a| a.name == encode(name));
        if present(b"animboardbackward") {
            self.flags ^= 0x8000_0000;
        }
        if present(b"mirrored") {
            self.flags ^= 0x4000_0000;
        }
        if present(b"switch") {
            self.publication.relative_stance = i32::from(self.publication.relative_stance == 0);
        }
    }

    pub fn prepare_publication(&mut self) {
        let p = &mut self.publication;
        p.orientation_bit31 = self.flags & 0x8000_0000 != 0;
        p.mirrored = self.flags & 0x4000_0000 != 0;
        p.riding_fakie = self.flags & 0x2000_0000 != 0;
        p.weight_on_nose = self.flags & 0x1000_0000 != 0;
        p.request_bit16 = self.flags & 0x0001_0000 != 0;
        p.request_bit15 = self.flags & 0x0000_8000 != 0;
        p.air_dismount_revert_requested = self.flags & 0x0010_0000 != 0;
    }

    pub fn finish_publication(&mut self) {
        self.flags &= !(0x0001_0000 | 0x0000_8000 | 0x0010_0000);
    }

    ///SkaterAnim virtual172/82B98078, selected by local-player bit27.
    pub fn cull_threshold(&self) -> f32 {
        f32::from_bits(if self.flags & 0x0800_0000 != 0 {
            0x3c23_d70a
        } else {
            0x3dcc_cccd
        })
    }
}
