//! Data-only package timing tool. Does not initialize the engine or simulate.
use std::{path::Path, time::Instant};
fn main() {
    let compare = std::env::args().any(|arg| arg == "--compare");
    for path in std::env::args().skip(1).filter(|arg| arg != "--compare") {
        for pass in 0..if compare { 1 } else { 3 } {
            let start = Instant::now();
            let bytes = std::fs::read(Path::new(&path)).expect("read map");
            let read = start.elapsed();
            let start = Instant::now();
            let map = skate_data::skate_map::SkateMap::parse(&bytes).expect("decode map");
            println!(
                "DATA_DECODE name={:?} pass={} read_ms={} parse_ms={} textures={} vertices={}",
                map.name,
                pass,
                read.as_millis(),
                start.elapsed().as_millis(),
                map.textures.len(),
                map.geometry.vertices.len()
            );
            if compare {
                let serial = skate_data::skate_map::SkateMap::parse_with_decode_workers(&bytes, 1)
                    .expect("serial decode");
                assert!(map == serial, "parallel decoding changed package contents");
                println!("DATA_EQUAL name={:?} all_fields=true", map.name);
            }
        }
    }
}
