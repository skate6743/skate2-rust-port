//! TU3 `PhysicalPlayerHiLOD` 0x82db3ce8's shove probe (0x82d7a440), the pass before
//! `Update1_ProcessInput` copies it to Processed+1792 and resets it (0x82d7a330). While the shove
//! behaviour holds state flag 86 it finds a pedestrian within reach of the hands
//! (`LivingWorld::Manager` 0x826bed80, answered by the host) and tests it against the strike zone
//! (0x82d7a770). Every other skater is a type 1 target first (0x82d7a440's physicals loop), with
//! the system position and velocity of its physical output; a pedestrian struck after one wins.
use super::skater::Skate3ControllerState;

/// `Sk8::Physics::kOBLT_TrajEndLength`: the query point's reach ahead of the skater.
const REACH: f32 = 0.8;
/// 0x826bed80's body margin (the folded constant IDA names `ROPA::cv_NormDistanceScale`).
const BODY_MARGIN: f32 = 1.5;
/// 0x82d7a770's look-ahead time (0x82099280) for the anticipation zone.
const LOOK_AHEAD: f32 = 0.2;

/// What the probe reads of the previous frame's physical output.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShoveFrame {
    /// `PhysOut_SystemReckoning` +0: the conditioned centre of mass velocity.
    velocity: [f32; 4],
    /// `PhysOut_SystemReckoning` +64.
    position: [f32; 4],
    /// `PhysOut_Skeleton` +432..+464: the effective animation-to-world X, Y and Z axes.
    rows: [[f32; 4]; 3],
}

/// A pedestrian the host's query found (`GetPedestriansWithinRange`, in its order).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShovePedestrian {
    pub(crate) id: u32,
    /// The entity core's translation and velocity (`Pedestrian` vslots 4 and 6).
    pub(crate) position: [f32; 4],
    pub(crate) velocity: [f32; 4],
    /// `ITargetable::IsWipedOut`.
    pub(crate) wiped_out: bool,
    /// The skater collision body's Part 0 transform, rows X, Y, Z and translation.
    pub(crate) part: [[f32; 4]; 4],
    /// The model's dimensions (`LWBasicEntity::GetWidth`, `GetHeight`, `GetLength`; 0 without one).
    pub(crate) dimensions: [f32; 4],
}

/// Another skater the probe tests (0x82d7a440: its PhysOut system reckoning +64 and +0).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShoveSkater {
    pub(crate) id: u32,
    pub(crate) position: [f32; 4],
    pub(crate) velocity: [f32; 4],
}

/// A shove that hit a pedestrian or a skater (type 2 or 1): its id and the velocity
/// `ProcessOutput` (0x82db7fc0) sends.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShoveHit {
    pub(crate) id: u32,
    pub(crate) velocity: [f32; 4],
    pub(crate) skater: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ShoveProbe {
    frame: Option<ShoveFrame>,
    pedestrians: Vec<ShovePedestrian>,
    skaters: Vec<ShoveSkater>,
    pub(crate) hit: Option<ShoveHit>,
}

impl ShoveProbe {
    /// After the frame's outputs: the next probe pass's inputs, kept only while flag 86 holds.
    pub(super) fn capture(skater: &mut Skate3ControllerState) {
        let enabled = skater.player_state.state_flags[86 - 52];
        skater.shove.frame = enabled.then(|| {
            let mut root = skater.animated_skeleton.roots.animation_to_world;
            // GetEffectiveRoot 0x82be3650 negates X and Z with Processed2476 bit 2.
            if skater.player_input.processed.flags_2476 & 4 != 0 {
                root[0] = root[0].map(|v| -v);
                root[2] = root[2].map(|v| -v);
            }
            ShoveFrame {
                velocity: skater.centre_of_mass_output.velocity,
                position: skater.player_input.physical.reckoning.vector_64.map(f32::from_bits),
                rows: [root[0], root[1], root[2]],
            }
        });
    }

    /// The pedestrian query's centre (0x82d7a670: position + Z axis * reach) while the probe runs.
    pub(crate) fn query_point(&self) -> Option<[f32; 3]> {
        self.frame.map(|frame| std::array::from_fn(|i| frame.rows[2][i].mul_add(REACH, frame.position[i])))
    }

    pub(crate) fn set_pedestrians(&mut self, pedestrians: Vec<ShovePedestrian>) {
        self.pedestrians = pedestrians;
    }

    pub(crate) fn set_skaters(&mut self, skaters: Vec<ShoveSkater>) {
        self.skaters = skaters;
    }
}

