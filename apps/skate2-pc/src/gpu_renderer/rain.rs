//! PC rain adapter: supplied friend rain ribbons, photographed sky and isolated lightning.
//! Geometry-derived basin levels and shared wheel contacts replace the old wet coat.
//! Open-source water research/notices are recorded in the basin evidence document.
use super::{CameraRelativeClip,GpuMap,GpuResourceBuilder,GpuTexture,RenderResult,weather::Frame};
use crate::{weather::Mode,weather_world::World};
use std::sync::{Arc,OnceLock};
use wgpu::util::DeviceExt;
#[path="rain_sky.rs"]mod sky;
const RIBBON_ATTRIBUTES:[wgpu::VertexAttribute;3]=wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x4,2=>Float32x2];
/// A streak instance: head and length, fall direction and opacity, physical half width.
const STREAK_ATTRIBUTES:[wgpu::VertexAttribute;3]=wgpu::vertex_attr_array![3=>Float32x4,4=>Float32x4,5=>Float32];
const STREAK_BYTES:usize=36;
/// The storm's drops at full intensity.
const RAIN_STREAKS:usize=6240;

pub(crate) type LoadingSurface=Arc<OnceLock<Option<Arc<Surface>>>>;
/// `SKATE2_TRACE_RAIN_TIMING`: the rain pass's CPU stages, averaged over 600 frames.
struct RainTiming {last:Option<std::time::Instant>,stages:[f64;6]}
static RAIN_TIMING:std::sync::Mutex<([f64;6],u32)>=std::sync::Mutex::new(([0.;6],0));
impl RainTiming {
    fn start()->Self {
        static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        let enabled=*ENABLED.get_or_init(||std::env::var_os("SKATE2_TRACE_RAIN_TIMING").is_some());
        Self {last:enabled.then(std::time::Instant::now),stages:[0.;6]}
    }
    fn mark(&mut self,stage:usize) {
        if let Some(last)=self.last {let now=std::time::Instant::now();self.stages[stage]+=(now-last).as_secs_f64()*1000.;self.last=Some(now);}
    }
    fn finish(self) {
        if self.last.is_none() {return;}
        let Ok(mut total)=RAIN_TIMING.lock() else {return};
        for (sum,value) in total.0.iter_mut().zip(self.stages) {*sum+=value;}
        total.1+=1;
        if total.1==600 {
            let average=total.0.map(|sum|sum/600.);
            eprintln!("rain timing ms: clock+hydrology {:.3} uniforms {:.3} streaks {:.3} spray {:.3} upload {:.3} pass {:.3}",
                average[0],average[1],average[2],average[3],average[4],average[5]);
            *total=([0.;6],0);
        }
    }
}

