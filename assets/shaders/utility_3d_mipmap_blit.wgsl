// Uniforms
[[group(0), binding(0)]] var t_in_color: texture_3d<f32>;
[[group(0), binding(1)]] var out_color: texture_storage_3d<rgba8unorm, write>;

[[stage(compute), workgroup_size(128)]]
fn main([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
	var c = vec4<f32>(0.,0.,0.,0.);
	for (var x = 0; x < 2; x=x+1) {
		for (var y = 0; y < 2; y=y+1) {
			for (var z = 0; z < 2; z=z+1) {
				c = c + textureLoad(t_in_color, vec3<i32>(2 * i32(invocation_id.x) + x, 2 * i32(invocation_id.y) + y, 2 * i32(invocation_id.z) + z), 0);
			}
		}
	}
	c = c / 8.;
	textureStore(out_color, vec3<i32>(invocation_id), c);
}
