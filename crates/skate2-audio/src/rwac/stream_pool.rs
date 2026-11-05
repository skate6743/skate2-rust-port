//! `rw::audio::core::StreamPool` (`CreateInstance` 0x82be9190, `GetInstance` 0x82be3240,
//! `AcquireStream` 0x82be3288, `ReleaseStream` 0x82be3570, `Release` 0x82bea900) and the
//! `rw::core::filesys::Stream` each pool stream owns, as `SndPlayer1` uses them for streamed
//! samples. A stream reads its `QueueFile` requests in order on its own thread, cutting each file
//! into chunks with the request's parser and keeping at most the pool's `bufferBytes` of chunks the
//! player has not released; `GetChunk` hands out the loaded chunks in order and nothing while the
//! next one is still loading. The read thread stands in for the 360's asynchronous reads, so when
//! a chunk becomes ready differs from the console. Stealing a stream from a lower priority
//! (`StreamLostCallback`) is not ported: every pool the port creates has one user at a time.

use std::collections::VecDeque;
use skate2_formats::host_path::{self, GameFile};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use skate2_formats::big::{BigArchive, BigEntryKey};
use skate2_formats::mount::MountedBig;

use super::decoder::DataRef;

/// The host's files (`rw::core::filesys` and `xcore::FileLoader`): game paths, with `|` between a
/// `.big` archive and an entry in it.
pub trait StreamFiles: Send + Sync {
    /// The file's size, `None` when it does not exist.
    fn size(&self, path: &str) -> Option<u64>;
    /// Fills `buf` from `offset`; false when the file is shorter or cannot be read.
    fn read_at(&self, path: &str, offset: u64, buf: &mut [u8]) -> bool;
}

/// The parser `QueueFile` cuts a request's file with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkParser {
    /// `SndPlayer1::Format0ChunkParser` 0x82bf2140: a big-endian size word whose top bit marks
    /// the last chunk; the parser clears that bit in the buffer.
    Format0,
    /// `SndPlayer1::Format1ChunkParser` 0x82bf21c8: a kind byte and a 24-bit size; an 'E' chunk is
    /// the last.
    Format1,
}

/// `Stream::ParseReturnValue`.
enum Parse {
    NeedMore,
    Chunk(u32),
    Last(u32),
}

impl ChunkParser {
    /// The chunk at the start of `data`, of which `available` bytes are loaded.
    fn parse(self, data: &[u8; 8], available: u64) -> Parse {
        match self {
            Self::Format0 => {
                if available < 8 {
                    return Parse::NeedMore;
                }
                let word = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                let size = word & 0x7FFF_FFFF;
                if u64::from(size) > available {
                    Parse::NeedMore
                } else if word >> 31 == 1 {
                    Parse::Last(size)
                } else {
                    Parse::Chunk(size)
                }
            }
            Self::Format1 => {
                if available <= 4 {
                    return Parse::NeedMore;
                }
                let size = u32::from_be_bytes([0, data[1], data[2], data[3]]);
                if available < u64::from(size) {
                    Parse::NeedMore
                } else if data[0] == b'E' {
                    Parse::Last(size)
                } else {
                    Parse::Chunk(size)
                }
            }
        }
    }
}

/// The buffer space a handed-out chunk holds until `ReleaseChunk`.
#[derive(Debug, Clone, Copy)]
pub struct ChunkTicket {
    size: u32,
    generation: u32,
}

impl ChunkTicket {
    /// `ChunkInfo` +4: the chunk's size.
    #[must_use]
    pub fn size(self) -> u32 {
        self.size
    }
}

/// A `ChunkInfo` `GetChunk` handed out: the chunk's bytes (`+8`) and its ticket.
pub struct StreamChunk {
    pub data: DataRef,
    pub ticket: ChunkTicket,
}

/// One `QueueFile` request.
struct FileRequest {
    id: i32,
    path: String,
    offset: u64,
    parser: ChunkParser,
}

/// A chunk loaded and not yet handed out.
struct LoadedChunk {
    request: i32,
    data: DataRef,
    size: u32,
}

#[derive(Default)]
struct StreamState {
    /// Requests not read to their end, oldest first.
    requests: VecDeque<FileRequest>,
    loaded: VecDeque<LoadedChunk>,
    /// Bytes of chunks loaded and not released.
    held: u64,
    /// Bumped by `Kill`, so reads and tickets from before it are dropped.
    generation: u32,
    last_id: i32,
    closed: bool,
}

