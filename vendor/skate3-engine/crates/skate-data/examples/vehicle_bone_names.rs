//! Export the exact native order for an authored vehicle animation package.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: vehicle_bone_names OnBoard.abin bone-names.json".into());
    }
    let bank = skate_data::abin::Bank::load(std::path::Path::new(&args[0]))?;
    let hierarchy = bank.hierarchy().ok_or("Bank has no skeleton hierarchy")?;
    std::fs::write(&args[1], serde_json::to_vec_pretty(&hierarchy.bone_names)?)?;
    println!("Exported {} bone names", hierarchy.bone_names.len());
    Ok(())
}
