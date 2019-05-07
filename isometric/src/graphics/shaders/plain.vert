#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec3 Color;

uniform mat4 projection;
uniform float z_mod;

out VS_OUTPUT {
    vec3 Color;
} OUT;

void main()
{
    gl_Position = projection * vec4(Position, 1.0);
    gl_Position.z += z_mod;
    OUT.Color = Color;
}