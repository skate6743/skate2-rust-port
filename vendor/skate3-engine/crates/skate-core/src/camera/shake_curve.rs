//! Camera shake Bezier inversion8296F530 and evaluation8296F388.
//! Basis initialized82F82738 from8232FB00/FAF0/FAE0/F660.
pub(super) fn sample(points: [[f32; 4]; 4], input: f32) -> f32 {
    let basis = [
        [-1.0, 3.0, -3.0, 1.0],
        [3.0, -6.0, 3.0, 0.0],
        [-3.0, 3.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let coefficients: [[f32; 4]; 4] = basis.map(|row| {
        core::array::from_fn(|lane| {
            points[3][lane].mul_add(
                row[3],
                points[2][lane].mul_add(
                    row[2],
                    points[1][lane].mul_add(row[1], points[0][lane] * row[0]),
                ),
            )
        })
    });
    let mut t = 0.5_f32;
    for iteration in 0..10 {
        let square = t * t;
        let evaluate = |powers: [f32; 4], lane: usize| {
            coefficients[3][lane].mul_add(
                powers[3],
                coefficients[2][lane].mul_add(
                    powers[2],
                    coefficients[1][lane].mul_add(powers[1], coefficients[0][lane] * powers[0]),
                ),
            )
        };
        let value = evaluate([square * t, square, t, 1.0], 0);
        let result = evaluate([square * t, square, t, 1.0], 1);
        let derivative = evaluate([square * 3.0, t * 2.0, 1.0, 0.0], 0);
        let residual = input - value;
        if iteration == 9
            || !(residual.abs() > f32::from_bits(0x3c23d70a)
                && derivative.abs() > f32::from_bits(0x34000000))
        {
            return result;
        }
        t = residual / derivative + t;
    }
    unreachable!()
}
