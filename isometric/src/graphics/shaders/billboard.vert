#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec2 TexCoord;
layout (location = 2) in vec2 Offset;

uniform mat4 projection;
uniform mat3 world_to_screen;
uniform float z_mod;

out VS_OUTPUT {
    vec2 TexCoord;
} OUT;

void main()
{
    gl_Position = projection * vec4(Position, 1.0);
    vec3 screen_offset = world_to_screen * vec3(Offset, -Offset.y);
    gl_Position.x += screen_offset.x;
    gl_Position.y += screen_offset.y;
    gl_Position.z += screen_offset.z;
    OUT.TexCoord = TexCoord;
}