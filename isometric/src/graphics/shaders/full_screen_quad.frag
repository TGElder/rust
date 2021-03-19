#version 330 core

in vec2 TexCoords;
out vec4 Color;

uniform sampler2D screenTexture;

void main()
{ 
    Color = texture(screenTexture, TexCoords);
}