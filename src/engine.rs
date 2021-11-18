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
	trace_buffer: wgpu::Buffer,
	trace_length: u32,
}

impl Engine {
	// Creating some of the wgpu types requires async code
	pub async fn new(window: &Window) -> Self {
		// Mechanical details of the GPU rendering process
		let context = Context::new(window).await;

		// Prepare the frame textures
		// let pbr_shaded_map = FrameTexture::new(&context.device, &context.surface_configuration, wgpu::TextureFormat::Rgba16Float, "PBR Shaded Map frame texture", None);

		let frame_textures = FrameTextures {
			// pbr_shaded_map,
		};

		// let trace = [30, 90, 1000, 300, 500, 400, 550, 450];
		// const AMPLITUDE: f32 = 150.;
		// const WIDTH_SCALE: f32 = 1.;
		// const SAMPLES: usize = 128;

		// let mut trace_vec = (0..SAMPLES)
		// 	.flat_map(|i| [i as f32 * WIDTH_SCALE + 400., f32::sin(i as f32 / SAMPLES as f32 * 2. * std::f32::consts::PI) * AMPLITUDE + 400.])
		// 	.collect::<Vec<_>>();
		// trace_vec.extend((0..SAMPLES).flat_map(|i| [i as f32 * WIDTH_SCALE + 400., f32::sin(i as f32 / SAMPLES as f32 * 2. * std::f32::consts::PI) * -AMPLITUDE + 400.]));

		// let trace = trace_vec.as_slice();
		let trace_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Trace Buffer"),
			size: 16777216,
			mapped_at_creation: false,
			usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
		});
		context.queue.write_buffer(&trace_buffer, 0, bytemuck::cast_slice(&[0_u32]));

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
			trace_buffer,
			trace_length: 0, // First 4 bytes is length (u32)
		}
	}

	pub fn load(&mut self, assets_path: &Path) {
		self.load_resources(assets_path);
		self.load_scene();
	}

	fn load_resources(&mut self, assets_path: &Path) {
		// Shaders
		let pass_sdf_brushwork_shader = {
			let brush_trace = ShaderBinding::Buffer(ShaderBindingBuffer {
				uniform_or_storage: wgpu::BufferBindingType::Storage { read_only: true },
				..ShaderBindingBuffer::default()
			});

			Shader::new(
				&self.context,
				assets_path,
				"pass_sdf_brushwork.wgsl",
				vec![brush_trace],
				vec![self.context.surface_configuration.format],
				None,
				false,
				None,
				None,
			)
		};
		self.scene.resources.shaders.insert(String::from("pass_sdf_brushwork.wgsl"), pass_sdf_brushwork_shader);

		// Textures
		// self.scene.resources.textures.insert(
		// 	String::from("SSAO_NOISE"),
		// 	Texture::from_f16_array(
		// 		&self.context.device,
		// 		&self.context.queue,
		// 		crate::ssao::generate_noise_texture().as_slice(),
		// 		(4, 4),
		// 		"SSAO_NOISE",
		// 		wgpu::TextureFormat::Rgba16Float,
		// 		wgpu::AddressMode::Repeat,
		// 	),
		// );

		// self.scene.resources.textures.insert(
		// 	String::from("cobblestone_albedo.jpg"),
		// 	Texture::load(
		// 		&self.context.device,
		// 		&self.context.queue,
		// 		assets_path,
		// 		"cobblestone_albedo.jpg",
		// 		wgpu::TextureFormat::Rgba8UnormSrgb,
		// 		wgpu::AddressMode::ClampToEdge,
		// 	)
		// 	.unwrap(),
		// );

		// Meshes
		let blit_quad_mesh = Mesh::new_blit_quad(&self.context.device, &self.context.queue);
		self.scene.resources.meshes.insert((String::from("BLIT"), String::from("QUAD")), blit_quad_mesh);
	}

	fn load_materials(&mut self) {
		// Materials

		self.scene.resources.materials.insert(
			String::from("pass_sdf_brushwork.material"),
			Material::new(
				"pass_sdf_brushwork.material",
				"pass_sdf_brushwork.wgsl",
				vec![MaterialDataBinding::Buffer(wgpu::BufferBinding {
					buffer: &self.trace_buffer,
					offset: 0,
					size: None,
				})],
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
				// self.camera_controller.process_mouse(delta.0, delta.1);
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
			// Mouse input
			WindowEvent::CursorMoved { position, .. } => {
				self.mouse_moved(position.x as f32, position.y as f32);
			}
			_ => {}
		}
	}

	fn mouse_moved(&mut self, x: f32, y: f32) {
		let coordinates_size = std::mem::size_of::<[f32; 2]>() as u64;

		self.context.queue.write_buffer(&self.trace_buffer, 0, bytemuck::cast_slice(&[self.trace_length]));
		self.context
			.queue
			.write_buffer(&self.trace_buffer, self.trace_length as u64 * coordinates_size + 4, bytemuck::cast_slice(&[x, y]));
		self.trace_length += 1;
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

	fn update(&mut self, delta_time: std::time::Duration) {}

	fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		let surface_texture = self.context.surface.get_current_texture()?;
		let surface_texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let passes = vec![Pass {
			label: String::from("Pass: HDR Exposure"),
			depth_attachment: None,
			color_attachment_types: vec![&surface_texture_view],
			blit_material: Some(String::from("pass_sdf_brushwork.material")),
			clear_color: wgpu::Color { r: 0., g: 0., b: 0., a: 1.0 },
		}];

		for pass in passes {
			// println!("Beginning pass: {}", pass.label);

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

			// println!("End of pass: {}", pass.label);
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

					let instances_buffer = model.instances.instances_buffer.as_ref();
					let instances_range = 0..model.instances.instance_list.len() as u32;

					render_pass.set_pipeline(&shader.render_pipeline);

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

		render_pass.set_pipeline(&shader.render_pipeline);

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
