//! Optional authored clips, separate from the SHA-bound stock animation banks.
use crate::skate3_native::math_host::*;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct File {
    version: u32,
    clips: Vec<Clip>,
}

#[derive(Deserialize)]
pub(super) struct Clip {
    pub name: String,
    pub fps: f32,
    pub names: Vec<String>,
    pub parents: Vec<i32>,
    pub frames: Vec<Vec<[f32; 10]>>,
}

pub(super) struct Clips {
    pub reach: Clip,
    pub mantle: Clip,
}
impl Clips {
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let path = root.join("private/custom/climbing.json");
        let bytes = match skate_data::asset_io::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let mut file: File =
            serde_json::from_slice(&bytes).map_err(|e| format!("Climbing clips: {e}"))?;
        if file.version != 1 {
            return Err("Unsupported climbing clip version".into());
        }
        for c in &file.clips {
            c.validate()?;
        }
        let mut take = |name| {
            file.clips
                .iter()
                .position(|c| c.name == name)
                .map(|i| file.clips.remove(i))
                .ok_or_else(|| format!("Missing climbing clip {name}"))
        };
        let reach = take("reach")?;
        let mantle = take("mantle")?;
        if reach.names != mantle.names || reach.parents != mantle.parents {
            return Err("Climbing clips must share a skeleton".into());
        }
        Ok(Some(Self { reach, mantle }))
    }
}
impl Clip {
    fn validate(&self) -> Result<(), String> {
        let valid = self.fps.is_finite()
            && self.fps > 0.
            && self.fps <= 240.
            && self.frames.len() >= 2
            && !self.names.is_empty()
            && self.parents.len() == self.names.len()
            && self
                .parents
                .iter()
                .enumerate()
                .all(|(i, &p)| p >= -1 && p < i as i32)
            && self
                .names
                .iter()
                .enumerate()
                .all(|(i, n)| !self.names[..i].contains(n))
            && self.frames.iter().all(|f| {
                f.len() == self.names.len()
                    && f.iter().all(|s| {
                        s.iter().all(|v| v.is_finite())
                            && s[..3].iter().all(|v| *v > 0.0001)
                            && (Quat::from_array([s[3], s[4], s[5], s[6]]).length() - 1.).abs()
                                < 0.01
                    })
            });
        if !valid {
            return Err(format!("Invalid climbing clip {}", self.name));
        }
        for name in [
            "HIPS",
            "SPINE3",
            "LEFTSHOULDER",
            "RIGHTSHOULDER",
            "LEFTARM",
            "RIGHTARM",
            "LEFTFOREARM",
            "RIGHTFOREARM",
            "LEFTHAND",
            "RIGHTHAND",
            "LEFTFOOT",
            "RIGHTFOOT",
            "SKATEBOARD_ROOT",
        ] {
            if !self.names.iter().any(|n| n == name) {
                return Err(format!("Missing climbing bone {name}"));
            }
        }
        Ok(())
    }
    pub fn duration(&self) -> f32 {
        (self.frames.len() - 1) as f32 / self.fps
    }
    pub fn sample(&self, time: f32) -> Vec<Transform> {
        let frame = (time * self.fps).clamp(0., (self.frames.len() - 1) as f32);
        let i = frame.floor() as usize;
        let j = (i + 1).min(self.frames.len() - 1);
        self.frames[i]
            .iter()
            .zip(&self.frames[j])
            .map(|(a, b)| blend(decode(a), decode(b), frame.fract()))
            .collect()
    }
    pub fn globals(&self, locals: &[Transform]) -> Vec<Mat4> {
        let mut result = Vec::with_capacity(locals.len());
        for (i, t) in locals.iter().enumerate() {
            let m = t.to_matrix();
            result.push(if self.parents[i] < 0 {
                m
            } else {
                result[self.parents[i] as usize] * m
            });
        }
        result
    }
    pub fn index(&self, name: &str) -> usize {
        self.names.iter().position(|n| n == name).unwrap()
    }
    pub fn hands(&self, globals: &[Mat4]) -> Vec3 {
        (globals[self.index("LEFTHAND")].w_axis.truncate()
            + globals[self.index("RIGHTHAND")].w_axis.truncate())
            * 0.5
    }
    pub fn feet(&self, globals: &[Mat4]) -> Vec3 {
        (globals[self.index("LEFTFOOT")].w_axis.truncate()
            + globals[self.index("RIGHTFOOT")].w_axis.truncate())
            * 0.5
            - Vec3::Y * 0.11
    }
}
fn decode(s: &[f32; 10]) -> Transform {
    Transform {
        scale: Vec3::new(s[0], s[1], s[2]),
        rotation: Quat::from_xyzw(s[3], s[4], s[5], s[6]).normalize(),
        translation: Vec3::new(s[7], s[8], s[9]),
    }
}
pub(super) fn blend(a: Transform, b: Transform, t: f32) -> Transform {
    Transform {
        translation: a.translation.lerp(b.translation, t),
        rotation: a.rotation.slerp(b.rotation, t),
        scale: a.scale.lerp(b.scale, t),
    }
}
