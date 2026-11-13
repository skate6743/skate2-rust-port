//! VLT fields identified through Scorable82DA2340 and ScoreHolder82DA5D18.
//! No fallback point values: a missing/invalid authored field is an error.
use crate::collections::Collections;
use skate_core::{
    animation::{output::attributes::AttributeName, skeleton_input::name::encode},
    point_graph::PointGraph,
    scoring::{Scorable, catalog::IDENTIFIERS},
};

const SCORABLE: &str = "Hash_6918469984A8C596";
const TUNING: &str = "Hash_349215E2E817703C";

fn scorable_keys(data: &Collections) -> std::collections::BTreeSet<String> {
    // Setup's name dictionary may resolve the class as `scoring_trick`.
    // Presence checks must use the same native identities as Collections::field.
    data.entries()
        .iter()
        .filter(|row| crate::attrib_hash::numeric_name(&row.class_name) == SCORABLE)
        .map(|row| crate::attrib_hash::numeric_name(&row.key))
        .collect()
}

#[derive(Clone, Debug)]
pub struct Definition {
    pub metadata: Scorable,
    pub identifier: &'static str,
    pub encoded_name: AttributeName,
    pub points: i32,
    pub label: String,
    pub trick_type: u32,
    pub completion_delay: f32,
    pub variant: i32,
    pub flags: u32,
    /// `IsGrindGrab` (layout byte 148): GrindCollector::Update2_TrickAnalysis 82DA9E34 scores a
    /// grind as the animation's trick when that trick is a grind grab.
    pub is_grind_grab: bool,
}

#[derive(Clone, Debug)]
pub struct ScoringData {
    pub collector: CollectorTuning,
    pub definitions: Vec<Definition>,
    pub repetition: PointGraph<8>,
    pub announcement: PointGraph<8>,
    pub line_drain: f32,
    pub line_capacity: f32,
    pub combo_drain: f32,
    pub combo_capacity: f32,
    /// Ascending point thresholds and their authored multiplier.
    pub combo_levels: [(f32, f32); 3],
    pub combo_refresh_threshold: f32,
    pub unannounced_factor: f32,
    pub bail_factor: f32,
    pub sketchy_side_speed: f32,
}

