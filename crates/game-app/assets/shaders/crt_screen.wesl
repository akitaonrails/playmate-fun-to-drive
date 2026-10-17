// Fast single-pass CRT treatment inspired by Timothy Lottes' CRTS/Lottes
// family: visible scanline beam, RGB aperture grille, tube warp, corner
// vignette, a restrained rolling beam, and exposure compensation.
import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var source: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var source_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> params: vec4<f32>;

fn warped_uv(uv: vec2<f32>) -> vec2<f32> {
    var p = uv * 2.0 - 1.0;
    p *= vec2<f32>(1.0 + p.y * p.y * 0.035, 1.0 + p.x * p.x * 0.024);
    return p * 0.5 + 0.5;
}

fn aperture_grille(pixel: vec2<f32>) -> vec3<f32> {
    let lane = i32(floor(pixel.x)) % 3;
    if lane == 0 { return vec3<f32>(1.24, 0.72, 0.72); }
    if lane == 1 { return vec3<f32>(0.72, 1.24, 0.72); }
    return vec3<f32>(0.72, 0.72, 1.24);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let enabled = params.x;
    let dimensions = vec2<f32>(textureDimensions(source));
    let clean = textureSample(source, source_sampler, mesh.uv).rgb;
    let uv = warped_uv(mesh.uv);
    if enabled > 0.5 && (uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0) {
        return vec4<f32>(0.004, 0.006, 0.009, 1.0);
    }

    let offset = vec2<f32>(1.15 / dimensions.x, 0.0);
    let center = textureSample(source, source_sampler, uv).rgb;
    let aberrated = vec3<f32>(
        textureSample(source, source_sampler, uv + offset).r,
        center.g,
        textureSample(source, source_sampler, uv - offset).b,
    );

    // A brightness-dependent beam keeps highlights broad while dark pixels
    // retain strong scanline separation.
    let luma = dot(aberrated, vec3<f32>(0.2126, 0.7152, 0.0722));
    let phase = uv.y * dimensions.y * 3.14159265;
    let scan = mix(0.62, 0.76, clamp(luma, 0.0, 1.0)) + 0.24 * sin(phase) * sin(phase);
    let mask = aperture_grille(uv * dimensions);
    let edge = 16.0 * uv.x * uv.y * (1.0 - uv.x) * (1.0 - uv.y);
    let vignette = mix(0.62, 1.0, pow(clamp(edge, 0.0, 1.0), 0.22));
    let roll_y = fract(params.y * 0.075);
    let rolling_beam = 1.0 + 0.055 * exp(-pow((uv.y - roll_y) * 24.0, 2.0));
    let crt = aberrated * scan * mask * vignette * rolling_beam * 1.34;
    let strength = clamp(params.z * 1.45, 0.55, 1.0);
    let color = mix(clean, crt, enabled * strength) * params.w;
    return vec4<f32>(color, 1.0);
}
