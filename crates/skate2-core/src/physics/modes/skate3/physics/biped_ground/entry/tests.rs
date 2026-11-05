use super::effective_root;

const ROOT: [[f32; 4]; 4] = [
    [1., 2., 3., 4.],
    [5., 6., 7., 8.],
    [9., 10., 11., 12.],
    [13., 14., 15., 16.],
];

#[test]
fn ordinary_entry_preserves_the_complete_root() {
    assert_eq!(effective_root(ROOT, 0), ROOT);
    assert_eq!(effective_root(ROOT, !4), ROOT);
}

#[test]
fn mirrored_entry_flips_only_x_and_z_including_fourth_lanes() {
    let result = effective_root(ROOT, 4);
    assert_eq!(result[0], [-1., -2., -3., -4.]);
    assert_eq!(result[2], [-9., -10., -11., -12.]);
    assert_eq!(result[1], ROOT[1]);
    assert_eq!(result[3], ROOT[3]);
}
