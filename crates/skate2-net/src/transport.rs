//! The connection layer under the online protocol, in place of the original's DirtySock stack
//! (ProtoTunnel, CommUDP and NetGameLink): datagrams of at most [`MAX_DATAGRAM`] bytes, sealed
//! once the handshake has agreed the connection's keys ([`crate::crypto`]), a reliable channel
//! delivered exactly once and in order (CommUDP's cumulative acks and resends), and an
//! unreliable channel for the replay frames, which carry their own tick and need neither.

use crate::codec::{DecodeError, Reader, Writer};
use crate::crypto::{self, KEY_BYTES, PROOF_BYTES, Session};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

/// Bytes identifying the protocol at the start of every datagram.
pub const MAGIC: u16 = 0x5332;
/// Raised whenever the wire format changes; a mismatch refuses the connection.
pub const PROTOCOL_VERSION: u8 = 12;
/// The largest datagram either side sends, below common path MTUs so no datagram fragments.
pub const MAX_DATAGRAM: usize = 1200;
/// The largest reliable fragment; longer messages are split.
pub const MAX_FRAGMENT: usize = 1000;
/// The largest unreliable message.
pub const MAX_UNRELIABLE: usize = MAX_PLAIN - HEADER_BYTES - 4;
/// Reliable fragments in flight at once, and the receiver's reorder window.
pub const WINDOW: u32 = 512;
/// Silence after which a connection is considered lost, and how long reliable data may wait for
/// the peer to take it.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// Idle time after which an empty datagram keeps the connection and its acks alive.
pub const KEEPALIVE: Duration = Duration::from_millis(250);
/// How long an acknowledgement waits for a datagram to ride on (a little over a run of replay
/// frames at 20 a second) before it goes on its own.
pub const ACK_DELAY: Duration = Duration::from_millis(60);

const HEADER_BYTES: usize = 12;
/// The room of a datagram before it is sealed.
const MAX_PLAIN: usize = MAX_DATAGRAM - crypto::SEAL_BYTES;
/// The bytes of a sealed datagram that stay readable: magic, version, kind and connection.
const CLEAR_BYTES: usize = 8;
const HALF_RANGE: u32 = 1 << 31;
/// The largest reliable message either side sends: a recipe with its header.
const LARGEST_MESSAGE: usize = crate::protocol::MAX_RECIPE + 1024;
const MIN_RESEND: Duration = Duration::from_millis(40);
const MAX_RESEND: Duration = Duration::from_millis(600);
const MAX_PACKETS_PER_POLL: usize = 64;

/// Datagram kinds.
pub mod kind {
    /// Client to server: `nonce u64`, the client's handshake key, its build proof; repeated
    /// until accepted or refused.
    pub const CONNECT: u8 = 0;
    /// Server to client: `nonce u64`, `connection u32`, the server's handshake key, its build
    /// proof.
    pub const ACCEPT: u8 = 1;
    /// Server to client: `nonce u64`, `reason u8`.
    pub const REJECT: u8 = 2;
    /// Either way: `connection u32`, then sealed: `ack u32` and chunks.
    pub const DATA: u8 = 3;
    /// Either way: `connection u32`, then sealed: `reason u8`.
    pub const DISCONNECT: u8 = 4;
}

const CHUNK_RELIABLE: u8 = 1;
const CHUNK_UNRELIABLE: u8 = 2;
const FRAGMENT_LAST: u8 = 1;

/// The fixed start of every datagram.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub kind: u8,
}

/// Reads a datagram's magic, version and kind; `None` for anything that is not ours.
#[must_use]
pub fn read_header(datagram: &[u8]) -> Option<(Header, Reader<'_>)> {
    let mut reader = Reader::new(datagram);
    if reader.u16().ok()? != MAGIC {
        return None;
    }
    let version = reader.u8().ok()?;
    let kind = reader.u8().ok()?;
    Some((Header { version, kind }, reader))
}

#[must_use]
pub fn write_header(kind: u8) -> Writer {
    let mut writer = Writer::with_capacity(MAX_DATAGRAM);
    writer.u16(MAGIC);
    writer.u8(PROTOCOL_VERSION);
    writer.u8(kind);
    writer
}

