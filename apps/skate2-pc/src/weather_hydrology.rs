//! PC visual rainwater over the actual map. Priority-flood identifies closed
//! depressions; accumulated runoff volume sets a horizontal level in each basin.
//! Algorithm references/provenance: docs/evidence/rain-basin-water-2026-10-26.md.
//! This bounded local height-field model never changes skating collision/physics.
use std::{cmp::{Ordering,Reverse},collections::{BinaryHeap,VecDeque},sync::{Arc,Weak,mpsc}};
use crate::{weather::Mode,weather_world::World};

pub(crate) const SIZE:usize=257;
pub(crate) const STEP:f32=0.5;
const EMPTY:f32=-1_000_000.;
const UNLABELLED:usize=usize::MAX;

#[derive(Clone,Copy,Default)]
pub(crate) struct Ground {pub height:f32,pub valid:bool,pub exposed:bool}
#[derive(Clone,Copy)]
pub(crate) struct Cell {pub ground:f32,pub bed:f32,pub valid:f32,pub exposed:f32,pub levels:[f32;4]}
#[derive(Clone,Copy,Debug)]
pub(crate) struct Contact {pub level:f32,pub ground:f32,pub depth:f32,pub wet_depth:f32}
pub(crate) struct Field {pub origin:[f32;2],pub step:f32,pub size:usize,pub cells:Vec<Cell>,render_cells:Vec<Cell>,pub basins:usize}

