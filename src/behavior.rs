use crate::entity::Entity;

use core::fmt::Debug;

pub trait Behavior: Debug {
	fn update(&self, entity: &mut Entity);
}
