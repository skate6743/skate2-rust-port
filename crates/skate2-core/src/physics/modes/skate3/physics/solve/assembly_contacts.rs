//! Original single-player SkaterSkaterCollisionPipeline8276556C.
//! Board construction precedes Skeleton construction in82DB18A0. Preserve
//! that assembly order: board self (all culled), board/rider, rider self.
use super::super::collision_groups::pair_allowed_online;
use skate3_core::physics::{
    board_step::{BoardCollision, CollisionBody},
    board_world::BoardWorldVolume,
    contact::{RetailContactInput, combine_contact_materials},
    skeleton_body::SkeletonCollisionMode,
    world_contact::{PrimitivePairSettings, primitive_pair_contacts},
};

pub(super) fn append(
    contacts: &mut Vec<BoardCollision>,
    board: &[BoardWorldVolume],
    rider: &[BoardWorldVolume],
    board_group: u32,
    collision: &SkeletonCollisionMode,
    online: bool,
) -> Result<(), String> {
    let group_pair_allowed = |a, b| pair_allowed_online(a, b, online);
    //8277AF90 first culls the assembly pair;8277A6C0 then culls each
    //primitive pair using its part group, not Volume84's world-query group.
    if board_group >= 21
        || collision.assembly_group >= 21
        || collision.parts.iter().any(|part| part.part_group >= 21)
    {
        return Err("Skater collision group is outside the original21x21 table".into());
    }
    if group_pair_allowed(board_group, collision.assembly_group) {
        for a in board {
            for b in rider {
                let CollisionBody::Attached(part) = b.body else {
                    unreachable!()
                };
                //82DC3BD0/82DC4260 copy Part92 into VolumeData212
                //(assembly96 + volume212 = assembly308). Skate2's
                //82E0B690 names this producer part.m_userData. Biped assigns
                //hands5, legs17, torso18 and controller capsules20.
                if !group_pair_allowed(board_group, collision.parts[part].part_group) {
                    continue;
                }
                let before = contacts.len();
                append_pair(contacts, a, b);
                if contacts.len() > before && trace_rider_contacts() {
                    eprintln!("skate3 board-rider contact: part={part} group={} contacts={}", collision.parts[part].part_group, contacts.len() - before);
                }
            }
        }
    }
    //8277AE80 suppresses the reversed assembly pair. The same-assembly
    //branch8277B2C0 instead visits both directed primitive pairs and uses
    //the skeleton's part bitmap. Board82C0B560 culls all7x7 self pairs.
    for a in rider {
        let CollisionBody::Attached(part_a) = a.body else {
            unreachable!()
        };
        for b in rider {
            let CollisionBody::Attached(part_b) = b.body else {
                unreachable!()
            };
            if part_a != part_b && !collision.self_culling[part_a][part_b] {
                append_pair(contacts, a, b);
            }
        }
    }
    Ok(())
}

/// SKATE2_TRACE_SKATE3_RIDER_CONTACTS: each tick's board/skeleton contacts by skeleton part.
fn trace_rider_contacts() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_RIDER_CONTACTS").is_some())
}


#[cfg(any())]
#[path = "assembly_contacts_tests.rs"]
mod tests;

pub(super) fn append_pair(contacts: &mut Vec<BoardCollision>, a: &BoardWorldVolume, b: &BoardWorldVolume) {
    //8277B0F0 and8277B2C0 enter the same8277A508 primitive walker with
    //identical padding/triangle settings. Keep A/B orientation and point order.
    let Some(manifold) = primitive_pair_contacts(
        a.primitive,
        b.primitive,
        PrimitivePairSettings::skater_self_collision(),
    ) else {
        return;
    };
    let material = combine_contact_materials(a.material, b.material);
    for points in &manifold.points[..manifold.count] {
        contacts.push(BoardCollision {
            body_a: a.body,
            body_b: b.body,
            contact: RetailContactInput {
                position_on_a: points.a,
                position_on_b: points.b,
                normal: manifold.normal,
                restitution: material.restitution,
                static_friction: material.static_friction,
                dynamic_friction: material.dynamic_friction,
                //8277A7B4/B8 packs original Volume88 tags, both zero for
                //stock skater/board constructors (not the part indices).
                tag: 0,
            },
        });
    }
}
