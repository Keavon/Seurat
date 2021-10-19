use crate::behavior::Behavior;
use crate::entity::Entity;

#[derive(Debug)]
pub struct LightCubeMovement;

impl Behavior for LightCubeMovement {
	fn update(&self, entity: &mut Entity) {
		entity.transform.location.y += 0.01;
	}
}
