#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec2 TexCoord;
layout (location = 2) in vec2 Offset;

uniform mat4 projection;
uniform mat2 pixel_to_screen;
uniform float z_mod;
uniform sampler2D sceneDepths;

out VS_OUTPUT {
    vec2 TexCoord;
    float deltaZ;
} OUT;

void main()
{
    gl_Position = projection * vec4(Position, 1.0);

    vec3 screen_coords = (gl_Position.xyz + 1.0) / 2.0;
    float expectedZ = screen_coords.z;
    float sceneZ = texture(sceneDepths, screen_coords.xy).r;
    OUT.deltaZ = expectedZ - sceneZ;

    vec2 screen_offset = pixel_to_screen * Offset;
    gl_Position.x += screen_offset.x;
    gl_Position.y += screen_offset.y;
    gl_Position.z = -1.0;
    OUT.TexCoord = TexCoord;
}