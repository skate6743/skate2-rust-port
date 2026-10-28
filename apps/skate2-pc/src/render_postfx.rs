//! Authored PostFX inputs. Shared with the existing WorldPainter/environment
//! owner; cube files are loaded through the existing mounted BIG reader.
use crate::{gpu_renderer::RenderResult, render_environment::VaultReader};
use skate2_core::challenge::attrib_db::AttribDb;
use skate2_formats::{attrib::string_hash64, mount::MountSet};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) const DEFAULT_BLOOM: u64 = 0x3c71d6b414304f99;
pub(crate) const DEFAULT_EXPOSURE: u64 = 0x2b00cf9bb190ce8d;
pub(crate) const DEFAULT_CUBE: u64 = 0xd7edbd362d7d2152;
/// `colour_cube`'s file name.
const CUBE_FILE_KEY: u64 = 0xbe88128a30be926e;
/// A `post_fx` bloom collection's threshold, saturation, coarse, medium and fine intensity.
const BLOOM_KEYS: [u64; 5] = [0x1c226b5fdd1a430d, 0x9ef7c9eade67e97c, 0x8e28805b5a916828, 0xf3dcd6b381bce484, 0x88ebac70289b826d];
/// A `rendering` auto-exposure collection's four values.
const EXPOSURE_KEYS: [u64; 4] = [0x1458f4fd2118e6cf, 0x971d1dc6e522cf09, 0xa3bc00dbc916b065, 0x9807fa11bf89a40e];

