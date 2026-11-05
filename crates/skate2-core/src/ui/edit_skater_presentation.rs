//! Authored CAS camera/body bindings; no room coordinates or poses are guessed.
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault};
use skate2_formats::{attrib::string_hash64, mount::MountSet};
use std::sync::Arc;

pub type EditSkaterResult<T> = Result<T, Box<dyn std::error::Error>>;

/// `CasCamera::SetTargetState` 825874f0. The render host supplies its usual
/// projection conversion; position/look/FOV remain the original Vault values.
#[derive(Clone, Debug)]
pub struct EditSkaterCamera {
    pub position: [f32; 3],
    pub look_position: [f32; 3],
    pub up: [f32; 3],
    pub fov: f32,
    /// CasCamera::SetTargetState changes PostFX.m_FEAttribsCSM, not the
    /// CameraPresentation rendering instance which owns the clipping planes.
    pub postfx_csm_collection: u64,
}

pub struct EditSkaterCameraProjection {
    pub clip_from_world: [f32; 16],
    pub axes: [[f32; 3]; 3],
}
impl EditSkaterPresentationAttributes {
    /// ComputeTransform82587350 followed by the shared RenderMan projection.
    /// Reciprocal estimate rounding has the existing host numeric boundary.
    pub fn project_camera(
        &self,
        camera: &EditSkaterCamera,
        near: f32,
    ) -> EditSkaterResult<EditSkaterCameraProjection> {
        use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};
        let normalize = |v: [f32; 3]| -> EditSkaterResult<[f32; 3]> {
            let mut numeric = HostReckoningNumeric;
            let vector = [v[0], v[1], v[2], 0.0];
            let length = numeric.dot3(vector, vector)?;
            if length <= 0.0 {
                return Err("degenerate authored CAS camera direction".into());
            }
            let seed = numeric.reciprocal_sqrt_estimate(length)?;
            let inverse =
                crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt(length, seed);
            Ok(v.map(|x| x * inverse))
        };
        let cross = |a: [f32; 3], b: [f32; 3]| {
            [
                (-a[2]).mul_add(b[1], a[1] * b[2]),
                (-a[0]).mul_add(b[2], a[2] * b[0]),
                (-a[1]).mul_add(b[0], a[0] * b[1]),
            ]
        };
        let z = normalize(std::array::from_fn(|i| {
            camera.look_position[i] - camera.position[i]
        }))?;
        let x = normalize(cross(camera.up, z))?;
        let y = cross(z, x);
        let matrix = crate::skeleton_root_transforms::SkeletonRootMatrix::known([
            x[0],
            x[1],
            x[2],
            0.0,
            y[0],
            y[1],
            y[2],
            0.0,
            z[0],
            z[1],
            z[2],
            0.0,
            camera.position[0],
            camera.position[1],
            camera.position[2],
            1.0,
        ]);
        // CameraPresentation ctor 827f217c..2194 retains rendering/default.
        // SetTargetState82587634..78 changes only PostFX.m_FEAttribsCSM.
        // Treating that RefSpec as rendering produced far=0 and collapsed
        // every surface to depth zero, making rear hair/skin cover the front.
        let key = 0xd7ed_bd36_2d7d_2152;
        let rendering = self.vault.instance(string_hash64("rendering"), key);
        if rendering.is_null() {
            return Err(format!("missing CAS rendering {key:016x}").into());
        }
        let far = layout_float(&rendering, 0x64)?;
        if !(near > 0.0 && far > near && far.is_finite()) {
            return Err(format!("invalid original CAS clipping planes: near={near}, far={far}").into());
        }
        // GetFOVY827f64b0 selects mICEFOV directly while CAS owns ICE;
        // rendering.override_fov applies only to the ordinary camera.
        let projection = crate::camera_session::producer_projection::project_producer(
            matrix, matrix, camera.fov, camera.fov, near, far, false, true, -1, 0,
        );
        Ok(EditSkaterCameraProjection {
            clip_from_world: projection.clip_from_world,
            axes: [x, y, z],
        })
    }
}

/// `CASCharacterEntity::SetBodyState` 8259e8c8 generated cac_body layout.
#[derive(Clone, Debug)]
pub struct EditSkaterBody {
    pub position: [f32; 3],
    pub rotation_radians: f32,
    pub animation: String,
    pub gesture: bool,
}

pub struct EditSkaterPresentationAttributes {
    vault: Arc<AttributeRuntimeVault>,
}

impl EditSkaterPresentationAttributes {
    pub fn load(mounts: &MountSet) -> EditSkaterResult<Self> {
        let classes = [
            string_hash64("cac_camera"),
            string_hash64("cac_body"),
            0xbabb_3ec1_1084_9e08,
            string_hash64("rendering"),
        ];
        let vault = AttributeRuntimeVault::try_load_mounted(mounts, "data/db/skater.vlt", &classes)
            .map_err(|e| e.to_string())?;
        Ok(Self { vault })
    }

