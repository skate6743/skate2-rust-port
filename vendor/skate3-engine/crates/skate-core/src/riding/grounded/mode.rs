//! Mode binding producer, TU3 ProcessInput 82DB4420..447C.
//! Numeric mode identities are preserved without assigning guessed names.

/// The two fields are deliberately separate: an unsupported request is still
/// published, while the previously selected attribute binding remains active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessedMode<T> {
    /// ProcessedPhysIn+2528, copied from skeleton+10928 on every update.
    pub requested_mode: u32,
    /// ProcessedPhysIn+2548. Supply actual resource bindings, not host addresses.
    pub selected_attributes: T,
}

impl<T: Copy> ProcessedMode<T> {
    /// `attributes` correspond in order to player+1408/1424/1440/1456/1472.
    /// Their constructor/asset loading is a separate dependency. There is no
    /// guessed initial binding and no default-mode fallback for invalid input.
    pub fn update(&mut self, requested_mode: u32, attributes: &[T; 5]) {
        if requested_mode <= 4 {
            self.selected_attributes = attributes[requested_mode as usize];
        }
        self.requested_mode = requested_mode;
    }
}
