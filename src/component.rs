use crate::camera::Camera;
use crate::light::Light;
use crate::model::Model;

pub enum Component {
	Model(Model),
	Light(Light),
	Camera(Camera),
	Behavior,
}
