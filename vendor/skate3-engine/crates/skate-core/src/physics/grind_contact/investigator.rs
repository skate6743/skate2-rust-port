//! Family/proximity arbitration from S3 82D875A8.
//! Geometry refinement and balance/engagement run afterward in the manager.
use super::{
    admission::Admission,
    families::{self, Contact},
    *,
};

#[derive(Clone, Copy, Debug)]
pub struct Query<'a> {
    pub board: [V; 4],
    pub admission: Admission<'a>,
    pub flags_2468: u32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2484: u32,
    pub tip_state: u32,
    pub truck_to_wheel: f32,
    pub deck_to_truck: f32,
    pub test_above: f32,
    pub test_below: f32,
    pub translation: f32,
    pub stability_nudge: f32,
    pub balance: f32,
    pub reference_right: V,
    pub ground_frames: u32,
    pub low_wheel_frames: u32,
    /// Grind permission OR retained70-frame exclusion while published kind!=2.
    pub forbidden: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Proximity {
    pub contact: TruckContact,
    /// Investigator412 bit26: deck/inverted rather than truck contact.
    pub deck_contact: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Investigation {
    pub candidate: Option<Contact>,
    /// Only the fallback path sets412 bit27. It does not authorize a grind.
    pub proximity: Option<Proximity>,
}

/// `edges` is already the native bounded query result, in its selection order.
pub fn investigate(query: &Query<'_>, edges: &[Primitive]) -> Investigation {
    if edges.is_empty() {
        return Investigation::default();
    }
    let q = query;
    let a = &q.admission;
    let trucks = truck_contacts(
        q.board,
        q.flags_2484,
        q.truck_to_wheel,
        q.deck_to_truck,
        edges,
    );
    let tips = families::tip_contacts(
        q.board,
        q.flags_2484,
        q.tip_state,
        q.flags_2468,
        q.deck_to_truck,
        edges,
    );
    let deck = deck_contact(
        q.board,
        q.flags_2484,
        q.test_above,
        q.test_below,
        q.deck_to_truck,
        edges,
    );
    let inverted =
        families::inverted_contact(q.board, q.flags_2484, q.test_above, q.test_below, edges);

    if !q.forbidden {
        let fifty = || {
            let geometry = fifty_fifty_candidate_on_splines(
                q.board,
                [q.translation, q.stability_nudge],
                trucks,
                edges,
            )?;
            let decision = a.test(401, geometry.direction, false);
            decision.allowed.then_some(Contact {
                geometry,
                front: false,
                kind: 0,
                entry_kind: decision.kind,
            })
        };
        let five = || {
            families::five_o(
                q.board,
                trucks,
                edges,
                a.velocity,
                q.flags_2476,
                q.flags_2472,
                q.ground_frames,
                q.translation,
                a,
            )
        };
        let tip = || {
            families::tipslide(
                q.board,
                tips,
                edges,
                a.velocity,
                a.category,
                q.flags_2476,
                q.flags_2472,
                q.ground_frames,
                q.balance,
                q.reference_right,
                a,
            )
        };
        let slide = || {
            let hit = deck?;
            let geometry = boardslide_candidate(
                q.board,
                hit,
                edges[hit.primitive],
                a.velocity,
                a.category,
                q.low_wheel_frames as i32,
                q.flags_2476,
                q.deck_to_truck,
                q.truck_to_wheel,
                a,
            )?;
            Some(Contact {
                geometry,
                front: false,
                kind: 1,
                entry_kind: a.test(400, geometry.direction, false).kind,
            })
        };
        let dark = || {
            let hit = inverted?;
            families::darkslide(
                q.board,
                hit,
                edges[hit.primitive],
                a.velocity,
                a.category,
                q.low_wheel_frames,
                a,
            )
        };
        // The original retries the truck family as 50-50 first, even in403.
        let retained = match a.state {
            401 | 403 => fifty().or_else(five),
            402 | 404 => tip(),
            400 => slide(),
            405 => dark(),
            _ => None,
        };
        if let Some(candidate) = retained
            .or_else(fifty)
            .or_else(five)
            .or_else(tip)
            .or_else(slide)
            .or_else(dark)
        {
            return Investigation {
                candidate: Some(candidate),
                proximity: None,
            };
        }
    }
    //82D87C5C..82D87E60: keep contact-only metadata even when entry is denied.
    let proximity = fallback(trucks, deck, inverted, tips);
    Investigation {
        candidate: None,
        proximity,
    }
}

fn fallback(
    trucks: [Option<TruckContact>; 2],
    deck: Option<TruckContact>,
    inverted: Option<TruckContact>,
    tips: [Option<TruckContact>; 2],
) -> Option<Proximity> {
    //82D87D24 short-circuits: v82 becomes1 only on an actual deck hit.
    trucks[0]
        .or(trucks[1])
        .map(|contact| Proximity {
            contact,
            deck_contact: false,
        })
        .or_else(|| {
            deck.map(|contact| Proximity {
                contact,
                deck_contact: true,
            })
        })
        .or_else(|| {
            inverted.or(tips[0]).or(tips[1]).map(|contact| Proximity {
                contact,
                deck_contact: false,
            })
        })
}

#[cfg(test)]
#[path = "investigator_tests.rs"]
mod tests;
