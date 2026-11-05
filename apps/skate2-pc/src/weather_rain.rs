//! Saved Underground rain look: world-anchored, wind-driven ribbons using the native pass.

use crate::{
    weather_runtime::{MAX_INTENSITY, Snapshot, WIND},
    weather_world::{World, dot, normalized, sub},
};
use rayon::prelude::*;
use skate2_core::world_particles::ParticleCamera;

pub(crate) const TEXTURE_ID: u64 = 0x5355_5241_494e_0001;
const RADIUS: f64 = 40.0;
const HEIGHT: f64 = 23.2;
const DENSITY: f64 = 42.0;
/// How far above a drop a roof shelters it, and how far below it the ground is looked for.
const SHELTER_RANGE: f32 = 256.0;
const FALL_RANGE: f32 = 128.0;
/// Drops a pool worker updates at a time.
const CHUNK: usize = 512;

/// Cosmetic weather advances on every presented frame, including repeated native
/// presentation samples. This clock never supplies simulation or game audio time.
#[derive(Default)]
pub(crate) struct PresentationClock {
    previous:Option<std::time::Instant>, seconds:f64, report_seconds:f64,
    frames:u32,native_repeats:u32,native_previous:Option<f32>,
}
impl PresentationClock {
    pub(crate) fn sample(&mut self,native:f32)->(f32,f32) {
        let now=std::time::Instant::now();
        let elapsed=self.previous.map_or(1.0/120.0,|last|now.saturating_duration_since(last).as_secs_f64());
        self.previous=Some(now);self.advance(elapsed,native)
    }
    fn advance(&mut self,elapsed:f64,native:f32)->(f32,f32) {
        // A suspended/minimized renderer resumes locally, without catching up a
        // whole paused interval in one visible frame.
        let delta=if !elapsed.is_finite()||elapsed<0.0 {0.0}else if elapsed>0.25{1.0/120.0}else{elapsed};
        self.seconds+=delta;self.report_seconds+=delta;self.frames+=1;
        self.native_repeats+=u32::from(self.native_previous.is_some_and(|last|native<=last));self.native_previous=Some(native);
        if self.report_seconds>=1.0 {
            if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some() {
                eprintln!("rain presentation clock: frames={} native_repeats={} elapsed={:.4}; weather uses render time",self.frames,self.native_repeats,self.report_seconds);
            }
            self.report_seconds=0.0;self.frames=0;self.native_repeats=0;
        }
        (self.seconds as f32,delta as f32)
    }
}

#[derive(Default)]
pub(crate) struct Rain {offset:[f64;3],drops:Vec<Particle>,world_key:(usize,usize),epoch:Option<f64>,
    world_generation:u64,frame:u64,keys:Vec<(f32,u32)>,parts:Vec<Vec<Streak>>}
/// One drawn streak for the GPU: its head and length, its fall direction and opacity, and its
/// physical half width. The vertex shader builds the camera-facing quad from it.
#[derive(Clone,Copy,Default,Debug)]
pub(crate) struct DropInstance {pub head:[f32;4],pub fall:[f32;4],pub half_width:f32}
/// A drawn drop's distance from the camera squared and its streak.
type Streak=(f32,DropInstance);
#[derive(Clone,Copy)]struct Cover {origin:[f32;3],clear:bool,until:f32,generation:u64}
/// `variant` and `shade` are the drop's fixed random length/width and opacity draws.
/// `fall` is the drop's path per metre of drop (its wind over its fall speed) and `fall_inverse`
/// one over that path's squared length; both change only with the wind, at spawn and wrap.
#[derive(Clone,Copy)]struct Particle {point:[f64;3],wind:[f32;3],fall:[f32;3],fall_inverse:f32,cover:Option<Cover>,visibility:f32,variant:f32,shade:f32}
impl Particle {
    fn set_wind(&mut self,wind:[f32;3]) {
        self.wind=wind;
        self.fall=wind.map(|v|v/-wind[1]);
        self.fall_inverse=1./dot(self.fall,self.fall);
    }
}
/// Authored coherent gusts. Each new droplet keeps its birth velocity, so the
/// front moves through the rain naturally and exact trajectory cover stays cached.
fn wind_at(time:f64,seed:u32)->[f32;3] {
    let phase=(time+3.5).rem_euclid(17.0);
    let gust=if (4.0..11.0).contains(&phase){((phase-4.0)/7.0*std::f64::consts::PI).sin().powi(2)}else{0.0};
    let bend=(time*0.19).sin();let variation=(random(seed^0xf192_3a67)-0.5)*0.7;
    [WIND[0]+(gust*(6.4+bend*1.4))as f32+variation as f32,WIND[1],WIND[2]+(gust*(2.7+(time*0.23).cos()))as f32]
}
fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}
fn random(value: u32) -> f64 {
    f64::from(hash(value) >> 8) / 16_777_216.0
}

