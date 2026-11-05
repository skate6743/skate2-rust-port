// Underground standing water; other materials retain their dry response.
// One depth-projected screen overlay. Geometry is queried only for admission and shelter.
struct Weather {
    clip:mat4x4<f32>, camera:vec4<f32>, light:vec4<f32>,
    state:vec4<f32>, look:vec4<f32>, fog_colour:vec4<f32>, fog_k1:vec4<f32>,
    volume:vec4<f32>,motion:vec4<f32>,colour:vec4<f32>,lightning:vec4<f32>,
}
@group(0) @binding(0) var<uniform> weather:Weather;
@group(0) @binding(1) var scene_depth:WEATHER_DEPTH_TYPE;
@group(0) @binding(2) var scene_colour:texture_2d<f32>;
@group(0) @binding(3) var native_stencil:WEATHER_STENCIL_TYPE;
@group(0) @binding(4) var cloud_photos:texture_2d_array<f32>;
@group(0) @binding(5) var cloud_sampler:sampler;
struct WaterCell { terrain:vec4<f32>,levels:vec4<f32> }
struct WaterDomain { grid:vec4<f32>, state:vec4<f32> }
@group(0) @binding(6) var<storage,read> water_cells:array<WaterCell>;
@group(0) @binding(7) var<uniform> water_domain:WaterDomain;
struct Wake {position:vec4<f32>,shape:vec4<f32>}
@group(0) @binding(8) var<storage,read> wakes:array<Wake>;
@group(1) @binding(0) var environment:texture_cube<f32>;
@group(1) @binding(1) var reflection_sampler:sampler;
@group(1) @binding(2) var<storage,read> distributions:array<f32>;

struct Node { minimum:vec3<f32>, first:u32, maximum:vec3<f32>, escape:u32 }
struct Receiver { a:vec3<f32>, flags:u32, e1:vec3<f32>, curve:u32, e2:vec3<f32>, pad:u32, normal:vec4<f32> }
@group(1) @binding(3) var<storage,read> nodes:array<Node>;
@group(1) @binding(4) var<storage,read> receivers:array<Receiver>;
struct Vertex { @builtin(position) clip:vec4<f32> }
@vertex fn vs(@builtin(vertex_index) index:u32)->Vertex {
    let xy=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));
    return Vertex(vec4<f32>(xy[index],0.,1.));
}
// Solve the three clip equations directly. Works for perspective, orthographic,
// asymmetric stereo projection and the camera-relative 10 km world placement.
fn unproject(pixel:vec2<f32>,depth:f32)->vec3<f32> {
    let ndc=vec3<f32>(pixel/vec2<f32>(textureDimensions(scene_depth))*vec2<f32>(2.,-2.)+vec2<f32>(-1.,1.),depth);
    let m=transpose(weather.clip);
    let a=m[0]-ndc.x*m[3]; let b=m[1]-ndc.y*m[3]; let c=m[2]-ndc.z*m[3];
    let bc=cross(b.xyz,c.xyz);let ca=cross(c.xyz,a.xyz);let ab=cross(a.xyz,b.xyz);
    return (-a.w*bc-b.w*ca-c.w*ab)/dot(a.xyz,bc);
}
fn intersects_box(node:Node,origin:vec3<f32>,direction:vec3<f32>,lo:f32,hi:f32)->bool {
    var enter=lo;var leave=hi;
    for(var axis=0u;axis<3u;axis++) {
        if abs(direction[axis])<.0000001 {
            if origin[axis]<node.minimum[axis]-.002 || origin[axis]>node.maximum[axis]+.002 {return false;}
        } else {
            let a=(node.minimum[axis]-.002-origin[axis])/direction[axis];
            let b=(node.maximum[axis]+.002-origin[axis])/direction[axis];
            enter=max(enter,min(a,b));leave=min(leave,max(a,b));
            if leave<enter {return false;}
        }
    }
    return true;
}
fn query_surface(origin:vec3<f32>,direction:vec3<f32>,lo:f32,hi:f32,cover:bool)->u32 {
    var index=0u;var found=0xffffffffu;var best=hi;
    loop {
        if index>=arrayLength(&nodes) {break;}
        let node=nodes[index];
        if !intersects_box(node,origin,direction,lo,best) {index=node.escape;continue;}
        if node.first==0xffffffffu {index++;continue;}
        let first=node.first&0x0fffffffu;let count=(node.first>>28u)+1u;
        for(var i=first;i<first+count;i++) {
            let t=receivers[i];
            if cover && (t.flags&2u)==0u {continue;}
            let h=cross(direction,t.e2);let det=dot(t.e1,h);
            if abs(det)<.00000001 {continue;}
            let s=origin-t.a;let u=dot(s,h)/det;
            if u<-.00001 || u>1.00001 {continue;}
            let q=cross(s,t.e1);let v=dot(direction,q)/det;
            if v<-.00001 || u+v>1.00001 {continue;}
            let travel=dot(t.e2,q)/det;
            if travel>=lo && travel<=best {
                if cover {return i;}
                best=travel;found=i;
            }
        }
        index=node.escape;
    }
    return found;
}

