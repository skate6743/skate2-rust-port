//! Final defaultcharacter/defaulthair programs, sharing stamp's material/SH/shadow owners.
use super::*;

pub(super) const HAIR_ATTRIBUTES:[wgpu::VertexAttribute;7]=wgpu::vertex_attr_array![
    0=>Float32x3,1=>Float32x2,2=>Uint8x4,3=>Uint8x4,4=>Float32x3,5=>Float32x3,6=>Float32x2];

impl GpuRenderer {
    /// Attach original material resources to the already owned character mesh.
    /// The ordinary palette, indexed draws, opacity and shadow owner are retained.
    pub(crate) fn bind_character_surface(&self,object:&mut GpuObject,geometry:&NativeMeshGeometry,
        normal:&GpuTexture,alpha:Option<&GpuTexture>,inputs:&stamp::Inputs)->RenderResult<()> {
        let character=object.character.as_ref().ok_or(CharacterGpuError::NotCharacterObject)?;
        let hair=character.family.is_hair();
        if !matches!(character.family,CharacterShaderFamily::Character|CharacterShaderFamily::Hair|CharacterShaderFamily::HairRopa|CharacterShaderFamily::Ropa){
            return Err("character surface material family".into());
        }
        if hair!=alpha.is_some(){return Err(CharacterGpuError::HairAlphaBinding.into());}
        let base=vertex_bytes(geometry,character.palette_count,character.family)?;
        let (tangent,binormal)=stamp::vectors(geometry)?;
        let mut vertices=Vec::with_capacity(geometry.positions.len()*if hair{60}else{52});
        for (index,base) in base.chunks_exact(if hair{36}else{28}).enumerate(){
            vertices.extend_from_slice(&base[..28]);
            vertices.extend(tangent[index].iter().chain(&binormal[index]).flat_map(|v|v.to_ne_bytes()));
            if hair{vertices.extend_from_slice(&base[28..36]);}
        }
        self.bind_character_lighting(object,normal,alpha,inputs)?;
        object.mesh.vertices=self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label:Some("original character tangent/binormal surface stream"),contents:&vertices,
            usage:wgpu::BufferUsages::VERTEX|wgpu::BufferUsages::COPY_DST}).into();
        Ok(())
    }
}

// Final PS: defaultcharacter pc90 SH floor,106 rim,109 specular clamp;
// defaulthair pc51 rim,55 SH floor,67 specular clamp. UPDB's rim/spec differs.
pub(super) const SHADER:&str=r"
@group(3) @binding(8) var surface_alpha:texture_2d<f32>;
@group(3) @binding(9) var surface_alpha_sampler:sampler;
@vertex fn vs_surface_hair(@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,
    @location(2) weights:vec4<u32>,@location(3) indices:vec4<u32>,
    @location(4) tangent:vec3<f32>,@location(5) binormal:vec3<f32>,@location(6) alpha_uv:vec2<f32>)->Output {
    var result=surface_vertex(position,uv,weights,indices,tangent,binormal);
    result.uv=vec4<f32>(uv,alpha_uv);return result;
}
fn surface_colour(input:Output,hair:bool,hair_ropa:bool)->vec4<f32>{
    let sample=textureSample(diffuse,normal_sampler,input.uv.xy);
    let colour=sample*sample;
    let packed=textureSample(normal_map,normal_sampler,input.uv.xy);
    let xy=packed.ag*2.-vec2<f32>(1.);
    let z=sqrt(abs(1.-dot(xy,xy)));
    let n=normalize(xy.x*input.tangent+xy.y*input.binormal+z*input.normal);
    let view=normalize(-input.relative);let light=stamp.light.xyz;
    let rim=normalize(light*vec3<f32>(-1.,.2,-1.)-view);
    let world=vec4<f32>(input.relative+stamp.camera_lod.xyz,1.);
    var shadow=world_shadow_value(world);
    if !hair {shadow*=csm_shadow(world);}
    let facing=1.-clamp(dot(n,view),0.,1.);
    let key_fresnel=pow(facing,stamp.material[1].w);
    let rim_fresnel=pow(facing,stamp.material[0].z);
    let key_diffuse=clamp(dot(n,light),0.,1.)*stamp.material[1].xyz*shadow;
    let rim_diffuse=clamp(dot(n,rim)*rim_fresnel,0.,1.)*stamp.material[6].xyz;
    let ambient=irradiance(n)*stamp.material[5].w;
    let key_spec=pow(clamp(dot(view,reflect(-light,n)),0.,1.),stamp.material[2].w)*shadow;
    let rim_spec=pow(clamp(dot(view,reflect(-rim,n)),0.,1.),stamp.material[3].w);
    // Final defaulthair_ropa_defaultPS pc61 squares RGB only; pc67 uses
    // the original diffuse alpha. Ordinary hair squares all four lanes.
    let specular_mask=select(colour.a,sample.a,hair_ropa);
    let spec=clamp((key_spec*stamp.material[2].xyz*key_fresnel+rim_spec*stamp.material[3].xyz*rim_fresnel)*specular_mask,vec3<f32>(0.),vec3<f32>(1.));
    if CHARACTER_LIGHT_VIEW>=4u && CHARACTER_LIGHT_VIEW<=6u {
        let p=vec2<f32>(dot(world_shadow.rows[0],world),dot(world_shadow.rows[1],world));
        let depth=dot(world_shadow.rows[2],world);
        let moments=textureSample(shadow_world,world_sampler,p*vec2<f32>(.5,-.5)+vec2<f32>(.5)).xy;
        if CHARACTER_LIGHT_VIEW==6u {
            let above=(world_shadow.rows[2].w-moments.x)/(-world_shadow.rows[2].y)-world.y;
            return vec4<f32>(clamp(above/5.,0.,1.),clamp(above/50.,0.,1.),clamp(above/500.,0.,1.),object.color.a);
        }
        if CHARACTER_LIGHT_VIEW==4u {return vec4<f32>(select(0.,1.,max(abs(p.x),abs(p.y))<.99),moments.y,select(0.,1.,moments.x>=depth),object.color.a);}
        return vec4<f32>(p*.5+vec2<f32>(.5),clamp(depth,0.,1.),object.color.a);
    }
    if CHARACTER_LIGHT_VIEW==1u {return vec4<f32>(select(csm_shadow(world),1.,hair),world_shadow_value(world),clamp(dot(n,light),0.,1.),object.color.a);}
    let lit=colour.xyz*(key_diffuse+ambient+rim_diffuse)+spec;
    let x=(lit*(stamp.material[0].y*input.fog.a)+input.fog.xyz)*stamp.attributes[2].z;
    let low=clamp(vec3<f32>(1.)-x,vec3<f32>(0.),vec3<f32>(1.));
    let mapped=(max(x*.25+vec3<f32>(.75),vec3<f32>(1.))-low*low)*.5;
    var alpha=object.color.a;
    if hair {alpha*=textureSample(surface_alpha,surface_alpha_sampler,input.uv.zw).r;}
    return vec4<f32>(sqrt(abs(mapped)),alpha);
}
@fragment fn fs_surface(input:Output)->@location(0) vec4<f32>{return surface_colour(input,false,false);}
@fragment fn fs_surface_ghost(input:Output)->@location(0) vec4<f32>{
    let colour=surface_colour(input,false,false);if object.color.a<.0039215689{discard;}return colour;
}
@fragment fn fs_surface_hair(input:Output)->@location(0) vec4<f32>{return surface_colour(input,true,false);}
@fragment fn fs_surface_hair_ropa(input:Output)->@location(0) vec4<f32>{return surface_colour(input,true,true);}
";
