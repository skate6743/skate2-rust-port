//! Own both stock skater banks once. Clip/tree names are disjoint; poses retain
//! their bank identity because the banks share several reference-pose names.
use crate::{abin::Bank, animation_metadata::AnimationMetadata, sha256};
use std::{path::Path, sync::Arc};

pub struct AnimationBanks {
    pub banks: Vec<Arc<Bank>>,
    pub identities: Vec<String>,
}
impl AnimationBanks {
    pub fn load(asset_root: &Path) -> Result<Self, String> {
        let mut banks = Vec::new();
        let mut identities = Vec::new();
        for name in ["OnBoard", "OffBoard"] {
            let bank = Bank::load(&asset_root.join(format!("private/stock/data/anim/{name}.abin")))
                .map_err(|e| format!("{name}: {e}"))?;
            bank.validate_positional_parts()
                .map_err(|e| e.to_string())?;
            identities.push(sha256::digest(bank.bytes()));
            banks.push(Arc::new(bank));
        }
        let first = banks[0]
            .hierarchy()
            .ok_or("OnBoard has no animation hierarchy")?;
        let second = banks[1]
            .hierarchy()
            .ok_or("OffBoard has no animation hierarchy")?;
        if !first.compatible_with(second) {
            return Err("Skater animation bank hierarchies differ".into());
        }
        Ok(Self { banks, identities })
    }
    pub fn metadata(&self) -> Result<AnimationMetadata, String> {
        let mut result = AnimationMetadata::from_bank(
            &self.banks[0],
            "OnBoard.abin".into(),
            self.identities[0].clone(),
        )?;
        result.merge(AnimationMetadata::from_bank(
            &self.banks[1],
            "OffBoard.abin".into(),
            self.identities[1].clone(),
        )?)?;
        Ok(result)
    }
}
