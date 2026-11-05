//! Lists the retail collision surface types in SKATE maps and details the
//! water surfaces (surface type 12, `(surface >> 7) & 31`).
//! Usage: cargo run -p skate-data --example water_surfaces -- <map.skate>...
use std::collections::BTreeMap;

const WATER: u16 = 12;

#[derive(Default)]
struct Water {
    triangles: usize,
    min: [f32; 3],
    max: [f32; 3],
    heights: BTreeMap<i32, usize>,
    surfaces: BTreeMap<u16, usize>,
    meshes: BTreeMap<String, usize>,
    flat: usize,
    one_sided: usize,
}

fn main() {
    let mut failed = false;
    for path in std::env::args_os().skip(1) {
        let map = match skate_data::skate_map::SkateMap::load(std::path::Path::new(&path)) {
            Ok(map) => map,
            Err(e) => {
                eprintln!("{}: {e}", path.to_string_lossy());
                failed = true;
                continue;
            }
        };
        if let Ok(ids) = std::env::var("MATERIAL") {
            let ids: Vec<usize> = ids
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            material_info(&map, &ids);
            continue;
        }
        if let Ok(ids) = std::env::var("TEXTURES") {
            let ids: Vec<usize> = ids
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            println!("{}: textures", map.name);
            texture_stats(&map, &ids);
            continue;
        }
        if let Ok(model) = std::env::var("MODEL_MATERIALS") {
            println!("{}: materials of {model}", map.name);
            model_materials(&map, &model);
            continue;
        }
        if let Ok(at) = std::env::var("RENDER_AT") {
            let v: Vec<f32> = at
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if let [x, z, ref rest @ ..] = v[..] {
                println!("{}: render materials near ({x}, {z})", map.name);
                render_at(&map, x, z, rest.first().copied().unwrap_or(3.));
            }
            continue;
        }
        render_water(&map);
        water_bindings(&map);
        let Some(archive) = map.extensions.iter().find(|e| e.tag == *b"RWCM") else {
            println!("{}: no RWCM collision", map.name);
            continue;
        };
        if let Ok(view) = std::env::var("WATER_VIEW") {
            let v: Vec<f32> = view
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if let [x, y, z, ref rest @ ..] = v[..] {
                println!("{}: view spots near ({x}, {y}, {z})", map.name);
                let reach = rest.first().copied().unwrap_or(20.);
                let rise = rest.get(1).copied().unwrap_or(3.);
                view_spots(&archive.payload, x, y, z, reach, rise);
            }
            continue;
        }
        let mut types = [0_usize; 32];
        let mut water = Water {
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
            ..Default::default()
        };
        let mut streams = BTreeMap::<String, usize>::new();
        // Surface IDs of the lowest geometry (harbour/sea beds below y = -1).
        let mut low = BTreeMap::<u16, (usize, f32)>::new();
        let result =
            skate_data::retail_collision::visit_clusters(&archive.payload, |name, cluster| {
                // Tile streams are cSim_<x>_<z>_<lod>.xsf; summarise by LOD/kind.
                let stream = name.split('#').next().unwrap_or(name);
                let kind = stream
                    .split('_')
                    .filter(|part| part.parse::<i32>().is_err())
                    .collect::<Vec<_>>()
                    .join("_");
                *streams.entry(kind).or_default() += cluster.len();
                for t in cluster {
                    let kind = (t.surface >> 7) & 31;
                    let top = t.points.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
                    if top < -1. {
                        let entry = low.entry(t.surface).or_insert((0, f32::MAX));
                        entry.0 += 1;
                        entry.1 = entry.1.min(top);
                    }
                    types[usize::from(kind)] += 1;
                    if kind != WATER {
                        continue;
                    }
                    water.triangles += 1;
                    if std::env::var_os("WATER_POINTS").is_some() {
                        let c: [f32; 3] = std::array::from_fn(|axis| {
                            t.points.iter().map(|p| p[axis]).sum::<f32>() / 3.
                        });
                        println!(
                            "  water centroid {:.1} {:.2} {:.1} ({name})",
                            c[0], c[1], c[2]
                        );
                    }
                    *water.surfaces.entry(t.surface).or_default() += 1;
                    *water.meshes.entry(name.to_owned()).or_default() += 1;
                    for p in t.points {
                        for axis in 0..3 {
                            water.min[axis] = water.min[axis].min(p[axis]);
                            water.max[axis] = water.max[axis].max(p[axis]);
                        }
                    }
                    let ys = t.points.map(|p| p[1]);
                    let spread = ys.iter().cloned().fold(f32::MIN, f32::max)
                        - ys.iter().cloned().fold(f32::MAX, f32::min);
                    if spread < 0.01 {
                        water.flat += 1;
                    }
                    if t.one_sided {
                        water.one_sided += 1;
                    }
                    *water
                        .heights
                        .entry((ys[0] * 10.).round() as i32)
                        .or_default() += 1;
                }
                Ok(())
            });
        if let Err(e) = result {
            eprintln!("{}: {e}", map.name);
            failed = true;
            continue;
        }
        let histogram: Vec<_> = types
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0)
            .map(|(kind, n)| format!("{kind}:{n}"))
            .collect();
        println!("{}: surface types {}", map.name, histogram.join(" "));
        println!("  collision streams (triangles): {streams:?}");
        let low: Vec<_> = low
            .iter()
            .map(|(s, (n, y))| format!("{s}(type {}):{n} min {y:.1}m", (s >> 7) & 31))
            .collect();
        println!("  surfaces below -1 m: {}", low.join(", "));
        if water.triangles == 0 {
            println!("  water: none");
            continue;
        }
        println!(
            "  water: {} triangles ({} flat, {} one-sided), bounds {:?}..{:?}",
            water.triangles, water.flat, water.one_sided, water.min, water.max
        );
        println!("  water surface IDs: {:?}", water.surfaces);
        let heights: Vec<_> = water
            .heights
            .iter()
            .map(|(y, n)| format!("{:.1}m:{n}", *y as f32 / 10.))
            .collect();
        println!("  water heights (first vertex): {}", heights.join(" "));
        let mut meshes: Vec<_> = water.meshes.into_iter().collect();
        meshes.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, n) in meshes.iter().take(12) {
            println!("  water mesh {name}: {n}");
        }
        if meshes.len() > 12 {
            println!("  ... {} more water meshes", meshes.len() - 12);
        }
    }
    if failed {
        std::process::exit(1);
    }
}

