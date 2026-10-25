//! Measures the frame coding over a `SKATE2_DIAGNOSTIC_ONLINE_FRAME_DUMP` file: the dump's ticks
//! in runs of `interval` (each ending in a sample, or with `every` each tick a sample and the
//! run's later ones carrying its first audio floats, as a watched skater's are; the dump keeps
//! every tick's pose), each run coded against what the server `delay` ticks behind would have
//! acknowledged, decoded there, coded again for a player whose acknowledgements are `delay` ticks
//! late and decoded to check it, with the bytes each part of the frame takes on each leg.
//!
//! `cargo run --release -p skate2-net --example frame_stats -- <dump> [delay] [interval] [every]`

use skate2_net::frames::{AckState, FrameReceiver, FrameSender, NetFrame, QuantFrame, encode_for, read_dump_record};

fn frames(path: &str) -> Vec<NetFrame> {
    let data = std::fs::read(path).expect("dump readable");
    let mut frames = Vec::new();
    let mut at = 0;
    while at + 4 <= data.len() {
        let length = u32::from_le_bytes(data[at..at + 4].try_into().expect("length")) as usize;
        at += 4;
        frames.push(read_dump_record(&data[at..at + length]).expect("record"));
        at += length;
    }
    frames
}

fn without(frame: &NetFrame, keep: &str) -> NetFrame {
    let mut frame = frame.clone();
    if keep != "header" {
        frame.header = [0; skate2_net::frames::HEADER_WORDS];
    }
    if keep != "pose" && keep != "pose+camera" {
        frame.pose = frame.pose.map(|pose| vec![[1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]; pose.len()]);
    }
    if keep != "twist" {
        frame.twist = [0.0; 4];
    }
    if keep != "pose+camera" {
        frame.camera.clear();
    }
    if keep != "audio" {
        frame.audio_floats.clear();
        frame.audio_words.clear();
    }
    if keep != "hud" {
        frame.hud.clear();
    }
    frame
}

