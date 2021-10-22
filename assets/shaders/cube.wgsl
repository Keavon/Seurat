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
[[group(2), binding(0)]] var t_diffuse: texture_2d<f32>;
[[group(2), binding(1)]] var s_diffuse: sampler;
[[group(2), binding(2)]] var t_normal: texture_2d<f32>;
[[group(2), binding(3)]] var s_normal: sampler;

// Attributes
struct VertexInput {
	[[location(0)]] position: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] normal: vec3<f32>;
	[[location(3)]] tangent: vec3<f32>;
};
struct InstanceInput {
	[[location(4)]] model_matrix_0: vec4<f32>;
	[[location(5)]] model_matrix_1: vec4<f32>;
	[[location(6)]] model_matrix_2: vec4<f32>;
	[[location(7)]] model_matrix_3: vec4<f32>;
};

// Varyings
struct VertexOutput {
	[[builtin(position)]] clip_space_position: vec4<f32>;
	[[location(0)]] world_space_fragment_location: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] tangent_space_fragment_location: vec3<f32>;
	[[location(3)]] tangent_space_eye_location: vec3<f32>;
	[[location(4)]] tangent_space_light_location: vec3<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	// MVP matrices
	let m = mat4x4<f32>(instance.model_matrix_0, instance.model_matrix_1, instance.model_matrix_2, instance.model_matrix_3);
	let v = camera.v_matrix;
	let p = camera.p_matrix;
	let vp = p * v;

	// Locations
	let eye_location = v[3].xyz;
	let light_location = light.location;
	let uv = model.uv;

	// Vertex data in model space
	let model_space_position = vec4<f32>(model.position, 1.0);
	let model_space_normal = vec4<f32>(model.normal, 0.0);
	let model_space_tangent = vec4<f32>(model.tangent, 0.0);

	// Vertex data in world space
	let world_space_fragment_location = m * model_space_position;
	let world_space_normal = normalize((m * model_space_normal).xyz);
	var world_space_tangent = normalize((m * model_space_tangent).xyz);
	world_space_tangent = normalize(world_space_tangent - dot(world_space_tangent, world_space_normal) * world_space_normal);
	let world_space_bitangent = cross(world_space_normal, world_space_tangent);

	// Vertex data in clip space (XY: -1 to 1, Z: 0 to 1)
	let clip_space_position = vp * world_space_fragment_location;

	// Location data in tangent-relative world space (required by normal maps)
	let to_tangent_space = transpose(mat3x3<f32>(world_space_tangent, world_space_bitangent, world_space_normal));
	let tangent_space_fragment_location = to_tangent_space * world_space_fragment_location.xyz;
	let tangent_space_eye_location = to_tangent_space * eye_location;
	let tangent_space_light_location = to_tangent_space * light_location;

	// Send varying values to the fragment shader
	return VertexOutput(
		clip_space_position,
		world_space_fragment_location.xyz,
		uv,
		tangent_space_fragment_location,
		tangent_space_eye_location,
		tangent_space_light_location,
	);
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	// Texture lookup
	let albedo: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.uv);
	let normal: vec3<f32> = textureSample(t_normal, s_normal, in.uv).xyz * 2. - 1.;

	let fragment_to_light_vector = normalize(in.tangent_space_light_location - in.tangent_space_fragment_location);
	let fragment_to_eye_vector = normalize(in.tangent_space_eye_location - in.tangent_space_fragment_location);
	let half_vector = normalize(fragment_to_eye_vector + fragment_to_light_vector);

	// Inverse square light falloff
	let distance = length(light.location - in.world_space_fragment_location.xyz);
	let falloff = 1.0 / (1.0 + 0.09 * distance + 0.032 * (distance * distance));

	// Ambient
	let ambient_strength = 0.1;
	let ambient_color = light.color * ambient_strength;

	// Diffuse
	let diffuse_strength = max(dot(normal, fragment_to_light_vector), 0.0);
	let diffuse_color = light.color * diffuse_strength;

	// Specular
	let specular_strength = pow(max(dot(normal, half_vector), 0.0), 4.0);
	let specular_color = specular_strength * light.color;

	// Result
	let result = (ambient_color + diffuse_color + specular_color) * falloff * albedo.rgb;
	return vec4<f32>(result, albedo.a);
}
