use crate::entity::Entity;
use crate::material::Material;
use crate::mesh::Mesh;
use crate::shader::Shader;
use crate::texture::Texture;

use indexmap::IndexMap;
use std::collections::HashMap;

pub struct Scene {
	pub root: Entity,
	pub entity_name_paths: HashMap<String, Vec<usize>>,
	pub resources: LoadedResources,
}

impl Scene {
	pub fn new() -> Self {
		let mut entity_name_paths = HashMap::new();
		entity_name_paths.insert(String::from("Scene Root"), vec![]);

		Self {
			root: Entity::new("Scene Root"),
			entity_name_paths,
			resources: LoadedResources::new(),
		}
	}

	pub fn find_entity(&self, name: &str) -> Option<&Entity> {
		if let Some(index_path) = self.entity_name_paths.get(name) {
			// First traverse the index paths for the non-mutabe entity to check if it exists
			let entity = index_path
				.iter()
				.fold(Some(&self.root), |accumulator, index| accumulator.and_then(|entity| entity.children.get(*index)));

			// If it exists, traverse again to get and immediately return the mutable entity reference
			if entity.is_some() {
				return index_path
					.iter()
					.fold(Some(&self.root), |accumulator, index| accumulator.and_then(|entity| entity.children.get(*index)));
			}
		}

		// If it wasn't cached, or the cache is now invalid, update the cache and return the new result from the expensive search operation
		self.root.find_descendant(name)

		// TODO: Find `found_entity` path and save it to the cache
	}

	pub fn find_entity_mut(&mut self, name: &str) -> Option<&mut Entity> {
		if let Some(index_path) = self.entity_name_paths.get(name) {
			// First traverse the index paths for the non-mutabe entity to check if it exists
			let entity = index_path
				.iter()
				.fold(Some(&self.root), |accumulator, index| accumulator.and_then(|entity| entity.children.get(*index)));

			// If it exists, traverse again to get and immediately return the mutable entity reference
			if entity.is_some() {
				return index_path
					.iter()
					.fold(Some(&mut self.root), |accumulator, index| accumulator.and_then(|entity| entity.children.get_mut(*index)));
			}
		}

		// If it wasn't cached, or the cache is now invalid, update the cache and return the new result from the expensive search operation
		self.root.find_descendant_mut(name)

		// TODO: Find `found_entity` path and save it to the cache
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

impl Default for LoadedResources {
	fn default() -> Self {
		Self::new()
	}
}
