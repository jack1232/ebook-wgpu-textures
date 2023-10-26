struct LightUniforms {
    lightDirection : vec4f,
    eyePosition : vec4f,
    specularColor : vec4f,
};
@group(1) @binding(0) var<uniform> light: LightUniforms;

struct MaterialUniforms {
    ambient: f32,
    diffuse: f32,
    specular: f32,
    shininess: f32,
};
@group(1) @binding(1) var<uniform> material : MaterialUniforms;
@group(2) @binding(0) var textureData : texture_2d<f32>;
@group(2) @binding(1) var textureSampler: sampler;

struct Input {
    @location(0) vPosition: vec4f, 
    @location(1) vNormal: vec4f, 
    @location(2) vUv: vec2f,
};

fn blinnPhong(N:vec3f, L:vec3f, V:vec3f) -> vec2f {
    let H = normalize(L + V);
    var diffuse:f32 = material.diffuse * max(dot(N, L), 0.0);
    diffuse += material.diffuse * max(dot(-N, L), 0.0);
    var specular: f32 = material.specular * pow(max(dot(N, H), 0.0), material.shininess);
    specular += material.specular * pow(max(dot(-N, H),0.0), material.shininess);
    return vec2(diffuse, specular);
}

@fragment
fn fs_main(in: Input) ->  @location(0) vec4f {
    var N = normalize(in.vNormal.xyz);                  
    let L = normalize(-light.lightDirection.xyz);  
    let V = normalize(light.eyePosition.xyz - in.vPosition.xyz);       
    let bp = blinnPhong(N, L, V);

    let texColor = textureSample(textureData, textureSampler, in.vUv).rgb;

    var finalColor = texColor * (material.ambient + bp[0] + light.specularColor.rgb * bp[1]); 
    finalColor = pow(finalColor, vec3(1.0/2.2));
    return vec4(finalColor.rgb, 1.0);
}