struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) shape: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex: u32,
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) shape: vec4<f32>,
) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    let uv = corners[vertex];
    var out: VertexOutput;
    out.position = vec4(rect.xy + uv * rect.zw, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    out.shape = shape;
    return out;
}

@group(0) @binding(0) var image: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // A circle covers integer pixel centers, including its boundary.
    if in.shape.x == 1.0 {
        let radius = in.shape.y;
        let pixel = floor(in.uv * (2.0 * radius + 1.0)) - radius;
        if dot(pixel, pixel) > radius * radius {
            discard;
        }
    }
    if in.shape.x == 2.0 {
        let size = textureDimensions(image);
        let destination_pixel = floor(in.uv * in.shape.yz);
        let pixel = min(vec2<u32>(destination_pixel * vec2<f32>(size) / in.shape.yz), size - vec2<u32>(1));
        return textureLoad(image, vec2<i32>(pixel), 0);
    }
    return in.color;
}
