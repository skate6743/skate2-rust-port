//! Ground contact identity/update leaf (`82D38430`).

use super::data::GroundContactHistory;

pub type Vector4 = [f32; 4];

/// Values copied into the native contact publication record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundContactStateInput {
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub velocity_400: Vector4,
    pub axis_464: Vector4,
    pub contact_position_560: Vector4,
    pub trajectory_position_592: Vector4,
    pub reckoning_vector_1200: Vector4,
    /// Selected global collection +396 -> layout+4 -> +560.
    pub differing_contact_frame_limit: i32,
}

/// Required owner/collision operations around state-owned counter logic.
pub trait GroundContactServices {
    type Error;
    type Handle: Eq;

    /// `82D62F20`, called first on the inactive branch.
    fn stop_contact_tracking_82d62f20(&mut self) -> Result<(), Self::Error>;
    /// Exact owner+3024 reset at `82D38468..82D3848C`.
    fn clear_inactive_contact_output_3024(&mut self) -> Result<(), Self::Error>;
    /// Owner+1920, read only when the retained handle is empty.
    fn initial_contact_handle_1920(&mut self) -> Result<Option<Self::Handle>, Self::Error>;
    /// `82D63C30` over the first three vectors of the publication record.
    fn identify_contact_82d63c30(
        &mut self,
        frame: &GroundContactStateInput,
    ) -> Result<Option<Self::Handle>, Self::Error>;
    /// `82D61268`, with all five vectors and the retained handle.
    fn publish_contact_82d61268(
        &mut self,
        frame: &GroundContactStateInput,
        retained: Option<&Self::Handle>,
    ) -> Result<(), Self::Error>;
}

/// Runs the full TU3 state/call order. Counter updates use wrapping integer
/// arithmetic because the native body uses ordinary `addi` instructions.
pub fn update<S: GroundContactServices>(
    history: &mut GroundContactHistory<S::Handle>,
    input: GroundContactStateInput,
    services: &mut S,
) -> Result<(), S::Error> {
    if input.flags_2476 & 0x0040_0000 == 0 {
        services.stop_contact_tracking_82d62f20()?;
        services.clear_inactive_contact_output_3024()?;
        return Ok(());
    }

    if input.flags_2480 & 0x2000_0000 != 0 || input.flags_2480 & 0x1000_0000 == 0 {
        history.tracked_contact_2760 = None;
        history.differing_contact_frames_2756 = 0;
    }

    if input.flags_2480 & 0x1000_0000 != 0 {
        if input.axis_464[1] < f32::from_bits(0x3f73_3333) {
            if history.tracked_contact_2760.is_none() {
                history.tracked_contact_2760 = services.initial_contact_handle_1920()?;
            }
            let current = services.identify_contact_82d63c30(&input)?;
            history.differing_contact_frames_2756 = if current == history.tracked_contact_2760 {
                0
            } else {
                history.differing_contact_frames_2756.wrapping_add(1)
            };
            if history.differing_contact_frames_2756 > input.differing_contact_frame_limit {
                history.tracked_contact_2760 = None;
            }
        } else {
            // Native label clears only the retained handle here.
            history.tracked_contact_2760 = None;
        }
    }

    services.publish_contact_82d61268(&input, history.tracked_contact_2760.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Services {
        events: Vec<&'static str>,
        initial: Option<u32>,
        identified: Option<u32>,
        published: Option<Option<u32>>,
    }

    impl GroundContactServices for Services {
        type Error = ();
        type Handle = u32;

        fn stop_contact_tracking_82d62f20(&mut self) -> Result<(), Self::Error> {
            self.events.push("stop");
            Ok(())
        }

        fn clear_inactive_contact_output_3024(&mut self) -> Result<(), Self::Error> {
            self.events.push("clear");
            Ok(())
        }

        fn initial_contact_handle_1920(&mut self) -> Result<Option<Self::Handle>, Self::Error> {
            self.events.push("initial");
            Ok(self.initial)
        }

        fn identify_contact_82d63c30(
            &mut self,
            _frame: &GroundContactStateInput,
        ) -> Result<Option<Self::Handle>, Self::Error> {
            self.events.push("identify");
            Ok(self.identified)
        }

        fn publish_contact_82d61268(
            &mut self,
            _frame: &GroundContactStateInput,
            retained: Option<&Self::Handle>,
        ) -> Result<(), Self::Error> {
            self.events.push("publish");
            self.published = Some(retained.copied());
            Ok(())
        }
    }

    fn input() -> GroundContactStateInput {
        GroundContactStateInput {
            flags_2476: 0x0040_0000,
            flags_2480: 0x1000_0000,
            velocity_400: [1.0; 4],
            axis_464: [0.0, 0.5, 0.0, 0.0],
            contact_position_560: [2.0; 4],
            trajectory_position_592: [3.0; 4],
            reckoning_vector_1200: [4.0; 4],
            differing_contact_frame_limit: 5,
        }
    }

    #[test]
    fn inactive_branch_preserves_history_and_runs_both_owner_calls() {
        let mut history = GroundContactHistory {
            differing_contact_frames_2756: 7,
            tracked_contact_2760: Some(9),
        };
        let mut frame = input();
        frame.flags_2476 = 0;
        let mut services = Services::default();

        update(&mut history, frame, &mut services).unwrap();

        assert_eq!(services.events, ["stop", "clear"]);
        assert_eq!(history.differing_contact_frames_2756, 7);
        assert_eq!(history.tracked_contact_2760, Some(9));
    }

    #[test]
    fn candidate_latches_initial_handle_then_counts_identity_changes() {
        let mut history = GroundContactHistory::empty();
        let mut services = Services {
            initial: Some(10),
            identified: Some(11),
            ..Default::default()
        };

        update(&mut history, input(), &mut services).unwrap();

        assert_eq!(services.events, ["initial", "identify", "publish"]);
        assert_eq!(history.differing_contact_frames_2756, 1);
        assert_eq!(history.tracked_contact_2760, Some(10));
        assert_eq!(services.published, Some(Some(10)));
    }

    #[test]
    fn limit_and_axis_rejection_clear_only_the_native_fields() {
        let mut history = GroundContactHistory {
            differing_contact_frames_2756: 5,
            tracked_contact_2760: Some(10),
        };
        let mut frame = input();
        frame.differing_contact_frame_limit = 5;
        let mut services = Services {
            identified: Some(11),
            ..Default::default()
        };
        update(&mut history, frame, &mut services).unwrap();
        assert_eq!(history.differing_contact_frames_2756, 6);
        assert_eq!(history.tracked_contact_2760, None);
        assert_eq!(services.published, Some(None));

        history.differing_contact_frames_2756 = 13;
        history.tracked_contact_2760 = Some(12);
        frame.axis_464[1] = 0.95;
        services.events.clear();
        services.published = None;
        update(&mut history, frame, &mut services).unwrap();
        assert_eq!(history.differing_contact_frames_2756, 13);
        assert_eq!(history.tracked_contact_2760, None);
        assert_eq!(services.events, ["publish"]);
    }
}
