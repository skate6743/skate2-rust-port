//! Prepared CC0 rain beds from the Skate 3 weather layer's audio.
//! One additional voice in the ordinary stereo stream/dump/export, at the player's SFX level.
use std::sync::OnceLock;
fn beds()->&'static [Vec<f32>;2] {
    static BEDS:OnceLock<[Vec<f32>;2]>=OnceLock::new();
    BEDS.get_or_init(||[include_bytes!("weather_audio/light.pcm").as_slice(),include_bytes!("weather_audio/heavy.pcm").as_slice()]
        .map(balanced_bed))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn balancing_attenuates_only_and_keeps_stereo_detail() {
        let raw:Vec<i16>=(0..2048).flat_map(|i|[1000,if i%2==0 {2000} else {-2000}]).collect();
        let bytes:Vec<u8>=raw.iter().flat_map(|v|v.to_le_bytes()).collect();let output=balanced_bed(&bytes);
        for (frame,input) in output.chunks_exact(2).zip(raw.chunks_exact(2)) {
            assert_eq!(frame[0],f32::from(input[0])/32768.);
            assert_eq!(frame[1],f32::from(input[1])/65536.);
            assert!(frame.iter().zip(input).all(|(a,b)|a.abs()<=f32::from(*b).abs()/32768.));
        }
    }
    #[test]
    fn after_rain_birds_are_sparse_quiet_balanced_and_chunk_invariant() {
        let mut a=Birds::default();let mut whole=vec![0.;48000*2*35];a.mix(&mut whole,1.,true);
        assert!(whole[..48000*2*2].iter().all(|v|*v==0.));
        assert!(whole.iter().any(|v|v.abs()>0.001));
        assert!(whole.iter().all(|v|v.is_finite()&&v.abs()<0.01));
        assert!(whole.chunks_exact(2).all(|s|s[0]==s[1]));
        assert!(whole.chunks_exact(2).filter(|s|s[0].abs()>0.00001).count()<48000*9);
        let mut b=Birds::default();let mut pieces=vec![0.;whole.len()];for block in pieces.chunks_mut(1024){b.mix(block,1.,true);}assert_eq!(whole,pieces);
        for(gain,enabled)in[(0.,true),(1.,false)] {let mut block=vec![0.123;48000];a.mix(&mut block,gain,enabled);assert!(block.iter().all(|v|*v==0.123));}
    }
    #[test]
    fn thunder_follows_a_strike_and_respects_mute_and_stereo() {
        let mut voice=Thunder::default();let mut first=vec![0.;1024];voice.mix(&mut first,1.,true,1,48000);
        assert!(first.iter().all(|v|*v==0.));
        let mut early=vec![0.;48000*2];voice.mix(&mut early,1.,true,2,48000);assert!(early.iter().all(|v|*v==0.));
        let mut sound=vec![0.;48000*2*9];voice.mix(&mut sound,1.,true,2,48000);
        assert!(sound.iter().any(|v|v.abs()>0.0001));
        assert!(sound.iter().all(|v|v.is_finite()&&v.abs()<0.25));
        assert!(sound.chunks_exact(2).all(|v|v[0]==v[1]));
        voice.mix(&mut first,1.,false,2,48000);let mut muted=vec![0.;48000*2*9];
        voice.mix(&mut muted,0.,true,2,48000);voice.mix(&mut muted,0.,true,3,48000);assert!(muted.iter().all(|v|*v==0.));
    }
    #[test]
    fn original_light_and_heavy_recordings_have_balanced_energy() {
        for data in [include_bytes!("weather_audio/light.pcm").as_slice(),include_bytes!("weather_audio/heavy.pcm").as_slice()] {
            let output=balanced_bed(data);assert!(!output.is_empty());let mut energy=[0_f64;2];
            for frame in output.chunks_exact(2) {for c in 0..2 {assert!(frame[c].is_finite());energy[c]+=f64::from(frame[c]).powi(2);}}
            assert!(energy[0]>0. && energy[1]>0.);assert!((energy[0]/energy[1]-1.).abs()<0.00001);
        }
    }
}
// Rain recordings have a persistent channel bias. Match channel energy by
// attenuating the louder channel only; preserve stereo detail and source peaks.
fn balanced_bed(bytes:&[u8])->Vec<f32> {
    let mut samples:Vec<f32>=bytes.chunks_exact(2).map(|s|f32::from(i16::from_le_bytes([s[0],s[1]]))/32768.).collect();
    let mut energy=[0_f64;2];
    for frame in samples.chunks_exact(2) {for ch in 0..2 {let sample=f64::from(frame[ch]);energy[ch]+=sample*sample;}}
    let quiet=energy[0].min(energy[1]);
    let gain=energy.map(|e|if e>0. {(quiet/e).sqrt() as f32} else {0.});
    for frame in samples.chunks_exact_mut(2) {for ch in 0..2 {frame[ch]*=gain[ch];}}
    samples
}
// CC0 field-recorded calls, scheduled as sparse distant ambience after rain.
fn birds()->&'static Vec<f32> {
    static BIRDS:OnceLock<Vec<f32>>=OnceLock::new();
    BIRDS.get_or_init(||include_bytes!("weather_audio/after-rain-birds.pcm").chunks_exact(2)
        .map(|s|f32::from(i16::from_le_bytes([s[0],s[1]]))/32768.).collect())
}
#[derive(Default)]
struct Birds { enabled:bool,wait:usize,phase:usize,clip:usize,seed:u32 }
impl Birds {
    fn mix(&mut self,block:&mut [f32],gain:f32,enabled:bool) {
        if !enabled || gain<=0. {self.enabled=false;return;}
        if !self.enabled {self.enabled=true;self.wait=120000;self.phase=0;self.clip=0;self.seed=0x7354a218;}
        let samples=birds();let clip_samples=48000;
        for frame in block.chunks_exact_mut(2) {
            if self.wait>0 {self.wait-=1;continue;}
            // Source24kHz -> ordinary48kHz stream; keep the field recording's pitch.
            let at=self.clip*clip_samples+self.phase/2;let next=(at+1).min((self.clip+1)*clip_samples-1);
            let fraction=(self.phase%2) as f32*0.5;
            let value=(samples[at]+(samples[next]-samples[at])*fraction)*gain*0.012;
            frame[0]+=value;frame[1]+=value;
            self.phase+=1;
            if self.phase==clip_samples*2 {
                self.seed=self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                self.clip=(self.clip+1+((self.seed>>16) as usize%3))%4;self.phase=0;
                self.wait=48000*(7+(self.seed as usize%528000)/48000);
            }
        }
    }
}
// Public-domain field recording, mixed in the ordinary game stream equally
// into both channels. No system or device volume is changed.
fn thunder()->&'static Vec<f32> {
    static DATA:OnceLock<Vec<f32>>=OnceLock::new();
    DATA.get_or_init(||include_bytes!("weather_audio/thunder.pcm").chunks_exact(2)
        .map(|s|f32::from(i16::from_le_bytes([s[0],s[1]]))/32768.).collect())
}
#[derive(Default)]
struct Thunder {enabled:bool,seen:u64,wait:usize,phase:usize,playing:bool}
impl Thunder {
    fn mix(&mut self,block:&mut [f32],gain:f32,active:bool,serial:u64,delay:usize) {
        if !active {self.enabled=false;self.playing=false;self.seen=serial;return;}
        if !self.enabled {self.enabled=true;self.seen=serial;return;}
        if self.seen!=serial {
            self.seen=serial;self.wait=delay;
            self.phase=0;self.playing=true;
            if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some(){eprintln!("storm diagnostic: thunder scheduled strike={serial} delay_frames={}",self.wait);}
        }
        if !self.playing {return;}
        let samples=thunder();
        for frame in block.chunks_exact_mut(2) {
            if self.wait>0 {self.wait-=1;continue;}
            let at=self.phase/2;if at>=samples.len(){self.playing=false;break;}
            let next=(at+1).min(samples.len()-1);let f=(self.phase%2)as f32*0.5;
            let value=(samples[at]+(samples[next]-samples[at])*f)*gain*0.24;
            frame[0]+=value;frame[1]+=value;self.phase+=1;
        }
    }
}
/// The recordings decode once, on a worker, the first time a weather sounds: never for Vanilla
/// and never on the mixer's thread.
fn loaded()->bool {
    static LOADED:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
    static STARTED:std::sync::Once=std::sync::Once::new();
    if LOADED.load(std::sync::atomic::Ordering::Acquire) {return true;}
    STARTED.call_once(||{
        let _=std::thread::Builder::new().name("weather audio decode".into()).spawn(||{
            let _=beds();let _=birds();let _=thunder();
            LOADED.store(true,std::sync::atomic::Ordering::Release);
        });
    });
    false
}
#[derive(Default)]
pub(crate) struct Voice { at:usize,light:f32,heavy:f32,birds:Birds,thunder:Thunder }
impl Voice {
    pub fn mix(&mut self,block:&mut [f32],sfx:f32,paused:bool) {
        let mode=crate::weather::current();
        // Vanilla once the beds have faded: nothing is mixed.
        if mode==crate::weather::Mode::Vanilla && self.light<1e-4 && self.heavy<1e-4 && !self.thunder.playing {
            if self.light!=0. || self.heavy!=0. || self.birds.enabled || self.thunder.enabled {*self=Self{at:self.at,..Self::default()};}
            return;
        }
        if !loaded() {return;}
        let active=!paused&&crate::weather::visible();
        let sfx=sfx*crate::weather::rain_volume();
        self.birds.mix(block,sfx,active&&mode==crate::weather::Mode::AfterRain);
        let (strike,delay)=crate::rain_lightning::strike_event();
        self.thunder.mix(block,sfx,active&&matches!(mode,crate::weather::Mode::Rain|crate::weather::Mode::Storm),strike,delay);
        let targets=match (active,mode){
            (true,crate::weather::Mode::Drizzle)=>[0.7*sfx,0.],
            (true,crate::weather::Mode::Rain)=>[0.30*sfx,1.80*sfx],
            (true,crate::weather::Mode::Storm)=>[0.,2.40*sfx],_=>[0.,0.],
        };
        if targets==[0.,0.] && self.light==0. && self.heavy==0. {return;}
        let beds=beds();
        for frame in block.chunks_exact_mut(2) {
            self.light+=(targets[0]-self.light)*0.0001;self.heavy+=(targets[1]-self.heavy)*0.0001;
            for (channel,sample) in frame.iter_mut().enumerate() {
                *sample+=beds[0][(self.at+channel)%beds[0].len()]*self.light+beds[1][(self.at+channel)%beds[1].len()]*self.heavy;
            }
            self.at=self.at.wrapping_add(2);
        }
    }
}
