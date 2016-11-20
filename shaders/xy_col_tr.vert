#version 150
uniform mat4 proj, view;

in vec2 pos;
in vec3 col;

out vec4 f_col;

void main()
{
    f_col = vec4(col, 1);

    gl_Position = proj * view * vec4(pos, 0, 1);
}
