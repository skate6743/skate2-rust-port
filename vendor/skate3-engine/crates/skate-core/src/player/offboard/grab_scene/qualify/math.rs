//! Scalar forms of original VMX steps; independent PC estimates, not bit parity.
pub type Vector = [f32; 4];
pub fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn flatten(mut a: Vector) -> Vector {
    a[1] = 0.;
    a
}
pub fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inv_sqrt(value: f32, steps: usize) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(value);
    for _ in 0..steps {
        r = (r * 0.5).mul_add((-value).mul_add(r * r, 1.), r);
    }
    r
}
pub fn reciprocal(value: f32) -> f32 {
    let mut r = 1. / value;
    for _ in 0..2 {
        r = r.mul_add((-value).mul_add(r, 1.), r);
    }
    r
}
pub fn length(v: Vector) -> f32 {
    let squared = dot(v, v);
    let result = squared * inv_sqrt(squared, 2);
    if squared == 0. { 0. } else { result }
}
pub fn normalize(v: Vector) -> Vector {
    let squared = dot(v, v);
    let r = inv_sqrt(squared, 2);
    let length = if squared == 0. { 0. } else { squared * r };
    if length > f32::from_bits(0x358637bd) {
        v.map(|x| x * r)
    } else {
        [0.; 4]
    }
}
///8296EBB0: separate squared-length guards and ONE refinement before acos.
pub fn angle(a: Vector, b: Vector) -> f32 {
    let aa = dot(a, a);
    let bb = dot(b, b);
    if !(aa > f32::from_bits(0x38d1b717) && bb > f32::from_bits(0x38d1b717)) {
        return 0.;
    }
    let a = a.map(|x| x * inv_sqrt(aa, 1));
    let b = b.map(|x| x * inv_sqrt(bb, 1));
    crate::trigonometry::acos(dot(a, b).max(-1.).min(1.))
}
///82E09D28 retains unnormalized direction below its native LENGTH threshold.
pub fn closest_point(position: Vector, [a, b]: [Vector; 2]) -> Vector {
    let mut direction = sub(b, a);
    let len = length(direction);
    if len > f32::from_bits(0x37800000) {
        let r = reciprocal(len);
        direction = direction.map(|v| v * r);
    }
    let along = dot(direction, sub(position, a)).min(len).max(0.);
    std::array::from_fn(|i| direction[i].mul_add(along, a[i]))
}
pub fn transform(frame: [Vector; 4], point: Vector) -> Vector {
    std::array::from_fn(|i| {
        frame[2][i].mul_add(
            point[2],
            frame[1][i].mul_add(point[1], frame[0][i].mul_add(point[0], frame[3][i])),
        )
    })
}
pub fn inverse_point(frame: [Vector; 4], point: Vector) -> Vector {
    let mut result = [0.; 4];
    for i in 0..3 {
        let translation = frame[i][0].mul_add(
            -frame[3][0],
            frame[i][1].mul_add(-frame[3][1], frame[i][2] * -frame[3][2]),
        );
        result[i] = frame[i][2].mul_add(
            point[2],
            frame[i][1].mul_add(point[1], frame[i][0].mul_add(point[0], translation)),
        );
    }
    result
}
