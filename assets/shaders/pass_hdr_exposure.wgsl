// Uniforms
[[group(0), binding(0)]] var t_frame: texture_2d<f32>;
[[group(0), binding(1)]] var s_frame: sampler;

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
	[[location(0)]] uv: vec2<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput) -> VertexOutput {
	return VertexOutput(vec4<f32>(model.position, 1.), model.position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5));
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	var color = textureSample(t_frame, s_frame, in.uv).rgb;

	// Tone mapping
	color = color / (color + vec3<f32>(1.));

	// Gamma correction (linear to gamma)
	color = pow(color, vec3<f32>(1. / 2.2));

	return vec4<f32>(color, 1.);
}
