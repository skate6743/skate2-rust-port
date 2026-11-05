use super::{NativeMatrix, PoseBufferError};

/// TU3 0x828D3B58: local matrices to hierarchy-composed matrices.
///
/// `detached_parent` is native r5 (hierarchy +8 at the caller). A parent equal to
/// it, or -1, causes a verbatim local copy. No world transform is introduced.
/// The first matrix is copied even for a nonpositive count. Other output slots
/// retain caller state until visited, including when a forward parent is read.
pub fn compose_hierarchy(
    bone_count: i32,
    parents: &[i32],
    detached_parent: i32,
    local: &[NativeMatrix],
    destination: &mut [NativeMatrix],
) -> Result<(), PoseBufferError> {
    let count = bone_count.max(0) as usize;
    validate(
        count,
        parents,
        detached_parent,
        local.len(),
        destination.len(),
    )?;
    destination[0] = local[0];
    for bone in 0..count {
        let parent = parents[bone];
        destination[bone] = if parent == -1 || parent == detached_parent {
            local[bone]
        } else {
            concatenate(local[bone], destination[parent as usize])
        };
    }
    Ok(())
}

/// The same native function with identical source/destination matrix pointers.
pub fn compose_hierarchy_in_place(
    bone_count: i32,
    parents: &[i32],
    detached_parent: i32,
    matrices: &mut [NativeMatrix],
) -> Result<(), PoseBufferError> {
    let count = bone_count.max(0) as usize;
    validate(
        count,
        parents,
        detached_parent,
        matrices.len(),
        matrices.len(),
    )?;
    for bone in 0..count {
        let parent = parents[bone];
        if parent != -1 && parent != detached_parent {
            matrices[bone] = concatenate(matrices[bone], matrices[parent as usize]);
        }
    }
    Ok(())
}

fn validate(
    count: usize,
    parents: &[i32],
    detached_parent: i32,
    input_len: usize,
    output_len: usize,
) -> Result<(), PoseBufferError> {
    if input_len < count.max(1) {
        return Err(PoseBufferError::ShortInput);
    }
    if output_len < count.max(1) {
        return Err(PoseBufferError::ShortOutput);
    }
    if parents.len() < count {
        return Err(PoseBufferError::ShortParents);
    }
    for (bone, &parent) in parents[..count].iter().enumerate() {
        if parent != -1
            && parent != detached_parent
            && (parent < 0 || parent as usize >= output_len)
        {
            return Err(PoseBufferError::InvalidParent { bone, parent });
        }
    }
    Ok(())
}

fn concatenate(local: NativeMatrix, parent: NativeMatrix) -> NativeMatrix {
    let mut result = local;
    for row in 0..4 {
        for lane in 0..4 {
            let first = if row == 3 {
                parent[0][lane].mul_add(local[row][0], parent[3][lane])
            } else {
                parent[0][lane] * local[row][0]
            };
            let second = parent[1][lane].mul_add(local[row][1], first);
            result[row][lane] = parent[2][lane].mul_add(local[row][2], second);
        }
    }
    result
}
