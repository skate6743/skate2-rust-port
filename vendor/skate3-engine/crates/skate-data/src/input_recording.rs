//! Standalone recorder JSONL. Capture timestamps are host observations, not
//! original-game simulation frames. Replay holds the latest observed raw state.
use serde::Deserialize;
use skate_core::input::xbox::XboxState;
use std::{io::BufRead, path::Path};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    schema: u32,
    pub t_us: u64,
    pub sample: u64,
    pub connected: bool,
    pub packet: Option<u32>,
    buttons: u16,
    triggers: [u8; 2],
    left: [i16; 2],
    right: [i16; 2],
}
impl Sample {
    pub fn raw_state(&self) -> XboxState {
        XboxState {
            buttons: self.buttons,
            triggers: self.triggers,
            left: self.left,
            right: self.right,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Recording {
    samples: Vec<Sample>,
}
impl Recording {
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Input recording {}: {e}", path.display()))?;
        Self::read(std::io::BufReader::new(file))
    }
    pub fn read(reader: impl BufRead) -> Result<Self, String> {
        let mut samples: Vec<Sample> = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Input recording line {}: {e}", index + 1))?;
            if line.trim().is_empty() {
                continue;
            }
            let sample: Sample = serde_json::from_str(&line)
                .map_err(|e| format!("Input recording line {}: {e}", index + 1))?;
            if sample.schema != 1 || sample.sample != samples.len() as u64 {
                return Err(format!(
                    "Input recording line {}: unsupported schema or missing/out-of-order sample",
                    index + 1
                ));
            }
            if samples
                .last()
                .is_some_and(|previous| sample.t_us <= previous.t_us)
            {
                return Err(format!(
                    "Input recording line {}: timestamps must increase",
                    index + 1
                ));
            }
            if sample.connected != sample.packet.is_some() {
                return Err(format!(
                    "Input recording line {}: connection and packet disagree",
                    index + 1
                ));
            }
            samples.push(sample);
        }
        if samples.is_empty() {
            return Err("Input recording is empty".into());
        }
        Ok(Self { samples })
    }
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }
    pub fn duration_us(&self) -> u64 {
        self.samples.last().unwrap().t_us - self.samples[0].t_us
    }
    /// Zero is the first captured sample. No axis interpolation, invented edges,
    /// or rescaling by sample count; the final sample remains until playback ends.
    pub fn at_elapsed_us(&self, elapsed: u64) -> &Sample {
        let timestamp = self.samples[0].t_us.saturating_add(elapsed);
        let next = self
            .samples
            .partition_point(|sample| sample.t_us <= timestamp);
        &self.samples[next - 1]
    }
}
