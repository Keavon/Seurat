use crate::{instance::Instances, scene::LoadedResources};

#[derive(Debug)]
pub struct Model {
	pub mesh_name: (String, String),
	pub mesh: Option<usize>,
	pub voxel_lightmap_material: Option<usize>,
	pub scene_deferred_material: Option<usize>,
	pub instances: Instances,
}

impl Model {
	pub fn new(mesh: (&str, &str)) -> Self {
		Self {
			mesh_name: (String::from(mesh.0), String::from(mesh.1)),
			mesh: None,
			voxel_lightmap_material: None,
			scene_deferred_material: None,
			instances: Instances::new(),
		}
	}

	pub fn load(&mut self, resources: &LoadedResources) {
		self.mesh = Some(resources.meshes.get_index_of(&(self.mesh_name.0.clone(), self.mesh_name.1.clone())).unwrap());
		let voxel_material_name = format!("calc_voxel_lightmap_{}.material", self.mesh_name.1);
		self.voxel_lightmap_material = Some(resources.materials.get_index_of(&voxel_material_name).unwrap());
		let scene_deferred_material_name = format!("scene_deferred_{}.material", self.mesh_name.1);
		self.scene_deferred_material = Some(resources.materials.get_index_of(&scene_deferred_material_name).unwrap());
	}
}