/// `rw::core::filesys::Stream`.
struct FileStream {
    state: Mutex<StreamState>,
    changed: Condvar,
    buffer_bytes: u64,
    files: Arc<dyn StreamFiles>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FileStream {
    fn queue_file(&self, path: &str, offset: u64, parser: ChunkParser) -> i32 {
        let mut state = lock(&self.state);
        state.last_id = if state.last_id == i32::MAX { 1 } else { state.last_id + 1 };
        let id = state.last_id;
        state.requests.push_back(FileRequest { id, path: path.to_owned(), offset, parser });
        self.changed.notify_all();
        id
    }

    fn get_chunk(&self) -> Option<StreamChunk> {
        let mut state = lock(&self.state);
        let chunk = state.loaded.pop_front()?;
        Some(StreamChunk { data: chunk.data, ticket: ChunkTicket { size: chunk.size, generation: state.generation } })
    }

    fn release_chunk(&self, ticket: ChunkTicket) {
        let mut state = lock(&self.state);
        if ticket.generation == state.generation {
            state.held = state.held.saturating_sub(u64::from(ticket.size));
            self.changed.notify_all();
        }
    }

    /// `GetRequestState(id) != REQUESTSTATE_FREE`: the request is still reading or has chunks
    /// waiting. A file that cannot be read frees its request.
    fn request_live(&self, id: i32) -> bool {
        let state = lock(&self.state);
        state.requests.iter().any(|request| request.id == id) || state.loaded.iter().any(|chunk| chunk.request == id)
    }

    /// `GetRequestState(id) == REQUESTSTATE_COMPLETED || GetState() == STATE_STOPPED`: the request
    /// was read to its end, or the stream stopped reading because its buffer is full.
    fn request_buffered(&self, id: i32) -> bool {
        let state = lock(&self.state);
        !state.requests.iter().any(|request| request.id == id) || (state.held >= self.buffer_bytes && !state.requests.is_empty())
    }

    /// `Stream::Kill`: drops every request and chunk.
    fn kill(&self) {
        let mut state = lock(&self.state);
        state.requests.clear();
        state.loaded.clear();
        state.held = 0;
        state.generation = state.generation.wrapping_add(1);
        self.changed.notify_all();
    }

    /// The read thread: the next chunk of the oldest request while the buffer has room.
    fn run(&self) {
        loop {
            let (id, path, offset, parser, generation) = {
                let mut state = lock(&self.state);
                loop {
                    if state.closed {
                        return;
                    }
                    if state.held < self.buffer_bytes {
                        if let Some(request) = state.requests.front() {
                            break (request.id, request.path.clone(), request.offset, request.parser, state.generation);
                        }
                    }
                    state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
                }
            };
            let read = read_chunk(&*self.files, &path, offset, parser);
            let mut state = lock(&self.state);
            if state.generation != generation || state.requests.front().is_none_or(|request| request.id != id) {
                continue;
            }
            match read {
                None => {
                    state.requests.pop_front();
                }
                Some((bytes, size, last)) => {
                    if last {
                        state.requests.pop_front();
                    } else if let Some(request) = state.requests.front_mut() {
                        request.offset += u64::from(size);
                    }
                    state.held += u64::from(size);
                    state.loaded.push_back(LoadedChunk { request: id, data: DataRef { bytes: Arc::from(bytes), offset: 0 }, size });
                }
            }
        }
    }
}

/// One chunk of `path` at `offset`: its bytes, size and whether it ends the request. `None` when
/// the file ends before a whole chunk (or an empty one that cannot advance).
fn read_chunk(files: &dyn StreamFiles, path: &str, offset: u64, parser: ChunkParser) -> Option<(Vec<u8>, u32, bool)> {
    let available = files.size(path)?.checked_sub(offset)?;
    let mut header = [0u8; 8];
    let head = available.min(8) as usize;
    if !files.read_at(path, offset, &mut header[..head]) {
        return None;
    }
    let (size, last) = match parser.parse(&header, available) {
        Parse::NeedMore => return None,
        Parse::Chunk(size) => (size, false),
        Parse::Last(size) => (size, true),
    };
    if size == 0 && !last {
        return None;
    }
    let mut bytes = vec![0u8; size as usize];
    if !files.read_at(path, offset, &mut bytes) {
        return None;
    }
    if parser == ChunkParser::Format0 && last {
        if let Some(first) = bytes.first_mut() {
            *first &= 0x7F;
        }
    }
    Some((bytes, size, last))
}

/// One stream of a pool and its read thread, which ends when the pool goes.
struct PoolStream {
    /// The `AcquireStream` context: the `SndPlayer1` that holds the stream.
    context: u64,
    ref_count: i16,
    in_use: bool,
    file: Arc<FileStream>,
}

impl PoolStream {
    fn new(guid: u32, files: Arc<dyn StreamFiles>, buffer_bytes: u64) -> Self {
        let file = Arc::new(FileStream { state: Mutex::new(StreamState::default()), changed: Condvar::new(), buffer_bytes, files });
        let reader = Arc::clone(&file);
        let name = format!("rwac stream {}", String::from_utf8_lossy(&guid.to_be_bytes()));
        if std::thread::Builder::new().name(name).spawn(move || reader.run()).is_err() {
            lock(&file.state).closed = true;
        }
        Self { context: 0, ref_count: 0, in_use: false, file }
    }
}

impl Drop for PoolStream {
    fn drop(&mut self) {
        lock(&self.file.state).closed = true;
        self.file.changed.notify_all();
    }
}

/// `rw::audio::core::StreamPool`.
pub struct StreamPool {
    guid: u32,
    /// +0x31: set by `Release`; no stream is handed out after it.
    released: bool,
    streams: Vec<PoolStream>,
}

impl StreamPool {
    /// `AcquireStream` 0x82be3288: the stream this context already holds (one more reference),
    /// else the first free one; `None` when every stream is held by another context.
    pub fn acquire(pool: &Arc<Mutex<Self>>, context: u64) -> Option<StreamHandle> {
        let mut guard = lock(pool);
        if guard.released {
            return None;
        }
        let index = match guard.streams.iter().position(|stream| stream.in_use && stream.context != 0 && stream.context == context) {
            Some(index) => index,
            None => {
                let index = guard.streams.iter().position(|stream| !stream.in_use)?;
                let stream = &mut guard.streams[index];
                stream.context = context;
                stream.in_use = true;
                index
            }
        };
        let stream = &mut guard.streams[index];
        stream.ref_count = stream.ref_count.wrapping_add(1);
        Some(StreamHandle { pool: Arc::clone(pool), index, file: Arc::clone(&stream.file) })
    }
}

/// A stream `AcquireStream` handed out (the `SndPlayer1` request's +0x24 and its
/// `rw::core::filesys::Stream` +0x28).
pub struct StreamHandle {
    pool: Arc<Mutex<StreamPool>>,
    index: usize,
    file: Arc<FileStream>,
}

impl StreamHandle {
    /// `Stream::QueueFile`: reads `path` from `offset` with `parser`; returns the request id.
    pub fn queue_file(&self, path: &str, offset: u64, parser: ChunkParser) -> i32 {
        self.file.queue_file(path, offset, parser)
    }

