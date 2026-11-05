//! TU3 Hall of Meat authored bones and curves. S2 caller offsets are translated
//! by stable field hashes; S3 added curves and moved the shared fields.
use crate::physics::PointGraph;
pub(super) const CLASS:u64=0x9e4c_471b_c6ea_b615;
const NAME:&str="Hash_9E4C471BC6EAB615";
// S2 host caller offsets -> shared field identities in the actual S3 schema.
const CURVES:[(usize,&str);6]=[
    (2100,"Hash_C2D28EFB70B37684"), // heavy landing damage (S3 offset2420)
    (2180,"Hash_96CDCC590988982E"), // airtime (S3 offset2500)
    (2260,"Hash_32B1A3C2961F4E6E"), // drop height (S3 offset2580)
    (2340,"Hash_B2A997F8579B1998"), // bail speed (S3 offset2820)
    (2420,"Hash_6CD9F0CD7675949C"), // rotation (S3 offset2900)
    (2500,"Hash_2451424AD330B08F"), // moving bail duration (S3 offset2980)
];
// The full Skate 3 layout (skaterschema): PointNegGraphData8 curves from +2100, 80 bytes apart,
// as TU3's WipeoutScorer reads them by offset.
const S3_CURVES:[&str;12]=["Hash_A9B81D5BF44762E4","Hash_95AE883C533FC1EB","Hash_2E61C1BB9D8A8A95",
    "Hash_05B6EAA1D6A19CCE","Hash_C2D28EFB70B37684","Hash_96CDCC590988982E","Hash_32B1A3C2961F4E6E",
    "Hash_36274FBAE087E832","Hash_1D49983B40C59E5A","Hash_B2A997F8579B1998","Hash_6CD9F0CD7675949C",
    "Hash_2451424AD330B08F"];
/// One authored body tweak (BodyTweakAttribData): ID +0, direction +8/+12, hold frames +16, points +20.
#[derive(Clone,Copy,Debug)]
pub(crate) struct Tweak { pub id:i32, pub direction:[f32;2], pub frames:u32, pub points:u32 }
pub(crate) struct Data {
    bones:[u32;525],curves:[PointGraph<8>;6],s3_curves:[PointGraph<8>;12],
    /// +3060 severe bone-score threshold, +3064 toe damage scale, +3068 damage scale, +3072 fade.
    pub severe_threshold:u32,pub toe_scale:f32,pub damage_scale:f32,pub damage_fade:f32,
    pub tweak_threshold:f32,pub tweaks:Vec<Tweak>,
}
impl Data {
    pub(super) fn load(collections:&skate_data::collections::Collections)->Result<Self,String> {
        let bones=collections.words::<525>(NAME,"default","Hash_1F9B5EA147CBEEE9")?;
        let graph=|field:&str|->Result<PointGraph<8>,String> {
            let words=collections.words::<20>(NAME,"default",field)?;
            Ok(PointGraph {x:std::array::from_fn(|i|f32::from_bits(words[i+4])),
                y:std::array::from_fn(|i|f32::from_bits(words[i+12]))})
        };
        let mut curves=std::array::from_fn(|_|PointGraph::<8>::default());
        for (i,(_,field)) in CURVES.iter().enumerate() {curves[i]=graph(field)?;}
        let mut s3_curves=std::array::from_fn(|_|PointGraph::<8>::default());
        for (i,field) in S3_CURVES.iter().enumerate() {s3_curves[i]=graph(field)?;}
        let word=|field:&str|collections.words::<1>(NAME,"default",field).map(|w|w[0]);
        let tweaks=collections.raw_items(NAME,"default","Hash_5717CFCCCA59BD47")?.iter().map(|item| {
            let u=|at:usize|->Result<u32,String> {Ok(u32::from_be_bytes(item.get(at..at+4)
                .ok_or("Short Skate 3 body tweak")?.try_into().expect("four bytes")))};
            Ok(Tweak {id:u(0)? as i32,direction:[f32::from_bits(u(8)?),f32::from_bits(u(12)?)],frames:u(16)?,points:u(20)?})
        }).collect::<Result<Vec<_>,String>>()?;
        Ok(Self{bones,curves,s3_curves,severe_threshold:word("Hash_8FB03ECD5DA26421")?,
            toe_scale:f32::from_bits(word("Hash_72A3CCC3C75697CD")?),
            damage_scale:f32::from_bits(word("Hash_B7514DE7EBDE9D01")?),
            damage_fade:f32::from_bits(word("Hash_D6CB5A9288BE7AC1")?),
            tweak_threshold:f32::from_bits(word("Hash_9C494E9614FA5ED6")?),tweaks})
    }
    pub(crate) fn bone_word(&self,offset:usize)->Option<u32> {
        (offset%4==0).then(||self.bones.get(offset/4).copied()).flatten()
    }
    /// A curve at its Skate 3 layout offset (+2100..+2980).
    pub(crate) fn s3_curve(&self,offset:usize,x:f32)->f32 {
        let index=offset.checked_sub(2100).filter(|o|o%80==0).map(|o|o/80);
        index.and_then(|i|self.s3_curves.get(i)).map_or(0.,|graph|graph.evaluate(x))
    }
    pub(super) fn curve(&self,offset:usize,x:f32)->Option<f32> {
        CURVES.iter().position(|&(o,_)|o==offset).map(|i|self.curves[i].evaluate(x))
    }
}
pub(crate) fn data()->Result<&'static Data,String> {
    static DATA:std::sync::OnceLock<Data>=std::sync::OnceLock::new();
    crate::skate3_native::physics::player::linked_once(&DATA, || {
        let collections=crate::skate3_native::physics::player::stock_collections()?;
        Data::load(&collections)
    })
}

#[cfg(test)]
mod tests {
    /// `SKATE3_DATA` names a Skate 3 folder: its Hall of Meat curves, scalars, bones and
    /// body tweaks load from the linked data (TU3 layout +2100..+3072).
    #[test]
    fn linked_hall_of_meat_data() {
        let Some(root) = std::env::var_os("SKATE3_DATA") else { return };
        let linked = crate::skate3::Skate3Data::open(std::path::Path::new(&root)).unwrap();
        crate::skate3::set_linked(Some(std::sync::Arc::new(linked)));
        let data = super::data().unwrap();
        for curve in &data.s3_curves {
            assert!(curve.x.windows(2).all(|pair| pair[0] <= pair[1]), "{:?}", curve.x);
            assert!(curve.y.iter().any(|&y| y > 0.0));
        }
        // Bail speed scores from 10 m/s, a bone's first damage level at 9.
        assert_eq!(data.s3_curve(2820, 9.9), 0.0);
        assert!(data.s3_curve(2820, 12.5) > 0.0);
        assert_eq!(f32::from_bits(data.bone_word(5 * 84 + 40).unwrap()), 9.0);
        assert!(data.damage_scale > 0.0 && data.toe_scale > 0.0);
        assert_eq!(data.tweaks.len(), 4);
    }
}
