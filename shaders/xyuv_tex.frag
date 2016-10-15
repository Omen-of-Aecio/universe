#version 150

in vec2 f_texpos;
uniform sampler2D sampler;

out vec4 out_color;


void main() {
    float inside = float(f_texpos.x >= 0.0 && f_texpos.y >= 0.0 && f_texpos.x <= 1.0 && f_texpos.y <= 1.0);
    float intensity = texture(sampler, f_texpos).x * inside;
    out_color = vec4(vec3(intensity), 1);
}