#[must_use]
pub fn connect_datagram(nonce: u64, key: &[u8; KEY_BYTES], proof: &[u8; PROOF_BYTES]) -> Vec<u8> {
    let mut writer = write_header(kind::CONNECT);
    writer.u64(nonce);
    writer.raw(key);
    writer.raw(proof);
    writer.into_vec()
}

#[must_use]
pub fn accept_datagram(nonce: u64, connection: u32, key: &[u8; KEY_BYTES], proof: &[u8; PROOF_BYTES]) -> Vec<u8> {
    let mut writer = write_header(kind::ACCEPT);
    writer.u64(nonce);
    writer.u32(connection);
    writer.raw(key);
    writer.raw(proof);
    writer.into_vec()
}

/// A handshake key read from a datagram.
///
/// # Errors
/// The datagram ends first.
pub fn read_key(reader: &mut Reader<'_>) -> Result<[u8; KEY_BYTES], DecodeError> {
    let mut key = [0; KEY_BYTES];
    key.copy_from_slice(reader.raw(KEY_BYTES)?);
    Ok(key)
}

#[must_use]
pub fn reject_datagram(nonce: u64, reason: u8) -> Vec<u8> {
    let mut writer = write_header(kind::REJECT);
    writer.u64(nonce);
    writer.u8(reason);
    writer.into_vec()
}

/// Why a datagram from a peer was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointError {
    Decode(DecodeError),
    /// The peer acknowledged messages that were never sent.
    BadAck,
    /// A reliable fragment outside the receive window.
    Window,
    /// A reliable message grew past the largest one allowed.
    Oversized,
}

impl From<DecodeError> for EndpointError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl std::fmt::Display for EndpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(f),
            Self::BadAck => f.write_str("acknowledgement of unsent data"),
            Self::Window => f.write_str("reliable fragment outside the window"),
            Self::Oversized => f.write_str("reliable message too large"),
        }
    }
}

impl std::error::Error for EndpointError {}

#[derive(Debug)]
struct Pending {
    id: u32,
    last: bool,
    data: Vec<u8>,
    first_sent: Option<Instant>,
    sent_at: Option<Instant>,
    sends: u32,
}

#[derive(Debug)]
struct Received {
    last: bool,
    data: Vec<u8>,
}

/// Traffic counters, for the server's log and the client's diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub datagrams_sent: u64,
    pub datagrams_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub resent_fragments: u64,
}

/// One side of an established connection.
#[derive(Debug)]
pub struct Endpoint {
    connection: u32,
    next_id: u32,
    pending: VecDeque<Pending>,
    expected: u32,
    reordered: BTreeMap<u32, Received>,
    assembling: Vec<u8>,
    reliable_in: VecDeque<Vec<u8>>,
    unreliable_out: VecDeque<Vec<u8>>,
    /// Unreliable messages that ride on the next datagram (acknowledgements), and since when.
    deferred_out: VecDeque<Vec<u8>>,
    deferred_since: Option<Instant>,
    unreliable_in: VecDeque<Vec<u8>>,
    /// Since when a received reliable fragment waits for its acknowledgement.
    ack_due: Option<Instant>,
    srtt: Duration,
    rttvar: Duration,
    last_received: Instant,
    last_sent: Instant,
    stats: Stats,
    session: Session,
}

impl Endpoint {
    #[must_use]
    pub fn new(connection: u32, session: Session, now: Instant) -> Self {
        Self {
            connection,
            next_id: 0,
            pending: VecDeque::new(),
            expected: 0,
            reordered: BTreeMap::new(),
            assembling: Vec::new(),
            reliable_in: VecDeque::new(),
            unreliable_out: VecDeque::new(),
            deferred_out: VecDeque::new(),
            deferred_since: None,
            unreliable_in: VecDeque::new(),
            ack_due: None,
            srtt: Duration::from_millis(100),
            rttvar: Duration::from_millis(50),
            last_received: now,
            last_sent: now,
            stats: Stats::default(),
            session,
        }
    }

