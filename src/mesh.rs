use crate::material::Material;
use crate::model::Model;

use std::{mem, ops::Range};

pub struct Mesh {
	pub name: String,
	pub vertex_buffer: wgpu::Buffer,
	pub index_buffer: wgpu::Buffer,
	pub num_elements: u32,
	pub material: usize,
}

pub trait DrawModel<'a> {
	fn draw_mesh(&mut self, mesh: &'a Mesh, material: &'a Material, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);
	fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, material: &'a Material, instances: Range<u32>, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);

	fn draw_model(&mut self, model: &'a Model, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);
	fn draw_model_instanced(&mut self, model: &'a Model, instances: Range<u32>, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
	'b: 'a,
{
	fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, camera: &'b wgpu::BindGroup, light: &'a wgpu::BindGroup) {
		self.draw_mesh_instanced(mesh, material, 0..1, camera, light);
	}

	fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, material: &'b Material, instances: Range<u32>, camera: &'b wgpu::BindGroup, light: &'a wgpu::BindGroup) {
		self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
		self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

		self.set_bind_group(0, &material.bind_group, &[]);
		self.set_bind_group(1, camera, &[]);
		self.set_bind_group(2, light, &[]);

		self.draw_indexed(0..mesh.num_elements, 0, instances);
	}

	fn draw_model(&mut self, model: &'b Model, camera: &'b wgpu::BindGroup, light: &'a wgpu::BindGroup) {
		self.draw_model_instanced(model, 0..1, camera, light);
	}

	fn draw_model_instanced(&mut self, model: &'b Model, instances: Range<u32>, camera: &'b wgpu::BindGroup, light: &'a wgpu::BindGroup) {
		for mesh in &model.meshes {
			let material = &model.materials[mesh.material];
			self.draw_mesh_instanced(mesh, material, instances.clone(), camera, light);
		}
	}
}

pub trait DrawLight<'a> {
	fn draw_light_mesh(&mut self, mesh: &'a Mesh, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);
	fn draw_light_mesh_instanced(&mut self, mesh: &'a Mesh, instances: Range<u32>, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);

	fn draw_light_model(&mut self, model: &'a Model, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);
	fn draw_light_model_instanced(&mut self, model: &'a Model, instances: Range<u32>, camera: &'a wgpu::BindGroup, light: &'a wgpu::BindGroup);
}
impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
where
	'b: 'a,
{
	fn draw_light_mesh(&mut self, mesh: &'b Mesh, camera: &'b wgpu::BindGroup, light: &'b wgpu::BindGroup) {
		self.draw_light_mesh_instanced(mesh, 0..1, camera, light);
	}

	fn draw_light_mesh_instanced(&mut self, mesh: &'b Mesh, instances: Range<u32>, camera: &'b wgpu::BindGroup, light: &'b wgpu::BindGroup) {
		self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
		self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
		self.set_bind_group(0, camera, &[]);
		self.set_bind_group(1, light, &[]);
		self.draw_indexed(0..mesh.num_elements, 0, instances);
	}

	fn draw_light_model(&mut self, model: &'b Model, camera: &'b wgpu::BindGroup, light: &'b wgpu::BindGroup) {
		self.draw_light_model_instanced(model, 0..1, camera, light);
	}
	fn draw_light_model_instanced(&mut self, model: &'b Model, instances: Range<u32>, camera: &'b wgpu::BindGroup, light: &'b wgpu::BindGroup) {
		for mesh in &model.meshes {
			self.draw_light_mesh_instanced(mesh, instances.clone(), camera, light);
		}
	}
}

pub trait Vertex {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
	pub position: [f32; 3],
	pub uv: [f32; 2],
	pub normal: [f32; 3],
	pub tangent: [f32; 3],
	pub bitangent: [f32; 3],
}

impl Vertex for ModelVertex {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				// position
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x3,
				},
				// uv
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x2,
				},
				// normal
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x3,
				},
				// tangent
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
					shader_location: 3,
					format: wgpu::VertexFormat::Float32x3,
				},
				// bitangent
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
					shader_location: 4,
					format: wgpu::VertexFormat::Float32x3,
				},
			],
		}
	}
}