fn lattice_hash(p:vec2<i32>)->f32 {
    var v=bitcast<u32>(p.x)^(bitcast<u32>(p.y)*0x9e3779b9u);
    v=(v^(v>>16u))*0x7feb352du;
    v=(v^(v>>15u))*0x846ca68bu;
    return f32(((v^(v>>16u))>>16u)&65535u)/65535.;
}
// Analytic world-space gradient gives a stable shoreline width regardless of
// mesh tessellation, camera distance or the surface's coverage distribution.
fn value_noise_gradient(p:vec2<f32>)->vec3<f32> {
    let cell=vec2<i32>(floor(p));let f=fract(p);let s=f*f*(3.-2.*f);
    let a=lattice_hash(cell);let b=lattice_hash(cell+vec2<i32>(1,0));
    let c=lattice_hash(cell+vec2<i32>(0,1));let d=lattice_hash(cell+vec2<i32>(1,1));
    let derivative=6.*f*(1.-f);
    return vec3<f32>(mix(mix(a,b,s.x),mix(c,d,s.x),s.y),
        mix(b-a,d-c,s.y)*derivative.x,mix(c-a,d-b,s.x)*derivative.y);
}
// Rotated, warped multi-scale contours break up the rounded lattice lobes.
// Analytic gradients include the warp Jacobian: shoreline width stays in metres
// rather than turning into a dark contour when the field's slope changes.
fn puddle_rank(world:vec2<f32>,curve:u32)->vec3<f32> {
    let p=world/2.4;
    let wx=value_noise_gradient(p*.43+vec2<f32>(19.2,5.7));
    let wz=value_noise_gradient(p*.43+vec2<f32>(-8.4,31.3));
    let warped=p+(vec2<f32>(wx.x,wz.x)-.5)*1.35;
    let rotation_a=mat2x2<f32>(.80,.60,-.60,.80);
    let rotation_b=mat2x2<f32>(.36,-.933,.933,.36);
    let a=value_noise_gradient(rotation_a*warped);
    let b=value_noise_gradient(rotation_b*warped*2.17+vec2<f32>(11.7,3.4));
    let c=value_noise_gradient(warped*4.61+vec2<f32>(-6.2,9.1));
    let field=a.x*.52+b.x*.32+c.x*.16;
    let g=transpose(rotation_a)*a.yz*.52+transpose(rotation_b)*b.yz*(.32*2.17)+c.yz*(.16*4.61);
    let jacobian=mat2x2<f32>(1.+wx.y*.5805,wz.y*.5805,wx.z*.5805,1.+wz.z*.5805);
    let t=clamp((field-.20)/.60,0.,1.);
    let gradient=transpose(jacobian)*g*(6.*t*(1.-t)/(.60*2.4));
    return vec3<f32>(t*t*(3.-2.*t),gradient);
}

