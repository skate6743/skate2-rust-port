//! `Sk8::FE::SubTitler`: the subtitles of a NIS. `Scene::Init` creates one for
//! the scene's name and starts it (`Start` `0x826a7508`: key 0, time 0, then
//! `Load` `0x826a7228`); `Scene::Update` feeds it the scene's time delta in
//! 1e-5 s units. `SubtitleManager::GetSubtitleData` (`0x82699c10`) finds the file
//! in the subtitle BIGs (`data\fe\subtitles\subtitles.big`, nested in
//! `miscboot.big`).

use skate2_formats::big::BigArchive;

pub const BIG_PATH: &str = "data\\fe\\subtitles\\subtitles.big";

/// `mNULLHash`: `ICELITE::StringHash("NULL")`; a key with it clears the line.
pub const NULL_HASH: u32 = 0x001a_20ba;

/// `SubtitleInfo_t`: the start in 0.1 s and the line's HAL id. The file is
/// little-endian; `Load` byte-swaps it in place on its first use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtitleKey {
    pub start_time: u16,
    pub hash: u32,
}

/// The keys up to and including the `startTime == 0xFFFF` terminator.
#[must_use]
pub fn parse(bytes: &[u8]) -> Vec<SubtitleKey> {
    let mut keys = Vec::new();
    for record in bytes.chunks_exact(8) {
        let key = SubtitleKey {
            start_time: u16::from_le_bytes([record[0], record[1]]),
            hash: u32::from_le_bytes([record[4], record[5], record[6], record[7]]),
        };
        keys.push(key);
        if key.start_time == 0xffff {
            break;
        }
    }
    keys
}

/// `Load`: `SubTitles_<language>\XBOX2_<name>.sub`, then the English
/// `XBOX2_` file, then `SubTitles_<language>\<name>.sub`, then the English one.
#[must_use]
pub fn load(subtitle_big: &[u8], language: &str, name: &str) -> Option<Vec<SubtitleKey>> {
    load_from(&[subtitle_big], language, name)
}

/// `Load` over every subtitle BIG the `SubtitleManager` holds: each of its names in turn,
/// searched through the BIGs in load order (`GetSubtitleData` walks `mBigFileList`).
/// `StartDLCSubtitleLoad` (`0x8269fb20`) and `DLCSubtitleLoadUpdate` (`0x826aaf28`) load every
/// `data\fe\subtitles\*.big` of every device, the game's then the downloadable content's.
#[must_use]
pub fn load_from(subtitle_bigs: &[&[u8]], language: &str, name: &str) -> Option<Vec<SubtitleKey>> {
    let archives: Vec<(BigArchive, &[u8])> =
        subtitle_bigs.iter().filter_map(|&big| BigArchive::parse(big).ok().map(|archive| (archive, big))).collect();
    let paths = [
        format!("data\\fe\\subtitles\\SubTitles_{language}\\XBOX2_{name}.sub"),
        format!("data\\fe\\subtitles\\SubTitles_English\\XBOX2_{name}.sub"),
        format!("data\\fe\\subtitles\\SubTitles_{language}\\{name}.sub"),
        format!("data\\fe\\subtitles\\SubTitles_English\\{name}.sub"),
    ];
    paths.iter().find_map(|path| {
        archives.iter().find_map(|(archive, big)| {
            let entry = archive.find_name(path)?;
            if entry.stored_size != entry.unpacked_size {
                return None;
            }
            big.get(entry.data_range().ok()?).map(parse)
        })
    })
}

#[derive(Clone, Debug, Default)]
pub struct SubTitler {
    data: Option<Vec<SubtitleKey>>,
    current: usize,
    /// `mTimeElapsed`, in 1e-5 s.
    elapsed: u32,
}

impl SubTitler {
    #[must_use]
    pub fn start(data: Option<Vec<SubtitleKey>>) -> Self {
        Self { data, current: 0, elapsed: 0 }
    }

    /// `Update(dT)` (`0x8269fbd0`): when the current key's start
    /// (`10000 * startTime`) has passed, `cMsgSetSubtitle` with its HAL id (0
    /// for the NULL hash) and the next key; then the time advances. One key
    /// at most per update.
    pub fn update(&mut self, dt: u32) -> Option<u32> {
        let data = self.data.as_ref()?;
        let mut sent = None;
        if let Some(key) = data.get(self.current)
            && 10_000 * u32::from(key.start_time) <= self.elapsed
        {
            sent = Some(if key.hash == NULL_HASH { 0 } else { key.hash });
            self.current += 1;
        }
        self.elapsed = self.elapsed.wrapping_add(dt);
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_hash_is_the_string_hash_of_null() {
        assert_eq!(super::super::directory::scene_hash("NULL"), NULL_HASH);
    }

    #[test]
    fn keys_fire_one_per_update_after_their_start() {
        let keys = vec![
            SubtitleKey { start_time: 0, hash: 0x96c6_5a5b },
            SubtitleKey { start_time: 22, hash: NULL_HASH },
            SubtitleKey { start_time: 0xffff, hash: NULL_HASH },
        ];
        let mut titler = SubTitler::start(Some(keys));
        assert_eq!(titler.update(1666), Some(0x96c6_5a5b));
        let mut fired = None;
        let mut updates = 1;
        while fired.is_none() {
            fired = titler.update(1666);
            updates += 1;
        }
        assert_eq!(fired, Some(0));
        // 2.2 s = 220000 units; checked before each add, so the key fires on
        // the update after the elapsed time reaches it.
        assert_eq!(updates, 1 + 220_000_u32.div_ceil(1666) as usize);
        for _ in 0..10_000 {
            assert_eq!(titler.update(1666), None);
        }
    }
}
