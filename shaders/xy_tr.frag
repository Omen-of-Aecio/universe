#version 150

uniform vec3 color;
out vec4 out_color;
void main() {
    /* out_color = vec4(color, 1); */
    out_color = vec4(0.5, 0.5, 0.5, 1);
}
