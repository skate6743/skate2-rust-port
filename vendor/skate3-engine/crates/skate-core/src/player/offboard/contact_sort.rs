//! Record sort82D95390, introsort82D95500, partition82D95AB0 and heap fallback.
//! Comparator82D81028 uses distance52 only. Equal distances must retain the
//! original algorithm's order because later classifiers scan vertical groups.
use super::{contact_records::Record, contact_segments::Candidate};
pub trait Distance: Copy {
    fn distance(self) -> f32;
}
impl Distance for Record {
    fn distance(self) -> f32 {
        self.distance
    }
}
impl Distance for Candidate {
    fn distance(self) -> f32 {
        self.position_distance
    }
}
fn less<T: Distance>(a: T, b: T) -> bool {
    a.distance() < b.distance()
}

pub fn sort<T: Distance>(records: &mut [T]) {
    if records.is_empty() {
        return;
    }
    let depth = 2 * (usize::BITS - 1 - records.len().leading_zeros());
    introsort(records, depth);
    //82D95650 sorts the first28, then95390 inserts each remaining record.
    //A bounded host cursor also avoids out-of-bounds reads for unordered input.
    insertion(records);
}
fn insertion<T: Distance>(a: &mut [T]) {
    for index in 1..a.len() {
        let value = a[index];
        let mut hole = index;
        while hole > 0 && less(value, a[hole - 1]) {
            a[hole] = a[hole - 1];
            hole -= 1;
        }
        a[hole] = value;
    }
}
fn introsort<T: Distance>(mut a: &mut [T], mut depth: u32) {
    while a.len() > 28 && depth > 0 {
        let first = a[0];
        let middle = a[a.len() / 2];
        let last = a[a.len() - 1];
        let pivot = if less(first, middle) {
            if less(middle, last) {
                middle
            } else if less(first, last) {
                last
            } else {
                first
            }
        } else if less(first, last) {
            first
        } else if less(middle, last) {
            last
        } else {
            middle
        };
        let mut left = 0;
        let mut right = a.len();
        let cut = loop {
            while less(a[left], pivot) {
                left += 1;
            }
            right -= 1;
            while less(pivot, a[right]) {
                right -= 1;
            }
            if left >= right {
                break left;
            }
            a.swap(left, right);
            left += 1;
        };
        depth -= 1;
        let (lower, upper) = a.split_at_mut(cut);
        introsort(upper, depth);
        a = lower;
    }
    if depth == 0 {
        heap_sort(a);
    }
}
fn heap_sort<T: Distance>(a: &mut [T]) {
    if a.len() < 2 {
        return;
    }
    for index in (0..=(a.len() - 2) / 2).rev() {
        let value = a[index];
        adjust_heap(a, index, value);
    }
    for end in (1..a.len()).rev() {
        let value = a[end];
        a[end] = a[0];
        adjust_heap(&mut a[..end], 0, value);
    }
}
fn adjust_heap<T: Distance>(a: &mut [T], top: usize, value: T) {
    let mut hole = top;
    let mut child = 2 * (hole + 1);
    while child < a.len() {
        //96070 chooses the RIGHT child on equal distances.
        if less(a[child], a[child - 1]) {
            child -= 1;
        }
        a[hole] = a[child];
        hole = child;
        child = 2 * (hole + 1);
    }
    if child == a.len() {
        a[hole] = a[child - 1];
        hole = child - 1;
    }
    while hole > top {
        let parent = (hole - 1) / 2;
        if !less(a[parent], value) {
            break;
        }
        a[hole] = a[parent];
        hole = parent;
    }
    a[hole] = value;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn records(n: usize) -> Vec<Record> {
        (0..n)
            .map(|i| Record {
                position: [i as f32, 0., 0., 0.],
                normal: [0.; 4],
                coordinates: [0.; 4],
                flags: 0,
                distance: 1.,
            })
            .collect()
    }
    #[test]
    fn equal_distance_order_matches_native_partition_threshold() {
        for n in [28, 32, 64] {
            let mut a = records(n);
            sort(&mut a);
            let expected: Vec<_> = match n {
                28 => (0..28).collect(),
                32 => (0..32).rev().collect(),
                64 => (32..64).chain(0..32).collect(),
                _ => unreachable!(),
            };
            assert_eq!(
                a.iter().map(|r| r.position[0] as usize).collect::<Vec<_>>(),
                expected
            );
        }
    }
    #[test]
    fn sort_and_heap_fallback_order_finite_distances() {
        for n in 0..=128 {
            let mut a = records(n);
            for (i, r) in a.iter_mut().enumerate() {
                r.distance = ((i * 17 + 3) % 29) as f32 - 14.;
            }
            let mut heap = a.clone();
            sort(&mut a);
            heap_sort(&mut heap);
            assert!(a.windows(2).all(|w| w[0].distance <= w[1].distance));
            assert!(heap.windows(2).all(|w| w[0].distance <= w[1].distance));
        }
    }
}