    pub fn camera(&self, key: u64) -> EditSkaterResult<EditSkaterCamera> {
        let instance = self.vault.instance(string_hash64("cac_camera"), key);
        if instance.is_null() {
            return Err(format!("missing cac_camera {key:016x}").into());
        }
        let up = instance.try_get_data(0x5130_70d9_cbbe_801a, 0)?;
        let up = match up {
            Some(data) => vector(data.bytes())?,
            None => [0.0; 3], // SetTargetState's DefaultDataArea(16).
        };
        Ok(EditSkaterCamera {
            position: layout_vector(&instance, 0)?,
            look_position: layout_vector(&instance, 16)?,
            up,
            fov: layout_float(&instance, 32)?,
            postfx_csm_collection: instance
                .try_ref_spec_collection(0x241c_c15b_a939_c1ba)?
                .unwrap_or(0),
        })
    }

    pub fn body(&self, key: u64) -> EditSkaterResult<EditSkaterBody> {
        let instance = self.vault.instance(string_hash64("cac_body"), key);
        if instance.is_null() {
            return Err(format!("missing cac_body {key:016x}").into());
        }
        let animation = instance
            .try_cached_layout_text(16)?
            .ok_or("cac_body animation name is null")?;
        Ok(EditSkaterBody {
            position: layout_vector(&instance, 0)?,
            rotation_radians: layout_float(&instance, 20)? * f32::from_bits(0x3c8e_fa35),
            animation: std::str::from_utf8(animation.bytes())?
                .trim_end_matches('\0')
                .to_owned(),
            gesture: instance
                .try_cached_layout_field(24, 1)?
                .is_some_and(|d| d.bytes()[0] != 0),
        })
    }
}

impl EditSkaterBody {
    /// Y rotation from UpdateTransform82587c88, using the existing native
    /// trigonometric helper (host estimate limits remain those of that helper).
    pub fn local_to_world(&self) -> EditSkaterResult<[f32; 16]> {
        let (s, c) = crate::reckoning_sin_cos::try_sin_cos(
            self.rotation_radians,
            &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
        )
        .map_err(|e| format!("CAS rotation: {e:?}"))?;
        Ok([
            c,
            0.0,
            -s,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            s,
            0.0,
            c,
            0.0,
            self.position[0],
            self.position[1],
            self.position[2],
            1.0,
        ])
    }
}

/// Synchronized CAS pose output. Sampling and bind/face ownership stay with the
/// caller; this method does not substitute a T pose for an unavailable clip.
/// `GetAnimTree8258e618`, `EvalCurrAnim8258f178`, `ComputeExpressions8258d188`.
impl EditSkaterPresentationAttributes {
    pub fn publish_pose(
        &self,
        hierarchy: &skate2_formats::andale::AndaleHierarchy,
        bind: &[skate2_formats::andale::AndaleSqt],
        delta: &[skate2_formats::andale::AndaleSqt],
        body: &EditSkaterBody,
        palette: &mut crate::character::palette::CharacterPalette,
    ) -> EditSkaterResult<[f32; 16]> {
        use crate::actor_animation_construction::initialization::body_dispatch::tree_commands::numeric::add_sqt;
        if bind.len() != hierarchy.bone_names.len() || delta.len() != bind.len() {
            return Err("CAS bind/clip hierarchy length mismatch".into());
        }
        let pose: Vec<_> = delta
            .iter()
            .zip(bind)
            .map(|(a, b)| add_sqt(*a, *b, true))
            .collect();
        self.publish_local_pose(hierarchy,bind,pose,body,palette)
    }
    fn publish_local_pose(&self,hierarchy:&skate2_formats::andale::AndaleHierarchy,
        bind:&[skate2_formats::andale::AndaleSqt],mut pose:Vec<skate2_formats::andale::AndaleSqt>,
        body:&EditSkaterBody,palette:&mut crate::character::palette::CharacterPalette)->EditSkaterResult<[f32;16]> {
        use crate::{actor_face_playback::presentation::{move_root,twist::try_twist_channels},
            animation::{AnimationSkeleton,LocalPose,apply_inverse_bind},reckoning_numeric::HostReckoningNumeric,
            reckoning_sin_cos::HostReckoningSinCosNumeric};
        let bone=|name:&str|hierarchy.bone_names.iter().position(|b|b.eq_ignore_ascii_case(name))
            .ok_or_else(||format!("CAS hierarchy missing {name}"));
        let hips = bone("Hips")?;
        let trajectory = bone("Trajectory")?;
        let saved_hips = move_root::rigid_matrix(pose[hips].rotation, pose[hips].translation);
        pose[hips].scale = [1.0; 4];
        pose[hips].rotation = [0.0, 0.0, 0.0, 1.0];
        pose[hips].translation = [0.0, 0.0, 0.0, 1.0];
        let ratios = self
            .vault
            .instance(0xbabb_3ec1_1084_9e08, 0x2db7_7096_45a1_9ce8);
        for side in ["Left", "Right"] {
            for (source, target, key) in [
                ("Arm", "ArmTwist", 0x3831_7705_e5a4_571a),
                ("ForeArm", "ForeArmTwist", 0x246f_c198_b6f1_624d),
                ("ForeArm", "ForeArmTwist1", 0x5cdb_40ca_8c5a_5044),
                ("UpLeg", "UpLegTwist", 0x3eb7_0888_6e6e_4607),
            ] {
                let value = f32::from_bits(ratios.try_scalar_word(key, 0)?.unwrap_or(0));
                try_twist_channels(
                    &mut CasChannels {
                        bind,
                        pose: &mut pose,
                    },
                    bone(&format!("{side}{source}"))?,
                    bone(&format!("{side}{target}"))?,
                    value,
                    &mut HostReckoningNumeric,
                    &mut HostReckoningSinCosNumeric,
                )
                .map_err(|e| format!("CAS twist: {e}"))?;
            }
        }
        let skeleton = AnimationSkeleton::from_andale(hierarchy)?;
        let global = skeleton.global_matrices(&LocalPose::from_andale(&pose))?;
        let mut matrices =
            apply_inverse_bind(&global, &palette.skinning().hierarchy_inverse_bind_pose)?;
        let root = move_root::multiply(
            move_root::multiply(body.local_to_world()?, matrices[trajectory]),
            saved_hips,
        );
        matrices[trajectory] = saved_hips;
        palette.update_bone_transforms(&matrices)?;
        palette.pack_for_upload(root)?;
        Ok(root)
    }
}

