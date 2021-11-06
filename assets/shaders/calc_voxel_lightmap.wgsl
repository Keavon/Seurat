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
[[group(2), binding(0)]] var t_albedo: texture_2d<f32>;
[[group(2), binding(1)]] var s_albedo: sampler;

[[group(2), binding(2)]] var t_voxel_lightmap: texture_3d<f32>;
[[group(2), binding(3)]] var s_voxel_lightmap: sampler;

// Attributes
struct VertexInput {
	[[location(0)]] position: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] normal: vec3<f32>;
	[[location(3)]] tangent: vec3<f32>;
};
struct InstanceInput {
	[[location(4)]] m_matrix_0: vec4<f32>;
	[[location(5)]] m_matrix_1: vec4<f32>;
	[[location(6)]] m_matrix_2: vec4<f32>;
	[[location(7)]] m_matrix_3: vec4<f32>;
};

// Varyings
struct VertexOutput {
	[[builtin(position)]] clip_space_fragment_location: vec4<f32>;
	[[location(0)]] world_space_fragment_location: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
};

// Frames
struct FragmentOutput {
	[[location(0)]] world_space_fragment_location: vec4<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	// MVP matrices
	let m = mat4x4<f32>(instance.m_matrix_0, instance.m_matrix_1, instance.m_matrix_2, instance.m_matrix_3);
	let vp = camera.p_matrix * camera.v_matrix;

	// Vertex data in world space
	let world_space_fragment_location = m * vec4<f32>(model.position, 1.0);

	// Vertex data in clip space (XY: -1 to 1, Z: 0 to 1)
	let clip_space_fragment_location = vp * world_space_fragment_location;

	// Send varying values to the fragment shader
	return VertexOutput(
		clip_space_fragment_location,
		world_space_fragment_location.xyz,
		model.uv,
	);
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> FragmentOutput {
	let uv = vec2<f32>(in.uv.x, 1. - in.uv.y);

	let pos = in.world_space_fragment_location;
	let color = textureSample(t_albedo, s_albedo, uv).rgba;

	return FragmentOutput(
		vec4<f32>(in.world_space_fragment_location, 1.),
	);
}
