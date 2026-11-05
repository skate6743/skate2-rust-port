//! PC port addition: an Xbox 360 disc image (`.iso`) read in place as the game's folder. A path
//! that runs through an image (`E:\games\Skate 2.iso\data\db\db.big`) names the file at the rest
//! of the path in the image's XDVDFS volume (the `MICROSOFT*XBOX*MEDIA` volume of the game
//! partition), ignoring case as the console does. Each XDVDFS file lies in one run of sectors, so
//! a file is read where it lies in the image and nothing is unpacked. An image's directory tree is
//! read once, the first time a path names it.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

const SECTOR: u64 = 2048;
const VOLUME_SECTOR: u64 = 32;
const MAGIC: &[u8; 20] = b"MICROSOFT*XBOX*MEDIA";
/// Where the game partition starts: a plain image's volume, or a whole disc's (original Xbox,
/// Xbox 360 XGD2 and XGD3 layouts).
const PARTITIONS: [u64; 4] = [0, 0x1830_0000, 0x0FD9_0000, 0x0208_0000];
const DIRECTORY: u8 = 0x10;
/// Deeper directories than any disc holds end the walk (a damaged image's table naming itself).
const MAX_DEPTH: u32 = 32;

#[derive(Debug)]
enum Node {
    File { offset: u64, size: u64 },
    /// Entries by lowercase name: their own spelling and node.
    Directory(BTreeMap<String, (String, Node)>),
}

/// One image's directory tree.
#[derive(Debug)]
struct Image {
    path: PathBuf,
    root: Node,
}

/// Images read so far, and the paths that were no image.
static IMAGES: RwLock<Vec<Arc<Image>>> = RwLock::new(Vec::new());
static NOT_IMAGES: RwLock<Vec<PathBuf>> = RwLock::new(Vec::new());

/// Whether `path` names a disc image by its extension.
#[must_use]
pub fn is_image(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("iso"))
}

/// What a path through an image names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    File { image_offset: u64, size: u64 },
    Directory,
    Missing,
}

/// What `path` names inside the disc image it runs through; `None` when it runs through none.
/// The image itself is a directory.
#[must_use]
pub fn find(path: &Path) -> Option<Found> {
    let (image, inner) = locate(path)?;
    Some(match image.node(&inner) {
        Some(Node::File { offset, size }) => Found::File { image_offset: *offset, size: *size },
        Some(Node::Directory(_)) => Found::Directory,
        None => Found::Missing,
    })
}

/// The entries of the directory `path` names inside a disc image, in their own spelling with
/// whether each is a directory; `None` when `path` runs through no image.
#[must_use]
pub fn entries(path: &Path) -> Option<io::Result<Vec<(String, bool)>>> {
    let (image, inner) = locate(path)?;
    Some(match image.node(&inner) {
        Some(Node::Directory(entries)) => Ok(entries.values().map(|(name, node)| (name.clone(), matches!(node, Node::Directory(_)))).collect()),
        Some(Node::File { .. }) => Err(io::Error::new(io::ErrorKind::NotADirectory, format!("{} is a file", path.display()))),
        None => Err(not_found(path)),
    })
}

/// Opens the file `path` names inside a disc image; `None` when `path` runs through no image.
#[must_use]
pub fn open(path: &Path) -> Option<io::Result<DiscFile>> {
    let (image, inner) = locate(path)?;
    Some(match image.node(&inner) {
        Some(&Node::File { offset, size }) => File::open(&image.path).map(|file| DiscFile { file, start: offset, len: size, position: 0, placed: false }),
        Some(Node::Directory(_)) => Err(io::Error::new(io::ErrorKind::IsADirectory, format!("{} is a directory", path.display()))),
        None => Err(not_found(path)),
    })
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("{} is not in the disc image", path.display()))
}

