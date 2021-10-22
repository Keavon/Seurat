use crate::camera::SceneCamera;
use crate::camera_controller::CameraController;
use crate::component::Component;
use crate::instance::Instance;
use crate::light::SceneLighting;
use crate::material::{Material, MaterialDataBinding};
use crate::mesh::Mesh;
use crate::model::Model;
use crate::scene::Scene;
use crate::shader::{Shader, ShaderBinding, ShaderBindingTexture};
use crate::texture::Texture;

use cgmath::{InnerSpace, Rotation3, Zero};
use std::path::Path;
use winit::event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::{event_loop::ControlFlow, window::Window};

pub struct Engine {
	context: Context,
	z_buffer: Texture,
	frame_time: std::time::Instant,
	scene: Scene,
	active_camera: String,
	camera_controller: CameraController,
	scene_lighting: SceneLighting,
	mouse_pressed: bool,
}

impl Engine {
	// Creating some of the wgpu types requires async code
	pub async fn new(window: &Window) -> Self {
		// Mechanical details of the GPU rendering process
		let context = Context::new(window).await;

		// Prepare the texture used for the Z buffer
		let z_buffer = Texture::create_z_buffer(&context.device, &context.config, "Z buffer texture");

		// Prepare the initial time value used to calculate the delta time since last frame
		let frame_time = std::time::Instant::now();

		// Camera
		let active_camera = String::from("Main Camera");
		let camera_controller = CameraController::new(4.0, 0.4);

		// Lights
		let scene_lighting = SceneLighting::new(&context);

		// Scene
		let scene = Scene::new();

		Self {
			context,
			z_buffer,
			frame_time,
			scene,
			active_camera,
			camera_controller,
			scene_lighting,
			mouse_pressed: false,
		}
	}

	pub fn load(&mut self, assets_path: &Path) {
		self.load_resources(assets_path);
		self.load_scene();
	}

	fn load_resources(&mut self, assets_path: &Path) {
		// Shaders
		let temporary_camera = SceneCamera::new(&self.context);
		let light_shader = Shader::new(&self.context, assets_path, "lamp.wgsl", vec![], &temporary_camera, &self.scene_lighting);
		let cube_shader = {
			let diffuse = ShaderBinding::Texture(ShaderBindingTexture::default());
			let normal = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(&self.context, assets_path, "pbr.wgsl", vec![diffuse, normal], &temporary_camera, &self.scene_lighting)
		};
		self.scene.resources.shaders.insert(String::from("lamp.wgsl"), light_shader);
		self.scene.resources.shaders.insert(String::from("pbr.wgsl"), cube_shader);

		// Textures
		self.scene.resources.textures.insert(
			String::from("cube-diffuse.jpg"),
			Texture::load(&self.context.device, &self.context.queue, assets_path, "cube-diffuse.jpg", false).unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cube-normal.png"),
			Texture::load(&self.context.device, &self.context.queue, assets_path, "cube-normal.png", true).unwrap(),
		);

		// Materials
		self.scene.resources.materials.insert(
			String::from("cube.material"),
			Material::new(
				"cube.material",
				"pbr.wgsl",
				vec![MaterialDataBinding::Texture("cube-diffuse.jpg"), MaterialDataBinding::Texture("cube-normal.png")],
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("lamp.material"),
			Material::new("lamp.material", "lamp.wgsl", vec![], &self.scene.resources, &self.context.device),
		);

		// Meshes
		let meshes = Mesh::load(&self.context.device, &self.context.queue, assets_path, "cube.obj");
		for mesh in meshes.unwrap_or_default() {
			self.scene.resources.meshes.insert((String::from("cube.obj"), mesh.name.clone()), mesh);
		}
	}

	fn load_scene(&mut self) {
		// Main camera
		let main_camera = self.scene.root.new_child("Main Camera");
		main_camera.add_component(Component::Camera(SceneCamera::new(&self.context)));

		// White cube representing the light
		let lamp = self.scene.root.new_child("Lamp Model");

		let mut lamp_model = Model::new(&self.scene.resources, ("cube.obj", "Cube_Finished_Cube.001"), "lamp.material");
		lamp_model.instances.instance_list[0].location.y = 4.;
		lamp_model.instances.update_buffer(&self.context.device);
		lamp.add_component(Component::Model(lamp_model));

		let light_cube_movement = crate::scripts::light_cube_movement::LightCubeMovement;
		lamp.add_component(Component::Behavior(Box::new(light_cube_movement)));

		// Array of cubes
		let cubes = self.scene.root.new_child("Cubes");

		let mut cube_model = Model::new(&self.scene.resources, ("cube.obj", "Cube_Finished_Cube.001"), "cube.material");

		const NUM_INSTANCES_PER_ROW: u32 = 10;
		const SPACE_BETWEEN: f32 = 3.0;
		cube_model.instances.instance_list = (0..NUM_INSTANCES_PER_ROW)
			.flat_map(|z| {
				(0..NUM_INSTANCES_PER_ROW).map(move |x| {
					let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
					let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

					let location = cgmath::Vector3 { x, y: 0., z };

					let rotation = if location.is_zero() {
						cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
					} else {
						cgmath::Quaternion::from_axis_angle(location.normalize(), cgmath::Deg(45.0))
					};

					let scale = cgmath::Vector3 { x: 1., y: 1., z: 1. };

					Instance { location, rotation, scale }
				})
			})
			.collect::<Vec<_>>();
		cube_model.instances.update_buffer(&self.context.device);

		cubes.add_component(Component::Model(cube_model));
	}

	fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.context.config.width = new_size.width;
			self.context.config.height = new_size.height;
			self.context.surface.configure(&self.context.device, &self.context.config);

			self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0]
				.projection
				.resize(new_size.width, new_size.height);

