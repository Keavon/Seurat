use crate::component::Component;
use crate::transform::Transform;

pub struct Entity {
	pub enabled: bool,
	pub transform: Transform,
	pub components: Vec<Component>,
	pub children: Vec<Entity>,
}

impl Entity {
	pub fn new() -> Self {
		Self {
			enabled: true,
			transform: Transform::default(),
			components: vec![],
			children: vec![],
		}
	}
}
