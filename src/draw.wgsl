// Vertex shader

struct CameraUniform {
    origin:vec2<f32>;
    scaling:vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> camera: CameraUniform;


struct VertexInput {
    [[location(0)]] boid_pos:vec2<f32>;
    [[location(1)]] boid_vel:vec2<f32>;
    [[location(2)]] boid_color:vec3<f32>;
    [[location(3)]] position:vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color:vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(in: VertexInput) -> VertexOutput {
    let sqrlen = dot(in.boid_vel, in.boid_vel);
    let angle = -atan2(in.boid_vel.x / sqrt(sqrlen), in.boid_vel.y / sqrt(sqrlen));
    let v_pos = vec2<f32>(
        in.position.x * cos(angle) - in.position.y * sin(angle),
        in.position.x * sin(angle) + in.position.y * cos(angle)
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>((v_pos + in.boid_pos - camera.origin) * camera.scaling, 0.0, 1.0);
    out.color = in.boid_color;
    return out;
}

// Fragment shader
[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
