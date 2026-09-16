//! Bounded ASCII DXF surfaces and local block references. Transforms follow
//! Autodesk's INSERT/OCS definitions; coordinates stay f64 until recentering.
use crate::{Scene, Triangle, Vertex, MAX_MODEL_BYTES};
use anyhow::{bail, ensure, Context};
use glam::{DMat3, DMat4, DVec3, Vec2, Vec3};
use std::{collections::HashMap, io::Read, path::Path};

const MAX_RECORDS: usize = 1_000_000;
const MAX_INSTANCES: usize = 100_000;
const DEFAULT_COLOR: [u8; 4] = [255; 4];

#[derive(Clone, Copy, Debug)]
enum Color {
    Layer,
    Block,
    Rgb([u8; 4]),
}
#[derive(Clone, Debug)]
struct Style {
    layer: Vec<u8>,
    color: Color,
    visible: bool,
}
#[derive(Clone, Copy)]
struct Layer {
    color: [u8; 4],
    visible: bool,
}
struct MeshVertex {
    point: DVec3,
    color: Option<Color>,
}
struct Face {
    indices: Vec<usize>,
    color: Option<Color>,
    visible: bool,
}
struct Insert {
    name: Vec<u8>,
    position: DVec3,
    scale: DVec3,
    basis: DMat4,
    angle: f64,
    columns: usize,
    rows: usize,
    spacing: [f64; 2],
}
enum Geometry {
    Face(Vec<DVec3>),
    Mesh(Vec<MeshVertex>, Vec<Face>),
    Insert(Insert),
    Unsupported(Vec<u8>),
}
struct Entity {
    style: Style,
    geometry: Geometry,
}
struct Block {
    base: DVec3,
    external: bool,
    entities: Vec<Entity>,
}
#[derive(Default)]
struct Document {
    blocks: HashMap<Vec<u8>, Block>,
    layers: HashMap<Vec<u8>, Layer>,
    entities: Vec<Entity>,
    storage: usize,
}

