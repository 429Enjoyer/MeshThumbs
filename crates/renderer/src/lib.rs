mod blend;
mod loaders;
mod raster;

use std::path::Path;

pub use raster::RgbaBitmap;

pub const MAX_MODEL_BYTES: u64 = 300 * 1024 * 1024;
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "obj", "fbx", "glb", "gltf", "stl", "dae", "ply", "3ds", "3mf", "vrm", "blend", "x3d", "off",
    "usd", "usda", "usdc", "usdz", "wrl", "vrml", "step", "stp", "abc", "igs", "iges", "3dm",
    "ifc", "pmx", "vox", "lwo", "smd", "md2", "md3", "md5mesh",
];

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub size: u32,
    /// Reject larger meshes instead of dropping faces and opening holes.
    pub max_triangles: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            size: 256,
            max_triangles: 5_000_000,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("unsupported model format")]
    UnsupportedFormat,
    #[error("model has no renderable triangles")]
    EmptyModel,
    #[error("model has {actual} triangles, exceeding the {limit} triangle limit")]
    TooManyTriangles { actual: usize, limit: usize },
    #[error(transparent)]
    Load(#[from] anyhow::Error),
}

pub fn render_thumbnail(
    path: impl AsRef<Path>,
    options: &RenderOptions,
) -> Result<RgbaBitmap, RenderError> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if !SUPPORTED_EXTENSIONS
        .iter()
        .any(|e| extension.eq_ignore_ascii_case(e))
    {
        return Err(RenderError::UnsupportedFormat);
    }
    if std::fs::metadata(path).map_err(anyhow::Error::from)?.len() > MAX_MODEL_BYTES {
        return Err(RenderError::Load(anyhow::anyhow!(
            "model exceeds the 300 MiB limit"
        )));
    }
    if extension.eq_ignore_ascii_case("blend") {
        return blend::render(path, options.size.clamp(32, 1024)).map_err(RenderError::Load);
    }
    if options.max_triangles == 0 {
        return Err(RenderError::EmptyModel);
    }
    let mut scene = loaders::load_scene(path, options.max_triangles)?;

    scene.prepare(options.max_triangles)?;
    Ok(raster::render(&scene, options.size.clamp(32, 1024)))
}

#[derive(Clone)]
pub(crate) struct Scene {
    triangles: Vec<Triangle>,
}

impl Scene {
    fn new() -> Self {
        Self {
            triangles: Vec::new(),
        }
    }

    fn prepare(&mut self, budget: usize) -> Result<(), RenderError> {
        if self.triangles.len() > budget {
            return Err(RenderError::TooManyTriangles {
                actual: self.triangles.len(),
                limit: budget,
            });
        }
        self.triangles
            .retain(|t| t.vertices.iter().all(|v| v.position.is_finite()));
        if self.triangles.is_empty() {
            return Err(RenderError::EmptyModel);
        }
        // Frame tiny, huge, and far-from-origin models without f32 overflow.
        let mut min = glam::DVec3::splat(f64::INFINITY);
        let mut max = glam::DVec3::splat(f64::NEG_INFINITY);
        for v in self.triangles.iter().flat_map(|t| &t.vertices) {
            min = min.min(v.position.as_dvec3());
            max = max.max(v.position.as_dvec3());
        }
        let extent = (max - min).max_element();
        if extent <= 0.0 {
            return Err(RenderError::EmptyModel);
        }
        let center = (min + max) * 0.5;
        for t in &mut self.triangles {
            for v in &mut t.vertices {
                v.position = ((v.position.as_dvec3() - center) / extent).as_vec3();
                if !v.uv.is_finite() {
                    v.uv = glam::Vec2::ZERO;
                }
            }
            t.vertices = loaders::fix_normals(t.vertices);
        }
        self.triangles.retain(|t| {
            (t.vertices[1].position - t.vertices[0].position)
                .cross(t.vertices[2].position - t.vertices[0].position)
                .length_squared()
                > 0.0
        });
        if self.triangles.is_empty() {
            return Err(RenderError::EmptyModel);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct Triangle {
    vertices: [Vertex; 3],
    color: [u8; 4],
    texture: Option<std::sync::Arc<Texture>>,
}

#[derive(Clone, Copy)]
pub(crate) struct Vertex {
    position: glam::Vec3,
    normal: glam::Vec3,
    uv: glam::Vec2,
    color: [u8; 4],
}

#[derive(Clone)]
pub(crate) struct Texture {
    width: u32,
    height: u32,
    pixels: std::sync::Arc<[u8]>,
    wrap_s: WrapMode,
    wrap_t: WrapMode,
    flip_v: bool,
}

impl Texture {
    fn from_image(image: image::DynamicImage) -> Self {
        Self::from_image_with_orientation(image, true)
    }

    pub(crate) fn from_gltf_image(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let image = image::RgbaImage::from_raw(width, height, pixels)?;
        Some(Self::from_image_with_orientation(image.into(), false))
    }

    fn from_image_with_orientation(image: image::DynamicImage, flip_v: bool) -> Self {
        let image = if image.width() > 1024 || image.height() > 1024 {
            image.thumbnail(1024, 1024)
        } else {
            image
        };
        let rgba = image.into_rgba8();
        Self {
            width: rgba.width(),
            height: rgba.height(),
            pixels: rgba.into_raw().into(),
            wrap_s: WrapMode::Repeat,
            wrap_t: WrapMode::Repeat,
            flip_v,
        }
    }

    pub(crate) fn with_wrap(mut self, wrap_s: WrapMode, wrap_t: WrapMode) -> Self {
        self.wrap_s = wrap_s;
        self.wrap_t = wrap_t;
        self
    }

    fn sample(&self, uv: glam::Vec2) -> [u8; 4] {
        let u = self.wrap_s.apply(uv.x);
        let raw_v = if self.flip_v { 1.0 - uv.y } else { uv.y };
        let v = self.wrap_t.apply(raw_v);
        let x = u * self.width.saturating_sub(1) as f32;
        let y = v * self.height.saturating_sub(1) as f32;
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width.saturating_sub(1));
        let y1 = (y0 + 1).min(self.height.saturating_sub(1));
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let c00 = self.pixel(x0, y0);
        let c10 = self.pixel(x1, y0);
        let c01 = self.pixel(x0, y1);
        let c11 = self.pixel(x1, y1);
        let mut out = [0u8; 4];
        for i in 0..4 {
            let top = c00[i] as f32 * (1.0 - tx) + c10[i] as f32 * tx;
            let bottom = c01[i] as f32 * (1.0 - tx) + c11[i] as f32 * tx;
            out[i] = (top * (1.0 - ty) + bottom * ty).round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }
}

#[derive(Clone, Copy)]
pub(crate) enum WrapMode {
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}

impl WrapMode {
    fn apply(self, value: f32) -> f32 {
        match self {
            WrapMode::ClampToEdge => value.clamp(0.0, 1.0),
            WrapMode::MirroredRepeat => {
                let wrapped = value.rem_euclid(2.0);
                if wrapped <= 1.0 {
                    wrapped
                } else {
                    2.0 - wrapped
                }
            }
            WrapMode::Repeat => value.rem_euclid(1.0),
        }
    }
}
