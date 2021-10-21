[[block]] struct Camera {
	v_matrix: mat4x4<f32>;
	p_matrix: mat4x4<f32>;
};
[[block]] struct Light {
	location: vec3<f32>;
	color: vec3<f32>;
};

// Uniforms
[[group(0), binding(0)]] var<uniform> camera: Camera;
[[group(1), binding(0)]] var<uniform> light: Light;

// Attributes
struct VertexInput {
	[[location(0)]] position: vec3<f32>;
};
struct InstanceInput {
	[[location(5)]] model_matrix_0: vec4<f32>;
	[[location(6)]] model_matrix_1: vec4<f32>;
	[[location(7)]] model_matrix_2: vec4<f32>;
	[[location(8)]] model_matrix_3: vec4<f32>;
};

// Varyings
struct VertexOutput {
	[[builtin(position)]] clip_space_position: vec4<f32>;
	[[location(0)]] color: vec3<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	let scale = 0.25;

	// MVP matrices
	let m = mat4x4<f32>(instance.model_matrix_0, instance.model_matrix_1, instance.model_matrix_2, instance.model_matrix_3);
	let v = camera.v_matrix;
	let p = camera.p_matrix;
	let vp = p * v;

	// Locations
	let eye_location = v[3].xyz;
	let light_location = light.location;

	// Vertex data in model space
	let model_space_position = vec4<f32>(model.position, 1.0);

	// Vertex data in world space
	let world_space_position = m * model_space_position;

	// Vertex data in clip space (XY: -1 to 1, Z: 0 to 1)
	let clip_space_position = vp * vec4<f32>(model.position * scale + light.location, 1.0);

	// Send varying values to the fragment shader
	return VertexOutput(
		clip_space_position,
		light.color,
	);
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	return vec4<f32>(in.color, 1.0);
}
