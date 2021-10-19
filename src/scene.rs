use crate::entity::Entity;
use crate::material::Material;
use crate::mesh::Mesh;
use crate::shader::Shader;
use crate::texture::Texture;

use indexmap::IndexMap;

pub struct Scene {
	pub root: Entity,
	pub resources: LoadedResources,
}

impl Scene {
	pub fn new() -> Self {
		Self {
			root: Entity::new("Scene Root"),
			resources: LoadedResources::new(),
		}
	}
}

pub struct LoadedResources {
	pub shaders: IndexMap<String, Shader>,
	pub textures: IndexMap<String, Texture>,
	pub materials: IndexMap<String, Material>,
	pub meshes: IndexMap<(String, String), Mesh>,
}

impl LoadedResources {
	pub fn new() -> Self {
		Self {
			shaders: IndexMap::new(),
			textures: IndexMap::new(),
			materials: IndexMap::new(),
			meshes: IndexMap::new(),
		}
	}
}
