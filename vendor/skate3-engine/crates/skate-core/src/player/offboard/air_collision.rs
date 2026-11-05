//! BipedAir collision response82D2F748 and postphysics checks82D2FFF8.
use super::{
    air_launch::{V, dot, length, madd, scale, sub, unit},
    air_state::State,
};
use crate::player::wipeout::{self, Requests};
pub struct Relaunch {
    pub velocity: V,
    pub normal: V,
    pub horizontal_normal: V,
}
pub fn response(
    s: &mut State,
    displacements: [V; 2],
    up: V,
    can_requery: bool,
    requests: &mut Requests,
) -> Option<Relaunch> {
    let along = displacements.map(|v| dot(v, up).abs());
    let remainder = std::array::from_fn::<_, 2, _>(|i| {
        sub(displacements[i], scale(up, along[i]).map(f32::abs))
    });
    if along.into_iter().any(|v| v > 0.2) || remainder.into_iter().any(|v| length(v) > 0.2) {
        requests.request(32, 0.);
    }
    let flat = displacements.map(|mut v| {
        v[1] = 0.;
        v
    });
    let magnitudes = flat.map(length);
    if magnitudes.iter().all(|&v| v <= 0.01) {
        return None;
    }
    if s.directed {
        s.collision_bail = true;
        return None;
    }
    let selected = if magnitudes[0] > magnitudes[1] { 0 } else { 1 };
    let normal = unit(flat[selected], [0.; 4]);
    let horizontal = normal;
    if (magnitudes[0] > 0.01 || magnitudes[1] > 0.01) && dot(horizontal, s.packet.velocity) < -10. {
        s.collision_bail = true;
    }
    if dot(s.packet.velocity, normal) >= 0. {
        return None;
    }
    let above_landing = s.packet.impact_y > s.frame[3][1] && s.packet.normal[1] > 0.7;
    if !can_requery || above_landing {
        return None;
    }
    s.collision_adjusted = true;
    //82C1E170 removes only motion into the supplied plane (called twice).
    let first = remove_into(s.packet.velocity, scale(horizontal, -1.));
    let velocity = madd(normal, 1., remove_into(first, scale(normal, -1.)));
    Some(Relaunch {
        velocity,
        normal,
        horizontal_normal: horizontal,
    })
}
fn remove_into(v: V, n: V) -> V {
    let amount = dot(v, n);
    if amount > 0. {
        sub(v, scale(n, amount))
    } else {
        v
    }
}
pub struct Settings {
    pub minimum_speed: f32,
    pub maximum_displacement: f32,
    pub maximum_contact: f32,
    pub maximum_arm_contact: f32,
    pub maximum_squash: f32,
}
pub struct PostInput<'a> {
    pub shared: &'a wipeout::Frame,
    pub root_velocity: V,
    pub check_squash: bool,
    pub air_settings: &'a wipeout::AirSettings,
    pub elapsed: f32,
    pub flags_2484: u32,
    pub forward: V,
    pub right: V,
    pub right_stick: [f32; 2],
}
pub fn post(s: &State, requests: &mut Requests, settings: &Settings, i: &PostInput<'_>) {
    if i.flags_2484 & 2 != 0 {
        skeletal(
            requests,
            i.shared,
            i.check_squash,
            i.air_settings.max_squash,
            i.air_settings.max_displacement,
            i.air_settings.max_contact,
            i.air_settings.max_arm_contact,
        );
    }
    if !s.packet.contact && i.elapsed > 1.9 {
        requests.request(26, 0.);
    }
    if s.collision_bail {
        requests.request(30, 0.);
    }
    requests.mode = 4;
    if dot(i.root_velocity, i.root_velocity) > settings.minimum_speed * settings.minimum_speed {
        skeletal(
            requests,
            i.shared,
            i.check_squash,
            settings.maximum_squash,
            settings.maximum_displacement,
            settings.maximum_contact,
            settings.maximum_arm_contact,
        );
    }
    if i.flags_2484 & 0x400 != 0 && s.remaining > f32::from_bits(0x3c888889) {
        requests.request(31, 0.);
    }
    if s.reversed && s.packet.velocity[1] < 0. {
        requests.request(31, 0.);
    }
    if s.packet.contact
        && -dot(s.packet.normal, s.packet.impact_velocity) > f32::from_bits(0x41473333)
        && s.remaining < f32::from_bits(0x3e23d70a)
    {
        requests.request(33, 0.);
    }
    if s.remaining <= 0. && !s.directed {
        let forward = dot(i.forward, s.packet.impact_velocity);
        if forward > 20. || dot(i.right, s.packet.impact_velocity) > 4. || forward < -2. {
            requests.request(31, 0.);
        }
    }
    if s.remaining < 0. && s.airborne {
        requests.request(31, 0.);
    }
    if s.remaining > 4. && i.right_stick.into_iter().any(|v| v.abs() > 0.01) {
        requests.request(26, 0.);
    }
}
fn skeletal(
    requests: &mut Requests,
    f: &wipeout::Frame,
    squash: bool,
    max_squash: f32,
    displacement: f32,
    body: f32,
    arms: f32,
) {
    if squash && f.maximum_pose_error > max_squash {
        requests.request(18, 0.);
    } else if dot(f.pose_error, f.pose_error) > displacement * displacement {
        requests.request(1, 0.);
    } else if wipeout::regional_force(f, body, arms) {
        requests.request(0, 0.);
    }
}
