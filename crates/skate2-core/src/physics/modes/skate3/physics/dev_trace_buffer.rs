//! Bounded development-only flight recorder. No disk writes during healthy ticks.
use std::collections::VecDeque;
pub const HISTORY: usize = 120;
pub const STAGES: usize = 32;
const LINE_BYTES: usize = 3500;
#[derive(Default)]
pub struct Buffer {
    pub tick_id: u64,
    pub history: VecDeque<String>,
    pub stages: VecDeque<String>,
    pub frozen: bool,
}
fn bounded(mut text: String) -> String {
    if text.len() > LINE_BYTES {
        let mut end = LINE_BYTES;
        while !text.is_char_boundary(end) { end -= 1; }
        text.truncate(end);
        text.push_str(" [truncated]");
    }
    text.replace(['\n','\r'], " ")
}
fn push(queue: &mut VecDeque<String>, entry: String, limit: usize) {
    if queue.len() == limit { queue.pop_front(); }
    queue.push_back(bounded(entry));
}
impl Buffer {
    pub fn tick(&mut self, entry: String) { if !self.frozen { push(&mut self.history, entry, HISTORY); } }
    pub fn stage(&mut self, entry: String) { if !self.frozen { push(&mut self.stages, entry, STAGES); } }
    pub fn freeze(&mut self, reason: &str) -> Option<Vec<String>> {
        if self.frozen { return None; }
        self.frozen = true;
        let mut lines = vec![bounded(format!("v=1 FIRST_FAILURE {reason}"))];
        lines.extend(self.history.iter().cloned());
        lines.extend(self.stages.iter().cloned());
        lines.push("END_PHYSICS_TRACE".into());
        Some(lines)
    }
}
#[cfg(any())]
mod tests {
    use super::*;
    #[test] fn bounded_history_retains_order_and_first_failure() {
        let mut b=Buffer::default();
        for i in 0..200 { b.tick(format!("tick={i}")); b.stage(format!("stage={i}")); }
        let dump=b.freeze("bad root").unwrap();
        assert_eq!(dump.len(),HISTORY+STAGES+2);
        assert_eq!(dump[1],"tick=80");
        assert_eq!(dump[HISTORY+1],"stage=168");
        b.tick("later".into()); b.stage("later".into());
        assert!(b.freeze("secondary error").is_none());
        assert_eq!(b.history.back().unwrap(),"tick=199");
    }
    #[test] fn unicode_and_newlines_cannot_break_report_records() {
        let mut b=Buffer::default(); b.stage("é\n".repeat(4000));
        let line=b.stages.front().unwrap();
        assert!(line.len()<4096); assert!(!line.contains('\n')); assert!(line.ends_with("[truncated]"));
    }
}
