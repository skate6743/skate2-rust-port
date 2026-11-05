//! PowerSliding Update82BB28E8, instance82BB2D70 and helper82595930.
use super::motion_animation::MotionAnimation;
use skate3_core::input::set_turning::SlideLatch;
use skate3_core::{animation::playback_parameters::ParameterInputs, point_graph::PointGraph};
use skate_data::{collections::Collections, state_graph::attributes::Attributes};

pub struct Settings {
    speed_threshold: f32,
    well_into_slide: f32,
    speed_factor: PointGraph<4>,
    time_threshold: PointGraph<4>,
    threshold: PointGraph<4>,
    speed_to_lean: PointGraph<8>,
    phys_to_anim_spin: PointGraph<8>,
    smooth: f32,
    minimum_entry: f32,
    turn: f32,
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let float = |name| data.float("anim_motion", "power_slide", name);
        let curve = |name| -> Result<PointGraph<4>, String> {
            let w = data.words::<12>("anim_motion", "power_slide", name)?;
            Ok(PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(w[4 + i])),
                y: std::array::from_fn(|i| f32::from_bits(w[8 + i])),
            })
        };
        let curve8 = |name| -> Result<PointGraph<8>, String> {
            let w = data.words::<20>("anim_motion", "power_slide", name)?;
            Ok(PointGraph {
                x: std::array::from_fn(|i| f32::from_bits(w[4 + i])),
                y: std::array::from_fn(|i| f32::from_bits(w[12 + i])),
            })
        };
        Ok(Self {
            speed_threshold: float("slide_speed_threshold")?,
            well_into_slide: float("well_into_slide_time")?,
            speed_factor: curve("slide_speed_factor")?,
            time_threshold: curve("not_sliding_time_thresh")?,
            threshold: curve("not_sliding_threshold")?,
            // Native layout0,80,1688,1692,1680 respectively.
            speed_to_lean: curve8("slide_speed_to_lean")?,
            phys_to_anim_spin: curve8("slide_phys_to_anim_spin")?,
            smooth: float("slide_smooth")?,
            minimum_entry: float("slide_min_entry")?,
            turn: float("slide_turn_value")?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Update,
    Create { right: bool },
    ManualAttribute,
    Deceleration,
    Spin,
    Candidate,
    IsPowerSliding,
}
impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        Some(match a.text("name")? {
            "PowerSliding" => Self::Update,
            // Factory82BC9100 explicitly defaults authored right to true.
            "CreateSlide" => Self::Create {
                right: a.boolean_byte("right", 1) != 0,
            },
            "PowerSlideManualAtt" => Self::ManualAttribute,
            "PowerSlideDecel" => Self::Deceleration,
            "PowerSlideSpin" => Self::Spin,
            "InCandidateSlidingState" => Self::Candidate,
            "IsPowerSliding" => Self::IsPowerSliding,
            _ => return None,
        })
    }
}
///Native instance8..20; the three high flags initialize clear.
#[derive(Default)]
pub struct State {
    not_sliding_time: f32,
    previous_right: f32,
    previous_left: f32,
    was_ground: bool,
    was_right_start: bool,
    was_left_start: bool,
}
impl State {
    pub fn update(
        &mut self,
        animation: &MotionAnimation,
        category: u32,
        speed: f32,
        direction: f32,
        dt: f32,
        settings: &Settings,
        output: &mut SlideLatch,
    ) {
        let right = animation.motion_intent("RightSlide");
        let left = animation.motion_intent("LeftSlide");
        let right_value = right.unwrap_or(0.0);
        let left_value = left.unwrap_or(0.0);
        let ground = category == 1;
        if ground && !self.was_ground {
            self.not_sliding_time = 0.0;
            self.previous_right = right_value;
            self.previous_left = left_value;
            self.was_right_start = false;
            self.was_left_start = false;
        }
        self.was_ground = ground;
        if !ground {
            return;
        }
        let fast = speed > settings.speed_threshold;
        let right_start = animation.motion_intent("RightSlideStart").is_some();
        output.set_start(
            true,
            fast && right_start && !self.was_right_start && self.previous_right < right_value,
        );
        self.previous_right = right_value;
        self.was_right_start = right_start;
        let left_start = animation.motion_intent("LeftSlideStart").is_some();
        output.set_start(
            false,
            fast && left_start && !self.was_left_start && self.previous_left > left_value,
        );
        self.previous_left = left_value;
        self.was_left_start = left_start;
        let limit = settings.time_threshold.evaluate(direction);
        let sliding = settings.speed_factor.evaluate(speed) * (1.0 - direction.abs());
        self.not_sliding_time = if sliding >= settings.threshold.evaluate(direction) {
            0.0
        } else {
            self.not_sliding_time + dt
        };
        let stop = self.not_sliding_time > limit;
        output.advance_elapsed(true, dt);
        output.advance_elapsed(false, dt);
        output.set_end(
            true,
            output.elapsed(true) > settings.well_into_slide && (right.is_none() || !fast || stop),
        );
        output.set_end(
            false,
            output.elapsed(false) > settings.well_into_slide && (left.is_none() || !fast || stop),
        );
    }
}