impl ScoringData {
    pub fn session_rules(&self) -> skate_core::scoring::session::Rules {
        skate_core::scoring::session::Rules {
            combo_capacity: self.combo_capacity,
            combo_levels: self.combo_levels,
            combo_refresh_threshold: self.combo_refresh_threshold,
            line_capacity: self.line_capacity,
            bail_factor: self.bail_factor,
        }
    }
    pub fn load(data: &Collections) -> Result<Self, String> {
        let mut definitions = Vec::new();
        let available = scorable_keys(data);
        for (id, &(identifier, class, score_type)) in IDENTIFIERS.iter().enumerate() {
            // The executable enum includes unused entries without VLT records.
            // Only actual records are eligible for recognition.
            if !available.contains(&crate::attrib_hash::numeric_name(identifier)) {
                continue;
            }
            let words = |name| data.words::<1>(SCORABLE, identifier, name).map(|v| v[0]);
            let actual_id = words("Hash_B2383F16252DFE8E")?;
            if actual_id != id as u32 {
                return Err(format!(
                    "Scorable {identifier}: enum {id} disagrees with VLT {actual_id}"
                ));
            }
            let label = data.field(SCORABLE, identifier, "Hash_843613E915014627")?;
            if label.type_name != "EA::Reflection::Text" {
                return Err(format!("Invalid scorable label {identifier}"));
            }
            definitions.push(Definition {
                metadata: Scorable {
                    id,
                    class,
                    score_type,
                },
                identifier,
                encoded_name: encode(identifier.as_bytes()),
                points: data.integer(SCORABLE, identifier, "Hash_937F62AE5C1ED284")? as i32,
                label: label.data.clone(),
                trick_type: words("TrickType")?,
                completion_delay: data.float(SCORABLE, identifier, "Hash_DC7F402E5680BA02")?,
                variant: words("Hash_2E90BC04042A0B5A")? as i32,
                flags: words("Hash_D34A84B044B60CE3")?,
                is_grind_grab: data.boolean(SCORABLE, identifier, "IsGrindGrab")?,
            });
        }
        if definitions.is_empty() {
            return Err("No native scorable definitions in owned VLT data".into());
        }
        let graph = |field| -> Result<PointGraph<8>, String> {
            let w = data.words::<20>(TUNING, "default", field)?;
            let x = std::array::from_fn(|i| f32::from_bits(w[4 + i]));
            let y = std::array::from_fn(|i| f32::from_bits(w[12 + i]));
            if x.iter().chain(y.iter()).any(|v| !v.is_finite()) || x.windows(2).any(|v| v[0] > v[1])
            {
                return Err(format!("Invalid native scoring curve {field}"));
            }
            Ok(PointGraph { x, y })
        };
        let f = |field| data.float(TUNING, "default", field);
        Ok(Self {
            collector: CollectorTuning::load(data)?,
            definitions,
            repetition: graph("Hash_59D91EAABF033A24")?,
            announcement: graph("Hash_263B277F8CA17126")?,
            line_drain: f("Hash_FE02A45231F7B06A")?,
            line_capacity: f("Hash_57478337ACCFFF18")?,
            combo_drain: f("Hash_88407506DC9780ED")?,
            combo_capacity: f("Hash_28767A5F961C7129")?,
            combo_levels: [
                (f("Hash_829887D8C24A5B8A")?, f("Hash_4752056364FF91FE")?),
                (f("Hash_FB8408A15C17A9D4")?, f("Hash_F4993A13ED7B44C1")?),
                (f("Hash_30AAE071A7868771")?, f("Hash_DC0C3853F6C5FC2E")?),
            ],
            combo_refresh_threshold: f("Hash_E36197BFC8EA1CAD")?,
            unannounced_factor: f("Hash_2577DF0FCE5251E4")?,
            bail_factor: f("Hash_B0B56FF046508506")?,
            sketchy_side_speed: f("Hash_6DC5982591F80A4C")?,
        })
    }
    pub fn by_name(&self, name: AttributeName) -> Option<&Definition> {
        self.definitions.iter().find(|d| d.encoded_name == name)
    }
    pub fn by_id(&self, id: usize) -> Option<&Definition> {
        self.definitions.iter().find(|d| d.metadata.id == id)
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn scoring_presence_accepts_readable_and_numeric_vault_names() {
        let key = IDENTIFIERS[0].0;
        let numeric_key = crate::attrib_hash::numeric_name(key);
        for class in ["scoring_trick", SCORABLE, "0x6918469984a8c596"] {
            for name in [key, numeric_key.as_str()] {
                let data: Collections = serde_json::from_value(serde_json::json!({
                    "version": 1,
                    "collections": [{"class": class, "key": name, "parent": "",
                        "source": "fixture", "sha256": "", "fields": {}},
                        {"class": "unrelated", "key": "other", "parent": "",
                        "source": "fixture", "sha256": "", "fields": {}}]
                }))
                .unwrap();
                assert_eq!(scorable_keys(&data), [numeric_key.clone()].into());
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct CollectorTuning {
    scalars: std::collections::BTreeMap<u16, f32>,
    curves: std::collections::BTreeMap<u16, PointGraph<8>>,
}
impl CollectorTuning {
    fn load(data: &Collections) -> Result<Self, String> {
        let mut out = Self {
            scalars: Default::default(),
            curves: Default::default(),
        };
        for &(offset, field, size) in crate::scoring_fields::FIELDS {
            if size == 4 {
                out.scalars.insert(
                    offset,
                    data.float("Hash_546C36B656038E04", "default", field)?,
                );
            } else {
                let w = data.words::<20>("Hash_546C36B656038E04", "default", field)?;
                out.curves.insert(
                    offset,
                    PointGraph {
                        x: std::array::from_fn(|i| f32::from_bits(w[4 + i])),
                        y: std::array::from_fn(|i| f32::from_bits(w[12 + i])),
                    },
                );
            }
        }
        Ok(out)
    }
    pub fn scalar(&self, native_offset: u16) -> f32 {
        self.scalars[&native_offset]
    }
    /// Checked lookup for a host collector whose schema may have different fields.
    pub fn try_curve(&self, native_offset: u16, input: f32) -> Option<f32> {
        self.curves.get(&native_offset).map(|graph| graph.evaluate(input))
    }
    pub fn curve(&self, native_offset: u16, input: f32) -> f32 {
        self.curves[&native_offset].evaluate(input)
    }
}
