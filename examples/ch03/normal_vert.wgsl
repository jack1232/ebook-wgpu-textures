// vertex shader
struct Uniforms {   
    vpMat : mat4x4f,
    modelMat : mat4x4f,           
    normalMat : mat4x4f,             
};
@binding(0) @group(0) var<uniform> uniforms : Uniforms;

struct Light {
    lightPosition: vec4f,
    eyePosition: vec4f,   
}
@binding(1) @group(0) var<uniform> light : Light; 

struct Input {
    @location(0) pos: vec4f, 
    @location(1) normal: vec4f, 
    @location(2) uv: vec2f,
    @location(3) tangent: vec4f,
    @location(4) bitangent: vec4f,
}; 

struct Output {
    @builtin(position) position : vec4f,
    @location(0) vUv: vec2f,
    @location(1) tPosition: vec3f,
    @location(2) tLightPosition: vec3f,
    @location(3) tEyePosition: vec3f,
};

@vertex
fn vs_main(in:Input) -> Output {    
    var output: Output;          
    
    // create the tangent matrix
    let wNormal = normalize(uniforms.normalMat * in.normal);
    let wTangent = normalize(uniforms.normalMat * in.tangent);
    let wBitangent = normalize(uniforms.normalMat * in.bitangent);
    let tbnMat = transpose(mat3x3(wTangent.xyz, wBitangent.xyz, wNormal.xyz));
    
    let wPosition = uniforms.modelMat * in.pos;
    output.position = uniforms.vpMat * wPosition;
    output.vUv  = in.uv;
    output.tPosition = tbnMat * wPosition.xyz;
    output.tEyePosition = tbnMat * light.eyePosition.xyz;
    output.tLightPosition = tbnMat * light.lightPosition.xyz;
         
    return output;
}