    #[must_use]
    pub fn connection(&self) -> u32 {
        self.connection
    }

    #[must_use]
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// The smoothed round trip of acknowledged reliable fragments.
    #[must_use]
    pub fn rtt(&self) -> Duration {
        self.srtt
    }

    /// Reliable fragments sent but not yet acknowledged.
    #[must_use]
    pub fn unacknowledged(&self) -> usize {
        self.pending.len()
    }

    /// Whether the peer is gone: silent for [`TIMEOUT`], or keeping the connection alive while it
    /// takes none of the reliable data sent that long ago, which would otherwise pile up.
    #[must_use]
    pub fn timed_out(&self, now: Instant) -> bool {
        let waiting = self.pending.front().and_then(|pending| pending.first_sent);
        now.saturating_duration_since(self.last_received) >= TIMEOUT || waiting.is_some_and(|sent| now.saturating_duration_since(sent) >= TIMEOUT)
    }

    /// Queues a message for exactly-once, in-order delivery.
    pub fn send_reliable(&mut self, message: &[u8]) {
        let mut chunks = message.chunks(MAX_FRAGMENT).peekable();
        if chunks.peek().is_none() {
            self.push_fragment(Vec::new(), true);
            return;
        }
        while let Some(chunk) = chunks.next() {
            self.push_fragment(chunk.to_vec(), chunks.peek().is_none());
        }
    }

    fn push_fragment(&mut self, data: Vec<u8>, last: bool) {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.pending.push_back(Pending { id, last, data, first_sent: None, sent_at: None, sends: 0 });
    }

    /// Queues a message that may be lost; dropped when it cannot fit one datagram.
    pub fn send_unreliable(&mut self, message: Vec<u8>) -> bool {
        if message.len() > MAX_UNRELIABLE {
            return false;
        }
        self.unreliable_out.push_back(message);
        true
    }

    /// Queues a message that may be lost and need not go at once (an acknowledgement): it rides
    /// on the next datagram sent, or goes on its own after [`ACK_DELAY`].
    pub fn send_deferred(&mut self, message: Vec<u8>, now: Instant) -> bool {
        if message.len() > MAX_UNRELIABLE {
            return false;
        }
        self.deferred_out.push_back(message);
        self.deferred_since.get_or_insert(now);
        true
    }

    pub fn take_reliable(&mut self) -> Option<Vec<u8>> {
        self.reliable_in.pop_front()
    }

    pub fn take_unreliable(&mut self) -> Option<Vec<u8>> {
        self.unreliable_in.pop_front()
    }

    /// A sealed `DATA` datagram from the peer: `Ok(false)` when it does not open (not this
    /// connection's, altered, or replayed) and is dropped.
    ///
    /// # Errors
    /// A malformed body, which the caller treats as a broken connection.
    pub fn receive_datagram(&mut self, datagram: &[u8], now: Instant) -> Result<bool, EndpointError> {
        let Some(plain) = self.session.open(datagram, CLEAR_BYTES) else { return Ok(false) };
        self.count_received(datagram.len());
        self.receive(&mut Reader::new(&plain), now)?;
        Ok(true)
    }

    /// The sealed `DISCONNECT` datagram telling the peer why the connection ends; `None` once
    /// the connection's counter is spent.
    pub fn disconnect_datagram(&mut self, reason: u8) -> Option<Vec<u8>> {
        let mut writer = write_header(kind::DISCONNECT);
        writer.u32(self.connection);
        writer.u8(reason);
        self.session.seal(&writer.into_vec(), CLEAR_BYTES)
    }

    /// The reason a sealed `DISCONNECT` datagram from the peer gives; `None` for one that does
    /// not open.
    pub fn open_disconnect(&mut self, datagram: &[u8]) -> Option<u8> {
        self.session.open(datagram, CLEAR_BYTES)?.first().copied()
    }

