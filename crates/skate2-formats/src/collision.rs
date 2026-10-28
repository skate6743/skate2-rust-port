//! Serialized `RenderWare` collision headers found in RW4 arena dictionaries.

use std::{collections::BTreeMap, fmt, ops::ControlFlow, sync::{Arc, Mutex}};

use crate::pegasus::ObjectId;

#[path = "collision_line_cursor.rs"]
pub mod line_cursor;

const COLLISION_HEADER_SIZE: usize = 96;
const ASSEMBLY_DEFINITION_SIZE: usize = 48;
const ASSEMBLY_SIZE: usize = 48;
const BIT_TABLE_HEADER_SIZE: usize = 12;
const PART_DEFINITION_SIZE: usize = 160;
const KD_LINE_STACK_CAPACITY: usize = 32;

pub const ASSEMBLY_TYPE_ID: u32 = 0x0004_0006;
pub const ASSEMBLY_DEFINITION_TYPE_ID: u32 = 0x0004_0007;
/// Arena type registered for `rw::BitTable` at `0x82bc3370..0x82bc33a4`.
pub const BIT_TABLE_TYPE_ID: u32 = 0x0001_000f;

/// Serialized `rw::BitTable` header and its flexible packed-word allocation.
///
/// The PDB fixes rows, columns, and stored word count at `+0x00`, `+0x04`,
/// and `+0x08`; packed row-major words start at `+0x0c`. This type retains the
/// stored word count without requiring it to agree with the dimensions because
/// native `SerialisedAssembly::Create` recomputes its transfer size from rows
/// and columns and never reads `m_arraySize`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitTable {
    pub rows: u32,
    pub columns: u32,
    pub array_size: u32,
    pub words: Vec<u32>,
}

impl BitTable {
    /// Parses the fixed three-word header and exactly `m_arraySize` packed
    /// words.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the header, word-count conversion,
    /// size arithmetic, or packed allocation is invalid.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, BIT_TABLE_HEADER_SIZE)?;
        let rows = be_u32(bytes, 0)?;
        let columns = be_u32(bytes, 4)?;
        let array_size = be_u32(bytes, 8)?;
        let word_count =
            usize::try_from(array_size).map_err(|_| CollisionError::IntegerOverflow)?;
        let words_size = word_count
            .checked_mul(4)
            .ok_or(CollisionError::IntegerOverflow)?;
        let required = BIT_TABLE_HEADER_SIZE
            .checked_add(words_size)
            .ok_or(CollisionError::IntegerOverflow)?;
        require_size(bytes, required)?;
        let words = (0..word_count)
            .map(|index| be_u32(bytes, BIT_TABLE_HEADER_SIZE + index * 4))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            rows,
            columns,
            array_size,
            words,
        })
    }
}

/// Serialized `rw::physics::Assembly`. Arena callback registration at
/// `0x82ae1908` binds type `0x00040006` to `Assembly::Fixup`; its serializer at
/// `0x82ae17a0` proves the definition is an arena object ID at byte 4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assembly {
    pub culling_table: Option<ObjectId>,
    pub definition: ObjectId,
    pub part_list_offset: u32,
    pub joint_list_offset: u32,
    pub drive_list_offset: u32,
    pub world_index: u32,
    pub group_id: u32,
    pub part_count: u32,
    pub joint_count: u32,
    pub drive_count: u32,
    pub user_data: u32,
    pub flags: u16,
    pub user_flags: u16,
}

impl Assembly {
    /// Parses the exact 48-byte runtime-assembly header.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the header is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, ASSEMBLY_SIZE)?;
        let culling_table = be_u32(bytes, 0)?;
        Ok(Self {
            culling_table: (culling_table != 0).then_some(ObjectId(culling_table)),
            definition: ObjectId(be_u32(bytes, 4)?),
            part_list_offset: be_u32(bytes, 8)?,
            joint_list_offset: be_u32(bytes, 12)?,
            drive_list_offset: be_u32(bytes, 16)?,
            world_index: be_u32(bytes, 20)?,
            group_id: be_u32(bytes, 24)?,
            part_count: be_u32(bytes, 28)?,
            joint_count: be_u32(bytes, 32)?,
            drive_count: be_u32(bytes, 36)?,
            user_data: be_u32(bytes, 40)?,
            flags: be_u16(bytes, 44)?,
            user_flags: be_u16(bytes, 46)?,
        })
    }
}

/// Values from the reference `rw::collision::ObjectType` debug enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ObjectType {
    Volume = 0x0008_0001,
    SimpleMappedArray = 0x0008_0002,
    TriangleKdTreeProcedural = 0x0008_0003,
    KdTreeMappedArray = 0x0008_0004,
    BoundingBox = 0x0008_0005,
    ClusteredMesh = 0x0008_0006,
}

/// Serialized `rw::physics::AssemblyDefinition`. The full PDB fixes the
/// 48-byte header and 160-byte part stride; the serializer at `0x82ae1650`
/// proves that `parts_offset` is relative to this object's base.
#[derive(Clone, Debug, PartialEq)]
pub struct AssemblyDefinition {
    pub parts_offset: u32,
    pub part_count: u32,
    pub joint_count: u32,
    pub joint_limits_offset: u32,
    pub joint_frames_offset: u32,
    pub joint_parts_offset: u32,
    pub drive_count: u32,
    pub drive_dynamics_offset: u32,
    pub drive_frames_offset: u32,
    pub drive_parts_offset: u32,
    pub culling_table: Option<ObjectId>,
    pub hashed_part_names_offset: u32,
    pub parts: Vec<AssemblyPartDefinition>,
}

/// The collision-facing fields of one exact 160-byte `PartDefinition`.
#[derive(Clone, Debug, PartialEq)]
pub struct AssemblyPartDefinition {
    pub volume: ObjectId,
    pub inertia: AssemblyPartInertia,
    pub inverse_body_transform: [f32; 16],
    pub material: AssemblyPartMaterial,
    pub has_inverse_body_transform: u32,
    pub is_static_body: u32,
}

/// Serialized values of `rw::physics::Inertia` embedded at
/// `PartDefinition +0x10`.
///
/// The PDB fixes the native member offsets and the 0x30-byte containing type.
/// The inverse-tensor field is retained as all four serialized vector lanes;
/// no meaning is assigned to its fourth lane here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssemblyPartInertia {
    pub inverse_tensor: [f32; 4],
    pub inverse_mass: f32,
    pub spherical: f32,
    pub max_velocity: f32,
    pub max_omega: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
}

/// Serialized `rw::physics::Material` embedded at `PartDefinition +0x80`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssemblyPartMaterial {
    pub static_friction: f32,
    pub dynamic_friction: f32,
    pub restitution: f32,
}

impl AssemblyDefinition {
    /// Parses the fixed header and inline part-definition array.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for truncated headers or part allocations.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        Self::parse_in_resource(bytes, 0)
    }

    /// Parses an assembly at `object_offset` in its complete RW4 resource.
    ///
    /// Assembly allocations are dictionary objects with a 48-byte declared
    /// size, while their part arrays are separately allocated in the same
    /// resource. The reference fixup sets the archive base to the assembly
    /// object and adds each serialized 32-bit relative pointer with native
    /// wraparound, so the target may lie before or after the header.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the header or resolved part allocation
    /// falls outside the resource.
    pub fn parse_in_resource(resource: &[u8], object_offset: u32) -> Result<Self, CollisionError> {
        let object_start =
            usize::try_from(object_offset).map_err(|_| CollisionError::IntegerOverflow)?;
        let object_end = object_start
            .checked_add(ASSEMBLY_DEFINITION_SIZE)
            .ok_or(CollisionError::IntegerOverflow)?;
        let bytes =
            resource
                .get(object_start..object_end)
                .ok_or(CollisionError::RangeOutsideObject {
                    start: object_start,
                    end: object_end,
                    object_size: resource.len(),
                })?;
        let parts_offset = be_u32(bytes, 0)?;
        let part_count = be_u32(bytes, 4)?;
        let start = usize::try_from(object_offset.wrapping_add(parts_offset))
            .map_err(|_| CollisionError::IntegerOverflow)?;
        let count = usize::try_from(part_count).map_err(|_| CollisionError::IntegerOverflow)?;
        let size = count
            .checked_mul(PART_DEFINITION_SIZE)
            .ok_or(CollisionError::IntegerOverflow)?;
        let end = start
            .checked_add(size)
            .ok_or(CollisionError::IntegerOverflow)?;
        let part_bytes = resource
            .get(start..end)
            .ok_or(CollisionError::RangeOutsideObject {
                start,
                end,
                object_size: resource.len(),
            })?;
        let parts = (0..count)
            .map(|index| {
                let part = &part_bytes[index * PART_DEFINITION_SIZE..];
                Ok(AssemblyPartDefinition {
                    volume: ObjectId(be_u32(part, 0)?),
                    inertia: AssemblyPartInertia {
                        inverse_tensor: be_f32_array::<4>(part, 16)?,
                        inverse_mass: be_f32(part, 32)?,
                        spherical: be_f32(part, 36)?,
                        max_velocity: be_f32(part, 40)?,
                        max_omega: be_f32(part, 44)?,
                        linear_drag: be_f32(part, 48)?,
                        angular_drag: be_f32(part, 52)?,
                    },
                    inverse_body_transform: be_f32_array::<16>(part, 64)?,
                    material: AssemblyPartMaterial {
                        static_friction: be_f32(part, 128)?,
                        dynamic_friction: be_f32(part, 132)?,
                        restitution: be_f32(part, 136)?,
                    },
                    has_inverse_body_transform: be_u32(part, 140)?,
                    is_static_body: be_u32(part, 144)?,
                })
            })
            .collect::<Result<_, CollisionError>>()?;
        let culling_table_raw = be_u32(bytes, 40)?;
        Ok(Self {
            parts_offset,
            part_count,
            joint_count: be_u32(bytes, 8)?,
            joint_limits_offset: be_u32(bytes, 12)?,
            joint_frames_offset: be_u32(bytes, 16)?,
            joint_parts_offset: be_u32(bytes, 20)?,
            drive_count: be_u32(bytes, 24)?,
            drive_dynamics_offset: be_u32(bytes, 28)?,
            drive_frames_offset: be_u32(bytes, 32)?,
            drive_parts_offset: be_u32(bytes, 36)?,
            culling_table: (culling_table_raw != 0).then_some(ObjectId(culling_table_raw)),
            hashed_part_names_offset: be_u32(bytes, 44)?,
            parts,
        })
    }
}

/// Values from the reference `rw::collision::VolumeType` debug enum. A
/// serialized volume stores this numeric ID in its vtable field; fixup turns
/// it into a native vtable pointer through `Volume::vTableArray`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VolumeType {
    Null = 0,
    Sphere = 1,
    Capsule = 2,
    Triangle = 3,
    Box = 4,
    Cylinder = 5,
    Aggregate = 6,
}

impl VolumeType {
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::Null,
            1 => Self::Sphere,
            2 => Self::Capsule,
            3 => Self::Triangle,
            4 => Self::Box,
            5 => Self::Cylinder,
            6 => Self::Aggregate,
            _ => return None,
        })
    }
}

impl ObjectType {
    #[must_use]
    pub const fn from_type_id(type_id: u32) -> Option<Self> {
        match type_id {
            0x0008_0001 => Some(Self::Volume),
            0x0008_0002 => Some(Self::SimpleMappedArray),
            0x0008_0003 => Some(Self::TriangleKdTreeProcedural),
            0x0008_0004 => Some(Self::KdTreeMappedArray),
            0x0008_0005 => Some(Self::BoundingBox),
            0x0008_0006 => Some(Self::ClusteredMesh),
            _ => None,
        }
    }
}

