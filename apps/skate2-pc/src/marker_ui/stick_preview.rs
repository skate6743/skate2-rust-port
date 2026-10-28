//! PC port addition: a Stick Settings page's box for one stick, drawn as RPCS3's pad settings draw
//! their stick previews (`pad_settings_dialog::RepaintPreviewLabel`): the stick's full travel, its
//! deadzone in red and its anti-deadzone in green, the stick as its pad reports it under the
//! multiplier as a blue dot, and the stick as the game reads it under every setting as a red one.
use super::{Batch, Colour, clips::Clip, quad, vertex};
use crate::gpu_renderer::RenderResult;
use skate2_core::stick_settings::{StickSettings, axis};
use skate2_formats::apt::Matrix;

/// The `_type` a box's clip carries.
pub(super) const TYPE: &str = "StickPreview";
/// The property holding what the box shows.
const PROPERTY: &str = "stick_preview";
/// The box's side in stage units.
pub(super) const SIZE: f32 = 160.;
/// The circle of the stick's full travel, as RPCS3's outer circle is, below the box's letter.
const CENTRE: [f32; 2] = [80., 88.];
const RADIUS: f32 = 60.;
/// The segments of a circle.
const SEGMENTS: u16 = 48;

/// What a box shows: its stick's settings and, with a pad, the stick's raw axes.
pub(super) fn set(clip: &mut Clip, stick: StickSettings, raw: Option<[i16; 2]>) {
    clip.properties.insert(PROPERTY.into(), value(stick, raw));
}

fn value(stick: StickSettings, raw: Option<[i16; 2]>) -> String {
    let raw = raw.map_or_else(|| "-".to_owned(), |[x, y]| format!("{x} {y}"));
    format!("{} {} {} {raw}", stick.multiplier, stick.deadzone, stick.anti_deadzone)
}

pub(super) fn draw(clip: &Clip, m: Matrix, colour: Colour, size: (u32, u32), out: &mut Vec<Batch>, flat: bool) -> RenderResult<()> {
    match clip.properties.get(PROPERTY) {
        Some(value) => draw_value(value, m, colour, size, out, flat),
        None => Ok(()),
    }
}

/// The box [`set`] gave `value`.
fn draw_value(value: &str, m: Matrix, colour: Colour, size: (u32, u32), out: &mut Vec<Batch>, flat: bool) -> RenderResult<()> {
    let mut words = value.split_whitespace();
    let mut word = || words.next().ok_or("stick preview property cut short");
    let stick = StickSettings { multiplier: word()?.parse()?, deadzone: word()?.parse()?, anti_deadzone: word()?.parse()? };
    let raw = match word()? {
        "-" => None,
        x => Some([x.parse::<i16>()?, word()?.parse::<i16>()?]),
    };
    // The box: dark, with a light edge.
    solid(out, quad(m, [0., 0., SIZE, SIZE], [0., 0., 1., 1.], colour.apply([0., 0., 0., 0.55]), size, flat));
    for edge in [[0., 0., SIZE, 2.], [0., SIZE - 2., SIZE, SIZE], [0., 0., 2., SIZE], [SIZE - 2., 0., SIZE, SIZE]] {
        solid(out, quad(m, edge, [0., 0., 1., 1.], colour.apply([1., 1., 1., 0.8]), size, flat));
    }
    // The stick's full travel, then the deadzone in red and the anti-deadzone in green.
    solid(out, ring(m, RADIUS, colour.apply([1., 1., 1., 0.9]), size, flat));
    if stick.deadzone > 0 {
        solid(out, ring(m, RADIUS * f32::from(stick.deadzone) / 100., colour.apply([1., 0.2, 0.2, 1.]), size, flat));
    }
    if stick.anti_deadzone > 0 {
        solid(out, ring(m, RADIUS * f32::from(stick.anti_deadzone) / 100., colour.apply([0.25, 1., 0.35, 1.]), size, flat));
    }
    // The stick under the multiplier alone in blue, then as the game reads it in red, over it.
    if let Some([x, y]) = raw {
        let (x, y) = (axis(x), axis(y));
        let point = |[x, y]: [f32; 2]| [x.mul_add(RADIUS, CENTRE[0]), (-y).mul_add(RADIUS, CENTRE[1])];
        solid(out, disc(m, point(stick.multiplied(x, y)), 3.5, colour.apply([0.25, 0.55, 1., 1.]), size, flat));
        solid(out, disc(m, point(stick.apply_axes(x, y)), 2.5, colour.apply([1., 0.25, 0.25, 1.]), size, flat));
    }
    Ok(())
}

