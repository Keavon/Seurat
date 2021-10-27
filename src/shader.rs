use crate::camera::SceneCamera;
use crate::context::Context;
use crate::instance::InstanceRaw;
use crate::light::SceneLighting;
use crate::mesh::{ModelVertex, Vertex};

use std::path::Path;
use wgpu::{BindGroupLayout, PipelineLayout, RenderPipeline};

pub struct Shader {
	pub bind_group_layout: BindGroupLayout,
	pub render_pipeline: RenderPipeline,
	pub render_pipeline_layout: PipelineLayout,
	pub shader_bindings: Vec<ShaderBinding>,
	pub includes_camera: bool,
	pub includes_lighting: bool,
}

impl Shader {
	pub fn new(
		context: &Context,
		directory: &Path,
		file: &str,
		in_shader_bindings: Vec<ShaderBinding>,
		out_color_formats: Vec<wgpu::TextureFormat>,
		depth_format: Option<wgpu::TextureFormat>,
		use_instances: bool,
		scene_camera: Option<&SceneCamera>,
		scene_lighting: Option<&SceneLighting>,
	) -> Self {
		let bind_group_layout_entries = build_bind_group_layout_entries(in_shader_bindings.as_slice());
		let bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: bind_group_layout_entries.as_slice(),
			label: Some(format!("Shader \"{}\" bind group layout", file).as_str()),
		});

		let camera_layout = scene_camera.map(|camera| &camera.camera_bind_group_layout);
		let lighting_layout = scene_lighting.map(|lighting| &lighting.light_bind_group_layout);
		let layout = Some(&bind_group_layout);
		let layouts = vec![camera_layout, lighting_layout, layout].into_iter().flatten().collect::<Vec<_>>();

		let bind_group_layouts = layouts.as_slice();
		let render_pipeline_layout = context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some(format!("Shader \"{}\" render pipeline layout", file).as_str()),
			bind_group_layouts,
			push_constant_ranges: &[],
		});

		let vertex_layouts = if use_instances {
			vec![ModelVertex::layout(), InstanceRaw::layout()]
		} else {
			vec![ModelVertex::layout()]
		};

		let shader_path = directory.join("shaders").join(file);
		let shader_code = std::fs::read_to_string(shader_path).unwrap();

		let render_pipeline = create_render_pipeline(
			&context.device,
			&render_pipeline_layout,
			out_color_formats,
			depth_format,
			vertex_layouts.as_slice(),
			wgpu::ShaderModuleDescriptor {
				label: Some(format!("Shader \"{}\" module descriptor", file).as_str()),
				source: wgpu::ShaderSource::Wgsl(shader_code.into()),
			},
		);

		Self {
			bind_group_layout,
			render_pipeline,
			render_pipeline_layout,
			shader_bindings: in_shader_bindings,
			includes_camera: scene_camera.is_some(),
			includes_lighting: scene_lighting.is_some(),
		}
	}
}

fn build_bind_group_layout_entries(bindings: &[ShaderBinding]) -> Vec<wgpu::BindGroupLayoutEntry> {
	let mut binding_index = 0;

	bindings
		.iter()
		.flat_map(|binding| match binding {
			ShaderBinding::Buffer(buffer) => {
				let binding = binding_index;
				binding_index += 1;

				vec![wgpu::BindGroupLayoutEntry {
					binding,
					visibility: buffer.visible_in_stages,
					ty: wgpu::BindingType::Buffer {
						ty: buffer.uniform_or_storage,
						has_dynamic_offset: buffer.has_dynamic_offset,
						min_binding_size: buffer.min_binding_size,
					},
					count: None,
				}]
			}
			ShaderBinding::Texture(texture) => {
				let binding = binding_index;
				binding_index += 2;

				vec![
					wgpu::BindGroupLayoutEntry {
						binding,
						visibility: texture.visible_in_stages,
						ty: wgpu::BindingType::Texture {
							multisampled: texture.multisampled,
							view_dimension: texture.dimensions,
							sample_type: texture.sampled_value_data_type,
						},
						count: None,
					},
					wgpu::BindGroupLayoutEntry {
						binding: binding + 1,
						visibility: texture.visible_in_stages,
						ty: wgpu::BindingType::Sampler {
							comparison: false,
							filtering: texture.sampled_value_data_type == wgpu::TextureSampleType::Float { filterable: true },
						},
						count: None,
					},
				]
			}
		})
		.collect()
}

fn create_render_pipeline(
	device: &wgpu::Device,
	layout: &wgpu::PipelineLayout,
	color_formats: Vec<wgpu::TextureFormat>,
	depth_format: Option<wgpu::TextureFormat>,
	vertex_layouts: &[wgpu::VertexBufferLayout],
	shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
	let shader = device.create_shader_module(&shader);

	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Render Pipeline"),
		layout: Some(layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: "main",
			buffers: vertex_layouts,
		},
		fragment: Some(wgpu::FragmentState {
			module: &shader,
			entry_point: "main",
			targets: color_formats
				.into_iter()
				.map(|format| wgpu::ColorTargetState {
					format,
					blend: Some(wgpu::BlendState {
						alpha: wgpu::BlendComponent::REPLACE,
						color: wgpu::BlendComponent::REPLACE,
					}),
					write_mask: wgpu::ColorWrites::ALL,
				})
				.collect::<Vec<_>>()
				.as_slice(),
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: Some(wgpu::Face::Back),
			// Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
			polygon_mode: wgpu::PolygonMode::Fill,
			// Requires Features::DEPTH_CLAMPING
			clamp_depth: false,
			// Requires Features::CONSERVATIVE_RASTERIZATION
			conservative: false,
		},
		depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
			format,
			depth_write_enabled: true,
			depth_compare: wgpu::CompareFunction::Less,
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState {
			count: 1,
			mask: !0,
			alpha_to_coverage_enabled: false,
		},
	})
}

pub enum ShaderBinding {
	Buffer(ShaderBindingBuffer),
	Texture(ShaderBindingTexture),
}

pub struct ShaderBindingBuffer {
	pub visible_in_stages: wgpu::ShaderStages,
	pub uniform_or_storage: wgpu::BufferBindingType,
	pub has_dynamic_offset: bool,
	pub min_binding_size: Option<std::num::NonZeroU64>,
}
impl Default for ShaderBindingBuffer {
	fn default() -> Self {
		Self {
			visible_in_stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
			uniform_or_storage: wgpu::BufferBindingType::Uniform,
			has_dynamic_offset: false,
			min_binding_size: None,
		}
	}
}

pub struct ShaderBindingTexture {
	pub visible_in_stages: wgpu::ShaderStages,
	pub multisampled: bool,
	pub dimensions: wgpu::TextureViewDimension,
	pub sampled_value_data_type: wgpu::TextureSampleType,
}
impl Default for ShaderBindingTexture {
	fn default() -> Self {
		Self {
			visible_in_stages: wgpu::ShaderStages::FRAGMENT,
			multisampled: false,
			dimensions: wgpu::TextureViewDimension::D2,
			sampled_value_data_type: wgpu::TextureSampleType::Float { filterable: true },
		}
	}
}