struct Group<'a> {
    code: i32,
    value: &'a [u8],
}
struct Record<'a> {
    kind: &'a [u8],
    fields: Vec<Group<'a>>,
}
struct Reader<'a> {
    lines: std::slice::Split<'a, u8, fn(&u8) -> bool>,
    pending: Option<Group<'a>>,
    groups: usize,
    records: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> anyhow::Result<Self> {
        ensure!(
            bytes.len() <= MAX_MODEL_BYTES as usize,
            "DXF exceeds 300 MiB"
        );
        ensure!(
            !bytes.starts_with(b"AutoCAD Binary DXF"),
            "binary DXF is unsupported"
        );
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        Ok(Self {
            lines: bytes.split(|b| *b == b'\n'),
            pending: None,
            groups: 0,
            records: 0,
        })
    }
    fn group(&mut self) -> anyhow::Result<Option<Group<'a>>> {
        loop {
            let Some(code) = self.lines.next() else {
                return Ok(None);
            };
            if code.trim_ascii().is_empty() && self.lines.clone().all(|s| s.trim_ascii().is_empty())
            {
                return Ok(None);
            }
            self.groups += 1;
            ensure!(self.groups <= 10_000_000, "too many DXF groups");
            let code = std::str::from_utf8(code.trim_ascii())?.parse::<i32>()?;
            ensure!((0..=1071).contains(&code), "invalid DXF group code");
            let value = self
                .lines
                .next()
                .context("truncated DXF group")?
                .trim_ascii();
            if code != 999 {
                return Ok(Some(Group { code, value }));
            }
        }
    }
    fn record(&mut self) -> anyhow::Result<Option<Record<'a>>> {
        let first = match self.pending.take() {
            Some(g) => Some(g),
            None => self.group()?,
        };
        let Some(first) = first else {
            return Ok(None);
        };
        ensure!(first.code == 0, "DXF record must start with group zero");
        self.records += 1;
        ensure!(self.records <= MAX_RECORDS, "too many DXF records");
        let mut fields = Vec::new();
        while let Some(group) = self.group()? {
            if group.code == 0 {
                self.pending = Some(group);
                break;
            }
            ensure!(fields.len() < 100_000, "DXF record is too large");
            fields.push(group);
        }
        Ok(Some(Record {
            kind: first.value,
            fields,
        }))
    }
}
impl Record<'_> {
    fn is(&self, kind: &[u8]) -> bool {
        self.kind.eq_ignore_ascii_case(kind)
    }
    fn get(&self, code: i32) -> Option<&[u8]> {
        self.fields.iter().find(|g| g.code == code).map(|g| g.value)
    }
    fn int(&self, code: i32, default: i32) -> anyhow::Result<i32> {
        self.get(code)
            .map(|v| Ok(std::str::from_utf8(v)?.parse()?))
            .unwrap_or(Ok(default))
    }
    fn real(&self, code: i32, default: f64) -> anyhow::Result<f64> {
        let value = self
            .get(code)
            .map(|v| Ok::<_, anyhow::Error>(std::str::from_utf8(v)?.parse::<f64>()?))
            .transpose()?
            .unwrap_or(default);
        ensure!(value.is_finite(), "non-finite DXF coordinate");
        Ok(value)
    }
    fn point(&self, code: i32, default: DVec3) -> anyhow::Result<DVec3> {
        Ok(DVec3::new(
            self.real(code, default.x)?,
            self.real(code + 10, default.y)?,
            self.real(code + 20, default.z)?,
        ))
    }
    fn color(&self) -> anyhow::Result<Option<Color>> {
        if self.get(420).is_some() {
            let rgb = self.int(420, 0)?;
            ensure!((0..=0xffffff).contains(&rgb), "invalid DXF true color");
            return Ok(Some(Color::Rgb([
                (rgb >> 16) as u8,
                (rgb >> 8) as u8,
                rgb as u8,
                255,
            ])));
        }
        if self.get(62).is_none() {
            return Ok(None);
        }
        let index = self
            .int(62, 256)?
            .checked_abs()
            .context("invalid DXF color")?;
        Ok(Some(match index {
            0 => Color::Block,
            256 => Color::Layer,
            1..=255 => Color::Rgb(aci(index as usize)),
            _ => bail!("invalid DXF color index"),
        }))
    }
    fn style(&self) -> anyhow::Result<Style> {
        Ok(Style {
            layer: name(self.get(8).unwrap_or(b"0"))?,
            color: self.color()?.unwrap_or(Color::Layer),
            visible: self.int(60, 0)? == 0 && self.int(67, 0)? == 0 && self.int(62, 256)? >= 0,
        })
    }
}
fn name(value: &[u8]) -> anyhow::Result<Vec<u8>> {
    ensure!(
        !value.is_empty() && value.len() <= 1024,
        "invalid DXF name length"
    );
    Ok(match std::str::from_utf8(value) {
        Ok(s) => s.to_uppercase().into_bytes(),
        Err(_) => value.to_ascii_uppercase(),
    })
}

// Conventional ACI hue/saturation/value ramps, independent of CAD viewport
// background and plot styles. True-color RGB takes precedence when present.
fn aci(index: usize) -> [u8; 4] {
    let rgb = match index {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 => [255; 3],
        8 => [128; 3],
        9 => [192; 3],
        250..=255 => [[51; 3], [80; 3], [105; 3], [130; 3], [190; 3], [255; 3]][index - 250],
        10..=249 => {
            let hue = (index - 10) / 10;
            let shade = (index - 10) % 10;
            let v = [255., 165., 127., 76., 38.][shade / 2];
            let low = if shade.is_multiple_of(2) { 0. } else { v * 0.5 };
            let rise = low + (v - low) * (hue % 4) as f64 / 4.;
            let fall = v - (v - low) * (hue % 4) as f64 / 4.;
            let c = match hue / 4 {
                0 => [v, rise, low],
                1 => [fall, v, low],
                2 => [low, v, rise],
                3 => [low, fall, v],
                4 => [rise, low, v],
                _ => [v, low, fall],
            };
            c.map(|c| c as u8)
        }
        _ => [255; 3],
    };
    [rgb[0], rgb[1], rgb[2], 255]
}

fn ocs(normal: DVec3) -> anyhow::Result<DMat4> {
    let maximum = normal.abs().max_element();
    ensure!(
        maximum > 0. && maximum.is_finite(),
        "invalid DXF extrusion direction"
    );
    let z = (normal / maximum).normalize();
    let reference = if z.x.abs() < 1. / 64. && z.y.abs() < 1. / 64. {
        DVec3::Y
    } else {
        DVec3::Z
    };
    let x = reference.cross(z).normalize();
    Ok(DMat4::from_mat3(DMat3::from_cols(
        x,
        z.cross(x).normalize(),
        z,
    )))
}

