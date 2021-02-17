use anyhow::Error;
use async_std::task::block_on;
use lazy_static::lazy_static;
use std::{ffi::CStr, os::raw::c_char, slice};
use wgpu_lut::Processor;

lazy_static! {
	static ref P: Processor = {
		block_on(async {
			let p = Processor::new(false).await?;
			Ok::<Processor, Error>(p)
		})
		.unwrap()
	};
}

#[no_mangle]
pub extern "C" fn add_lut(
	name: *const c_char,
	format: *const c_char,
	lut: *const u8,
	lut_len: u64,
) -> u8 {
	let (n, f, l) = unsafe {
		let n = CStr::from_ptr(name).to_str();
		if n.is_err() {
			return 1;
		}
		let f = CStr::from_ptr(format).to_str();
		if f.is_err() {
			return 2;
		}
		let l = slice::from_raw_parts(lut, lut_len as usize);
		(n.unwrap(), f.unwrap(), l)
	};
	if P.add_lut(n, f, l).is_err() {
		return 4;
	}
	return 0;
}

#[no_mangle]
pub extern "C" fn process(
	lut: *const c_char,
	sampler: *const c_char,
	format: *const c_char,
	width: u32,
	height: u32,
	data: *mut u8,
	data_len: u64,
) -> u8 {
	let (l, s, f, d) = unsafe {
		let l = CStr::from_ptr(lut).to_str();
		if l.is_err() {
			return 1;
		}
		let n = CStr::from_ptr(sampler).to_str();
		if n.is_err() {
			return 2;
		}
		let f = CStr::from_ptr(format).to_str();
		if f.is_err() {
			return 3;
		}
		let d = slice::from_raw_parts_mut(data, data_len as usize);
		(l.unwrap(), n.unwrap(), f.unwrap(), d)
	};
	if block_on(async { P.process(l, s, f, width, height, d).await }).is_err() {
		return 4;
	}
	return 0;
}
