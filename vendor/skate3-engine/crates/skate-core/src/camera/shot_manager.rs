//! Normal camera ShotManager82E06370/82E064B0/82E068B8/82E06E38.
//! The host owns immutable stock definitions and tree allocation. The authored
//! child order, filter history and transition calculations retain TU3 behavior.
use super::{Shot, blend_interval, filter_blend_value, position_from_angles};

#[derive(Clone, Debug, PartialEq)]
pub struct ShotDefinition {
    pub name: String,
    pub shot_type: u32,
    pub shot: Shot,
    pub transition_time: f32,
    pub transition_units: u32,
    pub children: [Option<String>; 3],
    pub blend_points: [f32; 3],
    pub blend_value: f32,
    pub blend_type: u32,
    pub blend_smoothing: f32,
}

impl ShotDefinition {
    fn initial() -> Self {
        Self {
            name: String::new(),
            shot_type: 0,
            shot: Shot::new(),
            transition_time: 0.5,
            transition_units: 0,
            children: [None, None, None],
            blend_points: [0.0; 3],
            blend_value: 0.0,
            blend_type: 0,
            blend_smoothing: 0.0,
        }
    }
}

pub trait ShotDatabase {
    fn load(&self, name: &str) -> Result<ShotDefinition, String>;
}

/// Runtime values are produced by CameraMan82E07708 and the subject compass.
/// The 6->5 compass override is applied here only when its subject flag is set.
pub trait ShotEnvironment {
    fn heading_mirror(&self) -> f32;
    fn compass_north(&self, entry: u32) -> f32;
    fn special_camera_flag(&self) -> bool;
    fn raw_blend_value(&mut self, kind: u32, authored: f32, dt: f32) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotPlacement {
    /// Native rig transform+48, not the anchor tracker's smoothed position.
    pub anchor: [f32; 4],
    pub camera_position: [f32; 4],
    pub clamp_height: u8,
    pub floor_height: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct ShotNode {
    definition: ShotDefinition,
    children: Vec<ShotNode>,
}

impl ShotNode {
    fn new(definition: ShotDefinition) -> Self {
        Self {
            definition,
            children: Vec::new(),
        }
    }

    fn setup(
        &mut self,
        database: &impl ShotDatabase,
        env: &mut impl ShotEnvironment,
        ancestry: &mut Vec<String>,
        count: &mut usize,
    ) -> Result<(), String> {
        if self.definition.shot_type != 1 {
            return Ok(());
        }
        ancestry.push(self.definition.name.clone());
        for name in self
            .definition
            .children
            .iter()
            .take_while(|name| name.is_some())
            .flatten()
        {
            if ancestry
                .iter()
                .any(|parent| parent.eq_ignore_ascii_case(name))
            {
                return Err(format!("Cyclic stock camera shot {name}"));
            }
            *count += 1;
            if *count > 64 {
                return Err("Stock camera shot exceeds its 64-child capacity".into());
            }
            let mut child = Self::new(database.load(name)?);
            child.setup(database, env, ancestry, count)?;
            self.children.push(child);
        }
        ancestry.pop();
        self.definition.blend_value =
            env.raw_blend_value(self.definition.blend_type, self.definition.blend_value, 0.0);
        Ok(())
    }

    fn update(&mut self, dt: f32, env: &mut impl ShotEnvironment) {
        if self.definition.shot_type == 0 {
            let entry = self.definition.shot.compass_north;
            let entry = if entry == 6 && env.special_camera_flag() {
                5
            } else {
                entry
            };
            self.definition
                .shot
                .update_normal(env.compass_north(entry), env.heading_mirror());
            return;
        }
        let definition = &mut self.definition;
        // Empty blended shots never reach the table indexing below in authored
        // stock data. Preserve their native no-publication result.
        if self.children.is_empty() {
            return;
        }
        let raw = env.raw_blend_value(definition.blend_type, definition.blend_value, dt);
        definition.blend_value = filter_blend_value(
            definition.blend_value,
            raw,
            definition.blend_smoothing,
            definition.blend_points[0],
            definition.blend_points[self.children.len() - 1],
        );
        let Some((left, right, fraction)) = blend_interval(
            definition.blend_points,
            core::array::from_fn(|i| i < self.children.len()),
            definition.blend_value,
        ) else {
            return;
        };
        // 82E06E98 and82E06EC4 are separate multiply/subtract/add operations.
        let fraction = crate::trigonometry::sin(
            fraction * f32::from_bits(0x40490fdb) - f32::from_bits(0x3fc90fdb),
        ) * 0.5
            + 0.5;
        self.children[left].update(dt, env);
        // Native updates the same child twice when both selected pointers match.
        self.children[right].update(dt, env);
        definition.shot.interpolate_from(
            self.children[left].definition.shot,
            self.children[right].definition.shot,
            fraction,
        );
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShotManager {
    current: ShotNode,
    pub previous: Shot,
    pub interpolated: Shot,
    pub elapsed: f32,
    pub transition_distance: f32,
    pub transition_duration: f32,
    pub transition_override: f32,
    pub instant_changes: bool,
}

impl ShotManager {
    pub fn new() -> Self {
        Self {
            current: ShotNode::new(ShotDefinition::initial()),
            previous: Shot::new(),
            interpolated: Shot::new(),
            elapsed: 0.0,
            transition_distance: 0.0,
            transition_duration: 0.0,
            transition_override: -1.0,
            instant_changes: false,
        }
    }
    pub fn current(&self) -> &ShotDefinition {
        &self.current.definition
    }

    /// CameraChooseShot::Begin82DF5C60, after SetShot. The previous behavior's
    /// outgoing override is used only when this one has no incoming override.
    pub fn apply_graph_transition(&mut self, incoming: f32, outgoing: f32) {
        if incoming >= 0.0 {
            self.current.definition.transition_time = incoming;
        } else if self.transition_override >= 0.0 {
            self.current.definition.transition_time = self.transition_override;
        }
        self.transition_override = if outgoing >= 0.0 { outgoing } else { -1.0 };
    }

    /// CameraMan reset follows child0 pointers to the authored leaf.
    pub fn first_leaf(&self) -> Shot {
        let mut node = &self.current;
        while let Some(child) = node.children.first() {
            node = child;
        }
        node.definition.shot
    }

    pub fn set_shot(
        &mut self,
        name: &str,
        force: bool,
        database: &impl ShotDatabase,
        env: &mut impl ShotEnvironment,
        placement: ShotPlacement,
    ) -> Result<bool, String> {
        // 82AE8B40 lowercases ASCII A..Z before the native comparison.
        if !force && self.current.definition.name.eq_ignore_ascii_case(name) {
            return Ok(false);
        }
        let mut current = ShotNode::new(database.load(name)?);
        self.previous = self.interpolated;
        if current.definition.shot.use_previous_shot != 0 {
            let shot = &mut current.definition.shot;
            shot.distance = self.interpolated.distance;
            shot.lens_length = self.interpolated.lens_length;
            shot.position_heading = self.interpolated.position_heading;
            shot.position_elevation = self.interpolated.position_elevation;
            shot.framing = self.interpolated.framing;
            shot.arm_orientation = self.interpolated.arm_orientation;
        }
        self.current = current;
        // Native calculates instant time before publishing the new distance.
        if self.instant_changes {
            self.make_instant();
        } else {
            self.elapsed = 0.0;
        }
        self.current.setup(database, env, &mut Vec::new(), &mut 0)?;
        let shot = self.current.definition.shot;
        let target = position_from_angles(
            placement.anchor,
            [0.0; 4],
            shot.position_elevation,
            shot.position_heading,
            shot.distance,
            placement.clamp_height,
            placement.floor_height,
        );
        self.transition_distance = super::vector_tracker::length(core::array::from_fn(|i| {
            target[i] - placement.camera_position[i]
        }));
        Ok(true)
    }

    pub fn update(&mut self, dt: f32, env: &mut impl ShotEnvironment) -> f32 {
        self.elapsed = dt + self.elapsed;
        self.current.update(dt, env);
        self.transition_duration = self.duration(false);
        let fraction = if self.transition_duration <= 0.0 {
            1.0
        } else {
            self.elapsed / self.transition_duration
        };
        let fraction = if -fraction >= 0.0 { 0.0 } else { fraction };
        let fraction = if 1.0 - fraction >= 0.0 { fraction } else { 1.0 };
        self.interpolated
            .interpolate_from(self.previous, self.current.definition.shot, fraction);
        fraction
    }

    pub fn make_instant(&mut self) {
        self.transition_override = -1.0;
        self.elapsed = self.duration(false);
    }

    /// CameraMan82DFF174 clamps a distance-transition speed <=0 to0.0001;
    /// ShotManager82E06418 and MakeInstant82E07450 do not apply that clamp.
    pub fn is_transitioning(&self) -> bool {
        self.elapsed < self.duration(true)
    }

    /// CameraMan82DFF964..82DFFA08 uses a separate sine-eased framing blend.
    pub fn framing_fraction(&self) -> f32 {
        let duration = self.duration(true);
        if duration <= 0.0 {
            return 1.0;
        }
        let fraction = super::manager_state::clamp(self.elapsed / duration, 0.0, 1.0);
        crate::trigonometry::sin(
            fraction.mul_add(f32::from_bits(0x40490fdb), -f32::from_bits(0x3fc90fdb)),
        )
        .mul_add(0.5, 0.5)
    }

    fn duration(&self, camera_man: bool) -> f32 {
        let definition = &self.current.definition;
        match definition.transition_units {
            1 => definition.transition_time * f32::from_bits(0x3c888889),
            2 => {
                let speed = if camera_man && definition.transition_time <= 0.0 {
                    f32::from_bits(0x38d1b717)
                } else {
                    definition.transition_time
                };
                self.transition_distance / speed
            }
            _ => definition.transition_time,
        }
    }
}