struct CasChannels<'a> {
    bind: &'a [skate2_formats::andale::AndaleSqt],
    pose: &'a mut [skate2_formats::andale::AndaleSqt],
}
impl crate::actor_face_playback::presentation::corrections::CorrectionChannels for CasChannels<'_> {
    fn bind(&self, i: usize) -> Option<skate2_formats::andale::AndaleSqt> {
        self.bind.get(i).copied()
    }
    fn replay(&self, i: usize) -> Option<skate2_formats::andale::AndaleSqt> {
        self.pose.get(i).copied()
    }
    fn rotation(
        &mut self,
        i: usize,
        v: [f32; 4],
    ) -> Result<(), crate::actor_face_playback::ActorFaceError> {
        self.pose
            .get_mut(i)
            .ok_or(crate::actor_face_playback::ActorFaceError::UnsupportedPose)?
            .rotation = v;
        Ok(())
    }
    fn translation(
        &mut self,
        i: usize,
        v: [f32; 4],
    ) -> Result<(), crate::actor_face_playback::ActorFaceError> {
        self.pose
            .get_mut(i)
            .ok_or(crate::actor_face_playback::ActorFaceError::UnsupportedPose)?
            .translation = v;
        Ok(())
    }
    fn scale(
        &mut self,
        i: usize,
        v: [f32; 4],
    ) -> Result<(), crate::actor_face_playback::ActorFaceError> {
        self.pose
            .get_mut(i)
            .ok_or(crate::actor_face_playback::ActorFaceError::UnsupportedPose)?
            .scale = v;
        Ok(())
    }
}

fn layout_vector(instance: &AttributeRuntimeInstance, offset: usize) -> EditSkaterResult<[f32; 3]> {
    let data = instance.try_cached_layout_field(offset, 16)?;
    data.map_or(Ok([0.0; 3]), |data| vector(data.bytes()))
}
fn vector(bytes: &[u8]) -> EditSkaterResult<[f32; 3]> {
    if bytes.len() < 12 {
        return Err("short CAS vector".into());
    }
    Ok(std::array::from_fn(|i| {
        f32::from_be_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("checked vector"))
    }))
}
fn layout_float(instance: &AttributeRuntimeInstance, offset: usize) -> EditSkaterResult<f32> {
    Ok(instance
        .try_cached_layout_field(offset, 4)?
        .map_or(0.0, |data| {
            f32::from_be_bytes(data.bytes().try_into().expect("four-byte field"))
        }))
}

