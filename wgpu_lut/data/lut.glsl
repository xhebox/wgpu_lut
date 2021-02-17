#version 450

layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(set = 0, binding = 0) uniform sampler u_sampler;

layout(set = 0, binding = 1) uniform texture3D u_lut;

layout(set = 0, binding = 2, rgba8) uniform image2D img;

void main() {
	ivec2 i = ivec2(gl_GlobalInvocationID.xy);
	vec4 pixel = imageLoad(img, i);
	imageStore(img, i, vec4(texture(sampler3D(u_lut, u_sampler), pixel.rgb).rgb, pixel.a));
}
