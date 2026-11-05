//! Joint/drive batch eligibility: TU3 82AE39D0 and 82AE1778.
//!
//! Native clears the output counts, traverses the registered list in order,
//! and compiles a row only when at least one endpoint has state bit 4. This
//! does not register, wake, freeze or reorder bodies.

pub fn compile_active<T, R>(
    registered: impl IntoIterator<Item = T>,
    endpoint_states: impl Fn(&T) -> [u32; 2],
    mut compile: impl FnMut(T) -> R,
) -> Vec<R> {
    registered
        .into_iter()
        .filter(|constraint| {
            let [a, b] = endpoint_states(constraint);
            (a | b) & 4 != 0
        })
        .map(&mut compile)
        .collect()
}
