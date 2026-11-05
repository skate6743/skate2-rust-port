//! Bounded parallel decompression with deterministic texture indices.
use super::{StoredBlock, Texture};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) fn decode(
    textures: &mut [Texture],
    blocks: &[StoredBlock<'_>],
    workers: usize,
) -> Result<usize, String> {
    let bytes = blocks
        .iter()
        .fold(0usize, |sum, block| sum.saturating_add(block.expected));
    let workers = if bytes < 1024 * 1024 {
        1
    } else {
        workers.clamp(1, 8).min(blocks.len().max(1))
    };
    if workers == 1 {
        for (texture, block) in textures.iter_mut().zip(blocks) {
            texture.rgba = block.decode()?;
        }
        return Ok(1);
    }
    // Dynamic assignment balances many tiny textures against a few large ones.
    // Compressed slices borrow the original file; no second package copy exists.
    let next = AtomicUsize::new(0);
    let decode_batch = || -> Result<Vec<(usize, Vec<u8>)>, String> {
        let mut decoded = Vec::new();
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            let Some(block) = blocks.get(index) else {
                break;
            };
            decoded.push((
                index,
                block
                    .decode()
                    .map_err(|e| format!("Texture {index}: {e}"))?,
            ));
        }
        Ok(decoded)
    };
    let decoded = std::thread::scope(|scope| -> Result<_, String> {
        let mut jobs = Vec::new();
        for _ in 1..workers {
            jobs.push(
                std::thread::Builder::new()
                    .name("map-texture-decode".into())
                    .spawn_scoped(scope, &decode_batch)
                    .map_err(|e| format!("Texture worker: {e}"))?,
            );
        }
        let local = decode_batch();
        // Join every worker before returning either success or an error.
        let batches: Vec<_> = jobs
            .into_iter()
            .map(|job| {
                job.join()
                    .unwrap_or_else(|_| Err("Texture decoder failed unexpectedly".into()))
            })
            .collect();
        let mut decoded = local?;
        for batch in batches {
            decoded.extend(batch?);
        }
        Ok(decoded)
    })?;
    for (index, rgba) in decoded {
        textures[index].rgba = rgba;
    }
    Ok(workers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn mixed_storage_keeps_order_and_rejects_corruption_in_parallel() {
        let raw: Vec<Vec<u8>> = (0..16)
            .map(|i| vec![i as u8; (i + 1) * 32 * 1024])
            .collect();
        let mut encoded: Vec<Vec<u8>> = raw
            .iter()
            .enumerate()
            .map(|(i, bytes)| match i % 3 {
                0 => bytes.clone(),
                1 => {
                    let mut encoder =
                        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
                    encoder.write_all(bytes).unwrap();
                    encoder.finish().unwrap()
                }
                _ => zstd::stream::encode_all(bytes.as_slice(), 1).unwrap(),
            })
            .collect();
        let make_textures = || {
            (0..raw.len())
                .map(|i| Texture {
                    name: format!("texture-{i}"),
                    width: 1,
                    height: 1,
                    color_space: 0,
                    rgba: Vec::new(),
                })
                .collect::<Vec<_>>()
        };
        {
            let blocks: Vec<_> = encoded
                .iter()
                .enumerate()
                .map(|(i, bytes)| StoredBlock {
                    bytes,
                    method: (i % 3) as u32,
                    expected: raw[i].len(),
                })
                .collect();
            let mut serial = make_textures();
            decode(&mut serial, &blocks, 1).unwrap();
            for workers in [2, 4, usize::MAX] {
                let mut parallel = make_textures();
                assert_eq!(
                    decode(&mut parallel, &blocks, workers).unwrap(),
                    workers.min(8)
                );
                assert_eq!(serial, parallel);
                for (texture, bytes) in parallel.iter().zip(&raw) {
                    assert_eq!(&texture.rgba, bytes);
                }
            }
        }
        encoded[5].truncate(3);
        let blocks: Vec<_> = encoded
            .iter()
            .enumerate()
            .map(|(i, bytes)| StoredBlock {
                bytes,
                method: (i % 3) as u32,
                expected: raw[i].len(),
            })
            .collect();
        assert!(decode(&mut make_textures(), &blocks, 4).is_err());
    }

    #[test]
    fn stored_block_retains_size_and_deflate_tail_validation() {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&[7; 64]).unwrap();
        let mut bytes = encoder.finish().unwrap();
        assert!(
            StoredBlock {
                expected: 63,
                method: 1,
                bytes: &bytes
            }
            .decode()
            .is_err()
        );
        bytes.push(42);
        assert!(
            StoredBlock {
                expected: 64,
                method: 1,
                bytes: &bytes
            }
            .decode()
            .is_err()
        );
        assert!(
            StoredBlock {
                expected: 1,
                method: 0,
                bytes: &[]
            }
            .decode()
            .is_err()
        );
        assert!(
            StoredBlock {
                expected: 0,
                method: 99,
                bytes: &[]
            }
            .decode()
            .is_err()
        );
    }
}
