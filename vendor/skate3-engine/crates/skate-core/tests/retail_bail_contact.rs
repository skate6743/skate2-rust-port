//! Compare native82BD60C8 output, not a second implementation's expectations.
use skate_core::physics::skeleton_body::{
    CollisionResponseSettings, ContactRegion, collision_response,
};
#[test]
#[ignore = "requires SKATE_RETAIL_BAIL_CONTACT_VECTORS"]
fn response_matches_native_instruction_execution() {
    let text = std::fs::read_to_string(
        std::env::var_os("SKATE_RETAIL_BAIL_CONTACT_VECTORS").expect("native vectors"),
    )
    .unwrap();
    assert!(text.starts_with("# TU3 bail_contact 82BD60C8;"));
    let mut count = 0;
    for (case, line) in text.lines().filter(|l| !l.starts_with('#')).enumerate() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let f = |i: usize| f32::from_bits(u32::from_str_radix(w[i], 16).unwrap());
        assert_eq!(w.len(), 104);
        let t = CollisionResponseSettings {
            force_scale: f(0),
            velocity_scale: f(1),
            divisor: f(2),
            region_scale: f(3),
        };
        let mut a = [f(4), f(5), 0.0];
        let mut regions = [ContactRegion::default(); 8];
        let mut velocities = [[0.0; 4]; 24];
        let mut weights = [0.0; 24];
        for i in 0..8 {
            let j = 6 + i * 11;
            let part: i32 = w[j].parse().unwrap();
            regions[i].part = (part >= 0).then_some((i + 1) as usize);
            regions[i].force = f(j + 1);
            weights[i + 1] = f(j + 2);
            for k in 0..4 {
                velocities[i + 1][k] = f(j + 3 + k * 2);
                regions[i].normal[k] = f(j + 4 + k * 2);
            }
        }
        let out = collision_response(t, &mut a, &regions, &velocities, &weights);
        assert_eq!(a[0].to_bits(), f(94).to_bits(), "velocity response {case}");
        assert_eq!(a[1].to_bits(), f(95).to_bits(), "force response {case}");
        for i in 0..8 {
            assert_eq!(
                out[i].to_bits(),
                f(96 + i).to_bits(),
                "region {i} case {case}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 4096);
}