/// Height range of render meshes whose retail shader is water.* or ocean.*.
/// SKATE vertex material ids are 1-based.
fn render_water(map: &skate_data::skate_map::SkateMap) {
    let shader = |m: &skate_data::skate_map::Material| {
        let bytes = m.retail_definition.as_deref()?;
        ["water.", "ocean."].iter().find_map(|prefix| {
            let at = bytes
                .windows(prefix.len())
                .position(|w| w == prefix.as_bytes())?;
            let end = bytes[at..]
                .iter()
                .position(|b| !(b.is_ascii_alphanumeric() || *b == b'.' || *b == b'_'))?;
            Some(String::from_utf8_lossy(&bytes[at..at + end]).into_owned())
        })
    };
    let shaders: Vec<_> = map.materials.iter().map(shader).collect();
    let mut ranges = BTreeMap::<(String, String), ([f32; 3], [f32; 3], usize)>::new();
    for v in &map.geometry.vertices {
        let Some(Some(name)) = shaders.get((v.material as usize).wrapping_sub(1)) else {
            continue;
        };
        let key = (
            name.clone(),
            map.materials[v.material as usize - 1].name.clone(),
        );
        let entry = ranges
            .entry(key)
            .or_insert(([f32::MAX; 3], [f32::MIN; 3], 0));
        for axis in 0..3 {
            entry.0[axis] = entry.0[axis].min(v.position[axis]);
            entry.1[axis] = entry.1[axis].max(v.position[axis]);
        }
        entry.2 += 1;
    }
    for ((shader, material), (min, max, n)) in ranges {
        println!(
            "  render {shader} {material}: {n} vertices, y {:.2}..{:.2}, x {:.0}..{:.0}, z {:.0}..{:.0}",
            min[1], max[1], min[0], max[0], min[2], max[2]
        );
    }
}

