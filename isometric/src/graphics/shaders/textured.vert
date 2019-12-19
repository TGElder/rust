#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec4 Color;
layout (location = 2) in vec2 TexCoord;

uniform mat4 projection;
uniform float z_mod;

out VS_OUTPUT {
    vec4 Color;
    vec2 TexCoord;
} OUT;

void main()
{
    gl_Position = projection * vec4(Position, 1.0);
    gl_Position.z += z_mod;
    OUT.Color = Color;
    OUT.TexCoord = TexCoord;
}