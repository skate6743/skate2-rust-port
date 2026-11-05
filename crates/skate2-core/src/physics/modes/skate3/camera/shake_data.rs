//! Owned .shk parser; numerical sequence centering follows82E05DC0 in core.
use skate3_core::camera::ShakeSamples;

pub(crate) fn parse(text:&str)->Result<ShakeSamples,String> {
    let mut tokens=text.split_whitespace();
    let count=tokens.next().ok_or("Empty camera .shk file")?
        .parse::<usize>().map_err(|e|format!("Invalid camera shake sample count: {e}"))?;
    let mut rows=Vec::with_capacity(count);
    for index in 0..count {
        let mut row=[0.0_f32;8];
        for field in &mut row {
            *field=tokens.next().ok_or_else(||format!("Truncated camera shake sample {index}"))?
                .parse::<f32>().map_err(|e|format!("Invalid camera shake sample {index}: {e}"))?;
        }
        rows.push(row);
    }
    if tokens.next().is_some() {return Err("Camera shake has extra sample data".into());}
    ShakeSamples::from_rows(&rows)
}
