// vertex shader
struct Uniforms {   
    vpMat : mat4x4f,
    modelMat : mat4x4f,           
    normalMat : mat4x4f,            
};
@binding(0) @group(0) var<uniform> uniforms : Uniforms;

struct Input {
    @location(0) pos: vec4f, 
    @location(1) normal: vec4f, 
    @location(2) uv: vec2f,
};

struct Output {
    @builtin(position) position : vec4f,
    @location(0) vPosition : vec4f,
    @location(1) vNormal : vec4f,
    @location(2) vUv: vec2f,
};

@vertex
fn vs_main(in: Input) -> Output {
    var output: Output;            
    let mPosition = uniforms.modelMat * in.pos; 
    output.vPosition = mPosition;                  
    output.vNormal =  uniforms.normalMat * in.normal;
    output.position = uniforms.vpMat * mPosition; 
    output.vUv = in.uv;              
    return output;
}