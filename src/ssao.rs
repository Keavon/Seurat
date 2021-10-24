use cgmath::{InnerSpace, Vector3};
use half::f16;
use rand::Rng;

pub fn generate_noise_texture() -> Vec<[f16; 4]> {
	let mut rng = rand::thread_rng();

	(0..16)
		.map(|_| {
			let (x, y): (f32, f32) = (rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0));

			[f16::from_f32(x * 2. - 1.), f16::from_f32(y * 2. - 1.), f16::from_f32(0.), f16::from_f32(0.)]
		})
		.collect::<Vec<_>>()
}

pub fn generate_sample_hemisphere() -> Vec<[f32; 4]> {
	let mut rng = rand::thread_rng();

	(0..64)
		.map(|i| {
			let (x, y, z, length): (f32, f32, f32, f32) = (rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0));
			let mut sample = Vector3::new(x * 2. - 1., y * 2. - 1., z).normalize() * length;

			// Weighted distribution closer to the center
			let scale = i as f32 / 64.;
			let scale = lerp(0.1, 1., scale * scale);
			sample *= scale;

			[sample.x, sample.y, sample.z, 0.]
		})
		.collect::<Vec<_>>()
}

fn lerp(a: f32, b: f32, factor: f32) -> f32 {
	a + (b - a) * factor
}
