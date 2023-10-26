struct LightUniforms {
    specularColor : vec4f,
};
@group(1) @binding(0) var<uniform> light : LightUniforms;

struct MaterialUniforms {
    ambient: f32,
    diffuse: f32,
    specular: f32,
    shininess: f32,
    withGammaCorrection: f32,
    heightScale: f32,
};
@group(1) @binding(1) var<uniform> material: MaterialUniforms;

@group(2) @binding(0) var textureData: texture_2d<f32>;     // base texture and sampler
@group(2) @binding(1) var textureSampler: sampler;          
@group(2) @binding(2) var textureData2: texture_2d<f32>;    // normal texture and sampler
@group(2) @binding(3) var textureSampler2: sampler;         
@group(2) @binding(4) var textureData3: texture_2d<f32>;    // depth texture and sampler
@group(2) @binding(5) var textureSampler3: sampler;         

struct Input {
    @location(0) vUv: vec2f,
    @location(1) tPosition: vec3f,
    @location(2) tLightPosition: vec3f,
    @location(3) tEyePosition: vec3f,
};

fn blinnPhong(N:vec3f, L:vec3f, V:vec3f) -> vec2f{
    let H = normalize(L + V);
    var diffuse = material.diffuse * max(dot(N, L), 0.0);
    diffuse += material.diffuse * max(dot(-N, L), 0.0);
    var specular = material.specular * pow(max(dot(N, H), 0.0), material.shininess);
    specular += material.specular * pow(max(dot(-N, H),0.0), material.shininess);
    return vec2(diffuse, specular);
}

fn parallaxMap(uv:vec2f, eyeDirection:vec3f) -> vec2f {
    let height = 1.0 - textureSample(textureData3, textureSampler3, uv).r;
    let delta = eyeDirection.xy * height * material.heightScale / eyeDirection.z;
    return uv - delta;
}

@fragment
fn fs_main(in: Input) ->  @location(0) vec4f {   
    let L = normalize(in.tLightPosition - in.tPosition);
    let V = normalize(in.tEyePosition - in.tPosition);       
   
    var uv = parallaxMap(in.vUv, V);
    
    let texColor = textureSample(textureData, textureSampler, uv);
    let texNormal = textureSample(textureData2, textureSampler2, uv);

    var N = normalize(texNormal.xyz * 2.0 - 1.0);
    let bp = blinnPhong(N, L, V);

    var finalColor = texColor.rgb * (material.ambient + bp[0] + light.specularColor.rgb * bp[1]); 
    if(material.withGammaCorrection == 1.0){
        finalColor = pow(finalColor, vec3(1.0/2.2));
    }
    return vec4(finalColor, 1.0);
}