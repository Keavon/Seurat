use crate::behavior::Behavior;
use crate::camera::Camera;
use crate::light::Light;
use crate::model::Model;

#[derive(Debug)]
pub enum Component {
	Model(Model),
	Light(Light),
	Camera(Camera),
	Behavior(Box<dyn Behavior>),
}
