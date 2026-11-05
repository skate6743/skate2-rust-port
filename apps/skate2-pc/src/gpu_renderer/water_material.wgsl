// water / flowingwater / ocean final programs. Binding declarations are
// generated per effect; helpers are shared without inventing missing channels.
struct WaterIn {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) fog: vec4<f32>,
    @location(3) @interpolate(flat) lightmap_channel: vec4<f32>,
    @location(4) @interpolate(flat) params0: vec4<f32>,
    @location(5) @interpolate(flat) params1: vec4<f32>,
    @location(6) @interpolate(flat) params2: vec4<f32>,
    @location(7) @interpolate(flat) params3: vec4<f32>,
    @location(8) normal: vec3<f32>,
    @location(9) tangent: vec3<f32>,
    @location(10) binormal: vec3<f32>,
}
@vertex fn waterVS(
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(7) lightmap: vec4<f32>, @location(8) params1: vec4<f32>,
    @location(9) params2: vec4<f32>, @location(10) params3: vec4<f32>,
    @location(11) params0: vec4<f32>, @location(12) lm: vec4<f32>,
    @location(13) tangent: vec3<f32>
) -> WaterIn {
    var output: WaterIn;
    output.position=world_clip(position);
    output.world=position;
    output.uv=vec4<f32>(uv,select(abs(lm.xy),lm.xy,IS_OCEAN));
    output.fog=compute_fog_factor(position);
    output.lightmap_channel=lightmap;
    output.params0=params0; output.params1=params1;
    output.params2=params2; output.params3=params3;
    output.normal=vec3<f32>(lm.zw,sign(lm.y)*sqrt(1.-min(0.999,dot(lm.zw,lm.zw))));
    output.tangent=tangent;
    output.binormal=sign(lm.x)*cross(output.normal,tangent);
    return output;
}
fn water_reflection(normal: vec3<f32>, view: vec3<f32>) -> vec3<f32> {
    let direction=reflect(view,normal)*vec3<f32>(-1.,-1.,1.);
    return textureSample(i_environment,cube_sampler,direction).rgb;
}
// PC port (user request): in a linked Skate 3 district, under Skate 2's PostFX, the sky the water
// mirrors and the sun's glint on it keep Skate 3's on-screen brightness, as the sky's sun disc
// does (animation_time.z; 0 in Skate 2's worlds).
fn district_mirror_scale() -> f32 {
    return select(1.,globals.animation_time.z,globals.animation_time.z>0.);
}

// __PCA_BEGIN__
// Final FPO remaps the reconstructed RGB to [-1,1] BEFORE toning down.
// The older UPDB GetPCANormal text omits this second remap.
fn pca_normal(uv: vec2<f32>, amount: f32) -> vec3<f32> {
    let a=textureSample(i_normal,wrap_sampler,uv)*2.-vec4<f32>(1.);
    let b=textureSample(i_normal2,wrap_sampler,uv)*2.-vec4<f32>(1.);
    let reconstructed=globals.pca[0].xyz+vec3<f32>(
        dot(a,globals.pca[1])+dot(b,globals.pca[2]),
        dot(a,globals.pca[3])+dot(b,globals.pca[4]),
        dot(a,globals.pca[5])+dot(b,globals.pca[6]));
    return normalize(mix(vec3<f32>(0.,1.,0.),reconstructed*2.-vec3<f32>(1.),amount));
}
// __PCA_END__

