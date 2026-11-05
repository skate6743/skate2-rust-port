//! Cached physics_footplantmanager layout used by82D6FD60/82D6F5F0/82D70070.
use skate_data::collections::Collections;
use skate3_core::point_graph::PointGraph;
#[derive(Clone)]
pub(super) struct Settings {
    pub deck_bounds: [f32; 4],      //0
    pub max_leg_angle_error: f32,   //280 degrees
    pub leg_length_on_landing: f32, //288
    pub foot_volume_y_offset: f32,  //296
    pub deck_bounds_y_offset: f32,  //300
    pub radial_speed_scale: PointGraph<8>, //16
    pub max_horizontal_speed: f32, //256
    pub max_descending_speed: f32, //260
    pub release_outward_speed: f32, //268
    pub end_handle: f32, //272
    pub end_angle: f32, //276 degrees
    pub end_leg_length: f32, //284
    pub start_handle: f32, //292
    pub min_duration: f32, //304
    pub max_duration: f32, //308
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let float = |name| data.float("physics_footplantmanager", "default", name);
        let graph = data.words::<20>("physics_footplantmanager", "default", "Hash_8A8887218466ED15")?.map(f32::from_bits);
        Ok(Self {
            deck_bounds: data
                .words::<4>("physics_footplantmanager", "default", "DeckBB")?
                .map(f32::from_bits),
            max_leg_angle_error: float("MaxLegAngleError")?,
            leg_length_on_landing: float("LegLengthOnLanding")?,
            foot_volume_y_offset: float("FootVolumeYOffset")?,
            deck_bounds_y_offset: float("DeckBBYOffset")?,
            radial_speed_scale: PointGraph { x: graph[4..12].try_into().unwrap(), y: graph[12..20].try_into().unwrap() },
            max_horizontal_speed: float("Hash_149A5D0D4768D2B1")?,
            max_descending_speed: float("Hash_52436D9F4FE56BCB")?,
            release_outward_speed: float("Hash_C5F92168DFAFD0B8")?,
            end_handle: float("Hash_628C228DCC7B6ED5")?,
            end_angle: float("Hash_4F45DA86685CF029")?,
            end_leg_length: float("Hash_0DE0027B9913C5D5")?,
            start_handle: float("Hash_5C4341EF35F4B2B0")?,
            min_duration: float("Hash_D6224B710A36D643")?,
            max_duration: float("Hash_68C05A7FCC6BB718")?,
        })
    }
}
