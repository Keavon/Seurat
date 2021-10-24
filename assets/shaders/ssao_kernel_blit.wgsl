[[block]] struct Camera {
	v_matrix: mat4x4<f32>;
	p_matrix: mat4x4<f32>;
};
[[block]] struct Samples {
	samples: array<vec4<f32>, 64>;
};

// Uniforms
[[group(0), binding(0)]] var<uniform> camera: Camera;
[[group(0), binding(1)]] var<uniform> samples: Samples;
[[group(0), binding(2)]] var t_noise: texture_2d<f32>;
[[group(0), binding(3)]] var s_noise: sampler;
[[group(0), binding(4)]] var t_view_space_fragment_location: texture_2d<f32>;
[[group(0), binding(5)]] var s_view_space_fragment_location: sampler;
[[group(0), binding(6)]] var t_view_space_normal: texture_2d<f32>;
[[group(0), binding(7)]] var s_view_space_normal: sampler;

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
	[[location(0)]] tex_coords: vec2<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput) -> VertexOutput {
	return VertexOutput(vec4<f32>(model.position, 1.), model.position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5));
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let KERNEL_SIZE = 32u;
	let RADIUS = 0.5;
	let BIAS = 0.01;

	let noise_scale = vec2<f32>(textureDimensions(t_view_space_fragment_location)) / vec2<f32>(textureDimensions(t_noise));

	let noise = textureSample(t_noise, s_noise, in.tex_coords * noise_scale).xyz;
	let view_space_fragment_location = textureSample(t_view_space_fragment_location, s_view_space_fragment_location, in.tex_coords).xyz;
	let view_space_normal = normalize(textureSample(t_view_space_normal, s_view_space_normal, in.tex_coords).xyz);

	let tangent = normalize(noise - view_space_normal * dot(noise, view_space_normal));
	let bitangent = cross(view_space_normal, tangent);
	let TBN = mat3x3<f32>(tangent, bitangent, view_space_normal);

	var occlusion = 0.0;
	for (var i = 0u; i < KERNEL_SIZE; i = i + 1u) {
		// get sample position
		var sample_position = TBN * samples.samples[i].xyz; // from tangent to view-space
		sample_position = view_space_fragment_location + sample_position * RADIUS;
		
		// Transform from view space to clip space
		var offset = vec4<f32>(sample_position, 1.);
		offset = camera.p_matrix * offset; // From view to clip-space
		offset = offset / offset.w; // Perspective divide
		offset.y = -offset.y;
		offset = offset * 0.5 + 0.5; // Transform from NDC to range 0.0 - 1.0 for texture lookup

		let sample_depth = textureSample(t_view_space_fragment_location, s_view_space_fragment_location, offset.xy).z;
		let range_check = smoothStep(0.0, 1.0, RADIUS / abs(view_space_fragment_location.z - sample_depth));
		if (sample_depth >= sample_position.z + BIAS) {
			occlusion = occlusion + range_check;
		}
	}
	occlusion = 1. - (occlusion / f32(KERNEL_SIZE));

	return vec4<f32>(occlusion, occlusion, occlusion, 1.);
	// return vec4<f32>(view_space_fragment_location.xyz, 1.);
}
