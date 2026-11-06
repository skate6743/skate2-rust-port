//! PlayerGrabSpline82D73FC0/74270/740F8/749D0, distinct from possession.
//! Scene execution is concrete; retained results become visible only at Sync.
mod selection;
#[cfg(any())]
mod tests;
use crate::skate3_native::physics::offboard::grab_scene::Scene;
use skate3_core::player::offboard::{
    grab_scene::{Descriptor, Hit, Line, Query, Record, closest_point},
    ground_entry::{Frame, Vector},
    ground_query::QueryContext,
};

#[derive(Default)]
pub(crate) struct Owner {
    queries: Vec<Query>,
    query_result: Option<Vec<Record>>,
    pending: Vec<Record>,
    validated: Vec<Record>,
    validation: Option<Vec<Option<Hit>>>,
    requests: [Option<Descriptor>; 2],
    data: [Option<Record>; 2],
    data_ready: [bool; 2],
    /// The records last published to ProcessedPhysIn's GrabSpline payloads, retained like them
    /// when a publication clears only the readiness bit.
    published: [Option<Record>; 2],
    interactable_request: Option<[Line; 5]>,
    interactable_result: Option<Option<u32>>,
    pub(crate) interactable_latched: bool,
    pub(crate) flags_12836: u8,
    query_position: Vector,
    validation_position: Vector,
}

///None for a record means clear its publication bit, retaining previous payload.
///Nested object Option distinguishes no completion from completed no-object.
pub(crate) struct Publication {
    pub records: [Option<Record>; 2],
    pub object: Option<Option<u32>>,
}

impl Owner {
    pub(crate) fn query(&mut self, query: Query) {
        self.query_position = query.position;
        self.query_result = None; //8275FA78 clears ready before enqueue.
        self.queries.push(query);
    }

    pub(crate) fn request_primary(&mut self, descriptor: Descriptor) {
        self.requests[0] = Some(descriptor);
        self.data[0] = None;
        self.data_ready[0] = false;
    }

    ///8275FD00 captures five actual forward lines; query execution is deferred.
    pub(crate) fn request_interactable(&mut self, frame: Frame, context: QueryContext) {
        let lines = std::array::from_fn(|i| {
            let mut start = frame[3];
            start[1] += (i as f32) * f32::from_bits(0x3eb851eb);
            let end = std::array::from_fn(|axis| frame[2][axis].mul_add(2., start[axis]));
            Line {
                start,
                end,
                radius: 0.2,
                group: context.matching_id_2952,
                reject_flags: 0,
                source_pool_mask: 2,
                selection_flags: context.selection_flags_2948,
            }
        });
        //The native manager keeps only its first outstanding interactable batch.
        if self.interactable_request.is_none() {
            self.interactable_request = Some(lines);
        }
        self.interactable_result = None;
    }

    ///82760060: bounded queries, requested data, then interactable results.
    pub(crate) fn execute_queries(&mut self, scene: &Scene<'_>) -> Result<(), String> {
        for query in &self.queries {
            self.query_result = Some(scene.query(query)?);
        }
        self.queries.clear();
        for i in 0..2 {
            if let Some(descriptor) = self.requests[i].take() {
                self.data[i] = scene.resolve(descriptor)?;
                self.data_ready[i] = true;
            }
        }
        if let Some(lines) = self.interactable_request.take() {
            let mut nearest = f32::MAX;
            let mut object = None;
            for line in lines {
                if let Some(hit) = scene.line(line)? {
                    // PC prop-menu repair: a line beginning inside the prop
                    // is touching it. The native eligibility helper rejects a
                    // zero fraction, so adapt only that eligibility argument.
                    let touching = Hit { fraction: f32::MIN_POSITIVE, ..hit };
                    if hit.fraction < nearest && hit.assembly.is_some() {
                        if let Some(id) = scene.eligible_object(if hit.fraction == 0. { touching } else { hit }) {
                            nearest = hit.fraction;
                            object = Some(id);
                        }
                    }
                }
            }
            self.interactable_result = Some(object);
        }
        Ok(())
    }

