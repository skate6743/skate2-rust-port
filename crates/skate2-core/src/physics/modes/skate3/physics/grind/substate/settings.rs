//! Stock mode endpoints and constructor-vtable-resolved jump getters.
use super::super::Family;
use skate3_core::{point_graph::PointGraph, riding::collision_response::CollisionResponseSettings};
use skate_data::collections::Collections;

pub(crate) struct Settings {
    pub(super) collision: CollisionResponseSettings,
    pub(super) animated_board_threshold: f32,
    //Mode order is the existing Processed.state_variant_index_2528 contract.
    vertical: [[[f32; 2]; 3]; 5],
}

impl Settings {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        let f = |name| data.float("physics_collision", "default", name);
        let graph = data
            .words::<20>("physics_collision", "default", "CollisionTorqueVsAngle")?
            .map(f32::from_bits);
        let mut vertical = [[[0.; 2]; 3]; 5];
        for (index, mode) in ["easy", "normal", "hardcore", "motorized", "test"]
            .into_iter()
            .enumerate()
        {
            for (family, fields) in [
                ["Hash_3097A69281990652", "Hash_FA4CDBAE0DFD1FAD"],
                ["Hash_B2B1170AFFC8AC69", "Hash_1B3E9F9C836D287D"],
                ["Hash_703829BD711E54DE", "Hash_F2473E9125079F0"],
            ]
            .into_iter()
            .enumerate()
            {
                vertical[index][family] = [
                    data.float("physics_mode", mode, fields[0])?,
                    data.float("physics_mode", mode, fields[1])?,
                ];
            }
        }
        Ok(Self {
            animated_board_threshold: data.float(
                "physics_animation",
                "default",
                "MaxDeckZAxisYForAnimatedDeck",
            )?,
            collision: CollisionResponseSettings {
                maximum_velocity_delta: f("MaxVelDelta")?,
                force_y_offset: f("ForceYOffset")?,
                force_scalar: f("CollisionForceScalar")?,
                target_displacement_velocity: f("TargetDisplacementVel")?,
                torque_vs_angle: PointGraph {
                    x: graph[4..12].try_into().unwrap(),
                    y: graph[12..20].try_into().unwrap(),
                },
            },
            vertical,
        })
    }

    pub(super) fn vertical(&self, family: Family, mode: u32, strength: f32) -> Result<f32, String> {
        let modes = self
            .vertical
            .get(mode as usize)
            .ok_or_else(|| format!("Invalid grind jump physics mode {mode}"))?;
        //Virtual64:82D40D60 common (including Darkslide),82D41CE8 board,
        //82D42720 tip. No strength clamp exists in these scalar getters.
        let group = match family {
            Family::Boardslide => 1,
            Family::Tipslide => 2,
            _ => 0,
        };
        let [low, high] = modes[group];
        Ok((1. - strength).mul_add(low, high * strength))
    }
}

pub(super) fn side(family: Family, geometry_kind: u32) -> f32 {
    match family {
        //82D41320 reads processed1464, returning literal820997B8 or82165A10.
        Family::Boardslide | Family::Darkslide => {
            if geometry_kind == 2 {
                f32::from_bits(0x3f66_6666)
            } else {
                0.
            }
        }
        //82D42AE8 literal820997B8.
        Family::Backslash => f32::from_bits(0x3f66_6666),
        //82D41FB0 literal8209975C; shared50-50/tip/5-O virtual60.
        _ => f32::from_bits(0x3f00_0000),
    }
}
