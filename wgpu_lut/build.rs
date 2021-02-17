use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() -> Result<()> {
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=data");

	let out_dir = env::var_os("OUT_DIR").unwrap();
	for e in fs::read_dir("data")? {
		let path = e?.path();
		let code = fs::read_to_string(&path)?;
		let spirv = glsl_to_spirv::compile(code.as_str(), glsl_to_spirv::ShaderType::Compute)
			.map_err(|e| anyhow!(e))?;

		let filename = path
			.file_stem()
			.and_then(|s| s.to_str())
			.ok_or(anyhow!("unknown filename"))?;
		let shader =
			gfx_auxil::read_spirv(spirv).map_err(|e| anyhow!("auxil_read {:?}", e.to_string()))?;
		let mut w = Vec::new();

		write!(
			&mut w,
			"pub const SHADER_{}: [u32; {}] = [",
			filename.to_uppercase(),
			shader.len()
		)?;
		for (i, j) in shader.iter().enumerate() {
			if i > 0 {
				write!(&mut w, ",")?;
			}
			write!(&mut w, "{}", j)?;
		}
		writeln!(&mut w, "];")?;

		let dest_path = Path::new(&out_dir).join(format!("shader_{}.rs", filename));
		fs::write(dest_path, w)?;
	}
	Ok(())
}