    /// `Stream::GetChunk`: the next loaded chunk, `None` while it is loading or nothing is left.
    #[must_use]
    pub fn get_chunk(&self) -> Option<StreamChunk> {
        self.file.get_chunk()
    }

    /// `Stream::ReleaseChunk`.
    pub fn release_chunk(&self, ticket: ChunkTicket) {
        self.file.release_chunk(ticket);
    }

    /// `Stream::GetRequestState(id) != REQUESTSTATE_FREE`.
    #[must_use]
    pub fn request_live(&self, id: i32) -> bool {
        self.file.request_live(id)
    }

    /// `GetRequestState(id) == REQUESTSTATE_COMPLETED || GetState() == STATE_STOPPED`.
    #[must_use]
    pub fn request_buffered(&self, id: i32) -> bool {
        self.file.request_buffered(id)
    }

    /// `StreamPool::ReleaseStream` 0x82be3570: the last reference kills the stream and frees it.
    pub fn release(self) {
        let mut pool = lock(&self.pool);
        let stream = &mut pool.streams[self.index];
        stream.ref_count = stream.ref_count.wrapping_sub(1);
        if stream.ref_count == 0 {
            self.file.kill();
            stream.in_use = false;
        }
    }
}

/// The pools `StreamPool::GetInstance` finds, and the host's files they read.
#[derive(Default)]
pub struct StreamPools {
    files: Option<Arc<dyn StreamFiles>>,
    /// Newest first.
    pools: Vec<Arc<Mutex<StreamPool>>>,
}

impl StreamPools {
    pub fn set_files(&mut self, files: Arc<dyn StreamFiles>) {
        self.files = Some(files);
    }

    /// The host's files, for `xcore::FileLoader::SyncLoad`.
    #[must_use]
    pub fn files(&self) -> Option<Arc<dyn StreamFiles>> {
        self.files.clone()
    }

