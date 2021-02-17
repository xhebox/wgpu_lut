#![feature(backtrace)]

mod formats;
mod utils;

use anyhow::{anyhow, bail, Result};
use dashmap::DashMap;
use std::{borrow::Cow, convert::TryFrom, mem::size_of, ptr};
use wgpu;

// remember modify the shader, too
const WORKGROUP_SIZE: u32 = 32;
include!(concat!(env!("OUT_DIR"), "/shader_lut.rs"));

#[derive(Debug)]
struct LUT {
	texture: wgpu::Texture,
	texture_view: wgpu::TextureView,
}

#[derive(Debug)]
pub struct Processor {
	adapter: wgpu::Adapter,
	device: wgpu::Device,
	queue: wgpu::Queue,
	shader: wgpu::ShaderModule,
	samplers: DashMap<String, wgpu::Sampler>,
	luts: DashMap<String, LUT>,
}

impl Processor {
	pub async fn new(validation: bool) -> Result<Self>
where {
		let instance = wgpu::Instance::new(wgpu::BackendBit::VULKAN);

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::HighPerformance,
				..wgpu::RequestAdapterOptions::default()
			})
			.await
			.ok_or(anyhow!("can not find a usable adapter"))?;

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					label: None,
					features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
					limits: wgpu::Limits::default(),
				},
				None,
			)
			.await?;

		let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::SpirV(Cow::Borrowed(&SHADER_LUT)),
			//source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
			flags: if validation {
				wgpu::ShaderFlags::VALIDATION | wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION
			} else {
				wgpu::ShaderFlags::default()
			},
		});

		let samplers = DashMap::new();

		let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..wgpu::SamplerDescriptor::default()
		});
		samplers.insert("nearest".to_string(), sampler_nearest);

		let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			..wgpu::SamplerDescriptor::default()
		});
		samplers.insert("linear".to_string(), sampler_linear);

		Ok(Self {
			adapter,
			device,
			queue,
			shader,
			samplers,
			luts: DashMap::new(),
		})
	}

	// r + g * N + b * N * N
	pub fn add_lut_raw(&self, name: &str, dim: u32, lut: &[f32]) -> Result<()> {
		if lut.len() != (dim * dim * dim * 3) as usize {
			bail!(
				"you should provide {} * {} * {} * 3(rgb) floats",
				dim,
				dim,
				dim
			);
		}
		let mut data = Vec::new();
		for i in lut.chunks(3) {
			for j in i {
				data.extend(j.to_ne_bytes().iter());
			}
			data.extend(1f32.to_ne_bytes().iter());
		}
		self.add_lut_raw_alpha(name, dim, data.as_slice())?;
		Ok(())
	}

	pub fn add_lut_raw_alpha(&self, name: &str, dim: u32, lut: &[u8]) -> Result<()> {
		if lut.len() != ((dim * dim * dim) as usize) * 4 * size_of::<f32>() {
			bail!(
				"you should provide {} * {} * {} * 4(rgba) * sizeof(f32)",
				dim,
				dim,
				dim
			);
		}
		let device = &self.device;

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

		let buffer_align = utils::BufferAlign::new(dim * dim * dim, (size_of::<f32>() * 4) as u32);

		let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: u64::try_from(buffer_align.padded_bytes_per_row)?,
			usage: wgpu::BufferUsage::MAP_WRITE | wgpu::BufferUsage::COPY_SRC,
			mapped_at_creation: true,
		});

		{
			let slice = staging_buffer.slice(..);
			let mut buf = slice.get_mapped_range_mut();
			for (dst, src) in buf
				.chunks_mut(buffer_align.padded_bytes_per_row as usize)
				.zip(lut.chunks(buffer_align.unpadded_bytes_per_row as usize))
			{
				unsafe {
					ptr::copy_nonoverlapping(
						&src[0],
						&mut dst[0],
						buffer_align.unpadded_bytes_per_row as usize,
					);
				}
			}
			drop(slice);
		}

		self.queue.write_texture(
			wgpu::TextureCopyView {
				texture: &texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
			},
			lut,
			wgpu::TextureDataLayout {
				offset: 0,
				bytes_per_row: (size_of::<f32>() * 4) as u32 * texture_size.width,
				rows_per_image: texture_size.height,
			},
			texture_size,
		);

		device.poll(wgpu::Maintain::Wait);

		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		self.luts.insert(
			name.to_string(),
			LUT {
				texture,
				texture_view,
			},
		);

		Ok(())
	}

	pub fn add_lut<I: std::io::Read>(&self, name: &str, format: &str, lut: I) -> Result<()> {
		if self.luts.contains_key(name) {
			return Ok(());
		}

		let lutc;
		match format {
			"cube" => {
				lutc = formats::cube(lut)?;
			}
			_ => bail!("unsupported lut format"),
		}

		self.add_lut_raw_alpha(name, lutc.0, lutc.1.as_slice())?;
		Ok(())
	}

	pub fn del_lut(&self, name: &str) {
		self.luts.remove(name);
	}

	pub async fn process(
		&self,
		lutname: &str,
		sampler: &str,
		format: &str,
		width: u32,
		height: u32,
		data: &mut [u8],
	) -> Result<()> {
		let real_format = match format {
			"bgra8" => wgpu::TextureFormat::Bgra8Unorm,
			"rgba8" => wgpu::TextureFormat::Rgba8Unorm,
			"rgb10a2" => wgpu::TextureFormat::Rgb10a2Unorm,
			_ => bail!("unsupported input format"),
		};

		let sampler = self
			.samplers
			.get(sampler)
			.ok_or(anyhow!("no such sampler"))?;

		let lutc = self.luts.get(lutname).ok_or(anyhow!("unknown lut"))?;

		let buffer_align = utils::BufferAlign::new(width, (size_of::<u8>() * 4) as u32);

		let device = &self.device;

		let staging_srcbuf = device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: u64::try_from(buffer_align.padded_bytes_per_row * height)?,
			usage: wgpu::BufferUsage::MAP_WRITE | wgpu::BufferUsage::COPY_SRC,
			mapped_at_creation: true,
		});

		{
			let slice = staging_srcbuf.slice(..);
			let mut buf = slice.get_mapped_range_mut();
			for (dst, src) in buf
				.chunks_mut(buffer_align.padded_bytes_per_row as usize)
				.zip(data.chunks(buffer_align.unpadded_bytes_per_row as usize))
			{
				unsafe {
					ptr::copy_nonoverlapping(
						&src[0],
						&mut dst[0],
						buffer_align.unpadded_bytes_per_row as usize,
					);
				}
			}
			drop(slice);
		}

		staging_srcbuf.unmap();

		let staging_dstbuf = device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: u64::try_from(buffer_align.padded_bytes_per_row * height)?,
			usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
			mapped_at_creation: false,
		});

		let compute_size = wgpu::Extent3d {
			width,
			height,
			depth: 1,
		};

		let compute = device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size: compute_size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: real_format,
			usage: wgpu::TextureUsage::COPY_SRC
				| wgpu::TextureUsage::COPY_DST
				| wgpu::TextureUsage::STORAGE,
		});

		let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: None,
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStage::COMPUTE,
					ty: wgpu::BindingType::Sampler {
						filtering: true,
						comparison: false,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStage::COMPUTE,
					ty: wgpu::BindingType::Texture {
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
						view_dimension: wgpu::TextureViewDimension::D3,
						multisampled: false,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStage::COMPUTE,
					ty: wgpu::BindingType::StorageTexture {
						access: wgpu::StorageTextureAccess::ReadWrite,
						format: real_format,
						view_dimension: wgpu::TextureViewDimension::D2,
					},
					count: None,
				},
			],
		});

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&layout],
			push_constant_ranges: &[],
		});

		let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
			label: None,
			layout: Some(&pipeline_layout),
			module: &self.shader,
			entry_point: "main",
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&lutc.texture_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(
						&compute.create_view(&wgpu::TextureViewDescriptor::default()),
					),
				},
			],
		});

		let cmdbuf = {
			let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
			encoder.copy_buffer_to_texture(
				wgpu::BufferCopyView {
					buffer: &staging_srcbuf,
					layout: wgpu::TextureDataLayout {
						offset: 0,
						bytes_per_row: buffer_align.padded_bytes_per_row,
						rows_per_image: 0,
					},
				},
				wgpu::TextureCopyView {
					texture: &compute,
					mip_level: 0,
					origin: wgpu::Origin3d::ZERO,
				},
				compute_size,
			);
			{
				let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
				cpass.set_pipeline(&pipeline);
				cpass.set_bind_group(0, &bind_group, &[]);
				cpass.dispatch(width / WORKGROUP_SIZE + 1, height / WORKGROUP_SIZE + 1, 1);
			}
			encoder.copy_texture_to_buffer(
				wgpu::TextureCopyView {
					texture: &compute,
					mip_level: 0,
					origin: wgpu::Origin3d::ZERO,
				},
				wgpu::BufferCopyView {
					buffer: &staging_dstbuf,
					layout: wgpu::TextureDataLayout {
						offset: 0,
						bytes_per_row: buffer_align.padded_bytes_per_row,
						rows_per_image: 0,
					},
				},
				compute_size,
			);

			encoder.finish()
		};

		self.queue.submit(Some(cmdbuf));

		let slice = staging_dstbuf.slice(..);
		let slice_future = slice.map_async(wgpu::MapMode::Read);

		device.poll(wgpu::Maintain::Wait);

		slice_future.await?;

		{
			let buf = slice.get_mapped_range();
			for (dst, src) in data
				.chunks_mut(buffer_align.unpadded_bytes_per_row as usize)
				.zip(buf.chunks(buffer_align.padded_bytes_per_row as usize))
			{
				unsafe {
					ptr::copy_nonoverlapping(
						&src[0],
						&mut dst[0],
						buffer_align.unpadded_bytes_per_row as usize,
					);
				}
			}
			drop(slice);
		}

		staging_dstbuf.unmap();

		staging_srcbuf.destroy();
		staging_dstbuf.destroy();
		compute.destroy();

		Ok(())
	}
}
