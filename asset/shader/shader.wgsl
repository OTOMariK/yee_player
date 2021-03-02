[[location(0)]]
var<in> in_pos: vec2<f32>;
[[location(1)]]
var<in> in_uv_vs: vec2<f32>;
[[location(2)]]
var<in> in_instance_loc: vec2<f32>;
[[location(3)]]
var<in> in_instance_size: vec2<f32>;
[[location(4)]]
var<in> in_instance_color: vec3<f32>;

[[location(0)]]
var<out> out_uv: vec2<f32>;
[[location(1)]]
var<out> out_color: vec4<f32>;
[[builtin(position)]]
var<out> out_position: vec4<f32>;

[[stage(vertex)]]
fn vs_main() {
    var pos2: vec2<f32> = in_pos * in_instance_size + in_instance_loc;
    out_uv = in_uv_vs * in_instance_size;
    out_color = vec4<f32>(in_instance_color, 1.0);
    out_position = vec4<f32>(pos2, 0.0, 1.0);
}

[[location(0)]]
var<in> in_uv_fs: vec2<f32>;
[[location(1)]]
var<in> in_color_fs: vec4<f32>;
[[location(0)]]
var<out> out_color: vec4<f32>;

[[stage(fragment)]]
fn fs_main() {
    out_color = in_color_fs;
}
