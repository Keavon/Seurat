mod camera;
mod light;
mod model;
mod texture;

use crate::model::Vertex;
use cgmath::{InnerSpace, Rotation3, Zero};
use light::LightUniform;
use model::{Instance, InstanceRaw};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, RenderPipeline};
use winit::{
	event::*,
	event_loop::{ControlFlow, EventLoop},
	window::{Window, WindowBuilder},
};

fn main() {
	env_logger::init();

	let event_loop = EventLoop::new();
	let window = WindowBuilder::new().with_title("Seurat").build(&event_loop).unwrap();
	let mut state = pollster::block_on(Application::new(&window));
	let mut last_render_time = std::time::Instant::now();

	event_loop.run(move |event, _, control_flow| {
		*control_flow = ControlFlow::Poll;

		match event {
			Event::DeviceEvent { ref event, .. } => {
				state.input(event);
			}
			Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
				WindowEvent::CloseRequested
				| WindowEvent::KeyboardInput {
					input: KeyboardInput {
						state: ElementState::Pressed,
						virtual_keycode: Some(VirtualKeyCode::Escape),
						..
					},
					..
				} => *control_flow = ControlFlow::Exit,
				WindowEvent::Resized(physical_size) => {
					state.resize(*physical_size);
				}
				WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
					state.resize(**new_inner_size);
				}
				_ => {}
			},
			Event::RedrawRequested(_) => {
				let now = std::time::Instant::now();
				let dt = now - last_render_time;
				last_render_time = now;
				state.update(dt);

				match state.render() {
					Ok(_) => {}
					// Reconfigure the surface if lost
					Err(wgpu::SurfaceError::Lost) => state.resize(window.inner_size()),
					// The system is out of memory, we should probably quit
					Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
					// All other errors (Outdated, Timeout) should be resolved by the next frame
					Err(e) => eprintln!("{:?}", e),
				}
			}
			Event::MainEventsCleared => {
				// RedrawRequested will only trigger once, unless we manually request it
				window.request_redraw();
			}
			_ => {}
		}
	});
}

struct Context {
	surface: wgpu::Surface,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
}
impl Context {
	async fn new(window: &Window) -> Self {
		// Get the pixel resolution of the window's render area
		let viewport_size = window.inner_size();

		// The WGPU runtime
		let instance = wgpu::Instance::new(wgpu::Backends::all());

		// The viewport to draw on
		let surface = unsafe { instance.create_surface(window) };

		// Handle to the GPU
		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
			})
			.await
			.unwrap();

		// Device is the living connection to the GPU
		// Queue is where commands are submitted to the GPU
		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					features: wgpu::Features::empty(),
					limits: wgpu::Limits::default(),
					label: None,
				},
				None,
			)
			.await
			.unwrap();

		// Build the configuration for the surface
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface.get_preferred_format(&adapter).unwrap(),
			width: viewport_size.width,
			height: viewport_size.height,
			present_mode: wgpu::PresentMode::Fifo,
		};

		// Configure the surface with the properties defined above
		surface.configure(&device, &config);

		Self { surface, device, queue, config }
	}
}

