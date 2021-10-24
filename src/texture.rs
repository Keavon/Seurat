use anyhow::Result;
use half::f16;
use image::GenericImageView;
use std::path::Path;

pub struct Texture {
	pub texture: wgpu::Texture,
	pub view: wgpu::TextureView,
	pub sampler: wgpu::Sampler,
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

	pub fn from_rgba_data(device: &wgpu::Device, queue: &wgpu::Queue, rgba_data: &[u8], dimensions: (u32, u32), label: &str, format: wgpu::TextureFormat, repeat_mode: wgpu::AddressMode) -> Self {
		let size = wgpu::Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: 1,
		};
		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
			size,
		);

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: repeat_mode,
			address_mode_v: repeat_mode,
			address_mode_w: repeat_mode,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});

		Self { texture, view, sampler }
	}
}