impl Document {
    fn charge(&mut self, bytes: usize) -> anyhow::Result<()> {
        self.storage = self
            .storage
            .checked_add(bytes)
            .context("DXF storage overflow")?;
        ensure!(
            self.storage <= MAX_MODEL_BYTES as usize,
            "DXF geometry storage exceeds 300 MiB"
        );
        Ok(())
    }
    fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut reader = Reader::new(bytes)?;
        let mut doc = Self::default();
        let mut section: Option<Vec<u8>> = None;
        let mut block: Option<(Vec<u8>, Block)> = None;
        let mut eof = false;
        while let Some(record) = reader.record()? {
            if record.is(b"SECTION") {
                ensure!(section.is_none(), "nested DXF section");
                section = Some(name(record.get(2).context("unnamed DXF section")?)?);
                continue;
            }
            if record.is(b"ENDSEC") {
                ensure!(
                    section.take().is_some() && block.is_none(),
                    "unclosed DXF block/section"
                );
                continue;
            }
            if record.is(b"EOF") {
                ensure!(section.is_none() && block.is_none(), "unclosed DXF section");
                eof = true;
                break;
            }
            let section = section.as_deref().context("DXF record outside section")?;
            if section == b"TABLES" && record.is(b"LAYER") {
                let key = name(record.get(2).context("unnamed DXF layer")?)?;
                ensure!(doc.layers.len() < 10_000, "too many DXF layers");
                let color = match record.color()?.unwrap_or(Color::Rgb(DEFAULT_COLOR)) {
                    Color::Rgb(c) => c,
                    _ => DEFAULT_COLOR,
                };
                let visible = record.int(62, 7)? >= 0 && record.int(70, 0)? & 1 == 0;
                ensure!(
                    doc.layers.insert(key, Layer { color, visible }).is_none(),
                    "duplicate DXF layer"
                );
                continue;
            }
            if section == b"BLOCKS" && record.is(b"BLOCK") {
                ensure!(
                    block.is_none() && doc.blocks.len() < 10_000,
                    "nested/excessive DXF blocks"
                );
                let key = name(
                    record
                        .get(2)
                        .or_else(|| record.get(3))
                        .context("unnamed DXF block")?,
                )?;
                block = Some((
                    key,
                    Block {
                        base: record.point(10, DVec3::ZERO)?,
                        external: record.int(70, 0)? & (4 | 8 | 16) != 0,
                        entities: Vec::new(),
                    },
                ));
                continue;
            }
            if section == b"BLOCKS" && record.is(b"ENDBLK") {
                let (key, value) = block.take().context("DXF ENDBLK without BLOCK")?;
                ensure!(
                    doc.blocks.insert(key, value).is_none(),
                    "duplicate DXF block"
                );
                continue;
            }
            if section == b"ENTITIES" || (section == b"BLOCKS" && block.is_some()) {
                if let Some(entity) = doc.entity(record, &mut reader)? {
                    doc.charge(std::mem::size_of::<Entity>() + entity.style.layer.len())?;
                    if let Some((_, block)) = block.as_mut() {
                        block.entities.push(entity);
                    } else {
                        doc.entities.push(entity);
                    }
                }
            }
        }
        ensure!(eof, "DXF EOF record is missing");
        Ok(doc)
    }

    fn entity(
        &mut self,
        record: Record<'_>,
        reader: &mut Reader<'_>,
    ) -> anyhow::Result<Option<Entity>> {
        let kind = record.kind.to_ascii_uppercase();
        let geometry = match kind.as_slice() {
            b"3DFACE" => {
                for code in [10, 20, 11, 21, 12, 22] {
                    ensure!(record.get(code).is_some(), "incomplete DXF 3DFACE");
                }
                let mut points = vec![
                    record.point(10, DVec3::ZERO)?,
                    record.point(11, DVec3::ZERO)?,
                    record.point(12, DVec3::ZERO)?,
                ];
                let fourth = record.point(13, points[2])?;
                if fourth != points[2] {
                    points.push(fourth);
                }
                self.charge(points.len() * std::mem::size_of::<DVec3>())?;
                Geometry::Face(points)
            }
            b"POLYLINE" => {
                let polyface = record.int(70, 0)? & 64 != 0;
                let mut vertices = Vec::new();
                let mut faces = Vec::new();
                let mut ended = false;
                while let Some(vertex) = reader.record()? {
                    if vertex.is(b"SEQEND") {
                        ended = true;
                        break;
                    }
                    ensure!(vertex.is(b"VERTEX"), "unterminated DXF POLYLINE");
                    if !polyface {
                        continue;
                    }
                    if vertex.get(71).is_some() {
                        let mut indices = Vec::new();
                        for code in 71..=74 {
                            let index = vertex
                                .int(code, 0)?
                                .checked_abs()
                                .context("invalid DXF face index")?;
                            if index != 0 {
                                indices.push(index as usize - 1);
                            }
                        }
                        ensure!(
                            indices.len() >= 3,
                            "DXF polyface has fewer than three indices"
                        );
                        self.charge(
                            std::mem::size_of::<Face>()
                                + indices.len() * std::mem::size_of::<usize>(),
                        )?;
                        faces.push(Face {
                            indices,
                            color: vertex.color()?,
                            visible: vertex.int(60, 0)? == 0 && vertex.int(62, 256)? >= 0,
                        });
                    } else {
                        self.charge(std::mem::size_of::<MeshVertex>())?;
                        vertices.push(MeshVertex {
                            point: vertex.point(10, DVec3::ZERO)?,
                            color: vertex.color()?,
                        });
                    }
                }
                ensure!(ended, "unterminated DXF POLYLINE");
                if !polyface {
                    return Ok(None);
                }
                for face in &faces {
                    ensure!(
                        face.indices.iter().all(|&i| i < vertices.len()),
                        "DXF polyface index outside vertex list"
                    );
                }
                Geometry::Mesh(vertices, faces)
            }
            b"INSERT" => {
                let name = name(record.get(2).context("unnamed DXF INSERT")?)?;
                let columns = record.int(70, 1)?;
                let rows = record.int(71, 1)?;
                ensure!(
                    columns > 0
                        && rows > 0
                        && columns as usize <= MAX_INSTANCES
                        && rows as usize <= MAX_INSTANCES,
                    "invalid DXF array dimensions"
                );
                let scale = DVec3::new(
                    record.real(41, 1.)?,
                    record.real(42, 1.)?,
                    record.real(43, 1.)?,
                );
                ensure!(
                    scale.x != 0. && scale.y != 0. && scale.z != 0.,
                    "zero DXF INSERT scale"
                );
                self.charge(name.len())?;
                Geometry::Insert(Insert {
                    name,
                    position: record.point(10, DVec3::ZERO)?,
                    scale,
                    basis: ocs(record.point(210, DVec3::Z)?)?,
                    angle: record.real(50, 0.)?.to_radians(),
                    columns: columns as usize,
                    rows: rows as usize,
                    spacing: [record.real(44, 0.)?, record.real(45, 0.)?],
                })
            }
            b"3DSOLID" | b"BODY" | b"REGION" | b"SURFACE" | b"MESH" | b"SOLID" | b"HATCH" => {
                Geometry::Unsupported(kind)
            }
            // Curves, text, attributes, and other non-surface records do not
            // acquire invented faces. A drawing containing only these is empty.
            _ => return Ok(None),
        };
        Ok(Some(Entity {
            style: record.style()?,
            geometry,
        }))
    }

    fn resolved_style<'a>(
        &self,
        style: &'a Style,
        parent_layer: &'a [u8],
        parent_color: [u8; 4],
    ) -> Option<(&'a [u8], [u8; 4])> {
        let layer = if style.layer == b"0" {
            parent_layer
        } else {
            &style.layer
        };
        let info = self.layers.get(layer).copied().unwrap_or(Layer {
            color: DEFAULT_COLOR,
            visible: true,
        });
        if !style.visible || !info.visible {
            return None;
        }
        Some((layer, color(style.color, info.color, parent_color)))
    }

    fn scene(&self, budget: usize) -> anyhow::Result<Scene> {
        let mut output = Output {
            scene: Scene::new(),
            origin: None,
            budget,
            visits: 0,
            instances: 0,
        };
        self.expand(
            &self.entities,
            DMat4::IDENTITY,
            b"0",
            DEFAULT_COLOR,
            &mut Vec::new(),
            &mut output,
        )?;
        Ok(output.scene)
    }

    fn expand(
        &self,
        entities: &[Entity],
        transform: DMat4,
        parent_layer: &[u8],
        parent_color: [u8; 4],
        stack: &mut Vec<Vec<u8>>,
        out: &mut Output,
    ) -> anyhow::Result<()> {
        for entity in entities {
            out.visits += 1;
            ensure!(
                out.visits <= MAX_RECORDS,
                "DXF expansion exceeds one million entities"
            );
            let Some((layer, base_color)) =
                self.resolved_style(&entity.style, parent_layer, parent_color)
            else {
                continue;
            };
            let layer_color = self
                .layers
                .get(layer)
                .map(|l| l.color)
                .unwrap_or(DEFAULT_COLOR);
            match &entity.geometry {
                Geometry::Face(points) => {
                    out.face(points, &vec![base_color; points.len()], transform)?
                }
                Geometry::Mesh(vertices, faces) => {
                    // Count even degenerate/hidden faces: otherwise a small
                    // repeated polyface could consume unbounded work while
                    // never advancing the rendered-triangle counter.
                    out.visits += faces.len();
                    ensure!(
                        out.visits <= MAX_RECORDS,
                        "DXF expansion exceeds one million entity/face visits"
                    );
                    for face in faces {
                        if !face.visible {
                            continue;
                        }
                        let face_color = face
                            .color
                            .map(|c| color(c, layer_color, parent_color))
                            .unwrap_or(base_color);
                        let points = face
                            .indices
                            .iter()
                            .map(|&i| vertices[i].point)
                            .collect::<Vec<_>>();
                        let colors = face
                            .indices
                            .iter()
                            .map(|&i| {
                                vertices[i]
                                    .color
                                    .map(|c| color(c, layer_color, parent_color))
                                    .unwrap_or(face_color)
                            })
                            .collect::<Vec<_>>();
                        out.face(&points, &colors, transform)?;
                    }
                }
                Geometry::Insert(insert) => {
                    ensure!(
                        stack.len() < 64 && !stack.contains(&insert.name),
                        "cyclic or excessively deep DXF block reference"
                    );
                    let block = self.blocks.get(&insert.name).with_context(|| {
                        format!(
                            "missing DXF block {}",
                            String::from_utf8_lossy(&insert.name)
                        )
                    })?;
                    ensure!(
                        !block.external,
                        "external DXF block references are unsupported"
                    );
                    let columns = if insert.spacing[0] == 0. {
                        1
                    } else {
                        insert.columns
                    };
                    let rows = if insert.spacing[1] == 0. {
                        1
                    } else {
                        insert.rows
                    };
                    let count = rows
                        .checked_mul(columns)
                        .context("DXF array size overflow")?;
                    out.instances = out
                        .instances
                        .checked_add(count)
                        .context("DXF instance overflow")?;
                    ensure!(
                        out.instances <= MAX_INSTANCES,
                        "DXF exceeds 100,000 block instances"
                    );
                    stack.push(insert.name.clone());
                    let rotation = DMat4::from_rotation_z(insert.angle);
                    for row in 0..rows {
                        for column in 0..columns {
                            let grid = DVec3::new(
                                column as f64 * insert.spacing[0],
                                row as f64 * insert.spacing[1],
                                0.,
                            );
                            // Array spacing rotates with the INSERT, but is not scaled.
                            let local = insert.basis
                                * DMat4::from_translation(insert.position)
                                * rotation
                                * DMat4::from_translation(grid)
                                * DMat4::from_scale(insert.scale)
                                * DMat4::from_translation(-block.base);
                            let matrix = transform * local;
                            ensure!(matrix.is_finite(), "DXF transform overflow");
                            self.expand(&block.entities, matrix, layer, base_color, stack, out)?;
                        }
                    }
                    stack.pop();
                }
                Geometry::Unsupported(kind) => bail!(
                    "unsupported DXF surface entity: {}",
                    String::from_utf8_lossy(kind)
                ),
            }
        }
        Ok(())
    }
}

