[[block]] struct Camera {
	v_matrix: mat4x4<f32>;
	p_matrix: mat4x4<f32>;
	inv_v_matrix: mat4x4<f32>;
	inv_p_matrix: mat4x4<f32>;
	prev_v_matrix: mat4x4<f32>;
	prev_p_matrix: mat4x4<f32>;
};
[[block]] struct Light {
	location: vec3<f32>;
	color: vec3<f32>;
};
struct SummedColorCell {
	r: atomic<u32>;
	g: atomic<u32>;
	b: atomic<u32>;
	count: atomic<u32>;
};
[[block]] struct SummedColors {
	cells: array<SummedColorCell>;
};

// Uniforms
[[group(0), binding(0)]] var<uniform> light: Light;
[[group(1), binding(0)]] var<uniform> camera: Camera;
[[group(1), binding(1)]] var t_albedo: texture_2d<f32>;
[[group(1), binding(2)]] var s_albedo: sampler;
[[group(1), binding(3)]] var<storage, read_write> voxel_buffer: SummedColors;

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

	var world_position = in.world_space_fragment_location;
	let color = textureSample(t_albedo, s_albedo, uv).rgba;

	let scene_offset = vec3<f32>(0., 5., 0.);
	let scene_dimensions = vec3<f32>(30., 14., 20.);
	let half_scene_dimensions = scene_dimensions * 0.5;
	var normalized_position = (world_position - scene_offset) / half_scene_dimensions; // -1 to 1
	normalized_position = normalized_position + vec3<f32>(1.); // 0 to 2
	normalized_position = normalized_position * 0.5; // 0 to 1
	let texture_dim_one = 128;
	let texture_dimensions = vec3<f32>(f32(texture_dim_one));
	var pos = vec3<i32>(normalized_position * texture_dimensions); // 0 to 256

	let buffer_index = pos.x + pos.y * texture_dim_one + pos.z * texture_dim_one * texture_dim_one;
	if (buffer_index > 0) {
		var old_value = atomicAdd(&voxel_buffer.cells[buffer_index].r, u32(color.r * 256.));
		old_value = atomicAdd(&voxel_buffer.cells[buffer_index].g, u32(color.g * 256.));
		old_value = atomicAdd(&voxel_buffer.cells[buffer_index].b, u32(color.b * 256.));
		old_value = atomicAdd(&voxel_buffer.cells[buffer_index].count, 1u);
	}

	return FragmentOutput(
		vec4<f32>(in.world_space_fragment_location, 1.),
	);
}
