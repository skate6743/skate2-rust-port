//! Read-only map inventory; no window, renderer, physics initialization or conversion.
use skate_data::skate_map::SkateMap;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
fn main() -> Result<(), String> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("Usage: map-performance-inventory MAP.skate")?;
    let map = SkateMap::load(std::path::Path::new(&path))?;
    let mut buckets: HashMap<(u32, u32, u32, u64), Vec<usize>> = HashMap::new();
    let mut bytes = 0usize;
    let mut unique_bytes = 0usize;
    let mut unique = 0;
    for (i, t) in map.textures.iter().enumerate() {
        bytes += t.rgba.len();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        t.rgba.hash(&mut hash);
        let bucket = buckets
            .entry((t.width, t.height, t.color_space, hash.finish()))
            .or_default();
        if !bucket.iter().any(|&j| map.textures[j].rgba == t.rgba) {
            bucket.push(i);
            unique += 1;
            unique_bytes += t.rgba.len();
        }
    }
    let mut bounds = vec![([f32::INFINITY; 3], [f32::NEG_INFINITY; 3], 0u64); map.materials.len()];
    let mut cells = HashSet::new();
    for tri in map.geometry.indices.chunks_exact(3) {
        let material = map.geometry.vertices[tri[0] as usize].material as usize - 1;
        let b = &mut bounds[material];
        b.2 += 1;
        let mut center = [0.; 3];
        for &i in tri {
            let p = map.geometry.vertices[i as usize].position;
            for a in 0..3 {
                b.0[a] = b.0[a].min(p[a]);
                b.1[a] = b.1[a].max(p[a]);
                center[a] += p[a] / 3.;
            }
        }
        cells.insert((
            material,
            (center[0] / 32.).floor() as i32,
            (center[2] / 32.).floor() as i32,
        ));
    }
    let used: Vec<_> = bounds.iter().filter(|b| b.2 > 0).collect();
    let spans: Vec<_> = [50.,100.,250.,500.].into_iter().map(|threshold| {
        let matching: Vec<_> = used.iter().filter(|b|(b.1[0]-b.0[0]).max(b.1[2]-b.0[2])>threshold).collect();
        serde_json::json!({"horizontal_extent_over_metres":threshold,"raw_material_groups":matching.len(),"triangles":matching.iter().map(|b|b.2).sum::<u64>()})
    }).collect();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({
        "schema":1,"map_version":map.version,"vertices":map.geometry.vertices.len(),"triangles":map.geometry.indices.len()/3,
        "materials":map.materials.len(),"used_raw_material_groups":used.len(),"raw_material_32m_cells":cells.len(),
        "textures":map.textures.len(),"unique_texture_payloads":unique,"decoded_texture_bytes":bytes,"unique_decoded_texture_bytes":unique_bytes,
        "rail_records":map.rails.len(),"bounds":spans,
        "interpretation":"Raw material groups before renderer canonicalization; cells are triangle-centroid estimates, not actual proposed batches. Texture bytes exclude role expansion, mip chains and GPU allocation. No visibility or frame-time measurement."
    })).unwrap());
    Ok(())
}
