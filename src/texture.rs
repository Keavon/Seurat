use anyhow::Result;
use half::f16;
use image::GenericImageView;
use std::{borrow::Cow, path::Path};

use crate::context::Context;

pub struct Texture {
	pub texture: wgpu::Texture,
	pub view: wgpu::TextureView,
	pub sampler: wgpu::Sampler,
	pub format: wgpu::TextureFormat,
	pub size: wgpu::Extent3d,
}

impl Texture {
	pub fn load(device: &wgpu::Device, queue: &wgpu::Queue, directory: &Path, file: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Result<Self> {
		let path = directory.join("models").join(file);
		let image = image::open(path.clone())?;

		Ok(Self::from_image(device, queue, &image, path.to_str().unwrap_or_default(), format, repeat_mode))
	}

	pub fn from_image_bytes(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], label: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Result<Self> {
		let img = image::load_from_memory(bytes)?;
		Ok(Self::from_image(device, queue, &img, label, format, repeat_mode))
	}

	pub fn from_f16_array(device: &wgpu::Device, queue: &wgpu::Queue, rgba: &[[f16; 4]], dimensions: (u32, u32), label: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Self {
		let rgba_data: &[u8] = bytemuck::cast_slice(rgba);

		Self::from_rgba_data(device, queue, rgba_data, dimensions, label, format, repeat_mode)
	}

	pub fn from_image(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::DynamicImage, label: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Self {
		let rgba: &[u8] = &img.to_rgba8();
		let dimensions = img.dimensions();

		Self::from_rgba_data(device, queue, rgba, dimensions, label, format, repeat_mode)
	}

	pub fn from_dimensions(device: &wgpu::Device, dimensions: (u32, u32), label: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Self {
		let size = wgpu::Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: 1,
		};

		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: size.max_mips(),
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
		});

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: repeat_mode,
			address_mode_v: repeat_mode,
			address_mode_w: repeat_mode,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});

		Self { texture, view, sampler, format, size }
	}

	pub fn from_rgba_data(device: &wgpu::Device, queue: &wgpu::Queue, rgba_data: &[u8], dimensions: (u32, u32), label: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Self {
		let size = wgpu::Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: 1,
		};

		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: size.max_mips(),
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
		});

		let color_bytes_per_channel = rgba_data.len() as u32 / (dimensions.0 * dimensions.1) / 4;
		queue.write_texture(
			wgpu::ImageCopyTexture {
				aspect: wgpu::TextureAspect::All,
				texture: &texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
			},
			rgba_data,
			wgpu::ImageDataLayout {
				offset: 0,
				bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0 * color_bytes_per_channel),
				rows_per_image: std::num::NonZeroU32::new(dimensions.1),
			},
			size.clone(),
		);

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: repeat_mode,
			address_mode_v: repeat_mode,
			address_mode_w: repeat_mode,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});

		Self { texture, view, sampler, format, size }
	}

	pub fn generate_mipmaps(&mut self, context: &Context) {
		let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let shader = context.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/shaders/utility_mipmap_blit.wgsl"))),
		});

		let pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("blit"),
			layout: None,
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",
				buffers: &[],
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: "fs_main",
				targets: &[self.format.into()],
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleStrip,
				..Default::default()
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
		});

		let bind_group_layout = pipeline.get_bind_group_layout(0);

		let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
			label: Some("mip"),
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});

		let mip_level_count = self.size.max_mips();

		let views = (0..mip_level_count)
			.map(|mip| {
				self.texture.create_view(&wgpu::TextureViewDescriptor {
					label: Some("mip"),
					format: None,
					dimension: None,
					aspect: wgpu::TextureAspect::All,
					base_mip_level: mip,
					mip_level_count: std::num::NonZeroU32::new(1),
					base_array_layer: 0,
					array_layer_count: None,
				})
			})
			.collect::<Vec<_>>();

		for target_mip in 1..mip_level_count as usize {
			let bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
				layout: &bind_group_layout,
				entries: &[
					wgpu::BindGroupEntry {
						binding: 0,
						resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
					},
					wgpu::BindGroupEntry {
						binding: 1,
						resource: wgpu::BindingResource::Sampler(&sampler),
					},
				],
				label: None,
			});

			let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: None,
				color_attachments: &[wgpu::RenderPassColorAttachment {
					view: &views[target_mip],
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
						store: true,
					},
				}],
				depth_stencil_attachment: None,
			});
			rpass.set_pipeline(&pipeline);
			rpass.set_bind_group(0, &bind_group, &[]);
			rpass.draw(0..4, 0..1);
		}

		context.queue.submit(Some(encoder.finish()));
	}
}
