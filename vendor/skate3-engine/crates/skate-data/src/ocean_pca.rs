//! The built-in PCA ocean/water animation table (cPCAWaterAnimationData) from
//! an unpacked retail executable, written as `assets/private/ocean-pca.json`.
//!
//! TU3 keeps the 30 frame means at 0x830118D8 and the weights right after them
//! at 0x83011A40 (tools/asset_pipeline/ocean_pca.py). Other builds place the
//! table elsewhere, so it is located the way the code reaches it: a `lis`
//! followed by a D-form use of the same register forms each address, and
//! cPCAWaterAnimationData::Init forms both within a few instructions.
use crate::xex::XexImage;

const FRAMES: usize = 30;
const MEANS: usize = FRAMES * 3;
const WEIGHTS: usize = FRAMES * 24;
const WEIGHTS_OFFSET: u32 = (MEANS * 4) as u32; // 0x168
/// Instructions between `lis` and the instruction completing an address.
const ADDRESS_WINDOW: usize = 8;
/// Instructions between the two address loads.
const PAIR_WINDOW: usize = 32;

pub struct OceanPca {
    pub means_address: u32,
    /// Per frame: mean row then six weight rows, in shader order, scaled /255.
    pub frames: Vec<[[f32; 4]; 7]>,
}

pub fn extract(image: &XexImage) -> Result<OceanPca, String> {
    let candidates = locate(image);
    let means_address = match candidates.as_slice() {
        [one] => *one,
        [] => return Err("Ocean PCA table not found in the executable".into()),
        many => {
            return Err(format!(
                "Ocean PCA table is ambiguous: {} candidates ({:08X?})",
                many.len(),
                many
            ));
        }
    };
    let means = floats(image, means_address, MEANS).ok_or("Ocean PCA means out of range")?;
    let weights = floats(image, means_address + WEIGHTS_OFFSET, WEIGHTS)
        .ok_or("Ocean PCA weights out of range")?;
    let frames = (0..FRAMES)
        .map(|frame| {
            let mean = &means[frame * 3..frame * 3 + 3];
            let weight = &weights[frame * 24..frame * 24 + 24];
            let mut rows = [[0.; 4]; 7];
            // Shader model order: mean X/Z/Y and weight pairs R/B/G.
            rows[0] = [mean[0] / 255., mean[2] / 255., mean[1] / 255., 0.];
            for (row, start) in rows[1..].iter_mut().zip([0, 4, 16, 20, 8, 12]) {
                *row = core::array::from_fn(|i| weight[start + i] / 255.);
            }
            rows
        })
        .collect();
    Ok(OceanPca {
        means_address,
        frames,
    })
}

impl OceanPca {
    /// The JSON read by the game (`retail_render::read_pca`), matching
    /// tools/asset_pipeline/ocean_pca.py.
    pub fn to_json(&self, source_sha256: &str) -> String {
        serde_json::json!({
            "source_sha256": source_sha256,
            "means_address": format!("0x{:08X}", self.means_address),
            "hz": 30.0,
            "frames": self.frames,
        })
        .to_string()
    }
}

fn floats(image: &XexImage, address: u32, count: usize) -> Option<Vec<f32>> {
    let bytes = image.at(address, count * 4)?;
    let values: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    values.iter().all(|v| v.is_finite()).then_some(values)
}

/// A table holds 30 normal-like means (0..255 components) and nonzero weights.
fn plausible(image: &XexImage, address: u32) -> bool {
    let (Some(means), Some(weights)) = (
        floats(image, address, MEANS),
        floats(image, address + WEIGHTS_OFFSET, WEIGHTS),
    ) else {
        return false;
    };
    means.iter().all(|v| (0.0..=255.0).contains(v))
        && means.iter().any(|v| *v != 0.)
        && weights.iter().all(|v| v.abs() <= 255.)
        && weights.iter().filter(|v| **v != 0.).count() > WEIGHTS / 2
}

fn locate(image: &XexImage) -> Vec<u32> {
    let words: Vec<u32> = image
        .image
        .chunks_exact(4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    // (instruction index, address) for every lis + D-form completion.
    let mut formed: Vec<(usize, u32)> = Vec::new();
    for (i, &w) in words.iter().enumerate() {
        let (op, rd, ra) = (w >> 26, (w >> 21) & 31, (w >> 16) & 31);
        if op != 15 || ra != 0 {
            continue;
        }
        let high = (w & 0xffff) << 16;
        for (j, &u) in words.iter().enumerate().skip(i + 1).take(ADDRESS_WINDOW) {
            let (uop, urd, ura) = (u >> 26, (u >> 21) & 31, (u >> 16) & 31);
            // addi and the integer/float loads and stores.
            if ura == rd && matches!(uop, 14 | 32 | 34 | 36 | 40 | 44 | 48 | 50 | 52 | 54) {
                let low = (u & 0xffff) as u16 as i16 as i32;
                formed.push((j, high.wrapping_add(low as u32)));
            }
            if uop == 15 && urd == rd {
                break;
            }
        }
    }
    let mut found: Vec<u32> = Vec::new();
    for (k, &(i, address)) in formed.iter().enumerate() {
        let pair = formed[k.saturating_sub(64)..(k + 64).min(formed.len())]
            .iter()
            .any(|&(j, other)| {
                other == address.wrapping_add(WEIGHTS_OFFSET) && i.abs_diff(j) <= PAIR_WINDOW
            });
        if pair && !found.contains(&address) && plausible(image, address) {
            found.push(address);
        }
    }
    found
}

/// Unpacks `xex`, extracts the table and returns the game's JSON.
pub fn json_from_xex(xex: &[u8]) -> Result<String, String> {
    let image = XexImage::parse(xex)?;
    let pca = extract(&image)?;
    Ok(pca.to_json(&crate::sha256::digest(&image.image)))
}
