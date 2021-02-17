use crate::{Processor, LUT};
use anyhow::{anyhow, bail, Result};
use std::{
	convert::TryFrom,
	io::{BufRead, BufReader, Read},
	mem::size_of,
};

pub fn parse<I>(backend: &Processor, input: I) -> Result<LUT>
where
	I: Read,
{
	let lines = BufReader::new(input).lines().peekable();

	let mut dim: u32 = 0;
	let mut expect = 0;
	let mut buffer = Vec::with_capacity((dim * dim * dim * 4) as usize);

	for l in lines {
		let line = l?;
		if line == "" {
			continue;
		} else if line.starts_with("TITLE") {
		} else if line.starts_with("LUT_3D_SIZE") {
			let u = line
				.trim_start_matches("LUT_3D_SIZE")
				.trim_start_matches(char::is_whitespace)
				.trim_start_matches('"')
				.trim_end_matches('"')
				.parse::<usize>()?;
			if !(2..65536).contains(&u) {
				bail!("not a valid lut size, should between [2,65535]");
			}
			dim = u32::try_from(u)?;
		} else {
			if dim == 0 {
				bail!("header did not contain LUT_3D_SIZE");
			}

			let floats: Vec<Result<f32>> = line
				.split_whitespace()
				.map(|f| {
					f.parse::<f32>()
						.map_err(|e| anyhow!("can not parse '{}' as float: {}", f, e))
				})
				.collect();
			if floats.len() != 3 {
				bail!("need 3 floats per line");
			}

			for p in floats.into_iter() {
				buffer.extend(p?.to_ne_bytes().iter());
			}
			buffer.extend(1f32.to_ne_bytes().iter());
			expect += 1;
		}
	}

	if expect != dim * dim * dim {
		bail!("need {} lines, got {}", dim * dim * dim, expect);
	}

	let device = &backend.device;

	let texture_size = wgpu::Extent3d {
		width: dim,
		height: dim,
		depth: dim,
	};

	let texture = device.create_texture(&wgpu::TextureDescriptor {
		label: None,
		size: texture_size,
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D3,
		format: wgpu::TextureFormat::Rgba32Float,
		usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
	});

	backend.queue.write_texture(
		wgpu::TextureCopyView {
			texture: &texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
		},
		buffer.as_slice(),
		wgpu::TextureDataLayout {
			offset: 0,
			bytes_per_row: (size_of::<f32>() * 4) as u32 * texture_size.width,
			rows_per_image: texture_size.height,
		},
		texture_size,
	);

	device.poll(wgpu::Maintain::Wait);

	let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

	Ok(LUT {
		texture,
		texture_view,
	})
}
