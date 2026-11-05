//! Native mod extension: resolve Skate 3 TU3's names into the existing Skate 2
//! scoring/HUD pipeline. Shared names keep their original Skate 2 identities.
use super::{
    scorable::TrickSettings,
    scoring_catalog::{SCORABLES, ScorableInfo},
};
use skate_data::scoring::ScoringData;
use skate3_core::scoring::{catalog::IDENTIFIERS, conversions::LINKS};
use std::sync::OnceLock;

pub(super) const CAPACITY: usize = SCORABLES.len() + IDENTIFIERS.len();
static DATA: OnceLock<ScoringData> = OnceLock::new();

pub(super) fn data() -> Result<&'static ScoringData, String> {
    crate::skate3_native::physics::player::linked_once(&DATA, || {
        let data=crate::skate3_native::physics::player::stock_collections()?;
        ScoringData::load(&data)
    })
}

fn host_id(source_id: i32) -> i32 {
    let Some(&(name, _, _)) = usize::try_from(source_id)
        .ok()
        .and_then(|id| IDENTIFIERS.get(id))
    else {
        return -1;
    };
    SCORABLES
        .iter()
        .find(|entry| entry.name == name)
        .map_or(SCORABLES.len() as i32 + source_id, |entry| entry.id)
}

pub(super) fn entries() -> &'static [ScorableInfo] {
    static ENTRIES: OnceLock<Vec<ScorableInfo>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        IDENTIFIERS
            .iter()
            .enumerate()
            .map(|(source_id, &(name, category, trick_type))| {
                let (consumed, base) = LINKS[source_id];
                ScorableInfo {
                    id: host_id(source_id as i32),
                    consumed: host_id(consumed),
                    base: host_id(base),
                    category: category as i32,
                    trick_type: trick_type as i32,
                    name,
                }
            })
            .collect()
    })
}

pub(super) fn info(id: usize) -> Option<&'static ScorableInfo> {
    let entry = entries().get(id.checked_sub(SCORABLES.len())?)?;
    (entry.id == id as i32).then_some(entry)
}

pub(super) fn animation_id(name: &crate::play_animation::FastString36) -> Option<i32> {
    static IDS: OnceLock<std::collections::BTreeMap<crate::play_animation::FastString36, i32>> =
        OnceLock::new();
    IDS.get_or_init(|| {
        entries()
            .iter()
            .map(|entry| {
                (
                    crate::play_animation::FastString36::encode(entry.name)
                        .expect("TU3 catalog name"),
                    entry.id,
                )
            })
            .collect()
    })
    .get(name)
    .copied()
}

pub(super) fn lookup(name: &str, index: u32, settings: &mut TrickSettings) -> Result<bool, String> {
    let Some(definition) = data()?
        .definitions
        .iter()
        .find(|definition| definition.identifier.eq_ignore_ascii_case(name))
    else {
        return Ok(false);
    };
    if definition.trick_type >= 14 {
        return Err(format!("Invalid Skate 3 trick type: {name}"));
    }
    *settings = TrickSettings::new();
    settings.is_valid = true;
    settings.trick_index = index;
    settings.scorable_id = host_id(definition.metadata.id as i32);
    settings.base_score = definition.points;
    settings.trick_type = definition.trick_type as i32;
    settings.min_trick_time = definition.completion_delay;
    settings.trick_name_hal = Some(definition.label.as_str().into());
    settings.fakie_variant_id = host_id(definition.variant);
    settings.trick_attrib_key = skate2_formats::attrib::string_hash64(definition.identifier);
    Ok(true)
}