struct Application {
	context: Context,
	z_buffer: texture::Texture,
	render_pipeline: wgpu::RenderPipeline,
	light_uniform: LightUniform,
	light_buffer: wgpu::Buffer,
	light_bind_group_layout: BindGroupLayout,
	light_bind_group: BindGroup,
	light_render_pipeline: RenderPipeline,
	camera: camera::Camera,
	projection: camera::Projection,
	camera_controller: camera::CameraController,
	camera_uniform: camera::CameraUniform,
	camera_buffer: wgpu::Buffer,
	camera_bind_group: BindGroup,
	obj_model: model::Model,
	instances: Vec<Instance>,
	instance_buffer: wgpu::Buffer,
	mouse_pressed: bool,
}
impl Application {
	// Creating some of the wgpu types requires async code
	async fn new(window: &Window) -> Self {
		let context = Context::new(window).await;
		let z_buffer = texture::Texture::create_z_buffer(&context.device, &context.config, "Z buffer");

		// TEXTURES

		let texture_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				// Diffuse map texture
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
					},
					count: None,
				},
				// Diffuse map sampler
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler {
						// This is only for TextureSampleType::Depth
						comparison: false,
						// This should be true if the sample_type of the texture is:
						//     TextureSampleType::Float { filterable: true }
						// Otherwise you'll get an error.
						filtering: true,
					},
					count: None,
				},
				// Normal map texture
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
						view_dimension: wgpu::TextureViewDimension::D2,
					},
					count: None,
				},
				// Normal map sampler
				wgpu::BindGroupLayoutEntry {
					binding: 3,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler { comparison: false, filtering: true },
					count: None,
				},
			],
			label: Some("texture_bind_group_layout"),
		});

		// MODELS

		const NUM_INSTANCES_PER_ROW: u32 = 10;
		const SPACE_BETWEEN: f32 = 3.0;

		let instances = (0..NUM_INSTANCES_PER_ROW)
			.flat_map(|z| {
				(0..NUM_INSTANCES_PER_ROW).map(move |x| {
					let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
					let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

					let position = cgmath::Vector3 { x, y: 0.0, z };

					let rotation = if position.is_zero() {
						cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
					} else {
						cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
					};

					Instance { position, rotation }
				})
			})
			.collect::<Vec<_>>();

		let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
		let instance_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Instance Buffer"),
			contents: bytemuck::cast_slice(&instance_data),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let res_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
		let obj_model = model::Model::load(&context.device, &context.queue, &texture_bind_group_layout, res_dir.join("cube.obj")).unwrap();

		// CAMERA

		let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
		let projection = camera::Projection::new(context.config.width, context.config.height, cgmath::Deg(45.0), 0.1, 100.0);
		let camera_controller = camera::CameraController::new(4.0, 0.4);

		let mut camera_uniform = camera::CameraUniform::new();
		camera_uniform.update_view_proj(&camera, &projection);

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

		// LIGHT

		let light_uniform = LightUniform {
			position: [2.0, 2.0, 2.0],
			_padding: 0,
			color: [1.0, 1.0, 1.0],
		};

		// We'll want to update our lights position, so we use COPY_DST
		let light_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Light VB"),
			contents: bytemuck::cast_slice(&[light_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let light_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

		let light_render_pipeline = {
			let layout = context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Light Pipeline Layout"),
				bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
				push_constant_ranges: &[],
			});
			let shader = wgpu::ShaderModuleDescriptor {
				label: Some("Light Shader"),
				source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/light.wgsl").into()),
			};
			Self::create_render_pipeline(
				&context.device,
				&layout,
				context.config.format,
				Some(texture::Texture::DEPTH_FORMAT),
				&[model::ModelVertex::desc()],
				shader,
			)
		};

		let render_pipeline_layout = context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Render Pipeline Layout"),
			bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout, &light_bind_group_layout],
			push_constant_ranges: &[],
		});

		let render_pipeline = Self::create_render_pipeline(
			&context.device,
			&render_pipeline_layout,
			context.config.format,
			Some(texture::Texture::DEPTH_FORMAT),
			&[model::ModelVertex::desc(), InstanceRaw::desc()],
			wgpu::ShaderModuleDescriptor {
				label: Some("Shader"),
				source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shader.wgsl").into()),
			},
		);

		Self {
			context,
			z_buffer,
			render_pipeline,
			light_uniform,
			light_buffer,
			light_bind_group_layout,
			light_bind_group,
			light_render_pipeline,
			camera,
			projection,
			camera_controller,
			camera_uniform,
			camera_buffer,
			camera_bind_group,
			obj_model,
			instances,
			instance_buffer,
			mouse_pressed: false,
		}
	}

	fn create_render_pipeline(
		device: &wgpu::Device,
		layout: &wgpu::PipelineLayout,
		color_format: wgpu::TextureFormat,
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
				targets: &[wgpu::ColorTargetState {
					format: color_format,
					blend: Some(wgpu::BlendState {
						alpha: wgpu::BlendComponent::REPLACE,
						color: wgpu::BlendComponent::REPLACE,
					}),
					write_mask: wgpu::ColorWrites::ALL,
				}],
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

	fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.context.config.width = new_size.width;
			self.context.config.height = new_size.height;
			self.context.surface.configure(&self.context.device, &self.context.config);

			self.projection.resize(new_size.width, new_size.height);

			self.z_buffer = texture::Texture::create_z_buffer(&self.context.device, &self.context.config, "depth_texture");
		}
	}

	fn input(&mut self, event: &DeviceEvent) -> bool {
		match event {
			DeviceEvent::Key(KeyboardInput {
				virtual_keycode: Some(key), state, ..
			}) => self.camera_controller.process_keyboard(*key, *state),
			DeviceEvent::MouseWheel { delta, .. } => {
				self.camera_controller.process_scroll(delta);
				true
			}
			DeviceEvent::Button {
				button: 1, // Left Mouse Button
				state,
			} => {
				self.mouse_pressed = *state == ElementState::Pressed;
				true
			}
			DeviceEvent::MouseMotion { delta } => {
				if self.mouse_pressed {
					self.camera_controller.process_mouse(delta.0, delta.1);
				}
				true
			}
			_ => false,
		}
	}

	fn update(&mut self, dt: std::time::Duration) {
		self.camera_controller.update_camera(&mut self.camera, dt);
		self.camera_uniform.update_view_proj(&self.camera, &self.projection);
		self.context.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

		// Update the light
		let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
		self.light_uniform.position = (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt.as_secs_f32())) * old_position).into();
		self.context.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));
	}

	fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		use crate::model::DrawLight;
		use model::DrawModel;

		let output = self.context.surface.get_current_frame()?.output;
		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
					store: true,
				},
			}],
			depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
				view: &self.z_buffer.view,
				depth_ops: Some(wgpu::Operations {
					load: wgpu::LoadOp::Clear(1.0),
					store: true,
				}),
				stencil_ops: None,
			}),
		});

		render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

		render_pass.set_pipeline(&self.light_render_pipeline);
		render_pass.draw_light_model(&self.obj_model, &self.camera_bind_group, &self.light_bind_group);

		render_pass.set_pipeline(&self.render_pipeline);
		render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32, &self.camera_bind_group, &self.light_bind_group);

		drop(render_pass);

		// submit will accept anything that implements IntoIter
		self.context.queue.submit(std::iter::once(encoder.finish()));

		Ok(())
	}
}