    /// `StreamPool::CreateInstance` 0x82be9190: `num_streams` streams buffering `buffer_bytes`
    /// each. `None` without the host's files.
    pub fn create(&mut self, guid: u32, num_streams: usize, buffer_bytes: u64) -> Option<Arc<Mutex<StreamPool>>> {
        let files = self.files.clone()?;
        let streams = (0..num_streams).map(|_| PoolStream::new(guid, Arc::clone(&files), buffer_bytes)).collect();
        let pool = Arc::new(Mutex::new(StreamPool { guid, released: false, streams }));
        self.pools.insert(0, Arc::clone(&pool));
        Some(pool)
    }

    /// `StreamPool::Release` 0x82bea900: no longer found or handed out; it goes once the streams
    /// held from it are released.
    pub fn release(&mut self, pool: &Arc<Mutex<StreamPool>>) {
        lock(pool).released = true;
        self.pools.retain(|other| !Arc::ptr_eq(other, pool));
    }

    /// `StreamPool::GetInstance` 0x82be3240.
    #[must_use]
    pub fn instance(&self, guid: u32) -> Option<Arc<Mutex<StreamPool>>> {
        self.pools.iter().find(|pool| lock(pool).guid == guid).cloned()
    }
}

/// [`StreamFiles`] over a game directory. An `<archive>|<entry>` path reads the entry in place
/// through the archive's own table (streamed entries are stored raw).
pub struct GameStreamFiles {
    root: PathBuf,
    /// PC port: a linked Skate 3 data folder, whose files are named under [`SKATE3_FILES`].
    skate3_root: Option<PathBuf>,
    archives: Mutex<Vec<(String, Option<Arc<OpenArchive>>)>>,
}

/// PC port: the prefix of a path in a linked Skate 3 data folder.
pub const SKATE3_FILES: &str = "skate3:";

struct OpenArchive {
    file: Mutex<GameFile>,
    table: BigArchive,
}

impl GameStreamFiles {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root, skate3_root: None, archives: Mutex::new(Vec::new()) }
    }

    /// PC port: these files with a linked Skate 3 data folder's under [`SKATE3_FILES`].
    #[must_use]
    pub fn with_skate3(mut self, root: Option<PathBuf>) -> Self {
        self.skate3_root = root;
        self
    }

    fn local(&self, path: &str) -> PathBuf {
        let (root, path) = match (path.strip_prefix(SKATE3_FILES), self.skate3_root.as_ref()) {
            (Some(path), Some(root)) => (root, path),
            _ => (&self.root, path),
        };
        skate2_formats::host_path::on_disk(&root.join(path.replace('\\', "/")))
    }

    fn archive(&self, path: &str) -> Option<Arc<OpenArchive>> {
        let key = path.replace('\\', "/").to_ascii_lowercase();
        let mut archives = lock(&self.archives);
        if let Some((_, archive)) = archives.iter().find(|(name, _)| *name == key) {
            return archive.clone();
        }
        let archive = MountedBig::open(self.local(path), false).ok().and_then(|mounted| {
            let file = host_path::open(mounted.data_path()).ok()?;
            Some(Arc::new(OpenArchive { file: Mutex::new(file), table: mounted.archive().clone() }))
        });
        archives.push((key, archive.clone()));
        archive
    }

    /// The archive and the entry's offset and size for an `<archive>|<entry>` path.
    fn entry(&self, path: &str) -> Option<(Arc<OpenArchive>, u64, u64)> {
        let (archive, name) = path.split_once('|')?;
        let archive = self.archive(archive)?;
        let entry = archive
            .table
            .entries
            .iter()
            .find(|entry| matches!(&entry.key, BigEntryKey::Name(candidate) if candidate.eq_ignore_ascii_case(name)))?;
        let (offset, size) = (entry.offset, entry.stored_size);
        Some((archive, offset, size))
    }
}

impl StreamFiles for GameStreamFiles {
    fn size(&self, path: &str) -> Option<u64> {
        if path.contains('|') {
            self.entry(path).map(|(_, _, size)| size)
        } else {
            host_path::file_len(&self.local(path)).ok()
        }
    }

    fn read_at(&self, path: &str, offset: u64, buf: &mut [u8]) -> bool {
        if path.contains('|') {
            let Some((archive, start, size)) = self.entry(path) else { return false };
            if offset.checked_add(buf.len() as u64).is_none_or(|end| end > size) {
                return false;
            }
            let mut file = lock(&archive.file);
            file.seek(SeekFrom::Start(start + offset)).is_ok() && file.read_exact(buf).is_ok()
        } else {
            let Ok(mut file) = host_path::open(&self.local(path)) else { return false };
            file.seek(SeekFrom::Start(offset)).is_ok() && file.read_exact(buf).is_ok()
        }
    }
}