/// Unfixed, big-endian `rw::collision::Volume` data.
#[derive(Clone, Debug, PartialEq)]
pub struct Volume {
    pub transform: [f32; 16],
    pub vtable_offset: u32,
    /// The 12-byte anonymous union retained as words until its active member
    /// is proven by the volume vtable/type.
    pub shape_words: [u32; 3],
    pub radius: f32,
    pub group_id: u32,
    pub surface_id: u32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveHit {
    pub line_parameter: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Volume {
    /// Parses the 96-byte `rw::collision::Volume` layout from debug symbols.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError::Truncated`] when the fixed header is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_header(bytes)?;
        Ok(Self {
            transform: be_f32_array::<16>(bytes, 0)?,
            vtable_offset: be_u32(bytes, 64)?,
            shape_words: [be_u32(bytes, 68)?, be_u32(bytes, 72)?, be_u32(bytes, 76)?],
            radius: be_f32(bytes, 80)?,
            group_id: be_u32(bytes, 84)?,
            surface_id: be_u32(bytes, 88)?,
            flags: be_u32(bytes, 92)?,
        })
    }

    #[must_use]
    pub const fn volume_type(&self) -> Option<VolumeType> {
        VolumeType::from_raw(self.vtable_offset)
    }

    /// Returns the arena object ID stored in an aggregate volume. The
    /// reference serializer resolves this word during volume fixup.
    #[must_use]
    pub const fn aggregate_object_id(&self) -> Option<u32> {
        if matches!(self.volume_type(), Some(VolumeType::Aggregate)) && self.shape_words[0] != 0 {
            Some(self.shape_words[0])
        } else {
            None
        }
    }

    /// Intersects a thin segment with serialized sphere, box, or cylinder
    /// volumes. `delta` is the endpoint minus the start point, matching the
    /// native mesh query API.
    #[must_use]
    pub fn thin_segment_intersection(
        &self,
        start: [f32; 3],
        delta: [f32; 3],
    ) -> Option<PrimitiveHit> {
        self.segment_intersection_with_fatness(start, delta, 0.0)
    }

    /// Intersects a segment with a primitive after expanding it by the line
    /// radius used by `RenderWare`'s fat-line query.
    ///
    /// Sphere, rounded-box, and rounded-cylinder volumes preserve their
    /// separate face/side, edge/rim, and initial-overlap regions.
    #[must_use]
    pub fn segment_intersection_with_fatness(
        &self,
        start: [f32; 3],
        delta: [f32; 3],
        fatness: f32,
    ) -> Option<PrimitiveHit> {
        if fatness < 0.0 {
            return None;
        }
        let frame = VolumeFrame::from_transform(self.transform);
        match self.volume_type()? {
            VolumeType::Sphere => sphere_segment(start, delta, frame.center, self.radius + fatness),
            VolumeType::Box => rounded_oriented_box_segment(
                start,
                delta,
                frame,
                self.shape_words.map(f32::from_bits),
                self.radius + fatness,
            ),
            VolumeType::Cylinder => rounded_cylinder_segment(
                start,
                delta,
                frame,
                f32::from_bits(self.shape_words[0]),
                f32::from_bits(self.shape_words[1]),
                self.radius + fatness,
            ),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct VolumeFrame {
    axes: [[f32; 3]; 3],
    center: [f32; 3],
}

impl VolumeFrame {
    fn from_transform(transform: [f32; 16]) -> Self {
        Self {
            axes: [
                normalize([transform[0], transform[1], transform[2]]),
                normalize([transform[4], transform[5], transform[6]]),
                normalize([transform[8], transform[9], transform[10]]),
            ],
            center: [transform[12], transform[13], transform[14]],
        }
    }

    fn local_point(self, point: [f32; 3]) -> [f32; 3] {
        let relative = subtract(point, self.center);
        self.axes.map(|axis| dot(relative, axis))
    }

    fn local_vector(self, vector: [f32; 3]) -> [f32; 3] {
        self.axes.map(|axis| dot(vector, axis))
    }

    fn world_normal(self, normal: [f32; 3]) -> [f32; 3] {
        normalize(std::array::from_fn(|component| {
            (0..3)
                .map(|axis| self.axes[axis][component] * normal[axis])
                .sum()
        }))
    }
}

fn sphere_segment(
    start: [f32; 3],
    delta: [f32; 3],
    center: [f32; 3],
    radius: f32,
) -> Option<PrimitiveHit> {
    let relative = subtract(start, center);
    let a = dot(delta, delta);
    let c = dot(relative, relative) - radius * radius;
    let parameter = if c <= 0.0 {
        0.0
    } else {
        let b = dot(relative, delta);
        let discriminant = b * b - a * c;
        if a <= f32::EPSILON || b >= 0.0 || discriminant < 0.0 {
            return None;
        }
        (-b - discriminant.sqrt()) / a
    };
    primitive_hit(
        start,
        delta,
        parameter,
        subtract(add(start, scale(delta, parameter)), center),
    )
}

fn oriented_box_segment(
    start: [f32; 3],
    delta: [f32; 3],
    frame: VolumeFrame,
    half_extents: [f32; 3],
) -> Option<PrimitiveHit> {
    let origin = frame.local_point(start);
    let direction = frame.local_vector(delta);
    let mut near = 0.0_f32;
    let mut far = 1.0_f32;
    let mut normal = [0.0; 3];
    for axis in 0..3 {
        if direction[axis].abs() <= f32::EPSILON {
            if origin[axis].abs() > half_extents[axis] {
                return None;
            }
            continue;
        }
        let mut first = (-half_extents[axis] - origin[axis]) / direction[axis];
        let mut second = (half_extents[axis] - origin[axis]) / direction[axis];
        let mut sign = -1.0;
        if first > second {
            std::mem::swap(&mut first, &mut second);
            sign = 1.0;
        }
        if first > near {
            near = first;
            normal = [0.0; 3];
            normal[axis] = sign;
        }
        far = far.min(second);
        if near > far {
            return None;
        }
    }
    if dot(normal, normal) <= f32::EPSILON {
        normal = normalize(scale(direction, -1.0));
    }
    primitive_hit(start, delta, near, frame.world_normal(normal))
}

fn rounded_oriented_box_segment(
    start: [f32; 3],
    delta: [f32; 3],
    frame: VolumeFrame,
    half_extents: [f32; 3],
    radius: f32,
) -> Option<PrimitiveHit> {
    if radius == 0.0 {
        return oriented_box_segment(start, delta, frame, half_extents);
    }
    if radius < 0.0 {
        return None;
    }

    let origin = frame.local_point(start);
    let direction = frame.local_vector(delta);
    let nearest_at_start =
        std::array::from_fn(|axis| origin[axis].clamp(-half_extents[axis], half_extents[axis]));
    let start_offset = subtract(origin, nearest_at_start);
    if dot(start_offset, start_offset) < radius * radius {
        let local_normal = if dot(start_offset, start_offset) > 0.0 {
            normalize(start_offset)
        } else {
            normalize(scale(direction, -1.0))
        };
        return primitive_hit(start, delta, 0.0, frame.world_normal(local_normal));
    }

    let mut nearest_parameter = None;

    // Six planar face regions of the rounded box.
    for axis in 0..3 {
        if direction[axis] == 0.0 {
            continue;
        }
        let other_axes = [(axis + 1) % 3, (axis + 2) % 3];
        for sign in [-1.0_f32, 1.0] {
            let plane = sign * (half_extents[axis] + radius);
            let parameter = (plane - origin[axis]) / direction[axis];
            if !(0.0..=1.0).contains(&parameter) {
                continue;
            }
            let point = add(origin, scale(direction, parameter));
            if other_axes
                .iter()
                .all(|other| point[*other].abs() <= half_extents[*other])
            {
                retain_nearest_parameter(&mut nearest_parameter, parameter);
            }
        }
    }

    // Twelve cylindrical edge regions.
    for edge_axis in 0..3 {
        let fixed_axes = [(edge_axis + 1) % 3, (edge_axis + 2) % 3];
        for first_sign in [-1.0_f32, 1.0] {
            for second_sign in [-1.0_f32, 1.0] {
                let mut edge_start = [0.0; 3];
                let mut edge_end = [0.0; 3];
                edge_start[edge_axis] = -half_extents[edge_axis];
                edge_end[edge_axis] = half_extents[edge_axis];
                edge_start[fixed_axes[0]] = first_sign * half_extents[fixed_axes[0]];
                edge_end[fixed_axes[0]] = edge_start[fixed_axes[0]];
                edge_start[fixed_axes[1]] = second_sign * half_extents[fixed_axes[1]];
                edge_end[fixed_axes[1]] = edge_start[fixed_axes[1]];
                if let Some(parameter) =
                    ray_capsule_body_parameter(origin, direction, edge_start, edge_end, radius)
                {
                    retain_nearest_parameter(&mut nearest_parameter, parameter);
                }
            }
        }
    }

    // Eight spherical corner regions.
    for x_sign in [-1.0_f32, 1.0] {
        for y_sign in [-1.0_f32, 1.0] {
            for z_sign in [-1.0_f32, 1.0] {
                let corner = [
                    x_sign * half_extents[0],
                    y_sign * half_extents[1],
                    z_sign * half_extents[2],
                ];
                if let Some(parameter) = ray_sphere_parameter(origin, direction, corner, radius) {
                    retain_nearest_parameter(&mut nearest_parameter, parameter);
                }
            }
        }
    }

    let parameter = nearest_parameter?;
    let local_hit = add(origin, scale(direction, parameter));
    let nearest_on_box =
        std::array::from_fn(|axis| local_hit[axis].clamp(-half_extents[axis], half_extents[axis]));
    let local_normal = normalize(subtract(local_hit, nearest_on_box));
    primitive_hit(start, delta, parameter, frame.world_normal(local_normal))
}

fn cylinder_segment(
    start: [f32; 3],
    delta: [f32; 3],
    frame: VolumeFrame,
    half_height: f32,
    radius: f32,
) -> Option<PrimitiveHit> {
    let origin = frame.local_point(start);
    let direction = frame.local_vector(delta);
    let mut candidates = Vec::with_capacity(4);
    let radial_direction_squared = direction[0] * direction[0] + direction[1] * direction[1];
    let radial_projection = origin[0] * direction[0] + origin[1] * direction[1];
    let radial_distance_squared = origin[0] * origin[0] + origin[1] * origin[1] - radius * radius;
    let discriminant =
        radial_projection * radial_projection - radial_direction_squared * radial_distance_squared;
    if radial_direction_squared > f32::EPSILON && discriminant >= 0.0 {
        for parameter in [
            (-radial_projection - discriminant.sqrt()) / radial_direction_squared,
            (-radial_projection + discriminant.sqrt()) / radial_direction_squared,
        ] {
            let height = origin[2] + parameter * direction[2];
            if (0.0..=1.0).contains(&parameter) && height.abs() <= half_height {
                candidates.push((
                    parameter,
                    [
                        origin[0] + parameter * direction[0],
                        origin[1] + parameter * direction[1],
                        0.0,
                    ],
                ));
            }
        }
    }
    if direction[2].abs() > f32::EPSILON {
        for cap in [-half_height, half_height] {
            let parameter = (cap - origin[2]) / direction[2];
            let cap_x = origin[0] + parameter * direction[0];
            let cap_y = origin[1] + parameter * direction[1];
            if (0.0..=1.0).contains(&parameter) && cap_x * cap_x + cap_y * cap_y <= radius * radius
            {
                candidates.push((parameter, [0.0, 0.0, cap.signum()]));
            }
        }
    }
    let (parameter, normal) = candidates
        .into_iter()
        .min_by(|left, right| left.0.total_cmp(&right.0))?;
    primitive_hit(start, delta, parameter, frame.world_normal(normal))
}

fn rounded_cylinder_segment(
    start: [f32; 3],
    delta: [f32; 3],
    frame: VolumeFrame,
    half_height: f32,
    cylinder_radius: f32,
    rounding_radius: f32,
) -> Option<PrimitiveHit> {
    if rounding_radius == 0.0 {
        return cylinder_segment(start, delta, frame, half_height, cylinder_radius);
    }
    if half_height < 0.0 || cylinder_radius < 0.0 || rounding_radius < 0.0 {
        return None;
    }

    let origin = frame.local_point(start);
    let direction = frame.local_vector(delta);
    let nearest_at_start = nearest_point_on_cylinder(origin, half_height, cylinder_radius);
    let start_offset = subtract(origin, nearest_at_start);
    if dot(start_offset, start_offset) < rounding_radius * rounding_radius {
        let local_normal = if dot(start_offset, start_offset) > 0.0 {
            normalize(start_offset)
        } else {
            normalize(scale(direction, -1.0))
        };
        return primitive_hit(start, delta, 0.0, frame.world_normal(local_normal));
    }

    let mut nearest_parameter = None;
    let expanded_radius = cylinder_radius + rounding_radius;
    let radial_direction_squared = direction[0] * direction[0] + direction[1] * direction[1];
    let radial_projection = origin[0] * direction[0] + origin[1] * direction[1];
    let radial_distance_squared =
        origin[0] * origin[0] + origin[1] * origin[1] - expanded_radius * expanded_radius;
    let discriminant =
        radial_projection * radial_projection - radial_direction_squared * radial_distance_squared;
    if radial_direction_squared > 0.0 && discriminant >= 0.0 {
        for parameter in [
            (-radial_projection - discriminant.sqrt()) / radial_direction_squared,
            (-radial_projection + discriminant.sqrt()) / radial_direction_squared,
        ] {
            let height = origin[2] + parameter * direction[2];
            if (0.0..=1.0).contains(&parameter) && height.abs() <= half_height {
                retain_nearest_parameter(&mut nearest_parameter, parameter);
            }
        }
    }

    if direction[2] != 0.0 {
        for sign in [-1.0_f32, 1.0] {
            let cap_height = sign * (half_height + rounding_radius);
            let parameter = (cap_height - origin[2]) / direction[2];
            let x = origin[0] + parameter * direction[0];
            let y = origin[1] + parameter * direction[1];
            if (0.0..=1.0).contains(&parameter)
                && x * x + y * y <= cylinder_radius * cylinder_radius
            {
                retain_nearest_parameter(&mut nearest_parameter, parameter);
            }
        }
    }

    for sign in [-1.0_f32, 1.0] {
        if let Some(parameter) = rounded_cylinder_rim_parameter(
            origin,
            direction,
            sign * half_height,
            cylinder_radius,
            rounding_radius,
        ) {
            retain_nearest_parameter(&mut nearest_parameter, parameter);
        }
    }

    let parameter = nearest_parameter?;
    let local_hit = add(origin, scale(direction, parameter));
    let nearest = nearest_point_on_cylinder(local_hit, half_height, cylinder_radius);
    let local_normal = normalize(subtract(local_hit, nearest));
    primitive_hit(start, delta, parameter, frame.world_normal(local_normal))
}

fn nearest_point_on_cylinder(point: [f32; 3], half_height: f32, radius: f32) -> [f32; 3] {
    let radial_length = point[0].hypot(point[1]);
    let radial_scale = if radial_length > radius && radial_length > 0.0 {
        radius / radial_length
    } else {
        1.0
    };
    [
        point[0] * radial_scale,
        point[1] * radial_scale,
        point[2].clamp(-half_height, half_height),
    ]
}

fn rounded_cylinder_rim_parameter(
    origin: [f32; 3],
    direction: [f32; 3],
    rim_height: f32,
    major_radius: f32,
    minor_radius: f32,
) -> Option<f32> {
    let torus_origin = [origin[0], origin[1], origin[2] - rim_height];
    let extent = major_radius + minor_radius;
    let minimum = [-extent, -extent, -minor_radius];
    let maximum = [extent, extent, minor_radius];
    let (entry, exit) = clip_segment_to_aabb(torus_origin, direction, minimum, maximum)?;
    let span = exit - entry;
    if span < 0.0 {
        return None;
    }
    let clipped_origin = add(torus_origin, scale(direction, entry));
    let clipped_direction = scale(direction, span);
    let local_parameter = torus_segment_parameter(
        clipped_origin,
        clipped_direction,
        major_radius,
        minor_radius,
    )?;
    if !(0.0..=1.0).contains(&local_parameter) {
        return None;
    }
    let parameter = local_parameter.mul_add(span, entry);
    let point = add(torus_origin, scale(direction, parameter));
    let radial = point[0].hypot(point[1]);
    let correct_half = if rim_height >= 0.0 {
        point[2] >= 0.0
    } else {
        point[2] <= 0.0
    };
    (correct_half && radial >= major_radius).then_some(parameter)
}

fn clip_segment_to_aabb(
    origin: [f32; 3],
    direction: [f32; 3],
    minimum: [f32; 3],
    maximum: [f32; 3],
) -> Option<(f32, f32)> {
    let mut entry = 0.0_f32;
    let mut exit = 1.0_f32;
    for axis in 0..3 {
        if direction[axis] == 0.0 {
            if origin[axis] < minimum[axis] || origin[axis] > maximum[axis] {
                return None;
            }
            continue;
        }
        let mut first = (minimum[axis] - origin[axis]) / direction[axis];
        let mut second = (maximum[axis] - origin[axis]) / direction[axis];
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }
        entry = entry.max(first);
        exit = exit.min(second);
        if entry > exit {
            return None;
        }
    }
    Some((entry, exit))
}

/// Reference: `rw::collision::rwcTorusLineSegIntersect` at `0x82ad8ac8`.
fn torus_segment_parameter(
    origin: [f32; 3],
    direction: [f32; 3],
    major_radius: f32,
    minor_radius: f32,
) -> Option<f32> {
    let direction_squared = dot(direction, direction);
    let origin_direction = dot(origin, direction);
    let origin_squared = dot(origin, origin);
    let major_squared = major_radius * major_radius;
    let minor_squared = minor_radius * minor_radius;
    let shifted_origin_squared = origin_squared - major_squared - minor_squared;
    let coefficients = [
        shifted_origin_squared * shifted_origin_squared
            - 4.0 * major_squared * (minor_squared - origin[2] * origin[2]),
        4.0 * origin_direction * shifted_origin_squared
            + 8.0 * major_squared * origin[2] * direction[2],
        4.0 * origin_direction * origin_direction
            + 2.0 * direction_squared * shifted_origin_squared
            + 4.0 * major_squared * direction[2] * direction[2],
        4.0 * direction_squared * origin_direction,
        direction_squared * direction_squared,
    ];
    solve_quartic_root_reference(coefficients)
}

/// Domain form of `rw::collision::SolveQuarticRoots` at `0x82acd450`.
/// The reference searches one root from zero with a decreasing Newton step.
#[allow(clippy::cast_precision_loss)] // The reference converts its 0..50 counter to float.
fn solve_quartic_root_reference(coefficients: [f32; 5]) -> Option<f32> {
    const CONVERGENCE_RESIDUAL: f32 = 1.0e-6;
    const ACCEPTANCE_RESIDUAL: f32 = 1.0e-3;

    let mut root = 0.0_f32;
    let mut residual = f32::MAX;
    let mut iteration = 0_u32;
    while residual.abs() > CONVERGENCE_RESIDUAL && iteration < 50 {
        let root_squared = root * root;
        let root_cubed = root_squared * root;
        let root_fourth = root_squared * root_squared;
        residual = coefficients[0]
            + coefficients[1] * root
            + coefficients[2] * root_squared
            + coefficients[3] * root_cubed
            + coefficients[4] * root_fourth;
        let derivative = coefficients[1]
            + 2.0 * coefficients[2] * root
            + 3.0 * coefficients[3] * root_squared
            + 4.0 * coefficients[4] * root_cubed;
        iteration += 1;
        let damping = (50.0 - iteration as f32) * 0.02;
        root -= damping * (residual / derivative);
    }
    (residual.abs() < ACCEPTANCE_RESIDUAL).then_some(root)
}

fn primitive_hit(
    start: [f32; 3],
    delta: [f32; 3],
    parameter: f32,
    normal: [f32; 3],
) -> Option<PrimitiveHit> {
    (0.0..=1.0).contains(&parameter).then(|| PrimitiveHit {
        line_parameter: parameter,
        position: add(start, scale(delta, parameter)),
        normal: normalize(normal),
    })
}

/// Fixed 64-byte `rw::collision::SimpleMappedArray` prefix. The full PDB and
/// serializer at `0x828b4530` place `m_volumes` at `+0x30`; it is an
/// object-relative offset to `m_numVolumes` inline 96-byte volumes.
#[derive(Clone, Debug, PartialEq)]
pub struct SimpleMappedArray {
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
    pub aggregate_vtable_offset: u32,
    pub tag_bit_count: u32,
    pub volume_count: u32,
    pub aggregate_padding: u32,
    pub volumes_offset: u32,
}

impl SimpleMappedArray {
    /// Parses the exact PDB layout and fixed serialized prefix.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError::Truncated`] when the prefix is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, 64)?;
        Ok(Self {
            bounding_box_min: be_f32_array::<4>(bytes, 0)?,
            bounding_box_max: be_f32_array::<4>(bytes, 16)?,
            aggregate_vtable_offset: be_u32(bytes, 32)?,
            tag_bit_count: be_u32(bytes, 36)?,
            volume_count: be_u32(bytes, 40)?,
            aggregate_padding: be_u32(bytes, 44)?,
            volumes_offset: be_u32(bytes, 48)?,
        })
    }

    /// Parses the inline volume array fixed up by the reference serializer.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the allocation is outside this object.
    pub fn volumes(&self, bytes: &[u8]) -> Result<Vec<Volume>, CollisionError> {
        inline_volumes(bytes, self.volumes_offset, self.volume_count)
    }
}

/// Fixed 80-byte `rw::collision::KDTreeMappedArray` prefix. Its volumes,
/// KD-tree header, and branch allocation are all stored in the same arena
/// object and addressed relative to the object base.
#[derive(Clone, Debug, PartialEq)]
pub struct KdTreeMappedArray {
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
    pub aggregate_vtable_offset: u32,
    pub tag_bit_count: u32,
    pub volume_count: u32,
    pub aggregate_padding: u32,
    pub volumes_offset: u32,
    pub kd_tree_offset: u32,
}

impl KdTreeMappedArray {
    /// Parses the exact PDB layout.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError::Truncated`] when the prefix is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, 80)?;
        Ok(Self {
            bounding_box_min: be_f32_array::<4>(bytes, 0)?,
            bounding_box_max: be_f32_array::<4>(bytes, 16)?,
            aggregate_vtable_offset: be_u32(bytes, 32)?,
            tag_bit_count: be_u32(bytes, 36)?,
            volume_count: be_u32(bytes, 40)?,
            aggregate_padding: be_u32(bytes, 44)?,
            volumes_offset: be_u32(bytes, 48)?,
            kd_tree_offset: be_u32(bytes, 64)?,
        })
    }

    /// Parses the inline 96-byte volume array.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the allocation is outside this object.
    pub fn volumes(&self, bytes: &[u8]) -> Result<Vec<Volume>, CollisionError> {
        inline_volumes(bytes, self.volumes_offset, self.volume_count)
    }

    /// Resolves the inline KD-tree header.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when its object-relative offset is invalid.
    pub fn kd_tree(&self, bytes: &[u8]) -> Result<KdTree, CollisionError> {
        KdTree::parse(offset_slice(bytes, self.kd_tree_offset)?)
    }

    /// Uses the mapped array's KD tree to select volume indices overlapping an
    /// AABB. Leaf starts are direct indices into `m_volumes`.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for malformed trees or leaf ranges outside
    /// the volume array.
    pub fn candidate_volume_indices_for_aabb(
        &self,
        bytes: &[u8],
        minimum: [f32; 3],
        maximum: [f32; 3],
    ) -> Result<Vec<usize>, CollisionError> {
        let ranges = self
            .kd_tree(bytes)?
            .leaf_ranges_for_aabb(bytes, minimum, maximum)?;
        let volume_count =
            usize::try_from(self.volume_count).map_err(|_| CollisionError::IntegerOverflow)?;
        let mut indices = Vec::new();
        for range in ranges {
            let start = usize::try_from(range.encoded_start)
                .map_err(|_| CollisionError::IntegerOverflow)?;
            let end = start
                .checked_add(
                    usize::try_from(range.unit_count)
                        .map_err(|_| CollisionError::IntegerOverflow)?,
                )
                .ok_or(CollisionError::IntegerOverflow)?;
            if end > volume_count {
                return Err(CollisionError::KdEntryOutsideMappedArray {
                    start,
                    end,
                    volume_count,
                });
            }
            indices.extend(start..end);
        }
        Ok(indices)
    }

    /// Uses the mapped array's dedicated KD line traversal to select volume
    /// indices in native near-first leaf order.
    ///
    /// `fatness` is forwarded to `KDTreeLineQuery`; this method intentionally
    /// does not call [`KdTreeLineRangeQuery::clip_end`] because it returns the
    /// complete candidate set rather than a nearest-hit query.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for malformed trees or leaf ranges outside
    /// the volume array. Traversal and every range are validated before any
    /// candidate vector is returned.
    pub fn candidate_volume_indices_for_line(
        &self,
        bytes: &[u8],
        start: [f32; 3],
        end: [f32; 3],
        fatness: f32,
    ) -> Result<Vec<usize>, CollisionError> {
        let query = self
            .kd_tree(bytes)?
            .line_range_query(bytes, start, end, fatness)?;
        let ranges = collect_kd_line_ranges(query)?;
        let volume_count =
            usize::try_from(self.volume_count).map_err(|_| CollisionError::IntegerOverflow)?;
        let checked_ranges = ranges
            .into_iter()
            .map(|range| {
                let start = usize::try_from(range.encoded_start)
                    .map_err(|_| CollisionError::IntegerOverflow)?;
                let end = start
                    .checked_add(
                        usize::try_from(range.unit_count)
                            .map_err(|_| CollisionError::IntegerOverflow)?,
                    )
                    .ok_or(CollisionError::IntegerOverflow)?;
                if end > volume_count {
                    return Err(CollisionError::KdEntryOutsideMappedArray {
                        start,
                        end,
                        volume_count,
                    });
                }
                Ok(start..end)
            })
            .collect::<Result<Vec<_>, CollisionError>>()?;
        Ok(checked_ranges.into_iter().flatten().collect())
    }
}

#[path = "collision_triangle_cache.rs"]
mod triangle_cache;
pub use triangle_cache::PreparedMeshTriangles;

/// Fixed `rw::collision::ClusteredMesh` prefix. Variable cluster and KD-tree
/// storage can follow this header inside the same arena object allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusteredMesh {
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
    pub aggregate_vtable_offset: u32,
    pub tag_bit_count: u32,
    pub volume_count: u32,
    pub aggregate_padding: u32,
    pub kd_tree_offset: u32,
    pub cluster_data_offset: u32,
    pub vertex_compression_granularity: f32,
    pub cluster_flags: u16,
    pub group_id_size: u8,
    pub surface_id_size: u8,
    pub cluster_count: u32,
    pub maximum_cluster_count: u32,
    pub unit_count: u32,
    pub maximum_unit_count: u32,
    pub allocation_size: u32,
    pub default_group_id: u16,
    pub default_surface_id: u16,
    pub default_edge_angle: u8,
}

