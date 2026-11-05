//! Data-only audit: no Bevy, input polling, physics ticks or window.
fn main() -> Result<(), String> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("Expected owned assets directory")?;
    let collections = skate_data::collections::Collections::load(std::path::Path::new(&root))?;
    let scoring = skate_data::scoring::ScoringData::load(&collections)?;
    for id in [96, 128] {
        let d = scoring
            .by_id(id)
            .ok_or(format!("Missing required scorable {id}"))?;
        println!(
            "{}: {} points; label {}; delay {}",
            d.identifier, d.points, d.label, d.completion_delay
        );
    }
    println!(
        "{} authored definitions; repetition {:?}; combo levels {:?}",
        scoring.definitions.len(),
        scoring.repetition,
        scoring.combo_levels
    );
    Ok(())
}
