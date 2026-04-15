use std::sync::Arc;

use crate::{
    constants::{BLACK, SCREEN_HEIGHT, SCREEN_WIDTH},
    terminal::TerminalGeometry,
    vector::Vector2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Color(u8, u8, u8, u8);

impl Color {
    fn from_rgba([r, g, b, a]: [u8; 4]) -> Self {
        Self(r, g, b, a)
    }
}

#[derive(Clone, Debug)]
pub struct RenderedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct FrameData {
    pub background: Option<Arc<RenderedImage>>,
    pub circles: Vec<Circle>,
    pub lines: Vec<Line>,
    pub sprites: Vec<Sprite>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center: Vector2,
    pub radius: f32,
    pub color: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub start: Vector2,
    pub end: Vector2,
    pub color: [u8; 4],
    pub thickness: f32,
}

pub struct Renderer {
    image_width: u32,
    image_height: u32,
    render_target: RenderedImage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteAnchor {
    Center,
    TopLeft,
}

#[derive(Clone, Debug)]
pub struct Sprite {
    pub image: Arc<RenderedImage>,
    pub position: Vector2,
    pub anchor: SpriteAnchor,
}

#[derive(Clone, Copy)]
struct SceneTransform {
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl Renderer {
    pub fn new(geometry: TerminalGeometry) -> Self {
        let (image_width, image_height) = raster_size(geometry);
        Self {
            image_width,
            image_height,
            render_target: RenderedImage::new_blank(image_width, image_height),
        }
    }

    pub fn resize(&mut self, geometry: TerminalGeometry) {
        let (image_width, image_height) = raster_size(geometry);
        self.image_width = image_width;
        self.image_height = image_height;
        self.render_target.resize(image_width, image_height);
    }

    pub fn render(&mut self, frame: &FrameData) -> &RenderedImage {
        self.render_target.clear(Color::from_rgba(BLACK));

        let transform = SceneTransform::new(self.image_width as f32, self.image_height as f32);

        if let Some(background) = &frame.background {
            let (x, y, width, height) = transform.rect(
                Vector2::default(),
                background.width as f32,
                background.height as f32,
                SpriteAnchor::TopLeft,
            );
            self.render_target
                .draw_image(background, x, y, width, height);
        }

        for line in &frame.lines {
            let (start_x, start_y) = transform.point(line.start);
            let (end_x, end_y) = transform.point(line.end);
            self.render_target.draw_line(
                start_x,
                start_y,
                end_x,
                end_y,
                Color::from_rgba(line.color),
                transform.scale_scalar(line.thickness),
            );
        }

        for circle in &frame.circles {
            let (center_x, center_y, radius) =
                transform.circle(circle.center.x, circle.center.y, circle.radius);
            self.render_target.draw_filled_circle(
                center_x,
                center_y,
                radius,
                Color::from_rgba(circle.color),
            );
        }

        for sprite in &frame.sprites {
            let (x, y, width, height) = transform.rect(
                sprite.position,
                sprite.image.width as f32,
                sprite.image.height as f32,
                sprite.anchor,
            );
            self.render_target
                .draw_image(&sprite.image, x, y, width, height);
        }

        &self.render_target
    }

    pub fn scene_position_for_terminal_cell(
        &self,
        geometry: TerminalGeometry,
        column: u16,
        row: u16,
    ) -> Option<Vector2> {
        if geometry.cols == 0 || geometry.rows == 0 {
            return None;
        }

        let image_x = (column as f32 + 0.5) * self.image_width as f32 / geometry.cols as f32;
        let image_y = (row as f32 + 0.5) * self.image_height as f32 / geometry.rows as f32;
        let transform = SceneTransform::new(self.image_width as f32, self.image_height as f32);

        Some(transform.inverse_point(image_x, image_y))
    }
}

impl RenderedImage {
    fn new_blank(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixels.resize(width as usize * height as usize * 4, 0);
    }

    fn clear(&mut self, color: Color) {
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel[0] = color.0;
            pixel[1] = color.1;
            pixel[2] = color.2;
            pixel[3] = color.3;
        }
    }

    fn draw_filled_circle(&mut self, center_x: i32, center_y: i32, radius: i32, color: Color) {
        let radius = radius.max(1);
        let radius_squared = radius * radius;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius_squared {
                    self.put_pixel(center_x + dx, center_y + dy, color);
                }
            }
        }
    }

    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, thickness: i32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);

        loop {
            self.stamp(x, y, color, thickness);
            if x == x1 && y == y1 {
                break;
            }
            let doubled = err * 2;
            if doubled >= dy {
                err += dy;
                x += sx;
            }
            if doubled <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn stamp(&mut self, x: i32, y: i32, color: Color, thickness: i32) {
        let radius = thickness.saturating_sub(1);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                self.put_pixel(x + dx, y + dy, color);
            }
        }
    }

    fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 {
            return;
        }

        let x = usize::try_from(x).ok();
        let y = usize::try_from(y).ok();
        let (Some(x), Some(y)) = (x, y) else {
            return;
        };

        let width = self.width as usize;
        let height = self.height as usize;
        if x >= width || y >= height {
            return;
        }

        let index = (y * width + x) * 4;
        self.pixels[index] = color.0;
        self.pixels[index + 1] = color.1;
        self.pixels[index + 2] = color.2;
        self.pixels[index + 3] = color.3;
    }

    fn draw_image(
        &mut self,
        image: &RenderedImage,
        dest_x: i32,
        dest_y: i32,
        dest_width: i32,
        dest_height: i32,
    ) {
        if dest_width <= 0 || dest_height <= 0 || image.width == 0 || image.height == 0 {
            return;
        }

        let start_x = dest_x.max(0);
        let start_y = dest_y.max(0);
        let end_x = (dest_x + dest_width).min(self.width as i32);
        let end_y = (dest_y + dest_height).min(self.height as i32);
        if start_x >= end_x || start_y >= end_y {
            return;
        }

        for y in start_y..end_y {
            let src_y = ((y - dest_y) as u32 * image.height / dest_height as u32) as usize;
            for x in start_x..end_x {
                let src_x = ((x - dest_x) as u32 * image.width / dest_width as u32) as usize;
                let src_index = (src_y * image.width as usize + src_x) * 4;
                let src = Color(
                    image.pixels[src_index],
                    image.pixels[src_index + 1],
                    image.pixels[src_index + 2],
                    image.pixels[src_index + 3],
                );
                if src.3 == 0 {
                    continue;
                }

                self.blend_pixel(x as usize, y as usize, src);
            }
        }
    }

    fn blend_pixel(&mut self, x: usize, y: usize, source: Color) {
        let index = (y * self.width as usize + x) * 4;
        if source.3 == 255 {
            self.pixels[index] = source.0;
            self.pixels[index + 1] = source.1;
            self.pixels[index + 2] = source.2;
            self.pixels[index + 3] = 255;
            return;
        }

        let alpha = source.3 as u16;
        let inverse = 255 - alpha;

        self.pixels[index] =
            ((source.0 as u16 * alpha + self.pixels[index] as u16 * inverse + 127) / 255) as u8;
        self.pixels[index + 1] =
            ((source.1 as u16 * alpha + self.pixels[index + 1] as u16 * inverse + 127) / 255) as u8;
        self.pixels[index + 2] =
            ((source.2 as u16 * alpha + self.pixels[index + 2] as u16 * inverse + 127) / 255) as u8;
        self.pixels[index + 3] = 255;
    }
}

