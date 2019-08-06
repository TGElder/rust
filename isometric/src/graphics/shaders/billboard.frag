#version 330 core

in VS_OUTPUT {
    vec2 TexCoord;
} IN;

out vec4 Color;

uniform sampler2D ourTexture;

void main()
{
    vec4 texel = texture(ourTexture, IN.TexCoord);
    if(texel.a == 0.0)
        discard;
    if(texel.a <= 1.0)
        texel.a = 1.0;
    Color = texel;
}