//! ACSsetData_job (TU3 0x828D80B8), expressed in bones and typed allocations.
use super::buffers::{BoneBuffers, BoneSlice, BufferError, PoseBuffers};
use crate::animation::output::{NativeMatrix, Sqt};

#[derive(Clone, Copy, Debug)]
pub enum InternalUpdate<T> {
    /// Native opcode 12: byte-sized partial SET count.
    Set {
        source: BoneSlice<T>,
        bone_count: u8,
    },
    /// Every other opcode skips the SET copy, but still permits extraction.
    Preserve,
}
#[derive(Clone, Copy, Debug)]
pub struct CopyCommand<T> {
    pub update: InternalUpdate<T>,
    pub internal: BoneSlice<T>,
    pub external: Option<BoneSlice<T>>,
}
#[derive(Clone, Copy, Debug)]
pub enum SetDataCommand {
    Sqt(CopyCommand<Sqt>),
    Matrix(CopyCommand<NativeMatrix>),
}
/// 828D7E08 uses the low byte of numExtBones when the encoded byte is zero.
pub fn resolve_set_count(encoded_count: u8, external_bone_count: u32) -> u8 {
    if encoded_count == 0 {
        external_bone_count as u8
    } else {
        encoded_count
    }
}
pub(crate) fn execute(
    buffers: &mut PoseBuffers,
    bone_count: u16,
    commands: &[SetDataCommand],
) -> Result<(), BufferError> {
    for command in commands {
        match command {
            SetDataCommand::Sqt(command) => copy(&mut buffers.sqt, bone_count, command)?,
            SetDataCommand::Matrix(command) => copy(&mut buffers.matrices, bone_count, command)?,
        }
    }
    Ok(())
}
fn copy<T: Copy>(
    buffers: &mut BoneBuffers<T>,
    skeleton_bones: u16,
    command: &CopyCommand<T>,
) -> Result<(), BufferError> {
    if let InternalUpdate::Set { source, bone_count } = command.update {
        buffers.copy(source, command.internal, usize::from(bone_count))?;
    }
    if let Some(external) = command.external {
        // Read internal after SET, including writes through earlier aliases.
        buffers.copy(command.internal, external, usize::from(skeleton_bones))?;
    }
    Ok(())
}
