//! Draws cached sprite textures and primitive instances directly on the GPU.

use std::{
    collections::HashMap,
    ops::Range,
    sync::{Arc, Weak},
};

use crate::{
    render::{FrameData, RenderedImage, SceneTransform, Sprite, SpriteAnchor},
    vector::Vector2,
};

pub(crate) const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Quad {
    rect: [f32; 4],
    color: [f32; 4],
    shape: [f32; 4],
}

struct Draw {
    texture: usize,
    instances: Range<u32>,
}

struct ScenePlan {
    size: [u32; 2],
    transform: SceneTransform,
    quads: Vec<Quad>,
    draws: Vec<Draw>,
}

struct CachedTexture {
    image: Weak<RenderedImage>,
    bind_group: wgpu::BindGroup,
}

pub(crate) struct SceneRenderer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    textures: HashMap<usize, CachedTexture>,
    white: Arc<RenderedImage>,
    instances: wgpu::Buffer,
    capacity: u64,
}

impl SceneRenderer {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Pac-Man sprite texture layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pac-Man GPU scene shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_scene.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pac-Man GPU scene layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pac-Man GPU scene pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                }],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            layout,
            textures: HashMap::new(),
            white: Arc::new(RenderedImage {
                width: 1,
                height: 1,
                pixels: vec![255; 4],
            }),
            instances: instance_buffer(device, 48),
            capacity: 48,
        }
    }

    fn texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Arc<RenderedImage>,
    ) -> usize {
        let key = Arc::as_ptr(image) as usize;
        self.textures.entry(key).or_insert_with(|| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Pac-Man cached sprite"),
                size: wgpu::Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SCENE_FORMAT,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            queue.write_texture(
                texture.as_image_copy(),
                &image.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width * 4),
                    rows_per_image: Some(image.height),
                },
                texture.size(),
            );
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Pac-Man cached sprite binding"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                }],
            });
            // Weak ownership permits expired score/text images to leave the GPU cache.
            CachedTexture {
                image: Arc::downgrade(image),
                bind_group,
            }
        });
        key
    }

    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
        frame: &FrameData,
    ) {
        self.textures
            .retain(|_, texture| texture.image.strong_count() > 0);
        let white = self.white.clone();
        let white_key = self.texture(device, queue, &white);
        let transform = SceneTransform::new(size[0] as f32, size[1] as f32);
        let mut plan = ScenePlan {
            size,
            transform,
            quads: Vec::new(),
            draws: Vec::new(),
        };
        if let Some(image) = &frame.background {
            self.add_image(
                device,
                queue,
                &mut plan,
                &Sprite {
                    image: image.clone(),
                    position: Vector2::default(),
                    anchor: SpriteAnchor::TopLeft,
                },
            );
        }
        let start = plan.quads.len() as u32;
        for line in &frame.lines {
            let (x0, y0) = transform.point(line.start);
            let (x1, y1) = transform.point(line.end);
            let radius = transform
                .scale_scalar(line.thickness)
                .saturating_sub(1)
                .max(0);
            // Submit Bresenham stamps as plan.quads; pixel coverage is rasterized by the GPU.
            line_points(x0, y0, x1, y1, |x, y| {
                plan.quads.push(quad(
                    size,
                    (x - radius, y - radius, radius * 2 + 1, radius * 2 + 1),
                    line.color,
                    [0.0; 4],
                ));
            });
        }
        for circle in &frame.circles {
            let (x, y, radius) = transform.circle(circle.center.x, circle.center.y, circle.radius);
            plan.quads.push(quad(
                size,
                (x - radius, y - radius, radius * 2 + 1, radius * 2 + 1),
                circle.color,
                [1.0, radius as f32, 0.0, 0.0],
            ));
        }
        if plan.quads.len() as u32 > start {
            plan.draws.push(Draw {
                texture: white_key,
                instances: start..plan.quads.len() as u32,
            });
        }
        for sprite in &frame.sprites {
            self.add_image(device, queue, &mut plan, sprite);
        }
        let bytes = bytemuck::cast_slice(&plan.quads);
        if bytes.len() as u64 > self.capacity {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            self.instances = instance_buffer(device, self.capacity);
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.instances, 0, bytes);
        }
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pac-Man GPU scene composition"),
            color_attachments: &attachments,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        for draw in plan.draws {
            pass.set_bind_group(0, &self.textures[&draw.texture].bind_group, &[]);
            pass.draw(0..6, draw.instances);
        }
    }

    fn add_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: &mut ScenePlan,
        sprite: &Sprite,
    ) {
        let image = &sprite.image;
        if image.width == 0 || image.height == 0 {
            return;
        }
        let key = self.texture(device, queue, image);
        let start = plan.quads.len() as u32;
        let rect = plan.transform.rect(
            sprite.position,
            image.width as f32,
            image.height as f32,
            sprite.anchor,
        );
        plan.quads.push(quad(
            plan.size,
            rect,
            [255; 4],
            [2.0, rect.2 as f32, rect.3 as f32, 0.0],
        ));
        plan.draws.push(Draw {
            texture: key,
            instances: start..start + 1,
        });
    }
}

fn instance_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Pac-Man scene instances"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn quad(size: [u32; 2], rect: (i32, i32, i32, i32), color: [u8; 4], shape: [f32; 4]) -> Quad {
    Quad {
        rect: [
            rect.0 as f32 * 2.0 / size[0] as f32 - 1.0,
            1.0 - rect.1 as f32 * 2.0 / size[1] as f32,
            rect.2 as f32 * 2.0 / size[0] as f32,
            -(rect.3 as f32) * 2.0 / size[1] as f32,
        ],
        color: color.map(|value| f32::from(value) / 255.0),
        shape,
    }
}

fn line_points(mut x: i32, mut y: i32, x1: i32, y1: i32, mut visit: impl FnMut(i32, i32)) {
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        visit(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let doubled = error * 2;
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
#[path = "gpu_scene_tests.rs"]
mod tests;