#[derive(PartialEq)]struct Visit(f32,usize);
impl Eq for Visit {}
impl Ord for Visit {fn cmp(&self,other:&Self)->Ordering{self.0.total_cmp(&other.0).then(self.1.cmp(&other.1))}}
impl PartialOrd for Visit {fn partial_cmp(&self,other:&Self)->Option<Ordering>{Some(self.cmp(other))}}
fn neighbours(index:usize,size:usize)->impl Iterator<Item=usize> {
    let x=index%size;let z=index/size;
    (-1_i32..=1).flat_map(move|dz|(-1_i32..=1).filter_map(move|dx|{
        let a=x as i32+dx;let b=z as i32+dz;
        ((dx!=0 || dz!=0) && a>=0 && b>=0 && a<size as i32 && b<size as i32)
            .then(||b as usize*size+a as usize)
    }))
}
pub(crate) fn mode_index(mode:Mode)->usize {
    match mode {Mode::Drizzle=>0,Mode::Rain=>1,Mode::Storm=>2,_=>3}
}
impl Field {
    fn build(world:&World,key:[i32;3])->Self {
        let origin=[key[0] as f32*32.-64.,key[1] as f32*32.-64.];
        let ceiling=(key[2] as f32+1.)*8.+4.;
        let ground=world.rain_columns(origin,STEP,SIZE,ceiling);
        Self::solve_named_pools(origin,STEP,SIZE,ground)
    }
    // The streamed presentation meshes omit portions of some authored pool
    // floors. These small, local grids were sampled from the corresponding
    // native collision surfaces. Only use them where neighbouring live ground
    // agrees, and only for these seven outdoor pools; the normal water field
    // remains the authority everywhere else.
    pub(crate) fn solve_named_pools(origin:[f32;2],step:f32,size:usize,ground:Vec<Ground>)->Self {
        const GRID:usize=73;
        const RECORD:usize=8+GRID*GRID*4;
        const DATA:&[u8]=include_bytes!(concat!(env!("OUT_DIR"), "/pool-ground.bin"));
        const CENTRES:[[f32;2];7]=[[1455.3,-288.2],[815.3,-418.5],[1251.3,-647.2],
            [1532.4,-591.4],[907.2,1318.4],[915.9,-411.5],[1316.0,-470.8]];
        // Party Pool is surrounded by another level; its verified floor is
        // restricted to the inner bowl so elevated geometry stays untouched.
        const RADII:[f32;7]=[18.,18.,18.,18.,18.,18.,10.];
        let mut augmented=ground.clone();let mut included=vec![false;ground.len()];
        let mut pool_levels=vec![None;ground.len()];let mut changed=false;
        if (step-0.5).abs()>0.00001 {return Self::solve(origin,step,size,ground,true);}
        for (pool,((record,center),radius)) in DATA.chunks_exact(RECORD).zip(CENTRES).zip(RADII).enumerate() {
            let num=|at:usize|f32::from_le_bytes(record[at..at+4].try_into().unwrap());
            let base=[num(0),num(4)];let mut samples=Vec::new();let mut all_samples=Vec::new();let mut inner_heights=Vec::new();let(mut compared,mut matches)=(0,0);
            for z in 0..GRID {for x in 0..GRID {
                let point=[base[0]+x as f32*step,base[1]+z as f32*step];
                if (point[0]-center[0]).powi(2)+(point[1]-center[1]).powi(2)>radius.powi(2) {continue;}
                let ax=((point[0]-origin[0])/step).round() as i32;let az=((point[1]-origin[1])/step).round() as i32;
                if ax<0||az<0||ax>=size as i32||az>=size as i32 {continue;}
                let index=az as usize*size+ax as usize;let y=num(8+(z*GRID+x)*4);
                if !y.is_finite(){continue;}
                if ground[index].valid {compared+=1;if (ground[index].height-y).abs()<0.28 {matches+=1;}else {continue;}}
                else {samples.push((index,y));}
                all_samples.push((index,y));
                if (point[0]-center[0]).powi(2)+(point[1]-center[1]).powi(2)<100. {inner_heights.push(y); }
            }}
            // Require a substantial, close match with this live map to avoid
            // transferring water to another floor, custom map or stale stream.
            if compared<64 || matches*100<compared*85 || inner_heights.len()<64 {continue;}
            for (i,y) in samples {augmented[i]=Ground{height:y,valid:true,exposed:true};included[i]=true;changed=true;}
            // A pool collects a shallow sheet at its lowest verified floor.
            // The percentile rejects isolated lower-level triangles. Four levels
            // correspond to light, medium, heavy and the after-rain setting.
            inner_heights.sort_unstable_by(f32::total_cmp);
            let bottom=inner_heights[inner_heights.len()/100];
            // Party Pool slopes more sharply into the verified inner basin.
            // Its shallower presets retain a smaller depth; heavy fills further.
            let levels=if pool==6 {[bottom+0.20,bottom+0.48,bottom+0.88,bottom+0.30]}
                else {[bottom+0.04,bottom+0.14,bottom+0.32,bottom+0.08]};
            for (i,y) in all_samples {if y<levels[2]-0.002 {pool_levels[i]=Some(levels);}}

        }
        let mut original=Self::solve(origin,step,size,ground,true);
        if !changed{return original;}
        let candidate=Self::solve(origin,step,size,augmented,true);
        for (i,new) in candidate.cells.iter().enumerate() {
            let mut proposed=*new;
            if let Some(levels)=pool_levels[i] {proposed.levels=levels;}
            // Never change an existing wet cell: this preserves already-good
            // pools and their physics. New water still follows the native floor.
            let old=original.cells[i];
            let already_wet=old.valid>0.5 && old.levels.iter().any(|h|*h>old.bed+0.002);
            let pool_area=CENTRES.iter().zip(RADII).any(|(c,radius)|{
                let x=origin[0]+(i%size)as f32*step;let z=origin[1]+(i/size)as f32*step;
                (x-c[0]).powi(2)+(z-c[1]).powi(2)<=radius.powi(2)
            });
            if !already_wet && (included[i]||pool_area) && proposed.valid>0.5 && proposed.levels.iter().any(|h|*h>proposed.bed+0.002) {
                original.cells[i]=proposed;
            }
        }
        original.render_cells=Self::continuous_cells(&original.cells,size);
        original
    }
    /// Independent equilibrium volumes for preset rainfall histories in metres.
    /// These are authored presentation amounts, not elapsed-time weather forecasts.
    pub(crate) fn solve(origin:[f32;2],step:f32,size:usize,ground:Vec<Ground>,micro_relief:bool)->Self {
        assert_eq!(ground.len(),size*size);
        // Missing samples enclosed by valid map geometry often represent a
        // sub-grid vertical wall or a tessellation seam, not a drainage outlet.
        // Close only bounded interior gaps in the solver; keep them invalid to
        // the renderer/contact sampler so no synthetic floor is drawn there.
        let mut support:Vec<Option<f32>>=ground.iter().map(|g|g.valid.then_some(g.height)).collect();
        let mut checked=vec![false;ground.len()];
        for start in 0..ground.len() {
            if ground[start].valid || checked[start] {continue;}
            let mut queue=VecDeque::from([start]);checked[start]=true;
            let mut gap=Vec::new();let mut boundary=false;let mut crest=f32::NEG_INFINITY;
            while let Some(i)=queue.pop_front() {
                gap.push(i);let x=i%size;let z=i/size;
                boundary|=x==0 || z==0 || x+1==size || z+1==size;
                for j in neighbours(i,size) {
                    if ground[j].valid {crest=crest.max(ground[j].height);}
                    else if !checked[j] {checked[j]=true;queue.push_back(j);}
                }
            }
            if !boundary && gap.len()<=256 && crest.is_finite()
                && gap.iter().all(|i|(-2_i32..=2).any(|dz|(-2_i32..=2).any(|dx|{
                    let x=(*i%size)as i32+dx;let z=(*i/size)as i32+dz;
                    x>=0&&z>=0&&x<size as i32&&z<size as i32&&ground[z as usize*size+x as usize].valid
                }))) {
                for i in gap {support[i]=Some(crest);}
            }
        }
        let supported:Vec<_>=support.iter().map(Option::is_some).collect();
        let mut cells:Vec<_>=ground.iter().enumerate().map(|(i,g)|{
            let point=[origin[0]+(i%size) as f32*step,0.,origin[1]+(i/size) as f32*step];
            // Bounded virtual surface relief gives otherwise perfectly flat
            // collision slabs shallow pockets. Actual bowl height dominates it.
            let relief=if micro_relief {crate::weather_puddles::field(point)*0.045}else{0.};
            Cell{ground:g.height,bed:support[i].unwrap_or(g.height)+relief,valid:if g.valid {1.}else{0.},
                exposed:if g.exposed{1.}else{0.},levels:[EMPTY;4]}
        }).collect();
        let mut filled=vec![EMPTY;cells.len()];let mut seen=vec![false;cells.len()];let mut heap=BinaryHeap::new();
        for i in 0..cells.len() {
            if !supported[i] {seen[i]=true;continue;}
            let x=i%size;let z=i/size;
            if x==0 || z==0 || x+1==size || z+1==size || neighbours(i,size).any(|j|!supported[j]) {
                seen[i]=true;filled[i]=cells[i].bed;heap.push(Reverse(Visit(filled[i],i)));
            }
        }
        while let Some(Reverse(Visit(level,i)))=heap.pop() {
            for j in neighbours(i,size) {if !seen[j] {
                seen[j]=true;filled[j]=cells[j].bed.max(level);heap.push(Reverse(Visit(filled[j],j)));
            }}
        }
        let mut labels=vec![UNLABELLED;cells.len()];let mut groups:Vec<Vec<usize>>=Vec::new();
        for i in 0..cells.len() {
            if !supported[i] || labels[i]!=UNLABELLED || filled[i]-cells[i].bed<0.0015 {continue;}
            let label=groups.len();let mut group=Vec::new();let mut queue=VecDeque::from([i]);labels[i]=label;
            while let Some(j)=queue.pop_front() {
                group.push(j);
                for k in neighbours(j,size) {
                    if supported[k] && labels[k]==UNLABELLED && filled[k]-cells[k].bed>=0.0015
                        && (filled[k]-filled[i]).abs()<0.001 {
                        labels[k]=label;queue.push_back(k);
                    }
                }
            }
            groups.push(group);
        }
        // Route each exposed cell's rain downhill, stopping at a closed basin.
        // Covered interiors contribute no rain; outside runoff may enter a basin.
        let area=step*step;let mut runoff:Vec<_>=cells.iter().map(|c|c.exposed*area).collect();
        let mut order:Vec<_>=(0..cells.len()).filter(|i|supported[*i]).collect();
        order.sort_unstable_by(|a,b|cells[*b].bed.total_cmp(&cells[*a].bed));
        let mut catchments=vec![0.;groups.len()];
        for i in order {
            if labels[i]!=UNLABELLED {catchments[labels[i]]+=runoff[i];continue;}
            if let Some(j)=neighbours(i,size).filter(|j|supported[*j] && cells[*j].bed<cells[i].bed-0.00001)
                .min_by(|a,b|cells[*a].bed.total_cmp(&cells[*b].bed)) {runoff[j]+=runoff[i];}
        }
        let rainfall=[0.006_f32,0.022,0.055,0.012];
        for (label,group) in groups.iter_mut().enumerate() {
            if catchments[label]<=0. {continue;}
            group.sort_unstable_by(|a,b|cells[*a].bed.total_cmp(&cells[*b].bed));
            let spill=filled[group[0]];let bottom=cells[group[0]].bed;
            let mut levels=[EMPTY;4];
            for (preset,rain) in rainfall.into_iter().enumerate() {
                let mut volume=catchments[label]*rain;let mut level=bottom;let mut count=1.;
                for &i in &group[1..] {
                    let next=cells[i].bed;let needed=(next-level)*count*area;
                    if needed>=volume {level+=volume/(count*area);volume=0.;break;}
                    volume-=needed;level=next;count+=1.;
                }
                if volume>0. {level+=volume/(count*area);}
                levels[preset]=level.min(spill).min(bottom+0.65);
            }
            for &i in group.iter() {cells[i].levels=levels;}
        }
        let render_cells=Self::continuous_cells(&cells,size);
        Self{origin,step,size,cells,render_cells,basins:groups.len()}
    }
    // Resolve sparse visual sampling seams once on the preparation worker,
    // instead of repeating25 neighbouring storage reads in millions of pixels.
    // These optical candidates never become skating collision/contact geometry.
    fn continuous_cells(cells:&[Cell],size:usize)->Vec<Cell> {
        let mut render=cells.to_vec();
        for i in 0..cells.len() {
            if cells[i].valid>0.5 {continue;}
            let x=(i%size) as i32;let z=(i/size) as i32;
            let mut weight=0.;let mut ground=0.;let mut relief=0.;let mut levels=[EMPTY;4];
            for dz in -2_i32..=2 {for dx in -2_i32..=2 {
                let a=x+dx;let b=z+dz;
                if a<0 || b<0 || a>=size as i32 || b>=size as i32 {continue;}
                let cell=cells[b as usize*size+a as usize];if cell.valid<0.5 {continue;}
                let w=1./(0.2+(dx*dx+dz*dz) as f32);weight+=w;ground+=cell.ground*w;relief+=(cell.bed-cell.ground)*w;
                for (m,level) in levels.iter_mut().enumerate() {
                    if cell.levels[m]>cell.ground && cell.levels[m]<cell.ground+1. {*level=level.max(cell.levels[m]);}
                }
            }}
            if weight>0. && levels.iter().any(|h|*h>EMPTY) {
                render[i]=Cell{ground:ground/weight,bed:(ground+relief)/weight,valid:1.,exposed:0.,levels};
            }
        }
        render
    }
    pub(crate) fn gpu_bytes(&self)->Vec<u8> {
        self.render_cells.iter().enumerate().flat_map(|(i,c)|[c.ground,c.bed,c.valid,self.cells[i].valid,c.levels[0],c.levels[1],c.levels[2],c.levels[3]])
            .flat_map(f32::to_ne_bytes).collect()
    }
    pub(crate) fn sample(&self,point:[f32;2],mode:Mode)->Option<Contact> {
        let x=(point[0]-self.origin[0])/self.step;let z=(point[1]-self.origin[1])/self.step;
        if x<0. || z<0. || x>=(self.size-1) as f32 || z>=(self.size-1) as f32 {return None;}
        let a=x.floor() as usize;let b=z.floor() as usize;let u=x.fract();let v=z.fract();
        let list=[(b*self.size+a,(1.-u)*(1.-v)),(b*self.size+a+1,u*(1.-v)),
            ((b+1)*self.size+a,(1.-u)*v),((b+1)*self.size+a+1,u*v)];
        if list.iter().any(|(i,w)|*w>0.0001 && self.cells[*i].valid<0.5) {return None;}
        let ground=list.iter().map(|(i,w)|self.cells[*i].ground*w).sum::<f32>();
        let su=u*u*u*(u*(u*6.-15.)+10.);let sv=v*v*v*(v*(v*6.-15.)+10.);
        let bed_weights=[(1.-su)*(1.-sv),su*(1.-sv),(1.-su)*sv,su*sv];
        let bed=list.iter().zip(bed_weights).map(|((i,_),weight)|self.cells[*i].bed*weight).sum::<f32>();
        // Match the rendered surface height, rather than selecting the highest
        // neighbouring cell and emitting contact above the local puddle.
        let mut weight=0.;let mut sum=0.;
        for ((i,_),w) in list.iter().zip(bed_weights){let h=self.cells[*i].levels[mode_index(mode)];
            if h>=ground && h<ground+1. {weight+=w;sum+=w*h;}}
        if weight<0.75{return None;}
        let level=sum/weight;
        (level>bed+0.0015).then_some(Contact{level,ground,depth:level-ground,wet_depth:level-bed})
    }
    // Recover visual contact at a small interpolation seam only when most
    // nearby cells are native geometry and the actor is at that same floor.
    // Render-only inferred cells never become collision or skating geometry.
    pub(crate) fn sample_contact(&self,point:[f32;2],mode:Mode,actor_y:f32)->Option<Contact> {
        if let Some(contact)=self.sample(point,mode){return Some(contact);}
        let x=(point[0]-self.origin[0])/self.step;let z=(point[1]-self.origin[1])/self.step;
        if !actor_y.is_finite() || x<0. || z<0. || x>=(self.size-1) as f32 || z>=(self.size-1) as f32{return None;}
        let a=x.floor() as usize;let b=z.floor() as usize;let u=x.fract();let v=z.fract();
        let list=[(b*self.size+a,(1.-u)*(1.-v)),(b*self.size+a+1,u*(1.-v)),
            ((b+1)*self.size+a,(1.-u)*v),((b+1)*self.size+a+1,u*v)];
        let confidence=list.iter().filter(|(i,_)|self.cells[*i].valid>0.5).map(|(_,w)|w).sum::<f32>();
        if confidence<0.75 || list.iter().any(|(i,w)|*w>0.0001&&self.render_cells[*i].valid<0.5){return None;}
        let ground=list.iter().map(|(i,w)|self.render_cells[*i].ground*w).sum::<f32>();
        if (actor_y-ground).abs()>0.16{return None;}
        let su=u*u*u*(u*(u*6.-15.)+10.);let sv=v*v*v*(v*(v*6.-15.)+10.);
        let weights=[(1.-su)*(1.-sv),su*(1.-sv),(1.-su)*sv,su*sv];
        let bed=list.iter().zip(weights).map(|((i,_),w)|self.render_cells[*i].bed*w).sum::<f32>();
        let mut weight=0.;let mut sum=0.;
        for ((i,_),w) in list.iter().zip(weights){let h=self.render_cells[*i].levels[mode_index(mode)];
            if h>=ground&&h<ground+1.{weight+=w;sum+=w*h;}}
        if weight<0.75{return None;}
        let level=sum/weight;
        (level>bed+0.0015).then_some(Contact{level,ground,depth:level-ground,wet_depth:level-bed})
    }
    pub(crate) fn bytes(&self)->Vec<u8> {
        self.cells.iter().flat_map(|c|[c.ground,c.bed,c.valid,c.exposed,c.levels[0],c.levels[1],c.levels[2],c.levels[3]])
            .flat_map(f32::to_ne_bytes).collect()
    }
    fn diagnostics(&self,key:[i32;3],elapsed:f32) {
        let wet:Vec<_>=(0..4).map(|m|self.cells.iter().filter(|c|c.valid>0.5 && c.levels[m]>c.bed+0.002).count()).collect();
        eprintln!("rain basin field: origin={:?} basins={} wet_cells={wet:?} cells={} prepared={elapsed:.3}s",self.origin,self.basins,self.cells.len());
        if let Some(root)=std::env::var_os("SKATE2_DIAGNOSTIC_RAIN_FIELD_DUMP") {
            let root=std::path::PathBuf::from(root);let _=std::fs::create_dir_all(&root);
            let header=[self.origin[0],self.origin[1],self.step,self.size as f32];
            let mut bytes:Vec<_>=header.into_iter().flat_map(f32::to_ne_bytes).collect();bytes.extend(self.bytes());
            let _=std::fs::write(root.join(format!("field-{}-{}-{}.bin",key[0],key[1],key[2])),bytes);
        }
    }
}

