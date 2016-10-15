#version 150

in vec2 pos;
in vec2 texpos;

out vec2 f_texpos; // fragment texture position

uniform vec2 tex_lefttop;
uniform vec2 tex_size;

void main() {
    // `texpos` isn't really the texture position... it just helps us calculate it
    f_texpos = tex_lefttop + tex_size * texpos;
    gl_Position = vec4(pos, 0, 1);
}
