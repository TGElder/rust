in vec2 position;
in vec2 texCoords;

out vec2 TexCoords;

void main()
{
    gl_Position = vec4(position.x, position.y, 0.0, 1.0); 
    TexCoords = texCoords;
}