fn solid(out: &mut Vec<Batch>, vertices: Vec<[f32; 10]>) {
    out.push(Batch { texture: None, wrapped: false, vertices });
}

/// The point at `radius` from `centre` where a segment starts.
fn at(centre: [f32; 2], radius: f32, segment: u16) -> [f32; 2] {
    let angle = f32::from(segment) * std::f32::consts::TAU / f32::from(SEGMENTS);
    [radius.mul_add(angle.cos(), centre[0]), radius.mul_add(angle.sin(), centre[1])]
}

/// A ring two units wide about the box's circle centre.
fn ring(m: Matrix, radius: f32, rgba: [f32; 4], size: (u32, u32), flat: bool) -> Vec<[f32; 10]> {
    let (inner, outer) = ((radius - 1.).max(0.), radius + 1.);
    let mut vertices = Vec::with_capacity(usize::from(SEGMENTS) * 6);
    for segment in 0..SEGMENTS {
        let corners = [at(CENTRE, inner, segment), at(CENTRE, outer, segment), at(CENTRE, outer, segment + 1), at(CENTRE, inner, segment + 1)];
        for index in [0, 1, 2, 0, 2, 3] {
            vertices.push(vertex(m, corners[index], [0., 0.], rgba, size, flat));
        }
    }
    vertices
}

/// A filled dot.
fn disc(m: Matrix, centre: [f32; 2], radius: f32, rgba: [f32; 4], size: (u32, u32), flat: bool) -> Vec<[f32; 10]> {
    let mut vertices = Vec::with_capacity(usize::from(SEGMENTS) * 3);
    for segment in 0..SEGMENTS {
        for point in [centre, at(centre, radius, segment), at(centre, radius, segment + 1)] {
            vertices.push(vertex(m, point, [0., 0.], rgba, size, flat));
        }
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Matrix = Matrix { a: 1., b: 0., c: 0., d: 1., tx: 0., ty: 0. };

    /// The batches of a box at the flat stage's origin.
    fn drawn(stick: StickSettings, raw: Option<[i16; 2]>) -> Vec<Batch> {
        let mut out = Vec::new();
        draw_value(&value(stick, raw), IDENTITY, Colour::default(), (1280, 720), &mut out, true).unwrap();
        out
    }

    /// A batch's mean stage position (a dot's centre), from the flat stage's clip space.
    fn mean(batch: &Batch) -> [f32; 2] {
        let (mut sum, mut count) = ([0.; 2], 0.);
        for v in &batch.vertices {
            sum = [sum[0] + (v[0] + 1.) * 640., sum[1] + (1. - v[1]) * 360.];
            count += 1.;
        }
        [sum[0] / count, sum[1] / count]
    }

    #[test]
    fn a_box_draws_its_rings_and_the_two_dots_where_rpcs3_puts_them() {
        let stick = StickSettings { multiplier: 100, deadzone: 20, anti_deadzone: 25 };
        // No pad: the box, its edges and three rings; no dots.
        let out = drawn(stick, None);
        assert_eq!(out.len(), 1 + 4 + 3);
        assert!(out.iter().all(|batch| batch.texture.is_none() && !batch.vertices.is_empty() && batch.vertices.len() % 3 == 0));
        // The stick a tenth to the right, inside the deadzone: the blue dot there, the red one on
        // its way to the anti-deadzone's ring, at half of it as the stick is at half the deadzone.
        let out = drawn(stick, Some([3277, 0]));
        assert_eq!(out.len(), 1 + 4 + 3 + 2);
        let (blue, red) = (mean(&out[out.len() - 2]), mean(&out[out.len() - 1]));
        assert!((blue[0] - 0.1f32.mul_add(RADIUS, CENTRE[0])).abs() < 0.05 && (blue[1] - CENTRE[1]).abs() < 0.05, "{blue:?}");
        assert!((red[0] - 0.125f32.mul_add(RADIUS, CENTRE[0])).abs() < 0.05 && (red[1] - CENTRE[1]).abs() < 0.05, "{red:?}");
        // Up on the pad is up in the box.
        let out = drawn(StickSettings::VANILLA, Some([0, 16384]));
        assert_eq!(out.len(), 1 + 4 + 1 + 2, "vanilla has no deadzone rings");
        assert!((mean(&out[out.len() - 1])[1] - (-0.5f32).mul_add(RADIUS, CENTRE[1])).abs() < 0.05);
        let mut out = Vec::new();
        assert!(draw_value("100 0", IDENTITY, Colour::default(), (1280, 720), &mut out, true).is_err(), "a property cut short");
    }
}