/// Texture bindings of each distinct water/ocean material definition and
/// whether the referenced 1-based texture id exists in the map.
/// Definition layout: retail_render::Definition::parse.
pub fn water_bindings(map: &skate_data::skate_map::SkateMap) {
    struct R<'a>(&'a [u8]);
    impl<'a> R<'a> {
        fn take(&mut self, n: usize) -> Option<&'a [u8]> {
            let v = self.0.get(..n)?;
            self.0 = &self.0[n..];
            Some(v)
        }
        fn u32(&mut self) -> Option<u32> {
            Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
        }
        fn text(&mut self) -> Option<String> {
            let n = self.u32()? as usize;
            Some(String::from_utf8_lossy(self.take(n)?).into_owned())
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for m in &map.materials {
        let Some(bytes) = m.retail_definition.as_deref() else {
            continue;
        };
        let mut r = R(bytes);
        let parsed = (|| {
            r.take(16)?;
            let shader = r.text()?;
            let family = r.u32()?;
            let _flags = r.u32()?;
            let mut bindings = Vec::new();
            for _ in 0..r.u32()? {
                let role = r.text()?;
                let texture = r.u32()?;
                let (_uv, _u, _v) = (r.u32()?, r.u32()?, r.u32()?);
                bindings.push((role, texture));
            }
            Some((shader, family, bindings))
        })();
        let Some((shader, family, bindings)) = parsed else {
            continue;
        };
        if !(shader.starts_with("water.") || shader.starts_with("ocean.")) {
            continue;
        }
        let described: Vec<_> = bindings
            .iter()
            .map(|(role, id)| {
                let t = (*id as usize)
                    .checked_sub(1)
                    .and_then(|i| map.textures.get(i));
                match t {
                    Some(t) => format!("{role}={id}:{}({}x{})", t.name, t.width, t.height),
                    None => format!("{role}={id}:MISSING"),
                }
            })
            .collect();
        let key = format!(
            "{shader} fam{family} textures={:?} | {}",
            m.textures,
            described.join(", ")
        );
        if seen.insert(key.clone()) {
            println!("  binding {key}");
        }
    }
}

/// `WATER_VIEW=x,y,z[,max_distance,max_height]`: flat, dry (non-water) ground
/// 6 m to max_distance (20) from a water point within max_height (3) of it,
/// nearest first: somewhere to stand and look.
pub fn view_spots(archive: &[u8], x: f32, y: f32, z: f32, reach: f32, rise: f32) {
    let mut spots = Vec::new();
    let _ = skate_data::retail_collision::visit_clusters(archive, |_, cluster| {
        for t in cluster {
            let kind = (t.surface >> 7) & 31;
            if !t.has_surface || kind == WATER || kind == 0 {
                continue;
            }
            let [a, b, c] = t.points;
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let area = len / 2.;
            // Retail winding is inconsistent: accept flat faces either way up.
            if len == 0. || (n[1] / len).abs() < 0.95 || area < 1. {
                continue;
            }
            let p: [f32; 3] = core::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.);
            let d = ((p[0] - x).powi(2) + (p[2] - z).powi(2)).sqrt();
            if (6.0..=reach).contains(&d) && (p[1] - y).abs() < rise {
                spots.push((d, p, kind, area));
            }
        }
        Ok(())
    });
    spots.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (d, p, kind, area) in spots.iter().take(6) {
        let heading = (x - p[0]).atan2(z - p[2]);
        println!(
            "  view {:.1} {:.2} {:.1} heading {heading:.3} (type {kind}, {area:.0} m2, {d:.1} m from water)",
            p[0], p[1], p[2]
        );
    }
}

/// `RENDER_AT=x,z[,radius]`: render materials of triangles whose centroid is
/// within `radius` (3 m) of x,z horizontally, with their shader and height.
pub fn render_at(map: &skate_data::skate_map::SkateMap, x: f32, z: f32, radius: f32) {
    let mut found = BTreeMap::<(u32, String), (usize, f32, f32)>::new();
    for tri in map.geometry.indices.chunks_exact(3) {
        let v: [&skate_data::skate_map::Vertex; 3] =
            core::array::from_fn(|k| &map.geometry.vertices[tri[k] as usize]);
        let c: [f32; 3] =
            core::array::from_fn(|a| v.iter().map(|v| v.position[a]).sum::<f32>() / 3.);
        // Triangles directly above/below x,z, or with a centroid within radius.
        let [a, b, d] = v.map(|v| [v.position[0], v.position[2]]);
        let side =
            |p: [f32; 2], q: [f32; 2]| (q[0] - p[0]) * (z - p[1]) - (q[1] - p[1]) * (x - p[0]);
        let (s1, s2, s3) = (side(a, b), side(b, d), side(d, a));
        let area = ((b[0] - a[0]) * (d[1] - a[1]) - (b[1] - a[1]) * (d[0] - a[0])).abs();
        let inside = area > 1e-4
            && ((s1 >= 0. && s2 >= 0. && s3 >= 0.) || (s1 <= 0. && s2 <= 0. && s3 <= 0.));
        if !inside && (c[0] - x).hypot(c[2] - z) > radius {
            continue;
        }
        let material = v[0].material;
        let m = &map.materials[material as usize - 1];
        let shader = m
            .retail_definition
            .as_deref()
            .and_then(|b| {
                let n = u32::from_le_bytes(b.get(16..20)?.try_into().ok()?) as usize;
                Some(String::from_utf8_lossy(b.get(20..20 + n)?).into_owned())
            })
            .unwrap_or_else(|| "(portable)".into());
        let e = found
            .entry((material, shader))
            .or_insert((0, f32::MAX, f32::MIN));
        e.0 += 1;
        e.1 = e.1.min(c[1]);
        e.2 = e.2.max(c[1]);
    }
    for ((material, shader), (n, lo, hi)) in found {
        println!(
            "  render at: material {material} {shader} ({}) {n} tris, y {lo:.2}..{hi:.2}",
            map.materials[material as usize - 1].name
        );
    }
}

