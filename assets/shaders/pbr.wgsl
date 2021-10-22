let PI: f32 = 3.14159265359;

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
	[[builtin(position)]] clip_space_fragment_location: vec4<f32>;
	[[location(0)]] world_space_fragment_location: vec3<f32>;
	[[location(1)]] world_space_normal: vec3<f32>;
	[[location(2)]] uv: vec2<f32>;
	[[location(3)]] tangent_space_fragment_location: vec3<f32>;
	[[location(4)]] tangent_space_eye_location: vec3<f32>;
	[[location(5)]] tangent_space_light_location: vec3<f32>;
};

// Frames
struct FragmentOutput {
	[[location(0)]] surface: vec4<f32>;
	[[location(1)]] albedo: vec4<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
	// MVP matrices
	let m = mat4x4<f32>(instance.model_matrix_0, instance.model_matrix_1, instance.model_matrix_2, instance.model_matrix_3);
	let v = camera.v_matrix;
	let p = camera.p_matrix;
	let vp = p * v;

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
	let clip_space_fragment_location = vp * world_space_fragment_location;

	// Location data in world space
	let world_space_eye_location = v[3].xyz;
	let world_space_light_location = light.location;

	// Location data in tangent-relative world space (required by normal maps)
	let to_tangent_space = transpose(mat3x3<f32>(world_space_tangent, world_space_bitangent, world_space_normal));
	let tangent_space_fragment_location = to_tangent_space * world_space_fragment_location.xyz;
	let tangent_space_eye_location = to_tangent_space * world_space_eye_location;
	let tangent_space_light_location = to_tangent_space * world_space_light_location;

	// Send varying values to the fragment shader
	return VertexOutput(
		clip_space_fragment_location,
		world_space_fragment_location.xyz,
		world_space_normal,
		model.uv,
		tangent_space_fragment_location,
		tangent_space_eye_location,
		tangent_space_light_location,
	);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
	return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
	let a = roughness * roughness;
	let a2 = a * a;

	let n_dot_h = max(dot(n, h), 0.0);
	let n_dot_h2 = n_dot_h * n_dot_h;

	let num = a2;
	let denom = (n_dot_h2 * (a2 - 1.0) + 1.0);
	let pi_denom_squared = PI * denom * denom;

	return num / pi_denom_squared;
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
	let r = (roughness + 1.0);
	let k = (r * r) / 8.0;

	let num = n_dot_v;
	let denom = n_dot_v * (1.0 - k) + k;

	return num / denom;
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
	let n_dot_v = max(dot(n, v), 0.0);
	let n_dot_l = max(dot(n, l), 0.0);

	let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
	let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);

	return ggx1 * ggx2;
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> FragmentOutput {
	// Texture lookup
	let albedo_map: vec4<f32> = textureSample(t_albedo, s_albedo, in.uv);
	let ao_roughness_metalness_map: vec4<f32> = textureSample(t_arm, s_arm, in.uv);
	var normal_map: vec3<f32> = textureSample(t_normal, s_normal, in.uv).xyz;

	// PBR input data
	let albedo = pow(albedo_map.rgb, vec3<f32>(2.2));
	let alpha = albedo_map.a;
	let normal_map_strength = 1.;
	let normal = mix(vec3<f32>(0., 0., 1.), normal_map * 2. - 1., normal_map_strength);
	let ambient = vec3<f32>(0.03);
	let ao = ao_roughness_metalness_map.x;
	let roughness = ao_roughness_metalness_map.y;
	let metallic = ao_roughness_metalness_map.z;

	// Locations
	// let world_space_fragment_location = in.world_space_fragment_location;
	// let world_space_eye_location = camera.v_matrix[3].xyz;
	// let world_space_light_location = light.location;
	// let world_space_normal = in.world_space_normal;

	let fragment_location = in.tangent_space_fragment_location;
	let eye_location = in.tangent_space_eye_location;
	let light_location = in.tangent_space_light_location;

	// Lights
	let lights_count = 1u;
	var light_locations = array<vec3<f32>, 1>(light_location);
	var light_colors = array<vec3<f32>, 1>(light.color);

	// Per-fragment unit vectors
	let v = normalize(eye_location - fragment_location);
	let n = normalize(normal);

	var color = vec3<f32>(0.0);
	for (var i: u32 = 0u; i < lights_count; i = i + 1u) {
		let light_location = light_locations[i];

		// Per-light unit vectors
		let l = normalize(light_location - fragment_location);
		let h = normalize(v + l);

		let n_dot_l = max(dot(n, l), 0.0);

		// Radiance contribution by this light
		let distance = length(light_location - fragment_location);
		let falloff = 1.0 / (distance * distance);
		let radiance = light_colors[i] * falloff;

		// Fresnel color
		let good_dielectric_f0 = vec3<f32>(0.04);
		let f0 = mix(good_dielectric_f0, albedo, metallic);
		let f = fresnel_schlick(max(dot(h, v), 0.0), f0);

		// Normal distribution factor (specular highlight alignment of microfacets with halfway vector)
		let ndf = distribution_ggx(n, h, roughness);

		// Geometry self-occlusion factor
		let g = geometry_smith(n, v, l, roughness);

		// Specular contribution
		let specular = (f * ndf * g) / (4.0 * max(dot(n, v), 0.0) * n_dot_l + 0.0001);

		// Portion of illumination that is not specular is diffuse
		let specular_component = f;
		var diffuse_component = (vec3<f32>(1.0) - specular_component);
		diffuse_component = diffuse_component * (1.0 - metallic); // Nullify diffuse when surface is metallic

		// Diffuse contribution
		let diffuse = diffuse_component * albedo / PI;

		// Bidirectional reflectance distribution function
		let reflectance = diffuse + specular;
		let light_illumination = reflectance * radiance * n_dot_l;

		// Add this light to the fragment's sum of illumination
		color = color + light_illumination;
	}

	// Add ambient occlusion
	let ambient_component = albedo * ambient * ao * ao;
	color = color + ambient_component;

	// Tone mapping
	color = color / (color + vec3<f32>(1.));

	// Gamma correction (linear to gamma)
	color = pow(color, vec3<f32>(1. / 2.2));
	return FragmentOutput(
		vec4<f32>(color, alpha),
		vec4<f32>(albedo, alpha),
	);
}
