//! Gameplay control does not require permission to open the front end.
//! In particular Net::ResumeGame masks simulation pauses independently of the
//! top PauseMan state used by CheckForPauseState826477b0.
pub(super) fn available(sim_input:bool,focused:bool,frontend:bool,loading:bool,popup:bool)->bool {
    trace([sim_input,focused,frontend,loading,popup]);
    sim_input && focused && !frontend && !loading && !popup
}
/// `SKATE2_TRACE_SKATE3=1`: the gate's inputs whenever they change.
fn trace(inputs:[bool;5]) {
    use std::sync::atomic::{AtomicU8,Ordering};
    static LAST:AtomicU8=AtomicU8::new(u8::MAX);
    if !crate::diagnostics::test_log::enabled() {return;}
    let bits=inputs.iter().enumerate().fold(0u8,|bits,(i,&on)|bits|(u8::from(on)<<i));
    if LAST.swap(bits,Ordering::Relaxed)!=bits {
        let [sim_input,focused,frontend,loading,popup]=inputs;
        crate::diagnostics::test_log::event("focus",format_args!(
            "sim_input={sim_input} focused={focused} frontend={frontend} loading={loading} popup={popup}"));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_network_input_survives_a_non_menu_pause_owner() {
        let pause=crate::selected_pause_manager::SelectedPauseManager::construct();
        pause.try_push(23).unwrap();pause.try_set_network_resumed(true).unwrap();
        assert!(!pause.try_can_open_frontend().unwrap());
        assert!(pause.try_is_active(1).unwrap());
        assert!(available(pause.try_is_active(1).unwrap(),true,false,false,false));
        for args in [(false,true,false,false,false),(true,false,false,false,false),
            (true,true,true,false,false),(true,true,false,true,false),(true,true,false,false,true)] {
            assert!(!available(args.0,args.1,args.2,args.3,args.4));
        }
    }
}
