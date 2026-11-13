//! PC port addition: what Edit Skater's colour picker draws beside the createskater movie's own
//! controls (`edit_skater_movie.rs`; `skate2_core::ui::colour_picker`): the hue and saturation
//! wheel, and the intensity's ramp along the Intensity row's line. Each is a clip of its own
//! `_type`, drawn as the editor draws its controls: cream edges (its selected titles' 0xfffeec)
//! over a soft shadow, fading with the movie.
use super::{Batch, Colour, clips::Clip, quad, vertex};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::colour_picker::{Hsv, intensity_at};
use skate2_formats::apt::Matrix;

/// The hue and saturation wheel, about its clip's origin.
pub(super) const WHEEL: &str = "ColourWheel";
/// The intensity's ramp, from its clip's origin to the right.
pub(super) const RAMP: &str = "ColourRamp";
/// The properties holding a ramp's span and its colour.
const SPAN: &str = "colour_ramp_span";
const COLOUR: &str = "colour_ramp_colour";
/// The wheel's radius in stage units.
pub(super) const WHEEL_RADIUS: f32 = 80.;
/// The ramp's height, centred on the morph line it replaces (4 units tall).
const RAMP_HEIGHT: f32 = 10.;
const CREAM: [f32; 4] = [1., 254. / 255., 236. / 255., 1.];
const EDGE: [f32; 4] = [0., 0., 0., 0.6];
const SHADOW: [f32; 4] = [0., 0., 0., 0.45];
/// The wheel's slices around and bands out from its centre.
const SEGMENTS: u16 = 96;
const BANDS: u16 = 8;
/// The ramp's slices along its colours.
const STEPS: u16 = 40;

/// Whether `kind` is one of the picker's clips.
pub(super) fn draws(kind: &str) -> bool {
    matches!(kind, WHEEL | RAMP)
}

/// A ramp clip's frame runs `width` units; its colours run from black at `from` (where the
/// knob's middle is at the bottom of its travel) to the brightest tint at `to`, each end's colour
/// held out to the frame.
pub(super) fn set_ramp_span(clip: &mut Clip, width: f32, from: f32, to: f32) {
    clip.properties.insert(SPAN.into(), format!("{width} {from} {to}"));
}

/// A ramp clip shows `hue` at `saturation`.
pub(super) fn set_ramp_colour(clip: &mut Clip, hue: f32, saturation: f32) {
    clip.properties.insert(COLOUR.into(), format!("{hue} {saturation}"));
}

