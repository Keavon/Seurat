mod behavior;
mod camera;
mod camera_controller;
mod component;
mod context;
mod debug_buffer;
mod engine;
mod entity;
mod frame_texture;
mod instance;
mod light;
mod material;
mod mesh;
mod model;
mod pass;
mod scene;
mod scripts;
mod shader;
mod ssao;
mod texture;
mod transform;
mod voxel_texture;

use crate::engine::Engine;

use winit::{
	dpi::PhysicalSize,
	event::Event,
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
};

fn main() {
	// Enable logging
	env_logger::init();

	// Root directory to load assets from
	let assets_path = std::path::Path::new(env!("OUT_DIR")).join("assets");

	// Initialize the window
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new().with_inner_size(PhysicalSize::new(1920, 1080)).with_title("Seurat").build(&event_loop).unwrap();

	// Initialize the engine
	let mut engine = pollster::block_on(Engine::new(&window));
	engine.load(&assets_path);

	// Handle events, simulate, and draw frames repeatedly until the program is closed
	event_loop.run(move |event, _, control_flow| {
		// Poll makes the event loop repeat immediately
		*control_flow = ControlFlow::Poll;

		// Process events and frame draw requests
		match event {
			// Handle user input from a human input device (mouse, keyboard, etc.)
			Event::DeviceEvent { ref event, .. } => engine.process_input(event),
			// Close, resize, etc. as requested by the window
			Event::WindowEvent { ref event, window_id: id } if id == window.id() => engine.process_window_event(event, control_flow),
			// Draw the next frame as requested
			Event::RedrawRequested(_) => engine.draw_frame(&window, control_flow),
			// Request the next frame be drawn
			Event::MainEventsCleared => window.request_redraw(),
			_ => (),
		}
	});
}