/// Rain streaks and splashes a frame can draw (the storm's drops and its spray).
const RAIN_QUADS:usize=6240+256;
/// A streak vertex: position, colour and UV as f32.
const VERTEX_BYTES:usize=36;
pub(crate) fn prepare(builder:&GpuResourceBuilder,world:World,cube:Option<GpuTexture>,target:&LoadingSurface)->RenderResult<()> {
    let target=Arc::downgrade(target);let builder=builder.clone();
    std::thread::Builder::new().name("rain map preparation".into()).spawn(move|| {
        if target.strong_count()==0 {return;}
        let start=std::time::Instant::now();let Some(world)=world.finish_geometry_while(||target.strong_count()>0) else {return;};
        if let Some(target)=target.upgrade(){let _=target.set(upload(&builder,world,cube));
            eprintln!("rain map prepared in {:.3}s; native map publication was independent",start.elapsed().as_secs_f64());}
    })?;Ok(())
}
pub(crate) struct Surface {
    nodes:wgpu::Buffer,triangles:wgpu::Buffer,distribution:wgpu::Buffer,
    cube:Option<GpuTexture>,binding:OnceLock<wgpu::BindGroup>,world:Arc<World>,
}
pub(crate) fn upload(builder:&GpuResourceBuilder,mut world:World,cube:Option<GpuTexture>)->Option<Arc<Surface>> {
    let mut nodes=vec![0u8;32];nodes[12..16].copy_from_slice(&u32::MAX.to_ne_bytes());nodes[28..32].copy_from_slice(&1u32.to_ne_bytes());
    let triangles=vec![0u8;64];
    let upload=|label,contents:&[u8]|builder.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label:Some(label),contents,usage:wgpu::BufferUsages::STORAGE});
    let data=vec![0u8;16];
    eprintln!("rain receivers: triangles={} patches={} curves={}",world.triangles.len(),world.patches.len(),world.curves.len());
    world.release_build_scratch();
    Some(Arc::new(Surface{nodes:upload("rain receiver BVH",&nodes),triangles:upload("rain receiver triangles",&triangles),
        distribution:upload("rain continuous puddle CDF",&data),cube:cube.filter(|t|t.cube),binding:OnceLock::new(),world:Arc::new(world)}))
}
/// The drops update on their own thread, a frame ahead of the pass that draws them: each frame
/// takes the streaks the thread built from the last frame's camera and hands it this frame's.
/// The render thread only waits on a finished build, and the idle thread sleeps rather than spin.
struct RainWorker {
    requests:std::sync::mpsc::Sender<RainRequest>,
    results:std::sync::mpsc::Receiver<Vec<crate::weather_rain::DropInstance>>,
    pending:bool,
}
struct RainRequest {
    camera:skate2_core::world_particles::ParticleCamera,
    dt:f32,
    state:crate::weather_runtime::Snapshot,
    world:Arc<World>,
}
impl RainWorker {
    fn spawn()->RenderResult<Self> {
        let (requests,receive)=std::sync::mpsc::channel::<RainRequest>();
        let (send,results)=std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new().name("rain".into()).spawn(move || {
            let mut rain=crate::weather_rain::Rain::default();
            while let Ok(request)=receive.recv() {
                if send.send(rain.streaks(&request.camera,request.dt,request.state,&request.world)).is_err() {break;}
            }
        })?;
        Ok(Self {requests,results,pending:false})
    }
    /// The streaks of the last request, none before the first, and the next request sent.
    fn exchange(&mut self,request:RainRequest)->RenderResult<Vec<crate::weather_rain::DropInstance>> {
        let streaks=if self.pending {self.results.recv().map_err(|_|"rain thread stopped")?} else {Vec::new()};
        self.requests.send(request).map_err(|_|"rain thread stopped")?;
        self.pending=true;
        Ok(streaks)
    }
}
pub(super) struct Rain {
    uniform:wgpu::Buffer,binding:wgpu::BindGroup,surface_layout:wgpu::BindGroupLayout,
    fallback:wgpu::TextureView,sampler:wgpu::Sampler,empty:Surface,
    compose:wgpu::RenderPipeline,particles:wgpu::RenderPipeline,
    sky:sky::Sky,frame_size:[u32;2],
    last_surface:std::sync::Mutex<Option<(String,Arc<Surface>)>>,
    ribbons:std::sync::Mutex<Option<RainWorker>>,presentation:std::sync::Mutex<crate::weather_rain::PresentationClock>,vertices:wgpu::Buffer,
    /// The drops as instances, one quad each built by vs_streak.
    streaks:wgpu::RenderPipeline,streak_instances:wgpu::Buffer,
    /// Each streak's two triangles over its four vertices, the same for every frame.
    quad_indices:wgpu::Buffer,
    splashes:std::sync::Mutex<crate::weather_splashes::Spray>,
    hydrology:std::sync::Mutex<crate::weather_hydrology::Manager>,
    water_cells:wgpu::Buffer,water_frame:wgpu::Buffer,wakes:wgpu::Buffer,
    uploaded_water:std::sync::Mutex<Option<Arc<crate::weather_hydrology::Field>>>,
    rainbow:std::sync::Mutex<(bool,String,f32)>,
}
impl Rain {
    pub fn new(device:&wgpu::Device,format:wgpu::TextureFormat,width:u32,height:u32,
        scene:&wgpu::TextureView,depth:&wgpu::TextureView,stencil:&wgpu::TextureView,multisampled:bool)->Self {
        let uniform=device.create_buffer(&wgpu::BufferDescriptor{label:Some("Underground rain frame"),size:224,
            usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let sky=sky::Sky::new(device);
        let sky_view=sky.texture.create_view(&wgpu::TextureViewDescriptor{dimension:Some(wgpu::TextureViewDimension::D2Array),..Default::default()});
        let sky_sampler=device.create_sampler(&wgpu::SamplerDescriptor{label:Some("cloud panorama wrap"),address_mode_u:wgpu::AddressMode::Repeat,mag_filter:wgpu::FilterMode::Linear,min_filter:wgpu::FilterMode::Linear,..Default::default()});
        let water_cells=device.create_buffer(&wgpu::BufferDescriptor{label:Some("local basin levels"),size:(crate::weather_hydrology::SIZE*crate::weather_hydrology::SIZE*32) as u64,
            usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let water_frame=device.create_buffer(&wgpu::BufferDescriptor{label:Some("local basin domain"),size:32,
            usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let wakes=device.create_buffer(&wgpu::BufferDescriptor{label:Some("eight water-contact wakes"),size:256,
            usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let mut entries=frame_layout_entries(multisampled);
        let frame_layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{label:Some("rain scene snapshot with native stencil"),entries:&entries});
        entries.clear();
        let binding=device.create_bind_group(&wgpu::BindGroupDescriptor{label:Some("rain snapshot"),layout:&frame_layout,entries:&[
            wgpu::BindGroupEntry{binding:0,resource:uniform.as_entire_binding()},
            wgpu::BindGroupEntry{binding:1,resource:wgpu::BindingResource::TextureView(depth)},
            wgpu::BindGroupEntry{binding:2,resource:wgpu::BindingResource::TextureView(scene)},
            wgpu::BindGroupEntry{binding:3,resource:wgpu::BindingResource::TextureView(stencil)},
            wgpu::BindGroupEntry{binding:4,resource:wgpu::BindingResource::TextureView(&sky_view)},
            wgpu::BindGroupEntry{binding:5,resource:wgpu::BindingResource::Sampler(&sky_sampler)},
            wgpu::BindGroupEntry{binding:6,resource:water_cells.as_entire_binding()},
            wgpu::BindGroupEntry{binding:7,resource:water_frame.as_entire_binding()},
            wgpu::BindGroupEntry{binding:8,resource:wakes.as_entire_binding()},
        ]});
        let storage=|binding|wgpu::BindGroupLayoutEntry{binding,visibility:wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Storage{read_only:true},has_dynamic_offset:false,min_binding_size:None},count:None};
        let surface_layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{label:Some("rain geometry/cube"),entries:&[
            wgpu::BindGroupLayoutEntry{binding:0,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture{sample_type:wgpu::TextureSampleType::Float{filterable:true},view_dimension:wgpu::TextureViewDimension::Cube,multisampled:false},count:None},
            wgpu::BindGroupLayoutEntry{binding:1,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),count:None},
            storage(2),storage(3),storage(4),
        ]});
        let texture=device.create_texture(&wgpu::TextureDescriptor{label:Some("rain no-cube binding"),size:wgpu::Extent3d{width:1,height:1,depth_or_array_layers:6},mip_level_count:1,sample_count:1,
            dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Rgba8Unorm,usage:wgpu::TextureUsages::TEXTURE_BINDING,view_formats:&[]});
        let fallback=texture.create_view(&wgpu::TextureViewDescriptor{dimension:Some(wgpu::TextureViewDimension::Cube),..Default::default()});
        let sampler=device.create_sampler(&wgpu::SamplerDescriptor{label:Some("rain reflection linear clamp"),mag_filter:wgpu::FilterMode::Linear,min_filter:wgpu::FilterMode::Linear,..Default::default()});
        let upload=|label,contents:&[u8]|device.create_buffer_init(&wgpu::util::BufferInitDescriptor{label:Some(label),contents,usage:wgpu::BufferUsages::STORAGE});
        let mut node=[0u8;32];node[12..16].copy_from_slice(&u32::MAX.to_ne_bytes());node[28..32].copy_from_slice(&1u32.to_ne_bytes());
        let empty=Surface{nodes:upload("rain empty BVH",&node),triangles:upload("rain empty triangle",&[0;64]),distribution:upload("rain empty CDF",&[0;16]),
            cube:None,binding:OnceLock::new(),world:Arc::default()};
        let layout=device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{label:Some("Underground rain"),bind_group_layouts:&[Some(&frame_layout),Some(&surface_layout)],immediate_size:0});
        let rain_source=std::env::var_os("SKATE2_DIAGNOSTIC_RAIN_SHADER").map_or_else(||include_str!("rain.wgsl").to_owned(),|path|std::fs::read_to_string(path).expect("diagnostic rain shader"));
        let code=format!("{}\n{}",rain_source,include_str!("rain_lightning.wgsl"))
            .replace("WEATHER_DEPTH_TYPE",if multisampled{"texture_depth_multisampled_2d"}else{"texture_depth_2d"})
            .replace("WEATHER_STENCIL_TYPE",if multisampled{"texture_multisampled_2d<u32>"}else{"texture_2d<u32>"})
            .replace("WEATHER_SAMPLESu",if multisampled{"textureNumSamples(scene_depth)"}else{"1u"});
        let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("friend rain and isolated Daytryan lightning"),source:wgpu::ShaderSource::Wgsl(code.into())});
        let ribbons=[Some(wgpu::VertexBufferLayout{array_stride:36,step_mode:wgpu::VertexStepMode::Vertex,attributes:&RIBBON_ATTRIBUTES})];
        let streak_layout=[Some(wgpu::VertexBufferLayout{array_stride:STREAK_BYTES as u64,step_mode:wgpu::VertexStepMode::Instance,attributes:&STREAK_ATTRIBUTES})];
        let pipeline=|vertex,fragment,blend,target|device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{label:Some(fragment),layout:Some(&layout),
            vertex:wgpu::VertexState{module:&shader,entry_point:Some(vertex),buffers:if vertex=="vs_drop" {&ribbons}else if vertex=="vs_streak" {&streak_layout}else{&[]},compilation_options:Default::default()},
            fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some(fragment),targets:&[Some(wgpu::ColorTargetState{format:target,blend,write_mask:wgpu::ColorWrites::ALL})],compilation_options:Default::default()}),
            primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},depth_stencil:None,multisample:Default::default(),multiview_mask:None,cache:None});
        let streak_instances=device.create_buffer(&wgpu::BufferDescriptor{label:Some("rain streak instances"),size:(RAIN_STREAKS*STREAK_BYTES) as u64,
            usage:wgpu::BufferUsages::VERTEX|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let vertices=device.create_buffer(&wgpu::BufferDescriptor{label:Some("native rain ribbons"),size:(RAIN_QUADS*4*VERTEX_BYTES) as u64,
            usage:wgpu::BufferUsages::VERTEX|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
        let indices:Vec<u8>=(0..RAIN_QUADS as u16).flat_map(|quad|[0,1,2,0,2,3].map(|corner|quad*4+corner))
            .flat_map(u16::to_ne_bytes).collect();
        let quad_indices=wgpu::util::DeviceExt::create_buffer_init(device,&wgpu::util::BufferInitDescriptor{label:Some("native rain ribbon quads"),
            contents:&indices,usage:wgpu::BufferUsages::INDEX});
        Self{uniform,binding,surface_layout,fallback,sampler,empty,compose:pipeline("vs","fs_compose",None,format),particles:pipeline("vs_drop","fs_drop",Some(wgpu::BlendState::ALPHA_BLENDING),format),
            streaks:pipeline("vs_streak","fs_drop",Some(wgpu::BlendState::ALPHA_BLENDING),format),streak_instances,
            sky,frame_size:[width.max(1),height.max(1)],last_surface:Default::default(),
            ribbons:Default::default(),presentation:Default::default(),vertices,quad_indices,splashes:Default::default(),hydrology:Default::default(),water_cells,water_frame,wakes,uploaded_water:Default::default(),rainbow:Default::default()}
    }
    fn resources<'a>(&'a self,device:&wgpu::Device,surface:&'a Surface)->&'a wgpu::BindGroup {
        surface.binding.get_or_init(||device.create_bind_group(&wgpu::BindGroupDescriptor{label:Some("rain receiver resources"),layout:&self.surface_layout,entries:&[
            wgpu::BindGroupEntry{binding:0,resource:wgpu::BindingResource::TextureView(surface.cube.as_ref().map_or(&self.fallback,|c|c.view.as_ref()))},
            wgpu::BindGroupEntry{binding:1,resource:wgpu::BindingResource::Sampler(&self.sampler)},
            wgpu::BindGroupEntry{binding:2,resource:surface.distribution.as_entire_binding()},
            wgpu::BindGroupEntry{binding:3,resource:surface.nodes.as_entire_binding()},
            wgpu::BindGroupEntry{binding:4,resource:surface.triangles.as_entire_binding()},
        ]}))
    }
    pub fn record(&self,device:&wgpu::Device,queue:&wgpu::Queue,encoder:&mut wgpu::CommandEncoder,map:&GpuMap,frame:Frame,output:&wgpu::TextureView)->RenderResult<()> {
        let mut frame=frame;
        let mut timing=RainTiming::start();
        let(clock,rain_delta)=self.presentation.lock().map_err(|_|"rain presentation clock lock")?.sample(frame.clock);
        frame.clock=clock;crate::weather::set_rain_clock(clock);
        self.sky.upload(queue);
        let ready=map.rain_surface.get().and_then(|value|value.as_ref()).cloned();
        let retained={let mut previous=self.last_surface.lock().map_err(|_|"rain surface retention lock")?;
            if let Some(surface)=ready { *previous=Some((map.rain_location.clone(),surface)); }
            previous.as_ref().filter(|(location,_)|*location==map.rain_location).map(|(_,surface)|Arc::clone(surface))};
        let surface=retained.as_deref().unwrap_or(&self.empty);
        let field=if retained.is_some() {self.hydrology.lock().map_err(|_|"rain basin lock")?.update(&surface.world,crate::weather_splashes::anchor().unwrap_or(frame.camera),&map.rain_location)}else{None};
        timing.mark(0);
        let after=frame.mode==Mode::AfterRain;
        let rainbow_yaw={let mut state=self.rainbow.lock().map_err(|_|"rainbow anchor lock")?;
            if after && (!state.0 || state.1!=map.rain_location) {state.2=frame.axes[2][2].atan2(frame.axes[2][0]);}
            state.0=after;if state.1!=map.rain_location {state.1=map.rain_location.clone();}state.2};
        let mut domain=[0.,0.,1.,1.,0.,crate::weather_hydrology::mode_index(frame.mode) as f32,0.,rainbow_yaw];
        if let Some(field)=&field {
            let mut uploaded=self.uploaded_water.lock().map_err(|_|"rain basin GPU upload lock")?;
            if uploaded.as_ref().is_none_or(|old|!Arc::ptr_eq(old,field)) {
                queue.write_buffer(&self.water_cells,0,&field.gpu_bytes());*uploaded=Some(Arc::clone(field));
            }
            domain[..4].copy_from_slice(&[field.origin[0],field.origin[1],field.step,field.size as f32]);domain[4]=1.;
        }
        queue.write_buffer(&self.water_frame,0,&domain.into_iter().flat_map(f32::to_ne_bytes).collect::<Vec<_>>());
        let intensity:f32=match frame.mode{Mode::Drizzle|Mode::AfterRain=>0.30,Mode::Rain=>0.65,_=>1.};
        let wet=0.82_f32*intensity.sqrt();
        let coverage=if intensity>0.75 {0.395+(0.699-0.395)*((intensity-0.75_f32)/0.25).powi(2)*(3.-2.*(intensity-0.75)/0.25)}else{0.395*(intensity/0.75).powf(0.85)};
        // Basin lookup/optical thickness now runs once at native resolution.
        let clip=CameraRelativeClip::new(&frame.clip,frame.camera).clip_from_camera;
        let mut values=Vec::from(clip);values.extend(frame.camera);values.push(1.);values.extend([self.frame_size[0] as f32,self.frame_size[1] as f32,0.,0.]);
        values.extend([if after {0.}else{intensity},wet,coverage.min(0.699),frame.clock.rem_euclid(4096.)]);
        values.extend([0.412,if after {0.}else{1.},if retained.is_some(){1.}else{0.},if surface.cube.is_some(){1.}else{0.}]);
        values.extend(frame.fog);values.extend([0.,0.,1.,0.]);
        // Supplied rain's wind, volume and streak look; presets scale density/opacity.
        values.extend([1.45,-0.58,40.,23.2]);values.extend([18.,0.56,0.032,0.28*intensity.sqrt()]);
        values.extend([0.72,0.79,0.86,if after {1.}else{0.}]);
        let lightning=crate::rain_lightning::state(frame.clock,if after {0.}else{intensity});
        crate::rain_lightning::observe(lightning);values.extend(lightning);
        queue.write_buffer(&self.uniform,0,&values.into_iter().flat_map(f32::to_ne_bytes).collect::<Vec<_>>());
        let camera=skate2_core::world_particles::ParticleCamera{position:frame.camera,
            right:[frame.axes[0][0],frame.axes[0][1],frame.axes[0][2]],
            up:[frame.axes[1][0],frame.axes[1][1],frame.axes[1][2]],
            forward:[frame.axes[2][0],frame.axes[2][1],frame.axes[2][2]]};
        timing.mark(1);
        let streaks={let mut ribbons=self.ribbons.lock().map_err(|_|"rain ribbon lock")?;
            let worker=match &mut *ribbons {Some(worker)=>worker,None=>ribbons.insert(RainWorker::spawn()?)};
            // Every actual present receives its own monotonic visual step.
            worker.exchange(RainRequest{camera,dt:rain_delta,world:Arc::clone(&surface.world),
                state:crate::weather_runtime::Snapshot{intensity:if after {0.}else{intensity},wetness:wet,puddles:coverage,time:frame.clock}})?};
        let streaks=&streaks[..streaks.len().min(RAIN_STREAKS)];
        let mut streak_bytes=Vec::with_capacity(streaks.len()*STREAK_BYTES);
        for streak in streaks {
            for value in streak.head.into_iter().chain(streak.fall).chain([streak.half_width]) {streak_bytes.extend_from_slice(&value.to_ne_bytes());}
        }
        if !streak_bytes.is_empty(){queue.write_buffer(&self.streak_instances,0,&streak_bytes);}
        timing.mark(2);
        let quads={let mut spray=self.splashes.lock().map_err(|_|"rain spray lock")?;
            queue.write_buffer(&self.wakes,0,&spray.wake_bytes());
            spray.vertices(&camera,frame.clock,field.as_deref(),frame.mode)};
        timing.mark(3);
        // Four vertices a streak over the fixed quad indices: a third less upload than six.
        let quads=&quads[..quads.len().min(RAIN_QUADS*4)/4*4];
        let mut bytes=Vec::with_capacity(quads.len()*VERTEX_BYTES);
        for vertex in quads {
            let [x,y,z]=vertex.position;let [r,g,b,a]=vertex.color;let [u,v]=vertex.uv;
            for value in [x,y,z,r,g,b,a,u,v] {bytes.extend_from_slice(&value.to_ne_bytes());}
        }
        let count=(quads.len()/4*6) as u32;
        if !bytes.is_empty(){queue.write_buffer(&self.vertices,0,&bytes);}
        timing.mark(4);
        let resources=self.resources(device,surface);
        let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{label:Some("Underground rain after full scene, before cinematic effects"),
            color_attachments:&[Some(wgpu::RenderPassColorAttachment{view:output,depth_slice:None,resolve_target:None,ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color::BLACK),store:wgpu::StoreOp::Store}})],
            depth_stencil_attachment:None,timestamp_writes:None,occlusion_query_set:None,multiview_mask:None});
        pass.set_bind_group(0,&self.binding,&[]);pass.set_bind_group(1,resources,&[]);pass.set_pipeline(&self.compose);pass.draw(0..3,0..1);
        if !streaks.is_empty() {pass.set_pipeline(&self.streaks);pass.set_vertex_buffer(0,self.streak_instances.slice(..));
            pass.set_index_buffer(self.quad_indices.slice(..),wgpu::IndexFormat::Uint16);pass.draw_indexed(0..6,0,0..streaks.len() as u32);}
        if count>0 {pass.set_pipeline(&self.particles);pass.set_vertex_buffer(0,self.vertices.slice(..));
            pass.set_index_buffer(self.quad_indices.slice(..),wgpu::IndexFormat::Uint16);pass.draw_indexed(0..count,0,0..1);}
        drop(pass);
        timing.mark(5);
        timing.finish();
        Ok(())
    }
}

