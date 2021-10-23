use crate::camera::SceneCamera;
use crate::camera_controller::CameraController;
use crate::component::Component;
use crate::context::Context;
use crate::frame_texture::{FrameTexture, FrameTextureTypes, FrameTextures};
use crate::instance::Instance;
use crate::light::SceneLighting;
use crate::material::{Material, MaterialDataBinding};
use crate::mesh::Mesh;
use crate::model::Model;
use crate::pass::Pass;
use crate::scene::Scene;
use crate::shader::{Shader, ShaderBinding, ShaderBindingTexture};
use crate::texture::Texture;

use cgmath::{InnerSpace, Rotation3, Zero};
use std::path::Path;
use winit::event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::{event_loop::ControlFlow, window::Window};

pub struct Engine {
	context: Context,
	frame_textures: FrameTextures,
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

		// Prepare the frame textures
		let z_buffer = FrameTexture::new(
			&context.device,
			&context.config,
			wgpu::TextureFormat::Depth32Float,
			"Z-buffer frame texture",
			Some(wgpu::CompareFunction::LessEqual),
		);
		let albedo = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Bgra8UnormSrgb, "Albedo frame texture", None);
		let arm = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Bgra8UnormSrgb, "ARM frame texture", None);
		let frame_textures = FrameTextures { z_buffer, albedo, arm };
		// UPDATE HERE TO ADD FRAME TEXTURE

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
			frame_textures,
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

		let blit_shader = {
			let color = ShaderBinding::Texture(ShaderBindingTexture::default());
			let arm = ShaderBinding::Texture(ShaderBindingTexture::default());
			// UPDATE HERE TO ADD FRAME TEXTURE

			Shader::new(&self.context, assets_path, "blit.wgsl", vec![color, arm], false, 1, None, None)
		};
		self.scene.resources.shaders.insert(String::from("blit.wgsl"), blit_shader);

		let light_shader = Shader::new(&self.context, assets_path, "lamp.wgsl", vec![], true, 1, Some(&temporary_camera), Some(&self.scene_lighting));
		self.scene.resources.shaders.insert(String::from("lamp.wgsl"), light_shader);

		let pbr_shader = {
			let albedo = ShaderBinding::Texture(ShaderBindingTexture::default()); // Albedo map
			let arm = ShaderBinding::Texture(ShaderBindingTexture::default()); // AO/Roughness/Metalness map
			let normal = ShaderBinding::Texture(ShaderBindingTexture::default()); // Normal map

			Shader::new(
				&self.context,
				assets_path,
				"pbr.wgsl",
				vec![albedo, arm, normal],
				true,
				2, // UPDATE HERE TO ADD FRAME TEXTURE
				Some(&temporary_camera),
				Some(&self.scene_lighting),
			)
		};
		self.scene.resources.shaders.insert(String::from("pbr.wgsl"), pbr_shader);

		// Textures
		self.scene.resources.textures.insert(
			String::from("cube_albedo.jpg"),
			Texture::load(&self.context.device, &self.context.queue, assets_path, "cube_albedo.jpg", false).unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cube_arm.jpg"),
			Texture::load(&self.context.device, &self.context.queue, assets_path, "cube_arm.jpg", true).unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cube_normal.jpg"),
			Texture::load(&self.context.device, &self.context.queue, assets_path, "cube_normal.jpg", true).unwrap(),
		);

		// Materials
		self.scene.resources.materials.insert(
			String::from("BLIT_QUAD.material"),
			Material::new_blit_quad(
				"BLIT_QUAD.material",
				"blit.wgsl",
				vec![&self.frame_textures.albedo.texture, &self.frame_textures.arm.texture], // UPDATE HERE TO ADD FRAME TEXTURE
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("pbr.material"),
			Material::new(
				"pbr.material",
				"pbr.wgsl",
				vec![
					MaterialDataBinding::Texture("cube_albedo.jpg"),
					MaterialDataBinding::Texture("cube_arm.jpg"),
					MaterialDataBinding::Texture("cube_normal.jpg"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("lamp.material"),
			Material::new("lamp.material", "lamp.wgsl", vec![], &self.scene.resources, &self.context.device),
		);

		// Meshes
		let blit_quad_mesh = Mesh::new_blit_quad(&self.context.device, &self.context.queue);
		self.scene.resources.meshes.insert((String::from("BLIT"), String::from("QUAD")), blit_quad_mesh);

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

		let mut cube_model = Model::new(&self.scene.resources, ("cube.obj", "Cube_Finished_Cube.001"), "pbr.material");

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

			self.frame_textures.recreate_all(&self.context.device, &self.context.config);
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
		let new_position = cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(20.0 * delta_time.as_secs_f32())) * old_position;
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
		let surface_texture = self.context.surface.get_current_texture()?;
		let surface_texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let frame_texture_from_id = |frame_texture_type: FrameTextureTypes| match frame_texture_type {
			FrameTextureTypes::Surface => &surface_texture_view,
			FrameTextureTypes::ZBuffer => &self.frame_textures.z_buffer.texture.view,
			FrameTextureTypes::Albedo => &self.frame_textures.albedo.texture.view,
			FrameTextureTypes::Arm => &self.frame_textures.arm.texture.view,
		};

		let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let passes = vec![
			Pass {
				label: String::from("Forward"),
				depth_attachment: true,
				color_attachment_types: vec![FrameTextureTypes::Albedo, FrameTextureTypes::Arm], // UPDATE HERE TO ADD FRAME TEXTURE
				draw_quad_not_scene: false,
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			},
			Pass {
				label: String::from("Deferred"),
				depth_attachment: true,
				color_attachment_types: vec![FrameTextureTypes::Surface],
				draw_quad_not_scene: true,
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			},
		];

		for pass in passes {
			let color_attachments = pass
				.color_attachment_types
				.iter()
				.map(|frame_texture_type| wgpu::RenderPassColorAttachment {
					view: frame_texture_from_id(*frame_texture_type),
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(pass.clear_color),
						store: true,
					},
				})
				.collect::<Vec<wgpu::RenderPassColorAttachment>>();

			let depth_stencil_attachment = pass.depth_attachment.then(|| wgpu::RenderPassDepthStencilAttachment {
				view: frame_texture_from_id(FrameTextureTypes::ZBuffer),
				depth_ops: Some(wgpu::Operations {
					load: wgpu::LoadOp::Clear(1.0),
					store: true,
				}),
				stencil_ops: None,
			});

			let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some(pass.label.as_str()),
				color_attachments: color_attachments.as_slice(),
				depth_stencil_attachment,
			});

			if pass.draw_quad_not_scene {
				self.scene.resources.materials.insert(
					String::from("blit.wgsl"),
					Material::new_blit_quad(
						"BLIT_QUAD.material",
						"blit.wgsl",
						vec![&self.frame_textures.albedo.texture, &self.frame_textures.arm.texture], // UPDATE HERE TO ADD FRAME TEXTURE
						&self.scene.resources,
						&self.context.device,
					),
				);

				self.draw_quad(render_pass);
			} else {
				self.draw_scene(render_pass);
			}
		}

		self.context.queue.submit(std::iter::once(encoder.finish()));
		surface_texture.present();

		Ok(())
	}

	fn draw_scene<'a>(&'a self, mut render_pass: wgpu::RenderPass<'a>) {
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
	}

	fn draw_quad<'a>(&'a self, mut render_pass: wgpu::RenderPass<'a>) {
		let mesh = &self.scene.resources.meshes.get(&(String::from("BLIT"), String::from("QUAD"))).unwrap();
		let material = &self.scene.resources.materials.get("BLIT_QUAD.material").unwrap();
		let shader = &self.scene.resources.shaders[material.shader_id];

		render_pass.set_pipeline(&shader.render_pipeline);

		render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

		render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

		render_pass.set_bind_group(0, &material.bind_group, &[]);

		render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
	}
}
