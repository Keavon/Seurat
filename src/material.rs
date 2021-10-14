use crate::shader::Shader;
use crate::texture::Texture;

pub struct Material {
	pub name: String,
	pub diffuse_texture: Texture,
	pub normal_texture: Texture,
	pub bind_group: wgpu::BindGroup,
}
impl Material {
	pub fn new(device: &wgpu::Device, name: &str, diffuse_texture: Texture, normal_texture: Texture, shader: &Shader) -> Self {
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &shader.bind_group_layout,
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
			name: String::from(name),
			diffuse_texture,
			normal_texture,
			bind_group,
		}
	}
}