/// Retained original base-animation library. The caller owns presentation dt;
/// gameplay's paused clock must not be passed here.
pub struct EditSkaterAnimation {
    database: skate2_formats::andale::AndaleDatabase,
    bytes: Vec<u8>,
    bind: Vec<skate2_formats::andale::AndaleSqt>,
    current: Option<usize>,
    mounts: MountSet,
    male: bool,
    character_name: String,
    gestures: [Option<GestureLibrary>; 5],
    active_gesture: Option<usize>,
    time: f32,
    previous: f32,
    #[cfg(feature="skate3-runtime")]
    skate3_preview:Option<(u32,crate::skate3_gesture_playback::Playback)>,
    #[cfg(feature="skate3-runtime")]
    skate3_preview_map:Vec<(usize,usize)>,
    #[cfg(feature="skate3-runtime")]
    skate3_preview_hands:Option<PreviewHands>,
}
impl EditSkaterAnimation {
    /// LoadBaseAnims8259be98 / CASAnimationLibrary.Load82596fa0.
    /// Init82597140 selects the bind pose by the hierarchy header's own name,
    /// then overlays the character face pose (CAC gender fallback, then rig).
    pub fn load(mounts: &MountSet, male: bool, character_name: &str) -> EditSkaterResult<Self> {
        use skate2_formats::andale::AndaleDatabase;
        let path = if male {
            "data/anim/cac_edit_male.abin"
        } else {
            "data/anim/cac_edit_female.abin"
        };
        let bytes = mounts.read_path(path)?;
        let database = AndaleDatabase::parse(&bytes)?;
        let bind = library_bind(mounts, &database, male, character_name)?;
        let mut retained_mounts = MountSet::new();
        for archive in mounts.archives() {
            retained_mounts.push(archive.clone());
        }
        Ok(Self {
            database,
            bytes,
            bind,
            current: None,
            mounts: retained_mounts,
            male,
            character_name: character_name.into(),
            gestures: std::array::from_fn(|_| None),
            active_gesture: None,
            time: 0.0,
            previous: 0.0,
            #[cfg(feature="skate3-runtime")]
            skate3_preview:None,
            #[cfg(feature="skate3-runtime")]
            skate3_preview_map:Vec::new(),
            #[cfg(feature="skate3-runtime")]
            skate3_preview_hands:None,
        })
    }

    pub fn set_skate3_gesture_preview(&mut self,id:Option<u32>)->EditSkaterResult<()> {
        #[cfg(feature="skate3-runtime")]
        {
            if self.skate3_preview.as_ref().map(|(id,_)|*id)!=id {
                self.skate3_preview=if let Some(id)=id {
                    if id>=37 {return Err("Invalid Skate 3 gesture preview".into());}
                    let runtime=crate::skate3_gesture_playback::Playback::new()?;
                    // The original CAS bank has148 bones (fingers/face/twists),
                    // whereas movement has36. Bind body bones by their authored
                    // names; keep the CAS-only channels and physical IK roots.
                    self.skate3_preview_map.clear();
                    for (source,name) in runtime.names().iter().enumerate() {
                        if name.to_ascii_uppercase().ends_with("_REPARENTED") {continue;}
                        // CAS holds the board under Hips; movement uses its
                        // own independent root. Keep that native CAS pose,
                        // together with its finger/face/twist writers.
                        let mut ancestor=source as i32;let mut board=false;
                        while ancestor>=0 {
                            if runtime.names()[ancestor as usize].eq_ignore_ascii_case("SKATEBOARD_ROOT") {board=true;break;}
                            ancestor=runtime.parents()[ancestor as usize];
                        }
                        if board {continue;}
                        let target=self.database.hierarchy.bone_names.iter().position(|target|target.eq_ignore_ascii_case(name))
                            .ok_or_else(||format!("CAS gesture body bone {name} missing"))?;
                        let parent=runtime.parents()[source];
                        let target_parent=self.database.hierarchy.parents[target];
                        if parent>=0 {
                            let parent=&runtime.names()[parent as usize];
                            let same=target_parent>=0 && self.database.hierarchy.bone_names[target_parent as usize].eq_ignore_ascii_case(parent);
                            // CAS roots Hips independently of Trajectory; its
                            // final publication already extracts that root.
                            if !same && !(target_parent==-1 && name.eq_ignore_ascii_case("Hips") && parent.eq_ignore_ascii_case("Trajectory")) {
                                return Err(format!("CAS gesture body bone {name} has a different parent").into());
                            }
                        }
                        self.skate3_preview_map.push((source,target));
                    }
                    if self.skate3_preview_hands.is_none() {
                        self.skate3_preview_hands=Some(PreviewHands::load(&self.mounts,&self.database.hierarchy)?);
                    }
                    Some((id,runtime))
                }else{None};
            }
        }
        #[cfg(not(feature="skate3-runtime"))]
        if id.is_some(){return Err("Skate 3 gestures require the embedded movement feature".into());}
        Ok(())
    }
    pub fn hierarchy(&self) -> crate::character::CharacterHierarchy {
        crate::character::CharacterHierarchy {
            bone_names: self.database.hierarchy.bone_names.clone(),
        }
    }

