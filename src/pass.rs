use wgpu::TextureView;

pub struct Pass<'a> {
	pub label: String,
	pub depth_attachment: Option<&'a TextureView>,
	pub color_attachment_types: Vec<&'a TextureView>,
	pub blit_material: Option<String>,
	pub clear_color: wgpu::Color,
}
