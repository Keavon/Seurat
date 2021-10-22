use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout};

use crate::engine::Context;

#[derive(Debug)]
pub enum Light {
	Lamp(Lamp),
	Sun(Sun),
}

#[derive(Debug)]
pub struct Lamp {}

#[derive(Debug)]
pub struct Sun {}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
	pub location: [f32; 3],
	// Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
	pub _padding: u32,
	pub color: [f32; 3],
}

pub struct SceneLighting {
	pub light_uniform: LightUniform,
	pub light_buffer: wgpu::Buffer,
	pub light_bind_group_layout: BindGroupLayout,
	pub light_bind_group: BindGroup,
}

impl SceneLighting {
	pub fn new(context: &Context) -> Self {
		let light_uniform = LightUniform {
			location: [2.0, 2.0, 2.0],
			_padding: 0,
			color: [1.0, 1.0, 1.0],
		};

		// We'll want to update our lights location, so we use COPY_DST
		let light_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Light VB"),
			contents: bytemuck::cast_slice(&[light_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let light_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
			label: None,
		});

		let light_bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &light_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: light_buffer.as_entire_binding(),
			}],
			label: None,
		});

		Self {
			light_uniform,
			light_buffer,
			light_bind_group_layout,
			light_bind_group,
		}
	}
}
