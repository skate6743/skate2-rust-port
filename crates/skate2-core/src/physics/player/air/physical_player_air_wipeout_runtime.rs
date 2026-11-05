//! CheckForAirWipeout(false)82de10b8 and `SensitiveCollision82dd4490`.
//! Child of existing Ground runtime; all reads/writes use its SAME-pair Source.
use super::{
    Arc, Attribute, AttributeRuntimeData, CollisionFlag, DebugOverlaySlot,
    GroundWipeoutRuntimeError, GroundWipeoutSource, PhysicalAssemblyWorld,
    PhysicalPlayerAssemblyOwner, ReckoningNumeric, Scalar, SimulationSkaterAssemblyHandle,
    SkateboardAttributeRead, Source, detector,
};
use crate::{
    dynamic_assembly_session::skater_pool::board_entry::try_current_board_effective_transform,
    input_cvar_runtime::wipeout_decision::WipeoutDecisionCvars,
    skateboard_attributes::SkateboardAttributeClass,
};
use detector::leaves;

const FRAMES: u64 = 0xc5d7_9b68_4bf0_2b40;
const SENSITIVE_SKELETON: u64 = 0x1bab_253c_c346_b466;
const SENSITIVE_DECK: u64 = 0x81f4_6d87_7dfc_ed38;
const AIR_XZ: u64 = 0xe061_242d_602f_612d;
const AIR_Y: u64 = 0xb482_ba6c_f272_0178;
const LIGHT_OBJECT: u64 = 0xedb9_734d_358f_0e54;
const SQUASH: u64 = 0x09ed_0901_2ea6_80e3;
const DISPLACEMENT: u64 = 0xaa44_05c5_949d_1845;
const ARMS: u64 = 0xd031_7181_ce40_bfbc;
const BODY: u64 = 0x68e0_e915_d4f3_af66;

