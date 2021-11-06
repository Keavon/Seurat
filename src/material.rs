use crate::scene::LoadedResources;
use crate::shader::ShaderBinding;
use crate::texture::Texture;

pub struct Material {
	pub shader_id: usize,
	pub name: String,
	pub bind_group: wgpu::BindGroup,
}

impl Material {
	pub fn new(material_name: &str, shader_name: &str, data_bindings: Vec<MaterialDataBinding>, resources: &LoadedResources, device: &wgpu::Device) -> Self {
		let shader = &resources.shaders[shader_name];

		let mut binding_index = 0;
		let entries = shader
			.shader_bindings
			.iter()
			.enumerate()
			.flat_map(|(index, binding)| match binding {
				ShaderBinding::Buffer(_) => {
					let binding = binding_index;
					binding_index += 1;

					let buffer_binding = data_bindings
						.get(index)
						.map(|material_data_binding| match material_data_binding {
							MaterialDataBinding::Buffer(buffer) => Some(buffer.clone()),
							MaterialDataBinding::Texture(_) | &MaterialDataBinding::TextureName(_) => None,
						})
						.flatten()
						.unwrap_or_else(|| panic!("Provided binding data for material '{}' does not match the shader definition", material_name));

					vec![wgpu::BindGroupEntry {
						binding,
						resource: wgpu::BindingResource::Buffer(buffer_binding),
					}]
				}
				ShaderBinding::Texture(_) => {
					let binding = binding_index;
					binding_index += 2;

					let texture_data = data_bindings
						.get(index)
						.map(|material_data_binding| match material_data_binding {
							&MaterialDataBinding::Texture(texture) => Some(texture),
							MaterialDataBinding::TextureName(texture) => Some(&resources.textures[*texture]),
							MaterialDataBinding::Buffer(_) => None,
						})
						.flatten()
						.unwrap_or_else(|| panic!("Provided binding data for material '{}' does not match the shader definition", material_name));

					vec![
						wgpu::BindGroupEntry {
							binding,
							resource: wgpu::BindingResource::TextureView(&texture_data.view),
						},
						wgpu::BindGroupEntry {
							binding: binding + 1,
							resource: wgpu::BindingResource::Sampler(&texture_data.sampler),
						},
					]
				}
				ShaderBinding::StorageTexture(_, _) => {
					let binding = binding_index;
					binding_index += 1;

					let texture_data = data_bindings
						.get(index)
						.map(|material_data_binding| match material_data_binding {
							&MaterialDataBinding::Texture(texture) => Some(texture),
							MaterialDataBinding::TextureName(texture) => Some(&resources.textures[*texture]),
							MaterialDataBinding::Buffer(_) => None,
						})
						.flatten()
						.unwrap_or_else(|| panic!("Provided binding data for material '{}' does not match the shader definition", material_name));

					vec![
						wgpu::BindGroupEntry {
							binding,
							resource: wgpu::BindingResource::TextureView(&texture_data.view),
						},
					]
				}
			})
			.collect::<Vec<wgpu::BindGroupEntry>>();

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &shader.bind_group_layout,
			entries: entries.as_slice(),
			label: Some(material_name),
		});

		Self {
			shader_id: resources.shaders.get_index_of(shader_name).unwrap(),
			name: String::from(material_name),
			bind_group,
		}
	}
}

pub enum MaterialDataBinding<'a> {
	Buffer(wgpu::BufferBinding<'a>),
	Texture(&'a Texture),
	TextureName(&'a str),
}