impl SceneTransform {
    fn new(image_width: f32, image_height: f32) -> Self {
        let scale = (image_width / SCREEN_WIDTH as f32).min(image_height / SCREEN_HEIGHT as f32);
        let content_width = SCREEN_WIDTH as f32 * scale;
        let content_height = SCREEN_HEIGHT as f32 * scale;
        let offset_x = (image_width - content_width) * 0.5;
        let offset_y = (image_height - content_height) * 0.5;

        Self {
            scale,
            offset_x,
            offset_y,
        }
    }

    fn circle(self, x: f32, y: f32, radius: f32) -> (i32, i32, i32) {
        let center_x = (self.offset_x + x * self.scale).round() as i32;
        let center_y = (self.offset_y + y * self.scale).round() as i32;
        let scaled_radius = (radius * self.scale).round() as i32;
        (center_x, center_y, scaled_radius.max(1))
    }

    fn point(self, point: Vector2) -> (i32, i32) {
        let x = (self.offset_x + point.x * self.scale).round() as i32;
        let y = (self.offset_y + point.y * self.scale).round() as i32;
        (x, y)
    }

    fn inverse_point(self, x: f32, y: f32) -> Vector2 {
        Vector2::new(
            (x - self.offset_x) / self.scale,
            (y - self.offset_y) / self.scale,
        )
    }

