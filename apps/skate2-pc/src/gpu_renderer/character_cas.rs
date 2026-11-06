//! Distinct original CAS material programs; common skinning/lighting owners are
//! shared with character_surface. See edit-skater-room evidence for boundaries.
use super::*;
pub(crate) fn mode(effect: &str) -> Option<f32> {
    match effect {
        "cacstamp_cloth" | "cacstamp_cloth_ropa" => Some(0.),
        "cacstamp_skin" => Some(1.),
        "cacstamp_face" => Some(2.),
        "cacstamp_shift" => Some(3.),
        "cac_hair" | "cac_hair_ropa" => Some(4.),
        _ => None,
    }
}
pub(super) fn cas_vertex_bytes(geometry:&NativeMeshGeometry,palette_count:u16,family:CharacterShaderFamily)->Result<Vec<u8>,CharacterGpuError>{
        let base = vertex_bytes(geometry, palette_count, family)?;
        let (tangent, binormal) = stamp::vectors(geometry)?;
        // D3D::PatchVertexShaderToMatchVertexDeclaration82c43eb0..bc
        // supplies (0,0,0,1) for an absent semantic. Some original CAS cloth
        // meshes omit TEXCOORD1; its zero XY disables the decal branch.
        let uv = declared_uv(geometry, 1)?;
        let mut vertices = Vec::with_capacity(geometry.positions.len() * 60);
        for (index, base) in base
            .chunks_exact(if family == CharacterShaderFamily::Hair {
                36
            } else {
                28
            })
            .enumerate()
        {
            vertices.extend_from_slice(&base[..28]);
            vertices.extend(
                tangent[index]
                    .iter()
                    .chain(&binormal[index])
                    .flat_map(|v| v.to_ne_bytes()),
            );
            vertices.extend(uv.map_or([0.0; 2], |uv| uv[index]).iter().flat_map(|v| v.to_ne_bytes()));
        }
    Ok(vertices)
}
impl GpuRenderer {
    pub(crate) fn bind_cas_surface(
        &self,
        object: &mut GpuObject,
        geometry: &NativeMeshGeometry,
        normal: &GpuTexture,
        alpha: Option<&GpuTexture>,
        decal: Option<&GpuTexture>,
        decal2: Option<&GpuTexture>,
        inputs: &stamp::Inputs,
        custom: [[f32; 4]; 5],
    ) -> RenderResult<()> {
        let character = object
            .character
            .as_ref()
            .ok_or(CharacterGpuError::NotCharacterObject)?;
        let vertices=cas_vertex_bytes(geometry,character.palette_count,character.family)?;
        self.bind_character_lighting_custom(
            object,
            normal,
            alpha,
            decal,
            decal2,
            inputs,
            Some(custom),
        )?;
        object.mesh.vertices = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("original CAS skin/tangent/decal stream"),
                contents: &vertices,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
            .into();
        Ok(())
    }
    pub(crate) fn set_character_fog(
        &self,
        object: &GpuObject,
        frame: super::super::FrameEnvironment,
    ) -> RenderResult<()> {
        let c = object
            .character
            .as_ref()
            .ok_or(CharacterGpuError::NotCharacterObject)?;
        let mut data = [0; 32];
        for (bytes, value) in data.chunks_exact_mut(4)
            .zip(frame.fog_k1.into_iter().chain(frame.fog_colour)) {
            bytes.copy_from_slice(&value.to_ne_bytes());
        }
        self.character_uploads
            .write(&c.rows, ((MAX_BONES * 3 + 2) * 16) as u64, &data);
        Ok(())
    }
}
pub(super) const SHADER: &str = r"
const CHARACTER_LIGHT_VIEW:u32=0u;
@group(3) @binding(10) var cas_decal:texture_2d<f32>;
@group(3) @binding(11) var cas_decal2:texture_2d<f32>;
@vertex fn vs_cas(@location(0) p:vec3<f32>,@location(1) uv:vec2<f32>,
 @location(2) w:vec4<u32>,@location(3) i:vec4<u32>,@location(4) t:vec3<f32>,
 @location(5) b:vec3<f32>,@location(6) uv2:vec2<f32>)->Output {
 var o=surface_vertex(p,uv,w,i,t,b);o.uv=vec4<f32>(uv,uv2);return o;
}
fn cas_tone(lit:vec3<f32>,fog:vec4<f32>)->vec3<f32>{
 let x=(lit*fog.a+fog.xyz)*stamp.attributes[2].z;
 let low=clamp(vec3<f32>(1.)-x,vec3<f32>(0.),vec3<f32>(1.));
 return sqrt(abs((max(x*.25+vec3<f32>(.75),vec3<f32>(1.))-low*low)*.5));
}
fn cas_colour(input:Output)->vec4<f32>{
 let mode=stamp.custom[4].x;
 let uv=input.uv.zw;
 let decal_sample=textureSample(cas_decal,normal_sampler,uv);
 let dirt_sample=textureSample(cas_decal2,normal_sampler,input.uv.xy);
 let alpha_sample=textureSample(surface_alpha,surface_alpha_sampler,input.uv.xy);
 let hair_alpha=textureSample(surface_alpha,surface_alpha_sampler,uv).r;
 var tattoo=stamp.custom[3];var tattoo_alpha=alpha_sample.g;
 if alpha_sample.r!=0. {tattoo=stamp.custom[2];tattoo_alpha=alpha_sample.r;}
 var tattoo_index=0.;
 if tattoo_alpha>.8 {tattoo_index=tattoo.w;}else if tattoo_alpha>.6 {tattoo_index=tattoo.z;}
 else if tattoo_alpha>.4 {tattoo_index=tattoo.y;}else if tattoo_alpha>.2 {tattoo_index=tattoo.x;}
 let tattoo_scale=select(vec2<f32>(.25,.5),vec2<f32>(0.),tattoo_index>7.);
 let tattoo_offset=vec2<f32>(.25*f32(i32(floor(tattoo_index))%4),select(0.,.5,tattoo_index>3.));
 let tattoo_sample=textureSample(cas_decal,normal_sampler,uv*tattoo_scale+tattoo_offset);
 let sample=textureSample(diffuse,normal_sampler,input.uv.xy);
 var colour=vec4<f32>(sample.xyz*sample.xyz,1.);let spec_mask=sample.a*sample.a;
 let packed=textureSample(normal_map,normal_sampler,input.uv.xy);
 let xy=packed.ag*2.-vec2<f32>(1.);let z=sqrt(abs(1.-dot(xy,xy)));
 let n=normalize(xy.x*input.tangent+xy.y*input.binormal+z*input.normal);
 let view=normalize(-input.relative);let light=stamp.light.xyz;
 let world=vec4<f32>(input.relative+stamp.camera_lod.xyz,1.);
 let world_shadow=world_shadow_value(world);
 if mode==4. {
   var diffuse=clamp(.75*dot(n,light)+.25,0.,1.);
   if stamp.light.w>0. {diffuse*=1.-stamp.light.w;}
   let h=normalize(light+view);let th=dot(input.tangent,h);
   let spec=stamp.custom[1].xyz*pow(sqrt(1.-th*th),stamp.material[2].w)*clamp(1.75*dot(n,light)+.25,0.,1.);
   var direct_spec=spec;
   if stamp.light.w>0. {direct_spec*=1.-stamp.light.w;}
   let lit=((vec3<f32>(diffuse*world_shadow)+irradiance(n)*stamp.material[5].w)*stamp.custom[0].xyz+direct_spec*world_shadow)*colour.xyz;
   return vec4<f32>(cas_tone(lit,input.fog),hair_alpha*object.color.a);
 }
 if mode!=3. {colour*=stamp.custom[0];}
 if uv.x>0. && uv.y>0. {
   if mode==0. || (mode==3. && uv.x<1. && uv.y<1.) {
     let decal=decal_sample;colour=colour*(1.-decal.a)+decal*decal.a;
   } else if mode==1. {
     colour*=tattoo_sample;
   }
   // CAC constructs bruise enables at zero. Face therefore keeps its tinted
   // diffuse; skin's bruise overlay is likewise zero until gameplay owns it.
 }
 if mode==0. || mode==3. {
   let dirt=dirt_sample;
   let amount=select(stamp.custom[4].y,1.,mode==3.);
   colour=(colour*(1.-dirt.a)+dirt*dirt.a)*amount+(1.-amount)*colour;
 }
 // cacstamp_shift_defaultPS pc22/54 uses depth+1 and pc36..53 uses
 // fixed +/-one-texel offsets. Cloth/skin/face instead read charattributes.
 // The original shift HLSL also passes scalar -1 as the entire bias vector.
 var self_shadow=csm_shadow(world);
 if mode==3. {self_shadow=csm_shadow_parameters(world,-1.,-1.);}
 let shadow=self_shadow*world_shadow;
 let rim=normalize(light*vec3<f32>(-1.,.2,-1.)-view);
 let facing=1.-clamp(dot(n,view),0.,1.);
 let key_fresnel=pow(facing,stamp.material[1].w);let rim_fresnel=pow(facing,stamp.material[0].z);
 let key_diffuse=clamp(dot(n,light),0.,1.)*stamp.material[1].xyz*shadow;
 let rim_diffuse=clamp(dot(n,rim)*rim_fresnel,0.,1.)*stamp.material[6].xyz;
 let ambient=irradiance(n)*stamp.material[5].w;
 let key_spec=pow(clamp(dot(view,reflect(-light,n)),0.,1.),stamp.material[2].w)*shadow;
 let rim_spec=pow(clamp(dot(view,reflect(-rim,n)),0.,1.),stamp.material[3].w);
 let spec=clamp((key_spec*stamp.material[2].xyz*key_fresnel+rim_spec*stamp.material[3].xyz*rim_fresnel)*spec_mask,vec3<f32>(0.),vec3<f32>(1.));
 if CHARACTER_LIGHT_VIEW==1u {return vec4<f32>(self_shadow,world_shadow,clamp(dot(n,light),0.,1.),object.color.a);}
 if CHARACTER_LIGHT_VIEW==2u {return vec4<f32>(sqrt(clamp(key_diffuse*.25,vec3<f32>(0.),vec3<f32>(1.))),object.color.a);}
 if CHARACTER_LIGHT_VIEW==3u {return vec4<f32>(sqrt(clamp(ambient+rim_diffuse,vec3<f32>(0.),vec3<f32>(1.))),object.color.a);}
 let lit=colour.xyz*(key_diffuse+ambient+rim_diffuse)+spec;
 return vec4<f32>(cas_tone(lit*stamp.material[0].y,input.fog),object.color.a);
}
@fragment fn fs_cas(input:Output)->@location(0) vec4<f32>{return cas_colour(input);}
@fragment fn fs_cas_ghost(input:Output)->@location(0) vec4<f32>{
 let c=cas_colour(input);if object.color.a<.0039215689 {discard;}return c;
}
";
