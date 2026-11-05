//! Native foot placement82D773F8/82D77558/82D77AE8. The persistent storage is
//! shared with the original state56 manager; all contacts come from Processed.
use super::{
    air_launch::{V, dot, length, limit_angle, madd, scale, sub, unit},
    board_possession::manager::State,
    ground_entry::Frame,
};
use crate::physics::skeleton_animation_record::transform_point;
pub struct Line {
    pub position: V,
    pub normal: V,
    pub surface: u32,
    pub valid: bool,
}
pub struct Input {
    pub lines: [Line; 2],
    pub local_foot_pairs: [[V; 2]; 2],
    pub world_foot_pairs: [[V; 2]; 2],
    pub root: Frame,
    pub inverse_root: Frame,
    pub effective_root: Frame,
    pub position: V,
    pub velocity: V,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub state: u32,
}
pub struct Target {
    pub position: V,
    pub world: bool,
    pub blend: f32,
    pub normal: Option<V>,
}
pub fn update(s: &mut State, i: &Input) -> [Target; 2] {
    for (index, (foot, line)) in s.hands.iter_mut().zip(&i.lines).enumerate() {
        foot.vectors_0_to_64[1] = i.world_foot_pairs[index][0];
        foot.vectors_0_to_64[3] = line.position;
        foot.vectors_0_to_64[4] = line.normal;
        foot.word_80 = line.surface;
        foot.flag_84 = line.valid;
        let side = index ^ usize::from(i.flags_2476 & 4 != 0);
        foot.flags_104_to_107[2] = i.flags_2484 & (1 << (6 + side)) != 0;
        foot.flags_104_to_107[3] = i.flags_2484 & (1 << (2 + side)) != 0;
    }
    if s.words_308_to_316[0] == 0 {
        s.vectors_224_to_272[2] = i.root[3];
        s.flags_304_to_307[3] = true;
    }
    //82D783A8: evaluate the old support mode before82D77CE0 replaces it.
    for index in 0..2 {
        let foot = &mut s.hands[index];
        let low = s.word_300 != 3 && i.local_foot_pairs[index].iter().all(|p| p[1] < 0.09);
        foot.flags_104_to_107[0] = low;
        let supported = if foot.flag_84 {
            let normal = clamp_normal(foot.vectors_0_to_64[4]);
            foot.vectors_0_to_64[2] = normal;
            let distances =
                i.world_foot_pairs[index].map(|p| dot(normal, sub(p, foot.vectors_0_to_64[3])));
            let distance = distances[0].max(distances[1]);
            distance < 0.09 || (low && distance < 0.2)
        } else {
            foot.vectors_0_to_64[2] = [0., 1., 0., 0.];
            low
        };
        foot.flags_104_to_107[1] = supported;
    }
    let mirrored = i.flags_2476 & 4 != 0;
    let forced_left = i.flags_2480 & (if mirrored { 1 << 5 } else { 1 << 6 }) != 0;
    let forced_right = i.flags_2480 & (if mirrored { 1 << 6 } else { 1 << 5 }) != 0;
    if forced_left {
        s.hands[0].flags_104_to_107[1] = true;
        s.hands[1].flags_104_to_107[1] = false;
    } else if forced_right {
        s.hands[0].flags_104_to_107[1] = false;
        s.hands[1].flags_104_to_107[1] = true;
    }
    s.scalars_288_to_296[1] += f32::from_bits(0x3c888889);
    let support = if i.state == 501 {
        3
    } else if s.flags_304_to_307[0] {
        match s.hands.map(|h| h.flags_104_to_107[1]) {
            [true, false] => 0,
            [false, true] => 1,
            _ => 2,
        }
    } else {
        2
    };
    if support != s.word_300 {
        s.word_300 = support;
        s.scalars_288_to_296[1] = 0.;
        s.vectors_224_to_272[3] = if support < 2 {
            i.world_foot_pairs[support as usize][0]
        } else {
            [0.; 4]
        };
    }
    for index in 0..2 {
        let support_index = index as u32;
        let foot = s.hands[index];
        let mut desired = foot.vectors_0_to_64[1];
        if s.word_300 == support_index {
            let mut flat = sub(s.vectors_224_to_272[3], desired);
            flat[1] = 0.;
            if dot(flat, flat) < f32::from_bits(0x3bd1b717) {
                desired = s.vectors_224_to_272[3];
            } else {
                let old = s.vectors_224_to_272[3];
                let distance = length(flat);
                desired = madd(flat, 0.08 / distance, desired);
                desired[1] = old[1];
                s.vectors_224_to_272[3] = desired;
            }
            s.hands[index].vectors_0_to_64[0] = desired;
        } else {
            if foot.flag_84 {
                desired[1] = foot_height(
                    s,
                    i,
                    foot.vectors_0_to_64[3][1],
                    desired[1] - foot.scalars_96_100[0].max(0.),
                );
            }
            let time = (foot.scalars_96_100[1] - f32::from_bits(0x3c888889)).max(0.);
            s.hands[index].scalars_96_100[1] = time;
            let weight = 1. - time * 10.;
            s.hands[index].vectors_0_to_64[0] =
                madd(desired, weight, scale(foot.vectors_0_to_64[0], 1. - weight));
        }
    }
    let targets = std::array::from_fn(|index| {
        let foot = &mut s.hands[index];
        let world = s.word_300 == index as u32;
        if world {
            foot.scalars_96_100[1] = 0.1;
        }
        Target {
            position: if world {
                foot.vectors_0_to_64[0]
            } else {
                transform_point(&i.inverse_root, foot.vectors_0_to_64[0])
            },
            world,
            blend: if world { 1. } else { 0.3 },
            normal: (foot.flags_104_to_107[1] && foot.flag_84).then_some(foot.vectors_0_to_64[2]),
        }
    });
    s.scalars_288_to_296[0] = i.root[3][1];
    s.scalars_288_to_296[2] = height_correction(s, i);
    targets
}
fn foot_height(s: &State, i: &Input, ground: f32, desired: f32) -> f32 {
    let base = ground + 0.02;
    let proposed = (desired - s.scalars_288_to_296[0]) + ground;
    let limited = proposed.min(base + 0.08).max(base);
    let output = desired.max(limited);
    let ceiling = i.velocity[1].mul_add(f32::from_bits(0x3c888889), i.position[1]) - 0.46;
    if output - desired > 0.4 || output > ceiling {
        desired
    } else {
        output
    }
}
fn clamp_normal(mut normal: V) -> V {
    if normal[1] >= 0.9 {
        return normal;
    }
    normal[1] = 0.;
    normal = scale(unit(normal, [0.; 4]), f32::from_bits(0x3e428f5f).sqrt());
    normal[1] = 0.9;
    normal
}
fn height_correction(s: &mut State, i: &Input) -> f32 {
    if s.hands.iter().all(|f| !f.flag_84) {
        return 0.;
    }
    let mut highest = f32::from_bits(0x501502f9);
    for foot in &mut s.hands {
        foot.scalars_96_100[0] = 0.;
        if !foot.flag_84 {
            continue;
        }
        if foot.flags_104_to_107[0] {
            let normal = foot.vectors_0_to_64[4];
            let side = i.effective_root[0];
            let normal = unit(sub(normal, scale(side, dot(side, normal))), [0.; 4]);
            foot.scalars_96_100[0] = dot(normal, i.effective_root[2]) * 0.2;
        }
        highest = highest.min(foot.vectors_0_to_64[3][1] + foot.scalars_96_100[0]);
    }
    s.scalars_288_to_296[0] - highest
}
///82BEDB78 normal setter, shared by both feet after placement.
pub fn set_normal(target: &mut crate::animation::foot_ik::external::ExternalTarget, normal: V) {
    if target.normal_blend > 0. {
        target.normal = limit_angle(normal, target.normal, 0.1);
    }
    target.normal_set = true;
    target.normal_blend = (target.normal_blend + 0.2).min(1.);
}
