#version 330 core

in VS_OUTPUT {
    vec2 TexCoord;
    float deltaZ;
} IN;

out vec4 Color;

uniform sampler2D ourTexture;

void main()
{
    if(IN.deltaZ >= 0.01)
        discard;
    vec4 texel = texture(ourTexture, IN.TexCoord);
    if(texel.a == 0.0)
        discard;
    Color = texel;
}