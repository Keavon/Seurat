use std::borrow::Cow;

use crate::{
	context::Context,
	material::{Material, MaterialDataBinding},
	scene::LoadedResources,
	texture::Texture,
};

pub struct VoxelTexture {
	pub texture: Texture,
	pub size: wgpu::Extent3d,
	pub format: wgpu::TextureFormat,
	pub label: String,
	pub compare: Option<wgpu::CompareFunction>,
}

impl VoxelTexture {
	pub fn new(device: &wgpu::Device, dimensions: (u32, u32, u32), format: wgpu::TextureFormat, label: &str, compare: Option<wgpu::CompareFunction>) -> Self {
		let size = wgpu::Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: dimensions.2,
		};
		let texture_descriptor = wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D3,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
		};
		let texture = device.create_texture(&texture_descriptor);

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			compare,
			lod_min_clamp: -100.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self {
			texture: Texture { texture, view, sampler, format, size },
			size,
			format,
			label: String::from(label),
			compare,
		}
	}

	pub fn generate_mipmaps(&mut self, resources: &LoadedResources, context: &Context) {
		// let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		// let shader = context.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
		// 	label: None,
		// 	source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/shaders/utility_3d_mipmap_blit.wgsl"))),
		// });

		// let pipeline = context.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
		// 	label: Some("3D Texture Mipmap Downsampler"),
		// 	layout: None,
		// 	module: &shader,
		// 	entry_point: "main",
		// });

		// let bind_group_layout = pipeline.get_bind_group_layout(0);

		// let mip_level_count = self.size.max_mips();

		// let views = (0..mip_level_count)
		// 	.map(|mip| {
		// 		self.texture.texture.create_view(&wgpu::TextureViewDescriptor {
		// 			label: Some("mip"),
		// 			format: None,
		// 			dimension: None,
		// 			aspect: wgpu::TextureAspect::All,
		// 			base_mip_level: mip,
		// 			mip_level_count: std::num::NonZeroU32::new(1),
		// 			base_array_layer: 0,
		// 			array_layer_count: None,
		// 		})
		// 	})
		// 	.collect::<Vec<_>>();

		// for target_mip in 1..mip_level_count as usize {
		// 	let bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
		// 		layout: &bind_group_layout,
		// 		entries: &[
		// 			wgpu::BindGroupEntry {
		// 				binding: 0,
		// 				resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
		// 			},
		// 			wgpu::BindGroupEntry {
		// 				binding: 1,
		// 				resource: wgpu::BindingResource::TextureView(&views[target_mip]),
		// 			},
		// 		],
		// 		label: None,
		// 	});

		// 	let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
		// 		label: Some(&format!("3D Mipmap Compute Pass: target mip {}", target_mip)),
		// 	});
		// 	cpass.set_pipeline(&pipeline);
		// 	cpass.set_bind_group(0, &bind_group, &[]);
		// 	let size = self.size.mip_level_size(target_mip.try_into().unwrap(), true);
		// 	cpass.dispatch(size.width, size.height, size.depth_or_array_layers);
		// }

		// context.queue.submit(Some(encoder.finish()));
	}
}
