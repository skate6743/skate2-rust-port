//! Physical-owner adapter for selector82D913C0 requests8..11 and82BE7280.
mod settings;
use crate::skate3_native::physics::skeleton_controller::SkeletonControllerState;
use skate3_core::physics::skeleton_body::{
    SkeletonBody, SkeletonCollisionFeedback, SkeletonCollisionMode, SkeletonJoints,
};
use skate_data::collections::Collections;
use std::path::Path;

pub(crate) struct RagdollSetup {
    settings: settings::Settings,
}
impl RagdollSetup {
    pub fn load(data: &Collections, assets: &Path, bank_sha: &str) -> Result<Self, String> {
        Ok(Self {
            settings: settings::Settings::load(data, assets, bank_sha)?,
        })
    }

    pub fn request(
        &self,
        controller: &mut SkeletonControllerState,
        requested: u32,
        body: &mut SkeletonBody,
        joints: &mut SkeletonJoints,
        collision: &mut SkeletonCollisionMode,
    ) -> Result<(), String> {
        let Some(mode) = controller.select_request(requested) else {
            return Ok(());
        };
        if !(7..=10).contains(&mode) {
            return collision.select_driven(mode).map_err(str::to_owned);
        }
        //Request11 calls82BE3350(true), selecting normal limits while the
        //body still receives ragdoll masses, materials, drag and group6.
        let limits = if mode == 10 {
            &self.settings.normal_limits
        } else {
            &self.settings.ragdoll_limits
        };
        for (joint, limits) in joints.records.iter_mut().zip(limits) {
            joint.parameters.words[10..14].copy_from_slice(limits);
        }
        collision.apply_ragdoll_properties(
            body,
            self.settings.inverse_mass,
            self.settings.inverse_inertia,
            self.settings.drag,
            self.settings.materials,
        );
        collision.finish_ragdoll_request(mode);
        Ok(())
    }

    ///Original SetUpNormal82BE7280: limits, normal table/group, observation
    ///reset, zero drags, authored animated mass/inertia/material, collision mode.
    ///Call at the original physical reset owner, never at WipeoutExit alone.
    pub fn restore_normal(
        &self,
        body: &mut SkeletonBody,
        joints: &mut SkeletonJoints,
        collision: &mut SkeletonCollisionMode,
        feedback: &mut SkeletonCollisionFeedback,
    ) {
        for (joint, limits) in joints.records.iter_mut().zip(&self.settings.normal_limits) {
            joint.parameters.words[10..14].copy_from_slice(limits);
        }
        feedback.set_up_normal();
        collision.restore_normal_properties(body);
    }
}