fn ripple_wave(distance:f32,front:f32,width:f32,frequency:f32)->vec2<f32> {
    let offset=distance-front;let q=offset/width;
    let envelope=exp(-q*q);let phase=offset*frequency;
    return vec2<f32>((cos(phase)*frequency-sin(phase)*2.*offset/(width*width))*envelope,
        cos(phase)*envelope);
}
fn ripple(p:vec2<f32>)->vec3<f32> {
    let grid=p*2.2;let cell=vec2<i32>(floor(grid));var result=vec3<f32>(0.);
    for(var x=-1;x<=1;x++){for(var y=-1;y<=1;y++){
        let neighbour=cell+vec2<i32>(x,y);
        let timeline=weather.state.w*.95+lattice_hash(neighbour+vec2<i32>(71,19));
        let age=fract(timeline)/.95;
        if age>=.95 {continue;} // Outside every possible impact lifetime.
        // A fresh deterministic seed per impact gives irregular spacing and
        // different durations, radii, thickness, strength and ring packets.
        let cycle=i32(floor(timeline));let seed=neighbour+vec2<i32>(cycle*103,cycle*47);
        let size=lattice_hash(seed+vec2<i32>(-31,83));
        let strength=lattice_hash(seed+vec2<i32>(59,-13));
        let lifetime=mix(.45,.95,size);
        let rain=clamp(weather.state.x,0.,1.);
        if age<lifetime && strength<mix(.40,.94,rain) {
            let centre=(vec2<f32>(neighbour)+.08+.84*vec2<f32>(lattice_hash(seed),lattice_hash(seed+vec2<i32>(17,53))))/2.2;
            let delta=p-centre;let distance=length(delta);
            let front=.012+age*mix(.18,.36,size);
            let width=mix(.022,.048,strength);let frequency=mix(62.,88.,1.-size);
            var wave=ripple_wave(distance,front,width,frequency);
            if size>.56 {
                wave+=ripple_wave(distance,front*.60,width*.85,frequency*1.08)*.38*smoothstep(.10,.30,age);
            }
            let fade=pow(max(0.,1.-age/lifetime),1.55)*smoothstep(0.,.055,age)*mix(.30,1.,strength);
            result+=vec3<f32>(normalize(delta+vec2<f32>(.00001))*wave.x*.0026,wave.y)*fade;
        }
    }}
    return clamp(result,vec3<f32>(-.24,-.24,-1.),vec3<f32>(.24,.24,1.));
}
// Evaluate puddles once per pixel, independent of the anti-aliasing sample count.
// Averaging planar sample depths reconstructs the pixel centre. Mixed sky/geometry
// edge pixels retain the underlying material, avoiding water over empty space.
fn puddle_depth(pixel:vec2<i32>)->f32 {
    var total=0.;
    for(var i=0u;i<WEATHER_SAMPLESu;i++) {
        let depth=textureLoad(scene_depth,pixel,i32(i));
        if depth>=.9999999 {return 1.;}
        total+=depth;
    }
    return total/f32(WEATHER_SAMPLESu);
}
// Recover the scale of the scene's camera ray at a sampled depth. This avoids
// a fixed normalized-depth tolerance, which changes with near plane and distance.
fn depth_scale(point:vec3<f32>,depth:f32)->f32 {
    let ray=weather.clip*vec4<f32>(point,0.);
    let origin=weather.clip[3];
    let denominator=depth*ray.w-ray.z;
    if abs(denominator)<.00000001 {return -1.;}
    return (origin.z-depth*origin.w)/denominator;
}

