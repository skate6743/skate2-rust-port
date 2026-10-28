//! `Island::InitCullingTable` 0x827b8c10, the create branch of the Island's
//! "culling%d" (`mFlags`) inventory table. The Island ctor 0x827bad10 and
//! `ChangeFlags` 0x827ba5c8 both run it, then store the same table into every
//! Island pipeline's `mCullingTable`, SkaterGround and SkaterSkater included.
//!
//! A set bit culls: `AssemblyGroundCollisionJob` 0x827cd2b0 tests
//! (columns * VolumeData+0xD4 + ground group) and skips the volume, and
//! `AssemblyAssemblyCollisionJob` 0x827cc76c tests the two Header groups.

use skate2_formats::collision::BitTable;

const GROUPS: u32 = 19;
const FLAG_BOTH: u32 = 3;

pub(crate) fn island_culling_table(flags: u32) -> BitTable {
    // rw::BitTable::Initialize 0x82bc3308: (rows * columns + 31) >> 5 zeroed words.
    let array_size = (GROUPS * GROUPS + 31) >> 5;
    let mut table = BitTable {
        rows: GROUPS,
        columns: GROUPS,
        array_size,
        words: vec![0; array_size as usize],
    };
    for i in 0..GROUPS {
        for j in 0..GROUPS {
            if i == 3 || i == 2 {
                set(&mut table, i, j);
                set(&mut table, j, i);
            }
            if j == 1 || i == 1 {
                set(&mut table, i, j);
            }
            if j >= 10 || i >= 10 {
                set(&mut table, i, j);
            }
        }
    }
    for (i, j) in [(2, 0), (0, 2), (3, 0), (0, 3)] {
        clear(&mut table, i, j);
    }
    for (i, j) in [
        (5, 5),
        (5, 4),
        (4, 5),
        (5, 7),
        (7, 5),
        (5, 6),
        (6, 5),
        (4, 6),
        (6, 4),
    ] {
        set(&mut table, i, j);
    }
    for (i, j) in [
        (18, 10),
        (10, 18),
        (18, 14),
        (14, 18),
        (10, 4),
        (4, 10),
        (10, 7),
        (7, 10),
        (10, 6),
        (6, 10),
        (11, 0),
        (0, 11),
        (12, 0),
        (0, 12),
        (11, 10),
        (10, 11),
        (12, 10),
        (10, 12),
        (13, 4),
        (4, 13),
        (13, 7),
        (7, 13),
        (13, 6),
        (6, 13),
        (13, 13),
        (14, 14),
        (14, 0),
        (0, 14),
        (14, 4),
        (4, 14),
        (14, 6),
        (6, 14),
        (14, 7),
        (7, 14),
        (14, 10),
        (10, 14),
        (14, 11),
        (11, 14),
        (14, 12),
        (12, 14),
        (16, 16),
        (14, 16),
        (16, 14),
        (16, 0),
        (0, 16),
        (16, 4),
        (4, 16),
        (16, 6),
        (6, 16),
        (16, 7),
        (7, 16),
        (16, 10),
        (10, 16),
        (16, 11),
        (11, 16),
        (16, 12),
        (12, 16),
        (16, 17),
        (17, 16),
        (17, 4),
        (4, 17),
        (17, 2),
        (2, 17),
        (17, 10),
        (10, 17),
        (15, 15),
        (15, 0),
        (0, 15),
        (15, 4),
        (4, 15),
        (15, 6),
        (6, 15),
        (15, 7),
        (7, 15),
        (15, 10),
        (10, 15),
        (15, 11),
        (11, 15),
        (15, 14),
        (14, 15),
        (15, 17),
        (17, 15),
    ] {
        clear(&mut table, i, j);
    }
    if flags != FLAG_BOTH {
        for (i, j) in [
            (4, 4),
            (6, 6),
            (7, 7),
            (7, 6),
            (6, 7),
            (4, 7),
            (7, 4),
            (4, 6),
            (6, 4),
        ] {
            set(&mut table, i, j);
        }
    }
    for (i, j) in [(8, 8), (8, 5), (5, 8), (8, 0), (0, 8)] {
        set(&mut table, i, j);
    }
    table
}

fn set(table: &mut BitTable, row: u32, column: u32) {
    let index = table.columns * row + column;
    table.words[(index >> 5) as usize] |= 1 << (index & 31);
}

fn clear(table: &mut BitTable, row: u32, column: u32) {
    let index = table.columns * row + column;
    table.words[(index >> 5) as usize] &= !(1 << (index & 31));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn culled(table: &BitTable, row: u32, column: u32) -> bool {
        let index = table.columns * row + column;
        table.words[(index >> 5) as usize] & (1 << (index & 31)) != 0
    }

    #[test]
    fn words_match_emulated_native_create_branch() {
        assert_eq!(
            island_culling_table(3).words,
            [
                0xfffe_2502,
                0xfd7f_ffbf,
                0x4186_efff,
                0x60fb_feff,
                0x2fc3_05d8,
                0xffe0_77fd,
                0x77fc_09cb,
                0x97eb_bfec,
                0xc88c_bbef,
                0xa32e_a665,
                0xfff3_df5c,
                0x0000_01ee,
            ]
        );
        assert_eq!(
            island_culling_table(1).words,
            [
                0xfffe_2502,
                0xfd7f_ffbf,
                0x418f_efff,
                0x63fb_feff,
                0x2fc3_1fd8,
                0xffe0_77fd,
                0x77fc_09cb,
                0x97eb_bfec,
                0xc88c_bbef,
                0xa32e_a665,
                0xfff3_df5c,
                0x0000_01ee,
            ]
        );
    }

    #[test]
    fn skater_rows_against_static_ground_group_zero() {
        let table = island_culling_table(3);
        for group in [2, 3, 4, 6, 7] {
            assert!(!culled(&table, group, 0), "group {group}");
        }
        for group in [0x11, 0x12] {
            assert!(culled(&table, group, 0), "group {group}");
        }
        assert!(culled(&table, 4, 6) && culled(&table, 6, 4));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn culled(table: &BitTable, row: u32, column: u32) -> bool {
        let index = table.columns * row + column;
        table.words[(index >> 5) as usize] & (1 << (index & 31)) != 0
    }

    /// Groups 4 (a skateboard) and 7 (an off-board Skeleton): an online game's island culls
    /// them, an off-board skater's own board included; a single player game's collides them.
    #[test]
    fn an_online_island_keeps_off_board_skaters_off_boards() {
        let single = island_culling_table(FLAG_BOTH);
        let online = island_culling_table(1);
        for (a, b) in [(4, 7), (7, 4), (4, 4), (7, 7)] {
            assert!(!culled(&single, a, b), "({a}, {b}) collides in a single player game");
            assert!(culled(&online, a, b), "({a}, {b}) is culled online");
        }
    }
}