/// The image `path` runs through, read if it was not yet, and the components after it.
fn locate(path: &Path) -> Option<(Arc<Image>, Vec<String>)> {
    let components: Vec<Component> = path.components().collect();
    let at = components.iter().position(|component| matches!(component, Component::Normal(name) if is_image(Path::new(name))))?;
    let image_path: PathBuf = components[..=at].iter().collect();
    let mut inner = Vec::new();
    for component in &components[at + 1..] {
        match component {
            Component::Normal(name) => inner.push(name.to_string_lossy().to_ascii_lowercase()),
            Component::ParentDir => {
                inner.pop()?;
            }
            _ => {}
        }
    }
    Some((image(&image_path)?, inner))
}

fn image(path: &Path) -> Option<Arc<Image>> {
    if let Some(found) = IMAGES.read().unwrap_or_else(PoisonError::into_inner).iter().find(|image| image.path == path) {
        return Some(Arc::clone(found));
    }
    if NOT_IMAGES.read().unwrap_or_else(PoisonError::into_inner).iter().any(|other| other == path) {
        return None;
    }
    let read = File::open(path).ok().filter(|_| path.is_file()).and_then(|file| Image::read(path, file).ok());
    let Some(read) = read else {
        NOT_IMAGES.write().unwrap_or_else(PoisonError::into_inner).push(path.to_owned());
        return None;
    };
    let mut images = IMAGES.write().unwrap_or_else(PoisonError::into_inner);
    if let Some(found) = images.iter().find(|image| image.path == path) {
        return Some(Arc::clone(found));
    }
    let read = Arc::new(read);
    images.push(Arc::clone(&read));
    Some(read)
}

/// Checks that `path` is an Xbox disc image, reading its directory tree.
///
/// # Errors
///
/// The image cannot be read, or holds no XDVDFS volume.
pub fn check_image(path: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let read = Image::read(path, file)?;
    let mut images = IMAGES.write().unwrap_or_else(PoisonError::into_inner);
    if !images.iter().any(|image| image.path == path) {
        images.push(Arc::new(read));
    }
    NOT_IMAGES.write().unwrap_or_else(PoisonError::into_inner).retain(|other| other != path);
    Ok(())
}

impl Image {
    fn read(path: &Path, file: impl Read + Seek) -> Result<Self, String> {
        let mut reader = Volume::find(file)?;
        let (sector, size) = reader.root()?;
        let root = reader.tree(sector, size, 0)?;
        Ok(Self { path: path.to_owned(), root })
    }

    fn node(&self, inner: &[String]) -> Option<&Node> {
        let mut node = &self.root;
        for name in inner {
            let Node::Directory(entries) = node else { return None };
            node = &entries.get(name)?.1;
        }
        Some(node)
    }
}

/// An image's game partition.
struct Volume<R> {
    file: R,
    partition: u64,
}

impl<R: Read + Seek> Volume<R> {
    fn find(mut file: R) -> Result<Self, String> {
        for partition in PARTITIONS {
            let mut magic = [0; 20];
            if file.seek(SeekFrom::Start(partition + VOLUME_SECTOR * SECTOR)).is_ok() && file.read_exact(&mut magic).is_ok() && &magic == MAGIC {
                return Ok(Self { file, partition });
            }
        }
        Err("not an Xbox disc image (no XDVDFS volume)".into())
    }

    /// The root directory table's sector and size.
    fn root(&mut self) -> Result<(u32, u32), String> {
        let mut volume = [0; 28];
        self.read_at(self.partition + VOLUME_SECTOR * SECTOR, &mut volume)?;
        Ok((u32::from_le_bytes(volume[20..24].try_into().expect("four bytes")), u32::from_le_bytes(volume[24..28].try_into().expect("four bytes"))))
    }

