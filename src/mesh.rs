use anyhow::Result;
use cgmath::InnerSpace;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{mem, path::Path};
use tobj::LoadOptions;
use wgpu::util::DeviceExt;

pub struct Mesh {
	pub name: String,
	pub vertex_buffer: wgpu::Buffer,
	pub index_buffer: wgpu::Buffer,
	pub index_count: u32,
	pub map_albedo: Option<String>,
	pub map_arm: Option<String>,
	pub map_normal: Option<String>,
}

impl Mesh {
	pub fn load(device: &wgpu::Device, queue: &wgpu::Queue, directory: &Path, file: &str) -> Result<Vec<Mesh>> {
		let path = directory.join("models").join(file);

		let (obj_models, obj_materials) = tobj::load_obj(
			path.clone(),
			&LoadOptions {
				triangulate: true,
				single_index: true,
				..Default::default()
			},
		)?;

		let obj_materials = obj_materials.unwrap_or_default();

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
							tangent: [0.0; 3], // Tangent value is calculated in the code below
						}
					})
					.collect::<Vec<_>>();

				// let mut triangles_included = (0..vertices.len()).collect::<Vec<_>>();

				// Calculate tangents. We're going to use the triangles, so we need to loop through the indices in chunks of 3
				for a in m.mesh.indices.chunks(3) {
					let i1 = a[0] as usize;
					let i2 = a[1] as usize;
					let i3 = a[2] as usize;

					let v1 = vertices[i1].position;
					let v2 = vertices[i2].position;
					let v3 = vertices[i3].position;

					let w1 = vertices[i1].uv;
					let w2 = vertices[i2].uv;
					let w3 = vertices[i3].uv;

					let x1 = v2[0] - v1[0];
					let x2 = v3[0] - v1[0];
					let y1 = v2[1] - v1[1];
					let y2 = v3[1] - v1[1];
					let z1 = v2[2] - v1[2];
					let z2 = v3[2] - v1[2];

					let s1 = w2[0] - w1[0];
					let s2 = w3[0] - w1[0];
					let t1 = w2[1] - w1[1];
					let t2 = w3[1] - w1[1];

					let r = 1. / (s1 * t2 - s2 * t1);
					let sdir = [(t2 * x1 - t1 * x2) * r, (t2 * y1 - t1 * y2) * r, (t2 * z1 - t1 * z2) * r];

					vertices[i1].tangent[0] += sdir[0];
					vertices[i1].tangent[1] += sdir[1];
					vertices[i1].tangent[2] += sdir[2];

					vertices[i2].tangent[0] += sdir[0];
					vertices[i2].tangent[1] += sdir[1];
					vertices[i2].tangent[2] += sdir[2];

					vertices[i3].tangent[0] += sdir[0];
					vertices[i3].tangent[1] += sdir[1];
					vertices[i3].tangent[2] += sdir[2];
				}

				for a in &mut vertices {
					let n = cgmath::Vector3::new(a.normal[0], a.normal[1], a.normal[2]);
					let t = cgmath::Vector3::new(a.tangent[0], a.tangent[1], a.tangent[2]);

					// Gram-Schmidt orthogonalize
					let orthogonalized = (t - n * cgmath::dot(n, t)).normalize();
					a.tangent[0] = orthogonalized.x;
					a.tangent[1] = orthogonalized.y;
					a.tangent[2] = orthogonalized.z;
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

				let (map_albedo, map_arm, map_normal) = if let Some(index) = m.mesh.material_id {
					let material = &obj_materials[index];

					(
						Some(material.diffuse_texture.clone()).filter(|name| !name.is_empty()),
						Some(material.shininess_texture.clone()).filter(|name| !name.is_empty()),
						Some(material.normal_texture.clone()).filter(|name| !name.is_empty()),
					)
				} else {
					(None, None, None)
				};

				Ok(Mesh {
					name: m.name.clone(),
					vertex_buffer,
					index_buffer,
					index_count: m.mesh.indices.len() as u32,
					map_albedo,
					map_arm,
					map_normal,
				})
			})
			.collect::<Result<Vec<_>>>()?;

		Ok(meshes)
	}

	pub fn new_blit_quad(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
		let corners = [(-1., -1.), (-1., 1.), (1., -1.), (1., 1.)];
		let vertices = corners.map(|point| ModelVertex {
			position: [point.0, point.1, 0.5],
			uv: [0.0; 2],
			normal: [0.0; 3],
			tangent: [0.0; 3],
		});

		let indices: [u32; 6] = [2, 1, 0, 3, 1, 2];

		let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Blit Quad Vertex Buffer"),
			contents: bytemuck::cast_slice(&vertices),
			usage: wgpu::BufferUsages::VERTEX,
		});
		let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Blit Quad Index Buffer"),
			contents: bytemuck::cast_slice(&indices),
			usage: wgpu::BufferUsages::INDEX,
		});

		Self {
			name: String::from("Blit Quad"),
			vertex_buffer,
			index_buffer,
			index_count: 6,
			map_albedo: None,
			map_arm: None,
			map_normal: None,
		}
	}
}

pub trait Vertex {
	fn layout<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
	pub position: [f32; 3],
	pub uv: [f32; 2],
	pub normal: [f32; 3],
	pub tangent: [f32; 3],
}

impl Vertex for ModelVertex {
	fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
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
			],
		}
	}
}
