// Uniforms
[[group(0), binding(0)]] var t_tangent_space_fragment_location: texture_2d<f32>;
[[group(0), binding(1)]] var s_tangent_space_fragment_location: sampler;
[[group(0), binding(2)]] var t_tangent_space_eye_location: texture_2d<f32>;
[[group(0), binding(3)]] var s_tangent_space_eye_location: sampler;
[[group(0), binding(4)]] var t_tangent_space_light_location: texture_2d<f32>;
[[group(0), binding(5)]] var s_tangent_space_light_location: sampler;
[[group(0), binding(6)]] var t_albedo_map: texture_2d<f32>;
[[group(0), binding(7)]] var s_albedo_map: sampler;
[[group(0), binding(8)]] var t_arm_map: texture_2d<f32>;
[[group(0), binding(9)]] var s_arm_map: sampler;
[[group(0), binding(10)]] var t_normal_map: texture_2d<f32>;
[[group(0), binding(11)]] var s_normal_map: sampler;

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
	return textureSample(t_albedo_map, s_albedo_map, in.tex_coords);
	// return vec4<f32>(1.0, 0.2, 0.2, 1.0);
}
