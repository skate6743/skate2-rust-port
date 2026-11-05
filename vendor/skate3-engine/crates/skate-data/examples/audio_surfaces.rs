//! Lists the audio material IDs (`surface & 0x7f`) used by retail collision in
//! SKATE maps, to label them for rolling/grind sounds (game_audio). For each ID:
//! triangle count, upward-facing area, the physics types (`(surface >> 7) & 31`)
//! it appears with, and the albedo textures of the rendered geometry found at
//! sample points on it.
//! Usage: cargo run -p skate-data --example audio_surfaces -- <map.skate>...
use std::collections::{BTreeMap, BTreeSet, HashMap};

const SAMPLES: usize = 12;
const CELL: f32 = 4.0;

#[derive(Default)]
struct Audio {
    triangles: usize,
    up_area: f32,
    physics: BTreeSet<u16>,
    samples: Vec<[f32; 3]>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Render triangles bucketed on the XZ plane.
struct Grid {
    cells: HashMap<(i32, i32), Vec<u32>>,
}
impl Grid {
    fn new(map: &skate_data::skate_map::SkateMap) -> Self {
        let mut cells = HashMap::<(i32, i32), Vec<u32>>::new();
        for (index, tri) in map.geometry.indices.chunks_exact(3).enumerate() {
            let p: [[f32; 3]; 3] =
                std::array::from_fn(|k| map.geometry.vertices[tri[k] as usize].position);
            let (x0, x1) = (
                p.iter().map(|v| v[0]).fold(f32::MAX, f32::min),
                p.iter().map(|v| v[0]).fold(f32::MIN, f32::max),
            );
            let (z0, z1) = (
                p.iter().map(|v| v[2]).fold(f32::MAX, f32::min),
                p.iter().map(|v| v[2]).fold(f32::MIN, f32::max),
            );
            if x1 - x0 > 200. || z1 - z0 > 200. {
                continue; // sky/backdrop sheets
            }
            for cx in (x0 / CELL).floor() as i32..=(x1 / CELL).floor() as i32 {
                for cz in (z0 / CELL).floor() as i32..=(z1 / CELL).floor() as i32 {
                    cells.entry((cx, cz)).or_default().push(index as u32);
                }
            }
        }
        Self { cells }
    }

