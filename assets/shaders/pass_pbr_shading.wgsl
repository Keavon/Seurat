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
[[group(2), binding(0)]] var t_world_space_fragment_location: texture_2d<f32>;
[[group(2), binding(1)]] var s_world_space_fragment_location: sampler;
[[group(2), binding(2)]] var t_world_space_normal: texture_2d<f32>;
[[group(2), binding(3)]] var s_world_space_normal: sampler;
[[group(2), binding(4)]] var t_albedo_map: texture_2d<f32>;
[[group(2), binding(5)]] var s_albedo_map: sampler;
[[group(2), binding(6)]] var t_arm_map: texture_2d<f32>;
[[group(2), binding(7)]] var s_arm_map: sampler;
[[group(2), binding(8)]] var t_ssao: texture_2d<f32>;
[[group(2), binding(9)]] var s_ssao: sampler;

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
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	// Texture lookup
	let fragment_location = textureSample(t_world_space_fragment_location, s_world_space_fragment_location, in.uv).xyz;
	let normal = textureSample(t_world_space_normal, s_world_space_normal, in.uv).xyz;
	let albedo_map = textureSample(t_albedo_map, s_albedo_map, in.uv);
	let arm_map = textureSample(t_arm_map, s_arm_map, in.uv);
	let ssao = textureSample(t_ssao, s_ssao, in.uv).r;

	// PBR input data
	let eye_location = camera.v_matrix[3].xyz;
	let light_location = light.location;
	let albedo = pow(albedo_map.rgb, vec3<f32>(2.2));
	let alpha = albedo_map.a;
	let ambient = vec3<f32>(0.05);
	let ao = (1. - arm_map.x);
	let roughness = arm_map.y;
	let metallic = arm_map.z;
	let light_color = vec3<f32>(5.);

	// Lights
	let lights_count = 1u;
	var light_locations = array<vec3<f32>, 1>(light_location);
	var light_colors = array<vec3<f32>, 1>(light_color);

	// Per-fragment unit vectors
	let v = normalize(fragment_location - eye_location);
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
	let ambient_removal = ao * ssao;
	let ambient_component = albedo * ambient * pow(ambient_removal, 3.);

	color = color + ambient_component;
	return vec4<f32>(color, 1.);
}
