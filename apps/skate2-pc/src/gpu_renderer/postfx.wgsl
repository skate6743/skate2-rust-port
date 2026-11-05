// Pixel programs8212b250/b748/f2e0/df38,82130478. The base-bloom copy from
// 82130038 remains, but its six lens reflections are intentionally omitted
// as a PC presentation choice (2026-10-12).
// Each render target has its own constant buffer; no late-write aliasing.
struct Parameters { v: array<vec4<f32>, 33>, }
@group(0) @binding(0) var<uniform> p: Parameters;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var low: texture_2d<f32>;
@group(0) @binding(3) var cube: texture_3d<f32>;
@group(0) @binding(4) var linear_clamp: sampler;
@group(0) @binding(5) var depth: texture_depth_2d;
@group(0) @binding(6) var overlay: texture_2d<f32>;
@group(0) @binding(7) var stencil: texture_2d<u32>;
@group(0) @binding(8) var left_eye: texture_2d<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, }
@vertex fn vs(@builtin(vertex_index) i: u32) -> Vertex {
    let x = f32((i << 1u) & 2u); let y = f32(i & 2u);
    return Vertex(vec4(x*2.-1.,1.-y*2.,0.,1.),vec2(x,y));
}
fn sample_at(uv: vec2<f32>) -> vec4<f32> { return textureSampleLevel(source,linear_clamp,uv,0.); }
@fragment fn inspect(v: Vertex) -> @location(0) vec4<f32> { return vec4(sample_at(v.uv).rgb,1.); }
fn matrix(value: vec4<f32>) -> vec3<f32> {
    return vec3(dot(p.v[10],value),dot(p.v[11],value),dot(p.v[12],value));
}
@fragment fn extract(v: Vertex) -> @location(0) vec4<f32> {
    // RenderQuad's explicit UV offset plus the Xenos-to-WGPU pixel-center
    // conversion. Downsample/Blur9 already compensate their native centers.
    let uv=v.uv+p.v[24].zw;
    let d=p.v[24].xy;
    let a=sample_at(uv+vec2(-d.x,-d.y)).rgb;
    let b=sample_at(uv+vec2(d.x,-d.y)).rgb;
    let c=sample_at(uv+vec2(-d.x,d.y)).rgb;
    let e=sample_at(uv+d).rgb;
    let avg=(e*e+c*c+b*b+a*a)*0.25;
    let alpha=clamp(dot(clamp(avg,vec3(0.),vec3(1.)),vec3(0.3,0.4,0.3)),0.,1.);
    let colour=clamp((avg-p.v[9].w)*p.v[9].z,vec3(0.),vec3(1.))+p.v[9].x;
    return vec4(clamp(matrix(vec4(colour,1.)),vec3(0.),vec3(1.)),alpha);
}
@fragment fn downsample(v: Vertex) -> @location(0) vec4<f32> {
    let d=p.v[24].xy;
    return clamp((sample_at(v.uv-d)+sample_at(v.uv+vec2(d.x,-d.y))+
        sample_at(v.uv+vec2(-d.x,d.y))+sample_at(v.uv+d))*0.25,vec4(0.),vec4(1.));
}
fn blur(uv:vec2<f32>,direction:vec2<f32>) -> vec4<f32> {
    var value=sample_at(uv+direction*p.v[7].x)*p.v[7].z;
    for(var i=6;i>=0;i=i-1) { value=sample_at(uv+direction*p.v[u32(i)].x)*p.v[u32(i)].z+value; }
    return sample_at(uv+direction*p.v[8].x)*p.v[8].z+value;
}
@fragment fn blur9(v:Vertex) -> @location(0) vec4<f32> {
    let direction=((v.uv-0.5)*5.-p.v[14].xy)*p.v[14].w+p.v[14].xy;
    let uv=v.uv+direction*(-0.75)*p.v[4].x*p.v[14].w;
    let result=blur(uv,direction);
    return vec4(result.rgb*((1.+p.v[14].w)*p.v[13].x),result.a);
}
@fragment fn combined(v:Vertex) -> @location(0) vec4<f32> {
    // Shipped combined3 shader samples only tf0 and tf1. The third level
    // remains the exposure surface; inventing a tf2 contribution is incorrect.
    let radial=p.v[14].w;
    let quad_uv=v.uv+p.v[24].zw;
    let direction=vec2((quad_uv.x-0.5)*5.*radial,1.-(quad_uv.y-0.5)*5.*radial);
    let uv=quad_uv+direction*(-0.75)*p.v[4].x*radial;
    var a=sample_at(uv+direction*p.v[8].x).rgb*p.v[8].z;
    for(var i=7;i>=0;i=i-1) { a=sample_at(uv+direction*p.v[u32(i)].x).rgb*p.v[u32(i)].z+a; }
    let b=textureSampleLevel(low,linear_clamp,quad_uv,0.).rgb;
    let edge=max(abs(quad_uv*2.-1.)-0.6,vec2(0.));
    let alpha=1.-clamp(length(edge)*5.-1.,0.,1.);
    return vec4(clamp(a*((1.+radial)*p.v[13].x)+b*p.v[13].y,vec3(0.),vec3(1.)),alpha);
}
@fragment fn bloom_copy(v:Vertex) -> @location(0) vec4<f32> {
    // Preserve the untransformed base sample from native 82130038 slots8/33,
    // including its sampling phase and UNORM intermediate. Removing only the
    // reflection contribution keeps the normal sign/light bloom path intact.
    return sample_at(v.uv+p.v[24].zw);
}
@fragment fn grade(v:Vertex) -> @location(0) vec4<f32> {
    let bloom=textureSampleLevel(low,linear_clamp,v.uv,0.).rgb;
    let value=matrix(vec4(sample_at(v.uv).rgb*p.v[15].x+bloom,1.));
    let uv=vec3(value.g*0.96875+0.015625,-value.r*0.96875+0.984375,value.b*0.96875+0.015625);
    return textureSampleLevel(cube,linear_clamp,uv,0.);
}
// ICE ApplyVisualFX82847610 (visualFX_PS8212e570) writes the near/far colour
// transform into an A8R8G8B8 intermediate before ApplyUber828334d8 samples
// the colour cube: the near (c7-c9) and far (c10-c12) matrices weighted by
// linear depth z = c2.x/(d - c3.x) as saturate(z*c5.xy + c6.xy). Its
// g_StencilLerpMod is characterEffectMaskValue, 0 in every shipped take, so
// the characters take the grade too. The blur amount it leaves in alpha
// feeds the unported DOF.
@fragment fn visual(v:Vertex) -> @location(0) vec4<f32> {
    let bloom=textureSampleLevel(low,linear_clamp,v.uv,0.).rgb;
    let colour=vec4(sample_at(v.uv).rgb*1.41412354+bloom,1.);
    let d=textureLoad(depth,vec2<i32>(v.position.xy),0);
    let z=p.v[31].x/(d-p.v[31].y);
    let w=clamp(z*p.v[30].xy+p.v[30].zw,vec2(0.),vec2(1.));
    let far=vec3(dot(p.v[27],colour),dot(p.v[28],colour),dot(p.v[29],colour));
    let graded=matrix(colour)*w.x+far*w.y;
    // visualFX_PS slot 14/19/27/28: stencil byte, saturate(2*s),
    // g_StencilLerpMod (1 during HoM, ICE's character mask otherwise).
    let s=f32(textureLoad(stencil,vec2<i32>(v.position.xy),0).x)/255.;
    let mask=clamp(2.*s,0.,1.)*p.v[32].y*(1.-p.v[32].z);
    return vec4(graded+(colour.rgb-graded)*mask,0.);
}
// uberPostFX_PS8212dc28 reads the screen at (u, frac(v + g_ScanlineOffset.x))
// (c0), the roll ICE's scanline channel drives, and ends with
// oC0 = r0 * g_FinalMult (c2).
@fragment fn cinematic_grade(v:Vertex) -> @location(0) vec4<f32> {
    let screen_uv=vec2(v.uv.x,fract(v.uv.y+p.v[26].x));
    let value=sample_at(screen_uv);
    let uv=vec3(value.g*0.96875+0.015625,-value.r*0.96875+0.984375,value.b*0.96875+0.015625);
    var graded=textureSampleLevel(cube,linear_clamp,uv,0.);
    // uberPostFX_PS 8212dc28 slots 11..18: xray.Texture modulation,
    // then the stencil byte restores the pre-cube character colour.
    let xray=textureSampleLevel(overlay,linear_clamp,screen_uv,0.);
    let size=textureDimensions(stencil);
    let pixel=clamp(vec2<i32>(screen_uv*vec2<f32>(size)),vec2(0),vec2<i32>(size)-1);
    let s=f32(textureLoad(stencil,pixel,0).x)/255.;
    graded*=vec4(1.)+p.v[32].x*(2.*xray-vec4(1.));
    graded+=(value-graded)*(s*p.v[32].x)*(1.-p.v[32].z);
    return graded*p.v[25];
}

// Original stereo_uberPostFX_PS8212e1a8: right scene *1.41 + bloom,
// near matrix, cube (G,-R,B); replace red with main2's left VisualFX red.
@fragment fn stereo_grade(v:Vertex) -> @location(0) vec4<f32> {
    let bloom=textureSampleLevel(low,linear_clamp,v.uv,0.).rgb;
    let value=matrix(vec4(sample_at(v.uv).rgb*1.41+bloom,1.));
    let uv=vec3(value.g*0.96875+0.015625,-value.r*0.96875+0.984375,value.b*0.96875+0.015625);
    let right=textureSampleLevel(cube,linear_clamp,uv,0.);
    let left=textureSampleLevel(left_eye,linear_clamp,v.uv,0.);
    return vec4(left.r,right.gba);
}