/// Additional original labels only; the original Skate 2 strings win on overlap.
pub(crate) fn localized_tricks() -> Result<Vec<(u32, Vec<u16>)>, String> {
    use skate2_formats::localization::{Histogram, LanguageCategory, string_hash};
    let root = crate::skate3_native::physics::player::assets_root()?;
    let path = root.join("private/stock/data/fe/languages/english/LANGUAGE_English_");
    let histogram = skate_data::asset_io::read(&std::path::PathBuf::from(format!(
        "{}HISTOGRAM_skate3ng.BIN",
        path.display()
    )))
    .map_err(|error| error.to_string())?;
    let category = skate_data::asset_io::read(&std::path::PathBuf::from(format!(
        "{}Global_skate3ng.BIN",
        path.display()
    )))
    .map_err(|error| error.to_string())?;
    let histogram = Histogram::parse(&histogram).map_err(|error| error.to_string())?;
    let category = LanguageCategory::parse(&category).map_err(|error| error.to_string())?;
    let mut result = Vec::new();
    for definition in &data()?.definitions {
        let hash = string_hash(definition.label.as_bytes());
        let packed = match category.packed_string(hash) {
            Some(packed) => packed,
            // TU3's unused slidelip label is absent in its English table. The
            // shared Skate 2 trick keeps its existing authored label/settings.
            None if SCORABLES
                .iter()
                .any(|entry| entry.name == definition.identifier) =>
            {
                continue;
            }
            None => return Err(format!("Missing Skate 3 trick label {}", definition.label)),
        };
        let text = histogram
            .unpack(packed, 1024, false)
            .map_err(|error| error.to_string())?;
        if !text.valid || !text.terminated {
            return Err(format!("Invalid Skate 3 trick label {}", definition.label));
        }
        result.push((hash, text.units));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires original Skate 2 Vault; Skate 3 metadata remains embedded"]
    fn skate3_darkcatch_scores_and_banks_in_the_original_skate2_pipeline() {
        use super::super::{
            scorable::Scorable,
            score_holder::ScoreHolder,
            scoring_database::{TRICK_CLASS, TrickDatabase},
            sequence::GlobalMetrics,
        };
        use std::sync::{Arc, Mutex};
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mut mounts = skate2_formats::mount::MountSet::new();
        mounts.push(
            skate2_formats::mount::MountedBig::open(root.join("data/db/db.big"), false).unwrap(),
        );
        // Use this installed build's actual Vault rather than a helper pinned
        // to a different retail build's complete file size/signature.
        let vault = crate::attribute_runtime::AttributeRuntimeVault::try_load_mounted(
            &mounts,
            "data\\db\\skater.vlt",
            &[TRICK_CLASS, 0x3492_15e2_e817_703c],
        )
        .unwrap();
        let db = TrickDatabase::load(&vault).unwrap();
        let definition = data()
            .unwrap()
            .definitions
            .iter()
            .find(|entry| entry.identifier == "kickflip_darkcatch")
            .unwrap();
        let id = host_id(definition.metadata.id as i32);
        assert!(id >= SCORABLES.len() as i32);
        let mut settings = TrickSettings::new();
        db.lookup_id(id, &mut settings).unwrap();
        assert!(settings.is_valid);
        assert_eq!(settings.base_score, definition.points);
        let mut by_index = TrickSettings::new();
        db.lookup_index(settings.trick_index as i32, &mut by_index)
            .unwrap();
        assert_eq!(by_index, settings);
        let occurrence = Arc::new(());
        let metrics = Arc::new(Mutex::new(GlobalMetrics::new(Arc::clone(&occurrence))));
        metrics.lock().unwrap().tick = 1000;
        let mut holder = ScoreHolder::before_sequence(occurrence, &vault);
        holder.construct_sequence(Some(metrics));
        holder.finish_constructor().unwrap();
        let mut trick = Scorable::new();
        trick
            .init(
                &db,
                &holder.general,
                definition.identifier,
                1.,
                false,
                false,
                0,
                0.,
            )
            .unwrap();
        assert!(holder.attempt_beginning(&mut trick).unwrap());
        assert_eq!(trick.score, definition.points as f32);
        holder.announce_ending(&db, &mut trick).unwrap();
        holder.end_trick(id, trick.score);
        let mut output = super::super::super::scoring::Scoring2::new();
        holder.fill_output(&mut output).unwrap();
        assert_eq!(output.tricks_begun.num_entries, 1);
        assert_eq!(output.tricks_in_current_sequence[id as usize], 1);
        assert_eq!(
            output.tricks_in_current_sequence[definition.metadata.id], 0,
            "Skate 3's numeric ID aliased another Skate 2 trick"
        );
        holder.end_sequence(trick.score, false, &mut output);
        assert_eq!(output.finished_sequence_score, definition.points as u32);
        assert_eq!(output.tricks_in_completed_sequence[id as usize], 1);
        assert_eq!(holder.tricks_in_sequence[id as usize], 0);
    }

    #[test]
    fn skate3_extra_tricks_keep_authored_scores_variants_and_labels() {
        let strings = localized_tricks().unwrap();
        let data = data().unwrap();
        let mut extra = 0;
        for definition in &data.definitions {
            let id = host_id(definition.metadata.id as i32);
            if id < SCORABLES.len() as i32 {
                continue;
            }
            extra += 1;
            let info = info(id as usize).unwrap();
            assert_eq!(info.name, definition.identifier);
            let encoded = crate::play_animation::FastString36::encode(info.name).unwrap();
            assert_eq!(animation_id(&encoded), Some(id));
            let mut settings = TrickSettings::new();
            assert!(lookup(info.name, id as u32, &mut settings).unwrap());
            assert_eq!(settings.base_score, definition.points);
            assert_eq!(settings.fakie_variant_id, host_id(definition.variant));
            assert!(strings.iter().any(|(hash, text)| *hash
                == skate2_formats::localization::string_hash(definition.label.as_bytes())
                && !text.is_empty()));
            assert!(info.id < CAPACITY as i32);
        }
        assert!(extra > 0, "no Skate 3-only trick metadata admitted");
        assert_eq!(host_id(0), 0);
        assert!(info(SCORABLES.len()).is_none());
    }
}

/// AudioPresentation::FillSkaterData (TU3 827A1F80..2014): AudioTrickType, AudioTrickFinishType
/// and AudioIsTrickWithScrape of the scorable's linked Skate 3 record. A name without a record
/// keeps the invalid audio trick, as the failed FindCollection does.
pub(super) fn audio(id: i32) -> Option<(i32, i32, bool)> {
    const SCORABLE: &str = "Hash_6918469984A8C596";
    static AUDIO: OnceLock<Vec<(i32, i32, bool)>> = OnceLock::new();
    let table = crate::skate3_native::physics::player::linked_once(&AUDIO, || {
        let data = crate::skate3_native::physics::player::stock_collections()?;
        Ok((0..CAPACITY as i32)
            .map(|id| {
                super::scoring_catalog::info(id)
                    .and_then(|entry| {
                        let word = |field| data.words::<1>(SCORABLE, entry.name, field).ok().map(|w| w[0] as i32);
                        Some((
                            word("AudioTrickType")?,
                            word("AudioTrickFinishType")?,
                            data.boolean(SCORABLE, entry.name, "AudioIsTrickWithScrape").ok()?,
                        ))
                    })
                    .unwrap_or((-1, -1, false))
            })
            .collect())
    })
    .ok()?;
    usize::try_from(id).ok().and_then(|id| table.get(id)).copied()
}

#[cfg(test)]
mod audio_tests {
    /// `SKATE3_DATA` names a Skate 3 folder: its Skate 3-only darkcatches carry real audio types.
    #[test]
    fn linked_darkcatches_have_audio_trick_types() {
        let Some(root) = std::env::var_os("SKATE3_DATA") else { return };
        let data = crate::skate3::Skate3Data::open(std::path::Path::new(&root)).unwrap();
        crate::skate3::set_linked(Some(std::sync::Arc::new(data)));
        let mut found = 0;
        for entry in super::entries() {
            if !entry.name.contains("darkcatch") {
                continue;
            }
            let audio = super::audio(entry.id).expect("linked audio table");
            eprintln!("{} id={} audio={audio:?}", entry.name, entry.id);
            assert!(audio.0 >= 0, "{} has no AudioTrickType", entry.name);
            found += 1;
        }
        assert!(found > 0);
    }
}
