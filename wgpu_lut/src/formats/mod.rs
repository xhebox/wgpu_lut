use wgpu;

mod cube;
pub use cube::parse as cube;

#[derive(Debug)]
pub struct LUT {
	texture: wgpu::Texture,
	pub(crate) texture_view: wgpu::TextureView,
}
