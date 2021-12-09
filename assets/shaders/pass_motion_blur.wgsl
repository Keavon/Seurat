[[block]] struct Camera {
	v_matrix: mat4x4<f32>;
	p_matrix: mat4x4<f32>;
	inv_v_matrix: mat4x4<f32>;
	inv_p_matrix: mat4x4<f32>;
	prev_v_matrix: mat4x4<f32>;
	prev_p_matrix: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]] var<uniform> camera: Camera;
[[group(1), binding(0)]] var t_color: texture_2d<f32>;
[[group(1), binding(1)]] var s_color: sampler;
[[group(1), binding(2)]] var t_z_buffer_previous: texture_depth_2d;
[[group(1), binding(3)]] var s_z_buffer_previous: sampler;
[[group(1), binding(4)]] var t_z_buffer: texture_depth_2d;
[[group(1), binding(5)]] var s_z_buffer: sampler;

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

fn world_position_from_depth(uv: vec2<f32>, z: f32) -> vec3<f32> {
	if (z == 1.) {
		return vec3<f32>(0.);
	}

	let xy = vec2<f32>(uv.x, 1. - uv.y) * 2. - 1.;
	let clip_space_position = vec4<f32>(xy, z, 1.);

	let view_space_position = (camera.inv_v_matrix * camera.inv_p_matrix) * clip_space_position;
	// return view_space_position.xyz;
	return view_space_position.xyz / view_space_position.w;
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let BLUR_DISTANCE = 0.1;
	
	let current_frame_depth = textureSample(t_z_buffer, s_z_buffer, in.uv);
	let current_frame_world_position = world_position_from_depth(in.uv, current_frame_depth);

	let xy = vec2<f32>(in.uv.x, 1. - in.uv.y) * 2. - 1.;
	let clip_space_position = vec4<f32>(xy, current_frame_depth, 1.);

	let currentPos = clip_space_position;
	let previousPos = (camera.prev_p_matrix * camera.prev_v_matrix) * vec4<f32>(current_frame_world_position, 1.);
	let previousPos = previousPos / previousPos.w;
	let velocity = (currentPos - previousPos).xy * BLUR_DISTANCE;

	var texCoord = in.uv;
	var color = textureSample(t_color, s_color, texCoord);
	texCoord = texCoord + velocity;
	for (var i = 1u; i < 10u; i = i + 1u) {
		color = color + textureSample(t_color, s_color, texCoord);

		texCoord = texCoord + velocity;
	}

	if (current_frame_depth != 1.) {
		return vec4<f32>(color.rgb, 1.);
	}
	else {
		return vec4<f32>(0., 0., 0., 1.);
	}
}
