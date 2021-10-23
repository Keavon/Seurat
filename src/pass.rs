use crate::frame_texture::FrameTextureTypes;

pub struct Pass {
	pub label: String,
	pub depth_attachment: bool,
	pub color_attachment_types: Vec<FrameTextureTypes>,
	pub draw_quad_not_scene: bool,
	pub clear_color: wgpu::Color,
}
