import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var color_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var color_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(color_texture, color_sampler, mesh.uv);
}