type Pending=([i32;3],Weak<World>,mpsc::Receiver<Arc<Field>>);
type Active=([i32;3],Weak<World>,Arc<Field>);
#[derive(Default)]
pub(crate) struct Manager {world:Weak<World>,location:String,active:Option<Active>,cache:VecDeque<([i32;3],Arc<Field>)>,pending:Option<Pending>}
fn stable_key(anchor:[f32;3],previous:Option<[i32;3]>)->[i32;3] {
    let wanted=[(anchor[0]/32.).round() as i32,(anchor[2]/32.).round() as i32,(anchor[1]/8.).floor() as i32];
    let Some(old)=previous else{return wanted;};
    let mut key=old;
    if (anchor[0]-old[0] as f32*32.).abs()>24. {key[0]=wanted[0];}
    if (anchor[2]-old[1] as f32*32.).abs()>24. {key[1]=wanted[1];}
    if anchor[1]<old[2] as f32*8.-3. || anchor[1]>(old[2]+1) as f32*8.+3. {key[2]=wanted[2];}
    key
}
impl Manager {
    pub(crate) fn update(&mut self,world:&Arc<World>,anchor:[f32;3],location:&str)->Option<Arc<Field>> {
        let weak=Arc::downgrade(world);
        if self.location!=location {self.location=location.into();self.active=None;self.cache.clear();}
        // Streaming publishes additional geometry for the SAME place. Keep its
        // current water while preparing the replacement, rather than flashing dry.
        if !Weak::ptr_eq(&self.world,&weak) {self.world=weak.clone();self.cache.clear();}
        if let Some((key,owner,receiver))=&self.pending {
            match receiver.try_recv() {
                Ok(field)=>{if Weak::ptr_eq(owner,&weak) {
                    self.cache.push_back((*key,Arc::clone(&field)));while self.cache.len()>4 {self.cache.pop_front();}
                    self.active=Some((*key,owner.clone(),field));
                }self.pending=None;},
                Err(mpsc::TryRecvError::Disconnected)=>self.pending=None,
                Err(mpsc::TryRecvError::Empty)=>{},
            }
        }
        let key=stable_key(anchor,self.active.as_ref().map(|(key,_,_)|*key));
        if self.active.as_ref().is_none_or(|(current,owner,_)|*current!=key || !Weak::ptr_eq(owner,&weak)) {
            if let Some((_,field))=self.cache.iter().find(|(cached,_)|*cached==key) {
                self.active=Some((key,weak.clone(),Arc::clone(field)));
            } else if self.pending.is_none() && !world.triangles.is_empty() {
                let (tx,rx)=mpsc::channel();let retained=Arc::clone(world);
                if std::thread::Builder::new().name("rain basin preparation".into()).spawn(move||{
                    let start=std::time::Instant::now();let field=Arc::new(Field::build(&retained,key));
                    field.diagnostics(key,start.elapsed().as_secs_f32());let _=tx.send(field);
                }).is_ok() {self.pending=Some((key,weak,rx));}
            }
        }
        self.active.as_ref().map(|(_,_,field)|Arc::clone(field))
    }
}

