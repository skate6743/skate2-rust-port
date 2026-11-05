//! Ground ordinary tail82D749D0 cancels pending trajectory work and clears
//!published validity. Rust owns the request lifetime; no guest pool is copied.
///The caller's actual pending request type can cancel work on Drop. Prediction
///data may remain cached, as native Reset clears validity rather than erasing
///all output vectors. Lower six flags retain their existing values.
pub(crate) struct GroundTrajectoryState<T> {
    pub pending_request: Option<T>,
    pub primary_valid_288: bool,
    pub secondary_valid_592: bool,
    pub result_valid_9840: bool,
    pub flags_12836: u8,
}
impl<T> GroundTrajectoryState<T> {
    ///Publish the native cancellation after releasing the current request.
    pub fn cancel(&mut self) {
        self.pending_request.take();
        self.result_valid_9840 = false;
        self.primary_valid_288 = false;
        self.secondary_valid_592 = false;
        self.flags_12836 &= 0x3f;
    }
}
