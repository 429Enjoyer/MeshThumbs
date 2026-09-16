//! Static MagicaVoxel 150/200 scenes. Only exposed voxel faces become triangles.
//! Format: https://github.com/ephtracy/voxel-model (base and extension specs).
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use anyhow::{ensure, Context};
use glam::{DMat3, DMat4, DVec3, Vec2, Vec3};

use crate::{Scene, Triangle, Vertex};

const MAX_RECORDS: usize = 100_000;
const MAX_VOXELS: usize = 8_000_000;
type Dict = BTreeMap<String, String>;

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> anyhow::Result<&'a [u8]> {
        ensure!(n <= self.0.len(), "truncated VOX data");
        let (out, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(out)
    }
    fn int(&mut self) -> anyhow::Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn count(&mut self, max: usize) -> anyhow::Result<usize> {
        let n = self.int()?;
        ensure!(
            n >= 0 && n as usize <= max,
            "invalid or excessive VOX count"
        );
        Ok(n as usize)
    }
    fn string(&mut self) -> anyhow::Result<String> {
        let n = self.count(1_048_576)?;
        Ok(std::str::from_utf8(self.take(n)?)?.to_owned())
    }
    fn dict(&mut self) -> anyhow::Result<Dict> {
        let n = self.count(MAX_RECORDS)?;
        let mut out = Dict::new();
        for _ in 0..n {
            let key = self.string()?;
            ensure!(
                out.insert(key, self.string()?).is_none(),
                "duplicate VOX attribute"
            );
        }
        Ok(out)
    }
}

struct Model<'a> {
    size: [usize; 3],
    voxels: &'a [u8],
}
enum Kind {
    Transform {
        child: i32,
        layer: i32,
        matrix: DMat4,
    },
    Group(Vec<i32>),
    Shape(usize),
}
struct Node {
    hidden: bool,
    kind: Kind,
}
struct File<'a> {
    models: Vec<Model<'a>>,
    palette: [[u8; 4]; 256],
    nodes: BTreeMap<i32, Node>,
    hidden_layers: HashSet<i32>,
}

fn hidden(d: &Dict) -> bool {
    d.get("_hidden").is_some_and(|v| v == "1")
}
fn frame(d: &Dict) -> anyhow::Result<i32> {
    let n = d
        .get("_f")
        .map(|v| v.parse::<i32>())
        .transpose()?
        .unwrap_or(0);
    ensure!(n >= 0, "negative VOX frame");
    Ok(n)
}
fn transform(d: &Dict) -> anyhow::Result<DMat4> {
    let rotation = d
        .get("_r")
        .map(|v| v.parse::<u8>())
        .transpose()?
        .unwrap_or(4);
    let a = (rotation & 3) as usize;
    let b = ((rotation >> 2) & 3) as usize;
    ensure!(
        a < 3 && b < 3 && a != b && rotation < 128,
        "invalid VOX rotation"
    );
    let mut rows = [[0.0; 3]; 3];
    for (row, col) in [a, b, 3 - a - b].into_iter().enumerate() {
        rows[row][col] = if rotation & (1 << (4 + row)) == 0 {
            1.0
        } else {
            -1.0
        };
    }
    let rotation = DMat3::from_cols_array_2d(&rows).transpose();
    let mut translation = DVec3::ZERO;
    if let Some(value) = d.get("_t") {
        let xyz = value
            .split_whitespace()
            .map(str::parse::<i32>)
            .collect::<Result<Vec<_>, _>>()?;
        ensure!(xyz.len() == 3, "invalid VOX translation");
        translation = DVec3::new(xyz[0] as f64, xyz[1] as f64, xyz[2] as f64);
    }
    Ok(DMat4::from_translation(translation) * DMat4::from_mat3(rotation))
}

// The specified default palette is a 6x6x6 RGB cube (excluding black), then
// ten red, green, blue, and gray shades. Entry zero means empty space.
fn default_palette() -> [[u8; 4]; 256] {
    let mut colors = [[0; 4]; 256];
    let mut i = 1;
    for r in [255, 204, 153, 102, 51, 0] {
        for g in [255, 204, 153, 102, 51, 0] {
            for b in [255, 204, 153, 102, 51, 0] {
                if r == 0 && g == 0 && b == 0 {
                    continue;
                }
                colors[i] = [r, g, b, 255];
                i += 1;
            }
        }
    }
    for channel in 0..4 {
        for value in [238, 221, 187, 170, 136, 119, 85, 68, 34, 17] {
            let mut c = [0, 0, 0, 255];
            if channel == 3 {
                c[..3].fill(value);
            } else {
                c[channel] = value;
            }
            colors[i] = c;
            i += 1;
        }
    }
    colors
}

