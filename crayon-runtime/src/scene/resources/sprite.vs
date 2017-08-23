#version 150
in vec3 Position;
in vec4 Color0;
in vec4 Color1;
in vec2 Texcoord0;

uniform mat4 u_View;
uniform mat4 u_Proj;

out vec4 v_Color;
out vec4 v_Additive;
out vec2 v_Texcoord;

void main() {
    gl_Position = u_Proj * u_View * vec4(Position.xy, 0.0, 1.0);
    v_Color = Color0;
    v_Additive = Color1;
    v_Texcoord = Texcoord0;
}