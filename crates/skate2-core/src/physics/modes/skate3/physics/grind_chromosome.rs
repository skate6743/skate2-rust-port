//! ZIP chromosome port corrected from S3 82DEE918/82DEF518/82DEE508.
//! S2 82E2E9E0/82E2EBE8/82E2F9D0 confirms history/publication structure;
//! only S3 has four-way orientation for5050/5O and the sixth darkslide family.
use super::grind::Family;
#[path = "grind_names.rs"]
pub(crate) mod names;
#[path = "grind_chromosome/pose.rs"]
mod pose;
pub(crate) use pose::{ApproachPose, Input};
use pose::{add, dot, signed, sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Components(pub [u32; 6]);
impl Components {
    pub fn name(self) -> names::Name {
        // Constructed only from booleans, orientation0..3 and the Family enum.
        names::lookup(self.0).expect("validated grind chromosome dimensions")
    }
}

///82DEE508's actual PhysOut publication, after update and only while grinding.
/// FastString encoding uses CURRENT's existing original-name encoder.
pub(crate) fn publish(
    p: Publication,
    out: &mut skate3_core::player::input_phase::GrindOutputFields,
) {
    use skate3_core::animation::skeleton_input::name::encode;
    out.volatile_chromosome_244 = p.volatile.0;
    if let Some(animation) = p.animation {
        let name = animation.name();
        out.animation_chromosome_268 = animation.0;
        out.animation_id_144 = name.skating_id as u32;
        out.animation_name_156 = Some(encode(name.attribute.as_bytes()));
    }
    if let Some(scoring) = p.scoring {
        let name = scoring.name();
        out.scoring_chromosome_292 = scoring.0;
        out.scoring_id_148 = name.skating_id as u32;
        out.scorable_id_152 = name.scorable_id as u32;
        out.scoring_name_176 = Some(encode(name.attribute.as_bytes()));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Publication {
    pub volatile: Components,
    pub animation: Option<Components>,
    pub scoring: Option<Components>,
}

pub(crate) struct Chromosome {
    history: std::collections::VecDeque<ApproachPose>,
    saved: ApproachPose,
    saved_fakie_initialized: bool,
    approach: u32,
    previous_category: u32,
    previous_kind: Option<Family>,
    away_frames: i32,
    reversed: bool,
    orientation: Option<u32>,
    pending: Option<Components>,
    pending_frames: i32,
    animation: Option<Components>,
    scoring: Option<Components>,
}

impl Chromosome {
    ///82DEE358 leaves byte144 unwritten. The false storage below is opaque
    ///until the host observes the first completed animation packet.
    pub fn uninitialized() -> Self {
        let mut state = Self::new(false);
        state.saved_fakie_initialized = false;
        state
    }

    ///Explicit host initialization of a native unwritten byte, not a claim
    ///about retail allocator contents. Seed once from completed Anim10372;
    ///do not replace constructor axes/position or any established history.
    pub fn initialize_host_fakie(&mut self, observed_fakie: bool) {
        if !self.saved_fakie_initialized {
            self.saved.fakie = observed_fakie;
            self.saved_fakie_initialized = true;
        }
    }

    /// S3 ctor82DEE358 explicitly initializes the saved axes/position but does
    /// not write saved fakie byte144. The supplied value is an explicit host
    /// initialization observation, not a recovered native constructor default.
    /// A real Air439 snapshot or30 Ground samples replaces this reference.
    pub fn new(constructor_fakie_144: bool) -> Self {
        Self {
            history: std::collections::VecDeque::with_capacity(30),
            saved: ApproachPose {
                right: [1.0, 0.0, 0.0, 0.0],
                position: [0.0; 4],
                feet: [[1.0, 0.0, 0.0, 0.0]; 2],
                fakie: constructor_fakie_144,
            },
            saved_fakie_initialized: true,
            approach: 0,
            previous_category: 0,
            previous_kind: None,
            away_frames: 31,
            reversed: false,
            orientation: None, // Native sentinel4, distinct from forward0.
            pending: None,
            pending_frames: 0,
            animation: None,
            scoring: None,
        }
    }

    fn reference(&self) -> ApproachPose {
        //82DEED28: fewer than30 samples uses saved, NOT newest ground pose.
        if self.history.len() == 30 {
            self.history[0]
        } else {
            self.saved
        }
    }

    /// Invoke after physical selection/FillOut with current PhysOut inputs.
    /// Category400 and grinding316 are distinct source observations.
    pub fn update(&mut self, input: Input) -> Option<Publication> {
        let snapshot = ApproachPose {
            right: input.basic_right_0,
            position: input.basic_position_48,
            feet: input.feet_256_272,
            fakie: input.fakie_155,
        };
        //82DEEB90/82DEEAF8, in this order even if both conditions hold.
        if input.air_event_439 {
            self.saved = snapshot;
            self.saved_fakie_initialized = true;
            self.history.clear();
        }
        if input.category == 100 {
            if self.history.len() == 30 {
                self.history.pop_front();
            }
            self.history.push_back(snapshot);
        }
        let new_approach =
            input.category == 400 && self.previous_category != 400 && self.away_frames > 30;
        if new_approach || (input.category == 400 && self.previous_kind != input.family) {
            self.saved = self.reference();
            self.saved_fakie_initialized |= self.history.len() == 30;
        }
        if new_approach {
            let across = signed(
                input.across,
                dot(input.across, sub(self.saved.position, input.point)) > 0.0,
            );
            let right = signed(
                self.saved.right,
                dot(
                    add(self.saved.feet[0], self.saved.feet[1]),
                    self.saved.right,
                ) > 0.0,
            );
            self.approach = u32::from(dot(right, across) > 0.0);
        }
        self.previous_kind = input.family;
        self.away_frames = if input.category == 400 {
            0
        } else {
            self.away_frames.wrapping_add(1)
        };
        let output = if let Some(family) = input.family.filter(|_| input.grinding_316) {
            //82DEF518: orientation, tilt, then twist, using actual Basic output
            //lanes rather than substituting the live board's current transform.
            let orientation = self.travel(input, family);
            let low = pose::low(input);
            let straight = pose::straight(input);
            let location = matches!(family, Family::Tipslide | Family::FiveO | Family::Backslash)
                && dot(
                    sub(input.point, input.basic_location_position_144),
                    input.basic_location_axis_96,
                ) <= 0.0;
            let components = Components([
                self.approach,
                u32::from(location),
                u32::from(straight),
                u32::from(low),
                orientation,
                family as u32,
            ]);
            if self.previous_category != 400 {
                self.pending = Some(components);
                self.pending_frames = 13;
            } else if self.pending == Some(components) {
                self.pending_frames = self.pending_frames.wrapping_add(1);
            } else {
                self.pending = Some(components);
                self.pending_frames = 0;
            }
            if self.pending_frames > 0 {
                self.animation = self.pending;
            }
            if self.pending_frames > 12 {
                self.scoring = self.pending;
            }
            Some(Publication {
                volatile: components,
                animation: self.animation,
                scoring: self.scoring,
            })
        } else {
            //82DEE508 tail, independent of the category.
            self.orientation = None;
            self.reversed = false;
            None
        };
        self.previous_category = input.category;
        output
    }

    fn travel(&mut self, input: Input, family: Family) -> u32 {
        let feet = add(input.feet_256_272[0], input.feet_256_272[1]);
        let right = signed(input.basic_right_0, dot(feet, input.basic_right_0) > 0.0);
        let forward = dot(right, input.direction) > 0.0;
        let orientation = if matches!(family, Family::FiftyFifty | Family::FiveO) {
            if self.orientation.is_none() {
                assert!(
                    self.history.len() == 30 || self.saved_fakie_initialized,
                    "saved fakie144 consumed before an actual history write"
                );
                let reference = self.reference();
                self.reversed = dot(feet, add(reference.feet[0], reference.feet[1])) < 0.0;
                if reference.fakie {
                    self.reversed = !self.reversed;
                }
            }
            match (forward, self.reversed) {
                (true, false) => 0,
                (false, false) => 1,
                (true, true) => 2,
                (false, true) => 3,
            }
        } else {
            u32::from(!forward)
        };
        self.orientation = Some(orientation);
        orientation
    }
}

#[cfg(any())]
#[path = "grind_chromosome/tests.rs"]
mod tests;
