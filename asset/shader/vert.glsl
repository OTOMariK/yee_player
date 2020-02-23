#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;

layout(location = 2) in vec2 loc;
layout(location = 3) in vec2 size;
layout(location = 4) in vec3 color;

layout(location = 0) out vec2 frag_uv;
layout(location = 1) out vec4 frag_color;

void main() {
    vec2 pos2 = pos * size + loc;
    frag_uv = uv * size;
    frag_color = vec4(color, 1.0);
    gl_Position = vec4(pos2, 1.0, 1.0);
}