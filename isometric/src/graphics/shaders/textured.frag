#version 330 core

in VS_OUTPUT {
    vec4 Color;
    vec2 TexCoord;
} IN;

out vec4 outColor;

uniform sampler2D ourTexture;

void main()
{
    vec4 texel = texture(ourTexture, IN.TexCoord);
    outColor = mix(texel, IN.Color, IN.Color.a);
    outColor.a = 1.0;
}