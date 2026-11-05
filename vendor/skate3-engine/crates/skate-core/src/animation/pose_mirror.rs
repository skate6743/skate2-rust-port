//! ACS immediate Mirror828CDAF8. Pairing comes from the stock hierarchy.
use super::{output::Sqt, pose_trajectory::multiply};

pub fn mirror(
    pose: &mut [Sqt],
    parents: &[i32],
    partners: &[i32],
    trajectory_mode: u32,
) -> Result<(), String> {
    if pose.len() != parents.len() || pose.len() != partners.len() {
        return Err("Mirror pose and hierarchy dimensions differ".into());
    }
    for (i, &partner) in partners.iter().enumerate() {
        if partner < -1 || partner >= pose.len() as i32 {
            return Err(format!("Invalid mirror partner for bone{i}"));
        }
        // -1 excludes a bone; the smaller index processes both sides of a pair.
        if partner < i as i32 {
            continue;
        }
        if trajectory_mode == 1 && (i == 0 || parents[i] == 0) {
            if i != 0 {
                // Literal quaternion8232F740, before the alternate reflection.
                pose[i].rotation = multiply(pose[i].rotation, [0.0, 1.0, 0.0, 0.0]);
            }
            pose[i].rotation[1] *= -1.0;
            pose[i].rotation[2] *= -1.0;
            pose[i].translation[0] *= -1.0;
        } else {
            reflect(&mut pose[i]);
        }
        if partner > i as i32 {
            pose.swap(i, partner as usize);
            reflect(&mut pose[i]);
        }
    }
    Ok(())
}

fn reflect(pose: &mut Sqt) {
    pose.rotation[0] *= -1.0;
    pose.rotation[1] *= -1.0;
    pose.translation[2] *= -1.0;
}
