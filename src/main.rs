use std::{
	borrow::Cow,
	fs::File,
	io::{BufRead, BufReader},
};
use wgpu::util::DeviceExt;

fn main() {
	env_logger::init();

	let input_numbers = if std::env::args().len() <= 1 {
		let default = vec![8, 7, 6, 5, 4, 3, 2, 1];
		println!("No numbers were provided, defaulting to {:?}", default);
		default
	} else if std::env::args().len() == 2 {
		let file = File::open(std::env::args().nth(1).unwrap()).expect("Invalid name/path of input file. If you're trying to directly input numbers, provide at least two.");
		let mut reader = BufReader::new(file);

		let mut line = String::new();
		reader.read_line(&mut line).unwrap();
		let mut sequence = vec![];
		for number in line.split(' ') {
			let s = String::from(number).parse::<i32>().unwrap();
			sequence.push(s);
		}
		sequence
	} else {
		std::env::args().skip(1).map(|s| s.parse::<i32>().expect("Please pass a list of positive integers")).collect()
	};

	pollster::block_on(run(input_numbers.as_slice()));
}

async fn run(input_numbers: &[i32]) {
	let application = Application::new().await;
	let result = application.compute(input_numbers).await;

	match result {
		Some(list) => println!("Sorted list: {:?}", list),
		None => println!("No result"),
	}
}

struct Context {
	device: wgpu::Device,
	queue: wgpu::Queue,
}
impl Context {
	async fn new() -> Self {
		// The WGPU runtime
		let instance = wgpu::Instance::new(wgpu::Backends::all());

		// Handle to the GPU
		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: None,
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

		Self { device, queue }
	}
}

struct Application {
	context: Context,
}
impl Application {
	async fn new() -> Self {
		let context = Context::new().await;
		Self { context }
	}

	async fn compute(self, input_numbers: &[i32]) -> Option<Vec<i32>> {
		// BUFFER

		// Get the size in bytes of the buffer
		let size = (input_numbers.len() * std::mem::size_of::<i32>()) as wgpu::BufferAddress;

		// Instantiate the buffer without data, readable outside the shader (MAP_READ) and acting as a destination to copy to (COPY_DST)
		let staging_buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size,
			usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		// Instantiate a buffer with the input number data, which is accessible in the shader and can be copied to and from
		let storage_buffer = self.context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Storage Buffer"),
			contents: bytemuck::cast_slice(input_numbers),
			usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
		});

		// BIND GROUP

		// Create the bind group with the bound buffer data on the GPU
		let bind_group_layout = self.context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Bind group layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::COMPUTE,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Storage { read_only: false },
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::COMPUTE,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Storage { read_only: false },
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
			],
		});

		// SHADER PIPELINE

		// Load the shader from WGSL
		let shader_module = self.context.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/shader.wgsl"))),
		});

		let compute_pipeline_layout = self.context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Compute pipeline layout"),
			bind_group_layouts: &[&bind_group_layout],
			push_constant_ranges: &[],
		});

		// Instantiate the compute shader pipeline
		let compute_pipeline = self.context.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
			label: None,
			layout: Some(&compute_pipeline_layout),
			module: &shader_module,
			entry_point: "main",
		});

		// COMMAND EXECUTION

		// A command encoder executes one or many pipelines, then when finished it becomes a CommandBuffer
		let mut command_encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

		// First, have the parallel computation be run
		for i in 0..input_numbers.len() {
			let even_or_odd_bool_buffer = self.context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Even or Odd Bool Buffer"),
				contents: bytemuck::cast_slice(&[(i % 2) as u32]),
				usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
			});

			let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
				label: None,
				layout: &bind_group_layout,
				entries: &[
					wgpu::BindGroupEntry {
						binding: 0,
						resource: storage_buffer.as_entire_binding(),
					},
					wgpu::BindGroupEntry {
						binding: 1,
						resource: even_or_odd_bool_buffer.as_entire_binding(),
					},
				],
			});

			let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
			compute_pass.set_pipeline(&compute_pipeline);
			compute_pass.set_bind_group(0, &bind_group, &[]);
			compute_pass.insert_debug_marker("Running the compute shader");
			compute_pass.dispatch((input_numbers.len() as f64 / (2.0 * 256.0)).ceil() as u32, 1, 1);
		}

		// Then, have the data copied back from the GPU to the CPU
		command_encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);

		// Submit the command encoder for processing
		self.context.queue.submit(Some(command_encoder.finish()));

		// RESULTING BUFFER

		// Block until the buffer is ready to be asynchronously read
		let buffer_slice = staging_buffer.slice(..);
		let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
		self.context.device.poll(wgpu::Maintain::Wait);
		buffer_future.await.is_err().then(|| panic!("Failed to run compute on the GPU"));

		// Get the contents of the buffer
		let buffer_view = buffer_slice.get_mapped_range();
		let result = bytemuck::cast_slice(&buffer_view).to_vec();
		drop(buffer_view);

		// Free the buffer from memory
		staging_buffer.unmap();

		// Return the resulting data
		Some(result)
	}
}
