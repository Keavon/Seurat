use crate::texture::Texture;

pub struct FrameTexture {
	pub texture: Texture,
	pub label: String,
	pub compare: Option<wgpu::CompareFunction>,
}

impl FrameTexture {
	pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, format: wgpu::TextureFormat, label: &str, compare: Option<wgpu::CompareFunction>) -> Self {
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
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
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
			compare,
			lod_min_clamp: -100.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self {
			texture: Texture { texture, view, sampler, format, size },
			label: String::from(label),
			compare,
		}
	}

	pub fn recreate(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
		self.texture = Self::new(device, config, self.texture.format, self.label.as_str(), self.compare).texture;
	}
}

pub struct FrameTextures {
	pub z_buffer: FrameTexture,
	pub z_buffer_previous: FrameTexture,
	pub world_space_normal: FrameTexture,
	pub albedo_map: FrameTexture,
	pub arm_map: FrameTexture,
	pub ssao_kernel_map: FrameTexture,
	pub ssao_blurred_map: FrameTexture,
	pub pbr_shaded_map: FrameTexture,
	pub motion_blur_map: FrameTexture,
}

impl FrameTextures {
	pub fn recreate_all(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
		self.z_buffer.recreate(device, config);
		self.z_buffer_previous.recreate(device, config);
		self.world_space_normal.recreate(device, config);
		self.albedo_map.recreate(device, config);
		self.arm_map.recreate(device, config);
		self.ssao_kernel_map.recreate(device, config);
		self.ssao_blurred_map.recreate(device, config);
		self.pbr_shaded_map.recreate(device, config);
		self.motion_blur_map.recreate(device, config);
	}
}
