use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use crate::*;

pub(crate) struct PrettyRender {
    /// the tree we render
    tree: Tree,
}

#[derive(Copy, Clone, Debug)]
pub struct Normal(Vector2);

impl Normal {
    pub fn implied_z_sqr(&self) -> f32 {
        1.0 - self.0.length_sqr()
    }
    pub fn implied_z(&self) -> f32 {
        self.implied_z_sqr().sqrt()
    }
    pub fn to_vec3(&self) -> Vector3 {
        Vector3::new(self.0.x, self.0.y, self.implied_z())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Pixel {
    /// (unlit) color of the pixel
    /// maybe just index into color palette?
    color: Color,
    /// Normal direction the drawn geometry points to
    normal: Normal,
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            // transparent
            color: Color::new(0, 0, 0, 0),
            // s.t. implied z is zero and this pixel gets overdrawn always
            normal: Normal(Vector2::new(0.0, 1.0)),
        }
    }
}

impl Pixel {
    /// returns whether this pixel should be drawn in front of the other pixel
    fn covers(&self, other: &Self) -> bool {
        self.normal.implied_z_sqr() > other.normal.implied_z_sqr()
    }
}

pub struct Sprite {
    pixels: Vec<(usize, usize, Pixel)>,
}

#[derive(Copy, Clone, Debug, Default)]
/// How much in shade a pixel is
///
/// 0 = exposed to light
/// 1 = shadow
/// 2 = double shadow??
pub struct ShadowSample(f32);

#[derive(Clone)]
pub struct Canvas {
    pixel_size: i32,
    sun: Normal,
    pixels: Vec<Vec<Pixel>>,
    light: Vec<Vec<ShadowSample>>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, sun: Normal, pixel_size: i32) -> Self {
        Self {
            pixels: vec![vec![Pixel::default(); height]; width],
            light: vec![vec![ShadowSample::default(); height]; width],
            sun,
            pixel_size,
        }
    }
    pub fn draw_pixel(&mut self, x: usize, y: usize, mut pixel: Pixel, translucency: f32) {
        pixel.normal.0 = pixel.normal.0.lerp(self.sun.0, translucency);

        if pixel.covers(&self.pixels[x][y]) {
            self.pixels[x][y] = pixel;
        }
        // TODO: shadow
    }
    /// Draws a sphere onto the canvas
    ///
    /// center: the center of the sphere
    ///
    /// radius: the radius of the sphere
    ///
    /// color: the color of the Sphere
    ///
    /// translucency: how much light the sphere lets through (0 = no light, 1 = full light)
    pub fn draw_sphere(&mut self, center: Vector2, radius: f32, color: Color, translucency: f32) {
        let from = |x: f32| (x.max(radius) - radius) as usize;
        let to = |x: f32, bound: usize| ((x + radius) as usize).min(bound - 1);
        let (from_x, from_y) = (from(center.x), from(center.y));
        let (to_x, to_y) = (
            to(center.x, self.pixels.len()),
            to(center.y, self.pixels[0].len()),
        );

        let inv_radius = 1.0 / radius;

        for y in from_y..=to_y {
            for x in from_x..=to_x {
                let (xf, yf) = (x as f32, y as f32);
                if (Vector2::new(xf, yf) - center).length_sqr() > radius * radius {
                    continue;
                }
                let normal = Normal(Vector2::new(xf - center.x, yf - center.y) * inv_radius);
                self.draw_pixel(x, y, Pixel { color, normal }, translucency);
            }
        }
    }
    pub fn draw_sprite(&mut self, ox: usize, oy: usize, sprite: &Sprite, translucency: f32) {
        for (x, y, pixel) in &sprite.pixels {
            self.draw_pixel(x + ox, y + oy, *pixel, translucency);
        }
    }
    pub fn width(&self) -> i32 {
        self.pixels.len() as _
    }
    pub fn height(&self) -> i32 {
        self.pixels[0].len() as _
    }
    pub fn render_to(&self, d: &mut RaylibDrawHandle) {
        for x in 0..self.width() {
            for y in 0..self.height() {
                // todo probably needs other light calculation because not smort enough
                let sun = self.sun.to_vec3();

                let light = sun
                    .dot(self.pixels[x as usize][y as usize].normal.to_vec3())
                    .max(0.0)
                    .max(0.2);

                // TODO parametrize
                let f = |c: u8| ((c as f32) * light) as u8;

                let c = self.pixels[x as usize][y as usize].color;
                let color = Color::new(f(c.r), f(c.g), f(c.b), c.a);
                d.draw_rectangle(
                    x * self.pixel_size,
                    (self.height() - y + 1) * self.pixel_size,
                    self.pixel_size,
                    self.pixel_size,
                    color,
                );
            }
        }
    }
}

impl PrettyRender {
    /// Creates a new renderer
    /// Expensive shading computations
    pub fn new(tree: Tree) -> Self {
        Self { tree }
    }
}

impl PrettyRender {
    pub fn render(&self, d: &mut RaylibDrawHandle) {
        let tree = &self.tree;
        let mut canvas = Canvas::new(
            tree.config.width as usize / tree.config.pixel_size + 10,
            tree.config.height as usize / tree.config.pixel_size + 10,
            Normal(Vector2::new(-2.0, 1.0).normalized() * 0.7),
            tree.config.pixel_size as _,
        );
        let mut leaf_canvas_front = canvas.clone();
        let mut leaf_canvas_back = canvas.clone();
        let scaling = 1.0 / tree.config.pixel_size as f32;

        let mut rng = ChaCha12Rng::seed_from_u64(0);

        for node in tree.nodes.iter() {
            let pos = node.pos;
            let need_leaf_drawing = tree.radius_of(node) < tree.config.leaf_max_width && node.alive;
            // rendering a leaf

            let mut offset = || (rng.gen::<f32>() * 2.0 - 1.0) * tree.config.leaf_size;
            let mut offset = || Vector2::new(offset(), offset());

            let mut draw_leaf = |canvas: &mut Canvas, radius: f32| {
                let o = offset();
                if need_leaf_drawing {
                    // only check aliveness here to make the same number of calls to rng to have it consistent even when branches die
                    canvas.draw_sphere((pos + o) * scaling, radius, Color::MAROON, 0.6);
                }
            };

            for _ in 0..2 {
                draw_leaf(&mut leaf_canvas_front, 2.0);
                draw_leaf(&mut leaf_canvas_back, 3.0);
            }

            if !need_leaf_drawing && node.alive {
                // rendering a branch
                let parent_pos = if let Some(parent_idx) = node.parent {
                    tree.nodes[parent_idx].pos
                } else {
                    pos - Vector2::new(0.0, tree.config.grow_dist)
                };
                for i in 0..10 {
                    let interp_pos = pos.lerp(parent_pos, i as f32 * 0.1);
                    canvas.draw_sphere(
                        interp_pos * scaling,
                        tree.radius_of(node) * scaling,
                        Color::from_hex("8b6354").unwrap(),
                        0.01,
                    );
                }
            }
        }
        leaf_canvas_back.render_to(d);
        canvas.render_to(d);
        leaf_canvas_front.render_to(d);
    }
}
