use wgpu::TextureView;

use crate::frame_texture::FrameTextureTypes;

pub struct Pass<'a> {
	pub label: String,
	pub depth_attachment: Option<&'a TextureView>,
	pub color_attachment_types: Vec<FrameTextureTypes>,
	pub blit_material: Option<String>,
	pub clear_color: wgpu::Color,
	pub repeat: usize,
}
