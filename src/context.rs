use winit::window::Window;

pub struct Context {
	pub surface: wgpu::Surface,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub config: wgpu::SurfaceConfiguration,
}

impl Context {
	pub async fn new(window: &Window) -> Self {
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