    /// CASCharacterEntity::Update8259f230 selects the gendered gesture library,
    /// recreates its animator at time zero, and evaluates its first frame with dt0.
    pub fn update(
        &mut self,
        dt: f32,
        body: &EditSkaterBody,
        attributes: &EditSkaterPresentationAttributes,
        palette: &mut crate::character::palette::CharacterPalette,
    ) -> EditSkaterResult<[f32; 16]> {
        use crate::{
            actor_animation_construction::initialization::body_dispatch::tree_commands::{
                numeric, sample,
            },
            actor_face_playback::tree_database::clip_fetch::timing::{
                BufferedFetchTiming, frame_pair,
            },
        };
        let previous_gesture = self.active_gesture;
        if body.gesture {
            let name = if self.male {
                body.animation.clone()
            } else {
                format!(
                    "cac_female_{}",
                    body.animation
                        .get(9..)
                        .ok_or("CAC gesture prefix too short")?
                )
            };
            let found = self.gestures.iter().position(|g| {
                g.as_ref()
                    .is_some_and(|g| g.name.eq_ignore_ascii_case(&name))
            });
            let slot = if let Some(i) = found {
                i
            } else {
                // ProcessRequestedGesture8259d280: first free slot, then first
                // loaded slot that is not the held gesture. Five native slots.
                let i = self
                    .gestures
                    .iter()
                    .position(Option::is_none)
                    .or_else(|| (0..5).find(|&i| Some(i) != self.active_gesture))
                    .ok_or("CAS gesture cache has no free slot")?;
                let bytes = self
                    .mounts
                    .read_path(&format!("data/anim/gesture/{name}.abin"))?;
                let database = skate2_formats::andale::AndaleDatabase::parse(&bytes)?;
                let bind = library_bind(&self.mounts, &database, self.male, &self.character_name)?;
                if database.hierarchy.bone_names != self.database.hierarchy.bone_names {
                    return Err("CAS gesture/base hierarchy differs".into());
                }
                self.gestures[i] = Some(GestureLibrary {
                    name,
                    database,
                    bytes,
                    bind,
                });
                i
            };
            self.active_gesture = Some(slot);
        } else {
            self.active_gesture = None;
        }
        let switched = self.active_gesture != previous_gesture;
        if switched {
            self.current = None;
            self.time = 0.0;
            self.previous = 0.0;
        }
        let (database, bytes, bind, animation) = if let Some(index) = self.active_gesture {
            let library = self.gestures[index]
                .as_ref()
                .ok_or("CAS held gesture missing")?;
            (
                &library.database,
                &library.bytes,
                &library.bind,
                library.name.as_str(),
            )
        } else {
            (
                &self.database,
                &self.bytes,
                &self.bind,
                body.animation.as_str(),
            )
        };
        let index = database
            .clips
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(animation))
            .ok_or_else(|| format!("CAS base animation {} is not a direct clip", body.animation))?;
        if self.current != Some(index) {
            // SetBodyState preserves mCurrentTime when switching base clips.
            self.current = Some(index);
            self.previous = self.time;
        }
        let clip = &database.clips[index];
        if dt < 0.0 {
            return Err("CAS negative animation dt".into());
        }
        self.time += if switched && body.gesture { 0.0 } else { dt };
        let speed = clip.base_speed;
        let length = (clip.frame_count - 1.0) / (speed * clip.frames_per_second);
        if !length.is_finite() || length <= 0.0 {
            return Err("CAS clip length is invalid".into());
        }
        let mut loops = 0u32;
        if clip.looping() {
            while self.time > length {
                let previous = self.time;
                self.time -= length;
                loops = loops.wrapping_add(1);
                if previous.to_bits() == self.time.to_bits() {
                    return Err("CAS animation loop made no progress".into());
                }
            }
        }
        let previous = (self.previous.min(length)) * speed;
        let current = (self.time.min(length)) * speed;
        // CAS Init82597370..82597464 creates a RAW ACS with truncationOffset
        // .5. ACSParams begins at stack+0x100: mode+52=0, fetchmode+56=1
        // (stw r21 at825973ec), category+60=0. Eval8258f1c8 requests
        // USEDEFAULT(2), which therefore resolves to FETCHBLEND(1).
        let timing = BufferedFetchTiming::encode(previous, current, loops, 2, 1)
            .map_err(|e| e.to_string())?;
        let (previous, current) = timing.decode();
        let pair = frame_pair(
            current,
            clip.frames_per_second,
            clip.frame_count,
            timing.blend_frames,
            0.5,
        )
        .map_err(|e| e.to_string())?;
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_CAS_FETCH").is_some()) {
            std::eprintln!("CAS fetch: clip={} time={current} blended={} pair={pair:?}",
                clip.name, timing.blend_frames);
        }
        let old_pair = frame_pair(
            previous,
            clip.frames_per_second,
            clip.frame_count,
            timing.blend_frames,
            0.5,
        )
        .map_err(|e| e.to_string())?;
        let channel = clip.flags & 0x2000_0000 != 0;
        let mut delta = vec![
            skate2_formats::andale::AndaleSqt {
                scale: [1.0; 4],
                rotation: [0.0, 0.0, 0.0, 1.0],
                translation: [0.0, 0.0, 0.0, if channel { 0.0 } else { 1.0 }],
            };
            bind.len()
        ];
        let record = database
            .records
            .iter()
            .find(|r| r.offset == clip.record_offset)
            .ok_or("CAS clip header missing")?;
        for (local_part, destination) in database.hierarchy.parts.iter().enumerate() {
            let Some(part) = clip
                .parts
                .iter()
                // This privately parsed library has not had UpdatePartNames
                // 82d6e728 register global IDs. Serialized name_index/high
                // offset bytes are zero; header bits1..5 hold the local part.
                // Source and destination belong to this same hierarchy.
                .find(|p| ((p.header >> 1) & 31) as usize == local_part)
            else {
                continue;
            };
            let mut values = sample(part, pair).map_err(|e| format!("CAS clip sample: {e:?}"))?;
            let start = destination.bone_offset as usize;
            let count = destination.bone_count as usize;
            if values.len() != count {
                return Err("CAS animation part size mismatch".into());
            }
            for (i, value) in values.iter_mut().enumerate() {
                value.translation[3] = if channel {
                    let offset = record.data_range.start
                        + (part.encoded_offset & 0x00ff_ffff) as usize
                        + 16
                        + i * 4;
                    f32::from_be_bytes(
                        bytes
                            .get(offset..offset + 4)
                            .ok_or("CAS channel weight outside clip")?
                            .try_into()?,
                    )
                } else {
                    1.0
                };
            }
            if start == 0 && !values.is_empty() {
                let old =
                    sample(part, old_pair).map_err(|e| format!("CAS trajectory sample: {e:?}"))?;
                let rotation = if timing.looped {
                    std::array::from_fn(|i| f32::from_bits(clip.fixed_words[4 + i]))
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                };
                let translation = if timing.looped {
                    std::array::from_fn(|i| f32::from_bits(clip.fixed_words[i]))
                } else {
                    [0.0; 3]
                };
                let weight = channel.then_some(values[0].translation[3]);
                numeric::trajectory(&mut values[0], old[0], rotation, translation, weight);
            }
            delta
                .get_mut(start..start + count)
                .ok_or("CAS part outside hierarchy")?
                .copy_from_slice(&values);
        }
        #[cfg(feature="skate3-runtime")]
        let root=if let Some((id,runtime))=&mut self.skate3_preview {
            use crate::actor_animation_construction::initialization::body_dispatch::tree_commands::numeric::add_sqt;
            runtime.prepare(Some(crate::input_pad::XboxPadSample {buttons:1,left_trigger:0,right_trigger:0,
                left_x:0,left_y:0,right_x:0,right_y:0}),[*id;4],crate::skate3_gesture_playback::Context {
                busy_hands:[0;2],filtered:6,hippy:false,holding_board:false,offboard:true,
                distance_to_cog:1.,blocked:false,skater_animation_flags:0},Default::default())?;
            runtime.advance(dt)?;
            let mut pose=delta.iter().zip(bind).map(|(a,b)|add_sqt(*a,*b,true)).collect::<Vec<_>>();
            for channel in runtime.evaluate(0.)? {
                let mut body=vec![skate2_formats::andale::AndaleSqt {scale:[1.;4],rotation:[0.,0.,0.,1.],translation:[0.;4]};runtime.names().len()];
                for &(source,target) in &self.skate3_preview_map {body[source]=pose[target];}
                channel.compose_over(&mut body)?;
                for &(source,target) in &self.skate3_preview_map {pose[target]=body[source];}
            }
            if let Some(hands)=&self.skate3_preview_hands {hands.apply(runtime,&mut pose)?;}
            attributes.publish_local_pose(&database.hierarchy,bind,pose,body,palette)?
        }else{attributes.publish_pose(&database.hierarchy,bind,&delta,body,palette)?};
        #[cfg(not(feature="skate3-runtime"))]
        let root=attributes.publish_pose(&database.hierarchy,bind,&delta,body,palette)?;
        self.previous = self.time;
        Ok(root)
    }
}

