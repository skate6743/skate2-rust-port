//! Authored PC water contact presentation. No skating forces or scoring changes.
use std::{sync::{Mutex,OnceLock},time::{Duration,Instant}};
use crate::{weather::Mode,weather_hydrology::Field,weather_world::{dot,sub,normalized}};
use skate2_core::world_particles::{ParticleCamera,ParticleVertex};
#[derive(Clone,Copy)]struct Deck {matrix:[f32;16],tick:u32,at:Instant,replay:bool}
#[derive(Clone,Copy)]struct Actor {feet:[f32;3],foot_positions:Option<[[f32;3];2]>,tick:u32,at:Instant,replay:bool}
fn latest()->&'static Mutex<Option<Deck>> {static DATA:OnceLock<Mutex<Option<Deck>>>=OnceLock::new();DATA.get_or_init(||Mutex::new(None))}
fn actor()->&'static Mutex<Option<Actor>> {static DATA:OnceLock<Mutex<Option<Actor>>>=OnceLock::new();DATA.get_or_init(||Mutex::new(None))}
pub(crate) fn publish(matrix:Option<[f32;16]>,tick:u32,replay:bool) {
    if let Ok(mut d)=latest().lock(){*d=matrix.filter(|m|m.iter().all(|v|v.is_finite())).map(|matrix|Deck{matrix,tick,at:Instant::now(),replay});}
}
pub(crate) fn publish_actor(hips:[f32;3],bounds:[[f32;3];2],foot_positions:Option<[[f32;3];2]>,tick:u32,replay:bool) {
    let feet=[hips[0],bounds[0][1],hips[2]];
    let foot_positions=foot_positions.filter(|pair|pair.iter().all(|p|
        p.iter().all(|v|v.is_finite()) && (p[0]-hips[0]).hypot(p[2]-hips[2])<1.5 && (p[1]-feet[1]).abs()<1.5));
    if let Ok(mut a)=actor().lock(){*a=feet.iter().all(|v|v.is_finite()).then(||Actor{feet,foot_positions,tick,at:Instant::now(),replay});}
}
fn fresh_actor()->Option<Actor>{actor().lock().ok().and_then(|a|*a).filter(|a|a.at.elapsed()<Duration::from_millis(350)&&!a.replay)}
pub(crate) fn anchor()->Option<[f32;3]>{fresh_actor().map(|a|a.feet)}
#[derive(Clone,Copy)]struct Droplet{position:[f32;3],velocity:[f32;3],age:f32,life:f32,width:f32,alpha:f32,floor:f32}
#[derive(Clone,Copy)]struct Wake{position:[f32;3],age:f32,direction:[f32;2],strength:f32,speed:f32}
#[derive(Default)]pub(crate) struct Spray{
    drops:Vec<Droplet>,wakes:Vec<Wake>,previous:Option<Deck>,previous_actor:Option<Actor>,clock:Option<f32>,
    fraction:f32,seed:u32,wet:bool,since_wake:f32,last_wake:Option<[f32;3]>,last_board:bool,foot_side:bool,board_axle:bool,
}
impl Spray {
    fn wake(&mut self,position:[f32;3],velocity:[f32;3],strength:f32,board:bool){
        if self.wakes.len()>=8 {self.wakes.remove(0);}
        let d=normalized([velocity[0],0.,velocity[2]]);let direction=if dot(d,d)>0.5{[d[0],d[2]]}else{[1.,0.]};
        let speed=if board {-velocity[0].hypot(velocity[2]).max(0.01)}else{0.28};
        self.wakes.push(Wake{position,age:0.,direction,strength,speed});
    }
    pub(crate) fn wake_bytes(&self)->Vec<u8>{
        (0..8).flat_map(|i|self.wakes.get(i).map_or([0.,0.,0.,-1.,1.,0.,0.,0.],|w|
            [w.position[0],w.position[2],w.position[1],w.age,w.direction[0],w.direction[1],w.strength,w.speed]))
            .flat_map(f32::to_ne_bytes).collect()
    }
    pub(crate) fn vertices(&mut self,camera:&ParticleCamera,clock:f32,field:Option<&Field>,mode:Mode)->Vec<ParticleVertex>{
        let step=self.clock.map_or(0.,|last|clock-last);self.clock=Some(clock);
        if !(0.0..=0.25).contains(&step){self.drops.clear();self.wakes.clear();self.previous=None;self.previous_actor=None;self.wet=false;self.fraction=0.;self.last_wake=None;}
        let dt=step.clamp(0.,0.1);self.since_wake+=dt;
        for d in &mut self.drops {d.age+=dt;d.velocity[1]-=9.81*dt;for i in 0..3{d.position[i]+=d.velocity[i]*dt;}}
        for w in &mut self.wakes{w.age+=dt;}
        self.drops.retain(|d|d.age<d.life&&d.position[1]>d.floor);self.wakes.retain(|w|w.age<0.95);
        let deck=latest().lock().ok().and_then(|d|*d).filter(|d|d.at.elapsed()<Duration::from_millis(350)&&!d.replay);
        let person=fresh_actor();let mut contact=None;let mut changed=false;let mut wheels:[Option<[f32;3]>;4]=[None;4];
        if let Some(deck)=deck {
            if let Some(previous)=self.previous.filter(|p|p.tick!=deck.tick){
                changed=true;let ticks=deck.tick.wrapping_sub(previous.tick);let pos=position(deck.matrix);let delta=sub(pos,position(previous.matrix));
                if (1..=12).contains(&ticks)&&dot(delta,delta)<4. {
                    let elapsed=ticks as f32/60.;let velocity=delta.map(|v|v/elapsed);let speed=velocity[0].hypot(velocity[2]);
                    let carried=person.is_some_and(|a|pos[1]-a.feet[1]>0.38);
                    if !carried && deck.matrix[5]>0.6 {
                        let forward=normalized([velocity[0],0.,velocity[2]]);let side=[forward[2],0.,-forward[0]];
                        self.fraction+=(speed*4.0).min(40.)*elapsed;let count=(self.fraction.floor() as usize).min(1);self.fraction=self.fraction.fract();
                        let mut centre=[0.;3];let mut contacts=0.;let mut water_depths=[0.;4];
                        for front in [-1.,1.] {for lateral in [-1.,1.]{
                            let probe:[f32;3]=std::array::from_fn(|i|pos[i]+deck.matrix[8+i]*front*0.27+deck.matrix[i]*lateral*0.09-deck.matrix[4+i]*0.055);
                            let Some(w)=field.and_then(|f|f.sample_contact([probe[0],probe[2]],mode,probe[1]))else{continue;};
                            if probe[1]>w.level+0.012||probe[1]<w.ground-0.08{continue;}
                            let index=usize::from(front>0.)*2+usize::from(lateral>0.);
                            if w.wet_depth>0.003 {water_depths[index]=w.wet_depth;}
                            if w.wet_depth<0.009{continue;}
                            let at=[probe[0],w.level,probe[2]];for i in 0..3{centre[i]+=at[i];}contacts+=1.;
                            wheels[index]=Some(at);
                            if speed>0.5||velocity[1]< -1.5 {
                                for _ in 0..count+usize::from(velocity[1]< -1.5){
                                    self.seed=self.seed.wrapping_mul(1664525).wrapping_add(1013904223);let r=(self.seed>>8)as f32/16_777_216.;
                                    if self.drops.len()<128 {
                                        if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some(){static ONCE:OnceLock<()>=OnceLock::new();ONCE.get_or_init(||eprintln!("rain wheel spray: fine droplets on actual water"));}
                                        self.drops.push(Droplet{position:[at[0],at[1]+0.003,at[2]],
                                            velocity:std::array::from_fn(|i|side[i]*lateral*(0.12+speed*0.05)*(0.5+r)-forward[i]*speed*0.14+if i==1{0.45+r*0.35+speed*0.020}else{0.}),
                                            age:0.,life:0.12+r*0.14,width:0.003+r*0.002,alpha:(0.16+r*0.16)*(w.wet_depth/0.045).clamp(0.25,1.),floor:at[1]+0.0005});
                                    }
                                }
                            }
                        }}
                        if contacts>0.{contact=Some((centre.map(|v|v/contacts),velocity,speed,true));}
                        // Puddle Physics: the wheels' water under the board the player rides.
                        if water_depths.iter().any(|d|*d>0.003) {skate2_core::ui::puddle_physics::publish(pos,water_depths);}
                        else{skate2_core::ui::puddle_physics::clear();}
                    }else{skate2_core::ui::puddle_physics::clear();}
                }else{skate2_core::ui::puddle_physics::clear();self.wet=false;self.wakes.clear();self.drops.clear();self.last_wake=None;}
            }
            if self.previous.is_none_or(|p|p.tick!=deck.tick){self.previous=Some(deck);}
        }else{self.previous=None;skate2_core::ui::puddle_physics::clear();}
        if let Some(person)=person {
            if let Some(previous)=self.previous_actor.filter(|a|a.tick!=person.tick){
                changed=true;let ticks=person.tick.wrapping_sub(previous.tick);let delta=sub(person.feet,previous.feet);
                if (1..=12).contains(&ticks)&&dot(delta,delta)<4. {
                    let on_foot=deck.is_none_or(|d|position(d.matrix)[1]-person.feet[1]>0.38);
                    if contact.is_none()&&on_foot{
                        let velocity=delta.map(|v|v/(ticks as f32/60.));let speed=velocity[0].hypot(velocity[2]);
                        if let Some(w)=field.and_then(|f|f.sample_contact([person.feet[0],person.feet[2]],mode,person.feet[1]))
                            && w.wet_depth>0.006&&person.feet[1]<w.level+0.04&&person.feet[1]>w.ground-0.16 {
                            contact=Some(([person.feet[0],w.level,person.feet[2]],velocity,speed,false));
                        }
                    }
                }else{self.wet=false;self.wakes.clear();self.drops.clear();self.last_wake=None;}
            }
            if self.previous_actor.is_none_or(|a|a.tick!=person.tick){self.previous_actor=Some(person);}
        }else{self.previous_actor=None;}
        if changed {
            if let Some((at,velocity,speed,board))=contact {
                let moved=self.last_wake.map_or(f32::INFINITY,|last|(at[0]-last[0]).hypot(at[2]-last[2]));
                let gap=if board {0.14}else{0.27};let stride=if board {0.28}else{0.34};
                let impact=!self.wet&&velocity[1]< -0.9;
                if self.since_wake>=gap && ((speed>0.35&&(moved>=stride||self.last_board!=board))||impact) {
                    let mut origin=at;
                    let mut foot_wet=false;
                    if !board {
                        let forward=normalized([velocity[0],0.,velocity[2]]);
                        let sign=if self.foot_side{1.}else{-1.};self.foot_side=!self.foot_side;
                        let side=[forward[2]*sign*0.09,0.,-forward[0]*sign*0.09];
                        let actor_foot=person.and_then(|a|a.foot_positions.map(|pair|pair[usize::from(sign>0.)]));
                        let candidate=actor_foot.map_or([at[0]+side[0],at[2]+side[2]],|foot|[foot[0],foot[2]]);
                        let foot_height=actor_foot.map_or_else(||person.map_or(at[1],|a|a.feet[1]),|foot|foot[1]);
                        if let Some(w)=field.and_then(|f|f.sample_contact(candidate,mode,foot_height)) && w.wet_depth>0.006 && foot_height<w.level+0.12 && foot_height>w.ground-0.16 {
                            origin=[candidate[0],w.level,candidate[1]];foot_wet=true;
                            if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some(){
                                eprintln!("rain foot alignment: native={} foot={candidate:?} foot_y={foot_height:.3} surface={:.3} ground={:.3}",actor_foot.is_some(),w.level,w.ground);
                            }
                        }
                        // A few tiny droplets, only for a moving step in solved water.
                        if foot_wet&&speed>0.35 {for _ in 0..15 {
                            self.seed=self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                            let r=(self.seed>>8)as f32/16_777_216.;let angle=r*std::f32::consts::TAU;
                            if self.drops.len()<128 {self.drops.push(Droplet{position:[origin[0],origin[1]+0.003,origin[2]],
                                velocity:[angle.cos()*(0.12+r*0.15),0.65+r*0.50,angle.sin()*(0.12+r*0.15)],
                                age:0.,life:0.16+r*0.12,width:0.005+r*0.003,alpha:0.20+r*0.15,floor:origin[1]+0.0005});}
                        }}
                    }
                    if board {
                        // Alternate axles; each actual wet wheel disturbs its own patch.
                        let axle=usize::from(self.board_axle)*2;self.board_axle=!self.board_axle;
                        let mut emitted=false;
                        for i in axle..axle+2 {if let Some(wheel)=wheels[i] {self.wake(wheel,velocity,(0.30+speed*0.035).min(0.60),true);emitted=true;}}
                        if !emitted {for wheel in wheels.into_iter().flatten().take(2) {self.wake(wheel,velocity,(0.30+speed*0.035).min(0.60),true);}}
                    }else if foot_wet {self.wake(origin,velocity,0.46,false);}
                    if board||foot_wet {
                        self.last_wake=Some(at);self.last_board=board;self.since_wake=0.;
                        if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some(){eprintln!("rain contact wake: board={board} position={origin:?} speed={speed:0.2}");}
                    }
                }
                self.wet=true;
            }else{self.wet=false;self.fraction=0.;}
        }
        let mut output=Vec::with_capacity(self.drops.len()*4);
        for d in &self.drops{
            let relative=sub(d.position,camera.position);let distance=dot(relative,relative).sqrt();
            if dot(relative,camera.forward)<=0.||distance<0.25{continue;}
            let vx=dot(d.velocity,camera.right);let vy=dot(d.velocity,camera.up);let norm=vx.hypot(vy).max(0.001);
            let axis=std::array::from_fn::<_,3,_>(|i|(camera.right[i]*vx+camera.up[i]*vy)/norm);
            let side=std::array::from_fn::<_,3,_>(|i|(camera.right[i]*-vy+camera.up[i]*vx)/norm);
            let length=0.006+dot(d.velocity,d.velocity).sqrt().min(12.)*0.003;
            let alpha=d.alpha*(1.-d.age/d.life).powi(2)*((distance-0.25)/0.4).clamp(0.,1.)*((10.-distance)/4.).clamp(0.,1.);
            for(x,y,u,v)in[(-1.,-1.,2.,1.),(1.,-1.,3.,1.),(1.,1.,3.,0.),(-1.,1.,2.,0.)]{
                output.push(ParticleVertex{position:std::array::from_fn(|i|d.position[i]+side[i]*x*d.width+axis[i]*y*length),color:[0.78,0.79,0.80,alpha],uv:[u,v]});
            }
        }
        output
    }
}
fn position(m:[f32;16])->[f32;3]{[m[12],m[13],m[14]]}
#[cfg(test)]mod tests{
    use super::*;use crate::weather_hydrology::Ground;
    #[test]fn water_contact_wakes_and_fine_spray_reject_dry_air_replay_and_teleports(){
        let n=41;let water=Field::solve([-10.,-10.],0.5,n,(0..n*n).map(|i|{let x=(i%n)as f32-20.;let z=(i/n)as f32-20.;Ground{height:((x*x+z*z)*0.004).min(1.),valid:true,exposed:true}}).collect(),false);
        let dry=Field::solve([-10.,-10.],0.5,n,vec![Ground{height:0.,valid:true,exposed:true};n*n],false);
        for(field,height,replay,expect)in[(&dry,0.08,false,false),(&water,2.,false,false),(&water,0.08,true,false),(&water,0.08,false,true)]{
            let mut spray=Spray::default();let mut emitted=false;
            let camera=ParticleCamera{position:[0.,2.,-3.],right:[1.,0.,0.],up:[0.,1.,0.],forward:[0.,0.,1.]};
            *actor().lock().unwrap()=None;
            for tick in 0..50{
                let mut m=[0.;16];m[0]=1.;m[5]=1.;m[10]=1.;m[15]=1.;m[13]=height;m[14]=tick as f32*0.035;
                publish(Some(m),tick,replay);let v=spray.vertices(&camera,tick as f32/60.,Some(field),Mode::Storm);emitted|=!v.is_empty();
                assert!(v.len()<=128*4&&spray.wakes.len()<=8);assert!(v.iter().all(|v|v.position.iter().chain(&v.color).all(|f|f.is_finite())));
            }
            assert_eq!(emitted,expect);assert_eq!(!spray.wakes.is_empty(),expect);assert_eq!(spray.wake_bytes().len(),256);
            if expect{assert!(spray.wakes.iter().all(|w|w.speed<0.),"rolling board uses directional wakes");assert!(spray.wakes.len()<=8);assert!(spray.wakes.chunks_exact(2).any(|pair|(pair[0].position[0]-pair[1].position[0]).abs()>0.12),"separate wet wheels must leave separate disturbances");let mut m=[0.;16];m[0]=1.;m[5]=1.;m[10]=1.;m[15]=1.;m[12]=100.;publish(Some(m),51,false);spray.vertices(&camera,0.85,Some(field),Mode::Storm);assert!(spray.wakes.is_empty()&&spray.drops.is_empty());}
        }
        publish(None,60,false);
        let camera=ParticleCamera{position:[0.,2.,-3.],right:[1.,0.,0.],up:[0.,1.,0.],forward:[0.,0.,1.]};let mut spray=Spray::default();
        for tick in 0..50{let hips=[0.,1.,tick as f32*0.025];publish_actor(hips,[[0.,0.,0.],[1.,2.,1.]],None,tick,false);
            let mut carried=[0.;16];carried[0]=1.;carried[5]=0.3;carried[10]=1.;carried[15]=1.;carried[13]=0.7;carried[14]=hips[2];publish(Some(carried),tick,false);
            spray.vertices(&camera,tick as f32/60.,Some(&water),Mode::Storm);}
        assert!(!spray.wakes.is_empty(),"walking through actual water creates ripples");
        assert!(spray.wakes.iter().any(|w|w.age>0.12),"carried board must not clear foot wakes every tick");assert!(spray.wakes.iter().all(|w|w.speed>0.),"carried board uses foot wakes, not board wakes");
        // Published physics feet determine horizontal contact instead of the hips.
        publish(None,0,false);let mut real_feet=Spray::default();
        for tick in 0..50 {let z=tick as f32*0.025;let hips=[0.,1.,z];
            publish_actor(hips,[[0.,0.,0.],[1.,2.,1.]],Some([[-0.22,0.,z],[0.22,0.,z]]),tick,false);
            real_feet.vertices(&camera,tick as f32/60.,Some(&water),Mode::Storm);}
        assert!(!real_feet.wakes.is_empty() && real_feet.wakes.iter().all(|w|(w.position[0].abs()-0.22).abs()<0.001),"foot wakes follow published skeleton feet");
        *actor().lock().unwrap()=None;
        // Idle sub-centimetre animation jitter must not create a string of footsteps.
        publish(None,0,false);let mut idle=Spray::default();
        for tick in 0..180 {let z=(tick as f32*0.25).sin()*0.004;publish_actor([0.,1.,z],[[0.,0.,0.],[1.,2.,1.]],None,tick,false);idle.vertices(&camera,tick as f32/60.,Some(&water),Mode::Storm);}
        assert!(idle.wakes.is_empty()&&idle.drops.is_empty(),"idle jitter must not create ripples or spray");
        *actor().lock().unwrap()=None;
    }
}
