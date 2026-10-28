//! ReplayEditorBackEnd::UpdateStall82567158 reads all three stream focuses.
//! Load cost is bytes / 133333 from cStreamFile::Update824e0f8c, not elapsed
//! frames. CalcStreamLoadStats824d6798 counts only the first eight entries,
//! omitting resident collections and reads that have already delivered bytes.
use skate2_formats::{stream::{CollectionSpace,CollectionMap,MAX_FOCUS_ENTRIES,merge_focus_collections},stream_xml::StreamXml};
use std::sync::Arc;

#[derive(Clone,Copy,Debug)]
pub struct Status {pub loaded:bool,pub ticks:u32}
impl Default for Status {fn default()->Self {Self {loaded:true,ticks:0}}}
impl Status {
    pub fn combine(self,other:Self)->Self {Self {loaded:self.loaded&&other.loaded,ticks:self.ticks.wrapping_add(other.ticks)}}
}
pub fn collection_ticks(index:usize,size:u32,loaded:bool,loaded_bytes:u32)->u32 {
    if index<8&&!loaded&&loaded_bytes==0 {size/133_333} else {0}
}

/// Immutable published renderer generation, retained while its next generation
/// loads. Membership follows the actual presentation/texture owners. Their PC
/// loader publishes a complete generation, rather than native per-asset reads.
#[derive(Clone)]
pub struct VisualFocus {
    pub space:Arc<CollectionSpace>,
    pub map:Arc<CollectionMap>,
    pub xml:Option<Arc<StreamXml>>,
    /// A loaded additional world's file of the type, which FindCollections824dfea8 merges
    /// with the first file's selection.
    pub additional:Option<(Arc<CollectionSpace>,Arc<CollectionMap>)>,
    pub radius:f32,
    pub resident:Vec<u64>,
    pub texture:bool,
}
impl VisualFocus {
    pub fn status(&self,position:[f32;3],xml_enabled:bool)->Result<Status,&'static str> {
        let mut selected=match self.xml.as_ref().filter(|_|xml_enabled) {
            Some(xml)=>xml.focus_collections(&self.space,position,self.radius,16),
            None=>self.space.focus_collections(position,self.radius,16),
        };
        if let Some((space,_))=&self.additional {
            selected=merge_focus_collections([selected,space.focus_collections(position,self.radius,MAX_FOCUS_ENTRIES)]);
            selected.truncate(16);
        }
        let mut loaded=0;let mut ticks=0u32;
        for (i,entry) in selected.iter().enumerate() {
            let resident=self.resident.contains(&entry.id);
            loaded+=usize::from(resident);
            let size=std::iter::once(&*self.map).chain(self.additional.as_ref().map(|(_,map)|&**map))
                .flat_map(|map|&map.collections).find(|c|c.id==entry.id).ok_or("replay visual focus collection missing from XSM")?.payload_size;
            ticks=ticks.wrapping_add(collection_ticks(i,size,resident,0));
        }
        // IsStreamFocusLoaded824d7a00/GameLoad requires all Pres/Sim, but
        // only one texture collection (or an empty texture focus).
        Ok(Status {loaded:if self.texture {selected.is_empty()||loaded>0} else {loaded==selected.len()},ticks})
    }
}