    /// Material of the render triangle under/at `point` (within 0.4 m vertically).
    fn material_at(&self, map: &skate_data::skate_map::SkateMap, point: [f32; 3]) -> Option<u32> {
        let key = (
            (point[0] / CELL).floor() as i32,
            (point[2] / CELL).floor() as i32,
        );
        let mut best: Option<(f32, u32)> = None;
        for &index in self.cells.get(&key)? {
            let tri = &map.geometry.indices[index as usize * 3..index as usize * 3 + 3];
            let v = [0, 1, 2].map(|k| &map.geometry.vertices[tri[k] as usize]);
            let [a, b, c] = v.map(|v| v.position);
            let (x, z) = (point[0], point[2]);
            let det = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
            if det.abs() < 1e-6 {
                continue;
            }
            let l1 = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / det;
            let l2 = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / det;
            let l3 = 1. - l1 - l2;
            if l1 < 0. || l2 < 0. || l3 < 0. {
                continue;
            }
            let y = l1 * a[1] + l2 * b[1] + l3 * c[1];
            let dy = (y - point[1]).abs();
            if dy < 0.4 && best.is_none_or(|(d, _)| dy < d) {
                best = Some((dy, v[0].material));
            }
        }
        best.map(|(_, m)| m)
    }
}

fn texture_name(map: &skate_data::skate_map::SkateMap, material: u32) -> String {
    let Some(m) = map.materials.get(material as usize - 1) else {
        return "?".into();
    };
    let albedo = m.textures[0];
    if albedo == 0 {
        return format!("({})", m.name);
    }
    map.textures
        .get(albedo as usize - 1)
        .map_or_else(|| m.name.clone(), |t| t.name.clone())
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
        let Some(archive) = map.extensions.iter().find(|e| e.tag == *b"RWCM") else {
            println!("{}: no RWCM collision", map.name);
            continue;
        };
        // AT=x,y,z[,r]: collision surfaces within r (default 3) m of a point.
        if let Ok(at) = std::env::var("AT") {
            let v: Vec<f32> = at
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if let [x, y, z, ref rest @ ..] = v[..] {
                let r = rest.first().copied().unwrap_or(3.0);
                let mut near = BTreeMap::<(u16, u16), (usize, f32, [usize; 3])>::new();
                let _ =
                    skate_data::retail_collision::visit_clusters(&archive.payload, |_, cluster| {
                        for t in cluster {
                            let c: [f32; 3] = std::array::from_fn(|a| {
                                t.points.iter().map(|p| p[a]).sum::<f32>() / 3.
                            });
                            let d = ((c[0] - x).powi(2) + (c[1] - y).powi(2) + (c[2] - z).powi(2))
                                .sqrt();
                            if d < r {
                                let e = near
                                    .entry((t.surface & 0x7F, (t.surface >> 7) & 31))
                                    .or_insert((0, f32::MAX, [0usize; 3]));
                                e.0 += 1;
                                e.1 = e.1.min(d);
                                // Slope: 0 flat (< 20°), 1 ramp (20-70°), 2 wall (> 70°).
                                let n = cross(
                                    sub(t.points[1], t.points[0]),
                                    sub(t.points[2], t.points[0]),
                                );
                                let len =
                                    (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
                                let angle = (n[1].abs() / len).clamp(0.0, 1.0).acos().to_degrees();
                                e.2[if angle < 20.0 {
                                    0
                                } else if angle < 70.0 {
                                    1
                                } else {
                                    2
                                }] += 1;
                            }
                        }
                        Ok(())
                    });
                println!("{}: surfaces within {r} m of ({x}, {y}, {z})", map.name);
                for ((audio, physics), (n, d, slopes)) in near {
                    println!(
                        "  audio {audio:3} physics {physics:2}: {n:4} tris, nearest {d:.2} m, flat/ramp/wall {slopes:?}"
                    );
                }
            }
            continue;
        }
        let mut ids = BTreeMap::<u16, Audio>::new();
        let result =
            skate_data::retail_collision::visit_clusters(&archive.payload, |_, cluster| {
                for t in cluster {
                    let entry = ids.entry(t.surface & 0x7F).or_default();
                    entry.triangles += 1;
                    entry.physics.insert((t.surface >> 7) & 31);
                    let n = cross(sub(t.points[1], t.points[0]), sub(t.points[2], t.points[0]));
                    let area = 0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    if area > 0. && n[1].abs() / (2. * area) > 0.7 {
                        entry.up_area += area;
                        // Keep a spread of samples: every k-th upward triangle.
                        if entry.samples.len() < SAMPLES * 8 {
                            entry.samples.push(std::array::from_fn(|a| {
                                t.points.iter().map(|p| p[a]).sum::<f32>() / 3.
                            }));
                        }
                    }
                }
                Ok(())
            });
        if let Err(e) = result {
            eprintln!("{}: {e}", map.name);
            failed = true;
            continue;
        }
        let grid = Grid::new(&map);
        println!("== {} ({} audio IDs)", map.name, ids.len());
        for (id, audio) in &ids {
            let mut textures = BTreeMap::<String, usize>::new();
            let step = (audio.samples.len() / SAMPLES).max(1);
            for point in audio.samples.iter().step_by(step) {
                if let Some(material) = grid.material_at(&map, *point) {
                    *textures.entry(texture_name(&map, material)).or_default() += 1;
                }
            }
            let mut top: Vec<_> = textures.into_iter().collect();
            top.sort_by(|a, b| b.1.cmp(&a.1));
            let top: Vec<String> = top
                .iter()
                .take(4)
                .map(|(name, n)| format!("{name} x{n}"))
                .collect();
            println!(
                "  audio {id:3}: {:7} tris, {:9.0} m2 up, physics {:?}, at: {}",
                audio.triangles,
                audio.up_area,
                audio.physics,
                top.join(", ")
            );
        }
    }
    if failed {
        std::process::exit(1);
    }
}
