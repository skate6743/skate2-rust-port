//! Obstacle-to-surface promotion82D82498. Consumes sorted input order, appends
//! promoted hits and invokes the native record sort before returning.
use super::{
    contact_queries::Input,
    contact_records::{Direction, Records, Source},
};

pub fn promote(input: Input, records: &mut Records) {
    let original_count = records.surface.len();
    for next in 1..original_count {
        let previous = records.surface[next - 1];
        let current = records.surface[next];
        let a = previous.coordinates;
        let b = current.coordinates;
        let mut low = if b[1] > a[1] { a[1] } else { b[1] };
        let mut high = if a[1] > b[1] { a[1] } else { b[1] };
        let mut index = 0;
        while index < records.obstacle.len() {
            let obstacle = records.obstacle[index];
            let p = obstacle.coordinates;
            if a[0] > p[0] || p[0] > b[0] {
                index += 1;
                continue;
            }
            if low > p[1] {
                records.obstacle[index].flags |= 2;
                index += 1;
                continue;
            }
            if high > p[1]
                || obstacle.distance < previous.distance
                || obstacle.distance > current.distance
            {
                index += 1;
                continue;
            }
            records.obstacle[index].flags |= 2;
            let mut lower = index;
            let mut upper = index;
            index += 1;
            while index < records.obstacle.len() {
                let p = records.obstacle[index].coordinates;
                if (records.obstacle[lower].coordinates[0] - p[0]).abs()
                    > f32::from_bits(0x3a83_126f)
                {
                    break;
                }
                records.obstacle[index].flags |= 2;
                if p[1] >= high && records.obstacle[lower].coordinates[1] > p[1] {
                    lower = index;
                }
                if p[1] > records.obstacle[upper].coordinates[1] {
                    upper = index;
                }
                index += 1;
            }
            records.obstacle[lower].flags = (records.obstacle[lower].flags & !2) | 1;
            records.obstacle[upper].flags = (records.obstacle[upper].flags & !2) | 1;
            let mut insert = |index: usize, low: &mut f32, high: &mut f32| {
                let r = records.obstacle[index];
                let mut normal = r.normal;
                records.insert(
                    input,
                    r.position,
                    &mut normal,
                    Source::Support,
                    Direction::Forward,
                    r.distance,
                );
                if r.coordinates[1] > *high {
                    *low = *high;
                    *high = r.coordinates[1];
                } else if r.coordinates[1] > *low {
                    *low = r.coordinates[1];
                }
            };
            insert(lower, &mut low, &mut high);
            if upper != lower {
                insert(upper, &mut low, &mut high);
            }
            //82D82900 increments even after the grouping loop has advanced the
            //cursor. Preserve this native skip of the first nonmatching record.
            index += 1;
        }
    }
    super::contact_sort::sort(&mut records.surface);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::offboard::contact_records::Record;
    fn input() -> Input {
        Input {
            position: [0.; 4],
            surface_forward: [0., 0., 1., 0.],
            surface_up: [0., 1., 0., 0.],
            surface_right: [1., 0., 0., 0.],
            velocity: [0.; 4],
            animation_up: [0., 1., 0., 0.],
            animation_right: [1., 0., 0., 0.],
        }
    }
    fn hit(y: f32, z: f32) -> Record {
        Record {
            position: [0., y, z, 0.],
            normal: [0., 1., 0., 0.],
            coordinates: [z, y, z, z],
            flags: 0,
            distance: z,
        }
    }
    #[test]
    fn below_surface_is_marked_and_vertical_group_promotes_only_extremes() {
        let mut records = Records {
            surface: vec![hit(0., 0.), hit(0., 2.)],
            obstacle: vec![hit(-0.1, 0.5), hit(0.2, 1.), hit(0.4, 1.), hit(0.3, 1.)],
            inserted_surface_count: 2,
        };
        promote(input(), &mut records);
        assert_eq!(
            records.obstacle.iter().map(|r| r.flags).collect::<Vec<_>>(),
            vec![2, 1, 1, 2]
        );
        assert_eq!(records.surface.len(), 4);
        assert_eq!(records.surface[1].position, [0., 0.2, 1., 0.]);
        assert_eq!(records.surface[2].position, [0., 0.4, 1., 0.]);
    }
    #[test]
    fn promotion_retains_native_group_cursor_skip() {
        let mut records = Records {
            surface: vec![hit(0., 0.), hit(0., 2.)],
            obstacle: vec![hit(0.2, 0.5), hit(0.3, 1.), hit(0.4, 1.5)],
            inserted_surface_count: 2,
        };
        promote(input(), &mut records);
        assert_eq!(records.obstacle[1].flags, 0);
        assert_eq!(records.surface.len(), 4);
    }
}
