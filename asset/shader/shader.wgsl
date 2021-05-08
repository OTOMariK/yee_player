struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] in_pos: vec2<f32>,
    [[location(1)]] in_uv_vs: vec2<f32>,
    [[location(2)]] in_instance_loc: vec2<f32>,
    [[location(3)]] in_instance_size: vec2<f32>,
    [[location(4)]] in_instance_color: vec3<f32>,
    ) -> VertexOutput {
    var pos2: vec2<f32> = in_pos * in_instance_size + in_instance_loc;
    var out: VertexOutput;
    out.uv = in_uv_vs * in_instance_size;
    out.color = vec4<f32>(in_instance_color, 1.0);
    out.position = vec4<f32>(pos2, 0.0, 1.0);
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32>{
    return in.color;
}
