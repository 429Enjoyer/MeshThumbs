use glam::{Vec2, Vec3};

use crate::{Scene, Triangle};

const THUMBNAIL_ZOOM: f32 = 1.0;
const FINAL_VISIBLE_FILL: f32 = 0.72;

pub struct RgbaBitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub(crate) fn render(scene: &Scene, size: u32) -> RgbaBitmap {
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let mut depth = vec![f32::INFINITY; (size * size) as usize];

    let view = View::surround(scene, size);
    let light = (-view.forward + Vec3::Y * 0.35 - view.right * 0.2).normalize();

    for tri in &scene.triangles {
        let mut screen = [Vec3::ZERO; 3];
        let mut normals = [Vec3::Z; 3];
        let mut uvs = [Vec2::ZERO; 3];

        for i in 0..3 {
            let p = view.project(tri.vertices[i].position);
            screen[i] = Vec3::new(
                size as f32 * 0.5 + (p.x - view.offset.x) * view.scale,
                size as f32 * 0.5 - (p.y - view.offset.y) * view.scale,
                p.z,
            );
            normals[i] = tri.vertices[i].normal.normalize_or_zero();
            uvs[i] = tri.vertices[i].uv;
        }

        draw_triangle(
            &mut pixels,
            &mut depth,
            size,
            screen,
            normals,
            uvs,
            tri,
            light,
        );
    }

    crop_visible_to_fit(&mut pixels, size);

    RgbaBitmap {
        width: size,
        height: size,
        pixels,
    }
}

struct View {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    focal: f32,
    offset: Vec2,
    scale: f32,
}

impl View {
    fn surround(scene: &Scene, size: u32) -> Self {
        let (center, extent) = bounds(scene);
        let radius = ((extent.x + extent.z) * 0.5).max(extent.y).max(0.0001);
        let eye = center + Vec3::new(2.5, 1.7, 2.5) * radius;
        let forward = (center - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let focal = 1.0 / (42.0_f32.to_radians() * 0.5).tan();

        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for tri in &scene.triangles {
            for vertex in &tri.vertices {
                let projected = project(vertex.position, eye, right, up, forward, focal).truncate();
                min = min.min(projected);
                max = max.max(projected);
            }
        }

        let projected_extent = max - min;
        let longest_side = projected_extent.max_element().max(0.0001);
        let usable = size as f32 * 0.92;
        let scale = usable / longest_side * THUMBNAIL_ZOOM;
        let offset = (min + max) * 0.5;

        Self {
            eye,
            right,
            up,
            forward,
            focal,
            offset,
            scale,
        }
    }

    fn project(&self, position: Vec3) -> Vec3 {
        project(
            position,
            self.eye,
            self.right,
            self.up,
            self.forward,
            self.focal,
        )
    }
}

fn project(position: Vec3, eye: Vec3, right: Vec3, up: Vec3, forward: Vec3, focal: f32) -> Vec3 {
    let local = position - eye;
    let z = local.dot(forward).max(0.0001);
    Vec3::new(local.dot(right) * focal / z, local.dot(up) * focal / z, z)
}

fn bounds(scene: &Scene) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for tri in &scene.triangles {
        for vertex in &tri.vertices {
            min = min.min(vertex.position);
            max = max.max(vertex.position);
        }
    }

    let center = (min + max) * 0.5;
    let extent = max - min;
    (center, extent)
}