impl ClusteredMesh {
    /// Parses the 96-byte `rw::collision::ClusteredMesh` prefix from the exact
    /// reference debug layout.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError::Truncated`] when the fixed header is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_header(bytes)?;
        Ok(Self {
            bounding_box_min: be_f32_array::<4>(bytes, 0)?,
            bounding_box_max: be_f32_array::<4>(bytes, 16)?,
            aggregate_vtable_offset: be_u32(bytes, 32)?,
            tag_bit_count: be_u32(bytes, 36)?,
            volume_count: be_u32(bytes, 40)?,
            aggregate_padding: be_u32(bytes, 44)?,
            kd_tree_offset: be_u32(bytes, 48)?,
            cluster_data_offset: be_u32(bytes, 52)?,
            vertex_compression_granularity: be_f32(bytes, 56)?,
            cluster_flags: be_u16(bytes, 60)?,
            group_id_size: bytes[62],
            surface_id_size: bytes[63],
            cluster_count: be_u32(bytes, 64)?,
            maximum_cluster_count: be_u32(bytes, 68)?,
            unit_count: be_u32(bytes, 72)?,
            maximum_unit_count: be_u32(bytes, 76)?,
            allocation_size: be_u32(bytes, 80)?,
            default_group_id: be_u16(bytes, 84)?,
            default_surface_id: be_u16(bytes, 86)?,
            default_edge_angle: bytes[88],
        })
    }

    /// Resolves the serialized KD-tree offset relative to this mesh object.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the offset or fixed tree header lies
    /// outside the mesh allocation.
    pub fn kd_tree(&self, mesh_bytes: &[u8]) -> Result<KdTree, CollisionError> {
        KdTree::parse(offset_slice(mesh_bytes, self.kd_tree_offset)?)
    }

    /// Reads the cluster-offset table. `mCluster` is an `unsigned int *` in the
    /// debug layout; each big-endian entry is relative to the mesh base.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the table is outside the mesh object.
    pub fn cluster_offsets(&self, mesh_bytes: &[u8]) -> Result<Vec<u32>, CollisionError> {
        let start = usize::try_from(self.cluster_data_offset)
            .map_err(|_| CollisionError::IntegerOverflow)?;
        let size = usize::try_from(self.cluster_count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or(CollisionError::IntegerOverflow)?;
        let end = start
            .checked_add(size)
            .ok_or(CollisionError::IntegerOverflow)?;
        let table = mesh_bytes
            .get(start..end)
            .ok_or(CollisionError::RangeOutsideObject {
                start,
                end,
                object_size: mesh_bytes.len(),
            })?;
        (0..self.cluster_count as usize)
            .map(|index| be_u32(table, index * 4))
            .collect()
    }

    /// Resolves and parses the first serialized cluster through its offset
    /// table.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the offset or cluster subranges are
    /// malformed.
    pub fn first_cluster(&self, mesh_bytes: &[u8]) -> Result<MeshCluster, CollisionError> {
        self.clusters(mesh_bytes)?
            .into_iter()
            .next()
            .ok_or(CollisionError::NoClusters)
    }

    /// Parses every cluster referenced by the mesh's relative offset table.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when an offset or cluster allocation is
    /// malformed.
    pub fn clusters(&self, mesh_bytes: &[u8]) -> Result<Vec<MeshCluster>, CollisionError> {
        self.cluster_offsets(mesh_bytes)?
            .into_iter()
            .map(|offset| MeshCluster::parse(offset_slice(mesh_bytes, offset)?))
            .collect()
    }

    /// Visits the serialized units selected by one encoded KD leaf range.
    ///
    /// A nonempty range resolves and decodes only its selected cluster, begins
    /// directly at the encoded unit byte offset, and advances by each decoded
    /// unit's encoded size. Units before the selected offset and bytes after
    /// the requested count are deliberately not decoded. Returning
    /// [`ControlFlow::Break`] from `visitor` stops before the next unit is
    /// inspected.
    ///
    /// A zero unit count mirrors the native rip-loop gate and returns without
    /// resolving the encoded cluster or offset.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when a visited cluster-table entry, cluster,
    /// unit offset, or requested unit record is outside the supplied mesh, or
    /// when a visited unit is malformed.
    pub fn visit_leaf_range_units<B, F>(
        &self,
        mesh_bytes: &[u8],
        range: KdLeafRange,
        visitor: F,
    ) -> Result<ControlFlow<B>, CollisionError>
    where
        F: FnMut(usize, &MeshCluster, &MeshUnit) -> ControlFlow<B>,
    {
        self.visit_leaf_range_units_in(mesh_bytes, range, &mut LeafClusterMemo::default(), visitor)
    }

    /// [`Self::visit_leaf_range_units`] for successive ranges of one mesh:
    /// `memo` keeps the last decoded cluster, so ranges in the same cluster
    /// decode it once. A memo serves a single mesh's bytes. A resident mesh may
    /// also supply a [`LeafClusterCache`] to reuse decoded clusters across visits.
    ///
    /// # Errors
    ///
    /// As [`Self::visit_leaf_range_units`].
    pub fn visit_leaf_range_units_in<B, F>(
        &self,
        mesh_bytes: &[u8],
        range: KdLeafRange,
        memo: &mut LeafClusterMemo<'_>,
        mut visitor: F,
    ) -> Result<ControlFlow<B>, CollisionError>
    where
        F: FnMut(usize, &MeshCluster, &MeshUnit) -> ControlFlow<B>,
    {
        if range.unit_count == 0 {
            return Ok(ControlFlow::Continue(()));
        }

        let shift = if self.maximum_cluster_count <= 0x1_0000 {
            16
        } else {
            20
        };
        let mask = (1_u32 << shift) - 1;
        let mut cluster_index = usize::try_from(range.encoded_start >> shift)
            .map_err(|_| CollisionError::IntegerOverflow)?;
        let mut unit_offset = usize::try_from(range.encoded_start & mask)
            .map_err(|_| CollisionError::IntegerOverflow)?;

        for _ in 0..range.unit_count {
            let cluster = self.leaf_cluster(mesh_bytes, memo, cluster_index)?;
            let remaining = cluster.unit_data.get(unit_offset..).ok_or(
                CollisionError::EncodedUnitOffsetNotFound {
                    cluster: cluster_index,
                    offset: unit_offset,
                },
            )?;
            let (unit, encoded_size) = parse_unit(remaining, unit_offset, self)?;
            if let ControlFlow::Break(value) = visitor(cluster_index, cluster, &unit) {
                return Ok(ControlFlow::Break(value));
            }
            unit_offset = unit_offset
                .checked_add(encoded_size)
                .ok_or(CollisionError::IntegerOverflow)?;
            // A range can run on past its cluster's unit data (two coalesced sibling leaves). The
            // RenderWare queries go on at the next cluster's first unit
            // (`BBoxOverlapQueryThis` 0x82ada530); `RipTriangles` 0x827cd7c8 instead reads on
            // into whatever follows the cluster. PC port: the ripper goes on as the queries do.
            if unit_offset >= cluster.unit_data.len() {
                cluster_index += 1;
                unit_offset = 0;
            }
        }

        Ok(ControlFlow::Continue(()))
    }

    /// The memo's decoded cluster `cluster_index`, decoded into it when it holds another.
    fn leaf_cluster<'m>(
        &self,
        mesh_bytes: &[u8],
        memo: &'m mut LeafClusterMemo<'_>,
        cluster_index: usize,
    ) -> Result<&'m MeshCluster, CollisionError> {
        let cluster_count =
            usize::try_from(self.cluster_count).map_err(|_| CollisionError::IntegerOverflow)?;
        if cluster_index >= cluster_count {
            return Err(CollisionError::EncodedClusterOutsideMesh {
                index: cluster_index,
                cluster_count,
            });
        }
        if memo
            .cluster
            .as_ref()
            .is_none_or(|(index, _)| *index != cluster_index)
        {
            let table_start = usize::try_from(self.cluster_data_offset)
                .map_err(|_| CollisionError::IntegerOverflow)?;
            let table_entry = cluster_index
                .checked_mul(4)
                .and_then(|offset| table_start.checked_add(offset))
                .ok_or(CollisionError::IntegerOverflow)?;
            let cluster_offset = be_u32(mesh_bytes, table_entry)?;
            memo.cluster = Some((
                cluster_index,
                match memo.cache {
                    Some(cache) => DecodedLeafCluster::Shared(cache.get_or_parse(
                        cluster_offset,
                        || MeshCluster::parse(offset_slice(mesh_bytes, cluster_offset)?),
                    )?),
                    None => DecodedLeafCluster::Local(MeshCluster::parse(
                        offset_slice(mesh_bytes, cluster_offset)?,
                    )?),
                },
            ));
        }
        let Some((_, cluster)) = &memo.cluster else {
            unreachable!("the cluster was decoded above")
        };
        Ok(match cluster {
            DecodedLeafCluster::Local(cluster) => cluster,
            DecodedLeafCluster::Shared(cluster) => cluster.as_ref(),
        })
    }

    /// Expands KD leaf ranges into owned collision triangles.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when an encoded cluster/unit location or
    /// vertex index does not match the decoded mesh.
    pub fn triangles_for_leaf_ranges(
        &self,
        mesh_bytes: &[u8],
        ranges: &[KdLeafRange],
    ) -> Result<Vec<MeshTriangle>, CollisionError> {
        let clusters = self.clusters(mesh_bytes)?;
        let shift = if self.maximum_cluster_count <= 0x1_0000 {
            16
        } else {
            20
        };
        let mask = (1_u32 << shift) - 1;
        let decoded_units: Vec<Vec<MeshUnit>> = clusters
            .iter()
            .map(|cluster| cluster.units(self))
            .collect::<Result<_, _>>()?;
        let mut triangles = Vec::new();
        for range in ranges {
            append_leaf_triangles(
                self,
                &clusters,
                &decoded_units,
                *range,
                shift,
                mask,
                &mut triangles,
            )?;
        }
        Ok(triangles)
    }

    /// Broad-phases a segment through the native KD line traversal, performs
    /// the reference thin one-sided triangle test, and stably sorts nearest
    /// first.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when mesh, KD, cluster, unit, or vertex data
    /// is inconsistent.
    pub fn thin_segment_intersections(
        &self,
        mesh_bytes: &[u8],
        start: [f32; 3],
        end: [f32; 3],
    ) -> Result<Vec<MeshTriangleHit>, CollisionError> {
        let tree = self.kd_tree(mesh_bytes)?;
        let query = tree.line_range_query(mesh_bytes, start, end, 0.0)?;
        let ranges = collect_kd_line_ranges(query)?;
        let triangles = self.triangles_for_leaf_ranges(mesh_bytes, &ranges)?;
        let delta = subtract(end, start);
        let mut hits: Vec<_> = triangles
            .into_iter()
            .filter_map(|triangle| {
                thin_segment_triangle_intersection(start, delta, triangle.vertices).map(|hit| {
                    MeshTriangleHit {
                        triangle,
                        intersection: hit,
                    }
                })
            })
            .collect();
        hits.sort_by(|left, right| {
            left.intersection
                .line_parameter
                .total_cmp(&right.intersection.line_parameter)
        });
        Ok(hits)
    }

    /// Broad-phases a swept sphere through the native fat-line KD traversal
    /// and returns candidate triangle contacts stably sorted nearest first.
    ///
    /// Reference: `rw::collision::TriangleLineSegIntersect` at `0x82adf658`,
    /// which selects the fat path whenever line or triangle fatness is nonzero.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when mesh, KD, cluster, unit, or vertex data
    /// is inconsistent.
    pub fn sphere_segment_intersections(
        &self,
        mesh_bytes: &[u8],
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Vec<MeshTriangleHit>, CollisionError> {
        let tree = self.kd_tree(mesh_bytes)?;
        let query = tree.line_range_query(mesh_bytes, start, end, radius)?;
        let ranges = collect_kd_line_ranges(query)?;
        let triangles = self.triangles_for_leaf_ranges(mesh_bytes, &ranges)?;
        let delta = subtract(end, start);
        let mut hits: Vec<_> = triangles
            .into_iter()
            .filter_map(|triangle| {
                sphere_segment_triangle_intersection(start, delta, radius, triangle.vertices).map(
                    |intersection| MeshTriangleHit {
                        triangle,
                        intersection,
                    },
                )
            })
            .collect();
        hits.sort_by(|left, right| {
            left.intersection
                .line_parameter
                .total_cmp(&right.intersection.line_parameter)
        });
        Ok(hits)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshTriangle {
    pub cluster_index: usize,
    pub unit_stream_offset: usize,
    pub vertices: [[f32; 3]; 3],
    pub group_id: Option<u16>,
    pub surface_id: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshTriangleHit {
    pub triangle: MeshTriangle,
    pub intersection: TriangleHit,
}

/// Serialized `rw::collision::KDTree` header.
#[derive(Clone, Debug, PartialEq)]
pub struct KdTree {
    /// Offset relative to the containing collision object, as established by
    /// `ClusteredMesh::Fixup` in the reference executable.
    pub branch_nodes_offset: u32,
    pub branch_node_count: u32,
    pub entry_count: u32,
    pub bounding_box_min: [f32; 4],
    pub bounding_box_max: [f32; 4],
}

impl KdTree {
    /// Parses the exact 48-byte debug layout.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError::Truncated`] when the fixed header is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, 48)?;
        Ok(Self {
            branch_nodes_offset: be_u32(bytes, 0)?,
            branch_node_count: be_u32(bytes, 4)?,
            entry_count: be_u32(bytes, 8)?,
            bounding_box_min: be_f32_array::<4>(bytes, 16)?,
            bounding_box_max: be_f32_array::<4>(bytes, 32)?,
        })
    }

    /// Parses branch nodes using the containing mesh as the pointer base.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] if the branch-node allocation is outside the
    /// mesh object.
    pub fn branch_nodes(&self, mesh_bytes: &[u8]) -> Result<Vec<BranchNode>, CollisionError> {
        self.branch_node_bytes(mesh_bytes)?
            .chunks_exact(32)
            .map(BranchNode::parse)
            .collect()
    }

    fn branch_node_bytes<'a>(&self, mesh_bytes: &'a [u8]) -> Result<&'a [u8], CollisionError> {
        if self.branch_node_count == 0 {
            return Ok(&[]);
        }
        let start = usize::try_from(self.branch_nodes_offset)
            .map_err(|_| CollisionError::IntegerOverflow)?;
        let size = usize::try_from(self.branch_node_count)
            .ok()
            .and_then(|count| count.checked_mul(32))
            .ok_or(CollisionError::IntegerOverflow)?;
        let end = start
            .checked_add(size)
            .ok_or(CollisionError::IntegerOverflow)?;
        let bytes = mesh_bytes
            .get(start..end)
            .ok_or(CollisionError::RangeOutsideObject {
                start,
                end,
                object_size: mesh_bytes.len(),
            })?;
        Ok(bytes)
    }

    /// Starts the dedicated stateful KD line query used by native collision
    /// traversal.
    ///
    /// The clipper construction and traversal state follow
    /// `AALineClipper::Init` at `0x827bd340`, the `KDTreeLineQuery`
    /// constructor at `0x827bd480`, and the inlined traversal in
    /// `ClusteredMeshLineQueryIterator::QueryRange` at `0x827bb9d8`.
    /// The recovered operation and comparison sequence is followed for
    /// ordinary finite inputs. The host reciprocal seeds the two recovered
    /// Newton refinements; exact Xenon `vrefp`, exceptional VMX values, and
    /// fused-rounding parity remain an explicit hardware-oracle boundary.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] if the branch-node allocation is outside
    /// the containing object. Malformed branch references and stack overflow
    /// are reported lazily as safe-host errors by
    /// [`KdTreeLineRangeQuery::next_range`].
    pub fn line_range_query<'a>(
        &self,
        mesh_bytes: &'a [u8],
        start: [f32; 3],
        end: [f32; 3],
        fatness: f32,
    ) -> Result<KdTreeLineRangeQuery<'a>, CollisionError> {
        let branch_nodes = KdLineBranches::Serialized(self.branch_node_bytes(mesh_bytes)?);
        let line_clipper = KdAxisAlignedLineClipper::new(
            start,
            end,
            fatness,
            self.bounding_box_min,
            self.bounding_box_max,
        );
        let mut stack = Vec::new();
        let mut leaf_count = 0;
        let next_entry = 0;

        let expanded_max: [f32; 3] =
            std::array::from_fn(|axis| self.bounding_box_max[axis] + line_clipper.padding[axis]);
        let expanded_min: [f32; 3] =
            std::array::from_fn(|axis| self.bounding_box_min[axis] - line_clipper.padding[axis]);
        let to_max: [f32; 3] =
            std::array::from_fn(|axis| expanded_max[axis] - line_clipper.origin[axis]);
        let to_min: [f32; 3] =
            std::array::from_fn(|axis| expanded_min[axis] - line_clipper.origin[axis]);
        let t_max_candidate: [f32; 3] =
            std::array::from_fn(|axis| line_clipper.recip[axis] * to_max[axis]);
        let t_min_candidate: [f32; 3] =
            std::array::from_fn(|axis| line_clipper.recip[axis] * to_min[axis]);
        let axis_exit: [f32; 3] =
            std::array::from_fn(|axis| vmx_max_host(t_min_candidate[axis], t_max_candidate[axis]));
        let axis_entry: [f32; 3] =
            std::array::from_fn(|axis| vmx_min_host(t_min_candidate[axis], t_max_candidate[axis]));

        let entry_xy = max_subsel(axis_entry[0], axis_entry[1]);
        let entry_xyz = max_subsel(entry_xy, axis_entry[2]);
        let pa = max_subsel(0.0, entry_xyz);
        let exit_xy = min_subsel(axis_exit[0], axis_exit[1]);
        let exit_xyz = min_subsel(exit_xy, axis_exit[2]);
        let pb = min_subsel(1.0, exit_xyz);

        // `fcmpu pa,pb; bge reject` accepts only an ordered strict interval.
        if pa < pb {
            if branch_nodes.is_empty() {
                leaf_count = self.entry_count;
            } else {
                stack.push(KdLineStackElement {
                    content: u32::MAX,
                    index: 0,
                    pa,
                    pb,
                });
            }
        }

        Ok(KdTreeLineRangeQuery {
            branch_nodes,
            line_clipper,
            stack,
            leaf_count,
            next_entry,
        })
    }

    /// Starts a stateful AABB range query in the reference traversal order.
    ///
    /// This mirrors `ClusteredMeshBBoxQuery::ProcessBranchNode` at
    /// `0x827ccc48` and `ClusteredMeshBBoxQuery::GetNext` at `0x827ccd80`:
    /// the high child is processed first, the low child second, and a leaf
    /// result is returned before another stacked branch is visited.
    ///
    /// The invalid-box and malformed-tree errors are safe-host validation
    /// boundaries. The reference routines assume a valid serialized tree.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] when the query box is invalid or the
    /// branch-node allocation is outside the containing object.
    pub fn aabb_range_query<'a>(
        &self,
        mesh_bytes: &'a [u8],
        minimum: [f32; 3],
        maximum: [f32; 3],
    ) -> Result<KdTreeAabbRangeQuery<'a>, CollisionError> {
        if (0..3).any(|axis| minimum[axis] > maximum[axis]) {
            return Err(CollisionError::InvalidBoundingBox);
        }
        let branch_nodes = self.branch_node_bytes(mesh_bytes)?;
        let (branch_stack, pending_branchless_range) = if branch_nodes.is_empty() {
            (
                Vec::new(),
                (self.entry_count != 0).then_some(KdLeafRange {
                    encoded_start: 0,
                    unit_count: self.entry_count,
                }),
            )
        } else {
            // Safe-host storage: the decompilation exposes an inline native
            // stack but does not establish its capacity here. A Vec preserves
            // the recovered LIFO behavior without inventing that limit.
            (vec![0], None)
        };
        Ok(KdTreeAabbRangeQuery {
            branch_nodes,
            minimum,
            maximum,
            branch_stack,
            pending_branchless_range,
        })
    }

    /// Collects the stateful AABB query into encoded leaf ranges.
    ///
    /// The returned vector is failure-atomic: a malformed branch encountered
    /// after earlier ranges still returns only the error. Range ordering and
    /// sibling-leaf coalescing follow [`Self::aabb_range_query`].
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for an invalid query box, branch axis, or
    /// branch child index.
    pub fn leaf_ranges_for_aabb(
        &self,
        mesh_bytes: &[u8],
        minimum: [f32; 3],
        maximum: [f32; 3],
    ) -> Result<Vec<KdLeafRange>, CollisionError> {
        let mut query = self.aabb_range_query(mesh_bytes, minimum, maximum)?;
        let mut ranges = Vec::new();
        while let Some(range) = query.next_range()? {
            ranges.push(range);
        }
        Ok(ranges)
    }

    /// Collects at most `limit` native-ordered AABB leaf ranges.
    ///
    /// Reference: `BoxTest<AssemblyTriangleIntersectionTester,1>` at
    /// `0x827CD960..0x827CDB98`, whose per-mesh loop stores at most 256 range
    /// descriptors before ripping them, without requesting another batch.
    /// The limit counts ranges, not units or triangles. Same-branch sibling
    /// counts still coalesce, including their wrapping-u32 addition.
    ///
    /// Branch bytes and metadata are read only when visited. Once the limit
    /// is reached no further branch is read or validated, even if its bytes
    /// are truncated or its axis/reference is malformed. Unlike the eager
    /// allocation check in [`Self::aabb_range_query`], this deliberately checks
    /// only each visited 32-byte record. A zero limit is a host API no-op that
    /// returns an empty vector before inspecting the box or tree.
    ///
    /// Results retain the full encoded start and u32 count. Native `BoxTest`'s
    /// subsequent conversion to three u16 descriptor fields is a separate
    /// caller operation and is not performed by this generic KD query.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for an invalid query box or a visited
    /// malformed/truncated branch. As in [`Self::leaf_ranges_for_aabb`], a
    /// failure before reaching the cap discards the accumulated result.
    pub fn leaf_ranges_for_aabb_limited(
        &self,
        mesh_bytes: &[u8],
        minimum: [f32; 3],
        maximum: [f32; 3],
        limit: usize,
    ) -> Result<Vec<KdLeafRange>, CollisionError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        if (0..3).any(|axis| minimum[axis] > maximum[axis]) {
            return Err(CollisionError::InvalidBoundingBox);
        }
        if self.branch_node_count == 0 {
            return Ok((self.entry_count != 0)
                .then_some(KdLeafRange {
                    encoded_start: 0,
                    unit_count: self.entry_count,
                })
                .into_iter()
                .collect());
        }

        let mut branch_stack = vec![0];
        let mut ranges = Vec::new();
        while ranges.len() < limit {
            let Some(index) = branch_stack.pop() else {
                break;
            };
            let node = self.branch_node_at(mesh_bytes, index)?;
            let axis = usize::try_from(node.axis).map_err(|_| CollisionError::IntegerOverflow)?;
            if axis >= 3 {
                return Err(CollisionError::InvalidKdAxis {
                    node: usize::try_from(index).map_err(|_| CollisionError::IntegerOverflow)?,
                    axis: node.axis,
                });
            }
            let mut encoded_start = 0;
            let mut unit_count = 0;
            if maximum[axis] >= node.extents[1] {
                process_kd_child(
                    &node,
                    1,
                    &mut branch_stack,
                    &mut encoded_start,
                    &mut unit_count,
                );
            }
            if minimum[axis] <= node.extents[0] {
                process_kd_child(
                    &node,
                    0,
                    &mut branch_stack,
                    &mut encoded_start,
                    &mut unit_count,
                );
            }
            if unit_count != 0 {
                ranges.push(KdLeafRange {
                    encoded_start,
                    unit_count,
                });
            }
        }
        Ok(ranges)
    }

    fn branch_node_at(&self, mesh_bytes: &[u8], index: u32) -> Result<BranchNode, CollisionError> {
        let index = usize::try_from(index).map_err(|_| CollisionError::IntegerOverflow)?;
        let branch_count =
            usize::try_from(self.branch_node_count).map_err(|_| CollisionError::IntegerOverflow)?;
        if index >= branch_count {
            return Err(CollisionError::KdBranchOutsideTree {
                index,
                branch_count,
            });
        }
        let base = usize::try_from(self.branch_nodes_offset)
            .map_err(|_| CollisionError::IntegerOverflow)?;
        let start = index
            .checked_mul(32)
            .and_then(|offset| base.checked_add(offset))
            .ok_or(CollisionError::IntegerOverflow)?;
        let end = start
            .checked_add(32)
            .ok_or(CollisionError::IntegerOverflow)?;
        let bytes = mesh_bytes
            .get(start..end)
            .ok_or(CollisionError::RangeOutsideObject {
                start,
                end,
                object_size: mesh_bytes.len(),
            })?;
        BranchNode::parse(bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KdLeafRange {
    pub encoded_start: u32,
    pub unit_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct KdAxisAlignedLineClipper {
    origin: [f32; 3],
    delta: [f32; 3],
    recip: [f32; 3],
    padding: [f32; 3],
}

impl KdAxisAlignedLineClipper {
    fn new(
        start: [f32; 3],
        end: [f32; 3],
        fatness: f32,
        bounding_box_min: [f32; 4],
        bounding_box_max: [f32; 4],
    ) -> Self {
        let input_padding: [f32; 3] = std::array::from_fn(|axis| {
            // 0x827bd514, raw 0x116d602e: VMX VA13 * VC0 + VB12.
            // VC0 is epsilon; VB12 is fatness (IDA prints B before C).
            vmx_max_host(clear_sign(start[axis]), clear_sign(end[axis])).mul_add(0.000_001, fatness)
        });
        let raw_delta: [f32; 3] = std::array::from_fn(|axis| end[axis] - start[axis]);
        let half_delta = raw_delta.map(|value| value * 0.5);
        let extension_magnitude: [f32; 3] = std::array::from_fn(|axis| {
            let abs_bounds = vmx_max_host(
                clear_sign(bounding_box_min[axis]),
                clear_sign(bounding_box_max[axis]),
            );
            let scale = vmx_max_host(abs_bounds, clear_sign(start[axis]));
            let minimum_half_delta = vmx_max_host(clear_sign(half_delta[axis]), scale * 0.000_001);
            minimum_half_delta - clear_sign(half_delta[axis])
        });
        let extension: [f32; 3] = std::array::from_fn(|axis| {
            let sign = if half_delta[axis] >= 0.0 { 1.0 } else { -1.0 };
            sign * extension_magnitude[axis]
        });
        let origin = std::array::from_fn(|axis| start[axis] - extension[axis]);
        let delta = std::array::from_fn(|axis| {
            let extended_end = end[axis] + extension[axis];
            extended_end - origin[axis]
        });
        let recip = delta.map(refined_host_reciprocal);
        let padding = std::array::from_fn(|axis| input_padding[axis] + extension_magnitude[axis]);
        Self {
            origin,
            delta,
            recip,
            padding,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct KdLineStackElement {
    content: u32,
    index: u32,
    pa: f32,
    pb: f32,
}

/// Immutable serialized branch view. The entire allocation is range-checked at
/// construction, exactly as the eager decoder; branch fields are decoded only
/// when native traversal visits them. No mesh mutation is possible while borrowed.
#[derive(Clone, Debug)]
enum KdLineBranches<'a> {
    Serialized(&'a [u8]),
    #[cfg(test)]
    Parsed(Vec<BranchNode>),
}
impl KdLineBranches<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Serialized(bytes) => bytes.len() / 32,
            #[cfg(test)]
            Self::Parsed(nodes) => nodes.len(),
        }
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, index: usize) -> Option<BranchNode> {
        match self {
            Self::Serialized(bytes) => {
                if index >= bytes.len() / 32 {
                    return None;
                }
                // The constructor validated a whole number of 32-byte nodes;
                // parse has no value-dependent errors, only fixed-field bounds.
                Some(
                    BranchNode::parse(&bytes[index * 32..index * 32 + 32])
                        .expect("validated complete KD branch record"),
                )
            }
            #[cfg(test)]
            Self::Parsed(nodes) => nodes.get(index).cloned(),
        }
    }
}
// Preserve the previous parsed-node equality semantics, including float NaNs
// and signed zero. Equality is diagnostic and does not run in traversal.
impl PartialEq for KdLineBranches<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && (0..self.len()).all(|index| self.get(index) == other.get(index))
    }
}

/// Stateful native-order traversal of KD leaves intersected by a line.
///
/// Native `KDTreeLineQuery` stores 32 stack elements. This host type preserves
/// that capacity as a checked boundary and reports malformed serialized trees
/// instead of performing the reference code's unchecked memory access.
#[derive(Clone, Debug, PartialEq)]
pub struct KdTreeLineRangeQuery<'a> {
    branch_nodes: KdLineBranches<'a>,
    line_clipper: KdAxisAlignedLineClipper,
    stack: Vec<KdLineStackElement>,
    leaf_count: u32,
    next_entry: u32,
}

impl KdTreeLineRangeQuery<'_> {
    /// Returns the next intersected leaf in native near-first traversal order.
    ///
    /// Native code pushes the geometrically far child first and the near child
    /// second, so the near child is consumed first by the LIFO stack. Both
    /// split comparisons are strict. This formats-level API returns and
    /// consumes a whole native leaf span at once, advancing `next_entry` by
    /// that span's count; callers needing native per-entry behavior can expand
    /// the returned range in order.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for a visited malformed branch reference or
    /// axis, or when a tree would exceed the PDB-proven 32-element native
    /// stack. These are safe-host failure boundaries absent from the unchecked
    /// native routine. Callers must discard the query after any error; retry
    /// behavior is not part of this API contract. In particular, the
    /// deterministic stack-overflow replay is an additional safe-host property
    /// and does not make malformed-query recovery generally supported.
    pub fn next_range(&mut self) -> Result<Option<KdLeafRange>, CollisionError> {
        loop {
            if self.leaf_count != 0 {
                let range = KdLeafRange {
                    encoded_start: self.next_entry,
                    unit_count: self.leaf_count,
                };
                self.next_entry = self.next_entry.wrapping_add(self.leaf_count);
                self.leaf_count = 0;
                return Ok(Some(range));
            }

            let Some(element) = self.stack.pop() else {
                return Ok(None);
            };
            if element.content != u32::MAX {
                self.leaf_count = element.content;
                self.next_entry = element.index;
                continue;
            }

            let node_index =
                usize::try_from(element.index).map_err(|_| CollisionError::IntegerOverflow)?;
            let node =
                self.branch_nodes
                    .get(node_index)
                    .ok_or(CollisionError::KdBranchOutsideTree {
                        index: node_index,
                        branch_count: self.branch_nodes.len(),
                    })?;
            let axis = usize::try_from(node.axis).map_err(|_| CollisionError::IntegerOverflow)?;
            if axis >= 3 {
                return Err(CollisionError::InvalidKdAxis {
                    node: node_index,
                    axis: node.axis,
                });
            }

            let t = [
                ((node.extents[0] + self.line_clipper.padding[axis])
                    - self.line_clipper.origin[axis])
                    * self.line_clipper.recip[axis],
                ((node.extents[1] - self.line_clipper.padding[axis])
                    - self.line_clipper.origin[axis])
                    * self.line_clipper.recip[axis],
            ];
            let positive = usize::from(self.line_clipper.recip[axis] > 0.0);
            let other = positive ^ 1;
            let push_far = element.pb > t[positive];
            let push_near = element.pa < t[other];
            let required = self.stack.len() + usize::from(push_far) + usize::from(push_near);
            if required > KD_LINE_STACK_CAPACITY {
                // Restore the popped branch so this safe-host fault replays
                // deterministically without exposing a half-pushed far child.
                self.stack.push(element);
                return Err(CollisionError::KdLineStackCapacityExceeded {
                    capacity: KD_LINE_STACK_CAPACITY,
                    required,
                });
            }

            if push_far {
                self.stack.push(KdLineStackElement {
                    content: node.child_content[positive],
                    index: node.child_index[positive],
                    pa: max_subsel(element.pa, t[positive]),
                    pb: element.pb,
                });
            }
            if push_near {
                self.stack.push(KdLineStackElement {
                    content: node.child_content[other],
                    index: node.child_index[other],
                    pa: element.pa,
                    pb: min_subsel(element.pb, t[other]),
                });
            }
        }
    }

    /// Shortens pending stack intervals to `end_parameter` in native stack
    /// order.
    ///
    /// As in `KDTreeLineQuery::ClipEnd` at `0x82ac8b70`, this does not modify
    /// the currently buffered leaf range.
    pub fn clip_end(&mut self, end_parameter: f32) {
        self.stack.retain_mut(|element| {
            if element.pa > end_parameter {
                false
            } else {
                element.pb = min_subsel(element.pb, end_parameter);
                true
            }
        });
    }
}

/// Stateful traversal of serialized KD leaves intersecting one AABB.
///
/// This type owns parsed branch metadata but does not retain the containing
/// object bytes. Call [`Self::next_range`] repeatedly to preserve the
/// reference query's observable range boundaries and order.
#[derive(Clone, Debug, PartialEq)]
pub struct KdTreeAabbRangeQuery<'a> {
    /// The whole serialized branch-node allocation, 32 bytes per node; a node
    /// is decoded when the traversal visits it.
    branch_nodes: &'a [u8],
    minimum: [f32; 3],
    maximum: [f32; 3],
    branch_stack: Vec<u32>,
    pending_branchless_range: Option<KdLeafRange>,
}