pub(super) fn draw(clip: &Clip, kind: &str, m: Matrix, colour: Colour, size: (u32, u32), out: &mut Vec<Batch>, flat: bool) -> RenderResult<()> {
    match kind {
        WHEEL => {
            wheel(m, colour, size, out, flat);
            Ok(())
        }
        RAMP => match (clip.properties.get(SPAN), clip.properties.get(COLOUR)) {
            (Some(span), Some(hue)) => ramp(span, hue, m, colour, size, out, flat),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

fn solid(out: &mut Vec<Batch>, vertices: Vec<[f32; 10]>) {
    out.push(Batch { texture: None, wrapped: false, vertices });
}

/// The point at `radius` from `centre`, `turn` (0 to 1) clockwise from the top.
fn at(centre: [f32; 2], radius: f32, turn: f32) -> [f32; 2] {
    let angle = turn * std::f32::consts::TAU;
    [radius.mul_add(angle.sin(), centre[0]), (-radius).mul_add(angle.cos(), centre[1])]
}

/// A filled circle.
fn disc(m: Matrix, centre: [f32; 2], radius: f32, rgba: [f32; 4], size: (u32, u32), flat: bool) -> Vec<[f32; 10]> {
    let mut vertices = Vec::with_capacity(usize::from(SEGMENTS) * 3);
    for segment in 0..SEGMENTS {
        let turns = [f32::from(segment), f32::from(segment + 1)].map(|s| s / f32::from(SEGMENTS));
        for point in [centre, at(centre, radius, turns[0]), at(centre, radius, turns[1])] {
            vertices.push(vertex(m, point, [0., 0.], rgba, size, flat));
        }
    }
    vertices
}

/// A ring between two radii.
fn ring(m: Matrix, inner: f32, outer: f32, rgba: [f32; 4], size: (u32, u32), flat: bool) -> Vec<[f32; 10]> {
    let mut vertices = Vec::with_capacity(usize::from(SEGMENTS) * 6);
    for segment in 0..SEGMENTS {
        let turns = [f32::from(segment), f32::from(segment + 1)].map(|s| s / f32::from(SEGMENTS));
        let corners = [at([0.; 2], inner, turns[0]), at([0.; 2], outer, turns[0]), at([0.; 2], outer, turns[1]), at([0.; 2], inner, turns[1])];
        for index in [0, 1, 2, 0, 2, 3] {
            vertices.push(vertex(m, corners[index], [0., 0.], rgba, size, flat));
        }
    }
    vertices
}

/// The wheel: its shadow, a dark edge and the cream rim, and the colours inside, each vertex the
/// colour at its hue and saturation and full intensity.
fn wheel(m: Matrix, colour: Colour, size: (u32, u32), out: &mut Vec<Batch>, flat: bool) {
    let radius = WHEEL_RADIUS;
    solid(out, disc(m, [2., 3.], radius + 5., colour.apply(SHADOW), size, flat));
    solid(out, ring(m, radius + 3., radius + 4.5, colour.apply(EDGE), size, flat));
    solid(out, ring(m, radius, radius + 3., colour.apply(CREAM), size, flat));
    let mut vertices = Vec::with_capacity(usize::from(SEGMENTS) * usize::from(BANDS) * 6);
    for band in 0..BANDS {
        let saturations = [f32::from(band), f32::from(band + 1)].map(|b| b / f32::from(BANDS));
        for segment in 0..SEGMENTS {
            let hues = [f32::from(segment), f32::from(segment + 1)].map(|s| s / f32::from(SEGMENTS));
            let corners = [(saturations[0], hues[0]), (saturations[1], hues[0]), (saturations[1], hues[1]), (saturations[0], hues[1])];
            for index in [0, 1, 2, 0, 2, 3] {
                let (saturation, hue) = corners[index];
                let [r, g, b] = Hsv { hue, saturation, intensity: 1. }.display();
                vertices.push(vertex(m, at([0.; 2], radius * saturation, hue), [0., 0.], colour.apply([r, g, b, 1.]), size, flat));
            }
        }
    }
    solid(out, vertices);
}

/// Numbers a property holds, `count` of them.
fn numbers(value: &str, count: usize) -> RenderResult<Vec<f32>> {
    let numbers = value.split_whitespace().map(str::parse::<f32>).collect::<Result<Vec<_>, _>>()?;
    if numbers.len() != count {
        return Err(format!("colour ramp property {value:?} is not {count} numbers").into());
    }
    Ok(numbers)
}

/// The ramp `set_ramp_span` and `set_ramp_colour` describe: its shadow, the cream frame, and the
/// colour along it.
fn ramp(span: &str, hue: &str, m: Matrix, colour: Colour, size: (u32, u32), out: &mut Vec<Batch>, flat: bool) -> RenderResult<()> {
    let (span, hue) = (numbers(span, 3)?, numbers(hue, 2)?);
    let (width, from, to) = (span[0], span[1], span[2]);
    let (hue, saturation) = (hue[0], hue[1]);
    let (top, bottom) = (2. - RAMP_HEIGHT / 2., 2. + RAMP_HEIGHT / 2.);
    solid(out, quad(m, [0.5, top + 1.5, width + 2.5, bottom + 3.], [0., 0., 1., 1.], colour.apply(SHADOW), size, flat));
    solid(out, quad(m, [-1.5, top - 1.5, width + 1.5, bottom + 1.5], [0., 0., 1., 1.], colour.apply(CREAM), size, flat));
    let tint = |at: f32| {
        let [r, g, b] = Hsv { hue, saturation, intensity: intensity_at(at) }.display();
        colour.apply([r, g, b, 1.])
    };
    let mut vertices = Vec::with_capacity((usize::from(STEPS) + 2) * 6);
    let mut slice = |left: f32, right: f32, colours: [[f32; 4]; 2]| {
        let corners = [([left, top], colours[0]), ([right, top], colours[1]), ([right, bottom], colours[1]), ([left, bottom], colours[0])];
        for index in [0, 1, 2, 0, 2, 3] {
            let (point, rgba) = corners[index];
            vertices.push(vertex(m, point, [0., 0.], rgba, size, flat));
        }
    };
    slice(0., from, [tint(0.); 2]);
    for step in 0..STEPS {
        let ats = [f32::from(step), f32::from(step + 1)].map(|s| s / f32::from(STEPS));
        slice((to - from).mul_add(ats[0], from), (to - from).mul_add(ats[1], from), [tint(ats[0]), tint(ats[1])]);
    }
    slice(to, width, [tint(1.); 2]);
    solid(out, vertices);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Matrix = Matrix { a: 1., b: 0., c: 0., d: 1., tx: 0., ty: 0. };

    /// The stage point of a flat-stage vertex.
    fn stage(v: &[f32; 10]) -> [f32; 2] {
        [(v[0] + 1.) * 640., (1. - v[1]) * 360.]
    }

    #[test]
    fn the_wheel_puts_red_at_its_top_and_white_at_its_centre() {
        let mut out = Vec::new();
        wheel(IDENTITY, Colour::default(), (1280, 720), &mut out, true);
        let colours = out.last().unwrap();
        assert!(colours.texture.is_none() && colours.vertices.len() % 3 == 0);
        let top = colours.vertices.iter().find(|v| { let p = stage(v); p[0].abs() < 0.01 && (p[1] + WHEEL_RADIUS).abs() < 0.01 }).unwrap();
        assert!((top[6] - 1.).abs() < 1e-4 && top[7].abs() < 1e-4 && top[8].abs() < 1e-4, "red at the top: {:?}", &top[6..]);
        let centre = colours.vertices.iter().find(|v| { let p = stage(v); p[0].abs() < 0.01 && p[1].abs() < 0.01 }).unwrap();
        assert!(centre[6..9].iter().all(|c| (c - 1.).abs() < 1e-4), "white at the centre");
    }

    #[test]
    fn the_ramp_runs_from_black_through_the_colour_inside_its_frame() {
        let mut clip_out = Vec::new();
        ramp("260 17 243", "0 1", IDENTITY, Colour::default(), (1280, 720), &mut clip_out, true).unwrap();
        let colours = clip_out.last().unwrap();
        let x = |v: &[f32; 10]| stage(v)[0];
        assert!(colours.vertices.iter().all(|v| x(v) > -0.01 && x(v) < 260.01), "every slice inside the frame");
        let black = |v: &[f32; 10]| v[6..9].iter().all(|c| c.abs() < 1e-4);
        assert!(colours.vertices.iter().filter(|v| x(v) < 17.01).all(black), "black up to the knob's lowest middle");
        let full = 17. + (243. - 17.) * skate2_core::ui::colour_picker::FULL_AT;
        let red = colours.vertices.iter().find(|v| (x(v) - full).abs() < 0.01).unwrap();
        assert!((red[6] - 1.).abs() < 1e-4 && red[7].abs() < 1e-4, "the full red where intensity is 1");
        assert!(ramp("260 17", "0 1", IDENTITY, Colour::default(), (1280, 720), &mut Vec::new(), true).is_err());
        assert_eq!(clip_out.len(), 3, "shadow, frame and colours");
    }
}
