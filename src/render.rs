use crate::{
    constants::{BLACK, SCREEN_HEIGHT, SCREEN_WIDTH},
    pacman::Pacman,
    terminal::TerminalGeometry,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Color(u8, u8, u8, u8);

impl Color {
    fn from_rgba([r, g, b, a]: [u8; 4]) -> Self {
        Self(r, g, b, a)
    }
}

pub struct RenderedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct Renderer {
    image_width: u32,
    image_height: u32,
}

struct PixelBuffer {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
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
        }
    }

    pub fn resize(&mut self, geometry: TerminalGeometry) {
        *self = Self::new(geometry);
    }

    pub fn render(&self, pacman: Option<&Pacman>) -> RenderedImage {
        let mut buffer = PixelBuffer::new(self.image_width as usize, self.image_height as usize);
        buffer.fill(Color::from_rgba(BLACK));

        if let Some(pacman) = pacman {
            let transform = SceneTransform::new(self.image_width as f32, self.image_height as f32);
            let (center_x, center_y, radius) =
                transform.circle(pacman.position().x, pacman.position().y, pacman.radius());
            buffer.draw_filled_circle(center_x, center_y, radius, Color::from_rgba(pacman.color()));
        }

        RenderedImage {
            width: self.image_width,
            height: self.image_height,
            pixels: buffer.pixels,
        }
    }
}

impl PixelBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height * 4],
        }
    }

    fn fill(&mut self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x as i32, y as i32, color);
            }
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

    fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 {
            return;
        }

        let x = usize::try_from(x).ok();
        let y = usize::try_from(y).ok();
        let (Some(x), Some(y)) = (x, y) else {
            return;
        };

        if x >= self.width || y >= self.height {
            return;
        }

        let index = (y * self.width + x) * 4;
        self.pixels[index] = color.0;
        self.pixels[index + 1] = color.1;
        self.pixels[index + 2] = color.2;
        self.pixels[index + 3] = color.3;
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
    use super::{Renderer, SceneTransform};
    use crate::{
        constants::{SCREEN_HEIGHT, SCREEN_WIDTH},
        pacman::Pacman,
        terminal::TerminalGeometry,
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

    #[test]
    fn pacman_render_paints_yellow_pixels() {
        let renderer = Renderer {
            image_width: SCREEN_WIDTH,
            image_height: SCREEN_HEIGHT,
        };
        let pacman = Pacman::new();

        let image = renderer.render(Some(&pacman));
        let pixel = sample_pixel(
            &image.pixels,
            image.width,
            pacman.position().x as u32,
            pacman.position().y as u32,
        );

        assert_eq!(pixel, [255, 255, 0, 255]);
    }

    #[test]
    fn renderer_uses_tutorial_screen_size_as_a_minimum() {
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
}
