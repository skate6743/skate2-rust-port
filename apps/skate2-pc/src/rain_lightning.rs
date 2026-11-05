//! Local presentation clock for the separately requested Daytryan lightning shader.
//! Bolt shape and flash gain are source-derived; this bounded schedule is a PC adapter.
use std::sync::atomic::{AtomicU32,AtomicU64,Ordering};
static LAST_CYCLE:AtomicU32=AtomicU32::new(u32::MAX);
// Atomically publish a strike serial together with its travel delay, so the
// audio callback cannot mix the distance from a different lightning event.
static STRIKE:AtomicU64=AtomicU64::new(0);
fn distance_metres(cycle:u32)->f32 {
    let mut x=cycle.wrapping_add(0x20c9_1f43);
    x^=x>>16;x=x.wrapping_mul(0x7feb_352d);x^=x>>15;
    350.+(x>>8) as f32/16_777_216.*1450.
}
fn travel_frames(metres:f32)->u32 { (metres/343.0*48_000.).round() as u32 }
/// One event for the two visual pulses of a single strike.
pub(crate) fn observe(flash:[f32;4]) {
    if flash[0]<=0. {return;}
    let cycle=flash[2] as u32;
    if LAST_CYCLE.swap(cycle,Ordering::Relaxed)!=cycle {
        let distance=flash[3].clamp(350.,1800.);let delay=travel_frames(distance);
        let _=STRIKE.fetch_update(Ordering::Release,Ordering::Relaxed,|old|Some((((old>>32)+1)<<32)|u64::from(delay)));
        if std::env::var_os("SKATE2_DIAGNOSTIC_RAIN").is_some(){eprintln!("storm diagnostic: lightning strike cycle={cycle} distance_m={distance:.0} delay_frames={delay}");}
    }
}
pub(crate) fn strike_event()->(u64,usize) {
    let event=STRIKE.load(Ordering::Acquire);(event>>32,(event as u32) as usize)
}
pub(crate) fn state(time:f32,intensity:f32)->[f32;4] {
    if !time.is_finite() || intensity<0.6 {return [0.;4];}
    let interval=if intensity>0.9 {24.}else{42.};
    let cycle=(time.max(0.)/interval).floor() as u32;
    let mut seed=cycle.wrapping_add(0x5354_4f52);
    seed^=seed>>16;seed=seed.wrapping_mul(0x7feb_352d);seed^=seed>>15;
    let unit=(seed>>8) as f32/16_777_216.;
    let age=time.rem_euclid(interval)-(4.+unit*(interval-8.));
    let pulse=|centre:f32,width:f32| (1.-((age-centre)/width).abs()).max(0.);
    let flash=(pulse(0.,0.075)+pulse(0.17,0.055)*0.55).min(1.);
    [flash,unit*std::f32::consts::TAU,cycle as f32,distance_metres(cycle)]
}
#[cfg(test)]mod tests{
    #[test]fn distance_controls_sound_travel_delay(){
        assert_eq!(super::travel_frames(343.),48_000);
        assert_eq!(super::travel_frames(1029.),144_000);
        assert!(super::distance_metres(0)>=350.&&super::distance_metres(0)<=1800.);
    }
    #[test]fn bounded_and_clear_or_light_never_flash(){
        for i in 0..20_000 {let time=i as f32/120.;for intensity in [0.,0.3,0.65,1.] {
            let state=super::state(time,intensity);assert!(state.iter().all(|v|v.is_finite()));assert!((0.0..=1.).contains(&state[0]));
            if intensity<0.6 {assert_eq!(state,[0.;4]);}
        }}
        assert!((0..20_000).any(|i|super::state(i as f32/120.,1.)[0]>0.5));
    }
}
