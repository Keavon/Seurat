use crate::behavior::Behavior;
use crate::camera::SceneCamera;
use crate::component::Component;
use crate::light::Light;
use crate::model::Model;
use crate::scene::LoadedResources;
use crate::transform::Transform;

#[derive(Debug)]
pub struct Entity {
	pub name: String,
	pub enabled: bool,
	pub transform: Transform,
	pub components: Vec<Component>,
	pub children: Vec<Entity>,
}

impl Entity {
	pub fn new(name: &str) -> Self {
		Self {
			name: String::from(name),
			enabled: true,
			transform: Transform::default(),
			components: vec![],
			children: vec![],
		}
	}

	pub fn new_child(&mut self, name: &str) -> &mut Entity {
		let child = Entity::new(name);
		self.children.push(child);
		self.children.last_mut().unwrap()
	}

	pub fn add_component(&mut self, component: Component) {
		self.components.push(component)
	}

	pub fn iter(&self) -> EntityIter<'_> {
		EntityIter { stack: vec![self] }
	}

	pub fn update_behaviors_of_descendants(&mut self) {
		let mut iter_components = vec![];
		std::mem::swap(&mut iter_components, &mut self.components);
		for component in iter_components.iter() {
			if let Component::Behavior(behavior) = component {
				behavior.update(self);
			}
		}
		std::mem::swap(&mut iter_components, &mut self.components);

		for child in self.children.iter_mut() {
			child.update_behaviors_of_descendants();
		}
	}

	pub fn load_models_on_descendants(&mut self, loaded_resources: &LoadedResources) {
		let mut iter_components = vec![];
		std::mem::swap(&mut iter_components, &mut self.components);
		for component in iter_components.iter_mut() {
			if let Component::Model(model) = component {
				model.load(loaded_resources);
			}
		}
		std::mem::swap(&mut iter_components, &mut self.components);

		for child in self.children.iter_mut() {
			child.load_models_on_descendants(loaded_resources);
		}
	}

	pub fn find_descendant(&self, name: &str) -> Option<&Entity> {
		self.children.iter().find(|entity| entity.name == name)
	}

	pub fn find_descendant_mut(&mut self, name: &str) -> Option<&mut Entity> {
		self.children.iter_mut().find(|entity| entity.name == name)
	}

	pub fn get_models(&self) -> Vec<&Model> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::Model(model) => Some(model),
				_ => None,
			})
			.collect()
	}

	pub fn get_models_mut(&mut self) -> Vec<&mut Model> {
		self.components
			.iter_mut()
			.filter_map(|component| match component {
				Component::Model(model) => Some(model),
				_ => None,
			})
			.collect()
	}

	pub fn get_lights(&self) -> Vec<&Light> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::Light(light) => Some(light),
				_ => None,
			})
			.collect()
	}

	pub fn get_lights_mut(&mut self) -> Vec<&mut Light> {
		self.components
			.iter_mut()
			.filter_map(|component| match component {
				Component::Light(light) => Some(light),
				_ => None,
			})
			.collect()
	}

	pub fn get_cameras(&self) -> Vec<&SceneCamera> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::Camera(camera) => Some(camera),
				_ => None,
			})
			.collect()
	}

	pub fn get_cameras_mut(&mut self) -> Vec<&mut SceneCamera> {
		self.components
			.iter_mut()
			.filter_map(|component| match component {
				Component::Camera(camera) => Some(camera),
				_ => None,
			})
			.collect()
	}

	pub fn get_behaviors(&self) -> Vec<&Box<dyn Behavior>> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::Behavior(behavior) => Some(behavior),
				_ => None,
			})
			.collect()
	}

	pub fn get_behaviors_mut(&mut self) -> Vec<&mut Box<dyn Behavior>> {
		self.components
			.iter_mut()
			.filter_map(|component| match component {
				Component::Behavior(behavior) => Some(behavior),
				_ => None,
			})
			.collect()
	}
}

impl<'a> IntoIterator for &'a Entity {
	type Item = &'a Entity;
	type IntoIter = EntityIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[derive(Debug)]
pub struct EntityIter<'a> {
	pub stack: Vec<&'a Entity>,
}

impl Default for EntityIter<'_> {
	fn default() -> Self {
		Self { stack: vec![] }
	}
}

impl<'a> Iterator for EntityIter<'a> {
	type Item = &'a Entity;

	fn next(&mut self) -> Option<Self::Item> {
		match self.stack.pop() {
			Some(entry) => {
				let entry_children = entry.children.iter().collect::<Vec<&Entity>>();
				self.stack.extend(entry_children);

				Some(entry)
			}
			None => None,
		}
	}
}
