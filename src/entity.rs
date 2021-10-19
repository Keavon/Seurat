use rayon::vec;

use crate::component::Component;
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

	pub fn find_descendant(&self, name: &str) -> Option<&Entity> {
		self.children.iter().find(|entity| entity.name == name)
	}

	pub fn find_descendant_mut(&mut self, name: &str) -> Option<&mut Entity> {
		self.children.iter_mut().find(|entity| entity.name == name)
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
