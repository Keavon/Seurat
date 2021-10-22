use crate::texture::Texture;

pub struct FrameTexture {
	pub texture: Texture,
	pub format: wgpu::TextureFormat,
	pub label: String,
}

impl FrameTexture {
	pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, format: wgpu::TextureFormat, label: &str) -> Self {
		let size = wgpu::Extent3d {
			width: config.width,
			height: config.height,
			depth_or_array_layers: 1,
		};
		let texture_descriptor = wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
		};
		let texture = device.create_texture(&texture_descriptor);

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Nearest,
			compare: Some(wgpu::CompareFunction::LessEqual),
			lod_min_clamp: -100.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self {
			texture: Texture { texture, view, sampler },
			format,
			label: String::from(label),
		}
	}

	pub fn recreate(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
		self.texture = Self::new(device, config, self.format, self.label.as_str()).texture;
	}
}

pub struct FrameTextures {
	pub z_buffer: FrameTexture,
	pub albedo: FrameTexture,
}

impl FrameTextures {
	pub fn recreate_all(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
		self.z_buffer.recreate(device, config);
		self.albedo.recreate(device, config);
	}
}
