use skate_data::input_recording::Recording;
fn line(sample: u64, t_us: u64, connected: bool, right_y: i16) -> String {
    format!(
        "{{\"schema\":1,\"sample\":{sample},\"t_us\":{t_us},\"connected\":{connected},\"packet\":{},\"buttons\":0,\"triggers\":[0,0],\"left\":[0,0],\"right\":[0,{right_y}]}}\n",
        if connected { "5" } else { "null" }
    )
}
#[test]
fn irregular_timing_keeps_edges_axes_and_disconnects() {
    let text =
        line(0, 18000, true, -32768) + &line(1, 60000, true, 32767) + &line(2, 65000, false, 0);
    let recording = Recording::read(text.as_bytes()).unwrap();
    assert_eq!(recording.duration_us(), 47000);
    assert_eq!(recording.at_elapsed_us(41999).raw_state().right[1], -32768);
    assert_eq!(recording.at_elapsed_us(42000).raw_state().right[1], 32767);
    assert!(!recording.at_elapsed_us(47000).connected);
    assert!(!recording.at_elapsed_us(u64::MAX).connected);
}
#[test]
fn truncated_misordered_and_unknown_capture_formats_fail_explicitly() {
    for text in [
        String::new(),
        line(0, 10, true, 0) + "{",
        line(1, 10, true, 0),
        line(0, 10, true, 0) + &line(1, 9, true, 0),
        line(0, 10, true, 0).replace("\"schema\":1", "\"schema\":2"),
        line(0, 10, true, 0).replace("\"packet\":5", "\"packet\":null"),
    ] {
        assert!(Recording::read(text.as_bytes()).is_err(), "accepted {text}");
    }
}
