use super::{integers, numbers3, token, value};
use anyhow::{ensure, Result};
use glam::{Mat3, Vec3};
use openusd::usd::{Prim, TimeCode};

#[derive(Default)]
pub(super) struct Geometry {
    pub points: Vec<Vec3>,
    pub counts: Vec<i32>,
    pub indices: Vec<i32>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
}
fn scalar(prim: &Prim, name: &str, time: TimeCode, fallback: f32) -> Result<f32> {
    let v = value(prim, name, time)?
        .map(|v| v.cast::<f32>())
        .transpose()?
        .unwrap_or(fallback);
    ensure!(v.is_finite() && v > 0.0, "invalid USD primitive dimension");
    Ok(v)
}
pub(super) fn read(prim: &Prim, kind: &str, time: TimeCode) -> Result<Geometry> {
    if kind == "Mesh" {
        return Ok(Geometry {
            points: numbers3(prim, "points", time)?
                .into_iter()
                .map(Vec3::from)
                .collect(),
            counts: integers(prim, "faceVertexCounts", time)?,
            indices: integers(prim, "faceVertexIndices", time)?,
            ..Default::default()
        });
    }
    let mut g = Geometry::default();
    if kind == "Cube" {
        let s = scalar(prim, "size", time, 2.0)? * 0.5;
        let p = [
            [-s, -s, -s],
            [s, -s, -s],
            [s, s, -s],
            [-s, s, -s],
            [-s, -s, s],
            [s, -s, s],
            [s, s, s],
            [-s, s, s],
        ]
        .map(Vec3::from);
        for face in [
            [4, 5, 6, 7],
            [1, 0, 3, 2],
            [5, 1, 2, 6],
            [0, 4, 7, 3],
            [7, 6, 2, 3],
            [0, 1, 5, 4],
        ] {
            let n = (p[face[1]] - p[face[0]])
                .cross(p[face[2]] - p[face[0]])
                .normalize();
            let start = g.points.len() as i32;
            g.counts.push(4);
            for (i, uv) in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
                .into_iter()
                .enumerate()
            {
                g.points.push(p[face[i]]);
                g.normals.push(n.to_array());
                g.uvs.push(uv);
                g.indices.push(start + i as i32);
            }
        }
    } else if kind == "Sphere" {
        let radius = scalar(prim, "radius", time, 1.0)?;
        let columns = 33;
        for row in 0..=16 {
            for col in 0..columns {
                let phi = row as f32 / 16.0 * std::f32::consts::PI;
                let theta = col as f32 / 32.0 * std::f32::consts::TAU;
                let n = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                g.points.push(n * radius);
                g.normals.push(n.to_array());
                g.uvs.push([col as f32 / 32.0, 1.0 - row as f32 / 16.0]);
            }
        }
        for row in 0..16 {
            for col in 0..32 {
                let a = row * columns + col;
                let b = a + columns;
                g.indices.extend_from_slice(&[a, b + 1, b, a, a + 1, b + 1]);
                g.counts.extend_from_slice(&[3, 3]);
            }
        }
    } else {
        let radius = scalar(prim, "radius", time, 1.0)?;
        let height = scalar(prim, "height", time, 2.0)?;
        let axis = match token(prim, "axis", time)?.as_deref().unwrap_or("Z") {
            "Z" => Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2),
            "X" => Mat3::from_rotation_z(-std::f32::consts::FRAC_PI_2),
            "Y" => Mat3::IDENTITY,
            _ => anyhow::bail!("invalid USD primitive axis"),
        };
        let top_radius = if kind == "Cone" { 0.0 } else { radius };
        for i in 0..32 {
            let u0 = i as f32 / 32.0;
            let u1 = (i + 1) as f32 / 32.0;
            let a = u0 * std::f32::consts::TAU;
            let b = u1 * std::f32::consts::TAU;
            let bottom0 = Vec3::new(a.cos() * radius, -height * 0.5, a.sin() * radius);
            let bottom1 = Vec3::new(b.cos() * radius, -height * 0.5, b.sin() * radius);
            let top0 = Vec3::new(a.cos() * top_radius, height * 0.5, a.sin() * top_radius);
            let top1 = Vec3::new(b.cos() * top_radius, height * 0.5, b.sin() * top_radius);
            let slope = (radius - top_radius) / height;
            let n0 = Vec3::new(a.cos(), slope, a.sin()).normalize();
            let n1 = Vec3::new(b.cos(), slope, b.sin()).normalize();
            let mut triangle = |vertices: [(Vec3, Vec3, [f32; 2]); 3]| {
                g.counts.push(3);
                for (p, n, uv) in vertices {
                    g.indices.push(g.points.len() as i32);
                    g.points.push(axis * p);
                    g.normals.push((axis * n).to_array());
                    g.uvs.push(uv);
                }
            };
            triangle([
                (bottom0, n0, [u0, 0.]),
                (top1, n1, [u1, 1.]),
                (bottom1, n1, [u1, 0.]),
            ]);
            if top_radius > 0.0 {
                triangle([
                    (bottom0, n0, [u0, 0.]),
                    (top0, n0, [u0, 1.]),
                    (top1, n1, [u1, 1.]),
                ]);
                triangle([
                    (Vec3::Y * height * 0.5, Vec3::Y, [0.5, 0.5]),
                    (top1, Vec3::Y, [b.cos() * 0.5 + 0.5, b.sin() * 0.5 + 0.5]),
                    (top0, Vec3::Y, [a.cos() * 0.5 + 0.5, a.sin() * 0.5 + 0.5]),
                ]);
            }
            triangle([
                (-Vec3::Y * height * 0.5, -Vec3::Y, [0.5, 0.5]),
                (
                    bottom0,
                    -Vec3::Y,
                    [a.cos() * 0.5 + 0.5, a.sin() * 0.5 + 0.5],
                ),
                (
                    bottom1,
                    -Vec3::Y,
                    [b.cos() * 0.5 + 0.5, b.sin() * 0.5 + 0.5],
                ),
            ]);
        }
    }
    Ok(g)
}