impl<N: ReckoningNumeric> Source<'_, N> {
    pub(super) fn air_attribute_pointer(
        &mut self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attributes| {
                attributes.lookup(SkateboardAttributeClass::Wipeout, key)
            })
            .map_err(E::BoardAttributes)?
            .map_err(E::Attribute)
    }
    pub(super) fn air_attribute(
        &mut self,
        key: u64,
    ) -> Result<f32, GroundWipeoutRuntimeError<N::Error>> {
        let pointer = self.air_attribute_pointer(key)?;
        self.wipeout_float(&pointer)
    }
    fn air_frame_threshold(&mut self) -> Result<i32, GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let pointer = self.air_attribute_pointer(FRAMES)?;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_ground_force_board(&self.world.backend, self.board, |_, _, attributes| {
                attributes.integer(&pointer)
            })
            .map_err(E::BoardAttributes)?
            .map_err(E::Attribute)
    }
    fn air_wheels(&mut self) -> Result<u8, GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        Ok(self
            .world
            .board_construction(self.board)
            .map_err(E::Pool)?
            .and_then(crate::skateboard_body_construction::SkateboardBodyConstruction::post_fields)
            .ok_or(E::Missing("board post fields"))?
            .num_wheels)
    }
    pub(super) fn air_display_debug(&self) -> Result<bool, GroundWipeoutRuntimeError<N::Error>> {
        WipeoutDecisionCvars::new(&self.owner.inputs().cvars)
            .display_debug()
            .map_err(GroundWipeoutRuntimeError::Cvar)
    }
    pub(super) fn air_require_overlay(&self) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        self.overlay
            .require()
            .map_err(GroundWipeoutRuntimeError::Overlay)?;
        // Source debug calls at1214/1418/16c0 target825187c0, a single BLR.
        Ok(())
    }
    fn air_deck_counter(&self) -> Result<i32, GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_air_deck_counter()
            .map_err(E::Owner)
    }
    fn air_store_deck_counter(
        &mut self,
        value: i32,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        self.owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_store_air_deck_counter(value)
            .map_err(E::Owner)
    }
    fn try_sensitive_collision(&mut self) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        // The second independent block rereads live collision and can request21
        // again. Parts are read only when that second collision test is false.
        if self.processed_flag(0x8cc, 0x1000_0000)?
            && self.collision_flag(CollisionFlag::CompliantJustCollided)?
        {
            self.request(21)?;
        }
        if self.processed_flag(0x8c8, 0x100)?
            && (self.collision_flag(CollisionFlag::CompliantJustCollided)?
                || self.num_parts_in_contact()? != 0)
        {
            self.request(21)?;
        }
        Ok(())
    }
    fn try_air_wipeout(
        &mut self,
        use_com: bool,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        if self.state()? != 2 {
            self.air_store_deck_counter(0)?; //10e8 BEFORE state10ec.
            self.set_state(2)?;
        }
        if self.num_parts_in_contact()? != 0 {
            self.air_store_deck_counter(4)?;
        }
        let counter = self.air_deck_counter()?;
        if counter > 0 {
            self.air_store_deck_counter(counter - 1)?;
        }
        let wheels = self.air_wheels()? != 0;
        if self.air_display_debug()? {
            let _fresh_wheels = self.air_wheels()?;
            let _fresh_parts = self.num_parts_in_contact()?;
            self.air_require_overlay()?;
        }
        let timer = self.invincible_timer()?;
        // Raw1228=40990024 BO4 BI25: GT-clear (including NaN) enters active.
        if timer > 0.0 {
            let dt = self.scalar(Scalar::TimeDelta)?;
            self.set_invincible_timer(timer - dt)?;
            return Ok(());
        }
        leaves::vehicle(self)?;
        let frames = self.air_frame_threshold()?;
        let _effective = try_current_board_effective_transform(
            self.world.board(self.board).map_err(E::Pool)?,
            &self.world.backend,
            self.owner,
        )
        .map_err(E::AirEffectiveTransform)?;
        let mut skeleton_scalar = 1.0;
        let mut deck_scalar = 1.0;
        if self.processed_flag(0x8cc, 0x1000_0000)? {
            skeleton_scalar = self.air_attribute(SENSITIVE_SKELETON)?;
            deck_scalar = self.air_attribute(SENSITIVE_DECK)?;
        }
        // lhz+8c8 &1 denotes FULL big-endian word bit00010000.
        let trick = self.processed_flag(0x8c8, 0x0001_0000)?
            && self.processed(|p| p.physics_state_fields().frames_since_jump_correct)? > frames;
        let (mut xz, y) = if trick {
            let xz = self.air_attribute(Attribute::TrickXZ.key())?;
            let y = self.air_attribute(Attribute::TrickY.key())?;
            if self.air_display_debug()? {
                self.air_require_overlay()?;
            }
            (xz, y) //This source branch overwrites, not multiplies deck scalar.
        } else {
            let xz = if wheels {
                self.air_attribute(Attribute::GroundXZ.key())?
            } else {
                self.air_attribute(AIR_XZ)?
            } * deck_scalar;
            let y = if wheels {
                self.air_attribute(Attribute::GroundY.key())?
            } else {
                self.air_attribute(AIR_Y)?
            };
            (xz, y)
        };
        if self.board_contact_flag(0x2000_0000)? {
            xz *= self.air_attribute(LIGHT_OBJECT)?;
        }
        leaves::deck_acceleration(self, xz, y)?;
        let squash = self.air_attribute(SQUASH)? * skeleton_scalar;
        if leaves::squashed(self, squash)? {
            self.request(18)?;
        } else {
            let displacement = self.air_attribute(DISPLACEMENT)? * skeleton_scalar;
            if leaves::displacement(self, displacement)? {
                self.request(1)?;
            } else {
                //1634 arms pointer,1648 body pointer,1650 arms load,1658 body load.
                let arms_pointer = self.air_attribute_pointer(ARMS)?;
                let body_pointer = self.air_attribute_pointer(BODY)?;
                let arms = self.wipeout_float(&arms_pointer)?;
                let body = self.wipeout_float(&body_pointer)?;
                if leaves::contact_force(self, skeleton_scalar * body, arms)? {
                    self.request(0)?;
                    if self.air_display_debug()? {
                        self.air_require_overlay()?;
                    }
                }
            }
        }
        let body_flip = self
            .owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("retained Reckoning"))?
            .does_request_body_flip_wipeout();
        if body_flip {
            self.request(4)?;
        }
        super::bad_landing::try_bad_landing(self, use_com)?;
        leaves::leaning(self)?;
        self.try_sensitive_collision()
    }
}

impl PhysicalAssemblyWorld {
    /// SAME-pair live CheckForAirWipeout, scheduled by Air/KnownAir.Post
    /// after Skeleton.Post. Does not claim prior stage completion or fill `PhysOut`.
    pub(crate) fn try_check_air_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
        use_com: bool,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay,
        }
        .try_air_wipeout(use_com)
    }
}