impl<'a> File<'a> {
    fn parse(data: &'a [u8]) -> anyhow::Result<Self> {
        let mut r = Reader(data);
        ensure!(r.take(4)? == b"VOX ", "invalid MagicaVoxel signature");
        ensure!(
            matches!(r.int()?, 150 | 200),
            "unsupported VOX version (expected 150 or 200)"
        );
        ensure!(
            r.take(4)? == b"MAIN" && r.int()? == 0,
            "invalid VOX MAIN chunk"
        );
        let bytes = r.count(crate::MAX_MODEL_BYTES as usize)?;
        let mut chunks = Reader(r.take(bytes)?);
        ensure!(r.0.is_empty(), "trailing VOX data");
        let mut file = Self {
            models: Vec::new(),
            palette: default_palette(),
            nodes: BTreeMap::new(),
            hidden_layers: HashSet::new(),
        };
        let mut size = None;
        let mut records = 0;
        let mut voxels = 0;
        let mut pack = None;
        while !chunks.0.is_empty() {
            records += 1;
            ensure!(records <= MAX_RECORDS, "too many VOX chunks");
            let id = chunks.take(4)?;
            let n = chunks.count(crate::MAX_MODEL_BYTES as usize)?;
            let children = chunks.count(crate::MAX_MODEL_BYTES as usize)?;
            let mut c = Reader(chunks.take(n)?);
            // Published scene chunks are flat. Unknown chunks are skipped as a
            // unit, including their bounded child region, without recursion.
            chunks.take(children)?;
            match id {
                b"PACK" => {
                    ensure!(pack.is_none(), "duplicate VOX PACK");
                    pack = Some(c.count(MAX_RECORDS)?);
                }
                b"SIZE" => {
                    ensure!(size.is_none(), "VOX SIZE without XYZI");
                    let s = [c.count(256)?, c.count(256)?, c.count(256)?];
                    ensure!(s.iter().all(|&n| n > 0), "empty VOX dimensions");
                    size = Some(s);
                }
                b"XYZI" => {
                    let s = size.take().context("VOX XYZI without SIZE")?;
                    let n = c.count(MAX_VOXELS)?;
                    voxels += n;
                    ensure!(
                        voxels <= MAX_VOXELS,
                        "VOX exceeds eight million stored voxels"
                    );
                    let data = c.take(n * 4)?;
                    ensure!(
                        data.as_chunks::<4>()
                            .0
                            .iter()
                            .all(|v| v[3] != 0 && (0..3).all(|i| (v[i] as usize) < s[i])),
                        "invalid VOX voxel coordinate or color index"
                    );
                    file.models.push(Model {
                        size: s,
                        voxels: data,
                    });
                }
                b"RGBA" => {
                    for i in 1..256 {
                        file.palette[i].copy_from_slice(c.take(4)?);
                    }
                    c.take(4)?; // Reserved palette index zero.
                }
                b"LAYR" => {
                    let id = c.int()?;
                    let d = c.dict()?;
                    ensure!(c.int()? == -1, "invalid VOX layer reserved id");
                    if hidden(&d) {
                        file.hidden_layers.insert(id);
                    }
                }
                b"nTRN" | b"nGRP" | b"nSHP" => {
                    let node_id = c.int()?;
                    ensure!(node_id >= 0, "negative VOX node id");
                    let d = c.dict()?;
                    let kind = match id {
                        b"nTRN" => {
                            let child = c.int()?;
                            ensure!(c.int()? == -1, "invalid VOX transform reserved id");
                            let layer = c.int()?;
                            let count = c.count(MAX_RECORDS)?;
                            ensure!(count > 0, "empty VOX transform frames");
                            let mut selected = None;
                            for _ in 0..count {
                                let d = c.dict()?;
                                let index = frame(&d)?;
                                let matrix = transform(&d)?;
                                if selected.as_ref().is_none_or(|(f, _)| index < *f) {
                                    selected = Some((index, matrix));
                                }
                            }
                            Kind::Transform {
                                child,
                                layer,
                                matrix: selected.context("missing VOX frame")?.1,
                            }
                        }
                        b"nGRP" => {
                            let count = c.count(MAX_RECORDS)?;
                            let mut children = Vec::with_capacity(count);
                            for _ in 0..count {
                                children.push(c.int()?);
                            }
                            Kind::Group(children)
                        }
                        _ => {
                            let count = c.count(MAX_RECORDS)?;
                            ensure!(count > 0, "empty VOX shape frames");
                            let mut selected = None;
                            for _ in 0..count {
                                let model = c.count(MAX_RECORDS)?;
                                let index = frame(&c.dict()?)?;
                                if selected.as_ref().is_none_or(|(f, _)| index < *f) {
                                    selected = Some((index, model));
                                }
                            }
                            Kind::Shape(selected.context("missing VOX shape")?.1)
                        }
                    };
                    ensure!(
                        file.nodes
                            .insert(
                                node_id,
                                Node {
                                    hidden: hidden(&d),
                                    kind
                                }
                            )
                            .is_none(),
                        "duplicate VOX node id"
                    );
                }
                _ => continue,
            }
            ensure!(c.0.is_empty() && children == 0, "invalid VOX chunk length");
        }
        ensure!(size.is_none(), "VOX SIZE without XYZI");
        if let Some(count) = pack {
            ensure!(count == file.models.len(), "VOX PACK model count mismatch");
        }
        Ok(file)
    }

