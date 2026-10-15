#import bevy_pbr::forward_io::VertexOutput

// Rolling-tube look for the toy screen (PLAN M10). All effects scale with
// `toy_look` (0 = flat modern game, 1 = full physical-drum illusion).

struct ScreenParams {
    toy_look: f32,
    time: f32,
    speed: f32, // normalized drum speed 0..1
    pad: f32,
}

@group(2) @binding(0) var game_texture: texture_2d<f32>;
@group(2) @binding(1) var game_sampler: sampler;
@group(2) @binding(2) var<uniform> params: ScreenParams;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7)) + params.time * 13.0) * 43758.5453);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let look = params.toy_look;
    var uv = in.uv;

    // drum curvature: horizontal pinch toward the cylinder center line as
    // you move away from the middle of the screen
    let vc = (uv.y - 0.5) * 2.0;
    uv.x = 0.5 + (uv.x - 0.5) * (1.0 - look * 0.10 * vc * vc);

    var color = textureSample(game_texture, game_sampler, uv).rgb;

    // warm lamp glow inside the tube, brighter toward the top (uv.y = 1)
    let lamp = 0.70 + 0.55 * uv.y;
    color = mix(color, color * lamp * vec3<f32>(1.06, 1.0, 0.90), look);

    // screen recess vignette
    let d = distance(uv, vec2<f32>(0.5, 0.5)) * 1.55;
    color *= 1.0 - look * 0.38 * d * d;

    // print banding on the drum, scrolling with the motor
    let band = sin(uv.y * 460.0 - params.time * params.speed * 34.0);
    color *= 1.0 - look * 0.05 * band;

    // paper grain + speed-tied brightness flicker
    color += look * (hash(in.uv) - 0.5) * 0.045;
    color *= 1.0 - look * 0.035 * (0.5 + 0.5 * sin(params.time * 93.0)) * params.speed;

    return vec4<f32>(color, 1.0);
}
