//! Per-frame WorldPainter fog and sky blending, as
//! `RenderMan::UpdateWorldPainterAttribs` 0x82848ba8 drives it.
//!
//! Reference (debug `sk82_na_m.xex`):
//! - `RenderMan::RenderMan` 0x82854268 builds `m_FogAttribs` and `m_SkyAttribs`
//!   on the `default` collection (key 0xd7edbd362d7d2152) with a 5 s blend.
//! - Each frame `UpdateWorldPainterAttribs` takes the VisualDirector keys
//!   (`VD_WorldPainterInfo`) when present, else the render location's
//!   `material_fog` / `material_sky` references, and calls
//!   `BlendAttribCollection<T>::Update(dt, key)` 0x828445f8 / 0x82844718: a
//!   changed key re-targets every attribute (`ChangeCollection` 0x8283d7b8 /
//!   0x8283c038; a missing collection targets the initial defaults), then
//!   `UpdateCollection` 0x8283d9a0 / 0x8283c0d8 advances the blend.
//! - `BlendAttrib<float>` / `BlendAttrib<Vector4>` (`Initialize` 0x828262b8 /
//!   0x828261d8, `Change` 0x8283bcf0, `Update` 0x8283bc48): lerp' =
//!   `fmadds(invBlend, dt, lerp)`, clamped to exactly 1 by `fsel`; output =
//!   `fmadds(1 - lerp, begin, end * lerp)`.
//!
//! Exposure parameters blend over5s and bloom over2s. GPU PostFX owns the
//! delayed luminance feedback; this owner supplies the authored targets.

use crate::{
    gpu_renderer::FrameEnvironment,
    render_environment::{DEFAULT_COLLECTION_KEY, EnvironmentLighting, FogAttributes},
};
use skate2_core::WorldPainterKeys;
use std::collections::BTreeMap;

/// `RenderMan::RenderMan` 0x82854268: `lfs f31, 5.0` for fog and sky.
const FOG_SKY_BLEND_TIME: f32 = 5.0;

#[derive(Clone, Copy, Debug)]
struct BlendScalar {
    default: f32,
    begin: f32,
    end: f32,
    lerp: f32,
    inv_blend_time: f32,
}

impl BlendScalar {
    fn new(value: f32, blend_time: f32) -> Self {
        Self {
            default: value,
            begin: value,
            end: value,
            lerp: 1.0,
            inv_blend_time: 1.0 / blend_time,
        }
    }

    fn current(&self) -> f32 {
        (1.0 - self.lerp).mul_add(self.begin, self.end * self.lerp)
    }

    fn change(&mut self, value: f32) {
        self.begin = self.current();
        self.end = value;
        self.lerp = 0.0;
    }

    fn update(&mut self, dt: f32) -> f32 {
        let lerp = self.inv_blend_time.mul_add(dt, self.lerp);
        self.lerp = if lerp - 1.0 >= 0.0 { 1.0 } else { lerp };
        self.current()
    }
}

#[derive(Clone, Copy, Debug)]
struct BlendVector {
    default: [f32; 4],
    begin: [f32; 4],
    end: [f32; 4],
    lerp: f32,
    inv_blend_time: f32,
}

impl BlendVector {
    fn new(value: [f32; 4], blend_time: f32) -> Self {
        Self {
            default: value,
            begin: value,
            end: value,
            lerp: 1.0,
            inv_blend_time: 1.0 / blend_time,
        }
    }

    fn current(&self) -> [f32; 4] {
        let remaining = 1.0 - self.lerp;
        std::array::from_fn(|lane| self.begin[lane].mul_add(remaining, self.end[lane] * self.lerp))
    }

    fn change(&mut self, value: [f32; 4]) {
        self.begin = self.current();
        self.end = value;
        self.lerp = 0.0;
    }

    fn update(&mut self, dt: f32) -> [f32; 4] {
        let lerp = self.inv_blend_time.mul_add(dt, self.lerp);
        self.lerp = if lerp - 1.0 >= 0.0 { 1.0 } else { lerp };
        self.current()
    }
}

/// `BlendAttribMaterialFogType`: colour, far (end), max (strength), near
/// (start) and power.
#[derive(Clone, Copy, Debug)]
struct FogBlend {
    current_key: u64,
    colour: BlendVector,
    far: BlendScalar,
    max: BlendScalar,
    near: BlendScalar,
    power: BlendScalar,
}

