use crate::material::Material;
use crate::model::Model;

use anyhow::Result;
use cgmath::InnerSpace;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{mem, ops::Range, path::Path};
use tobj::LoadOptions;
use wgpu::{util::DeviceExt, RenderPass};

pub struct Mesh {
	pub name: String,
	pub vertex_buffer: wgpu::Buffer,
	pub index_buffer: wgpu::Buffer,
	pub index_count: u32,
}

impl Mesh {
	pub fn load(device: &wgpu::Device, queue: &wgpu::Queue, directory: &Path, file: &str) -> Result<Vec<Self>> {
		let path = directory.join("models").join(file);

		let (obj_models, obj_materials) = tobj::load_obj(
			path.clone(),
			&LoadOptions {
				triangulate: true,
				single_index: true,
				..Default::default()
			},
		)?;
		let obj_materials = obj_materials?;

		let meshes = obj_models
			.par_iter()
			.map(|m| {
				let mut vertices = (0..m.mesh.positions.len() / 3)
					.into_par_iter()
					.map(|i| {
						ModelVertex {
							position: [m.mesh.positions[i * 3], m.mesh.positions[i * 3 + 1], m.mesh.positions[i * 3 + 2]],
							uv: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
							normal: [m.mesh.normals[i * 3], m.mesh.normals[i * 3 + 1], m.mesh.normals[i * 3 + 2]],
							// Calculated below
							tangent: [0.0; 3],
							bitangent: [0.0; 3],
						}
					})
					.collect::<Vec<_>>();

				let indices = &m.mesh.indices;
				let mut triangles_included = (0..vertices.len()).collect::<Vec<_>>();

				// Calculate tangents and bitangets. We're going to
				// use the triangles, so we need to loop through the
				// indices in chunks of 3
				for c in indices.chunks(3) {
					let v0 = vertices[c[0] as usize];
					let v1 = vertices[c[1] as usize];
					let v2 = vertices[c[2] as usize];

					let pos0: cgmath::Vector3<_> = v0.position.into();
					let pos1: cgmath::Vector3<_> = v1.position.into();
					let pos2: cgmath::Vector3<_> = v2.position.into();

					let uv0: cgmath::Vector2<_> = v0.uv.into();
					let uv1: cgmath::Vector2<_> = v1.uv.into();
					let uv2: cgmath::Vector2<_> = v2.uv.into();

					// Calculate the edges of the triangle
					let delta_pos1 = pos1 - pos0;
					let delta_pos2 = pos2 - pos0;

					// This will give us a direction to calculate the
					// tangent and bitangent
					let delta_uv1 = uv1 - uv0;
					let delta_uv2 = uv2 - uv0;

					// Solving the following system of equations will
					// give us the tangent and bitangent.
					//     delta_pos1 = delta_uv1.x * T + delta_u.y * B
					//     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
					// Luckily, the place I found this equation provided
					// the solution!
					let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
					let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
					let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

					// We'll use the same tangent/bitangent for each vertex in the triangle
					vertices[c[0] as usize].tangent = (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
					vertices[c[1] as usize].tangent = (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
					vertices[c[2] as usize].tangent = (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
					vertices[c[0] as usize].bitangent = (bitangent + cgmath::Vector3::from(vertices[c[0] as usize].bitangent)).into();
					vertices[c[1] as usize].bitangent = (bitangent + cgmath::Vector3::from(vertices[c[1] as usize].bitangent)).into();
					vertices[c[2] as usize].bitangent = (bitangent + cgmath::Vector3::from(vertices[c[2] as usize].bitangent)).into();

					// Used to average the tangents/bitangents
					triangles_included[c[0] as usize] += 1;
					triangles_included[c[1] as usize] += 1;
					triangles_included[c[2] as usize] += 1;
				}

				// Average the tangents/bitangents
				for (i, n) in triangles_included.into_iter().enumerate() {
					let denom = 1.0 / n as f32;
					let mut v = &mut vertices[i];
					v.tangent = (cgmath::Vector3::from(v.tangent) * denom).normalize().into();
					v.bitangent = (cgmath::Vector3::from(v.bitangent) * denom).normalize().into();
				}

				let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some(&format!("{:?} Vertex Buffer", path)),
					contents: bytemuck::cast_slice(&vertices),
					usage: wgpu::BufferUsages::VERTEX,
				});
				let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some(&format!("{:?} Index Buffer", m.name)),
					contents: bytemuck::cast_slice(&m.mesh.indices),
					usage: wgpu::BufferUsages::INDEX,
				});

				Ok(Mesh {
					name: m.name.clone(),
					vertex_buffer,
					index_buffer,
					index_count: m.mesh.indices.len() as u32,
				})
			})
			.collect::<Result<Vec<_>>>()?;

		Ok(meshes)
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
