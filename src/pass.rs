use wgpu::TextureView;

pub enum Pass<'a> {
	RenderPass(RenderPass<'a>),
	ComputePass(ComputePass),
}

pub struct RenderPass<'a> {
	pub label: String,
	pub depth_attachment: Option<&'a TextureView>,
	pub color_attachment_types: Vec<&'a TextureView>,
	pub blit_material: Option<String>,
	pub clear_color: wgpu::Color,
}

pub struct ComputePass {
	pub label: String,
	pub material: String,
}
