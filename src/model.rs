use crate::{instance::Instances, scene::LoadedResources};

#[derive(Debug)]
pub struct Model {
	pub mesh_name: (String, String),
	pub mesh: Option<usize>,
	pub material_name: String,
	pub material: Option<usize>,
	pub instances: Instances,
}

impl Model {
	pub fn new(mesh: (&str, &str), material: &str) -> Self {
		Self {
			mesh_name: (String::from(mesh.0), String::from(mesh.1)),
			mesh: None,
			material_name: String::from(material),
			material: None,
			instances: Instances::new(),
		}
	}

	pub fn load(&mut self, resources: &LoadedResources) {
		self.mesh = Some(resources.meshes.get_index_of(&(self.mesh_name.0.clone(), self.mesh_name.1.clone())).unwrap());
		self.material = Some(resources.materials.get_index_of(&self.material_name.clone()).unwrap());
	}
}
