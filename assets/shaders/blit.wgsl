// Uniforms
[[group(0), binding(0)]] var t_color: texture_2d<f32>;
[[group(0), binding(1)]] var s_color: sampler;

// Attributes
struct VertexInput {
	[[location(0)]] position: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] normal: vec3<f32>;
	[[location(3)]] tangent: vec3<f32>;
};

// Varyings
struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] tex_coords: vec2<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput) -> VertexOutput {
	return VertexOutput(vec4<f32>(model.position, 1.), model.position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5));
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	return textureSample(t_color, s_color, in.tex_coords);
	// return vec4<f32>(1.0, 0.2, 0.2, 1.0);
}
