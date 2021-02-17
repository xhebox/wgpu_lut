#[derive(Debug)]
pub struct BufferAlign {
	pub width: u32,
	pub unpadded_bytes_per_row: u32,
	pub padded_bytes_per_row: u32,
}

impl BufferAlign {
	pub fn new(width: u32, bytes_per_pixel: u32) -> Self {
		let unpadded_bytes_per_row = width * bytes_per_pixel;
		let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
		let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
		let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
		Self {
			width,
			unpadded_bytes_per_row,
			padded_bytes_per_row,
		}
	}
}