impl Rain {
    pub(crate) fn streaks(&mut self,camera:&ParticleCamera,dt:f32,state:Snapshot,world:&World)->Vec<DropInstance>{
        if !camera.position.iter().all(|v|v.is_finite()){return Vec::new();}
        let dt=if dt.is_finite(){dt.clamp(0.,0.25)}else{0.};
        let previous_offset=self.offset;
        for(offset,velocity)in self.offset.iter_mut().zip(WIND){*offset+=f64::from(dt)*f64::from(velocity);}
        let elapsed=self.offset[1]/f64::from(WIND[1]);
        let epoch=*self.epoch.get_or_insert_with(||if state.time.is_finite(){f64::from(state.time)}else{0.});let now=epoch+elapsed;
        if state.intensity<=0.0001{return Vec::new();}
        let strength=(state.intensity/MAX_INTENSITY).clamp(0.,1.);
        let count=(DENSITY*(2.*RADIUS).powi(2)*HEIGHT/1000.*f64::from(strength)).round()as usize;
        let key=(world as*const World as usize,world.triangles.len());
        // Streamed map updates formerly invalidated every drop at once. Spread
        // cover refresh over several frames and retain the last known cover in
        // the meantime, avoiding thousands of simultaneous BVH queries.
        if self.world_key!=key{self.world_key=key;self.world_generation=self.world_generation.wrapping_add(1);}
        self.frame=self.frame.wrapping_add(1);let generation=self.world_generation;let frame=self.frame;
        // Cover refreshes per second, not per frame: 384 a frame at 60 fps, fewer at higher rates.
        let ray_budget=((384.*f64::from(dt)*60.).ceil()as usize).clamp(16,384);
        let size=[RADIUS*2.,HEIGHT,RADIUS*2.];
        let low=[f64::from(camera.position[0])-RADIUS,f64::from(camera.position[1])-HEIGHT/3.,f64::from(camera.position[2])-RADIUS];
        while self.drops.len()<count{
            let seed=hash(self.drops.len()as u32+0x5355_5241);let seeds=[random(seed),random(seed^0x68bc_21eb),random(seed^0xa361_2d9b)];
            let point=std::array::from_fn(|i|low[i]+(seeds[i]*size[i]+previous_offset[i]-low[i]).rem_euclid(size[i]));
            let mut drop=Particle{point,wind:[0.;3],fall:[0.;3],fall_inverse:0.,cover:None,visibility:0.,
                variant:random(seed^0x02e5_be93)as f32,shade:random(seed^0x19ab_cdef)as f32};
            drop.set_wind(wind_at(now-f64::from(dt),seed));
            self.drops.push(drop);
        }
        self.drops.truncate(count);
        let step=Step{camera,world,low,size,now,dt,strength:strength.sqrt(),generation,frame};
        let timing=StreakTiming::start();
        let chunks=self.drops.len().div_ceil(CHUNK);
        let mut parts=std::mem::take(&mut self.parts);parts.resize_with(chunks,Vec::new);
        // Each chunk takes its share of the frame's cover queries. The drops update on this
        // thread: woken every frame, the shared pool's workers keep spinning between frames and
        // slow the simulation and render threads more than they save. SKATE2_DIAGNOSTIC_RAIN_POOL
        // runs them on the pool instead.
        let run=|chunk:usize,(drops,part):(&mut [Particle],&mut Vec<Streak>)|{
            part.clear();
            let share=ray_budget/chunks+usize::from(chunk<ray_budget%chunks);
            let mut budget=share;
            for(offset,p)in drops.iter_mut().enumerate(){
                if let Some(streak)=step.update(chunk*CHUNK+offset,p,&mut budget){part.push(streak);}
            }
            COVER_QUERIES.fetch_add(share-budget,std::sync::atomic::Ordering::Relaxed);
        };
        static POOL:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        if *POOL.get_or_init(||std::env::var_os("SKATE2_DIAGNOSTIC_RAIN_POOL").is_some()) {
            self.drops.par_chunks_mut(CHUNK).zip(parts.par_iter_mut()).enumerate().for_each(|(chunk,pair)|run(chunk,pair));
        } else {
            self.drops.chunks_mut(CHUNK).zip(parts.iter_mut()).enumerate().for_each(|(chunk,pair)|run(chunk,pair));
        }
        let timing=timing.mark(0);
        // Far to near in distance bands, a counting sort linear in the drops rather than a full
        // comparison sort of every streak each frame: order within a 16 cm band is invisible for
        // thin translucent streaks.
        const BANDS:usize=256;
        const FAR:f32=41.0;
        let band=|distance_squared:f32|BANDS-1-((distance_squared.sqrt()*(BANDS as f32/FAR))as usize).min(BANDS-1);
        let mut starts=[0u32;BANDS];
        for part in &parts {for streak in part {starts[band(streak.0)]+=1;}}
        let mut total=0u32;
        for start in &mut starts {let count=*start;*start=total;total+=count;}
        let mut keys=std::mem::take(&mut self.keys);keys.clear();keys.resize(total as usize,(0.0,0));
        for(chunk,part)in parts.iter().enumerate(){
            for(i,streak)in part.iter().enumerate(){
                let slot=&mut starts[band(streak.0)];
                keys[*slot as usize]=(streak.0,(chunk*CHUNK+i)as u32);*slot+=1;
            }
        }
        let timing=timing.mark(1);
        let streaks:Vec<DropInstance>=keys.iter().map(|&(_,key)|{let key=key as usize;parts[key/CHUNK][key%CHUNK].1}).collect();
        timing.mark(2).finish();
        self.keys=keys;self.parts=parts;streaks
    }
}

