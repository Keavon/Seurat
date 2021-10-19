pub use winit::dpi::Position;

#[derive(Debug)]
pub struct Transform {
	pub location: cgmath::Point3<f64>,
	pub rotation: cgmath::Quaternion<f64>,
	pub scale: cgmath::Vector3<f64>,
}

impl Transform {
	pub fn new(location: cgmath::Point3<f64>, rotation: cgmath::Quaternion<f64>, scale: cgmath::Vector3<f64>) -> Self {
		Self { location, rotation, scale }
	}
}

impl Default for Transform {
	fn default() -> Self {
		Self::new(cgmath::Point3::new(0., 0., 0.), cgmath::Quaternion::new(1., 0., 0., 0.), cgmath::Vector3::new(0., 0., 0.))
	}
}