    /// A `DATA` datagram's plaintext body: `ack u32`, then chunks.
    fn receive(&mut self, body: &mut Reader<'_>, now: Instant) -> Result<(), EndpointError> {
        let ack = body.u32()?;
        self.acknowledge(ack, now)?;
        while !body.is_empty() {
            match body.u8()? {
                CHUNK_RELIABLE => {
                    let id = body.u32()?;
                    let flags = body.u8()?;
                    let length = body.len(MAX_FRAGMENT)?;
                    let data = body.raw(length)?.to_vec();
                    self.ack_due.get_or_insert(now);
                    self.receive_fragment(id, flags & FRAGMENT_LAST != 0, data)?;
                }
                CHUNK_UNRELIABLE => {
                    let length = body.len(MAX_UNRELIABLE)?;
                    self.unreliable_in.push_back(body.raw(length)?.to_vec());
                }
                _ => return Err(DecodeError("unknown chunk").into()),
            }
        }
        self.last_received = now;
        Ok(())
    }

    fn count_received(&mut self, bytes: usize) {
        self.stats.datagrams_received += 1;
        self.stats.bytes_received += bytes as u64;
    }

    fn acknowledge(&mut self, ack: u32, now: Instant) -> Result<(), EndpointError> {
        let oldest = self.pending.front().map_or(self.next_id, |pending| pending.id);
        let newly = ack.wrapping_sub(oldest);
        if newly > self.next_id.wrapping_sub(oldest) {
            // Behind the oldest pending fragment is a late datagram; ahead of the next id is
            // data that was never sent.
            return if oldest.wrapping_sub(ack) < HALF_RANGE { Ok(()) } else { Err(EndpointError::BadAck) };
        }
        for _ in 0..newly {
            let Some(pending) = self.pending.pop_front() else { break };
            if pending.sends == 1
                && let Some(sent) = pending.sent_at
            {
                self.sample_rtt(now.saturating_duration_since(sent));
            }
        }
        Ok(())
    }

    fn sample_rtt(&mut self, sample: Duration) {
        let difference = self.srtt.abs_diff(sample);
        self.rttvar = (self.rttvar * 3 + difference) / 4;
        self.srtt = (self.srtt * 7 + sample) / 8;
    }

    fn receive_fragment(&mut self, id: u32, last: bool, data: Vec<u8>) -> Result<(), EndpointError> {
        let offset = id.wrapping_sub(self.expected);
        if offset >= WINDOW {
            // Behind the window is a resend of delivered data whose ack was lost.
            return if self.expected.wrapping_sub(id) < HALF_RANGE { Ok(()) } else { Err(EndpointError::Window) };
        }
        self.reordered.entry(id).or_insert(Received { last, data });
        while let Some(received) = self.reordered.remove(&self.expected) {
            self.expected = self.expected.wrapping_add(1);
            if self.assembling.len() + received.data.len() > LARGEST_MESSAGE {
                return Err(EndpointError::Oversized);
            }
            self.assembling.extend_from_slice(&received.data);
            if received.last {
                self.reliable_in.push_back(std::mem::take(&mut self.assembling));
            }
        }
        Ok(())
    }

    fn resend_interval(&self) -> Duration {
        (self.srtt + self.rttvar * 4).clamp(MIN_RESEND, MAX_RESEND)
    }