/// The fingers of a Skate 3 gesture preview: in game the hands' blend-pose streams pose them
/// through the hand command sets (`SetUpBlendPoseStream` 0x825f846c), which the room lacks.
#[cfg(feature="skate3-runtime")]
struct PreviewHands {
    /// Skate 2's 61 hand poses per hand (the fixed frames the game registers first).
    palettes: [Vec<Vec<skate2_formats::andale::AndaleSqt>>; 2],
    /// Each hand's `rig_TPose`, which the stream result is added to.
    tpose: [Vec<skate2_formats::andale::AndaleSqt>; 2],
    /// Hand bone to room bone, without the hand roots the arms own.
    map: [Vec<(usize, usize)>; 2],
}

#[cfg(feature="skate3-runtime")]
impl PreviewHands {
    fn load(mounts: &MountSet, hierarchy: &skate2_formats::andale::AndaleHierarchy) -> EditSkaterResult<Self> {
        use skate2_formats::andale::AndaleDatabase;
        let mut palettes = [Vec::new(), Vec::new()];
        let mut tpose = [Vec::new(), Vec::new()];
        let mut map = [Vec::new(), Vec::new()];
        for (hand, name) in ["SkaterLeftHand", "SkaterRightHand"].into_iter().enumerate() {
            let database = AndaleDatabase::parse(&mounts.read_path(&format!("data/anim/{name}.abin"))?)?;
            let clip = database.clips.iter().find(|c| c.name.eq_ignore_ascii_case("Poses")).ok_or("CAS hand Poses clip")?;
            if clip.frame_count != 61. {
                return Err("CAS hand palette must contain its authored 61 frames".into());
            }
            palettes[hand] = (0..61).map(|frame| clip.sample_raw_frame_sqt(&database.hierarchy, frame))
                .collect::<Result<_, _>>().map_err(|e| format!("CAS hand palette: {e:?}"))?;
            tpose[hand] = named_pose(&database, "rig_TPose")?.ok_or("CAS hand rig_TPose")?;
            for (source, bone) in database.hierarchy.bone_names.iter().enumerate() {
                if database.hierarchy.parents[source] < 0 {
                    continue;
                }
                if let Some(target) = hierarchy.bone_names.iter().position(|name| name.eq_ignore_ascii_case(bone)) {
                    map[hand].push((source, target));
                }
            }
        }
        Ok(Self { palettes, tpose, map })
    }