// Rain uses the static native environment cube only. No live scene/player rays.
struct Pixel {depth:f32,sky:f32,character:f32}
fn pixel_state(pixel:vec2<i32>)->Pixel {
    var z=0.;var sky=0.;var mask=0.;
    for(var i=0u;i<WEATHER_SAMPLESu;i++) {
        let sample=textureLoad(scene_depth,pixel,i32(i));z+=sample;
        sky+=select(0.,1.,sample>=.9999999);
        mask+=f32(textureLoad(native_stencil,pixel,i32(i)).x)/255.;
    }
    let count=f32(WEATHER_SAMPLESu);
    return Pixel(select(z/count,1.,sky>0.),sky/count,mask/count);
}
fn sky_coverage(pixel:vec2<i32>)->f32 {
    var sky=0.;for(var i=0u;i<WEATHER_SAMPLESu;i++){sky+=select(0.,1.,textureLoad(scene_depth,pixel,i32(i))>=.9999999);}
    return sky/f32(WEATHER_SAMPLESu);
}
fn clear_sky_neighbour(pixel:vec2<i32>,fallback:vec3<f32>)->vec3<f32> {
    let size=vec2<i32>(textureDimensions(scene_depth));
    let offsets=array<vec2<i32>,8>(vec2<i32>(0,-1),vec2<i32>(1,0),vec2<i32>(-1,0),vec2<i32>(0,1),vec2<i32>(0,-3),vec2<i32>(3,0),vec2<i32>(-3,0),vec2<i32>(0,3));
    for(var i=0u;i<8u;i++) {let at=clamp(pixel+offsets[i],vec2<i32>(0),size-1);if sky_coverage(at)>.9999 {return textureLoad(scene_colour,at,0).rgb;}}
    return fallback;
}
fn storm_tone(colour:vec3<f32>)->vec3<f32> {
    let amount=select(clamp(weather.state.x,0.,1.),.15,weather.colour.w>.5);let grey=dot(colour,vec3<f32>(.2126,.7152,.0722));
    return mix(colour,vec3<f32>(grey)*vec3<f32>(.79,.84,.92),amount*.50)*(1.-amount*.10-max(0.,amount-.30)*.19);
}
// Photographed CC0 cloud billows retain real light/shape detail. Two distinct
// panoramas and a thin drifting low layer avoid a flat repeated noise sheet.
// Azimuth drift loops exactly at the 4096-second weather clock wrap.
fn storm_sky(direction:vec3<f32>)->vec3<f32> {
    let after=weather.colour.w>.5;
    let rain=select(clamp(weather.state.x,0.,1.),.20,after);
    let storm=smoothstep(.30,1.,rain);let up=clamp(direction.y,0.,1.);
    let phase=weather.state.w*.001533980788;
    let azimuth=atan2(direction.z,direction.x)*.15915494309;
    let wind=vec2<f32>(sin(phase),cos(phase));
    // Crop excludes the original ground; horizon haze hides pole stretching.
    let elevation=acos(up)*.63661977236;
    let uv=vec2<f32>(azimuth+.46+weather.state.w*.0009765625,elevation+sin(phase*4.)*.006);
    var photo=vec3<f32>(0.);
    if storm<.001 {photo=textureSampleLevel(cloud_photos,cloud_sampler,uv,0,0.).rgb;}
    else if storm>.999 {photo=textureSampleLevel(cloud_photos,cloud_sampler,uv+vec2<f32>(.19+wind.y*.025,0.),1,0.).rgb;}
    else {
        let clearing=textureSampleLevel(cloud_photos,cloud_sampler,uv,0,0.).rgb;
        let tower=textureSampleLevel(cloud_photos,cloud_sampler,uv+vec2<f32>(.19+wind.y*.025,0.),1,0.).rgb;
        photo=mix(clearing,tower,storm);
    }
    let luminance=dot(photo,vec3<f32>(.2126,.7152,.0722));
    // Heavy has dark, neutral thunderheads, with the photograph's bright rims
    // preserved. Light has open blue breaks; Medium brings the storm bank in.
    photo=mix(photo,vec3<f32>(luminance)*vec3<f32>(.78,.86,.98),mix(.22,.88,storm));
    photo*=mix(.86,.48,storm);
    let dome=direction.xz/(up+.28)*2.1;
    let scud=value_noise_gradient(dome-wind*4.6).x;
    let wisps=smoothstep(.58,.82,scud)*storm*.26;
    photo=mix(photo,photo*.48,wisps);
    let haze=vec3<f32>(.19,.215,.245)*mix(1.22,.78,storm);
    photo=mix(haze,photo,smoothstep(0.,.24,up));
    // Distant cloud illumination uses the existing isolated lightning clock.
    let flash=weather.lightning.x*storm;
    return photo+vec3<f32>(.23,.27,.36)*flash*(.4+luminance*.6);
}
// The same solved terrain field feeds these levels and the wheel-contact test.
// Return water level, actual floor, virtual bed and domain validity. Dry points
// inside a basin retain its candidate level for ray/level intersection.
struct BasinSample {field:vec4<f32>,confidence:f32}
fn basin_sample(world:vec2<f32>)->BasinSample {
    let grid=water_domain.grid;let q=(world-grid.xy)/grid.z;
    if water_domain.state.x<.5 || any(q<vec2<f32>(0.)) || any(q>=vec2<f32>(grid.w-1.)) {return BasinSample(vec4<f32>(-1000000.,0.,0.,0.),0.);}
    let low=vec2<u32>(floor(q));let f=fract(q);let width=u32(grid.w);
    let indices=array<u32,4>(low.y*width+low.x,low.y*width+low.x+1u,(low.y+1u)*width+low.x,(low.y+1u)*width+low.x+1u);
    let weights=array<f32,4>((1.-f.x)*(1.-f.y),f.x*(1.-f.y),(1.-f.x)*f.y,f.x*f.y);
    let sf=f*f*f*(f*(f*6.-15.)+10.);let smooth_weights=array<f32,4>((1.-sf.x)*(1.-sf.y),sf.x*(1.-sf.y),(1.-sf.x)*sf.y,sf.x*sf.y);
    var ground=0.;var relief=0.;var valid=0.;var confidence=0.;var relief_weight=0.;let mode=u32(water_domain.state.y);
    for(var i=0u;i<4u;i++) {
        let cell=water_cells[indices[i]];let w=weights[i]*cell.terrain.z;
        ground+=cell.terrain.x*w;valid+=w;confidence+=cell.terrain.w*weights[i];
        let sw=smooth_weights[i]*cell.terrain.z;relief+=(cell.terrain.y-cell.terrain.x)*sw;relief_weight+=sw;
    }
    if valid>.0001 {ground/=valid;}if relief_weight>.0001 {relief/=relief_weight;}
    var level=0.;var support=0.;
    for(var i=0u;i<4u;i++){let cell=water_cells[indices[i]];let h=cell.levels[mode];
        if abs(h-ground)<3. && cell.terrain.z>.5 {let w=smooth_weights[i];level+=h*w;support+=w;}}
    if valid<.001 || support<.00001 {return BasinSample(vec4<f32>(-1000000.,0.,0.,0.),0.);}
    return BasinSample(vec4<f32>(level/support,ground,ground+relief,support*valid),confidence);
}
fn contact_ripples(point:vec3<f32>)->vec3<f32> {
    var result=vec3<f32>(0.);
    for(var i=0u;i<8u;i++) {
        let wake=wakes[i];let age=wake.position.w;let strength=wake.shape.z;
        if age<0. || age>.95 || strength<=0. || abs(point.y-wake.position.z)>.08 {continue;}
        let delta=point.xz-wake.position.xy;let direction=wake.shape.xy;let side=vec2<f32>(-direction.y,direction.x);
        let forward=dot(delta,direction);let lateral=dot(delta,side);
        let fade=pow(max(0.,1.-age/.95),2.1)*smoothstep(0.,.035,age)*strength;
        if wake.shape.w<0. {
            // Small non-circular, time-varying eddies trail each wet wheel.
            let speed=min(-wake.shape.w,8.);let trail=-forward;let span=.27+age*(.28+speed*.34);
            if trail<-.05 || trail>span {continue;}
            let width=.065+age*.03+trail*.09;let q=lateral/width;
            if abs(q)>2.4 {continue;}
            let variation=lattice_hash(vec2<i32>(floor(wake.position.xy*11.)))*6.2831853;
            let envelope=exp(-q*q)*(1.-smoothstep(span*.65,span,trail))*smoothstep(-.05,.04,trail);
            let phase=trail*34.+lateral*17.-age*min(28.,speed*8.)+variation;
            let wave=sin(phase);let gradient=(-direction*34.+side*17.)*cos(phase);
            result+=vec3<f32>(gradient*.0018,wave*.72)*fade*envelope;
        } else {
            // A soft, irregular footprint disturbance, with no circular ring.
            let long=.22+age*.11;let wide=.14+age*.07;
            let x=forward/long;let y=lateral/wide;
            if abs(x)>2. || abs(y)>2. {continue;}
            let variation=lattice_hash(vec2<i32>(floor(wake.position.xy*11.)))*6.2831853;
            let envelope=exp(-1.5*(x*x+y*y));
            let phase=forward*30.+sin(lateral*23.)*.35-age*10.+variation;
            let wave=sin(phase);let gradient=(direction*30.+side*(8.05*cos(lateral*23.)))*cos(phase);
            result+=vec3<f32>(gradient*.0020,wave*.80)*fade*envelope;
        }
    }
    return result;
}
fn character_at(pixel:vec2<i32>)->f32 {
    let size=vec2<i32>(textureDimensions(scene_depth));let at=clamp(pixel,vec2<i32>(0),size-1);var mask=0.;
    for(var i=0u;i<WEATHER_SAMPLESu;i++){mask=max(mask,f32(textureLoad(native_stencil,at,i32(i)).x)/255.);}
    return mask;
}
// A resolved water surface has a measurable entry/exit thickness. The
// bottom stays visible in the shallows and receives increasing absorption below it.
fn standing_water(pixel:vec2<f32>,state:Pixel)->vec4<f32> {
    let neutral=vec4<f32>(0.,0.,0.,1.);
    if state.depth>=.9999999 || water_domain.state.x<.5 || state.character>.001 {return neutral;}
    let relative=unproject(pixel,state.depth);let near=unproject(pixel,0.);
    let origin=weather.camera.xyz+near;let ray=normalize(relative-near);
    let native_travel=length(relative-near);
    let native_point=weather.camera.xyz+relative;
    if length(native_point.xz-weather.camera.xz)>52. {return neutral;}
    let initial=basin_sample(native_point.xz);var sample=initial.field;
    if sample.x< -999999. || native_point.y>sample.x+.02 {return neutral;}
    // Smooth native-floor agreement rejects curtains on walls and inferred gaps.
    let floor_admission=1.-smoothstep(.10,.28,abs(native_point.y-sample.y));
    let submerged=origin.y<sample.x+.015;
    if !submerged && ray.y>=-.004 {return neutral;}
    var travel=select((sample.x-origin.y)/min(-.0001,ray.y),0.,submerged);var point=select(origin+ray*travel,native_point,submerged);
    sample=basin_sample(point.xz).field;
    if sample.x< -999999. {return neutral;}
    travel=select((sample.x-origin.y)/min(-.0001,ray.y),0.,submerged);point=select(origin+ray*travel,native_point,submerged);
    sample=basin_sample(point.xz).field;
    // A ray may cross distinct basins; never shade an unconverged height switch.
    if sample.w<.00001 || travel<0. || travel>native_travel+.012 || (!submerged && abs(point.y-sample.x)>.025) {return neutral;}
    let bottom_vertical=max(0.,sample.x-(weather.camera.y+relative.y));
    let depth=max(bottom_vertical,max(0.,sample.x-sample.y));
    let wet_depth=mix(sample.x-sample.z,bottom_vertical,smoothstep(.025,.075,bottom_vertical));
    let q=(point.xz-water_domain.grid.xy)/water_domain.grid.z;
    let boundary=min(min(q.x,q.y),min(water_domain.grid.w-1.-q.x,water_domain.grid.w-1.-q.y))*water_domain.grid.z;
    let range=(1.-smoothstep(32.,46.,length(point.xz-weather.camera.xz)))*smoothstep(0.,5.,boundary);
    let full=vec2<f32>(textureDimensions(scene_depth));
    let footprint=native_travel/(full.y*max(.08,abs(ray.y)));
    let shore=smoothstep(0.,max(.004,footprint*.10),wet_depth);
    let bottom_path=max(0.,native_travel-travel);
    let contact_fade=smoothstep(0.,max(.003,footprint*.06),bottom_path);
    let alpha=shore*range*contact_fade*smoothstep(0.,.65,sample.w)*floor_admission;
    if alpha<.001 {return neutral;}
    let rain=clamp(weather.state.x,0.,1.);
    let ripple_detail=1.-smoothstep(.025,.12,footprint);
    var impacts=vec3<f32>(0.);if rain*ripple_detail>.002 {impacts=ripple(point.xz)*rain*ripple_detail;}
    let contact=contact_ripples(vec3<f32>(point.x,sample.x,point.z));
    impacts+=contact*ripple_detail;
    // Two smooth swell scales are damped close to shore, without moving the
    // simulation or changing the solved mean volume used by wheel contacts.
    let clock=weather.state.w;let damping=smoothstep(.002,.08,wet_depth);
    let phase=vec3<f32>(point.x*1.1+point.z*.45-clock*.65,
        point.x*-.55+point.z*1.6+clock*.85,(point.x+point.z)*3.1-clock*1.1);
    let amplitude=vec3<f32>(.0045,.0028,.0012)*damping;
    let height=dot(sin(phase),amplitude);
    let gradient=vec2<f32>(dot(cos(phase)*amplitude,vec3<f32>(1.1,-.55,3.1)),
        dot(cos(phase)*amplitude,vec3<f32>(.45,1.6,3.1)));
    // Stable mean surface; small waves perturb normals/refraction without toggling floor coverage.
    let normal=normalize(vec3<f32>(-gradient.x+impacts.x*.45,1.,-gradient.y+impacts.y*.45));
    let view=normalize(weather.camera.xyz-point);let reflected=reflect(-view,normal);
    let sky_reflection=storm_sky(reflected);var reflection=sky_reflection;
    if weather.look.w>.5 {
        let cube=textureSampleLevel(environment,reflection_sampler,reflected*vec3<f32>(-1.,-1.,1.),min(1.2,f32(textureNumLevels(environment)-1u))).rgb;
        reflection=mix(storm_tone(cube),reflection,smoothstep(.10,.65,reflected.y));
    }
    let raw_fresnel=.0204+.9796*pow(1.-clamp(dot(normal,view),0.,1.),5.);
    // Rough shallow water retains more of the bottom at grazing angles.
    let reflection_cap=mix(.28,.55,smoothstep(.015,.30,depth));
    let fresnel=select(min(raw_fresnel,reflection_cap),.02,submerged);
    let close=1.-smoothstep(6.,16.,length(point.xz-weather.camera.xz));
    reflection=mix(reflection,vec3<f32>(dot(reflection,vec3<f32>(.2126,.7152,.0722))),close*.65);
    // Project a refracted ray through the measured water column. Validate its
    // bottom sample so foreground skaters/walls cannot bleed into the water.
    let refracted=refract(ray,normal,1./1.333);
    let thickness=min(min(max(0.,native_travel-travel),2.5),max(.08,depth*2.5));
    let bottom=point+refracted*min(thickness,depth/max(.12,-refracted.y));
    let projected=weather.clip*vec4<f32>(bottom-weather.camera.xyz,1.);
    let original_uv=pixel/full;var uv=original_uv;
    if projected.w>.0001 && !submerged {
        let candidate=projected.xy/projected.w*vec2<f32>(.5,-.5)+.5;
        // Never clamp an offscreen ray onto a screen border: that stretched the
        // final colour row into the bottom of the picture in Preview30.
        let margin=3./full;
        if all(candidate>margin) && all(candidate<1.-margin) {
            let delta=(candidate-original_uv)*full;let pixels=length(delta);
            let edge=min(min(pixel.x,full.x-pixel.x),min(pixel.y,full.y-pixel.y));
            let camera_fade=smoothstep(.10,.35,abs(weather.camera.y-sample.x));
            let offset=delta*min(1.,3./max(.0001,pixels))*smoothstep(2.,12.,edge)*camera_fade;
            let refracted_uv=original_uv+offset/full;let at=vec2<i32>(refracted_uv*full);
            let z=puddle_depth(at);let hit=unproject(refracted_uv*full,z)+weather.camera.xyz;
            let mask=max(max(character_at(at+vec2<i32>(1,0)),character_at(at-vec2<i32>(1,0))),max(character_at(at+vec2<i32>(0,1)),character_at(at-vec2<i32>(0,1))));
            if z<.9999999 && hit.y<sample.x+.008 && mask<.001 {uv=refracted_uv;}
        }
    }
    let floor_colour=storm_tone(textureSampleLevel(scene_colour,reflection_sampler,uv,0.).rgb);
    // Unlit or missing cube faces must not turn water opaque black. A soft
    // floor/overcast fallback fades in only where that reflection is dark.
    let reflection_luma=dot(reflection,vec3<f32>(.2126,.7152,.0722));
    let sky_luma=dot(sky_reflection,vec3<f32>(.2126,.7152,.0722));
    let reflection_fallback=mix(floor_colour*.72,vec3<f32>(sky_luma),.20);
    reflection=mix(reflection_fallback,reflection,smoothstep(.008,.09,reflection_luma));
    // Beer-Lambert-style optical thickness, not an opaque puddle-colour decal.
    let transmission=exp(-thickness*vec3<f32>(.10,.095,.09));
    let scatter=vec3<f32>(.09,.09,.09)*(1.-transmission);
    let underwater=floor_colour*mix(vec3<f32>(1.),vec3<f32>(.94,.94,.94),shore)*transmission+scatter;
    let ripple_response=clamp(impacts.z,-1.5,1.5);
    let colour=mix(underwater*(1.+ripple_response*mix(.06,.10,close)),reflection,clamp(fresnel,.02,.78))
        +vec3<f32>(mix(.035,.060,close))*max(ripple_response,0.)
        +vec3<f32>(.20)*clamp(contact.z,0.,.60)*ripple_detail*close;
    return vec4<f32>(colour*alpha,1.-alpha);
}