fn color(color: Color, layer: [u8; 4], block: [u8; 4]) -> [u8; 4] {
    match color {
        Color::Layer => layer,
        Color::Block => block,
        Color::Rgb(c) => c,
    }
}
struct Output {
    scene: Scene,
    origin: Option<DVec3>,
    budget: usize,
    visits: usize,
    instances: usize,
}
impl Output {
    fn face(
        &mut self,
        points: &[DVec3],
        colors: &[[u8; 4]],
        transform: DMat4,
    ) -> anyhow::Result<()> {
        let mut ids = Vec::new();
        for (i, p) in points.iter().enumerate() {
            if ids.last().is_none_or(|&j| points[j] != *p) {
                ids.push(i);
            }
        }
        if ids.len() > 1 && points[ids[0]] == points[*ids.last().unwrap()] {
            ids.pop();
        }
        if ids.len() < 3 {
            return Ok(());
        }
        let local_origin = points[ids[0]];
        let extent = points
            .iter()
            .map(|p| (*p - local_origin).abs().max_element())
            .fold(0., f64::max);
        ensure!(extent.is_finite(), "DXF coordinate range overflow");
        if extent == 0. {
            return Ok(());
        }
        let normalized = points
            .iter()
            .map(|p| ((*p - local_origin) / extent).as_vec3())
            .collect::<Vec<_>>();
        let triangles = super::usd::triangulate(&ids, &normalized)?;
        ensure!(
            self.scene.triangles.len() + triangles.len() <= self.budget,
            "DXF exceeds the triangle limit"
        );
        let reflected = transform.determinant() < 0.;
        for mut triangle in triangles {
            if reflected {
                triangle.swap(1, 2);
            }
            let mut vertices = [Vertex {
                position: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                color: DEFAULT_COLOR,
            }; 3];
            for (vertex, corner) in vertices.iter_mut().zip(triangle) {
                let i = ids[corner];
                let point = transform.transform_point3(points[i]);
                ensure!(point.is_finite(), "DXF position overflow");
                let origin = *self.origin.get_or_insert(point);
                let p = point - origin;
                vertex.position = Vec3::new(p.x as f32, p.z as f32, -p.y as f32);
                ensure!(
                    vertex.position.is_finite(),
                    "DXF model range exceeds renderer precision"
                );
                vertex.color = colors[i];
            }
            self.scene.triangles.push(Triangle {
                vertices: super::fix_normals(vertices),
                color: DEFAULT_COLOR,
                texture: None,
            });
        }
        Ok(())
    }
}

