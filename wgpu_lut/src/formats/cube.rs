use anyhow::{anyhow, bail, Result};
use std::{
	convert::TryFrom,
	io::{BufRead, BufReader, Read},
};

pub fn parse<I>(input: I) -> Result<(u32, Vec<u8>)>
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

	Ok((dim, buffer))
}
