use crate::texture::Texture;

pub struct VoxelTexture {
	pub texture: Texture,
	pub dimensions: (u32, u32, u32),
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
			mipmap_filter: wgpu::FilterMode::Linear,
			compare,
			lod_min_clamp: -100.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self {
			texture: Texture { texture, view, sampler, format, size },
			dimensions,
			format,
			label: String::from(label),
			compare,
		}
	}

	pub fn recreate(&mut self, device: &wgpu::Device) {
		self.texture = Self::new(device, self.dimensions, self.format, self.label.as_str(), self.compare).texture;
	}
}