impl FogBlend {
    fn new(key: u64, fog: &FogAttributes) -> Self {
        Self {
            current_key: key,
            colour: BlendVector::new(fog.colour, FOG_SKY_BLEND_TIME),
            far: BlendScalar::new(fog.end, FOG_SKY_BLEND_TIME),
            max: BlendScalar::new(fog.strength, FOG_SKY_BLEND_TIME),
            near: BlendScalar::new(fog.start, FOG_SKY_BLEND_TIME),
            power: BlendScalar::new(fog.power, FOG_SKY_BLEND_TIME),
        }
    }

    fn update(&mut self, dt: f32, key: u64, fogs: &BTreeMap<u64, FogAttributes>) -> FogAttributes {
        if key != self.current_key {
            self.current_key = key;
            if let Some(fog) = fogs.get(&key) {
                self.colour.change(fog.colour);
                self.far.change(fog.end);
                self.max.change(fog.strength);
                self.near.change(fog.start);
                self.power.change(fog.power);
            } else {
                self.colour.change(self.colour.default);
                self.far.change(self.far.default);
                self.max.change(self.max.default);
                self.near.change(self.near.default);
                self.power.change(self.power.default);
            }
        }
        FogAttributes {
            colour: self.colour.update(dt),
            end: self.far.update(dt),
            strength: self.max.update(dt),
            start: self.near.update(dt),
            power: self.power.update(dt),
        }
    }
}

/// `BlendAttribMaterialParamsType<material_sky, 2>`: `m_params[0..2]`.
#[derive(Clone, Copy, Debug)]
struct SkyBlend {
    current_key: u64,
    params: [BlendVector; 2],
}

impl SkyBlend {
    fn new(key: u64, params: &[[f32; 4]; 2]) -> Self {
        Self {
            current_key: key,
            params: params.map(|value| BlendVector::new(value, FOG_SKY_BLEND_TIME)),
        }
    }

    fn update(&mut self, dt: f32, key: u64, skies: &BTreeMap<u64, [[f32; 4]; 2]>) -> [[f32; 4]; 2] {
        if key != self.current_key {
            self.current_key = key;
            for (index, blend) in self.params.iter_mut().enumerate() {
                let target = skies
                    .get(&key)
                    .map_or(blend.default, |params| params[index]);
                blend.change(target);
            }
        }
        [self.params[0].update(dt), self.params[1].update(dt)]
    }
}

/// The retained fog and sky blend state of one `RenderMan`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WorldPainterBlend {
    fog: FogBlend,
    sky: SkyBlend,
    bloom_key: u64,
    bloom: [BlendScalar; 5],
    exposure_key: u64,
    exposure: [BlendScalar; 4],
}

impl WorldPainterBlend {
    pub(crate) fn postfx(&mut self, environment: &EnvironmentLighting, dt: f32,
        keys: Option<&WorldPainterKeys>, now: std::time::Instant) -> crate::render_postfx::Frame {
        let [cube,exposure,bloom] = keys.filter(|_| environment.painted_postfx).map_or(environment.postfx.location,
            |k|[k.colour_cube,k.exposure,k.bloom]);
        if self.bloom_key != bloom {
            self.bloom_key=bloom;
            for (i,b) in self.bloom.iter_mut().enumerate() {
                b.change(environment.postfx.blooms.get(&bloom).map_or(b.default,|v|v[i]));
            }
        }
        if self.exposure_key != exposure {
            self.exposure_key=exposure;
            for (i,b) in self.exposure.iter_mut().enumerate() {
                b.change(environment.postfx.exposures.get(&exposure).map_or(b.default,|v|v[i]));
            }
        }
        crate::render_postfx::Frame {
            bloom: std::array::from_fn(|i|self.bloom[i].update(dt)),
            exposure: std::array::from_fn(|i|self.exposure[i].update(dt)),
            cube, dt, updated_at: now, cinematic: false, camera_planes: None, hom: None, final_mult: [1.; 4], keep_matrices: false, tazer: false, ice: None,
            fe_online_menu: false,
            luminance_scale: environment.luminance_scale,
        }
    }
    pub(crate) fn new(environment: &EnvironmentLighting) -> Self {
        let default_fog = environment
            .fogs
            .get(&DEFAULT_COLLECTION_KEY)
            .copied()
            .unwrap_or(FogAttributes {
                colour: [0.0; 4],
                power: 0.0,
                start: 0.0,
                strength: 0.0,
                end: 0.0,
            });
        let default_sky = environment
            .skies
            .get(&DEFAULT_COLLECTION_KEY)
            .copied()
            .unwrap_or([[0.0; 4]; 2]);
        Self {
            fog: FogBlend::new(DEFAULT_COLLECTION_KEY, &default_fog),
            sky: SkyBlend::new(DEFAULT_COLLECTION_KEY, &default_sky),
            bloom_key: crate::render_postfx::DEFAULT_BLOOM,
            bloom: environment.postfx.blooms[&crate::render_postfx::DEFAULT_BLOOM].map(|v|BlendScalar::new(v,2.)),
            exposure_key: crate::render_postfx::DEFAULT_EXPOSURE,
            exposure: environment.postfx.exposures[&crate::render_postfx::DEFAULT_EXPOSURE].map(|v|BlendScalar::new(v,5.)),
        }
    }

