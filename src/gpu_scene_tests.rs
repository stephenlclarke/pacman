//! Native GPU readback comparisons against the software reference.

use super::*;
use crate::{
    arcade::ORIGINAL_FRAME_TIME,
    game::{Game, UpdateInput},
    render::{Circle, Line, RenderTargetSize, Renderer, Sprite},
};

fn read_scene(
    renderer: &mut SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frame: &FrameData,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("GPU parity target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SCENE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU parity readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.encode(
        device,
        queue,
        &mut encoder,
        &target.create_view(&Default::default()),
        [width, height],
        frame,
    );
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        target.size(),
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("readback receiver remains alive");
    });
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU polling");
    receive
        .recv()
        .expect("readback callback")
        .expect("readback mapping");
    let mapped = buffer.slice(..).get_mapped_range();
    let pixels = mapped
        .chunks(stride as usize)
        .flat_map(|row| row[..width as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    pixels
}

#[test]
#[ignore = "requires a working native GPU; run with make gpu-test"]
fn gpu_composition_matches_software_and_reuses_textures() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("native GPU adapter");
    eprintln!("GPU parity adapter: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&Default::default())).expect("GPU device");
    let mut gpu = SceneRenderer::new(&device);
    let mut game = Game::new();
    let title = game.frame();
    game.update_with_input(
        ORIGINAL_FRAME_TIME,
        UpdateInput {
            start_requested: true,
            ..Default::default()
        },
    );
    for _ in 0..180 {
        game.update_with_input(ORIGINAL_FRAME_TIME, UpdateInput::default());
    }
    let gameplay = game.frame();
    let custom = FrameData {
        background: None,
        circles: vec![Circle {
            center: Vector2::new(40.0, 45.0),
            radius: 4.0,
            color: [255, 200, 0, 255],
        }],
        lines: vec![Line {
            start: Vector2::new(1.0, 3.0),
            end: Vector2::new(90.0, 80.0),
            color: [0, 255, 255, 255],
            thickness: 2.0,
        }],
        sprites: vec![Sprite {
            image: Arc::new(RenderedImage {
                width: 2,
                height: 2,
                pixels: vec![
                    255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 255, 255,
                ],
            }),
            position: Vector2::new(41.0, 45.0),
            anchor: SpriteAnchor::Center,
        }],
    };
    for (scene, frame) in [
        ("title", &title),
        ("gameplay", &gameplay),
        ("primitives", &custom),
    ] {
        for (width, height) in [(448, 576), (896, 1152), (641, 783), (801, 576)] {
            let expected = Renderer::new(RenderTargetSize::new(width, height))
                .render(frame)
                .pixels
                .clone();
            let actual = read_scene(&mut gpu, &device, &queue, frame, width, height);
            let differences: Vec<_> = expected
                .iter()
                .zip(&actual)
                .enumerate()
                .filter(|(_, (a, b))| a.abs_diff(**b) > 1)
                .take(12)
                .collect();
            assert!(
                differences.is_empty(),
                "{scene} {width}x{height} differs: {differences:?}"
            );
            let count = gpu.textures.len();
            let second = read_scene(&mut gpu, &device, &queue, frame, width, height);
            assert_eq!(actual, second);
            assert_eq!(
                count,
                gpu.textures.len(),
                "live images should reuse GPU textures"
            );
        }
    }
    drop(title);
    drop(gameplay);
    let expired_key = Arc::as_ptr(&custom.sprites[0].image) as usize;
    drop(custom);
    drop(game);
    read_scene(&mut gpu, &device, &queue, &FrameData::default(), 448, 576);
    assert!(
        !gpu.textures.contains_key(&expired_key),
        "expired images should leave the GPU cache"
    );
}
