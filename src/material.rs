use crate::scene::LoadedResources;

pub struct Material {
	pub shader: usize,
	pub name: String,
	pub diffuse_texture: String,
	pub normal_texture: String,
	pub bind_group: wgpu::BindGroup,
}
impl Material {
	pub fn new(device: &wgpu::Device, resources: &LoadedResources, name: &str, shader: &str, diffuse: &str, normal: &str) -> Self {
		let shader_structure = &resources.shaders[shader];
		let diffuse_texture = &resources.textures[diffuse];
		let normal_texture = &resources.textures[normal];

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &shader_structure.bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(&normal_texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 3,
					resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
				},
			],
			label: Some(name),
		});

		Self {
			shader: resources.shaders.get_index_of(shader).unwrap(),
			name: String::from(name),
			diffuse_texture: String::from(diffuse),
			normal_texture: String::from(normal),
			bind_group,
		}
	}
}
