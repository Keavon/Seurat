// Used as both input and output for convenience
[[block]]
struct InputOutputData {
	values: [[stride(4)]] array<i32>;
};

[[block]]
struct EvenOrOdd {
	not_even: bool;
};

[[group(0), binding(0)]]
var<storage, read_write> data: InputOutputData;
[[group(0), binding(1)]]
var<storage, read_write> state: EvenOrOdd;

[[stage(compute), workgroup_size(256)]]
fn main([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
	var i = invocation_id.x * 2u;

	if (state.not_even) {
		i = i + 1u;

		// When odd: first is 1, last is length-2 (skip length-1 and larger)
		// When even: first is 0, last is length-1 (skip length and larger)
		if (i >= arrayLength(&data.values) - 1u) {
			return;
		}
	}

	let a = data.values[i];
	let b = data.values[i + 1u];

	if (a > b) {
		data.values[i] = b;
		data.values[i + 1u] = a;
	}
}
