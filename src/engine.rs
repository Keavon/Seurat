use crate::camera::SceneCamera;
use crate::camera_controller::CameraController;
use crate::component::Component;
use crate::context::Context;
use crate::frame_texture::{FrameTexture, FrameTextures};
use crate::instance::Instance;
use crate::light::SceneLighting;
use crate::material::{Material, MaterialDataBinding};
use crate::mesh::Mesh;
use crate::model::Model;
use crate::pass::{ComputePass, Pass, RenderPass};
use crate::scene::Scene;
use crate::shader::{PipelineOptions, RenderPipelineOptions, Shader, ShaderBinding, ShaderBindingBuffer, ShaderBindingTexture};
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
}

impl Engine {
	// Creating some of the wgpu types requires async code
	pub async fn new(window: &Window) -> Self {
		// Mechanical details of the GPU rendering process
		let context = Context::new(window).await;

		// Prepare the frame textures
		let z_buffer = FrameTexture::new(
			&context.device,
			&context.surface_configuration,
			wgpu::TextureFormat::Depth32Float,
			"Z-buffer frame texture",
			Some(wgpu::CompareFunction::LessEqual),
		);

		let world_space_fragment_location = FrameTexture::new(
			&context.device,
			&context.surface_configuration,
			wgpu::TextureFormat::Rgba16Float,
			"World Space Fragment Location frame texture",
			None,
		);
		let world_space_normal = FrameTexture::new(
			&context.device,
			&context.surface_configuration,
			wgpu::TextureFormat::Rgba16Float,
			"World Space Normal frame texture",
			None,
		);

		let albedo_map = FrameTexture::new(&context.device, &context.surface_configuration, wgpu::TextureFormat::Bgra8UnormSrgb, "Albedo Map frame texture", None);
		let arm_map = FrameTexture::new(&context.device, &context.surface_configuration, wgpu::TextureFormat::Bgra8Unorm, "ARM Map frame texture", None);

		let ssao_kernel_map = FrameTexture::new(&context.device, &context.surface_configuration, wgpu::TextureFormat::Rgba16Float, "SSAO Kernel Map frame texture", None);
		let ssao_blurred_map = FrameTexture::new(
			&context.device,
			&context.surface_configuration,
			wgpu::TextureFormat::Rgba16Float,
			"SSAO Blurred Map frame texture",
			None,
		);

		let pbr_shaded_map = FrameTexture::new(&context.device, &context.surface_configuration, wgpu::TextureFormat::Rgba16Float, "PBR Shaded Map frame texture", None);

		let frame_textures = FrameTextures {
			z_buffer,
			world_space_fragment_location,
			world_space_normal,
			albedo_map,
			arm_map,
			ssao_kernel_map,
			ssao_blurred_map,
			pbr_shaded_map,
		};

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
		}
	}

	pub fn load(&mut self, assets_path: &Path) {
		self.load_resources(assets_path);
		self.load_scene();
	}

	fn load_resources(&mut self, assets_path: &Path) {
		// Shaders
		let temporary_camera = SceneCamera::new(&self.context);

		let scene_deferred_shader = {
			let albedo_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Albedo map
			let arm_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // AO/Roughness/Metalness map
			let normal_map = ShaderBinding::Texture(ShaderBindingTexture::default()); // Normal map

			Shader::new(
				&self.context,
				assets_path,
				"scene_deferred.wgsl",
				vec![albedo_map, arm_map, normal_map],
				PipelineOptions::RenderPipeline(RenderPipelineOptions {
					out_color_formats: vec![
						wgpu::TextureFormat::Rgba16Float,
						wgpu::TextureFormat::Rgba16Float,
						wgpu::TextureFormat::Bgra8UnormSrgb,
						wgpu::TextureFormat::Bgra8Unorm,
					],
					depth_format: Some(wgpu::TextureFormat::Depth32Float),
					use_instances: true,
					scene_camera: Some(&temporary_camera),
					scene_lighting: Some(&self.scene_lighting),
				}),
			)
		};
		self.scene.resources.shaders.insert(String::from("scene_deferred.wgsl"), scene_deferred_shader);

		let pass_ssao_kernel_shader = {
			let samples_array = ShaderBinding::Buffer(ShaderBindingBuffer::default());
			let ssao_noise_texture = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_fragment_location = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_normal = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(
				&self.context,
				assets_path,
				"pass_ssao_kernel.wgsl",
				vec![samples_array, ssao_noise_texture, world_space_fragment_location, world_space_normal],
				PipelineOptions::RenderPipeline(RenderPipelineOptions {
					out_color_formats: vec![wgpu::TextureFormat::Rgba16Float],
					depth_format: None,
					use_instances: false,
					scene_camera: Some(&temporary_camera),
					scene_lighting: None,
				}),
			)
		};
		self.scene.resources.shaders.insert(String::from("pass_ssao_kernel.wgsl"), pass_ssao_kernel_shader);

		let pass_ssao_blurred_shader = {
			let ssao_kernel = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(
				&self.context,
				assets_path,
				"pass_ssao_blurred.wgsl",
				vec![ssao_kernel],
				PipelineOptions::RenderPipeline(RenderPipelineOptions {
					out_color_formats: vec![wgpu::TextureFormat::Rgba16Float],
					depth_format: None,
					use_instances: false,
					scene_camera: None,
					scene_lighting: None,
				}),
			)
		};
		self.scene.resources.shaders.insert(String::from("pass_ssao_blurred.wgsl"), pass_ssao_blurred_shader);

		let pass_pbr_shading_shader = {
			let world_space_fragment_location = ShaderBinding::Texture(ShaderBindingTexture::default());
			let world_space_normal = ShaderBinding::Texture(ShaderBindingTexture::default());
			let albedo_map = ShaderBinding::Texture(ShaderBindingTexture::default());
			let arm_map = ShaderBinding::Texture(ShaderBindingTexture::default());
			let ssao_blurred_map = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(
				&self.context,
				assets_path,
				"pass_pbr_shading.wgsl",
				vec![world_space_fragment_location, world_space_normal, albedo_map, arm_map, ssao_blurred_map],
				PipelineOptions::RenderPipeline(RenderPipelineOptions {
					out_color_formats: vec![wgpu::TextureFormat::Rgba16Float],
					depth_format: None,
					use_instances: false,
					scene_camera: Some(&temporary_camera),
					scene_lighting: Some(&self.scene_lighting),
				}),
			)
		};
		self.scene.resources.shaders.insert(String::from("pass_pbr_shading.wgsl"), pass_pbr_shading_shader);

		// let compute_hdr_measuring_shader = {
		// 	let pbr_shaded = ShaderBinding::Texture(ShaderBindingTexture {
		// 		visible_in_stages: wgpu::ShaderStages::COMPUTE,
		// 		..ShaderBindingTexture::default()
		// 	});

		// 	Shader::new(
		// 		&self.context,
		// 		assets_path,
		// 		"compute_hdr_measuring.wgsl",
		// 		vec![pbr_shaded],
		// 		PipelineOptions::ComputePipeline(ComputePipelineOptions {}),
		// 	)
		// };
		// self.scene.resources.shaders.insert(String::from("pass_pbr_shading.wgsl"), compute_hdr_measuring_shader);

		let pass_hdr_exposure_shader = {
			let pbr_shaded = ShaderBinding::Texture(ShaderBindingTexture::default());

			Shader::new(
				&self.context,
				assets_path,
				"pass_hdr_exposure.wgsl",
				vec![pbr_shaded],
				PipelineOptions::RenderPipeline(RenderPipelineOptions {
					out_color_formats: vec![self.context.surface_configuration.format],
					depth_format: None,
					use_instances: false,
					scene_camera: None,
					scene_lighting: None,
				}),
			)
		};
		self.scene.resources.shaders.insert(String::from("pass_hdr_exposure.wgsl"), pass_hdr_exposure_shader);

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
			String::from("cobblestone_albedo.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"cobblestone_albedo.jpg",
				wgpu::TextureFormat::Rgba8UnormSrgb,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cobblestone_arm.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"cobblestone_arm.jpg",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("cobblestone_normal.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"cobblestone_normal.jpg",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);

		self.scene.resources.textures.insert(
			String::from("sponza_pbr_14650633544276105767.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"sponza_pbr_14650633544276105767.jpg",
				wgpu::TextureFormat::Rgba8UnormSrgb,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("sponza_pbr_4871783166746854860.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"sponza_pbr_4871783166746854860.jpg",
				wgpu::TextureFormat::Rgba8Unorm,
				wgpu::AddressMode::ClampToEdge,
			)
			.unwrap(),
		);
		self.scene.resources.textures.insert(
			String::from("sponza_pbr_2051777328469649772.jpg"),
			Texture::load(
				&self.context.device,
				&self.context.queue,
				assets_path,
				"sponza_pbr_2051777328469649772.jpg",
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

		let meshes = Mesh::load(&self.context.device, &self.context.queue, assets_path, "sponza_pbr.obj");
		for mesh in meshes.unwrap_or_default() {
			self.scene.resources.meshes.insert((String::from("sponza_pbr.obj"), mesh.name.clone()), mesh);
		}
	}

	fn load_materials(&mut self) {
		// Materials
		self.scene.resources.materials.insert(
			String::from("scene_deferred_cubes.material"),
			Material::new(
				"scene_deferred_cubes.material",
				"scene_deferred.wgsl",
				vec![
					MaterialDataBinding::TextureName("cobblestone_albedo.jpg"),
					MaterialDataBinding::TextureName("cobblestone_arm.jpg"),
					MaterialDataBinding::TextureName("cobblestone_normal.jpg"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("scene_deferred_Arches.material"),
			Material::new(
				"scene_deferred_Arches.material",
				"scene_deferred.wgsl",
				vec![
					MaterialDataBinding::TextureName("sponza_pbr_14650633544276105767.jpg"),
					MaterialDataBinding::TextureName("sponza_pbr_4871783166746854860.jpg"),
					MaterialDataBinding::TextureName("sponza_pbr_2051777328469649772.jpg"),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		let ssao_samples_buffer = self.context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("SSAO samples buffer"),
			contents: bytemuck::cast_slice(&crate::ssao::generate_sample_hemisphere()),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});
		self.scene.resources.materials.insert(
			String::from("pass_ssao_kernel.material"),
			Material::new(
				"pass_ssao_kernel.material",
				"pass_ssao_kernel.wgsl",
				vec![
					MaterialDataBinding::Buffer(wgpu::BufferBinding {
						buffer: &ssao_samples_buffer,
						offset: 0,
						size: None,
					}),
					MaterialDataBinding::TextureName("SSAO_NOISE"),
					MaterialDataBinding::Texture(&self.frame_textures.world_space_fragment_location.texture),
					MaterialDataBinding::Texture(&self.frame_textures.world_space_normal.texture),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("pass_ssao_blurred.material"),
			Material::new(
				"pass_ssao_blurred.material",
				"pass_ssao_blurred.wgsl",
				vec![MaterialDataBinding::Texture(&self.frame_textures.ssao_kernel_map.texture)],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("pass_pbr_shading.material"),
			Material::new(
				"pass_pbr_shading.material",
				"pass_pbr_shading.wgsl",
				vec![
					MaterialDataBinding::Texture(&self.frame_textures.world_space_fragment_location.texture),
					MaterialDataBinding::Texture(&self.frame_textures.world_space_normal.texture),
					MaterialDataBinding::Texture(&self.frame_textures.albedo_map.texture),
					MaterialDataBinding::Texture(&self.frame_textures.arm_map.texture),
					MaterialDataBinding::Texture(&self.frame_textures.ssao_blurred_map.texture),
				],
				&self.scene.resources,
				&self.context.device,
			),
		);

		self.scene.resources.materials.insert(
			String::from("pass_hdr_exposure.material"),
			Material::new(
				"pass_hdr_exposure.material",
				"pass_hdr_exposure.wgsl",
				vec![MaterialDataBinding::Texture(&self.frame_textures.pbr_shaded_map.texture)],
				&self.scene.resources,
				&self.context.device,
			),
		);
	}

	fn load_scene(&mut self) {
		let scene_camera = SceneCamera::new(&self.context);
		self.load_materials();

		// Main camera
		let main_camera = self.scene.root.new_child("Main Camera");
		main_camera.add_component(Component::Camera(scene_camera));

		// White cube representing the light
		let lamp = self.scene.root.new_child("Lamp Model");

		let mut lamp_model = Model::new(&self.scene.resources, ("cube.obj", "Cube_Finished_Cube.001"), "scene_deferred_Arches.material");
		lamp_model.instances.instance_list[0].location.y = 4.;
		lamp_model.instances.update_buffer(&self.context.device);
		lamp.add_component(Component::Model(lamp_model));

		let light_cube_movement = crate::scripts::light_cube_movement::LightCubeMovement;
		lamp.add_component(Component::Behavior(Box::new(light_cube_movement)));

		// Array of cubes
		let cubes = self.scene.root.new_child("Cubes");

		let mut cube_model = Model::new(&self.scene.resources, ("cube.obj", "Cube_Finished_Cube.001"), "scene_deferred_cubes.material");

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

		let mut sponza_model = Model::new(&self.scene.resources, ("sponza_pbr.obj", "Arches"), "scene_deferred_Arches.material");
		sponza_model.instances.update_buffer(&self.context.device);

		sponza.add_component(Component::Model(sponza_model));
	}

	fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.context.surface_configuration.width = new_size.width;
			self.context.surface_configuration.height = new_size.height;
			self.context.surface.configure(&self.context.device, &self.context.surface_configuration);

			self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0]
				.projection
				.resize(new_size.width, new_size.height);

			self.frame_textures.recreate_all(&self.context.device, &self.context.surface_configuration);
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
			// Mouse movement
			DeviceEvent::MouseMotion { delta } => {
				// self.scene.find_entity_mut(self.active_camera.as_str()).unwrap().get_cameras_mut()[0]
				self.camera_controller.process_mouse(delta.0, delta.1);
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
		let lamp_model = self.scene.find_entity_mut("Lamp Model").unwrap();
		let location = cgmath::Point3 {
			x: new_position.x as f64,
			y: new_position.y as f64,
			z: new_position.z as f64,
		};
		let rotation = lamp_model.transform.rotation;
		let scale = cgmath::Point3 { x: 0.25, y: 0.25, z: 0.25 };
		for model in &mut lamp_model.get_models_mut() {
			model.instances.transform_single_instance(location, rotation, scale, &self.context.device);
		}

		// Call update() on all entity behaviors
		self.scene.root.update_behaviors_of_descendants();
	}

	fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		let surface_texture = self.context.surface.get_current_texture()?;
		let surface_texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let passes = vec![
			Pass::RenderPass(RenderPass {
				label: String::from("Scene: Deferred"),
				depth_attachment: Some(&self.frame_textures.z_buffer.texture.view),
				color_attachment_types: vec![
					&self.frame_textures.world_space_fragment_location.texture.view,
					&self.frame_textures.world_space_normal.texture.view,
					&self.frame_textures.albedo_map.texture.view,
					&self.frame_textures.arm_map.texture.view,
				],
				blit_material: None,
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			}),
			Pass::RenderPass(RenderPass {
				label: String::from("Pass: SSAO Kernel"),
				depth_attachment: None,
				color_attachment_types: vec![&self.frame_textures.ssao_kernel_map.texture.view],
				blit_material: Some(String::from("pass_ssao_kernel.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			}),
			Pass::RenderPass(RenderPass {
				label: String::from("Pass: SSAO Blurred"),
				depth_attachment: None,
				color_attachment_types: vec![&self.frame_textures.ssao_blurred_map.texture.view],
				blit_material: Some(String::from("pass_ssao_blurred.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			}),
			Pass::RenderPass(RenderPass {
				label: String::from("Pass: PBR Shading"),
				depth_attachment: None,
				color_attachment_types: vec![&self.frame_textures.pbr_shaded_map.texture.view],
				blit_material: Some(String::from("pass_pbr_shading.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			}),
			// Pass::ComputePass(ComputePass {
			// 	label: String::from("Pass: HDR Measuring"),
			// 	material: String::from("pass_pbr_shading.material"),
			// }),
			Pass::RenderPass(RenderPass {
				label: String::from("Pass: HDR Exposure"),
				depth_attachment: None,
				color_attachment_types: vec![&surface_texture_view],
				blit_material: Some(String::from("pass_hdr_exposure.material")),
				clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
			}),
		];

		for pass in passes {
			match pass {
				Pass::RenderPass(pass) => {
					let color_attachments = pass
						.color_attachment_types
						.into_iter()
						.map(|frame_texture_type| wgpu::RenderPassColorAttachment {
							view: frame_texture_type,
							resolve_target: None,
							ops: wgpu::Operations {
								load: wgpu::LoadOp::Clear(pass.clear_color),
								store: true,
							},
						})
						.collect::<Vec<wgpu::RenderPassColorAttachment>>();

					let depth_stencil_attachment = pass.depth_attachment.map(|view| wgpu::RenderPassDepthStencilAttachment {
						view,
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
						None => self.draw_scene(render_pass),
						Some(material_name) => self.draw_quad(render_pass, material_name.as_str()),
					}
				}
				Pass::ComputePass(pass) => {}
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
					let pipeline = match &shader.pipeline {
						crate::shader::PipelineType::RenderPipeline(render_pipeline) => render_pipeline,
						crate::shader::PipelineType::ComputePipeline(_) => continue,
					};

					let instances_buffer = model.instances.instances_buffer.as_ref();
					let instances_range = 0..model.instances.instance_list.len() as u32;

					render_pass.set_pipeline(pipeline);

					render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
					render_pass.set_vertex_buffer(1, instances_buffer.unwrap().slice(..));

					render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

					let mut index = 0;
					if shader.includes_camera {
						let scene_camera = self.scene.find_entity(self.active_camera.as_str()).unwrap().get_cameras()[0];
						render_pass.set_bind_group(index, &scene_camera.camera_bind_group, &[]);
						index += 1;
					}
					if shader.includes_lighting {
						render_pass.set_bind_group(index, &self.scene_lighting.light_bind_group, &[]);
						index += 1;
					}
					render_pass.set_bind_group(index, &material.bind_group, &[]);

					render_pass.draw_indexed(0..mesh.index_count, 0, instances_range);
				}
			}
		}
	}

	fn draw_quad<'a>(&'a self, mut render_pass: wgpu::RenderPass<'a>, material_name: &str) {
		let mesh = &self.scene.resources.meshes.get(&(String::from("BLIT"), String::from("QUAD"))).unwrap();
		let material = &self.scene.resources.materials.get(material_name).unwrap();
		let shader = &self.scene.resources.shaders[material.shader_id];
		let pipeline = match &shader.pipeline {
			crate::shader::PipelineType::RenderPipeline(render_pipeline) => render_pipeline,
			crate::shader::PipelineType::ComputePipeline(_) => return,
		};

		render_pass.set_pipeline(pipeline);

		render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

		render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

		let mut index = 0;
		if shader.includes_camera {
			let scene_camera = self.scene.find_entity(self.active_camera.as_str()).unwrap().get_cameras()[0];
			render_pass.set_bind_group(index, &scene_camera.camera_bind_group, &[]);
			index += 1;
		}
		if shader.includes_lighting {
			render_pass.set_bind_group(index, &self.scene_lighting.light_bind_group, &[]);
			index += 1;
		}
		render_pass.set_bind_group(index, &material.bind_group, &[]);

		render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
	}
}
