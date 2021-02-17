use anyhow::Error;
use async_std::task::block_on;
use criterion::{async_executor::AsyncStdExecutor, criterion_group, criterion_main, Criterion};
use std::{mem::size_of, sync::Arc};
use wgpu_lut::Processor;

fn process(c: &mut Criterion) {
	let width: u32 = 1280;
	let height: u32 = 768;

	c.bench_function("process async", |b| {
		let p = block_on(async {
			let p = Processor::new(false).await?;

			p.add_lut("test", "cube", include_str!("./lut.cube").as_bytes())?;

			Ok::<Processor, Error>(p)
		})
		.unwrap();

		b.to_async(AsyncStdExecutor).iter(|| async {
			let mut img: Vec<u8> = (0..(width * height) as usize * 4 * size_of::<u8>())
				.map(|i| (i % 256) as u8)
				.collect();

			p.process("test", "linear", "rgba8", width, height, img.as_mut_slice())
				.await
				.unwrap();
		});
	});

	c.bench_function("process sync", |b| {
		let p = block_on(async {
			let p = Processor::new(false).await?;

			p.add_lut("test", "cube", include_str!("./lut.cube").as_bytes())?;

			Ok::<Processor, Error>(p)
		})
		.unwrap();

		b.iter(|| {
			block_on(async {
				let mut img: Vec<u8> = (0..(width * height) as usize * 4 * size_of::<u8>())
					.map(|i| (i % 256) as u8)
					.collect();

				p.process("test", "linear", "rgba8", width, height, img.as_mut_slice())
					.await
					.unwrap();
			})
		});
	});
}

criterion_group!(benches, process);

criterion_main!(benches);