// Composite water in front of only the actually submerged part of a
// character. Keep its own colour/depth in place: never refract its silhouette.
fn submerged_character(pixel:vec2<f32>,state:Pixel,colour:vec3<f32>)->vec3<f32> {
    if state.character<.5 || water_domain.state.x<.5 {return colour;}
    let relative=unproject(pixel,state.depth);let near=unproject(pixel,0.);
    let origin=weather.camera.xyz+near;let native=weather.camera.xyz+relative;
    let ray=normalize(relative-near);let travel_native=length(relative-near);
    if ray.y>=-.004 || length(native.xz-weather.camera.xz)>52. {return colour;}
    let at_body=basin_sample(native.xz);let body=at_body.field;
    if at_body.confidence<.75 || body.w<.5 || body.x-body.z<.006 ||
       native.y>=body.x-.002 || native.y<body.y-.12 || origin.y<=body.x {return colour;}
    var travel=(body.x-origin.y)/ray.y;
    if travel<0. || travel>travel_native {return colour;}
    var surface=origin+ray*travel;let surface_sample=basin_sample(surface.xz);let water=surface_sample.field;
    if surface_sample.confidence<.75 || water.w<.5 || water.x-water.z<.006 || abs(water.x-body.x)>.08 {return colour;}
    travel=(water.x-origin.y)/ray.y;
    if travel<0. || travel>travel_native {return colour;}
    surface=origin+ray*travel;
    let immersion=max(0.,water.x-native.y);
    let fade=smoothstep(.002,.06,immersion)*smoothstep(0.,.08,water.x-water.z);
    let reflection=storm_sky(reflect(ray,vec3<f32>(0.,1.,0.)));
    let amount=fade*mix(.18,.34,smoothstep(.03,.30,immersion))*state.character;
    return mix(colour,reflection,amount);
}