// __SURFACE_BEGIN__
fn water_lit(input: WaterIn, normal: vec3<f32>, lm_offset: vec2<f32>, diffuse_uv: vec2<f32>) -> vec4<f32> {
    let diffuse=textureSample(i_diffuse,wrap_sampler,diffuse_uv).rgb;
    let masks=clamp(textureSample(i_specular,wrap_sampler,diffuse_uv).rgb-vec3<f32>(input.params3.y),vec3<f32>(0.),vec3<f32>(1.));
    let view=normalize(globals.view_position.xyz-input.world);
    let light=globals.light_direction.xyz;
    let csm=csm_shadow_map(vec4<f32>(input.world,1.),vec3<f32>(-0.005,10.,25.));
    let lm_uv=input.uv.zw+0.01*lm_offset;
    let lm=shadowed_lightmap(textureSample(i_lightmap,clamp_sampler,lm_uv),
        textureSample(i_chromaticity,clamp_sampler,lm_uv),input.lightmap_channel,
        vec4<f32>(4.5,4.5,4.5,1.5),vec4<f32>(0.07,0.09,0.1,0.04),50.,select(1.,csm,dot(normal,light)>0.));
    let reflection_luminosity=0.3*clamp(4.*(masks.b-1.5),0.,1.);
    let reflected=masks.b*water_reflection(normal,view)*mix(lm.a,1.,reflection_luminosity)*1.5*district_mirror_scale();
    let calculated=masks.r*phong_specular(normal,view,light,input.params3.z)
        *vec3<f32>(18.,15.,10.5)*clamp(lm.a-0.1,0.,1.)*district_mirror_scale();
    let kd=tangent_light(normal,vec3<f32>(0.64,0.64,0.420465),light);
    let tone=fog_bloom_tone(vec4<f32>(lm.rgb*kd*diffuse*diffuse+calculated+reflected,1.)*input.params0.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,1.);
}
// __SURFACE_END__

// __WATER_BEGIN__
@fragment fn waterPS(input: WaterIn) -> @location(0) vec4<f32> {
    let uv=input.uv.xyxy*input.params3.x*input.params2+input.params1*globals.animation_time.x;
    let normal=pca_normal(uv.zw,input.params0.z);
    let offset=pca_normal(uv.xy,input.params0.w);
    return water_lit(input,normal,normal.xz,input.uv.xy+0.02*offset.xz);
}
// __WATER_END__

// __FLOW_BEGIN__
@fragment fn flowingPS(input: WaterIn) -> @location(0) vec4<f32> {
    let uv=input.uv.xyxy*input.params2+input.params1*globals.animation_time.x;
    let first=textureSample(i_normal,wrap_sampler,uv.xy).rgb*2.-vec3<f32>(1.);
    let second=textureSample(i_normal,wrap_sampler,uv.zw).rgb*2.-vec3<f32>(1.);
    let normal=normalize(input.params0.xzw*(first+second));
    let world_normal=normalize(normal.x*input.tangent+normal.y*input.binormal+normal.z*input.normal);
    return water_lit(input,world_normal,normal.xz,input.uv.xy);
}
// __FLOW_END__

// __OCEAN_BEGIN__
@fragment fn oceanPS(input: WaterIn) -> @location(0) vec4<f32> {
    let overlay=clamp((textureSample(i_macrooverlay,wrap_sampler,input.uv.xy*input.params3.x).r-0.5)*2.+0.5,0.,1.);
    let pca=pca_normal(input.uv.xy*input.params1.w,input.params1.z*2.*overlay);
    let normal=normalize(max(abs(pca),vec3<f32>(0.001)));
    let view=normalize(globals.view_position.xyz-input.world);
    let lm_uv=input.uv.zw+0.01*normal.xz;
    let gamma=dot(textureSample(i_lightmap,clamp_sampler,lm_uv),input.lightmap_channel);
    let lm=gamma*gamma;
    let colour=lm*textureSample(i_chromaticity,clamp_sampler,lm_uv).rgb*4.5;
    let reflected=water_reflection(normal,view)*colour;
    // Final pc75..83 uses the absolute base (including rounding past one).
    let fresnel=0.25+0.75*pow(abs(1.-abs(dot(normal,view))),5.);
    let u=normalize(vec3<f32>(-normal.z,0.,normal.x));
    let v=cross(normal,u);
    let spread=input.params2.z/input.params2.xy;
    let half_vector=normalize(globals.light_direction.xyz+view);
    let exponents=vec2<f32>(dot(half_vector,u),dot(half_vector,v))/spread;
    let ward=exp(-2.*dot(exponents,exponents)/(1.+dot(half_vector,normal)))
        /(sqrt(abs(dot(normal,globals.light_direction.xyz)*dot(normal,view)))*12.566371*spread.x*spread.y);
    let tone=fog_bloom_tone(vec4<f32>((reflected*fresnel+ward*lm*input.params0.rgb)*district_mirror_scale(),1.)*input.params1.y,input.fog,globals.envattributes2.x);
    return vec4<f32>(tone.rgb,0.);
}
// __OCEAN_END__