    /// PC port: the next step blends to the environment's values for every key, the keys it
    /// holds included. A linked Skate 3 district's environment names its collections as
    /// Skate 2's do (`default`, `auto_exposure`, `bloom`) with values of its own, which a blend
    /// holding the same key would otherwise never take; and a key the environment lacks (an
    /// unpainted point) blends to its defaults, which Skate 3's `RenderMan` builds from its own
    /// collections.
    pub(crate) fn retarget(&mut self, environment: &EnvironmentLighting) {
        self.fog.current_key = u64::MAX;
        self.sky.current_key = u64::MAX;
        self.bloom_key = u64::MAX;
        self.exposure_key = u64::MAX;
        if let Some(fog) = environment.fogs.get(&DEFAULT_COLLECTION_KEY) {
            self.fog.colour.default = fog.colour;
            self.fog.far.default = fog.end;
            self.fog.max.default = fog.strength;
            self.fog.near.default = fog.start;
            self.fog.power.default = fog.power;
        }
        if let Some(sky) = environment.skies.get(&DEFAULT_COLLECTION_KEY) {
            for (blend, value) in self.sky.params.iter_mut().zip(sky) {
                blend.default = *value;
            }
        }
        if let Some(bloom) = environment.postfx.blooms.get(&crate::render_postfx::DEFAULT_BLOOM) {
            for (blend, value) in self.bloom.iter_mut().zip(bloom) {
                blend.default = *value;
            }
        }
        if let Some(exposure) = environment.postfx.exposures.get(&crate::render_postfx::DEFAULT_EXPOSURE) {
            for (blend, value) in self.exposure.iter_mut().zip(exposure) {
                blend.default = *value;
            }
        }
    }

    /// One `UpdateWorldPainterAttribs` step: WorldPainter keys when the
    /// VisualDirector supplied them, else the render location's references.
    /// `light_direction` is the `g_vLightDir` `UpdateRenderParams` builds
    /// right after this blend.
    pub(crate) fn update(
        &mut self,
        environment: &EnvironmentLighting,
        dt: f32,
        keys: Option<&WorldPainterKeys>,
        light_direction: [f32; 4],
    ) -> FrameEnvironment {
        let (fog_key, sky_key) = keys.map_or(
            (environment.location_fog_key, environment.location_sky_key),
            |keys| (keys.fog, keys.sky),
        );
        let fog = self.fog.update(dt, fog_key, &environment.fogs);
        let sky_params = self.sky.update(dt, sky_key, &environment.skies);
        let (fog_k1, fog_colour) = fog.constants();
        FrameEnvironment {
            fog_k1,
            fog_colour,
            sky_params,
            light_direction,
            ..FrameEnvironment::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_starts_settled_and_reaches_target_after_blend_time() {
        let mut blend = BlendScalar::new(2.0, 5.0);
        assert_eq!(blend.update(0.5).to_bits(), 2.0_f32.to_bits());
        blend.change(4.0);
        assert!((blend.update(2.5) - 3.0).abs() < 1e-6);
        // Past the end the `fsel` clamp lands exactly on the target.
        assert_eq!(blend.update(10.0).to_bits(), 4.0_f32.to_bits());
    }

    #[test]
    fn change_mid_blend_starts_from_the_current_value() {
        let mut blend = BlendVector::new([0.0; 4], 5.0);
        blend.change([10.0; 4]);
        let halfway = blend.update(2.5);
        blend.change([0.0; 4]);
        assert_eq!(blend.update(0.0), halfway);
    }

    #[test]
    fn unknown_key_blends_back_to_the_defaults() {
        let default = FogAttributes {
            colour: [0.1, 0.2, 0.3, 1.0],
            power: 1.4,
            start: 50.0,
            strength: 1.0,
            end: 300.0,
        };
        let other = FogAttributes {
            colour: [0.5; 4],
            power: 2.0,
            start: 20.0,
            strength: 0.75,
            end: 300.0,
        };
        let fogs = BTreeMap::from([(DEFAULT_COLLECTION_KEY, default), (7, other)]);
        let mut blend = FogBlend::new(DEFAULT_COLLECTION_KEY, &default);
        assert_eq!(blend.update(10.0, 7, &fogs), other);
        assert_eq!(blend.update(10.0, 0, &fogs), default);
    }
}
