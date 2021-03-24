#version 330 core

in vec2 TexCoords;
out vec4 Color;

uniform sampler2D screenTexture;
uniform sampler2D depthTexture;

void main()
{ 
    vec4 color = texture(screenTexture, TexCoords);
    float a = texture(depthTexture, TexCoords).r;

    Color = vec4(color.rgb, a);
}