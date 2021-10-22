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
	[[location(1)]] world_space_normal: vec3<f32>;
	[[location(2)]] uv: vec2<f32>;
	[[location(3)]] tangent_space_fragment_location: vec3<f32>;
	[[location(4)]] tangent_space_eye_location: vec3<f32>;
	[[location(5)]] tangent_space_light_location: vec3<f32>;
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
		world_space_normal,
		uv,
		tangent_space_fragment_location,
		tangent_space_eye_location,
		tangent_space_light_location,
	);
}

fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
	return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

fn DistributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
	let a = roughness * roughness;
	let a2 = a * a;

	let NdotH = max(dot(N, H), 0.0);
	let NdotH2 = NdotH * NdotH;

	let num = a2;
	let denom = (NdotH2 * (a2 - 1.0) + 1.0);
	let pi_denom_squared = PI * denom * denom;

	return num / pi_denom_squared;
}

fn GeometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
	let r = (roughness + 1.0);
	let k = (r * r) / 8.0;

	let num = NdotV;
	let denom = NdotV * (1.0 - k) + k;

	return num / denom;
}

fn GeometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
	let NdotV = max(dot(N, V), 0.0);
	let NdotL = max(dot(N, L), 0.0);

	let ggx2 = GeometrySchlickGGX(NdotV, roughness);
	let ggx1 = GeometrySchlickGGX(NdotL, roughness);

	return ggx1 * ggx2;
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	// Texture lookup
	let albedo_map: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.uv);
	let normal_map: vec3<f32> = textureSample(t_normal, s_normal, in.uv).xyz;

	// PBR input data
	let albedo = pow(albedo_map.rgb, vec3<f32>(2.2));
	let alpha = albedo_map.a;
	let normal = normal_map * 2. - 1.;
	let ambient = vec3<f32>(0.00);
	let metallic = 0.0;
	let roughness = 0.9;

	// let fragment_to_light_vector = normalize(in.tangent_space_light_location - in.tangent_space_fragment_location);
	// let fragment_to_eye_vector = normalize(in.tangent_space_eye_location - in.tangent_space_fragment_location);
	// let half_vector = normalize(fragment_to_eye_vector + fragment_to_light_vector);

	/////////////////////////

	// TODO: camelCase/PascalCase to snake_case and more descriptive naming

	// Lights
	let lights_count = 1u;
	var light_locations = array<vec3<f32>, 1>(light.location);
	var light_colors = array<vec3<f32>, 1>(light.color);

	let world_space_eye_location = camera.v_matrix[3].xyz;
	let V = normalize(world_space_eye_location - in.world_space_fragment_location);
	let N = normalize(in.world_space_normal);
	let WorldPos = in.world_space_fragment_location;

	var Lo = vec3<f32>(0.0);
	for (var i: u32 = 0u; i < lights_count; i = i + 1u) {
		let LightPos = light_locations[i];

		let L = normalize(LightPos - WorldPos);
		let H = normalize(V + L);

		let distance = length(LightPos - WorldPos);
		let falloff = 1.0 / (distance * distance);
		let radiance = light_colors[i] * falloff;

		let good_dielectric_f0 = vec3<f32>(0.04);
		let F0 = mix(good_dielectric_f0, albedo, metallic);
		let F = fresnelSchlick(max(dot(H, V), 0.0), F0);

		let NDF = DistributionGGX(N, H, roughness);
		let G = GeometrySmith(N, V, L, roughness);

		let numerator = NDF * G * F;
		let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
		let specular = numerator / denominator;

		let kS = F;
		let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic); // Nullify diffuse when surface is metallic

		let NdotL = max(dot(N, L), 0.0);
		Lo = Lo + (kD * albedo / PI + specular) * radiance * NdotL;
	}

	let kA = ambient * albedo;// * ao;
	var color = kA + Lo;

	// Tone mapping
	color = color / (color + vec3<f32>(1.));

	// Gamma correction (linear to gamma)
	color = pow(color, vec3<f32>(1. / 2.2));
	return vec4<f32>(color, alpha);

	/////////////////////////

	// Inverse square light falloff
	// let distance = length(light.location - in.world_space_fragment_location.xyz);
	// let falloff = 1.0 / (1.0 + 0.09 * distance + 0.032 * (distance * distance));

	// // Ambient
	// let ambient_strength = 0.1;
	// let ambient_color = light.color * ambient_strength;

	// // Diffuse
	// let diffuse_strength = max(dot(normal, fragment_to_light_vector), 0.0);
	// let diffuse_color = light.color * diffuse_strength;

	// // Specular
	// let specular_strength = pow(max(dot(normal, half_vector), 0.0), 4.0);
	// let specular_color = specular_strength * light.color;

	// // Result
	// let result = (ambient_color + diffuse_color + specular_color) * falloff * albedo;
	// return vec4<f32>(result, alpha);
}
