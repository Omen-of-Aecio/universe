#version 150

in vec2 pos;

out vec2 texpos;            // in texels!

uniform vec2 view_size;     // viewport size (pixels)
uniform vec2 screen_center; // (pixels)


void main() {

    texpos = (screen_center + view_size/2.0 * pos);

    gl_Position = vec4(pos, 0, 1);
}