    ///82D74270: old validation is consumed BEFORE new query results spawn lines.
    pub(crate) fn sync(&mut self, scene: &Scene<'_>, context: QueryContext) -> Result<(), String> {
        self.flags_12836 &= !0x40;
        if let Some(hits) = self.validation.take() {
            if self.flags_12836 & 0x80 != 0 {
                let before = self.pending.len();
                validate(&mut self.pending, &hits);
                // SKATE2_TRACE_SKATE3_GRAB: candidates before and after the obstruction lines,
                // with each line's hit (fraction, assembly) and the candidates' assemblies.
                if std::env::var_os("SKATE2_TRACE_SKATE3_GRAB").is_some() && before != 0 {
                    eprintln!("skate3 grab validation: {before} -> {} hits={:?} assemblies={:?}",
                        self.pending.len(),
                        hits.iter().map(|h| h.as_ref().map(|h| (h.fraction, h.assembly))).collect::<Vec<_>>(),
                        self.pending.iter().map(Record::assembly).collect::<Vec<_>>());
                }
                //82D74888 refreshes geometry by descriptor from newest results.
                if let Some(latest) = &self.query_result {
                    for old in &mut self.pending {
                        if let Some(new) = latest.iter().find(|r| {
                            r.descriptor().kind == old.descriptor().kind
                                && r.descriptor().id == old.descriptor().id
                        }) {
                            *old = new.clone();
                        }
                    }
                }
            }
            self.validated = self.pending.clone();
            self.flags_12836 = ((self.flags_12836 >> 1) & 0x40) | (self.flags_12836 & 0x3f);
            self.validation_position = [0.; 4];
        }
        if let Some(results) = self.query_result.take() {
            self.pending = results.into_iter().take(5).collect();
            self.validation_position = self.query_position;
            self.query_position = [0.; 4];
            self.flags_12836 |= 0x80;
            let mut hits = Vec::with_capacity(self.pending.len() * 3);
            for record in &self.pending {
                let mut end = closest_point(self.validation_position, record.endpoints());
                let mut start = self.validation_position;
                let mut height = start[1] - f32::from_bits(0x3ecccccd);
                for _ in 0..3 {
                    start[1] = height;
                    end[1] = height;
                    hits.push(scene.line(Line {
                        start,
                        end,
                        radius: 0.2,
                        group: context.matching_id_2952,
                        reject_flags: 0,
                        source_pool_mask: 7,
                        selection_flags: context.selection_flags_2948,
                    })?);
                    height += f32::from_bits(0x3ecccccd);
                }
            }
            //Even an empty real query creates a completed empty batch.
            self.validation = Some(hits);
        }
        Ok(())
    }

    /// The validated candidates ConsiderGrabSplineQueryResults walks.
    pub(crate) fn validated(&self) -> &[Record] {
        &self.validated
    }

    /// ProcessedPhysIn's first GrabSpline payload, which Skitching copies.
    pub(crate) fn published_primary(&self) -> Option<&Record> {
        self.published[0].as_ref()
    }

    pub(crate) fn best(&self, position: Vector) -> Option<Record> {
        selection::best(&self.validated, position)
    }

    ///82D749D0 cancels spline queries/validation and clears only native readiness.
    ///Scene lines have already completed; dropping them does not publish results.
    pub(crate) fn invalidate(&mut self) {
        self.queries.clear();
        self.query_result = None;
        self.validation = None;
        self.data_ready = [false; 2];
        self.flags_12836 &= 0x3f;
    }

    ///GroundEnter82D30B64 additionally clears interactable publication readiness.
    pub(crate) fn enter_reset(&mut self) {
        self.invalidate();
        self.interactable_result = None;
    }

    ///82D740F8 consumes completion, not retained candidate existence.
    pub(crate) fn publish(&mut self) -> Publication {
        let records = std::array::from_fn(|i| {
            let record = if self.data_ready[i] {
                self.data[i].as_ref().filter(|r| r.valid()).cloned()
            } else {
                None
            };
            self.data_ready[i] = false;
            record
        });
        for (slot, record) in self.published.iter_mut().zip(&records) {
            if let Some(record) = record {
                *slot = Some(record.clone());
            }
        }
        let object = self.interactable_result.take();
        if let Some(result) = object {
            self.interactable_latched = result.is_some();
        }
        Publication { records, object }
    }
}

///82D74740, raw747B8/747F4/7481C..74874. S2 has an extra outer hit-index
///increment at82DCC0FC; S3 does not. Do not group hits into chunks of three.
fn validate(records: &mut Vec<Record>, hits: &[Option<Hit>]) {
    let mut record_index = 0;
    let mut hit_index = 0;
    while record_index < records.len() {
        let assembly = records[record_index].assembly();
        let mut obstructed = false;
        for _ in 0..3 {
            if let Some(hit) = &hits[hit_index] {
                obstructed =
                    hit.assembly.is_none() || (assembly.is_some() && hit.assembly != assembly);
            }
            hit_index += 1;
            if obstructed {
                break;
            }
        }
        if obstructed {
            records.swap_remove(record_index);
        } else {
            record_index += 1;
        }
    }
}
