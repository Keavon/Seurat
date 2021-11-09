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
[[group(2), binding(2)]] var t_arm: texture_2d<f32>;
[[group(2), binding(3)]] var s_arm: sampler;
[[group(2), binding(4)]] var t_normal: texture_2d<f32>;
[[group(2), binding(5)]] var s_normal: sampler;
[[group(2), binding(6)]] var t_voxel_lightmap: texture_3d<f32>;
[[group(2), binding(7)]] var s_voxel_lightmap: sampler;

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
	[[location(1)]] world_space_normal: vec3<f32>;
	[[location(2)]] world_space_tangent: vec3<f32>;
	[[location(3)]] uv: vec2<f32>;
};

// Frames
struct FragmentOutput {
	[[location(0)]] world_space_fragment_location: vec4<f32>;
	[[location(1)]] world_space_normal: vec4<f32>;
	[[location(2)]] world_space_albedo: vec4<f32>;
	[[location(3)]] world_space_arm: vec4<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	// MVP matrices
	let m = mat4x4<f32>(instance.m_matrix_0, instance.m_matrix_1, instance.m_matrix_2, instance.m_matrix_3);
	let vp = camera.p_matrix * camera.v_matrix;

	// Vertex data in world space
	let world_space_fragment_location = m * vec4<f32>(model.position, 1.0);
	let world_space_normal = m * vec4<f32>(model.normal, 0.0);
	let world_space_tangent = m * vec4<f32>(model.tangent, 0.0);

	// Vertex data in clip space (XY: -1 to 1, Z: 0 to 1)
	let clip_space_fragment_location = vp * world_space_fragment_location;

	// Send varying values to the fragment shader
	return VertexOutput(
		clip_space_fragment_location,
		world_space_fragment_location.xyz,
		world_space_normal.xyz,
		world_space_tangent.xyz,
		model.uv,
	);
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> FragmentOutput {
	let NORMAL_MAP_STRENGTH = 1.0;

	let uv = vec2<f32>(in.uv.x, 1. - in.uv.y);

	// Normal
	var world_space_normal = normalize(in.world_space_normal);

	// Tangent
	var world_space_tangent = normalize(in.world_space_tangent);
	world_space_tangent = normalize(world_space_tangent - dot(world_space_tangent, world_space_normal) * world_space_normal);

	// Bitangent
	let world_space_bitangent = cross(world_space_normal, world_space_tangent);

	// Normal map
	let from_tangent_space = mat3x3<f32>(world_space_tangent, world_space_bitangent, world_space_normal);
	var tangent_space_normal = textureSample(t_normal, s_normal, uv).xyz * 2. - 1.;
	world_space_normal = from_tangent_space * normalize(mix(vec3<f32>(0., 1., 0.), tangent_space_normal, NORMAL_MAP_STRENGTH));

	let world_position = in.world_space_fragment_location;
	let scene_offset = vec3<f32>(0., 5., 0.);
	let scene_dimensions = vec3<f32>(30., 14., 20.);
	let half_scene_dimensions = scene_dimensions * 0.5;
	var normalized_position = (world_position - scene_offset) / half_scene_dimensions; // -1 to 1
	normalized_position = normalized_position + vec3<f32>(1.); // 0 to 2
	normalized_position = normalized_position * 0.5; // 0 to 1
	let lightmap_sample = textureSample(t_voxel_lightmap, s_voxel_lightmap, normalized_position);

	return FragmentOutput(
		vec4<f32>(in.world_space_fragment_location, 1.),
		vec4<f32>(world_space_normal, 1.),
		lightmap_sample,
		// textureSample(t_albedo, s_albedo, uv).rgba,
		textureSample(t_arm, s_arm, uv).rgba,
	);
}
