// Shipped transparentdefaultenvironment_defaultPS: diffuse alpha is restored
// after fog/tone. Manifest uses SRCALPHA/INVSRCALPHA, alpha test GE1, depth write.
@fragment fn transparentDefaultPS(input: PsIn) -> @location(0) vec4<f32> {
    let colour = default_lit(input);
    if colour.a < 1.0/255.0 { discard; }
    let tone = fog_bloom_tone(colour*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,colour.a);
}

// advertisement has no shipped UPDB. Final VPO fetches only POSITION, UV0,
// packed LM/normal UV1; FPO uses that geometric normal, not the material's
// unused i_normal channel. Final c5.z = g_ViewDotLight.z backlit minimum.
@vertex fn advertisementVS(
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>, @location(11) params: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position=world_clip(position);
    output.world=position;
    output.uv=vec4<f32>(uv,abs(lm.xy));
    let sign_z=clamp(65535.0*lm.y,0.0,1.0)*2.0-1.0;
    output.geometric_normal=vec3<f32>(lm.zw,sign_z*sqrt(1.0-min(0.999,dot(lm.zw,lm.zw))));
    output.lightmap_channel=lightmap_dot;
    output.material=params;
    return output;
}
@fragment fn advertisementPS(input: PsIn) -> @location(0) vec4<f32> {
    let diffuse=textureSample(i_diffuse,wrap_sampler,input.uv.xy).rgb;
    let csm=csm_shadow_map(vec4<f32>(input.world,1.),vec3<f32>(-0.005,10.,25.));
    let lighting=shadowed_lightmap(
        textureSample(i_lightmap,clamp_sampler,input.uv.zw),
        textureSample(i_chromaticity,clamp_sampler,input.uv.zw),input.lightmap_channel,
        vec4<f32>(4.5,4.5,4.5,1.5),vec4<f32>(0.07,0.09,0.1,0.04),50.,
        select(1.,csm,dot(input.geometric_normal,globals.light_direction.xyz)>0.)).rgb;
    let colour=diffuse*diffuse*max(lighting,vec3<f32>(globals.view_dot_light.z))*input.material.y;
    // Final pc31..37: no fog addition and no high-range linear tone branch.
    let inverse=clamp(vec3<f32>(1.)-colour*globals.envattributes2.x,vec3<f32>(0.),vec3<f32>(1.));
    return vec4<f32>(sqrt((vec3<f32>(1.)-inverse*inverse)*0.5),0.);
}

// tree_defaultVS.vpo: world-space position; abs(LM.xy), no instance transform.
@vertex fn treeVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(11) m_params: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position=world_clip(position);
    output.world=position;
    output.uv=vec4<f32>(uv,abs(lm.xy));
    output.fog=compute_fog_factor(position);
    output.lightmap_channel=lightmap_dot;
    output.material=m_params;
    return output;
}

// treeanimate_defaultVS: TEXCOORD1.zw carry the authored wind amplitudes.
// The shipped program range-reduces before SIN (c254/c255), unlike generic sin.
@vertex fn treeAnimateVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) params1: vec4<f32>,
    @location(9) params2: vec4<f32>,
    @location(11) params0: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    let scale=vec3<f32>(lm.zw,lm.z+lm.w)-vec3<f32>(0.5,0.5,1.0);
    let angle=(globals.animation_time.xxx-scale)*params1.xyz+params2.xyz;
    let reduced=fract(angle*0.159154937+vec3<f32>(0.5))*6.28318548-vec3<f32>(3.14159274);
    // PC weather: the storm's gust (animation_time.y, zero otherwise) sways the authored foliage more.
    let moved=position+sin(reduced)*(scale*vec3<f32>(params0.w,params1.w,params2.w))*(1.0+globals.animation_time.y);
    var output: PsIn;
    output.position=world_clip(moved);
    output.world=moved;
    output.uv=vec4<f32>(uv,abs(lm.xy));
    output.fog=compute_fog_factor(moved);
    output.lightmap_channel=lightmap_dot;
    // treeanimate's final PS does not multiply the exposure by m_params[0].y.
    output.material=vec4<f32>(0.,1.,0.,0.);
    return output;
}

