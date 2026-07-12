use super::{bone_tree::BoneTree, mesh::Mesh};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoneWeight {
    pub bone_id: String,
    pub weight_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightSet {
    pub mesh_id: String,
    pub topology_revision: u64,
    pub by_vertex: BTreeMap<u32, Vec<BoneWeight>>,
}

impl WeightSet {
    pub fn validate(&self, mesh: &Mesh, bones: &BoneTree) -> Result<(), String> {
        if self.mesh_id != mesh.mesh_id || self.topology_revision != mesh.topology_revision {
            return Err("weights bound to stale topology".into());
        }
        let vertices: BTreeSet<_> = mesh.vertices.iter().map(|v| v.vertex_id).collect();
        if self.by_vertex.len() != vertices.len()
            || self
                .by_vertex
                .keys()
                .any(|vertex| !vertices.contains(vertex))
        {
            return Err("weights must cover exactly the current mesh vertices".into());
        }
        let bone_ids: BTreeSet<_> = bones.bones.iter().map(|b| b.bone_id.as_str()).collect();
        for vertex in vertices {
            let influences = self.by_vertex.get(&vertex).ok_or("vertex has no weights")?;
            if influences.is_empty() || influences.len() > 4 {
                return Err("vertex must have one to four bone influences".into());
            }
            let mut seen = BTreeSet::new();
            let mut sum = 0u32;
            for influence in influences {
                if !bone_ids.contains(influence.bone_id.as_str())
                    || !seen.insert(&influence.bone_id)
                {
                    return Err("invalid bone influence".into());
                }
                sum = sum
                    .checked_add(influence.weight_ppm)
                    .ok_or("weight overflow")?;
            }
            if sum != 1_000_000 {
                return Err("vertex weights must sum to 1000000 ppm".into());
            }
        }
        Ok(())
    }

    pub fn normalized(mut values: Vec<(String, u64)>) -> Result<Vec<BoneWeight>, String> {
        values.retain(|(_, v)| *v > 0);
        values.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        values.truncate(4);
        let sum: u64 = values.iter().map(|v| v.1).sum();
        if sum == 0 {
            return Err("cannot normalize empty weights".into());
        }
        let count = values.len();
        let mut result = Vec::with_capacity(count);
        let mut assigned = 0u32;
        for (index, (bone, value)) in values.into_iter().enumerate() {
            let ppm = if index + 1 == count {
                1_000_000 - assigned
            } else {
                ((value * 1_000_000) / sum) as u32
            };
            assigned += ppm;
            result.push(BoneWeight {
                bone_id: bone,
                weight_ppm: ppm,
            });
        }
        Ok(result)
    }
}
