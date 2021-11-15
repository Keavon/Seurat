use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout};
use winit::event::{ElementState, VirtualKeyCode};

use crate::context::Context;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugBufferUniform {
	pub values: [f32; 4],
}

pub struct DebugBuffer {
	pub debug_uniform: DebugBufferUniform,
	pub debug_buffer: wgpu::Buffer,
	pub debug_bind_group_layout: BindGroupLayout,
	pub debug_bind_group: BindGroup,
	pub modifying_index: i32,
	pub modifying_value: f32,
}

impl DebugBuffer {
	pub fn new(context: &Context) -> Self {
		let debug_uniform = DebugBufferUniform { values: [0., 0., 0., 0.] };

		// We'll want to update our debug data, so we use COPY_DST
		let debug_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Debug Buffer"),
			contents: bytemuck::cast_slice(&[debug_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let debug_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

		let debug_bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &debug_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: debug_buffer.as_entire_binding(),
			}],
			label: None,
		});

		Self {
			debug_uniform,
			debug_buffer,
			debug_bind_group_layout,
			debug_bind_group,
			modifying_index: -1,
			modifying_value: 0.,
		}
	}

	pub fn update(&mut self, delta_time: std::time::Duration, queue: &mut wgpu::Queue) {
		let index = {
			if self.modifying_index < 0 {
				return;
			}

			self.modifying_index as usize
		};

		self.debug_uniform.values[index] += self.modifying_value * delta_time.as_secs_f32();
		queue.write_buffer(&self.debug_buffer, 0, bytemuck::cast_slice(&[self.debug_uniform]));
	}

	pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
		self.update_modifying_value(key, state) || self.update_modifying_index(key, state)
	}

	fn update_modifying_value(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
		let value = match key {
			VirtualKeyCode::Up => 1.,
			VirtualKeyCode::Down => -1.,
			_ => return false,
		};

		self.modifying_value = match state {
			ElementState::Pressed => value,
			ElementState::Released => 0.,
		};

		true
	}

	fn update_modifying_index(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
		let index = match key {
			VirtualKeyCode::Key1 => 0,
			VirtualKeyCode::Key2 => 1,
			VirtualKeyCode::Key3 => 2,
			VirtualKeyCode::Key4 => 3,
			_ => return false,
		};

		self.modifying_index = match state {
			ElementState::Pressed => index,
			ElementState::Released => {
				if index == self.modifying_index {
					-1
				} else {
					self.modifying_index
				}
			}
		};

		true
	}
}
