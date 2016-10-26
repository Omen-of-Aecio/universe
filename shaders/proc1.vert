#version 330

in vec2 pos;
out vec2 texpos;

void main() {
    texpos = (pos + 1)/2;
    gl_Position = vec4(pos, 0, 1);
}