fn after_rain_sky(direction:vec3<f32>,clear:vec3<f32>)->vec3<f32> {
    var colour=mix(clear,storm_sky(direction),.72);
    // An authored clearing rainbow is anchored on preset/map entry, then stays
    // fixed in the world while the camera turns. It is not a HUD overlay.
    let yaw=water_domain.state.w;
    let centre=normalize(vec3<f32>(cos(yaw),-.12,sin(yaw)));
    let angle=acos(clamp(dot(direction,centre),-1.,1.));
    let band=(angle-.705)/.055;
    let fade=smoothstep(.025,.16,direction.y)*(1.-smoothstep(.70,.90,direction.y));
    let spectrum=vec3<f32>(exp(-pow((band-.86)/.24,2.)),exp(-pow((band-.50)/.25,2.)),exp(-pow((band-.16)/.24,2.)));
    return colour+spectrum*fade*.095;
}

@fragment fn fs_compose(v:Vertex)->@location(0) vec4<f32> {
    let uv=v.clip.xy/vec2<f32>(textureDimensions(scene_depth));let pixel=vec2<i32>(v.clip.xy);
    let original=textureSampleLevel(scene_colour,reflection_sampler,uv,0.);
    let state=pixel_state(pixel);let mask=state.character;let z=state.depth;var colour=storm_tone(original.rgb);
    let sky=state.sky>.9999;
    if sky {
        colour=storm_sky(normalize(unproject(v.clip.xy,.99999)));
    } else if z<.9999999 {
        let coat=standing_water(v.clip.xy,state);colour=colour*coat.a+coat.rgb;
        if mask>.001 {colour=submerged_character(v.clip.xy,state,colour);}
        let distance=length(unproject(v.clip.xy,z));
        colour=mix(colour,vec3<f32>(.14,.16,.18),smoothstep(28.,150.,distance)*weather.state.x*weather.state.x*.18);
    }
    if sky && weather.colour.w>.5 {colour=after_rain_sky(normalize(unproject(v.clip.xy,.99999)),original.rgb);}
    if state.sky>0. && !sky {
        // A resolved MSAA edge contains geometry plus native clear sky. Replace
        // only the sky share, preserving the geometry's original coverage.
        let direction=normalize(unproject(v.clip.xy,.99999));
        let clear=clear_sky_neighbour(pixel,original.rgb);
        var overcast=storm_sky(direction);
        if weather.colour.w>.5 {overcast=after_rain_sky(direction,clear);}
        colour=max(vec3<f32>(0.),colour-storm_tone(clear)*state.sky)+overcast*state.sky;
    }
    let flash=weather.lightning.x;colour*=1.+0.45*flash;
    if sky && flash>0. {let direction=normalize(unproject(v.clip.xy,.99999));colour=colour/(1.+.45*flash)+lightning(direction,weather.lightning.y,weather.lightning.z,flash);}
    return vec4<f32>(colour,original.a);
}
// The supplied CPU ribbon builder owns motion, shelter and geometry clipping.
struct Drop { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) alpha:f32,@location(2) tint:vec3<f32> }
@vertex fn vs_drop(@location(0) position:vec3<f32>,@location(1) colour:vec4<f32>,@location(2) uv:vec2<f32>)->Drop {
    return Drop(weather.clip*vec4<f32>(position-weather.camera.xyz,1.),uv,colour.a,colour.rgb);
}
// A drop instance's camera-facing streak (weather_rain DropInstance): the side across its fall as
// the camera sees it, widened with distance so a far streak keeps its pixel and its opacity scaled
// down by that widening; the camera's right when the camera looks along the fall.
@vertex fn vs_streak(@builtin(vertex_index) corner:u32,@location(3) head:vec4<f32>,@location(4) fall:vec4<f32>,@location(5) half_width:f32)->Drop {
    let relative=head.xyz-weather.camera.xyz;
    let across=cross(-relative,fall.xyz);
    let squared=dot(across,across);
    let right=normalize(vec3<f32>(weather.clip[0].x,weather.clip[1].x,weather.clip[2].x));
    let width=max(half_width,length(relative)*.00045);
    let side=select(right,across*inverseSqrt(squared),squared>1e-20)*width;
    let tail=head.xyz+fall.xyz*head.w;
    var position=head.xyz-side;var uv=vec2<f32>(0.,0.);
    if corner==1u {position=head.xyz+side;uv=vec2<f32>(1.,0.);}
    else if corner==2u {position=tail+side;uv=vec2<f32>(1.,1.);}
    else if corner==3u {position=tail-side;uv=vec2<f32>(0.,1.);}
    return Drop(weather.clip*vec4<f32>(position-weather.camera.xyz,1.),uv,fall.w*(half_width/width),vec3<f32>(.79,.80,.82));
}
@fragment fn fs_drop(v:Drop)->@location(0) vec4<f32> {
    let du=max(.001,fwidth(v.uv.x));let dv=max(.001,fwidth(v.uv.y));
    var nearest=1.;for(var sample=0u;sample<WEATHER_SAMPLESu;sample++){nearest=min(nearest,textureLoad(scene_depth,vec2<i32>(v.position.xy),i32(sample)));}
    if v.alpha<=0. || v.position.z>nearest+1e-6 {discard;}
    if v.uv.x>=2. {
        let p=vec2<f32>(v.uv.x-2.,v.uv.y)*2.-1.;
        let width=1.-smoothstep(.35,1.+du,abs(p.x));
        let tip=1.-smoothstep(.55,1.+dv,abs(p.y));
        return vec4<f32>(v.tint,v.alpha*width*tip);
    }
    let edge=abs(v.uv.x*2.-1.);
    let across=1.-smoothstep(.15,1.+du,edge);
    let taper=pow(max(0.,sin(3.14159265359*v.uv.y)),1.4);
    let bead=.80+.20*sin(v.uv.y*18.);
    return vec4<f32>(v.tint,v.alpha*across*taper*bead);
}