/// The dump's ticks in runs ending at every `interval`-th tick, the pose kept on those alone, or
/// with `every` on every tick, a run's later ticks repeating its first audio floats. Ticks without
/// a pose in the dump (one made with a sample interval) end no run.
fn runs(frames: &[NetFrame], interval: u32, every: bool) -> Vec<Vec<NetFrame>> {
    let phase = frames.iter().find(|frame| frame.pose.is_some()).map_or(interval - 1, |frame| frame.tick % interval);
    let mut runs = Vec::new();
    let mut run: Vec<NetFrame> = Vec::new();
    for frame in frames {
        let mut frame = frame.clone();
        let ends = frame.tick % interval == phase && frame.pose.is_some();
        if !ends && !every {
            frame.pose = None;
        }
        if every && let Some(first) = run.first() {
            frame.audio_floats.clone_from(&first.audio_floats);
        }
        run.push(frame);
        if ends || run.len() == skate2_net::frames::MAX_RUN {
            runs.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    runs
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("dump path");
    let delay: u32 = args.next().and_then(|delay| delay.parse().ok()).unwrap_or(6);
    let interval: u32 = args.next().and_then(|interval| interval.parse().ok()).unwrap_or(3);
    let every = args.next().is_some_and(|every| every == "every");
    let frames = frames(&path);
    let ticks = frames.len();
    println!("{ticks} frames, delay {delay}, runs of {interval}{}", if every { ", every tick a sample" } else { "" });
    if std::env::var_os("SKATE2_FRAME_FIELDS").is_some() {
        field_report(&frames);
    }
    let bones = frames.iter().find_map(|frame| frame.pose.as_ref().map(Vec::len)).unwrap_or(0);
    println!("{bones} bones; camera {} values, audio {}+{}", frames[0].camera.len(), frames[0].audio_floats.len(), frames[0].audio_words.len());
    for part in ["all", "header", "pose", "pose+camera", "twist", "audio", "hud"] {
        let runs = runs(&frames.iter().map(|frame| if part == "all" { frame.clone() } else { without(frame, part) }).collect::<Vec<_>>(), interval, every);
        let mut sender = FrameSender::default();
        let mut server = FrameReceiver::default();
        let mut player = FrameReceiver::default();
        let mut player_acks = AckState::default();
        let mut sent: Vec<u32> = Vec::new();
        let (mut up, mut down) = (0usize, 0usize);
        for run in &runs {
            let first = run[0].tick;
            // The server, and the player it relays to, acknowledge what they held `delay` ticks
            // before this run.
            if let Some(&acked) = sent.iter().rev().find(|&&tick| tick + delay <= first) {
                sender.acknowledge(acked, u32::MAX);
                player_acks.update(acked, u32::MAX);
            }
            let encoded = sender.encode(run).expect("a run codes");
            // The run message: its type, tick, length, references and payload, in a chunk.
            up += encoded.payload.len() + 1 + 3 + 1 + 4 + 4;
            let decoded = server.decode(&encoded).expect("decodes").expect("baselines held");
            for (decoded, frame) in decoded.iter().zip(run) {
                assert_eq!(*decoded, QuantFrame::quantize(frame), "tick {} round trips", frame.tick);
            }
            let relayed = encode_for(&decoded, server.ring(), &player_acks).expect("the server codes it again");
            down += relayed.payload.len() + 1 + 3 + 1 + 4 + 4;
            assert_eq!(player.decode(&relayed).expect("decodes").expect("baselines held"), decoded, "the relayed run round trips");
            sent.extend(run.iter().map(|frame| frame.tick));
        }
        #[expect(clippy::cast_precision_loss, reason = "statistics")]
        let rate = |total: usize| total as f64 / ticks as f64 * 60.0 / 1024.0;
        #[expect(clippy::cast_precision_loss, reason = "statistics")]
        let per_run = up as f64 / runs.len() as f64;
        println!("{part:>11}: {per_run:7.1} bytes/run, {:6.2} KB/s to the server, {:6.2} KB/s relayed", rate(up), rate(down));
    }
}

/// Per-field change rates: `SKATE2_FRAME_FIELDS=1`.
#[allow(dead_code)]
pub fn field_report(frames: &[NetFrame]) {
    let n = frames.len().max(2) - 1;
    println!("header words: index changed% avg_xor_bits");
    for word in 0..skate2_net::frames::HEADER_WORDS {
        let mut changed = 0;
        let mut bits = 0u64;
        for pair in frames.windows(2) {
            let x = pair[0].header[word] ^ pair[1].header[word];
            if x != 0 {
                changed += 1;
                bits += u64::from(32 - x.leading_zeros());
            }
        }
        if changed > 0 {
            println!("  h{word:02}: {:5.1}% {:4.1} first {:08x}", changed as f64 * 100.0 / n as f64, bits as f64 / changed as f64, frames[0].header[word]);
        }
    }
    let count = frames[0].audio_floats.len();
    let mut lines = Vec::new();
    for value in 0..count {
        let mut changed = 0;
        let mut total = 0.0f64;
        let mut max = 0.0f32;
        for pair in frames.windows(2) {
            let (a, b) = (pair[0].audio_floats.get(value), pair[1].audio_floats.get(value));
            if let (Some(a), Some(b)) = (a, b) && a.to_bits() != b.to_bits() {
                changed += 1;
                total += f64::from((a - b).abs());
            }
            max = max.max(pair[1].audio_floats.get(value).copied().unwrap_or(0.0).abs());
        }
        if changed > 0 {
            lines.push(format!("f{value:03}:{:4.0}%/{:.4}/{max:.1}", changed as f64 * 100.0 / n as f64, total / f64::from(changed.max(1))));
        }
    }
    println!("audio floats changed (index:%/avg delta/max): {}", lines.join(" "));
    let mut lines = Vec::new();
    for value in 0..frames[0].audio_words.len() {
        let changed = frames.windows(2).filter(|pair| pair[0].audio_words.get(value) != pair[1].audio_words.get(value)).count();
        if changed > 0 {
            lines.push(format!("w{value:02}:{:.1}%", changed as f64 * 100.0 / n as f64));
        }
    }
    println!("audio words changed: {}", lines.join(" "));
    let changed = frames.windows(2).filter(|pair| pair[0].camera != pair[1].camera).count();
    println!("camera changed {:.1}%", changed as f64 * 100.0 / n as f64);
}