impl KdTreeAabbRangeQuery<'_> {
    /// Returns the next native-ordered leaf range.
    ///
    /// High and low leaf counts from the same branch are combined exactly at
    /// that branch-processing boundary. Separate branch results remain
    /// separate even when their encoded ranges are adjacent.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] if a visited branch reference or axis is
    /// invalid. These checks are safe-host validation absent from the
    /// reference routine, which assumes a valid tree.
    pub fn next_range(&mut self) -> Result<Option<KdLeafRange>, CollisionError> {
        if let Some(range) = self.pending_branchless_range.take() {
            return Ok(Some(range));
        }

        while let Some(encoded_node_index) = self.branch_stack.pop() {
            let node_index =
                usize::try_from(encoded_node_index).map_err(|_| CollisionError::IntegerOverflow)?;
            let node = node_index
                .checked_mul(32)
                .and_then(|start| self.branch_nodes.get(start..start.checked_add(32)?))
                .ok_or(CollisionError::KdBranchOutsideTree {
                    index: node_index,
                    branch_count: self.branch_nodes.len() / 32,
                })?;
            let node = &BranchNode::parse(node)?;
            let axis = usize::try_from(node.axis).map_err(|_| CollisionError::IntegerOverflow)?;
            if axis >= 3 {
                return Err(CollisionError::InvalidKdAxis {
                    node: node_index,
                    axis: node.axis,
                });
            }

            let mut encoded_start = 0;
            let mut unit_count = 0_u32;
            if self.maximum[axis] >= node.extents[1] {
                process_kd_child(
                    node,
                    1,
                    &mut self.branch_stack,
                    &mut encoded_start,
                    &mut unit_count,
                );
            }
            if self.minimum[axis] <= node.extents[0] {
                process_kd_child(
                    node,
                    0,
                    &mut self.branch_stack,
                    &mut encoded_start,
                    &mut unit_count,
                );
            }
            if unit_count != 0 {
                return Ok(Some(KdLeafRange {
                    encoded_start,
                    unit_count,
                }));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchNode {
    pub parent: u32,
    pub axis: u32,
    pub child_content: [u32; 2],
    pub child_index: [u32; 2],
    pub extents: [f32; 2],
}

impl BranchNode {
    fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, 32)?;
        Ok(Self {
            parent: be_u32(bytes, 0)?,
            axis: be_u32(bytes, 4)?,
            child_content: [be_u32(bytes, 8)?, be_u32(bytes, 16)?],
            child_index: [be_u32(bytes, 12)?, be_u32(bytes, 20)?],
            extents: [be_f32(bytes, 24)?, be_f32(bytes, 28)?],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum VertexCompression {
    Uncompressed = 0,
    Unsigned16 = 1,
    Signed32 = 2,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClusterVertex {
    Uncompressed([f32; 4]),
    Unsigned16 {
        quantized_origin: [i32; 3],
        value: [u16; 3],
    },
    Signed32([i32; 3]),
}

impl ClusterVertex {
    /// Expands a vertex to world coordinates using the mesh bounding-box
    /// origin and compression granularity used by `ClusteredMesh`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn position(&self, mesh: &ClusteredMesh) -> [f32; 3] {
        match self {
            Self::Uncompressed(value) => [value[0], value[1], value[2]],
            Self::Unsigned16 {
                quantized_origin,
                value,
            } => std::array::from_fn(|index| {
                // `UnitGetOverlappingGPInstances` at `0x82acfdd0` expands
                // mode 1 with `vaddsws` before `vcsxwfp` and `vmulfp`.
                let quantized = quantized_origin[index].saturating_add(i32::from(value[index]));
                quantized as f32 * mesh.vertex_compression_granularity
            }),
            // Signed 32-bit vertices retain absolute quantized coordinates;
            // unlike the unsigned form they do not need the AABB origin.
            Self::Signed32(value) => std::array::from_fn(|index| {
                value[index] as f32 * mesh.vertex_compression_granularity
            }),
        }
    }
}

/// Lazily decoded clusters for ONE immutable resident mesh's bytes.
///
/// The owner must discard this cache when those bytes change or unload. Native
/// `BoxTest<...,1>` (`0x827cd960`, cluster lookup `0x827cdb64`) reuses resident
/// cluster allocations. This cache removes repeated host endian decoding; it
/// does not cache selected ranges, units, transformed vertices, or contacts.
/// Only successful parses are retained, when their range is first visited.
#[derive(Debug, Default)]
pub struct LeafClusterCache {
    clusters: Mutex<BTreeMap<u32, Arc<MeshCluster>>>,
}

impl LeafClusterCache {
    /// Starts a range-local memo borrowing this same immutable mesh's cache.
    #[must_use]
    pub fn memo(&self) -> LeafClusterMemo<'_> {
        LeafClusterMemo { cluster: None, cache: Some(self) }
    }

    fn get_or_parse(
        &self,
        offset: u32,
        parse: impl FnOnce() -> Result<MeshCluster, CollisionError>,
    ) -> Result<Arc<MeshCluster>, CollisionError> {
        if let Ok(clusters) = self.clusters.lock() {
            if let Some(cluster) = clusters.get(&offset) {
                return Ok(Arc::clone(cluster));
            }
        }
        // Never hold the cache lock while parsing or visiting physics units.
        // A poisoned host cache falls back to the original checked parser.
        let decoded = Arc::new(parse()?);
        if let Ok(mut clusters) = self.clusters.lock() {
            return Ok(Arc::clone(clusters.entry(offset).or_insert(decoded)));
        }
        Ok(decoded)
    }
}

#[derive(Debug)]
enum DecodedLeafCluster {
    Local(MeshCluster),
    Shared(Arc<MeshCluster>),
}

/// The cluster that successive leaf-range visits of one mesh decoded last.
#[derive(Debug, Default)]
pub struct LeafClusterMemo<'a> {
    cluster: Option<(usize, DecodedLeafCluster)>,
    cache: Option<&'a LeafClusterCache>,
}

/// Header, vertices, normals, and raw variable-length unit stream from one
/// `rw::collision::ClusteredMeshCluster` allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCluster {
    pub unit_count: u16,
    pub unit_data_size: u16,
    pub unit_data_start: u16,
    pub normal_start: u16,
    pub total_size: u16,
    pub vertex_count: u8,
    pub normal_count: u8,
    pub compression: VertexCompression,
    pub vertices: Vec<ClusterVertex>,
    pub normals: Vec<[f32; 4]>,
    pub unit_data: Vec<u8>,
}

impl MeshCluster {
    /// Parses the variable cluster using its embedded relative offsets.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for an unknown compression mode or any
    /// vertex, normal, or unit range outside the declared cluster size.
    pub fn parse(bytes: &[u8]) -> Result<Self, CollisionError> {
        require_size(bytes, 16)?;
        let total_size = be_u16(bytes, 8)?;
        let cluster =
            bytes
                .get(..usize::from(total_size))
                .ok_or(CollisionError::RangeOutsideObject {
                    start: 0,
                    end: usize::from(total_size),
                    object_size: bytes.len(),
                })?;
        let vertex_count = bytes[10];
        let normal_count = bytes[11];
        let compression = match bytes[12] {
            0 => VertexCompression::Uncompressed,
            1 => VertexCompression::Unsigned16,
            2 => VertexCompression::Signed32,
            value => return Err(CollisionError::UnknownVertexCompression { value }),
        };
        let unit_data_size = be_u16(bytes, 2)?;
        let unit_data_start = be_u16(bytes, 4)?;
        let normal_start = be_u16(bytes, 6)?;
        Ok(Self {
            unit_count: be_u16(bytes, 0)?,
            unit_data_size,
            unit_data_start,
            normal_start,
            total_size,
            vertex_count,
            normal_count,
            compression,
            vertices: parse_vertices(cluster, vertex_count, compression)?,
            normals: parse_normals(cluster, normal_start, normal_count)?,
            unit_data: checked_cluster_subrange(cluster, unit_data_start, unit_data_size)?.to_vec(),
        })
    }

    /// Decodes the variable unit stream using `ClusteredMesh::GetUnitSize` and
    /// `GetUnitVolumes` semantics from the reference executable.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] for unknown unit types, invalid optional ID
    /// widths, truncated records, or a count/size disagreement.
    pub fn units(&self, mesh: &ClusteredMesh) -> Result<Vec<MeshUnit>, CollisionError> {
        let mut cursor = 0_usize;
        let mut units = Vec::with_capacity(usize::from(self.unit_count));
        for _ in 0..self.unit_count {
            let remaining =
                self.unit_data
                    .get(cursor..)
                    .ok_or(CollisionError::UnitDataSizeMismatch {
                        decoded: cursor,
                        declared: self.unit_data.len(),
                    })?;
            let (unit, size) = parse_unit(remaining, cursor, mesh)?;
            cursor = cursor
                .checked_add(size)
                .ok_or(CollisionError::IntegerOverflow)?;
            units.push(unit);
        }
        if cursor != self.unit_data.len() {
            return Err(CollisionError::UnitDataSizeMismatch {
                decoded: cursor,
                declared: self.unit_data.len(),
            });
        }
        Ok(units)
    }
}

fn append_leaf_triangles(
    mesh: &ClusteredMesh,
    clusters: &[MeshCluster],
    decoded_units: &[Vec<MeshUnit>],
    range: KdLeafRange,
    shift: u32,
    mask: u32,
    triangles: &mut Vec<MeshTriangle>,
) -> Result<(), CollisionError> {
    let mut next_cluster = usize::try_from(range.encoded_start >> shift)
        .map_err(|_| CollisionError::IntegerOverflow)?;
    let unit_offset =
        usize::try_from(range.encoded_start & mask).map_err(|_| CollisionError::IntegerOverflow)?;
    let cluster_units = |cluster_index: usize| {
        let cluster = clusters
            .get(cluster_index)
            .ok_or(CollisionError::EncodedClusterOutsideMesh {
                index: cluster_index,
                cluster_count: clusters.len(),
            })?;
        let units = decoded_units
            .get(cluster_index)
            .ok_or(CollisionError::EncodedClusterOutsideMesh {
                index: cluster_index,
                cluster_count: decoded_units.len(),
            })?;
        Ok::<_, CollisionError>((cluster, units))
    };
    let (_, units) = cluster_units(next_cluster)?;
    let mut unit_index = units
        .binary_search_by_key(&unit_offset, |unit| unit.stream_offset)
        .map_err(|_| CollisionError::EncodedUnitOffsetNotFound {
            cluster: next_cluster,
            offset: unit_offset,
        })?;
    let count = usize::try_from(range.unit_count).map_err(|_| CollisionError::IntegerOverflow)?;
    for _ in 0..count {
        let cluster_index = next_cluster;
        let (cluster, units) = cluster_units(cluster_index)?;
        let unit = units
            .get(unit_index)
            .ok_or(CollisionError::EncodedUnitRangeOutsideCluster {
                cluster: cluster_index,
                first: unit_index,
                end: unit_index + 1,
                unit_count: units.len(),
            })?;
        // `ClusteredMesh::BBoxOverlapQueryThis` 0x82ada530: past a cluster's unit data the range
        // goes on at the next cluster's first unit (two coalesced sibling leaves can span both).
        unit_index += 1;
        if unit_index >= units.len() {
            next_cluster += 1;
            unit_index = 0;
        }
        for indices in unit.triangle_indices() {
            let vertices = indices
                .map(|index| cluster.vertices.get(usize::from(index)))
                .map(|vertex| vertex.map(|vertex| vertex.position(mesh)));
            let [Some(first), Some(second), Some(third)] = vertices else {
                return Err(CollisionError::VertexIndexOutsideCluster {
                    cluster: cluster_index,
                });
            };
            triangles.push(MeshTriangle {
                cluster_index,
                unit_stream_offset: unit.stream_offset,
                vertices: [first, second, third],
                group_id: unit.group_id,
                surface_id: unit.surface_id,
            });
        }
    }
    Ok(())
}

fn parse_unit(
    bytes: &[u8],
    stream_offset: usize,
    mesh: &ClusteredMesh,
) -> Result<(MeshUnit, usize), CollisionError> {
    let flags = *bytes.first().ok_or(CollisionError::Truncated {
        required: 1,
        actual: bytes.len(),
    })?;
    let unit_type = match flags & 0x0f {
        0 => UnitType::OldTriangle,
        1 => UnitType::Triangle,
        2 => UnitType::Quad,
        3 => UnitType::TriangleList,
        value => return Err(CollisionError::UnknownUnitType { value }),
    };
    let (triangle_count, mut cursor) = match unit_type {
        UnitType::OldTriangle | UnitType::Triangle => (1_usize, 1_usize),
        UnitType::Quad => (2, 1),
        UnitType::TriangleList => {
            let count = usize::from(*bytes.get(1).ok_or(CollisionError::Truncated {
                required: 2,
                actual: bytes.len(),
            })?);
            (count, 2)
        }
    };
    let index_count = triangle_count
        .checked_add(2)
        .ok_or(CollisionError::IntegerOverflow)?;
    let vertex_indices = take(bytes, &mut cursor, index_count)?.to_vec();
    let edge_data = if flags & 0x20 != 0 {
        take(bytes, &mut cursor, index_count)?.to_vec()
    } else {
        Vec::new()
    };
    let group_id = if flags & 0x40 != 0 {
        Some(read_optional_id(bytes, &mut cursor, mesh.group_id_size)?)
    } else {
        None
    };
    let surface_id = if flags & 0x80 != 0 {
        Some(read_optional_id(bytes, &mut cursor, mesh.surface_id_size)?)
    } else {
        None
    };
    Ok((
        MeshUnit {
            stream_offset,
            unit_type,
            flags,
            vertex_indices,
            edge_data,
            group_id,
            surface_id,
        },
        cursor,
    ))
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, size: usize) -> Result<&'a [u8], CollisionError> {
    let end = cursor
        .checked_add(size)
        .ok_or(CollisionError::IntegerOverflow)?;
    let result = bytes.get(*cursor..end).ok_or(CollisionError::Truncated {
        required: end,
        actual: bytes.len(),
    })?;
    *cursor = end;
    Ok(result)
}

fn read_optional_id(bytes: &[u8], cursor: &mut usize, size: u8) -> Result<u16, CollisionError> {
    match size {
        1 => Ok(u16::from(take(bytes, cursor, 1)?[0])),
        2 => {
            let value = take(bytes, cursor, 2)?;
            Ok(u16::from_le_bytes(value.try_into().unwrap()))
        }
        _ => Err(CollisionError::InvalidIdSize { size }),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum UnitType {
    OldTriangle = 0,
    Triangle = 1,
    Quad = 2,
    TriangleList = 3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshUnit {
    pub stream_offset: usize,
    pub unit_type: UnitType,
    pub flags: u8,
    pub vertex_indices: Vec<u8>,
    pub edge_data: Vec<u8>,
    pub group_id: Option<u16>,
    pub surface_id: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleHit {
    pub line_parameter: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub barycentric: [f32; 3],
}

/// Performs the one-sided, zero-fatness segment test reconstructed from
/// `rw::collision::ThinTriangleLineSegIntersect` at `0x82acecf0`.
///
/// `line_delta` is `end - start`; the returned parameter is therefore in the
/// inclusive segment interval, with the reference relative tolerance.
#[must_use]
pub fn thin_segment_triangle_intersection(
    line_start: [f32; 3],
    line_delta: [f32; 3],
    vertices: [[f32; 3]; 3],
) -> Option<TriangleHit> {
    let edge_one = subtract(vertices[1], vertices[0]);
    let edge_two = subtract(vertices[2], vertices[0]);
    let determinant = dot(edge_one, cross(line_delta, edge_two));
    if determinant <= 1.0e-8 {
        return None;
    }
    let lower = determinant * -1.0e-5;
    let upper = determinant - lower;
    let from_vertex = subtract(line_start, vertices[0]);
    let first_numerator = dot(from_vertex, cross(line_delta, edge_two));
    if first_numerator < lower || first_numerator > upper {
        return None;
    }
    let perpendicular = cross(from_vertex, edge_one);
    let second_numerator = dot(line_delta, perpendicular);
    if second_numerator < lower || first_numerator + second_numerator > upper {
        return None;
    }
    let line_numerator = dot(edge_two, perpendicular);
    if line_numerator < lower || line_numerator > upper {
        return None;
    }
    let reciprocal = determinant.recip();
    let line_parameter = line_numerator * reciprocal;
    let first = first_numerator * reciprocal;
    let second = second_numerator * reciprocal;
    Some(TriangleHit {
        line_parameter,
        position: add(line_start, scale(line_delta, line_parameter)),
        normal: normalize(cross(edge_one, edge_two)),
        barycentric: [1.0 - first - second, first, second],
    })
}

/// Sweeps a sphere center along a finite segment against a triangle.
///
/// This is the source-level geometric contract recovered from
/// `FatTriangleLineSegIntersect` (`0x82adeb48`) and its
/// `rwcSphereLineSegIntersect` helper (`0x82acdef0`): face, edge, and vertex
/// regions all compete for the earliest fraction, and an initial overlap is
/// reported at fraction zero. As `rw::collision::TriangleLineSegIntersect`
/// (`0x82adf658`) does after the fat test, `position` is the sphere center moved
/// back along `normal` by the radius: the contact point.
#[must_use]
pub fn sphere_segment_triangle_intersection(
    line_start: [f32; 3],
    line_delta: [f32; 3],
    radius: f32,
    vertices: [[f32; 3]; 3],
) -> Option<TriangleHit> {
    if radius == 0.0 {
        return thin_segment_triangle_intersection(line_start, line_delta, vertices);
    }
    if radius < 0.0 {
        return None;
    }

    let radius_squared = radius * radius;
    let (initial_nearest, initial_barycentric) = nearest_point_on_triangle(line_start, vertices);
    let initial_offset = subtract(line_start, initial_nearest);
    if dot(initial_offset, initial_offset) < radius_squared {
        return Some(sphere_triangle_hit(
            0.0,
            line_start,
            line_delta,
            radius,
            vertices,
            initial_nearest,
            initial_barycentric,
        ));
    }

    let edge_one = subtract(vertices[1], vertices[0]);
    let edge_two = subtract(vertices[2], vertices[0]);
    let triangle_normal = normalize(cross(edge_one, edge_two));
    let mut nearest_parameter: Option<f32> = None;

    let normal_speed = dot(line_delta, triangle_normal);
    if normal_speed != 0.0 {
        let start_distance = dot(subtract(line_start, vertices[0]), triangle_normal);
        for target_distance in [radius, -radius] {
            let parameter = (target_distance - start_distance) / normal_speed;
            if !(0.0..=1.0).contains(&parameter) {
                continue;
            }
            let center = add(line_start, scale(line_delta, parameter));
            let projected = subtract(center, scale(triangle_normal, target_distance));
            if point_barycentric(projected, vertices).is_some() {
                retain_nearest_parameter(&mut nearest_parameter, parameter);
            }
        }
    }

    for edge in [
        (vertices[0], vertices[1]),
        (vertices[0], vertices[2]),
        (vertices[1], vertices[2]),
    ] {
        if let Some(parameter) =
            ray_capsule_body_parameter(line_start, line_delta, edge.0, edge.1, radius)
        {
            retain_nearest_parameter(&mut nearest_parameter, parameter);
        }
    }
    for vertex in vertices {
        if let Some(parameter) = ray_sphere_parameter(line_start, line_delta, vertex, radius) {
            retain_nearest_parameter(&mut nearest_parameter, parameter);
        }
    }

    let parameter = nearest_parameter?;
    let center = add(line_start, scale(line_delta, parameter));
    let (nearest, barycentric) = nearest_point_on_triangle(center, vertices);
    Some(sphere_triangle_hit(
        parameter,
        line_start,
        line_delta,
        radius,
        vertices,
        nearest,
        barycentric,
    ))
}

/// Native-ordered separating-axis shape test used when collecting the
/// trajectory predictor's fixed ground-normal triangle cache.
///
/// `Sk8::Physics::AABoxTriangleOverlapTestOpt` at `0x827ace68` constructs the
/// center, extent, translated vertices, and three edges at `0x827ace90` through
/// `0x827aced8`. It then tests the nine edge-cross-box axes in explicit edge/X/Y/Z
/// order through `0x827ad310`, all three box axes at `0x827ad314..0x827ad364`,
/// and the recomputed triangle plane at `0x827ad368..0x827ad3dc`. Exact contact
/// is inclusive: only strict edge/box separation rejects a triangle.
///
/// The radius and normal helpers use [`f32::mul_add`] to retain the native
/// `vmaddfp`/`vnmsubfp` operation graph. The final native `vmsum3fp` has no exact
/// portable Rust equivalent, so its host approximation—and NaN, signed-zero,
/// denormal, infinity, and target-specific fused-rounding behavior—remains a
/// Xenon exceptional-value boundary.
#[must_use]
#[allow(clippy::manual_midpoint)] // Preserve the reference's add-then-half operation order.
pub fn triangle_aabb_overlap(
    minimum: [f32; 3],
    maximum: [f32; 3],
    vertices: [[f32; 3]; 3],
) -> bool {
    let center = std::array::from_fn(|axis| (minimum[axis] + maximum[axis]) * 0.5);
    let extent = std::array::from_fn(|axis| (maximum[axis] - minimum[axis]) * 0.5);
    let translated = vertices.map(|vertex| subtract(vertex, center));
    let edges = [
        subtract(translated[1], translated[0]),
        subtract(translated[2], translated[1]),
        subtract(translated[0], translated[2]),
    ];

    let vertex_0 = translated[0];
    let edge_0 = edges[0];
    let edge_1 = edges[1];

    if edge_axes_separate(translated, edges, extent) {
        return false;
    }

    if box_axes_separate(translated, extent) {
        return false;
    }

    triangle_plane_overlaps_box(vertex_0, edge_0, edge_1, extent)
}

fn retain_nearest_parameter(nearest: &mut Option<f32>, candidate: f32) {
    if nearest.is_none_or(|current| candidate < current) {
        *nearest = Some(candidate);
    }
}

fn sphere_triangle_hit(
    parameter: f32,
    line_start: [f32; 3],
    line_delta: [f32; 3],
    radius: f32,
    vertices: [[f32; 3]; 3],
    nearest: [f32; 3],
    barycentric: [f32; 3],
) -> TriangleHit {
    let center = add(line_start, scale(line_delta, parameter));
    let offset = subtract(center, nearest);
    let normal = if dot(offset, offset) > 0.0 {
        normalize(offset)
    } else {
        let face = normalize(cross(
            subtract(vertices[1], vertices[0]),
            subtract(vertices[2], vertices[0]),
        ));
        if dot(face, line_delta) > 0.0 {
            scale(face, -1.0)
        } else {
            face
        }
    };
    TriangleHit {
        line_parameter: parameter,
        position: subtract(center, scale(normal, radius)),
        normal,
        barycentric,
    }
}

fn ray_sphere_parameter(
    origin: [f32; 3],
    delta: [f32; 3],
    center: [f32; 3],
    radius: f32,
) -> Option<f32> {
    let from_center = subtract(origin, center);
    let a = dot(delta, delta);
    if a <= 0.0 {
        return None;
    }
    let b = dot(from_center, delta);
    let c = dot(from_center, from_center) - radius * radius;
    if c < 0.0 {
        return Some(0.0);
    }
    let discriminant = b.mul_add(b, -(a * c));
    if discriminant < 0.0 || b > 0.0 {
        return None;
    }
    let parameter = (-b - discriminant.sqrt()) / a;
    (0.0..=1.0).contains(&parameter).then_some(parameter)
}

fn ray_capsule_body_parameter(
    origin: [f32; 3],
    delta: [f32; 3],
    edge_start: [f32; 3],
    edge_end: [f32; 3],
    radius: f32,
) -> Option<f32> {
    let edge = subtract(edge_end, edge_start);
    let from_start = subtract(origin, edge_start);
    let edge_squared = dot(edge, edge);
    let delta_squared = dot(delta, delta);
    let edge_delta = dot(edge, delta);
    let edge_origin = dot(edge, from_start);
    let delta_origin = dot(delta, from_start);
    let origin_squared = dot(from_start, from_start);
    let a = edge_squared * delta_squared - edge_delta * edge_delta;
    if a <= 0.0 {
        return None;
    }
    let b = edge_squared * delta_origin - edge_origin * edge_delta;
    let c =
        edge_squared * origin_squared - edge_origin * edge_origin - radius * radius * edge_squared;
    let discriminant = b.mul_add(b, -(a * c));
    if discriminant < 0.0 {
        return None;
    }
    let parameter = (-b - discriminant.sqrt()) / a;
    if !(0.0..=1.0).contains(&parameter) {
        return None;
    }
    let edge_parameter = edge_origin + parameter * edge_delta;
    (edge_parameter > 0.0 && edge_parameter < edge_squared).then_some(parameter)
}

fn point_barycentric(point: [f32; 3], vertices: [[f32; 3]; 3]) -> Option<[f32; 3]> {
    let first_edge = subtract(vertices[1], vertices[0]);
    let second_edge = subtract(vertices[2], vertices[0]);
    let from_vertex = subtract(point, vertices[0]);
    let first_squared = dot(first_edge, first_edge);
    let first_second = dot(first_edge, second_edge);
    let second_squared = dot(second_edge, second_edge);
    let first_point = dot(first_edge, from_vertex);
    let second_point = dot(second_edge, from_vertex);
    let denominator = first_squared * second_squared - first_second * first_second;
    if denominator <= 0.0 {
        return None;
    }
    let reciprocal = denominator.recip();
    let first = (second_squared * first_point - first_second * second_point) * reciprocal;
    let second = (first_squared * second_point - first_second * first_point) * reciprocal;
    let origin = 1.0 - first - second;
    (origin >= 0.0 && first >= 0.0 && second >= 0.0).then_some([origin, first, second])
}

fn nearest_point_on_triangle(point: [f32; 3], vertices: [[f32; 3]; 3]) -> ([f32; 3], [f32; 3]) {
    let first_edge = subtract(vertices[1], vertices[0]);
    let second_edge = subtract(vertices[2], vertices[0]);
    let from_first = subtract(point, vertices[0]);
    let d1 = dot(first_edge, from_first);
    let d2 = dot(second_edge, from_first);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (vertices[0], [1.0, 0.0, 0.0]);
    }

    let from_second = subtract(point, vertices[1]);
    let d3 = dot(first_edge, from_second);
    let d4 = dot(second_edge, from_second);
    if d3 >= 0.0 && d4 <= d3 {
        return (vertices[1], [0.0, 1.0, 0.0]);
    }
    let first_region = d1 * d4 - d3 * d2;
    if first_region <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let first = d1 / (d1 - d3);
        return (
            add(vertices[0], scale(first_edge, first)),
            [1.0 - first, first, 0.0],
        );
    }

    let from_third = subtract(point, vertices[2]);
    let d5 = dot(first_edge, from_third);
    let d6 = dot(second_edge, from_third);
    if d6 >= 0.0 && d5 <= d6 {
        return (vertices[2], [0.0, 0.0, 1.0]);
    }
    let second_region = d5 * d2 - d1 * d6;
    if second_region <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let second = d2 / (d2 - d6);
        return (
            add(vertices[0], scale(second_edge, second)),
            [1.0 - second, 0.0, second],
        );
    }

    let third_region = d3 * d6 - d5 * d4;
    if third_region <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let second = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (
            add(
                vertices[1],
                scale(subtract(vertices[2], vertices[1]), second),
            ),
            [0.0, 1.0 - second, second],
        );
    }

    let reciprocal = (first_region + second_region + third_region).recip();
    let first = second_region * reciprocal;
    let second = first_region * reciprocal;
    (
        add(
            vertices[0],
            add(scale(first_edge, first), scale(second_edge, second)),
        ),
        [1.0 - first - second, first, second],
    )
}

fn edge_axis_projection_x(vertex: [f32; 3], edge: [f32; 3]) -> f32 {
    let positive = edge[2] * vertex[1];
    let negative = edge[1] * vertex[2];
    positive - negative
}

fn edge_axis_projection_y(vertex: [f32; 3], edge: [f32; 3]) -> f32 {
    let positive = edge[0] * vertex[2];
    let negative = edge[2] * vertex[0];
    positive - negative
}

fn edge_axis_projection_z(vertex: [f32; 3], edge: [f32; 3]) -> f32 {
    let positive = edge[1] * vertex[0];
    let negative = edge[0] * vertex[1];
    positive - negative
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EdgeBoxAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EdgeAxisCase {
    edge: usize,
    axis: EdgeBoxAxis,
    vertices: [usize; 2],
}

const NATIVE_EDGE_AXIS_CASES: [EdgeAxisCase; 9] = [
    EdgeAxisCase {
        edge: 0,
        axis: EdgeBoxAxis::X,
        vertices: [0, 2],
    },
    EdgeAxisCase {
        edge: 0,
        axis: EdgeBoxAxis::Y,
        vertices: [0, 2],
    },
    EdgeAxisCase {
        edge: 0,
        axis: EdgeBoxAxis::Z,
        vertices: [1, 2],
    },
    EdgeAxisCase {
        edge: 1,
        axis: EdgeBoxAxis::X,
        vertices: [0, 2],
    },
    EdgeAxisCase {
        edge: 1,
        axis: EdgeBoxAxis::Y,
        vertices: [0, 2],
    },
    EdgeAxisCase {
        edge: 1,
        axis: EdgeBoxAxis::Z,
        vertices: [0, 1],
    },
    EdgeAxisCase {
        edge: 2,
        axis: EdgeBoxAxis::X,
        vertices: [0, 1],
    },
    EdgeAxisCase {
        edge: 2,
        axis: EdgeBoxAxis::Y,
        vertices: [0, 1],
    },
    EdgeAxisCase {
        edge: 2,
        axis: EdgeBoxAxis::Z,
        vertices: [1, 2],
    },
];

fn edge_axes_separate(vertices: [[f32; 3]; 3], edges: [[f32; 3]; 3], extent: [f32; 3]) -> bool {
    NATIVE_EDGE_AXIS_CASES
        .into_iter()
        .any(|case| edge_axis_case_separates(case, vertices, edges, extent))
}

fn edge_axis_case_separates(
    case: EdgeAxisCase,
    vertices: [[f32; 3]; 3],
    edges: [[f32; 3]; 3],
    extent: [f32; 3],
) -> bool {
    let edge = edges[case.edge];
    let first = vertices[case.vertices[0]];
    let second = vertices[case.vertices[1]];
    let (first_projection, second_projection, radius) = match case.axis {
        EdgeBoxAxis::X => (
            edge_axis_projection_x(first, edge),
            edge_axis_projection_x(second, edge),
            edge_axis_radius_x(edge, extent),
        ),
        EdgeBoxAxis::Y => (
            edge_axis_projection_y(first, edge),
            edge_axis_projection_y(second, edge),
            edge_axis_radius_y(edge, extent),
        ),
        EdgeBoxAxis::Z => (
            edge_axis_projection_z(first, edge),
            edge_axis_projection_z(second, edge),
            edge_axis_radius_z(edge, extent),
        ),
    };
    edge_axis_separates(first_projection, second_projection, radius)
}

fn edge_axis_radius_x(edge: [f32; 3], extent: [f32; 3]) -> f32 {
    edge[2].abs().mul_add(extent[1], edge[1].abs() * extent[2])
}

fn edge_axis_radius_y(edge: [f32; 3], extent: [f32; 3]) -> f32 {
    edge[2].abs().mul_add(extent[0], edge[0].abs() * extent[2])
}

fn edge_axis_radius_z(edge: [f32; 3], extent: [f32; 3]) -> f32 {
    edge[1].abs().mul_add(extent[0], edge[0].abs() * extent[1])
}

fn edge_axis_separates(first: f32, second: f32, radius: f32) -> bool {
    let second_is_greater = second > first;
    let minimum = if second_is_greater { first } else { second };
    let maximum = if second_is_greater { second } else { first };
    minimum > radius || -radius > maximum
}

fn box_axes_separate(vertices: [[f32; 3]; 3], extent: [f32; 3]) -> bool {
    let [vertex_0, vertex_1, vertex_2] = vertices;
    let minimum: [f32; 3] =
        std::array::from_fn(|axis| vertex_1[axis].min(vertex_2[axis]).min(vertex_0[axis]));
    let maximum: [f32; 3] =
        std::array::from_fn(|axis| vertex_1[axis].max(vertex_2[axis]).max(vertex_0[axis]));
    -extent[0] > maximum[0]
        || minimum[0] > extent[0]
        || -extent[1] > maximum[1]
        || minimum[1] > extent[1]
        || -extent[2] > maximum[2]
        || minimum[2] > extent[2]
}

fn native_triangle_normal(edge_0: [f32; 3], edge_1: [f32; 3]) -> [f32; 3] {
    let products = [
        edge_0[0] * edge_1[1],
        edge_0[1] * edge_1[2],
        edge_0[2] * edge_1[0],
    ];
    [
        (-edge_0[2]).mul_add(edge_1[1], products[1]),
        (-edge_0[0]).mul_add(edge_1[2], products[2]),
        (-edge_0[1]).mul_add(edge_1[0], products[0]),
    ]
}

fn plane_supports(normal: [f32; 3], extent: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let negative = std::array::from_fn(|axis| {
        if normal[axis] > 0.0 {
            -extent[axis]
        } else {
            extent[axis]
        }
    });
    let positive = std::array::from_fn(|axis| {
        if normal[axis] > 0.0 {
            extent[axis]
        } else {
            -extent[axis]
        }
    });
    (negative, positive)
}

fn triangle_plane_overlaps_box(
    vertex_0: [f32; 3],
    edge_0: [f32; 3],
    edge_1: [f32; 3],
    extent: [f32; 3],
) -> bool {
    let normal = native_triangle_normal(edge_0, edge_1);
    let (negative_support, positive_support) = plane_supports(normal, extent);
    let plane_offset = dot3_vmx_host(normal, vertex_0);
    let negative_plane_offset = -plane_offset;
    let low = dot3_vmx_host(normal, negative_support) + negative_plane_offset;
    let high = dot3_vmx_host(normal, positive_support) + negative_plane_offset;
    0.0 >= low && high >= 0.0
}

fn dot3_vmx_host(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

impl MeshUnit {
    /// Expands triangle and quad units using the winding used by
    /// `ClusteredMesh::GetUnitVolumes`.
    #[must_use]
    pub fn triangle_indices(&self) -> Vec<[u8; 3]> {
        match self.unit_type {
            UnitType::OldTriangle | UnitType::Triangle => {
                vec![[
                    self.vertex_indices[0],
                    self.vertex_indices[1],
                    self.vertex_indices[2],
                ]]
            }
            UnitType::Quad => vec![
                [
                    self.vertex_indices[0],
                    self.vertex_indices[1],
                    self.vertex_indices[2],
                ],
                [
                    self.vertex_indices[3],
                    self.vertex_indices[2],
                    self.vertex_indices[1],
                ],
            ],
            // The stream supplies N+2 indices for an N-triangle strip.
            UnitType::TriangleList => (0..self.vertex_indices.len() - 2)
                .map(|index| {
                    if index % 2 == 0 {
                        [
                            self.vertex_indices[index],
                            self.vertex_indices[index + 1],
                            self.vertex_indices[index + 2],
                        ]
                    } else {
                        [
                            self.vertex_indices[index + 1],
                            self.vertex_indices[index],
                            self.vertex_indices[index + 2],
                        ]
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollisionError {
    Truncated {
        required: usize,
        actual: usize,
    },
    RangeOutsideObject {
        start: usize,
        end: usize,
        object_size: usize,
    },
    UnknownVertexCompression {
        value: u8,
    },
    NoClusters,
    UnknownUnitType {
        value: u8,
    },
    InvalidIdSize {
        size: u8,
    },
    UnitDataSizeMismatch {
        decoded: usize,
        declared: usize,
    },
    InvalidBoundingBox,
    InvalidKdAxis {
        node: usize,
        axis: u32,
    },
    KdBranchOutsideTree {
        index: usize,
        branch_count: usize,
    },
    KdLineStackCapacityExceeded {
        capacity: usize,
        required: usize,
    },
    EncodedClusterOutsideMesh {
        index: usize,
        cluster_count: usize,
    },
    EncodedUnitOffsetNotFound {
        cluster: usize,
        offset: usize,
    },
    EncodedUnitRangeOutsideCluster {
        cluster: usize,
        first: usize,
        end: usize,
        unit_count: usize,
    },
    VertexIndexOutsideCluster {
        cluster: usize,
    },
    KdEntryOutsideMappedArray {
        start: usize,
        end: usize,
        volume_count: usize,
    },
    IntegerOverflow,
}

impl fmt::Display for CollisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 collision object: {self:?}")
    }
}

impl std::error::Error for CollisionError {}

fn require_header(bytes: &[u8]) -> Result<(), CollisionError> {
    require_size(bytes, COLLISION_HEADER_SIZE)
}

fn inline_volumes(
    bytes: &[u8],
    volumes_offset: u32,
    volume_count: u32,
) -> Result<Vec<Volume>, CollisionError> {
    let start = usize::try_from(volumes_offset).map_err(|_| CollisionError::IntegerOverflow)?;
    let count = usize::try_from(volume_count).map_err(|_| CollisionError::IntegerOverflow)?;
    let size = count
        .checked_mul(COLLISION_HEADER_SIZE)
        .ok_or(CollisionError::IntegerOverflow)?;
    let end = start
        .checked_add(size)
        .ok_or(CollisionError::IntegerOverflow)?;
    let allocation = bytes
        .get(start..end)
        .ok_or(CollisionError::RangeOutsideObject {
            start,
            end,
            object_size: bytes.len(),
        })?;
    (0..count)
        .map(|index| Volume::parse(&allocation[index * COLLISION_HEADER_SIZE..]))
        .collect()
}

fn process_kd_child(
    node: &BranchNode,
    child: usize,
    branch_stack: &mut Vec<u32>,
    encoded_start: &mut u32,
    unit_count: &mut u32,
) {
    let content = node.child_content[child];
    let index = node.child_index[child];
    if content == u32::MAX {
        branch_stack.push(index);
    } else {
        // The reference stores the high count, adds the low count with
        // unsigned machine arithmetic, and lets the low leaf overwrite the
        // start even when its count is zero.
        *unit_count = if child == 1 {
            content
        } else {
            unit_count.wrapping_add(content)
        };
        *encoded_start = index;
    }
}

fn collect_kd_line_ranges(
    mut query: KdTreeLineRangeQuery<'_>,
) -> Result<Vec<KdLeafRange>, CollisionError> {
    let mut ranges = Vec::new();
    while let Some(range) = query.next_range()? {
        ranges.push(range);
    }
    Ok(ranges)
}

fn clear_sign(value: f32) -> f32 {
    f32::from_bits(value.to_bits() & 0x7fff_ffff)
}

fn vmx_max_host(left: f32, right: f32) -> f32 {
    // The native instruction is `vmaxfp`; exceptional operand bit parity is
    // intentionally left to the documented Xenon hardware oracle.
    left.max(right)
}

fn vmx_min_host(left: f32, right: f32) -> f32 {
    // The native instruction is `vminfp`; see `vmx_max_host`.
    left.min(right)
}

fn max_subsel(first: f32, second: f32) -> f32 {
    let selector = first - second;
    if selector >= 0.0 { first } else { second }
}

fn min_subsel(first: f32, second: f32) -> f32 {
    let selector = first - second;
    if selector >= 0.0 { second } else { first }
}

fn refined_host_reciprocal(value: f32) -> f32 {
    // Native starts with `vrefp`; host division is the explicit bit-parity
    // boundary, after which these are the two recovered Newton steps.
    let estimate0 = value.recip();
    let residual0 = (-estimate0).mul_add(value, 1.0);
    let estimate1 = estimate0.mul_add(residual0, estimate0);
    let residual1 = (-estimate1).mul_add(value, 1.0);
    estimate1.mul_add(residual1, estimate1)
}

fn require_size(bytes: &[u8], required: usize) -> Result<(), CollisionError> {
    if bytes.len() < required {
        Err(CollisionError::Truncated {
            required,
            actual: bytes.len(),
        })
    } else {
        Ok(())
    }
}

fn offset_slice(bytes: &[u8], offset: u32) -> Result<&[u8], CollisionError> {
    let start = usize::try_from(offset).map_err(|_| CollisionError::IntegerOverflow)?;
    bytes
        .get(start..)
        .ok_or(CollisionError::RangeOutsideObject {
            start,
            end: start,
            object_size: bytes.len(),
        })
}

fn checked_cluster_subrange(bytes: &[u8], offset: u16, size: u16) -> Result<&[u8], CollisionError> {
    let start = 16_usize
        .checked_add(usize::from(offset) * 16)
        .ok_or(CollisionError::IntegerOverflow)?;
    let end = start
        .checked_add(usize::from(size))
        .ok_or(CollisionError::IntegerOverflow)?;
    bytes
        .get(start..end)
        .ok_or(CollisionError::RangeOutsideObject {
            start,
            end,
            object_size: bytes.len(),
        })
}

fn parse_vertices(
    bytes: &[u8],
    count: u8,
    compression: VertexCompression,
) -> Result<Vec<ClusterVertex>, CollisionError> {
    let (start, stride) = match compression {
        VertexCompression::Uncompressed => (16_usize, 16_usize),
        // The signed quantized origin is three i32 lanes at 16..28; packed
        // u16 vertices immediately follow at byte 28 (confirmed by GetVertex).
        VertexCompression::Unsigned16 => (28, 6),
        VertexCompression::Signed32 => (16, 12),
    };
    let end = start
        .checked_add(usize::from(count) * stride)
        .ok_or(CollisionError::IntegerOverflow)?;
    let data = bytes
        .get(start..end)
        .ok_or(CollisionError::RangeOutsideObject {
            start,
            end,
            object_size: bytes.len(),
        })?;
    let quantized_origin = if compression == VertexCompression::Unsigned16 {
        [be_i32(bytes, 16)?, be_i32(bytes, 20)?, be_i32(bytes, 24)?]
    } else {
        [0; 3]
    };
    (0..usize::from(count))
        .map(|index| parse_vertex(&data[index * stride..], compression, quantized_origin))
        .collect()
}

fn parse_vertex(
    bytes: &[u8],
    compression: VertexCompression,
    quantized_origin: [i32; 3],
) -> Result<ClusterVertex, CollisionError> {
    Ok(match compression {
        VertexCompression::Uncompressed => {
            ClusterVertex::Uncompressed(be_f32_array::<4>(bytes, 0)?)
        }
        VertexCompression::Unsigned16 => ClusterVertex::Unsigned16 {
            quantized_origin,
            value: [be_u16(bytes, 0)?, be_u16(bytes, 2)?, be_u16(bytes, 4)?],
        },
        VertexCompression::Signed32 => {
            ClusterVertex::Signed32([be_i32(bytes, 0)?, be_i32(bytes, 4)?, be_i32(bytes, 8)?])
        }
    })
}

fn parse_normals(bytes: &[u8], offset: u16, count: u8) -> Result<Vec<[f32; 4]>, CollisionError> {
    let start = 16_usize
        .checked_add(usize::from(offset) * 16)
        .ok_or(CollisionError::IntegerOverflow)?;
    let end = start
        .checked_add(usize::from(count) * 16)
        .ok_or(CollisionError::IntegerOverflow)?;
    let data = bytes
        .get(start..end)
        .ok_or(CollisionError::RangeOutsideObject {
            start,
            end,
            object_size: bytes.len(),
        })?;
    (0..usize::from(count))
        .map(|index| be_f32_array::<4>(data, index * 16))
        .collect()
}

fn be_u16(bytes: &[u8], offset: usize) -> Result<u16, CollisionError> {
    bytes
        .get(offset..offset + 2)
        .map(|field| u16::from_be_bytes(field.try_into().unwrap()))
        .ok_or(CollisionError::Truncated {
            required: offset + 2,
            actual: bytes.len(),
        })
}

fn subtract(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|index| left[index] - right[index])
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|index| left[index] + right[index])
}

fn scale(value: [f32; 3], scalar: f32) -> [f32; 3] {
    value.map(|component| component * scalar)
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn normalize(value: [f32; 3]) -> [f32; 3] {
    let length = dot(value, value).sqrt();
    if length > 0.0 {
        scale(value, length.recip())
    } else {
        [0.0; 3]
    }
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, CollisionError> {
    bytes
        .get(offset..offset + 4)
        .map(|field| u32::from_be_bytes(field.try_into().unwrap()))
        .ok_or(CollisionError::Truncated {
            required: offset + 4,
            actual: bytes.len(),
        })
}

fn be_i32(bytes: &[u8], offset: usize) -> Result<i32, CollisionError> {
    Ok(i32::from_be_bytes(be_u32(bytes, offset)?.to_be_bytes()))
}

fn be_f32(bytes: &[u8], offset: usize) -> Result<f32, CollisionError> {
    Ok(f32::from_bits(be_u32(bytes, offset)?))
}

fn be_f32_array<const SIZE: usize>(
    bytes: &[u8],
    offset: usize,
) -> Result<[f32; SIZE], CollisionError> {
    let mut result = [0.0; SIZE];
    for (index, value) in result.iter_mut().enumerate() {
        *value = be_f32(bytes, offset + index * 4)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_reference_collision_object_enum() {
        assert_eq!(
            ObjectType::from_type_id(0x0008_0001),
            Some(ObjectType::Volume)
        );
        assert_eq!(
            ObjectType::from_type_id(0x0008_0006),
            Some(ObjectType::ClusteredMesh)
        );
        assert_eq!(ObjectType::from_type_id(0x00eb_0008), None);
    }

    #[test]
    fn parses_volume_header() {
        let mut bytes = [0_u8; COLLISION_HEADER_SIZE];
        bytes[60..64].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        bytes[64..68].copy_from_slice(&0x1234_u32.to_be_bytes());
        bytes[80..84].copy_from_slice(&0.25_f32.to_bits().to_be_bytes());
        bytes[84..88].copy_from_slice(&7_u32.to_be_bytes());
        bytes[88..92].copy_from_slice(&11_u32.to_be_bytes());

        let volume = Volume::parse(&bytes).unwrap();
        assert!((volume.transform[15] - 1.0).abs() < f32::EPSILON);
        assert_eq!(volume.vtable_offset, 0x1234);
        assert!((volume.radius - 0.25).abs() < f32::EPSILON);
        assert_eq!(volume.group_id, 7);
        assert_eq!(volume.surface_id, 11);
    }

    #[test]
    fn parses_bit_table_header_and_stored_word_count_without_dimension_policy() {
        let words = [2_u32, 33, 3, 0x8000_0001, 0x0123_4567, 0xfedc_ba98];
        let bytes = words
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>();
        let table = BitTable::parse(&bytes).unwrap();
        assert_eq!(table.rows, 2);
        assert_eq!(table.columns, 33);
        assert_eq!(table.array_size, 3);
        assert_eq!(table.words, words[3..]);

        let mut inconsistent = bytes;
        inconsistent[0..4].copy_from_slice(&0xffff_ffff_u32.to_be_bytes());
        inconsistent[4..8].copy_from_slice(&0xffff_fffe_u32.to_be_bytes());
        let retained = BitTable::parse(&inconsistent).unwrap();
        assert_eq!(retained.rows, u32::MAX);
        assert_eq!(retained.columns, u32::MAX - 1);
        assert_eq!(retained.array_size, 3);
    }

    #[test]
    fn rejects_truncated_bit_table_words() {
        let bytes = [
            1_u32.to_be_bytes(),
            1_u32.to_be_bytes(),
            2_u32.to_be_bytes(),
            0xaaaa_5555_u32.to_be_bytes(),
        ]
        .concat();
        assert_eq!(
            BitTable::parse(&bytes),
            Err(CollisionError::Truncated {
                required: 20,
                actual: 16,
            })
        );
    }

    #[test]
    fn parses_assembly_definition_and_part_collision_link() {
        let mut bytes = vec![0_u8; ASSEMBLY_DEFINITION_SIZE + PART_DEFINITION_SIZE];
        bytes[0..4].copy_from_slice(&48_u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&1_u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&ObjectId(7).0.to_be_bytes());
        let part = ASSEMBLY_DEFINITION_SIZE;
        bytes[part..part + 4].copy_from_slice(&ObjectId(11).0.to_be_bytes());
        for (index, bits) in [
            0x8000_0000_u32,
            0x7fc1_2345,
            0xffc5_4321,
            0x3f80_0000,
            0x3f00_0000,
            0xbf00_0000,
            0x7f7f_ffff,
            0xff7f_ffff,
            0x3dca_feba,
            0xbdca_feba,
        ]
        .into_iter()
        .enumerate()
        {
            let offset = part + 16 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&bits.to_be_bytes());
        }
        for (index, value) in [
            1.0_f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 4.0, 1.0, 1.0,
        ]
        .into_iter()
        .enumerate()
        {
            let offset = part + 64 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        }
        bytes[part + 140..part + 144].copy_from_slice(&1_u32.to_be_bytes());
        bytes[part + 144..part + 148].copy_from_slice(&0x89ab_cdef_u32.to_be_bytes());
        for (index, bits) in [0x8000_0000_u32, 0x7fc9_abcd, 0x3e4c_cccd]
            .into_iter()
            .enumerate()
        {
            let offset = part + 128 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&bits.to_be_bytes());
        }

        let assembly = AssemblyDefinition::parse(&bytes).unwrap();
        assert_eq!(assembly.part_count, 1);
        assert_eq!(assembly.culling_table, Some(ObjectId(7)));
        assert_eq!(assembly.parts[0].volume, ObjectId(11));
        assert_eq!(
            assembly.parts[0].inertia.inverse_tensor.map(f32::to_bits),
            [0x8000_0000, 0x7fc1_2345, 0xffc5_4321, 0x3f80_0000]
        );
        assert_eq!(
            assembly.parts[0].inertia.inverse_mass.to_bits(),
            0x3f00_0000
        );
        assert_eq!(assembly.parts[0].inertia.spherical.to_bits(), 0xbf00_0000);
        assert_eq!(
            assembly.parts[0].inertia.max_velocity.to_bits(),
            0x7f7f_ffff
        );
        assert_eq!(assembly.parts[0].inertia.max_omega.to_bits(), 0xff7f_ffff);
        assert_eq!(assembly.parts[0].inertia.linear_drag.to_bits(), 0x3dca_feba);
        assert_eq!(
            assembly.parts[0].inertia.angular_drag.to_bits(),
            0xbdca_feba
        );
        assert_eq!(
            assembly.parts[0].inverse_body_transform[12..15],
            [3.0, 4.0, 1.0]
        );
        assert_eq!(
            [
                assembly.parts[0].material.static_friction.to_bits(),
                assembly.parts[0].material.dynamic_friction.to_bits(),
                assembly.parts[0].material.restitution.to_bits(),
            ],
            [0x8000_0000, 0x7fc9_abcd, 0x3e4c_cccd]
        );
        assert_eq!(assembly.parts[0].has_inverse_body_transform, 1);
        assert_eq!(assembly.parts[0].is_static_body, 0x89ab_cdef);
    }

    #[test]
    fn parses_runtime_assembly_definition_link() {
        let mut bytes = [0_u8; ASSEMBLY_SIZE];
        bytes[0..4].copy_from_slice(&ObjectId(5).0.to_be_bytes());
        bytes[4..8].copy_from_slice(&ObjectId(7).0.to_be_bytes());
        bytes[28..32].copy_from_slice(&2_u32.to_be_bytes());
        bytes[44..46].copy_from_slice(&3_u16.to_be_bytes());
        bytes[46..48].copy_from_slice(&4_u16.to_be_bytes());

        let assembly = Assembly::parse(&bytes).unwrap();

        assert_eq!(assembly.culling_table, Some(ObjectId(5)));
        assert_eq!(assembly.definition, ObjectId(7));
        assert_eq!(assembly.part_count, 2);
        assert_eq!(assembly.flags, 3);
        assert_eq!(assembly.user_flags, 4);
    }

    #[test]
    fn resolves_assembly_part_allocation_before_dictionary_object() {
        let object = PART_DEFINITION_SIZE;
        let object_offset = u32::try_from(object).unwrap();
        let mut resource = vec![0_u8; object + ASSEMBLY_DEFINITION_SIZE];
        resource[object..object + 4]
            .copy_from_slice(&0_u32.wrapping_sub(object_offset).to_be_bytes());
        resource[object + 4..object + 8].copy_from_slice(&1_u32.to_be_bytes());
        resource[0..4].copy_from_slice(&ObjectId(23).0.to_be_bytes());

        let assembly = AssemblyDefinition::parse_in_resource(&resource, object_offset).unwrap();

        assert_eq!(assembly.parts[0].volume, ObjectId(23));
    }

    fn primitive_volume(volume_type: VolumeType) -> Volume {
        Volume {
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            vtable_offset: volume_type as u32,
            shape_words: [0; 3],
            radius: 0.0,
            group_id: 0,
            surface_id: 0,
            flags: 0,
        }
    }

    fn assert_axis_hit(hit: PrimitiveHit) {
        assert!((hit.line_parameter - 0.25).abs() < f32::EPSILON);
        for (actual, expected) in hit.position.into_iter().zip([-1.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
        for (actual, expected) in hit.normal.into_iter().zip([-1.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn intersects_serialized_primitive_volumes() {
        let mut sphere = primitive_volume(VolumeType::Sphere);
        sphere.radius = 1.0;
        assert_axis_hit(
            sphere
                .thin_segment_intersection([-2.0, 0.0, 0.0], [4.0, 0.0, 0.0])
                .unwrap(),
        );

        let mut oriented_box = primitive_volume(VolumeType::Box);
        oriented_box.shape_words = [1.0_f32.to_bits(); 3];
        assert_axis_hit(
            oriented_box
                .thin_segment_intersection([-2.0, 0.0, 0.0], [4.0, 0.0, 0.0])
                .unwrap(),
        );

        let mut cylinder = primitive_volume(VolumeType::Cylinder);
        cylinder.shape_words = [1.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        assert_axis_hit(
            cylinder
                .thin_segment_intersection([-2.0, 0.0, 0.0], [4.0, 0.0, 0.0])
                .unwrap(),
        );
    }

    #[test]
    fn expands_spheres_and_rounded_boxes_for_fat_segments() {
        let mut sphere = primitive_volume(VolumeType::Sphere);
        sphere.radius = 1.0;
        let sphere_hit = sphere
            .segment_intersection_with_fatness([-3.0, 0.0, 0.0], [6.0, 0.0, 0.0], 0.5)
            .unwrap();
        assert!((sphere_hit.line_parameter - 0.25).abs() < f32::EPSILON);
        for (actual, expected) in sphere_hit.position.into_iter().zip([-1.5, 0.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
        for (actual, expected) in sphere_hit.normal.into_iter().zip([-1.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }

        let mut rounded_box = primitive_volume(VolumeType::Box);
        rounded_box.shape_words = [1.0_f32.to_bits(); 3];
        let face_hit = rounded_box
            .segment_intersection_with_fatness([-3.0, 0.0, 0.0], [6.0, 0.0, 0.0], 0.5)
            .unwrap();
        assert!((face_hit.line_parameter - 0.25).abs() < f32::EPSILON);
        for (actual, expected) in face_hit.position.into_iter().zip([-1.5, 0.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
        for (actual, expected) in face_hit.normal.into_iter().zip([-1.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }

        let edge_hit = rounded_box
            .segment_intersection_with_fatness([-3.0, 1.5, 0.0], [6.0, 0.0, 0.0], 0.5)
            .unwrap();
        assert!((edge_hit.line_parameter - (1.0 / 3.0)).abs() < 1.0e-6);
        for (actual, expected) in edge_hit.normal.into_iter().zip([0.0, 1.0, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn expands_cylinder_side_cap_and_toroidal_rim_for_fat_segments() {
        let mut cylinder = primitive_volume(VolumeType::Cylinder);
        cylinder.shape_words = [1.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        let side = cylinder
            .segment_intersection_with_fatness([-3.0, 0.0, 0.0], [6.0, 0.0, 0.0], 0.5)
            .unwrap();
        assert!((side.line_parameter - 0.25).abs() < 1.0e-6);
        for (actual, expected) in side.normal.into_iter().zip([-1.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }

        let cap = cylinder
            .segment_intersection_with_fatness([0.0, 0.0, 3.0], [0.0, 0.0, -6.0], 0.5)
            .unwrap();
        assert!((cap.line_parameter - 0.25).abs() < 1.0e-6);
        for (actual, expected) in cap.normal.into_iter().zip([0.0, 0.0, 1.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }

        let rim = cylinder
            .segment_intersection_with_fatness([-3.0, 0.0, 1.3], [6.0, 0.0, 0.0], 0.5)
            .unwrap();
        assert!((rim.line_parameter - (1.6 / 6.0)).abs() < 1.0e-4);
        for (actual, expected) in rim.normal.into_iter().zip([-0.8, 0.0, 0.6]) {
            assert!((actual - expected).abs() < 1.0e-3);
        }
    }

    #[test]
    fn parses_clustered_mesh_header() {
        let mut bytes = [0_u8; COLLISION_HEADER_SIZE];
        bytes[48..52].copy_from_slice(&0x60_u32.to_be_bytes());
        bytes[52..56].copy_from_slice(&0x80_u32.to_be_bytes());
        bytes[56..60].copy_from_slice(&0.01_f32.to_bits().to_be_bytes());
        bytes[60..62].copy_from_slice(&5_u16.to_be_bytes());
        bytes[62] = 2;
        bytes[63] = 1;
        bytes[64..68].copy_from_slice(&3_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&42_u32.to_be_bytes());
        bytes[80..84].copy_from_slice(&0x160_u32.to_be_bytes());

        let mesh = ClusteredMesh::parse(&bytes).unwrap();
        assert_eq!(mesh.kd_tree_offset, 0x60);
        assert_eq!(mesh.cluster_data_offset, 0x80);
        assert_eq!(mesh.group_id_size, 2);
        assert_eq!(mesh.cluster_count, 3);
        assert_eq!(mesh.unit_count, 42);
        assert_eq!(mesh.allocation_size, 0x160);
    }

    #[test]
    fn parses_compressed_cluster_vertices_and_unit_stream() {
        let mut bytes = [0_u8; 70];
        bytes[0..2].copy_from_slice(&1_u16.to_be_bytes());
        bytes[2..4].copy_from_slice(&6_u16.to_be_bytes());
        bytes[4..6].copy_from_slice(&3_u16.to_be_bytes());
        bytes[6..8].copy_from_slice(&3_u16.to_be_bytes());
        bytes[8..10].copy_from_slice(&70_u16.to_be_bytes());
        bytes[10] = 2;
        bytes[12] = VertexCompression::Unsigned16 as u8;
        for (index, value) in [100_i32, 200, 300].into_iter().enumerate() {
            let offset = 16 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        for (index, value) in [1_u16, 2, 3, 4, 5, 6].into_iter().enumerate() {
            let offset = 28 + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
        }
        bytes[64..70].copy_from_slice(&[0x81, 0, 1, 1, 0x37, 0x03]);

        let cluster = MeshCluster::parse(&bytes).unwrap();
        assert_eq!(cluster.vertices.len(), 2);
        assert_eq!(
            cluster.vertices[0],
            ClusterVertex::Unsigned16 {
                quantized_origin: [100, 200, 300],
                value: [1, 2, 3]
            }
        );
        assert_eq!(cluster.unit_data, [0x81, 0, 1, 1, 0x37, 0x03]);

        let mut mesh_bytes = [0_u8; COLLISION_HEADER_SIZE];
        mesh_bytes[63] = 2;
        let mesh = ClusteredMesh::parse(&mesh_bytes).unwrap();
        let units = cluster.units(&mesh).unwrap();
        assert_eq!(units[0].surface_id, Some(0x0337));
        assert_eq!(units[0].triangle_indices(), [[0, 1, 1]]);
    }

    #[test]
    fn unsigned16_vertex_adds_in_signed_integer_domain_before_float_conversion() {
        let mut mesh_bytes = [0_u8; COLLISION_HEADER_SIZE];
        mesh_bytes[56..60].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        let mesh = ClusteredMesh::parse(&mesh_bytes).unwrap();
        let vertex = ClusterVertex::Unsigned16 {
            quantized_origin: [16_777_217, -16_777_217, i32::MIN],
            value: [1, 1, 1],
        };

        let position = vertex.position(&mesh);

        assert_eq!(
            position.map(f32::to_bits),
            [0x4b80_0001, 0xcb80_0000, 0xcf00_0000]
        );
    }

    #[test]
    fn unsigned16_vertex_saturates_the_signed_integer_addition() {
        let mut mesh_bytes = [0_u8; COLLISION_HEADER_SIZE];
        mesh_bytes[56..60].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        let mesh = ClusteredMesh::parse(&mesh_bytes).unwrap();
        let vertex = ClusterVertex::Unsigned16 {
            quantized_origin: [i32::MAX - 127; 3],
            value: [u16::MAX; 3],
        };

        assert_eq!(vertex.position(&mesh).map(f32::to_bits), [0x4f00_0000; 3]);
    }

    fn write_kd_branch(
        bytes: &mut [u8],
        node_index: usize,
        axis: u32,
        low: (u32, u32),
        high: (u32, u32),
        extents: [f32; 2],
    ) {
        write_kd_branch_at(bytes, 0x40, node_index, axis, low, high, extents);
    }

    fn write_kd_branch_at(
        bytes: &mut [u8],
        branch_base: usize,
        node_index: usize,
        axis: u32,
        low: (u32, u32),
        high: (u32, u32),
        extents: [f32; 2],
    ) {
        let start = branch_base + node_index * 32;
        let node = &mut bytes[start..start + 32];
        node[4..8].copy_from_slice(&axis.to_be_bytes());
        node[8..12].copy_from_slice(&low.0.to_be_bytes());
        node[12..16].copy_from_slice(&low.1.to_be_bytes());
        node[16..20].copy_from_slice(&high.0.to_be_bytes());
        node[20..24].copy_from_slice(&high.1.to_be_bytes());
        node[24..28].copy_from_slice(&extents[0].to_bits().to_be_bytes());
        node[28..32].copy_from_slice(&extents[1].to_bits().to_be_bytes());
    }

    fn write_f32_words(bytes: &mut [u8], start: usize, values: [f32; 4]) {
        for (index, value) in values.into_iter().enumerate() {
            let offset = start + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        }
    }

    type KdNodeSpec = (u32, (u32, u32), (u32, u32), [f32; 2]);

    fn mapped_array_with_kd(
        volume_count: u32,
        entry_count: u32,
        node: Option<KdNodeSpec>,
    ) -> (KdTreeMappedArray, Vec<u8>) {
        const KD_OFFSET: usize = 0x80;
        const BRANCH_OFFSET: usize = 0xb0;
        const VOLUMES_OFFSET: usize = 0xd0;
        let count = usize::try_from(volume_count).unwrap();
        let mut bytes = vec![0_u8; VOLUMES_OFFSET + count * COLLISION_HEADER_SIZE];
        write_f32_words(&mut bytes, 0, [-2.0, -2.0, -2.0, 0.0]);
        write_f32_words(&mut bytes, 16, [2.0, 2.0, 2.0, 0.0]);
        bytes[40..44].copy_from_slice(&volume_count.to_be_bytes());
        bytes[48..52].copy_from_slice(&u32::try_from(VOLUMES_OFFSET).unwrap().to_be_bytes());
        bytes[64..68].copy_from_slice(&u32::try_from(KD_OFFSET).unwrap().to_be_bytes());

        bytes[KD_OFFSET..KD_OFFSET + 4]
            .copy_from_slice(&u32::try_from(BRANCH_OFFSET).unwrap().to_be_bytes());
        bytes[KD_OFFSET + 4..KD_OFFSET + 8]
            .copy_from_slice(&u32::from(node.is_some()).to_be_bytes());
        bytes[KD_OFFSET + 8..KD_OFFSET + 12].copy_from_slice(&entry_count.to_be_bytes());
        write_f32_words(&mut bytes, KD_OFFSET + 16, [-2.0, -2.0, -2.0, 0.0]);
        write_f32_words(&mut bytes, KD_OFFSET + 32, [2.0, 2.0, 2.0, 0.0]);
        if let Some((axis, low, high, extents)) = node {
            write_kd_branch_at(&mut bytes, BRANCH_OFFSET, 0, axis, low, high, extents);
        }
        (KdTreeMappedArray::parse(&bytes).unwrap(), bytes)
    }

    fn clustered_mesh_with_kd(
        triangles: &[[[f32; 3]; 3]],
        axis: u32,
        low: (u32, u32),
        high: (u32, u32),
        extents: [f32; 2],
    ) -> (ClusteredMesh, Vec<u8>) {
        const CLUSTER_TABLE_OFFSET: usize = 0x60;
        const KD_OFFSET: usize = 0x80;
        const BRANCH_OFFSET: usize = 0xb0;
        const CLUSTER_OFFSET: usize = 0x100;
        let triangle_count = triangles.len();
        let vertex_count = triangle_count * 3;
        let unit_data_offset = 16 + vertex_count * 16;
        let unit_data_size = triangle_count * 4;
        let cluster_size = unit_data_offset + unit_data_size;
        let mut bytes = vec![0_u8; CLUSTER_OFFSET + cluster_size];

        write_f32_words(&mut bytes, 0, [-2.0, -2.0, -2.0, 0.0]);
        write_f32_words(&mut bytes, 16, [2.0, 2.0, 2.0, 0.0]);
        bytes[40..44].copy_from_slice(&u32::try_from(triangle_count).unwrap().to_be_bytes());
        bytes[48..52].copy_from_slice(&u32::try_from(KD_OFFSET).unwrap().to_be_bytes());
        bytes[52..56].copy_from_slice(&u32::try_from(CLUSTER_TABLE_OFFSET).unwrap().to_be_bytes());
        bytes[56..60].copy_from_slice(&1.0_f32.to_bits().to_be_bytes());
        bytes[64..68].copy_from_slice(&1_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&1_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&u32::try_from(triangle_count).unwrap().to_be_bytes());
        bytes[76..80].copy_from_slice(&u32::try_from(triangle_count).unwrap().to_be_bytes());
        let allocation_size = u32::try_from(bytes.len()).unwrap();
        bytes[80..84].copy_from_slice(&allocation_size.to_be_bytes());
        bytes[CLUSTER_TABLE_OFFSET..CLUSTER_TABLE_OFFSET + 4]
            .copy_from_slice(&u32::try_from(CLUSTER_OFFSET).unwrap().to_be_bytes());

        bytes[KD_OFFSET..KD_OFFSET + 4]
            .copy_from_slice(&u32::try_from(BRANCH_OFFSET).unwrap().to_be_bytes());
        bytes[KD_OFFSET + 4..KD_OFFSET + 8].copy_from_slice(&1_u32.to_be_bytes());
        bytes[KD_OFFSET + 8..KD_OFFSET + 12]
            .copy_from_slice(&u32::try_from(triangle_count).unwrap().to_be_bytes());
        write_f32_words(&mut bytes, KD_OFFSET + 16, [-2.0, -2.0, -2.0, 0.0]);
        write_f32_words(&mut bytes, KD_OFFSET + 32, [2.0, 2.0, 2.0, 0.0]);
        write_kd_branch_at(&mut bytes, BRANCH_OFFSET, 0, axis, low, high, extents);

        let cluster = &mut bytes[CLUSTER_OFFSET..];
        cluster[0..2].copy_from_slice(&u16::try_from(triangle_count).unwrap().to_be_bytes());
        cluster[2..4].copy_from_slice(&u16::try_from(unit_data_size).unwrap().to_be_bytes());
        cluster[4..6].copy_from_slice(&u16::try_from(vertex_count).unwrap().to_be_bytes());
        cluster[6..8].copy_from_slice(&u16::try_from(vertex_count).unwrap().to_be_bytes());
        cluster[8..10].copy_from_slice(&u16::try_from(cluster_size).unwrap().to_be_bytes());
        cluster[10] = u8::try_from(vertex_count).unwrap();
        cluster[11] = 0;
        cluster[12] = VertexCompression::Uncompressed as u8;
        for (triangle_index, triangle) in triangles.iter().enumerate() {
            for (local_index, vertex) in triangle.iter().enumerate() {
                let vertex_index = triangle_index * 3 + local_index;
                let vertex_offset = 16 + vertex_index * 16;
                write_f32_words(
                    cluster,
                    vertex_offset,
                    [vertex[0], vertex[1], vertex[2], 0.0],
                );
            }
            let unit_offset = unit_data_offset + triangle_index * 4;
            let first_vertex = u8::try_from(triangle_index * 3).unwrap();
            cluster[unit_offset..unit_offset + 4].copy_from_slice(&[
                UnitType::Triangle as u8,
                first_vertex,
                first_vertex + 1,
                first_vertex + 2,
            ]);
        }

        (ClusteredMesh::parse(&bytes).unwrap(), bytes)
    }

    fn clustered_mesh_for_selected_unit_visits() -> (ClusteredMesh, Vec<u8>) {
        const CLUSTER_TABLE_OFFSET: usize = 0x60;
        const SELECTED_CLUSTER_OFFSET: usize = 0x80;
        let units = [
            &[0x01, 0, 1, 2][..],
            &[0x62, 3, 4, 5, 6, 10, 11, 12, 13, 0x7a][..],
            &[0x03, 3, 7, 8, 9, 10, 11][..],
            &[0x81, 12, 13, 14, 0x34, 0x12][..],
        ]
        .concat();
        let cluster_size = 16 + units.len();
        let mut bytes = vec![0_u8; SELECTED_CLUSTER_OFFSET + cluster_size];
        bytes[52..56].copy_from_slice(&u32::try_from(CLUSTER_TABLE_OFFSET).unwrap().to_be_bytes());
        bytes[62] = 1;
        bytes[63] = 2;
        bytes[64..68].copy_from_slice(&2_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&2_u32.to_be_bytes());
        bytes[CLUSTER_TABLE_OFFSET..CLUSTER_TABLE_OFFSET + 4]
            .copy_from_slice(&u32::MAX.to_be_bytes());
        bytes[CLUSTER_TABLE_OFFSET + 4..CLUSTER_TABLE_OFFSET + 8].copy_from_slice(
            &u32::try_from(SELECTED_CLUSTER_OFFSET)
                .unwrap()
                .to_be_bytes(),
        );

        let cluster = &mut bytes[SELECTED_CLUSTER_OFFSET..];
        cluster[0..2].copy_from_slice(&4_u16.to_be_bytes());
        cluster[2..4].copy_from_slice(&u16::try_from(units.len()).unwrap().to_be_bytes());
        cluster[4..6].copy_from_slice(&0_u16.to_be_bytes());
        cluster[6..8].copy_from_slice(&0_u16.to_be_bytes());
        cluster[8..10].copy_from_slice(&u16::try_from(cluster_size).unwrap().to_be_bytes());
        cluster[12] = VertexCompression::Uncompressed as u8;
        cluster[16..].copy_from_slice(&units);

        (ClusteredMesh::parse(&bytes).unwrap(), bytes)
    }

    fn selected_unit_range(unit_offset: u32, unit_count: u32) -> KdLeafRange {
        KdLeafRange {
            encoded_start: (1_u32 << 16) | unit_offset,
            unit_count,
        }
    }

    fn kd_tree(branch_node_count: u32, entry_count: u32) -> KdTree {
        KdTree {
            branch_nodes_offset: 0x40,
            branch_node_count,
            entry_count,
            bounding_box_min: [-2.0, -2.0, -2.0, 0.0],
            bounding_box_max: [2.0, 2.0, 2.0, 0.0],
        }
    }

    #[test]
    fn selected_leaf_unit_visitor_starts_midstream_and_advances_by_encoded_size() {
        let (mesh, mut bytes) = clustered_mesh_for_selected_unit_visits();
        // A direct offset-4 decode must not inspect the deliberately malformed
        // preceding unit at offset zero.
        bytes[0x80 + 16] = 0x0f;
        let mut visited = Vec::new();

        let result = mesh
            .visit_leaf_range_units(&bytes, selected_unit_range(4, 2), |cluster, data, unit| {
                assert_eq!(cluster, 1);
                assert_eq!(data.unit_count, 4);
                visited.push((
                    unit.stream_offset,
                    unit.unit_type,
                    unit.group_id,
                    unit.surface_id,
                ));
                ControlFlow::<()>::Continue(())
            })
            .unwrap();

        assert_eq!(result, ControlFlow::Continue(()));
        assert_eq!(
            visited,
            [
                (4, UnitType::Quad, Some(0x7a), None),
                (14, UnitType::TriangleList, None, None),
            ]
        );
    }

    #[test]
    fn selected_leaf_unit_visitor_uses_native_sixteen_twenty_bit_boundary() {
        for (maximum_cluster_count, encoded_start) in
            [(0x1_0000, 1_u32 << 16), (0x1_0001, 1_u32 << 20)]
        {
            let (mut mesh, bytes) = clustered_mesh_for_selected_unit_visits();
            mesh.maximum_cluster_count = maximum_cluster_count;
            let mut visited = Vec::new();
            assert_eq!(
                mesh.visit_leaf_range_units(
                    &bytes,
                    KdLeafRange {
                        encoded_start,
                        unit_count: 1,
                    },
                    |cluster, _, unit| {
                        visited.push((cluster, unit.stream_offset));
                        ControlFlow::<()>::Continue(())
                    },
                ),
                Ok(ControlFlow::Continue(()))
            );
            assert_eq!(visited, [(1, 0)]);
        }
    }

    #[test]
    fn selected_leaf_unit_visitor_stops_after_exact_count_without_decoding_tail() {
        let (mesh, mut bytes) = clustered_mesh_for_selected_unit_visits();
        // The third selected-stream successor is malformed, but count two ends
        // immediately before it and must not inspect it.
        bytes[0x80 + 16 + 21] = 0x0f;
        let mut offsets = Vec::new();

        assert_eq!(
            mesh.visit_leaf_range_units(&bytes, selected_unit_range(4, 2), |_, _, unit| {
                offsets.push(unit.stream_offset);
                ControlFlow::<()>::Continue(())
            }),
            Ok(ControlFlow::Continue(()))
        );
        assert_eq!(offsets, [4, 14]);
    }

    #[test]
    fn selected_leaf_unit_visitor_breaks_before_decoding_the_next_unit() {
        let (mesh, mut bytes) = clustered_mesh_for_selected_unit_visits();
        bytes[0x80 + 16 + 14] = 0x0f;
        let mut offsets = Vec::new();

        let result = mesh
            .visit_leaf_range_units(&bytes, selected_unit_range(4, 3), |_, _, unit| {
                offsets.push(unit.stream_offset);
                ControlFlow::Break("done")
            })
            .unwrap();

        assert_eq!(result, ControlFlow::Break("done"));
        assert_eq!(offsets, [4]);
    }

    #[test]
    fn selected_leaf_unit_visitor_validates_only_reached_cluster_offset_and_count() {
        let (mesh, bytes) = clustered_mesh_for_selected_unit_visits();
        let mut called = false;
        assert_eq!(
            mesh.visit_leaf_range_units(
                &bytes,
                KdLeafRange {
                    encoded_start: u32::MAX,
                    unit_count: 0,
                },
                |_, _, _| {
                    called = true;
                    ControlFlow::<()>::Continue(())
                },
            ),
            Ok(ControlFlow::Continue(()))
        );
        assert!(!called);

        assert_eq!(
            mesh.visit_leaf_range_units(
                &bytes,
                KdLeafRange {
                    encoded_start: 2_u32 << 16,
                    unit_count: 1,
                },
                |_, _, _| ControlFlow::<()>::Continue(()),
            ),
            Err(CollisionError::EncodedClusterOutsideMesh {
                index: 2,
                cluster_count: 2,
            })
        );

        assert_eq!(
            mesh.visit_leaf_range_units(&bytes, selected_unit_range(28, 1), |_, _, _| {
                ControlFlow::<()>::Continue(())
            }),
            Err(CollisionError::EncodedUnitOffsetNotFound {
                cluster: 1,
                offset: 28,
            })
        );

        // Past the last cluster's unit data the range goes on at a cluster the mesh lacks.
        let mut offsets = Vec::new();
        assert_eq!(
            mesh.visit_leaf_range_units(&bytes, selected_unit_range(21, 2), |_, _, unit| {
                offsets.push(unit.stream_offset);
                ControlFlow::<()>::Continue(())
            }),
            Err(CollisionError::EncodedClusterOutsideMesh {
                index: 2,
                cluster_count: 2,
            })
        );
        assert_eq!(offsets, [21]);
    }

    /// `ClusteredMesh::BBoxOverlapQueryThis` 0x82ada530: a range running past its cluster's unit
    /// data goes on at the next cluster's first unit.
    #[test]
    fn selected_leaf_unit_visitor_goes_on_at_the_next_cluster() {
        let (_, mut bytes) = clustered_mesh_for_selected_unit_visits();
        bytes[64..68].copy_from_slice(&3_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&3_u32.to_be_bytes());
        bytes[0x68..0x6c].copy_from_slice(&0x80_u32.to_be_bytes());
        let mesh = ClusteredMesh::parse(&bytes).unwrap();
        let mut visited = Vec::new();
        assert_eq!(
            mesh.visit_leaf_range_units(&bytes, selected_unit_range(14, 3), |cluster, _, unit| {
                visited.push((cluster, unit.stream_offset));
                ControlFlow::<()>::Continue(())
            }),
            Ok(ControlFlow::Continue(()))
        );
        assert_eq!(visited, [(1, 14), (1, 21), (2, 0)]);
    }

    #[test]
    fn traverses_kd_aabb_leaf_ranges() {
        let mut mesh_bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut mesh_bytes, 0, 0, (2, 0), (3, 2), [0.0, 0.0]);
        let tree = kd_tree(1, 5);

        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [-2.0; 3], [-1.0; 3])
                .unwrap(),
            [KdLeafRange {
                encoded_start: 0,
                unit_count: 2
            }]
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [-1.0; 3], [1.0; 3])
                .unwrap(),
            [KdLeafRange {
                encoded_start: 0,
                unit_count: 5
            }]
        );
    }

    #[test]
    fn limited_kd_aabb_stops_before_malformed_branch_reference() {
        let mut bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut bytes, 0, 0, (u32::MAX, 9), (300, 7), [0.0, 0.0]);
        let tree = kd_tree(1, 300);
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 1)
                .unwrap(),
            [KdLeafRange {
                encoded_start: 7,
                unit_count: 300,
            }]
        );
        let expected = Err(CollisionError::KdBranchOutsideTree {
            index: 9,
            branch_count: 1,
        });
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 2),
            expected
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb(&bytes, [-1.0; 3], [1.0; 3]),
            expected
        );
    }

    #[test]
    fn limited_kd_aabb_reads_only_visited_branch_bytes() {
        let mut bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut bytes, 0, 0, (u32::MAX, 1), (1, 5), [0.0, 0.0]);
        let tree = kd_tree(2, 1);
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 1)
                .unwrap(),
            [KdLeafRange {
                encoded_start: 5,
                unit_count: 1,
            }]
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 2),
            Err(CollisionError::RangeOutsideObject {
                start: 0x60,
                end: 0x80,
                object_size: 0x60,
            })
        );
        // The existing unlimited entry point retains its eager allocation check.
        assert_eq!(
            tree.leaf_ranges_for_aabb(&bytes, [-1.0; 3], [1.0; 3]),
            Err(CollisionError::RangeOutsideObject {
                start: 0x40,
                end: 0x80,
                object_size: 0x60,
            })
        );
    }

    #[test]
    fn limited_kd_aabb_preserves_order_coalescing_and_untruncated_counts() {
        let mut bytes = vec![0_u8; 0x80];
        write_kd_branch(&mut bytes, 0, 0, (u32::MAX, 1), (1, 9), [0.0, 0.0]);
        write_kd_branch(&mut bytes, 1, 0, (40_000, 0), (40_000, 40_000), [0.0, 0.0]);
        let tree = kd_tree(2, 80_001);
        let expected = [
            KdLeafRange {
                encoded_start: 9,
                unit_count: 1,
            },
            KdLeafRange {
                encoded_start: 0,
                unit_count: 80_000,
            },
        ];
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 1)
                .unwrap(),
            expected[..1]
        );
        for limit in [2, 3, usize::MAX] {
            assert_eq!(
                tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], limit)
                    .unwrap(),
                expected
            );
        }
        assert_eq!(
            tree.leaf_ranges_for_aabb(&bytes, [-1.0; 3], [1.0; 3])
                .unwrap(),
            expected
        );
    }

    #[test]
    fn limited_kd_aabb_zero_limit_and_branchless_tree_do_not_read_branches() {
        let mut tree = kd_tree(u32::MAX, u32::MAX);
        tree.branch_nodes_offset = u32::MAX;
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&[], [1.0; 3], [-1.0; 3], 0)
                .unwrap(),
            []
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&[], [1.0; 3], [-1.0; 3], 1),
            Err(CollisionError::InvalidBoundingBox)
        );
        tree.branch_node_count = 0;
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&[], [-1.0; 3], [1.0; 3], 1)
                .unwrap(),
            [KdLeafRange {
                encoded_start: 0,
                unit_count: u32::MAX,
            }]
        );
        tree.entry_count = 0;
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&[], [-1.0; 3], [1.0; 3], 1)
                .unwrap(),
            []
        );
    }

    #[test]
    fn limited_kd_aabb_exact_256_cap_never_requests_a_257th_range() {
        let mut bytes = vec![0_u8; 0x40 + 257 * 32];
        for index in 0_u32..256 {
            write_kd_branch(
                &mut bytes,
                usize::try_from(index).unwrap(),
                0,
                (u32::MAX, index + 1),
                (1, index),
                [0.0, 0.0],
            );
        }
        write_kd_branch(&mut bytes, 256, 3, (1, 256), (0, 257), [0.0, 0.0]);
        let tree = kd_tree(257, 257);
        let capped = tree
            .leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 256)
            .unwrap();
        assert_eq!(
            capped,
            (0..256)
                .map(|encoded_start| KdLeafRange {
                    encoded_start,
                    unit_count: 1,
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb_limited(&bytes, [-1.0; 3], [1.0; 3], 257),
            Err(CollisionError::InvalidKdAxis { node: 256, axis: 3 })
        );
    }

    #[test]
    fn stateful_kd_query_emits_leaf_before_stacked_branch() {
        let mut mesh_bytes = vec![0_u8; 0x80];
        write_kd_branch(&mut mesh_bytes, 0, 0, (u32::MAX, 1), (1, 3), [0.0, 0.0]);
        write_kd_branch(&mut mesh_bytes, 1, 0, (1, 0), (2, 1), [0.0, 0.0]);
        let tree = kd_tree(2, 4);

        let mut query = tree
            .aabb_range_query(&mesh_bytes, [-1.0; 3], [1.0; 3])
            .unwrap();
        assert_eq!(
            query.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 3,
                unit_count: 1,
            })
        );
        assert_eq!(
            query.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 0,
                unit_count: 3,
            })
        );
        assert_eq!(query.next_range().unwrap(), None);
        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [-1.0; 3], [1.0; 3])
                .unwrap(),
            [
                KdLeafRange {
                    encoded_start: 3,
                    unit_count: 1,
                },
                KdLeafRange {
                    encoded_start: 0,
                    unit_count: 3,
                },
            ]
        );
    }

    #[test]
    fn stateful_kd_query_keeps_adjacent_separate_branch_ranges_distinct() {
        let mut mesh_bytes = vec![0_u8; 0xa0];
        write_kd_branch(
            &mut mesh_bytes,
            0,
            0,
            (u32::MAX, 1),
            (u32::MAX, 2),
            [0.0, 0.0],
        );
        write_kd_branch(&mut mesh_bytes, 1, 0, (1, 0), (0, 1), [0.0, 0.0]);
        write_kd_branch(&mut mesh_bytes, 2, 0, (1, 1), (0, 2), [0.0, 0.0]);
        let tree = kd_tree(3, 2);

        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [-1.0; 3], [1.0; 3])
                .unwrap(),
            [
                KdLeafRange {
                    encoded_start: 0,
                    unit_count: 1,
                },
                KdLeafRange {
                    encoded_start: 1,
                    unit_count: 1,
                },
            ]
        );
    }

    #[test]
    fn stateful_kd_query_uses_inclusive_extent_boundaries() {
        let mut mesh_bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut mesh_bytes, 0, 0, (1, 0), (1, 1), [0.0, 0.0]);
        let tree = kd_tree(1, 2);
        let next_above = f32::from_bits(1);
        let next_below = -f32::from_bits(1);

        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [next_above, 0.0, 0.0], [next_above, 0.0, 0.0],)
                .unwrap(),
            [KdLeafRange {
                encoded_start: 1,
                unit_count: 1,
            }]
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [next_below, 0.0, 0.0], [next_below, 0.0, 0.0],)
                .unwrap(),
            [KdLeafRange {
                encoded_start: 0,
                unit_count: 1,
            }]
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [0.0; 3], [0.0; 3])
                .unwrap(),
            [KdLeafRange {
                encoded_start: 0,
                unit_count: 2,
            }]
        );
    }

    #[test]
    fn stateful_kd_query_validates_only_when_a_branch_is_visited() {
        let mut mesh_bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut mesh_bytes, 0, 0, (u32::MAX, 9), (1, 7), [0.0, 0.0]);
        let tree = kd_tree(1, 1);
        let mut query = tree
            .aabb_range_query(&mesh_bytes, [-1.0; 3], [1.0; 3])
            .unwrap();

        assert_eq!(
            query.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 7,
                unit_count: 1,
            })
        );
        assert_eq!(
            query.next_range(),
            Err(CollisionError::KdBranchOutsideTree {
                index: 9,
                branch_count: 1,
            })
        );
        assert_eq!(
            tree.leaf_ranges_for_aabb(&mesh_bytes, [-1.0; 3], [1.0; 3]),
            Err(CollisionError::KdBranchOutsideTree {
                index: 9,
                branch_count: 1,
            })
        );

        assert_eq!(
            tree.aabb_range_query(&mesh_bytes, [1.0; 3], [-1.0; 3]),
            Err(CollisionError::InvalidBoundingBox)
        );
        write_kd_branch(&mut mesh_bytes, 0, 3, (1, 0), (1, 1), [0.0, 0.0]);
        let mut invalid_axis = tree
            .aabb_range_query(&mesh_bytes, [-1.0; 3], [1.0; 3])
            .unwrap();
        assert_eq!(
            invalid_axis.next_range(),
            Err(CollisionError::InvalidKdAxis { node: 0, axis: 3 })
        );
    }

    #[test]
    fn branchless_empty_kd_query_has_no_range() {
        let tree = kd_tree(0, 0);
        let mut query = tree.aabb_range_query(&[], [-1.0; 3], [1.0; 3]).unwrap();
        assert_eq!(query.next_range().unwrap(), None);
        assert_eq!(
            tree.leaf_ranges_for_aabb(&[], [-1.0; 3], [1.0; 3]).unwrap(),
            []
        );
    }

    #[test]
    fn kd_line_padding_scales_epsilon_not_radius_at_world_coordinates() {
        let clipper = KdAxisAlignedLineClipper::new(
            [1000.0, -2000.0, 3000.0],
            [1002.0, -1998.0, 3002.0],
            0.125,
            [-4000.0; 4],
            [4000.0; 4],
        );
        // Every half-delta is 1, so the scene-scale extension is exactly zero.
        // Scalar operands follow raw VMX VA*VC+VB, independently of the caller.
        assert_eq!(
            clipper.padding.map(f32::to_bits),
            [1002.0_f32, 2000.0, 3002.0].map(|m| m.mul_add(0.000_001, 0.125).to_bits())
        );
        assert!(clipper.padding.into_iter().all(|p| p > 0.125 && p < 0.129));
    }

    #[test]
    fn kd_fat_line_at_world_coordinates_does_not_visit_distant_leaf() {
        let mut bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut bytes, 0, 0, (1, 10), (1000, 20), [1050.0, 1050.0]);
        let tree = KdTree {
            bounding_box_min: [900.0, -10.0, -10.0, 0.0],
            bounding_box_max: [1100.0, 10.0, 10.0, 0.0],
            ..kd_tree(1, 1001)
        };
        let mut query = tree
            .line_range_query(&bytes, [1000.0, -1.0, -1.0], [1002.0, 1.0, 1.0], 0.125)
            .unwrap();
        assert_eq!(
            query.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 10,
                unit_count: 1,
            })
        );
        assert_eq!(query.next_range().unwrap(), None);
    }

    #[test]
    fn kd_line_clipper_matches_finite_power_of_two_golden_state() {
        let tree = kd_tree(0, 3);
        let mut query = tree
            .line_range_query(&[], [-1.0; 3], [1.0; 3], 0.25)
            .unwrap();

        assert_eq!(
            query.line_clipper.origin.map(f32::to_bits),
            [0xbf80_0000; 3]
        );
        assert_eq!(query.line_clipper.delta.map(f32::to_bits), [0x4000_0000; 3]);
        assert_eq!(query.line_clipper.recip.map(f32::to_bits), [0x3f00_0000; 3]);
        assert_eq!(
            query.line_clipper.padding.map(f32::to_bits),
            [0x3e80_0022; 3]
        );
        assert_eq!(query.stack, []);
        assert_eq!(
            query.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 0,
                unit_count: 3,
            })
        );
        assert_eq!(query.next_range().unwrap(), None);
    }

    #[test]
    fn kd_line_clipper_extends_zero_direction_at_scene_scale() {
        let tree = kd_tree(0, 1);
        let query = tree.line_range_query(&[], [0.5; 3], [0.5; 3], 0.0).unwrap();

        for axis in 0..3 {
            assert!(query.line_clipper.origin[axis] < 0.5);
            assert!(query.line_clipper.delta[axis] > 0.0);
            assert!(query.line_clipper.recip[axis].is_finite());
            assert!(query.line_clipper.recip[axis] > 0.0);
            assert!(query.line_clipper.padding[axis] > 0.000_001);
        }
    }

    #[test]
    fn kd_line_query_visits_near_child_first_in_both_directions() {
        let mut mesh_bytes = vec![0_u8; 0x60];
        write_kd_branch(&mut mesh_bytes, 0, 0, (1, 10), (1, 20), [0.0, 0.0]);
        let tree = kd_tree(1, 2);

        let mut positive = tree
            .line_range_query(&mesh_bytes, [-1.0; 3], [1.0; 3], 0.0)
            .unwrap();
        assert_eq!(
            positive.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 10,
                unit_count: 1,
            })
        );
        assert_eq!(
            positive.next_range().unwrap(),
            Some(KdLeafRange {
                encoded_start: 20,
                unit_count: 1,
            })
        );
        assert_eq!(positive.next_range().unwrap(), None);

        let mut negative = tree
            .line_range_query(&mesh_bytes, [1.0; 3], [-1.0; 3], 0.0)
            .unwrap();
        assert_eq!(negative.next_range().unwrap().unwrap().encoded_start, 20);
        assert_eq!(negative.next_range().unwrap().unwrap().encoded_start, 10);
        assert_eq!(negative.next_range().unwrap(), None);
    }

    fn synthetic_line_query(node: BranchNode) -> KdTreeLineRangeQuery<'static> {
        KdTreeLineRangeQuery {
            branch_nodes: KdLineBranches::Parsed(vec![node]),
            line_clipper: KdAxisAlignedLineClipper {
                origin: [0.0; 3],
                delta: [1.0; 3],
                recip: [1.0; 3],
                padding: [0.0; 3],
            },
            stack: vec![KdLineStackElement {
                content: u32::MAX,
                index: 0,
                pa: 0.0,
                pb: 1.0,
            }],
            leaf_count: 0,
            next_entry: 0,
        }
    }

    #[test]
    fn borrowed_kd_line_matches_eager_nodes_for_ranges_clips_and_errors() {
        let mut bytes = vec![0_u8; 0x40 + 3 * 32];
        let tree = kd_tree(3, 4);
        write_kd_branch(&mut bytes, 0, 0, (u32::MAX, 1), (u32::MAX, 2), [0.0; 2]);
        write_kd_branch(&mut bytes, 1, 1, (2, 10), (3, 20), [-0.25, 0.25]);
        write_kd_branch(&mut bytes, 2, 2, (4, 30), (1, 40), [-0.5, 0.5]);
        for x in [-2.0, -0.1, 0.0, 0.5, 2.0] {
            for y in [-2.0, -0.1, 0.0, 0.5, 2.0] {
                for z in [-2.0, -0.1, 0.0, 0.5, 2.0] {
                    for radius in [0.0, 0.2, 2.0] {
                        let mut borrowed = tree
                            .line_range_query(&bytes, [x, y, z], [-x, -y, -z], radius)
                            .unwrap();
                        let mut eager = borrowed.clone();
                        eager.branch_nodes =
                            KdLineBranches::Parsed(tree.branch_nodes(&bytes).unwrap());
                        assert_eq!(borrowed, eager);
                        for step in 0..8 {
                            let actual = borrowed.next_range();
                            let expected = eager.next_range();
                            assert_eq!(actual, expected);
                            if !matches!(actual, Ok(Some(_))) {
                                break;
                            }
                            let clip = if step == 0 { 0.75 } else { 0.5 };
                            borrowed.clip_end(clip);
                            eager.clip_end(clip);
                        }
                    }
                }
            }
        }
        // Unvisited invalid axes remain accepted; visited invalid axes produce
        // the same lazy error as the previous eagerly decoded representation.
        write_kd_branch(&mut bytes, 1, 7, (2, 10), (3, 20), [0.0; 2]);
        let mut borrowed = tree
            .line_range_query(&bytes, [-1.0; 3], [1.0; 3], 0.0)
            .unwrap();
        let mut eager = borrowed.clone();
        eager.branch_nodes = KdLineBranches::Parsed(tree.branch_nodes(&bytes).unwrap());
        assert_eq!(borrowed.next_range(), eager.next_range());
    }

    #[test]
    fn borrowed_kd_line_keeps_whole_allocation_validation_before_geometric_miss() {
        let tree = kd_tree(3, 4);
        let bytes = vec![0_u8; 0x40 + 3 * 32 - 1];
        assert_eq!(
            tree.line_range_query(&bytes, [100.0; 3], [101.0; 3], 0.0)
                .unwrap_err(),
            tree.branch_nodes(&bytes).unwrap_err()
        );
        let empty = KdTree {
            branch_nodes_offset: u32::MAX,
            ..kd_tree(0, 1)
        };
        assert!(
            empty
                .line_range_query(&[], [-1.0; 3], [1.0; 3], 0.0)
                .is_ok()
        );
    }

    #[test]
    fn kd_line_query_excludes_split_equality_on_each_side() {
        let mut far_equal = synthetic_line_query(BranchNode {
            parent: 0,
            axis: 0,
            child_content: [1, 1],
            child_index: [10, 20],
            extents: [2.0, 1.0],
        });
        assert_eq!(far_equal.next_range().unwrap().unwrap().encoded_start, 10);
        assert_eq!(far_equal.next_range().unwrap(), None);

        let mut near_equal = synthetic_line_query(BranchNode {
            parent: 0,
            axis: 0,
            child_content: [1, 1],
            child_index: [10, 20],
            extents: [0.0, -1.0],
        });
        assert_eq!(near_equal.next_range().unwrap().unwrap().encoded_start, 20);
        assert_eq!(near_equal.next_range().unwrap(), None);
    }

    #[test]
    fn kd_line_query_root_rejects_an_equality_only_interval() {
        let tree = KdTree {
            branch_nodes_offset: 0,
            branch_node_count: 0,
            entry_count: 1,
            bounding_box_min: [-1.0; 4],
            bounding_box_max: [1.0; 4],
        };
        let mut tangent = tree
            .line_range_query(&[], [-2.0, -0.5, -0.5], [-1.0, 0.5, 0.5], -0.000_002)
            .unwrap();
        assert_eq!(tangent.line_clipper.padding[0].to_bits(), 0);
        assert_eq!(tangent.next_range().unwrap(), None);

        let mut crossing = tree
            .line_range_query(&[], [-2.0, -0.5, -0.5], [-0.999, 0.5, 0.5], -0.000_002)
            .unwrap();
        assert_eq!(crossing.next_range().unwrap().unwrap().unit_count, 1);
    }

    #[test]
    fn kd_line_query_skips_a_zero_count_leaf_before_the_next_candidate() {
        let mut query = synthetic_line_query(BranchNode {
            parent: 0,
            axis: 0,
            child_content: [0, 1],
            child_index: [10, 20],
            extents: [0.5, -0.5],
        });

        assert_eq!(
            query.next_range().unwrap().unwrap(),
            KdLeafRange {
                encoded_start: 20,
                unit_count: 1,
            }
        );
        assert_eq!(query.next_range().unwrap(), None);
    }

    #[test]
    fn kd_line_clip_end_preserves_order_and_buffered_leaf() {
        let mut query = KdTreeLineRangeQuery {
            branch_nodes: KdLineBranches::Parsed(Vec::new()),
            line_clipper: KdAxisAlignedLineClipper {
                origin: [0.0; 3],
                delta: [1.0; 3],
                recip: [1.0; 3],
                padding: [0.0; 3],
            },
            stack: vec![
                KdLineStackElement {
                    content: 1,
                    index: 10,
                    pa: 0.1,
                    pb: 0.9,
                },
                KdLineStackElement {
                    content: 2,
                    index: 20,
                    pa: 0.8,
                    pb: 1.0,
                },
                KdLineStackElement {
                    content: 3,
                    index: 30,
                    pa: 0.3,
                    pb: 0.7,
                },
            ],
            leaf_count: 5,
            next_entry: 100,
        };

        query.clip_end(0.5);
        assert_eq!(query.stack.len(), 2);
        assert_eq!(query.stack[0].index, 10);
        assert_eq!(query.stack[0].pb.to_bits(), 0.5_f32.to_bits());
        assert_eq!(query.stack[1].index, 30);
        assert_eq!(query.stack[1].pb.to_bits(), 0.5_f32.to_bits());
        assert_eq!(
            query.next_range().unwrap().unwrap(),
            KdLeafRange {
                encoded_start: 100,
                unit_count: 5,
            }
        );
        assert_eq!(query.next_range().unwrap().unwrap().encoded_start, 30);
        assert_eq!(query.next_range().unwrap().unwrap().encoded_start, 10);
    }

    #[test]
    fn scalar_subsel_helpers_preserve_reference_ties_and_unordered_choice() {
        assert_eq!(max_subsel(-0.0, 0.0).to_bits(), (-0.0_f32).to_bits());
        assert_eq!(min_subsel(0.0, -0.0).to_bits(), (-0.0_f32).to_bits());
        assert_eq!(max_subsel(f32::NAN, 7.0).to_bits(), 7.0_f32.to_bits());
        assert_eq!(min_subsel(f32::NAN, 7.0).to_bits(), f32::NAN.to_bits());
    }

    #[test]
    fn kd_line_clip_end_retains_an_unordered_start_parameter() {
        let mut query = KdTreeLineRangeQuery {
            branch_nodes: KdLineBranches::Parsed(Vec::new()),
            line_clipper: KdAxisAlignedLineClipper {
                origin: [0.0; 3],
                delta: [1.0; 3],
                recip: [1.0; 3],
                padding: [0.0; 3],
            },
            stack: vec![KdLineStackElement {
                content: 1,
                index: 42,
                pa: f32::NAN,
                pb: 0.9,
            }],
            leaf_count: 2,
            next_entry: 7,
        };

        query.clip_end(0.5);
        assert_eq!(query.stack.len(), 1);
        assert!(query.stack[0].pa.is_nan());
        assert_eq!(query.stack[0].pb.to_bits(), 0.5_f32.to_bits());
        assert_eq!(query.leaf_count, 2);
        assert_eq!(query.next_entry, 7);
    }

    #[test]
    fn kd_line_clip_end_with_unordered_end_retains_all_old_intervals() {
        let mut query = KdTreeLineRangeQuery {
            branch_nodes: KdLineBranches::Parsed(Vec::new()),
            line_clipper: KdAxisAlignedLineClipper {
                origin: [0.0; 3],
                delta: [1.0; 3],
                recip: [1.0; 3],
                padding: [0.0; 3],
            },
            stack: vec![
                KdLineStackElement {
                    content: 1,
                    index: 11,
                    pa: 0.8,
                    pb: 0.9,
                },
                KdLineStackElement {
                    content: 2,
                    index: 22,
                    pa: f32::NAN,
                    pb: -0.0,
                },
            ],
            leaf_count: 3,
            next_entry: 33,
        };
        let old_pb = [query.stack[0].pb.to_bits(), query.stack[1].pb.to_bits()];

        query.clip_end(f32::NAN);

        assert_eq!(query.stack.len(), 2);
        assert_eq!(query.stack[0].index, 11);
        assert_eq!(query.stack[1].index, 22);
        assert_eq!(query.stack[0].pb.to_bits(), old_pb[0]);
        assert_eq!(query.stack[1].pb.to_bits(), old_pb[1]);
        assert_eq!(query.leaf_count, 3);
        assert_eq!(query.next_entry, 33);
    }

    #[test]
    fn kd_line_query_reports_native_stack_capacity_as_host_error() {
        let mut mesh_bytes = vec![0_u8; 0x40 + KD_LINE_STACK_CAPACITY * 32];
        for index in 0..KD_LINE_STACK_CAPACITY {
            let index_u32 = u32::try_from(index).unwrap();
            let low = if index + 1 == KD_LINE_STACK_CAPACITY {
                (1, 500)
            } else {
                (u32::MAX, u32::try_from(index + 1).unwrap())
            };
            let split = f32::from(u16::try_from(KD_LINE_STACK_CAPACITY - index - 1).unwrap());
            write_kd_branch(
                &mut mesh_bytes,
                index,
                0,
                low,
                (1, 1000 + index_u32),
                [split; 2],
            );
        }
        let capacity_u32 = u32::try_from(KD_LINE_STACK_CAPACITY).unwrap();
        let tree = KdTree {
            branch_nodes_offset: 0x40,
            branch_node_count: capacity_u32,
            entry_count: capacity_u32 + 1,
            bounding_box_min: [-100.0; 4],
            bounding_box_max: [100.0; 4],
        };
        let mut query = tree
            .line_range_query(&mesh_bytes, [-100.0; 3], [100.0; 3], 0.0)
            .unwrap();

        let expected = Err(CollisionError::KdLineStackCapacityExceeded {
            capacity: KD_LINE_STACK_CAPACITY,
            required: KD_LINE_STACK_CAPACITY + 1,
        });
        assert_eq!(query.next_range(), expected);
        assert_eq!(query.stack.len(), KD_LINE_STACK_CAPACITY);
        assert_eq!(query.stack.last().unwrap().index, capacity_u32 - 1);
        assert!(!query.stack.iter().any(|element| element.index == 1031));

        let before_replay = query.clone();
        assert_eq!(query.next_range(), expected);
        assert_eq!(query, before_replay);
    }

    #[test]
    fn kd_line_query_validates_a_visited_axis_and_branch_lazily() {
        let mut mesh_bytes = vec![0_u8; 0x60];
        let tree = kd_tree(1, 1);
        write_kd_branch(&mut mesh_bytes, 0, 3, (1, 0), (1, 1), [0.0; 2]);
        let mut invalid_axis = tree
            .line_range_query(&mesh_bytes, [-1.0; 3], [1.0; 3], 0.0)
            .unwrap();
        assert_eq!(
            invalid_axis.next_range(),
            Err(CollisionError::InvalidKdAxis { node: 0, axis: 3 })
        );

        write_kd_branch(&mut mesh_bytes, 0, 0, (u32::MAX, 9), (1, 1), [0.0; 2]);
        let mut invalid_branch = tree
            .line_range_query(&mesh_bytes, [-1.0; 3], [1.0; 3], 0.0)
            .unwrap();
        assert_eq!(
            invalid_branch.next_range(),
            Err(CollisionError::KdBranchOutsideTree {
                index: 9,
                branch_count: 1,
            })
        );
    }

    #[test]
    fn parses_and_queries_kd_tree_mapped_array() {
        let mut bytes = vec![0_u8; 224];
        bytes[40..44].copy_from_slice(&1_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&80_u32.to_be_bytes());
        bytes[64..68].copy_from_slice(&176_u32.to_be_bytes());
        bytes[80 + 64..80 + 68].copy_from_slice(&(VolumeType::Box as u32).to_be_bytes());
        bytes[176 + 8..176 + 12].copy_from_slice(&1_u32.to_be_bytes());

        let mapped = KdTreeMappedArray::parse(&bytes).unwrap();
        let volumes = mapped.volumes(&bytes).unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].volume_type(), Some(VolumeType::Box));
        assert_eq!(
            mapped
                .candidate_volume_indices_for_aabb(&bytes, [-1.0; 3], [1.0; 3])
                .unwrap(),
            [0]
        );
    }

    #[test]
    fn mapped_line_candidates_follow_native_direction_order() {
        let (mapped, bytes) = mapped_array_with_kd(2, 2, Some((0, (1, 0), (1, 1), [0.0; 2])));

        assert_eq!(
            mapped
                .candidate_volume_indices_for_line(&bytes, [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0,)
                .unwrap(),
            [0, 1]
        );
        assert_eq!(
            mapped
                .candidate_volume_indices_for_line(&bytes, [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 0.0,)
                .unwrap(),
            [1, 0]
        );
    }

    #[test]
    fn mapped_line_candidates_reject_an_out_of_bounds_leaf_atomically() {
        let (mapped, bytes) = mapped_array_with_kd(1, 2, None);

        assert_eq!(
            mapped.candidate_volume_indices_for_line(&bytes, [-1.0; 3], [1.0; 3], 0.0,),
            Err(CollisionError::KdEntryOutsideMappedArray {
                start: 0,
                end: 2,
                volume_count: 1,
            })
        );
    }

    #[test]
    fn mapped_line_candidates_discard_earlier_ranges_on_later_traversal_error() {
        let (mapped, bytes) =
            mapped_array_with_kd(1, 1, Some((0, (1, 0), (u32::MAX, 9), [0.0; 2])));

        assert_eq!(
            mapped.candidate_volume_indices_for_line(
                &bytes,
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                0.0,
            ),
            Err(CollisionError::KdBranchOutsideTree {
                index: 9,
                branch_count: 1,
            })
        );
    }

    #[test]
    fn clustered_mesh_thin_line_retains_near_and_far_hits() {
        let triangles = [
            [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]],
            [[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [-1.0, 1.0, -1.0]],
        ];
        let (mesh, bytes) = clustered_mesh_with_kd(&triangles, 2, (1, 4), (1, 0), [0.0; 2]);

        let hits = mesh
            .thin_segment_intersections(&bytes, [0.0, 0.0, 2.0], [0.0, 0.0, -2.0])
            .unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].triangle.unit_stream_offset, 0);
        assert_eq!(hits[1].triangle.unit_stream_offset, 4);
        assert_eq!(
            hits[0].intersection.line_parameter.to_bits(),
            0.25_f32.to_bits()
        );
        assert_eq!(
            hits[1].intersection.line_parameter.to_bits(),
            0.75_f32.to_bits()
        );
    }

    #[test]
    fn clustered_mesh_thin_equal_hits_keep_candidate_order_not_source_order() {
        let triangle = [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, 1.0, 0.0]];
        let (mesh, bytes) =
            clustered_mesh_with_kd(&[triangle, triangle], 0, (1, 4), (1, 0), [0.0; 2]);

        let hits = mesh
            .thin_segment_intersections(&bytes, [-0.5, 0.0, 1.0], [0.5, 0.0, -1.0])
            .unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].intersection.line_parameter.to_bits(),
            0.5_f32.to_bits()
        );
        assert_eq!(
            hits[1].intersection.line_parameter.to_bits(),
            0.5_f32.to_bits()
        );
        assert_eq!(
            hits.iter()
                .map(|hit| hit.triangle.unit_stream_offset)
                .collect::<Vec<_>>(),
            [4, 0]
        );
    }

    #[test]
    fn clustered_mesh_sphere_forwards_radius_to_fat_line_traversal() {
        let triangles = [[[0.9, -1.0, 0.0], [1.5, -1.0, 0.0], [0.9, 1.0, 0.0]]];
        let (mesh, bytes) = clustered_mesh_with_kd(&triangles, 0, (0, 0), (1, 0), [0.6; 2]);
        let tree = mesh.kd_tree(&bytes).unwrap();
        let thin_ranges = collect_kd_line_ranges(
            tree.line_range_query(&bytes, [0.4, 0.0, 2.0], [0.4, 0.0, -2.0], 0.0)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(thin_ranges, []);

        let hits = mesh
            .sphere_segment_intersections(&bytes, [0.4, 0.0, 2.0], [0.4, 0.0, -2.0], 0.75)
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].triangle.unit_stream_offset, 0);
        assert!(hits[0].intersection.line_parameter > 0.0);
        assert!(hits[0].intersection.line_parameter < 1.0);

        let reverse_hits = mesh
            .sphere_segment_intersections(&bytes, [0.4, 0.0, -2.0], [0.4, 0.0, 2.0], 0.75)
            .unwrap();
        assert_eq!(reverse_hits.len(), 1);
        assert_eq!(reverse_hits[0].triangle.unit_stream_offset, 0);
        assert!(reverse_hits[0].intersection.line_parameter > 0.0);
        assert!(reverse_hits[0].intersection.line_parameter < 1.0);
    }

    #[test]
    fn clustered_mesh_sphere_uses_fat_line_direction_order() {
        const KD_OFFSET: usize = 0x80;
        const BRANCH_OFFSET: usize = 0xb0;
        let triangle = [[-2.0, -1.0, 0.0], [2.0, -1.0, 0.0], [-2.0, 2.0, 0.0]];
        let (mesh, mut bytes) = clustered_mesh_with_kd(
            &[triangle, triangle, triangle],
            0,
            (1, 0),
            (u32::MAX, 1),
            [0.0; 2],
        );
        bytes[KD_OFFSET + 4..KD_OFFSET + 8].copy_from_slice(&2_u32.to_be_bytes());
        write_kd_branch_at(&mut bytes, BRANCH_OFFSET, 1, 1, (1, 4), (1, 8), [0.6; 2]);

        let tree = mesh.kd_tree(&bytes).unwrap();
        let thin_ranges = collect_kd_line_ranges(
            tree.line_range_query(&bytes, [0.4, 0.5, 2.0], [-0.4, 0.5, -2.0], 0.0)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            thin_ranges
                .iter()
                .map(|range| range.encoded_start)
                .collect::<Vec<_>>(),
            [4, 0]
        );

        let old_expanded_aabb_ranges = tree
            .leaf_ranges_for_aabb(
                &bytes,
                [-0.4 - 0.75, 0.5 - 0.75, -2.0 - 0.75],
                [0.4 + 0.75, 0.5 + 0.75, 2.0 + 0.75],
            )
            .unwrap();
        assert_eq!(
            old_expanded_aabb_ranges,
            [
                KdLeafRange {
                    encoded_start: 0,
                    unit_count: 1,
                },
                KdLeafRange {
                    encoded_start: 4,
                    unit_count: 2,
                },
            ]
        );

        let hits = mesh
            .sphere_segment_intersections(&bytes, [0.4, 0.5, 2.0], [-0.4, 0.5, -2.0], 0.75)
            .unwrap();
        assert_eq!(
            hits.iter()
                .map(|hit| hit.triangle.unit_stream_offset)
                .collect::<Vec<_>>(),
            [4, 8, 0]
        );
        let equal_parameter = hits[0].intersection.line_parameter;
        assert!(equal_parameter > 0.0 && equal_parameter < 1.0);
        assert!(
            hits.iter().all(|hit| {
                hit.intersection.line_parameter.to_bits() == equal_parameter.to_bits()
            })
        );
    }

    #[test]
    fn parses_simple_mapped_array_and_inline_volumes() {
        let mut bytes = vec![0_u8; 64 + 2 * COLLISION_HEADER_SIZE];
        bytes[36..40].copy_from_slice(&2_u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&2_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&64_u32.to_be_bytes());
        bytes[64 + 64..64 + 68].copy_from_slice(&(VolumeType::Sphere as u32).to_be_bytes());
        bytes[64 + COLLISION_HEADER_SIZE + 64..64 + COLLISION_HEADER_SIZE + 68]
            .copy_from_slice(&(VolumeType::Box as u32).to_be_bytes());

        let mapped = SimpleMappedArray::parse(&bytes).unwrap();
        assert_eq!(mapped.tag_bit_count, 2);
        assert_eq!(mapped.volume_count, 2);
        assert_eq!(
            mapped
                .volumes(&bytes)
                .unwrap()
                .iter()
                .map(Volume::volume_type)
                .collect::<Vec<_>>(),
            [Some(VolumeType::Sphere), Some(VolumeType::Box)]
        );
    }

    #[test]
    fn intersects_front_facing_thin_triangle_segment() {
        let triangle = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let hit = thin_segment_triangle_intersection([0.25, 0.25, 1.0], [0.0, 0.0, -2.0], triangle)
            .unwrap();

        assert!((hit.line_parameter - 0.5).abs() < f32::EPSILON);
        for (actual, expected) in hit.position.into_iter().zip([0.25, 0.25, 0.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
        for (actual, expected) in hit.normal.into_iter().zip([0.0, 0.0, 1.0]) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
        assert!(
            thin_segment_triangle_intersection([0.25, 0.25, -1.0], [0.0, 0.0, 2.0], triangle)
                .is_none()
        );
    }

    #[test]
    fn swept_sphere_hits_triangle_face_edge_and_vertex_regions() {
        let triangle = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

        let face = sphere_segment_triangle_intersection(
            [0.25, 0.25, 2.0],
            [0.0, 0.0, -4.0],
            0.5,
            triangle,
        )
        .unwrap();
        assert!((face.line_parameter - 0.375).abs() < 1.0e-6);
        for (actual, expected) in face.position.into_iter().zip([0.25, 0.25, 0.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
        for (actual, expected) in face.normal.into_iter().zip([0.0, 0.0, 1.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }

        let edge =
            sphere_segment_triangle_intersection([0.5, -0.4, 1.0], [0.0, 0.0, -2.0], 0.5, triangle)
                .unwrap();
        assert!((edge.line_parameter - 0.35).abs() < 1.0e-6);
        for (actual, expected) in edge.normal.into_iter().zip([0.0, -0.8, 0.6]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }

        let vertex = sphere_segment_triangle_intersection(
            [-0.3, -0.4, 1.0],
            [0.0, 0.0, -2.0],
            0.5,
            triangle,
        )
        .unwrap();
        assert!((vertex.line_parameter - 0.5).abs() < 1.0e-6);
        for (actual, expected) in vertex.normal.into_iter().zip([-0.6, -0.8, 0.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn swept_sphere_reports_initial_triangle_overlap() {
        let triangle = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let hit =
            sphere_segment_triangle_intersection([0.25, 0.25, 0.2], [0.0, 0.0, 1.0], 0.5, triangle)
                .unwrap();
        assert!(hit.line_parameter.abs() < f32::EPSILON);
        for (actual, expected) in hit.position.into_iter().zip([0.25, 0.25, -0.3]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
        for (actual, expected) in hit.normal.into_iter().zip([0.0, 0.0, 1.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn triangle_aabb_overlap_uses_face_and_cross_edge_axes() {
        let triangle = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        assert!(triangle_aabb_overlap(
            [0.2, 0.2, -0.1],
            [0.4, 0.4, 0.1],
            triangle
        ));
        assert!(!triangle_aabb_overlap(
            [1.5, 1.5, -0.1],
            [1.7, 1.7, 0.1],
            triangle
        ));
        assert!(!triangle_aabb_overlap(
            [0.2, 0.2, 1.0],
            [0.4, 0.4, 2.0],
            triangle
        ));
    }

    #[test]
    fn triangle_aabb_overlap_uses_native_fused_edge_radius() {
        let extent = [
            f32::from_bits(0x3684_800d),
            f32::from_bits(0x46e9_a732),
            f32::from_bits(0x3684_800d),
        ];
        let vertices = [
            [
                0.0,
                f32::from_bits(0x46e9_a732),
                f32::from_bits(0xb684_800d),
            ],
            [
                0.0,
                f32::from_bits(0x46ea_6d33),
                f32::from_bits(0x3949_622d),
            ],
            [
                f32::from_bits(0x3684_800d),
                f32::from_bits(0x46e9_a732),
                f32::from_bits(0xb684_800d),
            ],
        ];
        let edge_0 = subtract(vertices[1], vertices[0]);
        assert_eq!(edge_0[1].to_bits(), 0x42c6_0100);
        assert_eq!(edge_0[2].to_bits(), 0x394d_862d);

        let first = edge_axis_projection_x(vertices[0], edge_0);
        let second = edge_axis_projection_x(vertices[2], edge_0);
        let fused_radius = edge_axis_radius_x(edge_0, extent);
        let scalar_radius = extent[1] * edge_0[2].abs() + extent[2] * edge_0[1].abs();
        assert_eq!(first.to_bits(), 0x40bb_988e);
        assert_eq!(second.to_bits(), 0x40bb_988e);
        assert_eq!(scalar_radius.to_bits(), 0x40bb_988e);
        assert_eq!(fused_radius.to_bits(), 0x40bb_988d);
        assert!(edge_axis_separates(first, second, fused_radius));

        assert!(!triangle_aabb_overlap(
            extent.map(|component| -component),
            extent,
            vertices,
        ));
    }

    #[test]
    fn triangle_aabb_overlap_keeps_exact_contact_inclusive() {
        let touching = [[1.0, -0.5, -0.5], [1.0, 0.5, -0.5], [1.0, 0.0, 0.5]];
        assert!(triangle_aabb_overlap(
            [-1.0, -1.0, -1.0],
            [1.0, 1.0, 1.0],
            touching,
        ));

        let outside = f32::from_bits(1.0f32.to_bits() + 1);
        let separated = touching.map(|mut vertex| {
            vertex[0] = outside;
            vertex
        });
        assert!(!triangle_aabb_overlap(
            [-1.0, -1.0, -1.0],
            [1.0, 1.0, 1.0],
            separated,
        ));
    }

    #[test]
    fn edge_axis_cases_preserve_all_native_vertex_pairs_and_order() {
        assert_eq!(
            NATIVE_EDGE_AXIS_CASES.map(|case| (case.edge, case.axis, case.vertices)),
            [
                (0, EdgeBoxAxis::X, [0, 2]),
                (0, EdgeBoxAxis::Y, [0, 2]),
                (0, EdgeBoxAxis::Z, [1, 2]),
                (1, EdgeBoxAxis::X, [0, 2]),
                (1, EdgeBoxAxis::Y, [0, 2]),
                (1, EdgeBoxAxis::Z, [0, 1]),
                (2, EdgeBoxAxis::X, [0, 1]),
                (2, EdgeBoxAxis::Y, [0, 1]),
                (2, EdgeBoxAxis::Z, [1, 2]),
            ],
        );

        for case in NATIVE_EDGE_AXIS_CASES {
            let (edge, projection_component) = match case.axis {
                EdgeBoxAxis::X => ([0.0, 0.0, 1.0], 1),
                EdgeBoxAxis::Y => ([1.0, 0.0, 0.0], 2),
                EdgeBoxAxis::Z => ([0.0, 1.0, 0.0], 0),
            };
            let omitted = (0..3)
                .find(|vertex| !case.vertices.contains(vertex))
                .unwrap();
            let mut vertices = [[0.0; 3]; 3];
            vertices[case.vertices[0]][projection_component] = 1.0;
            vertices[case.vertices[1]][projection_component] = 1.0;
            vertices[omitted][projection_component] = -1.0;
            let mut edges = [[0.0; 3]; 3];
            edges[case.edge] = edge;

            let project = |vertex| match case.axis {
                EdgeBoxAxis::X => edge_axis_projection_x(vertex, edge),
                EdgeBoxAxis::Y => edge_axis_projection_y(vertex, edge),
                EdgeBoxAxis::Z => edge_axis_projection_z(vertex, edge),
            };
            let first = project(vertices[case.vertices[0]]);
            let second = project(vertices[case.vertices[1]]);
            let omitted_projection = project(vertices[omitted]);
            assert_eq!((first, second, omitted_projection), (1.0, 1.0, -1.0));
            let explicit = edge_axis_separates(first, second, 0.0);
            assert!(explicit);
            assert_eq!(
                edge_axis_case_separates(case, vertices, edges, [0.0; 3]),
                explicit
            );
            assert!(edge_axes_separate(vertices, edges, [0.0; 3]));
            assert!(!edge_axis_separates(first, omitted_projection, 0.0));
        }
    }

    #[test]
    fn edge_axis_radius_helpers_preserve_all_fused_term_orders() {
        let box_large = f32::from_bits(0x46e9_a732);
        let box_small = f32::from_bits(0x3684_800d);
        let edge_large = f32::from_bits(0x42c6_0100);
        let edge_small = f32::from_bits(0x394d_862d);
        let x_edge = [0.0, edge_large, edge_small];
        let x_extent = [box_small, box_large, box_small];
        let y_edge = [edge_large, 0.0, edge_small];
        let y_extent = [box_large, box_small, box_small];
        let z_edge = [edge_large, edge_small, 0.0];
        let z_extent = [box_large, box_small, box_small];

        let fused = [
            edge_axis_radius_x(x_edge, x_extent),
            edge_axis_radius_y(y_edge, y_extent),
            edge_axis_radius_z(z_edge, z_extent),
        ];
        let scalar = [
            x_edge[2].abs() * x_extent[1] + x_edge[1].abs() * x_extent[2],
            y_edge[2].abs() * y_extent[0] + y_edge[0].abs() * y_extent[2],
            z_edge[1].abs() * z_extent[0] + z_edge[0].abs() * z_extent[1],
        ];
        assert_eq!(fused.map(f32::to_bits), [0x40bb_988d; 3]);
        assert_eq!(scalar.map(f32::to_bits), [0x40bb_988e; 3]);
    }

    #[test]
    fn edge_axis_separation_keeps_both_contact_boundaries_inclusive() {
        let radius = 1.0f32;
        let outside = f32::from_bits(radius.to_bits() + 1);
        assert!(!edge_axis_separates(radius, radius, radius));
        assert!(!edge_axis_separates(-radius, -radius, radius));
        assert!(edge_axis_separates(outside, outside, radius));
        assert!(edge_axis_separates(-outside, -outside, radius));
    }

    #[test]
    fn box_axis_extrema_include_each_triangle_vertex() {
        let extent = [1.0; 3];
        for carrier in 0..3 {
            let axis = carrier;
            let mut positive = [[0.0; 3]; 3];
            for vertex in &mut positive {
                vertex[axis] = 2.0;
            }
            assert!(box_axes_separate(positive, extent));
            positive[carrier][axis] = 0.0;
            assert!(!box_axes_separate(positive, extent));

            let mut negative = [[0.0; 3]; 3];
            for vertex in &mut negative {
                vertex[axis] = -2.0;
            }
            assert!(box_axes_separate(negative, extent));
            negative[carrier][axis] = 0.0;
            assert!(!box_axes_separate(negative, extent));
        }
    }

    #[test]
    fn plane_supports_follow_each_mixed_normal_sign() {
        let edge_0 = [1.0, 0.0, 1.0];
        let edge_1 = [0.0, 1.0, 1.0];
        let extent = [1.0, 2.0, 3.0];
        let normal = native_triangle_normal(edge_0, edge_1);
        assert_eq!(
            normal.map(f32::to_bits),
            [(-1.0f32).to_bits(), (-1.0f32).to_bits(), 1.0f32.to_bits()]
        );
        let (negative_support, positive_support) = plane_supports(normal, extent);
        assert_eq!(
            negative_support.map(f32::to_bits),
            [1.0f32.to_bits(), 2.0f32.to_bits(), (-3.0f32).to_bits()]
        );
        assert_eq!(
            positive_support.map(f32::to_bits),
            [(-1.0f32).to_bits(), (-2.0f32).to_bits(), 3.0f32.to_bits()]
        );
        assert!(triangle_plane_overlaps_box(
            [0.0; 3], edge_0, edge_1, extent
        ));
        assert!(!triangle_plane_overlaps_box(
            [0.0, 0.0, 7.0],
            edge_0,
            edge_1,
            extent
        ));
    }

    #[test]
    fn triangle_normal_preserves_native_fused_operation_order() {
        let edge_0 = [
            f32::from_bits(0x3cb4_4620),
            f32::from_bits(0xce81_3438),
            f32::from_bits(0x481e_c994),
        ];
        let edge_1 = [
            f32::from_bits(0xa9ab_af40),
            f32::from_bits(0xcd7b_3925),
            f32::from_bits(0x3eee_2730),
        ];
        let scalar = cross(edge_0, edge_1).map(f32::to_bits);
        let fused = native_triangle_normal(edge_0, edge_1).map(f32::to_bits);
        assert_eq!(scalar, [0x561b_d29e, 0xbc27_b4d7, 0xcab0_e8ff]);
        assert_eq!(fused, [0x561b_d29e, 0xbc27_b4d8, 0xcab0_e8ff]);
    }
}
