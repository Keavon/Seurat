use crate::behavior::Behavior;
use crate::camera::SceneCamera;
use crate::light::Light;
use crate::model::Model;

#[derive(Debug)]
pub enum Component {
	Model(Model),
	Light(Light),
	Camera(SceneCamera),
	Behavior(Box<dyn Behavior>),
}