    fn instances(&self) -> anyhow::Result<Vec<(usize, DMat4)>> {
        // Legacy PACK stores animation frames, not an assembly. Show frame one.
        if self.nodes.is_empty() {
            return Ok(if self.models.is_empty() {
                vec![]
            } else {
                vec![(0, DMat4::IDENTITY)]
            });
        }
        let mut referenced = HashSet::new();
        for node in self.nodes.values() {
            let children = match &node.kind {
                Kind::Transform { child, .. } => std::slice::from_ref(child),
                Kind::Group(children) => children.as_slice(),
                Kind::Shape(model) => {
                    ensure!(*model < self.models.len(), "missing VOX model");
                    &[]
                }
            };
            for child in children {
                ensure!(self.nodes.contains_key(child), "missing VOX child node");
                referenced.insert(*child);
            }
        }
        let mut instances = Vec::new();
        let mut seen = HashSet::new();
        let mut visits = 0;
        for id in self.nodes.keys().filter(|id| !referenced.contains(id)) {
            self.visit(
                *id,
                DMat4::IDENTITY,
                false,
                &mut Vec::new(),
                &mut seen,
                &mut visits,
                &mut instances,
            )?;
        }
        ensure!(
            seen.len() == self.nodes.len(),
            "unreachable or cyclic VOX scene"
        );
        Ok(instances)
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(
        &self,
        id: i32,
        parent: DMat4,
        hidden_parent: bool,
        path: &mut Vec<i32>,
        seen: &mut HashSet<i32>,
        visits: &mut usize,
        out: &mut Vec<(usize, DMat4)>,
    ) -> anyhow::Result<()> {
        *visits += 1;
        ensure!(
            *visits <= MAX_RECORDS && path.len() < 64,
            "VOX scene expansion limit exceeded"
        );
        ensure!(!path.contains(&id), "cyclic VOX scene");
        path.push(id);
        seen.insert(id);
        let node = &self.nodes[&id];
        let hidden = hidden_parent || node.hidden;
        match &node.kind {
            Kind::Transform {
                child,
                layer,
                matrix,
            } => self.visit(
                *child,
                parent * *matrix,
                hidden || self.hidden_layers.contains(layer),
                path,
                seen,
                visits,
                out,
            )?,
            Kind::Group(children) => {
                for child in children {
                    self.visit(*child, parent, hidden, path, seen, visits, out)?;
                }
            }
            Kind::Shape(model) => {
                if !hidden {
                    out.push((*model, parent));
                }
            }
        }
        path.pop();
        Ok(())
    }

    fn scene(&self, budget: usize) -> anyhow::Result<Scene> {
        let instances = self.instances()?;
        let mut work = 0;
        let mut cells = 0;
        let mut scene = Scene::new();
        let origin = instances
            .first()
            .map(|(_, m)| m.transform_point3(DVec3::ZERO))
            .unwrap_or(DVec3::ZERO);
        for (id, transform) in instances {
            let model = &self.models[id];
            work += model.voxels.len() / 4;
            ensure!(work <= 32_000_000, "VOX exceeds expanded voxel limit");
            let [sx, sy, sz] = model.size;
            cells += sx * sy * sz;
            ensure!(cells <= 256_000_000, "VOX exceeds expanded grid limit");
            let index = |x: usize, y: usize, z: usize| x + sx * (y + sy * z);
            let mut grid = vec![0u8; sx * sy * sz];
            for v in model.voxels.as_chunks::<4>().0 {
                let i = index(v[0] as usize, v[1] as usize, v[2] as usize);
                ensure!(grid[i] == 0, "duplicate VOX voxel");
                grid[i] = v[3];
            }
            // MagicaVoxel scene pivots use integer half-dimensions, including
            // odd sizes. Convert Z-up to the renderer's right-handed Y-up.
            let pivot = DVec3::new((sx / 2) as f64, (sy / 2) as f64, (sz / 2) as f64);
            let position = |p: DVec3| {
                let p = transform.transform_point3(p - pivot) - origin;
                Vec3::new(p.x as f32, p.z as f32, -p.y as f32)
            };
            for v in model.voxels.as_chunks::<4>().0 {
                let xyz = [v[0] as usize, v[1] as usize, v[2] as usize];
                let color = self.palette[v[3] as usize];
                if color[3] == 0 {
                    continue;
                }
                for axis in 0..3 {
                    let u = (axis + 1) % 3;
                    let w = (axis + 2) % 3;
                    for positive in [false, true] {
                        let mut neighbor = xyz;
                        let inside = if positive {
                            neighbor[axis] += 1;
                            neighbor[axis] < model.size[axis]
                        } else if neighbor[axis] > 0 {
                            neighbor[axis] -= 1;
                            true
                        } else {
                            false
                        };
                        if inside {
                            let other = grid[index(neighbor[0], neighbor[1], neighbor[2])];
                            if other != 0
                                && (self.palette[other as usize][3] == 255 || other == v[3])
                            {
                                continue;
                            }
                        }
                        if scene.triangles.len() + 2 > budget {
                            return Err(crate::RenderError::TooManyTriangles {
                                actual: scene.triangles.len() + 2,
                                limit: budget,
                            }
                            .into());
                        }
                        let mut p = DVec3::new(xyz[0] as f64, xyz[1] as f64, xyz[2] as f64);
                        if positive {
                            p[axis] += 1.0;
                        }
                        let mut du = DVec3::ZERO;
                        du[u] = 1.0;
                        let mut dv = DVec3::ZERO;
                        dv[w] = 1.0;
                        let corners = [
                            position(p),
                            position(p + du),
                            position(p + du + dv),
                            position(p + dv),
                        ];
                        let reverse = !positive ^ (transform.determinant() < 0.0);
                        for mut tri in [[0, 1, 2], [0, 2, 3]] {
                            if reverse {
                                tri.swap(1, 2);
                            }
                            let vertices = tri.map(|i| Vertex {
                                position: corners[i],
                                normal: Vec3::ZERO,
                                uv: Vec2::ZERO,
                                color: [255; 4],
                            });
                            scene.triangles.push(Triangle {
                                vertices: super::fix_normals(vertices),
                                color,
                                texture: None,
                            });
                        }
                    }
                }
            }
        }
        Ok(scene)
    }
}

pub(super) fn load(path: &Path, budget: usize) -> anyhow::Result<Scene> {
    let data = std::fs::read(path)?;
    ensure!(
        data.len() <= crate::MAX_MODEL_BYTES as usize,
        "VOX exceeds file size limit"
    );
    File::parse(&data)?.scene(budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ints(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|n| n.to_le_bytes()).collect()
    }
    fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        [id.as_slice(), &ints(&[data.len() as i32, 0]), data].concat()
    }
    fn dict(values: &[(&str, &str)]) -> Vec<u8> {
        let mut out = ints(&[values.len() as i32]);
        for (k, v) in values {
            for s in [k, v] {
                out.extend(ints(&[s.len() as i32]));
                out.extend(s.as_bytes());
            }
        }
        out
    }
    fn file(chunks: &[Vec<u8>]) -> Vec<u8> {
        let data = chunks.concat();
        [
            b"VOX ".as_slice(),
            &ints(&[150]),
            b"MAIN",
            &ints(&[0, data.len() as i32]),
            &data,
        ]
        .concat()
    }
    fn model(size: [i32; 3], voxels: &[[u8; 4]]) -> Vec<Vec<u8>> {
        vec![
            chunk(b"SIZE", &ints(&size)),
            chunk(
                b"XYZI",
                &[ints(&[voxels.len() as i32]), voxels.concat()].concat(),
            ),
        ]
    }
    fn shape(id: i32, models: &[(i32, i32)]) -> Vec<u8> {
        let mut data = [ints(&[id]), dict(&[]), ints(&[models.len() as i32])].concat();
        for (model, frame) in models {
            data.extend(ints(&[*model]));
            data.extend(dict(&[("_f", &frame.to_string())]));
        }
        chunk(b"nSHP", &data)
    }
    fn trn(id: i32, child: i32, layer: i32, frames: &[Vec<u8>]) -> Vec<u8> {
        chunk(
            b"nTRN",
            &[
                ints(&[id]),
                dict(&[]),
                ints(&[child, -1, layer, frames.len() as i32]),
                frames.concat(),
            ]
            .concat(),
        )
    }
    fn group(id: i32, children: &[i32]) -> Vec<u8> {
        chunk(
            b"nGRP",
            &[
                ints(&[id]),
                dict(&[]),
                ints(&[children.len() as i32]),
                ints(children),
            ]
            .concat(),
        )
    }

