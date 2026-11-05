// Lightning only from Skate 3 Remaster build10 /038ff331. See LICENSE-Skate3-engine.txt.
fn hash(cell: vec2<f32>) -> f32 {
    let q = bitcast<vec2<u32>>(vec2<i32>(cell));
    var h = (q.x * 1597334677u) ^ (q.y * 3812015801u);
    h = h * 747796405u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    return f32((h >> 22u) ^ h) / 4294967295.0;
}
fn jag(x: f32, seed: f32) -> f32 {
    let i = floor(x);
    return mix(hash(vec2<f32>(i, seed)), hash(vec2<f32>(i + 1.0, seed)), fract(x)) - 0.5;
}
fn lightning(direction: vec3<f32>, azimuth: f32, seed: f32, flash: f32) -> vec3<f32> {
    let elevation = asin(clamp(direction.y, -1.0, 1.0));
    let toward = normalize(vec3<f32>(cos(azimuth), 0.4, sin(azimuth)));
    let glow = pow(saturate(dot(direction, toward)), 8.0) * 0.3 + 0.03;
    let wander = jag(elevation * 14.0, seed) * 0.08 + jag(elevation * 45.0, seed + 7.0) * 0.025;
    let delta = atan2(direction.z, direction.x) - azimuth - wander;
    let across = abs(atan2(sin(delta), cos(delta)));
    let along = step(0.0, elevation) * (1.0 - smoothstep(0.35, 0.45, elevation));
    let bolt = (exp(-across / 0.003) * 4.0 + exp(-across / 0.02) * 0.6) * along;
    // Two narrow offshoots use independent seeds and taper back into the
    // main stroke. A restrained per-strike colour changes the cloud glow too.
    let variant=hash(vec2<f32>(floor(seed),29.0));
    let fork=max(0.0,0.24-elevation);
    let bend=(0.13+0.12*variant)*fork;
    let left=abs(atan2(sin(delta-bend),cos(delta-bend)));
    let right=abs(atan2(sin(delta+bend*1.3),cos(delta+bend*1.3)));
    let fork_fade=smoothstep(0.02,0.10,elevation)*(1.0-smoothstep(0.22,0.29,elevation));
    let branches=(exp(-left/0.0020)+0.65*exp(-right/0.0025))*fork_fade*1.5;
    let tint=mix(vec3<f32>(0.80,0.85,1.0),vec3<f32>(0.93,0.82,1.0),variant*0.60);
    return tint*(glow+bolt+branches)*flash;
}
