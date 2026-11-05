//! PointNegGraphData has four bounds followed by N X values and N Y values.
use skate3_core::point_graph::PointGraph;
use skate_data::collections::Collections;
pub(super) fn load<const N: usize>(
    data: &Collections,
    class: &str,
    name: &str,
) -> Result<(PointGraph<N>, [f32; 4]), String> {
    let field = data.field(class, "default", name)?;
    if field.type_name != format!("Sk8::PointNegGraphData{N}") {
        return Err(format!(
            "Expected PointNegGraphData{N} at {class}/default/{name}"
        ));
    }
    parse::<N>(&field.data).map_err(|e| format!("{class}/default/{name}: {e}"))
}
pub(super) fn parse<const N: usize>(text: &str) -> Result<(PointGraph<N>, [f32; 4]), String> {
    let hex: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if N == 0 || !hex.is_ascii() || hex.len() != (4 + 2 * N) * 8 {
        return Err("Invalid native graph payload length".into());
    }
    let values = (0..4 + 2 * N)
        .map(|i| {
            u32::from_str_radix(&hex[i * 8..i * 8 + 8], 16)
                .map(f32::from_bits)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.iter().any(|v| !v.is_finite()) {
        return Err("Non-finite native graph value".into());
    }
    let graph = PointGraph {
        x: std::array::from_fn(|i| values[4 + i]),
        y: std::array::from_fn(|i| values[4 + N + i]),
    };
    // Duplicate authored X knots are valid and retained in source order.
    if graph.x.windows(2).any(|p| p[0] > p[1]) {
        return Err("Native graph X values are not ordered".into());
    }
    Ok((graph, std::array::from_fn(|i| values[i])))
}