    /// The datagrams due now: new and expired reliable fragments, queued unreliable messages,
    /// and an acknowledgement or keepalive when nothing else would carry one.
    pub fn poll(&mut self, now: Instant) -> Vec<Vec<u8>> {
        let resend = self.resend_interval();
        let mut datagrams = Vec::new();
        let mut current: Option<Writer> = None;
        let open = |connection: u32, ack: u32| {
            let mut writer = write_header(kind::DATA);
            writer.u32(connection);
            writer.u32(ack);
            writer
        };
        let mut resent = 0_u64;
        // The pending fragments are consecutive from the oldest unacknowledged one; the
        // receiver buffers at most a window of them.
        for pending in self.pending.iter_mut().take(WINDOW as usize) {
            let due = pending.sent_at.is_none_or(|sent| now.saturating_duration_since(sent) >= resend);
            if !due {
                continue;
            }
            let size = 1 + 4 + 1 + 3 + pending.data.len();
            if current.as_ref().is_some_and(|writer| writer.len() + size > MAX_PLAIN) {
                datagrams.push(current.take().expect("open datagram").into_vec());
                if datagrams.len() >= MAX_PACKETS_PER_POLL {
                    break;
                }
            }
            let writer = current.get_or_insert_with(|| open(self.connection, self.expected));
            writer.u8(CHUNK_RELIABLE);
            writer.u32(pending.id);
            writer.u8(if pending.last { FRAGMENT_LAST } else { 0 });
            writer.bytes(&pending.data);
            if pending.sent_at.is_some() {
                resent += 1;
            }
            pending.first_sent.get_or_insert(now);
            pending.sent_at = Some(now);
            pending.sends += 1;
        }
        self.stats.resent_fragments += resent;
        let deferred_due = self.deferred_since.is_some_and(|since| now.saturating_duration_since(since) >= ACK_DELAY);
        if !self.deferred_out.is_empty() && (current.is_some() || !datagrams.is_empty() || !self.unreliable_out.is_empty() || deferred_due) {
            self.unreliable_out.extend(self.deferred_out.drain(..));
            self.deferred_since = None;
        }
        while let Some(message) = self.unreliable_out.pop_front() {
            let size = 1 + 3 + message.len();
            if current.as_ref().is_some_and(|writer| writer.len() + size > MAX_PLAIN) {
                datagrams.push(current.take().expect("open datagram").into_vec());
            }
            if datagrams.len() >= MAX_PACKETS_PER_POLL {
                self.unreliable_out.clear();
                break;
            }
            let writer = current.get_or_insert_with(|| open(self.connection, self.expected));
            writer.u8(CHUNK_UNRELIABLE);
            writer.bytes(&message);
        }
        if let Some(writer) = current.take() {
            datagrams.push(writer.into_vec());
        }
        let ack_due = self.ack_due.is_some_and(|since| now.saturating_duration_since(since) >= ACK_DELAY);
        if datagrams.is_empty() && (ack_due || now.saturating_duration_since(self.last_sent) >= KEEPALIVE) {
            datagrams.push(open(self.connection, self.expected).into_vec());
        }
        let datagrams: Vec<Vec<u8>> = datagrams.iter().filter_map(|datagram| self.session.seal(datagram, CLEAR_BYTES)).collect();
        if !datagrams.is_empty() {
            self.ack_due = None;
            self.last_sent = now;
            self.stats.datagrams_sent += datagrams.len() as u64;
            self.stats.bytes_sent += datagrams.iter().map(|datagram| datagram.len() as u64).sum::<u64>();
        }
        datagrams
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two ends of one connection.
    fn connected(connection: u32, now: Instant) -> (Endpoint, Endpoint) {
        let (a, b) = Session::pair();
        (Endpoint::new(connection, a, now), Endpoint::new(connection, b, now))
    }

    /// Delivers every datagram from `from` to `to`, dropping those `lose` picks.
    fn pump(from: &mut Endpoint, to: &mut Endpoint, now: Instant, lose: &mut impl FnMut(usize) -> bool) {
        for (index, datagram) in from.poll(now).into_iter().enumerate() {
            if lose(index) {
                continue;
            }
            let (header, mut reader) = read_header(&datagram).expect("our datagram");
            assert_eq!(header.kind, kind::DATA);
            assert_eq!(reader.u32().unwrap(), to.connection());
            assert!(to.receive_datagram(&datagram, now).unwrap());
        }
    }

    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "byte patterns")]
    fn reliable_messages_arrive_once_in_order_under_loss() {
        let start = Instant::now();
        let (mut a, mut b) = connected(9, start);
        let messages: Vec<Vec<u8>> = (0..200_u32)
            .map(|i| (0..(i as usize * 37) % 3000).map(|j| (i as usize + j) as u8).collect())
            .collect();
        for message in &messages {
            a.send_reliable(message);
        }
        let mut seed = 12345_u32;
        let mut lose = |_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            seed % 100 < 30
        };
        let mut received = Vec::new();
        for step in 0..2000_u64 {
            let now = start + Duration::from_millis(step * 10);
            pump(&mut a, &mut b, now, &mut lose);
            pump(&mut b, &mut a, now, &mut lose);
            while let Some(message) = b.take_reliable() {
                received.push(message);
            }
            if received.len() == messages.len() && a.unacknowledged() == 0 {
                break;
            }
        }
        assert_eq!(received, messages);
        assert_eq!(a.unacknowledged(), 0);
    }

    #[test]
    fn unreliable_messages_pack_into_datagrams() {
        let now = Instant::now();
        let (mut a, mut b) = connected(1, now);
        for i in 0..20_u8 {
            assert!(a.send_unreliable(vec![i; 200]));
        }
        assert!(!a.send_unreliable(vec![0; MAX_DATAGRAM]));
        assert!(a.send_unreliable(vec![7; MAX_UNRELIABLE]));
        let datagrams = a.poll(now);
        assert!(datagrams.iter().all(|datagram| datagram.len() <= MAX_DATAGRAM));
        assert_eq!(datagrams.len(), 5);
        for datagram in datagrams {
            assert!(b.receive_datagram(&datagram, now).unwrap());
        }
        let mut count = 0;
        while b.take_unreliable().is_some() {
            count += 1;
        }
        assert_eq!(count, 21);
    }

    #[test]
    fn foreign_and_replayed_datagrams_are_dropped() {
        let now = Instant::now();
        let (mut a, mut b) = connected(4, now);
        let (mut stranger, _) = connected(4, now);
        a.send_reliable(b"ours");
        stranger.send_reliable(b"forged");
        let ours = a.poll(now);
        let forged = stranger.poll(now);
        assert!(!b.receive_datagram(&forged[0], now).unwrap(), "another connection's keys do not open");
        assert!(b.receive_datagram(&ours[0], now).unwrap());
        assert!(!b.receive_datagram(&ours[0], now).unwrap(), "a replay is dropped");
        assert_eq!(b.take_reliable().as_deref(), Some(&b"ours"[..]));
        assert_eq!(b.take_reliable(), None);
        let goodbye = a.disconnect_datagram(7).expect("sealed");
        assert_eq!(stranger.open_disconnect(&goodbye), None);
        assert_eq!(b.open_disconnect(&goodbye), Some(7));
    }

    #[test]
    fn acknowledgements_ride_on_the_next_datagram() {
        let start = Instant::now();
        let (mut a, mut b) = connected(1, start);
        a.send_reliable(b"facts");
        for datagram in a.poll(start) {
            assert!(b.receive_datagram(&datagram, start).unwrap());
        }
        assert!(b.send_deferred(b"frame acks".to_vec(), start));
        assert!(b.poll(start).is_empty(), "the ack and the frame acks wait");
        assert!(b.send_unreliable(b"frame".to_vec()));
        assert_eq!(b.poll(start + Duration::from_millis(5)).len(), 1, "all three share the frame's datagram");
        b.send_deferred(b"more acks".to_vec(), start);
        assert!(b.poll(start + Duration::from_millis(10)).is_empty());
        assert_eq!(b.poll(start + ACK_DELAY).len(), 1, "alone after the delay");
    }

    #[test]
    fn a_peer_that_never_takes_the_reliable_data_times_out() {
        let start = Instant::now();
        let (mut a, mut b) = connected(1, start);
        a.send_reliable(b"facts");
        let mut now = start;
        while now < start + TIMEOUT {
            assert!(!a.timed_out(now));
            // Every datagram to b is lost; b's keepalives reach a.
            a.poll(now);
            for datagram in b.poll(now) {
                assert!(a.receive_datagram(&datagram, now).unwrap());
            }
            now += KEEPALIVE;
        }
        assert!(a.timed_out(now), "keepalives do not keep reliable data waiting forever");
    }

    #[test]
    fn idle_endpoint_sends_keepalives() {
        let start = Instant::now();
        let (mut a, _) = connected(1, start);
        assert!(a.poll(start).is_empty());
        assert_eq!(a.poll(start + KEEPALIVE).len(), 1);
        assert!(a.timed_out(start + TIMEOUT));
    }
}
