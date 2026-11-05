fn main() {
    let mut failed = false;
    let mut render_only = false;
    for path in std::env::args_os().skip(1) {
        if path == "--render-only" {
            render_only = true;
            continue;
        }
        let result = if render_only {
            std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| skate_data::skate_map::SkateMap::parse_render_only(&bytes))
        } else {
            skate_data::skate_map::SkateMap::load(std::path::Path::new(&path))
        };
        match result {
            Ok(m) => {
                println!(
                    "{} v{}: vertices={} triangles={} collision={} textures={} rails={} doors={} lights={} routes={} spawn={:?} heading={}",
                    m.name,
                    m.version,
                    m.geometry.vertices.len(),
                    m.geometry.indices.len() / 3,
                    m.geometry.collision.len(),
                    m.textures.len(),
                    m.rails.len(),
                    m.doors.len(),
                    m.lights.len(),
                    m.routes.len(),
                    m.spawn,
                    m.heading
                );
                for e in &m.extensions {
                    println!(
                        "extension {} schema={} decoded_bytes={}",
                        String::from_utf8_lossy(&e.tag),
                        e.schema,
                        e.payload.len()
                    );
                }
                let mut angles = [0_usize; 32];
                let mut native = 0;
                for t in &m.geometry.collision {
                    if let Some(edges) = t.native_edges {
                        native += 1;
                        for e in edges {
                            angles[usize::from(e & 31)] += 1;
                        }
                    }
                }
                println!("native_collision_triangles={native} angle_histogram={angles:?}");
            }
            Err(e) => {
                eprintln!("{e}");
                failed = true;
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
}
