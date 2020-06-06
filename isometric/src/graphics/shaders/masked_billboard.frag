#version 330 core

in VS_OUTPUT {
    vec2 TexCoord;
    vec4 Color;
} IN;

out vec4 Color;

uniform sampler2D ourTexture;
uniform sampler2D ourMask;

void main()
{
    vec4 texel = texture(ourTexture, IN.TexCoord);
    vec4 mask = texture(ourMask, IN.TexCoord);
    if(texel.a == 0.0)
        discard;
    if(mask.a == 1.0) {
        Color = IN.Color;
    } else {
        Color = texel;
    }
}