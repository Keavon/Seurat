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
use crate::shader::{Shader, ShaderBinding, ShaderBindingBuffer, ShaderBindingTexture};
use crate::texture::Texture;

use cgmath::{InnerSpace, Rotation3, Zero};
use std::path::Path;
use wgpu::util::DeviceExt;
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
		let world_space_fragment_location = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "WorldSpaceFragmentLocation frame texture", None);
		let world_space_normal = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "WorldSpaceNormal frame texture", None);
		let world_space_eye_location = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "WorldSpaceEyeLocation frame texture", None);
		let world_space_light_location = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "WorldSpaceLightLocation frame texture", None);

		let view_space_fragment_location = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "ViewSpaceFragmentLocation frame texture", None);
		let view_space_normal = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "ViewSpaceNormal frame texture", None);
		let view_space_eye_location = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "ViewSpaceEyeLocation frame texture", None);
		let view_space_light_location = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "ViewSpaceLightLocation frame texture", None);

		let albedo_map = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Bgra8UnormSrgb, "AlbedoMap frame texture", None);
		let arm_map = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Bgra8Unorm, "ArmMap frame texture", None);
		let normal_map = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Bgra8Unorm, "NormalMap frame texture", None);

		let ssao_kernel_map = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "SSAOKernelMap frame texture", None);
		let ssao_blurred_map = FrameTexture::new(&context.device, &context.config, wgpu::TextureFormat::Rgba16Float, "SSAOBlurredMap frame texture", None);

		let frame_textures = FrameTextures {
			z_buffer,
			world_space_fragment_location,
			world_space_normal,
			world_space_eye_location,
			world_space_light_location,
			view_space_fragment_location,
			view_space_normal,
			view_space_eye_location,
			view_space_light_location,
			albedo_map,
			arm_map,
			normal_map,
			ssao_kernel_map,
			ssao_blurred_map,
		}; // UPDATE HERE TO ADD FRAME TEXTURE

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

		let pbr_deferred_world_space_shader = {
			let albedo_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Albedo map
			let arm_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // AO/Roughness/Metalness map
			let normal_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Normal map

			Shader::new(
				&self.context,
				assets_path,
				"pbr_deferred_world_space.wgsl",
				vec![albedo_map, arm_map, normal_map],
				vec![
					wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Rgba16Float,
					// wgpu::TextureFormat::Bgra8UnormSrgb,
					// wgpu::TextureFormat::Bgra8Unorm,
					// wgpu::TextureFormat::Bgra8Unorm,
				],
				true,
				Some(&temporary_camera),
				Some(&self.scene_lighting),
			)
		};
		self.scene.resources.shaders.insert(String::from("pbr_deferred_world_space.wgsl"), pbr_deferred_world_space_shader);

		let pbr_deferred_view_space_shader = {
			let albedo_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Albedo map
			let arm_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // AO/Roughness/Metalness map
			let normal_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Normal map

			Shader::new(
				&self.context,
				assets_path,
				"pbr_deferred_view_space.wgsl",
				vec![albedo_map, arm_map, normal_map],
				vec![
					wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Rgba16Float,
					// wgpu::TextureFormat::Bgra8UnormSrgb,
					// wgpu::TextureFormat::Bgra8Unorm,
					// wgpu::TextureFormat::Bgra8Unorm,
				],
				true,
				Some(&temporary_camera),
				Some(&self.scene_lighting),
			)
		};
		self.scene.resources.shaders.insert(String::from("pbr_deferred_view_space.wgsl"), pbr_deferred_view_space_shader);

		let pbr_deferred_pbr_data_shader = {
			let albedo_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Albedo map
			let arm_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // AO/Roughness/Metalness map
			let normal_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Normal map

			Shader::new(
				&self.context,
				assets_path,
				"pbr_deferred_pbr_data.wgsl",
				vec![albedo_map, arm_map, normal_map],
				vec![
					// wgpu::TextureFormat::Rgba16Float,
					// wgpu::TextureFormat::Rgba16Float,
					// wgpu::TextureFormat::Rgba16Float,
					wgpu::TextureFormat::Bgra8UnormSrgb,
					wgpu::TextureFormat::Bgra8Unorm,
					wgpu::TextureFormat::Bgra8Unorm,
				],
				true,
				Some(&temporary_camera),
				Some(&self.scene_lighting),
			)
		};
		self.scene.resources.shaders.insert(String::from("pbr_deferred_pbr_data.wgsl"), pbr_deferred_pbr_data_shader);

		let lamp_shader = Shader::new(
			&self.context,
			assets_path,
			"lamp.wgsl",
			vec![],
			vec![
				wgpu::TextureFormat::Rgba16Float,
				wgpu::TextureFormat::Rgba16Float,
				wgpu::TextureFormat::Rgba16Float,
				wgpu::TextureFormat::Rgba16Float,
				// wgpu::TextureFormat::Bgra8UnormSrgb,
				// wgpu::TextureFormat::Bgra8Unorm,
				// wgpu::TextureFormat::Bgra8Unorm,
			],
			true,
			Some(&temporary_camera),
			Some(&self.scene_lighting),
		);
		self.scene.resources.shaders.insert(String::from("lamp.wgsl"), lamp_shader);

		let ssao_kernel_blit_shader = {
			let view_matrix = ShaderBinding::Buffer(ShaderBindingBuffer::default());
			let samples_array = ShaderBinding::Buffer(ShaderBindingBuffer::default());
			let ssao_noise_texture = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_fragment_location = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_normal = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(
				&self.context,
				assets_path,
				"ssao_kernel_blit.wgsl",
				vec![view_matrix, samples_array, ssao_noise_texture, world_space_fragment_location, world_space_normal],
				vec![wgpu::TextureFormat::Rgba16Float],
				false,
				None,
				None,
			)
		};
		self.scene.resources.shaders.insert(String::from("ssao_kernel_blit.wgsl"), ssao_kernel_blit_shader);

		let ssao_blurred_blit_shader = {
			let ssao_kernel = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(
				&self.context,
				assets_path,
				"ssao_blurred_blit.wgsl",
				vec![ssao_kernel],
				vec![wgpu::TextureFormat::Rgba16Float],
				false,
				None,
				None,
			)
		};
		self.scene.resources.shaders.insert(String::from("ssao_blurred_blit.wgsl"), ssao_blurred_blit_shader);

		let pbr_blit_world_space_shader = {
			let world_space_fragment_location = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_normal = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_eye_location = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_light_location = ShaderBinding::Texture(ShaderBindingTexture::default());
			let albedo_map = ShaderBinding::Texture(ShaderBindingTexture::default());
			let arm_map = ShaderBinding::Texture(ShaderBindingTexture::default());
			let normal_map = ShaderBinding::Texture(ShaderBindingTexture::default());
			let ssao_blurred_map = ShaderBinding::Texture(ShaderBindingTexture::default());
			// UPDATE HERE TO ADD FRAME TEXTURE

			Shader::new(
				&self.context,
				assets_path,
				"pbr_blit_world_space.wgsl",
				vec![
					world_space_fragment_location,
					world_space_normal,
					world_space_eye_location,
					world_space_light_location,
					albedo_map,
					arm_map,
					normal_map,
					ssao_blurred_map,
				],
				vec![self.context.config.format],
				false,
				None,
				None,
			)
		};
		self.scene.resources.shaders.insert(String::from("pbr_blit_world_space.wgsl"), pbr_blit_world_space_shader);

		// Textures
		self.scene.resources.textures.insert(
			String::from("SSAO_NOISE"),
			Texture::from_f16_array(
				&self.context.device,
				&self.context.queue,
				crate::ssao::generate_noise_texture().as_slice(),
				(4, 4),
				"SSAO_NOISE",
				wgpu::TextureFormat::Rgba16Float,
				wgpu::AddressMode::Repeat,
			),
		);

		self.scene.resources.textures.insert(
			String::from("cube_albedo.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"cube_albedo.jpg",
				wgpu::TextureFormat::Rgba8UnormSrgb,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cube_arm.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"cube_arm.jpg",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cube_normal.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"cube_normal.jpg",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);

		self.scene.resources.textures.insert(
			String::from("white_albedo.png"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"white_albedo.png",
				wgpu::TextureFormat::Rgba8UnormSrgb,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("white_arm.png"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"white_arm.png",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("white_normal.png"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"white_normal.png",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);

		// Meshes
		let blit_quad_mesh = Mesh::new_blit_quad(&self.context.device, &self.context.queue);
		self.scene.resources.meshes.insert((String::from("BLIT"), String::from("QUAD")), blit_quad_mesh);

		let meshes = Mesh::load(&self.context.device, &self.context.queue, assets_path, "cube.obj");
		for mesh in meshes.unwrap_or_default() {
			self.scene.resources.meshes.insert((String::from("cube.obj"), mesh.name.clone()), mesh);
		}

		let meshes = Mesh::load(&self.context.device, &self.context.queue, assets_path, "sponza.obj");
		for mesh in meshes.unwrap_or_default() {
			self.scene.resources.meshes.insert((String::from("sponza.obj"), mesh.name.clone()), mesh);
		}
	}

	fn load_materials(&mut self, camera: &SceneCamera) {
		// Materials
		self.scene.resources.materials.insert(
			String::from("pbr_deferred_world_space_cubes.material"),
			Material::new(
				"pbr_deferred_world_space_cubes.material",
				"pbr_deferred_world_space.wgsl",
				vec![
					MaterialDataBinding::TextureName("cube_albedo.jpg"),
					MaterialDataBinding::TextureName("cube_arm.jpg"),
					MaterialDataBinding::TextureName("cube_normal.jpg"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("pbr_deferred_view_space_cubes.material"),
			Material::new(
				"pbr_deferred_view_space_cubes.material",
				"pbr_deferred_view_space.wgsl",
				vec![
					MaterialDataBinding::TextureName("cube_albedo.jpg"),
					MaterialDataBinding::TextureName("cube_arm.jpg"),
					MaterialDataBinding::TextureName("cube_normal.jpg"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("pbr_deferred_pbr_data_cubes.material"),
			Material::new(
				"pbr_deferred_pbr_data_cubes.material",
				"pbr_deferred_pbr_data.wgsl",
				vec![
					MaterialDataBinding::TextureName("cube_albedo.jpg"),
					MaterialDataBinding::TextureName("cube_arm.jpg"),
					MaterialDataBinding::TextureName("cube_normal.jpg"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("pbr_deferred_world_space_white.material"),
			Material::new(
				"pbr_deferred_world_space_white.material",
				"pbr_deferred_world_space.wgsl",
				vec![
					MaterialDataBinding::TextureName("white_albedo.png"),
					MaterialDataBinding::TextureName("white_arm.png"),
					MaterialDataBinding::TextureName("white_normal.png"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("pbr_deferred_view_space_white.material"),
			Material::new(
				"pbr_deferred_view_space_white.material",
				"pbr_deferred_view_space.wgsl",
				vec![
					MaterialDataBinding::TextureName("white_albedo.png"),
					MaterialDataBinding::TextureName("white_arm.png"),
					MaterialDataBinding::TextureName("white_normal.png"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);
		self.scene.resources.materials.insert(
			String::from("pbr_deferred_pbr_data_white.material"),
			Material::new(
				"pbr_deferred_pbr_data_white.material",
				"pbr_deferred_pbr_data.wgsl",
				vec![
					MaterialDataBinding::TextureName("white_albedo.png"),
					MaterialDataBinding::TextureName("white_arm.png"),
					MaterialDataBinding::TextureName("white_normal.png"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("lamp.material"),
			Material::new("lamp.material", "lamp.wgsl", vec![], &self.scene.resources, &self.context.device),
		);

		let ssao_samples_buffer = self.context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("SSAO samples buffer"),
			contents: bytemuck::cast_slice(&crate::ssao::generate_sample_hemisphere()),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});
		self.scene.resources.materials.insert(
			String::from("ssao_kernel_blit.material"),
			Material::new(
				"ssao_kernel_blit.material",
				"ssao_kernel_blit.wgsl",
				vec![
					MaterialDataBinding::Buffer(wgpu::BufferBinding {
						buffer: &camera.camera_buffer,
						offset: 0,
						size: None,
					}),
					MaterialDataBinding::Buffer(wgpu::BufferBinding {
						buffer: &ssao_samples_buffer,
						offset: 0,
						size: None,
					}),
					MaterialDataBinding::TextureName("SSAO_NOISE"),
					MaterialDataBinding::Texture(&self.frame_textures.view_space_fragment_location.texture),
					MaterialDataBinding::Texture(&self.frame_textures.view_space_normal.texture),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("ssao_blurred_blit.material"),
			Material::new(
				"ssao_blurred_blit.material",
				"ssao_blurred_blit.wgsl",
				vec![MaterialDataBinding::Texture(&self.frame_textures.ssao_kernel_map.texture)],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("pbr_blit_world_space.material"),
			Material::new(
				"pbr_blit_world_space.material",
				"pbr_blit_world_space.wgsl",
				vec![
					MaterialDataBinding::Texture(&self.frame_textures.world_space_fragment_location.texture),
					MaterialDataBinding::Texture(&self.frame_textures.world_space_normal.texture),
					MaterialDataBinding::Texture(&self.frame_textures.world_space_eye_location.texture),
					MaterialDataBinding::Texture(&self.frame_textures.world_space_light_location.texture),
					MaterialDataBinding::Texture(&self.frame_textures.albedo_map.texture),
					MaterialDataBinding::Texture(&self.frame_textures.arm_map.texture),
					MaterialDataBinding::Texture(&self.frame_textures.normal_map.texture),
					MaterialDataBinding::Texture(&self.frame_textures.ssao_blurred_map.texture),
					// UPDATE HERE TO ADD FRAME TEXTURE
				],
				&self.scene.resources,
				&self.context.device,
			),
		);
	}

	fn load_scene(&mut self) {
		let scene_camera = SceneCamera::new(&self.context);
		self.load_materials(&scene_camera);

		// Main camera
		let main_camera = self.scene.root.new_child("Main Camera");
		main_camera.add_component(Component::Camera(scene_camera));

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

		let mut cube_model = Model::new(&self.scene.resources, ("cube.obj", "Cube_Finished_Cube.001"), "pbr_deferred_world_space_cubes.material");

		const NUM_INSTANCES_PER_ROW: u32 = 10;
		const SPACE_BETWEEN: f32 = 1.0;
		cube_model.instances.instance_list = (0..NUM_INSTANCES_PER_ROW)
			.flat_map(|z| {
				(0..NUM_INSTANCES_PER_ROW).map(move |x| {
					let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
					let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

					let location = cgmath::Vector3 { x, y: 0.4, z };

					let rotation = if location.is_zero() {
						cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
					} else {
						cgmath::Quaternion::from_axis_angle(location.normalize(), cgmath::Deg(45.0))
					};

					let scale = cgmath::Vector3 { x: 0.25, y: 0.25, z: 0.25 };

					Instance { location, rotation, scale }
				})
			})
			.collect::<Vec<_>>();
		cube_model.instances.update_buffer(&self.context.device);

		cubes.add_component(Component::Model(cube_model));

		// Sponza
		let sponza = self.scene.root.new_child("Sponza");

		let mut sponza_model = Model::new(&self.scene.resources, ("sponza.obj", "sponza"), "pbr_deferred_world_space_white.material");
		sponza_model.instances.update_buffer(&self.context.device);

		sponza.add_component(Component::Model(sponza_model));
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
		scene_camera.update_v_p_matrices(&mut self.context.queue);

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
			FrameTextureTypes::WorldSpaceFragmentLocation => &self.frame_textures.world_space_fragment_location.texture.view,
			FrameTextureTypes::WorldSpaceNormal => &self.frame_textures.world_space_normal.texture.view,
			FrameTextureTypes::WorldSpaceEyeLocation => &self.frame_textures.world_space_eye_location.texture.view,
			FrameTextureTypes::WorldSpaceLightLocation => &self.frame_textures.world_space_light_location.texture.view,
			FrameTextureTypes::ViewSpaceFragmentLocation => &self.frame_textures.view_space_fragment_location.texture.view,
			FrameTextureTypes::ViewSpaceNormal => &self.frame_textures.view_space_normal.texture.view,
			FrameTextureTypes::ViewSpaceEyeLocation => &self.frame_textures.view_space_eye_location.texture.view,
			FrameTextureTypes::ViewSpaceLightLocation => &self.frame_textures.view_space_light_location.texture.view,
			FrameTextureTypes::AlbedoMap => &self.frame_textures.albedo_map.texture.view,
			FrameTextureTypes::ArmMap => &self.frame_textures.arm_map.texture.view,
			FrameTextureTypes::NormalMap => &self.frame_textures.normal_map.texture.view,
			FrameTextureTypes::SSAOKernelMap => &self.frame_textures.ssao_kernel_map.texture.view,
			FrameTextureTypes::SSAOBlurredMap => &self.frame_textures.ssao_blurred_map.texture.view,
			// UPDATE HERE TO ADD FRAME TEXTURE
		};

		let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let passes = vec![
			Pass {
				label: String::from("Forward: Deferred World Space"),
				depth_attachment: true,
				color_attachment_types: vec![
					FrameTextureTypes::WorldSpaceFragmentLocation,
					FrameTextureTypes::WorldSpaceNormal,
					FrameTextureTypes::WorldSpaceEyeLocation,
					FrameTextureTypes::WorldSpaceLightLocation,
				],
				blit_material: None,
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
				repeat: 0,
			},
			Pass {
				label: String::from("Forward: Deferred View Space"),
				depth_attachment: true,
				color_attachment_types: vec![
					FrameTextureTypes::ViewSpaceFragmentLocation,
					FrameTextureTypes::ViewSpaceNormal,
					FrameTextureTypes::ViewSpaceEyeLocation,
					FrameTextureTypes::ViewSpaceLightLocation,
				],
				blit_material: None,
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
				repeat: 1,
			},
			Pass {
				label: String::from("Forward: Deferred PBR Data"),
				depth_attachment: true,
				color_attachment_types: vec![FrameTextureTypes::AlbedoMap, FrameTextureTypes::ArmMap, FrameTextureTypes::NormalMap],
				blit_material: None,
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
				repeat: 2,
			},
			Pass {
				label: String::from("SSAO: Kernel Blit"),
				depth_attachment: true,
				color_attachment_types: vec![FrameTextureTypes::SSAOKernelMap],
				blit_material: Some(String::from("ssao_kernel_blit.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
				repeat: 0,
			},
			Pass {
				label: String::from("SSAO: Blurred Blit"),
				depth_attachment: true,
				color_attachment_types: vec![FrameTextureTypes::SSAOBlurredMap],
				blit_material: Some(String::from("ssao_blurred_blit.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
				repeat: 0,
			},
			Pass {
				label: String::from("Deferred: Blit to Surface"),
				depth_attachment: true,
				color_attachment_types: vec![FrameTextureTypes::Surface],
				blit_material: Some(String::from("pbr_blit_world_space.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
				repeat: 0,
			},
		];

		for pass in passes {
			// println!("Beginning pass: {}", pass.label);

			let color_attachments = pass
				.color_attachment_types
				.into_iter()
				.map(|frame_texture_type| wgpu::RenderPassColorAttachment {
					view: frame_texture_from_id(frame_texture_type),
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

			match pass.blit_material {
				None => self.draw_scene(render_pass, pass.repeat),
				Some(material_name) => self.draw_quad(render_pass, material_name.as_str()),
			}

			// println!("End of pass: {}", pass.label);
		}

		self.context.queue.submit(std::iter::once(encoder.finish()));
		surface_texture.present();

		Ok(())
	}

	fn draw_scene<'a>(&'a self, mut render_pass: wgpu::RenderPass<'a>, repeat: usize) {
		for entity in &self.scene.root {
			for component in &entity.components {
				if let Component::Model(model) = component {
					let mesh = &self.scene.resources.meshes[model.mesh];
					let mut material = &self.scene.resources.materials[model.material];
					match (repeat, material.name.as_str()) {
						(1, "pbr_deferred_world_space_cubes.material") => {
							material = self.scene.resources.materials.get("pbr_deferred_view_space_cubes.material").unwrap();
						}
						(1, "pbr_deferred_world_space_white.material") => {
							material = self.scene.resources.materials.get("pbr_deferred_view_space_white.material").unwrap();
						}
						(2, "pbr_deferred_world_space_cubes.material") => {
							material = self.scene.resources.materials.get("pbr_deferred_pbr_data_cubes.material").unwrap();
						}
						(2, "pbr_deferred_world_space_white.material") => {
							material = self.scene.resources.materials.get("pbr_deferred_pbr_data_white.material").unwrap();
						}
						_ if repeat > 0 => continue,
						_ => {}
					}
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

	fn draw_quad<'a>(&'a self, mut render_pass: wgpu::RenderPass<'a>, material_name: &str) {
		let mesh = &self.scene.resources.meshes.get(&(String::from("BLIT"), String::from("QUAD"))).unwrap();
		let material = &self.scene.resources.materials.get(material_name).unwrap();
		let shader = &self.scene.resources.shaders[material.shader_id];

		render_pass.set_pipeline(&shader.render_pipeline);

		render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

		render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

		render_pass.set_bind_group(0, &material.bind_group, &[]);

		render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
	}
}