    /// `emit_tokens`' postfix program over the 123-pose palette (Skate 2's 61, then Skate 3's),
    /// added to the hand's `rig_TPose` as the hands' `add_bind_pose` stream does.
    fn apply(&self, runtime: &crate::skate3_gesture_playback::Playback, pose: &mut [skate2_formats::andale::AndaleSqt]) -> EditSkaterResult<()> {
        use crate::actor_animation_construction::initialization::body_dispatch::tree_commands::numeric::{add_sqt, blend};
        use crate::actor_face_playback::blend_pose_stream::{decode_coefficient, BlendPoseTokenPool};
        use skate2_formats::andale::AndaleSqt;
        let mut pool = BlendPoseTokenPool::new();
        let mut buffer = pool.buffer(0).map_err(|e| format!("CAS hand streams: {e:?}"))?;
        runtime.apply_hand_face_streams(&mut buffer, &mut pool)?;
        for (hand, name) in [b"LeftHand".as_slice(), b"RightHand"].into_iter().enumerate() {
            let key = crate::actor_graph_construction::ActorGraphIntents::native_name(name);
            let Some(stream) = buffer.stream_index(&key) else { continue };
            let tokens = pool.tokens(&buffer, stream).map_err(|e| format!("CAS hand tokens: {e:?}"))?;
            let skate3 = runtime.skate3_hand_poses(hand);
            let mut stack: Vec<Vec<AndaleSqt>> = Vec::new();
            let mut coefficients = Vec::new();
            for &token in tokens {
                match token >> 30 {
                    0 => {
                        let index = ((token >> 23) & 0x7f) as usize;
                        let frame = match index.checked_sub(self.palettes[hand].len()) {
                            None => &self.palettes[hand][index],
                            Some(index) => skate3.get(index).ok_or("CAS hand pose outside the palette")?,
                        };
                        stack.push(frame.clone());
                    }
                    1 => coefficients.push(decode_coefficient(token)),
                    _ => {
                        if coefficients.len() == 1 && stack.len() > 1 {
                            let second = stack.pop().ok_or("CAS hand blend")?;
                            let first = stack.pop().ok_or("CAS hand blend")?;
                            stack.push(first.iter().zip(&second).map(|(a, b)| blend(*a, *b, coefficients[0])).collect());
                        } else if coefficients.len() > 1 && stack.len() >= coefficients.len() {
                            // NBlend: the weighted sum, then each rotation normalized.
                            let inputs = stack.split_off(stack.len() - coefficients.len());
                            let mut sum = inputs[0].iter().map(|sqt| AndaleSqt {
                                scale: sqt.scale.map(|v| coefficients[0] * v),
                                rotation: sqt.rotation.map(|v| coefficients[0] * v),
                                translation: sqt.translation.map(|v| coefficients[0] * v),
                            }).collect::<Vec<_>>();
                            for (input, &coefficient) in inputs.iter().zip(&coefficients).skip(1) {
                                for (total, sqt) in sum.iter_mut().zip(input) {
                                    for lane in 0..4 {
                                        total.scale[lane] = coefficient.mul_add(sqt.scale[lane], total.scale[lane]);
                                        total.rotation[lane] = coefficient.mul_add(sqt.rotation[lane], total.rotation[lane]);
                                        total.translation[lane] = coefficient.mul_add(sqt.translation[lane], total.translation[lane]);
                                    }
                                }
                            }
                            for total in &mut sum {
                                let length = total.rotation.iter().map(|v| v * v).sum::<f32>().sqrt();
                                total.rotation = total.rotation.map(|v| v / length);
                            }
                            stack.push(sum);
                        } else {
                            return Err("CAS hand token program".into());
                        }
                        coefficients.clear();
                    }
                }
            }
            let Some(delta) = stack.pop() else { continue };
            for &(source, target) in &self.map[hand] {
                pose[target] = add_sqt(delta[source], self.tpose[hand][source], true);
            }
        }
        Ok(())
    }
}

