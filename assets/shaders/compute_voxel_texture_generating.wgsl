struct SummedColorCell {
	r: u32;
	g: u32;
	b: u32;
	count: u32;
};
[[block]] struct SummedColors {
	cells: array<SummedColorCell>;
};

// Uniforms
[[group(0), binding(0)]] var t_voxel_lightmap: texture_storage_3d<rgba8unorm, write>;
[[group(0), binding(1)]] var<storage, read_write> voxel_buffer: SummedColors;

[[stage(compute), workgroup_size(128)]]
fn main([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
	let texture_dim = 128u;
	let buffer_index = invocation_id.x + invocation_id.y * texture_dim + invocation_id.z * texture_dim * texture_dim;
	let cell = voxel_buffer.cells[buffer_index];
	let color = (vec4<f32>(f32(cell.r), f32(cell.g), f32(cell.b), 0.) / f32(cell.count)) / 256.;
	textureStore(t_voxel_lightmap, vec3<i32>(invocation_id), color);
}
