struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // The mochiOS scene protocol uses a top-left origin. WGPU clip-space uses
    // positive Y at the top of the render target, so invert the protocol Y
    // coordinate when presenting the same scene on desktop Linux.
    output.position = vec4<f32>(input.position.x, -input.position.y, input.position.z, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(atlas, atlas_sampler, input.uv) * input.color;
}