///82595930 normalizes Motion80 with two root and reciprocal refinements,
/// then dots the authored Reckoning Z, reversing for Motion273.
pub fn direction(velocity: [f32; 4], mut z: [f32; 4], flipped: bool) -> f32 {
    use skate3_core::riding::ground_correction_math::dot_product;
    let squared = dot_product(velocity, velocity);
    //825959D4 ble also takes the unordered branch after fcmpu.
    if !(squared > f32::from_bits(0x3a83126f)) {
        return 1.0;
    }
    if flipped {
        z = z.map(|v| -v);
    }
    let mut root = 1.0 / squared.sqrt();
    for _ in 0..2 {
        let error = (-squared).mul_add(root * root, 1.0);
        root = (root * 0.5).mul_add(error, root);
    }
    let length = squared * root;
    let mut inverse = 1.0 / length;
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(length, 1.0), inverse);
    }
    dot_product(velocity.map(|v| v * inverse), z)
}

/// CreateSlide Update82BB2F60, ordered packet values (Slide, Turn).
pub fn create(
    latch: &SlideLatch,
    authored_right: bool,
    right_intent: Option<f32>,
    left_intent: Option<f32>,
    settings: &Settings,
) -> [f32; 2] {
    let right = authored_right ^ latch.captured_fakie();
    let mut slide = if right { right_intent } else { left_intent }.unwrap_or(0.0);
    if !(latch.elapsed(right) > settings.well_into_slide) {
        slide = if right {
            if settings.minimum_entry - slide >= 0.0 {
                settings.minimum_entry
            } else {
                slide
            }
        } else {
            let minimum = -settings.minimum_entry;
            if minimum - slide >= 0.0 {
                slide
            } else {
                minimum
            }
        };
    }
    let mut turn = if right { settings.turn } else { -settings.turn };
    if latch.captured_fakie() {
        turn = -turn;
        let negative = -slide;
        let absolute = negative.abs();
        let wrapped = if absolute > 0.5 {
            1.0 - absolute
        } else {
            absolute
        };
        slide = if negative >= 0.0 { wrapped } else { -wrapped };
    }
    [slide, turn]
}

/// PowerSlideDecel82BB31B0: inverse Ground basis rotates velocity without
/// translation. Source zeros local Y before measuring the horizontal speed.
pub fn deceleration(
    previous: &mut f32,
    velocity: [f32; 4],
    ground: [[f32; 4]; 4],
    settings: &Settings,
) -> f32 {
    let mut local = std::array::from_fn::<_, 4, _>(|lane| {
        let x = ground[lane][0] * velocity[0];
        let y = ground[lane][1].mul_add(velocity[1], x);
        ground[lane][2].mul_add(velocity[2], y)
    });
    local[1] = 0.0;
    let squared = skate3_core::riding::ground_correction_math::dot_product(local, local);
    let length = if squared == 0.0 {
        0.0
    } else {
        let mut inverse = 1.0 / squared.sqrt();
        for _ in 0..2 {
            let error = (-squared).mul_add(inverse * inverse, 1.0);
            inverse = (inverse * 0.5).mul_add(error, inverse);
        }
        squared * inverse
    };
    let target = settings.speed_to_lean.evaluate(length);
    let retained = (1.0 - settings.smooth) * *previous;
    *previous = target.mul_add(settings.smooth, retained);
    *previous
}

/// PowerSlideSpin Begin82BB33C8; the input is GetSlideDirection82595930.
pub fn spin(direction: f32, settings: &Settings) -> f32 {
    -settings.phys_to_anim_spin.evaluate(direction)
}
