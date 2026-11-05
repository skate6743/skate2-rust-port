//! ZIP short names verified entry-for-entry from original S3 initializer82DE6678.
//!82F92AC0 splits384 records;82DEE508 publishes skating IDs and scoring IDs
//! separately. S2 82E2F9D0 uses160 records (two orientations, five families).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Name {
    pub skating_id: i32,
    pub attribute: &'static str,
    pub display: &'static str,
    pub scorable_id: i32,
}
#[path = "grind_names/table0.rs"]
mod table0;
#[path = "grind_names/table1.rs"]
mod table1;
#[path = "grind_names/table2.rs"]
mod table2;
#[path = "grind_names/table3.rs"]
mod table3;
#[path = "grind_names/table4.rs"]
mod table4;
#[path = "grind_names/table5.rs"]
mod table5;
#[path = "grind_names/table6.rs"]
mod table6;
#[path = "grind_names/table7.rs"]
mod table7;

/// Dimensions: approach2, location2, twist2, tilt2, orientation4, family6.
/// Invalid/sentinel chromosomes are not silently mapped to a fifty-fifty.
pub(crate) fn lookup(c: [u32; 6]) -> Option<Name> {
    if c.into_iter()
        .zip([2, 2, 2, 2, 4, 6])
        .any(|(v, bound)| v >= bound)
    {
        return None;
    }
    let i = (((((c[0] * 2 + c[1]) * 2 + c[2]) * 2 + c[3]) * 4 + c[4]) * 6 + c[5]) as usize;
    let table = match i / 48 {
        0 => &table0::TABLE,
        1 => &table1::TABLE,
        2 => &table2::TABLE,
        3 => &table3::TABLE,
        4 => &table4::TABLE,
        5 => &table5::TABLE,
        6 => &table6::TABLE,
        _ => &table7::TABLE,
    };
    Some(table[i % 48])
}