#[cfg(test)]mod tests {
    use super::*;
    fn bowl(exposed:bool)->Field {
        let n=33;let ground=(0..n*n).map(|i|{
            let x=(i%n) as f32-16.;let z=(i/n) as f32-16.;
            Ground{height:((x*x+z*z)*0.015).min(2.),valid:true,exposed}
        }).collect();Field::solve([-8.,-8.],0.5,n,ground,false)
    }
    #[test]fn bowl_water_is_level_and_heavier_rain_collects_more(){
        let f=bowl(true);let light=f.sample([0.,0.],Mode::Drizzle).unwrap();let heavy=f.sample([0.,0.],Mode::Storm).unwrap();
        assert!(heavy.depth>light.depth*1.5 && heavy.depth<=0.651);
        let side=f.sample([0.5,0.],Mode::Storm).unwrap();assert!((side.level-heavy.level).abs()<0.0001);
        assert!(f.sample([0.,0.],Mode::AfterRain).unwrap().depth>light.depth);
        assert!(f.sample([7.,7.],Mode::Storm).is_none());
    }
    #[test]fn neighbouring_bowls_keep_independent_ground_and_water_levels(){
        let n=33;let mut ground=vec![Ground{height:2.,valid:true,exposed:true};n*n];
        for z in 6..27 {for x in 3..14 {ground[z*n+x].height=0.;}for x in 19..30 {ground[z*n+x].height= -0.35;}}
        let f=Field::solve([0.,0.],0.5,n,ground,false);
        let first=f.sample([4.,8.],Mode::Storm).expect("first bowl collects rain");
        let second=f.sample([12.,8.],Mode::Storm).expect("neighbour collects rain");
        assert!(first.depth>0.02&&second.depth>0.02);
        assert!((first.ground-0.).abs()<0.001 && (second.ground+0.35).abs()<0.001);
        assert!((first.level-second.level-0.35).abs()<0.001,"independent bowls must not share a floating plane");
        assert!(f.sample([8.,8.],Mode::Storm).is_none(),"high separating ground stays dry");
    }
    #[test]fn contact_height_matches_local_interpolated_water_surface(){
        let cells:Vec<_>=[0.10,0.20,0.30,0.40].into_iter().map(|h|Cell{ground:0.,bed:0.,valid:1.,exposed:1.,levels:[h;4]}).collect();
        let field=Field{origin:[0.,0.],step:1.,size:2,render_cells:cells.clone(),cells,basins:1};
        let h=field.sample([0.25,0.5],Mode::Storm).unwrap().level;
        let u=0.25f32;let smooth=u*u*u*(u*(u*6.-15.)+10.);
        let expected=0.20+smooth*0.10;
        assert!((h-expected).abs()<0.00001 && h<0.40,"surface uses local weighted level, not neighbour maximum");
    }
    #[test]fn covered_bowl_and_open_slope_do_not_gain_water(){
        assert!(bowl(false).sample([0.,0.],Mode::Storm).is_none());
        let n=17;let ground=(0..n*n).map(|i|Ground{height:(i%n) as f32*0.1,valid:true,exposed:true}).collect();
        let f=Field::solve([0.,0.],0.5,n,ground,false);assert!(f.cells.iter().all(|c|c.levels[2]==EMPTY));
    }
    #[test]fn enclosed_sampling_gap_preserves_basin_without_drawing_fake_floor(){
        let n=17;let mut ground=vec![Ground{height:2.,valid:true,exposed:true};n*n];
        for z in 4..13 {for x in 4..13 {ground[z*n+x].height=0.;}}
        let seam=8*n+8;ground[seam]=Ground::default();
        let field=Field::solve([0.,0.],0.5,n,ground,false);
        assert!(field.sample([3.,3.],Mode::Storm).unwrap().depth>0.05);
        assert!(field.sample([4.,4.],Mode::Storm).is_none());
        assert_eq!(field.cells[seam].valid,0.);
        assert_eq!(field.render_cells[seam].valid,1.);
        assert!(field.render_cells[seam].levels[2]>0.);
        let near=[3.6,3.6];
        assert!(field.sample(near,Mode::Storm).is_none());
        assert!(field.sample_contact(near,Mode::Storm,0.).is_some(),"a narrow visible seam can make contact over verified ground");
        assert!(field.sample_contact(near,Mode::Storm,3.).is_none(),"another floor must not create a contact");
        assert!(field.sample_contact([4.,4.],Mode::Storm,0.).is_none(),"missing native terrain cannot become contact");
        assert_eq!(field.gpu_bytes().len(),n*n*32);
    }
    #[test]fn missing_terrain_open_to_boundary_remains_a_drain(){
        let n=17;let mut ground=vec![Ground{height:2.,valid:true,exposed:true};n*n];
        for z in 4..13 {for x in 4..13 {ground[z*n+x].height=0.;}}
        for x in 0..9 {ground[8*n+x]=Ground::default();}
        let field=Field::solve([0.,0.],0.5,n,ground,false);
        assert!(field.sample([3.,3.],Mode::Storm).is_none());
    }
    #[test]fn water_keys_do_not_follow_camera_bob_or_jitter_at_tile_edges(){
        let key=[25,28,-8];
        assert_eq!(stable_key([817.,-56.1,911.],Some(key)),key);
        assert_eq!(stable_key([816.,-55.9,912.],Some(key)),key);
        assert_eq!(stable_key([825.,-50.,921.],Some(key)),[26,29,-7]);
    }
    #[test]fn broad_missing_geometry_is_not_invented_as_a_dam(){
        let n=17;let mut ground=vec![Ground{height:2.,valid:true,exposed:true};n*n];
        for z in 3..14 {for x in 3..14 {ground[z*n+x].height=0.;}}
        for z in 6..11 {for x in 6..11 {ground[z*n+x]=Ground::default();}}
        let f=Field::solve([0.,0.],0.5,n,ground,false);
        assert!(f.sample([2.,2.],Mode::Storm).is_none());
    }
    #[test]fn authored_pool_support_rejects_unrelated_ground(){
        let n=257;let origin=[1392.,-352.];
        let ground=vec![Ground{height:999.,valid:true,exposed:true};n*n];
        let original=Field::solve(origin,0.5,n,ground.clone(),true);
        let result=Field::solve_named_pools(origin,0.5,n,ground);
        assert_eq!(original.bytes(),result.bytes(),"a different floor must not use stored pool geometry");
    }
    #[test]fn flat_terrain_has_small_pockets_without_flooding_every_cell(){
        let n=65;let f=Field::solve([0.,0.],0.5,n,vec![Ground{height:0.,valid:true,exposed:true};n*n],true);
        let wet=f.cells.iter().filter(|c|c.levels[2]>c.bed+0.002).count();assert!(wet>n && wet<n*n*9/10,"wet={wet}");
        assert!(f.cells.iter().all(|c|c.levels[2]<=0.046));
    }
}
