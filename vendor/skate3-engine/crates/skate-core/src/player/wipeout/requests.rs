//! Original request arrays20/56,count200,mode208 and timers192/196/204.
#[derive(Clone, Debug, PartialEq)]
pub struct Requests {
    pub reasons: [bool; 34],
    pub values: [f32; 34],
    pub count: u32,
    pub cooldown: f32,
    pub contact_frames: i32,
    pub balance: f32,
    pub mode: u32,
}
impl Requests {
    ///Player ctor82DB20B8..D8 and first ClearRequests initialize this storage.
    pub fn new() -> Self {
        Self {
            reasons: [false; 34],
            values: [0.0; 34],
            count: 0,
            cooldown: 0.0,
            contact_frames: 0,
            balance: 0.0,
            mode: 0,
        }
    }
    ///Player initialization82DB3024 supplies the short initial grace period.
    pub fn initialize_player(&mut self) {
        self.cooldown = f32::from_bits(0x3d23_d70a);
    }
    ///Teleport82DB8DB4 overwrites only the cooldown; ResetSystems is separate.
    pub fn teleport(&mut self) {
        self.cooldown = f32::from_bits(0x3ecc_cccd);
    }
    ///Ground Enter82D378E0..F4 and CheckForGroundWipeout share this state.
    pub fn enter_ground(&mut self) {
        if self.mode != 1 {
            self.balance = 0.0;
            self.mode = 1;
        }
    }
    ///A repeated request increments the original counter even if its bit was set.
    pub fn request(&mut self, index: usize, value: f32) {
        self.reasons[index] = true;
        self.values[index] = value;
        self.count = self.count.wrapping_add(1);
    }
    ///SystemLogic82DB6000 after state selection: preserve cooldown and histories.
    pub fn clear_after_selection(&mut self) {
        self.reasons.fill(false);
        self.values.fill(0.0);
        self.count = 0;
    }
    ///ResetSystems82DB9354..9360 clears mode and count only.
    pub fn reset_systems(&mut self) {
        self.mode = 0;
        self.count = 0;
    }
    ///82DB9188: count reflects individual triggers, not unique reasons.
    pub fn requests_runout(&self, frame: &super::RequestInput) -> bool {
        (self.count == 1
            && (self.reasons[2] || self.reasons[24] || self.reasons[10])
            && frame.flags_2468 & (1 << 5) == 0
            && frame.flags_2476 & (1 << 26) == 0
            && frame.animation_up_y.abs() > f32::from_bits(0x3f4f_5c29))
            || (frame.flags_2484 & (1 << 12) != 0 && frame.category == 400)
    }
    ///82DB9100 first excludes runout, then combines reason OR with2480bit3.
    pub fn requests_wipeout(&self, frame: &super::RequestInput) -> bool {
        !self.requests_runout(frame)
            && (self.reasons.iter().any(|&v| v) || frame.flags_2480 & (1 << 3) != 0)
    }
}
