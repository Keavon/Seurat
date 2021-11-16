let THICKNESS = 100.;
let HARDNESS = 0.;

let SAMPLES = 256;
[[block]] struct TraceData {
	points: array<f32, 256>;
};

// Uniforms
// [[group(0), binding(0)]] var t_frame: texture_2d<f32>;
// [[group(0), binding(1)]] var s_frame: sampler;
[[group(0), binding(0)]] var<uniform> trace: TraceData;

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

fn sdf_line_segment(point: vec2<f32>, end_a: vec2<f32>, end_b: vec2<f32>) -> f32 {
	let point_to_a = point - end_a;
	let point_to_b = end_b - end_a;
	let factor = clamp(dot(point_to_a, point_to_b) / dot(point_to_b, point_to_b), 0.0, 1.0);

	return length(point_to_a - point_to_b * factor);
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	var min_distance = 999999.;

	for (var i = 0; i < (SAMPLES - 1) * 2; i = i + 2) {
		let point_a = vec2<f32>(f32(trace.points[i]), f32(trace.points[i + 1]));
		let point_b = vec2<f32>(f32(trace.points[i + 2]), f32(trace.points[i + 3]));
		var distance = sdf_line_segment(in.position.xy, point_a, point_b); //(distance(point, in.position.xy) - THICKNESS) / (1. + FEATHER);
		let feather = THICKNESS * (100. - HARDNESS) / 100.;
		let stroke = THICKNESS - feather;
		distance = distance - stroke;
		distance = distance / (1. + feather);
		min_distance = min(min_distance, distance);
	}

	var color = vec3<f32>(min_distance);

	// Gamma correction (linear to gamma)
	// color = pow(color, vec3<f32>(1. / 2.2));

	return vec4<f32>(color, 1.);
}