/// `MODEL_MATERIALS=0x<model guid>`: each material of one retail model, with
/// shader, vertex count, height range and whether its geometry is flat.
pub fn model_materials(map: &skate_data::skate_map::SkateMap, model: &str) {
    let shader_of = |m: &skate_data::skate_map::Material| {
        m.retail_definition
            .as_deref()
            .and_then(|b| {
                let n = u32::from_le_bytes(b.get(16..20)?.try_into().ok()?) as usize;
                Some(String::from_utf8_lossy(b.get(20..20 + n)?).into_owned())
            })
            .unwrap_or_else(|| "(portable)".into())
    };
    let mut ranges = BTreeMap::<u32, (usize, f32, f32, [f32; 4])>::new();
    for v in &map.geometry.vertices {
        if !map.materials[v.material as usize - 1]
            .name
            .starts_with(model)
        {
            continue;
        }
        let e = ranges.entry(v.material).or_insert((
            0,
            f32::MAX,
            f32::MIN,
            [f32::MAX, f32::MIN, f32::MAX, f32::MIN],
        ));
        e.0 += 1;
        e.1 = e.1.min(v.position[1]);
        e.2 = e.2.max(v.position[1]);
        e.3 = [
            e.3[0].min(v.position[0]),
            e.3[1].max(v.position[0]),
            e.3[2].min(v.position[2]),
            e.3[3].max(v.position[2]),
        ];
    }
    let mut rows: Vec<_> = ranges.into_iter().collect();
    rows.sort_by(|a, b| {
        map.materials[a.0 as usize - 1]
            .name
            .cmp(&map.materials[b.0 as usize - 1].name)
    });
    for (material, (n, lo, hi, xz)) in rows {
        let m = &map.materials[material as usize - 1];
        println!(
            "  model {} {:30} {n:5} verts y {lo:7.2}..{hi:7.2}{} x {:.0}..{:.0} z {:.0}..{:.0}",
            m.name,
            shader_of(m),
            if hi - lo < 0.01 { " FLAT" } else { "" },
            xz[0],
            xz[1],
            xz[2],
            xz[3]
        );
    }
}

/// `TEXTURES=id,id,...`: size, colour space and mean RGBA of 1-based texture ids.
pub fn texture_stats(map: &skate_data::skate_map::SkateMap, ids: &[usize]) {
    for &id in ids {
        let Some(t) = id.checked_sub(1).and_then(|i| map.textures.get(i)) else {
            println!("  texture {id}: missing");
            continue;
        };
        let mut sum = [0f64; 4];
        let mut max = [0u8; 4];
        let px = t.rgba.len() / 4;
        for p in t.rgba.chunks_exact(4) {
            for c in 0..4 {
                sum[c] += f64::from(p[c]);
                max[c] = max[c].max(p[c]);
            }
        }
        let mean = sum.map(|s| s / px.max(1) as f64);
        println!(
            "  texture {id} {} {}x{} space {} mean rgba {:.0} {:.0} {:.0} {:.0} max {:?}",
            t.name, t.width, t.height, t.color_space, mean[0], mean[1], mean[2], mean[3], max
        );
    }
}

/// `MATERIAL=id,...`: portable alpha mode/cutoff and retail flags of 1-based material ids.
pub fn material_info(map: &skate_data::skate_map::SkateMap, ids: &[usize]) {
    for &id in ids {
        let Some(m) = id.checked_sub(1).and_then(|i| map.materials.get(i)) else {
            continue;
        };
        let def = m.retail_definition.as_deref().unwrap_or(&[]);
        let shader_len = def
            .get(16..20)
            .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
        let at = 20 + shader_len;
        let word = |o: usize| {
            def.get(at + o..at + o + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        };
        println!(
            "  material {id} {} {} alpha_mode {} cutoff {} family {:?} render_flags {:?}",
            m.name,
            String::from_utf8_lossy(def.get(20..at).unwrap_or(&[])),
            m.alpha_mode,
            m.alpha_cutoff,
            word(0),
            word(4)
        );
    }
}