// tree_defaultPS.fpo pc6..14: square selected LM, multiply chromaticity*4.5,
// MAX with g_ViewDotLight.y, then multiply .x and linear diffuse, m_params[0].y.
fn foliage_colour(input: PsIn, minimum: f32) -> vec4<f32> {
    let diffuse=textureSample(i_diffuse,wrap_sampler,input.uv.xy);
    if diffuse.a < 30.0/255.0 { discard; }
    let lm=dot(textureSample(i_lightmap,clamp_sampler,input.uv.zw),input.lightmap_channel);
    let chroma=textureSample(i_chromaticity,clamp_sampler,input.uv.zw).rgb;
    let lighting=max(lm*lm*chroma*4.5,vec3<f32>(minimum))*globals.view_dot_light.x;
    let colour=vec4<f32>(lighting*diffuse.rgb*diffuse.rgb,diffuse.a);
    let tone=fog_bloom_tone(colour*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,diffuse.a);
}
@fragment fn treePS(input: PsIn) -> @location(0) vec4<f32> {
    return foliage_colour(input,globals.view_dot_light.y);
}
// vertexanimate final PS omits the tree minimum-light clamp.
@fragment fn vertexAnimatePS(input: PsIn) -> @location(0) vec4<f32> {
    return foliage_colour(input,0.0);
}

@vertex fn scrollIncandescentVS(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(7) lightmap_dot: vec4<f32>,
    @location(8) u_speed: vec4<f32>,
    @location(9) v_speed: vec4<f32>,
    @location(11) params0: vec4<f32>,
    @location(12) lm: vec4<f32>
) -> PsIn {
    var output: PsIn;
    output.position=world_clip(position);
    output.world=position;
    output.uv=vec4<f32>(uv+globals.animation_time.xx*vec2<f32>(u_speed.x,v_speed.x),lm.xy);
    output.fog=compute_fog_factor(position);
    output.lightmap_channel=lightmap_dot;
    output.material=params0;
    return output;
}

// Incandescent family final FPO: four half-texel lightmap fetches, sum in
// (-,-),(+,-),(-,+),(+,+) order, select/average THEN square. Chromaticity is
// single-tap. WGSL offsets are integer-only; fractional offsets here are at
// level zero. See evidence for the remaining Xenos mip-relative offset gap.
fn incandescent_colour(input: PsIn) -> vec4<f32> {
    let half_texel=vec2<f32>(0.5)/vec2<f32>(textureDimensions(i_lightmap));
    let uv=input.uv.zw;
    let lm0=textureSample(i_lightmap,clamp_sampler,uv-half_texel);
    let lm1=textureSample(i_lightmap,clamp_sampler,uv+half_texel*vec2<f32>(1.,-1.));
    let lm2=textureSample(i_lightmap,clamp_sampler,uv+half_texel*vec2<f32>(-1.,1.));
    let lm3=textureSample(i_lightmap,clamp_sampler,uv+half_texel);
    let lm=dot(((lm0+lm1)+lm2)+lm3,input.lightmap_channel)*0.25;
    let chroma=textureSample(i_chromaticity,clamp_sampler,uv).rgb;
    let lighting=vec4<f32>(lm*lm*chroma*4.5,lm*lm);
    let diffuse=textureSample(i_diffuse,wrap_sampler,input.uv.xy);
    return vec4<f32>(diffuse.rgb*diffuse.rgb,diffuse.a)*((lighting-vec4<f32>(1.))*input.material.x+vec4<f32>(1.));
}
@fragment fn incandescentPS(input: PsIn) -> @location(0) vec4<f32> {
    let colour=incandescent_colour(input);
    let tone=fog_bloom_tone(colour*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,0.);
}
@fragment fn incandescentAlphaPS(input: PsIn) -> @location(0) vec4<f32> {
    let colour=incandescent_colour(input);
    let tone=fog_bloom_tone(colour*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,colour.a);
}
@fragment fn incandescentBacklitPS(input: PsIn) -> @location(0) vec4<f32> {
    let colour=incandescent_colour(input);
    if colour.a < 16.0/255.0 { discard; }
    let tone=fog_bloom_tone(colour*input.material.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,colour.a);
}
