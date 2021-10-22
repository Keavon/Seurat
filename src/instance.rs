use wgpu::{util::DeviceExt, Device};

#[derive(Debug)]
pub struct Instances {
	pub instance_list: Vec<Instance>,
	pub instances_buffer: Option<wgpu::Buffer>,
}

impl Instances {
	pub fn new() -> Self {
		let origin = Instance::new();

		Self {
			instance_list: vec![origin],
			instances_buffer: None,
		}
	}

	pub fn transform_single_instance(&mut self, location: cgmath::Point3<f64>, rotation: cgmath::Quaternion<f64>, scale: cgmath::Point3<f64>, device: &Device) {
		let location = cgmath::Vector3::new(location.x as f32, location.y as f32, location.z as f32);
		let rotation = cgmath::Quaternion::new(rotation.s as f32, rotation.v.x as f32, rotation.v.y as f32, rotation.v.z as f32);
		let scale = cgmath::Vector3::new(scale.x as f32, scale.y as f32, scale.z as f32);

		self.instance_list = vec![Instance { location, rotation, scale }];
		self.update_buffer(device);
	}

	pub fn update_buffer(&mut self, device: &Device) {
		let instance_data = self.instance_list.iter().map(Instance::to_raw).collect::<Vec<_>>();

		let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Instance Buffer"),
			contents: bytemuck::cast_slice(&instance_data),
			usage: wgpu::BufferUsages::VERTEX,
		});

		self.instances_buffer = Some(instances_buffer);
	}
}

impl Default for Instances {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
pub struct Instance {
	pub location: cgmath::Vector3<f32>,
	pub rotation: cgmath::Quaternion<f32>,
	pub scale: cgmath::Vector3<f32>,
}

impl Instance {
	pub fn new() -> Self {
		Self {
			location: cgmath::Vector3::new(0., 0., 0.),
			rotation: cgmath::Quaternion::new(1., 0., 0., 0.),
			scale: cgmath::Vector3::new(1., 1., 1.),
		}
	}

	pub fn to_raw(&self) -> InstanceRaw {
		InstanceRaw {
			model: (cgmath::Matrix4::from_translation(self.location) * cgmath::Matrix4::from(self.rotation) * cgmath::Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z)).into(),
		}
	}
}

impl Default for Instance {
	fn default() -> Self {
		Self::new()
	}
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
	model: [[f32; 4]; 4],
}

impl InstanceRaw {
	pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
			// We need to switch from using a step mode of Vertex to Instance
			// This means that our shaders will only change to use the next
			// instance when the shader starts processing a new instance
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				// A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
				// for each vec4. We'll have to reassemble the mat4 in the shader.

				// model matrix (1/4)
				wgpu::VertexAttribute {
					offset: 0,
					// While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
					// be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
					shader_location: 4,
					format: wgpu::VertexFormat::Float32x4,
				},
				// model matrix (2/4)
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
					shader_location: 5,
					format: wgpu::VertexFormat::Float32x4,
				},
				// model matrix (3/4)
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
					shader_location: 6,
					format: wgpu::VertexFormat::Float32x4,
				},
				// model matrix (4/4)
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
					shader_location: 7,
					format: wgpu::VertexFormat::Float32x4,
				},
			],
		}
	}
}
