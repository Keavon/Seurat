use crate::context::Context;
use crate::transform::Transform;

use cgmath::{Euler, InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Vector3};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
	1.0, 0.0, 0.0, 0.0,
	0.0, 1.0, 0.0, 0.0,
	0.0, 0.0, 0.5, 0.0,
	0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
	pub location: Point3<f32>,
	pub pitch: Rad<f32>,
	pub yaw: Rad<f32>,
	pub projection: Projection,
	pub camera_uniform: CameraUniform,
	pub camera_buffer: Buffer,
	pub camera_bind_group_layout: BindGroupLayout,
	pub camera_bind_group: BindGroup,
}

impl Camera {
	pub fn new(context: &Context, projection: Projection) -> Self {
		let mut camera_uniform = CameraUniform::new();

		let location: Point3<f32> = (-10.0, 5.0, 0.0).into();
		let pitch: Rad<f32> = cgmath::Deg(-20.0).into();
		let yaw: Rad<f32> = cgmath::Deg(0.0).into();
		camera_uniform.v_matrix = Self::calculate_v_matrix(location, pitch, yaw).into();
		camera_uniform.p_matrix = match &projection {
			Projection::Perspective(p) => p.p_matrix().into(),
			Projection::Orthographic(o) => o.p_matrix().into(),
		};

		let camera_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Camera Buffer"),
			contents: bytemuck::cast_slice(&[camera_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let camera_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
			location,
			pitch,
			yaw,
			projection,
			camera_uniform,
			camera_buffer,
			camera_bind_group_layout,
			camera_bind_group,
		}
	}

	pub fn update_transform(&mut self, transform: &Transform) {
		self.location = Point3::new(transform.location.x as f32, transform.location.y as f32, transform.location.z as f32);
		let euler = Euler::from(transform.rotation);
		self.pitch = Rad(euler.x.0 as f32);
		self.yaw = Rad(euler.y.0 as f32);
	}

	pub fn update_v_p_matrices(&mut self, queue: &mut wgpu::Queue) {
		let v = Self::calculate_v_matrix(self.location, self.pitch, self.yaw);
		let p = match &self.projection {
			Projection::Perspective(p) => p.p_matrix(),
			Projection::Orthographic(o) => o.p_matrix(),
		};
		self.camera_uniform = CameraUniform::from_vp(v, p, self.camera_uniform.v_matrix, self.camera_uniform.p_matrix);

		queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
	}

	pub fn calculate_v_matrix(location: Point3<f32>, pitch: Rad<f32>, yaw: Rad<f32>) -> Matrix4<f32> {
		Matrix4::look_to_rh(location, Vector3::new(yaw.0.cos(), pitch.0.sin(), yaw.0.sin()).normalize(), Vector3::unit_y())
	}

	pub fn update_transform_and_matrices(&mut self, transform: &Transform, queue: &mut wgpu::Queue) {
		self.update_transform(transform);
		let translation = cgmath::Vector3::new(transform.location.x as f32, transform.location.y as f32, transform.location.z as f32);
		let rotation = cgmath::Quaternion::new(transform.rotation.s as f32, transform.rotation.v.x as f32, transform.rotation.v.y as f32, transform.rotation.v.z as f32);

		let v = cgmath::Matrix4::from_translation(translation) * cgmath::Matrix4::from(rotation);
		let p = match &self.projection {
			Projection::Perspective(p) => p.p_matrix(),
			Projection::Orthographic(o) => o.p_matrix(),
		};
		self.camera_uniform = CameraUniform::from_vp(v, p, self.camera_uniform.v_matrix, self.camera_uniform.p_matrix);

		queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
	}
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
	// We can't use cgmath with bytemuck directly so we'll have
	// to convert the Matrix4 into a 4x4 f32 array
	v_matrix: [[f32; 4]; 4],
	p_matrix: [[f32; 4]; 4],
	inv_v_matrix: [[f32; 4]; 4],
	inv_p_matrix: [[f32; 4]; 4],
	prev_v_matrix: [[f32; 4]; 4],
	prev_p_matrix: [[f32; 4]; 4],
}

impl CameraUniform {
	pub fn new() -> Self {
		Self::from_vp(
			cgmath::Matrix4::identity(),
			cgmath::Matrix4::identity(),
			cgmath::Matrix4::identity().into(),
			cgmath::Matrix4::identity().into(),
		)
	}

	pub fn from_vp(v: cgmath::Matrix4<f32>, p: cgmath::Matrix4<f32>, prev_v: [[f32; 4]; 4], prev_p: [[f32; 4]; 4]) -> Self {
		Self {
			v_matrix: v.into(),
			p_matrix: p.into(),
			inv_v_matrix: cgmath::Matrix4::invert(&v).unwrap().into(),
			inv_p_matrix: cgmath::Matrix4::invert(&p).unwrap().into(),
			prev_v_matrix: prev_v,
			prev_p_matrix: prev_p,
		}
	}
}

impl Default for CameraUniform {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
pub enum Projection {
	Perspective(PerspectiveProjection),
	Orthographic(OrthographicProjection),
}

#[derive(Debug)]
pub struct PerspectiveProjection {
	aspect: f32,
	fovy: Rad<f32>,
	znear: f32,
	zfar: f32,
}

impl PerspectiveProjection {
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

	pub fn p_matrix(&self) -> Matrix4<f32> {
		OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct OrthographicProjection {
	aspect: f32,
	size: f32,
	znear: f32,
	zfar: f32,
}

impl OrthographicProjection {
	pub fn new(width: u32, height: u32, size: f32, znear: f32, zfar: f32) -> Self {
		Self {
			aspect: width as f32 / height as f32,
			size,
			znear,
			zfar,
		}
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.aspect = width as f32 / height as f32;
	}

	pub fn p_matrix(&self) -> Matrix4<f32> {
		OPENGL_TO_WGPU_MATRIX
			* cgmath::ortho(
				-self.size * self.aspect * 0.5,
				self.size * self.aspect * 0.5,
				-self.size * (1. / self.aspect) * 0.5,
				self.size * (1. / self.aspect) * 0.5,
				self.znear,
				self.zfar,
			)
	}
}
