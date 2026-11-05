// Final shipped basictex_fisheye_PS8212cda8 / noise_PS8212cff0.
// This pass consumes gamma-encoded scene colour, as does the native blit.
struct Parameters {
    warp: vec4<f32>,
    vignette: vec4<f32>,
    scroll: vec4<f32>,
    fade: vec4<f32>,
    dot1: vec4<f32>,
    dot2: vec4<f32>,
    uv_transform: vec4<f32>,
    display_adjustment: vec4<f32>,
}
@group(0) @binding(0) var<uniform> parameters: Parameters;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var noise: texture_2d<f32>;
@group(0) @binding(3) var vignette: texture_2d<f32>;
@group(0) @binding(4) var clamp_sampler: sampler;
@group(0) @binding(5) var repeat_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex fn vs(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var output: VertexOutput;
    output.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    output.uv = uv * parameters.uv_transform.xy + parameters.uv_transform.zw;
    return output;
}

fn warped_scene(uv: vec2<f32>) -> vec4<f32> {
    let centered = uv - vec2<f32>(0.5);
    // Image warp intentionally uses the unscaled UV radius; only the
    // vignette lookup below uses the authored 16:9 aspect correction.
    let square = centered * centered;
    let radius2 = square.x + square.y;
    let weighted = radius2 * parameters.warp.x;
    let factor = fma(weighted, radius2, parameters.warp.y);
    let sample_uv = fma(centered, vec2<f32>(factor), vec2<f32>(0.5));
    let colour = textureSample(scene, clamp_sampler, sample_uv);
    // PC brightness lift for world/sky only. The original warp,
    // vignette and noise constants stay separate; HUD draws after this pass.
    var rgb = colour.rgb * parameters.display_adjustment.x;
    // PC weather: rain greys the image (z, zero otherwise).
    let rain = clamp(parameters.display_adjustment.z, 0.0, 1.0);
    if rain > 0.0 {
        let luminance = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        rgb = mix(rgb, vec3<f32>(luminance), rain * 0.5);
        rgb *= 1.0 - rain * 0.1;
    }
    return vec4<f32>(rgb, colour.a);
}
// PC Resolution Scale above 100%: four warped samples a quarter of a target pixel from its
// centre (`step`), a box over the larger scene: at 200% exactly the target pixel's 2x2 texels.
fn scene_colour(uv: vec2<f32>, step: vec2<f32>) -> vec4<f32> {
    if parameters.display_adjustment.y > 0.5 {
        return 0.25 * (warped_scene(uv - step) + warped_scene(uv + vec2<f32>(step.x, -step.y))
            + warped_scene(uv + vec2<f32>(-step.x, step.y)) + warped_scene(uv + step));
    }
    return warped_scene(uv);
}
fn quarter_pixel(uv: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(dpdx(uv.x), dpdy(uv.y)) * 0.25;
}
fn vignette_colour(uv: vec2<f32>, x: f32) -> vec4<f32> {
    let y = uv.y - 0.5;
    let radius = sqrt(abs(x * x + y * y));
    let u = fma(radius, -parameters.warp.z, parameters.vignette.x + 1.0);
    let colour = textureSample(vignette, clamp_sampler, vec2<f32>(u, parameters.vignette.y));
    return clamp(colour + vec4<f32>(parameters.vignette.w), vec4<f32>(0.0), vec4<f32>(1.0));
}
@fragment fn fs_standard(input: VertexOutput) -> @location(0) vec4<f32> {
    let x = fma(input.uv.x, 1.7777777910232544, -0.8888888955116272);
    return scene_colour(input.uv, quarter_pixel(input.uv)) * vignette_colour(input.uv, x);
}
@fragment fn fs_noise(input: VertexOutput) -> @location(0) vec4<f32> {
    let first_uv = input.uv * vec2<f32>(10.0, 5.625) + parameters.scroll.xy;
    let second_uv = fma(input.uv, vec2<f32>(11.0, 6.625), parameters.scroll.zw);
    let first = textureSample(noise, repeat_sampler, first_uv);
    let second = textureSample(noise, repeat_sampler, second_uv);
    // Preserve native dp4 operand permutation / addition ordering.
    let a = dot(first.yxzw, parameters.dot1.yxzw);
    let b = dot(second.yxwz, parameters.dot2.yxwz);
    let b_again = dot(second.yxzw, parameters.dot2.yxzw);
    let snow = clamp(((b + a) - 2.0) + b_again + a, 0.0, 1.0);
    let base = scene_colour(input.uv, quarter_pixel(input.uv)) * parameters.fade.x;
    // Noise slots4/11 use separate MUL/ADD; standard slot3 uses MAD.
    let x = input.uv.x * 1.7777777910232544 - 0.8888888955116272;
    return fma(base, vignette_colour(input.uv, x), vec4<f32>(snow * parameters.fade.y));
}
