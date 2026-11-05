//! TU3 actor checkpoint history (82BFB068..82BFCB38).
//! Scene predicates are supplied by the host; absence is never acceptance.
use crate::physics::skeleton_animation_record::{AnimationPartTransform as Matrix, IDENTITY};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub transform: Matrix,
    pub stance: u32,
    pub offboard: bool,
    pub score: f32,
}

/// Separate native predicates: history selection does not repeat ground or edge
/// validation, whereas current-position selection does (82BFC038/82BFC378).
pub trait Validation {
    type Error;
    fn ground(&mut self, transform: &Matrix) -> Result<Option<Ground>, Self::Error>;
    fn location(&mut self, transform: &Matrix) -> Result<bool, Self::Error>;
    fn occupants(&mut self, transform: &Matrix) -> Result<bool, Self::Error>;
    fn edges(&mut self, transform: &Matrix) -> Result<bool, Self::Error>;
}

#[derive(Clone, Copy, Debug)]
pub struct Ground {
    pub position: [f32; 4],
    pub offboard: bool,
}

///Completed native packet fields. The caller must provide actual producer
///values, especially Ground323 and conditioner164; neither means grounded.
pub struct Observation {
    pub measurements: i32,
    pub root_position: [f32; 4], //Skeleton416, unmirrored11920 translation
    pub com_position: [f32; 4],  //Skeleton64, physical COM16144
    pub teleport_requested: bool, //State69
    pub physical_state: u32,     //State16
    pub state_frames: u32,       //State4
    pub ground_suppressed: bool, //Ground323
    pub offboard_correction: bool, //OffBoard333
    pub ground_category: u32,    //Collision16 low16
    pub foot_categories: [u32; 2], //OffBoard52/56 packed categories
    pub riding_transform: Matrix, //82591E30(false)
    pub offboard_transform: Matrix, //82591E30(true)
    pub stance: u32,             //82592B68
    pub alternate_world: bool,   //this+0x50 slot0: INetworkGameQuery IsNetworkGame (as 82BFBB48)
}

pub struct History {
    initial: Candidate,
    entries: Vec<Candidate>, // native33 slots reserve one sentinel:32 entries
    cooldown: i32,
    pub current_orientation: Matrix, //208; only eligible riding states write it
    pub current_position: [f32; 4],  //272; every actor update writes Skeleton64
}
impl History {
    pub fn new(initial: Candidate) -> Self {
        Self {
            initial,
            entries: vec![initial],
            cooldown: 20,
            current_orientation: IDENTITY,
            current_position: initial.transform[3],
        }
    }

    ///A new Actor's manager (82BFB068): mSpawnPosition, the identity orientation, the spawn's
    ///translation as the current position, and the spawn as the only entry.
    pub fn respawn(&mut self, initial: Candidate) {
        *self = Self::new(initial);
    }

    ///Live entries, oldest first.
    pub fn entries(&self) -> &[Candidate] {
        &self.entries
    }

    ///mSpawnPosition, the empty history's last resort.
    pub fn initial(&self) -> &Candidate {
        &self.initial
    }

    ///The entries the same Actor's manager holds, oldest first, in place of these; the ring
    ///keeps the newest 32.
    pub fn import(&mut self, entries: &[Candidate]) {
        self.entries.clear();
        self.entries
            .extend_from_slice(&entries[entries.len().saturating_sub(32)..]);
    }

    ///82BFB1E0/82BFB3F8. Do not call this with last-grounded observations;
    ///the native actor consumes one completed physical publication per update.
    pub fn observe<V: Validation>(
        &mut self,
        input: &Observation,
        minimum_state_frames: u32,
        scene: &mut V,
    ) -> Result<(), V::Error> {
        self.current_position = input.com_position;
        if !self.recording_due(input.measurements, input.root_position) || input.teleport_requested
        {
            return Ok(());
        }
        let (transform, offboard, score) = match input.physical_state {
            100 | 104 => {
                //This write precedes both the15-frame gate and terrain checks.
                self.current_orientation = input.riding_transform;
                if input.state_frames <= minimum_state_frames
                    || input.ground_suppressed
                    || !surface_allowed(input.ground_category)
                {
                    return Ok(());
                }
                (
                    input.riding_transform,
                    input.ground_category == 8,
                    surface_score(input.ground_category),
                )
            }
            500 | 502 => {
                if input.state_frames <= minimum_state_frames || input.offboard_correction {
                    return Ok(());
                }
                (
                    input.offboard_transform,
                    true,
                    surface_score(input.foot_categories[0])
                        .max(surface_score(input.foot_categories[1])),
                )
            }
            _ => return Ok(()),
        };
        if !scene.location(&transform)? || !scene.edges(&transform)? {
            return Ok(());
        }
        if !input.alternate_world && scene.ground(&transform)?.is_none() {
            return Ok(());
        }
        //The recording path discards the ray's offboard byte and hit position.
        self.insert(Candidate {
            transform,
            stance: input.stance,
            offboard,
            score,
        });
        Ok(())
    }

