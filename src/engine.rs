use crate::camera::{Camera, CameraController, CameraUniform, Projection};
use crate::entity::Entity;
use crate::light::LightUniform;
use crate::mesh::{DrawLight, DrawModel};
use crate::model::{Instance, Model};
use crate::shader::{Shader, ShaderBinding, ShaderBindingTexture};
use crate::texture::Texture;

use cgmath::{InnerSpace, Rotation3, Zero};
use std::path::Path;
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Buffer};
use winit::event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::{event_loop::ControlFlow, window::Window};

pub struct Engine {
	context: Context,
	z_buffer: Texture,
	frame_time: std::time::Instant,
	scene: Entity,
	scene_camera: SceneCamera,
	scene_lighting: SceneLighting,
	light_shader: Shader,
	cube_shader: Shader,
	cube_model: Model,
	cube_instances: Vec<Instance>,
	cube_instances_buffer: wgpu::Buffer,
	mouse_pressed: bool,
}

impl Engine {
	// Creating some of the wgpu types requires async code
	pub async fn new(window: &Window, assets_path: &Path) -> Self {
		// Mechanical details of the GPU rendering process
		let context = Context::new(window).await;

		// Prepare the texture used for the Z buffer
		let z_buffer = Texture::create_z_buffer(&context.device, &context.config, "Z buffer texture");

		// Prepare the initial time value used to calculate the delta time since last frame
		let frame_time = std::time::Instant::now();

		// Load the scene
		let scene = Entity::new();

		// Camera
		let scene_camera = SceneCamera::new(&context);

		// Lights
		let scene_lighting = SceneLighting::new(&context);

		// Shaders
		let light_shader = Shader::new(&context, assets_path, "light.wgsl", &[], &scene_camera, &scene_lighting);
		let cube_shader = {
			let diffuse = ShaderBinding::Texture(ShaderBindingTexture::default());
			let normal = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(&context, assets_path, "shader.wgsl", &[diffuse, normal], &scene_camera, &scene_lighting)
		};

		// Models
		const NUM_INSTANCES_PER_ROW: u32 = 10;
		const SPACE_BETWEEN: f32 = 3.0;

		let cube_instances = (0..NUM_INSTANCES_PER_ROW)
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

		let instance_data = cube_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
		let cube_instances_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Instance Buffer"),
			contents: bytemuck::cast_slice(&instance_data),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let cube_model = Model::load(&context.device, &context.queue, &cube_shader, assets_path, "cube.obj").unwrap();

		Self {
			context,
			z_buffer,
			frame_time,
			scene,
			scene_camera,
			scene_lighting,
			light_shader,
			cube_shader,
			cube_model,
			cube_instances,
			cube_instances_buffer,
			mouse_pressed: false,
		}
	}

	fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.context.config.width = new_size.width;
			self.context.config.height = new_size.height;
			self.context.surface.configure(&self.context.device, &self.context.config);

			self.scene_camera.projection.resize(new_size.width, new_size.height);

