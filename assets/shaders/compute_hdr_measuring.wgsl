// Used as both input and output for convenience
// [[block]]
// struct InputOutputData {
// 	values: [[stride(4)]] array<i32>;
// };

// [[block]]
// struct EvenOrOdd {
// 	not_even: bool;
// };

// Uniforms
[[group(0), binding(0)]] var t_frame: texture_2d<f32>;
[[group(0), binding(1)]] var s_frame: sampler;
// [[group(0), binding(2)]]
// var<storage, read_write> data: InputOutputData;
// [[group(0), binding(3)]]
// var<storage, read_write> state: EvenOrOdd;

[[stage(compute), workgroup_size(256)]]
fn main([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
	// var color = textureSample(t_frame, s_frame, vec2<f32>(0., 0.)).rgb;

	// var i = invocation_id.x * 2u;

	// if (state.not_even) {
	// 	i = i + 1u;

	// 	// When odd: first is 1, last is length-2 (skip length-1 and larger)
	// 	// When even: first is 0, last is length-1 (skip length and larger)
	// 	if (i >= arrayLength(&data.values) - 1u) {
	// 		return;
	// 	}
	// }

	// let a = data.values[i];
	// let b = data.values[i + 1u];

	// if (a > b) {
	// 	data.values[i] = b;
	// 	data.values[i + 1u] = a;
	// }
}