/// `SKATE2_TRACE_RAIN_TIMING`: the streak build's drop updates, ordering and vertices, averaged
/// over 600 frames.
struct StreakTiming {last:Option<std::time::Instant>,stages:[f64;3]}
static STREAK_TIMING:std::sync::Mutex<([f64;3],u32)>=std::sync::Mutex::new(([0.;3],0));
/// Cover queries the drops spent, for the timing report.
static COVER_QUERIES:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
static COVER_NANOS:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
impl StreakTiming {
    fn start()->Self {
        static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        let enabled=*ENABLED.get_or_init(||std::env::var_os("SKATE2_TRACE_RAIN_TIMING").is_some());
        Self {last:enabled.then(std::time::Instant::now),stages:[0.;3]}
    }
    fn mark(mut self,stage:usize)->Self {
        if let Some(last)=self.last {let now=std::time::Instant::now();self.stages[stage]+=(now-last).as_secs_f64()*1000.;self.last=Some(now);}
        self
    }
    fn finish(self) {
        if self.last.is_none() {return;}
        let Ok(mut total)=STREAK_TIMING.lock() else {return};
        for (sum,value) in total.0.iter_mut().zip(self.stages) {*sum+=value;}
        total.1+=1;
        if total.1==600 {
            let average=total.0.map(|sum|sum/600.);
            let queries=COVER_QUERIES.swap(0,std::sync::atomic::Ordering::Relaxed) as f64/600.;
            let cover_ms=COVER_NANOS.swap(0,std::sync::atomic::Ordering::Relaxed) as f64/600./1e6;
            eprintln!("rain streak timing ms: drops {:.3} order {:.3} vertices {:.3}; cover queries {queries:.1} a frame taking {cover_ms:.3} ms",average[0],average[1],average[2]);
            *total=([0.;3],0);
        }
    }
}

