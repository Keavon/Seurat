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

		let shader_id = resources.shaders.get_index_of(shader_name).unwrap();
		let name = String::from(material_name);
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &shader.bind_group_layout,
			entries: bind_group_entries(material_name, shader, data_bindings, resources).as_slice(),
			label: Some(material_name),
		});

		Self { shader_id, name, bind_group }
	}
}

fn bind_group_entries<'a>(material_name: &'a str, shader: &'a crate::shader::Shader, data_bindings: Vec<MaterialDataBinding<'a>>, resources: &'a LoadedResources) -> Vec<wgpu::BindGroupEntry<'a>> {
	let mut binding_index = 0;

	shader
		.shader_bindings
		.iter()
		.enumerate()
		.flat_map(|(index, binding)| match binding {
			ShaderBinding::Buffer(_) => {
				let binding = binding_index;

				binding_index += 1;

				let buffer_binding = data_bindings
					.get(index)
					.and_then(|material_data_binding| match material_data_binding {
						MaterialDataBinding::Buffer(buffer) => Some(buffer.clone()),
						MaterialDataBinding::Texture(_) | &MaterialDataBinding::TextureName(_) | MaterialDataBinding::SampleableDepthTexture(_, _) | MaterialDataBinding::StorageTexture(_, _) => None,
					})
					.unwrap_or_else(|| panic!("Provided binding data for material '{}' does not match the shader definition", material_name));

				let resource = wgpu::BindingResource::Buffer(buffer_binding);

				vec![wgpu::BindGroupEntry { binding, resource }]
			}
			ShaderBinding::Texture(_) => {
				let binding = binding_index;

				binding_index += 2;

				let (sampler, texture_view) = data_bindings
					.get(index)
					.and_then(|material_data_binding| match material_data_binding {
						&MaterialDataBinding::Texture(texture) => Some((&texture.sampler, &texture.view)),
						&MaterialDataBinding::SampleableDepthTexture(texture, sampler) => Some((sampler, &texture.view)),
						&MaterialDataBinding::StorageTexture(texture, view) => Some((&texture.sampler, view.unwrap_or(&texture.view))),
						MaterialDataBinding::TextureName(texture) => Some((&resources.textures[*texture].sampler, &resources.textures[*texture].view)),
						MaterialDataBinding::Buffer(_) => None,
					})
					.unwrap_or_else(|| panic!("Provided binding data for material '{}' does not match the shader definition", material_name));

				vec![
					wgpu::BindGroupEntry {
						binding,
						resource: wgpu::BindingResource::TextureView(texture_view),
					},
					wgpu::BindGroupEntry {
						binding: binding + 1,
						resource: wgpu::BindingResource::Sampler(sampler),
					},
				]
			}
			ShaderBinding::StorageTexture(_, _) => {
				let binding = binding_index;

				binding_index += 1;

				let (_sampler, texture_view) = data_bindings
					.get(index)
					.and_then(|material_data_binding| match material_data_binding {
						&MaterialDataBinding::Texture(texture) => Some((&texture.sampler, &texture.view)),
						&MaterialDataBinding::SampleableDepthTexture(texture, sampler) => Some((sampler, &texture.view)),
						&MaterialDataBinding::StorageTexture(texture, view) => Some((&texture.sampler, view.unwrap_or(&texture.view))),
						MaterialDataBinding::TextureName(texture) => Some((&resources.textures[*texture].sampler, &resources.textures[*texture].view)),
						MaterialDataBinding::Buffer(_) => None,
					})
					.unwrap_or_else(|| panic!("Provided binding data for material '{}' does not match the shader definition", material_name));

				let resource = wgpu::BindingResource::TextureView(texture_view);

				vec![wgpu::BindGroupEntry { binding, resource }]
			}
		})
		.collect()
}

pub enum MaterialDataBinding<'a> {
	Buffer(wgpu::BufferBinding<'a>),
	Texture(&'a Texture),
	SampleableDepthTexture(&'a Texture, &'a wgpu::Sampler),
	StorageTexture(&'a Texture, Option<&'a wgpu::TextureView>),
	TextureName(&'a str),
}