struct GestureLibrary {
    name: String,
    database: skate2_formats::andale::AndaleDatabase,
    bytes: Vec<u8>,
    bind: Vec<skate2_formats::andale::AndaleSqt>,
}
fn library_bind(
    mounts: &MountSet,
    database: &skate2_formats::andale::AndaleDatabase,
    male: bool,
    character_name: &str,
) -> EditSkaterResult<Vec<skate2_formats::andale::AndaleSqt>> {
    use crate::animation::{AnimationSkeleton, LocalPose};
    use skate2_formats::andale::AndaleDatabase;
    let hierarchy_name = database
        .records
        .iter()
        .find(|r| r.header.data_type == 4)
        .ok_or("CAS animation hierarchy header missing")?
        .header
        .name
        .decode()?;
    let bind = named_pose(&database, &hierarchy_name)?.ok_or("CAS named bind pose missing")?;
    let face = AndaleDatabase::parse(&mounts.read_path("data/anim/SkaterFacePres.abin")?)?;
    let gender = if male { "CAC_male" } else { "CAC_female" };
    let face_pose = match named_pose(&face, character_name)? {
        Some(pose) => pose,
        None => match named_pose(&face, gender)? {
            Some(pose) => pose,
            None => named_pose(&face, "rig_Tpose")?.ok_or("CAS face fallback pose missing")?,
        },
    };
    let mut merged = LocalPose::from_andale(&bind);
    merged.overlay_matching_parts(
        &AnimationSkeleton::from_andale(&database.hierarchy)?,
        &AnimationSkeleton::from_andale(&face.hierarchy)?,
        &LocalPose::from_andale(&face_pose),
    )?;
    let bind = merged
        .bones
        .into_iter()
        .map(|v| skate2_formats::andale::AndaleSqt {
            scale: v.scale,
            rotation: v.rotation,
            translation: v.translation,
        })
        .collect();
    Ok(bind)
}

pub(crate) fn named_pose(
    database: &skate2_formats::andale::AndaleDatabase,
    name: &str,
) -> EditSkaterResult<Option<Vec<skate2_formats::andale::AndaleSqt>>> {
    let record = database.records.iter().find(|r| {
        r.header.data_type == 3
            && r.header
                .name
                .decode()
                .is_ok_and(|n| n.eq_ignore_ascii_case(name))
    });
    let Some(record) = record else {
        return Ok(None);
    };
    let pose = database
        .poses
        .iter()
        .find(|p| p.record_offset == record.offset)
        .ok_or("CAS pose record has no payload")?;
    if record.header.codec == 0x0000_4452 {
        return Ok(Some(pose.sample_raw_sqt(&database.hierarchy)?));
    }
    let mut output = Vec::new();
    for part in &pose.parts {
        output.extend(
            part.constant_vbr_sqt()
                .ok_or("CAS bind pose compression unsupported")?,
        );
    }
    if output.len() != database.hierarchy.bone_names.len() {
        return Err("CAS pose hierarchy mismatch".into());
    }
    Ok(Some(output))
}
