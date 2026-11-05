//! GestureTrickMapping82BA07F0 and CreateTrickIntentFromGesture82BA1C30..2228.
use super::{gesture_catalog::Group, gesture_mapping_data::TABLES};
use skate3_core::{animation::playback_parameters::intent_key, graph::intents::IntentMap};

pub(crate) fn select(group: Group, intents: &IntentMap, mirrored: bool) -> Option<&'static str> {
    let rows = TABLES[group as usize];
    // 82BC0D90 uses greatest prime <=46, then maximum load10000.
    // 82BC1BE8 prepends entries; 82BA07F0 visits ascending buckets.
    let mut selected = None;
    let mut bucket = usize::MAX;
    for row in rows.iter().rev() {
        let current = intent_key(row.0)
            .iter()
            .fold(0u32, |sum, word| sum.wrapping_add(*word)) as usize
            % 43;
        if current < bucket && intents.contains_key(row.0) {
            selected = Some(row);
            bucket = current;
        }
    }
    let row = selected?;
    let mirrored = mirrored && !intents.contains_key("DontMirrorTrick");
    Some(if intents.contains_key("DarkCatch") && row.3 != 0 {
        match (row.3, mirrored) {
            (2, false) | (1, true) => "Heelflip",
            (3, true) | (4, false) => "N_Heelflip",
            (3, false) | (4, true) => "N_Kickflip",
            _ => "Kickflip",
        }
    } else if mirrored {
        row.2
    } else {
        row.1
    })
}

#[derive(Clone, Debug)]
pub(crate) struct State {
    first_update: bool,
    selected: Option<String>,
    successor: Option<&'static str>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            first_update: true,
            selected: None,
            successor: None,
        }
    }
}
impl State {
    pub fn begin(
        &mut self,
        group: Group,
        override_name: Option<&str>,
        ag: &IntentMap,
        mg: &mut IntentMap,
        mirrored: bool,
    ) {
        let Some(name) = select(group, ag, mirrored) else {
            return;
        };
        let mut name = override_name.unwrap_or(name).to_owned();
        if ag.contains_key("Underflip") {
            name = format!("U_{name}");
        }
        self.successor = match name.as_str() {
            "Kickflip" => Some("KickflipHold"),
            "Heelflip" => Some("HeelflipHold"),
            "N_Kickflip" => Some("N_KickflipHold"),
            "N_Heelflip" => Some("N_HeelflipHold"),
            _ => None,
        };
        self.selected = Some(name.clone());
        mg.insert(&name, 0.0);
    }
    pub fn update(&mut self, mg: &mut IntentMap) {
        if !self.first_update {
            if let (Some(name), Some(successor)) = (&self.selected, self.successor) {
                mg.remove(name);
                mg.insert(successor, 0.0);
            }
        }
        self.first_update = false;
    }
    pub fn end(&mut self, mg: &mut IntentMap) {
        mg.remove("DarkCatch");
        if let Some(name) = &self.selected {
            if let Some(successor) = self.successor {
                mg.remove(successor);
            }
            mg.remove(name);
        }
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn tre_flip_uses_original_mirror_mapping_and_presence_not_strength() {
        let mut ag = IntentMap::new();
        ag.insert("360Flip", 0.0);
        assert_eq!(select(Group::Square, &ag, false), Some("360Flip"));
        assert_eq!(select(Group::Square, &ag, true), Some("Laserflip"));
        ag.insert("DontMirrorTrick", 0.0);
        assert_eq!(select(Group::Square, &ag, true), Some("360Flip"));
    }
    #[test]
    fn first_update_retains_launch_intent_then_successor_is_cleaned_on_exit() {
        let mut ag = IntentMap::new();
        ag.insert("Kickflip", 1.0);
        let mut mg = IntentMap::new();
        let mut state = State::default();
        state.begin(Group::Square, None, &ag, &mut mg, false);
        state.update(&mut mg);
        assert!(mg.contains_key("Kickflip"));
        state.update(&mut mg);
        assert!(!mg.contains_key("Kickflip"));
        assert!(mg.contains_key("KickflipHold"));
        state.end(&mut mg);
        assert!(mg.is_empty());
    }
}
