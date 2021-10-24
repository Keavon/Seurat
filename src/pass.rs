use crate::frame_texture::FrameTextureTypes;

pub struct Pass {
	pub label: String,
	pub depth_attachment: bool,
	pub color_attachment_types: Vec<FrameTextureTypes>,
	pub blit_material: Option<String>,
	pub clear_color: wgpu::Color,
	pub repeat: usize,
}