    fn tree(&mut self, sector: u32, size: u32, depth: u32) -> Result<Node, String> {
        if depth > MAX_DEPTH {
            return Err("the disc image's directories nest too deep".into());
        }
        let mut entries = BTreeMap::new();
        for (name, sector, size, attributes) in self.directory(sector, size)? {
            if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
                continue;
            }
            let node = if attributes & DIRECTORY != 0 {
                self.tree(sector, size, depth + 1)?
            } else {
                Node::File { offset: self.partition + u64::from(sector) * SECTOR, size: u64::from(size) }
            };
            entries.entry(name.to_ascii_lowercase()).or_insert((name, node));
        }
        Ok(Node::Directory(entries))
    }

    /// A directory table's entries (name, sector, size, attributes): a binary tree whose nodes
    /// name their left and right subtrees by offset in 4-byte units.
    fn directory(&mut self, sector: u32, size: u32) -> Result<Vec<(String, u32, u32, u8)>, String> {
        if size == 0 {
            return Ok(Vec::new());
        }
        let mut table = vec![0; usize::try_from(size).map_err(|_| "directory too large")?];
        self.read_at(self.partition + u64::from(sector) * SECTOR, &mut table)?;
        let mut entries = Vec::new();
        let mut pending = vec![0_usize];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(offset) = pending.pop() {
            if !seen.insert(offset) || offset + 14 > table.len() {
                continue;
            }
            let node = &table[offset..];
            let left = u16::from_le_bytes([node[0], node[1]]);
            let right = u16::from_le_bytes([node[2], node[3]]);
            if left == 0xFFFF && right == 0xFFFF {
                continue;
            }
            let length = usize::from(node[13]);
            let Some(name) = node.get(14..14 + length) else { continue };
            entries.push((
                String::from_utf8_lossy(name).into_owned(),
                u32::from_le_bytes(node[4..8].try_into().expect("four bytes")),
                u32::from_le_bytes(node[8..12].try_into().expect("four bytes")),
                node[12],
            ));
            for child in [left, right] {
                if child != 0 && child != 0xFFFF {
                    pending.push(usize::from(child) * 4);
                }
            }
        }
        Ok(entries)
    }

    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> Result<(), String> {
        self.file.seek(SeekFrom::Start(offset)).map_err(|error| error.to_string())?;
        self.file.read_exact(buffer).map_err(|error| format!("the disc image ends early ({error})"))
    }
}

/// One file in a disc image: `Read` and `Seek` over its run of the image.
#[derive(Debug)]
pub struct DiscFile {
    file: File,
    start: u64,
    len: u64,
    position: u64,
    /// The image's own position is the file's.
    placed: bool,
}

