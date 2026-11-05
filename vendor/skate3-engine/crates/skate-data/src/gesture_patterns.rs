//! Authored joystick PAT data consumed by TU3 82696B18.
use skate_core::input::gesture::Pattern;
use std::path::Path;

pub fn load(path: &Path) -> Result<Vec<Pattern>, String> {
    parse(&crate::asset_io::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn parse(source: &str) -> Result<Vec<Pattern>, String> {
    let mut patterns: Vec<Pattern> = Vec::new();
    let mut tolerance = None;
    for (line, source) in source.lines().enumerate() {
        let words: Vec<_> = source
            .split("//")
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if words.is_empty() {
            continue;
        }
        let fail = || format!("invalid PAT directive at line {}: {source}", line + 1);
        let number = |index: usize| -> Result<f32, String> {
            let value: f32 = words
                .get(index)
                .ok_or_else(fail)?
                .parse()
                .map_err(|_| fail())?;
            if !value.is_finite() {
                return Err(fail());
            }
            Ok(value)
        };
        match words[0] {
            "global_tolerance_dist" if words.len() == 2 => {
                let v = number(1)?;
                tolerance = Some(v * v);
            }
            // The native loader reads but does not store these values.
            "global_tolerance_time"
            | "global_tolerance_speed"
            | "global_anticipation_delay"
            | "tolerance_time"
                if words.len() == 2 =>
            {
                number(1)?;
            }
            "pattern" if words.len() == 2 => patterns.push(Pattern {
                name: words[1].into(),
                points: Vec::new(),
                tolerance_squared: tolerance
                    .ok_or_else(|| "PAT lacks global_tolerance_dist".to_owned())?,
            }),
            "tolerance_dist" if words.len() == 2 => {
                let v = number(1)?;
                patterns.last_mut().ok_or_else(fail)?.tolerance_squared = v * v;
            }
            "coord" if words.len() == 3 => {
                let point = [number(1)?, number(2)?];
                patterns.last_mut().ok_or_else(fail)?.points.push(point);
            }
            // Fail explicitly on polar or modified directives until their
            // transcendental implementation is provided. Stock skater.pat uses coord.
            _ => return Err(fail()),
        }
    }
    skate_core::input::gesture::Recognizer::new(patterns.clone())?;
    Ok(patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_order_and_per_pattern_tolerances_are_preserved() {
        let p = parse("global_tolerance_dist 0.4\nglobal_anticipation_delay 15\npattern Ollie\ntolerance_dist 0.55\ncoord 0 1\ncoord 0 -1\npattern Test\ncoord 1 0\ncoord -1 0").unwrap();
        assert_eq!(p[0].points, vec![[0.0, 1.0], [0.0, -1.0]]);
        assert_eq!(p[0].tolerance_squared, 0.55 * 0.55);
        assert_eq!(p[1].tolerance_squared, 0.4 * 0.4);
        assert!(parse("global_tolerance_dist 0.4\npattern Broken\ncoord 0 1").is_err());
    }
}