pub(super) fn load(path: &Path, budget: usize) -> anyhow::Result<Scene> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_MODEL_BYTES + 1)
        .read_to_end(&mut bytes)?;
    Document::parse(&bytes)?.scene(budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn face(points: [[f64; 3]; 3], properties: &str) -> String {
        let mut text = format!("0\n3DFACE\n{properties}");
        for (i, p) in points.iter().enumerate() {
            for (j, v) in p.iter().enumerate() {
                text.push_str(&format!("{}\n{v}\n", 10 + i + 10 * j));
            }
        }
        text
    }
    fn triangle(properties: &str) -> String {
        face([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], properties)
    }
    fn block(name: &str, base: [f64; 3], contents: &str) -> String {
        format!(
            "0\nBLOCK\n2\n{name}\n10\n{}\n20\n{}\n30\n{}\n{contents}0\nENDBLK\n",
            base[0], base[1], base[2]
        )
    }
    fn insert(name: &str, properties: &str) -> String {
        format!("0\nINSERT\n2\n{name}\n{properties}")
    }
    fn document(blocks: &str, entities: &str, layers: &str) -> Document {
        let data = format!("0\nSECTION\n2\nTABLES\n{layers}0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n{blocks}0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n");
        Document::parse(data.as_bytes()).unwrap()
    }
    fn assert_point(actual: Vec3, expected: [f32; 3]) {
        assert!(
            (actual - Vec3::from(expected)).length() < 0.0001,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn block_base_rotation_translation_and_nonuniform_scale() {
        let mesh = face([[2., 2., 0.], [1., 3., 0.], [1., 2., 0.]], "");
        let blocks = block("Base", [1., 2., 0.], &mesh);
        let entities = triangle("") + &insert("base", "10\n10\n20\n20\n41\n2\n42\n3\n50\n90\n");
        let scene = document(&blocks, &entities, "").scene(10).unwrap();
        assert_eq!(scene.triangles.len(), 2);
        assert_point(scene.triangles[1].vertices[0].position, [10., 0., -22.]);
        assert_point(scene.triangles[1].vertices[1].position, [7., 0., -20.]);
    }

    #[test]
    fn nested_insert_transforms_compose_in_order() {
        let mesh = face([[1., 0., 0.], [0., 1., 0.], [0., 0., 0.]], "");
        let blocks = block("inner", [0.; 3], &mesh)
            + &block(
                "outer",
                [0.; 3],
                &insert("inner", "10\n3\n20\n4\n42\n2\n50\n90\n"),
            );
        let entities = triangle("") + &insert("outer", "10\n10\n20\n20\n41\n2\n42\n3\n50\n90\n");
        let scene = document(&blocks, &entities, "").scene(10).unwrap();
        assert_point(scene.triangles[1].vertices[0].position, [-5., 0., -26.]);
    }

    #[test]
    fn mirrored_geometry_keeps_outward_normals() {
        let blocks = block("shape", [0.; 3], &triangle(""));
        let scene = document(&blocks, &insert("shape", "41\n-2\n42\n3\n"), "")
            .scene(10)
            .unwrap();
        assert!(scene.triangles[0]
            .vertices
            .iter()
            .all(|v| v.normal.y > 0.99));
    }

    #[test]
    fn extrusion_uses_autodesk_arbitrary_axes() {
        let blocks = block("shape", [0.; 3], &triangle(""));
        let entities =
            triangle("") + &insert("shape", "10\n2\n20\n3\n30\n4\n210\n0\n220\n1\n230\n0\n");
        let scene = document(&blocks, &entities, "").scene(10).unwrap();
        assert_point(scene.triangles[1].vertices[0].position, [-2., 3., -4.]);
        assert!(scene.triangles[1].vertices[0].normal.z < -0.99);
    }

    #[test]
    fn arrays_rotate_spacing_without_scaling_it() {
        let blocks = block("shape", [0.; 3], &triangle(""));
        let entities = insert(
            "shape",
            "41\n2\n42\n3\n50\n90\n70\n3\n71\n2\n44\n5\n45\n4\n",
        );
        let doc = document(&blocks, &entities, "");
        let scene = doc.scene(6).unwrap();
        assert_eq!(scene.triangles.len(), 6);
        assert_point(scene.triangles[1].vertices[0].position, [0., 0., -5.]);
        assert_point(scene.triangles[3].vertices[0].position, [-4., 0., 0.]);
        assert!(doc.scene(5).is_err());
        let dedup = document(&blocks, &insert("shape", "70\n100000\n71\n100000\n"), "");
        assert_eq!(dedup.scene(1).unwrap().triangles.len(), 1);
    }

    #[test]
    fn colors_inherit_through_nested_blocks_and_layer_zero() {
        let layers = "0\nLAYER\n2\nBLUE\n62\n5\n420\n1122986\n0\nLAYER\n2\nGREEN\n62\n3\n";
        let faces = triangle("")
            + &triangle("62\n0\n")
            + &triangle("8\nGREEN\n")
            + &triangle("62\n1\n420\n1193046\n");
        let blocks = block("child", [0.; 3], &faces)
            + &block("parent", [0.; 3], &insert("child", "62\n0\n"));
        let scene = document(&blocks, &insert("parent", "8\nBLUE\n62\n1\n"), layers)
            .scene(10)
            .unwrap();
        let colors = scene
            .triangles
            .iter()
            .map(|t| t.vertices[0].color)
            .collect::<Vec<_>>();
        assert_eq!(
            colors,
            vec![
                [17, 34, 170, 255],
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [18, 52, 86, 255]
            ]
        );
    }

    #[test]
    fn hidden_frozen_and_paper_space_geometry_is_omitted() {
        let layers = "0\nLAYER\n2\nOFF\n62\n-2\n0\nLAYER\n2\nFROZEN\n62\n3\n70\n1\n";
        let faces = triangle("8\nOFF\n")
            + &triangle("8\nFROZEN\n")
            + &triangle("60\n1\n")
            + &triangle("67\n1\n")
            + &triangle("");
        assert_eq!(
            document("", &faces, layers)
                .scene(10)
                .unwrap()
                .triangles
                .len(),
            1
        );
        let blocks = block("shape", [0.; 3], &triangle(""));
        assert!(document(&blocks, &insert("shape", "8\nOFF\n"), layers)
            .scene(10)
            .unwrap()
            .triangles
            .is_empty());
    }

    #[test]
    fn rejects_cycles_missing_external_and_excessive_instances() {
        let cycle = block("A", [0.; 3], &insert("B", "")) + &block("B", [0.; 3], &insert("A", ""));
        assert!(document(&cycle, &insert("A", ""), "").scene(10).is_err());
        assert!(document("", &insert("missing", ""), "").scene(10).is_err());
        let external = block("X", [0.; 3], &triangle("")).replacen("10\n0", "70\n4\n10\n0", 1);
        assert!(document(&external, &insert("X", ""), "").scene(10).is_err());
        let empty = block("empty", [0.; 3], "");
        assert!(document(
            &empty,
            &insert("empty", "70\n1000\n71\n1000\n44\n1\n45\n1\n"),
            ""
        )
        .scene(10)
        .is_err());
        // Unused definitions do not alter the model-space drawing.
        assert_eq!(
            document(&cycle, &triangle(""), "")
                .scene(10)
                .unwrap()
                .triangles
                .len(),
            1
        );
    }

    #[test]
    fn deep_block_hierarchy_is_bounded() {
        let mut blocks = String::new();
        for i in 0..65 {
            blocks += &block(
                &format!("b{i}"),
                [0.; 3],
                &insert(&format!("b{}", i + 1), ""),
            );
        }
        blocks += &block("b65", [0.; 3], &triangle(""));
        assert!(document(&blocks, &insert("b0", ""), "").scene(10).is_err());
    }

    #[test]
    fn degenerate_polyface_instances_still_consume_work_budget() {
        let mut mesh = String::from("0\nPOLYLINE\n70\n64\n");
        for _ in 0..3 {
            mesh.push_str("0\nVERTEX\n70\n192\n10\n0\n20\n0\n30\n0\n");
        }
        for _ in 0..11 {
            mesh.push_str("0\nVERTEX\n70\n128\n71\n1\n72\n2\n73\n3\n");
        }
        mesh.push_str("0\nSEQEND\n");
        let blocks = block("degenerate", [0.; 3], &mesh);
        let entities = insert("degenerate", "70\n1000\n71\n100\n44\n1\n45\n1\n");
        assert!(document(&blocks, &entities, "").scene(5_000_000).is_err());
    }

    #[test]
    fn large_world_coordinates_retain_local_detail() {
        let mesh = face(
            [
                [1e12, 1e12, 1e12],
                [1e12 + 1., 1e12, 1e12],
                [1e12, 1e12 + 1., 1e12],
            ],
            "",
        );
        let scene = document("", &mesh, "").scene(1).unwrap();
        assert_point(scene.triangles[0].vertices[1].position, [1., 0., 0.]);
        assert_point(scene.triangles[0].vertices[2].position, [0., 0., -1.]);
    }

    #[test]
    fn polyfaces_support_hidden_edge_indices_and_vertex_colors() {
        let mesh="0\nPOLYLINE\n70\n64\n0\nVERTEX\n70\n192\n10\n0\n20\n0\n30\n0\n62\n1\n0\nVERTEX\n70\n192\n10\n1\n20\n0\n30\n0\n62\n2\n0\nVERTEX\n70\n192\n10\n0\n20\n1\n30\n0\n62\n3\n0\nVERTEX\n70\n128\n71\n-1\n72\n2\n73\n3\n0\nSEQEND\n";
        let scene = document("", mesh, "").scene(1).unwrap();
        assert_eq!(
            scene.triangles[0].vertices.map(|v| v.color),
            [[255, 0, 0, 255], [255, 255, 0, 255], [0, 255, 0, 255]]
        );
    }

    #[test]
    fn malformed_and_unsupported_inputs_fail() {
        for bytes in [
            b"AutoCAD Binary DXF\r\n".as_slice(),
            b"0\nSECTION\n2\n",
            b"0\nSECTION\n2\nENTITIES\n0\nEOF\n",
            b"0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nx\n0\nENDSEC\n0\nEOF\n",
        ] {
            assert!(Document::parse(bytes).is_err());
        }
        assert!(document("", "0\n3DSOLID\n", "").scene(1).is_err());
        assert!(ocs(DVec3::ZERO).is_err());
        assert_eq!(aci(1), [255, 0, 0, 255]);
        assert_eq!(aci(145), [63, 111, 127, 255]);
    }
}
