use cgmath::{InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer};

use crate::engine::Context;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
	1.0, 0.0, 0.0, 0.0,
	0.0, 1.0, 0.0, 0.0,
	0.0, 0.0, 0.5, 0.0,
	0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct SceneCamera {
	pub position: Point3<f32>,
	pub pitch: Rad<f32>,
	pub yaw: Rad<f32>,
	pub projection: Projection,
	pub camera_uniform: CameraUniform,
	pub camera_buffer: Buffer,
	pub camera_bind_group_layout: BindGroupLayout,
	pub camera_bind_group: BindGroup,
}

impl SceneCamera {
	pub fn new(context: &Context) -> Self {
		let position: Point3<f32> = (0.0, 5.0, 10.0).into();
		let pitch: Rad<f32> = cgmath::Deg(-20.0).into();
		let yaw: Rad<f32> = cgmath::Deg(-90.0).into();
		let projection = Projection::new(context.config.width, context.config.height, cgmath::Deg(45.0), 0.1, 100.0);

		let mut camera_uniform = CameraUniform::new();
		camera_uniform.view_position = position.to_homogeneous().into();
		let calc_matrix = Matrix4::look_to_rh(position, Vector3::new(yaw.0.cos(), pitch.0.sin(), yaw.0.sin()).normalize(), Vector3::unit_y());
		camera_uniform.view_proj = (projection.calc_matrix() * calc_matrix).into();

		let camera_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Camera Buffer"),
			contents: bytemuck::cast_slice(&[camera_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let camera_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			}],
			label: Some("camera_bind_group_layout"),
		});

		let camera_bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &camera_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: camera_buffer.as_entire_binding(),
			}],
			label: Some("camera_bind_group"),
		});

		Self {
			position,
			pitch,
			yaw,
			projection,
			camera_uniform,
			camera_buffer,
			camera_bind_group_layout,
			camera_bind_group,
		}
	}

	pub fn update_view_proj(&mut self) {
		self.camera_uniform.view_position = self.position.to_homogeneous().into();
		self.camera_uniform.view_proj = (self.projection.calc_matrix() * self.calc_matrix()).into();
	}

	pub fn calc_matrix(&self) -> Matrix4<f32> {
		Matrix4::look_to_rh(self.position, Vector3::new(self.yaw.0.cos(), self.pitch.0.sin(), self.yaw.0.sin()).normalize(), Vector3::unit_y())
	}
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
	// We can't use cgmath with bytemuck directly so we'll have
	// to convert the Matrix4 into a 4x4 f32 array
	view_position: [f32; 4],
	view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
	pub fn new() -> Self {
		Self {
			view_position: [0.0; 4],
			view_proj: cgmath::Matrix4::identity().into(),
		}
	}
}

#[derive(Debug)]
pub struct Projection {
	aspect: f32,
	fovy: Rad<f32>,
	znear: f32,
	zfar: f32,
}

impl Projection {
	pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
		Self {
			aspect: width as f32 / height as f32,
			fovy: fovy.into(),
			znear,
			zfar,
		}
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.aspect = width as f32 / height as f32;
	}

	pub fn calc_matrix(&self) -> Matrix4<f32> {
		OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
	}
}