			self.z_buffer = Texture::create_z_buffer(&self.context.device, &self.context.config, "Z buffer texture");
		}
	}

	pub fn process_input(&mut self, event: &DeviceEvent) {
		match event {
			// Keyboard keys
			DeviceEvent::Key(KeyboardInput {
				virtual_keycode: Some(key), state, ..
			}) => {
				// self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0]
				self.camera_controller.process_keyboard(*key, *state);
			}
			// Scroll wheel movement
			DeviceEvent::MouseWheel { delta, .. } => {
				// self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0]
				self.camera_controller.process_scroll(delta);
			}
			// LMB
			DeviceEvent::Button { button: 1, state } => {
				self.mouse_pressed = *state == ElementState::Pressed;
			}
			// Mouse movement
			DeviceEvent::MouseMotion { delta } => {
				if self.mouse_pressed {
					// self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0]
					self.camera_controller.process_mouse(delta.0, delta.1);
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

	fn update(&mut self, delta_time: std::time::Duration) {
		// Camera
		let scene_camera = &mut self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0];
		self.camera_controller.update_camera(scene_camera, delta_time);
		scene_camera.update_v_p_matrices();
		self.context.queue.write_buffer(&scene_camera.camera_buffer, 0, bytemuck::cast_slice(&[scene_camera.camera_uniform]));

		// Light
		let old_position: cgmath::Vector3<_> = self.scene_lighting.light_uniform.location.into();
		let new_position = cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * delta_time.as_secs_f32())) * old_position;
		self.scene_lighting.light_uniform.location = new_position.into();
		self.context
			.queue
			.write_buffer(&self.scene_lighting.light_buffer, 0, bytemuck::cast_slice(&[self.scene_lighting.light_uniform]));

		// Call update() on all entity behaviors
		self.scene.root.update_behaviors_of_descendants();

		// TODO: Remove this hack used to set the lamp model instance to its entity's transform
		// let lamp_model = self.scene.find_entity_mut("Lamp Model").unwrap();
		// let location = lamp_model.transform.location;
		// let rotation = lamp_model.transform.rotation;
		// for model in &mut lamp_model.get_models_mut() {
		// 	model.instances.transform_single_instance(location, rotation, &self.context.device);
		// }
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

		for entity in &self.scene.root {
			for component in &entity.components {
				if let Component::Model(model) = component {
					let mesh = &self.scene.resources.meshes[model.mesh];
					let material = &self.scene.resources.materials[model.material];
					let shader = &self.scene.resources.shaders[material.shader_id];
					let scene_camera = self.scene.find_entity(self.active_camera.as_str()).unwrap().get_cameras()[0];

					let instances_buffer = model.instances.instances_buffer.as_ref();
					let instances_range = 0..model.instances.instance_list.len() as u32;

					render_pass.set_pipeline(&shader.render_pipeline);

					render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
					render_pass.set_vertex_buffer(1, instances_buffer.unwrap().slice(..));

					render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

					render_pass.set_bind_group(0, &scene_camera.camera_bind_group, &[]);
					render_pass.set_bind_group(1, &self.scene_lighting.light_bind_group, &[]);
					render_pass.set_bind_group(2, &material.bind_group, &[]);

					render_pass.draw_indexed(0..mesh.index_count, 0, instances_range);
				}
			}
		}

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
