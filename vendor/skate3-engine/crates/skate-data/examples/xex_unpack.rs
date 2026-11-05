//! Unpacks a retail XEX2 to its mapped base image and locates the ocean PCA table.
//! Usage: cargo run -p skate-data --example xex_unpack -- <default.xex> [<image.bin>]
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let xex = std::fs::read(&args[0]).expect("read XEX");
    let started = std::time::Instant::now();
    let image = skate_data::xex::XexImage::parse(&xex).unwrap_or_else(|e| panic!("{e}"));
    println!(
        "base 0x{:08X}, image {} bytes, {:.2} s",
        image.base_address,
        image.image.len(),
        started.elapsed().as_secs_f64()
    );
    match skate_data::ocean_pca::extract(&image) {
        Ok(pca) => println!(
            "ocean PCA table at 0x{:08X}, frame 0 mean {:?}, {:.2} s",
            pca.means_address,
            pca.frames[0][0],
            started.elapsed().as_secs_f64()
        ),
        Err(e) => println!("ocean PCA: {e}"),
    }
    if let Some(out) = args.get(1) {
        std::fs::write(out, &image.image).expect("write image");
    }
}
