//! Rain-only constants from the supplied Skate Underground source.
//! Native weather menu selects intensity; its unrelated random schedule is not imported.
pub(crate) const LOOK_PUDDLE_SIZE:f32=3.5;
pub(crate) const WIND:[f32;3]=[1.45,-18.,-0.58];
pub(crate) const MAX_INTENSITY:f32=1.;
#[cfg(test)]pub(crate) const MAX_PUDDLE_COVERAGE:f32=0.69999;

#[derive(Clone,Copy,Debug,Default)]pub(crate) struct Snapshot {
    pub(crate) intensity:f32,pub(crate) wetness:f32,pub(crate) puddles:f32,pub(crate) time:f32,
}
