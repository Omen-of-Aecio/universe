#version 150
uniform mat4 proj, view;

uniform vec2 center; // or translation
uniform float orientation;

in vec2 pos;


void main()
{
    float cosine = cos(orientation);
    float sine = sin(orientation);
    gl_Position = vec4(
            cosine * pos.x - sine   * pos.y   + center.x,
            sine   * pos.x + cosine * pos.y   + center.y,
            0,
            1);
    gl_Position = proj * view * gl_Position;
}