/// 0x82db3ce8 -> 0x82d7a440, before this frame's input phase copies the probe. The animation
/// packet's external physics flag 10689 skips it.
pub(super) fn update(skater: &mut Skate3ControllerState, external_physics_flag: u8) {
    let frame = skater.shove.frame.take();
    let pedestrians = std::mem::take(&mut skater.shove.pedestrians);
    let skaters = std::mem::take(&mut skater.shove.skaters);
    let Some(frame) = frame.filter(|_| external_physics_flag == 0) else { return };
    for target in &skaters {
        strike(skater, &frame, target.position, target.velocity, 1, target.id);
    }
    let point: [f32; 3] = std::array::from_fn(|i| frame.rows[2][i].mul_add(REACH, frame.position[i]));
    let target = pedestrians.iter().find(|pedestrian| in_reach(pedestrian, point));
    trace(format_args!("probe at {point:?}: {} pedestrians, in reach {:?}", pedestrians.len(), target.map(|t| t.id)));
    let Some(target) = target else { return };
    strike(skater, &frame, target.position, target.velocity, 2, target.id);
}

/// `SKATE2_TRACE_SHOVE`.
pub(crate) fn trace(message: std::fmt::Arguments<'_>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SHOVE").is_some()) {
        eprintln!("shove: {message}");
    }
}

/// 0x826bed80's per-pedestrian test: not wiped out, and the point within the skater collision
/// body's box (half its dimensions plus the margin) in that body's frame.
fn in_reach(pedestrian: &ShovePedestrian, point: [f32; 3]) -> bool {
    if pedestrian.wiped_out {
        return false;
    }
    let [x_axis, y_axis, z_axis, origin] = pedestrian.part;
    let d: [f32; 3] = std::array::from_fn(|i| point[i] - origin[i]);
    let local = |axis: [f32; 4]| axis[2].mul_add(d[2], axis[1].mul_add(d[1], axis[0] * d[0]));
    let within = |value: f32, extent: f32| value.abs() < extent.mul_add(0.5, BODY_MARGIN);
    within(local(z_axis), pedestrian.dimensions[1])
        && within(local(x_axis), pedestrian.dimensions[0])
        && within(local(y_axis), pedestrian.dimensions[2])
}

/// 0x82d7a770: the target in the skater's frame now (strike zone) and 0.2 s ahead (anticipation
/// zone). Either records the target; a strike latches it.
fn strike(skater: &mut Skate3ControllerState, frame: &ShoveFrame, a: [f32; 4], b: [f32; 4], kind: u32, id: u32) {
    let d: [f32; 4] = std::array::from_fn(|i| a[i] - frame.position[i]);
    let e: [f32; 4] = std::array::from_fn(|i| (b[i] - frame.velocity[i]).mul_add(LOOK_AHEAD, d[i]));
    let local = |v: [f32; 4], axis: usize| {
        let row = frame.rows[axis];
        row[2].mul_add(v[2], row[1].mul_add(v[1], row[0] * v[0]))
    };
    let anticipated = local(e, 0).abs() < 1.0 && local(e, 2) < 1.0 && local(e, 2) > 0.0;
    let hit = local(d, 0).abs() < 0.8 && local(d, 2) < 0.9 && local(d, 2) > 0.0;
    trace(format_args!("strike {id}: local now [{}, {}] ahead [{}, {}] -> anticipated {anticipated} hit {hit}",
        local(d, 0), local(d, 2), local(e, 0), local(e, 2)));
    if !(anticipated || hit) {
        return;
    }
    let probe = &mut skater.player_input.player.probe;
    probe.vectors_16_32_48 = [
        a.map(f32::to_bits),
        std::array::from_fn(|i| (b[i] - frame.velocity[i]).mul_add(LOOK_AHEAD, a[i]).to_bits()),
        b.map(f32::to_bits),
    ];
    probe.words_64_68 = [kind, id];
    probe.bytes_72_73 = [1, u8::from(hit)];
}

/// `ProcessOutput` 0x82db7fc0: the shove's velocity, 7 m/s from the skater's system position to
/// the target (none when they coincide) plus the board's velocity relative to the target's.
pub(super) fn shove_velocity(target: [f32; 4], target_velocity: [f32; 4], system_position: [f32; 4], board_velocity: [f32; 4]) -> [f32; 4] {
    let d: [f32; 4] = std::array::from_fn(|i| target[i] - system_position[i]);
    let squared = d[2].mul_add(d[2], d[1].mul_add(d[1], d[0] * d[0]));
    let length = if squared == 0.0 { 0.0 } else { squared.sqrt() };
    let scale = if length > 1e-6 { 7.0 / length } else { 0.0 };
    std::array::from_fn(|i| d[i].mul_add(scale, board_velocity[i] - target_velocity[i]))
}

/// `SKATE2_TRACE_SHOVE`: a wipeout request's reasons, once per change.
pub(crate) fn trace_wipeout(state: impl std::fmt::Debug, reasons: &[bool], wipeout: bool, runout: bool, forces: [f32; 3]) {
    static LAST: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
    let active: Vec<usize> = reasons.iter().enumerate().filter_map(|(index, &on)| on.then_some(index)).collect();
    let Ok(mut last) = LAST.lock() else { return };
    if *last != active {
        trace(format_args!("wipeout request in {state:?}: reasons {active:?} wipeout {wipeout} runout {runout} forces {forces:?}"));
        *last = active;
    }
}