			self.z_buffer = Texture::create_z_buffer(&self.context.device, &self.context.config, "Z buffer texture");
		}
	}

	pub fn process_input(&mut self, event: &DeviceEvent) {
		match event {
			// Keyboard keys
			DeviceEvent::Key(KeyboardInput {
				virtual_keycode: Some(key), state, ..
			}) => {
				self.scene_camera.camera_controller.process_keyboard(*key, *state);
			}
			// Scroll wheel movement
			DeviceEvent::MouseWheel { delta, .. } => {
				self.scene_camera.camera_controller.process_scroll(delta);
			}
			// LMB
			DeviceEvent::Button { button: 1, state } => {
				self.mouse_pressed = *state == ElementState::Pressed;
			}
			// Mouse movement
			DeviceEvent::MouseMotion { delta } => {
				if self.mouse_pressed {
					self.scene_camera.camera_controller.process_mouse(delta.0, delta.1);
				}
			}
			_ => {}
		}
	}

	pub fn process_window_event(&mut self, window_event: &WindowEvent, control_flow: &mut ControlFlow) {
		match window_event {
			// Close window
			WindowEvent::KeyboardInput {
				input: KeyboardInput {
					state: ElementState::Pressed,
					virtual_keycode: Some(VirtualKeyCode::Escape),
					..
				},
				..
			}
			| WindowEvent::CloseRequested => {
				*control_flow = ControlFlow::Exit;
			}
			// Resize window
			WindowEvent::Resized(physical_size) => {
				self.resize(*physical_size);
			}
			WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
				self.resize(**new_inner_size);
			}
			_ => {}
		}
	}

	pub fn draw_frame(&mut self, window: &Window, control_flow: &mut ControlFlow) {
		let now = std::time::Instant::now();
		let dt = now - self.frame_time;
		self.frame_time = now;
		self.update(dt);

		match self.render() {
			Ok(_) => {}
			// Reconfigure the surface if lost
			Err(wgpu::SurfaceError::Lost) => self.resize(window.inner_size()),
			// The system is out of memory, we should probably quit
			Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
			// All other errors (Outdated, Timeout) should be resolved by the next frame
			Err(e) => eprintln!("{:?}", e),
		}
	}

	fn update(&mut self, dt: std::time::Duration) {
		self.scene_camera.camera_controller.update_camera(&mut self.scene_camera.camera, dt);
		self.scene_camera.camera_uniform.update_view_proj(&self.scene_camera.camera, &self.scene_camera.projection);
		self.context
			.queue
			.write_buffer(&self.scene_camera.camera_buffer, 0, bytemuck::cast_slice(&[self.scene_camera.camera_uniform]));

		// Update the light
		let old_position: cgmath::Vector3<_> = self.scene_lighting.light_uniform.position.into();
		self.scene_lighting.light_uniform.position = (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt.as_secs_f32())) * old_position).into();
		self.context
			.queue
			.write_buffer(&self.scene_lighting.light_buffer, 0, bytemuck::cast_slice(&[self.scene_lighting.light_uniform]));
	}

	fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

		render_pass.set_vertex_buffer(1, self.cube_instances_buffer.slice(..));

		render_pass.set_pipeline(&self.light_shader.render_pipeline);
		render_pass.draw_light_model(&self.cube_model, &self.scene_camera.camera_bind_group, &self.scene_lighting.light_bind_group);

		render_pass.set_pipeline(&self.cube_shader.render_pipeline);
		render_pass.draw_model_instanced(
			&self.cube_model,
			0..self.cube_instances.len() as u32,
			&self.scene_camera.camera_bind_group,
			&self.scene_lighting.light_bind_group,
		);

		drop(render_pass);

		// submit will accept anything that implements IntoIter
		self.context.queue.submit(std::iter::once(encoder.finish()));

		Ok(())
	}
}

pub struct Context {
	pub surface: wgpu::Surface,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub config: wgpu::SurfaceConfiguration,
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

pub struct SceneLighting {
	pub light_uniform: LightUniform,
	pub light_buffer: wgpu::Buffer,
	pub light_bind_group_layout: BindGroupLayout,
	pub light_bind_group: BindGroup,
}

impl SceneLighting {
	pub fn new(context: &Context) -> Self {
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

		Self {
			light_uniform,
			light_buffer,
			light_bind_group_layout,
			light_bind_group,
		}
	}
}

pub struct SceneCamera {
	pub camera: Camera,
	pub projection: Projection,
	pub camera_controller: CameraController,
	pub camera_uniform: CameraUniform,
	pub camera_buffer: Buffer,
	pub camera_bind_group_layout: BindGroupLayout,
	pub camera_bind_group: BindGroup,
}

impl SceneCamera {
	pub fn new(context: &Context) -> Self {
		let camera = Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
		let projection = Projection::new(context.config.width, context.config.height, cgmath::Deg(45.0), 0.1, 100.0);
		let camera_controller = CameraController::new(4.0, 0.4);

		let mut camera_uniform = CameraUniform::new();
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

		Self {
			camera,
			projection,
			camera_controller,
			camera_uniform,
			camera_buffer,
			camera_bind_group_layout,
			camera_bind_group,
		}
	}
}
