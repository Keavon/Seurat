[[block]] struct CameraUniform {
	view_position: vec4<f32>;
	view_proj: mat4x4<f32>;
};
[[block]] struct Light {
	position: vec3<f32>;
	color: vec3<f32>;
};

// Uniforms
[[group(0), binding(0)]] var<uniform> camera_uniform: CameraUniform;
[[group(1), binding(0)]] var<uniform> light: Light;

// Attributes
struct VertexInput {
	[[location(0)]] position: vec3<f32>;
};
struct InstanceInput {
	// Model matrix (4x4)
	[[location(5)]] model_matrix_0: vec4<f32>;
	[[location(6)]] model_matrix_1: vec4<f32>;
	[[location(7)]] model_matrix_2: vec4<f32>;
	[[location(8)]] model_matrix_3: vec4<f32>;

	// Normal matrix (3x3)
	[[location(9)]] normal_matrix_0: vec3<f32>;
	[[location(10)]] normal_matrix_1: vec3<f32>;
	[[location(11)]] normal_matrix_2: vec3<f32>;
};

// Varyings
struct VertexOutput {
	[[builtin(position)]] clip_position: vec4<f32>;
	[[location(0)]] color: vec3<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	let scale = 0.25;
	var out: VertexOutput;
	out.clip_position = camera_uniform.view_proj * vec4<f32>(model.position * scale + light.position, 1.0);
	out.color = light.color;
	return out;
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	return vec4<f32>(in.color, 1.0);
}
