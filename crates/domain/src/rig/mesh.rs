use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    pub vertex_id: u32,
    pub x_milli_px: i64,
    pub y_milli_px: i64,
    pub u_ppm: i32,
    pub v_ppm: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Triangle(pub u32, pub u32, pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mesh {
    pub mesh_id: String,
    pub layer_id: String,
    pub topology_revision: u64,
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    pub fn validate(&self) -> Result<(), String> {
        if self.vertices.len() > 10_000 {
            return Err("mesh vertex limit exceeded".into());
        }
        let by_id: std::collections::BTreeMap<_, _> =
            self.vertices.iter().map(|v| (v.vertex_id, v)).collect();
        if by_id.len() != self.vertices.len() {
            return Err("duplicate vertex id".into());
        }
        for v in &self.vertices {
            if !(0..=1_000_000).contains(&v.u_ppm) || !(0..=1_000_000).contains(&v.v_ppm) {
                return Err("UV outside normalized range".into());
            }
        }
        let mut seen = BTreeSet::new();
        for Triangle(a, b, c) in &self.triangles {
            if a == b || b == c || a == c {
                return Err("triangle repeats a vertex".into());
            }
            let (va, vb, vc) = (
                by_id.get(a).ok_or("unknown triangle vertex")?,
                by_id.get(b).ok_or("unknown triangle vertex")?,
                by_id.get(c).ok_or("unknown triangle vertex")?,
            );
            let area2 = (vb.x_milli_px - va.x_milli_px) * (vc.y_milli_px - va.y_milli_px)
                - (vb.y_milli_px - va.y_milli_px) * (vc.x_milli_px - va.x_milli_px);
            if area2 == 0 {
                return Err("degenerate triangle".into());
            }
            let mut key = [*a, *b, *c];
            key.sort();
            if !seen.insert(key) {
                return Err("duplicate triangle".into());
            }
        }
        Ok(())
    }
}