impl DiscFile {
    /// The file's size.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Read for DiscFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.len.saturating_sub(self.position);
        let wanted = usize::try_from(left).unwrap_or(usize::MAX).min(buf.len());
        if wanted == 0 {
            return Ok(0);
        }
        if !self.placed {
            self.file.seek(SeekFrom::Start(self.start + self.position))?;
            self.placed = true;
        }
        let read = self.file.read(&mut buf[..wanted])?;
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for DiscFile {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let position = match to {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(delta) => self.len.checked_add_signed(delta),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before the file's start"))?;
        if position != self.position {
            self.position = position;
            self.placed = false;
        }
        Ok(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A disc whose volume sits `partition` bytes in, holding `default.xex`, `data/a.big`,
    /// `data/Sub/b.big` and a `$SystemUpdate` folder.
    pub(crate) fn disc(partition: u64) -> Vec<u8> {
        let mut image = vec![0_u8; usize::try_from(partition).unwrap() + 64 * SECTOR as usize];
        let base = usize::try_from(partition).unwrap();
        let sector = |n: usize| base + n * SECTOR as usize;
        let volume = sector(32);
        image[volume..volume + 20].copy_from_slice(MAGIC);
        let node = |image: &mut Vec<u8>, at: usize, left: u16, right: u16, sector: u32, size: u32, attributes: u8, name: &str| {
            image[at..at + 2].copy_from_slice(&left.to_le_bytes());
            image[at + 2..at + 4].copy_from_slice(&right.to_le_bytes());
            image[at + 4..at + 8].copy_from_slice(&sector.to_le_bytes());
            image[at + 8..at + 12].copy_from_slice(&size.to_le_bytes());
            image[at + 12] = attributes;
            image[at + 13] = u8::try_from(name.len()).unwrap();
            image[at + 14..at + 14 + name.len()].copy_from_slice(name.as_bytes());
        };
        // Root at sector 40: data (left 8 -> default.xex, right 16 -> $SystemUpdate).
        image[volume + 20..volume + 24].copy_from_slice(&40_u32.to_le_bytes());
        image[volume + 24..volume + 28].copy_from_slice(&96_u32.to_le_bytes());
        node(&mut image, sector(40), 8, 16, 41, 64, DIRECTORY, "data");
        node(&mut image, sector(40) + 32, 0, 0, 50, 5, 0x80, "default.xex");
        node(&mut image, sector(40) + 64, 0, 0, 43, 16, DIRECTORY, "$SystemUpdate");
        // data at sector 41: a.big (right 8 -> Sub).
        node(&mut image, sector(41), 0, 8, 51, 3, 0x80, "a.big");
        node(&mut image, sector(41) + 32, 0, 0, 42, 32, DIRECTORY, "Sub");
        node(&mut image, sector(42), 0, 0, 52, 2, 0x80, "b.big");
        node(&mut image, sector(43), 0, 0, 53, 4, 0x80, "big.bin");
        image[sector(50)..sector(50) + 5].copy_from_slice(b"XEX2!");
        image[sector(51)..sector(51) + 3].copy_from_slice(b"abc");
        image[sector(52)..sector(52) + 2].copy_from_slice(b"bb");
        image
    }

    #[test]
    fn finds_the_volume_at_each_partition_and_walks_the_tree() {
        // A plain image, and a whole XGD3 disc (the smallest real offset).
        for partition in [0, 0x0208_0000] {
            let image = Image::read(Path::new("game.iso"), io::Cursor::new(disc(partition))).unwrap();
            let Node::Directory(root) = &image.root else { panic!("root") };
            let names: Vec<&str> = root.values().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, ["$SystemUpdate", "data", "default.xex"]);
            let b = image.node(&["data".into(), "sub".into(), "b.big".into()]);
            assert!(matches!(b, Some(&Node::File { offset, size: 2 }) if offset == partition + 52 * SECTOR));
        }
    }

    #[test]
    fn paths_through_an_image_read_its_files_in_place() {
        let folder = std::env::temp_dir().join(format!("disc-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("Game.ISO");
        std::fs::write(&path, disc(0)).unwrap();
        assert_eq!(find(&path), Some(Found::Directory));
        assert_eq!(find(&path.join("DATA").join("sub")), Some(Found::Directory));
        assert_eq!(find(&path.join("data").join("missing.big")), Some(Found::Missing));
        assert_eq!(find(&folder.join("other.txt")), None);
        let mut file = open(&path.join("data/A.BIG")).unwrap().unwrap();
        assert_eq!(file.len(), 3);
        let mut text = String::new();
        file.read_to_string(&mut text).unwrap();
        assert_eq!(text, "abc");
        file.seek(SeekFrom::Start(1)).unwrap();
        let mut one = [0; 4];
        assert_eq!(file.read(&mut one).unwrap(), 2);
        assert_eq!(&one[..2], b"bc");
        assert_eq!(file.seek(SeekFrom::End(-1)).unwrap(), 2);
        let mut names = entries(&path.join("data")).unwrap().unwrap();
        names.sort();
        assert_eq!(names, [("Sub".to_owned(), true), ("a.big".to_owned(), false)]);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// With `SKATE_DISC_IMAGE` naming a game's disc image, its `data\big` archives open and read
    /// in place.
    #[test]
    #[ignore = "requires an external disc image"]
    fn a_real_image_opens_its_archives_in_place() {
        let image = PathBuf::from(std::env::var_os("SKATE_DISC_IMAGE").unwrap());
        let started = std::time::Instant::now();
        assert_eq!(find(&image), Some(Found::Directory));
        assert!(crate::host_path::is_file(&image.join("default.xex")));
        let mut archives = 0;
        for (path, folder) in crate::host_path::read_dir(&image.join("data").join("big")).unwrap() {
            if folder || !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("big")) {
                continue;
            }
            let mounted = crate::mount::MountedBig::open(&path, true).unwrap();
            if let Some(entry) = mounted.archive().entries.first() {
                mounted.read_entry(entry).unwrap();
            }
            archives += 1;
        }
        assert!(archives > 0);
        eprintln!("{archives} archives opened and read in {:?}", started.elapsed());
    }
}