    fn rect(
        self,
        position: Vector2,
        width: f32,
        height: f32,
        anchor: SpriteAnchor,
    ) -> (i32, i32, i32, i32) {
        let (mut x, mut y) = self.point(position);
        let scaled_width = (width * self.scale).round() as i32;
        let scaled_height = (height * self.scale).round() as i32;

        if anchor == SpriteAnchor::Center {
            x -= scaled_width / 2;
            y -= scaled_height / 2;
        }

        (x, y, scaled_width.max(1), scaled_height.max(1))
    }

    fn scale_scalar(self, value: f32) -> i32 {
        (value * self.scale).round() as i32
    }
}

fn raster_size(geometry: TerminalGeometry) -> (u32, u32) {
    let source_width = if geometry.pixel_width > 0 {
        geometry.pixel_width as u32
    } else {
        SCREEN_WIDTH
    };
    let source_height = if geometry.pixel_height > 0 {
        geometry.pixel_height as u32
    } else {
        SCREEN_HEIGHT
    };

    scale_to_fit(
        source_width,
        source_height,
        SCREEN_WIDTH * 2,
        SCREEN_HEIGHT * 2,
    )
}

fn scale_to_fit(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (SCREEN_WIDTH, SCREEN_HEIGHT);
    }

    let scale = (max_width as f32 / width as f32)
        .min(max_height as f32 / height as f32)
        .min(1.0);

    let scaled_width = ((width as f32 * scale).round() as u32).max(SCREEN_WIDTH);
    let scaled_height = ((height as f32 * scale).round() as u32).max(SCREEN_HEIGHT);
    (scaled_width, scaled_height)
}

#[cfg(test)]
mod tests {
    use super::{Circle, FrameData, Line, Renderer, SceneTransform};
    use crate::{
        constants::{SCREEN_HEIGHT, SCREEN_WIDTH},
        terminal::TerminalGeometry,
        vector::Vector2,
    };

    fn sample_pixel(image: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * width + x) * 4) as usize;
        [
            image[index],
            image[index + 1],
            image[index + 2],
            image[index + 3],
        ]
    }

    fn screen_renderer() -> Renderer {
        Renderer::new(TerminalGeometry {
            cols: 10,
            rows: 10,
            pixel_width: SCREEN_WIDTH as u16,
            pixel_height: SCREEN_HEIGHT as u16,
        })
    }

    #[test]
    fn pacman_render_paints_yellow_pixels() {
        let mut renderer = screen_renderer();
        let frame = FrameData {
            background: None,
            circles: vec![Circle {
                center: Vector2::new(200.0, 400.0),
                radius: 10.0,
                color: [255, 255, 0, 255],
            }],
            lines: Vec::new(),
            sprites: Vec::new(),
        };

        let image = renderer.render(&frame);
        let pixel = sample_pixel(&image.pixels, image.width, 200, 400);

        assert_eq!(pixel, [255, 255, 0, 255]);
    }

    #[test]
    fn renderer_uses_the_logical_screen_size_as_a_minimum() {
        let renderer = Renderer::new(TerminalGeometry {
            cols: 10,
            rows: 10,
            pixel_width: 0,
            pixel_height: 0,
        });

        assert_eq!(renderer.image_width, SCREEN_WIDTH);
        assert_eq!(renderer.image_height, SCREEN_HEIGHT);
    }

    #[test]
    fn scene_transform_centers_the_logical_screen() {
        let transform = SceneTransform::new(SCREEN_WIDTH as f32 * 2.0, SCREEN_HEIGHT as f32 * 2.0);
        let (x, y, radius) = transform.circle(0.0, 0.0, 10.0);

        assert_eq!(x, 0);
        assert_eq!(y, 0);
        assert_eq!(radius, 20);
    }

    #[test]
    fn line_render_paints_white_pixels() {
        let mut renderer = screen_renderer();
        let frame = FrameData {
            background: None,
            circles: Vec::new(),
            lines: vec![Line {
                start: Vector2::new(80.0, 80.0),
                end: Vector2::new(160.0, 80.0),
                color: [255, 255, 255, 255],
                thickness: 4.0,
            }],
            sprites: Vec::new(),
        };

        let image = renderer.render(&frame);
        let pixel = sample_pixel(&image.pixels, image.width, 120, 80);

        assert_eq!(pixel, [255, 255, 255, 255]);
    }
}
