//! PC weather's pass: the rain (`rain::Rain`) over a copy of the scene.
use super::{GpuMap,RenderResult};
use crate::weather::Mode;


#[derive(Clone,Copy)]
pub(super) struct Frame {
    pub clip:[f32;16], pub camera:[f32;3], pub axes:[[f32;4];3],
    pub clock:f32,pub fog:[f32;4],pub mode:Mode,
}

pub(super) struct Weather {
    output:wgpu::TextureView, pub texture:wgpu::Texture,
    rain:super::rain::Rain,
}
impl Weather {
    #[expect(clippy::too_many_arguments,reason="the retained scene and depth resources")]
    pub fn new(device:&wgpu::Device,format:wgpu::TextureFormat,width:u32,height:u32,
        scene:&wgpu::TextureView,depth:&wgpu::TextureView,stencil:&wgpu::TextureView,multisampled:bool)->Self {
        let texture=device.create_texture(&wgpu::TextureDescriptor {
            label:Some("local weather scene"),size:wgpu::Extent3d{width,height,depth_or_array_layers:1},mip_level_count:1,sample_count:1,
            dimension:wgpu::TextureDimension::D2,format,usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::COPY_SRC,view_formats:&[],
        });
        let output=texture.create_view(&Default::default());
        let rain=super::rain::Rain::new(device,format,width,height,scene,depth,stencil,multisampled);
        Self{output,texture,rain}
    }
    pub fn record(&self,device:&wgpu::Device,queue:&wgpu::Queue,encoder:&mut wgpu::CommandEncoder,map:&GpuMap,frame:Frame)->RenderResult<()> {
        self.rain.record(device,queue,encoder,map,frame,&self.output)
    }
}
