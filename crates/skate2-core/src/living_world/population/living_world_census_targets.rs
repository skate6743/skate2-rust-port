//! GetPointCensusInfo82702a28 and GetCensusTargetInfo8271e738. Reuses the
//! existing WorldPainter/Vault owners and original weighted member arrays.
use super::CensusState;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    world_painter_layer::{WorldPainterLayerError, WorldPainterLayerMan},
};
use std::sync::Arc;
pub(crate) const CENSUS_CLASS: u64 = 0x1b7b_a277_2f01_1bcd;
pub(crate) const GROUPS_CLASS: u64 = 0xd4dd_1d82_b998_1bc5;
pub(crate) const CATEGORIES_CLASS: u64 = 0xadcb_4d6b_d3b4_f85e;
const MEMBERS: u64 = 0xd5e1_267e_2d71_5124;

#[derive(Clone, Copy)]
pub(crate) enum PopulationKind {
    Pedestrian,
    Vehicle,
}
#[derive(Debug)]
pub(crate) enum CensusTargetError<E> {
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Painter(WorldPainterLayerError),
    Random(E),
    InvalidPosition,
    EmptyGroup(u64),
    ProbabilityCapacity(u32),
    InvalidRandomFraction,
}
/// The source uses TWO different RNG occurrences. Weighted group selection
/// consumes rw::math::Random; the member choice consumes Manager's Sk8::Random.
pub(crate) trait CensusRandom {
    type Error;
    fn global_word(&mut self) -> Result<u32, Self::Error>;
    fn manager_fraction(&mut self) -> Result<f32, Self::Error>;
}
pub(crate) struct LiveCensusRandom<'a>(
    pub(crate) &'a mut crate::native_skater_random::NativeSkaterRandom,
);
#[derive(Debug)]
pub(crate) enum CensusRandomError {
    Global(crate::native_skater_random::rw_math::RwRandomLockError),
    Manager(crate::native_skater_random::NativeRandomLocked),
}
impl CensusRandom for LiveCensusRandom<'_> {
    type Error = CensusRandomError;
    fn global_word(&mut self) -> Result<u32, Self::Error> {
        crate::native_skater_random::rw_math::global_next().map_err(CensusRandomError::Global)
    }
    fn manager_fraction(&mut self) -> Result<f32, Self::Error> {
        self.0
            .try_living_world_range(
                &crate::native_skater_random::NativeRandomGate::production_shared(),
                0.,
                1.,
            )
            .map_err(CensusRandomError::Manager)
    }
}
#[derive(Debug, PartialEq)]
pub(crate) struct PointCensus {
    pub(crate) group: u64,
    pub(crate) spawn_minimum: u32,
}
impl CensusState {
    pub(crate) fn point_info<RandomError>(
        &self,
        kind: PopulationKind,
        position: [f32; 3],
        vault: &Arc<AttributeRuntimeVault>,
        painter: &WorldPainterLayerMan,
        cvars: &InputCvarReferences,
    ) -> Result<PointCensus, CensusTargetError<RandomError>> {
        use CensusTargetError as E;
        let mut result = PointCensus {
            group: 0xd7ed_bd36_2d7d_2152,
            spawn_minimum: 0,
        };
        let (layer, fallback, slider) = match kind {
            PopulationKind::Pedestrian => (2, 0x5b6a_0051_dd6a_295c, self.pedestrian_slider),
            PopulationKind::Vehicle => (3, 0x8be1_d151_f9da_2d29, self.vehicle_slider),
        };
        // PC port: inside a linked Skate 3 district, TU3's GetPointCensusInfo (0x826b8a28) reads
        // the district's painted layer: its key names a census of Skate 3's database, and an
        // unpainted point (key 0) has none, so nothing spawns there. The census's spawn minimum
        // stands; Skate 3's groups name Skate 3's people, so the default census's group stands
        // in for who spawns.
        let skate3_minimum = if crate::skate3::district_active() {
            let key = painter
                .try_skate3_key(layer, [position[0], position[2]])
                .map_err(E::Painter)?;
            let minimum = crate::skate3::linked()
                .filter(|data| key != 0 && data.db.has_collection(CENSUS_CLASS, key))
                .map_or(0, |data| data.db.layout_word(CENSUS_CLASS, key, 6));
            static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_PAINTER").is_some()) {
                eprintln!("census: skate 3 layer {layer} at {position:?}: key {key:016x}, spawn minimum {minimum}");
            }
            Some(minimum)
        } else {
            None
        };
        let key = if skate3_minimum.is_some()
            || cvars
                .try_ignore_world_painter_entity_categories()
                .map_err(E::Cvar)?
        {
            fallback
        } else {
            painter
                .try_attrib_key(layer, [position[0], position[2]])
                .map_err(E::Painter)?
        };
        if key != 0 {
            let instance = vault.instance(CENSUS_CLASS, key);
            if let Some(layout) = instance
                .try_cached_layout_field(0, 32)
                .map_err(E::Attribute)?
            {
                result.group = read_u64(layout.bytes(), 8);
                result.spawn_minimum = read_u32(layout.bytes(), 24);
            } else {
                // Generated constructor uses DefaultDataArea(32), all zero.
                result.group = 0;
            }
        }
        if let Some(minimum) = skate3_minimum {
            result.spawn_minimum = minimum;
        }
        if !cvars.try_ignore_fe_census_sliders().map_err(E::Cvar)? {
            result.spawn_minimum = (result.spawn_minimum as f32 * slider) as u32;
        }
        Ok(result)
    }
    pub(crate) fn target<R: CensusRandom>(
        &self,
        kind: PopulationKind,
        position: [f32; 3],
        captured_count: u32,
        vault: &Arc<AttributeRuntimeVault>,
        painter: &WorldPainterLayerMan,
        cvars: &InputCvarReferences,
        random: &mut R,
    ) -> Result<Option<u64>, CensusTargetError<R::Error>> {
        use CensusTargetError as E;
        if position.iter().any(|x| x.is_nan()) {
            return Err(E::InvalidPosition);
        }
        let point = self.point_info(kind, position, vault, painter, cvars)?;
        if point.spawn_minimum == 0 || captured_count >= point.spawn_minimum {
            return Ok(None);
        }
        let group = vault.instance(GROUPS_CLASS, point.group);
        let members = group
            .try_reference_members(MEMBERS, true)
            .map_err(E::Attribute)?;
        let members = members
            .filter(|m| m.len() != 0)
            .ok_or(E::EmptyGroup(point.group))?;
        if members.len() > 32 {
            return Err(E::ProbabilityCapacity(members.len()));
        }
        let probabilities: Vec<f32> = (0..members.len())
            .map(|i| f32::from_bits(read_u32(members.get(i).expect("member within count"), 24)))
            .collect();
        let word = random.global_word().map_err(E::Random)?;
        let Some(index) = discrete_choice(&probabilities, word) else {
            return Ok(None);
        };
        let category_key = read_u64(members.get(index as u32).expect("selected group"), 8);
        let category = vault.instance(CATEGORIES_CLASS, category_key);
        let Some(entities) = category
            .try_reference_members(MEMBERS, false)
            .map_err(E::Attribute)?
        else {
            return Ok(None);
        };
        if entities.len() == 0 {
            return Ok(None);
        }
        let fraction = random.manager_fraction().map_err(E::Random)?;
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(E::InvalidRandomFraction);
        }
        // This is intentionally percentage truncation followed by modulo,
        // not floor(fraction*length). Raw fmuls/fctiwz/divwu8271e9ec..ea04.
        let index = ((fraction * 100.) as u32) % entities.len();
        Ok(Some(read_u64(
            entities.get(index).expect("modulo nonempty"),
            8,
        )))
    }
}
/// Math.DiscreteRandom82704880: integer draw1..100, no normalization or
/// fallback to the last entry when authored probabilities sum to less than1.
pub(crate) fn discrete_choice(probabilities: &[f32], random_word: u32) -> Option<usize> {
    let draw = (random_word % 100 + 1) as f32;
    let mut cumulative = 0.;
    for (index, probability) in probabilities.iter().enumerate() {
        cumulative = probability + cumulative;
        // PPC ble tests !GT, including unordered; don't invert it to >=.
        if !(draw > cumulative * 100.) {
            return Some(index);
        }
    }
    None
}
/// `Math::DiscreteRandom<xvector<VecFloat,32>>` 0x82d1e7d8: the same 1..100 draw against the
/// running sum times 100, taken where that is at least the draw (`vcmpgefp`: NaN passes on).
pub(crate) fn vector_discrete_choice(probabilities: &[f32], random_word: u32) -> Option<usize> {
    let draw = (random_word % 100 + 1) as f32;
    let mut cumulative = 0.;
    for (index, probability) in probabilities.iter().enumerate() {
        cumulative += probability;
        if cumulative * 100. >= draw {
            return Some(index);
        }
    }
    None
}
/// `CensusUtils::ResolveCategoryGroupToEntity` 0x82e60238: a member category of the group
/// (`ped_class_business` for a group without members) by weight (`rw::math::Random`, the first
/// without a pick), then one of its entities (`LW::GetRandom(random, 0, count)` of the Manager's
/// RNG), `business_male01` for a category without one.
pub(crate) fn resolve_category_group_to_entity(group_key:u64,vault:&Arc<AttributeRuntimeVault>,
    random:&mut crate::native_skater_random::NativeSkaterRandom)->Result<u64,String>{
    use skate2_formats::attrib::string_hash64 as hash;
    let mut group=vault.instance(GROUPS_CLASS,group_key);
    if group.try_length(MEMBERS).map_err(|e|e.to_string())?==0{
        group=vault.instance(GROUPS_CLASS,hash("ped_class_business"));
    }
    let count=group.try_length(MEMBERS).map_err(|e|e.to_string())?;
    let members=group.try_reference_members(MEMBERS,true).map_err(|e|e.to_string())?;
    let member=|index:u32|members.as_ref().and_then(|members|members.get(index));
    let weights:Vec<f32>=(0..count).map(|index|member(index).map_or(0.,|data|f32::from_bits(read_u32(data,24)))).collect();
    let word=crate::native_skater_random::rw_math::global_next().map_err(|e|format!("category group RNG: {e:?}"))?;
    let index=vector_discrete_choice(&weights,word).unwrap_or(0) as u32;
    let category=vault.instance(CATEGORIES_CLASS,member(index).map_or(0,|data|read_u64(data,8)));
    let entities=category.try_reference_members(MEMBERS,false).map_err(|e|e.to_string())?;
    let length=category.try_length(MEMBERS).map_err(|e|e.to_string())?;
    if length==0{return Ok(hash("business_male01"));}
    #[expect(clippy::cast_precision_loss,clippy::cast_possible_truncation,reason="LW::GetRandom's int range")]
    let pick=random.try_living_world_range(&crate::native_skater_random::NativeRandomGate::production_shared(),0.,length as f32)
        .map_err(|e|format!("category entity RNG: {e:?}"))? as i32;
    Ok(u32::try_from(pick).ok().and_then(|pick|entities.as_ref()?.get(pick)).map_or(0,|data|read_u64(data,8)))
}
///Math.ScaledDiscreteRandom82704940: separate rounded multiplies, then
///scalar ble (!GT) threshold. Caller consumes global RNG even for empty input.
pub(crate) fn scaled_discrete_choice(probabilities:&[f32],total:f32,random_word:u32)->Option<usize>{
    let product=random_word as f32*total;
    let draw=product*f32::from_bits(0x2f80_0000);
    let mut cumulative=0.;
    for (index,probability) in probabilities.iter().enumerate(){
        cumulative=*probability+cumulative;
        if !(draw>cumulative){return Some(index);}
    }
    None
}
///8272199c..82721be4 prop-specific population selection. Group weighting is
///multiplied by each member entity's FIRST matching prop-use probability.
///Unlike ordinary target(), every category member is considered in order.
pub(crate) fn prop_target(group_key:u64,source_key:u64,vault:&Arc<AttributeRuntimeVault>)
    ->Result<Option<u64>,String>{
    const USE:u64=0xf893_23e4_20a6_aaa3;
    const USE_TYPE:u64=0xe4aa_c2ea_bc05_3d0a;
    let group=vault.instance(GROUPS_CLASS,group_key);
    let mut keys=Vec::new();
    let mut probabilities=Vec::new();
    let mut total=0.;
    if let Some(groups)=group.try_reference_members(MEMBERS,true).map_err(|e|e.to_string())?{
        for index in 0..groups.len(){
            let member=groups.get(index).ok_or("prop category group member absent")?;
            let category=vault.instance(CATEGORIES_CLASS,read_u64(member,8));
            let group_weight=f32::from_bits(read_u32(member,24));
            let Some(entities)=category.try_reference_members(MEMBERS,false).map_err(|e|e.to_string())?
                else{continue;};
            for index in 0..entities.len(){
                let entity_key=read_u64(entities.get(index).ok_or("prop category entity absent")?,8);
                let entity=vault.instance(crate::living_world_grind_services::dmo_objects::ENTITIES_CLASS,entity_key);
                for index in 0..entity.try_length(USE).map_err(|e|e.to_string())?{
                    let key=entity.try_array_field_data(USE,index,USE_TYPE,32,8..16).map_err(|e|e.to_string())?
                        .map_or(0,|d|u64::from_be_bytes(d.bytes().try_into().expect("prop key")));
                    // Native reads probability before comparing full64 key.
                    let probability=entity.try_array_field_data(USE,index,USE_TYPE,32,24..28).map_err(|e|e.to_string())?
                        .map_or(0.,|d|f32::from_be_bytes(d.bytes().try_into().expect("prop probability")));
                    if key!=source_key{continue;}
                    if keys.len()==32{return Err("native prop census probability capacity32 exceeded".into());}
                    let weight=probability*group_weight;
                    keys.push(entity_key);probabilities.push(weight);total=weight+total;
                    break;
                }
            }
        }
    }
    let word=crate::native_skater_random::rw_math::global_next().map_err(|e|format!("prop census RNG: {e:?}"))?;
    Ok(scaled_discrete_choice(&probabilities,total,word).map(|index|keys[index]))
}
fn read_u32(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(data[at..at + 4].try_into().expect("checked field"))
}
fn read_u64(data: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(data[at..at + 8].try_into().expect("checked field"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_probability_gaps_reject_and_boundary_is_inclusive() {
        assert_eq!(discrete_choice(&[0.25, 0.25], 24), Some(0));
        assert_eq!(discrete_choice(&[0.25, 0.25], 25), Some(1));
        assert_eq!(discrete_choice(&[0.25, 0.25], 49), Some(1));
        assert_eq!(discrete_choice(&[0.25, 0.25], 50), None);
        assert_eq!(discrete_choice(&[0., 1.], 99), Some(1));
    }
}