    #[test]
    fn removes_internal_faces_and_enforces_budget() {
        let data = file(&model([2, 1, 1], &[[0, 0, 0, 1], [1, 0, 0, 2]]));
        let f = File::parse(&data).unwrap();
        let scene = f.scene(20).unwrap();
        assert_eq!(scene.triangles.len(), 20);
        assert!(f.scene(19).is_err());
        for t in scene.triangles {
            assert!((t.vertices[0].normal.length() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn palette_indices_and_default_colors() {
        let p = default_palette();
        assert_eq!(p[1], [255; 4]);
        assert_eq!(p[2], [255, 255, 204, 255]);
        assert_eq!(p[216], [238, 0, 0, 255]);
        assert_eq!(p[255], [17, 17, 17, 255]);
        let mut chunks = model([1; 3], &[[0, 0, 0, 1]]);
        let mut colors = vec![0; 1024];
        colors[..4].copy_from_slice(&[12, 34, 56, 255]);
        chunks.push(chunk(b"RGBA", &colors));
        let data = file(&chunks);
        assert_eq!(
            File::parse(&data).unwrap().scene(12).unwrap().triangles[0].color,
            [12, 34, 56, 255]
        );
    }

    #[test]
    fn hidden_voxels_do_not_occlude_visible_faces() {
        let mut chunks = model([2, 1, 1], &[[0, 0, 0, 1], [1, 0, 0, 2]]);
        let mut colors = vec![0; 1024];
        colors[..4].copy_from_slice(&[255; 4]);
        chunks.push(chunk(b"RGBA", &colors));
        let data = file(&chunks);
        assert_eq!(
            File::parse(&data)
                .unwrap()
                .scene(12)
                .unwrap()
                .triangles
                .len(),
            12
        );
    }

    #[test]
    fn hierarchy_earliest_frames_and_hidden_layers() {
        let mut chunks = model([1; 3], &[[0, 0, 0, 1]]);
        chunks.extend(model([1; 3], &[[0, 0, 0, 2]]));
        chunks.extend([
            trn(0, 1, -1, &[dict(&[("_t", "10 0 0")])]),
            group(1, &[2, 4]),
            trn(
                2,
                3,
                0,
                &[
                    dict(&[("_f", "12"), ("_t", "100 0 0")]),
                    dict(&[("_f", "0"), ("_t", "2 3 4")]),
                ],
            ),
            shape(3, &[(1, 10), (0, 0)]),
            trn(4, 5, 7, &[dict(&[])]),
            shape(5, &[(1, 0)]),
            chunk(
                b"LAYR",
                &[ints(&[7]), dict(&[("_hidden", "1")]), ints(&[-1])].concat(),
            ),
        ]);
        let data = file(&chunks);
        let f = File::parse(&data).unwrap();
        let instances = f.instances().unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].0, 0);
        assert_eq!(
            instances[0].1.transform_point3(DVec3::ZERO),
            DVec3::new(12.0, 3.0, 4.0)
        );
        assert_eq!(f.scene(12).unwrap().triangles.len(), 12);
    }

    #[test]
    fn signed_rotation_and_odd_model_pivot() {
        let d = Dict::from([("_r".into(), "17".into()), ("_t".into(), "1 2 3".into())]);
        assert_eq!(
            transform(&d)
                .unwrap()
                .transform_point3(DVec3::new(4.0, 5.0, 6.0)),
            DVec3::new(-4.0, 6.0, 9.0)
        );
        let data = file(&model([3, 1, 1], &[[0, 0, 0, 1]]));
        let scene = File::parse(&data).unwrap().scene(12).unwrap();
        let min_x = scene
            .triangles
            .iter()
            .flat_map(|t| t.vertices)
            .map(|v| v.position.x)
            .fold(f32::INFINITY, f32::min);
        assert_eq!(min_x, -1.0);
    }

    #[test]
    fn pack_is_animation_not_overlapping_models() {
        let mut chunks = vec![chunk(b"PACK", &ints(&[2]))];
        chunks.extend(model([1; 3], &[[0, 0, 0, 1]]));
        chunks.extend(model([1; 3], &[[0, 0, 0, 2]]));
        let data = file(&chunks);
        let f = File::parse(&data).unwrap();
        assert_eq!(f.scene(12).unwrap().triangles.len(), 12);
        assert_eq!(f.scene(12).unwrap().triangles[0].color, [255; 4]);
    }

    #[test]
    fn rejects_truncation_at_every_byte() {
        let data = file(&model([1; 3], &[[0, 0, 0, 1]]));
        for len in 0..data.len() {
            assert!(File::parse(&data[..len]).is_err(), "length {len}");
        }
    }

    #[test]
    fn rejects_bad_indices_dimensions_counts_and_duplicate_voxels() {
        for (size, voxel) in [
            ([0, 1, 1], [0, 0, 0, 1]),
            ([257, 1, 1], [0, 0, 0, 1]),
            ([1; 3], [1, 0, 0, 1]),
            ([1; 3], [0; 4]),
        ] {
            assert!(File::parse(&file(&model(size, &[voxel]))).is_err());
        }
        assert!(File::parse(&file(&[chunk(b"PACK", &ints(&[-1]))])).is_err());
        let data = file(&model([1; 3], &[[0, 0, 0, 1]; 2]));
        assert!(File::parse(&data).unwrap().scene(100).is_err());
    }

    #[test]
    fn rejects_cycles_missing_references_and_duplicate_nodes() {
        for nodes in [
            vec![group(0, &[0])],
            vec![group(0, &[1]), group(1, &[0])],
            vec![group(0, &[99])],
            vec![shape(0, &[(99, 0)])],
        ] {
            let data = file(&nodes);
            assert!(File::parse(&data).unwrap().instances().is_err());
        }
        assert!(File::parse(&file(&[group(0, &[]), group(0, &[])])).is_err());
    }

    #[test]
    fn bounds_depth_and_accepts_instances() {
        let mut nodes = (0..65).map(|i| group(i, &[i + 1])).collect::<Vec<_>>();
        nodes.push(group(65, &[]));
        let data = file(&nodes);
        assert!(File::parse(&data).unwrap().instances().is_err());
        let mut chunks = model([1; 3], &[[0, 0, 0, 1]]);
        chunks.extend([group(0, &[1, 1]), shape(1, &[(0, 0)])]);
        let data = file(&chunks);
        assert_eq!(
            File::parse(&data)
                .unwrap()
                .scene(24)
                .unwrap()
                .triangles
                .len(),
            24
        );
    }

    #[test]
    fn skips_unknown_chunks_and_rejects_invalid_rotation() {
        let mut chunks = model([1; 3], &[[0, 0, 0, 1]]);
        chunks.push(chunk(b"NOTE", b"ignored metadata"));
        assert!(File::parse(&file(&chunks)).is_ok());
        for rotation in ["0", "3", "15", "128", "999", "no"] {
            assert!(transform(&Dict::from([("_r".into(), rotation.into())])).is_err());
        }
    }
}
