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
	[[location(4)]] bitangent: vec3<f32>;
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
	[[location(0)]] world_position: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] tangent_position: vec3<f32>;
	[[location(3)]] tangent_light_position: vec3<f32>;
	[[location(4)]] tangent_view_position: vec3<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	// Reconstruct matrices from attribute vector components
	let model_matrix = mat4x4<f32>(
		instance.model_matrix_0,
		instance.model_matrix_1,
		instance.model_matrix_2,
		instance.model_matrix_3,
	);
	let normal_matrix = mat3x3<f32>(
		instance.normal_matrix_0,
		instance.normal_matrix_1,
		instance.normal_matrix_2,
	);

	let world_normal = normalize(normal_matrix * model.normal);
	let world_tangent = normalize(normal_matrix * model.tangent);
	let world_bitangent = normalize(normal_matrix * model.bitangent);
	let tangent_matrix = transpose(mat3x3<f32>(world_tangent, world_bitangent, world_normal));

	let world_position = model_matrix * vec4<f32>(model.position, 1.0); // model_matrix is M

	var out: VertexOutput;

	out.clip_position = camera_uniform.view_proj * world_position; // view_proj is P
	out.world_position = world_position.xyz;
	out.uv = model.uv;

	// Positions in tangent-space (compatible with normal maps)
	out.tangent_position = tangent_matrix * world_position.xyz;
	out.tangent_view_position = tangent_matrix * camera_uniform.view_position.xyz; // view_position is V if it were turned into a matrix
	out.tangent_light_position = tangent_matrix * light.position;

	return out;
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.uv);
	let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.uv);
	let tangent_normal = object_normal.xyz * 2.0 - 1.0;

	let light_dir = normalize(in.tangent_light_position - in.tangent_position);
	let view_dir = normalize(in.tangent_view_position - in.tangent_position);
	let half_dir = normalize(view_dir + light_dir);

	let distance = length(light.position - in.world_position.xyz);
	let attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * (distance * distance));

	// Ambient
	let ambient_strength = 0.1;
	let ambient_color = light.color * ambient_strength;

	// Diffuse
	let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
	let diffuse_color = light.color * diffuse_strength;

	// Specular
	let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 4.0);
	let specular_color = specular_strength * light.color;

	// Result
	let result = (ambient_color + diffuse_color + specular_color) * attenuation * object_color.rgb;
	return vec4<f32>(result, object_color.a);
}