fn draw_triangle(
    pixels: &mut [u8],
    depth: &mut [f32],
    size: u32,
    p: [Vec3; 3],
    n: [Vec3; 3],
    uv: [Vec2; 3],
    tri: &Triangle,
    light: Vec3,
) {
    let min_x = p
        .iter()
        .map(|v| v.x)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as i32;
    let max_x = p
        .iter()
        .map(|v| v.x)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(size as f32 - 1.0) as i32;
    let min_y = p
        .iter()
        .map(|v| v.y)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as i32;
    let max_y = p
        .iter()
        .map(|v| v.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(size as f32 - 1.0) as i32;

    let area = edge(p[0], p[1], p[2]);
    if area.abs() < 0.00001 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let sample = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);
            let w0 = edge(p[1], p[2], sample) / area;
            let w1 = edge(p[2], p[0], sample) / area;
            let w2 = edge(p[0], p[1], sample) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }

            let z = p[0].z * w0 + p[1].z * w1 + p[2].z * w2;
            let di = (y as u32 * size + x as u32) as usize;
            if z >= depth[di] {
                continue;
            }
            depth[di] = z;

            let inv_z0 = 1.0 / p[0].z.max(0.0001);
            let inv_z1 = 1.0 / p[1].z.max(0.0001);
            let inv_z2 = 1.0 / p[2].z.max(0.0001);
            let inv_z = inv_z0 * w0 + inv_z1 * w1 + inv_z2 * w2;
            let tex_uv = if inv_z > 0.0 {
                (uv[0] * inv_z0 * w0 + uv[1] * inv_z1 * w1 + uv[2] * inv_z2 * w2) / inv_z
            } else {
                uv[0] * w0 + uv[1] * w1 + uv[2] * w2
            };
            let mut color = tri
                .texture
                .as_ref()
                .map(|t| modulate(t.sample(tex_uv), tri.color))
                .unwrap_or(tri.color);
            let weights = if inv_z > 0.0 {
                [
                    w0 * inv_z0 / inv_z,
                    w1 * inv_z1 / inv_z,
                    w2 * inv_z2 / inv_z,
                ]
            } else {
                [w0, w1, w2]
            };
            let vertex_color = std::array::from_fn(|channel| {
                (0..3)
                    .map(|i| tri.vertices[i].color[channel] as f32 * weights[i])
                    .sum::<f32>()
                    .round()
                    .clamp(0.0, 255.0) as u8
            });
            color = modulate(color, vertex_color);

            let normal = (n[0] * w0 + n[1] * w1 + n[2] * w2).normalize_or_zero();
            let shade = 0.68 + normal.dot(light).max(0.0) * 0.32;
            color[0] = (color[0] as f32 * shade).min(255.0) as u8;
            color[1] = (color[1] as f32 * shade).min(255.0) as u8;
            color[2] = (color[2] as f32 * shade).min(255.0) as u8;
            color[3] = color[3].max(230);

            let pi = di * 4;
            pixels[pi..pi + 4].copy_from_slice(&color);
        }
    }
}

fn edge(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

fn modulate(texture: [u8; 4], color: [u8; 4]) -> [u8; 4] {
    [
        ((texture[0] as u16 * color[0] as u16) / 255) as u8,
        ((texture[1] as u16 * color[1] as u16) / 255) as u8,
        ((texture[2] as u16 * color[2] as u16) / 255) as u8,
        ((texture[3] as u16 * color[3] as u16) / 255) as u8,
    ]
}

fn crop_visible_to_fit(pixels: &mut Vec<u8>, size: u32) {
    let mut min_x = size;
    let mut min_y = size;
    let mut max_x = 0;
    let mut max_y = 0;

    for y in 0..size {
        for x in 0..size {
            let alpha = pixels[((y * size + x) * 4 + 3) as usize];
            if alpha > 8 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x > max_x || min_y > max_y {
        return;
    }

    let visible_width = (max_x - min_x + 1) as f32;
    let visible_height = (max_y - min_y + 1) as f32;
    if visible_width < 1.0 || visible_height < 1.0 {
        return;
    }

    let target = size as f32 * FINAL_VISIBLE_FILL;
    let scale = (target / visible_width).min(target / visible_height);
    if scale <= 1.02 {
        return;
    }

    let source = pixels.clone();
    pixels.fill(0);

    let source_center = Vec2::new(
        min_x as f32 + visible_width * 0.5,
        min_y as f32 + visible_height * 0.5,
    );
    let target_center = Vec2::splat(size as f32 * 0.5);

    for y in 0..size {
        for x in 0..size {
            let dst = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let src = (dst - target_center) / scale + source_center;
            if src.x < 0.0 || src.y < 0.0 || src.x >= size as f32 || src.y >= size as f32 {
                continue;
            }

            let color = sample_bitmap(&source, size, src);
            let di = ((y * size + x) * 4) as usize;
            pixels[di..di + 4].copy_from_slice(&color);
        }
    }
}

fn sample_bitmap(pixels: &[u8], size: u32, p: Vec2) -> [u8; 4] {
    let x = p.x.clamp(0.0, size.saturating_sub(1) as f32);
    let y = p.y.clamp(0.0, size.saturating_sub(1) as f32);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(size.saturating_sub(1));
    let y1 = (y0 + 1).min(size.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let c00 = bitmap_pixel(pixels, size, x0, y0);
    let c10 = bitmap_pixel(pixels, size, x1, y0);
    let c01 = bitmap_pixel(pixels, size, x0, y1);
    let c11 = bitmap_pixel(pixels, size, x1, y1);
    let mut out = [0u8; 4];
    for i in 0..4 {
        let top = c00[i] as f32 * (1.0 - tx) + c10[i] as f32 * tx;
        let bottom = c01[i] as f32 * (1.0 - tx) + c11[i] as f32 * tx;
        out[i] = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn bitmap_pixel(pixels: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * size + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}
