// Uniforms
[[group(0), binding(0)]] var t_ao: texture_2d<f32>;
[[group(0), binding(1)]] var s_ao: sampler;

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
	let texel_size = 1. / vec2<f32>(textureDimensions(t_ao));

	var result = 0.;
	for (var x = -2; x < 2; x = x + 1) {
		for (var y = -2; y < 2; y = y + 1) {
			let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
			result = result + textureSample(t_ao, s_ao, in.uv).r;
		}
	}
	result = result / (4.0 * 4.0);
	return vec4<f32>(result, result, result, 1.);
}
