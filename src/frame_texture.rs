use crate::texture::Texture;

pub struct FrameTexture {
	pub texture: Texture,
	pub format: wgpu::TextureFormat,
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
			compare,
			lod_min_clamp: -100.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self {
			texture: Texture { texture, view, sampler },
			format,
			label: String::from(label),
			compare,
		}
	}

	pub fn recreate(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
		self.texture = Self::new(device, config, self.format, self.label.as_str(), self.compare).texture;
	}
}

pub struct FrameTextures {
	pub z_buffer: FrameTexture,
	pub world_space_fragment_location: FrameTexture,
	pub world_space_normal: FrameTexture,
	pub world_space_eye_location: FrameTexture,
	pub world_space_light_location: FrameTexture,
	pub view_space_fragment_location: FrameTexture,
	pub view_space_normal: FrameTexture,
	pub view_space_eye_location: FrameTexture,
	pub view_space_light_location: FrameTexture,
	pub albedo_map: FrameTexture,
	pub arm_map: FrameTexture,
	pub normal_map: FrameTexture,
	pub ssao_kernel_map: FrameTexture,
	pub ssao_blurred_map: FrameTexture,
	// UPDATE HERE TO ADD FRAME TEXTURE
}

impl FrameTextures {
	pub fn recreate_all(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
		self.z_buffer.recreate(device, config);
		self.world_space_fragment_location.recreate(device, config);
		self.world_space_normal.recreate(device, config);
		self.world_space_eye_location.recreate(device, config);
		self.world_space_light_location.recreate(device, config);
		self.view_space_fragment_location.recreate(device, config);
		self.view_space_normal.recreate(device, config);
		self.view_space_eye_location.recreate(device, config);
		self.view_space_light_location.recreate(device, config);
		self.albedo_map.recreate(device, config);
		self.arm_map.recreate(device, config);
		self.normal_map.recreate(device, config);
		self.ssao_kernel_map.recreate(device, config);
		self.ssao_blurred_map.recreate(device, config);
		// UPDATE HERE TO ADD FRAME TEXTURE
	}
}

#[derive(Copy, Clone)]
pub enum FrameTextureTypes {
	Surface,
	ZBuffer,
	WorldSpaceFragmentLocation,
	WorldSpaceNormal,
	WorldSpaceEyeLocation,
	WorldSpaceLightLocation,
	ViewSpaceFragmentLocation,
	ViewSpaceNormal,
	ViewSpaceEyeLocation,
	ViewSpaceLightLocation,
	AlbedoMap,
	ArmMap,
	NormalMap,
	SSAOKernelMap,
	SSAOBlurredMap,
	// UPDATE HERE TO ADD FRAME TEXTURE
}