fn frame_layout_entries(multisampled:bool)->Vec<wgpu::BindGroupLayoutEntry> {
    vec![
        wgpu::BindGroupLayoutEntry{binding:0,visibility:wgpu::ShaderStages::VERTEX_FRAGMENT,ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
        wgpu::BindGroupLayoutEntry{binding:1,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture{sample_type:wgpu::TextureSampleType::Depth,view_dimension:wgpu::TextureViewDimension::D2,multisampled},count:None},
        wgpu::BindGroupLayoutEntry{binding:2,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture{sample_type:wgpu::TextureSampleType::Float{filterable:true},view_dimension:wgpu::TextureViewDimension::D2,multisampled:false},count:None},
        wgpu::BindGroupLayoutEntry{binding:3,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture{sample_type:wgpu::TextureSampleType::Uint,view_dimension:wgpu::TextureViewDimension::D2,multisampled},count:None},
        wgpu::BindGroupLayoutEntry{binding:4,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Texture{sample_type:wgpu::TextureSampleType::Float{filterable:true},view_dimension:wgpu::TextureViewDimension::D2Array,multisampled:false},count:None},
        wgpu::BindGroupLayoutEntry{binding:5,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),count:None},
        wgpu::BindGroupLayoutEntry{binding:6,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Storage{read_only:true},has_dynamic_offset:false,min_binding_size:None},count:None},
        wgpu::BindGroupLayoutEntry{binding:7,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None},count:None},
        wgpu::BindGroupLayoutEntry{binding:8,visibility:wgpu::ShaderStages::FRAGMENT,ty:wgpu::BindingType::Buffer{ty:wgpu::BufferBindingType::Storage{read_only:true},has_dynamic_offset:false,min_binding_size:None},count:None},
    ]
}
