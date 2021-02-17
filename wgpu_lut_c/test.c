#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef uint8_t (*add_lut_t)(const char *name,		// lut table name, unique
														 const char *format,	// "cube"(.cube)
														 const uint8_t *lut,
														 uint64_t lut_len);
typedef uint8_t (*add_lut_raw_t)(
		const char *name,
		uint32_t lut_dim,
		const uint8_t *lut);	// dim * dim * dim * 3(rgb) * f32 array, it is slow
typedef uint8_t (*add_lut_raw_alpha_t)(
		const char *name,
		uint32_t lut_dim,
		const uint8_t *lut);	// dim * dim * dim * 4(rgba) * f32 array
typedef uint8_t (*del_lut_t)(const char *name);
typedef uint8_t (*process_t)(const char *lut,
														 const char *sampler,
														 const char *format,
														 uint32_t width,
														 uint32_t height,
														 const uint8_t *data,
														 uint64_t data_len);
char *const lut =
		"TITLE \"Generate by Resolve\"\n"
		"LUT_3D_SIZE 2\n"
		"0.000000 0.000000 0.000000\n"
		"0.011627 0.000366 0.001144\n"
		"0.024033 0.000000 0.001678\n"
		"0.033310 0.000031 0.001007\n"
		"0.043091 0.002518 0.000198\n"
		"0.056031 0.008942 0.000000\n"
		"0.078706 0.008453 0.000000\n"
		"0.109682 0.003082 0.000000\n";

int main(void) {
	void *handle = dlopen("./target/debug/libwgpu_lut_c.so", 0);
	if (handle == NULL) {
		goto clean;
	}

	add_lut_t add_lut = (add_lut_t)dlsym(handle, "add_lut");
	add_lut_raw_t add_lut_raw = (add_lut_raw_t)dlsym(handle, "add_lut_raw");
	add_lut_raw_alpha_t add_lut_raw_alpha =
			(add_lut_raw_alpha_t)dlsym(handle, "add_lut_raw_alpha");
	del_lut_t del_lut = (del_lut_t)dlsym(handle, "del_lut");
	process_t process = (process_t)dlsym(handle, "process");

	add_lut("test", "cube", (uint8_t *const)lut, strlen(lut));

clean:
	return dlclose(handle);
}
