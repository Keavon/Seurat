use crate::{instance::Instances, scene::LoadedResources};

#[derive(Debug)]
pub struct Model {
	pub mesh: usize,
	pub material: usize,
	pub instances: Instances,
}

impl Model {
	pub fn new(resources: &LoadedResources, mesh: (&str, &str), material: &str) -> Self {
		let mesh = resources.meshes.get_index_of(&(String::from(mesh.0), String::from(mesh.1))).unwrap();
		let material = resources.materials.get_index_of(&String::from(material)).unwrap();
		let instances = Instances::new();
		Self { mesh, material, instances }
	}
}
