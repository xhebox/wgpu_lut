use anyhow::Error;
use async_std::task::block_on;
use clap::{App, Arg};
use image::io::Reader as ImageReader;
use std::fs;
use wgpu_lut::Processor;

fn main() {
	let matches = App::new("Apply LUT")
		.version("0.1")
		.author("xhe <xw897002528@gmail.com>")
		.arg(
			Arg::with_name("sampler")
				.short("s")
				.long("sampler")
				.default_value("linear")
				.help("set sampler, there is nearest/linear")
				.takes_value(true),
		)
		.arg(
			Arg::with_name("lut")
				.short("l")
				.long("lut")
				.help("Sets the input lut filter, only 3d .cube")
				.required(true)
				.index(1),
		)
		.arg(
			Arg::with_name("input")
				.short("i")
				.long("input")
				.help("Sets the input image")
				.required(true)
				.index(2),
		)
		.arg(
			Arg::with_name("output")
				.short("o")
				.long("output")
				.default_value("output.png")
				.help("Sets the ouput image")
				.index(3),
		)
		.get_matches();

	wgpu_subscriber::initialize_default_subscriber(None);

	block_on(async {
		let sampler = matches.value_of("sampler").unwrap();
		let input = matches.value_of("input").unwrap();
		let output = matches.value_of("output").unwrap();
		let lut = matches.value_of("lut").unwrap();

		let img = ImageReader::open(input)?.decode()?;

		let mut img_rgba8 = img.to_rgba8();

		let mut p = Processor::new(true).await?;

		p.add_lut("test", "cube", fs::read_to_string(lut)?.as_bytes())?;

		p.process("test", sampler, "rgba8", 
							img_rgba8.width(),
							img_rgba8.height(),
							img_rgba8.as_mut()).await?;

		img_rgba8.save(output)?;

		Ok::<(), Error>(())
	})
	.unwrap();
}
