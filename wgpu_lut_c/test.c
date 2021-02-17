#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef uint8_t (*add_lut_t)(char *name,
														 char *format,
														 uint8_t *const lut,
														 uint64_t lut_len);
typedef uint8_t (*del_lut_t)(char *name);
typedef uint8_t (*process_t)(char *lut,
														 char *sampler,
														 char *format,
														 uint32_t width,
														 uint32_t height,
														 uint8_t *const data,
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
	del_lut_t del_lut = (del_lut_t)dlsym(handle, "del_lut");
	process_t process = (process_t)dlsym(handle, "process");

	add_lut("test", "cube", (uint8_t *const)lut, strlen(lut));

clean:
	return dlclose(handle);
}