    ///82BFB1E0: call even when the physical state cannot supply a candidate.
    ///The120 gate uses conditioner164 (EmpiricalMeasurements82DE5858), not
    ///State4's time in the current state. Its reset lifecycle is caller-owned.
    pub fn recording_due(&mut self, measurements: i32, root_position: [f32; 4]) -> bool {
        let mut eligible = measurements >= 120;
        if !eligible {
            self.cooldown = 0;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            eligible = false;
        }
        eligible
            && !self
                .entries
                .iter()
                .rev()
                .any(|entry| distance_squared(entry.transform[3], root_position) < 2.25)
    }

    ///82BFCB38. Construction and successful recording both refresh cooldown.
    pub fn insert(&mut self, candidate: Candidate) {
        if self.entries.len() == 32 {
            self.entries.remove(0);
        }
        self.entries.push(candidate);
        self.cooldown = 20;
    }

    ///82BFC550: descending score, newest wins ties; selected entry is removed
    ///before validation, including successful selections. Repeated bails thus
    ///cannot keep selecting the same history entry.
    fn pop_best(&mut self) -> Option<Candidate> {
        let mut best = None;
        let mut score = -99990.0;
        for (age, index) in (0..self.entries.len()).rev().enumerate() {
            let mut value = self.entries[index].score;
            if age < 1 {
                value -= 100.0;
            }
            if age > 5 {
                value -= 200.0;
            }
            if value > score {
                best = Some(index);
                score = value;
            }
        }
        best.map(|index| self.entries.remove(index))
    }

    ///82BFC9B0, used by automatic actor reset82592518. Manual/session-marker
    ///selection82BFC828 has a different history walk and must not call this.
    pub fn automatic<V: Validation>(
        &mut self,
        stance: u32,
        scene: &mut V,
    ) -> Result<Candidate, V::Error> {
        if let Some(spot) = self.wipeout_spot(stance, scene)? {
            return Ok(spot);
        }
        while let Some(candidate) = self.pop_best() {
            if scene.location(&candidate.transform)? && scene.occupants(&candidate.transform)? {
                return Ok(candidate);
            }
        }
        Ok(self.last_resort(stance))
    }

    ///GetSuggestedRespawnPosition82BFC828 (PlayerUI, cTeleportManager, NIS): the wipeout spot,
    ///else GetBreadCrumb82BFC1E8's newest entry the living world accepts, which stays in the
    ///history (no edge or network test), else the last resort.
    pub fn suggested<V: Validation>(&self, stance: u32, scene: &mut V) -> Result<Candidate, V::Error> {
        if let Some(spot) = self.wipeout_spot(stance, scene)? {
            return Ok(spot);
        }
        for candidate in self.entries.iter().rev() {
            if scene.occupants(&candidate.transform)? {
                return Ok(*candidate);
            }
        }
        Ok(self.last_resort(stance))
    }

    ///GetWipeoutSpotBreadCrumb82BFC038: the riding orientation with the current position,
    ///ground82BFBC18, network82BFBB48, living world82BFB928, then edges82BFB6F8. A category8
    ///ground puts the off-board spot on the hit's height.
    fn wipeout_spot<V: Validation>(&self, stance: u32, scene: &mut V) -> Result<Option<Candidate>, V::Error> {
        let mut transform = self.current_orientation;
        transform[3] = self.current_position;
        let Some(ground) = scene.ground(&transform)? else {
            return Ok(None);
        };
        if !scene.location(&transform)? || !scene.occupants(&transform)? || !scene.edges(&transform)? {
            return Ok(None);
        }
        if ground.offboard {
            transform[3][1] = ground.position[1];
        }
        Ok(Some(Candidate {
            transform,
            stance,
            offboard: ground.offboard,
            score: 0.0,
        }))
    }

    ///GetLastResortBreadCrumb82BFC718: the newest entry, or with none mSpawnPosition with the
    ///current stance, on board.
    fn last_resort(&self, stance: u32) -> Candidate {
        self.entries.last().copied().unwrap_or(Candidate {
            transform: self.initial.transform,
            stance,
            offboard: false,
            score: 0.0,
        })
    }
}

///82BFB3F8 rejects these categories; category8 explicitly requests offboard.
pub fn surface_allowed(category: u32) -> bool {
    !matches!(category, 5 | 6 | 9 | 12 | 13)
}

///Jump table82BFB698: category1/2=100,3/4=50,11=20, all others=0.
pub fn surface_score(category: u32) -> f32 {
    match category {
        1 | 2 => 100.0,
        3 | 4 => 50.0,
        11 => 20.0,
        _ => 0.0,
    }
}

fn distance_squared(a: [f32; 4], b: [f32; 4]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

#[cfg(test)]
#[path = "tests/respawn.rs"]
mod tests;
