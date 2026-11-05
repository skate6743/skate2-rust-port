//! The toolkit's zero-acceleration, duration-one subset of82770910/8276C558.
//! The query remains a swept sphere. A returned point is a surface contact,
//! not the sphere centre or an unadjusted ray intersection.
use super::{LineHit, LineProbe, UP, Vector, ZERO, cross, dot, length, scale, sub};
use crate::air::trajectory::QueryResult;

pub fn query_sweep<E>(
    probe: LineProbe,
    mut line: impl FnMut(LineProbe) -> Result<Option<LineHit>, E>,
    mut nearby: impl FnMut(Vector, f32) -> Result<Vec<[Vector; 3]>, E>,
) -> Result<QueryResult, E> {
    let velocity = sub(probe.end, probe.start);
    //82770910 minimum squared distance is zero for both error scalars zero.
    if !(dot(velocity, velocity) > 0.)
        || !velocity[..3]
            .iter()
            .any(|x| x.abs() > f32::from_bits(0x37800000))
    {
        return Ok(QueryResult::miss());
    }
    let Some(hit) = line(probe)? else {
        return Ok(QueryResult::miss());
    };
    let distance = length(sub(hit.position, probe.start));
    let speed = length(velocity);
    let epsilon = f32::from_bits(0x38d1b717);
    let frames = if distance.abs() > epsilon && speed.abs() > epsilon {
        reciprocal(speed) * distance
    } else {
        0.
    };
    //8276C794 vrfim is floor, followed by vcfpsxws. This zero-acceleration
    //branch differs from the ceiling helper used in the accelerated branch.
    let contact_frame = frames.floor() as i32;
    let triangles = nearby(hit.position, probe.radius)?;
    Ok(QueryResult {
        contact_position: hit.position,
        contact_normal: hit.normal,
        landing_normal: landing_normal(&triangles, velocity),
        contact_time: frames * f32::from_bits(0x3c888889),
        contact_frame,
        contact_transform: hit.mesh_frame,
        surface: u32::from(hit.surface),
        geometry: hit.geometry,
    })
}
fn reciprocal(value: f32) -> f32 {
    let mut r = crate::physics::native_arithmetic::reciprocal_estimate(value);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(value, 1.), r);
    }
    r
}
fn inverse_length(square: f32) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(square);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-square).mul_add(r * r, 1.), r);
    }
    r
}
///82771018 chooses the most upward eligible face, then averages faces whose
///dot with that face is strictly greater than0.5, retaining traversal order.
fn landing_normal(triangles: &[[Vector; 3]], velocity: Vector) -> Vector {
    let mut normals = Vec::with_capacity(64);
    let mut best = ZERO;
    let mut best_y = -1.;
    for [a, b, c] in triangles.iter().take(64) {
        let n = cross(sub(*b, *a), sub(*c, *a));
        let n = scale(n, inverse_length(dot(n, n)));
        if n[1] > 0.7 || dot(n, velocity) <= 0. {
            if n[1] > best_y {
                best = n;
                best_y = n[1];
            }
            normals.push(n);
        }
    }
    if normals.is_empty() {
        return UP;
    }
    let mut total = ZERO;
    for n in normals {
        if dot(best, n) > 0.5 {
            for i in 0..4 {
                total[i] += n[i];
            }
        }
    }
    let square = dot(total, total);
    let inverse = inverse_length(square);
    let magnitude = if square == 0. { 0. } else { square * inverse };
    //830BD350 is initialized by82F826F8 from82181A88=358637bd.
    if magnitude > f32::from_bits(0x358637bd) {
        scale(total, inverse)
    } else {
        ZERO
    }
}
