use super::{Provider, Query, Record, Registry, closest_point, math::*, qualify};
///82760508 mode4, actual provider enumeration, cap32 BEFORE distance sort.
pub fn query(registry: &Registry, q: &Query) -> Result<Vec<Record>, &'static str> {
    if q.mode != 4 {
        return Err("This offboard provider implements native bounded mode4 only");
    }
    if q.capacity > 32 {
        return Err("Native grab query buffer capacity is32");
    }
    if q.position
        .iter()
        .chain(q.sort_position.iter())
        .chain(q.bounds.extents.iter())
        .chain(q.bounds.frame.iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err("Nonfinite grab query");
    }
    let radius = length(q.bounds.extents);
    let center = q.bounds.frame[3];
    let mut out = Vec::with_capacity(32);
    for object in &registry.objects {
        if object.disabled || !object.assembly_ready || object.assembly.is_none() {
            continue;
        }
        match object.provider {
            Provider::Dmo {
                selection_variant,
                matching_group,
                record_enabled,
            } => {
                if !record_enabled
                    || u32::from(selection_variant) != (q.context.selection_flags_2948 >> 1) & 1
                    || !(q.context.matching_id_2952 == -1
                        || matching_group == -1
                        || matching_group == q.context.matching_id_2952)
                {
                    continue;
                }
            }
            Provider::LivingWorld => {
                let d = sub(object.frame[3], center);
                let limit = radius + 15.0;
                if dot(d, d) >= limit * limit {
                    continue;
                }
            }
        }
        for spline in &object.splines {
            if spline.descriptor.kind != 2
                || spline.geometry.points.len() < 2
                || spline.geometry.approach_vectors.is_empty()
            {
                continue;
            }
            let intersects = spline.geometry.points.windows(2).any(|pair| {
                let delta = sub(pair[1], pair[0]);
                dot(delta, delta) > f32::from_bits(0x3727c5ac)
                    && sphere_segment(
                        center,
                        radius,
                        point(object.frame, pair[0]),
                        point(object.frame, pair[1]),
                    )
            });
            if !intersects || out.len() == 32 {
                continue;
            }
            let record = object.record(spline)?;
            if qualify(&record, q.position, q.bounds, q.limits) {
                out.push(record);
            }
        }
    }
    //82761C58 stores squared closest-segment distance at184. Since count<=32,
    //82762178 always dispatches stable insertion sort827629D8 (strict shifts).
    for r in &mut out {
        let delta = sub(
            q.sort_position,
            closest_point(q.sort_position, r.endpoints()),
        );
        r.0[46] = dot(delta, delta).to_bits();
    }
    for i in 1..out.len() {
        let mut j = i;
        while j > 0 && out[j].scalar(184) < out[j - 1].scalar(184) {
            out.swap(j, j - 1);
            j -= 1;
        }
    }
    out.truncate(q.capacity);
    Ok(out)
}