/// One frame's values every drop's update reads; `strength` is the square root of the
/// intensity's share, which scales every streak's opacity.
struct Step<'a> {camera:&'a ParticleCamera,world:&'a World,low:[f64;3],size:[f64;3],now:f64,dt:f32,strength:f32,generation:u64,frame:u64}
impl Step<'_> {
    /// Drop `index`'s fall this frame and its streak, if one shows; a cover query spends one of
    /// `budget`.
    fn update(&self,index:usize,p:&mut Particle,budget:&mut usize)->Option<Streak>{
        let (camera,low,size,dt)=(self.camera,self.low,self.size,self.dt);
        for i in 0..3{p.point[i]+=f64::from(p.wind[i])*f64::from(dt);}
        let top=low[1]+HEIGHT;
        if p.point[1]<low[1]||p.point[1]>=top{
            // Preserve phase in both directions. Clamping above-top drops to
            // the top synchronized a camera-cut cohort into a repeating sheet.
            let wrapped=low[1]+(p.point[1]-low[1]).rem_euclid(HEIGHT);
            let residual=(top-wrapped)/-f64::from(WIND[1]);
            let old_wind=p.wind;p.set_wind(wind_at(self.now-residual,hash(index as u32+0x5355_5241)));
            for i in [0,2]{p.point[i]+=f64::from(p.wind[i]-old_wind[i])*residual;}
            p.point[1]=wrapped;
            p.cover=None;
        }
        for i in [0,2]{if p.point[i]<low[i]||p.point[i]>=low[i]+size[i]{p.point[i]=low[i]+(p.point[i]-low[i]).rem_euclid(size[i]);p.cover=None;}}
        let point=p.point.map(|v|v as f32);let relative=sub(point,camera.position);
        if dot(relative,camera.forward)<0.0{return None;}
        let distance=(relative[0]*relative[0]+relative[2]*relative[2]).sqrt();
        let fade=((40.-distance)/6.).clamp(0.,1.)*((distance-0.65)/1.5).clamp(0.,1.)
            *((point[1]-low[1]as f32)/1.5).clamp(0.,1.)*((top as f32-point[1])/1.5).clamp(0.,1.);
        if fade<=0.{return None;}
        let (fall,inverse)=(p.fall,p.fall_inverse);
        // How far down its cached cover ray the drop is: the cover holds while the drop stays on it.
        let mut along=0.;
        let cached=p.cover.filter(|c|{let delta=sub(point,c.origin);let travelled=dot(delta,fall)*inverse;let error=sub(delta,fall.map(|v|v*travelled));
            let holds=travelled>=0.&&travelled<c.until&&dot(error,error)<0.000001;if holds{along=travelled;}holds});
        let refresh=cached.is_none() || cached.is_some_and(|c|c.generation!=self.generation && (index as u64+self.frame)%16==0);
        let cover=if refresh && *budget>0 {
            // Shelter within 256 m above and ground within 128 m below decide a drop; the
            // BVH stops at those distances instead of crossing the whole map.
            *budget-=1;let started=std::time::Instant::now();
            let sky=normalized(p.wind.map(|v|-v));
            let cover=Cover{origin:point,clear:self.world.ray_distance(point,sky,SHELTER_RANGE).is_none(),until:self.world.ray_distance(point,fall,FALL_RANGE).unwrap_or(10_000.),generation:self.generation};
            COVER_NANOS.fetch_add(started.elapsed().as_nanos() as usize,std::sync::atomic::Ordering::Relaxed);
            along=0.;cover
        }else if let Some(previous)=cached {previous}else{return None;};
        p.cover=Some(cover);
        if !cover.clear{p.visibility=0.;return None;}
        p.visibility=(p.visibility+dt*6.).min(1.);
        let variant=p.variant;
        let length=(cover.until-along-0.015).min(0.22+0.54*variant);
        if length<=0.{return None;}
        // The quad's side, its distance-widened width and that width's opacity scale are the
        // vertex shader's (rain.wgsl vs_streak), from the camera.
        let physical_width=(0.0045+0.009*variant)*0.5;
        let alpha=(0.16+0.16*p.shade)*fade*self.strength*p.visibility;
        Some((dot(relative,relative),DropInstance{head:[point[0],point[1],point[2],length],
            fall:[fall[0],fall[1],fall[2],alpha],half_width:physical_width}))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "soft streak alpha is clamped by construction to 0..255 UNORM"
)]
pub(crate) fn texture() -> skate2_core::texture::NativeTexture {
    use skate2_formats::material::{
        LinearTextureLevel, PIXEL_FORMAT_A8R8G8B8, TextureEndian, TextureType,
    };
    let mut data = Vec::with_capacity(8 * 32 * 4);
    for y in 0..32_u16 {
        for x in 0..8_u16 {
            let u = f32::from(x) / 7.0;
            let v = f32::from(y) / 31.0;
            let alpha =
                (1.0 - (u * 2.0 - 1.0).abs()).powi(2) * (std::f32::consts::PI * v).sin().powi(2);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    skate2_core::texture::NativeTexture {
        dictionary_index: 0,
        guid: TEXTURE_ID,
        name: "Underground.Rain".into(),
        texture_type: TextureType::TwoDimensional,
        endian: TextureEndian::None,
        width: 8,
        height: 32,
        depth: 1,
        mip_levels: 1,
        pixel_format: PIXEL_FORMAT_A8R8G8B8,
        packed_mips: false,
        mip_data_offset: 0,
        encoded_data: Vec::new(),
        linear_levels: vec![LinearTextureLevel {
            level: 0,
            face: 0,
            width: 8,
            height: 32,
            width_in_blocks: 8,
            height_in_blocks: 32,
            bytes_per_block: 4,
            row_pitch: 32,
            data,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn camera(position: [f32; 3]) -> ParticleCamera {
        ParticleCamera {
            position,
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, 1.0],
        }
    }
    #[test]
    fn presented_rain_keeps_moving_when_native_samples_repeat_or_rewind() {
        let mut clock=PresentationClock::default();let mut rain=Rain::default();let world=World::default();let camera=camera([0.,2.,0.]);let mut previous:Option<f64>=None;
        for frame in 0..240 {
            // Source presentation remains frozen, then rewinds. Its former
            // max(native-last,0) driver would repeatedly supply a zero delta.
            let native=if frame<120 {2.0}else{1.0};let(time,delta)=clock.advance(1.0/120.0,native);
            assert!(delta>0.0);let vertices=rain.streaks(&camera,delta,Snapshot{intensity:1.,time,..Default::default()},&world);assert!(!vertices.is_empty());
            let point=rain.drops[0].point;
            if let Some(last)=previous {let movement=(point[1]-last).abs();assert!(movement>0.10,"rain must move on every presented frame");}
            previous=Some(point[1]);
        }
        assert!((clock.seconds-2.0).abs()<0.00001);
    }
    #[test]
    fn rain_resume_does_not_jump_a_suspended_interval() {
        let mut clock=PresentationClock::default();clock.advance(1.0/120.0,0.0);let(before,_)=clock.advance(1.0/120.0,0.0);let(after,delta)=clock.advance(20.0,0.0);
        assert!(delta>0.0&&delta<0.02&&after-before<0.02);
    }
    #[test]fn coherent_gust_changes_angle_without_unbounded_wind(){
        let mut max=0_f32;let mut min=100_f32;
        for i in 0..170{let w=wind_at(f64::from(i)*0.1,4);assert!(w.iter().all(|v|v.is_finite()));assert_eq!(w[1],WIND[1]);max=max.max(w[0]);min=min.min(w[0]);}
        assert!(max-min>5.0&&max<10.0);
    }
    #[test]
    fn saved_look_is_bounded_and_stable_across_city_teleports() {
        let mut rain = Rain::default();
        let world = World::default();
        let state = Snapshot {
            intensity: MAX_INTENSITY,
            ..Snapshot::default()
        };
        for position in [[0.0, 2.0, 0.0], [10_000.0, 2.0, 0.0], [-10_000.0, 2.0, 0.0]] {
            let streaks = rain.streaks(&camera(position), 1.0 / 60.0, state, &world);
            assert!(!streaks.is_empty() && streaks.len() < 6500);
            assert!(
                streaks
                    .iter()
                    .all(|v| v.head.iter().all(|f| f.is_finite())
                        && (v.head[0] - position[0]).abs() < 41.0)
            );
        }
        assert!(
            rain.streaks(&camera([0.0, 2.0, 0.0]), 0.0, Snapshot::default(), &world)
                .is_empty()
        );
    }
    #[test]
    fn frame_rate_does_not_change_fall_speed() {
        let mut a = Rain::default();
        let mut b = Rain::default();
        let world = World::default();
        let state = Snapshot {
            intensity: 0.6,
            ..Snapshot::default()
        };
        let camera = camera([0.0, 2.0, 0.0]);
        for _ in 0..30 {
            a.streaks(&camera, 1.0 / 30.0, state, &world);
        }
        for _ in 0..60 {
            b.streaks(&camera, 1.0 / 60.0, state, &world);
        }
        for i in 0..3 {
            assert!((a.offset[i] - b.offset[i]).abs() < 1e-6);
        }
        for (a,b) in a.drops.iter().zip(&b.drops){for i in 0..3{assert!((a.point[i]-b.point[i]).abs()<0.002,"particle motion differs by frame rate");}}
    }
    #[test]
    fn streamed_ground_refresh_spreads_cover_queries_across_frames() {
        let mut rain=Rain::default();let camera=camera([0.,2.,0.]);let a=World::default();let b=World::default();
        let state=Snapshot{intensity:1.,..Snapshot::default()};
        rain.streaks(&camera,1./120.,state,&a);
        assert!(rain.drops.iter().filter(|p|p.cover.is_some()).count()<=384);
        for _ in 0..24 {rain.streaks(&camera,1./120.,state,&a);}
        let cached=rain.drops.iter().filter(|p|p.cover.is_some()).count();assert!(cached>500);
        rain.streaks(&camera,1./120.,state,&b);let generation=rain.world_generation;
        assert!(rain.drops.iter().filter(|p|p.cover.is_some_and(|c|c.generation==generation)).count()<=384);
        assert!(rain.drops.iter().any(|p|p.cover.is_some_and(|c|c.generation!=generation)),"old cover remains while replacement warms");
        for _ in 0..32 {rain.streaks(&camera,1./120.,state,&b);}
        assert!(rain.drops.iter().filter(|p|p.cover.is_some_and(|c|c.generation==generation)).count()>500);
    }
    #[test]
    fn rain_does_not_query_or_emit_ribbons_wholly_behind_the_camera() {
        let mut rain=Rain::default();let camera=camera([0.0,2.0,0.0]);
        let drops=rain.streaks(&camera,0.0,Snapshot{intensity:1.0,..Snapshot::default()},&World::default());
        assert!(drops.len()<3500);
        assert!(drops.iter().all(|drop| dot(sub([drop.head[0],drop.head[1],drop.head[2]],camera.position),camera.forward)>-0.8));
        assert!(drops.iter().any(|drop| drop.head[2]>20.0));
    }

}

#[cfg(test)]mod burst_regression {
use super::*;
#[test]fn camera_drop_must_not_synchronize_raindrops_into_a_sheet(){
 let world=World::default();let mut rain=Rain::default();let mut worst=0;let mut count=0;
 for frame in 0..720 {
  let time=frame as f32/120.;let y=if frame<120{12.}else{2.+(time*1.5).sin()*0.3};
  let camera=ParticleCamera{position:[0.,y,0.],right:[1.,0.,0.],up:[0.,1.,0.],forward:[0.,0.,1.]};
  rain.streaks(&camera,1./120.,Snapshot{intensity:1.,time,..Default::default()},&world);
  let low=f64::from(y)-HEIGHT/3.;let mut bins=[0usize;64];
  for drop in &rain.drops{let bin=(((drop.point[1]-low)/HEIGHT*64.).floor()as usize).min(63);bins[bin]+=1;}
  count=rain.drops.len();if frame>=120{worst=worst.max(*bins.iter().max().unwrap());}
 }
 println!("camera-drop vertical concentration: worst_bin={worst} particles={count} fraction={:.4}",worst as f64/count as f64);
 assert!(worst<count/20,"camera motion must preserve each droplet phase, not turn a cap into a synchronized rain sheet");
}
}
