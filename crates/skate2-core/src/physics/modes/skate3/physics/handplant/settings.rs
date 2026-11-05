//! TU3 cached layouts55B5940075F78842 and0F38084037CB9175.
use skate3_core::point_graph::PointGraph;
use skate_data::collections::Collections;

pub(super) struct Settings {
    pub window: [PointGraph<4>; 7],
    pub depth: f32,
    pub window_drop: f32,
    pub time_warp: PointGraph<8>,
    pub out_heading: PointGraph<8>,
    pub into_rotation: PointGraph<8>,
    pub out_rotation: PointGraph<8>,
    pub hand_radius: PointGraph<4>,
    pub curve_half_time: f32,
    pub entry_blend: f32,
    pub hand_out: f32,
    pub hand_into: f32,
    pub hand_release: f32,
    pub minimum_speed: f32,
    pub minimum_slope: f32,
    pub rotation_time: f32,
    pub committed_time: f32,
    pub minimum_out_speed: f32,
    pub hand_approach: f32,
    pub direction_frames: i32,
    pub apex_radius: f32,
    pub apex_angle: f32,
    pub animation: [f32; 3],
    pub truck_distance: f32,
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let root = "Hash_55B5940075F78842";
        let graph4 = |class, name| -> Result<PointGraph<4>, String> {
            let w = data
                .words::<12>(class, "default", name)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: w[4..8].try_into().unwrap(),
                y: w[8..12].try_into().unwrap(),
            })
        };
        let graph8 = |name| -> Result<PointGraph<8>, String> {
            let w = data
                .words::<20>("physics_handplantmanager", "default", name)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: w[4..12].try_into().unwrap(),
                y: w[12..20].try_into().unwrap(),
            })
        };
        let f = |name| data.float("physics_handplantmanager", "default", name);
        Ok(Self {
            truck_distance: data.float("physics_grinds", "default", "DeckCenterToTruck")?,
            window: [
                graph4(root, "Hash_FF0DE8A42210E8C0")?,
                graph4(root, "Hash_B90C78786ED04A7D")?,
                graph4(root, "Hash_DC7947C6DD03CDAA")?,
                graph4(root, "Hash_0ACB7AAE2662FBB2")?,
                graph4(root, "Hash_CF7F668AB995380D")?,
                graph4(root, "Hash_8348DA199CC87B7A")?,
                graph4(root, "Hash_F27F622CAE33BD50")?,
            ],
            depth: data.float(root, "default", "Hash_3D9138A04C6D43F6")?,
            window_drop: data.float(root, "default", "Hash_A046D0516C0BD62B")?,
            time_warp: graph8("Hash_F551DF3289124FB0")?,
            out_heading: graph8("Hash_8FC9A4F9002CF9E0")?,
            into_rotation: graph8("Hash_F61B0FB73C39311B")?,
            out_rotation: graph8("Hash_D4EF764AF4D5F1CA")?,
            hand_radius: graph4("physics_handplantmanager", "Hash_5494E3F9BE572D6B")?,
            curve_half_time: f("Hash_2460D6544C3B64B7")?,
            entry_blend: f("Hash_CCD7A82CE1BBFFF2")?,
            hand_out: f("Hash_E0895C189D77CE16")?,
            hand_into: f("Hash_01EB855A0ACBD84C")?,
            hand_release: f("Hash_9C02B9E6998D1321")?,
            minimum_speed: f("Hash_63303CA6C611DC79")?,
            minimum_slope: f("Hash_D1622917F6DD215F")?,
            rotation_time: f("Hash_D595A32C5F02178F")?,
            committed_time: f("Hash_B5EC5A0941B407A4")?,
            minimum_out_speed: f("Hash_49C91803479B9BED")?,
            hand_approach: f("Hash_7C8B87F7435221C4")?,
            direction_frames: data.words::<1>(
                "physics_handplantmanager",
                "default",
                "Hash_DDE5BFBCB6A1AFD5",
            )?[0] as i32,
            apex_radius: f("Hash_A046D0516C0BD62B")?,
            apex_angle: f("Hash_90CDD541098AB4B5")?,
            animation: [
                data.float("anim_handplant", "default", "Hash_B9012B86EB5258FE")?,
                data.float("anim_handplant", "default", "Hash_512342FB1D4A7D57")?,
                data.float("anim_handplant", "default", "Hash_673F3B1BD056C39E")?,
            ],
        })
    }
}