/// A colour cube's 32^3 RGB file, decoded once per name; `None` when the file is not mounted (the
/// vault includes unreleased locations, e.g. X Games). ASyncLoad/UpdateColourCube retain the
/// current cube when the requested asset has not loaded. Never invent a LUT.
fn cube_rgba(mounts: &MountSet, name: String, files: &mut BTreeMap<String, Arc<[u8]>>) -> RenderResult<Option<Arc<[u8]>>> {
    if let Some(bytes) = files.get(&name) {
        return Ok(Some(bytes.clone()));
    }
    let bytes = match mounts.read_path(&format!("data/colourcubes/{name}")) {
        Ok(bytes) => bytes,
        Err(skate2_formats::mount::MountError::EntryNotFound(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if bytes.len() != 32 * 32 * 32 * 3 {
        return Err(format!(
            "colour cube {name}: expected32^3 RGB bytes, got{}",
            bytes.len()
        )
        .into());
    }
    // Blender expands directly into the native tiled texture.
    // The PC volume texture needs linear order instead.
    let rgba = crate::render_postfx_math::decode_cube(&bytes)
        .ok_or("colour cube volume layout")?;
    let rgba: Arc<[u8]> = rgba.into();
    files.insert(name, rgba.clone());
    Ok(Some(rgba))
}

#[derive(Clone, Debug)]
pub(crate) struct Assets {
    pub cubes: BTreeMap<u64, Arc<[u8]>>,
    /// threshold, saturation, coarse/medium/fine intensity (native layout).
    pub blooms: BTreeMap<u64, [f32; 5]>,
    pub exposures: BTreeMap<u64, [f32; 4]>,
    pub location: [u64; 3],
    pub matrix: [[f32; 4]; 3],
    pub hom: HomTreatment,
    /// GenerateBloomBuffers 0x82832ddc..eb8: postfx/Debug UV offset,
    /// expressed in original main2 texels before the quarter-size draw.
    pub bloom_sample_offset: f32,
    /// PostFX constructor82839530 / DownsampleForScreenshot82832a0c.
    pub watermark_position: [f32;2],
    /// `m_deckAOArtribs` (the PostFX constructor 0x828394b0: `rendering` 0x031c083f119190b8) as
    /// `CalculateDeckShadowMatrix` 0x82846458..0x82846524 reads it: `DeckAO_XLen`, `ZLen`,
    /// `Radius` and `HeightFallOff`, each 0 (`DefaultDataArea`) when absent.
    pub deck_ao: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub bloom: [f32; 5],
    pub exposure: [f32; 4],
    pub cube: u64,
    pub dt: f32,
    /// Identifies the presentation update; several draws can reuse this frame.
    pub updated_at: std::time::Instant,
    pub cinematic: bool,
    /// SetParameters8284bd48 always samples the current presentation camera,
    /// including ICELite's cinematic pass when no ICE/HoM treatment is active.
    pub camera_planes: Option<[f32; 2]>,
    pub hom: Option<HomInputs>,
    /// `g_FinalMult`, the NIS uber pass's last multiply.
    pub final_mult: [f32; 4],
    /// `VisualFX_Animation` (`0x82847610`) left the grades (`m_FXMatrixR/G/B_A` and `_B`) as
    /// the last render had them: its challenge fade branch writes neither.
    pub keep_matrices: bool,
    /// VisualFX_Animation82847ac0: both colour grades invert while tazed.
    pub tazer: bool,
    /// What `PostFX::SetParameters` (`0x8284bbf0`) takes while the ICE
    /// post-fx is active (`mICEPostfxActive`).
    pub ice: Option<IceInputs>,
    /// `PostFX::m_bFEOnlineMenu`, set by `FrontEndState_MainMenuOnlineOptions::OnEnter`
    /// (`0x8264b6b0`) and cleared by its `OnLeave` (`0x8264b724`).
    pub fe_online_menu: bool,
}

/// `SetParameters`' ICE branch inputs and the camera planes it turns into
/// `m_DOFCameraInfoX`/`Y`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct IceInputs {
    /// `m_RadialBlur`.
    pub radial_blur: f32,
    /// `m_ScalineOffset.x`.
    pub scanline: f32,
    /// `fisheyeWarpMultiplier`.
    pub fisheye: f32,
    /// `m_FXMatrixR/G/B_A` (near) and `_B` (far).
    pub matrices: [[[f32; 4]; 3]; 2],
    /// `m_NISNearDist` (`nisFarEnd`) and `m_NISFarDist` (`nisFarStart`).
    pub lerp: [f32; 2],
    /// `tonemapValue`.
    pub tonemap: f32,
    /// `CameraPresentation::GetNearPlane` and `GetFarPlane`.
    pub planes: [f32; 2],
}

/// PostFX constructor 828397cc/8283984c/828398cc binds these authored
/// Hall of Meat collections. PointNegGraph has a 16-byte editor header.
#[derive(Clone, Debug, Default)]
pub(crate) struct HomTreatment {
    matrices: [[[f32; 4]; 3]; 2],
    pub(crate) normal_far: [[f32; 4]; 3],
    distance: [f32; 2],
    pub(crate) normal_distance: [f32; 2],
    bloom: [f32; 2],
    curves: [[f32; 12]; 3],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HomInputs {
    pub matrices: [[[f32; 4]; 3]; 2],
    pub lerp: [f32; 2],
    pub planes: [f32; 2],
    pub overlay: f32,
    pub vignette: f32,
}

impl HomTreatment {
    fn read(vault: &VaultReader<'_>, post: u64) -> RenderResult<Self> {
        let matrix = |key, far| -> RenderResult<[[f32; 4]; 3]> {
            let keys = if far {
                [0xf7b10b6e4abb612c, 0xab5872d5385f99b2, 0xab72f15930ed87f1, 0x81713a2d07544613, 0x671ba9ab8e163c3b]
            } else {
                [0x297a6d21445ae451, 0xf50a2257b30758ca, 0xeb1a6826b18e44a3, 0x04394c39632a1f2d, 0x5f75df46f653ce96]
            };
            Ok(crate::render_postfx_math::colour_matrix(
                vault.floats::<1>(post,key,keys[0])?[0], vault.floats::<1>(post,key,keys[1])?[0],
                vault.floats::<1>(post,key,keys[2])?[0], vault.floats::<3>(post,key,keys[3])?, vault.floats::<3>(post,key,keys[4])?))
        };
        let distance = |key| -> RenderResult<[f32; 2]> {
            Ok([vault.floats::<1>(post,key,0xe6820c3b7f79cc08)?[0], vault.floats::<1>(post,key,0x3d5a3f5fc9f0fda3)?[0]])
        };
        let key = 0xbbd0b8dae3af1522;
        let functions = 0x65d868cee4be7ba4;
        let bloom = 0x7e9942d39fb220e8;
        Ok(Self {
            matrices: [matrix(key,false)?,matrix(key,true)?],
            normal_far: matrix(0x2d9c5e5c45ce93dc,true)?,
            distance: distance(key)?, normal_distance: distance(0x2d9c5e5c45ce93dc)?,
            bloom: [vault.floats::<1>(post,bloom,0x1c226b5fdd1a430d)?[0],vault.floats::<1>(post,bloom,0x9ef7c9eade67e97c)?[0]],
            curves: [vault.floats::<12>(post,functions,0x259acd98ca3cfa0c)?,
                vault.floats::<12>(post,functions,0x137fd3ae19dda9a5)?,
                vault.floats::<12>(post,functions,0xdadfe11fc593d1dd)?],
        })
    }
}

impl Assets {
    /// VisualFX_Animation 82847b00..82847e78: HoM precedes challenge results
    /// and fades. ICE subsequently replaces the grade/bloom, but keeps the
    /// score overlay and vignette produced here.
    pub(crate) fn apply_hom(&self, frame: &mut Frame, bail: skate2_core::BailVisual, planes: [f32; 2]) {
        let eval = |curve: [f32; 12], value: f32| {
            let graph = skate2_core::physics::PointGraph::<4> {
                x: curve[4..8].try_into().expect("four x points"),
                y: curve[8..12].try_into().expect("four y points"),
            };
            graph.evaluate(value)
        };
        let mut effect = eval(self.hom.curves[0], bail.score as f32);
        let time = eval(self.hom.curves[1], bail.time);
        if time > effect { effect = time; }
        let blend = |a: f32,b: f32| (1.0-effect)*a + b*effect;
        let normal = [self.matrix,self.hom.normal_far];
        frame.hom = Some(HomInputs {
            matrices: std::array::from_fn(|side| std::array::from_fn(|row| std::array::from_fn(|lane|
                blend(normal[side][row][lane],self.hom.matrices[side][row][lane])))),
            lerp: std::array::from_fn(|i| blend(self.hom.normal_distance[i],self.hom.distance[i])),
            planes, overlay: eval(self.hom.curves[2], bail.score as f32),
            vignette: effect*0.5+0.25,
        });
        frame.cinematic = true; // native m_b30HzMode selects VisualFX + Uber
        frame.keep_matrices = false;
        // The branch leaves m_bloomMultiply at its initialized (1,1,0,0).
        frame.bloom = [blend(frame.bloom[0],self.hom.bloom[0]),blend(frame.bloom[1],self.hom.bloom[1]),0.,1.,1.];
    }

    /// The colour cube, bloom and auto-exposure collections of every vault `db` holds that these
    /// lack: the downloadable content's, which its render locations name.
    pub(crate) fn add_vault_collections(&mut self, mounts: &MountSet, db: &AttribDb) -> RenderResult<()> {
        let cube = string_hash64("colour_cube");
        let mut files = BTreeMap::new();
        for &key in db.collections(cube) {
            if self.cubes.contains_key(&key) {
                continue;
            }
            let Some(name) = db.attribute(cube, key, CUBE_FILE_KEY).and_then(|value| value.text(0)).filter(|name| !name.is_empty()) else {
                continue;
            };
            if let Some(rgba) = cube_rgba(mounts, name.to_owned(), &mut files)? {
                self.cubes.insert(key, rgba);
            }
        }
        let float = |class: u64, key: u64, attribute: u64| db.attribute(class, key, attribute).and_then(|value| value.f32(0));
        let post = string_hash64("post_fx");
        for &key in db.collections(post) {
            if !self.blooms.contains_key(&key)
                && let [Some(a), Some(b), Some(c), Some(d), Some(e)] = BLOOM_KEYS.map(|attribute| float(post, key, attribute))
            {
                self.blooms.insert(key, [a, b, c, d, e]);
            }
        }
        let rendering = string_hash64("rendering");
        for &key in db.collections(rendering) {
            if !self.exposures.contains_key(&key)
                && let [Some(a), Some(b), Some(c), Some(d)] = EXPOSURE_KEYS.map(|attribute| float(rendering, key, attribute))
            {
                self.exposures.insert(key, [a, b, c, d]);
            }
        }
        Ok(())
    }

    pub(crate) fn read(
        vault: &VaultReader<'_>,
        mounts: &MountSet,
        location: u64,
    ) -> RenderResult<Self> {
        let post = string_hash64("post_fx");
        let rendering = string_hash64("rendering");
        let cube = string_hash64("colour_cube");
        let mut cubes = BTreeMap::new();
        let mut files: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
        for key in vault.collection_keys(cube) {
            let name = vault.string(cube, key, CUBE_FILE_KEY)?;
            if name.is_empty() {
                continue;
            } // ASyncLoad8284dad4 rejects empty filenames.
            let Some(rgba) = cube_rgba(mounts, name, &mut files)? else { continue };
            cubes.insert(key, rgba);
        }
        let mut blooms = BTreeMap::new();
        for key in vault.collection_keys(post) {
            let mut bloom = [0.0; 5];
            for (value, attribute) in bloom.iter_mut().zip(BLOOM_KEYS) {
                *value = vault.floats::<1>(post, key, attribute)?[0];
            }
            blooms.insert(key, bloom);
        }
        let mut exposures = BTreeMap::new();
        for key in vault.collection_keys(rendering) {
            let mut exposure = [0.0; 4];
            for (value, attribute) in exposure.iter_mut().zip(EXPOSURE_KEYS) {
                *value = vault.floats::<1>(rendering, key, attribute)?[0];
            }
            exposures.insert(key, exposure);
        }
        let matrix_key = 0x2d9c5e5c45ce93dc; // PostFXconstructor8283980c.
        let matrix = crate::render_postfx_math::colour_matrix(
            vault.floats::<1>(post, matrix_key, 0x297a6d21445ae451)?[0],
            vault.floats::<1>(post, matrix_key, 0xf50a2257b30758ca)?[0],
            vault.floats::<1>(post, matrix_key, 0xeb1a6826b18e44a3)?[0],
            vault.floats::<3>(post, matrix_key, 0x04394c39632a1f2d)?,
            vault.floats::<3>(post, matrix_key, 0x5f75df46f653ce96)?,
        );
        let loc = string_hash64("render_locations");
        let location = [cube, DEFAULT_EXPOSURE, DEFAULT_BLOOM].map(|attribute| {
            vault
                .reference(loc, location, attribute)
                .map_or(0, |(_, key)| key)
        });
        if !cubes.contains_key(&DEFAULT_CUBE)
            || !blooms.contains_key(&DEFAULT_BLOOM)
            || !exposures.contains_key(&DEFAULT_EXPOSURE)
        {
            return Err("original PostFX defaults absent".into());
        }
        Ok(Self {
            cubes,
            blooms,
            exposures,
            location,
            matrix,
            hom: HomTreatment::read(vault, post)?,
            bloom_sample_offset: vault.floats::<1>(
                string_hash64("postfx"), 0x550740b00aa8d4ac, 0x88a3da4a1d1d4179,
            )?[0],
            watermark_position: vault.floats::<2>(rendering,0x7084fd225e986209,0xdf5765a619fffc5d)?,
            deck_ao: [0x196a_ed84_ab3e_f932, 0xc80d_f108_8e67_a19b, 0xdad1_d10d_a322_6ba2, 0xc541_5dd3_823b_6b29]
                .map(|key| vault.floats::<1>(rendering, 0x031c_083f_1191_90b8, key).map_or(0.0, |[value]| value)),
        })
    }
}
